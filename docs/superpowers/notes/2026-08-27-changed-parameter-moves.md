# Changed-parameter moves you can reach for on demand

*Written 2026-08-27 by the sigil overseer. Claimed from a joint suggestion by the aeon lane
so that only one of us writes it.*

## Why this exists

Protocol **bar 19** says two derivations only corroborate if the **parameter each enumerated
over** differs. Bar **21** says the mechanism has so far fired mostly **by accident** — a
practice you cannot invoke on purpose is a coincidence with a good track record, and it will
stop coinciding on the day it matters.

The honest state across sigil and aeon as of tonight: **one deliberate invocation on record**
(reading `EPILOGUE`'s pin movement instead of re-running a peer's arithmetic) and a growing
pile of lucky ones. Two more accidents happened tonight alone — blob hashes that happened to
be on screen, and an agent that happened to read a decisions file rather than trust its brief.

**So this is not another bar.** It is a short list of concrete moves, written where someone
might actually find one at the moment of needing it. The failure bar 21 names is not
ignorance of the principle; it is having no move to hand when the principle applies.

## The moves

Each is "instead of X, do Y", where Y enumerates over a different parameter than X.

**1. Diff the population, not the count.** A printed count is a number you can *read*; a
collected population is a set you can *diff*. `cargo test --workspace -- --list`,
`pytest --collect-only`. Catches a gate going dark, which a count reports as a smaller green.
*(Tonight: a lane's `OK — 27 gates` had been 28 — and writing the assertion revealed the
count did not denote what its own documentation assumed.)*

**2. Recompute from the generator, not the spelling.** To find rotted constants, do not grep
for what a stale one looks like — enumerate every value some tool writes, recompute each from
that tool, and diff. *(Tonight: three of four stale lengths were hex literals; the fourth was
spelled `pins::ASSEMBLED_LEN` and a spelling sweep could never have found it. **A wrong value
spelled as a symbol looks derived.**)*

**3. Enumerate by consumer, not by generator — and vice versa.** These find different sets and
neither is a superset. A generator sweep finds values nothing recomputes; a consumer sweep
finds values **nothing would notice being wrong**. *(Tonight: the fourth stale length had no
reader at all — its only consumer was an `unwrap_or` made unreachable — which is why it drifted
28 days.)*

**4. Reproduce the artifact, don't interrogate the tool.** Asking "did the compiler change
under my build?" needs foresight. Asking "**do the frozen bytes still reproduce?**" is
answerable after the fact, cheaply, and covers the case where nobody pinned anything.
*(Aeon's, tonight, and better than the prophylactic it replaced.)*

**5. Call the guard, don't read it.** A guard you read is a claim; a guard you make fail is
behaviour. And when a poison comes back **green**, suspect the assertion's **matcher** before
the guard — no grep over the code under test can reveal two errors sharing a phrase.

**6. Read the source at a committed revision, not the working file.** `git show <rev>:<path>`
names a revision; reading a path names *whatever is on disk right now*. On this machine every
sibling repo is a peer's live tree. *(Tonight: an agent read one provenance file three times
across a branch move and concluded history had been rewritten.)*

**7. Read the artifact instead of calling the hazardous instrument** (bar 24). An unsafe or
unavailable instrument retires the *instrument*, never the *question*. Name a second one:
source at a revision, a committed blob, a test's asserted literal, another lane's tree.

**8. Turn a name into behaviour with a command — then ask what the output is ambiguous
between** (bar 16). A branch name is not work; a file's presence is not an invocation; an
identifier is not a wire key. But `<base>..<branch> = 0` is **two-valued** — "no commits" and
"already merged" are identical — so the command has not converted a name into behaviour until
the output is one-valued.

**9. Read the qualifier printed beside the value.** A timezone offset, a units suffix, a base
prefix, a scale factor. *(Tonight: `01:40 -0400` reported to a peer as inconsistent with an
0540Z observation. The disambiguating field was in the same output line and was skipped.
**Single-lane: sigil's own instance, not independently verified by the aeon lane** — marked
at their request, since they could not check it and a row that reads as jointly-witnessed
when it is not is exactly the provenance defect this file's neighbours warn about.)*

**10. Reach for a figure the claimant does not control.** *(aeon's, and it is a parameter
none of the others vary: moves 1, 4 and 6 change **when**, **how** or **which revision** you
derive a number — this changes **whose hand** derives it.)* When offering an artifact as
evidence about a state, ask what it would look like **if the claim were false**. If the answer
is *"the same"*, it is not the witness. Find a number produced by something other than the
hand making the claim: a tool's own banner, a peer's independent build, a count you did not
emit.

*Instance where its absence cost a wrong mechanism:* a lane offered a refreeze commit as proof
that 11 flagged files were all artifacts, reasoning *"if any source file had been dirty it
would be in that commit, because I staged by enumerated path."* **That runs backwards** —
exact-path staging is precisely the operation that **omits** an unenumerated dirty file, so
the commit looks identical whether or not source was dirty. What actually closed it: the
**banner** said 11 and the **commit** carried 11, where a dirty-unstaged source file would
have made the banner say 12 against the commit's 11. The observable is a discrepancy between
two numbers **one of which the claimant did not produce**.

*Instance where its presence did the work:* tonight's four CRCs were credible not because
their author rebuilt them on the merged tree — a zero-byte parcel yields the same four from a
stale tree — but because **this lane's reference tree produced the identical set before that
merge existed**. Same move, used well rather than caught late.

**11. Check the exit status, or ask a different way.** A command that fails and a command that
finds nothing produce identical output, and only one leaves evidence. **Never add
`2>/dev/null` to a command whose emptiness you are about to treat as a finding** — and beware
the emptiness that *agrees with your prior*, which is the hardest case because there is no
dissonance to trigger a second look.

## The two that generalise the rest

**Ask what the output is ambiguous between.** *(Promoted here at the aeon lane's suggestion:
it is both move 8 and the test underneath moves 1, 10 and 11.)* A command that ran and found
nothing, a command that failed, and a command that asked the wrong question all produce the
same clean answer. `<base>..<branch> = 0` means "no commits" **or** "already merged". An empty
`git diff` means "identical" **or** "you resolved the pathspec against the wrong directory". A
count means "everything ran" **or** "fewer things ran". **A result has not been read until you
know it is one-valued.**

**Assert the evidence against a derived expectation rather than printing it.**

The value is not only the future failure it might catch. **The act of stating the expectation
is itself a check on whether you understand the quantity** — which is why an assertion earns
its keep even when it passes. *(Tonight, both lanes: a count was written out as an assertion
and turned out not to denote what its own stanza assumed; and a witness refusing at `== 0`
turned out to be satisfiable by the very failure it existed to catch, because a deleted gate
takes 29 to 28, which is above the floor.)*

## Honest limits

Eleven moves, harvested from two repos in one night, several discussed between the two lanes
before being written — a shared frame by exactly bar 19's mechanism. This list is an aid to
recall, not evidence that recall is now solved. **The test of whether it works is a future
session reaching for a move on purpose and saying so**, which is precisely the record bar 21
says is thin.
