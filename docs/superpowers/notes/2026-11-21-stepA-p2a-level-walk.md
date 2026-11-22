# Step A piece 2a, the interrupt level walk inside one routine: working notes (baseline)

## Values (every later task uses these)

- Branch `parcel/stepA-p2a-level-walk`, worktree `/home/volence/sonic_hacks/.sigil-stepA-p2a`, cut from master `5a6c2dd875c2ecbc4a3d742b5d6aa4140622c7e7`.
- A11 `9f8e1bc6` is an ancestor of HEAD (`git merge-base --is-ancestor 9f8e1bc6 HEAD` exit 0).
- Scratch: `/home/volence/sonic_hacks/.stepA-p2a-scratch/`. BASE binary `.../sigil-base` (`sigil 0.1.0 (5a6c2dd8)`, tree clean), built with `CARGO_TARGET_DIR=.../target-base cargo build --release -p sigil-cli` (22s, incremental).
- Reference trees (reused, verified): aeon `/home/volence/sonic_hacks/.aeon-stepA-p1-ref` at `9b80db1d237b43d945fcef676a848e7007cfb839`, clean; Cherub `/home/volence/sonic_hacks/.cherub-stepA-p1-ref` at `8e478cb`. Cherub needs `AEON_DIR=<aeon ref>`.

## Baseline

- Shapes (`scripts/step0_shapes_cmp.sh --base B --cand B --aeon-ref AEON`, 40s): `SHAPES 7/7 IDENTICAL`, `STEP0_CMP_END rc=0`. plain 142a39b9/850262, debug 20acdb67/870606, demo 4587df52/99980, demo_debug 37d66535/106981, config_a 2ce7438f/870972, config_b 128361b3/617715, lean f6b07847/798198. Same as piece 1.
- Contract reports (`build --project <p> --shape release --report contracts`, stdout, all rc 0), saved in scratch: `rn-base-sonic4.txt` 154 lines, `rn-base-demo.txt` 127, `rn-base-cherub.txt` 127 (same line counts as piece 1). Stderr in `*.err`.
- Front-end tests (`AEON_DIR=aeon ref`, `cargo test --release -p sigil-frontend-emp --no-fail-fast`, rc 0): 149 `test result:` lines, summed 2921 passed, 0 failed, 0 ignored, 0 measured. Log `tc.log`.
- Landing baseline: 6302 (`docs/QUEUE.md` line ~3169, STEP0-A11 entry: `reconciles 6233 baseline + 69 new = 6302 observed`, `RESULT GREEN` at `9f8e1bc6`).

## W2 "before" (all accepted by the base binary)

- `emp w2_planted.emp --hex`: rc 0, 18 bytes, no diagnostics (`... 54 8F 4E 75 ...`, the `addq.l #2,sp` exit).
- `emp a_leak_pop_d0.emp --hex`: rc 0, 18 bytes, no diagnostics.
- `emp e_bare_sr_planted.emp --hex`: rc 0, 18 bytes, one existing advisory `[proc.ccr-advisory]` (bare `preserves(sr)`, `rts` between save and restore); accepted.
- Whole-build plant: fresh `git clone` of the aeon ref at `9b80db1d` into `aeon-plant`. Line 183 of `engine/system/dma_queue.emp` before: `        move.w  (sp)+, sr` (the `.full:` exit); after: `        addq.l  #2, sp`. `build --project aeon-plant/games/sonic4 --shape release`: exit 0, `contract closure gate: passed (sonic4 release: 196 module(s) analysed ...)`, 850262 bytes, CRC ee52408a (differs from the clean 142a39b9, as the plant changes bytes). Restored with `git checkout 9b80db1d -- engine/system/dma_queue.emp`; `git status --short` empty; line 183 back to `move.w  (sp)+, sr`.
- Probe sources: `survey-probes/{w2_planted,a_leak_pop_d0,e_bare_sr_planted}.emp`; outputs `p-*.out`.
