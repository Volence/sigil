//! Width selection + (next task) the bounded layout fixpoint (spec §5.4/§5.6).
//! The only length-variable fragment in Aeon is bare-symbol `jmp`/`jsr`.

/// The chosen absolute-addressing width for a `jmp`/`jsr`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbsWidth {
    /// `abs.w`: opcode word + 2-byte operand (4 bytes total).
    W,
    /// `abs.l`: opcode word + 4-byte operand (6 bytes total).
    L,
}

impl AbsWidth {
    /// Total instruction length in bytes for this width.
    pub fn inst_len(self) -> u32 {
        match self {
            AbsWidth::W => 4,
            AbsWidth::L => 6,
        }
    }
}

/// asl's `abs.w` vs `abs.l` selection for a `jmp`/`jsr` target address. Confirmed
/// byte-for-byte against asl 1.42 by a boundary sweep of `jmp $ADDR` (with AND
/// without `-A` — identical results, so `-A` is irrelevant to jmp/jsr width).
/// `abs.w` iff the 24-bit address sign-extends losslessly from 16 bits:
/// `[0, 0x7FFF] ∪ [0xFF_8000, 0xFF_FFFF]`. Examples: $7FFF→.w, $8000→.l,
/// $FF8000→.w (= -$8000 sign-extended), $FFFFFE→.w.
pub fn asl_width_rule(target: i64, _dash_a: bool) -> AbsWidth {
    let a = (target & 0xFF_FFFF) as u32;
    if a <= 0x7FFF || a >= 0xFF_8000 {
        AbsWidth::W
    } else {
        AbsWidth::L
    }
}

use sigil_ir::expr::Fold;
use sigil_ir::{DataFragment, Expr, Fixup, FixupKind, Fragment, Label, Section, SymbolTable, SymbolValue};
use sigil_span::{Diagnostic, Level, Span};

const MAX_PASSES: usize = 64;

/// Current byte length of a fragment; `JmpJsrSym` uses the given width.
fn frag_len(frag: &Fragment, w: AbsWidth) -> u32 {
    match frag {
        Fragment::Data(d) => d.bytes.len() as u32,
        Fragment::Fill { count, .. } => *count,
        Fragment::Reserve { count, .. } => *count,
        Fragment::JmpJsrSym { .. } => w.inst_len(),
    }
}

/// Breakpoints mapping an all-abs.w (baseline) offset to the growth delta at that
/// fragment boundary under the current widths. `widths[fi]` is the chosen width
/// of fragment `fi` (only meaningful for `JmpJsrSym`; ignored otherwise).
fn shift_breakpoints(sec: &Section, widths: &[AbsWidth]) -> Vec<(u32, i64)> {
    let mut cur: u32 = 0;
    let mut orig: u32 = 0;
    let mut bps = vec![(0u32, 0i64)];
    for (fi, frag) in sec.fragments.iter().enumerate() {
        cur += frag_len(frag, widths[fi]);
        orig += frag_len(frag, AbsWidth::W); // baseline: every JmpJsrSym at abs.w
        bps.push((orig, cur as i64 - orig as i64));
    }
    bps
}

/// Map an all-abs.w label offset to its current-layout offset.
fn shift_offset(bps: &[(u32, i64)], orig_off: u32) -> u32 {
    let mut d = 0i64;
    for &(bo, bd) in bps {
        if bo <= orig_off {
            d = bd;
        } else {
            break;
        }
    }
    (orig_off as i64 + d) as u32
}

/// jmp abs.w=4EF8/abs.l=4EF9, jsr abs.w=4EB8/abs.l=4EB9 (`.l` = `.w | 1`);
/// operand at offset 2, Abs16Be (.w) / Abs32Be (.l).
fn lower_jmp_jsr(is_jsr: bool, target: Expr, w: AbsWidth, span: Span) -> Fragment {
    let base: u16 = if is_jsr { 0x4EB8 } else { 0x4EF8 };
    match w {
        AbsWidth::W => Fragment::Data(DataFragment {
            bytes: vec![(base >> 8) as u8, (base & 0xFF) as u8, 0, 0],
            fixups: vec![Fixup { kind: FixupKind::Abs16Be, offset: 2, target }],
            span,
        }),
        AbsWidth::L => {
            let op = base | 0x0001;
            Fragment::Data(DataFragment {
                bytes: vec![(op >> 8) as u8, (op & 0xFF) as u8, 0, 0, 0, 0],
                fixups: vec![Fixup { kind: FixupKind::Abs32Be, offset: 2, target }],
                span,
            })
        }
    }
}

fn frag_span(f: &Fragment) -> Span {
    match f {
        Fragment::Data(d) => d.span,
        Fragment::Fill { span, .. } | Fragment::Reserve { span, .. } | Fragment::JmpJsrSym { span, .. } => *span,
    }
}

/// Resolve every `JmpJsrSym`'s width via a bounded, grow-only fixpoint, then lower
/// each to a concrete `Data` fragment AND shift every label to its final offset,
/// so the returned sections contain only Data/Fill/Reserve and `link()` runs on
/// them unchanged.
pub fn resolve_layout(
    sections: &[Section],
    stubs: &SymbolTable,
    dash_a: bool,
) -> Result<Vec<Section>, Vec<Diagnostic>> {
    // Per-section, per-fragment width; JmpJsrSym entries start at abs.w (minimum).
    let mut widths: Vec<Vec<AbsWidth>> =
        sections.iter().map(|s| vec![AbsWidth::W; s.fragments.len()]).collect();

    for _ in 0..MAX_PASSES {
        // (a) Build the symbol table with label VMAs shifted under current widths.
        let mut syms = stubs.clone();
        for (si, sec) in sections.iter().enumerate() {
            let origin = sec.vma_origin();
            let bps = shift_breakpoints(sec, &widths[si]);
            for label in &sec.labels {
                syms.define(&label.name, SymbolValue::Int((origin + shift_offset(&bps, label.offset)) as i64));
            }
        }

        // (b) Re-select each JmpJsrSym width from its resolved target (grow-only).
        let mut grew = false;
        for (si, sec) in sections.iter().enumerate() {
            for (fi, frag) in sec.fragments.iter().enumerate() {
                if let Fragment::JmpJsrSym { target, span, .. } = frag {
                    let v = match target.fold(&|n| syms.resolve(n, None)) {
                        Fold::Value(v) => v,
                        Fold::Poison => {
                            return Err(vec![Diagnostic {
                                level: Level::Error,
                                message: format!("unresolved jmp/jsr target in section {}", sec.name),
                                primary: *span,
                            }]);
                        }
                    };
                    if asl_width_rule(v, dash_a) == AbsWidth::L && widths[si][fi] == AbsWidth::W {
                        widths[si][fi] = AbsWidth::L;
                        grew = true;
                    }
                }
            }
        }

        if !grew {
            // (c) Converged: lower fragments + shift labels into final sections.
            let out = sections
                .iter()
                .enumerate()
                .map(|(si, sec)| {
                    let bps = shift_breakpoints(sec, &widths[si]);
                    let labels = sec
                        .labels
                        .iter()
                        .map(|l| Label { name: l.name.clone(), offset: shift_offset(&bps, l.offset) })
                        .collect();
                    let fragments = sec
                        .fragments
                        .iter()
                        .enumerate()
                        .map(|(fi, frag)| match frag {
                            Fragment::JmpJsrSym { is_jsr, target, span } => {
                                lower_jmp_jsr(*is_jsr, target.clone(), widths[si][fi], *span)
                            }
                            other => other.clone(),
                        })
                        .collect();
                    Section {
                        name: sec.name.clone(),
                        cpu: sec.cpu,
                        vma_base: sec.vma_base,
                        lma: sec.lma,
                        labels,
                        fragments,
                    }
                })
                .collect();
            return Ok(out);
        }
    }

    Err(vec![Diagnostic {
        level: Level::Error,
        message: format!("jmp/jsr width selection did not converge within {MAX_PASSES} passes"),
        primary: sections
            .iter()
            .flat_map(|s| s.fragments.iter())
            .map(frag_span)
            .next()
            .unwrap_or(Span { source: sigil_span::SourceId(0), start: 0, end: 0 }),
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_ir::{Cpu, DataFragment, Expr, Fragment, Label, Section, SymbolTable, SymbolValue};
    use sigil_span::{SourceId, Span};

    fn sp() -> Span {
        Span { source: SourceId(0), start: 0, end: 0 }
    }

    #[test]
    fn resolve_lowers_jmp_to_absw_for_low_target() {
        // Section at LMA 0: [jmp Low] then Low: nop (0x4E71). Low VMA = 4 (abs.w).
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![Label { name: "Low".into(), offset: 4 }],
            fragments: vec![
                Fragment::JmpJsrSym { is_jsr: false, target: Expr::Sym("Low".into()), span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0x4E, 0x71], fixups: vec![], span: sp() }),
            ],
        };
        let out = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap();
        let linked = crate::link(&out, &SymbolTable::new()).unwrap();
        // jmp abs.w = 4EF8 + word(0x0004), then nop.
        assert_eq!(linked.section("c").unwrap().bytes, vec![0x4E, 0xF8, 0x00, 0x04, 0x4E, 0x71]);
    }

    #[test]
    fn resolve_lowers_jmp_to_absl_for_high_target() {
        let mut stubs = SymbolTable::new();
        stubs.define("Hi", SymbolValue::Int(0x12_3456));
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![Fragment::JmpJsrSym { is_jsr: false, target: Expr::Sym("Hi".into()), span: sp() }],
        };
        let out = resolve_layout(&[sec], &stubs, true).unwrap();
        let linked = crate::link(&out, &stubs).unwrap();
        // jmp abs.l = 4EF9 + long(0x00123456).
        assert_eq!(linked.section("c").unwrap().bytes, vec![0x4E, 0xF9, 0x00, 0x12, 0x34, 0x56]);
    }

    #[test]
    fn resolve_lowers_jsr_uses_4eb8_4eb9() {
        // jsr to a low target → 4EB8 (abs.w).
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![Label { name: "T".into(), offset: 4 }],
            fragments: vec![
                Fragment::JmpJsrSym { is_jsr: true, target: Expr::Sym("T".into()), span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0x4E, 0x75], fixups: vec![], span: sp() }),
            ],
        };
        let out = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap();
        let linked = crate::link(&out, &SymbolTable::new()).unwrap();
        assert_eq!(&linked.section("c").unwrap().bytes[..4], &[0x4E, 0xB8, 0x00, 0x04]);
    }

    #[test]
    fn resolve_grows_jmp_to_absl_and_shifts_following_label() {
        // jmp Hi (high → abs.l, 6 bytes) then After: nop. The After label was
        // authored at all-abs.w offset 4; after growth it MUST shift to 6, and
        // link() must resolve it consistently (regression test for the label-shift bug).
        let mut stubs = SymbolTable::new();
        stubs.define("Hi", SymbolValue::Int(0x12_3456));
        let code = Section {
            name: "code".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![Label { name: "After".into(), offset: 4 }],
            fragments: vec![
                Fragment::JmpJsrSym { is_jsr: false, target: Expr::Sym("Hi".into()), span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0x4E, 0x71], fixups: vec![], span: sp() }),
            ],
        };
        let out = resolve_layout(&[code], &stubs, true).unwrap();
        assert_eq!(out[0].labels.iter().find(|l| l.name == "After").unwrap().offset, 6);
        let linked = crate::link(&out, &stubs).unwrap();
        assert_eq!(linked.section("code").unwrap().bytes, vec![0x4E, 0xF9, 0x00, 0x12, 0x34, 0x56, 0x4E, 0x71]);
    }

    #[test]
    fn resolve_reports_unresolved_target() {
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![Fragment::JmpJsrSym { is_jsr: false, target: Expr::Sym("Nope".into()), span: sp() }],
        };
        assert!(resolve_layout(&[sec], &SymbolTable::new(), true).is_err());
    }

    #[test]
    fn width_rule_matches_asl_boundary_sweep() {
        assert_eq!(asl_width_rule(0x0000, true), AbsWidth::W);
        assert_eq!(asl_width_rule(0x7FFF, true), AbsWidth::W);
        assert_eq!(asl_width_rule(0x8000, true), AbsWidth::L);
        assert_eq!(asl_width_rule(0xFFFF, true), AbsWidth::L);
        assert_eq!(asl_width_rule(0x1_0000, true), AbsWidth::L);
        assert_eq!(asl_width_rule(0xFF_8000, true), AbsWidth::W);
        assert_eq!(asl_width_rule(0xFF_FFFF, true), AbsWidth::W);
    }

    #[test]
    fn dash_a_does_not_change_width() {
        // -A is irrelevant to jmp/jsr width (confirmed by the asl sweep).
        for addr in [0x0000i64, 0x7FFF, 0x8000, 0xFF_8000, 0xFF_FFFF] {
            assert_eq!(asl_width_rule(addr, true), asl_width_rule(addr, false));
        }
    }

    #[test]
    fn inst_len_is_4_for_w_and_6_for_l() {
        assert_eq!(AbsWidth::W.inst_len(), 4);
        assert_eq!(AbsWidth::L.inst_len(), 6);
    }
}
