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
    Section, SectionPlacement, SymbolTable, SymbolValue,
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
/// no length. Its repositioning is handled by the callers that walk fragments
/// (`shift_breakpoints`/`frag_start_vma`/`run_overrun_diag` all treat `Org` as
/// a run barrier that seeks their cursor to `target`, NOT as a length-0 chunk),
/// so the value here is never summed as content — it exists only so the `match`
/// is total.
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

/// The section's FINAL image extent under the CURRENT rungs (R7p.2): delegates
/// to `Section::replay_extent` (ir lib.rs) — the same cursor-replay shape as
/// `Section::placement_span`/`vma_len` — but counting each relaxable fragment
/// at its CURRENT rung width (via `frag_len(frag, rungs[fi])`) instead of its
/// MAX width. This is what the link-time placement pass advances the group
/// cursor by — a chained successor's base derives from its predecessors' FINAL
/// sizes, not their baked baselines.
///
/// `replay_extent` handles `Org`'s seek itself (never calls `frag_len` for it);
/// `Reserve` advances the cursor via `frag_len`'s `*count` arm but contributes
/// no image bytes, mirroring `placement_span`/`vma_len` so the max-extent is
/// the address-space span, matching what the placer reserved.
fn final_size(sec: &Section, rungs: &[usize]) -> u32 {
    sec.replay_extent(|fi, frag| frag_len(frag, rungs[fi]))
}

/// The section's final IMAGE extent under the current rungs — the byte count
/// `link()`/`flatten` actually place at the LMA, which is `final_size` MINUS the
/// address-only `Reserve` (`ds`) span. Mirrors the `link` cursor replay
/// (lib.rs): `Data`/`Fill` advance the image cursor, `Org` seeks it, and
/// `Reserve` is a no-op (RAM `ds` reserves VMA/PC space but emits no image
/// bytes). A section that is ALL `Reserve` (Aeon's phased `$FFFF0000+` RAM
/// blocks, whose VMA base is disp'd into RAM while their LMA anchors at the
/// physical counter) has an image extent of 0 here even though `final_size`
/// (VMA span) is large — so the overlap check must key on THIS, not
/// `final_size`, to match what `flatten` places (a reserve-only section
/// contributes no bytes and can neither clobber nor be clobbered).
fn image_final_size(sec: &Section, rungs: &[usize]) -> u32 {
    let mut cursor: u32 = 0;
    let mut max_extent: u32 = 0;
    for (fi, frag) in sec.fragments.iter().enumerate() {
        match frag {
            Fragment::Org { target, .. } => cursor = *target,
            Fragment::Reserve { .. } => {} // address-only: no image bytes
            other => cursor += frag_len(other, rungs[fi]),
        }
        if cursor > max_extent {
            max_extent = cursor;
        }
    }
    max_extent
}

/// The link-time placement pass (R7p.2). Walk sections in vec order with a cursor
/// PER `group`: a `Pinned` section resets its group cursor to its baked anchor
/// (`sec.lma`); a `Chained` section lands at its group cursor. The cursor then
/// advances by `max(reserved_span, final_size(sec, rungs))` — the reserved-span
/// arm preserves the multi-module max-span gaps (byte-identity, R7p.6), the
/// final-size arm corrects the growth-past-baseline understatement (the L-H.1
/// fix). REWRITES `sec.lma` in place. Returns whether any lma moved this pass (so
/// the joint fixpoint knows placement is not yet stable).
fn place_pass(placed: &mut [Section], rungs: &[Vec<usize>]) -> bool {
    // Per-group write cursor. `None` group shares one anonymous cursor.
    let mut cursors: std::collections::HashMap<Option<String>, u32> =
        std::collections::HashMap::new();
    let mut moved = false;
    for (si, sec) in placed.iter_mut().enumerate() {
        let mut base = match sec.placement {
            SectionPlacement::Pinned => {
                // A pin resets its group cursor to its baked anchor value.
                sec.lma
            }
            SectionPlacement::Chained => {
                // A chained section lands at its group cursor (or its baked lma if
                // it is the first section seen in its group — defensive: the
                // front-ends always stamp the first-per-group `Pinned`).
                *cursors.get(&sec.group).unwrap_or(&sec.lma)
            }
        };
        // The section's FINAL image extent under the current rungs. Hoisted (not
        // recomputed) so the bank seam below reuses this one value — do NOT add a
        // fifth cursor-replay loop (the T4 carry-forward constraint).
        let final_sz = final_size(sec, &rungs[si]);
        // #7-main: bank bump seam (D7.2). If this section carries a bank
        // constraint and its `[base, base+final)` extent would STRADDLE an
        // N-boundary, bump a CHAINED base to the next multiple of N. Bump ONLY
        // when straddling — a section that fits before the boundary stays put
        // (D7.2: aeon's always-`align $8000` wastes up to N bytes; the invariant
        // is no-straddle, alignment is just one strategy). PINNED sections are
        // NEVER moved — their address is authoritative — but they are still
        // checked post-fixpoint (a straddling pin is a loud error, not a bump).
        // A content-larger-than-N section is unsatisfiable → the §7.3 "over by K
        // bytes" budget error, reported post-fixpoint by `bank_diag`: it may
        // bump ONCE here to an aligned base, where `next_multiple_of` becomes a
        // no-op, so placement stays stable while the error fires. A bump only
        // ever increases `base`, so grow-only termination is preserved.
        if let Some(n) = sec.bank {
            if sec.placement == SectionPlacement::Chained
                && final_sz > 0
                && base / n != (base + final_sz - 1) / n
            {
                base = base.next_multiple_of(n);
            }
        }
        let advance = sec.reserved_span.max(final_sz);
        if sec.lma != base {
            sec.lma = base;
            moved = true;
        }
        cursors.insert(sec.group.clone(), base + advance);
    }
    moved
}

/// The R7m.2 always-on bank checks, run POST-fixpoint against the FINAL converged
/// placement (D7.5 discharged STRUCTURALLY in the linker — same diagnostic channel
/// as the overlap check, no synthesized LinkAssert rows). Two failure modes, each
/// returning the FIRST offender as an `Error` diagnostic:
///   - content larger than the bank (`final > n`) → the §7.3 "over by K bytes"
///     budget error naming the section (K decimal, matching map.rs validate_section);
///   - a section whose final `[first_byte, last_byte]` straddles an N-boundary
///     (`first / n != last / n`) → an error naming the section, its `[start,end)`
///     extent, and the boundary it crosses. For CHAINED sections the constructive
///     bump in `place_pass` makes the straddle case unreachable; this catches
///     straddling PINS (which are never moved). Empty (final == 0) sections place
///     no bytes and are skipped.
fn bank_diag(placed: &[Section], rungs: &[Vec<usize>]) -> Option<Diagnostic> {
    for (si, sec) in placed.iter().enumerate() {
        let Some(n) = sec.bank else { continue };
        let final_sz = final_size(sec, &rungs[si]);
        if final_sz == 0 {
            continue;
        }
        let span = sec.fragments.first().map(frag_span).unwrap_or(Span {
            source: sigil_span::SourceId(0),
            start: 0,
            end: 0,
        });
        // Content larger than the bank is unsatisfiable — §7.3 budget style
        // ("over by K bytes", K decimal per map.rs validate_section).
        if final_sz > n {
            return Some(Diagnostic {
                level: Level::Error,
                message: format!(
                    "section `{}` ({:#X} bytes) cannot fit a {:#X} bank — over by {} bytes",
                    sec.name,
                    final_sz,
                    n,
                    final_sz - n
                ),
                primary: span,
            });
        }
        let start = sec.lma;
        let end = sec.lma + final_sz;
        // Straddle: first and last byte fall in different N-windows. Unreachable
        // for chained sections (the bump discharges it constructively); this is
        // the catch for a straddling PIN, which is never moved.
        if start / n != (end - 1) / n {
            let boundary = (start / n + 1) * n;
            return Some(Diagnostic {
                level: Level::Error,
                message: format!(
                    "section `{}` [{start:#X}, {end:#X}) straddles the {boundary:#X} bank boundary ({n:#X}-byte bank)",
                    sec.name
                ),
                primary: span,
            });
        }
    }
    None
}

/// The R7p.4 overlap check: after the joint fixpoint converges, return the first
/// pair of NON-EMPTY placed sections whose `[lma, lma + image_final_size)` ranges
/// intersect, as an `Error` diagnostic naming BOTH sections and both hex extents.
/// Emptiness and the range width key on the IMAGE extent (`image_final_size`,
/// which drops the address-only `ds`/`Reserve` span), NOT the VMA `final_size`:
/// `flatten`/`flatten_checked` place only image bytes, so a reserve-only section
/// (Aeon's phased `$FFFF0000+` RAM blocks — VMA in RAM, LMA at the physical
/// counter, zero image bytes) places nothing and can neither clobber nor be
/// clobbered. Using `final_size` here spuriously collided such a RAM block's
/// LMA-0 anchor with the ROM reset section.
fn overlap_diag(placed: &[Section], rungs: &[Vec<usize>]) -> Option<Diagnostic> {
    // Collect (start, end, name, span) for every non-empty section, then scan
    // every pair. O(n²), but n is the section count (small), and this runs once
    // at convergence — not per pass.
    let ranges: Vec<(u32, u32, &str, Span)> = placed
        .iter()
        .enumerate()
        .filter_map(|(si, sec)| {
            let size = image_final_size(sec, &rungs[si]);
            if size == 0 {
                return None;
            }
            let span = sec.fragments.first().map(frag_span).unwrap_or(Span {
                source: sigil_span::SourceId(0),
                start: 0,
                end: 0,
            });
            Some((sec.lma, sec.lma + size, sec.name.as_str(), span))
        })
        .collect();
    for i in 0..ranges.len() {
        for j in (i + 1)..ranges.len() {
            let (a_lo, a_hi, a_name, a_span) = ranges[i];
            let (b_lo, b_hi, b_name, _) = ranges[j];
            // Half-open ranges intersect iff each starts before the other ends.
            if a_lo < b_hi && b_lo < a_hi {
                return Some(Diagnostic {
                    level: Level::Error,
                    message: format!(
                        "sections `{a_name}` [{a_lo:#X}, {a_hi:#X}) and `{b_name}` [{b_lo:#X}, {b_hi:#X}) overlap in the image (colliding pins)"
                    ),
                    primary: a_span,
                });
            }
        }
    }
    None
}

/// Post-fixpoint run-overrun check (replaces the M1.C T6b categorical
/// `Org`+relaxable refusal). An `Org` is a position BARRIER: the run of
/// fragments before it must fit within the barrier's org target. Once the
/// relaxation fixpoint has converged (rungs final), replay the section's
/// fragments with a write cursor — every `Org` opens a new run anchored at its
/// `target`, and the run BEFORE it must not have advanced the cursor PAST that
/// target. If it did (a relaxable grew the run's content across the barrier),
/// return a loud error naming the section, the org target, the run's actual
/// extent, and the overrun in bytes — the precise replacement for the old
/// blanket refusal. AS/asl reject the same source (an `org` seeking backward
/// into content that has grown to overrun it) rather than silently overlap;
/// this matches that spirit.
///
/// Only a FORWARD org can be overrun by preceding growth. "Forward" is judged
/// at the BASELINE (rung-0) layout: an org whose target sits at-or-ahead of the
/// run's baseline extent is a forward barrier the run must fit inside, so if
/// growth pushes the current extent past it that is an overrun. An org whose
/// target sits BEHIND the baseline extent is the backward overwrite idiom
/// (`org Hdr / dc.b n / org End`) — it deliberately seeks into already-written
/// content, never an overrun (its overwrite semantics are `image_bytes`'
/// concern, unchanged). Tracking both cursors keeps the two apart.
fn run_overrun_diag(sec: &Section, rungs: &[usize]) -> Option<Diagnostic> {
    let mut cursor: u32 = 0;
    let mut baseline: u32 = 0;
    for (fi, frag) in sec.fragments.iter().enumerate() {
        match frag {
            Fragment::Org { target, span, .. } => {
                // Forward barrier (target at/ahead of baseline extent) that the
                // grown run overran: current content extends past the org target.
                if baseline <= *target && cursor > *target {
                    return Some(Diagnostic {
                        level: Level::Error,
                        message: format!(
                            "section `{}`: content before `org {:#X}` grew to {:#X} — it overruns the org target by {} bytes (a relaxable instruction widened past the org barrier)",
                            sec.name,
                            target,
                            cursor,
                            cursor - target
                        ),
                        primary: *span,
                    });
                }
                cursor = *target;
                baseline = *target;
            }
            other => {
                cursor += frag_len(other, rungs[fi]);
                baseline += frag_len(other, 0);
            }
        }
    }
    None
}

/// Breakpoints mapping an all-rung-0 (baseline) offset to the growth delta at
/// that fragment boundary under the current rungs. `rungs[fi]` is the chosen
/// rung of fragment `fi` (meaningful for the relaxables; ignored otherwise —
/// fixed fragments have identical length at every rung).
///
/// # Org-awareness: runs and barriers
///
/// An `Org` fragment is a POSITION BARRIER, not a run of bytes. Content after
/// an `Org` is anchored to the org target — its authored (rung-0) offset is
/// already measured from that target (the front-end resolves org targets and
/// post-org label offsets against the same VMA cursor the org repositioned),
/// and its current offset must be too. So at each `Org` BOTH cursors seek to
/// `target`, which RESETS the running delta to `target − target = 0` at the
/// barrier: growth of a relaxable WITHIN a run shifts only the fragments after
/// it in that SAME run (up to the next `Org`), never content past the barrier.
///
/// This is why a section can freely mix an `Org` back-patch with a relaxable:
/// the delta is a per-run step function, reset to the org-anchored baseline at
/// every barrier, so a growing relaxable before an `Org` never mis-shifts the
/// org-pinned content after it. `shift_offset` reads the delta for the run
/// containing an offset (last matching breakpoint wins — which, for a BACKWARD
/// org that revisits an authored-offset range, mirrors `image_bytes`'
/// later-write-wins overwrite semantics).
fn shift_breakpoints(sec: &Section, rungs: &[usize]) -> Vec<(u32, i64)> {
    let mut cur: u32 = 0;
    let mut orig: u32 = 0;
    let mut bps = vec![(0u32, 0i64)];
    for (fi, frag) in sec.fragments.iter().enumerate() {
        if let Fragment::Org { target, .. } = frag {
            // Barrier: both cursors seek to the org target. The current and
            // baseline anchors coincide there, so the delta resets to 0 — the
            // run after this org is pinned to `target` in both layouts.
            cur = *target;
            orig = *target;
        } else {
            cur += frag_len(frag, rungs[fi]);
            orig += frag_len(frag, 0); // baseline: every width-variable fragment at rung 0
        }
        bps.push((orig, cur as i64 - orig as i64));
    }
    bps
}

/// Map an all-rung-0 label offset to its current-layout offset. The last
/// breakpoint at-or-before `orig_off` supplies the run's delta (for a backward
/// `Org` that revisits an authored range, the later run wins — mirroring
/// `image_bytes`' overwrite order).
fn shift_offset(bps: &[(u32, i64)], orig_off: u32) -> u32 {
    let mut d = 0i64;
    for &(bo, bd) in bps {
        if bo <= orig_off {
            d = bd;
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
///
/// Org-aware like `shift_breakpoints`: a preceding `Org` SEEKS the baseline
/// cursor to its target (a barrier, not a run of bytes), so `fi`'s baseline
/// offset is measured from the last org anchor at or before it — the same
/// per-run anchoring `shift_offset` then maps into the current layout.
// TODO(perf): O(fi) prefix walk per ladder per pass; once ladders get dense, thread a
// running accumulator through the selection loop + convergence sweep instead.
fn frag_start_vma(sec: &Section, bps: &[(u32, i64)], origin: u32, fi: usize) -> u32 {
    let mut baseline_off: u32 = 0;
    for prev in &sec.fragments[..fi] {
        if let Fragment::Org { target, .. } = prev {
            baseline_off = *target;
        } else {
            baseline_off += frag_len(prev, 0);
        }
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

    // Guard: `Org` is a POSITION BARRIER, and relaxation is now Org-aware — a
    // relaxable BEFORE an `Org` shifts only the fragments after it in its OWN
    // run (up to the barrier), never the org-pinned content past it (see
    // `shift_breakpoints`'s run/barrier doc). So a section may freely mix an
    // `Org` back-patch with `JmpJsrSym`/`RelaxAbsSym`/`RelaxLadder`; the M1.C
    // T6b categorical refusal is REPLACED by a precise post-fixpoint overrun
    // check (`run_overrun_diag`), which fires only when a run actually grows
    // past its barrier's org target (a real, named overlap).
    //
    // ONE hazard survives: an `Org` mixed with a `Reserve` (`ds`). `IrBuilder`
    // counts `Reserve` toward the cursor/extent the front-end resolves an `org`
    // target against (VMA space), but `Section::image_bytes` and `link()`'s
    // fixup walk treat `Reserve` as zero image bytes and apply `Org.target` as
    // an IMAGE-byte offset — so a `Reserve` before an `org` back-patch diverges
    // the resolved VMA offset from the physical image offset and the patch lands
    // on the wrong byte. Latent today (parallax sections are pure `dc.b`, no
    // `ds`), but fail loudly rather than mislink silently.
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

    // The joint placement⇄relaxation fixpoint (R7p.3) operates on a MUTABLE copy:
    // each outer pass re-derives every chained section's lma from the current
    // rungs (the placement pass, R7p.2), so `placed`'s lmas are truth-telling
    // final addresses. `rungs` persists across passes and is grow-only (the
    // existing ladder invariant); placement is a deterministic function of
    // rungs + pins, so once rungs stabilize one final placement is fixed.
    let mut placed: Vec<Section> = sections.to_vec();

    // Provably-sufficient pass cap: each pass that reports `grew` advances at
    // least one relaxable fragment's rung by ≥1 (a length change), and each
    // fragment can advance at most `rung_count − 1` times (grow-only). So at most
    // `Σ(rung_count − 1)` passes can grow — after that, one placement pass settles
    // the chained lmas from the stable rungs and the next pass observes neither a
    // rung growth nor an lma move → convergence. (JmpJsrSym/RelaxAbsSym contribute
    // 1 each = the old total-flips bound; a 4-rung ladder contributes 3.) The
    // `.max(64)` is the honesty backstop the ruling asks for; the non-convergence
    // Err below is unreachable-in-practice by the grow-only/deterministic argument.
    let total_flips: usize = sections
        .iter()
        .flat_map(|s| s.fragments.iter())
        .map(|f| rung_count(f) - 1)
        .sum();
    let cap = (total_flips + 2).max(64);

    // Span of a fragment that grew on the most recent pass, for the backstop diag.
    let mut last_grown_span: Option<Span> = None;

    for _ in 0..cap {
        // (0) Placement pass (R7p.2): re-derive every chained section's lma from
        // the current rungs. A moved lma moves that section's labels (its
        // `vma_origin` shifts when `vma_base` is None), which the symbol-table
        // rebuild in (a) picks up — the intended truth-telling per D7.4.
        let moved = place_pass(&mut placed, &rungs);

        // (a) Build the symbol table with label VMAs shifted under current rungs.
        let mut syms = stubs.clone();
        for (si, sec) in placed.iter().enumerate() {
            let origin = sec.vma_origin();
            let bps = shift_breakpoints(sec, &rungs[si]);
            for label in &sec.labels {
                syms.define(&label.name, SymbolValue::Int((origin + shift_offset(&bps, label.offset)) as i64));
            }
        }

        // (a2) Task 5 (R-T0.6): overlay a best-effort `equ` fold on top of this
        // pass's label table, so an ABS-ONLY relaxable fragment's `target`
        // (`RelaxAbsSym`/`JmpJsrSym` — NOT `RelaxLadder`, whose pc-relative
        // rungs must never treat an absolute equ value as a branch destination;
        // see the ladder arm) may name an `equ` (directly, an equ-on-equ chain,
        // or an equ derived from a label) — not just a bare layout label.
        // `equ_lookup` is used ONLY by rung selection below; the FINAL,
        // authoritative equ fold (with its loud unresolved/cycle diagnostic)
        // still runs once at convergence (c4).
        let equ_overlay = equ_lookup_overlay(&placed, &syms);
        let equ_lookup = |n: &str| equ_overlay.get(n).copied().or_else(|| syms.resolve(n, None));

        // (b) Re-select each relaxable fragment's rung from its resolved target
        // (grow-only). `grew` is set ONLY when the selection changes the
        // fragment's byte LENGTH — a same-length rung move (e.g. bra.w → jmp
        // abs.w, both 4 bytes) is recorded but needs no relayout.
        let mut grew = false;
        for (si, sec) in placed.iter().enumerate() {
            let origin = sec.vma_origin();
            let bps = shift_breakpoints(sec, &rungs[si]);
            for fi in 0..sec.fragments.len() {
                let frag = &sec.fragments[fi];
                match frag {
                    Fragment::JmpJsrSym { target, span, .. } => {
                        // GLOBAL scope only (scope None): a bare `jmp .local` to a
                        // dotted local would not resolve here. The front-end must
                        // qualify such targets to fully-dotted names first.
                        let v = match target.fold(&equ_lookup) {
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
                        let v = match target.fold(&equ_lookup) {
                            Fold::Value(v) => v,
                            Fold::Poison => {
                                return Err(vec![unresolved_relax_abs_sym_diag(
                                    target, &placed, &sec.name, &syms, &equ_overlay, *span,
                                )]);
                            }
                        };
                        if asl_width_rule(v, dash_a) == AbsWidth::L && rungs[si][fi] == 0 {
                            rungs[si][fi] = 1;
                            last_grown_span = Some(*span);
                            grew = true;
                        }
                    }
                    Fragment::RelaxLadder { candidates, target, span } => {
                        // LABELS ONLY — deliberately NOT `equ_lookup` (review
                        // ruling on the Task 5 commit): a ladder's pc-relative
                        // rungs (bra.s/bra.w) would treat an equ's ABSOLUTE
                        // value as a branch destination and, when near, silently
                        // encode a pc-relative displacement to it (`jbra R` with
                        // `equ R = $420` near the section → `60 1E`). Branch
                        // targets must be spelled as labels; jmp/jsr (abs-only
                        // rungs, safe by construction) keep the equ overlay.
                        let v = match target.fold(&|n| syms.resolve(n, None)) {
                            Fold::Value(v) => v,
                            Fold::Poison => {
                                return Err(vec![unresolved_ladder_target_diag(
                                    target, &placed, &sec.name, &syms, *span,
                                )]);
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

        // Converged only when NEITHER a rung grew NOR an lma moved this pass
        // (R7p.3): a placement move can change a cross-section branch distance, so
        // we must re-run selection at the new addresses before lowering.
        if !grew && !moved {
            // (c) Convergence sweep: every RelaxLadder's chosen candidate must
            // actually reach the target. A ladder that maxed at its last rung and
            // still cannot reach (tonight: a conditional/unsized branch whose last
            // rung is PcRelDisp16 — the only ladder shape that can exhaust) is a
            // hard error naming the signed distance. Collect ALL such errors.
            let mut errs: Vec<Diagnostic> = Vec::new();
            for (si, sec) in placed.iter().enumerate() {
                let origin = sec.vma_origin();
                let bps = shift_breakpoints(sec, &rungs[si]);
                for fi in 0..sec.fragments.len() {
                    if let Fragment::RelaxLadder { candidates, target, span } = &sec.fragments[fi] {
                        // LABELS ONLY, matching the selection arm in (b): ladder
                        // targets never resolve through the equ overlay (review
                        // ruling — see the (b) arm's comment). An unresolvable
                        // target already errored loudly in (b) this same pass.
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

            // (c1b) Run-overrun check: `Org` is a position barrier, and a
            // relaxable before an `Org` may only grow WITHIN its run (up to the
            // barrier). If a run's content grew past its org target, that is a
            // loud error naming the section + overrun — the precise replacement
            // for the M1.C T6b categorical `Org`+relaxable refusal. Runs FIRST
            // (a run overrun makes the section's byte positions meaningless).
            for (si, sec) in placed.iter().enumerate() {
                if let Some(diag) = run_overrun_diag(sec, &rungs[si]) {
                    return Err(vec![diag]);
                }
            }

            // (c2) Overlap check (R7p.4): the joint fixpoint has converged, so
            // every section's placed `[lma, lma + final_size)` range is final.
            // Any two NON-EMPTY ranges that intersect are a loud link error naming
            // BOTH sections and both extents (hex). Chained sections cannot overlap
            // by construction (their cursor advances past each predecessor); this
            // catches colliding PINS on any path — single-file, multi-module, or
            // harness — since they all funnel through `resolve_layout`.
            if let Some(diag) = overlap_diag(&placed, &rungs) {
                return Err(vec![diag]);
            }

            // (c3) Bank no-straddle check (R7m.2 / D7.5): every `bank:` section
            // (pinned included) must fit within a single N-window at its FINAL
            // placement. Over-bank content and straddling pins are loud errors
            // here — same diagnostic channel as the overlap check, discharged
            // structurally rather than via a synthesized LinkAssert row.
            if let Some(diag) = bank_diag(&placed, &rungs) {
                return Err(vec![diag]);
            }

            // (c4) equ fold (R-T0.3): the placement⇄relaxation fixpoint has
            // converged and every label's VMA in `syms` is FINAL, so an `equ`
            // whose value references a label (`bankid(L)`/`winptr(L)`/`L + 4`)
            // now folds to a stable integer. Fold all sections' `equ_syms`
            // multi-pass (an equ may reference another equ) against `syms`; the
            // result rewrites each equ's `expr` to `Expr::Int(v)` in the lowered
            // sections below, so `link()` defines them as concrete `SymbolValue`s
            // BEFORE it applies fixups (a cross-section fixup can then target an
            // equ). An unresolvable equ (after the pass cap) or a cycle is a loud
            // link error naming the symbol and its first unresolved dependency.
            let folded = fold_equ_syms(&placed, &syms)?;

            // (d) Converged & every ladder reaches: lower fragments + shift labels.
            let out = placed
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
                        // Provenance is carried through the relax rebuild verbatim
                        // (R7p.1): relaxation only lowers fragments/shifts labels;
                        // it never re-places a section. `bank` (R7m.1) is the same
                        // kind of provenance — carried verbatim for Task 2's
                        // placement seam to read.
                        placement: sec.placement,
                        reserved_span: sec.reserved_span,
                        group: sec.group.clone(),
                        bank: sec.bank,
                        // R-T0.3: each equ's `expr` is REPLACED by its folded
                        // integer (`Expr::Int(v)`), computed above against the
                        // final label VMAs. `link()` re-folds these (now trivial)
                        // and defines them before fixups.
                        equ_syms: folded[si].clone(),
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

/// Bounded multi-pass cap for the `equ` fold (R-T0.3): an `equ` may reference
/// ANOTHER equ (`equ A = B + 1; equ B = bankid(L)`), so one pass over the equ
/// list is not enough — a later-declared dependency resolves an earlier
/// dependent on the next pass. Each pass that makes progress resolves at least
/// one more equ, so a chain of N equs settles in ≤ N passes; the cap bounds a
/// cycle (`equ X = Y; equ Y = X`) to a loud error instead of a spin. 8 is well
/// above any realistic hand-authored equ chain depth.
const MAX_EQU_PASSES: usize = 8;

/// Best-effort `equ` fold (Task 5 / R-T0.6) against the CURRENT pass's label
/// table `syms` — used by the relaxation fixpoint's rung-selection step (b) so
/// a [`Fragment::RelaxAbsSym`]/`JmpJsrSym` `target` naming an `equ` (not just
/// a layout label) can resolve there too. Deliberately NOT `RelaxLadder`
/// (review ruling — see the ladder selection arm): those two are abs-only, so
/// any equ value is a clean absolute; a ladder's pc-relative rungs would
/// silently branch pc-relative to a near absolute equ value. Unlike
/// [`fold_equ_syms`] (the FINAL post-convergence fold, which errors loudly on
/// an unresolved equ), this is silently partial: an equ that can't fold YET —
/// because it depends on another equ not yet resolved this pass, or on a
/// symbol that genuinely never resolves — is simply absent from the returned
/// map, and the CALLER'S existing `Fold::Poison` handling at the target-fold
/// site reports it (unchanged diagnostic shape, now able to name an equ).
/// Since every label name is already present in `syms` from pass 1 onward
/// (only its VALUE shifts as placement iterates), an equ chain resting on
/// labels+equs fully resolves within one call here — no cross-pass equ state
/// needs to persist between OUTER relaxation passes.
fn equ_lookup_overlay(placed: &[Section], syms: &SymbolTable) -> std::collections::HashMap<String, i64> {
    use std::collections::HashMap;
    let mut folded_vals: HashMap<String, i64> = HashMap::new();
    for _ in 0..MAX_EQU_PASSES {
        let mut progressed = false;
        for sec in placed {
            for eq in &sec.equ_syms {
                if folded_vals.contains_key(&eq.name) {
                    continue;
                }
                let lookup =
                    |n: &str| folded_vals.get(n).copied().or_else(|| syms.resolve(n, None));
                if let Fold::Value(v) = eq.expr.fold(&lookup) {
                    folded_vals.insert(eq.name.clone(), v);
                    progressed = true;
                }
            }
        }
        if !progressed {
            break;
        }
    }
    folded_vals
}

/// Fold every section's `equ_syms` against the FINAL post-placement symbol table
/// `syms` (R-T0.3), returning per-section `Vec<EquSym>` whose every `expr` is a
/// concrete `Expr::Int(v)`. An equ may reference a label (its VMA is final in
/// `syms`) OR another equ — so this iterates up to [`MAX_EQU_PASSES`], seeding a
/// scratch lookup that overlays already-folded equ values on top of `syms`.
///
/// Unresolvable after the cap (an equ naming a symbol that never resolves, or a
/// cycle) is a loud `Error` naming the symbol and its FIRST unresolved
/// dependency — never a silent poison. Duplicate NAMES are not this function's
/// concern: `link()`'s Pass-1 `defined_here` map (which also sees labels) is the
/// single dup-symbol channel, so an equ colliding with a label or another equ is
/// caught there.
fn fold_equ_syms(
    placed: &[Section],
    syms: &SymbolTable,
) -> Result<Vec<Vec<sigil_ir::EquSym>>, Vec<Diagnostic>> {
    use std::collections::HashMap;

    // Fast path: no equs anywhere → return the (empty) per-section lists verbatim.
    let total: usize = placed.iter().map(|s| s.equ_syms.len()).sum();
    if total == 0 {
        return Ok(placed.iter().map(|s| s.equ_syms.clone()).collect());
    }

    // `folded_vals[name] = value` for every equ resolved so far, overlaid on `syms`.
    let mut folded_vals: HashMap<String, i64> = HashMap::new();

    for _ in 0..MAX_EQU_PASSES {
        let mut progressed = false;
        let mut all_done = true;
        for sec in placed {
            for eq in &sec.equ_syms {
                if folded_vals.contains_key(&eq.name) {
                    continue;
                }
                // Lookup overlays already-folded equ values on the label table.
                let lookup =
                    |n: &str| folded_vals.get(n).copied().or_else(|| syms.resolve(n, None));
                match eq.expr.fold(&lookup) {
                    Fold::Value(v) => {
                        folded_vals.insert(eq.name.clone(), v);
                        progressed = true;
                    }
                    Fold::Poison => all_done = false,
                }
            }
        }
        if all_done {
            break;
        }
        if !progressed {
            // No equ resolved this pass yet some remain unresolved → a cycle or a
            // dangling dependency. Name the first still-unresolved equ and its
            // first unresolved dependency (a symbol the current table cannot fold).
            return Err(vec![unresolved_equ_diag(placed, syms, &folded_vals)]);
        }
    }

    // Any equ still unresolved after the cap is an error (same diagnostic shape).
    if placed.iter().any(|s| s.equ_syms.iter().any(|e| !folded_vals.contains_key(&e.name))) {
        return Err(vec![unresolved_equ_diag(placed, syms, &folded_vals)]);
    }

    // Rewrite every equ's expr to its folded integer.
    Ok(placed
        .iter()
        .map(|s| {
            s.equ_syms
                .iter()
                .map(|e| sigil_ir::EquSym {
                    name: e.name.clone(),
                    // Every equ is resolved here (checked above), so the map is
                    // total; fall back to the original expr defensively.
                    expr: folded_vals
                        .get(&e.name)
                        .map(|v| Expr::Int(*v))
                        .unwrap_or_else(|| e.expr.clone()),
                    span: e.span,
                })
                .collect()
        })
        .collect())
}

/// Build the loud "unresolvable equ" diagnostic (R-T0.3) for the FIRST equate
/// that could not be folded: name the equate and the first symbol its `expr`
/// references that neither the label table nor a resolved equ can supply (its
/// first unresolved dependency — which, for `equ X = Y; equ Y = X`, is the other
/// arm of the cycle). Points at the equate's own span.
fn unresolved_equ_diag(
    placed: &[Section],
    syms: &SymbolTable,
    folded_vals: &std::collections::HashMap<String, i64>,
) -> Diagnostic {
    for sec in placed {
        for eq in &sec.equ_syms {
            if folded_vals.contains_key(&eq.name) {
                continue;
            }
            let dep = first_unresolved_sym(&eq.expr, syms, folded_vals)
                .unwrap_or_else(|| "<unknown>".to_string());
            return Diagnostic {
                level: Level::Error,
                message: format!(
                    "unresolvable equ `{}`: its first unresolved dependency `{}` is not defined \
                     (an undefined symbol, or an equ cycle)",
                    eq.name, dep
                ),
                primary: eq.span,
            };
        }
    }
    // Unreachable: only called when some equ is unresolved.
    Diagnostic {
        level: Level::Error,
        message: "internal: unresolved equ with no unresolved equ found".to_string(),
        primary: Span { source: sigil_span::SourceId(0), start: 0, end: 0 },
    }
}

/// The first symbol in `expr` that neither `folded_vals` (already-folded equs)
/// nor `syms` (final label VMAs) can resolve — the concrete dependency to name
/// in the unresolvable-equ diagnostic.
fn first_unresolved_sym(
    expr: &Expr,
    syms: &SymbolTable,
    folded_vals: &std::collections::HashMap<String, i64>,
) -> Option<String> {
    match expr {
        Expr::Int(_) => None,
        Expr::Sym(name) => {
            if folded_vals.contains_key(name) || syms.resolve(name, None).is_some() {
                None
            } else {
                Some(name.clone())
            }
        }
        Expr::Unary { operand, .. } => first_unresolved_sym(operand, syms, folded_vals),
        Expr::Binary { lhs, rhs, .. } => first_unresolved_sym(lhs, syms, folded_vals)
            .or_else(|| first_unresolved_sym(rhs, syms, folded_vals)),
    }
}

/// The `unresolved symbolic absolute operand` diagnostic (Task 5 / R-T0.6),
/// naming the actual unresolved SYMBOL rather than just the section. `target`
/// folded to [`Fold::Poison`] against `equ_lookup` (`equ_overlay` ∪ `syms`,
/// see `equ_lookup_overlay`), so walk it to find the first leaf name neither
/// table supplies, then choose wording by WHY it's unresolved:
///
/// - The name is not an `equ` anywhere in `placed` (never even attempted a
///   fold) → the Item-C cross-seam-standalone wording
///   (`check_link_asserts`'s "references symbol(s) ... not defined in this
///   link"), the common case of compiling a cross-seam module standalone.
/// - The name IS an `equ` somewhere but never resolved within
///   `equ_lookup_overlay`'s bounded passes → a cycle or a dangling
///   equ-on-equ dependency; phrased distinctly (not duplicating
///   `unresolved_equ_diag`'s wording, but naming the same shape of cause) so
///   a reader doesn't mistake a cycle for a plain missing symbol.
fn unresolved_relax_abs_sym_diag(
    target: &Expr,
    placed: &[Section],
    section: &str,
    syms: &SymbolTable,
    equ_overlay: &std::collections::HashMap<String, i64>,
    span: Span,
) -> Diagnostic {
    let name = first_unresolved_sym(target, syms, equ_overlay)
        .unwrap_or_else(|| "<unknown>".to_string());
    let is_equ_elsewhere = placed.iter().any(|s| s.equ_syms.iter().any(|e| e.name == name));
    let message = if is_equ_elsewhere {
        format!(
            "unresolved symbolic absolute operand in section {section}: equ `{name}` never \
             resolved (an equ cycle, or a dependency that never resolves)"
        )
    } else {
        format!(
            "unresolved symbolic absolute operand in section {section} references symbol \
             `{name}` not defined in this link — expected when compiling a cross-seam module \
             standalone; supply the map/harness composition that defines it"
        )
    };
    Diagnostic { level: Level::Error, message, primary: span }
}

/// The `unresolved branch/ladder target` diagnostic. Ladder targets resolve
/// against LABELS only (review ruling on the Task 5 commit — see the
/// `RelaxLadder` selection arm), so when the unresolved name IS an `equ`
/// defined in this link, the refusal is deliberate and the message says so
/// with the steer; otherwise it's a plain missing symbol.
fn unresolved_ladder_target_diag(
    target: &Expr,
    placed: &[Section],
    section: &str,
    syms: &SymbolTable,
    span: Span,
) -> Diagnostic {
    // Labels-only view (empty equ overlay) — the same view the ladder folds.
    let name = first_unresolved_sym(target, syms, &std::collections::HashMap::new())
        .unwrap_or_else(|| "<unknown>".to_string());
    let is_equ = placed.iter().any(|s| s.equ_syms.iter().any(|e| e.name == name));
    let message = if is_equ {
        format!(
            "unresolved branch/ladder target in section {section}: `{name}` is an equ — \
             branch targets must be labels; use jmp/jsr for an absolute target"
        )
    } else {
        format!("unresolved branch/ladder target in section {section} (symbol `{name}`)")
    };
    Diagnostic { level: Level::Error, message, primary: span }
}

/// The `[branch.out-of-reach]` diagnostic for a ladder that maxed at its last
/// rung and still cannot reach. The message is DERIVED from the last candidate's
/// fixup kind: today the only ladder shape that can exhaust is an unsized
/// branch (`Bcc`/`bra`/`bsr`) whose last rung is `PcRelDisp16` (no far form), so
/// we name the signed distance and the ±32766 word range, then steer BOTH
/// mnemonic classes (Core cannot see which one built the ladder): conditionals
/// have no far form (jbcc deferred, D2.18); unconditional bra/bsr should use
/// jbra/jbsr. Any other exhausting kind would be a new far form and should
/// extend this message.
fn out_of_reach_diag(cand: &RelaxCandidate, frag_start: u32, target: i64, span: Span, section: &str) -> Diagnostic {
    let site_vma = frag_start as i64 + cand.fixup.offset as i64;
    let msg = match cand.fixup.kind {
        FixupKind::PcRelDisp16 => {
            // The signed distance N is measured from the disp word's own VMA.
            // Core cannot see the MNEMONIC behind the ladder (only its fixup
            // kinds), so the steer covers both exhausting shapes honestly: a
            // conditional has no far form at all, while an unconditional
            // bra/bsr should switch to jbra/jbsr (which fall back to jmp/jsr).
            let disp = target - site_vma;
            format!(
                "[branch.out-of-reach] branch target in section {section} is {disp} bytes away (max \u{00B1}32766); a conditional branch has no far form (jbcc trampolines are deferred, D2.18) — for an unconditional bra/bsr, use jbra/jbsr instead"
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
    fn resolve_layout_allows_org_and_jmpjsr_in_the_same_section() {
        // The real Aeon shape: the object-bank section (opened by `org $10000`)
        // contains BOTH a bare `jsr` (player/object code) AND the parallax
        // `parallax_section_end` back-patch (`org` / dc.b / `org`) later in the
        // SAME still-open section. This was categorically refused by the M1.C
        // T6b guard; now `Org` is a barrier and relaxation is run-aware, so a
        // NON-overrunning mix resolves. `Sub` is low → the jsr stays abs.w, no
        // growth, and the org target (0x10) is well ahead of the tiny run.
        let mut stubs = SymbolTable::new();
        stubs.define("Sub", SymbolValue::Int(0x1200));
        let sec = Section {
            name: "objbank".into(),
            cpu: Cpu::M68000,
            vma_base: Some(0x10000),
            lma: 0x10000,
            labels: vec![Label { name: "Post".into(), offset: 0x11 }],
            fragments: vec![
                Fragment::JmpJsrSym { is_jsr: true, target: Expr::Sym("Sub".into()), span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0], fixups: vec![], span: sp() }),
                Fragment::Org { target: 0x10, fill: 0x00, span: sp() },
                Fragment::Data(DataFragment { bytes: vec![1], fixups: vec![], span: sp() }),
            ],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let out = resolve_layout(&[sec], &stubs, true).unwrap();
        // The jsr stayed abs.w (4 bytes), Post is pinned to the org target 0x10.
        assert_eq!(out[0].labels.iter().find(|l| l.name == "Post").unwrap().offset, 0x11);
    }

    #[test]
    fn resolve_layout_allows_org_and_ladder_in_the_same_section() {
        // A RelaxLadder alongside an Org is likewise no longer refused — a
        // near, non-overrunning `jbra` before a forward org resolves. `L` is a
        // label in the SAME run (offset 1), so the ladder picks bra.s (2 bytes)
        // and the run never reaches the org barrier at 0x10.
        let sec = Section {
            name: "code".into(),
            cpu: Cpu::M68000,
            vma_base: Some(0x1000),
            lma: 0x1000,
            labels: vec![
                Label { name: "L".into(), offset: 1 },
                Label { name: "After".into(), offset: 0x11 },
            ],
            fragments: vec![
                jbra("L"),
                Fragment::Org { target: 0x10, fill: 0x00, span: sp() },
                Fragment::Data(DataFragment { bytes: vec![1], fixups: vec![], span: sp() }),
            ],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let out = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap();
        assert_eq!(out[0].labels.iter().find(|l| l.name == "After").unwrap().offset, 0x11);
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
        // naming the signed distance and steering BOTH mnemonic classes (Core
        // cannot see which built the ladder): jbcc-deferred for conditionals,
        // jbra/jbsr for unconditional bra/bsr.
        let mut stubs = SymbolTable::new();
        stubs.define("VeryFar", SymbolValue::Int(0x20_0000)); // >> i16 from site ~0
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![bne_ladder("VeryFar")],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let err = resolve_layout(&[sec], &stubs, true).unwrap_err();
        assert!(
            err.iter().any(|d| d.message.contains("[branch.out-of-reach]")
                && d.message.contains("bytes away")
                && d.message.contains("jbcc trampolines are deferred, D2.18")
                && d.message.contains("use jbra/jbsr instead")),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let err = resolve_layout(&[sec], &stubs, true).unwrap_err();
        assert!(
            err.iter().any(|d| d.message.contains("[branch.out-of-reach]") && d.message.contains('-')),
            "expected a negative distance, got: {:?}",
            err
        );
    }

    #[test]
    fn reserve_only_section_does_not_collide_at_shared_lma() {
        // Aeon's phased `$FFFF….` RAM: a section whose VMA is in RAM but whose
        // LMA anchors at the physical counter (here 0, colliding with the ROM
        // reset). It is ALL `Reserve` (`ds`), so it places zero image bytes and
        // must NOT trip the overlap check against a real ROM section at LMA 0.
        // Regression: the overlap check used `final_size` (VMA span, reserve-
        // inclusive) and spuriously flagged this as a colliding pin.
        let ram = Section {
            name: "ram".into(),
            cpu: Cpu::M68000,
            vma_base: Some(0xFFFF_0000),
            lma: 0,
            labels: vec![],
            fragments: vec![Fragment::Reserve { count: 0x1000, span: sp() }],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0x1000,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let rom = Section {
            name: "reset".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
                fixups: vec![],
                span: sp(),
            })],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 4,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        // Must resolve cleanly — no "colliding pins" error.
        let out = resolve_layout(&[ram, rom], &SymbolTable::new(), true)
            .expect("reserve-only RAM section must not collide with the ROM at LMA 0");
        assert_eq!(out.len(), 2);
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
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

    // ---- equ fold post-placement (R-T0.3) ------------------------------------

    use sigil_ir::expr::BinOp;

    /// The `bankid` residual tree the evaluator builds: `(Sym & $7F8000) >> 15`.
    fn bankid_tree(name: &str) -> Expr {
        Expr::Binary {
            op: BinOp::Shr,
            lhs: Box::new(Expr::Binary {
                op: BinOp::And,
                lhs: Box::new(Expr::Sym(name.into())),
                rhs: Box::new(Expr::Int(0x7F_8000)),
            }),
            rhs: Box::new(Expr::Int(15)),
        }
    }

    /// The `winptr` residual tree (post R-T0.5): `(Sym & $7FFF) | $8000`.
    fn winptr_tree(name: &str) -> Expr {
        Expr::Binary {
            op: BinOp::Or,
            lhs: Box::new(Expr::Binary {
                op: BinOp::And,
                lhs: Box::new(Expr::Sym(name.into())),
                rhs: Box::new(Expr::Int(0x7FFF)),
            }),
            rhs: Box::new(Expr::Int(0x8000)),
        }
    }

    fn equ(name: &str, expr: Expr) -> sigil_ir::EquSym {
        sigil_ir::EquSym { name: name.into(), expr, span: sp() }
    }

    /// A section pinned at `lma` (no vma phase, so labels follow the LMA per
    /// R7p.5) with a label `L` at offset 0 and the given equ_syms.
    fn equ_section(lma: u32, label: &str, equ_syms: Vec<sigil_ir::EquSym>) -> Section {
        Section {
            name: "s".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma,
            labels: vec![Label { name: label.into(), offset: 0 }],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x11, 0x22, 0x33, 0x44, 0x55, 0x66],
                fixups: vec![],
                span: sp(),
            })],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms,
        }
    }

    #[test]
    fn equ_bankid_folds_to_symbol_at_link() {
        // A section PINNED at $58000 with label `L` @ $58000 (VMA == LMA, R7p.5).
        // Three equs: B = bankid(L), P = winptr(L), N = 6 (comptime int).
        //   bankid($58000) = ($58000 & $7F8000) >> 15 = $58000 >> 15 = 0xB.
        //   winptr($58000) = ($58000 & $7FFF) | $8000 = 0 | $8000 = $8000.
        let sec = equ_section(
            0x5_8000,
            "L",
            vec![
                equ("B", bankid_tree("L")),
                equ("P", winptr_tree("L")),
                equ("N", Expr::Int(6)),
            ],
        );
        let out = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap();
        // The folded equ_syms carry concrete Int exprs.
        let e = &out[0].equ_syms;
        assert_eq!(e.iter().find(|s| s.name == "B").unwrap().expr, Expr::Int(0xB));
        assert_eq!(e.iter().find(|s| s.name == "P").unwrap().expr, Expr::Int(0x8000));
        assert_eq!(e.iter().find(|s| s.name == "N").unwrap().expr, Expr::Int(6));
        // And link() defines them (no fixups here — the folded-expr assertion above
        // is the primary proof; link succeeding proves Pass-1b accepts them).
        let linked = crate::link(&out, &SymbolTable::new()).unwrap();
        assert_eq!(&linked.section("s").unwrap().bytes[..6], &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
    }

    #[test]
    fn equ_chain_folds_and_cycle_is_loud() {
        // Chain: A = B + 1 ; B = bankid(L). B folds pass 1 ($58000 → 0xB); A folds
        // pass 2 (0xB + 1 = 0xC). Multi-pass proof.
        let a_expr = Expr::Binary {
            op: BinOp::Add,
            lhs: Box::new(Expr::Sym("B".into())),
            rhs: Box::new(Expr::Int(1)),
        };
        let sec = equ_section(0x5_8000, "L", vec![equ("A", a_expr), equ("B", bankid_tree("L"))]);
        let out = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap();
        let e = &out[0].equ_syms;
        assert_eq!(e.iter().find(|s| s.name == "B").unwrap().expr, Expr::Int(0xB));
        assert_eq!(e.iter().find(|s| s.name == "A").unwrap().expr, Expr::Int(0xC));

        // Cycle: X = Y ; Y = X → unresolvable, loud error naming the cycle.
        let cyc = equ_section(
            0x5_8000,
            "L",
            vec![equ("X", Expr::Sym("Y".into())), equ("Y", Expr::Sym("X".into()))],
        );
        let err = resolve_layout(&[cyc], &SymbolTable::new(), true).unwrap_err();
        assert!(
            err.iter().any(|d| d.message.contains("unresolvable equ")
                && (d.message.contains('X') || d.message.contains('Y'))),
            "expected a loud equ-cycle diagnostic naming X/Y, got: {err:?}"
        );
    }

    #[test]
    fn equ_referencing_undefined_symbol_is_loud() {
        // An equ referencing a symbol no label/equ/stub defines → loud error
        // naming the equ and its first unresolved dependency.
        let sec = equ_section(0x5_8000, "L", vec![equ("Q", Expr::Sym("Nope".into()))]);
        let err = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap_err();
        assert!(
            err.iter().any(|d| d.message.contains("unresolvable equ `Q`")
                && d.message.contains("Nope")),
            "got: {err:?}"
        );
    }

    #[test]
    fn cross_section_fixup_targets_equ_symbol() {
        // The seam Task 5 relies on: section `s` defines `equ B = bankid(L)`;
        // section `w` (a SECOND section) has a Value8 fixup targeting `B`. After
        // resolve_layout folds B ($58000 → bank 0xB) and link() defines it, the
        // cross-section fixup writes 0x0B.
        let a = equ_section(0x5_8000, "L", vec![equ("B", bankid_tree("L"))]);
        let w = Section {
            name: "w".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0x1000,
            labels: vec![],
            fragments: vec![Fragment::Data(DataFragment {
                bytes: vec![0x00],
                fixups: vec![Fixup {
                    kind: FixupKind::Value8,
                    offset: 0,
                    target: Expr::Sym("B".into()),
                }],
                span: sp(),
            })],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let out = resolve_layout(&[a, w], &SymbolTable::new(), true).unwrap();
        let linked = crate::link(&out, &SymbolTable::new()).unwrap();
        assert_eq!(linked.section("w").unwrap().bytes, vec![0x0B]);
    }

    #[test]
    fn relax_abs_sym_targeting_an_equ_selects_short() {
        // Task 5 (port #1 follow-up): a `RelaxAbsSym` operand whose TARGET is an
        // `equ` (not a label) must resolve through the relaxation fixpoint, not
        // just through layout labels. `equ R = $FFFF8022` masks (asl_width_rule,
        // 24-bit) into the abs.w RAM range → short (abs.w) candidate selected,
        // 4-byte `2078 8022` block (opcode 0x2078 + Abs16Be $8022).
        let sec = Section {
            name: "hblank".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![relax_move("R")],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: vec![equ("R", Expr::Int(0xFFFF_8022u32 as i64))],
        };
        let out = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap();
        match &out[0].fragments[0] {
            Fragment::Data(d) => {
                assert_eq!(d.bytes, vec![0x31, 0xC0, 0x00, 0x00]);
                assert_eq!(d.fixups[0].kind, FixupKind::Abs16Be);
            }
            other => panic!("expected lowered Data, got {other:?}"),
        }
        let linked = crate::link(&out, &SymbolTable::new()).unwrap();
        assert_eq!(linked.section("hblank").unwrap().bytes, vec![0x31, 0xC0, 0x80, 0x22]);
    }

    #[test]
    fn relax_abs_sym_targeting_an_equ_selects_long() {
        // `equ X = $12345` is outside asl_width_rule's abs.w ranges → abs.l: the
        // 6-byte `long` candidate, Abs32Be operand.
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![relax_move("X")],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: vec![equ("X", Expr::Int(0x1_2345))],
        };
        let out = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap();
        match &out[0].fragments[0] {
            Fragment::Data(d) => {
                assert_eq!(d.bytes, vec![0x33, 0xC0, 0x00, 0x00, 0x00, 0x00]);
                assert_eq!(d.fixups[0].kind, FixupKind::Abs32Be);
            }
            other => panic!("expected lowered Data, got {other:?}"),
        }
        let linked = crate::link(&out, &SymbolTable::new()).unwrap();
        assert_eq!(linked.section("c").unwrap().bytes, vec![0x33, 0xC0, 0x00, 0x01, 0x23, 0x45]);
    }

    #[test]
    fn relax_abs_sym_targeting_an_equ_chain_selects_short() {
        // `equ A = $FFFF8022 ; equ B = A` — B is an equ-on-equ chain; the
        // RelaxAbsSym target `B` must resolve through BOTH links.
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![relax_move("B")],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: vec![
                equ("A", Expr::Int(0xFFFF_8022u32 as i64)),
                equ("B", Expr::Sym("A".into())),
            ],
        };
        let out = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap();
        match &out[0].fragments[0] {
            Fragment::Data(d) => {
                assert_eq!(d.bytes, vec![0x31, 0xC0, 0x00, 0x00]);
                assert_eq!(d.fixups[0].kind, FixupKind::Abs16Be);
            }
            other => panic!("expected lowered Data, got {other:?}"),
        }
        let linked = crate::link(&out, &SymbolTable::new()).unwrap();
        assert_eq!(linked.section("c").unwrap().bytes, vec![0x31, 0xC0, 0x80, 0x22]);
    }

    #[test]
    fn relax_abs_sym_targeting_an_equ_derived_from_a_label_matches_direct_label() {
        // `equ P = SomeLabel` where SomeLabel is a placed section label — must
        // match width/bytes of targeting the label directly (same section, so
        // the equ and the RelaxAbsSym see the SAME final VMA).
        let via_label = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0x12_0000,
            labels: vec![Label { name: "SomeLabel".into(), offset: 0 }],
            fragments: vec![
                Fragment::Data(DataFragment { bytes: vec![0, 0, 0, 0], fixups: vec![], span: sp() }),
                relax_move("SomeLabel"),
            ],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let via_equ = Section {
            equ_syms: vec![equ("P", Expr::Sym("SomeLabel".into()))],
            fragments: vec![
                Fragment::Data(DataFragment { bytes: vec![0, 0, 0, 0], fixups: vec![], span: sp() }),
                relax_move("P"),
            ],
            ..via_label.clone()
        };
        let out_label = resolve_layout(&[via_label], &SymbolTable::new(), true).unwrap();
        let out_equ = resolve_layout(&[via_equ], &SymbolTable::new(), true).unwrap();
        let linked_label = crate::link(&out_label, &SymbolTable::new()).unwrap();
        let linked_equ = crate::link(&out_equ, &SymbolTable::new()).unwrap();
        assert_eq!(
            linked_label.section("c").unwrap().bytes,
            linked_equ.section("c").unwrap().bytes
        );
        // SomeLabel = 0x120000 → abs.l (long candidate, 6 bytes): sanity that
        // this test actually exercises a non-trivial width, not both-zero.
        assert_eq!(linked_label.section("c").unwrap().bytes.len(), 4 + 6);
    }

    #[test]
    fn jbra_targeting_a_near_integer_equ_is_a_loud_error_not_a_pcrel_branch() {
        // Review finding on the Task 5 commit (reviewer-probed, real bytes):
        // `jbra R` with `equ R = $420` and the section at lma $400 would, if the
        // equ overlay fed the ladder, compute bra.s disp $1E and silently emit
        // `60 1E` — a PC-RELATIVE branch to what the author meant as an ABSOLUTE
        // address. Ratified narrowing: ladder targets resolve against LABELS
        // only; an equ target is the parent commit's loud refusal. This test is
        // the regression pin: LOUD error, never `60 1E`.
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0x400,
            labels: vec![],
            fragments: vec![jbra("R")],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: vec![equ("R", Expr::Int(0x420))],
        };
        let err = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap_err();
        assert!(
            err.iter().any(|d| d.level == Level::Error
                && d.message.contains("unresolved branch/ladder target")),
            "expected the loud unresolved-ladder-target error, got: {err:?}"
        );
    }

    #[test]
    fn jmp_jsr_sym_targeting_an_equ_selects_abs_w() {
        // The KEPT half of the review narrowing: `JmpJsrSym` (jmp/jsr) is
        // abs-only by construction, so an equ target is a clean absolute.
        // `jmp R` with `equ R = $FFFF8022` → asl_width_rule masks to 24-bit →
        // abs.w: `4EF8 8022`.
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![Fragment::JmpJsrSym {
                is_jsr: false,
                target: Expr::Sym("R".into()),
                span: sp(),
            }],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: vec![equ("R", Expr::Int(0xFFFF_8022u32 as i64))],
        };
        let out = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap();
        let linked = crate::link(&out, &SymbolTable::new()).unwrap();
        assert_eq!(linked.section("c").unwrap().bytes, vec![0x4E, 0xF8, 0x80, 0x22]);
    }

    #[test]
    fn jbra_targeting_an_equ_alias_of_a_label_is_refused_by_review_ruling() {
        // Deliberate narrowing (reviewed out on the Task 5 commit): even an equ
        // that merely ALIASES a code label (`equ P = TheLabel`) is refused as a
        // jbra/ladder target — branch targets must be spelled as their label
        // (`jbra TheLabel`); use jmp/jsr for an absolute target. A ladder's
        // pc-relative rungs cannot distinguish an equ-of-label from an
        // arbitrary absolute int, and nothing needs the alias spelling.
        let sec = Section {
            name: "c".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0x400,
            labels: vec![Label { name: "TheLabel".into(), offset: 0 }],
            fragments: vec![
                Fragment::Data(DataFragment { bytes: vec![0x4E, 0x71], fixups: vec![], span: sp() }),
                jbra("P"),
            ],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: vec![equ("P", Expr::Sym("TheLabel".into()))],
        };
        let err = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap_err();
        assert!(
            err.iter().any(|d| d.level == Level::Error
                && d.message.contains("unresolved branch/ladder target")),
            "expected the loud unresolved-ladder-target error (equ targets refused for \
             ladders), got: {err:?}"
        );
    }

    #[test]
    fn relax_abs_sym_targeting_a_never_resolving_equ_names_the_symbol() {
        // An equ that NEVER resolves (references a genuinely undefined symbol)
        // used as a RelaxAbsSym target must produce a diagnostic naming the
        // SYMBOL (not just the section), in the Item-C cross-seam-standalone
        // style — distinguishing "not defined in this link" from a cycle.
        let sec = Section {
            name: "hblank".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![],
            fragments: vec![relax_move("HBlank_Handler_Ptr")],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let err = resolve_layout(&[sec], &SymbolTable::new(), true).unwrap_err();
        assert!(
            err.iter().any(|d| d.message.contains("HBlank_Handler_Ptr")
                && d.message.contains("not defined in this link")),
            "expected a diagnostic naming `HBlank_Handler_Ptr` in Item-C cross-seam-standalone \
             wording, got: {err:?}"
        );
    }

    #[test]
    fn duplicate_equ_name_is_dup_symbol_error() {
        // Two equs with the SAME name across two sections → the existing
        // dup-symbol channel (equ funnels through the same `defined_here` map).
        let a = equ_section(0x5_8000, "L", vec![equ("Dup", Expr::Int(1))]);
        let mut b = equ_section(0x6_0000, "M", vec![equ("Dup", Expr::Int(2))]);
        b.name = "s2".into();
        let out = resolve_layout(&[a, b], &SymbolTable::new(), true).unwrap();
        let err = crate::link(&out, &SymbolTable::new()).unwrap_err();
        assert!(
            err.iter().any(|d| d.message.contains("Dup")
                && d.message.to_lowercase().contains("redefin")),
            "expected a dup-symbol diagnostic for `Dup`, got: {err:?}"
        );
    }

    // ================= Org-aware relaxation (runs / barriers) =================

    #[test]
    fn org_forward_relaxable_grows_within_run_and_post_org_content_is_pinned() {
        // Section: [ jmp Hi (grows abs.w→abs.l), nop(2), <Pre @6>, org 0x20,
        //            data(2) @0x20, <Post @0x22> ]. `Hi` is high → the jmp grows
        //   +2. RUN 1 (before the org) shifts: `Pre` 6 → 8. RUN 2 (after the org)
        //   is ANCHORED to the org target 0x20 — the +2 growth before the barrier
        //   must NOT move it, so `Post` STAYS at 0x22. Within-budget growth
        //   (run-1 extent 8 ≤ org target 0x20), so it resolves, both sides right.
        let mut stubs = SymbolTable::new();
        stubs.define("Hi", SymbolValue::Int(0x12_3456));
        let sec = Section {
            name: "objbank".into(),
            cpu: Cpu::M68000,
            vma_base: Some(0x1000),
            lma: 0x1000,
            labels: vec![
                Label { name: "Pre".into(), offset: 6 },
                Label { name: "Post".into(), offset: 0x22 },
            ],
            fragments: vec![
                Fragment::JmpJsrSym { is_jsr: false, target: Expr::Sym("Hi".into()), span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0x4E, 0x71], fixups: vec![], span: sp() }),
                Fragment::Org { target: 0x20, fill: 0x00, span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0xAA, 0xBB], fixups: vec![], span: sp() }),
            ],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let out = resolve_layout(&[sec], &stubs, true).unwrap();
        // Pre shifts with run-1 growth; Post is pinned to the org target.
        assert_eq!(out[0].labels.iter().find(|l| l.name == "Pre").unwrap().offset, 8);
        assert_eq!(out[0].labels.iter().find(|l| l.name == "Post").unwrap().offset, 0x22);
        // The jmp lowered to abs.l (6 bytes), so link places 6 + 2 bytes then the
        // org gap-fills to 0x20 and the trailing 2 bytes land at 0x20..0x22.
        let linked = crate::link(&out, &stubs).unwrap();
        let bytes = &linked.section("objbank").unwrap().bytes;
        assert_eq!(&bytes[0..8], &[0x4E, 0xF9, 0x00, 0x12, 0x34, 0x56, 0x4E, 0x71]);
        assert_eq!(&bytes[0x20..0x22], &[0xAA, 0xBB]);
    }

    #[test]
    fn org_forward_run_growth_past_barrier_errors_loudly() {
        // The org barrier is only 4 bytes ahead, but the jmp grows to 6 bytes:
        // run-1 content (6) OVERRUNS the org target (4). That is a loud error
        // naming the section and the overrun — never a silent overlap.
        let mut stubs = SymbolTable::new();
        stubs.define("Hi", SymbolValue::Int(0x12_3456));
        let sec = Section {
            name: "objbank".into(),
            cpu: Cpu::M68000,
            vma_base: Some(0x1000),
            lma: 0x1000,
            labels: vec![],
            fragments: vec![
                Fragment::JmpJsrSym { is_jsr: false, target: Expr::Sym("Hi".into()), span: sp() },
                Fragment::Org { target: 4, fill: 0x00, span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0xAA], fixups: vec![], span: sp() }),
            ],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let err = resolve_layout(&[sec], &stubs, true).unwrap_err();
        assert!(
            err.iter().any(|d| d.message.contains("objbank")
                && d.message.contains("org")
                && (d.message.contains("overrun") || d.message.contains("past"))),
            "expected a loud run-overrun error naming the section + org, got: {err:?}"
        );
    }

    #[test]
    fn org_backward_overwrite_with_earlier_relaxable_is_byte_identical() {
        // Backward-org (overwrite) case with a relaxable earlier in the section,
        // NON-growing (`Lo` is low → jmp stays abs.w = 4 bytes). The image bytes
        // must be byte-identical to the pre-change `image_bytes` overwrite
        // semantics: [jmp Lo (4)] then data 0xAA,0xBB (offset 4..6), then
        // `org 4` seeks back and data 0xCC overwrites offset 4. Final image:
        // 4EF8 xxxx  CC  BB.
        let mut stubs = SymbolTable::new();
        stubs.define("Lo", SymbolValue::Int(0x1000));
        let sec = Section {
            name: "back".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            labels: vec![Label { name: "Tail".into(), offset: 5 }],
            fragments: vec![
                Fragment::JmpJsrSym { is_jsr: false, target: Expr::Sym("Lo".into()), span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0xAA, 0xBB], fixups: vec![], span: sp() }),
                Fragment::Org { target: 4, fill: 0x00, span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0xCC], fixups: vec![], span: sp() }),
            ],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let out = resolve_layout(&[sec], &stubs, true).unwrap();
        // No growth (abs.w), so labels keep their authored offsets.
        assert_eq!(out[0].labels.iter().find(|l| l.name == "Tail").unwrap().offset, 5);
        let linked = crate::link(&out, &stubs).unwrap();
        assert_eq!(
            linked.section("back").unwrap().bytes,
            vec![0x4E, 0xF8, 0x10, 0x00, 0xCC, 0xBB]
        );
    }

    #[test]
    fn multiple_orgs_three_runs_shift_run_locally() {
        // Three runs separated by two forward orgs, with a growing relaxable in
        // run 1 AND run 3. Each run's growth stays LOCAL to that run:
        //   run 1: [jmp Hi @0, <A @4>]         org→ 0x10
        //   run 2: [data(2) @0x10, <B @0x12>]  org→ 0x20
        //   run 3: [jmp Hi @0x20, <C @0x24>]
        // Both jmps grow +2. A (run 1) shifts 4 → 6. B (run 2) is pinned to org
        // 0x10, so 0x12 stays 0x12 — run-1 growth does NOT reach it. C (run 3)
        // shifts within run 3 from the org-0x20 anchor: 0x24 → 0x26.
        let mut stubs = SymbolTable::new();
        stubs.define("Hi", SymbolValue::Int(0x12_3456));
        let sec = Section {
            name: "runs".into(),
            cpu: Cpu::M68000,
            vma_base: Some(0x2000),
            lma: 0x2000,
            labels: vec![
                Label { name: "A".into(), offset: 4 },
                Label { name: "B".into(), offset: 0x12 },
                Label { name: "C".into(), offset: 0x24 },
            ],
            fragments: vec![
                Fragment::JmpJsrSym { is_jsr: false, target: Expr::Sym("Hi".into()), span: sp() },
                Fragment::Org { target: 0x10, fill: 0x00, span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0x01, 0x02], fixups: vec![], span: sp() }),
                Fragment::Org { target: 0x20, fill: 0x00, span: sp() },
                Fragment::JmpJsrSym { is_jsr: false, target: Expr::Sym("Hi".into()), span: sp() },
                Fragment::Data(DataFragment { bytes: vec![0x03, 0x04], fixups: vec![], span: sp() }),
            ],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        };
        let out = resolve_layout(&[sec], &stubs, true).unwrap();
        assert_eq!(out[0].labels.iter().find(|l| l.name == "A").unwrap().offset, 6);
        assert_eq!(out[0].labels.iter().find(|l| l.name == "B").unwrap().offset, 0x12);
        assert_eq!(out[0].labels.iter().find(|l| l.name == "C").unwrap().offset, 0x26);
    }
}
