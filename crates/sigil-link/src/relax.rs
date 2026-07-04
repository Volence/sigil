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

/// Current byte length of a fragment; `JmpJsrSym` uses the given width.
///
/// `Org` returns 0: it is a cursor *reposition*, not a run of bytes, so it has
/// no length in the monotonic-prefix-sum sense `shift_breakpoints` relies on.
/// This is only sound because `resolve_layout` refuses (see the guard below)
/// any section that mixes `Org` with a real-growth `JmpJsrSym`: with zero
/// `JmpJsrSym` fragments in a section, every width in `widths[..]` stays
/// `AbsWidth::W`, so `frag_len(frag, w)` is independent of `w` for every
/// fragment (Org included) and `shift_offset` reduces to the identity
/// function regardless of what value `Org` contributes here.
fn frag_len(frag: &Fragment, w: AbsWidth) -> u32 {
    match frag {
        Fragment::Data(d) => d.bytes.len() as u32,
        Fragment::Fill { count, .. } => *count,
        Fragment::Reserve { count, .. } => *count,
        Fragment::Org { .. } => 0,
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
        Fragment::Fill { span, .. }
        | Fragment::Reserve { span, .. }
        | Fragment::Org { span, .. }
        | Fragment::JmpJsrSym { span, .. } => *span,
    }
}

/// Resolve every `JmpJsrSym`'s width via a bounded, grow-only fixpoint, then lower
/// each to a concrete `Data` fragment AND shift every label to its final offset,
/// so the returned sections contain only Data/Fill/Reserve and `link()` runs on
/// them unchanged.
///
/// # Grow-only vs asl's bidirectional relaxation at the 0xFF8000 wrap
///
/// `asl_width_rule` is *non-monotonic* in the target address: it selects abs.w on
/// `[0, 0x7FFF]`, abs.l on `[0x8000, 0xFF_7FFF]`, then abs.w *again* on
/// `[0xFF_8000, 0xFF_FFFF]` (16-bit sign extension). This fixpoint only ever flips
/// a width W→L, never L→W. So a `jmp`/`jsr` whose target crosses the
/// `0xFF_7FFF → 0xFF_8000` boundary because of *earlier* growth stays locked to
/// abs.l even though its final address would only need abs.w — i.e. we may emit an
/// abs.l that is 2 bytes larger than asl's abs.w in that exact range.
///
/// This is deliberate: real asl 1.42's *bidirectional* relaxer is itself
/// non-terminating (it oscillates) on that self-referential construction, so there
/// is no stable asl output to be byte-exact against there. Byte-exactness is
/// therefore not claimed in the `0xFF_8000` sign-extension wrap. Reaching it
/// requires a bare `jmp`/`jsr` to a `$FF8000+` (RAM) target that is *also*
/// self-referentially width-affected — which does not occur in Aeon, where
/// `jmp`/`jsr` target ROM code labels. Grow-only is what buys guaranteed
/// termination in exchange.
pub fn resolve_layout(
    sections: &[Section],
    stubs: &SymbolTable,
    dash_a: bool,
) -> Result<Vec<Section>, Vec<Diagnostic>> {
    // Guard: a section mixing `Org` (the back-patch/absolute-org marker) with a
    // bare `jmp`/`jsr` (`JmpJsrSym`) is architecturally unverified — the
    // `shift_breakpoints`/`shift_offset` label-shift math assumes every
    // fragment's length sums to a monotonic prefix (see `frag_len`'s doc
    // comment), which `Org`'s cursor reposition breaks the instant a REAL width
    // grows in the same section. Rather than silently compute a wrong offset,
    // fail loudly here; today's real Aeon sections either mix pure back-patched
    // `dc.b`/`dc.w`/`dc.l` data with no `jmp`/`jsr` (parallax sections, safe) or
    // `jmp`/`jsr`-bearing code with no `Org` (engine code, safe) — see M1.C T6b.
    for sec in sections {
        let has_org = sec.fragments.iter().any(|f| matches!(f, Fragment::Org { .. }));
        if !has_org {
            continue;
        }
        // `has_org` holds from here, so an Org fragment provably exists — the
        // find below cannot miss (documented via `expect`, not a fabricated span).
        let org_span = sec
            .fragments
            .iter()
            .find(|f| matches!(f, Fragment::Org { .. }))
            .map(frag_span)
            .expect("has_org implies an Org fragment");
        let has_jmpjsr = sec.fragments.iter().any(|f| matches!(f, Fragment::JmpJsrSym { .. }));
        if has_jmpjsr {
            return Err(vec![Diagnostic {
                level: Level::Error,
                message: format!(
                    "section `{}` mixes an `org` back-patch with a bare jmp/jsr — unsupported (resolve_layout's width-shift math is not Org-aware)",
                    sec.name
                ),
                primary: org_span,
            }]);
        }
        // A `Reserve` in the same section is the analogous hazard: `IrBuilder`
        // counts `Reserve` toward the cursor/extent the front-end resolves an
        // `org` target against (VMA space), but `Section::image_bytes` and
        // `link()`'s fixup walk treat `Reserve` as zero image bytes and apply
        // `Org.target` as an IMAGE-byte offset — so a `Reserve` before an `org`
        // back-patch diverges the resolved VMA offset from the physical image
        // offset and the patch lands on the wrong byte. Latent today (parallax
        // sections are pure `dc.b`, no `ds`), but fail loudly rather than
        // mislink silently, mirroring the jmp/jsr guard above.
        if sec.fragments.iter().any(|f| matches!(f, Fragment::Reserve { .. })) {
            return Err(vec![Diagnostic {
                level: Level::Error,
                message: format!(
                    "section `{}` mixes an `org` back-patch with a `ds`/reserve — unsupported (reserve advances VMA but not image bytes, so the org target's image offset diverges)",
                    sec.name
                ),
                primary: org_span,
            }]);
        }
    }

    // Per-section, per-fragment width; JmpJsrSym entries start at abs.w (minimum).
    let mut widths: Vec<Vec<AbsWidth>> =
        sections.iter().map(|s| vec![AbsWidth::W; s.fragments.len()]).collect();

    // Provably-sufficient pass cap: each pass that reports `grew` flips at least
    // one JmpJsrSym W→L, and each fragment flips at most once, so at most
    // `total_jmpjsr` passes can grow — pass `total_jmpjsr + 1` is guaranteed to
    // observe no growth and converge. The non-convergence Err below is then an
    // unreachable-in-practice backstop.
    let total_jmpjsr: usize = sections
        .iter()
        .flat_map(|s| s.fragments.iter())
        .filter(|f| matches!(f, Fragment::JmpJsrSym { .. }))
        .count();
    let cap = total_jmpjsr + 1;

    // Span of a fragment that grew on the most recent pass, for the backstop diag.
    let mut last_grown_span: Option<Span> = None;

    for _ in 0..cap {
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
                    // GLOBAL scope only (scope None): a bare `jmp .local`/`jsr .local`
                    // to a dotted local would not resolve here. The front-end
                    // (sub-project C) must qualify such targets to fully-dotted names
                    // before emitting `JmpJsrSym`.
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
                        last_grown_span = Some(*span);
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
        message: format!("jmp/jsr width selection did not converge within {cap} passes"),
        // Point at a fragment that grew on the final pass (the likely culprit);
        // fall back to the first fragment's span if nothing grew.
        primary: last_grown_span
            .or_else(|| sections.iter().flat_map(|s| s.fragments.iter()).map(frag_span).next())
            .unwrap_or(Span { source: sigil_span::SourceId(0), start: 0, end: 0 }),
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_ir::{Cpu, DataFragment, Expr, Fixup, FixupKind, Fragment, Label, Section, SymbolTable, SymbolValue};
    use sigil_span::{SourceId, Span};

    fn sp() -> Span {
        Span { source: SourceId(0), start: 0, end: 0 }
    }

    #[test]
    fn resolve_boundary_wrap_terminates_safely() {
        // Documented safe-oversized case: as the jmp grows +2, Target's VMA crosses
        // 0xFF_7FFE → 0xFF_8000. asl_width_rule is non-monotonic there (L then W
        // again), but grow-only never shrinks back, so we emit abs.l (6 bytes) — 2
        // bytes larger than asl's abs.w would be. This is where asl 1.42's
        // bidirectional relaxer itself oscillates, so there is no stable target.
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0xFF_7FFA,
            labels: vec![Label { name: "Target".into(), offset: 4 }],
            fragments: vec![
                Fragment::JmpJsrSym { is_jsr: false, target: Expr::Sym("Target".into()), span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0x4E, 0x71], fixups: vec![], span: sp() }),
            ],
        };
        let out = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap();
        // The jmp lowered to abs.l (6-byte Data), and Target shifted 4 → 6.
        assert!(matches!(&out[0].fragments[0], Fragment::Data(d) if d.bytes.len() == 6));
        assert_eq!(out[0].labels.iter().find(|l| l.name == "Target").unwrap().offset, 6);
    }

    #[test]
    fn resolve_multi_jmp_cascade_converges() {
        // A genuine multi-pass cascade: `jmp A` and `jmp Hi` in one section. Pass 1
        // grows `jmp Hi` (high stub) → +2; that shift pushes label A's VMA from
        // 0x7FFF across 0x8000, so `jmp A` grows in pass 2. Proves shift_breakpoints
        // composes multiple growths and the fixpoint re-converges over passes.
        // origin = vma_base = 0x7FF0; baseline: jmp A [0,4), jmp Hi [4,8),
        // fill(7) [8,0x0F), nop @0x0F, A@0x0F → baseline A VMA = 0x7FFF.
        let mut stubs = SymbolTable::new();
        stubs.define("Hi", SymbolValue::Int(0x12_3456));
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: Some(0x7FF0),
            lma: 0,
            labels: vec![Label { name: "A".into(), offset: 0x0F }],
            fragments: vec![
                Fragment::JmpJsrSym { is_jsr: false, target: Expr::Sym("A".into()), span: sp() },
                Fragment::JmpJsrSym { is_jsr: false, target: Expr::Sym("Hi".into()), span: sp() },
                Fragment::Fill { value: 0, count: 7, span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0x4E, 0x71], fixups: vec![], span: sp() }),
            ],
        };
        let out = resolve_layout(&[sec], &stubs, true).unwrap();
        // Both jmps grew to abs.l; A shifted 0x0F → 0x13 (0x0F + 4).
        assert_eq!(out[0].labels.iter().find(|l| l.name == "A").unwrap().offset, 0x13);
        let linked = crate::link(&out, &stubs).unwrap();
        // frag0 jmp A abs.l → A VMA = 0x7FF0 + 0x13 = 0x8003; frag1 jmp Hi abs.l →
        // 0x123456; then 7 fill zeros; then nop.
        assert_eq!(
            linked.section("c").unwrap().bytes,
            vec![
                0x4E, 0xF9, 0x00, 0x00, 0x80, 0x03, // jmp A abs.l
                0x4E, 0xF9, 0x00, 0x12, 0x34, 0x56, // jmp Hi abs.l
                0, 0, 0, 0, 0, 0, 0, // 7 fill bytes
                0x4E, 0x71, // nop
            ]
        );
    }

    #[test]
    fn resolve_phased_section_shifts_correctly() {
        // Phased section (VMA≠LMA): vma_base 0xFF_0000, lma 0x000100. A growing
        // `jmp Hi` shifts label After (baseline 4 → 6); a following Abs32Be fixup on
        // After proves link() computes its VMA from vma_origin under phase.
        let mut stubs = SymbolTable::new();
        stubs.define("Hi", SymbolValue::Int(0x12_3456));
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: Some(0xFF_0000),
            lma: 0x000100,
            labels: vec![Label { name: "After".into(), offset: 4 }],
            fragments: vec![
                Fragment::JmpJsrSym { is_jsr: false, target: Expr::Sym("Hi".into()), span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0x4E, 0x71], fixups: vec![], span: sp() }),
                Fragment::Data(DataFragment {
                    bytes: vec![0, 0, 0, 0],
                    fixups: vec![Fixup { kind: FixupKind::Abs32Be, offset: 0, target: Expr::Sym("After".into()) }],
                    span: sp(),
                }),
            ],
        };
        let out = resolve_layout(&[sec], &stubs, true).unwrap();
        assert_eq!(out[0].labels.iter().find(|l| l.name == "After").unwrap().offset, 6);
        let linked = crate::link(&out, &stubs).unwrap();
        // jmp Hi abs.l; nop; then Abs32Be(After) = 0xFF_0000 + 6 = 0x00FF0006.
        assert_eq!(
            linked.section("c").unwrap().bytes,
            vec![0x4E, 0xF9, 0x00, 0x12, 0x34, 0x56, 0x4E, 0x71, 0x00, 0xFF, 0x00, 0x06]
        );
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

    #[test]
    fn resolve_layout_refuses_org_and_jmpjsr_in_the_same_section() {
        // The real Aeon collision this guard exists for: main.asm's object-bank
        // section (opened by `org $10000`) contains BOTH bare `jmp`/`jsr` calls
        // (player/object code) AND the parallax `parallax_section_end` back-patch
        // (`org pscStart / dc.b n / org pscEndPos`) later in the SAME still-open
        // section. `shift_breakpoints`'s label-shift math assumes a monotonic
        // fragment-length prefix sum, which a real-growth `JmpJsrSym` alongside
        // an `Org` reposition would violate — so `resolve_layout` must fail
        // loudly here instead of silently computing a wrong offset.
        let sec = Section {
            name: "objbank".into(),
            cpu: Cpu::M68000,
            vma_base: Some(0x10000),
            lma: 0x10000,
            labels: vec![],
            fragments: vec![
                Fragment::JmpJsrSym { is_jsr: true, target: Expr::Sym("Sub".into()), span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0], fixups: vec![], span: sp() }),
                Fragment::Org { target: 4, fill: 0x00, span: sp() },
                Fragment::Data(DataFragment { bytes: vec![1], fixups: vec![], span: sp() }),
            ],
        };
        let err = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap_err();
        assert!(
            err.iter().any(|d| d.message.contains("org") && d.message.contains("jmp/jsr")),
            "got: {:?}",
            err
        );
    }

    #[test]
    fn resolve_layout_refuses_org_and_reserve_in_the_same_section() {
        // The analogous hazard to Org+JmpJsrSym: a `Reserve` (from `ds`) counts
        // toward the front-end's cursor/extent (VMA space) when it resolves an
        // `org` target, but contributes no image bytes — so applying `Org.target`
        // as an image offset would land the back-patch on the wrong physical
        // byte. Guard must fail loudly, with a message distinct from the
        // jmp/jsr one, pointing at the first `Org`.
        let sec = Section {
            name: "data".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![
                Fragment::Reserve { count: 4, span: sp() },
                Fragment::Org { target: 0, fill: 0x00, span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0x63], fixups: vec![], span: sp() }),
            ],
        };
        let err = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap_err();
        assert!(
            err.iter().any(|d| d.message.contains("org") && d.message.contains("reserve")),
            "got: {:?}",
            err
        );
        // ...and it must NOT be misreported as the jmp/jsr hazard.
        assert!(
            !err.iter().any(|d| d.message.contains("jmp/jsr")),
            "Org+Reserve wrongly reported as the jmp/jsr hazard: {:?}",
            err
        );
    }

    #[test]
    fn resolve_layout_allows_org_alone_with_no_jmpjsr() {
        // The parallax-data-only case (real Aeon usage today): pure `dc.b`
        // back-patch, zero `jmp`/`jsr` in the section — must pass through
        // resolve_layout unperturbed (the identity-shift argument in
        // `frag_len`'s doc comment).
        let sec = Section {
            name: "data".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![Label { name: "After".into(), offset: 5 }],
            fragments: vec![
                Fragment::Data(DataFragment { bytes: vec![0, 1, 2, 3], fixups: vec![], span: sp() }),
                Fragment::Org { target: 0, fill: 0x00, span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0x63], fixups: vec![], span: sp() }),
                Fragment::Org { target: 4, fill: 0x00, span: sp() },
                Fragment::Data(DataFragment { bytes: vec![4], fixups: vec![], span: sp() }),
            ],
        };
        let out = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap();
        assert_eq!(out[0].labels.iter().find(|l| l.name == "After").unwrap().offset, 5);
        let linked = crate::link(&out, &SymbolTable::new()).unwrap();
        assert_eq!(linked.section("data").unwrap().bytes, vec![0x63, 1, 2, 3, 4]);
    }
}
