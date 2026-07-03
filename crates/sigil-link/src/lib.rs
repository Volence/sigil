//! The Sigil linker: assign each section its LMA, compute label VMAs under
//! phase (VMA≠LMA), resolve every `Fixup` against the layout + symbol table,
//! materialize `Fill`/`Reserve`, and assemble the image.
//!
//! CPU-agnostic: consumes only `sigil-ir` types. Concrete backends are injected
//! upstream (the caller lowers instructions to `DataFragment`s first).

use sigil_ir::expr::Fold;
use sigil_ir::{Fixup, FixupKind, Fragment, Section, SymbolTable, SymbolValue};
use sigil_span::{Diagnostic, Level, Span};

/// One section's resolved bytes and where they load.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkedSection {
    pub name: String,
    pub lma: u32,
    pub bytes: Vec<u8>,
}

/// The result of a successful link: per-section resolved bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkedImage {
    pub sections: Vec<LinkedSection>,
}

impl LinkedImage {
    /// Look up a linked section by name.
    pub fn section(&self, name: &str) -> Option<&LinkedSection> {
        self.sections.iter().find(|s| s.name == name)
    }
}

/// Resolve `sections` into a `LinkedImage`, seeding the symbol table with
/// `stubs` (fixed external values, e.g. 68k leaf symbols in the harness).
/// Returns all diagnostics on failure.
pub fn link(sections: &[Section], stubs: &SymbolTable) -> Result<LinkedImage, Vec<Diagnostic>> {
    let mut diags: Vec<Diagnostic> = Vec::new();

    // Pass 1: build the symbol table — stubs first, then each section's labels
    // at their phased VMA (vma_origin + offset).
    let mut syms = stubs.clone();
    for sec in sections {
        let origin = sec.vma_origin();
        for label in &sec.labels {
            syms.define(&label.name, SymbolValue::Int((origin + label.offset) as i64));
        }
    }

    // Pass 2: per section, copy image bytes and apply fixups.
    let mut linked = Vec::new();
    for sec in sections {
        let mut bytes = sec.image_bytes();
        let origin = sec.vma_origin();

        // Walk fragments to find each Data fragment's byte offset within the
        // section image, so fixup offsets and site VMAs are correct.
        let mut frag_img_off: u32 = 0; // offset within the image bytes
        for frag in &sec.fragments {
            match frag {
                Fragment::Data(d) => {
                    for fx in &d.fixups {
                        let site_abs = frag_img_off + fx.offset; // offset within section image
                        let site_vma = origin + site_abs;
                        apply_fixup(&mut bytes, site_abs, site_vma, fx, &syms, sec.name.as_str(), d.span, &mut diags);
                    }
                    frag_img_off += d.bytes.len() as u32;
                }
                Fragment::Fill { count, .. } => frag_img_off += *count,
                Fragment::Reserve { .. } => {} // no image bytes
            }
        }

        linked.push(LinkedSection { name: sec.name.clone(), lma: sec.lma, bytes });
    }

    if diags.is_empty() {
        Ok(LinkedImage { sections: linked })
    } else {
        Err(diags)
    }
}

fn diag(message: String, span: Span) -> Diagnostic {
    Diagnostic { level: Level::Error, message, primary: span }
}

#[allow(clippy::too_many_arguments)]
fn apply_fixup(
    bytes: &mut [u8],
    site_abs: u32,
    site_vma: u32,
    fx: &Fixup,
    syms: &SymbolTable,
    section: &str,
    span: Span,
    diags: &mut Vec<Diagnostic>,
) {
    // Fold the target against the symbol table (global scope at link time; the
    // front-end will pre-qualify local names into fully-dotted `Sym`s in Plan 4).
    let value = match fx.target.fold(&|name| syms.resolve(name, None)) {
        Fold::Value(v) => v,
        Fold::Poison => {
            diags.push(diag(
                format!("unresolved fixup target in section {section} at offset {site_abs}"),
                span,
            ));
            return;
        }
    };

    match fx.kind {
        FixupKind::BankPtr16Le => {
            let v = value as u16;
            let lo = (v & 0xFF) as u8;
            let hi = (v >> 8) as u8;
            bytes[site_abs as usize] = lo;
            bytes[(site_abs + 1) as usize] = hi;
        }
        FixupKind::Z80JrRel8 => {
            // disp measured from the END of the 2-byte instruction. The opcode
            // is at site_abs-1; the instruction end VMA = (site_vma - 1) + 2.
            let inst_end_vma = (site_vma as i64 - 1) + 2;
            let disp = value - inst_end_vma;
            if !(-128..=127).contains(&disp) {
                diags.push(diag(
                    format!("jr/djnz displacement out of range ({disp}) in section {section}"),
                    span,
                ));
                return;
            }
            bytes[site_abs as usize] = disp as i8 as u8;
        }
        FixupKind::Abs16Be | FixupKind::Abs32Be => {
            diags.push(diag(
                format!("68000 fixup kind {:?} not supported in M0", fx.kind),
                span,
            ));
        }
    }
}

/// Materialize a full contiguous image: place each section's bytes at its LMA,
/// filling all gaps (and the head) with `fill`. Sections must not overlap.
pub fn flatten(image: &LinkedImage, fill: u8) -> Vec<u8> {
    let end = image
        .sections
        .iter()
        .map(|s| s.lma as usize + s.bytes.len())
        .max()
        .unwrap_or(0);
    let mut out = vec![fill; end];
    for s in &image.sections {
        let start = s.lma as usize;
        out[start..start + s.bytes.len()].copy_from_slice(&s.bytes);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_ir::{Cpu, DataFragment, Expr, Fixup, FixupKind, Fragment, Label, Section, SymbolTable, SymbolValue};
    use sigil_span::{SourceId, Span};

    fn span() -> Span {
        Span { source: SourceId(0), start: 0, end: 0 }
    }

    // Region B: defines SfxBlobWinTab at VMA base $8000 + offset $45F = $845F.
    fn region_b() -> Section {
        let frags = vec![
            // 0x45F bytes of filler so the label lands at offset 0x45F.
            Fragment::Fill { value: 0xAA, count: 0x45F, span: span() },
            // The table's first bytes (content irrelevant to this test).
            Fragment::Data(DataFragment { bytes: vec![0x9A, 0xD6], fixups: vec![], span: span() }),
        ];
        Section {
            name: "regionB".to_string(),
            cpu: Cpu::Z80,
            vma_base: Some(0x8000),
            lma: 0x60000,
            labels: vec![Label { name: "SfxBlobWinTab".to_string(), offset: 0x45F }],
            fragments: frags,
        }
    }

    // Region A: `ld de,SfxBlobWinTab` = 11 <lo> <hi>, fixup at offset 1.
    fn region_a() -> Section {
        Section {
            name: "regionA".to_string(),
            cpu: Cpu::Z80,
            vma_base: Some(0x0000),
            lma: 0x400,
            labels: vec![],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x11, 0x00, 0x00],
                fixups: vec![Fixup {
                    kind: FixupKind::BankPtr16Le,
                    offset: 1,
                    target: Expr::Sym("SfxBlobWinTab".to_string()),
                }],
                span: span(),
            })],
        }
    }

    #[test]
    fn cross_region_fixup_resolves_to_phased_vma_little_endian() {
        let linked = link(&[region_a(), region_b()], &SymbolTable::new()).unwrap();
        let a = linked.section("regionA").unwrap();
        // 11 5F 84  — $845F little-endian.
        assert_eq!(a.bytes, vec![0x11, 0x5F, 0x84]);
        assert_eq!(a.lma, 0x400);
    }

    #[test]
    fn dw_bank_pointer_from_functions_emits_little_endian() {
        // dw sfx_winptr(Sfx_33) with Sfx_33 stubbed to 0x6569A:
        //   (Sfx_33 & 0x7FFF) | 0x8000 = 0xD69A  → LE 9A D6.
        let mut stubs = SymbolTable::new();
        stubs.define("Sfx_33", SymbolValue::Int(0x6569A));
        let winptr = Expr::Binary {
            op: sigil_ir::expr::BinOp::Or,
            lhs: Box::new(Expr::Binary {
                op: sigil_ir::expr::BinOp::And,
                lhs: Box::new(Expr::Sym("Sfx_33".to_string())),
                rhs: Box::new(Expr::Int(0x7FFF)),
            }),
            rhs: Box::new(Expr::Int(0x8000)),
        };
        let sec = Section {
            name: "tab".to_string(),
            cpu: Cpu::Z80,
            vma_base: Some(0x8000),
            lma: 0x60000,
            labels: vec![],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x00, 0x00],
                fixups: vec![Fixup { kind: FixupKind::BankPtr16Le, offset: 0, target: winptr }],
                span: span(),
            })],
        };
        let linked = link(&[sec], &stubs).unwrap();
        assert_eq!(linked.section("tab").unwrap().bytes, vec![0x9A, 0xD6]);
    }

    #[test]
    fn z80_jr_rel8_in_range_resolves() {
        // A `jr` at VMA $8000 targeting VMA $8000 → disp = 0 - ... let target be site+2 → 0.
        // Fragment: [0x18, 0x00] with Z80JrRel8 fixup at offset 1 targeting VMA 0x8002.
        let sec = Section {
            name: "code".to_string(),
            cpu: Cpu::Z80,
            vma_base: Some(0x8000),
            lma: 0x60000,
            labels: vec![Label { name: "here".to_string(), offset: 2 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x18, 0x00],
                fixups: vec![Fixup { kind: FixupKind::Z80JrRel8, offset: 1, target: Expr::Sym("here".to_string()) }],
                span: span(),
            })],
        };
        let linked = link(&[sec], &SymbolTable::new()).unwrap();
        // site VMA of the disp byte's instruction = 0x8000; target = 0x8002; disp = 0x8002 - (0x8000 + 2) = 0.
        assert_eq!(linked.section("code").unwrap().bytes, vec![0x18, 0x00]);
    }

    #[test]
    fn z80_jr_rel8_out_of_range_diagnoses() {
        // Target 0x9000 from site 0x8000 → disp = 0x9000 - 0x8002 = 0xFFE (>127).
        let sec = Section {
            name: "code".to_string(),
            cpu: Cpu::Z80,
            vma_base: Some(0x8000),
            lma: 0x60000,
            labels: vec![Label { name: "far".to_string(), offset: 0x1000 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x18, 0x00],
                fixups: vec![Fixup { kind: FixupKind::Z80JrRel8, offset: 1, target: Expr::Sym("far".to_string()) }],
                span: span(),
            })],
        };
        let err = link(&[sec], &SymbolTable::new()).unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("out of range")), "got: {:?}", err);
    }

    #[test]
    fn unresolved_target_diagnoses() {
        let sec = region_a(); // references SfxBlobWinTab, which no section defines here.
        let err = link(&[sec], &SymbolTable::new()).unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("unresolved")), "got: {:?}", err);
    }

    #[test]
    fn flatten_places_sections_at_lma_with_gap_fill() {
        let a = Section {
            name: "a".to_string(),
            cpu: Cpu::Z80,
            vma_base: None,
            lma: 2,
            labels: vec![],
            fragments: vec![Fragment::Data(DataFragment { bytes: vec![0xAA, 0xBB], fixups: vec![], span: span() })],
        };
        let linked = link(&[a], &SymbolTable::new()).unwrap();
        // Bytes at LMA 2..4; positions 0,1 gap-filled with 0x00.
        assert_eq!(flatten(&linked, 0x00), vec![0x00, 0x00, 0xAA, 0xBB]);
    }
}
