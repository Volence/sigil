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

## The surface neither grep reaches: PROSE

*(aeon's, 2026-08-27, from the sprite-owner freeze; accepted here and recorded as a joint
commitment rather than an agreement in mail.)*

Bar 14's remedy is to run **both** an identifier grep and a quoted-key grep and reconcile
them, because neither is a superset of the other. Prose is the population **neither one
reaches**: a bound asserted in a doc comment, a help string, a docstring, a refusal message,
a comment naming a length or an address. Nothing executes it, so **no gate can contradict
it** — and that is the whole difference in consequence. *A stale bound in code eventually
fails something. A stale bound in prose just teaches.*

Live instance, from the parcel that prompted this: `sprites_port.rs`'s own header asserted
the sprites region is *"same-LENGTH ($420 both)"*. The aeon parcel made that false, and the
same file's reference-window block twelve lines below already said `0x408` plain and `0x4DA`
debug — so the file contradicted itself, in prose, and had done since an earlier parcel.
Nothing could have caught it: there is no assertion to fail.

**The fix has two halves and the second is the one people skip.** Removing the stale number
is the first. The second is **not writing a fresh one** — re-authoring a hardcoded bound one
value later is the identical defect with a newer date, and it will rot on the same clock.
Point at where the number lives (`pins::SPRITES.{plain,debug}_len`) instead.

**A BOOKING IS PROSE, AND IT FAILS BY INVENTING A BLOCKER.** *(Both lanes hit this the
same morning, from opposite triggers; aeon's framing and this lane's, kept as a pair because
one trigger alone reads as carelessness and two read as a mechanism.)*

The prose surface above is mostly about **bounds** — a length, an address. Queue rows and
booked items are prose too, and they rot in a direction that is worse than a stale number:
they **invent constraints that no longer exist**, and a constraint nobody can execute is one
nobody can contradict.

- **sigil's trigger — the item LANDED.** `docs/OVERSEER.md` carried a row saying aeon's
  `parcel/rom-relayout` was `IN FLIGHT` for a full day after it landed, **twenty lines above
  the section in the same file recording the landing.** A dispatched agent read the row
  rather than the section and handed back a retired blocker as its reason for holding. The
  conclusion happened to be right; the reason was a year of prose out of date.
  **Rule: when an item lands, the row that BOOKED it is the one that has to move.**
- **aeon's trigger — the OWNER RULED.** A `DEFERRED_WORK.md` item said *"revisit only on a
  user ruling"* when the owner had already ruled.
  **Rule: when the owner rules, grep the bookings for the condition text that ruling
  discharges.**

Same defect, two triggers, and neither is found by re-reading the section that superseded it
— the superseding text is correct and reads correctly. **What has to be swept is the row
that stated the precondition**, and it is found by searching for the condition's own wording,
never by re-reading the resolution.

**The joint sweep, scoped deliberately.** When a parcel moves a set of values, each lane
sweeps its **own** tree's prose for **those specific values** — bounded by what actually
moved, rather than an open-ended audit, and each lane sweeping the tree it can actually
judge. Sigil takes the pins and lengths it writes terms for; aeon takes their `.emp` headers
and `docs/`.

**A second instance, found before the sweep had started, and it was load-bearing.**
`crates/sigil-harness/repin.toml`'s header states the tool *"resolves every entry against
BOTH aeon listings (`s4.lst` / `s4.debug.lst`)"*. It does not. `src/bin/repin.rs:68` calls
`native::sigil_native_symbol_listing`, which calls `resolve_canonical_sections` — it resolves
aeon **source**, and no `.lst` is opened anywhere in the generator. The sentence describes an
implementation that has been replaced.

**Why this one is not cosmetic.** Two lanes were about to treat the pin gate as an
independent check on the pin generator. It is not:

- `repin` (generator) → sigil's native resolver
- `repin_pins::pins_rs_is_current` (gate) → sigil's native resolver

The gate is generator-versus-file. It catches a hand-edited `pins.rs` and **cannot catch a
resolver that is wrong, because it asks the resolver.** That is bar 19 — same enumeration
parameter twice — sitting inside this repo's own pipeline, with the agreement guaranteed by
construction. The stale prose is what made it look like two instruments. The aeon lane's
`.lst`-derived measurement is the only outside instrument the pin chain has, which is a much
larger claim about it than "a helpful cross-check", and it was invisible while that sentence
stood.

**And an empty prose sweep is this note's own shape pointed at the sweep.** Nothing was found
and the grep never matched anything produce the same artifact, so **an empty result is
reportable only alongside the exact patterns grepped for.** Report the query, not just the
verdict; otherwise the sweep is instance (2) — a check that could not see its subject —
wearing a clean bill of health.

## When the SEARCH TERM is one of the values being swept for

*(2026-08-27, both lanes, on a process sweep — sigil made the error, aeon found the third
one it hid.)*

Bar 19 says name the parameter each derivation enumerated over and check they differ. This
is its cheapest and most invisible failure: **enumerating a population by a literal that is
itself a VARIABLE of that population.**

Instance. Three leaked shell loops were polling for a tool to exit. This lane enumerated them
with `pgrep -f "refreeze --attest"` and found **two**. The third polled
`refreeze --freeze` — a different subcommand of the same tool — so the sweep **structurally
could not see it**, and returned a clean, confident, plausible answer. The subcommand was
precisely the axis the population varied along, and it had been baked into the query.

The parameter that works is the one that does not name any instance: aeon enumerated the
**loop condition itself** out of `/proc/<pid>/cmdline` (`grep -o "until ![^;]*;"`), which
finds every waiter regardless of what it waits for. **Enumerate by the SHAPE of the thing,
never by the value one instance happens to carry.**

**The self-referential kicker, and it is why all three were immortal:** each loop's own
command line contains the pattern it polls for, so `pgrep` always matched at least itself and
the condition could never go false. They could not fire, and had been sleeping since 03:55.
A warning about a process that "will report against the wrong run" and one that "will never
report at all" want different responses, and only the second was true.

## A `/clear` retires the SESSION and keeps the PROCESS

Same episode, and it corrected an ownership model rather than a count. This lane reasoned:
the scratchpad path in those commands names a session that no longer exists, therefore the
spawning process is gone, therefore these are unowned orphans. **Wrong.** All three were
children of a **live** `claude` process — a peer's, still running. Clearing a session retires
the session and leaves the process, so earlier sessions leave children inside the *same*
process.

**The check is `/proc/<ppid>` and one's own shell ancestry, never a session id embedded in a
path.** A session id is a name; the parent pid is behaviour.

This lane declined to kill them on the boundary rule — do not reach into another lane's
processes on your own judgement — and that refusal was correct. But it was correct **for the
wrong reason**, and the reason is the part that would have travelled: acting on
"unowned orphans, dead parent" would have been acting on a false model that happened to
recommend the right move this once.

**What makes the by-PID rule more than ritual, measured in the same minute:** a real
`cargo test --workspace` belonging to a *third* lane was running at that moment. Any
pattern-shaped kill aimed at the waiters was one careless regex from taking that lane's
verification with it.

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
