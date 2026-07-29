# 2026-07-29 — t38 P4 player_sensors — CHECKPOINT (b) PACKET

The player cluster CLOSES. Porter: t38 (continuation of the killed porter-1).
Overseer: Fable. STOP at checkpoint (b) for the gate.

## Evidence block

- **Tips**: sigil `2317466` (branch `port-tranche38`), aeon `0601cad` (branch
  `port-tranche38`). Both worktrees clean. No push, no rebase (masters aeon
  `b26efc8` / sigil `c74d9d9` — overseer rebases at the gate).
- **CRCs, both shapes, DIRECT own-build**: plain **`4b66cace`** (s4.bin) · debug
  **`1c256b3b`** (s4.debug.bin) — **UNCHANGED from canonical**. The step-2
  control-flow wave landed **BYTE-NEUTRAL** (see the wave outcome below): NO new
  canonical.
- **Strict suite** (`SIGIL_STRICT_GATE=1 AEON_DIR=<aeon-worktree> cargo test
  --workspace --release --no-fail-fast`): **2851 passed / 0 failed / 1 ignored**
  (was 2841/1/1 at the countersigned step-1 tips — the closure-residue failure
  cleared, +9 sp_cleanup constraint tests, +1 residue now passing). The 1 ignored
  is the long-standing aeon-tree subcommands test.
- **Named byte gates, both shapes**: `test_p4_player_sensors_port` (4/4:
  region+debug_region + undoctored-control + doctored-diverges) ·
  `test_p2_player_states_port` (12/12) · `test_p1_player_port` (8/8) ·
  `mixed_tranche35_{,debug_}` · `mixed_tranche38_{,debug_}` — all green.
- **Closure gate**: `corpus_closure_residue_is_empty_the_error_gate` GREEN
  (10 a0 firings → 0).

## THE HEADLINE — the a0 firings had TWO root causes; the ruling named ONE

The overseer's sp-cleanup ruling was NECESSARY but not SUFFICIENT. Implementing
it alone left all 10 firings. The tree held a SECOND cause:

- **Cause 1 (ruled)**: `preserves.rs` bailed on ALL explicit sp arithmetic, so the
  probe cores' `addq.l #N,sp` layer-drop made `preserves(a0)` unverifiable. FIXED
  by the immediate-sp-cleanup model (sigil `be78538`; the `Slot` gains a true
  byte width, `addq/adda #N,sp` drops exactly N bytes of whole slots; 9 constraint
  tests, full frontend suite 1730/0).
- **Cause 2 (DISCOVERED)**: the shared CFG (`flag_check::edges`) mis-classifies
  `bsr` as a CONDITIONAL BRANCH (`starts_with('b') && len==3`), so `bsr.s .cell`
  flows INTO the `.cell` local subroutine and treats its `rts` as the enclosing
  proc's return — where a0 is clobbered (Collision_GetType) and unrestored →
  refutes `preserves(a0)`. `jsr`/`jbsr` are correctly modeled as calls. RESOLVED
  byte-neutrally by the `bsr.s .cell → jbsr .cell` house-format conversion (removes
  all corpus exposure). The classifier lie is ledgered latent-zero-exposure, kill
  sharpened by the overseer (next-toucher-of-the-shared-CFG fixes it).

The a0 gap was LATENT in the countersigned (a) — masked by the closure gate's
panic order (the extern_collisions panic fired before the firing set). Corrections
row + a single-failure-class-reporting ledger note.

## Two-ruling execution (this session, pre-final-leg)

1. **bsr-CFG gap row**: ADDED to the gap-ledger with the overseer-sharpened kill
   (`ca0ac12`). Latent-ZERO-exposure; kill = the next parcel already in the shared
   CFG / flag_check fixes the classifier (`CALL_MNEMONICS` uniform-as-calls).
2. **t34 offsets premise OVERTURNED**: the gap-ledger [t34 panel B1] row's kill
   ("P4 makes the tables pure-internal = the SIMPLE path") AMENDED — FALSE: the
   `PState_*` targets are `pub proc`s in the OTHER player modules → cross-module
   `Ref`s, and the `offsets Name {}` DECLARATION form has ZERO corpus adoption
   precedent → construct-feature-scale. New kill: a dedicated parcel or Spec 5.
   Plus a **corrections row attributing the wrong premise UP the chain** to the
   overseer's t34 ruling (the record owns its errors up, not only down).

## The step-2 WAVE — ripple-checklist outcomes (BYTE-NEUTRAL, no ripple triggered)

The file-wide branch modernization (aeon `ef22869`): conditional branches →
BARE, `bra.s → jbra`, `bsr.w → jbsr`; computed targets keep `jsr (a2)` /
`jmp .table(pc,d2.w)`. **FINDING: zero byte movement** (unlike t35's −0x20). The
step-1 port faithfully transcribed asl's LISTING widths, which are ALREADY minimal
(asl optimized them: `.s` every reachable short branch, `.w` only for the far
cross-proc/cross-seam calls). So bare/jbra/jbsr auto-select the identical widths →
nothing relaxes. Wave-Ripple Checklist, each item:
1. reference rebuild both shapes: done — CRCs 4b66cace/1c256b3b UNCHANGED.
2. org-resume arms (main.asm sensors per-shape `ifdef __DEBUG__` org +
   act_descriptor self-gates): NOT TOUCHED — region length unchanged, no slide.
3. row-1257 downstream-slide sweep: N/A — region size unchanged.
4. 5-site hand-check (engine.inc / mixed_dac_rom / repin_pins / repin.toml /
   test doc-addresses): NO repin — no size change; pins.rs PLAYER_SENSORS
   unchanged. (mixed_dac_rom touched only for the coords test-ambient, not pins.)
5. $8000 bar: N/A — no DEBUG growth.
6. neighbour gates as canaries: mixed_tranche34/35/38 all green.
7. per-region delta table: PLAYER_SENSORS Δ = 0 both shapes.
8. rebuild-after-rebase: no rebase.

## What each pass added (step-3 vs step-5, per pass)

**Opening (step-2 open, `6f22d36`)** — demanded/contract: 8 extern-proc deletions;
honest `preserves(a0)` ×4 (verified via the new model); the d6 under-claim catch
(campaign's FIRST 68k header under-claim — the balance-path ledge probe writes d6
= SOLID_TOP mask; propagated to Player_Animate/Player_Display); the 3 `bsr.s .cell
→ jbsr .cell` (cause-2 resolution). Kill row 80 CLOSED.

**Step-2 wave (`ef22869`, `f216ead`)** — step-2 items walked:
- (1) branches → bare/jbra/jbsr: BYTE-NEUTRAL (above).
- (5) bare-abs-EA: 4 sites fixed — `(SolidityTable).l`/`(AngleTable).l` → bare +
  `// (Sym).l` twin-note; `(Player_Quadrant).w` ×2 → bare (RAM). Byte-neutral.
- (5) Sst field access: already the bare `field(a0)` sibling convention.
- (5) contract clause order: correct (clobbers→out→preserves, falls_into last,
  `/` groups); `out(d0, d1, d2)` comma form is the corpus norm (5×) — not a miss.
- (2) mem-to-mem pins: none. (4) brace-indent: compliant. (7) module header:
  carries a checkable hardware claim ("no bus access") — C3-verifiable, holds.
- (6) type-layer: Angle (d1) / Coord (x_pos,y_pos) in byte/word add-sub chains →
  arithmetic-preservation-gated → LEDGER candidates, not forced.

**Loop pass 1 (`0601cad` + sigil `2317466`)** — step-4 ADOPT: `abs_w` at
Player_AtLedgeEdge's `|center−center|` probe (the t35 abs_w step-6 sweep predates
the sensors port, so this file's one abs site was never swept). Byte-neutral;
needed the coords test-ambient (the t35 abs_w blast-radius — added to
test_p4_player_sensors_port + build_mixed_tranche38). step-3(b) comment-claim
audit: 3 d6-lagging Clobbers comments fixed (sensors:488, common:354/:372).

**Loop pass 2** — EMPTY at all three (step-3 nothing new, step-4 nothing to
adopt/build, step-5 no change) → the file is DRY. No new shapes introduced by
pass 1 (abs_w is a known construct; comment fixes are cleanups), so no step-6
sweep obligation.

**Step 5 (in-tranche, C1 ACTIVE named-sites)** — hot path (per-frame collision
probes). Interrogation: invariant ladder — the 3 table-base `lea`s inside `.cell`
reload per cell (~288 cyc/frame steady-state, UNDER the ~1k bar; register-hoisting
is net-negative on the dominant 1-cell path + byte-changing) → log-and-skip to the
post-conversion pass with numbers. Dispatch form — the `move.w table(pc)/jmp
table(pc)` pair is the canonical 68000 minimum (not a redundant re-index). Guard
coverage — the `and.b d6,d0` class gate is load-bearing + correct (covers SOLID_ALL
and SOLID_x). Hardware cross-check — no bus access, header claim holds. Silent-
tradeoff comments — coverage complete. **No in-tranche optimization taken; hot path
already tight (recorded with numbers).**

## Dry-panel outcomes (A1 · B1 · C1 · C2 — one round; A1 re-run after it failed to report)

- **A1 (cold-reader)**: CLEAN. NEW ledger: a ProbeDir/Cardinal newtype for the 0-3
  dispatch direction (d2) — LEDGER (cross-module producer; pays once callers are
  typed; would also name Player_Quadrant + the d7 selector). NEW ask (low conf):
  a proc-type conformance form `pub proc Name : SensorProbe { }` to DRY the 4
  verbatim core signatures against the `SensorProbe` type — LEDGERED as an ask.
  Confirmed abs_w adopt + abs_b ledger + the inline-jump-table / interact_off
  known items.
- **B1 (corpus-pattern)**: NOTHING new/actionable. Confirmed (frontend-cited) the
  inline jump tables CANNOT byte-exactly become `offsets`/`dispatch` (both
  TOP-LEVEL-only, emit module-qualified labels + their own placement, and neither
  emits the `move.w/jmp` computed-dispatch pair) — matches the `animate.emp`
  `.cc_table` precedent (kept hand-written). The 4 probe-setup clones FAIL the
  taste gate (the AS twin `probeCore` macro deliberately left them longhand too).
  `interact_off` duplication = the already-ledgered deferred cross-file mirror.
- **C1 (cycle/perf)**: hot path already tight; the only candidate (table-lea
  reload ~288 cyc/frame) is under-bar + net-negative to hoist → log-and-skip. AND
  gate verified. Hardware claim confirmed.
- **C2 (correctness-hazard)**: all FOUR owed re-derivations PASS — RelOffset
  emission (both tables' `dc.w .t-.b` words correct, no i16 overflow, dispatch adds
  to the same pc base); probe_core comptime-fn (the 4 instantiations' neg/subflip/
  step/axis roles match the S.C.E. model + the AS twin line-for-line); post-deletion
  contracts honest (clobbers = true callee-union incl. GetType d0-d3/a0; the d6
  widening is real; preserves(a0) round-trips); sp-cleanup preserves(a0) traced by
  hand through the NEW slot model on all 4 probe paths (incl. `.full_back`'s
  `addq #4,sp` dropping two word slots) — every immediate sp-increase lands on a
  slot boundary, the a0 long slot only removed by `movea.l (sp)+,a0`. ONE actionable
  finding: the emp:488 clobbers PROSE lagged the d6 widening → FIXED (+ the 2
  player_common siblings via the corpus follow-up).

**Panel adjudication**: actionable findings (C2 comment ×3, abs_w adopt) RESOLVED
byte-neutrally; the rest LEDGERED with reasons (ProbeDir newtype, proc-type
conformance ask, abs_b, table-lea reload). No finding re-opens the cycle
(none in-tranche actionable, no new shapes). **DRY declared.** Per the cost-bounded
rule, a second panel round is not run — pass-1 added only a known-construct
adoption + comment cleanups.

## THE RETIREMENT-CHECKLIST OUTCOMES TABLE

| # | Item | Outcome |
|---|---|---|
| 1 | Row 80 — 8 68k extern-proc decls DELETE | **EXECUTED.** 7 sensor decls (ground/air/spindash) + Player_AtLedgeEdge (common). Kill row 80 CLOSED. NO new 68k extern proc (Collision_GetType bare-links to ported engine.collision_lookup). 68k game side = ZERO extern proc (t28 headline for 68k; all remaining corpus extern-procs are Z80 sound seams, rows 70/71/78). |
| 2 | t34 Player_AtLedgeEdge boundary decl DIES | **EXECUTED** (part of #1). Net: 8 deleted, 0 created. |
| 3 | 5 guarded PlayerV `_pl_*` fields (row 74) | **CONDITION NOT MET.** player_sensors reads NO `_pl_*`; the `_pl_*` readers survive in player_{ground,air,spindash,common}.asm (row-79 gate-off twins) + config/game.asm. Rows 72/74/75 kill at Spec 5, not P4. Brief's "sensors is the LAST `_pl_*` reader" premise FALSE (corrections row). |
| 4 | offsets-construct adoption (Player_States) | **DEFERRED — construct-feature-scale.** Cross-module `Ref` targets + zero declaration-form precedent; t34 kill premise OVERTURNED (corrections row up the chain). New kill: dedicated parcel or Spec 5. No correctness gap (current `extern-diff` form lowers to the identical `RelOffset` words). |
| 5 | Row 81 P4 arm (PPHYS/game-config) | **NOT FIRED by sensors.** player_sensors uses ZERO `PPHYS_*`. Its own SOLID_TOP/LRB mirror is a fresh 1st-consumer drift-guarded mirror, not a row-81 item. Row 81 stays open (kill = a game-constants `.emp` module is born, or Spec 5). |

## Corrections list (this tranche)

- (up the chain) the overseer's t34 offsets-adoption kill premise was FALSE
  (ledger amended).
- (recon) the predicted new `Collision_GetType` extern-proc was not needed
  (bare-link to the ported collision_lookup).
- (brief) the "sensors is the LAST `_pl_*` reader" premise is falsified by the
  row-79 twins.
- (self, countersigned-(a)) the a0 gap was latent in (a), masked by the closure
  gate's panic order → single-failure-class-reporting ledger note.

## CENSUS AMENDMENT

**THE PLAYER CLUSTER: COMPLETE** (player_common + sonic [t34] · player_ground/air/
spindash [t35] · player_sensors [t38]). Remaining game side: the 3 objects + T1.
Z80 side: rung-4 driver + the seams.

## The drift diagnostic now has FIVE faces (verifier-side cousin added)

Z80 silent-tolerate · Z80 local-pass · 68k refuse-outright · 68k verifier-
precision-limit · **68k CFG-classifier lie (the bsr-vs-jbsr asymmetry, t38)** —
all feeding the seam diagnostic ask.
