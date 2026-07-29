# 2026-07-29 — t38 close packet (P4 player_sensors — THE PLAYER CLUSTER CLOSES)

Porter: t38 (continuation of the killed porter-1). Overseer: Fable. Checkpoints
(a)/(b)/(c) all countersigned from the porter's tips.

**Tips**: aeon `0601cad` · sigil `a4fe9f7` (branch `port-tranche38` both). CRCs,
overseer own-build both shapes: plain **`4b66cace`** EXACT · debug **`1c256b3b`**
EXACT — **UNCHANGED from canonical**. Strict (`--no-fail-fast`, AEON_DIR =
worktree): **2851 / 0 / 1 ignored**. No push · no rebase (masters aeon `b26efc8` /
sigil `c74d9d9` — the overseer rebases at the merge gate).

## 0. Bars (overseer-countersigned at a + b + c)

- CANONICAL-BYTES tranche: step-1 delta ZERO; every step-2/4 movement rode the
  full wave discipline. Outcome: the whole tranche is byte-neutral (§2).
- Baseline strict was 2827/0 at dispatch → 2851/0/1 at close (+9 sp_cleanup
  constraint tests, +1 dc_self_rel already in; the closure-residue failure that
  rode the uncommitted step-1 delta cleared).

## 1. What landed

- `games/sonic4/player/player_sensors.asm` (493 L) → `player_sensors.emp`, the
  ENGINE-BLOCK collision primitive (gate `SIGIL_EMP_PLAYER_SENSORS`, per-shape
  `ifdef __DEBUG__` org arm; shape-varying base, shape-invariant own layout).
- **TWO ratified language/verifier features, both born from THIS file's demands**
  (§3.1) — the dc.w label-label self-relative word (porter-1, `c79fa0e`) and the
  immediate-sp-cleanup slot model (`be78538`), each shipped under 6 constraints.
- **8 68k extern-proc boundary decls DELETED** — the 68k game side reaches ZERO
  extern proc (§3.2). Kill row 80 CLOSED.
- Honest `preserves(a0)` on the 4 probe cores (verified by the new model); the d6
  under-claim catch (§3.4); the step-2 control-flow wave + bare-abs-EA + the abs_w
  adoption — all byte-neutral.

## 2. Byte-delta table — ZERO

| region | plain Δ | debug Δ |
|---|---|---|
| PLAYER_SENSORS | 0 | 0 |
| (whole ROM) | 0 (4b66cace) | 0 (1c256b3b) |

**The wave was a clean NEGATIVE result** — worth contrasting with t35. t35's
identical step-2 control-flow modernization moved −0x20 (16 shrink sites) because
the t35 step-1 `.emp` carried transcribed widths that then relaxed. t38 moved
ZERO: the step-1 port faithfully transcribed asl's LISTING widths, which asl had
ALREADY minimized (`.s` on every reachable short branch, `.w` only for the far
cross-proc/cross-seam calls), so bare/jbra/jbsr auto-select the identical widths
and nothing relaxes. No repin, no org-arm change, no downstream slide, no $8000
concern; neighbour canaries (mixed_tranche34/35/38) green. The AS twin keeps its
explicit widths (byte-level lockstep; no shrink to mirror).

## 3. THE HEADLINE ARC — the player cluster closes

### 3.1 TWO ratified features in one tranche, both demanded by this file

- **`dc.w <local> - <local>`** (porter-1, ratified, `c79fa0e`): the inline
  self-relative jump table (`.case_table` / `.dir_table`) had no `.emp` spelling —
  `Value::Label` rejects arithmetic and `offsets`/`dispatch` are top-level-only. A
  surgical `lower_dc` special-case emits `Cell::RelOffset` for a SUB of two
  proc-local labels; 6 constraints, each a test (`dc_self_rel.rs`): local-only,
  dc.w-only, i16-overflow-at-link, GLOBAL/int operands fall through to the loud
  label-arith error. Reuses the existing difference semantic (player_common's
  Player_States is already `extern(x)-extern(y)`).
- **The immediate-sp-cleanup slot model** (`be78538`): `preserves.rs` bailed on
  ALL explicit sp arithmetic, so the probe cores' `addq.l #N,sp` layer-drop made
  `preserves(a0)` unverifiable. The model now treats an IMMEDIATE sp-INCREASE
  (`add/addq/adda #N,sp`) as dropping exactly N bytes of tracked slots; the `Slot`
  gains a TRUE byte width (`.l`=4/`.w`=2/byte-on-a7=2/per-movem-member) — porter-1's
  prerequisite finding. 6 constraints, each a test (`sp_cleanup_*`, 5 red-then-green
  + 4 controls): immediate-increase-only (sub-family + register-source stay
  bailing), whole-slot boundary (partial bails), over-drop refutes via the wrong-slot
  pop, true widths, non-vacuity. Full frontend suite 1730/0. `find_dead_saves`
  untouched (safe direction). Two stale-under-new-semantics tests updated for honesty.

### 3.2 68k game side → ZERO extern proc; kill row 80 CLOSED

player_sensors ported at step 1 → the sensor primitives resolve module-to-module.
All 8 68k extern-proc boundary decls DELETED: 7 sensor decls (Floor/Ceiling/WallDir/
WallAt across player_{ground,air,spindash}) + `Player_AtLedgeEdge` (player_common,
t34's row-32-class die-at-port). **NO new 68k extern proc born** — `Collision_GetType`
bare-links to the already-ported `engine.collision_lookup` (recon predicted a new
decl; the port needed none — corrections row). Every remaining `extern proc` in the
corpus is a Z80 sound seam (rows 70/71/78, die at Spec 5). The t28 zero-extern-proc
headline is achieved for the 68k side.

### 3.3 THE TWO-ROOT-CAUSE a0 STORY (told in full)

The 10 transitive a0 firings (`corpus_closure_residue`) had TWO causes; the
overseer ruling named ONE. Implementing the ruling alone left all 10.

- **Cause 1 (RULED)**: the sp-cleanup bail (§3.1). FIXED → still 10 firings.
- **Cause 2 (DISCOVERED IN THE TREE)**: the shared CFG (`flag_check::edges`)
  classifies `bsr` as a CONDITIONAL BRANCH (`mnem.starts_with('b') && len==3`), so
  `bsr.s .cell` flows INTO the `.cell` local subroutine AND treats its `rts` as the
  enclosing proc's return — where a0 is clobbered (Collision_GetType) and unrestored
  → refutes `preserves(a0)`. `jsr`/`jbsr` do NOT match the pattern → correctly
  modeled as calls (Follow-next-only). `bsr` and `jbsr` are the SAME instruction
  (both in `CALL_MNEMONICS`), so this is a latent classifier LIE. Isolated with a
  hand-built repro (`bsr .cell / .cell: jbsr X / rts` → a0 NotPreserved).
- **RESOLUTION (byte-neutral)**: convert the 3 `bsr.s .cell` → `jbsr .cell` (short
  local reach → jbsr picks bsr.s; house format anyway). This removes ALL corpus
  exposure (post-step-2 no `bsr` remains in `.emp`). With honest `preserves(a0)` on
  the 4 cores now verifying, closure residue **10 → 0**, cleared via honest
  preserves, NOT widened clobbers.
- **KILL (overseer-sharpened)**: latent-ZERO-exposure gap; kill = the next parcel
  ALREADY touching the shared CFG / flag_check fixes the classifier (model
  `CALL_MNEMONICS` uniformly as calls — one `edges()` line + the blast-radius test
  sweep). NOT fixed at t38 (shared-CFG change carries flag_check blast radius; the
  jbsr conversion is the lower-risk, campaign-aligned resolution).

### 3.4 The d6 catch — the campaign's FIRST 68k header under-claim

`Player_AtLedgeEdge`'s balance path writes d6 (`.terrain: moveq #SOLID_TOP, d6`),
but the t34 header + all caller decls declared `clobbers(d0-d5/...)`. Corrected to
`d0-d6`, propagated to its callers `Player_Animate` / `Player_Display` (`d0-d5` →
`d0-d6`). Z80 had three over-claims (sound_psg/fm); this is the FIRST 68k-side
UNDER-claim. **The header-accuracy scoreboard now tracks under-claims on BOTH CPUs.**
C2 later caught the three prose `Clobbers:` comments that lagged the decl widening
(sensors:488 + common:354/:372) — fixed (§5).

### 3.5 The latent-behind-the-mask corrections item (overseer-shared)

The a0 gap was LATENT in the countersigned checkpoint (a): the closure gate's
PANIC ORDER masked it — with the (a) tree the `extern_collisions` panic fired
BEFORE the firing set was computed, so the 10 firings only surfaced once the
uncommitted extern-proc deletions cleared the panic. An overseer-shared miss (the
gate was countersigned green). Ledger note: single-failure-class reporting — a gate
that panics on the first class hides later classes; the residue assert should
survive the earlier panics (or the checkpoint should run BOTH the panic-clearing
delta and the residue).

## 4. What each pass added (step-3 vs step-5 + neither-bucket)

**Step-1 / neither-bucket (demanded features + contract):** the dc.w feature
(porter-1); the sp-cleanup model; the two-root-cause resolution (jbsr .cell); the 8
extern-proc deletions + honest preserves(a0); the d6 under-claim catch.

**Step-2 (modernize, all byte-neutral):** control-flow wave (bare Bcc / jbra /
jbsr) — byte-neutral (§2); bare-abs-EA ×4 (`(SolidityTable).l`/`(AngleTable).l` →
bare + `// (Sym).l` twin-notes; `(Player_Quadrant).w` ×2 → bare); Sst field access
already the bare `field(a0)` sibling convention; contract clause order correct
(`out(d0,d1,d2)` comma is the corpus norm, 5×); brace-indent compliant; module
header carries a checkable "no bus access" hardware claim; type-layer — Angle (d1)
+ Coord (x_pos/y_pos) in add-sub chains → arithmetic-preservation-gated → LEDGER.

**Loop pass 1 — step-3 findings:** step-3(b) comment-claim audit found the 3
d6-lagging prose `Clobbers:` comments → FIXED. step-4 ADOPT: `abs_w` at
`Player_AtLedgeEdge`'s `|player-center − object-center|` probe (the t35 abs_w
step-6 sweep predates the sensors port, so this newly-ported file's one abs site
was never swept) — byte-neutral, needed the coords test-ambient (the t35 abs_w
blast-radius, added to `test_p4_player_sensors_port` + `build_mixed_tranche38`).

**Loop pass 1 — step-5 findings (C1 ACTIVE named-sites):** hot path (per-frame
collision probes). Invariant ladder — the 3 table-base `lea`s inside `.cell` reload
per cell (~288 cyc/frame steady-state, UNDER the ~1k bar; register-hoisting is
net-negative on the dominant 1-cell-per-probe path + byte-changing) → **log-and-skip
to the post-conversion step-5 pass with the numbers**. Dispatch form — the
`move.w table(pc)/jmp table(pc)` pair is the canonical 68000 minimum (not a
redundant re-index; the two `(pc,d2.w)` computations use different d2 by design).
Guard coverage — the `and.b d6,d0` class gate is load-bearing + correct (covers
SOLID_ALL and SOLID_x). Hardware cross-check — no bus, header claim holds.
Silent-tradeoff comments — coverage complete. **No in-tranche optimization taken.**

**Loop pass 2:** the FULL 3→4→5 circuit came up empty (step-3 nothing new, step-4
nothing to adopt/build, step-5 no change). Pass-1 introduced no new shapes (abs_w
is a known construct; comment fixes are cleanups) → no step-6 sweep obligation. DRY.

**Dry-panel (A1·B1·C1·C2, one round; A1 re-run after it failed to report first
time) — neither-bucket + takeables:**
- **C2 (correctness-hazard)**: all FOUR owed re-derivations PASS — RelOffset
  emission (both `dc.w .t-.b` tables correct, no i16 overflow, dispatch adds to the
  same pc base); probe_core comptime-fn (the 4 instantiations' neg/subflip/step/axis
  roles match the S.C.E. model + the AS twin line-for-line); post-deletion contracts
  honest (clobbers = true callee-union incl. GetType d0-d3/a0; d6 widening real;
  preserves(a0) round-trips); sp-cleanup preserves(a0) traced by hand through the
  NEW slot model on ALL 4 probe paths incl. `.full_back`'s `addq #4,sp` two-word-slot
  drop. ONE actionable finding: the emp:488 prose lag → FIXED (+ 2 siblings).
- **B1 (corpus-pattern)**: NOTHING new/actionable — frontend-CITED that the inline
  jump tables cannot byte-exactly become `offsets`/`dispatch` (both top-level-only,
  emit module-qualified labels + own placement, and neither emits the `move.w/jmp`
  computed-dispatch pair) — matches the `animate.emp .cc_table` precedent (kept
  hand-written); the 4 probe-setup clones FAIL the taste gate (the AS twin
  `probeCore` macro deliberately left them longhand too); `interact_off` dup = the
  already-ledgered deferred cross-file mirror.
- **C1 (cycle/perf)**: hot path already tight; only candidate (table-lea reload
  ~288 cyc/frame) under-bar + net-negative → log-and-skip. AND gate + hardware claim
  verified.
- **A1 (cold-reader)**: two LEDGER/ASK candidates — a **ProbeDir/Cardinal newtype**
  for the 0-3 dispatch direction (d2; cross-module producer → pays once callers are
  typed; would also name Player_Quadrant + the d7 selector) → LEDGERED; a
  **`pub proc Name : SensorProbe { }` proc-type conformance form** to DRY the 4
  verbatim core signatures against the `SensorProbe` type → LEDGERED (low-conf ask).

**Panel adjudication**: actionable findings (C2 comment ×3 + abs_w) RESOLVED
byte-neutrally; the rest LEDGERED with reasons. No finding re-opens the cycle (none
in-tranche actionable). **DRY.** Second panel round not run (cost-bounded rule —
pass-1 added only a known-construct adoption + comment cleanups).

## 5. THE RETIREMENT-CHECKLIST OUTCOMES TABLE (verbatim)

| # | Item | Outcome |
|---|---|---|
| 1 | Row 80 — 8 68k extern-proc decls DELETE | **EXECUTED.** 7 sensor decls (ground/air/spindash) + Player_AtLedgeEdge (common). Kill row 80 CLOSED. NO new 68k extern proc (Collision_GetType bare-links to ported engine.collision_lookup). 68k game side = ZERO extern proc (t28 headline for 68k; remaining corpus extern-procs are Z80 sound seams, rows 70/71/78). |
| 2 | t34 Player_AtLedgeEdge boundary decl DIES | **EXECUTED** (part of #1). Net: 8 deleted, 0 created. |
| 3 | 5 guarded PlayerV `_pl_*` fields (row 74) | **CONDITION NOT MET → Spec 5.** player_sensors reads NO `_pl_*`; the readers survive in player_{ground,air,spindash,common}.asm (row-79 gate-off twins) + config/game.asm. Rows 72/74/75 kill at Spec 5, not P4. |
| 4 | offsets-construct adoption (Player_States) | **DEFERRED — construct-feature-scale.** Cross-module `Ref` targets + zero declaration-form precedent; t34 kill premise OVERTURNED. New kill: dedicated parcel or Spec 5. No correctness gap (current `extern-diff` form lowers to the identical `RelOffset` words). |
| 5 | Row 81 P4 arm (PPHYS/game-config) | **NOT FIRED by sensors.** player_sensors uses ZERO `PPHYS_*`; its SOLID_TOP/LRB mirror is a fresh 1st-consumer drift-guarded mirror. Row 81 stays open (kill = a game-constants `.emp` module born, or Spec 5). |

## 6. Corrections list (BOTH directions — the record owns its errors up AND down)

- **UP the chain**: the OVERSEER's t34 offsets-adoption kill premise ("P4 makes the
  tables pure-internal = the SIMPLE path") was FALSE — the `PState_*` targets are
  `pub proc`s in the OTHER player modules → cross-module `Ref`s, and the `offsets`
  DECLARATION form has zero corpus precedent. Gap-ledger [t34 panel B1] row AMENDED,
  with an explicit corrections entry attributing the premise up the chain.
- (recon) the predicted new `Collision_GetType` extern-proc was not needed
  (bare-link to the ported collision_lookup).
- (brief) the "sensors is the LAST `_pl_*` reader" premise — FALSE (row-79 twins).
- (self, countersigned-(a)) the a0 gap was latent in (a), masked by the closure
  gate's panic order (§3.5) — single-failure-class-reporting ledger note.

## 7. Census update

**THE PLAYER CLUSTER: COMPLETE** — player_common + sonic (t34) · player_ground /
player_air / player_spindash (t35) · player_sensors (t38). P1–P4 all ported.

**Game side remaining**: the 3 objects + the 2 harness states (T1) + main/config.
**Z80 side remaining**: rung-4 driver (T-state first) + the seam sub-tranche (which
retires rows 70/71/78 psg/fm/sequencer twins + the extern-decl-vs-def drift row) +
the generator.

## 8. The drift diagnostic — FIVE faces (the fifth stated precisely)

The seam-diagnostic ask now has five contributing faces:
1. Z80 silent-tolerate (extern-decl-vs-def, t36 probe).
2. Z80 local-pass (the trampoline / invariant(ix) checker-limitation, t36).
3. 68k refuse-outright (the pre-t38 extern-proc closure firing on real drift).
4. 68k verifier-precision-limit (the sp-cleanup bail — a TRUE preserve the checker
   couldn't verify; FIXED this tranche).
5. **68k CFG-classifier LIE (t38, NEW): `flag_check::edges` models `bsr` as a
   conditional branch, not a call — it flows into `bsr <local>` subroutine bodies
   and treats their `rts` as the enclosing proc's return, silently mis-analyzing
   preserves/flags for any un-modernized local-`bsr` proc.** Latent-zero-exposure
   after the jbsr conversion; kill = next-toucher-of-the-shared-CFG.

## 9. Kill-list + ledger state

- Kill row **80 CLOSED** (this packet).
- Rows 72/74/75/77/81 stay OPEN → Spec 5 (the `_pl_*` twins + game-config mirrors
  survive; the player_common internal gate collapses at Spec 5, not P4 — the
  condition-not-met finding).
- Ledger ADDED: the bsr-CFG classifier gap (sharpened kill); the t34 offsets premise
  amendment + up-the-chain corrections row; abs_b candidate; ProbeDir newtype;
  the proc-type conformance ask.

## 10. Overseer rulings applied (recorded)

- The immediate-sp-cleanup model under 6 constraints (manual-honor annotations
  REJECTED — the Fm_ReparkDac precedent kept).
- The bsr-CFG gap kill sharpened to next-toucher-of-the-shared-CFG.
- The t34 offsets premise overturned → dedicated-parcel-or-Spec-5, corrections up
  the chain.
- The byte-neutral wave outcome recorded as a clean negative result vs t35's −0x20.

## 11. Residue for the campaign

Game side: 3 objects + T1 (2 harness states) + main/config close the game. Z80
side: rung-4 driver (T-state first) + the seams + the generator. The bsr-CFG
classifier fix rides the next shared-CFG/flag_check parcel. The offsets adoption +
the abs_b + the two newtype/conformance asks ride their dedicated parcels or Spec 5.
