# 2026-09-07: CI's default target directory, and the one row that could not measure from it

Branch `parcel/ci-default-target`, off `9670325a` (the SHA GitHub run 34131201403 ran
on; the brief named `82838687`, which is an ancestor). Byte-neutral: one test, one
helper, the workflow, `.gitignore`. No emulator, no aeon tree, no SIGIL_BUILD.

## The finding, reproduced

`crates/sigil-harness/tests/shared_target_defaults.rs`,
`refreeze_tells_its_children_which_build_directory_to_use`. The first assertion (the
directory `refreeze` hands its child equals the directory `refreeze` was built into) is
the claim about `refreeze`. The second, `assert_ne!(got, sigil_root().join("target"))`,
is a claim about the environment: `got` IS the binary's own target directory (the first
assertion says so), so the second holds exactly when the build that produced the test
binary named a `CARGO_TARGET_DIR`. Every local landing run does; GitHub's runner did not.

Red, built into this worktree's own default `target/` with `CARGO_TARGET_DIR` unset (the
worktree's `target/`, not the main checkout's; deleted afterwards, 163M):

```
thread 'refreeze_tells_its_children_which_build_directory_to_use' panicked at crates/sigil-harness/tests/shared_target_defaults.rs:375:5:
assertion `left != right` failed: the shared checkout's target/ is the directory this whole file is about
  left: "/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a0200b38410f09053/target"
 right: "/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a0200b38410f09053/target"
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 12 filtered out
```

Green, same commit, `CARGO_TARGET_DIR=/home/volence/sonic_hacks/.target-cidefault`:

```
Running tests/shared_target_defaults.rs (/home/volence/sonic_hacks/.target-cidefault/release/deps/shared_target_defaults-cc7e3acbfb358d26)
test refreeze_tells_its_children_which_build_directory_to_use ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out
```

## The test change

The first assertion always runs. Then the test compares the binary's own target
directory with `sigil_root().join("target")`; when they are equal the row is
unmeasurable in this environment and it closes through a new
`test_support::built_into_default_target(what, target)`, a sibling of
`suite_root_absent` with d-18's three answers (`docs/OVERSEER-REFERENCE.md`, widened
to the suite-root class on 2026-09-07; this is the same shape for a third class). When
they differ, the `assert_ne!` runs unchanged.

Measured in the default-target build (commit `e782526e`):

Bare run:

```
panicked at crates/sigil-harness/src/test_support.rs:1487:9:
UNMEASURABLE: the shared-target row of refreeze_tells_its_children_which_build_directory_to_use: this test binary was built into the checkout's DEFAULT target/ (/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a0200b38410f09053/target), the directory the row holds refreeze's answer apart from, so the row cannot be measured from here. Build with CARGO_TARGET_DIR naming a directory outside the checkout's target/, or declare a partial run with SIGIL_ALLOW_PARTIAL=1, in which case this is left unmeasured and the run says so.
test result: FAILED. 0 passed; 1 failed
```

`SIGIL_STRICT_GATE=1`:

```
panicked at crates/sigil-harness/src/test_support.rs:1478:5:
SIGIL_STRICT_GATE set but this test binary was built into the checkout's DEFAULT target/ (/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a0200b38410f09053/target), so the shared-target row of refreeze_tells_its_children_which_build_directory_to_use cannot be measured. A strict run builds into a named CARGO_TARGET_DIR outside the checkout's target/ or fails here by name.
test result: FAILED. 0 passed; 1 failed
```

`SIGIL_ALLOW_PARTIAL=1` (with `--nocapture`):

```
skip: the shared-target row of refreeze_tells_its_children_which_build_directory_to_use: this test binary was built into the checkout's DEFAULT target/ (/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a0200b38410f09053/target), so the row cannot be measured here; left unmeasured under SIGIL_ALLOW_PARTIAL
test refreeze_tells_its_children_which_build_directory_to_use ... ok
test result: ok. 1 passed; 0 failed
```

No once-per-process banner for this class: the class is one row in one binary and the
skip line carries its own size. The strict census classifies the helper's
`assert!(!strict_gate(), ...)` as a missing-reference path, exactly as
`suite_root_absent`'s is, so no detector-A expectation is added for a site a healthy
strict run never reaches. The nightly script's accessor closure does not pick the helper
up (it reads no `AEON*` variable and calls no accessor); `--audit` of this tree:
`SOURCE_GATES=47 scanned=142 source=47 artifact=88 no-reference=7 unclassified=0`.

## The workflow diff, line by line

```
   test:
     runs-on: ubuntu-latest
+    # (comment block: why, and why dot-prefixed)
+    env:
+      CARGO_TARGET_DIR: ${{ github.workspace }}/.target-ci
     steps:
```

* `env:` at JOB level, not on the `test` step: the `test`, `clippy` and `crate-graph
  guard` steps then share one build directory, so clippy and the guard reuse the test
  step's artifacts instead of rebuilding into a second directory.
* `${{ github.workspace }}/.target-ci`: a named directory under the workspace, so the
  test binary is not built into the checkout's default `target/` and the row measures.
* `.target-ci`, NOT the brief's example `target-ci`: `drift_nightly_harness`'s
  `no_landing_path_invokes_the_drift_job` walks the whole workspace skipping only
  `target`, `.git` and dot-prefixed names, so a plain `target-ci` inside the checkout
  would be walked file by file. Dot-prefixed also matches the house's other named
  target directories (`.target-land`, `.target-ref`, `.cargo-target`).
* There is NO cache step in this workflow and nothing else in it names `target/`, so
  there was no cache key or path to update. The brief's conditional ("if there is a
  cache step") did not fire.
* `.gitignore` gains `/.target-ci/`, as every other named target directory has, so a
  local run of CI's shape leaves no untracked artifact.

GitHub Actions cannot be run from here; the workflow edit is unexercised until the
branch is pushed. Its intended effect is measured locally by the named-target run above,
which is the same shape the job now has.

## Red-first for the changed gates (mutation applied on disk, restored from the commit)

A. The lint must SEE the new announcement. Mutation, quoted from disk after `sed`:

```
1497:        "skipping {what}: this test binary was built into the checkout's DEFAULT target/ ({}), \
```

`cargo test --release -p sigil-harness --test skip_marker_lint`:

```
1 announced early return(s) do not start with the canonical marker "skip: ".
  .../crates/sigil-harness/src/test_support.rs:1496  [skip vocabulary]
      "skipping {what}: this test binary was built into the checkout's DEFAULT target/ ({}), so the row cannot be measured here; left unmeasured under {ALLOW_PARTIAL_VAR}"
test result: FAILED. 5 passed; 1 failed
```

Restored with `git checkout -- crates/sigil-harness/src/test_support.rs`; `git diff | wc -l` = 0.

B. The always-running assertion still gates `refreeze`. Mutation in
`crates/sigil-harness/src/bin/refreeze.rs::child_target_dir`, quoted from disk:

```
372:    Ok(PathBuf::from("/nonexistent/mutated-refreeze-handed-another-directory"))
```

Under the named target directory:

```
panicked at crates/sigil-harness/tests/shared_target_defaults.rs:378:5:
assertion `left == right` failed: the directory handed to the child must be the one this refreeze was built into
  left: "/nonexistent/mutated-refreeze-handed-another-directory"
 right: "/home/volence/sonic_hacks/.target-cidefault"
test result: FAILED. 0 passed; 1 failed
```

Restored with `git checkout -- crates/sigil-harness/src/bin/refreeze.rs`; `git diff | wc -l` = 0.

C. The three arms above ARE the red-first for the new helper: the default-target build
is the environment the strict arm must go red in, and it did, by name, at
`test_support.rs:1478`; the partial arm printed the skip line there.

## Verification (all `CARGO_TARGET_DIR=/home/volence/sonic_hacks/.target-cidefault`, release)

* `cargo test -p sigil-harness --test shared_target_defaults --test skip_marker_lint --test strict_census_lint`: 13/0, 6/0, 5/0.
* `cargo clippy --release -p sigil-harness --all-targets -- -D warnings`: exit 0.
* `SIGIL_ALLOW_PARTIAL=1 cargo test -p sigil-harness --lib --test source_gate_classification --test shared_target_defaults --test skip_marker_lint --test strict_census_lint --no-fail-fast`: 205/0, 13/0, 6/0, 3/0, 5/0.
* `cargo test -p sigil-cli --test reference_dependence_is_named --test drift_nightly_harness`: 1/0, 7/0.
* A BARE `--lib` run has one red, `test_support::tests::sec_field_equ_names_match_the_harvest`, panicking at `test_support.rs:1214` with `NO REFERENCE TREE IS NAMED ... STOPS`: that is the d-18 bare-run refusal of a reference-reading unit test, present before this parcel and unrelated to it; the same run under `SIGIL_ALLOW_PARTIAL=1` is 205/0.
* The main checkout's `target/release/sigil` was never touched (mtime 08:36, before this session). The worktree's own `target/` was used for the default-target reproduction only and is deleted.

## Left open

* `refreeze --attest` (`src/bin/refreeze.rs:846`) still falls back to `<root>/target/attest`
  for its witness and log files when `CARGO_TARGET_DIR` is unset. That is a scratch
  location, not a build, and `no_script_supplies_a_default_for_cargo_target_dir` scans
  shell scripts only, so it is outside this parcel; noted because it is the last
  `root.join("target")` in the harness.
* Pre-existing em dashes in the files touched are left as they were; only the new text
  is dash-free.
* The workflow change is unexercised until pushed.
