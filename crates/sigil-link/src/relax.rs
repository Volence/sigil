//! Width selection + the bounded layout fixpoint (spec §5.4/§5.6).
//! The length-variable fragments in Aeon are bare-symbol `jmp`/`jsr`
//! (`JmpJsrSym`), a straight-line instruction with a width-deferred symbolic
//! absolute operand (`RelaxAbsSym`), and the generic `RelaxLadder` (an ordered
//! ladder of complete candidate encodings — unsized branches / `jbra`). Every
//! one defers its final encoding to this bounded, grow-only fixpoint.
//!
//! # The unified per-fragment state: a RUNG INDEX
//!
//! Rather than three parallel state kinds, every relaxable fragment carries a
//! single `usize` **rung index** into its own ordered list of encodings:
//!
//! - `JmpJsrSym`/`RelaxAbsSym` have exactly two rungs: `0 → abs.w`, `1 → abs.l`
//!   (`rung_width` maps the index back to an `AbsWidth`, so their behavior is
//!   byte-identical to the pre-ladder two-width fixpoint).
//! - `RelaxLadder` has `candidates.len()` rungs, `0` = smallest encoding.
//!
//! The fixpoint only ever GROWS a rung index (never shrinks it), which is what
//! bounds the number of passes (see the pass-cap termination argument below).

use sigil_ir::expr::Fold;
pub use sigil_ir::{
    asl_width_rule, AbsWidth, DataFragment, Expr, Fixup, FixupKind, Fragment, Label, RelaxCandidate,
    Section, SymbolTable, SymbolValue,
};
use sigil_span::{Diagnostic, Level, Span};

/// Map a `JmpJsrSym`/`RelaxAbsSym` rung index back to its `AbsWidth` (rung 0 →
/// `abs.w`, rung 1 → `abs.l`). These two fragments have exactly two rungs, so
/// any index ≥ 1 is `abs.l` — the fixpoint never produces an out-of-range index
/// for them (it grows at most 0 → 1).
fn rung_width(rung: usize) -> AbsWidth {
    if rung == 0 {
        AbsWidth::W
    } else {
        AbsWidth::L
    }
}

/// The number of distinct rungs a fragment can occupy — used both for the pass
/// cap and to clamp a grow. `JmpJsrSym`/`RelaxAbsSym` have 2 (abs.w/abs.l);
/// `RelaxLadder` has one per candidate; a fixed-length fragment has 1.
fn rung_count(frag: &Fragment) -> usize {
    match frag {
        Fragment::JmpJsrSym { .. } | Fragment::RelaxAbsSym { .. } => 2,
        Fragment::RelaxLadder { candidates, .. } => candidates.len().max(1),
        _ => 1,
    }
}

/// Current byte length of a fragment at the given RUNG index.
///
/// `Org` returns 0: it is a cursor *reposition*, not a run of bytes, so it has
/// no length in the monotonic-prefix-sum sense `shift_breakpoints` relies on.
/// This is only sound because `resolve_layout` refuses (see the guard below)
/// any section that mixes `Org` with a real-growth width-variable fragment —
/// and the guard refuses `JmpJsrSym`, `RelaxAbsSym`, AND `RelaxLadder` (the
/// `has_relaxable` predicate covers all three). So in any section that also
/// contains an `Org`, there are zero width-variable fragments, every rung in
/// `rungs[..]` stays `0`, `frag_len(frag, 0)` is independent of the rung for
/// every fragment (Org included), and `shift_offset` reduces to the identity
/// function regardless of what value `Org` contributes here.
fn frag_len(frag: &Fragment, rung: usize) -> u32 {
    match frag {
        Fragment::Data(d) => d.bytes.len() as u32,
        Fragment::Fill { count, .. } => *count,
        Fragment::Reserve { count, .. } => *count,
        Fragment::Org { .. } => 0,
        Fragment::JmpJsrSym { .. } => rung_width(rung).inst_len(),
        // The chosen candidate's real byte length (abs.w = short, abs.l = long),
        // so layout accounts for the exact instruction width like JmpJsrSym does.
        Fragment::RelaxAbsSym { short, long, .. } => match rung_width(rung) {
            AbsWidth::W => short.bytes.len() as u32,
            AbsWidth::L => long.bytes.len() as u32,
        },
        // The chosen candidate's real byte length. `candidates` is non-decreasing
        // in `bytes.len()`, so growing the rung only ever grows (or holds) length.
        Fragment::RelaxLadder { candidates, .. } => {
            candidates.get(rung).map(|c| c.bytes.len() as u32).unwrap_or(0)
        }
    }
}

/// Breakpoints mapping an all-rung-0 (baseline) offset to the growth delta at
/// that fragment boundary under the current rungs. `rungs[fi]` is the chosen
/// rung of fragment `fi` (meaningful for the relaxables; ignored otherwise —
/// fixed fragments have identical length at every rung).
fn shift_breakpoints(sec: &Section, rungs: &[usize]) -> Vec<(u32, i64)> {
    let mut cur: u32 = 0;
    let mut orig: u32 = 0;
    let mut bps = vec![(0u32, 0i64)];
    for (fi, frag) in sec.fragments.iter().enumerate() {
        cur += frag_len(frag, rungs[fi]);
        orig += frag_len(frag, 0); // baseline: every width-variable fragment at rung 0
        bps.push((orig, cur as i64 - orig as i64));
    }
    bps
}

/// Map an all-rung-0 label offset to its current-layout offset.
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

/// The current-layout START VMA of fragment `fi` under the given rungs, i.e. the
/// VMA of the first byte the fragment emits. Its baseline offset is the prefix
/// sum of the rung-0 lengths of the preceding fragments (exactly what
/// `shift_breakpoints`/`shift_offset` are built on), shifted into the current
/// layout, plus the section origin. This is the reach-test site VMA a ladder
/// candidate measures its displacement from.
// TODO(perf): O(fi) prefix walk per ladder per pass; once ladders get dense, thread a
// running accumulator through the selection loop + convergence sweep instead.
fn frag_start_vma(sec: &Section, bps: &[(u32, i64)], origin: u32, fi: usize) -> u32 {
    let mut baseline_off: u32 = 0;
    for prev in &sec.fragments[..fi] {
        baseline_off += frag_len(prev, 0);
    }
    origin + shift_offset(bps, baseline_off)
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
        | Fragment::JmpJsrSym { span, .. }
        | Fragment::RelaxAbsSym { span, .. }
        | Fragment::RelaxLadder { span, .. } => *span,
    }
}

/// Does a `RelaxLadder` candidate REACH the resolved `target` from a fragment
/// whose current start VMA is `frag_start`? **Derived entirely from the
/// candidate's `FixupKind`** — this is what keeps the ladder CPU-agnostic: no
/// branch/jmp semantics are baked into a tag; a future Z80 `jr → jp` ladder just
/// carries `Z80JrRel8` candidates and this function grows a new arm for it.
///
/// The site VMA a displacement is measured from is `frag_start + fixup.offset`
/// — each candidate may place its disp at a different offset (a 2-byte branch's
/// disp byte is at offset 1; a 4-byte word form's disp word is at offset 2), so
/// the reach test uses THIS candidate's own `fixup.offset`.
///
/// Returns `Err` for any `FixupKind` that must never appear inside a ladder
/// (a front-end construction-contract violation) — never a silent `false`.
fn rung_reaches(
    cand: &RelaxCandidate,
    frag_start: u32,
    target: i64,
    dash_a: bool,
    span: Span,
    section: &str,
) -> Result<bool, Diagnostic> {
    let site_vma = frag_start as i64 + cand.fixup.offset as i64;
    match cand.fixup.kind {
        // bra.s/Bcc.s: PC ref = op+2, disp byte at op+1 = site_vma. disp must fit
        // i8 AND be non-zero — the 68000 treats a 0x00 byte displacement as the
        // word-form escape, so a byte branch to op+2 is unencodable as `.s`.
        FixupKind::PcRel8 => {
            let disp = target - (site_vma + 1);
            Ok((-128..=127).contains(&disp) && disp != 0)
        }
        // bra.w/Bcc.w: disp measured from the extension word's own VMA = site_vma.
        FixupKind::PcRelDisp16 => {
            let disp = target - site_vma;
            Ok((-0x8000..=0x7FFF).contains(&disp))
        }
        // jmp/jsr abs.w reaches iff the absolute target sign-extends from 16 bits.
        FixupKind::Abs16Be => Ok(asl_width_rule(target, dash_a) == AbsWidth::W),
        // jmp/jsr abs.l reaches any 32-bit address.
        FixupKind::Abs32Be => Ok(true),
        other => Err(Diagnostic {
            level: Level::Error,
            message: format!(
                "internal: RelaxLadder candidate carries unsupported fixup kind {other:?} in section {section} — a ladder rung's reach must be one of PcRel8/PcRelDisp16/Abs16Be/Abs32Be (construction-contract violation)"
            ),
            primary: span,
        }),
    }
}

/// Resolve every relaxable fragment's encoding via a bounded, grow-only fixpoint,
/// then lower each to a concrete `Data` fragment AND shift every label to its
/// final offset, so the returned sections contain only Data/Fill/Reserve and
/// `link()` runs on them unchanged.
///
/// # Grow-only vs asl's bidirectional relaxation at the 0xFF8000 wrap
///
/// `asl_width_rule` is *non-monotonic* in the target address: it selects abs.w on
/// `[0, 0x7FFF]`, abs.l on `[0x8000, 0xFF_7FFF]`, then abs.w *again* on
/// `[0xFF_8000, 0xFF_FFFF]` (16-bit sign extension). This fixpoint only ever
/// grows a rung index, never shrinks it. So a `jmp`/`jsr` whose target crosses
/// the `0xFF_7FFF → 0xFF_8000` boundary because of *earlier* growth stays locked
/// to abs.l even though its final address would only need abs.w — i.e. we may
/// emit an abs.l that is 2 bytes larger than asl's abs.w in that exact range.
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
    // Defensive: an empty or mis-ordered RelaxLadder is a front-end
    // construction-contract violation. `debug_assert!` catches both in tests; in
    // release we refuse loudly rather than silently mis-lower a zero-rung ladder
    // or break the grow-only length argument (a decreasing pair would let a rung
    // grow while the fragment SHRINKS, corrupting the prefix-sum layout math).
    let mut construction_errs: Vec<Diagnostic> = Vec::new();
    for sec in sections {
        for frag in &sec.fragments {
            if let Fragment::RelaxLadder { candidates, span, .. } = frag {
                debug_assert!(!candidates.is_empty(), "RelaxLadder must have ≥1 candidate");
                debug_assert!(
                    candidates.windows(2).all(|w| w[0].bytes.len() <= w[1].bytes.len()),
                    "RelaxLadder candidates must be non-decreasing in bytes.len()"
                );
                if candidates.is_empty() {
                    construction_errs.push(Diagnostic {
                        level: Level::Error,
                        message: format!(
                            "internal: empty RelaxLadder (zero candidates) in section {} — the front-end must emit at least one candidate encoding",
                            sec.name
                        ),
                        primary: *span,
                    });
                } else if let Some(w) =
                    candidates.windows(2).find(|w| w[0].bytes.len() > w[1].bytes.len())
                {
                    construction_errs.push(Diagnostic {
                        level: Level::Error,
                        message: format!(
                            "internal: mis-ordered RelaxLadder in section {} — candidate lengths must be non-decreasing (found {} bytes followed by {}); the front-end must order candidates smallest → largest",
                            sec.name,
                            w[0].bytes.len(),
                            w[1].bytes.len()
                        ),
                        primary: *span,
                    });
                }
            }
        }
    }
    if !construction_errs.is_empty() {
        return Err(construction_errs);
    }

    // Guard: a section mixing `Org` (the back-patch/absolute-org marker) with a
    // relaxable fragment is architecturally unverified — the
    // `shift_breakpoints`/`shift_offset` label-shift math assumes every
    // fragment's length sums to a monotonic prefix (see `frag_len`'s doc
    // comment), which `Org`'s cursor reposition breaks the instant a REAL width
    // grows in the same section. Rather than silently compute a wrong offset,
    // fail loudly here; today's real Aeon sections either mix pure back-patched
    // `dc.b`/`dc.w`/`dc.l` data with no relaxable (parallax sections, safe) or
    // relaxable-bearing code with no `Org` (engine code, safe) — see M1.C T6b.
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
        // A length-variable fragment (`JmpJsrSym`, `RelaxAbsSym`, OR
        // `RelaxLadder`) alongside an `Org` is the same hazard: the
        // `shift_breakpoints` prefix-sum math is not Org-aware once a real width
        // grows in the section.
        let has_relaxable = sec.fragments.iter().any(|f| {
            matches!(
                f,
                Fragment::JmpJsrSym { .. } | Fragment::RelaxAbsSym { .. } | Fragment::RelaxLadder { .. }
            )
        });
        if has_relaxable {
            return Err(vec![Diagnostic {
                level: Level::Error,
                message: format!(
                    "section `{}` mixes an `org` back-patch with a relaxable instruction (jmp/jsr, a width-deferred operand, or a relaxation ladder) — unsupported (resolve_layout's width-shift math is not Org-aware)",
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

    // Per-section, per-fragment RUNG index; all entries start at rung 0 (minimum
    // encoding). For `JmpJsrSym`/`RelaxAbsSym` rung 0 = abs.w.
    let mut rungs: Vec<Vec<usize>> =
        sections.iter().map(|s| vec![0usize; s.fragments.len()]).collect();

    // Provably-sufficient pass cap: each pass that reports `grew` advances at
    // least one relaxable fragment's rung by ≥1 (a length change), and each
    // fragment can advance at most `rung_count − 1` times (grow-only). So at most
    // `Σ(rung_count − 1)` passes can grow — one more pass observes no growth and
    // converges. (JmpJsrSym/RelaxAbsSym contribute 1 each = the old total-flips
    // bound; a 4-rung ladder contributes 3.) The non-convergence Err below is an
    // unreachable-in-practice backstop.
    let total_flips: usize = sections
        .iter()
        .flat_map(|s| s.fragments.iter())
        .map(|f| rung_count(f) - 1)
        .sum();
    let cap = total_flips + 1;

    // Span of a fragment that grew on the most recent pass, for the backstop diag.
    let mut last_grown_span: Option<Span> = None;

    for _ in 0..cap {
        // (a) Build the symbol table with label VMAs shifted under current rungs.
        let mut syms = stubs.clone();
        for (si, sec) in sections.iter().enumerate() {
            let origin = sec.vma_origin();
            let bps = shift_breakpoints(sec, &rungs[si]);
            for label in &sec.labels {
                syms.define(&label.name, SymbolValue::Int((origin + shift_offset(&bps, label.offset)) as i64));
            }
        }

        // (b) Re-select each relaxable fragment's rung from its resolved target
        // (grow-only). `grew` is set ONLY when the selection changes the
        // fragment's byte LENGTH — a same-length rung move (e.g. bra.w → jmp
        // abs.w, both 4 bytes) is recorded but needs no relayout.
        let mut grew = false;
        for (si, sec) in sections.iter().enumerate() {
            let origin = sec.vma_origin();
            let bps = shift_breakpoints(sec, &rungs[si]);
            for fi in 0..sec.fragments.len() {
                let frag = &sec.fragments[fi];
                match frag {
                    Fragment::JmpJsrSym { target, span, .. } => {
                        // GLOBAL scope only (scope None): a bare `jmp .local` to a
                        // dotted local would not resolve here. The front-end must
                        // qualify such targets to fully-dotted names first.
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
                        if asl_width_rule(v, dash_a) == AbsWidth::L && rungs[si][fi] == 0 {
                            rungs[si][fi] = 1;
                            last_grown_span = Some(*span);
                            grew = true;
                        }
                    }
                    Fragment::RelaxAbsSym { target, span, .. } => {
                        let v = match target.fold(&|n| syms.resolve(n, None)) {
                            Fold::Value(v) => v,
                            Fold::Poison => {
                                return Err(vec![Diagnostic {
                                    level: Level::Error,
                                    message: format!(
                                        "unresolved symbolic absolute operand in section {}",
                                        sec.name
                                    ),
                                    primary: *span,
                                }]);
                            }
                        };
                        if asl_width_rule(v, dash_a) == AbsWidth::L && rungs[si][fi] == 0 {
                            rungs[si][fi] = 1;
                            last_grown_span = Some(*span);
                            grew = true;
                        }
                    }
                    Fragment::RelaxLadder { candidates, target, span } => {
                        let v = match target.fold(&|n| syms.resolve(n, None)) {
                            Fold::Value(v) => v,
                            Fold::Poison => {
                                return Err(vec![Diagnostic {
                                    level: Level::Error,
                                    message: format!(
                                        "unresolved branch/ladder target in section {}",
                                        sec.name
                                    ),
                                    primary: *span,
                                }]);
                            }
                        };
                        let frag_start = frag_start_vma(sec, &bps, origin, fi);
                        // Minimal rung whose fixup kind reaches the target.
                        let mut min_reaching: Option<usize> = None;
                        for (k, cand) in candidates.iter().enumerate() {
                            match rung_reaches(cand, frag_start, v, dash_a, *span, &sec.name) {
                                Ok(true) => {
                                    min_reaching = Some(k);
                                    break;
                                }
                                Ok(false) => {}
                                Err(d) => return Err(vec![d]),
                            }
                        }
                        // If nothing reaches mid-pass, HOLD at the last rung
                        // provisionally: addresses are still moving, and a
                        // premature error here could be spurious. The real
                        // out-of-reach error is raised at the convergence sweep.
                        let want = min_reaching.unwrap_or(candidates.len() - 1);
                        let new = rungs[si][fi].max(want); // grow-only
                        if new != rungs[si][fi] {
                            let len_before = candidates[rungs[si][fi]].bytes.len();
                            let len_after = candidates[new].bytes.len();
                            rungs[si][fi] = new;
                            last_grown_span = Some(*span);
                            // Only a LENGTH change forces a relayout pass; a
                            // same-length rung move must still persist (done
                            // above) but does not set `grew`.
                            if len_before != len_after {
                                grew = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if !grew {
            // (c) Convergence sweep: every RelaxLadder's chosen candidate must
            // actually reach the target. A ladder that maxed at its last rung and
            // still cannot reach (tonight: a conditional/unsized branch whose last
            // rung is PcRelDisp16 — the only ladder shape that can exhaust) is a
            // hard error naming the signed distance. Collect ALL such errors.
            let mut errs: Vec<Diagnostic> = Vec::new();
            for (si, sec) in sections.iter().enumerate() {
                let origin = sec.vma_origin();
                let bps = shift_breakpoints(sec, &rungs[si]);
                for fi in 0..sec.fragments.len() {
                    if let Fragment::RelaxLadder { candidates, target, span } = &sec.fragments[fi] {
                        let v = match target.fold(&|n| syms.resolve(n, None)) {
                            Fold::Value(v) => v,
                            Fold::Poison => continue, // reported in pass (b) already
                        };
                        let frag_start = frag_start_vma(sec, &bps, origin, fi);
                        let cand = &candidates[rungs[si][fi]];
                        match rung_reaches(cand, frag_start, v, dash_a, *span, &sec.name) {
                            Ok(true) => {}
                            Ok(false) => {
                                errs.push(out_of_reach_diag(cand, frag_start, v, *span, &sec.name));
                            }
                            Err(d) => errs.push(d),
                        }
                    }
                }
            }
            if !errs.is_empty() {
                return Err(errs);
            }

            // (d) Converged & every ladder reaches: lower fragments + shift labels.
            let out = sections
                .iter()
                .enumerate()
                .map(|(si, sec)| {
                    let bps = shift_breakpoints(sec, &rungs[si]);
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
                                lower_jmp_jsr(*is_jsr, target.clone(), rung_width(rungs[si][fi]), *span)
                            }
                            // SELECT the width candidate the fixpoint chose and emit
                            // it verbatim (no m68k encoding in the linker): the abs.w
                            // `short` block for rung 0, the abs.l `long` block
                            // otherwise, each carrying its own operand fixup.
                            Fragment::RelaxAbsSym { short, long, span, .. } => {
                                let cand = match rung_width(rungs[si][fi]) {
                                    AbsWidth::W => short,
                                    AbsWidth::L => long,
                                };
                                Fragment::Data(DataFragment {
                                    bytes: cand.bytes.clone(),
                                    fixups: vec![cand.fixup.clone()],
                                    span: *span,
                                })
                            }
                            // Lower the chosen ladder rung — the same shape as the
                            // RelaxAbsSym arm: the candidate's bytes + its single
                            // fixup as a Data fragment; the linker encodes nothing.
                            Fragment::RelaxLadder { candidates, span, .. } => {
                                let cand = &candidates[rungs[si][fi]];
                                Fragment::Data(DataFragment {
                                    bytes: cand.bytes.clone(),
                                    fixups: vec![cand.fixup.clone()],
                                    span: *span,
                                })
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
        message: format!("relaxation width selection did not converge within {cap} passes"),
        // Point at a fragment that grew on the final pass (the likely culprit);
        // fall back to the first fragment's span if nothing grew.
        primary: last_grown_span
            .or_else(|| sections.iter().flat_map(|s| s.fragments.iter()).map(frag_span).next())
            .unwrap_or(Span { source: sigil_span::SourceId(0), start: 0, end: 0 }),
    }])
}

/// The `[branch.out-of-reach]` diagnostic for a ladder that maxed at its last
/// rung and still cannot reach. The message is DERIVED from the last candidate's
/// fixup kind: today the only ladder shape that can exhaust is a conditional /
/// unsized branch whose last rung is `PcRelDisp16` (no far form), so we name the
/// signed distance and the ±32766 word range and end with the D2.18 trampoline
/// note. Any other exhausting kind would be a new far form and should extend
/// this message.
fn out_of_reach_diag(cand: &RelaxCandidate, frag_start: u32, target: i64, span: Span, section: &str) -> Diagnostic {
    let site_vma = frag_start as i64 + cand.fixup.offset as i64;
    let msg = match cand.fixup.kind {
        FixupKind::PcRelDisp16 => {
            // The signed distance N is measured from the disp word's own VMA.
            let disp = target - site_vma;
            format!(
                "[branch.out-of-reach] branch target in section {section} is {disp} bytes away (max \u{00B1}32766); conditional branches have no far form — jbcc trampolines are deferred (D2.18)"
            )
        }
        FixupKind::PcRel8 => {
            let disp = target - (site_vma + 1);
            format!("[branch.out-of-reach] branch target in section {section} is {disp} bytes away (max \u{00B1}127)")
        }
        other => {
            format!("[branch.out-of-reach] ladder in section {section} cannot reach its target (last rung fixup {other:?})")
        }
    };
    Diagnostic { level: Level::Error, message: msg, primary: span }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_ir::{
        Cpu, DataFragment, Expr, Fixup, FixupKind, Fragment, Label, RelaxCandidate, Section,
        SymbolTable, SymbolValue,
    };
    use sigil_span::{SourceId, Span};

    fn sp() -> Span {
        Span { source: SourceId(0), start: 0, end: 0 }
    }

    /// A hand-built `RelaxAbsSym` modelling `move.w D0, (target).W/.L`: the abs.w
    /// candidate is a 4-byte block (opcode 0x31C0 + Abs16Be operand at offset 2),
    /// the abs.l candidate a 6-byte block (opcode 0x33C0 + Abs32Be operand at
    /// offset 2). This task doesn't need the real encoder — only correct SELECTION
    /// + fixup emission + length accounting — so plausible opcodes suffice.
    fn relax_move(target: &str) -> Fragment {
        Fragment::RelaxAbsSym {
            short: RelaxCandidate {
                bytes: vec![0x31, 0xC0, 0x00, 0x00],
                fixup: Fixup { kind: FixupKind::Abs16Be, offset: 2, target: Expr::Sym(target.into()) },
            },
            long: RelaxCandidate {
                bytes: vec![0x33, 0xC0, 0x00, 0x00, 0x00, 0x00],
                fixup: Fixup { kind: FixupKind::Abs32Be, offset: 2, target: Expr::Sym(target.into()) },
            },
            target: Expr::Sym(target.into()),
            span: sp(),
        }
    }

    // ---- RelaxLadder test builders (REAL m68k branch/jmp encodings) -----------

    /// One `bra.s` candidate: `[0x60, 0x00]`, PcRel8 disp byte at offset 1.
    fn bra_s(target: &str) -> RelaxCandidate {
        RelaxCandidate {
            bytes: vec![0x60, 0x00],
            fixup: Fixup { kind: FixupKind::PcRel8, offset: 1, target: Expr::Sym(target.into()) },
        }
    }
    /// One `bra.w` candidate: `[0x60, 0x00, 0x00, 0x00]`, PcRelDisp16 word at offset 2.
    fn bra_w(target: &str) -> RelaxCandidate {
        RelaxCandidate {
            bytes: vec![0x60, 0x00, 0x00, 0x00],
            fixup: Fixup { kind: FixupKind::PcRelDisp16, offset: 2, target: Expr::Sym(target.into()) },
        }
    }
    /// One `jmp abs.w` candidate: `[0x4E, 0xF8, 0, 0]`, Abs16Be operand at offset 2.
    fn jmp_w(target: &str) -> RelaxCandidate {
        RelaxCandidate {
            bytes: vec![0x4E, 0xF8, 0, 0],
            fixup: Fixup { kind: FixupKind::Abs16Be, offset: 2, target: Expr::Sym(target.into()) },
        }
    }
    /// One `jmp abs.l` candidate: `[0x4E, 0xF9, 0, 0, 0, 0]`, Abs32Be operand at offset 2.
    fn jmp_l(target: &str) -> RelaxCandidate {
        RelaxCandidate {
            bytes: vec![0x4E, 0xF9, 0, 0, 0, 0],
            fixup: Fixup { kind: FixupKind::Abs32Be, offset: 2, target: Expr::Sym(target.into()) },
        }
    }
    /// The full 4-rung `jbra` ladder to `target`.
    fn jbra(target: &str) -> Fragment {
        Fragment::RelaxLadder {
            candidates: vec![bra_s(target), bra_w(target), jmp_w(target), jmp_l(target)],
            target: Expr::Sym(target.into()),
            span: sp(),
        }
    }
    /// A 2-rung conditional branch ladder (`bne.s` → `bne.w`, opcode 0x66) — no far form.
    fn bne_ladder(target: &str) -> Fragment {
        Fragment::RelaxLadder {
            candidates: vec![
                RelaxCandidate {
                    bytes: vec![0x66, 0x00],
                    fixup: Fixup { kind: FixupKind::PcRel8, offset: 1, target: Expr::Sym(target.into()) },
                },
                RelaxCandidate {
                    bytes: vec![0x66, 0x00, 0x00, 0x00],
                    fixup: Fixup { kind: FixupKind::PcRelDisp16, offset: 2, target: Expr::Sym(target.into()) },
                },
            ],
            target: Expr::Sym(target.into()),
            span: sp(),
        }
    }

    // --------------------------------------------------------------------------

    #[test]
    fn resolve_relax_abs_selects_short_for_low_target() {
        // Lo resolves into the abs.w range [0, 0x7FFF] → pick the `short` (abs.w)
        // candidate: 4-byte block, Abs16Be operand fixup at offset 2.
        let mut stubs = SymbolTable::new();
        stubs.define("Lo", SymbolValue::Int(0x1000));
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![relax_move("Lo")],
        };
        let out = resolve_layout(&[sec], &stubs, true).unwrap();
        match &out[0].fragments[0] {
            Fragment::Data(d) => {
                assert_eq!(d.bytes, vec![0x31, 0xC0, 0x00, 0x00]);
                assert_eq!(d.fixups.len(), 1);
                assert_eq!(d.fixups[0].kind, FixupKind::Abs16Be);
                assert_eq!(d.fixups[0].offset, 2);
            }
            other => panic!("expected lowered Data, got {other:?}"),
        }
        // link() patches the Abs16Be operand with Lo's VMA (0x1000).
        let linked = crate::link(&out, &stubs).unwrap();
        assert_eq!(linked.section("c").unwrap().bytes, vec![0x31, 0xC0, 0x10, 0x00]);
    }

    #[test]
    fn resolve_relax_abs_selects_short_for_ram_target() {
        // A RAM-range target $FF8000 is abs.w too (16-bit sign extension) — the
        // upper half of `asl_width_rule`'s abs.w set. Abs16Be writes the low 16
        // bits (0x8000).
        let mut stubs = SymbolTable::new();
        stubs.define("Ram", SymbolValue::Int(0xFF_8000));
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![relax_move("Ram")],
        };
        let out = resolve_layout(&[sec], &stubs, true).unwrap();
        match &out[0].fragments[0] {
            Fragment::Data(d) => {
                assert_eq!(d.bytes, vec![0x31, 0xC0, 0x00, 0x00]);
                assert_eq!(d.fixups[0].kind, FixupKind::Abs16Be);
                assert_eq!(d.fixups[0].offset, 2);
            }
            other => panic!("expected lowered Data, got {other:?}"),
        }
        let linked = crate::link(&out, &stubs).unwrap();
        assert_eq!(linked.section("c").unwrap().bytes, vec![0x31, 0xC0, 0x80, 0x00]);
    }

    #[test]
    fn resolve_relax_abs_selects_long_for_mid_rom_target() {
        // Hi = 0x12_3456 is > 0x7FFF and < 0xFF_8000 → abs.l: pick the `long`
        // 6-byte candidate with an Abs32Be operand fixup at offset 2.
        let mut stubs = SymbolTable::new();
        stubs.define("Hi", SymbolValue::Int(0x12_3456));
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![relax_move("Hi")],
        };
        let out = resolve_layout(&[sec], &stubs, true).unwrap();
        match &out[0].fragments[0] {
            Fragment::Data(d) => {
                assert_eq!(d.bytes, vec![0x33, 0xC0, 0x00, 0x00, 0x00, 0x00]);
                assert_eq!(d.fixups.len(), 1);
                assert_eq!(d.fixups[0].kind, FixupKind::Abs32Be);
                assert_eq!(d.fixups[0].offset, 2);
            }
            other => panic!("expected lowered Data, got {other:?}"),
        }
        let linked = crate::link(&out, &stubs).unwrap();
        assert_eq!(
            linked.section("c").unwrap().bytes,
            vec![0x33, 0xC0, 0x00, 0x12, 0x34, 0x56]
        );
    }

    #[test]
    fn resolve_relax_abs_grows_and_shifts_following_label() {
        // Layout-length accounting: a `RelaxAbsSym` to a high (abs.l) target grows
        // from the baseline 4-byte `short` to the 6-byte `long`. A label `After`
        // authored at all-abs.w offset 4 (immediately past the fragment) MUST shift
        // to 6, and an Abs32Be reference to `After` in the following fragment must
        // resolve to that shifted VMA — proving downstream addresses account for
        // the chosen candidate's length.
        let mut stubs = SymbolTable::new();
        stubs.define("Hi", SymbolValue::Int(0x12_3456));
        let sec = Section {
            name: "code".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![Label { name: "After".into(), offset: 4 }],
            fragments: vec![
                relax_move("Hi"),
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
        // long move to Hi (6 bytes), then Abs32Be(After) = 0x00000006.
        assert_eq!(
            linked.section("code").unwrap().bytes,
            vec![0x33, 0xC0, 0x00, 0x12, 0x34, 0x56, 0x00, 0x00, 0x00, 0x06]
        );
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
    fn resolve_layout_refuses_org_and_ladder_in_the_same_section() {
        // The generalized guard must ALSO refuse a RelaxLadder alongside an Org —
        // same prefix-sum hazard as JmpJsrSym. The message now names the ladder.
        let sec = Section {
            name: "code".into(),
            cpu: Cpu::M68000,
            vma_base: Some(0x1000),
            lma: 0x1000,
            labels: vec![Label { name: "L".into(), offset: 4 }],
            fragments: vec![
                jbra("L"),
                Fragment::Org { target: 8, fill: 0x00, span: sp() },
                Fragment::Data(DataFragment { bytes: vec![1], fixups: vec![], span: sp() }),
            ],
        };
        let err = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap_err();
        assert!(
            err.iter().any(|d| d.message.contains("org") && d.message.contains("ladder")),
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

    // ======================= RelaxLadder tests ================================

    #[test]
    fn ladder_selects_bra_s_for_near_forward_target() {
        // jbra L; ...gap...; L: nop. The bra.s at op VMA 0 has its disp byte at
        // VMA 1; a target a few bytes ahead fits i8 and is non-zero → rung 0.
        // Baseline layout: bra.s [0,2), Data(6 bytes) [2,8), L @ 8.
        // disp = 8 - (1+1) = 6, fits i8, ≠ 0 → bra.s. link: 0x60, 0x06.
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![Label { name: "L".into(), offset: 8 }],
            fragments: vec![
                jbra("L"),
                Fragment::Data(DataFragment { bytes: vec![0; 6], fixups: vec![], span: sp() }),
            ],
        };
        let out = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap();
        match &out[0].fragments[0] {
            Fragment::Data(d) => {
                assert_eq!(d.bytes.len(), 2);
                assert_eq!(d.fixups[0].kind, FixupKind::PcRel8);
            }
            other => panic!("expected 2-byte bra.s Data, got {other:?}"),
        }
        // L stays at offset 8 (bra.s did not grow the baseline).
        assert_eq!(out[0].labels.iter().find(|l| l.name == "L").unwrap().offset, 8);
        let linked = crate::link(&out, &SymbolTable::new()).unwrap();
        assert_eq!(&linked.section("c").unwrap().bytes[..2], &[0x60, 0x06]);
    }

    #[test]
    fn ladder_selects_bra_w_for_far_forward_target() {
        // A target ~0x400 bytes ahead overflows i8 (max +127 from the disp byte)
        // but fits i16 → rung 1 (bra.w). Fill(0x400) after the branch, then L.
        // Baseline (rung 0, 2-byte branch): jbra [0,2), Fill(0x400) [2, 0x402),
        // L @ 0x402. After the branch grows to 4 bytes, L shifts +2 → 0x404.
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![Label { name: "L".into(), offset: 0x402 }],
            fragments: vec![
                jbra("L"),
                Fragment::Fill { value: 0, count: 0x400, span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0x4E, 0x71], fixups: vec![], span: sp() }),
            ],
        };
        let out = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap();
        match &out[0].fragments[0] {
            Fragment::Data(d) => {
                assert_eq!(d.bytes.len(), 4);
                assert_eq!(d.fixups[0].kind, FixupKind::PcRelDisp16);
                assert_eq!(d.fixups[0].offset, 2);
            }
            other => panic!("expected 4-byte bra.w Data, got {other:?}"),
        }
        // L shifted from baseline 0x402 (2-byte rung 0) to 0x404 (4-byte rung 1).
        assert_eq!(out[0].labels.iter().find(|l| l.name == "L").unwrap().offset, 0x404);
        let linked = crate::link(&out, &SymbolTable::new()).unwrap();
        // disp = target(0x404) - site_vma(2) = 0x402.
        assert_eq!(&linked.section("c").unwrap().bytes[..4], &[0x60, 0x00, 0x04, 0x02]);
    }

    #[test]
    fn ladder_selects_jmp_absw_for_low_far_target() {
        // A target that is FAR (word-branch out of reach) but sits in the abs.w
        // absolute range [0, 0x7FFF] selects rung 2 (jmp abs.w). We force this
        // with a low absolute stub the branch cannot reach relatively but jmp.w can.
        // frag_start = 0; PcRelDisp16 disp = 0x7000 - 2 = 0x6FFE fits i16... so we
        // must push the target beyond i16 branch reach. Use a phased section: the
        // branch site VMA is huge, but the target is a low absolute stub.
        let mut stubs = SymbolTable::new();
        stubs.define("Lo", SymbolValue::Int(0x1000)); // abs.w range, far from the site
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: Some(0x40_0000), // site VMA ~0x400000; branch to 0x1000 is > i16 away
            lma: 0x40_0000,
            labels: vec![],
            fragments: vec![jbra("Lo")],
        };
        let out = resolve_layout(&[sec], &stubs, true).unwrap();
        match &out[0].fragments[0] {
            Fragment::Data(d) => {
                assert_eq!(d.bytes, vec![0x4E, 0xF8, 0, 0]);
                assert_eq!(d.fixups[0].kind, FixupKind::Abs16Be);
            }
            other => panic!("expected jmp abs.w Data, got {other:?}"),
        }
        let linked = crate::link(&out, &stubs).unwrap();
        assert_eq!(linked.section("c").unwrap().bytes, vec![0x4E, 0xF8, 0x10, 0x00]);
    }

    #[test]
    fn ladder_selects_jmp_absl_for_high_far_target() {
        // A far target in the abs.l range (> 0x7FFF, < 0xFF_8000) exhausts bra.s,
        // bra.w, and jmp abs.w → rung 3 (jmp abs.l).
        let mut stubs = SymbolTable::new();
        stubs.define("Hi", SymbolValue::Int(0x12_3456));
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: Some(0x40_0000),
            lma: 0x40_0000,
            labels: vec![],
            fragments: vec![jbra("Hi")],
        };
        let out = resolve_layout(&[sec], &stubs, true).unwrap();
        match &out[0].fragments[0] {
            Fragment::Data(d) => {
                assert_eq!(d.bytes, vec![0x4E, 0xF9, 0, 0, 0, 0]);
                assert_eq!(d.fixups[0].kind, FixupKind::Abs32Be);
            }
            other => panic!("expected jmp abs.l Data, got {other:?}"),
        }
        let linked = crate::link(&out, &stubs).unwrap();
        assert_eq!(linked.section("c").unwrap().bytes, vec![0x4E, 0xF9, 0x00, 0x12, 0x34, 0x56]);
    }

    #[test]
    fn ladder_selects_bra_s_for_backward_target() {
        // L: nop nop ...; jbra L (backward). The disp is negative and fits i8 → bra.s.
        // Layout: L@0, Data(4 bytes) [0,4), jbra [4,6). bra.s disp byte VMA = 5;
        // disp = 0 - (5+1) = -6, fits i8, ≠ 0 → bra.s.
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![Label { name: "L".into(), offset: 0 }],
            fragments: vec![
                Fragment::Data(DataFragment { bytes: vec![0; 4], fixups: vec![], span: sp() }),
                jbra("L"),
            ],
        };
        let out = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap();
        match &out[0].fragments[1] {
            Fragment::Data(d) => {
                assert_eq!(d.bytes.len(), 2);
                assert_eq!(d.fixups[0].kind, FixupKind::PcRel8);
            }
            other => panic!("expected 2-byte bra.s Data, got {other:?}"),
        }
        let linked = crate::link(&out, &SymbolTable::new()).unwrap();
        // disp = 0 - (5+1) = -6 = 0xFA.
        assert_eq!(&linked.section("c").unwrap().bytes[4..], &[0x60, 0xFA]);
    }

    #[test]
    fn ladder_skips_bra_s_on_disp_zero() {
        // The 0x00 word-form escape: a bra.s whose disp would be exactly 0 is
        // UNENCODABLE, so rung 0 is skipped and rung 1 (bra.w) is chosen even
        // though the target is "in i8 range". Target = op+2 makes disp 0.
        // jbra L; L: (immediately after). At rung 0 (2 bytes) L baseline = 2;
        // bra.s disp = 2 - (1+1) = 0 → skip. At rung 1 (4 bytes) L shifts to 4;
        // bra.w disp = 4 - 2 = 2, fits i16 → bra.w.
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![Label { name: "L".into(), offset: 2 }],
            fragments: vec![jbra("L")],
        };
        let out = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap();
        match &out[0].fragments[0] {
            Fragment::Data(d) => {
                assert_eq!(d.bytes.len(), 4, "disp-0 must escape bra.s to bra.w");
                assert_eq!(d.fixups[0].kind, FixupKind::PcRelDisp16);
            }
            other => panic!("expected 4-byte bra.w Data, got {other:?}"),
        }
        // L shifted 2 → 4 as the branch grew to 4 bytes.
        assert_eq!(out[0].labels.iter().find(|l| l.name == "L").unwrap().offset, 4);
        let linked = crate::link(&out, &SymbolTable::new()).unwrap();
        // bra.w disp = 4 - 2 = 2.
        assert_eq!(linked.section("c").unwrap().bytes, vec![0x60, 0x00, 0x00, 0x02]);
    }

    #[test]
    fn ladder_growth_cascade_pushes_second_branch_across_reach_boundary() {
        // A GENUINE relative-branch cascade: one ladder's +2 growth is inserted
        // BETWEEN a second (backward) branch and its target, pushing that branch's
        // displacement across the i8 boundary so it must ALSO grow. A relative
        // disp is translation-invariant, so only growth *between* the site and the
        // target can trigger a cascade — this is that shape.
        //
        // Baseline (rung 0, branches = 2 bytes):
        //   Back @ 0
        //   frag0 jbra Far   [0, 2)     -- Far is way ahead → grows to bra.w (+2)
        //   frag1 Fill(0x7C) [2, 0x7E)
        //   frag2 jbra Back  [0x7E, 0x80)
        //   ...pad out to Far...
        // frag2 backward disp (baseline): Back(0) - (0x7F + 1) = -0x80 = -128,
        // exactly fits bra.s. After frag0 grows +2, frag2's site shifts to 0x81
        // while Back stays at 0: disp = 0 - (0x81 + 1) = -130 → OVERFLOWS i8, so
        // frag2 cascades to bra.w. Proves growth re-drives a downstream rung.
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![
                Label { name: "Back".into(), offset: 0 },
                Label { name: "Far".into(), offset: 0x100 },
            ],
            fragments: vec![
                jbra("Far"),                                          // [0, 2) grows +2
                Fragment::Fill { value: 0, count: 0x7C, span: sp() }, // [2, 0x7E)
                jbra("Back"),                                         // [0x7E, 0x80)
                Fragment::Fill { value: 0, count: 0x80, span: sp() }, // [0x80, 0x100), Far @ 0x100
            ],
        };
        let out = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap();
        // frag0 grew to bra.w (Far ~0x100 ahead, out of i8).
        assert!(matches!(&out[0].fragments[0], Fragment::Data(d) if d.bytes.len() == 4),
            "frag0 should be bra.w");
        // frag2 CASCADED to bra.w because frag0's +2 tipped its backward disp over -128.
        assert!(matches!(&out[0].fragments[2], Fragment::Data(d) if d.bytes.len() == 4),
            "frag2 should have cascaded to bra.w, got {:?}", &out[0].fragments[2]);
        // Labels shifted consistently: frag0 +2 and frag2 +2 = +4 total before Far.
        assert_eq!(out[0].labels.iter().find(|l| l.name == "Far").unwrap().offset, 0x104);
        // And the whole thing links (both backward/forward disps in range).
        let linked = crate::link(&out, &SymbolTable::new()).unwrap();
        // frag2 starts at image offset 0x80 (frag0=4, fill=0x7C → 4+0x7C=0x80). As
        // bra.w its disp word sits at VMA 0x82; disp = Back(0) - 0x82 = -0x82 = 0xFF7E.
        let bytes = &linked.section("c").unwrap().bytes;
        assert_eq!(&bytes[0x80..0x84], &[0x60, 0x00, 0xFF, 0x7E]);
    }

    #[test]
    fn ladder_same_length_rung_move_converges() {
        // A same-LENGTH rung transition: bra.w (rung 1, 4 bytes) → jmp abs.w
        // (rung 2, 4 bytes). Both are 4 bytes, so moving between them changes NO
        // downstream layout and must NOT trigger an extra `grew` relayout pass —
        // but the move must still PERSIST and be lowered. We engineer a target
        // that bra.w cannot reach (out of i16) but jmp abs.w can (abs.w range).
        // Same construction as ladder_selects_jmp_absw: a low absolute target far
        // from a high site VMA. rung 0 (bra.s) and rung 1 (bra.w) both fail reach;
        // rung 2 (jmp abs.w) is the minimal reaching rung. Confirms selection
        // lands on rung 2 directly (the "same-length move" is 1→2, both 4 bytes,
        // vs rung 0 = 2 bytes; the +2 growth from rung 0 is the only length pass).
        // frag0 (`jbra Far`) grows 2 -> 4, shifting frag1's absolute site UP by 2
        // relative to a FIXED low stub `Lo`. Tuned so frag1's backward bra.w disp
        // sits at the -i16 edge at baseline and crosses out of i16 after frag0's
        // +2 -- at which point jmp abs.w (Lo is in the abs.w absolute range)
        // becomes the minimal reaching rung. Both rungs are 4 bytes, so this move
        // changes NO downstream layout: it must still PERSIST and lower to jmp
        // abs.w, and the fixpoint must converge.
        let mut stubs = SymbolTable::new();
        stubs.define("Lo", SymbolValue::Int(0));
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![Label { name: "Far".into(), offset: 0x1_0000 }],
            fragments: vec![
                jbra("Far"),                                            // [0,2) grows +2 (Far high)
                Fragment::Fill { value: 0, count: 0x7FFA, span: sp() }, // pad frag2 site near 0x8000
                jbra("Lo"),                                             // backward to VMA 0
                Fragment::Fill { value: 0, count: 0x8000, span: sp() }, // pad out to Far
            ],
        };
        let out = resolve_layout(&[sec], &stubs, true).unwrap();
        // frag2 must be a 4-byte jmp abs.w (rung 2), NOT bra.w: its backward disp
        // left i16 once frag0 grew, so it moved same-length 1 -> 2.
        match &out[0].fragments[2] {
            Fragment::Data(d) => {
                assert_eq!(d.bytes.len(), 4);
                assert_eq!(d.bytes[..2], [0x4E, 0xF8], "expected jmp abs.w opcode, got {:?}", d.bytes);
                assert_eq!(d.fixups[0].kind, FixupKind::Abs16Be);
            }
            other => panic!("expected 4-byte jmp abs.w Data, got {other:?}"),
        }
        // Converges and links: frag2 jmp abs.w to Lo (VMA 0) -> operand 0x0000.
        // frag2's image offset = the summed lengths of frag0 (bra.w = 4) + the
        // 0x7FFA fill = 0x7FFE... but frag0 itself may have needed abs.l; rather
        // than hardcode, locate frag2 by summing the emitted fragment lengths.
        let mut off = 0usize;
        for f in &out[0].fragments[..2] {
            if let Fragment::Data(d) = f {
                off += d.bytes.len();
            } else if let Fragment::Fill { count, .. } = f {
                off += *count as usize;
            }
        }
        let linked = crate::link(&out, &stubs).unwrap();
        let bytes = &linked.section("c").unwrap().bytes;
        assert_eq!(&bytes[off..off + 4], &[0x4E, 0xF8, 0x00, 0x00]);
    }

    #[test]
    fn ladder_conditional_out_of_reach_reports_signed_distance() {
        // A 2-rung conditional (bne.s → bne.w) whose target is beyond i16 word
        // reach has NO far form → the convergence sweep reports [branch.out-of-reach]
        // naming the signed distance and the trampoline-deferred note.
        let mut stubs = SymbolTable::new();
        stubs.define("VeryFar", SymbolValue::Int(0x20_0000)); // >> i16 from site ~0
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![bne_ladder("VeryFar")],
        };
        let err = resolve_layout(&[sec], &stubs, true).unwrap_err();
        assert!(
            err.iter().any(|d| d.message.contains("[branch.out-of-reach]")
                && d.message.contains("bytes away")
                && d.message.contains("jbcc trampolines are deferred (D2.18)")),
            "got: {:?}",
            err
        );
        // The signed distance must be present (target 0x200000 - site 2 = 0x1FFFFE).
        assert!(err.iter().any(|d| d.message.contains("2097150")), "distance missing: {:?}", err);
    }

    #[test]
    fn ladder_conditional_backward_out_of_reach_reports_negative_distance() {
        // A backward conditional branch too far behind → negative signed distance.
        let mut stubs = SymbolTable::new();
        stubs.define("WayBack", SymbolValue::Int(0x10)); // site is high, target low
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: Some(0x20_0000),
            lma: 0x20_0000,
            labels: vec![],
            fragments: vec![bne_ladder("WayBack")],
        };
        let err = resolve_layout(&[sec], &stubs, true).unwrap_err();
        assert!(
            err.iter().any(|d| d.message.contains("[branch.out-of-reach]") && d.message.contains('-')),
            "expected a negative distance, got: {:?}",
            err
        );
    }

    #[test]
    fn ladder_reports_unresolved_target_as_poison() {
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![jbra("Nope")],
        };
        let err = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap_err();
        assert!(
            err.iter().any(|d| d.message.contains("unresolved") && d.message.contains("ladder")),
            "got: {:?}",
            err
        );
    }

    #[test]
    fn empty_ladder_is_a_loud_construction_error_in_release() {
        // In debug builds `debug_assert!` fires; the defensive release path emits
        // a loud diagnostic instead of mis-lowering. We can only exercise the
        // diagnostic path when debug assertions are OFF.
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![Fragment::RelaxLadder {
                candidates: vec![],
                target: Expr::Sym("L".into()),
                span: sp(),
            }],
        };
        let secs = [sec];
        if cfg!(debug_assertions) {
            // debug_assert! would panic; assert the panic happens.
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                resolve_layout(&secs, &SymbolTable::new(), true)
            }));
            assert!(r.is_err(), "empty ladder must trip the debug_assert");
        } else {
            let err = resolve_layout(&secs, &SymbolTable::new(), true).unwrap_err();
            assert!(err.iter().any(|d| d.message.contains("empty RelaxLadder")), "got: {:?}", err);
        }
    }

    #[test]
    fn misordered_ladder_is_a_loud_construction_error_in_release() {
        // Symmetric to the empty-ladder guard: candidates whose bytes.len()
        // DECREASES at an adjacent pair (here 4-byte bra.w before 2-byte bra.s)
        // violate the construction contract — a decreasing pair would let a rung
        // grow while the fragment shrinks, corrupting the prefix-sum layout math.
        // Debug builds trip the debug_assert; release builds must refuse with a
        // loud diagnostic rather than proceed.
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![Label { name: "L".into(), offset: 8 }],
            fragments: vec![Fragment::RelaxLadder {
                candidates: vec![bra_w("L"), bra_s("L")], // 4 bytes then 2: mis-ordered
                target: Expr::Sym("L".into()),
                span: sp(),
            }],
        };
        let secs = [sec];
        if cfg!(debug_assertions) {
            // debug_assert! would panic; assert the panic happens.
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                resolve_layout(&secs, &SymbolTable::new(), true)
            }));
            assert!(r.is_err(), "mis-ordered ladder must trip the debug_assert");
        } else {
            let err = resolve_layout(&secs, &SymbolTable::new(), true).unwrap_err();
            assert!(
                err.iter().any(|d| d.message.contains("mis-ordered RelaxLadder")
                    && d.message.contains("non-decreasing")),
                "got: {:?}",
                err
            );
        }
    }
}
