# t18 — 3→4→5 loop findings: parallax transition-logic (step-3(b)) + step-5 sizing

Porting agent, after the trampoline. Applies the step-0 gate's Q2 in-tranche/split
rule to B1/B2/B3 and sizes the step-5 perf target. **One rule-worthy scoping
call for the gate** (§4).

---

## 1. B1/B2/B3 — surfaced, characterized, ALL SPLIT (Q2(ii) fails in-tranche)

The three transition-logic issues (row 1085) are all **behavior-affecting** and
share one property: they only manifest on a **section-boundary crossing that
changes the active parallax_config** (a `Parallax_StartTransition` firing).
The t18 test scene (OJZ scroll) is input-static and does not cross section
boundaries in normal play — the SAME live-verify barrier the gate just ruled for
the window-slide rider. So **Q2 criterion (ii) "live-verifiable in this tranche's
oracle session" FAILS for all three** → follow-on per the pre-ruled split rule.

### B3 — geometric lerp ends ~36% short → promotion-frame pop (MATH-PROVEN)
- **Root cause (found this step):** `constants.asm:308` claimed
  `PARALLAX_LERP_SHIFT >>4 ≈ 16-frame convergence to ~95%`. FALSE — the real
  math: the per-band BG lerp is `current += (target-current)>>4` (parallax.emp
  `.write_b`, :427-430), a geometric decay of `15/16` per frame. After the
  16-frame `PARALLAX_TRANS_DEFAULT` window: `(15/16)^16 ≈ 0.356` of the gap
  REMAINS (~64% closed, not 95%). ~95% convergence would need ~46 frames.
- **The pop:** on the frame the counter hits 0 (parallax.emp :356-364), the
  config is promoted (Current=Target) AND `Transition_Frames→0`, so that same
  frame's band loop takes `.snap_b` (:425-426 `tst Transition_Frames / beq
  .snap_b`) → `current_b = target` (snap). The ~36% residual snaps in one frame
  = a visible end-of-transition pop.
- **Comment fixed byte-neutral this step** (the smoking gun; step-3(b) comment
  audit — present-tense function, no bug narration): `constants.asm:308` now
  states the real `~36% remains after 16 frames`.
- **Design sketch (follow-on):** replace the fixed `>>4` with a
  frames-remaining-aware step so the lerp converges by frame 0 — linear
  `step = (target-current)/frames_remaining` (division/band, or a per-frame
  reciprocal table), OR a recalibrated (shift, frame-count) pair that lands the
  residual under the perceptible threshold before the snap. Byte-changing hot
  band-loop formula; behavior-changing → needs a boundary-crossing live A/B.
- **Split reason:** locally PROVABLE (the math), but not locally
  live-VERIFIABLE (does the pop vanish) without a crossing → Q2(ii) fails.

### B1 — re-cross into the current config mid-transition doesn't cancel
- **Confirmed:** `Parallax_StartTransition` (:224-229) no-ops when `a0 ==
  Current_Config` (:227). Mid-transition (Current=A, Target=B, frames>0),
  re-crossing back into A's section calls `StartTransition(A)` → `a0 ==
  Current_Config` → `.no_change` → the transition to B **continues** and
  completes at B, even though the camera returned to A's section.
- **Reachability:** rare — requires oscillating across a boundary within the
  16-frame window (camera moves a few px/frame; a full section is 2048 px).
- **Fix = a cancel branch** in StartTransition (if `a0==Current` AND a transition
  is staged → clear Target/frames, snap to Current). Adds a state-machine path →
  borderline Q2(iii) "restructure". **Split** (rare + borderline-restructure +
  not live-verifiable).

### B2 — mode/data disagreement mid-transition (the design pass)
- **Confirmed:** `StartTransition` sets the VDP `$0B` Mode-Set-3 shadow to the
  NEW config's mode at frame 0 (:250-266), but `Parallax_Update` lerps the band
  DATA from the old toward the new over 16 frames. If the two configs' HScroll
  modes differ (per-cell `%10` vs per-line `%11`) or VScroll modes differ, the
  VDP renders in the NEW mode while the data is intermediate — and the DMA length
  (per-cell 112 B `Static_Hscroll_Cell` vs per-line 896 B `Static_Hscroll_Line`)
  keys off which. A ≤16-frame tear.
- **Fix** = decide the "active mode" contract during a transition (defer the mode
  switch to completion, or build the intermediate data to match the switched
  mode). A genuine transition-state-machine **design pass** → **Split** (Q2(iii)
  restructure + not live-verifiable).

---

## 2. d6-across-CheckBoundary — contract audit: SOUND, latent fragility (ledger only)

`Parallax_CheckBoundary` contract = `clobbers(d0-d3/a0/a2)` (preserves d4-d7 etc.)
— verified accurate against the body (uses only d0/d2/d3/a0/a2). The caller
`games/sonic4/test/ojz_scroll_test.asm` holds `d6` live across `jsr
Parallax_CheckBoundary` (`:215 move.w d0,d6` … `:220 jsr` … `:224 move.w d6,d0`),
relying on the preserve. **Sound today.** The fragility: the caller is a `.asm`
test file, so the verifier does NOT check its cross-seam liveness reliance — only
the `.emp` side's contract discipline protects it. If CheckBoundary ever grows a
d6 use, this caller breaks silently. Ledger row, no code change.

---

## 3. Step-5 sizing — Parallax_Fill_PerLine (row 1058, the 21k bulk)

This is the in-tranche live-verifiable target: the per-line fill runs EVERY frame
in the static scene, so a perf cut is A/B-profileable without a boundary crossing.
The proc is already heavily hand-optimized — per-band invariant hoisting + FOUR
specialized line loops (`.lp_both`/`.lp_fg`/`.lp_bg`/`.lp_flat`) + per-band
register packing (d0 = FG<<16|BG, d3 = shift_a|shift_b<<16, etc.). The
H2/H3/M1-M5 candidates (row 1085 item 22, Tier-B) are micro-opts WITHIN the tight
line loops (addressing-mode / instruction-selection / redundant-recompute).
**This is the substantive remaining in-tranche byte-changing work — its own
focused push with the oracle profiler** (mind the profiler-cache-flush fix,
gap-ledger 2026-07-12: flush on reload). Sized here, not yet cut.

---

## 4. THE SCOPING CALL — COUNTERSIGNED (gate, 2026-07-23): ONE post-merge parcel

B1/B2/B3 all fail Q2(ii) for the same reason the window-slide rider does — they
need a **section-boundary crossing with a config change**, which the static test
scene can't drive. **Additional grounds (gate):** B1/B2/B3 are *faithfully-ported
shipped behavior* — a behavior fix rides its own gated parcel with its own live
proof, NEVER a port merge. t18 ports the behavior as-is; the fixes are follow-on.

**RATIFIED PARCEL STRUCTURE (post-merge, one gated parcel):** build the
crossing-drive capability ONCE (drive the scroll target / Game_Entry soak,
wedge-avoiding), then in this ORDER:
1. **Window-slide observation** — close the carried rider (the capability's first
   use; a null result is still logged).
2. **B2 mode-contract FIRST** — it *defines the transition state machine* that B1
   and B3's fixes land inside (defer-mode-to-completion vs build-intermediate-to-
   -match-mode is the structural decision; B1/B3 are shaped by it).
3. **B3 frames-remaining ramp** — inside B2's ratified state machine.
4. **B1 cancel branch** — inside B2's ratified state machine.

Each behavior fix = its own byte-changing commit + full ripple + its own live
proof (boundary-crossing A/B). t18's in-tranche remaining work is ONLY step-5
(Fill_PerLine perf, live-verifiable now).

**Next in-tranche:** step-5 Fill_PerLine interrogation (profiler-first, numbers
decide) → dry-panel debut (A1+B1+C1+C2+C3, C3 active). Ledger rows for
B1/B2/B3 + d6 appended.

## 5. STEP-5 Fill_PerLine — interrogation rule (gate, 2026-07-23)

Bounded, numbers decide (NOT skipped, NOT a marathon). Oracle profiler on the
current bg scene FIRST (mind the reload-flush fix, gap-ledger 2026-07-12).
**Decision rule:** a candidate projecting **≥ ~1k cyc/frame** → design + cut +
live A/B (own byte-changing commit, full ripple); candidates all projecting micro
(**< ~500 cyc/frame**) → log-and-skip each *with its number* (logged-decision
doctrine). **H2/H3/M1-M5 each get a STATED VERDICT** either way — the panel's C1
lens re-audits, so the skip-log is the evidence. Volence has flagged parallax
perf as a want (recent bg change) — raises the interest, not the threshold.
