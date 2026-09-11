# 2026-09-11: aeon lens-Z3 parcel, does it break any sigil test

STATUS: IN PROGRESS. The source-only record below is complete. The prepared-tree
measurement, which the verdict rests on, has not run yet. Do not read a verdict
into this file until a `## Verdict` section exists.

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

(Results pending.)

## Name lists

`2026-09-11-aeon-lensz3-portcheck/`: `source-only-{base,parcel}-failing.txt`,
`source-only-{new,fixed,both}.txt`, one `binary::test` per line.
