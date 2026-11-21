# Step A piece 1, shared walker: Task 0 baseline

## Values (every later task uses these)

- Branch: `parcel/stepA-p1-shared-walker`, worktree `/home/volence/sonic_hacks/.sigil-stepA-p1`
- Cut point: `10dcf37a6e47d7ff14c9f2b5bbeaa9b267ce3734` (sigil master `1e200e1b` plus the plan document)
- `BASE=/home/volence/sonic_hacks/.stepA-p1-base/target/release/sigil` (detached worktree of 10dcf37a; `sigil 0.1.0 (10dcf37a)`, tree clean). Release build 55.7s wall.
- `AEON_REF=/home/volence/sonic_hacks/.aeon-stepA-p1-ref` at `9b80db1d237b43d945fcef676a848e7007cfb839`, read from the last entry's `aeon_rev` in `crates/sigil-harness/golden/provenance.toml`. Provisioned with `SIGIL_BIN=$BASE scripts/provision-aeon-ref.sh ... 9b80db1d...`, exit 0, 147.9s wall. Rebuild controls: s4.bin 142a39b9/850262 and s4.debug.bin 20acdb67/870606 MATCH THE GOLDEN. Witness `repin --check`: `pins.rs unchanged` (24.9s wall). The script built its own tool into `.target-ref` inside this worktree (untracked/ignored, not committed).
- `CHERUB_REF=/home/volence/sonic_hacks/.cherub-stepA-p1-ref`, `cp -a` of cherub with `build/` removed (it had none to remove beyond what rm -rf did). Pins in `ci/`: aeon `9b80db1d237b43d945fcef676a848e7007cfb839` (same as AEON_REF, so no per-row override), empyrean `85f65cf6...`, oracle `3e617113...`, sigil `57a18265...`. The copy's own HEAD is `8e478cb0`.

## Runs

- BN base vs base: `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`, 151s wall (shape CRCs: plain 142a39b9/850262, debug 20acdb67/870606, demo 4587df52/99980, demo_debug 37d66535/106981, config_a 2ce7438f/870972, config_b 128361b3/617715, lean f6b07847/798198).
- RN base outputs in `/home/volence/sonic_hacks/.stepA-p1-scratch/`: `rn-base-sonic4.txt` 154 lines (2s), `rn-base-demo.txt` 127 lines (1s), `rn-base-cherub.txt` 127 lines (2s). All rc 0.
  - CHERUB needs `AEON_DIR="$AEON_REF"` in the environment: without it the build fails with `engine aeon (root engine) not found: set AEON_DIR ...`. The RN command tail in constraints.md omits it. Every later RN cherub run (base and candidate) must set `AEON_DIR="$AEON_REF"`.
  - `--report contracts` wrote nothing into either reference tree that I looked for; outputs went to stdout (the files above). Builds write ROMs under the project's own output dir inside the scratch reference trees.
- TC on the untouched branch (`.target-land`, release): 147 `test result:` lines, all `ok`; summed **2867 passed, 0 failed, 0 ignored, 0 measured**. 113s wall. Task 2 must match 2867 plus Task 1's new tests.

## Task 1: Lattice and forward_in_states (context.rs only)

- Commit `6490452c` "context: add Lattice and forward_in_states, proven on a non-Tri state": `Lattice` trait, `impl Lattice for Tri` (delegates to `tri_meet`), generic `forward_in_states<S: Lattice>` (a copy of the `must_in_states` loop with the state generic and the instruction index passed to `step`), and `walker_tests` with 8 tests. `must_in_states` untouched (no removed lines).
- RED: `cargo test --release -p sigil-frontend-emp --lib walker_tests` gave 8 errors (E0405 no trait `Lattice`, 7x E0425 no function `forward_in_states`). GREEN: `8 passed; 0 failed; 273 filtered out`.
- RF (first): mutated `context.rs` line 183 `Some(existing) => existing.meet(&st_out)` to `Some(existing) => existing.clone(),` (1 insertion, 1 deletion). `join_of_unequal_states_requeues` FAILED at `context.rs:666` (`left: Some(Levels(1))`, `right: Some(Levels(3))`); the loop-fixpoint test also failed (left 3, right 255). Restored with `git restore --source=HEAD`, status clean, 8 passed. Order note: the work was committed before the RF (a restore from HEAD would otherwise have destroyed it) and the commit body amended after.
- Fix round 1, commit `8d9390fd`: review found the Z80 test passed under either edge model. `z80_cpu_uses_z80_edges` now routes `jr .a`, `nop`, `.a:`, `ret` under `Cpu::Z80` and asserts the `nop` index is absent ("nop is skipped by the jr") and `ret` present. RF: forced `edges_for`'s Z80 arm (line 198) to `Cpu::Z80 => cfg.edges(idx),` (the 68k model); the test FAILED at `context.rs:730` with `nop is skipped by the jr` (7 passed, 1 failed). Restored, clean, 8 passed.

## Task 2: both callers moved, must_in_states deleted

- Commit `7500a887` "sigil-frontend-emp: move both walk callers onto forward_in_states; delete must_in_states". Files: `context.rs`, `z80_bus.rs`. `check_regions` seeds `(entry, Tri::Held)` with step `&|st,_,_,_| *st` and follow `region.contains`; `z80_bus` uses step `&|st,_,mnem,ops| step(*st,mnem,ops)`, same seed, follow `|_| true`. `grep -rn must_in_states crates/` prints nothing.
- TC (89 s): 147 `test result:` lines, 2875 passed, 0 failed, 0 ignored (2867 baseline + 8 new). `tests/context_brackets.rs` 44 passed; `tests/z80_bus.rs` 8 passed.
- BN (release build 8 s, compare 132 s): `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`.
- RN: diff exit 0 for sonic4, demo and cherub; 154, 127, 127 lines (see the stderr note below).
- RF: mutated `context.rs:138` `if !follow(succ) {` to `if follow(succ) {` (1 insertion, 1 deletion). `cargo test --release -p sigil-frontend-emp --test z80_bus double_stop_fires` FAILED at `tests/z80_bus.rs:46:5`, `assertion left == right failed: []  left: 0 right: 1` (31 s including the rebuild). `git restore --source=HEAD -- context.rs`, status clean, re-run: `double_stop_fires ... ok`. (The commit body says "RF: see follow-up note"; this is that note.)
- Clippy `--workspace --all-targets -- -D warnings` rc 0 (13 s).

## Controller's independent re-check at 7500a887

Candidate rebuilt from the committed tree: `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`. The three contract reports (sonic4, demo, cherub; stdout only) are identical to the Task 0 base outputs. Timings: not recorded.

## Branch tip

Code tip `7500a887` (before this notes commit). Test total went from 2867 to 2875 passed, 0 failed.

## What the plan did not expect

- The cherub RN command needs `AEON_DIR="$AEON_REF"` set (both sides), or it fails with `engine aeon (root engine) not found`.
- The plan's original Z80 test row did not discriminate the edge model (it passed under both); fixed in `8d9390fd`, proven by the RF above.
- One `#[allow(clippy::type_complexity)]` remains on `forward_in_states` because clippy fires on the `step` parameter type (no alias; signature unchanged). All `dead_code` allows are gone.
- `cargo fmt --check` reports differences in files this branch did not touch (for example `ast.rs`); fmt was not run.
- The saved `rn-cand-*.txt` files hold the `2>&1` form, so `rn-cand-sonic4.txt` has one extra stderr line (`warning: 15 warnings, module.path-mismatch 15; ...`) that the stdout-only base files lack. The base binary prints the same line under `2>&1`; the stdout-only candidate diff is empty.
- The owed worklist move found eight private walkers, not the brief's seven (`preserves.rs` is the eighth); booked in `docs/QUEUE.md` as `STEPA-PRIVATE-WORKLISTS`.
