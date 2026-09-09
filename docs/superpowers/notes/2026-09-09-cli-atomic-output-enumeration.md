# CLI output artifacts install by rename: the enumeration, the calls, and the gap

Parcel `CLI-OUTPUT-WRITE-IS-NOT-ATOMIC`, branch `worktree-agent-a5ac413f33abd0932`,
off `299fb6bf`.

## The defect

`std::fs::write` is open-create-truncate then write. Between the two there is a
window in which the destination exists and is empty or partial. Writing an output
in place is ordinary for a build tool; what changed is that other lanes now poll
these artifacts while builds run. A missing file is loud and a reader handles it.
A truncated ROM is silent, and a consumer that loads one gets findings from half a
build shaped exactly like findings from a whole one.

## The enumeration

### Instrument, and its positive control

`grep -r` is a shell function on this machine that silently skips gitignored paths
and exits 0 with no output, so the sweep used `git grep` for tracked source and
`/usr/bin/grep` by absolute path elsewhere.

**The first sweep returned empty and the empty was wrong.** The pathspec
`crates/*/src` matched no tracked path, so `git grep` exited 0 with no output over
a population of zero files, which reads exactly like a clean workspace. It was
caught by refusing to accept the zero without an instrument that could have
returned non-empty. The corrected sweep asserts its own input first:

```
git ls-files 'crates/*/src/*.rs' 'crates/*/src/**/*.rs' | wc -l     # 123 files
git grep -nE "fs::write|File::create|OpenOptions|fs::copy|fs::rename" \
    -- 'crates/*/src/*.rs' 'crates/*/src/**/*.rs'                   # 45 hits
```

45 hits over 123 files is the positive control: the pattern demonstrably fires,
and it fires on this subject rather than merely on some subject of its class,
because among the hits are the four sites the parcel is about.

### The count

**The controller's count of four is confirmed, not refuted.** The `sigil` binary
had exactly four output-writing sites, and all four moved together:

| What it writes | Consumer |
| --- | --- |
| the `-o` image on the AS path | whoever passed `-o` |
| the `-o` image on the shared emp output tail | whoever passed `-o` |
| the `.lst` listing | convsym, the debugger symbol path, and lanes reading symbols |
| the native ROM | aeon's `build.sh`, and the lanes that poll its output |

Fixing the ROM alone would have been the exact defect this lane bans: a lane
polling the ROM while the listing beside it is still written in place learns
nothing from the ROM's atomicity. The enumeration to make is of the CONSUMING end,
and the `.lst` has consumers.

### Writing sites deliberately left alone, with reasons

The sweep reaches past the `sigil` binary. These were found and NOT moved:

* **`sigil-harness/src/native.rs`, three sites** (a `rom.bin`, and two `.lst`
  files). These write into a per-process private directory under
  `std::env::temp_dir()`, named from the pid, the build shape, and a monotonic
  counter, and the only reader is a `convsym` subprocess the same function spawns
  and waits for. Nothing outside the process can name the path, so there is no
  consumer to observe a partial file. Left alone.

* **`sigil-harness/src/seam1.rs` and `seam2.rs`** (the sound-blob and
  sound-tables build inputs). These are consumer-readable and are the closest
  thing to a real remaining member of the class. Left alone deliberately: they are
  outside this parcel's declared scope (`crates/sigil-cli/` plus one shared
  module), the helper they would need now exists and is public, and moving them
  changes byte-mover surface that another lane may be holding. **Flagged for the
  controller as the obvious follow-up**, not closed.

* **`sigil-harness/src/bin/{repin,refreeze,derive_offcanon}.rs`** are separate
  developer binaries run by hand, not artifacts a lane polls during a build. Out
  of scope; noted rather than moved.

* **`sigil-harness/src/test_support.rs`, `harness_root.rs`, and every `tests/`
  and `#[cfg(test)]` hit** are test fixtures with no external consumer.

* **`sigil-isa` and `sigil-frontend-as` `src/bin/gen_*_vectors.rs`** are golden
  regenerators run by hand. Also: `sigil-frontend-as` is being edited by two other
  agents this session and was not touched.

**Do not read the above as "the class is closed workspace-wide."** It is closed
for the `sigil` binary, which is what the parcel scoped, and the seam writers are
a known live remainder.

## The prior art, and the duplication removed

The mechanism already existed as `provenance::write_atomic`, private to the freeze
ledger and text-only, written by the FREEZE-WRITES-HOME parcel. Rather than add a
second implementation it was promoted to `sigil_harness::atomic_write`,
generalized to bytes, and `provenance::write_atomic` now delegates. One
implementation, not two that drift.

That parcel also left a warning worth carrying forward, and the module doc states
it: **temp-and-rename would not have prevented the bug it was fixing.** An atomic
rename installs a complete-but-invalid file atomically. Atomicity is not validity,
and conflating the two is a mistake two lanes already made in one morning.

## The file-mode contract, measured

A rename replaces a NAME, so permission comes from the containing DIRECTORY's
write bit; the destination file's own mode does not enter into it. Truncation is
the opposite. Measured here, `mv (GNU coreutils) 9.11`:

| Case | Result |
| --- | --- |
| rename over a 0444 file, directory 0755 | **succeeds**, contents change |
| rename over a 0444 file, directory 0555 | refused, `EACCES` |
| truncate a 0444 file, directory 0755 | **refused** |

The third row is the one the brief did not state and it is the one that matters:
`chmod a-w <artifact>` **does** stop a sigil write today, and after this parcel it
does **not**. Anyone freezing a sigil output artifact by file mode alone is relying
on behaviour this parcel removes, and must deny the containing directory as well.
This is written into the helper's own doc comment as a present-tense contract fact.

## The open calls, and what was chosen

* **Home for the helper: `sigil-harness`, its own module.** `sigil-cli` already
  depends on `sigil-harness` as a regular (not dev) dependency, so no new edge
  enters the shipping binary's graph and the crate-graph closure gate is unmoved.
  A module of its own rather than a second function in `provenance`, because a
  generic file installer living inside the freeze-ledger module is where the next
  reader will fail to find it.

* **Temp naming: `.<destination name>.<pid>.<seq>.tmp`, beside the destination.**
  Sibling because rename is atomic only within a filesystem and the destination's
  own directory is the only one guaranteed to share it. Dot-prefixed so a glob
  over an output directory does not sweep it up. A monotonic counter as well as
  the pid, which the existing precedent lacked: two threads installing the same
  destination would otherwise collide on one temp name.

* **Cleanup on failure: yes**, the temp is removed.

* **A failed write leaves the previous file intact: yes**, and this is a real
  improvement over truncation, where a failure part-way leaves a stub. It is what
  makes the callers' `exit(1)` on a write error a safe response rather than a
  half-destructive one.

* **Permissions: the destination's existing mode is carried across the rename.**
  `std::fs::rename` does not do this, because the installed file is a new inode
  carrying the temporary's mode, so without it a deliberately-set 0444 would be
  silently reset to the umask default. The doc says plainly that this is cosmetic
  with respect to blocking: preserving the mode must not be read as preserving the
  enforcement, which is exactly the misreading a preserved 0444 invites.

* **Two error shapes.** `write_atomic` renders `write <path>: <cause>` for callers
  that want a sentence; `write_atomic_io` returns the cause alone for the CLI,
  which names the destination itself and would otherwise print the path twice. No
  user-visible message changed.

## What could not be proved, named plainly

* **The race itself is not tested.** The property is that a concurrent reader
  never observes a short file. A test that writes then reads does not exercise it,
  and a test that spins a reader thread against the writer is timing-shaped: it
  would pass on a quiet machine and flake under load, and a flaky gate is worse
  than an honest gap. No sleep-based test was shipped. The guarantee rests on
  `rename(2)` being atomic within a filesystem, which is a documented kernel
  property and is argued, not measured, here.

* **Crash consistency is not claimed.** `sync_all` orders the data before the
  rename within this process, which a kill respects. A power loss may expose the
  directory entry against writes the kernel has not flushed.

* **Multi-file atomicity is not provided.** The ROM and the `.lst` are two
  renames, so a reader can still observe a new ROM beside an old listing. Each is
  individually whole; the SET is not transactional. A consumer that needs the pair
  to agree needs a staging protocol, which `golden/atomic_freeze.sh` already
  models for the goldens and which this does not attempt.

* **The reference-tree rows were not measured.** The parcel forbids setting
  `AEON_DIR`, so the full-workspace run leaves 379 reference-dependent rows
  unmeasured (plus 3 poisoned-lock failures downstream of the same panic). The
  controller runs that gate at landing.

## The gate, and why this shape

`crates/sigil-cli/tests/atomic_output_gate.rs` fails if any `.rs` file under
`crates/sigil-cli/src` contains a truncating whole-file write, so a future output
site cannot silently reintroduce the old behaviour.

It was adopted after judging it, not because it was suggested. The reasoning:

* **It can be absolute without being an always-red hazard.** A check that fires on
  correct code trains people to weaken it. This one has no correct code to fire
  on, because the installer is a strict superset of the banned call: same path,
  same bytes, installed by rename. Even a fixture written by an inline unit test is
  served by it.

* **A source-text check's own source must not be inside its subject.** The gate
  lives in `tests/` and scans `src/`, so it cannot read its own text; and the
  banned literal is assembled with `concat!` so the file's own mention of it is not
  the string it searches for.

* **Its doc states the SHAPE and never a live name from the population.** This lane
  has already had a sweep count its own doc comment and drop a real member from the
  list it documented. The population is a directory walked at test time, not a list
  written down.

* **Two vacuity guards.** An input assertion on the file count, with the floor
  derived from what a binary crate must minimally contain rather than copied from
  today's count, plus an explicit check that the walk reached the binary root; and
  an in-process positive control asserting the predicate fires on a planted
  truncating write and stays quiet on the installer.

**Red-first proof.** A truncating write was appended to `crates/sigil-cli/src/bin/
emp_census.rs`, quoted back from disk and confirmed dirty by `git status
--porcelain` before the run. The gate failed naming that file at line 133 with the
truncation message, which is this parcel's own mechanism and not a different guard
firing first; the positive control still passed in the same run, so the red was not
the control misfiring. Planting in `src/bin/` rather than beside `main.rs` also
proved the walk recurses. Restored with `git checkout HEAD -- <path>` from a
COMMITTED baseline and verified with `git status --porcelain`, not `git diff
--stat`, which a checkout's staging would have shown as clean regardless.

## Verification

`SIGIL_ALLOW_PARTIAL=1 cargo test --release --workspace --no-fail-fast`, run from
this worktree on this branch: **4890 passed, 0 failed, 2 ignored.** Without the
partial flag the same tree is 4508 passed / 382 failed, every one of which carries
`NO REFERENCE TREE IS NAMED` or is a lock poisoned by a sibling that did.
