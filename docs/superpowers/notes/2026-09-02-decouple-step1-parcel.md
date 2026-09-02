# SIGIL-DECOUPLE step 1 — the first parcel, named and anchored

Written 2026-09-02 at the hub's request, so the parcel can start the moment the aeon lane
reports item 5 merged and its freeze lifted. **Nothing here is started.**

## The order question, answered from the source rather than guessed

**Anchor:** aeon `822c382a`, `docs/DEFERRED_WORK.md:10243-10266` — verified reachable from
aeon `origin/master`, and it is the commit that *carries* the nomination (`--stat`: one file,
+25), not a later board update standing in for it.

The hub asked whether my reading puts step 3 before step 1. **It does not.** The plan says
**"Four steps, in order"** and lists them 1-4. Its only sequencing exception is the closing
line, verbatim:

> Sequencing: after the showcase lands; step 2 first.

Step 2 (placement authority comes home — aeon's re-layout) **has landed**. With that exception
discharged, the numbered order resumes, so **step 1 is next and step 3 follows it**.

**And the order is not merely nominal — there is a dependency that makes 3-before-1 unsafe.**
Step 3 is *"retire repin/pins.rs from the landing path"*. `pins.rs` is what guards placement
against goldens that, today, **are live aeon ROMs**. Retiring it while step 1 is undone removes
a guard while the coupling it guards still exists. Step 1 is precisely what makes step 3 safe:
once goldens describe a **pinned corpus** and drift detection has moved to a **non-blocking
nightly observer**, the landing path no longer depends on live aeon bytes, and repin has nothing
left to guard there. Doing 3 first would delete the instrument and keep the exposure.

## The first parcel

Step 1 as written has two halves; this parcel is the first half plus the observer, because the
observer is what makes the cutover non-blocking rather than a leap.

**(a) Vendor a pinned aeon source snapshot as the test corpus.** Goldens describe *that corpus*,
bumped on sigil's cadence rather than on aeon's landings.

**(b) Move drift detection to the nightly job, as a NON-BLOCKING observer.** Plan's own words:
*"builds aeon master with sigil master against CRCs aeon commits for itself (aeon-owned expected
values; detection without blocking)."* The expected values are **aeon's to own** — this lane
consumes them and must not author them, or the observer becomes a second frozen table with a
new name.

## What already exists here, so the parcel does not re-derive it

- **The eight constraints the frozen tables enforce without declaring** —
  `docs/superpowers/notes/2026-08-26-placement-constraint-inventory.md`, rechecked in
  `2026-08-27-constraint-recheck.md`. This is the groundwork the board row refers to.
- **The drift harness is built and its record seam is deliberately empty** — see `OVERSEER.md`.
  `scripts/nightly_ref_drift.sh`, `scripts/drift_report.py` (verbs `observe`, `report`,
  `selftest`; `cmd_selftest` builds its states in memory and patches no file).
- **`DRIFT-TIMER-NOT-INSTALLED` is a live prerequisite of (b)**, and it is on the board: the
  units are committed but a `systemd --user` timer lives outside every repo, so nothing installs
  it. The observer accumulates nothing until `systemctl --user enable --now
  sigil-ref-drift.timer` is run once by hand.

## Sequencing and the one hard gate

**Serialized behind the aeon lane's item-5 chain.** Step 1 ends the paired freezes, and that
chain is *using* paired freezes right now. Starting before it closes would pull the mechanism out
from under a live measurement — the same shape as relinking a shared binary mid-run.

**The prerequisite that is not negotiable, carried from the plan's own step-2 warning and true
here too:** *every constraint the frozen tables encode must be recaptured as an explicit rule
BEFORE the tables stop being authority, or it silently stops being enforced.* For step 1 the
analogue is: **the pinned corpus must be shown to exercise what the live corpus exercised**,
enumerated, before the live goldens stop being the reference. A corpus that is merely smaller and
green is the always-green trap with a new coat on.

## Accepted cost, restated so it is not rediscovered as a surprise

From the plan, agreed with the owner before his yes: **assembler regressions surface nightly
rather than at the next aeon landing.** That is a real loss of immediacy, deliberately taken in
exchange for content authoring not being taxed by a byte gate on every aeon change.
