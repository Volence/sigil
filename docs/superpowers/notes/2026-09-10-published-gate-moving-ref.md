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
returns the ref's current value plus every value its reflog records.

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
