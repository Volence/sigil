# 2026-07-29 — t39 brief: the FINAL THREE OBJECTS (test_enemy + test_player + path_swap)

Status: **DISPATCH BRIEF** (overseer: Fable; porter: Opus subagent, direct-dispatch).
Target = the game-side object bank's last three unported files. After t39 the object
bank is ALL-.emp and the game side is down to T1 (2 harness states) + main/config
(Spec-5 flip). Sigil master = THIS brief's commit; aeon master **`fa474cd`**.

## 0. Bars

- Canonical: plain **`4b66cace`/421041** · debug **`1c256b3b`/429102**. Strict baseline
  **2856/0 (1 ignored)**. CANONICAL-BYTES tranche: step-1 delta ZERO; step-2/5 movement
  rides the FULL Wave-Ripple Checklist.
- Branches `port-tranche39` BOTH repos, worktrees `.worktrees/port-tranche39`; full
  standard rules (editor rsync, one shape per invocation, cd-every-call, explicit
  paths, no `git add -u`, failures-first --no-fail-fast, rebuild-worktree-ROMs-after-
  rebase, keep commits small). Checkpoints (a)/(b)/(c); loop text; t24 controls; valve
  standing. Parallel porter on `port-tranche40` (Z80 rung-4 — different files).

## 1. Scope (FIRM: the census's last three object rows)

- `games/sonic4/objects/test_enemy.asm` (63 L, shape-invariant) — badnik-shaped;
  Draw_Sprite/ObjectMove/TouchResponse consumers; the shipped test_solid/test_particle
  template applies directly.
- `games/sonic4/objects/test_player.asm` (293 L, shape-invariant, 3 asserts →
  `ensure`) — sensor-consuming: the t38-ported player_sensors procs resolve MODULE-TO-
  MODULE (bare link — NO extern proc; the ladder + the zero-extern-proc headline must
  survive this tranche). Sound_PlaySFX typed call sites per precedent.
- `games/sonic4/objects/path_swap.asm` (132 L, **DEBUG-DIVERGENT: 2 `__DEBUG__`
  blocks**) — the game-side shape-dependent class: `if DEBUG == 1` bodies in the .emp
  (the t36 Z80 precedent + the vblank/core game-side gates), SEPARATE per-shape byte
  gates + per-shape pins (PORTER-VERIFY the deltas; the census marks it canonical,
  shape-dependent). Section calls = the cross-seam surface — resolve per the ladder.
- NOT in scope: T1 (object_test_state/ojz_scroll_test — the harness tranche, kill row
  35 entanglement), main/config, any Z80 file, the G9 d7 item (still ledgered).

## 2. Known inputs (verify at step 0; tree wins)

- The object template is FULLY WORKED (test_solid.emp/test_particle.emp + the t29-31
  arc): per-file gates, objroutine()/code_addr dispatch blessing, overlay classes
  (guarded vs unguarded per the surviving-AS-consumer test — likely UNGUARDED now,
  the state files are all .emp), Draw_Sprite/engine callee contracts all defined.
- engine.constants + engine.coords ambients are LIVE (the hoist parcel); abs_w
  available; no new file-local mirrors for engine truth.
- test_player reads player fields → the PlayerV overlay import path from
  player_common.emp (NOT fresh equates); any `_pl_*` AS-read it carries feeds the
  row-74 guard survival question — re-derive per field and report (the t38 grep found
  config/game.asm readers too; test_player.asm porting REMOVES one AS reader set).
- Every census PORTER-VERIFY item on the three rows; corrections list for anything
  the census got wrong (the standing pattern — it has been wrong twice).

## 3. Template + panel

Per-file gates `SIGIL_EMP_TEST_ENEMY` / `SIGIL_EMP_TEST_PLAYER` / `SIGIL_EMP_PATH_SWAP`
(path_swap's arm per its shape behavior — derive from the listings); one test file
`test_g4_final_objects_port.rs`, one mixed fn `mixed_tranche39`; windowed both shapes
+ whole-ROM + t24 controls. Panel **A1 + B1 + C2**; **C3 ACTIVE for path_swap only**
(section/camera claims); C1 conditional named-basis (these are test objects, not hot
paths — flag if the tree disagrees). Dry by panel.

## 4. Duties

Kill rows same-commit (the 3 gate rows; path_swap = the shape-dependent class row);
ledger per pass; close packet with per-pass breakdown + census amendment (**THE OBJECT
BANK: ALL-.EMP**) + the row-74 reader-set update; corrections list. After t39: T1
closes the game side (minus the Spec-5 flip); rung 4 + seams close the Z80 side.
