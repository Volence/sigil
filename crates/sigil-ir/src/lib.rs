//! IR for the Sigil assembler: fragments, sections, symbols, streaming, and image assembly.

pub mod backend;
pub use backend::Cpu;

pub mod builder;
pub use builder::IrBuilder;

pub mod expr;
pub mod fixup;
pub mod map;
pub mod symbols;
mod width;
pub use expr::Expr;
pub use fixup::{Fixup, FixupKind};
pub use symbols::{SymbolTable, SymbolValue};
pub use width::{asl_width_rule, AbsWidth};

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

/// One width candidate of a [`Fragment::RelaxAbsSym`]: a complete instruction
/// encoding (with the operand-address bytes zeroed as a placeholder) together
/// with the single [`Fixup`] that patches its symbolic operand. The front-end
/// builds both the `abs.w` and `abs.l` candidates; `resolve_layout` selects one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelaxCandidate {
    /// The complete instruction bytes for this width (operand address zeroed).
    pub bytes: Vec<u8>,
    /// The operand relocation (`Abs16Be` for the `abs.w` candidate, `Abs32Be`
    /// for `abs.l`); its `offset` is relative to the start of `bytes`.
    pub fixup: Fixup,
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
    /// A bare-symbol `jmp`/`jsr` whose operand width (`abs.w`/`abs.l`) is not yet
    /// chosen — the ONLY length-variable fragment. `resolve_layout` (sigil-link)
    /// picks the width and lowers this to a `Data` fragment (opcode word +
    /// Abs16Be/Abs32Be operand fixup) BEFORE `link()` runs, so the helpers below
    /// never see it at link time.
    JmpJsrSym { is_jsr: bool, target: crate::expr::Expr, span: Span },
    /// A straight-line instruction carrying ONE symbolic absolute-address operand
    /// whose width (`abs.w`/`abs.l`) is deferred to the linker, so it can be
    /// lowered **byte-exact** to whichever width `asl_width_rule` picks for the
    /// resolved address of `target`. Like [`JmpJsrSym`](Self::JmpJsrSym) it is the
    /// full instruction encoding — but that encoding is done by the FRONT-END, not
    /// the linker: the caller supplies BOTH candidate encodings ([`short`] = the
    /// `abs.w` form, [`long`] = the `abs.l` form), each a complete byte block plus
    /// its own operand fixup (address bytes zeroed as placeholders), and
    /// `resolve_layout` (sigil-link) merely SELECTS one — no m68k encoding logic
    /// lives in the linker. Both candidates' fixups reference the SAME symbol
    /// (`target`), whose resolved address drives the width choice via
    /// `asl_width_rule`; `resolve_layout` lowers this to a `Data` fragment (the
    /// chosen candidate's bytes + fixup) BEFORE `link()` runs, so the helpers
    /// below never see it at link time. It is length-variable exactly like
    /// `JmpJsrSym` (contributes `short.bytes.len()` at baseline, `long` when it
    /// grows) and participates in the same relaxation fixpoint.
    ///
    /// [`short`]: RelaxCandidate
    /// [`long`]: RelaxCandidate
    RelaxAbsSym {
        /// The `abs.w` encoding: complete instruction bytes (operand address
        /// zeroed) plus its `Abs16Be` operand fixup at the right in-block offset.
        short: RelaxCandidate,
        /// The `abs.l` encoding: complete instruction bytes (operand address
        /// zeroed) plus its `Abs32Be` operand fixup at the right in-block offset.
        long: RelaxCandidate,
        /// The symbol whose resolved address selects `short` vs `long` (the same
        /// symbol both candidates' fixups reference).
        target: crate::expr::Expr,
        /// The source span that produced the instruction.
        span: Span,
    },
    /// AS `org <target>`: reposition the write cursor to `target` (a byte offset
    /// from the section start, already resolved by the front-end). Used both for
    /// the within-section back-patch idiom (`org Hdr / dc.b n / org End`, e.g.
    /// `parallax_section_end`) and — via the front-end's `directive_org` — as the
    /// trigger for a phase-like new section when `target` is beyond anything the
    /// section has written yet. `Section::image_bytes`/`vma_len` replay fragments
    /// with a write cursor that `Org` seeks (backward OR forward); forward seeks
    /// past the current image extent are gap-filled with `fill`.
    Org { target: u32, fill: u8, span: Span },
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
    /// (`Data` + `Fill`, replayed through a write cursor that `Org` may seek
    /// backward or forward; `Reserve` contributes nothing). Equal to the
    /// highest byte offset ever written, matching asl: a trailing back-patch
    /// `org` that never re-advances past the prior extent does not truncate
    /// the image (see `image_bytes`).
    pub fn image_len(&self) -> u32 {
        self.image_bytes().len() as u32
    }

    /// Number of bytes of address space (VMA/PC) this section spans
    /// (`Data` + `Fill` + `Reserve`, cursor-replayed the same way as
    /// `image_len`/`image_bytes` so `Org` seeks are accounted for).
    pub fn vma_len(&self) -> u32 {
        let mut cursor: u32 = 0;
        let mut max_extent: u32 = 0;
        for frag in &self.fragments {
            match frag {
                Fragment::Data(d) => cursor += d.bytes.len() as u32,
                Fragment::Fill { count, .. } => cursor += *count,
                Fragment::Reserve { count, .. } => cursor += *count,
                Fragment::Org { target, .. } => cursor = *target,
                Fragment::JmpJsrSym { .. } => {
                    unreachable!("JmpJsrSym must be lowered by resolve_layout before layout/link")
                }
                Fragment::RelaxAbsSym { .. } => {
                    unreachable!("RelaxAbsSym must be lowered by resolve_layout before layout/link")
                }
            }
            if cursor > max_extent {
                max_extent = cursor;
            }
        }
        max_extent
    }

    /// Replay every image-contributing fragment through a write cursor: `Data`/
    /// `Fill` write at the cursor and advance it; `Reserve` contributes no image
    /// bytes and leaves the cursor untouched; `Org` seeks the cursor to `target`
    /// — backward (into already-written bytes, which `Data`/`Fill` then
    /// overwrite in place) or forward (extending the image with `fill` bytes up
    /// to `target`, exactly like a real gap). The final image never shrinks: its
    /// length is the highest offset ever reached, so a trailing backward `org`
    /// that doesn't re-advance past the prior extent leaves the tail bytes
    /// intact (asl-confirmed).
    pub fn image_bytes(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        let mut cursor: usize = 0;
        for frag in &self.fragments {
            match frag {
                Fragment::Data(data) => {
                    let end = cursor + data.bytes.len();
                    if end > out.len() {
                        out.resize(end, 0);
                    }
                    out[cursor..end].copy_from_slice(&data.bytes);
                    cursor = end;
                }
                Fragment::Fill { value, count, .. } => {
                    let end = cursor + *count as usize;
                    if end > out.len() {
                        out.resize(end, 0);
                    }
                    for b in &mut out[cursor..end] {
                        *b = *value;
                    }
                    cursor = end;
                }
                Fragment::Reserve { .. } => {}
                Fragment::Org { target, fill, .. } => {
                    let t = *target as usize;
                    if t > out.len() {
                        out.resize(t, *fill);
                    }
                    cursor = t;
                }
                Fragment::JmpJsrSym { .. } => {
                    unreachable!("JmpJsrSym must be lowered by resolve_layout before layout/link")
                }
                Fragment::RelaxAbsSym { .. } => {
                    unreachable!("RelaxAbsSym must be lowered by resolve_layout before layout/link")
                }
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

    #[test]
    fn jmpjsr_sym_variant_constructs() {
        let f = Fragment::JmpJsrSym {
            is_jsr: true,
            target: Expr::Sym("Sub".into()),
            span: Span { source: SourceId(0), start: 0, end: 0 },
        };
        match f {
            Fragment::JmpJsrSym { is_jsr, .. } => assert!(is_jsr),
            _ => panic!("wrong variant"),
        }
    }

    /// The `org Hdr / dc.b n / org End` back-patch idiom (`parallax_section_end`):
    /// a backward `Org` seek overwrites the placeholder byte at offset 0 in
    /// place, then a forward `Org` seek (to the extent already reached) resumes
    /// so the trailing byte appends normally. asl-verified: `63 01 02 03 04`.
    #[test]
    fn org_backpatch_overwrites_in_place_and_resumes() {
        let span = Span { source: SourceId(0), start: 0, end: 0 };
        let sec = Section {
            name: "s".to_string(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![
                Fragment::Data(DataFragment { bytes: vec![0x00, 0x01, 0x02, 0x03], fixups: vec![], span }),
                Fragment::Org { target: 0, fill: 0x00, span }, // org Hdr (back to offset 0)
                Fragment::Data(DataFragment { bytes: vec![0x63], fixups: vec![], span }), // dc.b 99
                Fragment::Org { target: 4, fill: 0x00, span }, // org End (resume at offset 4)
                Fragment::Data(DataFragment { bytes: vec![0x04], fixups: vec![], span }),
            ],
        };
        // The byte at the back-patched offset (0x00) now differs from the
        // original placeholder — proving a real overwrite, not an append.
        assert_eq!(sec.image_bytes(), vec![0x63, 0x01, 0x02, 0x03, 0x04]);
        assert_eq!(sec.image_len(), 5);
    }

    /// A forward `Org` seek past the current image extent (asl's absolute-org
    /// and back-patch-resume behavior alike) gap-fills with the fragment's
    /// `fill` byte, growing the image rather than overwriting.
    #[test]
    fn org_forward_seek_gap_fills_with_fill_byte() {
        let span = Span { source: SourceId(0), start: 0, end: 0 };
        let sec = Section {
            name: "s".to_string(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![
                Fragment::Data(DataFragment { bytes: vec![1, 2, 3, 4], fixups: vec![], span }),
                Fragment::Org { target: 16, fill: 0x00, span },
                Fragment::Data(DataFragment { bytes: vec![5, 6], fixups: vec![], span }),
            ],
        };
        let mut want = vec![1, 2, 3, 4];
        want.extend(std::iter::repeat_n(0x00, 12));
        want.extend([5, 6]);
        assert_eq!(sec.image_bytes(), want);
        assert_eq!(sec.image_len(), 18);
    }

    /// A trailing backward `Org` that never re-advances past the prior extent
    /// must NOT truncate the image (asl-verified: the max offset ever written
    /// wins, not the final cursor position).
    #[test]
    fn org_trailing_backward_seek_does_not_truncate() {
        let span = Span { source: SourceId(0), start: 0, end: 0 };
        let sec = Section {
            name: "s".to_string(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![
                Fragment::Data(DataFragment {
                    bytes: vec![1, 2, 3, 4, 5, 6, 7, 8],
                    fixups: vec![],
                    span,
                }),
                Fragment::Org { target: 0, fill: 0x00, span },
                Fragment::Data(DataFragment { bytes: vec![0x63], fixups: vec![], span }),
            ],
        };
        assert_eq!(sec.image_bytes(), vec![0x63, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(sec.image_len(), 8);
    }
}
