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
| `the_step_3_derivation_is_proven_from_a_linked_worktree` | same binary | the contract's 2026-09-02 worktree clause; the bed is a git repo the test builds, so the assertion does not depend on where `cargo test` was invoked from — see the amendment section at the end |
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

---

# Parcel 2 — `parcel/d18-refuse-bare-run`, stacked on parcel 1

## The ruling, and the reason it carries

`d-18` was answered **`refuse`** by the hub on 2026-09-02 under the owner's widened
delegation (`docs/OVERSEER.md` R4; the delegation banked at empyrean `4e8e865b`), against
this card's own `say_only` recommendation. The hub's reason is the better one, and every
message this parcel prints carries it:

> a run that prints how much it skipped **still exits 0**, and a silent green is the class
> never dropped, because a green is trusted the moment it is in the run.

Parcel 1's bare transcript is that argument in four lines — two passes, exit 0, and the
subject of the measurement was whatever the owner's working tree happened to contain.

## Where the refusal lives, and why there

In `aeon_dir()`'s step-3/step-4 path — the same call sites parcel 1's announcement already
covered — and not in a gate. One gate refusing would leave every other reference-dependent
row measuring against the derived tree, which is the state being refused; the resolver is the
only place that sees all of them.

| Environment | Behaviour |
|---|---|
| `AEON_DIR` or `EMPYREAN_SUITE_ROOT` names a tree | runs; one announcement line naming path and step |
| nothing named | **panics**, naming both variables, the derived path it DECLINED and why, and the opt-in |
| `SIGIL_ALLOW_PARTIAL=1`, nothing named | reference rows skip against `NO_REFERENCE_TREE`; the run prints the derived not-measured size |
| `SIGIL_ALLOW_PARTIAL=1` **and** `SIGIL_STRICT_GATE=1`, nothing named | refused by name — the two flags describe opposite runs |

Four decisions inside that are worth stating rather than leaving to be read off the diff.

**The partial run answers with an absent path, not the derived one.** `NO_REFERENCE_TREE` is
`/nonexistent/SIGIL_ALLOW_PARTIAL-no-reference-tree-was-named`. Handing back the derived live
checkout would make the "partial" run quietly measure against it — the thing being refused,
wearing the opt-in as cover. Absent *and* self-describing means the reason travels **with**
each skip —
`skip: reference ROM not at /nonexistent/SIGIL_ALLOW_PARTIAL-no-reference-tree-was-named/s4.bin`
— rather than living in a banner a reader scrolled past six hundred lines earlier.

**Strict and partial together are refused rather than ordered.** Strict is the run that may
not skip a gate; it cannot also be the partial one. Measured, not assumed: with the combined
check removed the child still fails, because `reference_tree` reaches the absent path and the
pre-existing strict assertion refuses it — but the message a reader gets is
`SIGIL_STRICT_GATE set but reference missing: /nonexistent/…`, which names a symptom and
leaves the contradiction to be inferred. See the Q4 note below; that is why the gate pins
*which* refusal fired.

**`SIGIL_STRICT_GATE` is read directly here, not through `strict_gate()`.** That accessor
RECORDS every reached consultation into the strict witness, and `strict_census` diffs that
population against the one it derives from the test tree; a consultation from inside the
resolver is not a strict-gated test body and would enter the census as a site with no
counterpart.

**The refusal is not countable as a skip.** It carries neither `skip:` nor `skipping`, the
two spellings `scripts/landing-run.sh:369` and `refreeze.rs:533` count out of a run's log. A
stop that registered as a skip would be reported by the very run it stopped. It surfaces as
`test … FAILED`, which both readers already count as a failure.

**The not-measured count is derived, never typed.** `partial_run_banner` reads
`reference_dependence::reference_dependent_binaries` — the same walk
`reference_dependence_is_named` reports with. A derivation that came back below its own floor
prints "COULD NOT BE ESTABLISHED … unknown rather than small" rather than a number it cannot
stand behind.

## Interactions checked

* **`reference_dependence_is_named`** — the gate that names the population. It consults
  `aeon_checkout()` (parcel 1), not `aeon_dir()`, so the refusal does not stop the one gate
  whose job is to explain the situation. Had it kept calling `aeon_dir()`, parcel 2 would
  have silenced it.
* **`strict_gate()`** — a missing reference was already a failure there. The refusal now
  fires earlier and names the cause instead of the symptom; a strict run always sets
  `AEON_DIR`, so this path is not reached in a landing.
* **`scripts/landing-run.sh:369`'s skip counting** — unchanged. Partial-run skips are real
  skips and count; the refusal is a failure and does not.
* **`repin` and `cycle_fraction`** (routed in parcel 1) — a bare `repin` now refuses instead
  of silently reading the live checkout. That is the correct outcome for a tool that
  regenerates `pins.rs`, and every documented invocation
  (`scripts/provision-aeon-ref.sh:179` and the proof command it prints) already names
  `AEON_DIR`.
* **`sigil build --aeon .`** — `main.rs:871` publishes the argument as `AEON_DIR` before any
  build work, so aeon's own `build.sh` is unaffected.

## The gate, and the red-first proof of each direction

`crates/sigil-harness/tests/bare_run_refuses.rs`, run by
`cargo test -p sigil-harness --test bare_run_refuses` and by the workspace suite. Three
directions in subprocesses of the file itself — which is reference-dependent, so the subject
is the real path and not a mock of it. The parent refuses to believe a child that produced no
libtest result line, because a child that never started looks exactly like one that ran and
stayed quiet.

**The third direction is not a formality.** Without a "named tree runs normally" arm, a
refusal that fired unconditionally — from a resolver broken in any way at all — would satisfy
the other two, and this file would report `d-18` implemented when what it had measured is
that nothing works.

| Poison | Failing assertion, verbatim |
|---|---|
| **Q1 — say-only**: the rejected alternative, announce then return the derived tree | `a bare run with no reference tree named PASSED. That is the whole defect d-18 closed: the run measured nothing it could attribute and reported success.` — with the child's own line `CHILD ran against /home/volence/sonic_hacks/aeon` underneath it |
| **Q2 — the partial run answers with the derived checkout** instead of the absent path | `the partial run must leave the reference-dependent row UNMEASURED. It reported otherwise, which means it found a tree — and a partial run that quietly measures against the live checkout is the behaviour the refusal exists to prevent.` |
| **Q3 — the banner drops the derived size** | `the banner must carry the DERIVED count of what went unmeasured (126); got: PARTIAL RUN (SIGIL_ALLOW_PARTIAL is set). No reference tree is named, so some test binaries are reference-dependent and …` |
| **Q4 — the strict+partial check removed** | `the run stopped, but not with the resolver's own refusal — so what this gate measured is that SOMETHING failed downstream of the contradiction rather than that the contradiction itself was caught. A reader of that failure is told a path is missing, not that the two flags they set cannot both hold.` |

### Q4 is worth reading twice: the first version of that gate came back GREEN

The first `strict_and_partial_together_are_refused` asserted only that the child failed and
that both flag names appeared in its output. With the resolver's combined check removed it
still passed — the run *does* stop either way, via `reference_tree`'s pre-existing strict
assertion, and both flag names appear (one in the partial banner, one in that assertion). A
rebuild was forced first, to rule out a stale artifact rather than assume the matcher; it
stayed green, so the matcher was the problem. The assertion now pins the resolver's own
wording, and why that matters is recorded in the test beside it: the two failures stop the
same run and tell the reader different things.

## What a run says now

Bare — `AEON_DIR`, `EMPYREAN_SUITE_ROOT`, `SIGIL_STRICT_GATE` and `SIGIL_ALLOW_PARTIAL` all
unset, `cargo test -p sigil-cli --test header_port`:

```
thread 'demo_header_matches_reference' panicked at crates/sigil-harness/src/test_support.rs:1018:9:
NO REFERENCE TREE IS NAMED, so this run can measure nothing it could attribute, and STOPS. This run DECLINED to use /home/volence/sonic_hacks/aeon, which step 3 derived from this checkout's own location: it is a working checkout outside this repository, its revision changes under a run without notice, and a result measured against it would be attributable to whatever it happened to contain rather than to the code under test.

The resolver's own answer: reference-tree: /home/volence/sonic_hacks/aeon (SUITE_PATHS step 3 — DERIVED from this checkout's own location — nobody named it)

Either name a provisioned tree — AEON_DIR=<aeon checkout> (scripts/provision-aeon-ref.sh), or EMPYREAN_SUITE_ROOT=<the directory holding the suite> — or declare a partial run with SIGIL_ALLOW_PARTIAL=1, in which case every reference-dependent row is left unmeasured and the run says how many. Ruled d-18 (docs/OVERSEER.md, 2026-09-02): a run that only PRINTS how much it did not measure still exits 0, and a green is trusted the moment it is in the run.

failures:
    demo_header_matches_reference
    sonic4_header_matches_reference

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Declared partial — the same environment plus `SIGIL_ALLOW_PARTIAL=1`:

```
running 2 tests
PARTIAL RUN (SIGIL_ALLOW_PARTIAL is set). No reference tree is named, so 126 test binaries are reference-dependent and every row in them is left UNMEASURED. A green result from this run does NOT mean those rows passed — it means they were not run. Name a tree with AEON_DIR to measure them.
reference-tree: /home/volence/sonic_hacks/aeon (SUITE_PATHS step 3 — DERIVED from this checkout's own location — nobody named it)
skip: reference ROM not at /nonexistent/SIGIL_ALLOW_PARTIAL-no-reference-tree-was-named/s4.bin (set AEON_DIR)
skip: reference ROM not at /nonexistent/SIGIL_ALLOW_PARTIAL-no-reference-tree-was-named/demo.bin (set AEON_DIR)
test sonic4_header_matches_reference ... ok
test demo_header_matches_reference ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## One classifier consequence, handled the way this lane already handles it

`bare_run_refuses` calls `reference_tree(` — in a child, with a scrubbed environment, aimed
at a path that does not exist — so the static classifier reads it as reference-reading, and it
landed UNCLASSIFIED, which makes the nightly lane refuse to run. It is added to `SOURCE_GATES`
with a comment saying exactly why, which is the treatment `reference_dependence_is_named`
already has in that file for the identical reason: a file that reads sigil's own test sources,
opens nothing in any aeon tree, and cannot be told apart from a real reader by a static grep.
Audit after: `SOURCE_GATES=46 scanned=134 source=45 artifact=85 no-reference=4
unclassified=0`.

## Hand-off to `parcel/scripts-name-their-tree`

`scripts/landing-run.sh` belongs to the concurrent agent and was **not touched** by either
parcel. One item for them, if their branch is still open:

* the wrapper's help/usage text does not describe what a bare run does. It now **stops**
  rather than measuring against the derived checkout, and `SIGIL_ALLOW_PARTIAL=1` is the
  declared partial run. If that text is being rewritten anyway for the suite-paths work, this
  is the line to add. Nothing breaks without it — `landing-run.sh` always sets `AEON_DIR`, so
  its own path is unaffected; the gap is documentation for someone invoking the suite by
  hand.

## Booked in the gap ledger

Three entries appended to `docs/superpowers/notes/campaign-gap-ledger.md`: the six
`env::var("AEON_DIR")` reads in the artifact-writing tools that deliberately did not route
(recorded as a decision, not an omission); the classifier's blind spot for a reach through
`ResolvedCheckout.path`; and `aeon_dir_is_unnamed()`, which has no consumers.

---

# The contract moved mid-parcel, and it landed on this lane's step-3 row

`contract/SUITE_PATHS.md` gained a fourth bullet under "What a resolver owes its reader"
on 2026-09-02, from **aurora's O68**, which they found on their own already-merged
resolver. Read at `empyrean` `08dd3f6` (reachable from `origin/main`; byte-identical at the
then-current tip `f96fbf6`):

> **The step-3 proof runs from a linked worktree, or says in the run's own output that it
> did not.** The property step 3 is written for, `--git-common-dir` answering where
> `--show-toplevel` answers wrongly, is only observable from a linked worktree; in the main
> checkout the two agree, so a test asserting it there proves nothing, and a test that skips
> there is honest but never runs where the suite normally runs. […] Every lane with a
> resolver has this shape to check, not only the one that found it.

**This lane's first version had exactly the shape the clause forbids**, and it is worth
being precise about how it passed. It asked whether the TEST PROCESS happened to be running
in a worktree, asserted the property if so, and printed `NOT MEASURED` otherwise. Every
sigil agent runs in a linked worktree under `.claude/worktrees/`, so it asserted for whoever
wrote it — and would have printed `NOT MEASURED` from
`/home/volence/sonic_hacks/sigil`, which is where the landing run and the nightly source
lane actually invoke the suite. The row would have been live only in the one place it was
never needed.

## The bed

`the_step_3_derivation_is_proven_from_a_linked_worktree` in
`crates/sigil-harness/tests/suite_paths_precedence.rs` now builds its own:

```
<scratch>/suite/            <- a suite root by the marker rule
  aeon/                     <- the sibling the resolver must reach
  empyrean/
  repo/                     <- a real git repo (init, one commit)
    nested/wt/              <- a LINKED worktree, `git worktree add`, removed after
```

**Nested inside the repo, not beside the suite root, and that is load-bearing** — the
refinement the concurrent scripts lane measured for its own resolver rather than assuming,
matched here so the two halves of one contract do not disagree about what proves it. From a
worktree that happens to sit beside the suite root, `--show-toplevel` plus a sibling join
lands on the right answer *by accident*, and a test built on that bed passes for the wrong
reason.

Three properties, and the row asserts all three:

1. **a control before the property** — on this bed the two methods must actually disagree,
   or the assertion has nothing to bite on and is reported UNMEASURABLE;
2. **the property** — `derive_suite_root_from(<the nested worktree>)` reaches the suite root
   the repo hangs off;
3. **the production half** — the same function, applied to this crate's own compile-time
   location, agrees with an independent marker walk. Without this, a helper proven on a bed
   and a production path that calls something else is the classic way a gate ends up
   measuring nothing.

`derived_suite_root()` is now one line: `derive_suite_root_from(CARGO_MANIFEST_DIR)`, cached.
The mechanism became a function over a directory precisely so the proof could be made
somewhere the process is not.

**Why a directory argument rather than a subprocess with a different cwd.** This resolver's
step 3 does not read cwd at all — it runs git from `env!("CARGO_MANIFEST_DIR")`, the calling
repo's own location fixed at compile time, which is the contract's phrase and is more robust
than cwd (a test process's cwd is a cargo convention; a subprocess's is whatever it
inherited). A cwd-driven bed would therefore have proved nothing about this implementation.
The directory argument is the equivalent, and it needs no subprocess to be deterministic.

An unbuildable bed — no `git`, no writable scratch, a git too old for `worktree add` —
PRINTS its reason into the run's output and does not assert. That is the clause's own
escape, taken as a printed line rather than an `ignored`, because a green log and an absent
run are the same artifact.

## Red-first, both halves

**R1 — the derivation switched to `--show-toplevel`.** The row fails on its own bed, naming
the bed's own paths — which is the evidence that it no longer depends on where the suite was
invoked from:

> `step 3 could not derive a suite root from the linked worktree /tmp/sigil-suite-paths-worktree-bed-…/suite/repo/nested/wt — /tmp/sigil-suite-paths-worktree-bed-…/suite/repo/nested is this repository's parent but holds no aeon/ + empyrean/ — it is not a suite root. This is the shape every sigil agent runs in, so a derivation that fails here fails in ordinary use.`

**R2 — the bed's worktree moved beside the suite root** instead of nested. The control
fires, which is what makes "nested" a measured requirement rather than a stylistic one:

> `assertion 'left != right' failed: UNMEASURABLE: on this bed --show-toplevel's parent IS the suite root, so the wrong method would give the right answer and passing proves nothing. The worktree must be nested inside the repo, not beside the suite root. / left: Some(".../suite") / right: Some(".../suite")`

Both poisons were reverted and the file re-run green.

---

# The red-first audit: both ways a poison goes green while proving nothing

A red-first proof can be worthless in two ways, and the second was found by another lane
reproducing the first:

1. **the patch never landed** — the run executed the original file;
2. **the patch landed and the runner executed a CACHED ARTIFACT built from the old source**
   (their case was Python's `__pycache__`).

Applied-but-green is a runner defect to fix before claiming the gate, never a pass. Every
row this parcel claims is sorted below against both, including the ones claimed before the
rule reached me — a rule applied only forward leaves the existing proofs resting on the
method it just condemned.

## Mechanism 1: did the patch land?

Structurally, for all twelve. Every poison was a Python script that `assert`s its anchor
text is present in the file before writing and prints `poisoned <name>` after; a poison whose
anchor had moved aborts instead of silently writing nothing. Two were additionally confirmed
by grepping the file back (`grep -n 'if false && partial && strict'` before the Q4 run).

## Mechanism 2: could a prebuilt artifact have answered?

**Sigil's exposure is the prebuilt binary passed by path.** `SIGIL_EMIT` in this session's
environment names `.sigil-target-resolver/release/emit_sound_blob`, built before the parcel
started. **No row below has its subject inside it, or inside any other prebuilt artifact.**
Enumerated rather than asserted:

| Test binary | What it executes | Prebuilt artifact consumed |
|---|---|---|
| `suite_paths_precedence` | the linked harness library, `std::env::current_exe()` (itself), `git` | none |
| `bare_run_refuses` | the linked harness library, `current_exe()`, sigil's own test SOURCES | none |
| `source_gate_classification` | `bash scripts/nightly_source_gates.sh --audit`, which reads `test_support.rs` as TEXT | none — `--audit` is read-only and builds nothing, by its own contract in that file |

`SIGIL_EMIT` was used once in this parcel, for the `repin --check` provisioning witness, which
is not a red-first proof. The two gates that do reach an emitter —
`reference_tree_named_write` and `reference_tree_write_guard` — call
`sigil_harness::seam1::emit_sound_blob` as a **linked library function**, not the binary, and
neither was poisoned here.

`current_exe()` is the one place the cached-artifact mechanism could bite in this file: a
subprocess gate re-runs the test binary, so a cargo that did not rebuild would re-run the
pre-mutation build. Cargo rebuilds the library the tests link and relinks the binary, and the
discipline of checking that is what Q4 below records.

## The sorting

**Class A — went RED on the poison.** A red cannot be produced by a mutation that did not
take effect, nor by a stale artifact built from the un-poisoned source: both would run the
original code, which passes. Sound by construction.

| Row | Poison |
|---|---|
| P1 | step 1 set-but-wrong falls through to step 2 |
| P2 | the derivation uses `--show-toplevel` |
| P3 | step 2 set-but-wrong falls through to the derivation |
| P4 | `unnamed_default_tree` consults step 1 |
| P5 | the resolver's env read moved into a private helper (subject is the shell classifier, no build at all) |
| P6 | the accessor closure drops one accessor (same — shell over source text) |
| Q1 | say-only: announce, then return the derived tree |
| Q2 | the partial run answers with the derived checkout |
| Q3 | the banner drops the derived size |
| R1 | the derivation uses `--show-toplevel` (the bed row) |
| R2 | the bed's worktree moved beside the suite root (the control) |
| the `Drop` proof | an assertion forced to fail after the bed exists |

**Class B — printed `ok` on the poison, and was only established by a later red.** One row,
**Q4**, and it is written up in full above. What matters for this audit is the order in which
it was diagnosed: the green was **not** attributed to the matcher until a rebuild had been
forced (`touch crates/sigil-harness/src/lib.rs`, then observing the `Compiling sigil-harness`
line) and the run had come back green a second time from a build that provably postdated the
mutation. Only then was the assertion tightened, and only then did the same poison produce a
red. Mechanism 2 was the first hypothesis, not an afterthought.

One honest correction to a reading of that transcript: the second Q4 invocation reported
`Finished … in 0.02s` with no `Compiling` line, which is the stale-artifact signature — but
the *first* invocation had already built, and its build output was filtered away by the grep.
So no staleness incident actually occurred. The point stands that it could not have been
known from that run alone, which is why the rebuild was forced rather than assumed.

## Cleanup, confirmed rather than asserted

The clause says the bed's worktree is "removed after". Checked on the path that matters:

* `git worktree list` in this checkout — whose registry IS
  `/home/volence/sonic_hacks/sigil`'s, since a linked worktree shares the common repo —
  reads **39 entries** after a green run and **39 after a failing one**, all of them real
  lanes and agents. The bed's `git worktree add` runs inside the bed's OWN repository, so it
  never registers in the real checkout.
* Scratch directories: with an assertion forced to fail *after* the bed exists, the run
  FAILED and left **zero** `/tmp/sigil-suite-paths-*` directories. Cleanup is an
  `impl Drop for Scratch`, so it runs on unwind.

**That cleanup was a real finding, not a formality.** Before it, this file swept with a
`remove_dir_all` at the end of the test — a line a failing assertion never reaches. Sixty-eight
directories had survived this parcel's own red-first runs, two of them beds carrying a git
repository, and the precedence row had been leaking four fixtures per invocation since it was
written. The same defect the concurrent lane hit as 43 registered worktrees, arriving here in
the shape this file happened to have.

## The contract paragraphs, by SHA

The step-3 clause landed and was sharpened three times after this parcel's brief was written,
so each paragraph is cited where this lane's work meets it:

| empyrean SHA | Paragraph | Where it lands here |
|---|---|---|
| `08dd3f6` | the bullet itself — the proof runs from a linked worktree or says it did not | `the_step_3_derivation_is_proven_from_a_linked_worktree` |
| `9c86639` | the worktree must be NESTED, credited to the concurrent scripts lane on this work | the bed's `<repo>/nested/wt`, and the control that refuses a bed where the wrong method would be right |
| `923cfd4`, `9d21bf9` | aurora's addition: the bed runs the resolver ANCHORED at the bed, not the main copy with a different cwd | `derive_suite_root_from(anchor)` and the call against the bed's path |
| `3865ad4` | assert INSIDE the bed that the wrong and right derivations disagree there | the `assert_ne!` that reports UNMEASURABLE rather than skipping |
| `2eaadd8`, `f17940b` | the compiled-language form, with this lane's shape quoted as the worked example | the whole row |

**On the compiled-language reading.** The clause's literal words are that the bed runs the
worktree's *copy* of the resolver, which maps onto a runtime module load — aurora's
`sibling-root.mjs` is loaded from a path. Rust has one compiled copy and no runtime load, so
the faithful translation of the intent — measure the real walk against a bed where the wrong
method is demonstrably wrong — is to invoke the extracted walk with the bed as its anchor,
not to compile the crate inside the bed. That reading is now contract text (`2eaadd8`), with
this shape quoted as the worked example for a compiled lane (`f17940b`), so it is cited here
rather than flagged as a deviation.

Worth recording why the sharpening was needed twice, because sigil had the identical
exposure: aurora's resolver runs `git` with `cwd: AURORA_ROOT`, its own computed anchor, so a
bed that only changes directory runs git in the MAIN checkout and passes having measured the
main-checkout case under a worktree-shaped name. Sigil's `derived_suite_root()` had the same
pattern — `.current_dir(here)` with `here = env!("CARGO_MANIFEST_DIR")`, a compile-time
constant, additionally cached in a `OnceLock`. **A cwd-only bed was provably inert here, not
merely weak**, which is why the walk had to become parameterizable before any bed could
prove anything.

## The two arms, and what each one claims

The step-3 row makes two different claims and the packet should not blur them:

* **the bed arm** — `derive_suite_root_from(<the bed's nested worktree>)`. Invariant to where
  `cargo test` was invoked from, because the bed is constructed by the test. **This is the row
  that must always run and always mean something**, and it is what the contract requires.
* **the ambient arm** (`ambient_worktree_check`) — the same walk against
  `env!("CARGO_MANIFEST_DIR")`, the anchor `derived_suite_root()` actually deploys behind, in
  whatever checkout this binary was compiled in. It reaches something the bed cannot: the real
  deployed anchor. But it can only ASSERT when the ambient run happens to be in a linked
  worktree, so it is explicitly secondary, and each of its three unmeasurable cases prints a
  line saying which check was skipped and that the bed still proved the walk — so a reader is
  never left thinking step 3 went unmeasured when it did not.

Production behaviour is unchanged by the split: `derived_suite_root()` is
`derive_suite_root_from(env!("CARGO_MANIFEST_DIR"))` with the same `OnceLock` and the same
doc reasoning for the anchor.
