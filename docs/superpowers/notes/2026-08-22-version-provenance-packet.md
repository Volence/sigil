# `sigil --version` — the source-revision witness

Branch `feat/version-provenance`, off master `5c75b5b6`.

## The problem this answers

The shared `target/release/sigil` sat three days stale while every aeon ROM build
invoked it, and nothing detected it because nothing *could*. Byte identity is
silent on provenance by construction: a stale assembler and a current one emit
byte-identical ROMs whenever the source did not change, so a matching CRC is
exactly as consistent with a three-day-old binary as with a fresh one. No amount
of CRC discipline closes that, because the CRC is not the thing in doubt.

The fix is to stop inferring and start asking. `sigil --version` now reports the
source revision the executable was built from.

## What the binary says

```
sigil 0.1.0 (8af046eb)
  revision:  8af046eb2ab6ebef74a5c6a48ada37dfb0548b3c
  branch:    feat/version-provenance
  committed: 2026-08-22T16:35:56-04:00
  tree:      clean at capture — no uncommitted changes
  source:    /home/volence/sonic_hacks/sigil/.claude/worktrees/agent-adf07383946606196
  freshness: revision is re-captured whenever git HEAD or refs move (cargo tracks HEAD,refs,packed-refs).
             tree state is a build-time snapshot; cargo has no trigger for uncommitted
             edits, so it may under-report dirt if this binary was relinked without
             HEAD moving. Compare `revision` against `git rev-parse HEAD` to check
             this binary against a source tree.
```

A dirty build tags as `(8af046eb-dirty)` on the first line, so it cannot be read
as the clean commit it was built beside. A build with no git checkout, or with no
`git` on PATH, reports:

```
sigil 0.1.0 (revision-unknown)
  revision:  unknown — git unavailable: <reason>
  tree:      unknown — not determined
  source:    unknown
  freshness: this binary carries NO revision, so nothing here can confirm it matches any source tree. Do not treat it as current.
```

Never a blank, never a plausible-looking placeholder. The first line is the
greppable identity for a build script; `-V` prints the identical banner, so a
script that reaches for the cheap flag cannot miss the caveat.

## The rerun-trigger analysis — the actual deliverable

A staleness witness that can itself go stale is worse than no witness, because it
converts "I don't know" into a confident wrong answer. A `build.rs` that bakes a
SHA and is then not re-run by cargo reproduces the original failure one level
down: the binary relinks against newer code while still reporting the old SHA,
and now the thing you built to catch staleness is the thing lying to you. So the
triggers are the parcel; the printing is incidental.

### Measured, not assumed

Two cargo behaviours were established empirically with a throwaway probe crate
(cargo 1.97.1) rather than reasoned about:

1. **A `rerun-if-changed` naming a missing path forces an unconditional rerun,
   and that rerun recompiles the dependent crate even when the build script's
   output is byte-identical.** Cargo reports
   `Dirty rerunprobe: the file '__always_rerun__' is missing` and re-invokes
   rustc. There is no "output unchanged, skip the dependent" optimisation to
   lean on. This is what prices the always-rerun option.
2. **Directory tracking is recursive and catches file *creation*, not only
   content edits.** Touching a file inside a tracked directory and adding a new
   file to it both produce `Dirty: the file 'watched' has changed`. This is what
   makes `refs/` a sound trigger rather than a partial one.

### What is tracked

| Trigger | Why |
|---|---|
| `<git-dir>/HEAD` | Moves on every checkout, and *is* the revision when HEAD is detached. Resolved from the worktree's own git dir — in a linked worktree HEAD is per-worktree while refs are shared, so the two dirs are resolved separately (`git rev-parse --path-format=absolute --git-dir --git-common-dir`). |
| `<common-dir>/refs` | Tracked as a directory. Catches a commit rewriting a loose ref, and catches a loose ref being *created* where the ref had been packed — finding (2) above is what closes that hole. |
| `<common-dir>/packed-refs` | The other half of ref resolution. |
| `build.rs` | Emitting any `rerun-if-changed` replaces cargo's default whole-package tracking, so the script must name itself or edits to it stop taking effect. |

Each is emitted only if it exists, because a missing trigger *is* the
unconditional-rerun cost this design refuses.

**Proven to fire.** With zero source changes, an empty commit followed by
`cargo build`:

```
$ git commit --allow-empty -m probe && cargo build -p sigil-cli --bin sigil -v
Dirty sigil-cli: the file `/home/volence/sonic_hacks/sigil/.git/refs` has changed
   Compiling sigil-cli
$ ./target/debug/sigil --version | head -1
sigil 0.1.0 (a7ec9ae1)      # was 8af046eb
```

Note the path cargo names: `.git/refs` in the **main checkout's** common dir, not
the worktree's — the git-dir/common-dir split is load-bearing here, not
defensive coding.

### Cost, measured

| Situation | Cost |
|---|---|
| HEAD unchanged — `cargo build --release -p sigil-cli` | **0.02 s** (`Fresh`; the build script does not run) |
| HEAD moved — `cargo build --release -p sigil-cli --bin sigil` | **1.27 s** (what a ROM-build workflow pays) |
| HEAD moved — `cargo test --release -p sigil-cli --no-run` | **13.2 s** wall / 167 s CPU (recompile + relink of all 138 test binaries) |

Steady-state cost is nil. The 13.2 s figure is the important one, because it is
precisely the tax the unconditional-rerun option would levy on **every** `cargo
test` and every `cargo build`, changed or not, to keep one boolean fresh. Paying
it only when HEAD actually moves — the moment you *want* a rebuilt binary — is
the whole design.

### What cannot be tracked, and the call made

**Working-tree dirtiness has no file whose mtime follows it.** Two ways to cover
it were considered and both are refused, for reasons that are facts about this
repository rather than preferences:

* **Track the repository root as a directory.** Not merely expensive —
  impossible. `target/` lives at the repo root and every build mutates it, so the
  trigger would be dirty on every build *because of the build*, and the build
  never reaches a fixed point. Cargo cannot exclude a subtree from a tracked
  directory. Enumerating siblings instead (`crates`, `docs`, `examples`, …) is
  fragile in the worst way: a top-level directory added later escapes tracking
  silently, which is the same confident-wrong-answer class.
* **Force the script to rerun on every build.** Priced by finding (1): every
  `cargo build` and every `cargo test` would recompile `sigil-cli` and relink all
  138 of its integration-test binaries, whether or not anything changed, and
  whether or not the captured provenance differs. Measured at **13.2 s wall /
  167 s CPU per invocation** (table below) — a permanent tax, paid every time,
  to refresh one boolean.

**The call: capture tree state as an explicitly-labelled snapshot, and make the
binary say so in its own output.** A version string that admits "this may be
stale under condition X" is a real witness; one that silently claims freshness it
cannot back is the defect being fixed. The banner therefore separates its two
claims by confidence rather than presenting them as equally solid:

* `revision` / `branch` / `committed` — re-captured by cargo whenever HEAD or the
  refs move. No caveat.
* `tree` — truthful as of the last capture. It can **under-report dirt** if the
  binary was relinked without HEAD moving (edit a file in another crate, build:
  the binary is new code, the tree line is old). The banner names that direction
  explicitly and tells the reader the one command that settles it.

### Residual gaps (named, not papered over)

1. **Tree state can under-report dirt**, as above. Not closable at acceptable
   cost; disclosed in the output itself and asserted by a test so the disclosure
   cannot be dropped in a later tidy-up.
2. **Tree state is not cross-checkable by any test.** A mismatch between the
   reported state and `git status` at test time is legitimate in *both*
   directions — the tree may have been edited after capture, or cleaned after it.
   Asserting either direction would be a flake, so the tests assert the shape of
   the claim and its internal consistency with the first-line tag, and stop
   there.
3. **`--version` says nothing about the *toolchain* or build profile.** A binary
   built from the right revision with a different rustc, or a debug binary
   mistaken for the release one, is outside this witness. Not a gap this parcel
   claimed, but adjacent enough to be worth knowing it is open.
4. **Nothing yet *consumes* the witness.** Aeon's build still invokes `sigil`
   without asking what it is. Turning the report into an enforced gate (build
   refuses, or warns loudly, when the assembler's revision does not match the
   tree it is assembling) is a separate parcel and belongs to the aeon side.

## Verification

Runner: plain `cargo test`. `crates/sigil-cli/tests/version_provenance.rs` is
auto-discovered — `sigil-cli/Cargo.toml` declares no explicit `[[test]]` targets
— so it runs under the repo bar `cargo test --release --workspace --no-fail-fast`
and under `scripts/nightly_source_gates.sh:209`, which invokes exactly that.
The test takes no `AEON_DIR` and needs no aeon tree.

### Suite result

Scope run: `AEON_DIR=/home/volence/sonic_hacks/.aeon-landing cargo test --release
-p sigil-cli --no-fail-fast` — the whole `sigil-cli` crate, which is where the
new file lives and the only crate this parcel touches.

```
138 test binaries | 601 passed | 0 failed | 3 ignored | exit 0
tests/version_provenance.rs: 9 passed, 0 failed
```

No `FAILED` line, no panic, no `error:` in the run. The `.aeon-landing` tree was
read only.

The full-workspace bar (`--workspace`, 3810/0/4 at master `cba0a0bc`) was not
re-run; nothing here is reachable from another crate — `sigil-cli` is a leaf
binary that no workspace crate depends on.

Nine tests, all driving the built binary via `CARGO_BIN_EXE_sigil`, with every
expectation derived rather than pinned: the semver from this package's own
`CARGO_PKG_VERSION`, the revision and branch from asking git about
`CARGO_MANIFEST_DIR` at test time. No SHA appears as a literal anywhere.

| Test | What it holds |
|---|---|
| `version_reports_the_head_of_the_tree_it_was_built_from` | **The incident test.** Ties the running binary's reported revision to this checkout's HEAD. |
| `version_reports_the_branch_this_tree_is_on` | Second independent read of the `HEAD`-file trigger; a branch switch landing on the same commit still moves HEAD. |
| `short_flag_prints_the_same_banner` | `-V` is not a truncated banner that lets a script skip the caveat. |
| `first_line_carries_the_crate_version_and_a_revision_tag` | The greppable identity line, semver derived from the package. |
| `no_banner_field_is_blank_or_a_bare_placeholder` | No field renders empty, `-`, `n/a`, `0`, `null`, `none`, or a dangling em-dash promising a reason it does not give. |
| `the_revision_tag_agrees_with_the_reported_tree_state` | The line-one tag and the `tree:` line cannot disagree; a dirty build cannot wear the bare short SHA. |
| `an_empty_porcelain_reads_as_clean_not_as_unknown` | An empty `git status --porcelain` is the *clean* answer, not a failed probe. |
| `the_banner_discloses_what_it_cannot_track` | The snapshot caveat, the direction it can be wrong, and the check command are all present. |
| `the_banner_names_the_rerun_triggers_backing_the_revision` | A reported revision must name `.git/HEAD` as tracked; `cargo tracks none` alongside a revision is refused. |

### Red-first evidence

Every check was made to fail on purpose before being trusted.

**(1) The incident itself, reproduced.** Rerun triggers stripped from `build.rs`,
binary built, then HEAD moved by an empty commit — exactly the "relinked without
re-capture" state:

```
thread 'version_reports_the_head_of_the_tree_it_was_built_from' panicked:
assertion `left == right` failed: the `sigil` binary reports revision
8af046eb2ab6ebef74a5c6a48ada37dfb0548b3c but this checkout's HEAD is
c33c3bae0f9dba194b27abe2617ee5fe200c6e8a. Either build.rs did not re-run when
HEAD moved (the rerun triggers are the fix), or HEAD moved while the suite was
running (re-run to distinguish).
  left: "8af046eb2ab6ebef74a5c6a48ada37dfb0548b3c"
 right: "c33c3bae0f9dba194b27abe2617ee5fe200c6e8a"

test result: FAILED. 7 passed; 2 failed
```

The second failure was `the_banner_names_the_rerun_triggers_backing_the_revision`
firing on `cargo tracks none` — the banner refusing to claim a freshness it had
lost. Triggers restored → green, at the new HEAD, without touching a source file.

**(2) A clean tree read as unknown.** The status probe routed through the
value-query helper, which rejects empty output so a blank revision can never be
baked in:

```
thread 'an_empty_porcelain_reads_as_clean_not_as_unknown' panicked:
the tree probe treated empty porcelain output as a failure; empty output IS the
clean answer
tree: unknown at capture — git --no-optional-locks status --porcelain=v1
      --untracked-files=normal produced no output

test result: FAILED. 8 passed; 1 failed
```

**(3) An unknown with no reason.** Provenance forced to the unknown path with an
empty reason string:

```
thread 'no_banner_field_is_blank_or_a_bare_placeholder' panicked:
field `revision` promises a reason and gives none: `unknown —`
```

## Two defects the red-first work caught

Both are the reason the red-first bar exists; neither would have been visible
from a green run.

1. **A clean tree rendered as `tree-unknown`.** `git status --porcelain` prints
   nothing for a clean tree, and the shared git helper treats empty output as a
   failed probe — deliberately, so an empty revision can never be baked in.
   Routing the status probe through it made the *healthiest possible state*
   render as unmeasurable. Loud-on-unmeasurable is a duty owed to states that
   genuinely cannot be measured; spending it on a non-problem is how a witness
   stops being read. Fixed by splitting the helper into a value query (empty is
   an error) and a query whose answer may legitimately be empty.
   Found by inspecting the first clean-tree run, not by a test — a test now
   covers it in both directions.

2. **Every unknown-path branch in the tests was dead code.** The rendered line is
   `unknown — <reason>`, so the guards comparing the parsed field to the literal
   `"unknown"` never matched, and the shape and disclosure assertions were
   silently skipping the exact case they exist to cover. Surfaced only because
   red-first (3) forced the unknown path and the wrong tests went red. Guards now
   match on the prefix, and re-forcing the unknown path confirms the shape and
   disclosure tests pass through it while the two git cross-checks correctly
   refuse to go green with no revision to check against.

## Commits

| SHA | What |
|---|---|
| `db498607` | `feat(cli): sigil --version reports the source revision it was built from` |
| `8af046eb` | `fix(cli): an empty porcelain is a clean tree, not a failed probe` |
| `527d08ff` | `fix(cli): reach the unknown-provenance branches, and give every unknown a reason` |

Files: `crates/sigil-cli/build.rs` (new), `crates/sigil-cli/src/main.rs`,
`crates/sigil-cli/tests/version_provenance.rs` (new).

No `Cargo.toml` change is needed — cargo auto-detects `build.rs` at the package
root. No build-dependency is added; the script shells out to `git` and uses only
`std`. `crates/sigil-cli/tests/crate_graph.rs` filters `build-`dependencies out
of its one-way-graph invariants, so nothing here is visible to that gate.

The aeon-paired lane was not touched: no change reaches
`crates/sigil-harness/golden/`, `crates/sigil-harness/src/pins.rs`, or
`crates/sigil-harness/repin.toml`. Nothing this parcel does can change an emitted
byte — the banner is a new subcommand path that returns before any assembly, and
the build script only sets env vars read by that path.
