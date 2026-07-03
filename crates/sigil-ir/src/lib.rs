//! IR for the Sigil assembler: fragments, sections, symbols, streaming, and image assembly.

pub mod backend;
pub use backend::Cpu;

pub mod expr;
pub mod fixup;
pub use expr::Expr;
pub use fixup::{Fixup, FixupKind};

use sigil_span::Span;

/// A contiguous run of raw bytes with source provenance and pending fixups.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataFragment {
    /// The raw bytes emitted by the assembler front-end (fixup sites hold placeholders).
    pub bytes: Vec<u8>,
    /// Relocations the linker patches into `bytes` after layout.
    pub fixups: Vec<Fixup>,
    /// The source span that produced these bytes.
    pub span: Span,
}

/// A label defined within a [`Section`], at a byte offset from the section start.
/// The linker computes its VMA = `vma_base.unwrap_or(lma) + offset`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Label {
    pub name: String,
    pub offset: u32,
}

/// A single unit of assembled content inside a [`Section`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Fragment {
    /// A raw-byte fragment (may carry fixups).
    Data(DataFragment),
    /// Emit `count` copies of `value` into the image (gap fill / padding).
    Fill { value: u8, count: u32, span: Span },
    /// Reserve `count` bytes of address space with NO image bytes (RAM `ds`
    /// under phase/dephase); contributes to VMA length, not to image length.
    Reserve { count: u32, span: Span },
}

/// A named, ordered collection of [`Fragment`]s laid out at a fixed LMA, whose
/// labels/PC are computed at `vma_base` (VMA≠LMA when phased).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Section {
    /// Section name (e.g. `"regionA"`, `"regionB"`).
    pub name: String,
    /// Which CPU this section's instruction bytes target.
    pub cpu: Cpu,
    /// VMA base for labels/PC; `None` ⇒ VMA == LMA.
    pub vma_base: Option<u32>,
    /// Load address: where this section's bytes land in the ROM image.
    pub lma: u32,
    /// Labels defined in this section (name + byte offset from section start).
    pub labels: Vec<Label>,
    /// Ordered list of fragments that make up this section.
    pub fragments: Vec<Fragment>,
}

impl Section {
    /// The VMA base labels/PC are measured from (`vma_base`, else `lma`).
    pub fn vma_origin(&self) -> u32 {
        self.vma_base.unwrap_or(self.lma)
    }

    /// Number of bytes this section contributes to the ROM image
    /// (`Data` + `Fill`; `Reserve` contributes nothing).
    pub fn image_len(&self) -> u32 {
        let mut n: u32 = 0;
        for frag in &self.fragments {
            n += match frag {
                Fragment::Data(d) => d.bytes.len() as u32,
                Fragment::Fill { count, .. } => *count,
                Fragment::Reserve { .. } => 0,
            };
        }
        n
    }

    /// Number of bytes of address space (VMA/PC) this section spans
    /// (`Data` + `Fill` + `Reserve`).
    pub fn vma_len(&self) -> u32 {
        let mut n: u32 = 0;
        for frag in &self.fragments {
            n += match frag {
                Fragment::Data(d) => d.bytes.len() as u32,
                Fragment::Fill { count, .. } => *count,
                Fragment::Reserve { count, .. } => *count,
            };
        }
        n
    }

    /// Concatenate every image-contributing fragment's bytes in order.
    pub fn image_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for frag in &self.fragments {
            match frag {
                Fragment::Data(data) => out.extend_from_slice(&data.bytes),
                Fragment::Fill { value, count, .. } => {
                    out.extend(std::iter::repeat_n(*value, *count as usize));
                }
                Fragment::Reserve { .. } => {}
            }
        }
        out
    }
}

/// A named symbol with an integer value (e.g. a label resolved to an address).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Symbol {
    /// The symbol's name as it appeared in source.
    pub name: String,
    /// The resolved integer value of the symbol.
    pub value: i64,
}

/// A complete assembled module composed of one or more [`Section`]s.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Module {
    /// Ordered list of sections.
    pub sections: Vec<Section>,
}

/// Trait for types that can receive a stream of raw bytes from a front-end.
pub trait Streamer {
    /// Append `bytes` to the current emission target.
    fn emit_bytes(&mut self, bytes: &[u8]);
}

/// Builds a [`Module`] by accepting streaming byte emissions from a front-end.
///
/// All emitted bytes are collected into a single `"text"` section.  Each call
/// to [`Streamer::emit_bytes`] produces one [`Fragment::Data`] entry so that
/// source provenance is preserved at fragment granularity.
pub struct ModuleBuilder {
    span: Span,
    fragments: Vec<Fragment>,
}

impl ModuleBuilder {
    /// Create a new builder whose fragments will carry `span` as provenance.
    pub fn new(span: Span) -> Self {
        ModuleBuilder { span, fragments: Vec::new() }
    }

    /// Consume the builder and return the assembled [`Module`].
    pub fn finish(self) -> Module {
        Module {
            sections: vec![Section {
                name: "text".to_string(),
                cpu: Cpu::Z80,
                vma_base: None,
                lma: 0,
                labels: Vec::new(),
                fragments: self.fragments,
            }],
        }
    }
}

impl Streamer for ModuleBuilder {
    /// Push `bytes` as a new [`DataFragment`] tagged with this builder's span.
    fn emit_bytes(&mut self, bytes: &[u8]) {
        self.fragments.push(Fragment::Data(DataFragment {
            bytes: bytes.to_vec(),
            fixups: Vec::new(),
            span: self.span,
        }));
    }
}

/// Flatten all sections of `module` into a single image by concatenating their
/// [`Section::image_bytes`] in order.
pub fn assemble_to_image(module: &Module) -> Vec<u8> {
    let mut out = Vec::new();
    for section in &module.sections {
        out.extend_from_slice(&section.image_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_span::{SourceId, Span};

    #[test]
    fn data_fragment_holds_bytes_and_span() {
        let span = Span { source: SourceId(0), start: 2, end: 5 };
        let frag = DataFragment { bytes: vec![0x00, 0x3E, 0x05], fixups: vec![], span };
        assert_eq!(frag.bytes, vec![0x00, 0x3E, 0x05]);
        assert_eq!(frag.span, span);
    }

    #[test]
    fn section_image_bytes_concatenates_fragments() {
        let span = Span { source: SourceId(0), start: 0, end: 0 };
        let section = Section {
            name: "text".to_string(),
            cpu: Cpu::Z80,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![
                Fragment::Data(DataFragment { bytes: vec![0x00, 0x3E], fixups: vec![], span }),
                Fragment::Data(DataFragment { bytes: vec![0x05], fixups: vec![], span }),
            ],
        };
        assert_eq!(section.image_bytes(), vec![0x00, 0x3E, 0x05]);
    }

    #[test]
    fn module_builder_streams_and_assembles() {
        let span = Span { source: SourceId(0), start: 0, end: 0 };
        let mut builder = ModuleBuilder::new(span);
        builder.emit_bytes(&[0x00, 0x3E, 0x05]);
        builder.emit_bytes(&[0x06, 0x0A]);
        let module = builder.finish();
        assert_eq!(module.sections.len(), 1);
        assert_eq!(module.sections[0].name, "text");
        assert_eq!(module.sections[0].fragments.len(), 2);
        assert_eq!(
            assemble_to_image(&module),
            vec![0x00, 0x3E, 0x05, 0x06, 0x0A]
        );
    }

    #[test]
    fn symbol_constructs() {
        let sym = Symbol { name: "start".to_string(), value: 0x1234 };
        assert_eq!(sym.name, "start");
        assert_eq!(sym.value, 0x1234);
    }

    #[test]
    fn section_is_phase_aware_and_fragments_size() {
        let span = Span { source: SourceId(0), start: 0, end: 0 };
        let sec = Section {
            name: "regionB".to_string(),
            cpu: Cpu::Z80,
            vma_base: Some(0x8000),
            lma: 0x60000,
            labels: vec![Label { name: "SfxBlobWinTab".to_string(), offset: 0x45F }],
            fragments: vec![
                Fragment::Data(DataFragment { bytes: vec![0x11, 0x00, 0x00], fixups: vec![], span }),
                Fragment::Fill { value: 0x00, count: 4, span },
                Fragment::Reserve { count: 8, span },
            ],
        };
        // Data(3) + Fill(4) contribute image bytes; Reserve(8) contributes NONE.
        assert_eq!(sec.image_len(), 3 + 4);
        assert_eq!(sec.image_bytes(), vec![0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        // VMA span (labels/PC) counts Reserve too.
        assert_eq!(sec.vma_len(), 3 + 4 + 8);
    }
}
