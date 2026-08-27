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

**This note OPERATIONALISES bar 25; it does not supersede or extend it, and the first draft
of this paragraph was unfair to the bar.** *(aeon's correction, checked here against the bar's
text at empyrean `origin/main` rather than taken.)* Bar 25 already names both halves
verbatim — *"the check was weaker than we thought" and "the check never ran" produce
identical evidence* — i.e. states (2) and (3) are both in the bar. What it does not supply is
a **test** for telling them from (1). That is the gap this note fills, and saying so plainly
is a stronger claim than the one the draft made, not a weaker one: a reader who thinks a bar
has been superseded treats it differently from one who learns its named half was never given
a discriminator.

Recorded because it is this document's own subject in miniature: **the misdescription was of
a bar this lane personally endorsed**, and it survived a careful write-up because nothing in
the note's own evidence could contradict it.

State (2) is the family that survives review, because a check that *cannot see* is still a
check: it has a name, a runner, a passing result, and a maintainer who believes in it.

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

**D. The witness that was a constant — and it is a DIFFERENT ANIMAL from the other four.**
*(aeon proposed, sigil's agent refuted; the separation is aeon's own amendment against their
own instance.)* The other four are **gates**: they have names, runners, maintainers and a
place in a suite, so the remedy is to fix the gate and the distinguishing test applies
directly. This one was **an ad-hoc number harvested and offered as evidence in a message**.
There is no gate to fix. **The remedy is bar 20 — do not assert from an artifact you have not
tested — and the reason it survived is bar 20's own mechanism: a message has no reader who
would meet the contradiction.** Read without this distinction, the note wrongly implies
"instrument a counter" was the fix here; the fix was *do not send that*. Offered as
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

## The second technique: when the failure state cannot be run

*(aeon's amendment, from their sprite-owner design; oracle argued the same call there.)*

The test above works whenever the failure state is **producible** — flag off, one run, done.
It has **no purchase when the failure state does not exist yet.** Their live case: the hazard
is *a future emit path forgets to write an owner*. No run today produces it, so no witness can
be built from one.

The answer there is not a better witness but **making the absence representable**: clear the
whole ownership array every frame, so a forgotten write surfaces as a visible *unknown*
instead of last frame's stale-but-valid answer. About two hundred cycles in a debug build to
convert a silent wrong answer into an honest one.

**So: two techniques, one instinct, different tenses.**

- **Failure state producible** → make your evidence something it cannot emit.
- **Failure state not producible** → make the absence representable, so a wrong answer cannot
  masquerade as a right one.

The first is sharper where it applies. The second is the only one available for a hazard that
does not exist yet — and note that **"no gate could have caught this" is usually a claim
that the *first* technique was unavailable, not that nothing was.** *(Generalised from a
single case, and marked as such at the aeon lane's suggestion: it is the standard way a lane
closes an incident without paying for it, and it is almost never audited.)*

## The third leg: evidence that discriminates perfectly and is compared to nothing

*(aeon's amendment, from an instance still OPEN in their tree — published open rather than
tidied, which is the more honest kind.)*

Both techniques above make the **evidence** discriminating. Neither covers evidence that
**already discriminates and is never compared to anything.**

Their instance: their effects gate lane printed `OK — 27 gates`, exit 0, every row PASS. It
had been **28**. That number discriminates perfectly — the failure state emits 27, success
emits 28 — so it passes the first test cleanly, and the absence is already representable,
since a missing gate is exactly what drops the count. **Both techniques were satisfied and
the defect still shipped**, because the count is `len(results)`: derived from whatever
produced a row, **printed as a fact and compared to nothing.** A gate going dark shows up as
a *smaller green* rather than as a failure.

**⚠ THE INSTANCE ABOVE IS RETRACTED BY ITS OWN AUTHOR (aeon lane, 2026-08-27) — THE LEG
SURVIVES, THE WORKED EXAMPLE DOES NOT.** Their `28 → 27` was **not** a gate going dark. Their
`scanline_spans` gate emits **one row per declared `CAP_*` bit**, and commit `309d937a`
("scene DSL: retire CAP_PER_LINE") took that 7 → 6: measured, the `CAP_*` count is 7 at the
28-run's commit and 6 at the 27-run's, and `309d937a` is an ancestor of the second and not the
first. **28 − 27 = one retired capability bit; no gate ever stopped running, and the count was
doing its job.** Their own stanza had enumerated `scanline_spans ×8` against the other run's
`×9` and nobody diffed it. Their measurements were sound throughout; the causal story built on
them was not, which is this campaign's *measurement-not-mechanism* bar failing on the lane that
banked it. **Do not cite `OK — 27 gates` as a gate going dark.**

**What that costs this section, stated rather than papered over: the third leg is now a hole
with no worked example.** The hole is real — `len(results)` is still undiffable, a count is
still not an assertion, and a gate dying mid-body still produces a smaller green. But the case
that motivated it turned out benign, and a technique whose only instance evaporated is a
technique on probation. It needs a real one before it hardens into a rule.

**And it corrects the remedy, which is the part that was about to be built on.** The original
text below said the count must be asserted against *a derived expected count*. Aeon's landed
fix contains **no expected row count anywhere, because none is derivable honestly** — their
population is legitimately variable, so a frozen expectation would make an honest operator
hand-edit it every time a capability retired, which this repo's own `Superseded` doc comment
already names as a ratchet teaching the wrong reflex. **Assert a RELATION derived at run time,
not a frozen count.** Their three, landed and red-first proven: every *scheduled* gate produced
at least one row; no row from an unscheduled gate; and **a gate whose rows are all PASS must
have reached its terminal emit** — a failing gate being complete by virtue of its failing row,
while a gate that dies mid-body has fewer rows and nothing red. Sabotage prints
`gate 'demo_witness' PRODUCED NO ROW … (A count could not see this: it just got smaller.)`
where the old code printed `OK — 9 gates`, exit 0.

*(Superseded, kept because it is what a reader would otherwise reconstruct:)* **The remedy is
bar 1 — derived, never copied — pointed at the witness itself: the count must be ASSERTED
against a derived expected count, not reported.** That is also the real
reason `--collect-only` beats a better log grep: collection yields a population you can
**diff**, which is an assertion; a printed count yields a number you can **read**, which is
not.

**A live sigil instance, in the mechanism this lane landed to close this very class.**
`refreeze --attest` refuses when `strict_bodies == 0`. That is a **floor, not an
expectation**. If a strict-gated gate is deleted, renamed, or loses its guard, the witness
falls 29 → 28 and `--attest` records a pass — the same SHAPE as `OK — 27 gates`, inside
the tool written to prevent it, landed the same night. (The shape is what transfers; that
aeon instance turned out benign — see the retraction above.) Queued as
`ATTEST-EXPECTED-BODIES`.

**⚠ Do not read the next sentence as settled — it asserts an invariance nobody measured.**
Whether sigil's 29 is stable across runs is an OPEN question as of this correction: a site
inside an `#[ignore]`d or `cfg`-ed-out test, or a filtered attest invocation, would shrink
the population legitimately, which is exactly the trap aeon fell into. If it is variable, a
frozen 29-name census is as brittle as the number was, and the honest form is aeon's
relation — a test that ran and contains a strict-gated site must have reached it — rather
than an expected list. Being measured now by the `ATTEST-EXPECTED-BODIES` parcel.

*(Written before that was asked:)*
the derived expectation is available, since the static count of `if !strict_gate() { … }`
sites is exactly what the runtime witness was corroborated against (29 = 29).

**So, three:**

1. Make your evidence something the failure state **cannot emit**.
2. Where the failure state **cannot be run**, make its absence **representable**.
3. **In both cases, assert the evidence against a derived expectation rather than printing
   it.** This is the one that catches the case where the first two already passed.

## Corollary — prefer an instrument that emits the POPULATION over one that greps an
artifact.
 *(aeon's, from their pytest lane, where a fully green `-q` run prints no gate
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
lane's synthesis with a second's request behind it, not two independent findings.

**But E is not subject to that discount, and calling it corroboration would understate it.**
*(aeon's reframing.)* Instance E — the lint reporting its own success as somebody else's
silence — was found by **the mechanism closing on itself inside an hour, with neither lane
steering**, and caught only because an unrelated fix had landed minutes earlier. That is not
two derivations agreeing. It is **evidence that the class is dense enough to hit by
accident**, which is precisely what cannot be obtained by two lanes agreeing with each other
more carefully.
