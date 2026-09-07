# 2026-09-07: suite-root readers under a declared partial run (`CI-SUITE-ROOT-READERS`)

Branch `parcel/ci-suite-root-readers` off master `ee37d65b`. Commits: `03f186f8` (the
harness change), `94899bc2` (refusal wording, docs), then this note. Builds went to
`CARGO_TARGET_DIR=/home/volence/sonic_hacks/.target-suiteroot/{worktree,clone}`; no
emulator, no aeon build, no `SIGIL_BUILD`/`SIGIL_EMIT`. Byte-neutral by construction: the
diff is the test harness, two test files, and prose.

## The ruling, as implemented

Three rows read the SUITE ROOT (the directory holding `aeon/` + `empyrean/` beside the
checkout) rather than the reference tree, and on a runner with no such siblings they FAILED
under `SIGIL_ALLOW_PARTIAL=1`, because d-18's partial opt-in named reference-TREE readers.
Now, when a reader's own derivation finds no suite root, it closes with
`test_support::suite_root_absent(what, why)`:

| run | answer |
|---|---|
| `SIGIL_STRICT_GATE=1` | FAILS by name (`assert!(!strict_gate(), ..)`, the census-classified idiom) |
| `EMPYREAN_SUITE_ROOT` set, root still not established | FAILS: set but wrong is never a skip |
| `SIGIL_ALLOW_PARTIAL=1` | one banner per process naming the class, its derived size and the binaries, then a `skip:` line per row; the test returns green over the rows it did not measure and nothing else |
| bare (neither variable) | stops with `UNMEASURABLE`, naming the opt-in |

The present-root path of all three tests is untouched. The rows that read no suite root in
the same tests still run in the partial run: direction 2 of `reference_tree_named_write`
(`AEON_DIR` set), steps 1, 2 and 4 and the set-but-wrong arms of
`the_resolver_follows_the_contract_precedence`, and the bed row of
`the_step_3_derivation_is_proven_from_a_linked_worktree`. Only the rows whose expectation
IS the suite root are left unmeasured, and each says which.

The banner is a SECOND line for a second class (the brief's explicit alternative), on its
own once-per-process latch. The two classes are independent: a tree named by `AEON_DIR`
beside no suite root owes only this line, and the ordinary local partial run (no tree
named, suite root present) owes only the reference line. Its count comes from
`reference_dependence::suite_root_reading_binaries` (guard `SUITE_ROOT_GUARDS`), the same
source walk as the reference class; an empty answer prints COULD NOT BE ESTABLISHED, and a
unit test holds the banner to the derived count, the names, the class, the markers, the
flag, and no skip spelling.

## Red-first, in a CI-shaped clone

The clone: `git clone` of this worktree at master `ee37d65b` into
`/home/volence/.ci-shaped-clone/ci/sigil`. No ancestor of it holds `aeon/` + `empyrean/`
(checked at `/home/volence`, `/home`, `/`). Every run unset `AEON_DIR`,
`EMPYREAN_SUITE_ROOT`, `EMPYREAN_DIR`, `SIGIL_DIR`, `SIGIL_STRICT_GATE`,
`SIGIL_STRICT_WITNESS` and `SIGIL_ALLOW_PARTIAL`, then set only the one the arm names, and
stamped the log with the tree, HEAD and environment. Baseline and after are both committed
states reached by `git checkout`, so the mutation is the commit itself and the restore is
the checkout back; the tree was clean at every run (`git status --short` empty).

### Baseline, master `ee37d65b`, `SIGIL_ALLOW_PARTIAL=1`: cargo exit 101

```
# tree=/home/volence/.ci-shaped-clone/ci/sigil HEAD=ee37d65b0bf58340ad0545553b5299a6500d3fa3 mode=partial
# env: SIGIL_ALLOW_PARTIAL=1
thread 'a_write_into_the_reference_tree_refuses_unless_aeon_dir_named_it' panicked at crates/sigil-harness/tests/reference_tree_named_write.rs:336:5:
the `unset` child failed (exit status: 101). Its assertions are the property; read them below.
  (child) panicked at crates/sigil-harness/tests/reference_tree_named_write.rs:133:13:
  UNMEASURABLE: with AEON_DIR removed the resolver names no default tree (no aeon checkout could be resolved. ...
  /home/volence/.ci-shaped-clone/ci is this repository's parent but holds no aeon/ + empyrean/, it is not a suite root. ...
test a_write_into_the_reference_tree_refuses_unless_aeon_dir_named_it ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
thread 'the_step_3_derivation_is_proven_from_a_linked_worktree' panicked at crates/sigil-harness/tests/suite_paths_precedence.rs:523:24:
step 3 cannot derive from this crate's own location: /home/volence/.ci-shaped-clone/ci is this repository's parent but holds no aeon/ + empyrean/, it is not a suite root
test the_step_3_derivation_is_proven_from_a_linked_worktree ... FAILED
test step_3_survives_every_shape_git_rev_parse_can_answer ... ok
thread 'the_resolver_follows_the_contract_precedence' panicked at crates/sigil-harness/tests/suite_paths_precedence.rs:277:38:
UNMEASURABLE: no ancestor of this crate holds the suite markers, so the step-3 expectation cannot be established independently of the resolver
test the_resolver_follows_the_contract_precedence ... FAILED
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

### After, `94899bc2`, `SIGIL_ALLOW_PARTIAL=1`: cargo exit 0, three `skip:` lines, one banner per binary

```
# tree=/home/volence/.ci-shaped-clone/ci/sigil HEAD=94899bc2670dae82bf36d897c2ac1ea078341740 mode=partial
# env: SIGIL_ALLOW_PARTIAL=1
PARTIAL RUN (SIGIL_ALLOW_PARTIAL is set). No suite root holds aeon/ + empyrean/ beside this checkout, so 2 test binaries read the SUITE ROOT (reference_tree_named_write, suite_paths_precedence) and the rows in them that compare against it are left UNMEASURED, each announced in the skip form. A green result from this run does NOT mean those rows passed, it means they were not run. Run from a checkout inside a suite root to measure them.
skip: direction 1 (AEON_DIR removed) of a_write_into_the_reference_tree_refuses_unless_aeon_dir_named_it: no suite root holds aeon/ + empyrean/ beside this checkout (no aeon checkout could be resolved. ... it is not a suite root. ...); left unmeasured under SIGIL_ALLOW_PARTIAL
test a_write_into_the_reference_tree_refuses_unless_aeon_dir_named_it ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test step_3_survives_every_shape_git_rev_parse_can_answer ... ok
PARTIAL RUN (SIGIL_ALLOW_PARTIAL is set). No suite root holds aeon/ + empyrean/ beside this checkout, so 2 test binaries read the SUITE ROOT (reference_tree_named_write, suite_paths_precedence) and ... (the same line, once for this binary)
skip: the production-anchor half of the_step_3_derivation_is_proven_from_a_linked_worktree: no suite root holds aeon/ + empyrean/ beside this checkout (no ancestor of this crate holds the suite markers, and the derivation from its own location agrees: ... it is not a suite root); left unmeasured under SIGIL_ALLOW_PARTIAL
test the_step_3_derivation_is_proven_from_a_linked_worktree ... ok
skip: the step-3 row and the unnamed-default row of the_resolver_follows_the_contract_precedence: no suite root holds aeon/ + empyrean/ beside this checkout (no ancestor of this crate holds the suite markers, so the expectation those rows compare the resolver against cannot be established independently of it); left unmeasured under SIGIL_ALLOW_PARTIAL
test the_resolver_follows_the_contract_precedence ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Every `skip:` line starts at column 0 on stderr, which is what `.github/workflows/ci.yml`'s
reporter greps (`^skip: `), so they join the count CI already prints.

### After, `94899bc2`, `SIGIL_STRICT_GATE=1`: cargo exit 101, three FAILED by name

The mutation the strict arm runs, quoted from the clone's disk at the panic line
(`crates/sigil-harness/src/test_support.rs:1425-1432`, HEAD `94899bc2`):

```rust
pub fn suite_root_absent(what: &str, why: &str) {
    let markers = SUITE_ROOT_MARKERS.map(|m| format!("{m}/")).join(" + ");
    assert!(
        !strict_gate(),
        "SIGIL_STRICT_GATE set but no suite root for {what}. The SUITE ROOT is the directory \
         holding {markers} beside this checkout, and none was found ({why}). A strict run \
         measures this from a checkout inside a suite root or fails here by name."
    );
```

```
# tree=/home/volence/.ci-shaped-clone/ci/sigil HEAD=94899bc2670dae82bf36d897c2ac1ea078341740 mode=strict
# env: SIGIL_STRICT_GATE=1
thread 'a_write_into_the_reference_tree_refuses_unless_aeon_dir_named_it' panicked at crates/sigil-harness/src/test_support.rs:1427:5:
SIGIL_STRICT_GATE set but no suite root for direction 1 (AEON_DIR removed) of a_write_into_the_reference_tree_refuses_unless_aeon_dir_named_it. The SUITE ROOT is the directory holding aeon/ + empyrean/ beside this checkout, and none was found (...)
test a_write_into_the_reference_tree_refuses_unless_aeon_dir_named_it ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
thread 'the_step_3_derivation_is_proven_from_a_linked_worktree' panicked at crates/sigil-harness/src/test_support.rs:1427:5:
SIGIL_STRICT_GATE set but no suite root for the production-anchor half of the_step_3_derivation_is_proven_from_a_linked_worktree. ...
test the_step_3_derivation_is_proven_from_a_linked_worktree ... FAILED
test step_3_survives_every_shape_git_rev_parse_can_answer ... ok
thread 'the_resolver_follows_the_contract_precedence' panicked at crates/sigil-harness/src/test_support.rs:1427:5:
SIGIL_STRICT_GATE set but no suite root for the step-3 row and the unnamed-default row of the_resolver_follows_the_contract_precedence. ...
test the_resolver_follows_the_contract_precedence ... FAILED
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

### After, `94899bc2`, bare: cargo exit 101, three FAILED with `UNMEASURABLE` at `test_support.rs:1442`

```
UNMEASURABLE: no suite root for direction 1 (AEON_DIR removed) of a_write_into_the_reference_tree_refuses_unless_aeon_dir_named_it. The SUITE ROOT is the directory holding aeon/ + empyrean/ beside this checkout, and none was found (...)
test a_write_into_the_reference_tree_refuses_unless_aeon_dir_named_it ... FAILED
UNMEASURABLE: no suite root for the production-anchor half of the_step_3_derivation_is_proven_from_a_linked_worktree. ...
test the_step_3_derivation_is_proven_from_a_linked_worktree ... FAILED
UNMEASURABLE: no suite root for the step-3 row and the unnamed-default row of the_resolver_follows_the_contract_precedence. ...
test the_resolver_follows_the_contract_precedence ... FAILED
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

The same run at `03f186f8` (before the wording commit) gave the same three outcomes with
the earlier sentence shape; `94899bc2` changed only the words.

## Present root, in this worktree (`/home/volence/sonic_hacks/sigil/.claude/worktrees/...`)

The two binaries, `94899bc2`, all suite variables unset except the arm's own:

| arm | result |
|---|---|
| `SIGIL_ALLOW_PARTIAL=1` | `reference_tree_named_write` 1 passed / 0 failed; `suite_paths_precedence` 3 passed / 0 failed; cargo exit 0; no `skip:` line, no banner |
| `SIGIL_STRICT_GATE=1` | 1/0 and 3/0, cargo exit 0 |
| bare | 1/0 and 3/0, cargo exit 0 |

Positive witness that the present-root path ran the rows this parcel guards, from the
partial log: `production anchor .../crates/sigil-harness -> git --git-common-dir answers
/home/volence/sonic_hacks/sigil/.git (absolute, a linked worktree)` and `ambient anchor:
compiled inside a linked worktree; the deployed derivation reached the checkout's suite
root`, which are printed only past the `(Ok, Some)` arm.

Harness lib: `cargo test -p sigil-harness --lib` under `SIGIL_ALLOW_PARTIAL=1` is
205 passed / 0 failed, including `the_partial_run_suite_root_banner_names_the_class_and_its_derived_size`
and `the_partial_run_banner_carries_a_derived_size`; the reference banner printed alone
there (no tree named) and the suite-root banner did not (root present), which is the
two-latch behaviour. Run BARE, the lib fails one row, `sec_field_equ_names_match_the_harvest`,
at the d-18 bare-run refusal (`test_support.rs:1214`, a reference-dependent row with no tree
named); that is d-18 working and predates this parcel.

## Lints, and that they SEE the new sites

| lint | master `ee37d65b` (clone) | `94899bc2` (worktree) |
|---|---|---|
| `skip_marker_lint` census | 170 announcement sites, 34 lexical-only | 171 sites, 35 lexical-only; 6/0 |
| `strict_census_lint` census | 114 reference-refusal paths excluded | 115 excluded, 29 declared sites unchanged; 5/0 |
| `source_gate_classification` | | 3/0 (the new helper is not path-yielding, so `GUARDS` is unchanged and the closure agrees) |
| `sigil-cli reference_dependence_is_named` | | 1/0 |
| `cargo clippy --release -p sigil-harness --all-targets -- -D warnings` | | clean |

The +1 in each census is the `eprintln!("skip: ...")` and the `assert!(!strict_gate(), ..)`
in `suite_root_absent`. Had the strict idiom been unclassifiable, the strict census would
have refused outright rather than counted it.

## What is left open

- The three tests skip at the granularity of the rows that read the suite root; a reader
  of CI's per-line skip report sees three lines, one per row group, not one per binary.
  That is by design here and is stated in the banner.
- `docs/OVERSEER-REFERENCE.md`'s d-18 block gained a paragraph; the ledger row is closed on
  the branch, and the controller's merge SHA is not in it.
- `/home/volence/sonic_hacks/.target-suiteroot/worktree` (debug and release artifacts for
  this branch) is left for the landing gate; the clone and its target subdirectory were
  deleted.

## What in the brief was wrong

- **The CI-shaped clone cannot live under `/home/volence/sonic_hacks/`** (the brief's
  example path). `walked_suite_root()` climbs every ancestor to `/` and the real suite root
  is `/home/volence/sonic_hacks` itself, so two of the three rows would have measured
  against it and the baseline would not have been red. The clone went to
  `/home/volence/.ci-shaped-clone/`, whose ancestors were checked for the markers.
- The line numbers in the brief were approximate on purpose and were right: the baseline
  panics were at `reference_tree_named_write.rs:133` (in the child; the parent surfaced it
  at `:336`), `suite_paths_precedence.rs:277` and `:523`.
- Nothing had to weaken a present-root assertion, so the BLOCKED clause was not reached.
