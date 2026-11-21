# G0b: the game as data

Sigil builds every game on the Aeon engine from the game's `game.toml` and aeon's
`engine/engine.toml` (`sigil build --project <game> --shape <row>`). No crate the driver links
names a game (the no-game-names gate), and a second game, Cherub, builds from its own repo.
This note is the parcel's landing record; the ledger it was kept in is deleted after landing,
so every ruling is copied here, one line each. The Task 11 record (the retired comparison
gates' last output) follows the parcel record unchanged.

## What landed, task by task

- Tasks 1-4: the description formats (`sigil-build::describe`, FORMAT 1, closed keys), the
  agree-with-Rust test, `refreeze --repin-aeon` (T-REPIN), and the game-independent driver
  code moved out of the harness into `sigil-build`.
- Task 5: the game-as-data driver (`sigil_build::driver`) and `--project/--shape`; WALK
  (packing measured in place, #210) landed on its own branch and merged at Task 7.
- Tasks 6-9a: hermetic builds (a build writes only its own build directory), listing and
  export through one writer, the comparison job, demo's cutover, two named source roots
  (`engine`, `game`, plus `generated`).
- Task 10: sonic4's cutover; aeon builds only through `--project`; oracle re-pinned to the
  shape names (`feat/g0b-shape-names`).
- Task 11: the profile-driven driver deleted in one commit (`sigil build --legacy`,
  `--aeon/--game`, `GameProfile` and every table it read); engine facts moved to
  `engine.toml` (`as_feature_defines`, helpers, appendix facts).
- Task 12: the no-game-names gate (`crates/sigil-cli/tests/no_game_names.rs`): the driver's
  normal-dependency closure, module tree followed as rustc does, `#[cfg(test)]` stripped
  structurally; five allow rows, all G0c sound-seam facts.
- Task 13 + 13a: Cherub, a minimal second game in its own repo (`/home/volence/sonic_hacks/cherub`),
  built by a Sigil pinned at a fix commit, booted headless in Oracle, ROM sha256 `797d41b9...`;
  export rows are root plus root-relative path for every game (`G0B-EXPORT-BY-ROOTS`; empyrean
  §11.55, contract 1.0); the generated mount ratified.
- Final review fix wave (2026-11-21): the four seats' Importants fixed or booked
  (`required_labels` enforced as `[engine.required-label]`; anchor placement checks the held
  section, not a gap inference; format read before the closed parse; the RAM report on an
  out-of-tree game; the no-game-names counts made honest and CamelCase-aware); residue in
  QUEUE `G0B-FINAL-REVIEW-RESIDUE` and aeon DEFERRED_WORK of the same name.

## Rulings (from the ledger; one line each)

- Base (proxy, Fable): the parcel stacked on G0a's tip, not master, to avoid a conflict-heavy rebase.
- P1, revised at Task 2: description tests use the standard `AEON_DIR` guard, and G0b landing runs take a copy of
  the `.aeon-g0b` tip (dropping `__pycache__`) as the reference; `AEON_G0B_DIR` dropped.
- P2: aeon `feat/g0b-descriptions` stacked on aeon `feat/g0a-romsha256` (G0b needs its romSha256 harness).
- P3: `df -h /` before every landing run; under 60 GB free is BLOCKED.
- P4: Cherub's headless boot used oracle built from `feat/g0a-build-export` until G0a merged.
- T3: the agree-with-Rust test counts resolved (game, shape) rows (10), not profile constructors.
- M4 (Task 2): the sound labels stay in `engine.toml` `[sound]` (conditional on sound), commented as game-defined
  labels the sound engine requires (G0c: `G0C-SOUND-SEAMS`).
- SUITE_PATHS step 3: deferred to Task 5, which took steps 1, 2 and 4 (QUEUE G0B-DESCRIBE-TASK2-RESIDUE item 2).
- `no_build_json_writes_none`: a G0a test-robustness bug fixed in G0b (the claim is "a build without -o writes no
  sidecar").
- Pairing gate (proxy, Fable; owner accepted 2026-11-19): `refreeze --freeze --repin-aeon`, built at T-REPIN and run
  once, at landing.
- Landing order (2026-11-19, amended by the final review D-I1): see `docs/OVERSEER-REFERENCE.md` "The G0b landing
  order": empyrean first, aeon, sigil repin/attest/strict landing, oracle, Cherub.
- Task 4: shared landing runs for small move commits accepted (each commit had its own 7/7).
- Task 5 CLI: `--project/--shape` added beside the old grammar (later superseded at Task 10: no `--legacy` flip);
  Task 5 owned the glob matcher and appendix facts from `engine.toml`.
- WALK-IDENTITY (proxy, Fable): packing measured in place as its own parcel ahead of the cutover; no frozen labels,
  no comparison normalisation; #209 attested as that branch's first commit.
- `sigil-build` may depend on `sigil-frontend-as` (the driver assembles the residual AS side itself).
- `cp -a` copies of `.aeon-g0b` drop `__pycache__` (later: never `cp -a` a worktree; `git clone --no-hardlinks`).
- WALK player_climb: `end = section:player_climb`; the `end_measures = "allotment"` kind retired.
- Task 6: "7/7 (old and new driver)" means old 7/7 plus new byte-neutral across the task; the global `AEON_DIR` write
  removed.
- WALK M1: deb2 appendix names differing only within shared-address groups accepted as the same name set.
- The WALK merge is Task 7's step 0; after it the 7/7 base is sigil `87b5c195`.
- COMPARE-IDENTITY-NAMES (proxy, Fable): a narrow declared delta (old side transformed, new side never stripped).
- The contract gate, `--check` and `--report` on `--project` are Task 8's step 0.
- Task 8: the DemoBox_Main alignment plant is the red-first in place of the plan's inert ObjCodeBase drop.
- ENGINE-NAMES-GAME-CONST: `engine.toml` requires `SFXID_REV_LOOP` from every game (no default), demo declares it
  feature-off; the "other games' rows" baseline rule deleted.
- Task 9: the listing delta narrowing DIGEST-SCAN and READ rows to the build's roots accepted as declared.
- Task 10: no `--legacy` flip (Task 11 deletes the old grammar); oracle re-pinned on `.oracle-g0b`
  `feat/g0b-shape-names`; `gate_cut_shape`'s (game, game) spelling dropped; oracle e6c6fb78 (WALK byte move re-pin)
  accepted.
- Task 11: "one commit" is the deletion itself, preparatory moves first; its 7/7 is cross-driver; the "each green"
  red window (87540c32..a265daf1) waived, no history rewrite (QUEUE G0B-TASK11-RESIDUE items 6-7).
- Task 11 `as_feature_defines` (proxy, Fable): the three rows stay in `engine.toml` and go with the residual AS;
  the anti-goal meant the port-gate defines (`SIGIL_EMP_*`, `extra_as_defines`).
- Task 12: comments are not gated, on condition the printed count is honest (done in the fix wave);
  `freeze_journal` moved beside the refreeze binary.
- Task 13 Q1 (proxy, Fable): export rows are (root name, root-relative path) for every game; contract stays 1.0
  with a dated amendment. Q2: Cherub re-pinned to the fix commit, history not rewritten, the README says so. Q3: only
  friction item 6 fixed; the rest booked (aeon `G0B-SECOND-GAME-FRICTIONS`, sigil `G0B-BLIND-REFUSAL-UNNAMED`).
- Task 13a: a fresh implementer; Task 13 and 13a reviewed together. I1: the generated mount ratified (contract text
  states it; dropping it stays booked). Oracle 430509a1's two causes in one commit stand.
- Final review: four parallel seats; seat D's tracked-files-only Cherub build accepted (the ROM reproduces).
- Fix wave: every Important fixed except A-I3 (booked `G0C-SOUND-SEAM-GAME-IDENTITY`); `required_labels` enforced
  rather than deleted; CamelCase fixed, output stems booked (5 hits, 3 of them the S4LZ codec); the aeon
  description comments fixed in one commit though it moves export hashes; minors fixed or booked; the doc sync done
  on the branches.

## Booked for G0c and step 4

- G0c (the sound seams' removal): `G0C-SOUND-SEAMS` (aeon), `G0C-SOUND-SEAM-GAME-IDENTITY`, the five no-game-names
  allow rows, `LEGACY_SOUND_MOUNT` and the `[root.stale-generated]` refusal, `[sound]` naming game-defined labels
  (aeon DEFERRED), `G0B-UNROOTED-TREE-READS` (aeon's generator spellings), Cherub's debug shape building from its
  own repo.
- Step 4 (the engine/game contract): aeon `G0B-SECOND-GAME-FRICTIONS` items 1, 3, 4, 5, 7, 8, 10; whether
  `required_labels` and `required_consts` grow; per-zone scope; `entry_seeds` as named keys.
- Landing-time: QUEUE `G0B-ORACLE-PIN-DRIFT`'s lane decision; the vendored `SOURCE` branch line.

# Task 11 record

Task 11 deleted Sigil's profile-driven build driver (`sigil build --legacy`, `--aeon`/`--game`,
the `GameProfile` constructors and every table they read). This note keeps the last output of
each gate that compared the two drivers, or the descriptions with the old driver's Rust tables,
before it was retired: those gates have no second side left to compare.

## `scripts/compare-drivers.sh` and `crates/sigil-cli/tests/driver_comparison.rs`

Built every described shape with both drivers from one aeon tree and required the same ROM,
`.lst` (after a declared delta) and G0a export.

Last fully green run, at the task's base (sigil `5e45644c`, aeon `bfbc395f`):

```text
COMPARE sigil=/home/volence/sonic_hacks/.sigil-g0b/.target-land/release/sigil (sigil 0.1.0 (5e45644c))
COMPARE aeon=/home/volence/sonic_hacks/.g0b-scratch-t11-1539716/aeon head=bfbc395ff5197508ef4aa1e8a686740f46f2b52b uncommitted-tracked=0 work=/home/volence/sonic_hacks/.g0b-scratch-t11-1539716/cmpwork
SHAPE release game=sonic4 shape=release rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=884838a436174eee/884838a436174eee MATCH (declared delta)
SHAPE debug game=sonic4 shape=debug rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=4615c71a29caa82f/4615c71a29caa82f MATCH (declared delta)
SHAPE demo game=demo shape=release rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=bb36f7a7a9ad89c2/bb36f7a7a9ad89c2 MATCH (declared delta)
SHAPE demo_debug game=demo shape=debug rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=f7b391546270ae6b/f7b391546270ae6b MATCH (declared delta)
SHAPE config_a game=sonic4 shape=config-a rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=dec6e05738692369/dec6e05738692369 MATCH (declared delta)
SHAPE config_b game=sonic4 shape=config-b rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=f0fd1f120b1471fd/f0fd1f120b1471fd MATCH (declared delta)
SHAPE lean game=sonic4 shape=lean rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=66e8e58b5d7c7f2d/66e8e58b5d7c7f2d MATCH (declared delta)
SHAPE config_a_keystones_flipped game=sonic4 shape=config-a-keystones-flipped rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=dec6e05738692369/dec6e05738692369 MATCH (declared delta)
SHAPE stress_evict game=sonic4 shape=stress-evict rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=b7aef01aab46fc4a/b7aef01aab46fc4a MATCH (declared delta)
SHAPE stress_art game=sonic4 shape=stress-art rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=00d02de7e928af2c/00d02de7e928af2c MATCH (declared delta)
SHAPES 10/10 MATCH
COMPARE_DRIVERS_END rc=0
END rc=0
```

From `87540c32` (the first Task 11 prep commit that moved seam 2's alignment input onto the
descriptions) the profile-driven driver itself reads `games/sonic4/game.toml` (seam 2's
`[[align]]` rows, `seam2::SOUND_GAME_DESC_REL`), so its listing digest already names the file
the script's declared delta adds, and the delta refuses on every sound-on shape. The ROM,
export, gate, report and check columns stayed MATCH. The test run at `ec859616`:

```text
=== cargo test --release -p sigil-cli --test driver_comparison -- --nocapture --test-threads=1
     Running tests/driver_comparison.rs (.target-land/release/deps/driver_comparison-fdef0337b00bbdfb)
running 7 tests
test checks_match_on_every_shape ... reference-tree: /home/volence/sonic_hacks/.g0b-t11-cand (SUITE_PATHS step 1, named by AEON_DIR)
COMPARE sigil=/home/volence/sonic_hacks/.sigil-g0b/.target-land/release/sigil (sigil 0.1.0 (ec859616))
COMPARE aeon=/home/volence/sonic_hacks/.g0b-t11-cand head=50a96ab9a1f42c175a768bfb0ff3fe3f38fa930c uncommitted-tracked=0 work=/home/volence/sonic_hacks/.compare-drivers-.g0b-t11-cand-3626177-1795214917
SHAPE release game=sonic4 shape=release rom=MATCH lst=DIFFERS export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=884838a436174eee/884838a436174eee DIFFERS
    lst: cannot apply the declared delta: legacy listing already reads games/sonic4/game.toml
SHAPE debug game=sonic4 shape=debug rom=MATCH lst=DIFFERS export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=4615c71a29caa82f/4615c71a29caa82f DIFFERS
    lst: cannot apply the declared delta: legacy listing already reads games/sonic4/game.toml
SHAPE demo game=demo shape=release rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=bb36f7a7a9ad89c2/bb36f7a7a9ad89c2 MATCH (declared delta)
SHAPE demo_debug game=demo shape=debug rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=f7b391546270ae6b/f7b391546270ae6b MATCH (declared delta)
SHAPE config_a game=sonic4 shape=config-a rom=MATCH lst=DIFFERS export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=dec6e05738692369/dec6e05738692369 DIFFERS
    lst: cannot apply the declared delta: legacy listing already reads games/sonic4/game.toml
SHAPE config_b game=sonic4 shape=config-b rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=f0fd1f120b1471fd/f0fd1f120b1471fd MATCH (declared delta)
SHAPE lean game=sonic4 shape=lean rom=MATCH lst=DIFFERS export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=66e8e58b5d7c7f2d/66e8e58b5d7c7f2d DIFFERS
    lst: cannot apply the declared delta: legacy listing already reads games/sonic4/game.toml
SHAPE config_a_keystones_flipped game=sonic4 shape=config-a-keystones-flipped rom=MATCH lst=DIFFERS export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=dec6e05738692369/dec6e05738692369 DIFFERS
    lst: cannot apply the declared delta: legacy listing already reads games/sonic4/game.toml
SHAPE stress_evict game=sonic4 shape=stress-evict rom=MATCH lst=DIFFERS export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=b7aef01aab46fc4a/b7aef01aab46fc4a DIFFERS
    lst: cannot apply the declared delta: legacy listing already reads games/sonic4/game.toml
SHAPE stress_art game=sonic4 shape=stress-art rom=MATCH lst=DIFFERS export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=00d02de7e928af2c/00d02de7e928af2c DIFFERS
    lst: cannot apply the declared delta: legacy listing already reads games/sonic4/game.toml
SHAPES 3/10 MATCH
COMPARE_DRIVERS_END rc=1
driver_comparison: check identical on 10 shape(s)
ok
test closure_reports_match_on_every_shape ... driver_comparison: report identical on 10 shape(s)
ok
test exports_match_on_every_shape ... driver_comparison: export identical on 10 shape(s)
ok
test gate_verdicts_agree_on_every_shape ... driver_comparison: gate identical on 10 shape(s)
ok
test listings_match_on_every_shape ... 
thread 'listings_match_on_every_shape' (3922242) panicked at crates/sigil-cli/tests/driver_comparison.rs:178:5:
7 of 10 shape(s) differ in the lst: ["release (lst=DIFFERS)", "debug (lst=DIFFERS)", "config_a (lst=DIFFERS)", "lean (lst=DIFFERS)", "config_a_keystones_flipped (lst=DIFFERS)", "stress_evict (lst=DIFFERS)", "stress_art (lst=DIFFERS)"]
test rom_bytes_match_on_every_shape ... driver_comparison: rom identical on 10 shape(s)
test the_job_builds_every_described_shape_with_both_drivers ... driver_comparison: 10 shape(s) built by both drivers
failures:
failures:
    listings_match_on_every_shape
test result: FAILED. 6 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 135.93s
=== rc=101
```

The same job with only that refusal turned into "already read, nothing to add" (a scratch copy
of the script, run at `ec859616` plus the uncommitted Task 11 work, aeon `50a96ab9`):

```text
COMPARE sigil=/home/volence/sonic_hacks/.sigil-g0b/.target-land/release/sigil (sigil 0.1.0 (ec859616-dirty))
COMPARE aeon=/home/volence/sonic_hacks/.g0b-t11-cand head=50a96ab9a1f42c175a768bfb0ff3fe3f38fa930c uncommitted-tracked=0 work=/home/volence/sonic_hacks/.compare-drivers-.g0b-t11-cand-1336576-1795216183
SHAPE release game=sonic4 shape=release rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=884838a436174eee/884838a436174eee MATCH (declared delta)
SHAPE debug game=sonic4 shape=debug rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=4615c71a29caa82f/4615c71a29caa82f MATCH (declared delta)
SHAPE demo game=demo shape=release rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=bb36f7a7a9ad89c2/bb36f7a7a9ad89c2 MATCH (declared delta)
SHAPE demo_debug game=demo shape=debug rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=f7b391546270ae6b/f7b391546270ae6b MATCH (declared delta)
SHAPE config_a game=sonic4 shape=config-a rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=dec6e05738692369/dec6e05738692369 MATCH (declared delta)
SHAPE config_b game=sonic4 shape=config-b rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=f0fd1f120b1471fd/f0fd1f120b1471fd MATCH (declared delta)
SHAPE lean game=sonic4 shape=lean rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=66e8e58b5d7c7f2d/66e8e58b5d7c7f2d MATCH (declared delta)
SHAPE config_a_keystones_flipped game=sonic4 shape=config-a-keystones-flipped rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=dec6e05738692369/dec6e05738692369 MATCH (declared delta)
SHAPE stress_evict game=sonic4 shape=stress-evict rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=b7aef01aab46fc4a/b7aef01aab46fc4a MATCH (declared delta)
SHAPE stress_art game=sonic4 shape=stress-art rom=MATCH lst=MATCH(declared-delta) export=MATCH(declared-delta) gate=AGREE(passed) report=MATCH check=MATCH sha=00d02de7e928af2c/00d02de7e928af2c MATCH (declared delta)
SHAPES 10/10 MATCH
COMPARE_DRIVERS_END rc=0
```

Retired with the driver (Task 11): the job compares the two drivers, and one of them is gone.
The cross-driver identity is still shown once per Task 11 commit by
`scripts/step0_shapes_cmp.sh` (the base binary on the old grammar against the new binary on
`--project`), and the byte identity by the goldens.

## `crates/sigil-harness/tests/descriptions_agree_with_rust.rs`

Held every description field to the Rust profile it replaced. Last green, at `ec859616`:

```text
=== cargo test --release -p sigil-harness --test descriptions_agree_with_rust -- --nocapture --test-threads=1
     Running tests/descriptions_agree_with_rust.rs (.target-land/release/deps/descriptions_agree_with_rust-45ccc14bb270b1cc)
running 2 tests
test ctor_census_refuses_a_wrapped_signature ... ok
test descriptions_agree_with_rust ... reference-tree: /home/volence/sonic_hacks/.g0b-t11-cand (SUITE_PATHS step 1, named by AEON_DIR)
  align      56 facts
  baseline   346 facts
  defines    200 facts
  engine     44 facts
  entry      80 facts
  features   30 facts
  game       47 facts
  gate rows  408 facts
  placement  1768 facts
  shapes     12 facts
compared 2991 facts across 10 shapes (10 Rust rows from 8 constructors)
ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.64s
=== rc=0
```

## `crates/sigil-harness/tests/evenness_default.rs`

Derived every row of the old driver's `section_align::DECLARED` from the new rule. Last green,
at `ec859616`:

```text
=== cargo test --release -p sigil-harness --test evenness_default -- --nocapture --test-threads=1
     Running tests/evenness_default.rs (.target-land/release/deps/evenness_default-37a69849942b2325)
running 1 test
test evenness_default_reproduces_the_102_rows ... reference-tree: /home/volence/sonic_hacks/.g0b-t11-cand (SUITE_PATHS step 1, named by AEON_DIR)
evenness default: 102 row(s) from EVEN, 6 from [[align]] rows, of 108
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
=== rc=0
```

## `crates/sigil-harness/tests/frozen_tables_embedded.rs`

Held `native::FROZEN_TABLES` (the old driver's compiled-in size tables) to the committed
directory. Last green, at `ec859616`:

```text
=== cargo test --release -p sigil-harness --test frozen_tables_embedded -- --nocapture --test-threads=1
     Running tests/frozen_tables_embedded.rs (.target-land/release/deps/frozen_tables_embedded-4bdbbd7988bd4cbf)
running 1 test
test frozen_tables_embed_every_committed_table ... 7 frozen table(s) embedded, each its own file
ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
=== rc=0
```

## `crates/sigil-build/tests/game_descriptions.rs` (its agree-with-Rust halves)

Both tests stay; the halves that read expectations out of the deleted Rust source as text
(shape counts, define literals, alignment rows, D1c tables, helper lists, sound bank-id consts,
module ids) are retired. Last green with those halves, at `ec859616`:

```text
=== cargo test --release -p sigil-build --test game_descriptions -- --nocapture --test-threads=1
     Running tests/game_descriptions.rs (.target-land/release/deps/game_descriptions-47b42216538c1d15)
running 2 tests
test engine_toml_loads ... sigil-build reference tree: /home/volence/sonic_hacks/.g0b-t11-cand (step 1: AEON_DIR)
ok
test sonic4_game_toml_loads ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
=== rc=0
```
