//! The Sigil linker: assign each section its LMA, compute label VMAs under
//! phase (VMA≠LMA), resolve every `Fixup` against the layout + symbol table,
//! materialize `Fill`/`Reserve`, and assemble the image.
//!
//! CPU-agnostic: consumes only `sigil-ir` types. Concrete backends are injected
//! upstream (the caller lowers instructions to `DataFragment`s first).

use sigil_ir::expr::Fold;
use sigil_ir::map::MemoryMap;
use sigil_ir::{Expr, Fixup, FixupKind, Fragment, Section, SymbolTable, SymbolValue};
use sigil_span::{Diagnostic, Level, Span};

mod relax;
pub use relax::{asl_width_rule, AbsWidth};

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
///
/// Symbol redefinition (the same name defined by multiple sections/stubs) is
/// currently last-write-wins; collision diagnostics tied to the real producer
/// land in Plan 4 when the front-end drives this.
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
                        // The WHOLE fixup (offset..offset+width) must fit within
                        // THIS fragment's own bytes; otherwise a multi-byte write
                        // would silently clobber the next fragment.
                        let width = fx.kind.byte_width() as usize;
                        if fx.offset as usize + width > d.bytes.len() {
                            diags.push(diag(
                                format!(
                                    "fixup at offset {} (width {}) exceeds fragment length {} in section {}",
                                    fx.offset,
                                    width,
                                    d.bytes.len(),
                                    sec.name
                                ),
                                d.span,
                            ));
                            continue;
                        }
                        let site_abs = frag_img_off + fx.offset; // offset within section image
                        let site_vma = origin + site_abs;
                        apply_fixup(&mut bytes, site_abs, site_vma, fx, &syms, sec.name.as_str(), d.span, &mut diags);
                    }
                    frag_img_off += d.bytes.len() as u32;
                }
                Fragment::Fill { count, .. } => frag_img_off += *count,
                Fragment::Reserve { .. } => {} // no image bytes
                Fragment::JmpJsrSym { .. } => {
                    unreachable!("JmpJsrSym must be lowered by resolve_layout before link")
                }
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
            let what = match &fx.target {
                Expr::Sym(name) => format!("symbol `{name}`"),
                _ => "target expression".to_string(),
            };
            diags.push(diag(
                format!("unresolved {what} for fixup in section {section} at offset {site_abs}"),
                span,
            ));
            return;
        }
    };

    match fx.kind {
        FixupKind::BankPtr16Le => {
            if (site_abs as usize) + 1 >= bytes.len() {
                diags.push(diag(
                    format!("BankPtr16Le fixup at offset {site_abs} would write past section end in section {section}"),
                    span,
                ));
                return;
            }
            let v = value as u16;
            let lo = (v & 0xFF) as u8;
            let hi = (v >> 8) as u8;
            bytes[site_abs as usize] = lo;
            bytes[(site_abs + 1) as usize] = hi;
        }
        FixupKind::Z80JrRel8 => {
            if site_abs as usize >= bytes.len() {
                diags.push(diag(
                    format!("Z80JrRel8 fixup at offset {site_abs} would write past section end in section {section}"),
                    span,
                ));
                return;
            }
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
        FixupKind::Abs16Be => {
            // abs.w holds a sign-extended 16-bit address: the VMA must fit i16
            // (asl errors otherwise; matching that keeps us byte-exact).
            let v = value as i64;
            if !(-0x8000..=0x7FFF).contains(&v) && !(0xFF_8000..=0xFF_FFFF).contains(&(v & 0xFF_FFFF)) {
                diags.push(diag(
                    format!("value {v:#X} does not fit abs.w (16-bit sign-extended) in section {section}"),
                    span,
                ));
                return;
            }
            let w = (value & 0xFFFF) as u16;
            bytes[site_abs as usize] = (w >> 8) as u8;
            bytes[site_abs as usize + 1] = (w & 0xFF) as u8;
        }
        FixupKind::Abs32Be => {
            let w = value as u32;
            bytes[site_abs as usize] = (w >> 24) as u8;
            bytes[site_abs as usize + 1] = (w >> 16) as u8;
            bytes[site_abs as usize + 2] = (w >> 8) as u8;
            bytes[site_abs as usize + 3] = (w & 0xFF) as u8;
        }
        FixupKind::PcRel8 => {
            // disp measured from op+2; the disp byte sits at op+1 = site_vma.
            let disp = value - (site_vma as i64 + 1);
            if !(-128..=127).contains(&disp) {
                diags.push(diag(format!("bra.s/Bcc.s displacement out of range ({disp}) in section {section}"), span));
                return;
            }
            bytes[site_abs as usize] = disp as i8 as u8;
        }
        FixupKind::PcRelDisp16 => {
            // disp measured from the extension word's own VMA = site_vma.
            let disp = value - site_vma as i64;
            if !(-0x8000..=0x7FFF).contains(&disp) {
                diags.push(diag(format!("(d16,PC)/bra.w displacement out of range ({disp}) in section {section}"), span));
                return;
            }
            let w = disp as i16 as u16;
            bytes[site_abs as usize] = (w >> 8) as u8;
            bytes[site_abs as usize + 1] = (w & 0xFF) as u8;
        }
        FixupKind::PcRelDisp8 => {
            let disp = value - site_vma as i64;
            if !(-128..=127).contains(&disp) {
                diags.push(diag(format!("(d8,PC,Xn) displacement out of range ({disp}) in section {section}"), span));
                return;
            }
            bytes[site_abs as usize] = disp as i8 as u8;
        }
        FixupKind::HeaderChecksum => {
            diags.push(diag("HeaderChecksum is a post-image pass, not an in-fragment fixup".into(), span));
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

/// Like `flatten`, but errors if any two sections' `[lma, lma+len)` ranges
/// overlap (a mis-assigned LMA map would otherwise silently clobber bytes).
pub fn flatten_checked(image: &LinkedImage, fill: u8) -> Result<Vec<u8>, String> {
    let mut ranges: Vec<(usize, usize, &str)> =
        image.sections.iter().map(|s| (s.lma as usize, s.lma as usize + s.bytes.len(), s.name.as_str())).collect();
    ranges.sort_by_key(|r| r.0);
    for w in ranges.windows(2) {
        if w[0].1 > w[1].0 {
            return Err(format!("sections `{}` and `{}` overlap in the image", w[0].2, w[1].2));
        }
    }
    Ok(flatten(image, fill))
}

/// The single-image ROM output (`p2bin` + `fixheader` replacement):
/// validate each section against the map, place bytes at LMA, gap-fill with the
/// map default, append NOTHING (the `convsym` no-op), then apply the header
/// checksum as the final pass. The ROM ends at the last section byte — no
/// power-of-two padding.
pub fn emit_rom(image: &LinkedImage, map: &MemoryMap) -> Result<Vec<u8>, String> {
    for s in &image.sections {
        map.validate_section(&s.name, s.lma, s.bytes.len() as u32)?;
    }
    let mut rom = flatten_checked(image, map.fill)?;
    // convsym no-op: append nothing.
    apply_header_checksum(&mut rom); // Task 6
    Ok(rom)
}

/// Sega header checksum: 16-bit big-endian additive word-sum over `[0x200, EOF)`,
/// written big-endian at `0x18E`. The genuinely-last byte-mutating pass. An odd
/// trailing byte is summed as the high half of a word (low half 0x00).
pub fn apply_header_checksum(rom: &mut [u8]) {
    if rom.len() < 0x200 {
        return;
    }
    let mut sum: u16 = 0;
    let mut i = 0x200;
    while i + 1 < rom.len() {
        sum = sum.wrapping_add(((rom[i] as u16) << 8) | rom[i + 1] as u16);
        i += 2;
    }
    if i < rom.len() {
        sum = sum.wrapping_add((rom[i] as u16) << 8);
    }
    rom[0x18E] = (sum >> 8) as u8;
    rom[0x18F] = (sum & 0xFF) as u8;
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
    fn pcrel_disp16_measured_from_extension_word() {
        // bra.w at op VMA 0x1000: [0x60,0x00, hi,lo]. Disp word at offset 2 (VMA 0x1002).
        // target 0x1080 → disp = 0x1080 - 0x1002 = 0x7E.
        let sec = Section {
            name: "c".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x1000,
            labels: vec![Label { name: "t".into(), offset: 0x80 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x60, 0x00, 0x00, 0x00],
                fixups: vec![Fixup { kind: FixupKind::PcRelDisp16, offset: 2, target: Expr::Sym("t".into()) }],
                span: span(),
            })],
        };
        let linked = link(&[sec], &SymbolTable::new()).unwrap();
        assert_eq!(linked.section("c").unwrap().bytes, vec![0x60, 0x00, 0x00, 0x7E]);
    }

    #[test]
    fn pcrel8_measured_from_op_plus_two() {
        // bra.s at op VMA 0x2000: [0x60, disp]. disp byte at offset 1 (VMA 0x2001).
        // target 0x2010 → disp = 0x2010 - (0x2001 + 1) = 0x0E.
        let sec = Section {
            name: "c".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x2000,
            labels: vec![Label { name: "t".into(), offset: 0x10 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x60, 0x00],
                fixups: vec![Fixup { kind: FixupKind::PcRel8, offset: 1, target: Expr::Sym("t".into()) }],
                span: span(),
            })],
        };
        assert_eq!(link(&[sec], &SymbolTable::new()).unwrap().section("c").unwrap().bytes, vec![0x60, 0x0E]);
    }

    #[test]
    fn pcrel8_out_of_range_diagnoses() {
        let sec = Section {
            name: "c".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x2000,
            labels: vec![Label { name: "far".into(), offset: 0x200 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x60, 0x00],
                fixups: vec![Fixup { kind: FixupKind::PcRel8, offset: 1, target: Expr::Sym("far".into()) }],
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
    fn fixup_offset_past_fragment_diagnoses() {
        // Fragment is 2 bytes, but the fixup is at offset 5. Target is resolvable,
        // so the offset overrun is the ONLY error.
        let mut stubs = SymbolTable::new();
        stubs.define("Ok", SymbolValue::Int(0x1234));
        let sec = Section {
            name: "s".to_string(),
            cpu: Cpu::Z80,
            vma_base: Some(0x8000),
            lma: 0x60000,
            labels: vec![],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x00, 0x00],
                fixups: vec![Fixup {
                    kind: FixupKind::BankPtr16Le,
                    offset: 5,
                    target: Expr::Sym("Ok".to_string()),
                }],
                span: span(),
            })],
        };
        let err = link(&[sec], &stubs).unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("exceeds fragment length")), "got: {:?}", err);
    }

    #[test]
    fn bankptr16le_at_fragment_boundary_diagnoses() {
        // Two Data fragments; a 2-byte BankPtr16Le at offset 1 of the FIRST
        // fragment ([0x00,0x00]) would write its high byte into the second
        // fragment ([0xCC,0xDD]). The width-aware check must catch this loudly,
        // and the second fragment's 0xCC must NOT be clobbered.
        let mut stubs = SymbolTable::new();
        stubs.define("Ptr", SymbolValue::Int(0xBEEF));
        let sec = Section {
            name: "s".to_string(),
            cpu: Cpu::Z80,
            vma_base: Some(0x8000),
            lma: 0x60000,
            labels: vec![],
            fragments: vec![
                Fragment::Data(DataFragment {
                    bytes: vec![0x00, 0x00],
                    fixups: vec![Fixup {
                        kind: FixupKind::BankPtr16Le,
                        offset: 1,
                        target: Expr::Sym("Ptr".to_string()),
                    }],
                    span: span(),
                }),
                Fragment::Data(DataFragment { bytes: vec![0xCC, 0xDD], fixups: vec![], span: span() }),
            ],
        };
        let err = link(&[sec], &stubs).unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("exceeds fragment length")), "got: {:?}", err);
    }

    #[test]
    fn abs32be_writes_big_endian_target_vma() {
        // A 4-byte data fragment; Abs32Be fixup at offset 0 targeting VMA 0x00123456.
        let mut stubs = SymbolTable::new();
        stubs.define("T", SymbolValue::Int(0x0012_3456));
        let sec = Section {
            name: "s".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x400,
            labels: vec![],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0, 0, 0, 0],
                fixups: vec![Fixup { kind: FixupKind::Abs32Be, offset: 0, target: Expr::Sym("T".into()) }],
                span: span(),
            })],
        };
        let linked = link(&[sec], &stubs).unwrap();
        assert_eq!(linked.section("s").unwrap().bytes, vec![0x00, 0x12, 0x34, 0x56]);
    }

    #[test]
    fn abs16be_writes_big_endian_and_rejects_overflow() {
        let mut stubs = SymbolTable::new();
        stubs.define("Ok", SymbolValue::Int(0x1234));
        stubs.define("Big", SymbolValue::Int(0x1_0000)); // does not fit abs.w sign-extension
        let ok = Section {
            name: "ok".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x400, labels: vec![],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0, 0],
                fixups: vec![Fixup { kind: FixupKind::Abs16Be, offset: 0, target: Expr::Sym("Ok".into()) }],
                span: span(),
            })],
        };
        assert_eq!(link(&[ok], &stubs).unwrap().section("ok").unwrap().bytes, vec![0x12, 0x34]);

        let bad = Section {
            name: "bad".to_string(), cpu: Cpu::M68000, vma_base: None, lma: 0x400, labels: vec![],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0, 0],
                fixups: vec![Fixup { kind: FixupKind::Abs16Be, offset: 0, target: Expr::Sym("Big".into()) }],
                span: span(),
            })],
        };
        let err = link(&[bad], &stubs).unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("abs.w")), "got: {:?}", err);
    }

    #[test]
    fn unresolved_names_the_symbol() {
        let sec = region_a(); // references SfxBlobWinTab, undefined here.
        let err = link(&[sec], &SymbolTable::new()).unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("SfxBlobWinTab")), "got: {:?}", err);
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

    #[test]
    fn flatten_checked_errors_on_overlap() {
        // Two sections: lma 0 len 4 ([0,4)) and lma 2 len 4 ([2,6)) overlap.
        let img = LinkedImage {
            sections: vec![
                LinkedSection { name: "a".to_string(), lma: 0, bytes: vec![0x11, 0x22, 0x33, 0x44] },
                LinkedSection { name: "b".to_string(), lma: 2, bytes: vec![0x55, 0x66, 0x77, 0x88] },
            ],
        };
        let err = flatten_checked(&img, 0x00).unwrap_err();
        assert!(err.contains("overlap"), "got: {err}");
    }

    #[test]
    fn flatten_checked_ok_when_disjoint() {
        // lma 0 len 2 ([0,2)) and lma 2 len 2 ([2,4)) are adjacent, not overlapping.
        let img = LinkedImage {
            sections: vec![
                LinkedSection { name: "a".to_string(), lma: 0, bytes: vec![0xAA, 0xBB] },
                LinkedSection { name: "b".to_string(), lma: 2, bytes: vec![0xCC, 0xDD] },
            ],
        };
        assert_eq!(flatten_checked(&img, 0x00).unwrap(), vec![0xAA, 0xBB, 0xCC, 0xDD]);
    }

    #[test]
    fn emit_rom_places_sections_and_validates_regions() {
        use sigil_ir::map::{MemoryMap, Region, RegionKind};
        let map = MemoryMap::new(
            vec![Region { name: "rom".into(), lma_base: 0, size: 0x1_0000, kind: RegionKind::Rom, vma_base: None }],
            0x00,
        );
        let img = LinkedImage {
            sections: vec![
                LinkedSection { name: "a".into(), lma: 2, bytes: vec![0xAA, 0xBB] },
                LinkedSection { name: "b".into(), lma: 6, bytes: vec![0xCC] },
            ],
        };
        // head 0,1 filled; bytes at 2..4; gap at 4,5; byte at 6. Terminus = 7 (no padding).
        assert_eq!(emit_rom(&img, &map).unwrap(), vec![0x00, 0x00, 0xAA, 0xBB, 0x00, 0x00, 0xCC]);
    }

    #[test]
    fn emit_rom_rejects_section_outside_region() {
        use sigil_ir::map::{MemoryMap, Region, RegionKind};
        let map = MemoryMap::new(
            vec![Region { name: "rom".into(), lma_base: 0, size: 4, kind: RegionKind::Rom, vma_base: None }],
            0x00,
        );
        let img = LinkedImage { sections: vec![LinkedSection { name: "a".into(), lma: 8, bytes: vec![1] }] };
        assert!(emit_rom(&img, &map).is_err());
    }

    #[test]
    fn header_checksum_is_be_wordsum_over_200_to_eof_at_18e() {
        // Build a >0x200-byte ROM; put known words after 0x200; assert the
        // checksum word at 0x18E equals the BE word-sum over [0x200, EOF).
        let mut rom = vec![0u8; 0x210];
        rom[0x200] = 0x12;
        rom[0x201] = 0x34; // word 0x1234
        rom[0x202] = 0x00;
        rom[0x203] = 0x01; // word 0x0001
        // remaining 0x204..0x210 are zero words → sum = 0x1235.
        apply_header_checksum(&mut rom);
        assert_eq!(rom[0x18E], 0x12);
        assert_eq!(rom[0x18F], 0x35);
    }

    #[test]
    fn header_checksum_handles_odd_trailing_byte() {
        // Odd length: last lone byte forms a word with a 0x00 low half (BE hi-byte).
        let mut rom = vec![0u8; 0x203];
        rom[0x200] = 0x00;
        rom[0x201] = 0x10; // word 0x0010
        rom[0x202] = 0x05; // lone byte → word 0x0500
        apply_header_checksum(&mut rom);
        assert_eq!(((rom[0x18E] as u16) << 8) | rom[0x18F] as u16, 0x0510);
    }
}
