# 2026-07-29 — dispatch brief: the AMBIENT-HOIST parcel (engine.constants + abs_w engine-hoist, combined)

Status: **DISPATCH BRIEF** (overseer: Fable; porter: Opus subagent, direct-dispatch).
The two post-t35 hoists share one blast-radius class (test-ambient additions), so they
run as ONE byte-neutral parcel. Sigil master = THIS brief's commit; aeon master
**`1cbd4fd`**.

## 0. Bars

- Canonical: plain **`4b66cace`/421041** · debug **`1c256b3b`/429102**. Strict baseline
  **2827/0 (1 ignored)**. **HARD BYTE-NEUTRALITY**: any CRC movement = STOP + report,
  never absorb.
- Branches `hoist-ambient` BOTH repos, worktrees `.worktrees/hoist-ambient` (doc/cleanup
  parcels mandate worktrees like every other parcel — the nine-rules-merge incident).
  Standard rules (editor rsync, one shape per invocation, cd-every-call, explicit paths,
  no `git add -u`, failures-first). Single-checkpoint parcel (the style-bare-abs-ea
  class): one STOP at the end with the full evidence block.

## 1. Scope (FIRM — the two ledgered hoists + the unblocked adoption)

1. **The engine.constants hoist** (kill rows 76/77/81; the t35 step-4 adjudication
   ruled DEFER-to-this-parcel): move the file-local `const`+`ensure` mirrors of
   `engine/constants.asm` truth (PHYS_* / ST_* / PPHYS_* / BUTTON_* / radii / CURL /
   EDGE_* / COLLISION_NONE) out of `player_common.emp`/`sonic.emp`/
   `player_{ground,air,spindash}.emp` into the shared engine-constants `.emp` twin
   module (the corpus's existing constants home — verify its name/structure at recon;
   tree wins). Game-config mirrors (PSTATE_*/ANIM_*/VRAM_TEST_*/SFXID_*) are NOT
   engine truth — they stay (rows 76/77's game-constants-module future is NOT this
   parcel).
2. **The abs_w engine-hoist + the 6 blocked folds** (kill row 82's sweep completion;
   the t35 wrong-home lesson): re-home `abs_w` from `player_common.emp` to the engine
   coords module (`pixels_to_coord`'s home), keep a re-export or import path so the
   12 existing game-side folds stand unchanged, and fold the 6 verified engine sites
   (entity_window `.sx_pos`/`.sy_pos`, collision `.solid_ax_pos`/`.solid_ay_pos`,
   core `.culled_xpos`/`.culled_ypos`).
3. **BUTTON_*_BIT adoption** (the t35 item-13 ledger's waits-on-the-hoist item): the
   ported player files' commented raw `btst #N` sites adopt the named consts once
   they're ambient — byte-neutral folds, comments die.

The blast radius IS the parcel: every test/mixed-build compiling the touched engine
modules gains the needed ambient (the t35 porter's evidence names the 3 port tests +
core_negative_probes + tranche7_negative_probes + the mixed placed_module_sections
arms; re-derive the full set, don't trust the list).

## 2. Duties

- Kill rows updated same-commit: 76/77 (engine-block clause → HOISTED), 81 retired or
  reduced to its game-config remainder, 82's sweep clause → COMPLETE.
- Drift guards MOVE with the consts (the `ensure(extern(...)==...)` pairs live at the
  new home; no guard is silently dropped).
- Ledger rows closed with refs (the two blocked-hoist rows + the BUTTON adoption row);
  corrections list if recon contradicts the brief.
- Evidence block at the STOP: both-shape CRCs (must be EXACT), own strict counts
  (2827/0 bar; new-test deltas named), the per-site fold list, the ambient-addition
  list, the kill/ledger row diffs. No pushes; the merge gate is the overseer's.
