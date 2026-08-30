# Absolute-path classify: what the suite says when its reference tree is not there

Sigil's tests read an aeon reference tree through `AEON_DIR`. This note measures what the
suite SAYS when that tree is absent or incomplete, and buckets every affected row by
whether the failure names the missing input or misdirects into a vocabulary that never
mentions it. It classifies; it changes no row.

The headline is structural: **the bucket is a property of the (row, missing-part) pair,
not of the row.** 398 rows change behaviour when the reference tree changes, and every one
of the 61 rows that misdirects under one poison tree is SILENT under another. No single
scenario can classify this suite, and the emptiest scenario classifies the least.

## Measurement setup

| | |
|---|---|
| sigil tree | `.claude/worktrees/agent-af1763fc6c39336b9`, branch `apc/absolute-path-classify`, at `cf38b1e5` |
| reference tree | `/home/volence/sonic_hacks/.aeon-ref-af1763fc`, aeon `ec6a4791db346ec8c6672632109f85415b873e49` |
| provisioned by | `scripts/provision-aeon-ref.sh` |
| provisioning witness | `repin --check` → **`pins.rs unchanged`** |
| rebuild control | `s4.bin 6e2f9b22/719315` and `s4.debug.bin 6516fc68/736315`, both MATCHES THE GOLDEN |
| target dir | `/home/volence/sonic_hacks/.sigil-target-af1763fc` — dedicated, on the nvme root, never `/tmp` |
| baseline, ordinary run | 4168 passed / 0 failed / 2 ignored, exit 0, 31 skip lines |
| baseline, `SIGIL_STRICT_GATE=1` | 4168 passed / 0 failed / 2 ignored, exit 0, **0** skip lines |
| `--list` baseline | **4170** rows, keyed on the deps-binary hash |

Row identity is `binary-hash::row`. Keyed on the *label* cargo prints (`tests/foo.rs`,
`unittests src/lib.rs`) instead, 4170 rows collapse to 4169 — the label repeats across
crates. A count that quietly loses a row is the shape of defect this audit is about, so
the key is the hash.

`SIGIL_BUILD` / `SIGIL_EMIT` are not required in the environment: `scripts/landing-run.sh`
derives both into the run's own target directory and builds them. Not a blocker.

## Scenarios

Each poison tree is built by copying reference-tree CONTENT into a fresh plain directory
(`rsync -a --exclude .git`). Never a `cp -a` of a worktree: the reference tree's `.git` is
a FILE pointing at aeon's metadata, and a copy of it makes git in the copy touch the
original's index.

| scenario | what is missing |
|---|---|
| `absent` | the directory does not exist |
| `empty` | the directory exists and is empty |
| `markers` | only the two files `reference_tree_for_profile` probes per game (`game_root.asm`, `map.toml`) |
| `roms-only` | the 4 ROMs and 2 listings; no source |
| `source-only` | every source file; no ROM, no listing |
| `no-listings` | full tree minus `s4.lst` / `s4.debug.lst` |
| `no-art` | full tree minus `art/` |
| `no-generated` | full tree minus the gitignored `engine/{sound,debug}/generated` — exactly what a bare `git worktree add --detach` produces |
| `no-games-data` | full tree minus each game's `data/` subtree |

Every scenario was run twice: the ordinary run (someone else's machine) and the same run
under `SIGIL_STRICT_GATE=1` (the landing bar).

## The classification

`SILENT` is derived rather than parsed: a row is silent in a scenario when it PASSES that
scenario's ordinary run and FAILS the same scenario's strict run. Every announced early
return in this tree is a strict-gated one, so the strict run is exactly the population the
ordinary run rendered green — and it carries the row name on its panic line, which an
interleaved `--nocapture` skip line does not.

| scenario | collected | NAMES-THE-INPUT | MISDIRECTS | SILENT | CORRECT-AS-IS | ordinary failed | strict failed |
|---|---|---|---|---|---|---|---|
| `absent` | 4170 | 0 | **0** | 392 | 0 | **0** | 398 |
| `empty` | 4170 | 16 | 0 | 342 | 37 | 53 | 398 |
| `markers` | 4170 | 55 | 13 | 286 | 37 | 105 | 394 |
| `roms-only` | 4170 | 109 | **29** | 221 | 37 | 175 | 397 |
| `source-only` | 4170 | 0 | 0 | 164 | 0 | **0** | 166 |
| `no-listings` | 4170 | 0 | 2 | 4 | 0 | 2 | 6 |
| `no-art` | 4170 | 64 | 5 | 4 | 1 | 70 | 74 |
| `no-generated` | 4170 | 40 | 1 | 3 | 19 | 60 | 62 |
| `no-games-data` | 4170 | 86 | **28** | 50 | 0 | 114 | 165 |
| **UNION** | | **184** | **61** | **397** | **38** | | |

The union is stated separately on purpose. No scenario finds more than 29 of the 61
misdirecting rows, and the two scenarios a reader would reach for first — the tree deleted,
the tree source-only — find **zero failures at all**.

The `strict failed` column is libtest's own summed counter. `NAMES`/`MISDIRECTS`/`SILENT`
come from parsed panic headers, which lose a handful of strict-run rows to `--nocapture`
interleaving, so the SILENT figures are lower bounds by up to 6 rows per scenario.

### Collection is intact; nothing vanished

libtest's own counters sum to **4170 in every scenario without exception** —
`passed + failed + ignored`, per scenario, equal to the `--list` baseline. No test binary
died at collection, none was filtered out, and no row left the totals. This is a positive
result, not an absence: it is the reconciliation correction (4) asks for, and it holds
because nothing in this workspace reads an aeon path at compile time — no `include_str!` /
`include_bytes!` of a reference path, and `crates/sigil-cli/build.rs` (the only build
script that is not a vendored C shim) touches git metadata and cargo manifests only.

### The buckets are behaviours, not populations

- rows affected in at least one scenario: **398**
- of the 61 rows that MISDIRECT somewhere, **61** are SILENT somewhere else
- of the 184 rows that NAME THE INPUT somewhere, **184** are SILENT somewhere else
- 31 rows both MISDIRECT under one poison tree and NAME the input under another

There is one reference-dependent population of 398 rows. Which of the four things it does
depends entirely on which part of the tree is gone.

## The misdirect vocabularies, per scenario

Never summed — the same row lands in different vocabularies under different scenarios.

| scenario | vocabularies |
|---|---|
| `roms-only` | io error on an unnamed tree 24, other 4, unresolved-symbol/no-module 1 |
| `no-games-data` | unresolved-symbol/no-module 12, io error on an unnamed tree 7, other 9 |
| `markers` | other 6, empty/zero census 3, unresolved-symbol/no-module 2, io error 2 |
| `no-art` | unresolved-symbol/no-module 5 |
| `no-listings` | unresolved-symbol/no-module 2 |
| `no-generated` | other 1 |

Four shapes account for all 61:

1. **A bare basename with no directory.** 24 rows in `roms-only`, e.g.
   `cannot read collision_lookup.emp: No such file or directory (os error 2)` at
   `crates/sigil-cli/tests/collision_lookup_port.rs:211`. Nothing in that sentence is a
   path a reader can act on.
2. **A relative path with no root**, e.g. `read games/sonic4/config/header.emp: No such
   file or directory` at `header_port.rs:55`. Reads as a repo-relative path into sigil.
3. **A link or build failure with no input named at all**:
   `link failed: unresolved symbol 'RingSparkle_Spawn' for fixup in section rings at
   offset 522` (`rings_port.rs:384`, `no-listings`), and `build_program: 26 error(s);`
   (`tranche4_negative_probes.rs:414`, `no-art`). The `no-listings` case is the failure
   `scripts/provision-aeon-ref.sh`'s step 6 comment predicts in prose; it is now measured.
4. **A path that actively points somewhere else.**
   `emit_sound_blob (blob): read /tmp/sigil-shadow-aeon-2314581-0/games/sonic4/data/sound/
   movingtrucks_pitchtable.emp: No such file or directory` (`native.rs:1150`). The reader is
   sent to a temp shadow copy. So is `called Result::unwrap() on an Err value:
   PoisonError { .. }` (`dac_bank_port.rs:72`, `mt_bank_port.rs:85`, `sfx_bank_port.rs:69`) —
   a mutex poisoned by another row's panic, which names neither the input nor the real cause.

## CORRECT-AS-IS — 38 rows that must keep failing

Their subject IS the reference tree's completeness. Making an incomplete tree green would
destroy exactly the gates that detect an incomplete tree.

- `contract_closure_corpus.rs:105` — the `paths.len() >= 120` corpus floor, whose message
  says why: *"too few to be the whole corpus, so every assert-empty gate below would pass
  vacuously"*. 18 rows.
- `contract_closure_corpus.rs:878`, `out_verify_corpus.rs:51`, `cfg_blind_spots.rs:324/453`,
  `movem_restore_guard_corpus.rs:194`, `parcel_8b_stage_gen_touchers.rs:147` — `no .emp
  files under <aeon>`.
- `slot_type_corpus.rs:93` — the same floor over the slot-type corpus. 4 rows.
- `extra_entry.rs:119` — every aeon fixture this file names still resolves.
- The named-witness assertion beside the corpus floor
  (`engine/debug/generated/compression_vectors.emp` present), which is why `no-generated`
  contributes 19 correct-as-is rows.

These already name the input; they are listed apart so no later parcel "fixes" them.

## Two urgencies, not a ranking

**Loud-in-the-wrong-place is URGENT and self-limiting.** 61 rows send a reader to a bare
basename, a relative path, a link error, or `/tmp`. It hurts, so it gets chased down — and
each chase costs a diagnosis, which is what makes it urgent.

**Silent-green is IMPORTANT and does not self-correct.** 397 rows report green while
measuring nothing. Nothing hurts, so nothing prompts a fix. The older phrasing *"a silent
pass costs nothing until trusted"* is FALSE and is retracted: a green test in a suite is
trusted immediately and by construction, with no grace period.

The sharpest instance: with the reference tree entirely absent, the ordinary run is
**4168 passed / 0 failed, exit 0** — a fully green suite that measured 398 fewer rows than
it appears to. `SIGIL_STRICT_GATE=1` turns those same 398 into failures that name the path.
Both facts are true of the same tree at the same moment; only the flag differs.

## The walk-up-finder check — none found, and how that was established

**Result: no walk-up path finder for the reference tree exists in this repo.** That is a
measured result, not a null.

Source enumeration. Every aeon resolution in the workspace is one of two shapes, both of
which honour the override outright:

- Rust: `std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon")` —
  114 non-comment occurrences, plus `test_support::aeon_dir()` (the same expression) and
  four `PathBuf::from(...)` spellings in `tranche{2,3,4,5}_negative_probes.rs`.
- Shell: `"${AEON_DIR:-/home/volence/sonic_hacks/aeon}"` in
  `crates/sigil-harness/golden/capture_goldens.sh:75`,
  `crates/sigil-harness/golden/derive_offcanonical_sizes.sh:25`, and
  `scripts/landing-run.sh:207`.

`SIGIL_ROOT` was checked specifically. In all three named scripts it is derived from the
script's OWN location (`cd "$(dirname "$0")/../../.."`), which follows the script into a
worktree and is not an ancestor search. It names the sigil tree, never the reference tree.
`scripts/landing-run.sh:94` does contain a real ancestor walk
(`while [[ ! -e $p && $p != / ]]; do p=$(dirname "$p"); done`) but it is inside `abspath`,
resolving a not-yet-existing path's nearest existing ancestor; it selects nothing.
`harness_root::resolve_harness_root` resolves via `git rev-parse --show-toplevel` and
verifies `ROOT_MARKERS`, and it addresses the sigil harness root, not `AEON_DIR`.

fs-level trace. `strace`, `perf` and `bpftrace` are all absent on this machine, so the
witness is an `LD_PRELOAD` interposer over `open`, `openat`, `stat`, `lstat`, `__xstat`,
`statx`, `fstatat`, `access` and `opendir`, logging every path containing
`/home/volence/sonic_hacks/aeon` — the same call set `strace -f -e
trace=openat,stat,newfstatat` would have covered.

- **Red-first control, three binaries**: with `AEON_DIR` unset, the shim recorded **4442**
  touches of the real tree (`opendir /home/volence/sonic_hacks/aeon`, `…/art`, `…/engine`, …)
  across `native_rom` + `boot_port` + `test_mappings_port`.
- **Red-first control, full workspace**: with `AEON_DIR` unset, **396785** touches, 359
  suites reporting, 4168 passed / 0 failed / 2 ignored. The witness sees what it claims to
  see at exactly the scale the measurements below are taken at. It also shows the reason
  this defect is invisible here: unset, the hardcoded default silently finds the live tree
  and the suite is green.
- **Measurement, full workspace, `AEON_DIR` → an absent directory**: **0 touches**,
  359 suites reporting.
- **Measurement, full workspace, `AEON_DIR` → the partial `markers` tree**: **0 touches**,
  359 suites reporting.

A first attempt at those two runs passed `--release` twice and cargo refused before running
anything; it reported `0 touches` from a run that executed no tests. The wrapper now counts
`test result:` lines and says *UNMEASURABLE* when there are none, because a zero from a run
that did not happen looks exactly like a zero from a run that did.

## The suite writes into `AEON_DIR`, and the write flips other rows' guards

Found while reconciling a scenario that would not reproduce.

`native::ensure_generated(aeon)` (`crates/sigil-harness/src/native.rs:1149`) targets
`aeon.join("engine/sound/generated")`, and `seam1::emit_sound_blob`
(`crates/sigil-harness/src/seam1.rs:729`) opens with `create_dir_all(&out_dir)` — before it
reads anything. So the suite creates `$AEON_DIR/engine/sound/generated/` even when
`$AEON_DIR` does not exist, and then panics on the missing source.

The consequence is not cosmetic. `contract_closure_corpus::aeon_dir()` guards on
`if !aeon.exists()` — a probe of the ROOT. Once another row has mkdir'd a path under it,
the root exists, the guard stops skipping, the corpus walk finds 0 files, and the floor
fires. Measured, both directions, same command:

| tree state | ordinary run |
|---|---|
| `absent`, pristine | 4168 passed / **0 failed** / 2 ignored, exit 0 |
| after the suite's own `mkdir` | 4115 passed / **53 failed** / 2 ignored, exit 101 |
| directory deleted again | 4168 passed / **0 failed** / 2 ignored, exit 0 |

Two things follow. The `absent` scenario is not stable across repeated runs of the same
suite in the same session — its second run is a different scenario. And a read-side
analysis gate mutating the tree it is reading is worth a look on its own terms;
`contract_closure_corpus`'s own doc comment declines to emit build products for exactly
this reason ("generating one would be a WRITE into `AEON_DIR`, racing any concurrent
build"), while `ensure_generated` does it unconditionally.

**CLOSED.** All seven emitters carried the same shape, not just the first. Each now checks
`seam2::require_reference_tree` before it creates anything and creates its output directory
only after the bytes exist, so an absent tree stays absent and the three states read the
same. Byte-neutral on all four shapes. See
`docs/superpowers/notes/2026-08-30-reference-tree-write-guard.md`, whose gate
(`crates/sigil-harness/tests/reference_tree_write_guard.rs`) holds the property with an
emitter set parsed out of `ensure_generated` itself.

## Rows gated, and why it is a different unit from the literal count

Three quantities, three units. They are not interchangeable and only the third prices risk.

| unit | value | what it is good for |
|---|---|---|
| hardcoded-path literals, non-comment | **110** across 90 test files (146 lines incl. doc comments) | edit-sizing |
| helper call sites (`aeon_dir()` / `reference_tree()` / `reference_tree_for_profile()`) | **319** across 97 files | edit-sizing |
| rows in a file that reads the reference tree | **466** of 4170 (11.2%) | static exposure ceiling |
| rows whose behaviour actually changes | **398** of 4170 (9.5%) | **the risk unit** |

The two are far apart per site: `contract_closure_corpus.rs` carries 2 literals and gates
28 rows; `diag_assert_vector.rs` carries 1 literal and gates 15. `grep -c` measures typing,
not coverage.

The figure **322 / 181 / 129** published for this lane does not reproduce against this tree
under any of the units above. Whatever it counted, it is not the literal count (110 / 146),
not the helper call count (319), not rows-in-an-aeon-reading-file (466) and not
rows-that-change-behaviour (398). It should be re-derived before anything prices work off it.

## Left open

- **Not fixed, by instruction.** The 61 misdirecting rows and the 397 silent ones are
  classified here and untouched. Sequencing is the controller's.
- **SILENT is a lower bound.** Parsed strict-run failure sets lose up to 6 rows per scenario
  to `--nocapture` interleaving. Exact per-row attribution wants `--test-threads=1` over the
  affected binaries, which was not run for cost.
- **Nine scenarios, not all of them.** Correction (2) says which rows misdirect depends on
  which part is missing, and the union here already exceeds every single scenario by 2x.
  A tenth scenario would likely add rows.
- **The `AEON_DIR` write ORDERING is closed; the write itself is not.** The emitters no
  longer create anything before validating, and that landed byte-neutral without touching
  `golden/`, `pins.rs` or `repin.toml`. Whether `ensure_generated` should write into the
  reference tree AT ALL — rather than into a caller-supplied directory — remains a design
  question for the aeon-owned landing lane.
- **The hardcoded `AEON_DIR` default is CLOSED ON THE WRITE SIDE** (d-17, 2026-08-30, `2026-08-30-aeon-dir-write-requires-naming.md`); the read-only fallback is unchanged.
- **The hardcoded `AEON_DIR` default is enumerated, not changed.** 93 `.rs` files / 113
  non-comment occurrences; 29 of the 127 files that resolve `AEON_DIR` by literal or helper
  can reach the write. Recommendation and blast radius in the write-guard note.
- **No runtime confirmation.** Nothing here was checked against an emulator, and nothing
  here needs one.

## Reproducing

Logs, per-scenario row lists and the bucket JSON live outside the repo at
`/home/volence/sonic_hacks/.apc-af1763fc/` (`run-<scenario>.<mode>.log`,
`list-baseline.raw`, `classification.json`, `final.json`, `spy-*.tsv`). They are a
measurement of a moment, not an artifact this repo carries; the scenario recipe in this
note is what reproduces them.
