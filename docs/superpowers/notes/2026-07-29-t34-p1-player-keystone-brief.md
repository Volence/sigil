# 2026-07-29 — t34 brief: game-side P1 — the player keystone (player_common + sonic)

Status: **DISPATCH BRIEF** (overseer: Fable; porter: Opus subagent, direct-dispatch).
Target = the census's P1 tranche — the HARD ORDERING ROOT of the player cluster. Sigil
master = THIS brief's commit; aeon master **`a4008ca`**.

## 0. Bars

- Canonical: plain **`85111814`/421041** · debug **`eb5e94be`/429102`. Strict baseline
  **2783/0 (1 ignored)**.
- Branches `port-tranche34` BOTH repos, worktrees `.worktrees/port-tranche34`; full
  standard rules (editor rsync, one shape per invocation, cd-every-call, explicit paths,
  no `git add -u`, failures-first, rebuild-worktree-ROMs-after-rebase). CANONICAL-BYTES
  tranche: step-1 delta ZERO; step-2/5 byte movement rides the FULL wave discipline
  (t31 = the live precedent). Parallel porter on `port-tranche33` (sound_fm) — different
  files; merge order ruled at the gates.
- Checkpoints (a)/(b)/(c); loop text; t24 controls; valve standing.

## 1. Scope (FIRM: the census P1 pair)

- `games/sonic4/player/player_common.asm` → `player_common.emp` — **THE KEYSTONE**
  (main.asm:30's own comment: "player_common first — it defines the overlay equates and
  macros"). Carries: the **PlayerV struct** (13 fields per the census; the row-11/25
  overlay class over Sst), the `_pl_*`/`PPHYS_*` equates, and **3 macros** that the
  ground/air/spindash state files need — those become comptime-fn templates under the
  macro-port rule (ADOPT `pixels_to_coord` from coords.emp where the promote idiom
  appears — kill row 49; `distToFix` IS `pixels_to_coord`, census fact — adopt, never
  re-roll).
- `games/sonic4/player/sonic.asm` → `sonic.emp` (54 L, asset hooks) — the census pairs
  it to keep P1 LEAN.
- NOT in scope: ground/air/spindash/sensors (P2-P4), test_player (T1-harness tranche).

## 2. Known inputs (verify at step 0; tree wins)

- The t30 callee-preserves oracle makes `preserves(a0)`-bearing player procs EXPRESSIBLE
  — the capability was built for this cluster; report every use (the oracle consumer
  count grows from 1 — census row updated at close).
- The census P1 rows carry PORTER-VERIFY items (per-shape deltas; the 13-field layout);
  the $8000 bank-shift bar applies if debug-shape bytes move in any wave.
- Overlay rules per t29/t31: shadow-rename where a field would collide with Sst's
  (`[overlay.shadows-field]`); drift guards per the surviving-AS-consumer test (P2-P4
  files SURVIVE as AS consumers of `_pl_*` until they port → this is the row-61 DplcV
  class WITH guards, NOT the single-consumer class — expect offsetof ensures per field
  the AS side still reads).
- Macro→comptime-template ports cite the t24 `refresh_piece_count` / coords.emp
  precedent; templates live where the format lives (player_common.emp), consumers
  import.
- The slot-type gate will fire wherever typed engine procs are called with raw registers
  (the t26/t31 precedent) — blesses are demanded, not optional.

## 3. Template + panel

The t29-31 standing pattern: per-file gates (`SIGIL_EMP_PLAYER_COMMON` /
`SIGIL_EMP_SONIC`), one test file `test_p1_player_port.rs`, one mixed fn
`mixed_tranche34`; windowed both shapes + whole-ROM + t24 controls; regions derived
with byte-gate-adjudicated anchors. Panel **A1 + B1 + C2**; **C1 LIVE-QUESTION** (the
player macros are per-frame physics primitives — if step 5 sees a takeable, the t31
wave discipline applies; flagged call with named sites); C3 inactive unless VDP claims
appear. Dry by panel.

## 4. Duties

Kill rows same-commit (the PlayerV overlay twin = the guarded class; the macro
templates; gates/orgs); ledger per pass; item-13/oracle census counts updated; close
packet + census STATUS AMENDMENT (P1 ported → P2-P4 unblocked). After t34: P2/P3 state
machines ride the keystone; fm/rung-3 continues on the Z80 front.
