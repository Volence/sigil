# The suite stops mkdir'ing inside a reference tree it has not established is there

`native::ensure_generated` drives seven emitters at `$AEON_DIR/engine/sound/generated`.
Each of the seven opened with `create_dir_all(out_dir)` as its FIRST statement, before it
read anything. Pointed at an absent tree they therefore created `$AEON_DIR`'s root and then
failed on the missing source — and a root that exists flips every root-probing skip guard in
this suite (`if !aeon.exists()`) from "skip" to "run against an empty tree". That is why an
absent-tree run was not reproducible twice in one session: its second run measured a
different tree than its first.

This note records the fix, the gate that holds it, the byte proof, and the enumeration of
the wider `AEON_DIR`-default exposure, which is REPORTED and NOT CHANGED.

## What was enumerated, not assumed

All seven, checked individually — the audit had confirmed only the first:

| emitter | site | shape before |
|---|---|---|
| `seam1::emit_sound_blob` | `seam1.rs:729` | `create_dir_all` first statement |
| `seam2::emit_dac_artifacts` | `seam2.rs:584` | `create_dir_all` first statement |
| `seam2::emit_sound_tables_artifacts` | `seam2.rs:894` | `create_dir_all` first statement |
| `seam2::emit_pitchtable_artifacts` | `seam2.rs:988` | `create_dir_all` first statement |
| `seam2::emit_seq_opcode_artifacts` | `seam2.rs:999` | `create_dir_all` first statement |
| `seam2::emit_sfx_artifacts` | `seam2.rs:1017` | `create_dir_all` first statement |
| `seam2::emit_mt_artifacts` | `seam2.rs:1160` | `create_dir_all` first statement |

Seven of seven, not one of seven. The audit's finding generalised across the whole set.

## The shape of the fix

Two changes, and the second is the one that makes the property structural rather than
conventional.

1. **A precondition each emitter checks before it creates anything.**
   `seam2::require_reference_tree(aeon)` returns `Ok(())` when the tree carries
   `SOUND_PLACEMENT_MAP_REL`, and otherwise an error NAMING the absent path with nothing
   created. The probe is not a second opinion about what an aeon tree is: `bank_anchors` —
   the first thing every one of these emitters reads through — resolves its path from the
   SAME constant, so the guard and the input it guards cannot name different files.

2. **The `create_dir_all` moved after the bytes exist.** Every emitter now produces its
   artifacts and only then creates the output directory. So even inside a tree that IS
   present, a failing emit leaves no directory behind. The three looping emitters (seq,
   sfx, mt) accumulate their artifacts and write in a second pass.

`ensure_generated` also checks the precondition at its own entry, so its refusal does not
depend on which emitter happens to be first in its body.

## Byte identity — the answer the aeon lane was waiting for

Reference tree `/home/volence/sonic_hacks/.aeon-ref-adf95ba3`, aeon
`ec6a4791db346ec8c6672632109f85415b873e49`, provisioned by `scripts/provision-aeon-ref.sh`;
provisioning witness `repin --check` -> **`pins.rs unchanged`**. All four canonical shapes
rebuilt from scratch through `SIGIL_BUILD`/`SIGIL_EMIT` carrying this change:

| shape | built CRC32/size | golden CRC32/size | verdict |
|---|---|---|---|
| `s4.bin` | `6e2f9b22`/719315 | `6e2f9b22`/719315 | MATCHES THE GOLDEN |
| `s4.debug.bin` | `6516fc68`/736315 | `6516fc68`/736315 | MATCHES THE GOLDEN |
| `demo.bin` | `9223a60d`/96450 | `9223a60d`/96450 | MATCHES THE GOLDEN |
| `demo.debug.bin` | `d30c3636`/101333 | `d30c3636`/101333 | MATCHES THE GOLDEN |

**Baseline note.** These are the **pre-chain-189** goldens; they were correct against the baseline
standing when this ran. The freeze then moved two of them. Re-proved post-freeze at aeon `3f143178`
(merge `f7b20982`): all four rebuilt and matched — `s4 63451f96/719315`, `s4.debug 3aa7cb12/736315`,
`demo 9223a60d/96450`, `demo.debug d30c3636/101333`. Byte-neutral against both baselines.


**BYTE-NEUTRAL, all four.** Nothing in `golden/`, `pins.rs` or `repin.toml` is touched.

## The gate

`crates/sigil-harness/tests/reference_tree_write_guard.rs`, two rows, run by every
`cargo test --workspace` — which is what `scripts/landing-run.sh` invokes. It needs no
reference tree of its own, so it never skips and is not strict-gated.

* `no_emitter_creates_anything_under_an_absent_reference_tree` points every emitter at a
  path that does not exist and requires of each: an `Err`, an `Err` that NAMES the path, and
  the path still absent afterwards.
* `ensure_generated_refuses_before_it_touches_an_absent_tree` holds the same property at the
  entry point.

**The expectation is derived, not copied.** The set of emitters under test is PARSED OUT OF
`ensure_generated`'s own body in `src/native.rs`. An eighth emitter added there and not here
fails the gate by name; the coverage cannot silently become six-sevenths.

**It distinguishes "no writes" from "nothing ran"**, which from the filesystem look
identical. Four refusals, each panicking with `UNMEASURABLE` rather than passing: the source
must be readable; the parse must yield a non-empty emitter set; every parsed emitter must
have an arm and every arm must have been invoked (`ran` reconciles against the parsed count);
and every invocation must have RETURNED — an emitter reporting SUCCESS against a
non-existent tree is a failure, because nothing could have been read.

### Red-first, three ways

1. **`seam1` reverted to mkdir-first** — `1 of 7 emitters created a path inside a reference
   tree that does not exist … emit_sound_blob created /tmp/sigil-absent-aeon-…`, and
   `ensure_generated created … while refusing. The mkdir must follow the validation, not
   precede it.` Both rows FAILED.
2. **`emit_dac_artifacts` also reverted** — the same row reported `2 of 7`, naming both. The
   seam2 arms are live, not decorative.
3. **One arm removed from `EXERCISED`** — ``ensure_generated` drives
   `emit_pitchtable_artifacts`, which this gate does not exercise. Its writes into the
   reference tree are unmeasured`. The derivation is what fails, not a hand-maintained list.

All three restored; the gate is green on the fixed tree.

## The three-state re-measurement

Run the way the original was — an ordinary `cargo test --release --workspace` with
`AEON_DIR` pointed at a directory that does not exist, three times in one session.

| tree state | ordinary run | suites | exit | `$AEON_DIR` after |
|---|---|---|---|---|
| absent, pristine | 4170 passed / **0 failed** / 2 ignored | 360 | 0 | **absent** |
| same command again | 4170 passed / **0 failed** / 2 ignored | 360 | 0 | **absent** |
| path deleted, run again | 4170 passed / **0 failed** / 2 ignored | 360 | 0 | **absent** |

Before the fix these three states read 0 failed / **53 failed** / 0 failed. **The
instability is gone: the second run is now the same scenario as the first**, because nothing
conjures the root between them. (4170 rather than 4168 is this change's two new gate rows —
the landing run reconciles it as `4168 baseline + 2 new = 4170 observed`.)

The wrapper that took these three states printed BLANK counters on its first attempt (`bc`
is absent on this machine) and the run was re-tallied from the saved logs rather than
believed. A blank is the correct failure; a `0` would have read as a clean result. The same
distinction is what the gate above encodes.

## The `AEON_DIR` default — enumerated, NOT changed

`AEON_DIR` falls back to a hardcoded default of the LIVE aeon checkout,
`/home/volence/sonic_hacks/aeon`. Counted here rather than taken from the brief; the numbers
agree for the Rust-code unit and the disagreements are in units the brief did not name.

| unit | files | occurrences |
|---|---|---|
| `.rs`, non-comment (the brief's unit) | **93** | **113** |
| `.rs`, including doc comments | 108 | 145 |
| all file types, non-comment (`.rs`/`.sh`/`.toml`/`.py`) | 108 | 138 |
| all file types, including prose and `.md` | 145 | 225 |

**The brief's 93 files / 113 occurrences reproduces exactly** as the non-comment `.rs` unit.
It is not the whole exposure: `.sh` 5, `.py` 9, `.toml` 11 (all eleven in
`golden/provenance.toml`, recorded A/B evidence paths, not resolvable inputs). The standard
fallback expression accounts for 109 of the 113 Rust occurrences across 89 files; the
remaining four are the `PathBuf::from(...)` spelling in
`tranche{2,3,4,5}_negative_probes.rs`.

The audit note's `110 across 90 test files` does not reproduce against this tree either; the
present figure is 113 across 93. Neither number should be quoted without its unit.

### Which of them can reach a WRITE

**The only write into `$AEON_DIR` in this workspace is `ensure_generated` and the seven
emitters it drives.** Every other `create_dir_all`/`fs::write` in the harness that looked
like a candidate targets `std::env::temp_dir()` (the shadow trees, the convsym scratch dirs)
or the sigil-side golden/target trees.

Transitive closure over the harness sources seeded on `ensure_generated` gives 19 functions;
the load-bearing ones are `build_native_rom`, `build_native_rom_with_listing`,
`build_native_rom_chained`, `build_rom_chained{,_with_listing}`, `build_native_full_file`,
`build_full_file_chained` and `resolve_frozen_{sections,layout}` — i.e. **every sound-on
native ROM build reaches the write**, not only the sound port gates.

| population | files | write-reaching | read-only |
|---|---|---|---|
| `.rs` carrying the literal default | 93 | **11** | 82 |
| `.rs` resolving `AEON_DIR` at all (literal OR `aeon_dir()`/`reference_tree*()`) | 127 | **29** | 98 |

The second row is the honest exposure unit: the helper carries the same default, so a file
using `test_support::aeon_dir()` is exposed identically to one spelling the literal itself.
Both counts come from a syntactic name-match closure, which over-approximates (a name shared
with an unrelated local) and under-approximates (dynamic dispatch); treat them as the
edit-sizing figure, not a proof of reach.

### What CANNOT be established, in either direction

`engine/sound/generated/` is **gitignored in aeon**. A write there leaves **no record in
git**, measured directly: a filesystem snapshot of the reference tree either side of ONE
two-row suite file (`sigil-cli --test dac_bank_port`, `2 passed / 0 failed`) shows all
**nineteen** artifacts rewritten — `z80_sound_blob{,_debug}`, `dac_{blip,shared}_bank`,
`dac_sample_tab`, `mt_bank_body{,_debug}`, `mt_songtable{,_debug}`,
`mt_songpatchtable{,_debug}`, `sfx_bank{,_debug}`, `sfx_blob_win_tab{,_debug}`,
`seq_opcode_tab{,_debug}`, `sound_tables_z80`, `movingtrucks_pitchtable` — while
`git status` in that tree stayed **completely clean** and `git check-ignore` confirms all
nineteen are ignored. The file count either side is identical (1381), so the write is a
rewrite, invisible to every git-based review. All nineteen files in aeon's live `generated/`
are stamped
2026-08-30 03:28, and **that timestamp is not evidence of anything**: aeon's own build
produces exactly those files through `SIGIL_EMIT`, so the stamp is equally consistent with
an aeon build and with a sigil suite run.

**The finding is the ambiguity itself.** Nothing in either repo records WHO wrote those
files. This note therefore claims neither that sigil's suite has raced a live aeon build nor
that it has not; both are unfalsifiable with the evidence that exists. What is now true is
that a sigil run pointed at an ABSENT tree cannot create one — the remaining exposure is a
run pointed at a PRESENT live tree, which is what the default makes easy.

### Recommendation, with its blast radius

**TAKEN 2026-08-30 (d-17), narrow option only** — `2026-08-30-aeon-dir-write-requires-naming.md`. Byte-neutral on all four shapes. The measured write-reaching population is 34 source files, and it includes the `sigil` CLI, which the syntactic closure below could not see.

**Recommended (not taken here — sequencing is the controller's).** Retire the hardcoded
default from the write-reaching population FIRST, not from all 93 sites at once: make
`AEON_DIR` REQUIRED on any path that reaches `ensure_generated`, and leave the read-only
sites' fallback alone for now.

* **Blast radius, narrow option (29 files):** every write-reaching file must have `AEON_DIR`
  exported. `scripts/landing-run.sh` and `provision-aeon-ref.sh` already export it, so the
  landing and nightly paths are unaffected. A developer running a bare
  `cargo test -p sigil-cli` with no `AEON_DIR` would move from "silently builds against the
  live aeon checkout" to "refuses by name" — a behaviour change that is the point, but one
  other lanes must be told about before it lands.
* **Blast radius, wide option (all 93/127 sites):** every reference-dependent row in the
  suite starts skipping (or, under `SIGIL_STRICT_GATE`, failing) when `AEON_DIR` is unset,
  where today they silently find the live tree and pass. That is 398 rows changing behaviour
  for anyone who has not exported the variable, which is a suite-wide announcement, not a
  parcel.
* **Cheapest partial, if neither is scheduled:** the 11 literal-carrying write-reaching
  files are a two-hour edit and close the "a suite run mutates the live checkout" hole for
  the sound stack specifically, leaving the read-only default as a convenience.

## Left open

* **The 93 defaults are untouched, by instruction.** Only the write-side ordering changed.
* **No runtime confirmation.** Nothing here was checked against an emulator, and nothing
  here needs one.
* **The write-reach closure is syntactic.** An fs-level `LD_PRELOAD` interposer over the
  full workspace would settle reach exactly; the name-match closure is what was run.
