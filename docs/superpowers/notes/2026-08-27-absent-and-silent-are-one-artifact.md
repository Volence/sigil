# A check that reports nothing and a check that never ran are one artifact

*Written 2026-08-27 by the sigil overseer, at the aeon lane's request. They asked for the
unification rather than guessing at sigil's half; the instances below are drawn from both
lanes in one night and are named to their finders.*

Protocol **bar 25** says a green log and an absent run are the same artifact. In one night
this lane hit **five** instances, and the point of writing them together is that they were
found in five different-looking ways and only afterwards turned out to be one shape.

## The shape

A verification produces an artifact — a log, a count, a green tick. The artifact is read as
*"the thing was checked and was fine"*. But three distinguishable states collapse into the
same artifact:

1. the check ran and found nothing wrong,
2. the check ran and **could not see** the thing it was looking for,
3. the check **never ran at all**.

Bar 25 named (1) vs (3). The night's harvest says (2) is the larger family and the one that
survives review, because a check that *cannot see* is still a check: it has a name, a runner,
a passing result, and a maintainer who believes in it.

## The five instances

**A. The strict flag that was never set.** *(aeon's finding.)* The landing rule spelled the
suite as a bare `cargo test --release --workspace --no-fail-fast`, omitting
`SIGIL_STRICT_GATE=1`. Every `strict_gate()`-guarded gate early-returns without it, so two
chains landed having run a suite that structurally could not execute the gates the refreeze
exists to move. **The rule was complete in its steps and inert in its spelling**, and nobody
audits a command line for a missing environment variable.

**B. The 27 skips the bar could not match.** *(aeon's finding, sigil's to own.)* The bar
requires zero `skip:` lines. Twenty-seven sites — thirty, once enumerated by property rather
than by spelling — announced themselves as `skipping …`. A gate could no-op and clear the
bar. State (2): the check ran, and could not see.

**C. The bar that could never have failed.** *(sigil's, and the one that reframes the rest.)*
Even with the spelling fixed, **libtest captures a passing test's output**. Measured, same
binary, same conditions: `cargo test … --test seam2_dac_emit` → 0 skip-shaped lines;
`… -- --nocapture` → 2. So the zero-`skip:` requirement had been **structurally incapable of
failing for its entire life**, and every hand-run landing that reported it was reporting on
an empty page. Note B and C are *independent*: closing B alone would have left the bar blind.

**D. The witness that was a constant.** *(aeon proposed, sigil's agent refuted.)* Offered as
a free witness that strict bodies ran: 66 `_port` hits in the strict log. Measured: **43 of
46 are cargo's own `Running tests/…` lines, printed before any test binary starts**, so the
flag cannot affect them and a flag-off run reproduces the count. Not a weak witness — a
quantity that **cannot vary with the thing it claims to measure**.

**E. The gate that reported its own success as somebody's silence.** *(sigil's, caught at
landing.)* The new skip-marker lint printed a census line that *quoted the marker*. The
nightly script greps the log for that marker and exits 2 on a hit. A **fully green** run of
the gate would therefore have taken the whole nightly lane dark, every night, starting the
first — the exact failure the gate exists to prevent. Visible only because C had been fixed
minutes earlier; under capture the line is swallowed and it ships invisibly.

## What actually distinguishes the three states

The instances that were *closed* were closed the same way, and it is not a better grep.

**Make the check's own evidence something the failure state cannot produce.**

- D failed this test and A/B/C failed it silently. `_port` hits, a green tick and an absent
  `skip:` line are all producible by a run that did nothing.
- The replacement passes it: `strict_bodies` counts strict-gated decision points reached
  **with the flag observed set**, written only on the branch that has already seen it. It is
  **structurally zero** when the flag is unset. Measured 29, matching an independent static
  count of 29 — two derivations over different enumeration parameters.

**Corollary — prefer an instrument that emits the POPULATION over one that greps an
artifact.** *(aeon's, from their pytest lane, where a fully green `-q` run prints no gate
names at all, making bar 25's own corrective (1) unrunnable there.)* Their remedy is
`pytest --collect-only`; the cargo equivalent is `cargo test --workspace -- --list` — 3954
ids in 2 s here, agreeing exactly with `git grep -c '#[test]'`. **A log grep can only ever
see what a passing run chose to print. Collection cannot quietly shrink without the diff
showing it.** This is the generator-enumeration rule (see the OFFCANON-ROT note: recompute
every value from the tool that writes it, rather than pattern-matching what a stale one
looks like) pointed at test existence instead of at constants.

## The practical test, for a check you are about to trust

1. **Name the failure state.** What is the world in which this check *should* go red?
2. **Ask what artifact that world produces.** If it is the same artifact the healthy world
   produces, the check is decorative. This is the whole thing.
3. **Ask whether the check can see its own subject at all** — capture, buffering, a matcher
   that never matched, a grep for a spelling nothing uses.
4. **Prefer a value the failure state cannot fabricate** over a value the healthy state
   happens to emit.
5. **Poison it in the failure direction, not the passing one.** Every instance above passes
   its happy-path test.

## Honest limits

One night, two lanes, five instances, several discussed between us before being written —
which is a shared frame by exactly the mechanism bar 19 names, and this document is one
lane's synthesis with a second's request behind it, not two independent findings. The
strongest evidence it is real is E: the shape closed on the gate written to close it, inside
an hour, and was caught only by an unrelated fix landing first.
