# G0b: the game as data (Task 11 record)

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
