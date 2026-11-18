---
kind: plan
status: awaiting-owner
description: "Empyrean v2 step 0, G0b: games and the engine described as TOML (game.toml per game, aeon engine/engine.toml), the generic driver extracted into sigil-build, hermetic seams, two-root resolution, demo then sonic4 cut over by a bytes-plus-export comparison job, the old driver deleted, and Cherub (a new minimal game in its own repo) built by a pre-Cherub Sigil binary."
---
# G0b Game-as-Data Build Driver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Every dispatch carries the `dispatching-empyrean-agents` invariant block.

**Goal:** Sigil's source names no game. Every shipped shape builds from `game.toml` + `engine/engine.toml` + the unchanged `map.toml`, identical in bytes and in G0a's export to the old driver, and Cherub, a new game in its own repo, builds and boots with a Sigil binary older than Cherub.

**Architecture:** Extract beside, not rewrite: the proven, game-agnostic driver code (packing, budgets, emit with checksum, deb2 appendix, residual-AS harvest, drift guards, closure gates, listing) moves unchanged into a new crate `sigil-build`, and the old driver calls it. New code exists only where the game lives: a description reader (`serde` with `deny_unknown_fields`), closed module gating, per-game defines, evenness by default with declared exceptions, placement without frozen tables, and two named source roots. A landing-bar comparison job builds every shape with both drivers and compares bytes and G0a exports; cutover is per game; then the old driver is deleted.

**Tech Stack:** Rust (`sigil-*`, `serde` + `toml`, check the workspace lock first and add `toml` only if absent), aeon `build.sh`, Oracle and BlastEm for the Cherub boot.

**Spec:** `empyrean/docs/v2/specs/2026-11-17-G0b-game-as-data-driver-design.md` (revision 2, owner answers 2026-11-18: Q1 (a) `aeon/engine/engine.toml`; Q2 no change to Sigil's CI; Q3 seams hermetic now, removal is G0c; Q4 Cherub, a new minimal game in a sibling repo, `games/demo` stays in aeon). Review: `empyrean/docs/v2/specs/reviews/2026-11-17-G0b-review.md`. Depends on: `sigil/docs/superpowers/plans/2026-11-17-g0a-build-data-export.md` (its 68000 export, Tasks 1-6, must land before Task 7 here).

## Global Constraints

- **Not byte-changing through cutover.** The old driver keeps building the 7 shapes (s4, s4 debug, demo, demo debug, config A, config B, lean) byte-identical (`scripts/step0_shapes_cmp.sh`, `SHAPES 7/7 IDENTICAL`) at every commit. A byte that moves is a separate explained commit with its own proof, never folded into a task.
- The descriptions hold only facts the Rust driver holds today. No per-zone scope, asset pipeline, shape count, capacity or engine/game contract change (anti-goal; step 4 decides those).
- **No engine fact in a game's file.** If it is true for every game on Aeon it goes in `engine/engine.toml`.
- **No port history in either format:** no frozen size tables, no AS definedness defines, no even-alignment rows, no seam name lists.
- Closed inputs: unknown key → refused by name; a placed module matching no gating rule → refused by name; a module matching two rules → refused.
- Root names are exactly `engine`, `game`, `generated` (shared with G0a's files table).
- Sigil CI unchanged (owner Q2): the comparison job and agree-with-Rust test run in the landing bar; the Cherub gate runs in Cherub's own CI against pinned Sigil and aeon.
- Branches: sigil `parcel/g0b-game-as-data` (from sigil master at execution time); aeon `feat/g0b-descriptions`; Cherub is a new repo `/home/volence/sonic_hacks/cherub`, default branch `main`, created locally (creating the GitHub remote and the first push need the owner's OK). Never commit to a default branch except Cherub's first commit.
- Paired landing (invariant 7): aeon descriptions and the sigil driver that reads them merge as a PAIR.
- `SIGIL_BUILD`/`SIGIL_EMIT` from this branch's `.target-land` build. Never run `sigil build` on a live aeon tree (Task 6 makes this hermetic; until then build in a run-unique copy).
- Gates per invariant 8 (RF protocol in `2026-11-16-sigil-soundness.md`). Never `mcp__oracle__*`; the Cherub boot uses a headless Oracle process and BlastEm.
- Exact-path `git add`; finding in each commit body; no Claude co-author trailer.
- Doc sync at landing: parcel note `sigil/docs/superpowers/notes/2026-11-17-g0b-game-as-data.md`, `empyrean/docs/v2/tasks/G0b Build driver rewrite now.md`, aeon `docs/DEFERRED_WORK.md` (G0c booked with the seam facts), suite paths (`empyrean/contract/SUITE_PATHS.md` gains `cherub`).

## Review Focus

1. **A module added to a game with no gating rule** (a new `games/sonic4/objects/foo.emp`): today it silently ships in every shape; the new driver must refuse it naming the file and the rule table. Test in Task 5 (`unmatched_module_is_refused`).
2. **A shape that `extends` another and overrides one define** (config A from s4 debug): the override wins, everything else inherits, and a cycle in `extends` is refused. Test in Task 2 (`extends_overrides_one_define`, `extends_cycle_refused`).
3. **Placement without frozen tables on content growth:** adding 2 KB to a module before an island must not brick the build (H4-08); the new driver places it or refuses with the region and overrun, never silently overlaps. Test in Task 5 (`growth_before_island_places_or_refuses`).
4. **A build run twice on the same tree with `--check`:** the source tree is byte-unchanged afterwards (`git status --porcelain` empty), including `engine/sound/generated/`. Test in Task 6 (`check_build_writes_nothing_in_tree`).
5. **A path that exists under two roots** (`engine/x.emp` and the game's own `x.emp`): `embed` and `include` resolve by the root the path names, never by search order, and an unrooted ambiguous path is refused. Test in Task 9 (`same_relpath_under_two_roots_resolves_by_root`).

## The inventory to move (verified at sigil `c08bf4ce`)

| Fact | Today | Goes to |
|---|---|---|
| Profiles: `sonic4_profile(debug)`, `demo_profile(debug)`, `config_b_profile`, `config_a_profile`, `config_a_keystones_flipped_profile`, `lean_profile`, `stress_evict_profile`, `stress_art_profile` | `sigil-harness/src/native.rs` (from `sonic4_profile` to `stress_art_profile`) | `games/sonic4/game.toml` shapes `release`, `debug`, `config-a`, `config-a-keystones-flipped` (fixture), `config-b`, `lean`, `stress-evict` (fixture), `stress-art` (fixture); `games/demo/game.toml` shapes `release`, `debug` |
| Per-game `emp_defines` (`MAX_RING_BUFFER` 128/16, `VRAM_RING_PLACEHOLDER`, `COLLECTED_WINDOW_SLOTS`, `HAS_ACT_ART_POOL`) | each profile | `[defines]` in each `game.toml` |
| `SHAPE_GATES` game rows, `SIGIL_PROBE_FIXTURES` | `module_roots.rs` | `[[gate]]` rows in `game.toml` |
| `SHAPE_GATES` engine rows, `ENGINE_TERMINUS`, entry seeds `engine.game_contract`/`engine.ram`, error-handler and release-fault ids | `module_roots.rs`, `native.rs` | `engine.toml` |
| `COMPTIME_HELPERS` (order matters), `RAM_HELPERS`, `STRUCT_OFFSET_TWINS` | `native.rs` | `engine.toml` |
| 6 alignment exceptions (`Dac_Temp_Blip`, `SoundTablesZ80_Head`, `SongBank2_Head` at `$8000`; `Sfx_33`, `Song_MovingTrucks` at 8; `ObjCodeBase` at `$10000`) | `section_align.rs` | `[[align]]` in `game.toml` (all six are game labels); the other 102 rows become the driver's 68000 evenness default |
| Baselines (`Z80_OUT_UNVERIFIED_BASELINE`, `D1C_BASELINE`, debug and demo extras), `is_demo_family` | `contract_baseline.rs` | engine rows in `engine/baseline.toml`, game rows in `games/<g>/baseline.toml`, named by path from each description |
| `BLOB_BANK_ID_CONSTS`, `SoundTablesZ80_Head`/`SfxBlobWinTab`/`Sfx_33` | `sound_bank_ids.rs` | engine (`engine.toml` `[sound]`); the label names are game labels the engine requires (`required_labels`) |
| Sound driver files, generated names, 399 driver constant names, bank chain heads | `seam1.rs`, `seam2.rs` | stay in Rust as engine facts until G0c (owner Q3); not in either format |
| `extra_as_defines` on config A | `native.rs` (`config_a_profile`) | deleted (Task 1) |
| `FROZEN_TABLES` (7) | `native.rs` | deleted at Task 11 after `[[anchor]]` placement reproduces them |

---

### Task 1: Delete the inert `extra_as_defines`

**Files:** Modify `crates/sigil-harness/src/native.rs` (`GameProfile.extra_as_defines` and its 8 initialisers, and its readers).
- [ ] **Step 1:** Measure: `grep -rn "SOUND_DEBUG_HOTKEYS\|SOUND_DBG_MIRROR" aeon/engine aeon/games --include=*.asm` (expect 0) and `--include=*.emp` (record).
- [ ] **Step 2:** Remove the field. **Step 3:** `SHAPES 7/7 IDENTICAL` and config A `.lst` identical. If config A moves: revert, BLOCKED (the defines were not inert). **Step 4:** commit with the grep counts.

### Task 2: Description formats and the three descriptions

**Files:**
- Create: `crates/sigil-build/` (new crate: `Cargo.toml`, `src/lib.rs`, `src/describe.rs`)
- Create (aeon): `games/sonic4/game.toml`, `games/demo/game.toml`, `engine/engine.toml`, `engine/baseline.toml`, `games/sonic4/baseline.toml`, `games/demo/baseline.toml`
- Test: `crates/sigil-build/src/describe.rs` (unit)

**Interfaces:**
- Produces:
  - `pub struct GameDesc { pub id: String, pub name: String, pub format: u32 /* 1 */, pub engine: EngineRef, pub sources: Sources, pub entry: EntryModules, pub defines: BTreeMap<String, i64>, pub shapes: BTreeMap<String, ShapeDesc>, pub gate: Vec<GateRule>, pub map: PathBuf, pub align: Vec<AlignRow>, pub baseline: PathBuf, pub outputs: BTreeMap<String, String> }`, all `#[serde(deny_unknown_fields)]`
  - `pub struct EngineRef { pub suite_path: String /* "aeon" */, pub root: PathBuf /* "engine" */ }` (resolved through `empyrean/contract/SUITE_PATHS.md`'s environment rule, never a literal absolute path)
  - `pub struct ShapeDesc { pub extends: Option<String>, pub features: Features, pub defines: BTreeMap<String, i64>, pub fixture: bool }`; `pub struct Features { pub debug: bool, pub crash_report: bool, pub sound: bool }`
  - `pub struct GateRule { pub modules: String /* id or glob */, pub place_in: PlaceIn }` where `PlaceIn` is `All`, `Shapes(Vec<String>)` or `Feature(FeatureExpr)` (`debug`, `!debug`, `sound`, `crash_report`)
  - `pub struct AlignRow { pub label: String, pub align: u32 }`
  - `pub struct EngineDesc { pub format: u32, pub helpers: Vec<String> /* ordered */, pub ram_helpers: Vec<String>, pub struct_twins: Vec<(String, String)>, pub entry_seeds: Vec<String>, pub terminus: String, pub error_handler: String, pub release_fault: String, pub gate: Vec<GateRule>, pub required_labels: Vec<String>, pub sound: SoundDesc, pub baseline: PathBuf }`
  - `pub fn load_game(path: &Path) -> Result<GameDesc, DescError>`, `pub fn load_engine(path: &Path) -> Result<EngineDesc, DescError>`, `pub fn resolve_shape(g: &GameDesc, shape: &str) -> Result<ResolvedShape, DescError>` (flattens `extends`; `DescError::ExtendsCycle(Vec<String>)`)
- `__MDDBG__` and `__DEBUG__` are derived from `crash_report` and `debug`, not fields.
- [ ] **Step 1:** Tests: `unknown_key_refused_by_name`, `extends_overrides_one_define`, `extends_cycle_refused`, `sonic4_game_toml_loads` (reads the aeon file via A8's reference guard), `engine_toml_loads`.
- [ ] **Steps 2-5:** FAIL; implement; green; commit the crate in sigil and the six files in aeon (aeon files inert under the old driver: 7/7 unchanged).

### Task 3: Agree-with-Rust test

**Files:** Create `crates/sigil-harness/tests/descriptions_agree_with_rust.rs` (landing bar).
- [ ] **Step 1:** For each of the 9 profiles × its shape name, assert the resolved description equals the Rust profile field by field: modules placed (via `SHAPE_GATES` evaluation vs `GateRule` evaluation over the full module scan), defines, features, entry modules, alignment exceptions, baseline rows, engine lists. Every comparison counts its items and the test prints `compared N facts across 9 shapes`; a zero count is a failure.
- [ ] **Step 2:** RF: plant a missing shape gate in `games/sonic4/game.toml` → red naming the module; plant a reordered `helpers` entry in `engine.toml` → red. **Step 3:** green; commit.

### Task 4: Extract the generic driver into `sigil-build`

**Files:** Move (unchanged bodies) the packing walk, budgets, `emit_rom`, `append_deb2_appendix`, residual-AS harvest, drift-guard verdicts, closure gates and listing glue from `sigil-harness/src/native.rs`, `map_placement.rs`, `seam2.rs` (generic parts only) into `crates/sigil-build/src/{place.rs,emit.rs,appendix.rs,harvest.rs,guards.rs}`; `sigil-harness` re-exports or calls them.
- [ ] **Step 1:** `git mv`-style moves, one module per commit, each commit `SHAPES 7/7 IDENTICAL` + `landing-run.sh` green with totals. A function that needs a game fact to move is NOT moved; it is listed in the parcel note as "game-touching" for Task 5.
- [ ] **Step 2:** After the last move, `sigil-build` must not depend on `sigil-harness` (`cargo tree -p sigil-build` shows no `sigil-harness`). Commit.

### Task 5: The new driver: reader, closed gating, defines, evenness, placement without frozen tables

**Files:** Create `crates/sigil-build/src/{driver.rs,gating.rs,align.rs}`; modify `crates/sigil-cli/src/main.rs` (`--project <dir|file>`, `--shape <name>`; `--aeon`/`--game` only behind `--legacy`).

**Interfaces:**
- Produces: `pub fn build(project: &Path, shape: &str, opts: &BuildOptions) -> Result<BuildOutput, BuildError>` with `BuildOutput { rom: Vec<u8>, listing: Vec<ListingSymbol>, export_inputs: ExportInputs /* G0a Task 4 */, warnings: Vec<BuildWarning> }`; `pub fn gate_modules(scan: &[ModuleId], rules: &[GateRule], shape: &ResolvedShape) -> Result<Vec<ModuleId>, BuildError>` (`BuildError::Ungated(ModuleId)`, `BuildError::DoubleGated(ModuleId, usize, usize)`); evenness: every 68000 section base is even unless an `[[align]]` row raises it.
- [ ] **Step 1:** Tests: `unmatched_module_is_refused` (Review Focus 1), `double_gated_module_is_refused`, `evenness_default_reproduces_the_102_rows` (derive every row of `section_align.rs`'s table except the 6 from the default, count == 102), `growth_before_island_places_or_refuses` (Review Focus 3).
- [ ] **Step 2:** FAIL. **Step 3:** implement. Placement reads `[[anchor]]` rows from `map.toml`; where frozen-table placement and anchor placement disagree on a shape, fix the map or the source with its own byte-proven commit (spec §5.1 caution); never reintroduce a table. **Step 4:** green. **Step 5:** commit.

### Task 6: Hermetic seams

**Files:** Modify `crates/sigil-harness/src/native.rs` (`emit_generated_in`, `ensure_generated`) and `sigil-build`: seams write into `<build dir>/generated/`; `embed("engine/sound/generated/...")` resolves through the `generated` root.
- [ ] **Step 1:** Test `check_build_writes_nothing_in_tree` (Review Focus 4): copy aeon at HEAD, `git -C copy status --porcelain` empty, run `sigil build --check` and a full build twice, status still empty.
- [ ] **Step 2:** FAIL today (15 files rewritten under `engine/sound/generated/`). **Steps 3-5:** implement; green; 7/7 (old and new driver); commit. The checked-in `engine/sound/generated/*` stay as they are until G0c (book this in aeon DEFERRED_WORK under G0c).

### Task 7: Comparison job (bytes + G0a export)

**Files:** Create `sigil/scripts/compare-drivers.sh` and `crates/sigil-cli/tests/driver_comparison.rs` (landing bar).
- [ ] **Step 1:** For every shape (the 7 plus the 3 fixtures), build with `--legacy` and with `--project`, in two run-unique copies of one tree; compare ROM SHA-256, `.lst` bytes, and the G0a exports with `tool` and `inputsSha256` removed. Print one line per shape and `SHAPES n/10 MATCH`; a shape that did not build is `DID NOT RUN`, which fails the job.
- [ ] **Step 2:** RF: plant a dropped alignment exception (`ObjCodeBase`) in `game.toml` → that shape DIFFERS. **Step 3:** green for demo and demo debug at least; commit.

### Task 8: Demo cutover

**Files (aeon):** `build.sh` demo arm uses `sigil build --project games/demo --shape {release|debug}`; fold `build.conf`'s demo settings into `games/demo/game.toml` and delete them from `build.conf`.
- [ ] **Step 1:** Comparison job `demo`, `demo-debug` MATCH. **Step 2:** `./build.sh demo`, `DEBUG=1 ./build.sh demo`, both sonic4 shapes, `tools/landing_build.sh` exit 0 with its stamp. **Step 3:** commit the pair.

### Task 9: Two-root resolution

**Files:** Modify `crates/sigil-frontend-emp/src/resolve/manifest.rs` (`Manifest::scan` takes `&[(RootName, PathBuf)]`), `embed` resolution, `crates/sigil-frontend-as/src/eval.rs` (`include`), `crates/sigil-link/src/listing.rs` (`DigestRoot` names the root), `sigil-build` driver (roots `engine` = aeon `engine/`, `game` = the project dir, `generated` = build dir).
- [ ] **Step 1:** Tests: `same_relpath_under_two_roots_resolves_by_root` (Review Focus 5); `engine_root_alone_does_not_pull_other_games` (scan with roots `engine` + `games/demo`: no `games.sonic4.*` module in the corpus); `digest_names_each_root`.
- [ ] **Steps 2-5:** FAIL; implement; green; the comparison job still matches every shape (the digest's root naming changes `.lst` text: if it does, that is a declared listing change — STOP and record it; the `.lst` feeds the deb2 appendix, so any listing change must be proven byte-neutral in the ROM in all shapes before it lands); commit.

### Task 10: Sonic4 cutover

**Files (aeon):** `build.sh` sonic4 arms (release, debug, config-a, config-b; `lean` stays refused by `build.sh` and is proven by the comparison job only; the stress fixtures stay on `build.sh`'s rebake path but build through `--project ... --shape stress-*`).
- [ ] **Step 1:** Comparison job `SHAPES 10/10 MATCH`. **Step 2:** all four `build.sh` shapes + landing build stamp. **Step 3:** commit the pair.

### Task 11: Delete the old driver and its tables

**Files:** delete the profiles, `SHAPE_GATES`, `SIGIL_PROBE_FIXTURES`, `section_align.rs`'s row table, `contract_baseline.rs`'s tables and `is_demo_family`, `sound_bank_ids.rs`'s literals, `FROZEN_TABLES`, `--legacy`; the goldens gate the new driver.
- [ ] **Step 1:** One commit. 7/7 against the goldens; `landing-run.sh` green with totals. **Step 2:** the comparison job is retired (its last green output quoted in the parcel note).

### Task 12: No-game-names gate

**Files:** Create `crates/sigil-cli/tests/no_game_names.rs` (runs in CI: it reads only sigil's own source).
- [ ] **Step 1:** Scan every `.rs` file of the crates the driver links (`cargo metadata` for `sigil-cli`'s dependency closure, not a hand list) for game ids (`sonic4`, `demo`, `cherub`, read from the aeon descriptions' `id` fields recorded in a fixture, plus any `games.<x>` module prefix and `games/<x>/` path), excluding `#[cfg(test)]` modules and `tests/`. Print the engine-id count beside it, not gated.
- [ ] **Step 2:** RF: red before Task 11 (record the count); after Task 11, plant `"games.sonic4.x"` in `sigil-build/src/driver.rs` → red. **Step 3:** green; commit.

### Task 13: Cherub

**Files (new repo `/home/volence/sonic_hacks/cherub`):** `game.toml`, `map.toml`, `vram.toml`, `game_root.asm` (residual, from demo's skeleton), `src/game.emp` (`implement Game`), `src/ram.emp`, `src/ship.emp` (one object: a ship moved by the pad), a scrolling background, `baseline.toml`, `.github/workflows/ci.yml` (checks out pinned Sigil and aeon), `README.md`.
- Cherub is written against the public engine interface only: its source may `use engine.*` modules the engine marks `pub`, and nothing under `games/`. No sound (`features.sound = false`), crash report off or on as the engine allows without game code.
- [ ] **Step 1:** The gate script `ci/build.sh`: build with a Sigil binary built from a sigil commit **older than Cherub's first commit** (recorded SHA in `ci/sigil.pin`), at a pinned aeon commit (`ci/aeon.pin`). Assert: build exit 0; G0a export validates and binds; headless Oracle boots it to a frame where the ship's position changes under a scripted pad input (frame hash differs between input on/off); BlastEm boots it (headless screenshot, non-blank).
- [ ] **Step 2:** Red first: before Task 11 and Task 9 the build fails (record the error). **Step 3:** green; Cherub's first commit on `main`, local only; the GitHub remote and push wait for the owner.
- [ ] **Step 4:** Add `cherub` to `empyrean/contract/SUITE_PATHS.md`.

## Dependencies and parallelism

1 → 2 → 3; 4 in parallel with 2-3; 5 after 3 and 4; 6 after 4; 7 after 5, 6 and G0a Tasks 1-6; 8 → 9 → 10 → 11 → 12 → 13. Keep the overlap between Task 8 and Task 11 short (spec §5.5: demo cutover within days of the job going green, sonic4 within a week after), because every engine change in between lands twice. Estimate (spec §8, ESTIMATED): about 4 to 5 weeks; placement without frozen tables (Task 5) is the least certain. If Task 5 exceeds 5 working days, report to the owner with what remains.

## Out of scope (booked)

- G0c: removing the sound seams and G0a's Z80 bridge (aeon DEFERRED_WORK, booked in Task 6).
- A contract JSON Schema for `game.toml` (when Aurora reads it).
- Porting `build.sh`'s 24 post-build gates; splitting the engine into its own repo; grow-only placement proof; shape count policy (H2-11).
