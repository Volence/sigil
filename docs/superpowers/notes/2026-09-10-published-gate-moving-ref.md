# The published anchor read a moving ref twice

2026-09-10, branch `parcel/published-gate-race`.

Two tests in `crates/sigil-cli/tests/version_provenance.rs` went red on correct
work whenever anybody pushed or fetched while they ran:

* `the_published_line_states_this_revision_s_position_against_a_named_remote_ref`
* `the_published_drift_check_runs_and_is_anchored_at_the_named_ref`

## The mechanism, and the part of it that was not obvious

The banner's `published:` line names a remote-tracking ref and bakes that ref's
tip into the text. Both tests then resolved the same ref again, at test time,
and required the two readings to agree.

The window is not an instant. The tip is captured when `build.rs` runs, which is
the start of the cargo invocation, not when `sigil --version` is invoked. So the
race spans the whole build-and-test run: compile, link, and every test that runs
before this one. A push landing anywhere in there makes two truthful readings
disagree.

`refs/remotes/origin/master` is updated locally and immediately by a push from
any checkout on this machine, and by any fetch. On a machine with four lanes
live that is an ordinary event, not a rare one.

## The two tests do not share a trigger

This was assumed and is false. Measured in a throwaway local clone:

| driver | old test 1 | old test 2 |
|---|---|---|
| fast-forward move of the ref | FAILED | ok |
| rewind of the ref | FAILED | ok |
| ref flapped across a divergent position, 60 runs | n/a | 36 red |

An ordinary push reddens only the first test. The second cannot be reddened by a
fast-forward at all: its check is that the printed revision is reachable from the
named ref, and advancing the ref preserves reachability. Its window is the gap
between the printed command resolving the ref and the reachability check
resolving it again, and it needs a move that is not a fast-forward. That window
is real, demonstrated above, and sub-millisecond.

Anything that reddened test 2 in the field, absent a force-update, is therefore
still unexplained by this mechanism.

## The fix: ask the recorded positions, not the current one

The assertion that cannot be made race-free is `git rev-parse <name> == tip`. It
checks that git agrees with git about a ref that is allowed to move.

It is not droppable, though. It is the only thing that catches a banner naming a
plausible tip taken from HEAD. The contained-verdict check cannot: with the tip
set to HEAD, `merge-base --is-ancestor HEAD HEAD` succeeds, the banner prints
"yes, this revision is contained in origin/master (HEAD)", and every other
assertion in the file passes. That was measured, not reasoned about.

So the property is kept and re-expressed against a fixed set instead of a moving
value. A reflog is append-only: advancing a ref appends an entry, rewinding it
appends another, and a value that is in the set stays in the set. `positions_of`
walks `<ref>@{n}` for the most recent `RECENT_POSITIONS` steps, which is every
position the ref is on record for within a bounded window. See the two sections
below for why the window is bounded and why `@{n}` rather than the reflog's
rendered values.

* Test 1 requires the named tip to be a commit that exists here, and to be a
  member of that set. Membership is nearly the original strength: the original
  said the tip equals the ref's value now, this says it equals one of the values
  the ref has held.
* Test 2 requires the printed revision to be contained in some member of that
  set, rather than in the ref as re-resolved a moment later.

Where a checkout keeps no reflog for the ref, test 1 falls back to containment
and says so in the failure text. That case is a real configuration, and an
always-red check on correct configuration is worse than a weaker one, which is a
standing ruling in that module.

Deliberately not a retry loop. Re-running until green hides real faults, and a
bounded retry keyed on "the ref moved" is the same move wearing a condition.

## Reproduction, which is deterministic

No real ref is touched. Clone locally so the clone owns its refs:

```
git clone /home/volence/sonic_hacks/sigil <scratch>
cd <scratch>
CARGO_TARGET_DIR=<on-disk> cargo test -p sigil-cli --test version_provenance --no-run
A=$(git rev-parse origin/master)
git update-ref refs/remotes/origin/master $(git commit-tree -p $A -m push "$A^{tree}")
<target>/debug/deps/version_provenance-<hash> --test-threads=1
```

Building with `--no-run` and then running the built binary directly is what makes
it deterministic: the build script captures the tip, the ref moves, and nothing
rebuilds in between. That is the real sequence, with the timing luck removed.
Moving the ref with `commit-tree` and `update-ref` leaves HEAD and the working
tree alone, so the tree-state tests in the same file stay meaningful.

For test 2, run the ref back and forth between two divergent positions in a
background loop while running that test repeatedly.

Use a FRESH clone for the ref-move drivers. A clone that has already been
flapped carries hundreds of positions in its reflog and will pass things a
fresh one fails, which is how the omitted starting position went unseen. The
flapper and the ref-move drivers want separate clones for that reason.

## Evidence, both directions, same driver

* old code, fast-forward driver: 18 passed, 1 failed, at the `rev-parse`
  equality
* old code, rewind driver: the same single failure
* old code, flapper, test 2 alone: 36 of 60 runs red at the reachability check
* new code, fast-forward driver: 19 passed 0 failed
* new code, rewind driver: 19 passed 0 failed
* new code, flapper, test 2 alone: 0 of 60 runs red

Positive controls, because a race fix that quietly deletes the property looks
exactly like a race fix that keeps it:

* tip baked from HEAD instead of from the ref: test 1 red, and note the banner
  still printed "yes, contained", so nothing else in the file caught it
* printed command names the ref but answers about HEAD
  (`log -1 --format=%H HEAD --not <ref> --`): test 2 red at the reachability
  assertion, which is the line that changed

Both controls were applied to `build.rs`, shown on disk before the red run, and
restored from the committed baseline afterwards.

### What the first control does NOT establish

It ran in a checkout where HEAD is a branch tip that the ref has never held.
There the tip-from-HEAD defect is visible, and the control shows the assertion
can fire.

It does not show the assertion firing on the tree the gate actually guards at
the moment that tree matters most. In the main checkout at a landing moment,
HEAD and `origin/master` are the same commit: both were `8b3091bc` while this
was written. A tip-from-HEAD banner in that state produces a tip that is the
ref's current position, so it passes the membership check, and it passed the
old equality check too for the same reason, the two values being one string.

So the honest scope: **the gate is blind to a HEAD-sourced tip precisely on a
freshly-pushed master, which is the state at every landing.** Nothing regressed,
old and new are equally blind, and the blindness is inherent to comparing a
value against a set it belongs to. It is written down because a canary proves
the pattern can fire, never that the input arrives, and the reading it invites
otherwise is that the control covers the landing case.

## The bound: a set that only grows is a check that only weakens

The first version of this fix accepted every position the ref had ever held.
That is 860 positions here, over 27 days, and it has two costs. The failure
path forks one `git merge-base` per position, so its cost is monotonic in the
repository's age. Far worse, membership in 860 historical positions is a much
softer question than membership in a handful, the set grows every push, and
nothing would ever notice, because a passing check looks identical at every
strength.

`RECENT_POSITIONS = 32`, derived rather than chosen. The only interval that has
to be covered is one cargo invocation, because the tip is captured when the
build script runs and compared in the same invocation. Measured build-and-test
runs: 0.4 s warm, 9 s for a cold build of this crate, 53 s for this crate's
whole suite. Against that, this ref's reflog over 27 days holds:

| window | busiest observed |
|---|---|
| 1 minute | 3 updates |
| 5 minutes | 5 updates |
| 15 minutes | 8 updates |
| 30 minutes | 13 updates |
| 1 hour | 21 updates |

at a mean of 32 updates per day. So 32 covers the worst recorded half hour two
and a half times over, covers the worst recorded hour outright, and is about a
day of movement at the measured rate. Worst-case failure cost measured at
**0.034 s for 32 forks**, against roughly 0.6 s for all 860, and it is now
constant rather than growing.

Both failure texts name the bound, the number of positions actually available,
and the fact that anything older was not asked about, so a run that consulted
3 positions does not read the same as one that consulted 32.

## A hole in the first version of this fix, found by the bound's reproduction

The first version built the position set from
`git reflog show --format=%H <ref>`. That renders the value each entry moved
**to**. The value the ref moved **from** on the oldest entry in the window is in
no such line, and `git clone` writes **zero** reflog entries for
`refs/remotes/origin/master`. So in a freshly cloned checkout, after one move,
the set held exactly one position and omitted the one the ref started at, which
is precisely the position a banner built before that move names. The check
reddened correct work in the case it exists to tolerate.

It was invisible at first because the clone used for the earlier runs had been
flapped and carried the tip among 342 entries. A real hole looked like a pass
because the reproduction was more thorough than the situation it modelled, which
is the pleasant direction of the same error.

`<ref>@{n}` is the value the ref HELD n steps ago. It reports both sides of every
entry, so it cannot omit the starting position; it fails cleanly past the end of
the reflog and when no reflog exists at all, which is what the no-reflog fallback
already keys on. Verified: with no reflog `@{0}` exits 1, and after one move
`@{0}` is the new value and `@{1}` the old one.

## Booked separately: a partial run's skipped rows read as `ok`

Not part of this parcel, recorded here so the trail exists.

With `SIGIL_ALLOW_PARTIAL=1`, `act_descriptor_region_matches_reference` and
`act_descriptor_debug_region_matches_reference` print plain `ok` in a `cargo
test -p sigil-cli` log. An aggregate of 766 passed / 0 failed therefore includes
two rows that measured nothing against a reference tree, and the log does not
say so.

The marker is not missing. `SKIP_MARKER` is `"skip: "`
(`crates/sigil-harness/src/test_support.rs:1477`) and the rows do print
`skip: reference ROM not at
/nonexistent/SIGIL_ALLOW_PARTIAL-no-reference-tree-was-named/s4.bin (set
AEON_DIR)`. Cargo captures stdout for PASSING tests, so it never reaches the
log. A reader has to run `cargo test ... -- --nocapture` and grep `skip: `, or
grep the stand-in path, which is self-describing for exactly this reason.

`scripts/nightly_source_gates.sh` is **not** affected: it invokes
`-- --nocapture` and greps the marker, so its skip detection is sound. The
exposure is ad-hoc runs, which is what an agent reports totals from.
