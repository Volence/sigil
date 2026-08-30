# The source-gate lane's classifier was one question short — and one premise wrong

*2026-08-30, branch `fix/source-gate-third-bucket`.*

## What was broken

`scripts/nightly_source_gates.sh` refused to run. Its own log, read at
`~/.local/state/sigil-source-gates/nightly.log`:

```
2026-08-30T05:17:32-04:00 COULD NOT RUN: 4 aeon-reading gate(s) are neither in
SOURCE_GATES nor artifact-dependent — classify each at sigil 6ef35d36 / aeon 07430004:
hole_interior_reserved section_alignment_declared reference_tree_write_guard
region_end_contracts
```

Every such refusal is `notify-send -u critical` on the owner's desktop and **zero coverage
for the night**. By the time this parcel started, a concurrently-landed branch had added a
fifth file in the same shape, so the count on master was **5**, not 4.

## The premise the brief carried, and how it broke

The brief's diagnosis was that all four files match the selector on **prose** — comments
explaining what the reference-tree variable is — while reading no tree at all. That is
true of **one** of them.

The discriminator is not a reading of the source; it is one command. Point the suite at a
tree that does not exist, under `SIGIL_STRICT_GATE=1`, where a missing reference is a
panic rather than a skip:

| file | absent tree, strict | verdict |
|---|---|---|
| `hole_interior_reserved` | **FAILED** 0 passed / 3 failed, naming the missing path | reads the tree |
| `section_alignment_declared` | **FAILED** 1 passed / 2 failed, naming the missing path | reads the tree |
| `region_end_contracts` | **FAILED** 0 passed / 2 failed, naming the missing path | reads the tree |
| `reference_tree_write_guard` | ok, 2 passed, 0.00s | obtains nothing |
| `reference_tree_named_write` (the fifth) | ok, 1 passed, 0.00s | obtains nothing |

Against a real provisioned tree all five are green (3.92s / 11.03s / 35.01s / 0.00s /
0.00s), so the failures above are the guard firing, not a defect.

**Three of the five were genuine source gates nobody had classified the day they landed** —
the exact obligation this document's lane section already names. They are oracle'd on
declarations (a shape's own `map.toml`, `section_align.rs`, the *kind* of contract in
`repin.toml`) rather than on any committed artifact, so they are not the excluded third
shape, and they belong in the run list. Classifying a source gate means the lane runs it;
that half is enumeration, and it is enumeration-as-invocation, not a roster of exceptions.

**The general form: a file's bucket is a behaviour, and prose about it is not evidence.**
Both the brief and a reading of the headers agreed on a story the one available command
refuted in ninety seconds.

## The derived third bucket

For the other half the brief was right, and its shape matters: the classifier asked *which
artifact does this file name?* and presumed every selected file reads a tree. The selector
matches an **identifier**, and an identifier appears in the file that explains it as
readily as in the file that calls it.

The rule, stated precisely:

> A file **obtains** the reference tree iff its text contains a call to one of the
> reference-tree accessors, or reads the reference-tree environment variable itself. The
> **accessor set is derived**: seeded with the public function of
> `crates/sigil-harness/src/test_support.rs` that reads that variable, closed under calls
> between that file's own public functions, with comment-only lines stripped first (a doc
> comment showing a caller how to open a gate otherwise manufactures accessors nothing
> calls). A selected file that is not in `SOURCE_GATES`, names no built artifact, and
> obtains no tree is **`no-reference`**: counted, not run, not a defect.

Today that derives `aeon_dir`, `reference_tree`, `reference_tree_for_profile`. Nothing
names a member of the bucket.

**Asked third, on purpose.** The two established questions answer first, so the new one can
only ever speak for a file that used to fall through to `unclassified`. Bucket stability is
therefore structural, not merely observed — and it was also measured, holding the run list
at master's 41 so the only variable was the new question:

```
OLD:  85 artifact   40 source    4 unclassified
NEW:  85 artifact   40 source    1 no-reference   3 unclassified
source bucket identical:   YES
artifact bucket identical: YES
files that changed bucket: reference_tree_write_guard, unclassified -> no-reference
```

**It is a rule, not a list, and that was tested rather than asserted.** Mid-parcel, the
concurrent branch landed `reference_tree_named_write` — a file this rule had never seen.
Rebasing onto it, the audit classified it correctly with no edit:
`scanned=130 source=43 artifact=85 no-reference=2 unclassified=0`, and the absent-tree run
above confirms the answer behaviourally. Master's own classifier on the same tree says
`unclassified=5`.

## Loud on unmeasurable, proven in the direction that matters

An empty accessor set would make every file look like it reads nothing — the whole
population into `no-reference`, the lane green over something it never classified. That is
the failure worth engineering against, so the rule refuses when it cannot be derived.
Proven red-first by breaking the derivation and restoring it:

- rename the variable `aeon_dir` reads →
  `UNMEASURABLE: no reference-tree environment variable is extractable …`, exit 2;
- drop `pub` from `aeon_dir` (empty closure) →
  `UNMEASURABLE: no reference-tree accessor is derivable … so every file would falsely
  look like it reads nothing`, exit 2;
- cut the closure's iteration bound to one, forcing non-convergence →
  `UNMEASURABLE: … did not reach a fixed point — a truncated closure is short accessors,
  and each one it is short makes some file look like it reads nothing`, exit 2.

Each restored and the audit re-run green. The third of these was a hole found in review
rather than by the brief: the loop originally capped at five rounds and **returned the
partial set**, which is wrong in the quiet direction — a closure short an accessor makes
some file that reads the tree look like it reads nothing, i.e. moves it into the bucket
this lane does not run. A bound that truncates silently is a smaller version of the
vacuous gate the whole lane exists to prevent.

## The runner, and why the lane stayed dark

**Nothing in `cargo test` saw this lane.** The only thing that ever asked the question was
the 05:17 timer, and the answer arrived as a popup on the owner's lock screen.

`--audit` now runs the classification alone: read-only, no worktree, no build, and it never
reaches `note()`, so it cannot page anybody. `crates/sigil-harness/tests/source_gate_classification.rs`
invokes it on every `cargo test --workspace` — which is what `scripts/landing-run.sh` and
`.github/workflows/ci.yml` run — so **an unclassified file now fails a landing run**. One
definition of the rule, two callers: a second implementation in Rust would be a second
thing to keep in step, which is the defect class this lane has already been bitten by twice
(a retyped skip marker, a retyped gate count).

Red-first: with a poison reader in the tree the gate fails, quoting the audit
(`unclassified: zz_poison_reader`); removed, it passes. Its second test reconciles
`source + artifact + no-reference + unclassified` against `scanned` and refuses a zero
population, so a classifier that silently dropped files could not report `unclassified=0`
and be believed.

That test file deliberately **does not write** any of the four selector spellings — it
describes them instead — so it stays out of the population it judges. Same remedy the
`feat/version-provenance` catch used, and worth keeping in any new test file.

## The lane runs — end to end, at aeon's live tip

The deliverable is a running lane, not a passing unit test about a classifier, so the real
script was run at this branch's committed SHA via `SIGIL_SOURCE_GATES_REF` — the mechanism
that exists for exactly this — against `AEON_SOURCE_GATES_REF=master`, in the lane's own
shared checkouts, with `notify-send` stubbed.

**A silent terminal is evidence about stdout and nothing else**, so the outcome was read
out of `~/.local/state/sigil-source-gates/nightly.log`:

```
2026-08-30T05:28:08-04:00 OK at sigil c3808d41 / aeon 07430004 (181 passed, 44 gates;
85 aeon-reading gates skipped as artifact-lane (…); 2 no-reference (name the tree, obtain
none — the workspace suite runs them))
```

Exit 0. 84 seconds wall (05:26:44 → 05:28:08; uptime at the start of the parcel `up 4 days,
21:14`). From the lane's own `gates.log`, aggregate and not a tail: **44 `test result:`
lines for 44 named gates, 181 passed / 0 failed / 1 ignored, zero `skip:` lines** under
`SIGIL_STRICT_GATE=1`. The three newly-classified gates are present in it by name
(`Running tests/hole_interior_reserved.rs`, `section_alignment_declared.rs`,
`region_end_contracts.rs`) — a green log that does not contain the parcel's own tests is a
green log about other code — and they are green against aeon's **live master**, not only
against the pinned revision.

The two open warn-tier findings the verdict prints (`import.no-names`, 12 days) are the
register doing its job and are not this parcel's.

## Notifications during this work

The real `notify-send` was never invoked. A stub at
`/home/volence/sonic_hacks/.classify-stub-bin/notify-send` was put first on `PATH` for this
agent's processes only, logging what would have popped up; `--selftest-fail` was run once
through it to prove interception (`NOTIFY-SEND-STUB -u critical sigil source gates
SELFTEST: …`). `/usr/bin/notify-send` was not touched, and the stub directory was removed
afterwards. **The timer was not masked, disabled or softened** — the ruling stands: the
lane yields no coverage while it refuses, so silencing it would trade an urgent
self-limiting failure for a permanent invisible one. The popup stops because the lane runs.

## Left open

`SOURCE_GATES` remains a hand-kept run list, and three of the five files that darkened this
lane were source gates nobody had added. The derived rule already computes the property
that decides it, so the lane **could** derive its run list and a new source-only gate would
join automatically rather than refusing. That is a soundness trade, not a parcel-local
call: strictly better than dark, but a third-shape gate (source inputs, oracle'd on a
golden or on `pins.rs`) that stopped naming its artifact would then auto-join and be red
through every refreeze window, and nightly criticals nobody can clear are how a lane gets
ignored. It needs its own ruling. The mechanism to implement it is in place either way.
