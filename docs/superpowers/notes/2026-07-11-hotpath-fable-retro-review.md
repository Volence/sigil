# Hot-path retro review — core / collision+aabb / rings / animate (Fable)

2026-07-11 · The first run of the new step-3(b)+step-5 interrogations,
applied retroactively to the previously-merged hot-path files at
Volence's request. Line refs at current master. NOT reviewed this pass
(smaller/colder — next wave or on-touch): dplc, game_loop, hblank,
controllers, math, sound files, sprites (has its own addendum).

## core.emp

**C-A1 — RULE-VIOLATION CLASS (take in the follow-up wave).** Three
outlawed byte-locks survive on master, comments citing the exact
reasoning removed from the loop this morning:
- `bne.w RunObjects_Frozen` (184) — THE incident site, old comment
  intact ("the AS twin pins bne.w").
- 2× `bsr.w Debug_AssertObjLoop` (220, 257) — "byte-locked to the AS
  twin's ifdebug bsr.w". The ifdebug macro doesn't generate the width;
  the twin's author chose it — it's OURS. → jbsr.
All three: convert + shrink the twin in lockstep + re-pin (the bsr sites
are DEBUG-shape only → debug-pin re-derive).

**C-B1 — Comment-claim FAIL (fix the comment; cross-ref occupancy §3).**
InitObjectRAM (43-44): "push addresses from last to first so first slot
is popped first" — the code does the OPPOSITE: pushes slot 0→39, so
slot 39 pops FIRST; the first spawn gets the HIGHEST slot. The behavior
may even be fine (allocation order is an artifact either way — exactly
occupancy-spec §3's point) but the comment states a falsehood. If
slot-order allocation is ever wanted, flip the push loop — that's a
behavior change, decide alongside the occupancy ordering call.

**C-B2** — DeleteObject's out-of-range arm (145, "shouldn't happen")
silently zeroes 80 bytes at a garbage pointer. DEBUG `assert` candidate
the moment the diagnostics construct ships (add to its retrofit list).

**C-B3** — AllocEffect skips the SLOT_TAG_UNTAGGED tag that AllocDynamic
sets (83 vs 98-105). Correct (effects are exempt from entity-window
despawn tagging) but undocumented — one comment line.

**C-C1** — `.run_always` builds the moveq/swap bank prefix BEFORE the
emptiness test (210-213): ~8 wasted cycles per empty slot vs
`.run_culled`'s tst-first shape. Fold into the occupancy build (which
rewrites these loops) rather than a standalone wave.

**Note** — Debug_AssertObjLoop's `d7 < NUM_DYNAMIC` bound is coarse for
the smaller pools (a clobber to 20 during the effects walk passes).
Acceptable as-is; recorded so nobody thinks it's tight.

## collision.emp + aabb.emp

**VERDICT: the min-pen core is CORRECT.** Verified end-to-end: the aabb
template tests `2·|delta| < (adim+bdim)` (full-width semantics);
Touch_Solid's `lsr #1` halving produces the exactly-matching penetration
metric; the `ble .solid_done` guards are LOAD-BEARING for odd combined
sizes (protect class — never "clean up"). The interact_off()
drift-ensure, the per-pass claim lifecycle comments, and aabb's
ratified-and-documented `.s` pins are the house standard done right.

**co-B1 (gameplay question, not a defect)** — Touch_Solid's side push
zeroes x_vel regardless of the player's motion direction (260-266);
classic engines zero it only when pressing INTO the wall. If push-back
feel ever reads sticky, this is the knob. Volence's call.

**co-B2** — Touch_HandlerTable slot 0 (Touch_None) is unreachable via
dispatch (collision_resp==0 filtered at 72) — it exists for stride.
One comment line.

(aabb's Unit-branch zero-copy latency is already covered by the
code-splice spec's acceptance item — no new row.)

## rings.emp

**R-A1 — Visual defect class (take in the follow-up wave, live-verify).**
DrawRings' cull window is MIS-CENTERED for its center-based coords (the
SAT write subtracts 8): culls use `addi #16` vs 336/240 (213-214,
221-222), giving left/top +8 slack but clipping right/bottom 8px EARLY —
a ring still half-visible at screen right/bottom pops out. Fix: `addi
#8` (same byte count, constants unchanged), update the "320+16" margin
comments, live-verify by scrolling ring rows across all four edges.

**Praise where due:** RingCollision's rolling-pointer × swap-with-last
interaction is CORRECT (removals only rewrite already-visited higher
indices) and its survival-analysis header comment is the best-documented
tricky code in the corpus — cite it as the step-3(b) exemplar.

## animate.emp

**A-B1 (contract-audit find)** — `.evt_sound`'s `movem.l a1/d1` save
(197-199) is redundant: Sound_PlaySFX declares (and enforces)
`preserves(d1/a0)` and never touches a1. Either drop it (−8 bytes,
byte-changing) or keep it with a "defensive vs future callee change"
comment — taste call, but decide on the record.

**A-B2** — `andi.b #$F9` / `andi.b #$06` (70-72) are uncommented flip-
bit surgery (clear RF_XFLIP|RF_YFLIP, copy status bits 1-2 in). Name
the masks or comment the line.

**A-B3 (data-hazard asserts, post-diagnostics-construct)** — two script-
author traps with no DEBUG rail: cyclic AF_CHANGE (A→B→A hangs the frame
in the .cc_change → AnimateSprite restart loop) and .cc_back rewinding
past frame 0 (reads garbage as script bytes). Both become one-line
asserts when the construct ships — add to its retrofit list.

**A-C1 (step-4 structural clone)** — the fetch-classify-dispatch tail
(`moveq/move.b anim_frame/move.b 1(a1,d1)/cmpi AF_SET_FIELD/bhs/jbra`)
appears ~4× (90-94, 141-145, 152-157, 219-224). Comptime-fn candidate
(cleaner post-splice), byte-neutral dedup for a step-6-style pass.

## Process outcome

The interrogations earned their keep on their first retro run: the
comment-claim audit caught C-B1 (a false comment carried from the twin),
the contract audit caught A-B1, the invariant/asymmetry lenses caught
C-C1, and the rule-violation sweep caught C-A1 — none of which any
tranche's own passes had surfaced. Work items route: C-A1 + R-A1 into
the existing sprites follow-up wave (same byte-changing + re-pin batch);
C-B2/A-B3 onto the diagnostics-construct retrofit list; the comment-only
items ride any next touch.
