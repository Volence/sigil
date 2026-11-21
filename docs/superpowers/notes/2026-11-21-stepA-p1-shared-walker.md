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
