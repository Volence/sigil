//! IR for the Sigil assembler: fragments, sections, symbols, streaming, and image assembly.

use sigil_span::Span;

/// A contiguous run of raw bytes with source provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataFragment {
    /// The raw bytes emitted by the assembler front-end.
    pub bytes: Vec<u8>,
    /// The source span that produced these bytes.
    pub span: Span,
}

/// A single unit of assembled content inside a [`Section`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Fragment {
    /// A raw-byte fragment.
    Data(DataFragment),
}

/// A named, ordered collection of [`Fragment`]s that maps to a contiguous output region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Section {
    /// Section name (e.g. `"text"`, `"data"`).
    pub name: String,
    /// Ordered list of fragments that make up this section.
    pub fragments: Vec<Fragment>,
}

impl Section {
    /// Concatenate every fragment's bytes in order into a single image buffer.
    pub fn image_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for frag in &self.fragments {
            match frag {
                Fragment::Data(data) => out.extend_from_slice(&data.bytes),
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
        let frag = DataFragment { bytes: vec![0x00, 0x3E, 0x05], span };
        assert_eq!(frag.bytes, vec![0x00, 0x3E, 0x05]);
        assert_eq!(frag.span, span);
    }

    #[test]
    fn section_image_bytes_concatenates_fragments() {
        let span = Span { source: SourceId(0), start: 0, end: 0 };
        let section = Section {
            name: "text".to_string(),
            fragments: vec![
                Fragment::Data(DataFragment { bytes: vec![0x00, 0x3E], span }),
                Fragment::Data(DataFragment { bytes: vec![0x05], span }),
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
}
