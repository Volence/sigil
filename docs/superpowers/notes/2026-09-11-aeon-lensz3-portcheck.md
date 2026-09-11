# 2026-09-11: aeon lens-Z3 parcel, does it break any sigil test

STATUS: COMPLETE. The verdict rests on part 2 (prepared trees, all four ROMs
built). Part 1 (source-only exports) is kept as a record only and cannot answer
the question; the section "Why part 1 cannot answer the question" says why.

## The question

The engine lane holds parcel `79e789803b3938c8e487cb351efa4cfe2355cc41` (merge-base
with aeon origin/master: `e4b4f38f3d7ee05b20c623b2303be5c9f9a65b3f`). It adds
cross-module names: `script_display_frames` / `script_last_frame` in
`engine/objects/animate.emp`, `art_nibbles_outside` in `engine/objects/dplc.emp`,
consumed from `games/sonic4/player/player_instashield.emp` and
`games/sonic4/objects/ring_sparkle.emp`, plus three branches in
`engine/effects/raster.emp` `Raster_VBlank` made unsized. Aeon's record says this
kind of change can turn sigil `*_port` tests red while all four ROM shapes build.

Measured with sigil `8d31a0030cdfbaeb6368c418f2e49c9670c87a5f`, on branch
`measure/aeon-lensz3-portcheck`. No file under `crates/` changed.

The two trees differ in exactly 10 paths (`diff -rq`): the five `.emp` files above,
two poison fixtures (`games/sonic4/test/poison/poison_instashield_frames.emp`,
`poison_ring_sparkle_frames.emp`), `tools/emp_expect_fail.py`, and two new files
(`tools/test_z80_clobbers_census.py`, a lens note). The raster diff is three
`beq.s`/`bne.s` to unsized, plus comments.

## Method

A differential: the same full strict suite on both trees, one after the other,
never concurrently, private target dir.

    SIGIL_STRICT_GATE=1 AEON_DIR=<tree> ORACLE_DIR=/home/volence/sonic_hacks/oracle-old \
    CARGO_TARGET_DIR=/home/volence/sonic_hacks/.scratch/aeon-lensz3-portcheck/target \
    cargo test --release --workspace --no-fail-fast -- --nocapture

Failing names are qualified by binary, taken from both the `test X ... FAILED`
lines and the per-binary `failures:` lists (union; the two agree except for one
interleaved line). NEW = parcel-only, FIXED = base-only, BOTH = both. Sets are
compared, never totals. Because a parcel break can hide inside BOTH as a changed
message under an unchanged name, every BOTH name's panic text is also compared,
normalised only for the tree path and thread ids.

## Part 1: source-only exports (a first record, NOT the verdict)

Trees: `git archive` exports at `.scratch/aeon-lensz3-portcheck/{base,parcel}`, no
`.git`, no build products.

| run | binaries launched | reported `test result:` | passed | failed | ignored |
|---|---|---|---|---|---|
| base   | 467 | 467 | 4995 | 244 | 2 |
| parcel | 467 | 467 | 4995 | 244 | 2 |

Both logs end in the runner's own end marker (exit 101). No binary launched without
reporting.

- NEW: empty. FIXED: empty. BOTH: 244.
- BOTH text comparison: 3 of 244 differ, none attributable to the parcel.
  `dac_bank_port::dac_banks_matches_reference` and `dac_banks_debug_matches_reference`
  swap which of the two hits `reference missing` first and which then sees the
  shared mutex's `PoisonError` (lock order, varies run to run).
  `vectors_port::vectors_debug_region_matches_reference` differs only by a stray
  `FAILED` token interleaved into the base capture by `--nocapture`.
- `*_port` view: 62 binaries, zero per-test outcome differences between the trees.
  29 port tests pass on both; every other port test fails on both.
- `corpus_builds`: "4 of 7 shipped shapes build from source" on both. The three that
  fail (`sonic4 debug`, `demo debug`, `config_a`) fail on the same 10 errors, all
  from the absent generated `engine/debug/generated/compression_vectors.emp` and
  its embedded `.bin` files.
- The suite wrote the same 19 files into `engine/sound/generated/` of each tree and
  nothing else; the sigil worktree stayed clean.

### Why part 1 cannot answer the question

All 8 tests in the three port binaries that lower the changed engine modules
(`animate_port` 2, `dplc_port` 4, `raster_port` 2) read `s4.bin` / `s4.debug.bin`
from the tree BEFORE they lower anything, and under `SIGIL_STRICT_GATE=1` a missing
ROM panics there ("SIGIL_STRICT_GATE set but reference missing", e.g.
`animate_port.rs:362`, `dplc_port.rs:310` and `:436`, `raster_port.rs:410` and
`:430`). With no ROMs in either export, those 8 fail on both trees before reaching
the parcel's code, so a lowering break in `animate.emp`, `dplc.emp` or `raster.emp`
would be invisible to this differential. The consumers
(`player_instashield.emp`, `ring_sparkle.emp`) are named by no `*_port` test at all;
only whole-tree tests (`corpus_builds` and its siblings) lower them, and those did
lower them here (the 4 plain shapes build).

A second limit, independent of the ROMs: `animate_port`, `dplc_port` and
`raster_port` lower ONE file with its dependency modules' items prepended
(`with_ambient`), through single-file `lower_module`, which does not validate a
`use` list against its target module (`lower/proc.rs::collect_use_names` only
gathers the names for the context check). The multi-module resolver is what
rejects an unknown import (`resolve/imports.rs`: "module `X` has no `pub` name
`n`"). So a broken `use` name may not turn those port tests red even when they do
lower. This is to be measured by the canary in part 2, not assumed.

## Part 2: prepared trees

The controller redirected the method: provision one tree per revision with the four
ROMs built, so the port gates get past the reference read.

`scripts/provision-aeon-ref.sh` was read and NOT run as given, for two reasons:

1. Step 1 refuses any revision not reachable from aeon `origin/master` ("Refusing
   to pin to it"). The parcel `79e78980` is unpushed, so the parcel tree cannot be
   provisioned by it at all.
2. Steps 1 and 2 run `git -C <aeon> fetch` (moves aeon's remote-tracking refs) and
   `git -C <aeon> worktree add` (writes aeon's `.git/worktrees`). This session is a
   worktree-isolated agent whose harness refuses `git -C` into another repository,
   and the brief's invariant is to move no ref but its own branch. Running the same
   operations inside a script would evade that refusal, not satisfy it.

Instead each export (already exactly the source at its commit) is copied to
`/home/volence/sonic_hacks/.aeon-sigil-lensz3-{base,parcel}` and provisioned by a
script that performs the provisioner's own steps 3 to 6 and nothing from steps 1
and 2: copy and CRC-verify the four goldens, build the vendored salvador, generate
the compression vectors, and build all four shapes with aeon's `build.sh` using an
assembler and `emit_sound_blob` built from this sigil revision into a private
`ref-target`. The script's `repin --check` witness is meaningful only at the pinned
revision, so the evidence of a prepared tree here is the four ROMs present and
built, with CRC32 and size recorded below.

The assembler and `emit_sound_blob` were rebuilt into `ref-target` after this
note's first commit, so the binary's `--version` revision equals the sigil HEAD
the trees were provisioned under (`ca3ebfac`, tree clean); aeon's `build.sh`
compares the two and printed no mismatch banner.

### Provisioning deviations, and the attempt that failed

- `NO_LINT=1` in the environment is DEAD in aeon's `build.sh`: line 355 resets
  `NO_LINT=0` before parsing flags, and only `-nl`/`--no-lint` sets it. The
  official provisioner passes the dead environment variable, so it runs aeon's
  pytest tool-suite lane on every shape; in its own git worktree that lane can pass.
- Attempt 1 (base, no flag) failed in that lane's PRE-build half before any ROM was
  written (the tree's `s4.bin` still carried the golden's size and mtime, and no
  `s4.lst` existed): 29 failed / 2394 passed. Every visible cause is the export
  having no `.git`: `git ls-files` failing, citation and "tracked by git" checks,
  an empty population in `test_no_baked_home_paths`. Logs kept as
  `provision-base.attempt1.log` and `provision-base-build-sonic4.attempt1.log`.
- Fix: each shape is built as `./build.sh <game> -nl` (the game first, because
  `GAME="${1:-sonic4}"`). `-nl` skips the lint and tool-suite lane only, which emits
  no ROM bytes. The half-provisioned base tree was deleted and re-copied from its
  export, not patched.

### The prepared trees

Both provisioned to their end marker (base 19:20:09, parcel 19:20:57). The four
goldens verified against the provenance tail on each before being replaced: s4
`b09ccd65/820229`, s4.debug `1b7fe316/846529`, demo `0ad17404/96863`, demo.debug
`2565ece2/103185`. Then all four shapes were BUILT (mtime after the build start):

| file | base CRC32 (hex / decimal) / size | parcel |
|---|---|---|
| s4.bin         | `064e0ae6` / 105777894 / 821123   | byte-identical to base |
| s4.debug.bin   | `cb0e2019` / 3406700569 / 847389  | byte-identical to base |
| demo.bin       | `bc230dd7` / 3156413911 / 97075   | byte-identical to base |
| demo.debug.bin | `523e0287` / 1379795591 / 103359  | byte-identical to base |
| s4.lst         | `42a3edd1` / 341447               | `659ec894` / 341447 |
| s4.debug.lst   | `6c5ca146` / 411733               | `e59ba591` / 411733 |

So with one assembler on both trees, all four ROM shapes are byte-identical at
base and parcel, which is the engine lane's "the ROM does not move" claim, now
observed rather than relayed. The listings differ, as they must: the parcel adds
source lines and symbols.

The engine lane reported its canonical s4 as CRC32 377796925 (`1684a93d`) / 821123
bytes. The size agrees; the CRC does not. Not established why. Both of these trees
were built by the same sigil (`ca3ebfac`), which the differential needs and has;
their assembler is not this one, and sigil's revision reaches its source digest
(`crates/sigil-cli/src/main.rs:2810`), but whether that digest reaches the image
was not checked. Do not read the mismatch as a defect in either build.

### Prepared base suite

Run from sigil `1c286a5f` (clean), started 19:22:55, own end marker at 19:29:05
(exit 101). 467 binaries launched, 467 reported `test result:`; 5078 passed /
161 failed / 2 ignored (83 fewer failures than the source-only base).

The 8 tests that lower the parcel's changed engine modules now get PAST the
reference read and lower. Each fails later, at a point that is only reachable after
`lower_module` returned without errors:

| test | now fails at | what that point is |
|---|---|---|
| `animate_port::animate_region_matches_reference` | `animate_port.rs:347` | region byte compare, "first diff at offset 0x0" |
| `animate_port::animate_debug_region_matches_reference` | `animate_port.rs:347` | same, debug shape |
| `dplc_port::dplc_region_matches_reference` | `dplc_port.rs:296` | region byte compare |
| `dplc_port::dplc_debug_region_matches_reference` | `dplc_port.rs:296` | same, debug shape |
| `dplc_port::two_module_ownership_flip_plain` | `dplc_port.rs:535` | link-assert drift guard: `DMA_Queue_End` not defined in this link |
| `dplc_port::two_module_ownership_flip_debug` | `dplc_port.rs:535` | same, debug shape |
| `raster_port::raster_region_matches_reference` | `raster_port.rs:364` | region byte compare |
| `raster_port::raster_debug_region_matches_reference` | `raster_port.rs:364` | same, debug shape |

They stay red because `pins.rs` places each window at the `ec640bcf` addresses and
today's ROM carries other code there (the "expected" bytes at offset 0 are not the
module). That red is drift since the pinned revision, the same on both trees, and
it sits AFTER lowering, so a lowering error in `animate.emp`, `dplc.emp` or
`raster.emp` would change these tests' failure text. One limit: a region compare
prints only the first differing offset and 16 bytes around it, and every one of
these differs at offset 0, so a byte change deeper in a module would not change
the text. The four ROMs being byte-identical at base and parcel (above) covers that
gap for emitted bytes.

In all, 75 port tests pass on the prepared base (29 on the source-only base). The
ROMs unmask these 27 (source-only base FAILED, prepared base ok):
`compression_selftest_port` (2),
`controllers_port` (1), `dac_bank_port` (2), `hblank_port` (1), `header_port` (2),
`mt_bank_port` (4), `particle_anims_port` (1), `soundbankhead_port` (3), the
`test_g1`..`test_g4` doctored-reference tests (4), `test_p1_player_port` (3),
`test_p2_player_states_port` (3), `test_p4_player_sensors_port` (1).

The instrument that compares these 8 tests' failure text across two logs was
checked both ways before use: prepared base against itself gives IDENTICAL for all
8 with real panic text on both sides (no empty capture passing as agreement), and
source-only base against prepared base gives DIFFERS for all 8 ("reference
missing" against the region and link-assert failures above), so it can see a
change.

One confound is known and bounded. The prepared base ran with the sigil worktree
clean throughout; this note was edited (uncommitted) about a minute into the
prepared parcel run. The only test binary that reads the live sigil worktree's git
status at runtime is `version_provenance` (`current_dir(REPO)`, `REPO` =
`CARGO_MANIFEST_DIR`); its tree-status test only asserts MORE when the tree is
clean. Any NEW failure in that binary is re-run on both trees in one worktree state
before being attributed.

### Prepared parcel suite, and the differential

Same sigil `1c286a5f`, same test binaries, own end marker at 19:36:43 (exit 101).
467 binaries launched, 467 reported; 5078 passed / 161 failed / 2 ignored, the same
totals as the prepared base.

- NEW: empty. FIXED: empty. BOTH: 161.
- BOTH text comparison: 5 of 161 differ, none attributable to the parcel.
  `camera_port::camera_region_matches_reference` is identical apart from an `ok`
  token `--nocapture` interleaved into it; `load_object_port::load_object_region_matches_reference`
  likewise with a `FAILED` token; `sfx_bank_port::sfx_bank_matches_reference` and
  `sfx_bank_debug_matches_reference` swap which of the two hits the length assertion
  and which sees the shared mutex's `PoisonError` (the lock-order race `dac_bank_port`
  showed in part 1); `test_g3_objects_port::g3_undoctored_compile_equals_the_reference_window`
  had its parcel-side panic header merged with another test's result line
  (`run-prep-parcel.log:2772`), so the parser missed it, and read directly the
  assertion text (message, left, right, 2483 characters each side) is identical.
- `corpus_builds`: "7 of 7 shipped shapes build from source" on both.
- The 8 target tests: FAILED on both trees with IDENTICAL failure text, including the
  first 16 bytes of the lowered module (`candidate`) at each region compare.
- `*_port` view: 62 binaries. Two per-test differences, both "ok on base, absent on
  parcel", both parse artifacts: `camera_port::jump_lock_off_compiles_without_game_symbols`
  (its result line split by an interleave, `run-prep-parcel.log:415`) and
  `test_g3_objects_port::g3_doctored_reference_diverges` (merged into the panic header
  at `:2772`). Both binaries report "2 passed; 2 failed" on both trees. Zero real
  port differences.
- Flake control: owed for NEW names only, and NEW is empty in both differentials, so
  there was nothing to re-run. The `version_provenance` confound above produced no
  NEW.

### Canaries on the prepared parcel

Three `animate_port` runs, one after another, same binaries, trees differing from
the prepared parcel by exactly one line each (checked with `diff -r`):

| run | tree | planted edit | `animate_port` result |
|---|---|---|---|
| control | prepared parcel | none | FAILED at `animate_port.rs:347`, region compare; text identical to the suite's |
| canary A (the briefed one) | `prep-canary-use` | `animate.emp:40` use list: `refresh_piece_count` renamed `refresh_piece_count_CANARY` | IDENTICAL to control. NOT SEEN |
| canary B | `prep-canary-body` | `animate.emp:299` body call renamed the same way, import untouched | FAILED at `animate_port.rs:257`, "animate.emp lower errors: [... unknown function `refresh_piece_count_CANARY` ...]". SEEN |

Canary B is the positive control: on a prepared tree, `animate_port` lowers the real
`animate.emp`, and a cross-module name that nothing supplies moves its failure from
the region compare to the lowering assert. The test is red on both trees already
(pin drift), so "goes red" cannot be the observable here; the changed failure point
and text is, and the BOTH text comparison is what reports it.

Canary A is invisible, and not only to the port test. `animate_port` prepends
`frames.emp`'s items and lowers one file, so the import line is never consulted.
But the whole-tree build accepts it as well: `corpus_builds` on `prep-canary-use`
says "7 of 7 shipped shapes build from source" at that tree with zero `CANARY`
mentions, and a direct `sigil build --native` of it exits 0 with no diagnostic, the
same warning tally as the control (165, same breakdown), and a ROM byte-identical
(`cmp`) to the control build and to the provisioned parcel `s4.bin`. So a `use`
list naming an item that does not exist is accepted SILENTLY by sigil itself.

The mechanism is NOT established, and the code read says it should not happen. The
"module `X` has no `pub` name `n`" Error is pushed by `resolve_use`
(`crates/sigil-frontend-emp/src/resolve/imports.rs:384`) whenever
`ExportIndex::is_exported` (`:104`) is false; `resolve_use` runs from
`collect_uses` inside `ResolveEnv::build` (`:295`); and `ResolveEnv::build`'s only
caller extends the build's diagnostics with what it returns
(`crates/sigil-frontend-emp/src/resolve/mod.rs:879-880`), inside the loop over
every reachable module, which includes `engine.objects.animate` (its code is in the
ROM). Yet the build printed no such diagnostic. So either `is_exported` answered
true for `refresh_piece_count_CANARY`, or something after `:880` drops or
downgrades the Error. Which one was not measured. This is a sigil finding
independent of the parcel, reported here and not fixed (no file under `crates/`
changed); the one-line reproduction is canary A on any prepared tree.

## Verdict

**No. The parcel breaks no sigil test.** On prepared trees with all four ROMs built,
the full strict suite gives NEW = FIXED = empty; every failing name fails on both
trees with the same text apart from five output-ordering artifacts; the 8 tests
that lower `animate.emp`, `dplc.emp` and `raster.emp` get past the reference read,
lower the parcel's versions, and fail identically on both trees at post-lowering
points; the four ROM shapes are byte-identical at base and parcel; `corpus_builds`
builds 7 of 7 shapes on both; and canary B shows the same run would have seen a
lowering break of this kind.

What the verdict does NOT say:

- It does not say those 8 port tests pass. They are red on both trees because
  `pins.rs` is at `ec640bcf`. It says the parcel changes nothing they observe.
- A byte change deeper than 16 bytes into a region would not change a region
  compare's text; the byte-identical ROMs cover that for emitted bytes.
- Because a broken `use` name alone is invisible to the whole suite and to
  `sigil build` (canary A), nothing in sigil checks the parcel's new `use` lines
  as imports. They are exercised only through their callers, which build.

## What the brief got wrong

1. **The source-only differential was blind to the question.** Absent ROMs, the 8
   tests that lower the changed modules stop at the reference read on both trees, so
   they sit in BOTH regardless of the parcel. The controller redirected to prepared
   trees; part 1 stays only as a record.
2. **`provision-aeon-ref.sh` cannot provision this parcel.** Its step 1 refuses any
   revision not reachable from aeon `origin/master`, and `79e78980` is unpushed; its
   steps 1 and 2 also run `git fetch` and `worktree add` inside aeon's repository,
   which this isolated worktree may not do and the brief's ref invariant forbids.
   Steps 3 to 6 were run on the exports instead.
3. **`NO_LINT=1` does nothing in aeon's `build.sh`** (reset at line 355); the
   official provisioner passes it and so runs the pytest lane, which fails
   wholesale in a tree without `.git`.
4. **The canary as specified could not prove sight.** "Break one `use` name ... show
   that test goes red": the target test is already red on both trees, and a broken
   `use` name alone changes nothing sigil can see anywhere. A break of a REFERENCED
   cross-module name (canary B) is the one that proves the run is sighted.
5. **The engine lane's canonical CRC did not reproduce.** Its s4 (377796925 /
   821123) and this sigil's s4 (`064e0ae6` / 821123) agree on size only; why was not
   established. Its other claim, "the ROM does not move", did reproduce for all four
   shapes.

## Name lists

`2026-09-11-aeon-lensz3-portcheck/`, one `binary::test` per line:
`source-only-{base,parcel}-failing.txt`, `source-only-{new,fixed,both}.txt`,
`prepared-{base,parcel}-failing.txt`, `prepared-{new,fixed,both}.txt`.

## Name lists

`2026-09-11-aeon-lensz3-portcheck/`: `source-only-{base,parcel}-failing.txt`,
`source-only-{new,fixed,both}.txt`, one `binary::test` per line.
