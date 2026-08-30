# A write into the reference tree refuses unless `AEON_DIR` named it

`test_support::aeon_dir` falls back to a hardcoded path when `AEON_DIR` is unset, and that
path is the owner's LIVE aeon working checkout. Every sound emitter
`native::ensure_generated` drives writes INSIDE the tree it resolves. Decision `d-17` rules
that an unset `AEON_DIR` must refuse by name rather than resolve to that checkout silently.

This note records what was changed, the population it was derived from and how, the gate,
the byte proof, and what is deliberately left on the other side of the line.

## The argument is invisibility, not damage

Aeon's build regenerates all nineteen artifacts unconditionally, so it is self-healing
against a foreign write; nothing is corrupted by one. The hazard is the narrower **stale
read**: a process that reads those artifacts without regenerating first gets whatever last
wrote them. What justifies a refusal is that a fallback to somebody's live working tree is
**structurally incapable of announcing its own failure** — and `engine/sound/generated/` is
gitignored in aeon, which removes the last surface where a person could notice. A refusal is
loud at the caller's own site and costs one exported variable.

The refusal text says exactly that and no more. A corruption story would be scarier,
falsifiable, and would deserve to lose.

## The change: one precondition, plus two argv callers that declare their tree

`seam2::require_named_reference_tree(aeon)` returns `Ok(())` when `AEON_DIR` is set, and
otherwise an error naming the variable, the fallback it would have resolved to, and the path
the write was aimed at. `seam2::require_reference_tree` — already the shared precondition of
all seven emitters and of `ensure_generated`'s own entry — calls it FIRST, before the tree's
contents are probed.

The order is the load-bearing part. Reversed, an unset `AEON_DIR` sends the content probe to
the live checkout, where it finds a complete tree and passes, and the write proceeds into
exactly the tree the check exists to keep it out of.

`test_support::LIVE_TREE_FALLBACK` is now a constant that `aeon_dir` resolves to and the
refusal names, so the fallback and the sentence describing it cannot drift apart. This is the
same shape as `SOUND_PLACEMENT_MAP_REL`, which `bank_anchors` reads and
`require_reference_tree` probes.

**Two callers name their tree on the command line rather than through the environment**, and
both publish it as their own `AEON_DIR` before any emit runs:

| binary | how aeon drives it | why it must not refuse |
|---|---|---|
| `emit_sound_blob` | `build.sh:353` — `--aeon . --out-dir engine/sound/generated` | THE hard aeon→sigil build dependency; there is no fallback, the `.asm` is gone |
| `sigil build` | `build.sh:620` — `sigil build --aeon . --native …` | `run_build_native` → `native::build_*` → `ensure_generated` |

Without this, a refusal on unset `AEON_DIR` breaks **every aeon build**, in a repo this lane
does not own. With it, one rule covers every writing process and the argv caller is not an
exception to it. `refreeze --freeze` already refused on unset `AEON_DIR` for this same reason
(`refreeze.rs:118`); this generalises that precedent to the write path.

## The write-reaching population, measured rather than name-matched

The previous enumeration used a syntactic closure over harness sources and said so. This one
is dynamic, from two independent witnesses over ONE full-workspace run — 360 suites
reporting, `SIGIL_STRICT_GATE=1`, reference tree provisioned, `AEON_DIR` set.

**Witness A — fs level.** An `LD_PRELOAD` interposer over `open`/`open64`/`openat`/`creat`/
`fopen`/`mkdir`/`mkdirat`/`unlink`/`unlinkat`/`rmdir`/`rename`/`truncate`, logging every
write-capable call whose path lies under the reference tree. Control first: a bare `touch`
and `rm` inside the tree logged one `open` and one `unlinkat`, so the witness sees what it
claims to see.

* **2938 write events** into the reference tree across the run.
* **Every one of them targets `engine/sound/generated`** — 2147 file creates inside it, 791
  `mkdir`s of the directory itself. **Zero writes anywhere else in the tree.**
* 26 distinct executables performed them.

**Witness B — in-process attribution.** A temporary probe at the emitters' shared write
precondition logging the executable and the libtest thread (which carries the row name).
Removed before commit; it is a measurement, not a feature.

* **985 entries**, **33 distinct executables** (one further entry is a spliced append, not a
  real executable), **68 distinct (binary, row) pairs**.
* Every one of Witness A's 26 writing executables appears in Witness B's 33. **No writer
  reaches the reference tree except through the precondition**, which is what makes a single
  check at that choke point cover the whole write population.
* Resolved to sources: **32 test source files + the `sigil` binary**, plus `emit_sound_blob`,
  which no test spawns but aeon's build does — **34 source files reach the write**, against
  the brief's syntactic estimate of 29. The syntactic closure was close on the test files and
  structurally blind to the two argv-driven binaries.

**What this found that a name-match closure did not.** The `sigil` CLI binary is itself a
write-reaching executable — 234 write events, entering the precondition 72 times — because
`run_build_native` calls the native builders, which call `ensure_generated`. Nothing in
`sigil-cli/src` mentions `ensure_generated`, `seam1`, `seam2` or any generated filename, so a
grep-shaped closure over the harness's own sources does not reach it. It is also the binary
aeon's `build.sh` drives. **The measurement is the reason this parcel does not break aeon's
build.**

### Limits of the method, stated

* It measures the rows that RAN. A row that is `#[ignore]`d, feature-gated off, or skipped in
  this configuration cannot appear. `SIGIL_STRICT_GATE=1` was used to maximise the executed
  set.
* Thread-name attribution is best-effort: a handful of appends interleaved and a few entries
  carry an empty or spliced thread name. Binary-level attribution is unaffected.
* Subprocess writes are attributed to the process, not to the row that spawned it — `sigil ::
  main` is one row in the table for that reason. The spawning rows appear separately.
* The interposer filters on an ABSOLUTE path prefix, so it under-counts a process whose cwd
  is the tree and whose paths are relative. That is exactly aeon's `build.sh`, and it is why
  the aeon-build check below is witnessed by artifact mtimes rather than by the interposer.

### The literal counts, re-derived

Counted here, not taken from the brief.

| unit | files | occurrences |
|---|---|---|
| `.rs`, non-comment (the brief's unit) | **93** | **113** |
| `.rs`, including comments | 108 | 145 |
| `.sh` / `.toml` / `.py`, non-comment | 15 | 25 |

**Both of the brief's figures reproduce exactly.** 93 files / 113 occurrences as the
non-comment `.rs` literal, and **127** as "literal OR `aeon_dir()` / `reference_tree*()`
call" (93 literal-carrying ∪ 98 helper-calling). Widening the predicate once more to include
files that name the env var directly gives 135 — a third unit, not a correction. No
disagreement with the brief on either figure.

None of them counts coverage. **The risk unit is rows, and the rows that reach a WRITE are
the 68 measured above.** The brief's `29 of 127` write-reaching files was a syntactic
file-level approximation and was described as one; measured it is 34 source files — close on
the test files, and structurally blind to the two argv-driven binaries.

## The gate

`crates/sigil-harness/tests/reference_tree_named_write.rs`, one row, run by every
`cargo test --workspace` — which is what `scripts/landing-run.sh` invokes. It needs no
reference tree of its own, so it never skips and is not strict-gated.

The property needs `AEON_DIR` ABSENT, and a landing run always sets it. The gate therefore
asserts in a CHILD process spawned from the test binary with the variable removed, rather than
mutating the environment of a running suite.

**The expectation is derived, three ways.** The emitter set is PARSED OUT OF
`ensure_generated`'s own body (`tests/common/mod.rs`, now shared with
`reference_tree_write_guard` rather than copied into it). The path the refusal must name is
`test_support::LIVE_TREE_FALLBACK` itself. The ordering is read off
`seam2::SOUND_PLACEMENT_MAP_REL`: the unset-env refusal must NOT mention it — the content
probe never ran — and the `AEON_DIR`-set refusal MUST.

**It distinguishes "the refusal fired" from "nothing ran".** The child must exit successfully
AND print libtest's own `test result: ok.` line; it emits one `WITNESS refused` line per
emitter plus one for `ensure_generated`'s entry and one `WITNESS content-refusal` line per
emitter, and the parent reconciles both counts against the parsed set. An emitter reporting
SUCCESS against an empty tree is a failure, not an absence of evidence. Every count that
cannot be established panics with `UNMEASURABLE`, carrying the child's output.

The parent also checks the scratch tree itself after the child exits, independently of the
child's own assertions.

### Red-first, four ways

1. **The precondition call removed** — `emit_sound_blob refused without naming the fallback
   tree /home/volence/sonic_hacks/aeon its write would have gone to`. FAILED.
2. **The precondition moved AFTER the content probe** — same row FAILED, reported through the
   fallback-naming assertion, which fires before the dedicated ordering assertion because the
   content error arrives first. The ordering assertion is the second line of defence, not the
   first; both are live.
3. **One arm dropped from the gate's table** — ``ensure_generated` drives
   `emit_pitchtable_artifacts`, which this gate does not exercise. Its writes into the
   reference tree are unmeasured`. The derivation fails, not a hand-maintained list.
4. **The child made something other than the test binary** — `UNMEASURABLE: the `unset` child
   exited 0 without libtest's own `test result: ok.` line, so it cannot be established that it
   ran any test at all.` A green from a run that did not happen is refused by name.

All four restored; both gates green on the fixed tree.

### The refusal, live

`repin --check` with `AEON_DIR` unset, which before this change wrote all nineteen artifacts
into the owner's live checkout without a trace:

```
ensure_generated writes into the reference tree: AEON_DIR is not set, so the reference
tree written into is one nobody named. Unset, it resolves to the hardcoded fallback
/home/volence/sonic_hacks/aeon — the live aeon working checkout — whose
engine/sound/generated/ is gitignored, so a write there leaves no trace in `git status`
and nothing records which process produced the bytes a later read picks up. Set AEON_DIR
to a reference tree you provisioned (scripts/provision-aeon-ref.sh). Nothing was created.
Refused write target: /home/volence/sonic_hacks/aeon
```

## Byte identity — the answer the aeon lane is waiting on

Reference tree `/home/volence/sonic_hacks/.aeon-ref-a312190e`, aeon
`ec6a4791db346ec8c6672632109f85415b873e49`, provisioned by `scripts/provision-aeon-ref.sh`;
provisioning witness `repin --check` -> **`pins.rs unchanged`**.

All four canonical shapes rebuilt from scratch, one shape per invocation, through the PATCHED
`sigil` and `emit_sound_blob` — and deliberately with **`AEON_DIR` UNSET**, which is how
aeon's own `build.sh` runs them:

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

That run is doing double duty: it is the byte proof AND the proof that the refusal leaves
aeon's build alone, because those four builds are aeon's `build.sh` with no `AEON_DIR` in the
environment. The artifacts were rewritten during them (`engine/sound/generated` mtimes move),
so the emit path ran rather than being skipped.

## The suite can still run, and how that was checked

`scripts/landing-run.sh:334` exports `AEON_DIR="$AEON"` into the cargo command, so every
suite row runs with the variable set and sees no change. The gate that needs it ABSENT
removes it for a child process only. This was checked by running the landing wrapper itself,
not by reading it.

The wrapper's refusal path was also exercised, unintentionally and usefully: pointed at a tree
missing two of the four ROMs it **REFUSED with exit 2** naming the missing files, rather than
running and going red for a provisioning reason.

## Left open — and one of these is a hole, not a nicety

* **The shell-side fallbacks are UNCHANGED and one of them defeats this check.**
  `scripts/landing-run.sh:207` resolves
  `${AEON_ARG:-${AEON_DIR:-/home/volence/sonic_hacks/aeon}}` and then EXPORTS the result, so a
  landing run invoked with neither `--aeon` nor `AEON_DIR` sets `AEON_DIR` to the live checkout
  and the write precondition passes. The same literal is in
  `golden/capture_goldens.sh:75` and `golden/derive_offcanonical_sizes.sh:25`.
  It is materially better than the Rust fallback — the wrapper PRINTS the tree it chose, in
  the log header and on stdout, so it announces itself — but it is still a silent selection of
  the live tree. Closing it is a change to the runner's contract for every lane, and
  `capture_goldens.sh` is inside the aeon-owned `golden/` tree, so it is **reported here and
  not taken**.
* **`scripts/nightly_source_gates.sh`'s SELF-AUDIT IS ALREADY REFUSING, and this parcel adds
  a fifth row to the pile.** The lane classifies every `crates/*/tests/*.rs` matching
  `AEON_DIR|aeon_dir|reference_tree|--aeon` as either listed in `SOURCE_GATES` or derivably
  artifact-dependent (naming a ROM, a `.lst`, or `golden`), and REFUSES TO RUN — `COULD NOT
  RUN`, exit 2 — on anything else. Reproduced against this tree: **5 unclassified**, of which
  **4 are on master already** — `hole_interior_reserved`, `section_alignment_declared`,
  `region_end_contracts`, and `reference_tree_write_guard` (added by yesterday's parcel). The
  fifth is this parcel's `reference_tree_named_write`.

  Both reference-tree gates match on PROSE — their headers explain what `AEON_DIR` is and say
  they need no reference tree of their own — which is the exact false-positive shape
  `docs/OVERSEER.md` already predicts. Neither reads a reference tree, so neither belongs in
  `SOURCE_GATES` (whose stated contract is "inputs are aeon SOURCE") and neither is
  artifact-dependent. The classifier needs a third bucket, derived rather than hand-listed —
  a file that carries no `aeon_dir()` / `reference_tree(` CALL and no `AEON_DIR` env read.
  **Reported, not taken:** the majority of the pile is not this parcel's, the fix changes a
  nightly lane's contract, and it wants its own red-first proof. Nothing in `cargo test`
  detects this, which is why it has been standing.

* **The read-only fallback is untouched, by instruction.** The wide option changes ~398 rows'
  behaviour for anyone who has not exported the variable; that is a suite-wide announcement,
  not a parcel.
* **Whether `ensure_generated` should write into the reference tree AT ALL** — rather than
  into a caller-supplied directory — remains the design question it was, and is the aeon-owned
  landing lane's to sequence.
* **The measurement covers rows that ran.** See the limits above; an ignored or
  differently-configured row cannot appear in it.
* **No runtime confirmation.** Nothing here was checked against an emulator, and nothing here
  needs one.
