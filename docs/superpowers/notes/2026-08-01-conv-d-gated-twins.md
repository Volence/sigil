# 2026-08-01 — CONV-D PARCEL D: the gated code twins (close packet)

Status: **Checkpoint for the overseer's countersign + merge.** Branch pair
`conv-d-twins` (aeon + sigil). All FOUR gated code twins (#13 z80_init, #47
test_enemy, #48 test_player, #49 player_common) are flipped: the `.emp` is the
sole source, the `.asm` twins are DELETED, and every cross-seam header consumer
is re-pointed at the `.emp`. Every target byte-identical to the chain-9 goldens.
NOT merged — the merge is the overseer's.

## §0 — THE HEADLINE

The last four AS-owned code twins in the aeon tree are gone. `z80_init.asm`,
`test_enemy.asm`, `test_player.asm`, `player_common.asm` are all DELETED; their
always-emitted header equates/structs/macros are now `.emp`-owned, and the
surviving cross-seam consumers (drift guards, camera's `_pl_state` late-bind,
test_objects' `ENEMY_PATROL_SPEED`) are re-pointed at the `.emp` side. **Byte-
identical to the chain-9 goldens across all six targets.** Strict 2849 → 2846
(net −3: the retired `z80_init_port` AS-twin oracle).

## §1 — PREMISE CORRECTIONS (census vs reality)

The census (2026-07-31, Parcel D) said "drop the `SIGIL_EMP_*` gate, make the
`.emp` unconditional, delete the `.asm` header+twin" and named "Parcel A (structs
flip) moves their struct headers" as the dependency. Re-verified against source:

1. **The struct headers are NOT in `structs.asm` — they are LOCAL to each twin
   file** (`TEnemyV` in test_enemy.asm, `DplcV`/`TPlayerV` in test_player.asm,
   `PlayerV` in player_common.asm). Parcel A (structs.asm flip) does NOT move
   them; the flip moves each header into its OWN `.emp` module. So Parcel D has NO
   real Parcel-A dependency.

2. **The `SIGIL_EMP_*` gate STAYS armed** (in `code_gate_defines()` + repin.toml).
   The census framing "drop the gate" is wrong: all 56 `.emp` code regions use the
   same gate mechanism, and 52 of them are already flipped with the gate present-
   but-inert (no `.asm` reads it). Deleting the `.asm` matches that established
   end-state; the gate becomes inert, not removed. The `repin.toml` region + the
   `native.rs` registry entry STAY (they place the `.emp` body). Only the `.asm`
   file + its `include` + its dangling extern consumers change.

3. **The headers have LIVE cross-seam `.emp` consumers via `extern()` drift
   guards** — the D11 pattern deliberately kept the `.asm` header as the "source of
   truth" with `.emp`-side drift guards. The flip INVERTS that: the `.emp` becomes
   the truth, the guards (whose extern target is deleted) are removed, and the one
   real (non-guard) consumer — engine.camera's `extern("_pl_state")` — is served by
   a new `.emp` link-export. This is a cross-module rewire (≈10 files), NOT a
   mechanical delete, but every change is at unchanged bytes.

4. **Each twin had an AS-twin ORACLE** (`z80_init_port`) or region byte gate
   reading the `.asm`. Only `z80_init_port` reads the `.asm` file directly — it
   RETIRES with the twin. The region byte gates (test_g1/test_g4/test_p1/test_p2)
   read the `.emp` + the reference ROM, so they survive untouched (minus the equ
   supplies they no longer need).

5. **`.emp` CAN link-export a named scalar equate** (`equ NAME = expr` →
   `add_equ_sym`, frontend-emp `lower/mod.rs:685`), so camera's engine→game
   late-bind of `_pl_state` is solvable without a language feature — the crux that
   let player_common flip cleanly rather than block (unlike Parcel C).

## §2 — THE FLIP (per file)

### #13 z80_init — `engine/system/z80_init.asm` DELETED
Canonically dead: both shipped sonic4 shapes have sound ENABLED, so boot_data.asm
never reaches the include; the no-sound shapes (demo/config_b) always arm
`SIGIL_EMP_Z80_INIT` and take the `.emp` path. The flip collapses boot_data.asm's
no-sound else-arm to the `.emp`-only path (removes the `ifndef SIGIL_EMP_Z80_INIT`
include + the two `ifndef`-gated positional asserts that measured the pre-
relocation residual where the idle is a hole — dead in every built shape), keeping
the numeric `Z80_IDLE_SIZE = $3FE-$3D8` + `org $3FE`. **`z80_init.emp` unchanged.**

### #47 test_enemy — `games/sonic4/objects/test_enemy.asm` DELETED
main.asm include → resume `org $10F4A`. `ENEMY_PATROL_SPEED` ($100, the
ObjDef_Enemy patrol x_vel) is now owned in-module by `test_objects.emp` (its sole
consumer); its extern drift guard is removed. `TEnemyV`/`_enemy_*`/`objvarsCheck`
were code-body-only — test_enemy.emp's UNGUARDED overlay owns them.

### #48 test_player — `games/sonic4/objects/test_player.asm` DELETED
main.asm include → resume `org $10F02`. The header equates become `.emp`-owned:
`TPlayerV` (test_player.emp), `DplcV` `$2E/$32` (test_animated.emp, byte-identical
prefix), physics consts `GRAVITY`/`JUMP_*`/… (file-local, test_player.emp),
`STUB_FLOOR_Y=192` (object_test_state.emp, its sole consumer). The 6 dangling
extern drift guards (3 test_player + 2 test_animated + 1 object_test_state) are
removed; the `VRAM_TEST_SONIC` guard stays (its home config/constants.asm survives
to Parcel F).

### #49 player_common — `games/sonic4/player/player_common.asm` DELETED
main.asm include → resume `org $10448`. player_common.emp owns the `PlayerV`
overlay + `PPHYS_*`/`BUTTON_JUMP_MASK`/`PLAYER_DEBUG_FLY_SPEED` consts + the four
macro→comptime-fn templates outright; the state files already imported them by
`use`. The one cross-seam reader — engine.camera's GAME_CAMERA_JUMP_LOCK probe
`extern("Player_1") + extern("_pl_state")` — is served by a new link-export:
```
equ _pl_state = offsetof(Sst, sst_custom) + offsetof(PlayerV, player_state)
```
so camera's extern resolves unchanged ($30). All other `_pl_*` were read only by
now-removed self drift guards. Removed dangling guards: 5 `_pl_*` (player_common),
5 `PPHYS_*` (player_ground), `BUTTON_JUMP_MASK` + 4 `PPHYS_*` (player_air) — their
file-local consts stay, protected by the region byte gate. **The offset tables
(`Player_States`/`EnterHooks`/`ExitHooks`) keep the pre-ruled extern-difference
form; the offsets-DSL adoption stays deferred (ledger 1767).**

## §3 — THE BYTE-IDENTITY PROOF (all six vs chain-9)

| target | built | chain-9 golden | match |
|---|---|---|---|
| s4          | `6cf74e65` / 412127 | `6cf74e65` / 412127 | ✓ |
| s4.debug    | `16615e46` / 421958 | `16615e46` / 421958 | ✓ |
| demo        | `9bb8c993` / 90506  | `9bb8c993` / 90506  | ✓ |
| demo.debug  | `bc7678d0` / 93006  | `bc7678d0` / 93006  | ✓ |
| config_a    | `78df5e6a` / 422297 | `78df5e6a` / 422297 | ✓ (strict-suite byte-identity) |
| config_b    | `f38f609b` / 303501 | `f38f609b` / 303501 | ✓ (strict-suite byte-identity) |

s4/s4.debug/demo/demo.debug verified directly; config_a/config_b via
`native_offcanonical_full::config_a_full_file` / `config_b_full_file` (whole-ROM
byte-identity). Ownership move at unchanged bytes — no re-freeze, no oracle A/B
(the `.emp` twins passed their oracle proofs when written).

## §4 — THE RETIRED-TEST ENUMERATION (strict 2849 → 2846, net −3)

| retired test | file | re-homed to |
|---|---|---|
| `z80_init_matches_as_twin` | `z80_init_port.rs` (DELETED) | `mixed_offcanonical_rom::mixed_z80_init_config_b` (whole-ROM) + demo byte-identity |
| `emp_diverges_from_doctored_twin` | `z80_init_port.rs` (DELETED) | — (the AS-twin doctored-falsification dies with the twin; the whole-ROM gate + `config_b` byte-identity are the surviving proof) |
| `doctored_both_sides_stay_equal` | `z80_init_port.rs` (DELETED) | — (same) |

Net: −3 → **2846 passed / 0 failed / 4 ignored.** The whole file `z80_init_port.rs`
was deleted (it is the only test that READ a deleted `.asm`).

### Count-neutral test edits (no test retired)
- `test_objects_port.rs`: dropped the `ENEMY_PATROL_SPEED` equ supply (owned in-
  module now; no extern guard). Guard-count assert unchanged (game-config guards
  aren't in `twin_guards()`).
- `test_g4_final_objects_port.rs`: dropped `_dplc_ptr`/`_art_base`/`_debug_flag`
  equ supplies; `test_player` guard-count assert `+4 → +1` (VRAM_TEST_SONIC only).
- `test_g1_objects_port.rs`: dropped `_dplc_ptr`/`_art_base` supplies;
  `test_animated` guard-count assert `+3 → +1`.
- `test_p1_player_port.rs`: dropped the 5 `_pl_*` equ supplies (player_common.emp
  now DEFINES `_pl_state` as an `equ` — supplying it would double-define). Loose
  guard assert (`>20`) still holds (PSTATE_*/ANIM_*/config mirrors remain).
- `test_p2_player_states_port.rs`: dropped `BUTTON_JUMP_MASK` + 8 `PPHYS_*` + 5
  `_pl_*` supplies (state-file guards retired; no guard-count assert in p2).
- `camera_port.rs`: **UNCHANGED** — camera still `extern`s `_pl_state` (value $30);
  the port test still supplies it standalone (player_common.emp not in that
  compile). Only camera.emp's comment updated (source is now the `.emp` equ).
- `native.rs`: 4 registry comments + the config_b comment updated (the twins
  flipped; z80_init's no-sound arm takes the numeric path).

## §5 — STEP-3 (retrospect) vs STEP-5 (engine optimization) FINDINGS

- **Step-3:** the flip removed the last four AS code twins and **≈18 drift-wall
  `ensure`s** across 6 `.emp` files (5 `_pl_*` + 5+9 `PPHYS`/`BUTTON` + 6
  DplcV/STUB + 1 ENEMY) plus the `z80_init_port` oracle — a clean reduction in
  twin scaffolding with zero behavior change. It also proved the **`.emp`-`equ`
  link-export as the engine→game late-bind mechanism** (camera's `_pl_state`),
  which generalizes the sound_api extern-equ-sum idiom to a `.emp`-owned game
  symbol — the pattern any future engine-reads-game-offset seam will reuse. Kill-
  list rows 55/85/61/84/72/74/75 CLOSED; 76/81 advanced to config-file-mirror-only.
- **Step-5:** none. Pure ownership move; no lowering changed, no bytes moved. The
  §17 opt-sweep owns byte-changers.

## §6 — NEITHER-BUCKET HEADLINES

- **The four `SIGIL_EMP_*` gates are now fully inert** (no `.asm` reads them) but
  stay armed in `code_gate_defines()` + repin.toml, matching the 52 already-flipped
  regions. They are dead comments awaiting the residual-split capstone (Parcel K),
  exactly like the engine.inc/main.asm resume `org`s (which these flips now feed
  four more of).
- **The four macros in player_common.asm were DEAD** — `setStandingSize`/
  `setBallSize`/`maskOpposingLR`/`distToFix` had NO external consumer (only
  player_common.asm's own gated-off body used them). Deleting the `.asm` removed
  them; the `.emp` `comptime fn` templates were already the sole live definitions.
- **`_pl_state` is the ONLY `_pl_*` offset any external module needs** — the census
  premise (kill-list row 74: "the surviving `_pl_*` readers vanish") missed that
  engine.camera reads `_pl_state` cross-seam. The other four (`_pl_gsp`/
  `_pl_move_lock`/`_pl_spindash`/`_pl_stick_convex`) were read ONLY by
  player_common.emp's own now-removed self-guards; the state files reach every
  field through the imported `PlayerV` struct.
- **`ENEMY_PATROL_SPEED`'s natural home is `test_objects.emp`, not `test_enemy.emp`**
  — the value is the ObjDef_Enemy initial x_vel (a data-table value), and
  test_enemy.emp uses `ENEMY_PATROL_RANGE`, never the speed. So the flip put it in
  its sole consumer rather than the behavior module.

## §7 — GAP-LEDGER SWEEP (nice-to-haves NOT implemented → campaign-gap-ledger.md)

1. **`PPHYS_*` are duplicated file-local consts** in player_ground.emp (5) +
   player_air.emp (4). Kill-list row 81's deferred hoist: move to player_common.emp
   `pub const` + `use` (byte-neutral; the state files already import from that
   module). Region byte gate catches drift meanwhile.
2. **The player-state offset tables keep the extern-difference form** — the
   offsets-DSL cross-module `Ref` path (ledger 1767) stays its dedicated parcel /
   Spec 5, per the overseer's pre-ruling. Recorded here as the DSL deferral.
3. **Stale `player_common.asm`/`test_player.asm`/`test_enemy.asm` "truth" comment
   references** survive in untouched sigil port tests (e.g. `animate_port.rs`'s
   outbound-consumer comment). Fix-at-next-touch per the exhibit-comments rule.

## §8 — KILL-LIST CLOSURES (same-commit)

| row | twin | status |
|---|---|---|
| 55 | z80_init | ✅ CLOSED (twin deleted; oracle retired; re-homed to config_b whole-ROM) |
| 85 | test_enemy | ✅ CLOSED (twin deleted; ENEMY_PATROL_SPEED → test_objects.emp) |
| 61 | DplcV overlay | ✅ CLOSED (`.emp` overlays become the definitions) |
| 84 | test_player scaffolding | ✅ CLOSED (twin deleted; header equates `.emp`-owned) |
| 72 | player_common internal gate + twin | ✅ CLOSED (twin deleted; `_pl_state` link-export) |
| 74 | PlayerV overlay + 5 guards | ✅ CLOSED (overlay owned; guards removed) |
| 75 | 4 macro templates | ✅ CLOSED (AS macros deleted; `.emp` fns sole) |
| 76 | player_common game-config mirrors | PARTIAL (BUTTON_JUMP_MASK flipped; PSTATE_*/ANIM_*/SFXID mirrors → Parcel F) |
| 81 | ground/air PPHYS + config mirrors | PARTIAL (PPHYS/BUTTON guards retired; config mirrors → Parcel F; PPHYS hoist → gap-ledger) |

## §9 — FILE MANIFEST

**aeon (`conv-d-twins`), 4 commits:**
- #13: `engine/system/boot_data.asm` (M), `engine/system/z80_init.asm` (D)
- #47: `games/sonic4/objects/test_enemy.asm` (D), `test_enemy.emp` (M),
  `data/objdefs/test_objects.emp` (M), `main.asm` (M)
- #48: `games/sonic4/objects/test_player.asm` (D), `test_player.emp` (M),
  `test_animated.emp` (M), `test/object_test_state.emp` (M), `main.asm` (M)
- #49: `games/sonic4/player/player_common.asm` (D), `player_common.emp` (M),
  `player_ground.emp` (M), `player_air.emp` (M), `engine/level/camera.emp` (M),
  `main.asm` (M)

**sigil (`conv-d-twins`), 4 commits:**
- #13: `crates/sigil-cli/tests/z80_init_port.rs` (D), `native.rs` (M)
- #47: `test_objects_port.rs` (M), `native.rs` (M), `twin-scaffolding-kill-list.md` (M)
- #48: `test_g1_objects_port.rs` (M), `test_g4_final_objects_port.rs` (M),
  `native.rs` (M), `twin-scaffolding-kill-list.md` (M)
- #49: `test_p1_player_port.rs` (M), `test_p2_player_states_port.rs` (M),
  `native.rs` (M), `twin-scaffolding-kill-list.md` (M)
- close: `campaign-gap-ledger.md` (M), this note.
