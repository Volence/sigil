# 2026-07-30 — FLIP STAGE 2 · CLOSE PACKET (the point of no return, closed)

Worktrees at close: sigil `flip-stage2` (this note's commit) · aeon `flip-stage2`
`5fc6ba1`. Baseline at open: masters aeon `bcb8f64` / sigil `9f40dc6`.

## 0. THE OVERSEER'S CLOSURE RULING (the frame)

Stage 2 CLOSES HERE with its essential claims achieved:

- **`sigil build` IS the build** — asl / p2bin / fixheader are out of the default
  pipeline (`build.sh` drives one `sigil build --native` invocation:
  assemble → declared-order link → emit_rom with the checksum folded →
  sigil-canonical `.lst` → convsym deb2 appendix → full ROM). convsym survives
  (it consumes sigil's listing).
- **~55 of the ~60 code twins deleted** — 54 `.asm` twin files removed across 10+
  byte-neutral subsystem groups (§4), each proven at the four/six native pins.
- **The test suite transformed** — the AS-reassembly oracle family retired; the
  row-91 DSM composition witness hardened (bars a-d) and PRESERVED per the Volence
  ruling; t24 positive-control/negative-probe discipline kept verbatim on every
  surviving golden gate (§3).
- **The six goldens hold at every commit** (§1).

**THE DEFERRED TAIL = THE STAGE-3 OPENING ITEM**, behind its own designed gate:
(a) the five remaining AS-side code files — player_common, test_player, test_enemy
(the keystone code halves), act_descriptor.asm, z80_init.asm — whose flips are
proven CRC-MOVING re-baseline events (the sigil-canonical appendix shrinks ~1389 B
when the `.emp` keystone locals mangle `$module$Proc$local` instead of the AS
frontend's `Proc.local`) AND expose a real chainer bug (config_a anchor divergence
at 0x11412 when the keystones enter the chained set — the assembled-anchor bar
makes this a FIX, not a re-baseline); (b) the folded Phase-B cargo (AS-residual
section-split, the $20000 map-region budget, pins→map placement authority, repin's
asl-`.lst`-parse retirement / row 34); (c) the entangled machinery that stays alive
meanwhile (the STAGE1 drift-guard allowlist, `as_owned_keystones`); (d) the
`engine/structs.asm` CODE-twin arm. These files remain legitimate residual-AS under
the frontend-as-permanent ruling until that gate.

## 1. THE PROOF MATRIX — six targets, both layers, pre/post

Two layers per target: the **assembled anchor** (pre-convsym region bytes; the
PRIMARY provenance the byte gate emits — header-neutral) and the **full file** (the
default build's ROM, including the checksum and the sigil-canonical convsym deb2
appendix).

| target | assembled anchor (PRIMARY) | full file (default build output) | size |
|---|---|---|---|
| sonic4 plain | `e5765873` | `2198deb2` | 395374 |
| sonic4 debug | `dab4f06c` | `1d895fcb` | 402696 |
| demo plain | `cfda98d3` | `0646d4bf` | 76851 |
| demo debug | `20c5571d` | `7e4a358a` | 77244 |
| config_a (debug+hotkeys+mirror) | `3d9bac53` | `80e602df` | 402742 |
| config_b (silent sonic4) | `fd3f7f8e` | `9eb2e8a1` | 286904 |

**Pre-flip → post-flip.** Pre-flip the default `./build.sh` ran asl and its ROM's
provenance was the assembled anchor. Post-flip the default `sigil build` emits the
**full file** (sigil-canonical). The transition moved ONLY the full-file/appendix
layer: **the six assembled anchors did NOT move through the entire parcel** — every
native region reproduces byte-for-byte, so the flip is byte-neutral at the layer the
game's behavior lives in. The full-file CRCs are the new default artifacts (they
differ from the old asl full files only in the convsym appendix, which is now
sigil-emitted rather than convsym-off-asl-listing).

**Verification, this close** (scratch `sigil build --native` per target — never
`./build.sh` in-tree; the stale-artifact-trap-guarded proof, all six, exit-checked):

```
sonic4_plain: OK 2198deb2/395374    demo_plain: OK 0646d4bf/76851    config_a: OK 80e602df/402742
sonic4_debug: OK 1d895fcb/402696    demo_debug: OK 7e4a358a/77244    config_b: OK 9eb2e8a1/286904
```

Both layers are carried by the surviving native gates: `native_full_rom` +
`native_offcanonical_full` assert the full files vs the frozen goldens;
`native_rom` + `native_offcanonical_placement`/`_rom` assert the assembled anchors;
the S1.4 functional gate family + t24 controls ride each.

## 2. THE ARTIFACT LEDGER

- **Default build output, post-flip = the SIGIL-CANONICAL full files** (the six
  full-file CRCs above). This is the artifact a fresh `./build.sh` / `sigil build`
  now produces.
- **PRIMARY assembled anchors = UNMOVED throughout** (the six anchor CRCs above).
  These are the frozen `crates/sigil-harness/golden/` region blobs; they were the
  correctness oracle before the flip and remain the invariant the whole parcel was
  measured against. Not one moved.
- The frozen goldens in `crates/sigil-harness/golden/` (`s4.bin`, `s4.debug.bin`,
  the demo/config blobs, `PROVENANCE.md`) are the surviving regression oracle; the
  witness comparands re-point tree→golden (§3) so the composition proofs stay
  independent of the now-sigil-built tree ROM.

## 3. TEST-SUITE TRANSFORMATION (itemized, whole parcel)

**Strict trajectory** (`cargo test --release --no-fail-fast`, worktree pair, the
native/region binaries run with `AEON_DIR=<flip-stage2 aeon worktree>` +
`SIGIL_EMIT`):

| boundary | count | cause |
|---|---|---|
| pre-D green boundary (`97a9127`/`08d6a62`) | 2939 / 0 | — |
| after D1 (row-91 witness) | **2944** / 0 | +5 row-91 t24 doctored probes added (DAC body+head, SFX head, seq_opcode_tab, sound_tables_z80, pitchtable) |
| after D2 (reassembly family retired) | **2862** / 0 | −82: the AS-twin-reassembly oracle family retired (coverage subsumed by the native whole-ROM goldens + surviving `.emp`-region gates + the row-91 witness) |
| after engine/objects | **2858** / 0 | −4 AS-twin lockstep oracles (sprites/animate/dplc/rings `*_matches_as_twin`) |
| after engine/system | **2857** / 0 | −1 game_loop AS-twin combo matrix (`combo_matrix_matches_as_twin` + `as_twin_bytes`) |
| after game test/debug (close) | **2854** / 0 (1 ignored) | −3 game_debug_port AS-twin (`as_twin_bytes` reader + `emp_diverges_from_doctored_twin` + `game_debug_{plain,debug}_is_empty`; `matches_as_twin` transformed) |

Intermediate groups (math, compression, engine/level, engine/debug specials,
engine/sound, player-state, game objects, game data) are **net 0** — no AS-half read
the deleted twin, so deletion alone kept them green.

**By class (whole parcel):**

- **Retired — AS-reassembly oracle files (whole `git rm`):** `m1d_rom`,
  `m1d_debug_rom`, `m0_regions`, `mixed_dac_rom`, `mixed_offcanonical_rom` (harness);
  `seam2_dac_rom`, `seam2_mt_rom`, `seam2_sfx_rom`, `test_t1_harness_states_port`
  (cli). Plus the harness reassembly fns (`assemble_full_rom*`, all
  `assemble_mixed_*_as_side`, `assemble_seam2_*_rom_as_side`) and the legacy
  no-`--native` build path + `run_diff`.
- **Retired — partial (twin-inclusive tests dropped, witness/region tests kept):**
  `seam1_native_link` (mixed_seam1 + `build_seam1_rom`), `vblank_port` (twin-parity
  + `as_full_module`/`run_twin_parity`), `boot_port` (twin-parity + `as_full_module`
  /`oracle_value`).
- **Retired — twin-parity arms (per subsystem):** objects −4, system −1,
  game_debug −3 (as above).
- **Added — row-91 witness t24 probes:** +5 (D1, the composition-vs-golden bars).
- **Transformed (comparand re-point / gate reshape, net 0):**
  `parcel_8b_stage_gen_touchers` (the `.asm`/`.inc` census half retired with
  tile_cache.asm; the `.emp` census is now the sole gen-bump audit);
  `boot_port` s4.lst crutch → frozen constants ($181C/$189A · $3 · $5C7EC/$5E2DA) +
  golden comparand; `math_port` → golden comparand; `game_debug_port::matches_as_twin`
  → `.emp` compile + guards-pass + non-empty gate. The row-91 witnesses re-comparand
  live `aeon/s4.bin` → frozen `golden/s4.bin`.
- **Preserved verbatim:** every surviving golden gate keeps its t24
  positive-control + negative doctored-probe (what stops the goldens going vacuous).

**Close count: 2854 passed / 0 failed / 1 ignored.**
NOTE for re-runners: 9 native/region CLI binaries (`native_full_rom`, `native_rom`,
`native_chained_resume`, `native_declared_chain`, `native_offcanonical_{full,rom,
placement}`, `entity_window_port`, `rings_port`) default `AEON_DIR` to the MAIN aeon
checkout; a bare `cargo test` scans the pre-flip master tree and reports 28 spurious
failures. Point `AEON_DIR` at the flip-stage2 aeon worktree (and set `SIGIL_EMIT`) —
all 28 pass, giving the 2854/0/1.

## 4. DELETION INVENTORY (every deleted `.asm`, grouped by commit)

54 code-twin `.asm` files, ~14,747 source lines, across the aeon `flip-stage2`
commits. (`main.asm` edits are gate collapses, not deletions.)

**`de41581` — compression (D3), 2 files, 347 lines:**
`engine/compression/s4lz_decompress.asm` (234), `zx0_decompress.asm` (113).

**`0e7f722` — math, 1 file, 27 lines:** `engine/system/math.asm` (27).

**`246018c` — engine/objects, 9 files, 4613 lines:** `entity_window.asm` (1572),
`children.asm` (661), `core.asm` (611), `sprites.asm` (656), `collision.asm` (316),
`animate.asm` (282), `rings.asm` (291), `dplc.asm` (121), `load_object.asm` (103).

**`a05dd0c` — engine/level, 9 files, 4380 lines:** `tile_cache.asm` (1818),
`parallax.asm` (902), `section.asm` (559), `plane_buffer.asm` (418), `camera.asm`
(272), `bg_anim.asm` (151), `load_art.asm` (120), `bg.asm` (104),
`collision_lookup.asm` (36).

**`8d985e7` — engine/system, 9 files, 1133 lines** (z80_init DEFERRED):
`boot.asm` (243), `dma_queue.asm` (267), `vblank.asm` (206), `buffers.asm` (191),
`controllers.asm` (64), `vectors.asm` (47), `vdp_init.asm` (46), `hblank.asm` (44),
`game_loop.asm` (25).

**`5c95d5b` — engine/debug specials, 3 files, 406 lines:** `error_handler.asm` (209),
`sound_debug.asm` (103), `compression_selftest.asm` (94).

**`fea702c` — engine/sound, 1 file, 369 lines:** `sound_api.asm` (369).

**`6d82a32` — player-state, 5 files, 1919 lines:** `player_ground.asm` (783),
`player_sensors.asm` (493), `player_air.asm` (470), `player_spindash.asm` (119),
`sonic.asm` (54). (+ `main.asm` gate collapse −51/+24.)

**`8566bd2` — game object/data/test, 15 files, 1553 lines:** `object_test_state.asm`
(365), `ojz_scroll_test.asm` (310), `test_parent.asm` (205), `path_swap.asm` (132),
`game_debug.asm` (121), `test_churn.asm` (85), `sonic_anims.asm` (78),
`test_stress_emitter.asm` (51), `test_emitter.asm` (50), `test_animated.asm` (48),
`test_particle.asm` (46), `test_solid.asm` (22), `particle_anims.asm` (15),
`test_objects.asm` (14), `test_static.asm` (11). (+ `main.asm` gate collapse −178/+79.)

Plus the aeon flip commits carrying no `.asm` deletion: `d5d4ebf` (build.sh gains the
SIGIL_NATIVE path), `97a9127` (THE FLIP: sigil build IS the build), `22513f4`
(objdef.emp imports engine.constants by its real id), `5fc6ba1` (verify_emit_bin.py
retired — §6).

## 5. KILL-ROW CLOSURES (STAGE-2 EXECUTION LOG cross-check)

The kill-list `STAGE-2 EXECUTION LOG` tracks each subsystem's closures same-commit;
the roll-up:

- **Rows 5 / 6** — progressively closed per subsystem: 54 code twins deleted; each
  `ifndef SIGIL_EMP_X` gate collapsed to a bare resume-`org` block. Row 6 downgraded
  from "mirror" (a re-pin tax) to "residual placement literal" (no dual build to keep
  in lockstep). Row 5 STAYS OPEN for the 5 deferred code files + `structs.asm`; row 6
  STAYS OPEN until the map-manifest owns placement (Phase-B cargo).
- **Rows 10 / 24 / 36 / 43** — CLOSED (engine/sound group `fea702c`): sound_api.asm
  gone; the `.emp` slot-address extern-equ sums + immediate mirrors + the
  stop_z80/start_z80/sr_masked comptime-fn templates are now the sole spelling.
- **Rows 52 / 90** — CLOSED (engine/debug specials `5c95d5b`): the
  `ErrorHandler = ErrorHandlerBlob` link alias + per-shape EndOfRom orgs are the only
  spelling; the numeric ErrorHandler/Game_Entry equs resolve by link (precursor
  `28098af` flipped `_is_unresolved_today` → `_resolves`).
- **Row 59** — CLOSED (sound_debug.asm gone).
- **Row 73** (sonic keystone whole-file gate) — CODE half CLOSED (sonic.asm deleted,
  player-state group); the game-config mirror (row 77) is Stage 3.
- **Row 86** (path_swap) — CODE half CLOSED (deleted, game objects group).
- **Rows 6 / 58 / 88 / 89** — ADVANCE (game test/debug `8566bd2`): object_test_state
  / ojz_scroll_test bodies deleted + per-shape org arms collapsed to residual
  placement literals; game_debug is the Config-A-only collapse. The overlay/const
  halves (88/89) and the off-canonical z80_init org arm (58) are Stage 3.
- **Row 9** — OPEN: game_loop's AS-twin combo matrix retired, but the gameDebugTick
  H2-mirror is the Stage-3 game-contract-hook item.
- **Row 34** — OPEN: asl listings as address ground truth — the core Stage-3 flip
  item (repin's `.lst` parse + convsym's asl-listing source).
- **Row 35** — OPEN (carried): the OJZ per-frame mode-register force-write, ported
  verbatim into `ojz_scroll_test.emp`; closed by the parallax-hardening parcel or
  Spec-5-carried.
- **Row 55** — OPEN (DEFERRED): z80_init — the no-sound Z80 idle body + its
  `Z80_IdleProgram` comptime-assert-wall consumer; needs the off-canonical native
  driver to place `z80_init.emp` and reverse-seam-export `Z80_IdleProgram`.
- **Rows 72 / 74 / 75 / 76 / 84 / 85** — DEFERRED (keystone class, CRC-moving): the
  player_common / test_player / test_enemy code halves + their always-emitted
  zero-byte headers stay AS-side; their `_port` region gates survive as the proof.
- **Row 91** — OPEN (WITNESS HARDENED + PRESERVED per the Volence ruling): the DSM
  in-memory bank-composition witness rebuilt to bars a-d (recompute from `.emp`,
  assert vs frozen-golden slice, t24-doctor to diverge, cover every `.emp`-sole
  bank); the `SIGIL_EMP_*_BODY_STUB` collapse itself stays DEFERRED (coverage-reducing).

## 6. verify_emit_bin.py RULING — RETIRED (aeon `5fc6ba1`)

**Ruling: RETIRE.** `tools/verify_emit_bin.py` byte-compared each generated sound
`.asm`'s `dc.b`/`db`/`pbyte` payload against its `--emit-bin` `.bin` twin. Every
subject `.asm` is now deleted: the Moving-Trucks `.asm` retired at seam-2 stage-2c,
the 20 SFX `.asm` at seam-2 stage-2d. `_FIXED_TARGETS` is empty and the `sfx/` dir
holds only `.bin`, so the verifier discovers **0 targets and passes vacuously** —
there is no twin left to drift. The `.bin` blobs are single-source now (design §1.5):
the sigil build BINCLUDEs them directly (`main.asm` / `boot_data.asm` /
`sound_bank.inc`), and the emitters' `--emit-bin` mode is their sole producer.

Executed as its own commit: `git rm tools/verify_emit_bin.py`; removed the build.sh
preflight block; reworded the build.sh comment and the two `song_packer.py`
docstrings + the `verify_level_bin.py` header that named it. **Byte-neutral** — all
six native targets held at their pinned CRCs after. `verify_level_bin.py` (the
level-tree drift check) STAYS — its subjects (the committed OJZ tree) are live.

## 7. PER-PASS step-3 vs step-5 FINDINGS + neither-bucket headlines

This was a DELETION parcel (the ports themselves landed t1-t41 / seam-1/2 / Stage-0/1),
so step-3 (retrospect) and step-5 (engine-optimize) surfaced little new — by design,
the flip must not change bytes.

- **step-3 (retrospect / language asks):** none new. The comptime-fn template rows
  (23/26/27/28/37/40/41/44) and macro-mirror rows all resolve mechanically — the
  `.emp` fn becomes the sole spelling once the twin dies; no construct gap surfaced.
- **step-5 (engine optimize):** deliberately NONE — every deletion is byte-proven at
  the assembled anchor. The §17 half-cost optimization sweep is explicitly the
  post-flip arc (Stage-3+), where the oracle A/B rig measures real wins; folding any
  optimization into the flip would break the byte bar that IS the flip's safety.
- **Neither-bucket headlines** (the load-bearing findings that are neither step-3 nor
  step-5):
  1. **The keystone flip is CRC-MOVING, two independent ways** — the appendix shrinks
     ~1389 B (local-label mangling `$module$Proc$local` vs `Proc.local`) AND the
     Frozen chainer misplaces a config_a data pointer (anchor diverges at 0x11412,
     sig `0x12` != gold `0x13`). This is the designed Stage-3 gate, NOT a bug in the
     flip — reverted clean; the headers stay AS-side.
  2. **The chainer has a real bug** the keystone attempt exposed: config_a's anchor
     divergence at 0x11412 is a contiguity/label-derivation defect that the
     assembled-anchor bar makes a FIX (not a re-baseline). Rowed for Stage 3.
  3. **The stale-artifact trap** (MEMORY-flagged): a failed `sigil build` leaves the
     `-o` file untouched, so CRC'ing it reads a prior success as green. The scratch
     proof now `rm`s the output + checks the exit code, all six targets. Any
     native-driver-coupled deletion MUST use the fixed proof.
  4. **The AEON_DIR default trap** (this close): the native/region CLI gates default
     `AEON_DIR` to the MAIN aeon checkout; a bare strict run scans the pre-flip tree
     and reports 28 spurious failures. The true strict needs `AEON_DIR` at the
     worktree — documented in §3 for every future closer.

## 8. STAGE-3 HANDOFF

### 8.1 The overseer's closure ruling (the deferred tail is the Stage-3 opening)

Stage 2 closes with `sigil build` as THE build, ~55 twins deleted, the suite
transformed, the six goldens holding. The Stage-3 OPENING item is the deferred tail,
behind its own designed gate:

- **(a) The five remaining AS-side code files** — player_common, test_player,
  test_enemy (keystone code halves), act_descriptor.asm, z80_init.asm. Their flips
  are proven CRC-MOVING re-baseline events (appendix shrinks ~1389 B) AND expose the
  real chainer bug (config_a anchor divergence at 0x11412) — the assembled-anchor bar
  makes that a FIX. Flip = add the gate to `code_gate_defines` + add the ModuleSpec +
  drop from `as_owned_keystones` + reconcile the chainer + re-freeze the six pins.
  z80_init additionally needs native no-sound placement + the `Z80_IdleProgram`
  reverse-seam export to boot_data's comptime wall.
- **(b) The folded Phase-B cargo** — AS-residual section-split at the natural
  boundaries, the $20000 object-bank budget as a `sigil.map.toml` map-region check,
  pins→map placement authority, repin's asl-`.lst`-parse retirement (row 34).
- **(c) The entangled machinery that stays alive meanwhile** — the STAGE1 drift-guard
  allowlist (`STAGE1_INAPPLICABLE_GUARDS`, native.rs:817) and `as_owned_keystones`;
  they retire WITH the keystone flip (retiring the allowlist alone needs the
  Poison-producing `.emp` guards removed, which is multi-file and entangled).
- **(d) engine/structs.asm CODE-twin arm** — the ownership flip (rows 7/11/25):
  `sst.emp`/`act_descriptor.emp`/`EntityScanState` become the definition; residual
  AS takes exported equs.

These files are legitimate residual-AS under the frontend-as-permanent ruling until
that gate.

### 8.2 The pre-existing Stage-3 items (from the flip design)

- **Physical toolchain removal:** delete `tools/asl` + `tools/p2bin` (kept out of
  this parcel to stay focused; fixheader already left the default path with the flip).
- **Ownership flips:** `engine/constants.asm` (row 1 twin family — the largest mirror
  block) + `engine/structs.asm` (rows 7/11/25) → `.emp` becomes the definition.
- **A game-constants `.emp` module is born** (design §3.2-C): absorbs the SONG_*/
  SFXID_*/VRAM_*/BUTTON_* + PPHYS mirror truths (rows 18/22/54/62/65/76/77/79), so
  `config/constants.asm` + `config/sound_ids.asm` can retire and the 2 typed SfxId
  mirrors (row 10 tail) close on the typed-extern grammar.
- **The debug-runtime rewrite** (rows 21/52/53): the `.emp`-native diagnostics runtime
  owns the message format + `b<cond>.w` pin once `debugger.asm`'s macro tower dies.
- **repin's `.lst` machinery** (row 34): sigil's own symbol table becomes address
  ground truth; repin's `repin.rs:57-58` parse + the debug-`.lst` cp/suffix machinery
  + convsym's asl-listing source are deleted.
- **The ledgered post-flip arc** (MEMORY / design §5, §17): the appendix demangler +
  name-quality audit (the ~1389 B keystone appendix delta lives here); the §17
  half-cost optimization sweep with oracle A/B; the ~20-ask language round; THE
  CAPSTONE SWEEP — then build the game.

### 8.3 Standing discipline for Stage 3

Per-commit bar unchanged: both games' native builds at their pinned CRCs (the
stale-artifact-trap-guarded six-target proof) + strict worktree pair green
(failures-first, explicit counts, `AEON_DIR` at the worktree), t24 verbatim on every
surviving golden gate + the row-91 witness. Deletion commits standalone + plain-spoken.
The valve stands; STOP on any guard relaxation, identity surprise, or design fork.
The keystone re-baseline needs explicit overseer authorization to move the pins.
Merges are the overseer's.
