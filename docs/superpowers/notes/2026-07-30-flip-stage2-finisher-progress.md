# 2026-07-30 — FLIP STAGE 2 FINISHER · progress note (valve stop at a commit boundary)

Span: the remaining twin-deletion groups + folded Phase-B cargo + close packet
(picking up after D1/D2/D3). Worktrees: sigil `.worktrees/flip-stage2`, aeon
`.worktrees/flip-stage2`. This note is the handoff for the next session.

## DONE this session (all byte-neutral across ALL SIX targets; strict green each commit)

Verification bar met after every commit: scratch `sigil build --native` (never
`./build.sh` in-tree) for sonic4/demo ×{plain,debug} + config_a + config_b at
their pinned CRCs (2198deb2/1d895fcb · 0646d4bf/7e4a358a · 80e602df · 9eb2e8a1)
+ strict `--no-fail-fast` failures-first with explicit totals.

- **Task 0 — boot_port s4.lst crutch (sigil `f7bf095`):** the 3 golden-backed
  boot_port tests no longer parse Z80_SOUND_SIZE/GAME_ENTRY_ID/Game_Entry from the
  tree's `s4.lst` (sigil-canonical now, asl can't regenerate). Pinned as frozen
  constants w/ provenance ($181C/$189A · $3 · $5C7EC/$5E2DA); region comparand
  re-pointed tree→golden. Sweep: boot_port was the ONLY test reading the
  asl-canonical tree `s4.lst` as a value oracle (repin's `.lst` parse = Task 3 /
  row 34; m1b_gate self-generates its listing from `emit_listing`).
- **math** (aeon `0e7f722`, sigil `5df8e6d`): `engine/system/math.asm`. pcrel_port
  keeps its INLINE AS/EMP parity snippets (never read the file).
- **engine/objects** (aeon `246018c`, sigil `736dfe3`): 9 files (dplc/core/sprites/
  animate/collision/rings/entity_window/children/load_object). Dropped 4 AS-twin
  oracles (sprites/animate/dplc/rings `*_matches_as_twin`); t24 doctored-mirror
  probes SURVIVE. aabb.inc survives. 2862→2858.
- **engine/level** (aeon `a05dd0c`, sigil `48d4b88`): 9 files (plane_buffer/
  tile_cache/collision_lookup/section/camera/parallax[CODE]/load_art/bg/bg_anim).
  No AS-reassembly halves. Transformed ONE tree-scanning parity audit:
  `parcel_8b_stage_gen_touchers` — its `.asm/.inc` census half retired (TileCache
  routines lived in the deleted tile_cache.asm), `.emp` census is now the sole
  gen-bump audit. Net 0 tests.
- **engine/system** (aeon `8d985e7`, sigil `bb93172`): 9 files (boot/buffers/
  controllers/dma_queue/game_loop/hblank/vblank/vdp_init/vectors). vectors = the
  fixed-size no-`ifdef` gate → bare `org $100`. Dropped game_loop's AS-twin combo
  matrix (row 9 stays open). 2858→2857. **z80_init DEFERRED — see below.**

Kill-list `STAGE-2 EXECUTION LOG` (appended to twin-scaffolding-kill-list.md)
tracks each subsystem's rows-5/6 closure.

## TWO FINDINGS THE NEXT SESSION MUST CARRY

1. **z80_init CANNOT be plain-deleted (deferred, NOT done).** `z80_init.asm` is
   the no-sound (Config-B + demo) Z80 idle program, pulled via boot_data.asm's AS
   `include` arm — the off-canonical profiles do NOT define SIGIL_EMP_Z80_INIT
   (config_b_profile comment: "the Z80 idle stays AS-side"). It also DEFINES
   `Z80_IdleProgram`, consumed by boot_data.asm's COMPTIME layout-assert wall
   (`if (Z80_IdleProgram-BootData) <> 54`). Deleting it + collapsing the gate
   breaks EVERY no-sound build (CLI and harness) with "unresolved long
   expression" — z80_init.emp is placed by NO ONE for no-sound, and a cross-seam
   `.emp` link can't satisfy a same-file comptime `if`. **To flip it:** the
   off-canonical native driver (native.rs config_b/demo profiles) must place
   `engine.z80_init` in its registry AND export `Z80_IdleProgram` back to the
   boot_data assert wall (reverse-seam), OR the assert wall relaxes. A distinct
   Stage-2 sub-item, same class as the keystone flip. Row 55 stays OPEN.
   z80_init_port's AS-twin oracle survives as the region proof meanwhile.

2. **Stale-artifact trap in the CRC proof (fixed).** A failed `sigil build`
   leaves the `-o` file untouched → CRC'ing it reads a PRIOR success as green.
   This masked the z80_init breakage for one cycle. The proof
   (`scratchpad/crcproof.sh`) now `rm`s the output, checks the build exit code,
   and covers all SIX targets. Any native-driver-coupled deletion (keystones,
   z80_init, error_handler alias) MUST be proven on all six with the fixed proof.

## REMAINING WORK (ordered; the delicate specials are deliberately fresh-session)

Twins still present (deletable):
- **engine/debug** — `compression_selftest.asm` (DEBUG-only gate + it keeps the
  `include engine/debug/generated/vectors.asm` in the debug arm — do NOT drop
  that; compression_selftest_port reads generated/vectors.asm for CSELF values,
  NOT the twin), `error_handler.asm` (SPECIAL: the gate-ON arm defines
  `ErrorHandler = ErrorHandlerBlob` link alias + `org EndOfRom` + mddbg_symbols
  derives MDDBG__ off it — rows 52/90; error_handler_port has a
  `derived_equ_off_external_base_is_unresolved_today` test to reconcile),
  `sound_debug.asm` (Config-A-only, nested under DEBUG+SOUND; row 59). KEEP
  debugger.asm + mddbg_symbols.asm (buckets D/E).
- **engine/sound** — `sound_api.asm` (SPECIAL: nested under SOUND_DRIVER_ENABLED +
  own gate; rows 10/24/36/43; sound_api_port assembles synthetic strings only, no
  twin read).
- **player state** — player_sensors/player_ground/player_air/player_spindash/sonic
  (main.asm gates; test_p1/p2/p4 read .emp + ROM, NO twin reads → clean, but VERIFY
  no comptime-assert dep like z80_init). NOTE main.asm player gates use 2-space
  `include` indent + some are `ifndef … include … endif` with the resume org — the
  collapse script targets `    ifndef`/`    endif`; main.asm may need hand edits or
  a tweaked matcher.
- **player keystones** — player_common/test_player/test_enemy: UNCONDITIONALLY
  AS-included (no gate), in `as_owned_keystones`. FLIP = git rm + remove from
  as_owned_keystones (native.rs sonic4/config_a/config_b profiles) + add to the
  registry. NATIVE-DRIVER change — z80_init-class risk; prove on all six.
- **game objects** — path_swap/test_animated/test_churn/test_emitter/test_parent/
  test_particle/test_solid/test_static/test_stress_emitter (main.asm gates;
  test_g1..g4/objdef/test_objects_port; agent found NO twin reads).
- **game test/debug** — object_test_state (per-shape org arm, row 6/88),
  ojz_scroll_test (row 89, per-shape org, kill-row-35 stays), game_debug (SPECIAL:
  Config-A-only, main.asm gate + org $6408 row 58; game_debug_port HAS an AS-half —
  as_twin_bytes L188-221 + 4 tests: game_debug_matches_as_twin_at_hotkeys_shape,
  emp_diverges_from_doctored_twin, game_debug_{plain,debug}_is_empty).
- **game data** — data/animations/sonic_anims.asm, particle_anims.asm,
  data/objdefs/test_objects.asm (SIGIL_EMP_OBJDEFS gate), act_descriptor.asm
  (works against residual-AS entity_data per the levelgen ruling — CHECK the
  entity_data.asm coupling).
- **z80_init native flip** (finding 1).

Then: **Task 2** scaffolding finale (main.asm/engine.inc gate collapse remainder;
the 4 drift guards + STAGE1_INAPPLICABLE_GUARDS allowlist retire — native.rs:817,
their twins bg.asm/camera.asm are already gone so the guards still fold to Poison
consistently, safe to retire; objdef.emp import-id one-liner; rows 6/58/52/90/91).
**Task 3** folded Phase-B cargo (AS-residual section-split; $20000 object-bank
budget as a map-region check; pins→map placement authority; repin's asl-.lst-parse
retirement = row 34; declared-sizes doctrine stands). **Task 4** verify_emit_bin.py.
**Task 5** close packet.

## AS-half surgery map (from the mapping-agent sweep — authoritative)

Only these test files read+assemble a deleted twin (AS-reassembly halves): DONE =
sprites/animate/dplc/rings/game_loop. REMAINING = game_debug_port (above). ALL
OTHER twins have NO test that reads the `.asm` — deletion alone keeps them green
(tree s4.bin stays byte-identical to golden). The mass tree-`s4.bin`→`golden`
region-gate re-point (~45 files, Task-0 sweep mandate / D4+) is OPTIONAL hygiene
(tree stays byte-identical through deletions) — NOT yet done except boot_port +
math_port; ride it into groups or do as one mechanical commit. Whole-tree
`.asm`-scanning meta-tests (like parcel_8b) can surprise per-symbol — run strict
`--no-fail-fast` after every group.
