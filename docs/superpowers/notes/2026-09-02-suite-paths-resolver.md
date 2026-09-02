# SUITE-PATHS-MIGRATION + D18-REFUSE-BARE-RUN — the resolver, and the refusal on top of it

Two stacked parcels, no ROM bytes, `golden/` `pins.rs` `repin.toml` `provenance.toml`
untouched:

* `parcel/suite-paths-resolver` — sigil's implementation of the suite-wide path contract,
  and the ~100 private copies routed through it.
* `parcel/d18-refuse-bare-run` — the hub's `d-18` ruling on top of it: a bare run stops
  instead of measuring against a tree nobody named.

## The authorities, read at a committed revision

| What | Where | Revision |
|---|---|---|
| the resolver contract | `empyrean` `contract/SUITE_PATHS.md` | `origin/main` = `82982b7ff3c057f347d538fcf61b7c62b18ee813` |
| the `d-18` ruling (R4) and the migration list (R7) | this repo, `docs/OVERSEER.md` §"d-18 IS ANSWERED" (2026-09-02) | in-tree |
| the delegation the ruling was taken under | `empyrean` `4e8e865b7c6e821cc23cb3683776aa71243cac0b` | cited by the OVERSEER entry |

The contract's precedence, the same in every resolver in the suite:

1. the explicit checkout variable `AEON_DIR`;
2. `EMPYREAN_SUITE_ROOT` joined with the repo's directory name;
3. derivation from the calling repo's own location — `git rev-parse --git-common-dir`,
   never `--show-toplevel`, which answers wrongly from a worktree;
4. refuse, naming what was looked for and where.

A variable that is **set but wrong** is a hard error at its own step, not a null that lets
the next step run.

## The reference-tree witness this work was done against

`AEON_DIR=/home/volence/sonic_hacks/.aeon-ref-resolver` (a detached aeon worktree at the
provenance tip `027ec1620dd977bf7b8ee47cbafe2b2197059092`, all four shapes built).
`repin --check` re-run from this worktree before anything was written, and the witness is
the **last line**: `pins.rs unchanged`. Above it, ten `declared allotment` burndown
warnings — `entity_window`, `children`, `objdefs`, `dust_spindash`, `player_climb`, each in
both the plain and the debug shape.

**The warning COUNT is not a bar and is not treated as one here.** A prediction of four was
in circulation and a second, independently provisioned tree at the same aeon revision
measured ten; neither number is a witness of anything. One observation offered for free,
since it costs no machine time: **`pins.rs unchanged` proves the placement is byte-identical
to the pinned one**, so two runs disagreeing about which regions warn cannot be disagreeing
about the aeon-side layout — the warning fires on a region whose declared `end` is the *next
placement* rather than its own last byte, and that predicate is evaluated against
`repin.toml`'s region declarations and `repin.rs`'s burndown logic, both of which live in the
**sigil** tree. A differing warning set at identical placement is therefore evidence about
which sigil revision produced the log, not about aeon determinism. Not chased further; not
this parcel.

---

# Parcel 1 — `parcel/suite-paths-resolver`

## The design call: a checkout and a reference tree are two questions

The contract resolves a **checkout**. Sigil's byte and port gates need a **reference tree**.
Step 3, run from this repo, derives `<suite root>/aeon` — the owner's live working
checkout, whose revision moves under a run. That is precisely the tree `d-18` refuses for
reference-dependent measurement, and `provenance_chain::aeon_dir_matches_the_provenance_tip`
already refuses it under the strict gate.

So `crates/sigil-harness/src/test_support.rs` now carries two functions and they stay two:

| Function | Answers | Accepts |
|---|---|---|
| `aeon_checkout() -> Result<ResolvedCheckout, String>` | the contract's question, steps 1→4 | any step |
| `aeon_dir() -> PathBuf` | sigil's — the tree a gate measures against | steps 1 and 2; step 3 announces (parcel 1) then refuses (parcel 2) |
| `unnamed_default_tree() -> Result<ResolvedCheckout, String>` | what a run resolves to when NOBODY names a tree | steps 2→4 |

`ResolvedCheckout` carries the path and a `PathStep` **enum** — not a string. The one
consumer that branches on it (`names_a_reference_tree()`) is taking a decision, and a
decision taken by matching prose changes when the prose is reworded.

`LIVE_TREE_FALLBACK` is gone. Its role — the value the `d-17` write guard refuses to touch
— is `unnamed_default_tree()`, the same precedence with step 1 skipped, so the guard and
the resolver cannot name different paths and neither is a literal. That is the contract's
own instruction: *"a guard that must refuse to touch the live tree compares against the
resolved default, not against a literal."*

### Set-but-wrong is exactly "not a directory", and no wider

Deliberate. A tree's **contents** are `reference_tree(rels)`'s question, asked per gate
against the paths that gate actually reads. Answering it in the resolver would replace
those precise per-path messages with a blunt one — and would refuse the empty stand-in
trees `reference_tree_named_write.rs` deliberately points `AEON_DIR` at, whose whole
purpose is to be a directory that exists and is empty.

Step 2 is stricter, and can afford to be: a value is a suite root only if it holds every
`SUITE_ROOT_MARKERS` entry (`aeon/` + `empyrean/`) — **the same marker set aeon's own
`tools/suite_paths.py` uses, and for the reason it gives.** Two resolvers answering the
same question must not answer it differently.

### Why the derivation runs from `CARGO_MANIFEST_DIR`

Compile-time, so the derivation is about the checkout the code was *built* from rather than
whatever a test process happens to have as its cwd. Cached in a `OnceLock` — the answer
cannot change within a process, and 125 test binaries would otherwise each pay a
subprocess per call.

## The two greps, reconciled

Two different questions, neither a superset of the other. Measured on `master` before the
change:

| Grep | Files | What it finds |
|---|---|---|
| `git grep -l 'sonic_hacks/aeon' -- '*.rs'` | **108** | the home literal, in code AND in prose |
| `git grep -l 'AEON_DIR' -- '*.rs'` | **136** | the variable name, wherever it appears |

* **In the literal list only (1 file):** `crates/sigil-isa/tests/encode_base_8bit.rs` — a
  module doc naming the tree as the ground truth for its golden vectors, with no variable
  anywhere. A grep for `AEON_DIR` alone would have left the literal standing.
* **In the `AEON_DIR` list only (29 files):** `crates/sigil-cli/src/main.rs`,
  `sigil-harness/src/{harness_root,native,provenance,rev_reachability,seam2}.rs`,
  `sigil-harness/src/bin/{derive_offcanon,emit_sound_blob,refreeze}.rs`, and 20 test files
  that name the variable in a doc comment or a usage line without ever spelling a default.
  A grep for the literal alone would have missed every one.

Site shapes actually found (code only, by `uniq -c` over the matching lines):

```
     60  std::env::var("AEON_DIR").unwrap_or_else(|_| "…/aeon".to_string()),
     31  std::env::var("AEON_DIR").unwrap_or_else(|_| "…/aeon".to_string());
     11  let aeon = std::env::var("AEON_DIR").unwrap_or_else(|_| "…/aeon".to_string());
      4  std::env::var("AEON_DIR").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("…/aeon"))
      4  PathBuf::from(std::env::var("AEON_DIR").unwrap_or_else(|_| "…/aeon".into()))
      1  pub const LIVE_TREE_FALLBACK
```

**After:** `git grep -c 'sonic_hacks/aeon' -- '*.rs'` is **0**. 92 files rewritten to call
`test_support::aeon_dir()`; the remaining literal sites were doc comments, rewritten to
present-tense contract fact (`` (`AEON_DIR`, or `EMPYREAN_SUITE_ROOT`) ``) with no
change-history narration.

### The `env::var("AEON_DIR")` reads that DELIBERATELY did not route

Nine remain, and each is right where it is:

* `test_support.rs:846` — the resolver itself.
* `seam2.rs:121` — the `d-17` write guard's naming check. It asks "was this tree NAMED",
  which is step 1 by definition; a resolver's later steps are exactly what it must not
  accept.
* `sigil-cli/src/main.rs:871` — publishes `--aeon` as `AEON_DIR` so the argv caller is not
  an exception to the write guard. Unchanged behaviour.
* `refreeze.rs` (×4), `derive_offcanon.rs`, `emit_sound_blob.rs` — tools that **write**
  committed artifacts. Each reads step 1 and refuses if it is unset, with no fallback and
  no derivation. That is stricter than the contract's precedence, on purpose: a refreeze
  that DERIVED its reference tree is the exact failure the whole reference-tree arc exists
  to close. Booked in the gap ledger rather than migrated.
* `reference_tree_named_write.rs` (×2) — a gate asserting what its own child's environment
  is; it is measuring the variable, not resolving through it.

## The classifier — a consumer this change moved, and a hole it opened

`scripts/nightly_source_gates.sh` derives BOTH halves of its read rule from
`test_support.rs`: `reference_env_var` extracts the variable name by matching the literal
`env::var("AEON…")`, and `accessor_closure` seeds on the **public** function containing it,
then closes over public functions of that file that call one already in the set. It refuses
to run on an unclassified aeon-reading file.

Two consequences, both handled:

**1. The seed must stay a public function in that file.** The first draft put the
precedence in a private `resolve_checkout(bool)` helper. Measured, not reasoned about:

```
UNMEASURABLE: no reference-tree accessor is derivable from
crates/sigil-harness/src/test_support.rs in … — the read rule has no pattern, so every
file would falsely look like it reads nothing
```

`crates/sigil-harness/tests/source_gate_classification.rs` goes red on it — two rows, both
naming the cause. So step 1 lives in its two public callers (`aeon_checkout` consults the
variable, `unnamed_default_tree` deliberately does not), and the comment above the resolver
says why in the code rather than only here.

**2. The closure got wider than its own answer.** Its seed has to be the function that
READS the variable, which is now `aeon_checkout` — a function that answers *which checkout*
and hands back a step for the caller to judge, not one that yields a tree to measure
against. Unnarrowed, the closure published `aeon_checkout aeon_dir aeon_dir_is_unnamed
reference_tree reference_tree_for_profile`, and the resolver's own precedence gate — which
never opens a file in any aeon tree — came back **UNCLASSIFIED**, which makes the whole
lane refuse to run.

The narrowing is derived from the source like everything else there: keep only closure
members whose signature returns a **path**. That is exactly the distinction — a function
handing back a `PathBuf` is handing back a tree to read; one handing back a verdict is not.
The published set is now `aeon_dir reference_tree reference_tree_for_profile`, and
`suite_paths_precedence` buckets `no-reference`, which is what it is.

**The hole this leaves is stated in the script**: a caller that takes `aeon_checkout()`'s
answer apart and joins onto its `.path` reaches the tree through a dropped member. One file
does that today and it is a `SOURCE_GATES` member, so it never reaches the question. What
keeps the hole from widening quietly is the new gate below — not the script, which cannot
see its own shortness.

Audit, before and after, same tree:

```
before  SOURCE_GATES=45 scanned=131 source=44 artifact=85 no-reference=2 unclassified=0
after   SOURCE_GATES=45 scanned=133 source=44 artifact=85 no-reference=4 unclassified=0
        accessors: aeon_dir reference_tree reference_tree_for_profile
```

`scanned` rose by two: `suite_paths_precedence` is new, and `encode_base_8bit` entered the
selector's population because its rewritten doc comment now names `AEON_DIR`. Both bucket
`no-reference`, correctly — neither opens anything in an aeon tree.

## One derivation, several readers

`crates/sigil-harness/src/reference_dependence.rs` lifts the reference-dependent population
walk out of `sigil-cli/tests/reference_dependence_is_named.rs` (commit `526fdd0e`) into the
harness, with `GUARDS` and `FLOOR` as declarations. Parcel 2's refusal reads the same
walk — the ruling asks the partial run to print a derived not-measured count, and a second
derivation would be a second thing to keep in step.

**A finding worth foregrounding.** Routing the private copies raised the derived population
from **40 test binaries to 125**. Those 85 files were reference-dependent all along; the
derivation could not see them because each spelled the environment read itself instead of
calling a guard. The gate that exists to say how much of the suite went unmeasured was
itself under-reporting by a factor of three, and nothing could have noticed — the
under-count was in the same direction as a green.

`reference_dependence_is_named` also now consults `aeon_checkout()` rather than
`aeon_dir()`. Its whole subject is the state where no tree was named, and `aeon_dir()` is
the function that ACTS on that state; asking it made the gate a consumer of the behaviour
it reports on — and in parcel 2 it would have been stopped by the refusal it exists to
describe.

## The gates, and the red-first proof of each

| Gate | Runner | Expectation derived from |
|---|---|---|
| `the_resolver_follows_the_contract_precedence` | `cargo test -p sigil-harness --test suite_paths_precedence` (workspace suite; classified `no-reference` for the nightly lane) | the contract's precedence; step-3 expectation from an INDEPENDENT marker walk, not the resolver's own git call; fixtures are directories the test creates |
| `the_derived_accessor_set_is_the_declared_guard_set` | `cargo test -p sigil-harness --test source_gate_classification` (workspace suite) | `sigil_harness::reference_dependence::GUARDS`, the same declaration the population walk uses |

Every case below was made to fail on purpose and the failing assertion's own wording is
quoted. Each poison was reverted and re-run green.

**P1 — step 1 set-but-wrong falls through to step 2** (`return Err(…)` replaced with
`resolve_from_step_2(…)`):

> `expected a REFUSAL and the resolver ANSWERED: RESULT ok step=2 path=/tmp/sigil-suite-paths-root-a-…/aeon. A variable that is set but wrong must stop at its own step; an answer here means the resolver went on to a later step and returned a tree nobody asked for, while reporting success.`

**P2 — the derivation uses `--show-toplevel`** instead of `--git-common-dir`. The refusal
is the worktree lie in one line, from a real agent worktree:

> `expected a resolved answer, got: RESULT err no aeon checkout could be resolved. AEON_DIR is unset; EMPYREAN_SUITE_ROOT is unset; derivation from this checkout's own location: /home/volence/sonic_hacks/sigil/.claude/worktrees is this repository's parent but holds no aeon/ + empyrean/ — it is not a suite root.`

**P3 — step 2 set-but-wrong falls through to the derivation.** Note what it fell through
*to*:

> `expected a REFUSAL and the resolver ANSWERED: RESULT ok step=3 path=/home/volence/sonic_hacks/aeon.`

**P4 — `unnamed_default_tree` consults step 1** (`if let Ok(c) = aeon_checkout() { return Ok(c); }`):

> `assertion 'left == right' failed: unnamed_default_tree answers the question 'what does a run resolve to when nobody names a tree' — a set AEON_DIR must not answer it / left: 1 / right: 3`

**P5 — the resolver's env read moved into a private helper** (the classifier decision):
`source_gate_classification.rs` goes red on two rows —

> `the source-gate lane cannot classify this tree, so it will refuse to run and produce no coverage until this is fixed. … UNMEASURABLE: no reference-tree accessor is derivable from crates/sigil-harness/src/test_support.rs`

**P6 — the accessor closure drops one accessor** (a `name != "reference_tree_for_profile"`
clause in the narrowing):

> `the source-gate lane derives its reference-tree accessors from test_support.rs by closure, and the harness declares the same set as GUARDS. They disagree. Whichever side is short, some test file that reads the reference tree is about to be classified as reading nothing — which the lane reports as green coverage. / left: ["aeon_dir", "reference_tree"] / right: ["aeon_dir", "reference_tree", "reference_tree_for_profile"]`

## What a run says now

Named (`AEON_DIR=/home/volence/sonic_hacks/.aeon-ref-resolver`), one line per process on
stderr:

```
reference-tree: /home/volence/sonic_hacks/.aeon-ref-resolver (SUITE_PATHS step 1 — named by AEON_DIR)
```

Bare (`AEON_DIR`, `EMPYREAN_SUITE_ROOT` and `SIGIL_STRICT_GATE` all unset),
`cargo test -p sigil-cli --test header_port -- --nocapture`, verbatim:

```
running 2 tests
reference-tree: /home/volence/sonic_hacks/aeon (SUITE_PATHS step 3 — DERIVED from this checkout's own location — nobody named it) — nobody NAMED a reference tree, so every reference-dependent result in this run was measured against a checkout this run derived. That tree is outside this repository and its revision can change under a run without notice, so a pass or a failure here is attributable to whatever it happened to contain. Set AEON_DIR to a provisioned tree (scripts/provision-aeon-ref.sh) to make the result name its own subject.
test demo_header_matches_reference ... ok
test sonic4_header_matches_reference ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Both lines carry neither `skip:` nor `skipping`, the two spellings
`scripts/landing-run.sh:369` and `refreeze.rs:533` count out of a run's log. That property
was already deliberate for the old notice and is preserved; the step-1 announcement is new
and holds it too.

That transcript is also the case for parcel 2 in four lines: **exit 0, two passes, and the
subject of the measurement was whatever the owner's working tree happened to contain.**

## One incidental dependency edge

`sigil-frontend-emp` gains a dev-dependency on `sigil-harness` for the two corpus censuses
in `tests/cfg_blind_spots.rs`, which reach the reference tree. The harness depends on that
crate, so the edge closes a cycle **through a dev-dependency**, which cargo resolves — the
harness's own build does not see it. Verified by a clean `cargo test --workspace --no-run`.
