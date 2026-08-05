# 2026-08-01 — CONV-F PARCEL F: the game-config / P6 module (close packet)

Status: Merge state lives in the campaign log, not here. Branch pair
`conv-f-game-config` (aeon + sigil). **#21 `config/constants.asm` is FLIPPED to
`.emp` and DELETED; #22 `game.asm` and #24 `sound_ids.asm` are SCOPED with premise
corrections and remainder analysis but NOT yet flipped** (see §8). Every built
target byte-identical to the chain goldens; strict 2868 → 2867 (−1 retired probe).

## §0 — THE HEADLINE

`games/sonic4/config/constants.asm` is **DELETED**. Its ~30 game constants moved to
`games/sonic4/config/constants.emp` (module `games.sonic4.constants`) as `pub const`s,
the SOLE authority. Game-side `.emp` consumers (`player_common`/`player_ground`/
`player_air`/`player_spindash`/`sonic`/the `test_*` objects/`object_test_state`/
`ojz_scroll_test`/`test_objects`) drop their `const` mirror + `ensure(extern(…))`
drift guard and `use games.sonic4.constants`. A new sigil harvest
(`harvest_game_constants`) reads the module's pub consts and injects them as GUARDED
AS `-D` defines + link EquSyms — so the residual AS and the game-AGNOSTIC engine
`.emp` (`rings`/`entity_window`/`ram` drift guards, `sonic_anims` ordinal guards)
resolve them against the `.emp` authority. **Six-target byte-identical.**

## §1 — PREMISE CORRECTIONS (census vs reality, numbers-grounded)

The census (Parcel F, row 21) said: *"the untyped half folds through the reverse-seam
(link-position) — likely no guarded-define channel needed."* **Half right.**

1. **The reverse-seam claim HOLDS for game-side consumers.** `.emp` code that uses a
   game constant only in a LINK POSITION (`moveq #PSTATE_ROLL`, `dc.b ANIM_WALK`)
   leaves an unresolved immediate the joint link fills from the AS-exported symbol —
   `player_air.emp` used `moveq #PSTATE_ROLL` with NO local const or `use` and built.
   Post-flip these consumers `use games.sonic4.constants` (comptime fold) — byte-identical.

2. **The claim FAILS for the game-agnostic engine.** `rings`/`entity_window`/`ram`
   read game constants via **local `const` / `-D` + `ensure(extern("X") == X)` drift
   guards** — all `extern()` resolve against the AS config today. Deleting the `.asm`
   breaks every one UNLESS a **per-game harvest** (`harvest_game_constants`, the #7c
   RAM-harvest precedent) re-exports the `.emp` pub consts as link EquSyms. So the
   **guarded-define channel IS needed** — the census's "likely not" is wrong for this
   subset. The engine cannot `use` a game module (it is game-agnostic), so its guards
   cross-check its local copy against the harvested authority (KEPT, genuine).

3. **3 game-VARYING constants can never live in `.emp`.** `MAX_RING_BUFFER`,
   `VRAM_RING_PLACEHOLDER`, `COLLECTED_WINDOW_SLOTS` vary per game (sonic4 128/0x3E8/9,
   demo 16/0x3E4/4) and the engine reads them at COMPTIME (RAM sizing, `vram_art`,
   `moveq` immediates) — so they are per-profile `-D` (`native.rs emp_defines`). The
   codebase FORBIDS declaring a `-D` name in any `.emp` module (`validate_defines`
   `[defines.collision]` — a hard error, hit on the first build attempt). They were
   already dual-homed (native.rs `-D` + config.asm); the flip drops the redundant
   `.asm` home + the now-tautological extern guards, leaving native.rs the sole home
   (byte-identity is the drift net — a wrong `-D` moves ROM bytes).

4. **Effort was L, not M.** New `.emp` module + a new sigil harvest fn + ~18 aeon
   consumer edits + heavy sigil test ripple (7 port tests + the m1c harness root).

## §2 — THE FLIP (aeon)

- **NEW `games/sonic4/config/constants.emp`** (`module games.sonic4.constants`):
  `pub const` for GS_OBJECT_TEST, ANIM_RUN_THRESHOLD, SPINDASH_BASE/CHARGE_STEP/MAX,
  PSTATE_* ×8 (+ the curled-states-last `ensure`), ANIM_* ×12, GS_OJZ_SCROLL_TEST,
  RING_BUFFER_ENTRY_SIZE, RING_WIDTH, COLLECTED_SLOT_SIZE/PARK_SLOTS/PARK_ENTRY_SIZE
  (`= 1 + 2 * COLLECTED_MASK_BYTES`, via `use engine.constants`), VRAM_TEST_OBJ/
  MARKER/SONIC (`: VramTile`, via `use engine.types`). The 3 game-VARYING `-D`
  constants are DELIBERATELY absent (§1.3), documented in-file.
- `games/sonic4/config/constants.asm`: **DELETED**; `gameConfigIncludes` (main.asm)
  drops the include with a note.
- **Game-side consumers** (mirror + guard → `use`): `player_common.emp` (PSTATE_*/
  ANIM_*/ANIM_RUN_THRESHOLD/VRAM_TEST_MARKER), `player_ground.emp` / `player_air.emp`
  / `player_spindash.emp` (PSTATE_*/SPINDASH_*/ANIM_SPINDASH — SFXID mirrors left for
  #24), `sonic.emp` / `test_player.emp` / `test_animated.emp` (VRAM_TEST_SONIC),
  `test_churn`/`test_emitter`/`test_parent`/`test_stress_emitter`/`path_swap`/
  `test_objects`/`object_test_state`/`ojz_scroll_test` (VRAM_TEST_OBJ/MARKER).
  `sonic_anims.emp`'s `extern("ANIM_*")` ordinal guards are LEFT (genuine table-vs-id
  cross-check; extern resolves via the harvest).
- **Engine-side** (KEPT unchanged except retired game-VARYING guards): `rings.emp`
  retired the MAX_RING_BUFFER / VRAM_RING_PLACEHOLDER extern guards (kept
  RING_BUFFER_ENTRY_SIZE / RING_WIDTH); `entity_window.emp` retired the
  COLLECTED_WINDOW_SLOTS guard (kept SLOT_SIZE/PARK_*). All surviving guards now
  resolve against the `.emp` authority via the harvest.

## §2b — THE FLIP (sigil)

- `native.rs`: new `harvest_game_constants(aeon, rel)` — seeds `harvest_engine_constants`
  as defines first (so the module's lone cross-module ref, `COLLECTED_MASK_BYTES`,
  folds in the standalone `eval_all_pub_consts`), then reads the game module's pub
  consts. New `GameProfile.game_constants_rel: Option<&str>` (Some for the 4 sonic4
  profiles, None for demo — whose config is still `.asm`, Parcel H). `assemble_as_side`
  extends `guarded_defines` with it when Some.
- Port-test ambients gain `games/sonic4/config/constants.emp` (a flat-merge dep):
  `objdef_port`, `test_p1_player_port` (×2 lowerings), `test_g1`/`g2`/`g3`/`g4`.
- `rings_port`: guard-count `+4 → +2`; `doctored_game_mirror_fires_its_guard` RETIRED.
- `test_g1`–`g4`: the `+1 VRAM_TEST_*` guard drops from each count assert.
- `test_p1`: `p1_drift_guards_all_pass` `> 20 → >= 1` (only SFXID_SKID remains).
- `m1c_root.asm` drops the `config/constants.asm` include; `m1c_vector_table.rs`
  seeds `harvest_game_constants` alongside the engine harvests.

## §3 — THE BYTE-IDENTITY PROOF (all six)

| target | built | golden | match |
|---|---|---|---|
| s4          | `6cf74e65` / 412127 | `6cf74e65` / 412127 | ✓ (direct) |
| s4.debug    | `16615e46` / 421958 | `16615e46` / 421958 | ✓ (direct) |
| demo        | `9bb8c993` / 90506  | `9bb8c993` / 90506  | ✓ (direct) |
| demo.debug  | `bc7678d0` / 93006  | `bc7678d0` / 93006  | ✓ (direct) |
| config_a    | `78df5e6a` / 422297 | `78df5e6a` / 422297 | ✓ (strict native_offcanonical_full) |
| config_b    | `f38f609b` / 303501 | `f38f609b` / 303501 | ✓ (strict native_offcanonical_full) |

s4/s4.debug/demo/demo.debug verified directly (crc32). config_a/config_b via the
strict suite's whole-ROM byte-identity tests. Ownership move at unchanged bytes — no
re-freeze, no oracle A/B.

## §4 — RETIRED-TEST ENUMERATION (strict 2868 → 2867, net −1)

| retired test | file | re-homed to |
|---|---|---|
| `doctored_game_mirror_fires_its_guard` | `rings_port.rs` | six-target byte-identity (MAX_RING_BUFFER is a byte-affecting `-D` with no `.emp`/AS authority to doctor; the undoctored `rings_region_matches_reference{,_debug}` gates are the surviving proof) |

Net: −1 → **2867 passed / 0 failed / 4 ignored.** Count-neutral test edits: the
guard-count asserts in `rings_port` (4→2), `test_g1`–`g4` (`+1`→`+0`), `test_p1`
(`>20`→`>=1`) — the retired mirror guards, re-homed to the region byte gates + the
six-target byte-identity.

## §5 — STEP-3 (retrospect) vs STEP-5 (engine optimization)

- **Step-3:** removed ~30 `const` mirrors + ~26 `ensure(extern(…))` drift guards
  across 18 aeon `.emp` files, replaced by one authority module + `use`. The per-game
  harvest (`harvest_game_constants`) is the game analog of `harvest_engine_constants`
  — the mechanism a future new-game reuses. Kill-list rows 18 (advanced), 76/81
  (constants half CLOSED) discharged.
- **Step-5:** none. Pure ownership move; no lowering changed, no bytes moved.

## §6 — NEITHER-BUCKET HEADLINES

- **`GS_OBJECT_TEST` is DEAD** — declared in the old `constants.asm`, ZERO consumers
  in any `.asm`/`.emp` (grep-confirmed). Moved faithfully (ownership move, not a cull);
  a delete-candidate.
- **The 3 game-VARYING `-D` constants are a genuine game→engine interface** that the
  `.emp` grammar structurally cannot home (a `-D` name can never be an `.emp`
  declaration). This is not scaffolding to remove — it is the price of a game-agnostic
  engine (demo already carries its own `-D` set). Gap-ledger row logs the harness
  nicety that would fully home them.
- **`sonic_anims.emp` and `camera.emp` keep `extern()` guards, not `use`** — they
  cross-check a DERIVED quantity (an anim-table ordinal; a comptime-selected camera
  path) against the id contract; extern is the right tool, and the harvest keeps it
  resolving. Not every game-const reader flips to `use`.

## §7 — KILL-ROW CLOSURES (same-commit)

| row | scope | status |
|---|---|---|
| 18 | rings game-config mirrors | ADVANCED: RING_BUFFER_ENTRY_SIZE/RING_WIDTH guards cross-check the `.emp` authority (kept); MAX_RING_BUFFER/VRAM_RING_PLACEHOLDER guards + probe RETIRED (`-D` interface, byte-identity net) |
| 76 | player_common game-config mirrors | CONSTANTS HALF CLOSED (PSTATE_*/ANIM_*/VRAM_TEST_MARKER → `use`); SFXID_SKID → #24 |
| 81 | player_ground/air/spindash game-config mirrors | CONSTANTS HALF CLOSED (PSTATE_*/SPINDASH_*/ANIM_SPINDASH → `use`); SFXID_* → #24 |

## §8 — #22 game.asm and #24 sound_ids.asm — SCOPED, NOT FLIPPED

Empirical scoping (the census "untyped half" split, made precise):

**#24 `sound_ids.asm` — SPLIT.** The clean subset flips exactly like #21:
- **FLIP** (→ a `games.sonic4.sound_ids` `.emp`): `SFXID_*` (untyped ints, consumers
  sound_api/sound_sfx/player_ground/player_spindash/game_debug via drift-guard mirrors
  + the harvest), `SONG_*` (`if DEBUG == 1` for DRUMTEST/HCZ2; consumers mt_bank/
  game_debug/boot; game.asm's gameBootHook reads SONG_MOVINGTRUCKS but is NEVER INVOKED
  → no AS read), `SFXPRI_*` + the 7-bit guard, `SONG_COUNT`.
- **REMAINDER (a):** typed `SFXID_RING_*` — PRE-RULED deferred to the language round
  (SfxId newtype); `sound_api.emp` keeps its typed `const SFXID_RING_*: SfxId` mirror.
- **REMAINDER (b):** `SFX_ID_BASE` / `SFX_COUNT` / `SFX_TABLE_LEN` — read at AS-time by
  `engine/sound/sound_bank.inc` (`if (SfxBlobWinTab_End-…) <> SFX_TABLE_LEN*2 fatal`)
  AND are UNCHECKED MIRRORS of `sfx_bank.emp`'s consts of the same name AND are
  hardcoded in `seam1.rs`/`seam2.rs`. A triple-mirror entangled with the sound seam
  (Parcel-E territory). Flip only after ruling the sfx_bank.emp/seam authority; else
  park with this reason. (The `if/fatal` would resolve via the harvest `-D`, but the
  three-way authority question should be settled first.)

**#22 `game.asm` — mostly a documented REMAINDER (little flips):**
- gameBootHook / gameDebugTick macros: DEFINED-but-never-INVOKED in the residual
  (boot.emp / game_loop.emp expand `.emp` mirrors) — no AS reads, BUT their bodies are
  the game_loop combo-matrix LOCKSTEP reference (kill row 9) → **PARK (AS)**.
- Header strings (GAME_CONSOLE…GAME_REGION): read by the INVOKED `gameHeader`
  (header.inc) as ROM-header `dc.b` data → **PARK (AS)**.
- `Game_Entry = GameState_OJZScroll_Init`: cross-seam label equalate (main.asm
  `move.l #Game_Entry`) → **PARK (AS)**.
- `GAME_ENTRY_ID = GS_OJZ_SCROLL_TEST`: reads a now-`.emp` game const via the harvest;
  boot.emp reads GAME_ENTRY_ID via `-D` → stays AS (harvest serves it).
- `GAME_CAMERA_JUMP_LOCK`: a `-D` (emp_defines) → cannot move to `.emp` (§1.3).
- Only flip candidate: `SFXID_REV_LOOP = SFXID_SPINDASH` (a sound const) → ships with
  #24. So game.asm is a NAMED REMAINDER, not a flip.

## §9 — FILE MANIFEST

**aeon (`conv-f-game-config`):**
- NEW `games/sonic4/config/constants.emp`; DELETED `games/sonic4/config/constants.asm`;
  M `games/sonic4/main.asm`.
- M consumers: `player_common`, `player_ground`, `player_air`, `player_spindash`,
  `sonic`, `test_player`, `test_animated`, `test_churn`, `test_emitter`, `test_parent`,
  `test_stress_emitter`, `path_swap`, `data/objdefs/test_objects`, `test/object_test_state`,
  `test/ojz_scroll_test` (game-side `use`); `engine/objects/rings`, `engine/objects/entity_window`
  (retired game-VARYING guards).

**sigil (`conv-f-game-config`):**
- M `crates/sigil-harness/src/native.rs` (harvest + profile field + wiring),
  `crates/sigil-harness/m1c_root.asm`, `crates/sigil-harness/tests/m1c_vector_table.rs`.
- M port tests: `objdef_port`, `rings_port`, `test_g1`/`g2`/`g3`/`g4`, `test_p1`.
- Bookkeeping: `twin-scaffolding-kill-list.md` (rows 18/76/81),
  `2026-07-31-conversion-tail-census.md` (row 21 + Parcel F block),
  `campaign-gap-ledger.md`, this note.
