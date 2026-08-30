# Sigil Overseer

How a Claude session runs **sigil** as its overseer. The shared role — solo-first
posture, delegation discipline, review bars, peer protocol — lives in
`empyrean/docs/OVERSEER-PROTOCOL.md`; read it once, then this file for what is
sigil-specific: the landing-lane division, the worktree/test quirks, and the queue.

## NO ACTIVE HOLD — read this before running anything that builds

**A hold that lives only in a chat message does not survive a `/clear`** *(aurora's)*. An
announcement reaches the sessions that exist; only a committed artifact reaches the ones that
do not exist yet. **A committed hold has the opposite failure: it outlives its reason and
nothing announces that either.** So every row carries its date, who to ask, and what ends it —
a successor EVALUATES it rather than obeying it, and a row that cannot be evaluated is expired.

| Raised | Artifact | Why | Ends when | Ask |
|---|---|---|---|---|
| *(none)* | — | — | — | — |

**LIFTED 2026-08-29 by the aeon lane — and chain 181 will RE-ARM it, on a fresh grant, not
this one.** They said so explicitly rather than letting the old row carry, which is the
standing-permission-expires rule working in the right direction for once.

**THE CHAIN-180 FALSIFIER DISCHARGED — verified here, not taken.** Both lanes pre-agreed that
BALL-SEATING would move byte values and no pin. It held: `pins.rs` is untouched in the freeze
commit `fa0e6540` (checked by `git show --stat` on that path, not by their report), both ROM
sizes are unchanged (719205 / 735818), and `repin --check` reported `pins.rs unchanged`.
**Record it as a DISCHARGED prediction rather than a green run** — a pre-agreed prediction that
comes true is worth something only if it is written down that it was agreed first.

**A BETTER FREEZE WITNESS THAN `pins.rs unchanged`, found by the aeon lane in one of THIS
repo's own artifacts.** `pins.rs unchanged` is an **absence**: it is equally consistent with a
correct length-neutral parcel and with a build that never ran. `golden/offcanonical_sizes/s4.txt`
is a **positive** witness — on chain 180 exactly two lines moved, both CRC headers
(`golden_crc32`, `assembled_anchor`), while `assembled_end=0xa5c90` and every label held.
Verified here on `fa0e6540`. A table of unmoved labels beside two changed CRCs cannot be
produced by a build that did not run. **Prefer it to the pin file for length-neutral parcels.**

**Its two limits, stated by the aeon lane while it was fresh so it is not adopted wider than
it earns.** (1) It witnesses that a build **ran and produced these labels**; it is **silent on
which source that build used**, so it **composes with the assembler md5 rather than replacing
it**. (2) It is a positive witness **only for a length-neutral parcel**. On chain 181 — debug
+4 bytes, 282 symbols sliding +16 — that table moving is *expected*, and it reverts to
something to reconcile rather than something that proves anything.

### AEON-REF-DRIFT-NIGHTLY — THE HARNESS IS BUILT AND THE RECORD SEAM IS DELIBERATELY EMPTY (2026-08-30)

SIGIL-DECOUPLE step 1's job exists: `scripts/nightly_ref_drift.sh`, fired by
`scripts/systemd/sigil-ref-drift.timer` at 07:17, with `scripts/drift_report.py` (the state
machine), `scripts/drift_paths_sweep.py` (the reference-path sweep) and
`scripts/drift-nightly.conf` (N). `docs/DRIFT_RECORD_SEAM.md` is the contract with the aeon
lane. `crates/sigil-cli/tests/drift_nightly_harness.rs` is what makes the workspace suite run
the two selftests and assert the lane's structural properties.

**IT MEASURES NOTHING YET, ON PURPOSE.** `DRIFT_RECORD_READER` is empty until the aeon lane
lands its committed CRCs, so every run reports `STATUS: NOTHING MEASURED` and exits 2. That is
the honest state and it is not a pass: no chain is credited, N does not move, and the count
lines print `— nothing measured` rather than `0`. **Do not seed a record from a sigil build to
make the lane green.** A drift job whose expectations it generated itself is the vacuous gate
this project exists to retire, and the harness has no path to one — every expectation enters
through the reader command and nowhere else.

**A CHAIN IS A DISTINCT REVISION PAIR, NOT A NIGHT.** The provenance chain counts one entry per
paired landing (186 today), which is the unit the N ruling was about. Five nights on one pair
are one chain's worth of evidence, so the report keys on the pair and a repeated night adds
nothing. Derived from the chain's own definition rather than chosen.

**THE REPORT REFUSES TO RENDER QUIET AS A VERDICT, AND THAT IS THE DELIVERABLE.** `N reached,
quiet` and `N reached, verdict` are separate states. A red settles the question in ONE
observation and N is irrelevant to it; quiet accumulates and never concludes. The refusal is
checkable rather than habitual: `FORBIDDEN_ON_QUIET` lists the readings a person would
otherwise take away from a page of clean rows, no quiet rendering may contain any of them, and
the scan is whitespace-normalised so a line break cannot smuggle one past.

**THIS CONTRADICTS ONE SENTENCE OF AEON'S PLAN, AND THE PLAN'S OWN PARAGRAPH IS THE ARGUMENT.**
Their §3 bullet reads *"Engine stays byte-clean while nothing blocks on it → the gate was
spent, and step 4 says so with evidence."* Two sentences earlier the same section says the two
hypotheses *"reads as the gate is spent"* and *"reads equally as the gate is why the engine is
clean"* cannot be distinguished from inside. Removing the blocking removes one mechanism by
which the gate could be causing the cleanliness, so quiet-under-non-blocking is **better**
evidence than quiet-under-blocking — but it is not self-executing, because the deterrent half
of the gate survives: the paired freeze still runs at every chain landing and people still
know a divergence would be found. **The tool therefore presents the observation and its
confounder and leaves the weighing to the owner.** That is a design call, not a re-ruling: if
he wants the tool to draw the conclusion at N, that is one edit to the quiet branch.

**N LIVES IN CONFIG AND HAS NO DEFAULT ANYWHERE.** `DRIFT_CHAIN_TARGET_N=5` in
`scripts/drift-nightly.conf`, overridable for one run with `SIGIL_DRIFT_N`, and the report
always names which of the two the value came from. `drift_report.py report` requires `--n` and
the shell refuses to run when the config does not set it — proven by making each refusal fire.
Overturning N costs an edit, not a parcel.

**A SUB-DECISION THE N RULING DID NOT SETTLE, AND THE REPORT SHOWS BOTH RATHER THAN PICKING.**
A chain in which the ASSEMBLER did not move says nothing about the assembler. Five aeon-only
chains would reach N while carrying no evidence about the question step 4 asks. So the report
counts `quiet` and `quiet AND evidence-bearing` separately, evaluates N against the second, and
renders `N REACHED ON THE WEAK POPULATION ONLY` as its own state when the two disagree. **Which
population N counts is the owner's to say.**

**THE FOUR CASES, AND THE ONE CONSTRAINT THEY PUT ON AEON'S FORMAT.** Exact pair hit with
different bytes is the unambiguous defect. A pair miss with an engine-revision hit is the
assembler moving bytes under identical engine source — **the red step 4 actually needs**. An
engine-revision miss is unverified, because no expectation existed. Both missing is
unattributable and the job says so instead of picking. **This only works if the record does not
mint an entry for a pair from the build that pair is about to be judged against**; if it did,
case 2 collapses into a self-authored expectation and assembler drift becomes invisible. That
is the one property of their format this job cannot check for itself.

**THE KEY IS THE ASSEMBLER, NOT SIGIL'S HEAD.** `SIGIL_BUILD`/`SIGIL_EMIT` come from the
environment and `SIGIL_EMIT` writes `engine/sound/generated`, so a clean tracked tree at a fixed
engine revision can build a different ROM with no cause visible in the tree. The job asks
`sigil --version` and records three things: `revision` (linked), `closure-revision` (the last
commit touching what cargo compiles this binary from), and the tree state. **`closure-revision`
is the better key component** — `revision` moves on every commit including ones no compilation
can see, so keying on it manufactures case-2 misses that carry no evidence. A `dirty` tree
makes the key non-identifying and the job never advances N on one.

**THE 192-PATH SWEEP IS BUILT AND THE MEASURED NUMBER IS 115 AT 425 SITES.** Across the 139
sigil sources that read the reference tree, 115 distinct paths are named, every one of them
resolves at the engine lane's live tip today, and the one path asserted ABSENT still is. The
`extra_entry.rs` narrow version stays where it is: this sweep is a `stat` check and cannot tell
you a fixture still fires the guard it used to. Which files are swept is EXTRACTED from
`nightly_source_gates.sh` rather than retyped, because that script already classifies files by
the same question. Sweeping every source instead costs precision that matters — 8 misses of
which 7 are synthetic fixtures a test writes into its own temp tree.

**TWO THINGS FOUND WHILE BUILDING IT, BOTH WORTH MORE THAN THE HARNESS.**

1. **`nightly_source_gates.sh`'s classifier matches `--aeon` as a SUBSTRING.** Any test file
   naming a `--aeon…`-prefixed flag — in code OR in a comment — is classified as
   reference-reading, found in neither the gate list nor the artifact bucket, and **the whole
   source-gate lane then refuses to run**. This gate reads only sigil's own scripts and would
   have darkened that lane over a flag spelling; it now assembles the flag rather than writing
   it. The classifier is one spelling axis wide in the other direction too, and this is the
   first case of it being over-wide rather than under-wide.
2. **`provision-aeon-ref.sh` hard-refused at any revision but the pinned one.** Its step-6
   rebuild control asserts a built ROM matches the frozen golden — a real control at the pinned
   revision and a guaranteed failure at a live tip, where the goldens describe other source.
   The drift job provisions at the live tip by definition. The control is now **derived from the
   revision** rather than gated by a flag: `required` when the revision is the pinned one,
   `not-applicable` otherwise, with the CRCs printed as data. A flag would have been an opt-out
   somebody could aim at the pinned case.

**WHY IT IS A SECOND SCRIPT AND NOT A SECTION OF THE SOURCE-GATE LANE.** That lane's checkout is
source-only by construction: it deletes `*.bin` and `*.lst` out of its reference tree on every
run, and its own header warns that an artifact-dependent run sharing that tree loses its ROMs
mid-suite and reads as ~127 golden mismatches. This job builds ROMs and compares CRCs. Two
lanes, two checkouts, two cadences — 07:17 here, two hours behind the gate lane, because this
one compiles the assembler and both ROM shapes and takes worktree locks in both repositories.

**THE RUNNER, HONESTLY.** The workspace suite runs the two selftests and the structural
assertions every landing (`drift_nightly_harness.rs`). The JOB itself is operator-run until
somebody installs the timer: the units are committed, `scripts/systemd/README.md` carries the
three-command install, and **nothing installs them automatically** — a `systemd --user` unit
lives outside every repo. Until `systemctl --user enable --now sigil-ref-drift.timer` is run,
this lane accumulates nothing.

**THE LEDGER IS MACHINE-LOCAL.** It lives under `$XDG_STATE_HOME/sigil-ref-drift/`, outside
every repo, so the accumulated evidence does not survive this machine — and the report says so
on every run rather than letting a reader assume durability. If step 4's answer has to be
defensible months from now, the ledger wants a committed home; that is a decision, not an
oversight, and it is unmade.

### THE CORRECTION TO THAT SECTION: NOTICING A LUCKY PASS WAS ITSELF A LUCKY CATCH (2026-08-30, aeon's, about their own disclosure)

**The section below credits aeon with disclosing a near-miss they could have banked as a save. They
have corrected that credit, and the correction is sharper than the thing it corrects.** Their words:
*"my near-miss was not modesty. I noticed it only because I went back to check what my own check had
actually asked, and I have no control that makes me do that; it happened this once. So treat the
disclosure as evidence about that one instance, not as evidence that this lane reliably catches its
own lucky passes."*

**This is `fine ≠ protected` applied to itself, one level up, and I had missed it.** I banked their
disclosure as though it demonstrated a property of the lane. It demonstrates a property of one
occasion. A lane that caught its own lucky pass once has shown that catching is *possible*, not that
it is *reliable* — and the difference matters exactly when someone later reasons "aeon would have
caught it."

**What this does to the bar I filed: the durable half is the two-command control, not anybody's
noticing.** A control that must be *remembered* at the moment of a near-miss is not a control; it is
a disposition, and dispositions are precisely what fail under time pressure at 5am. Aeon says so
about their own, and they are right to insist the record not credit them with more.

**The general form, which is the one to carry:** when a lane reports its own near-miss, ask whether
a mechanism produced the report or an accident did. Both are worth having and only one is worth
*planning around*. Crediting the second as the first manufactures a safety net out of a good mood.

### THE DRIFT KEYING DEFECT NOW HAS AN INSTANCE WITH A COUNT (2026-08-30, aeon's measurement)

`DRIFT-MISS-MUST-NAME-ITSELF` was ruled off **one** measured pair-move. It now has a live instance:
aeon master moved `3f143178 -> bfdd28e6` in **four consecutive docs-only commits** — verified here,
five files, all under `docs/`, **zero non-docs** — so the ROM never moved and the job has **four
consecutive misses on a tree that did not change**.

**Aeon's framing is the one to keep: this is the failure mode reading as HEALTH, not as an outage.**
Four misses in a row is what a watch looks like when it is running and structurally unable to
advance, and nothing distinguishes it from a watch with nothing to say. That is precisely the
ambiguity the ruling kills, and the ruling is now measured rather than hypothetical.

**The conservative default stays right and neither lane is moving it.** A check whose whole job is to
resist counting weak evidence should err toward missing. What changes is only that the silence is no
longer free.

### "WE WERE FINE" AND "WE WERE PROTECTED" ARE DIFFERENT CLAIMS (2026-08-30, aeon's, about their own near-miss)

**No freeze window was burned by my false readiness claim — and aeon refused to bank that as a
save.** Half an hour before the retraction they had already decided not to ride R7 on item 1's
byte-mover, for an **attribution** reason: this lane's own standing ruling that R7 gets its own
freeze range, because mixing a large blast radius with a `+38`-byte parcel makes the two
indistinguishable in the goldens. So the sequencing was already separate.

**Their own words for why that is not a defence:** *"that reasoning would have survived intact while
still assuming the parcel existed, and I would have opened a window for it next. The correct check
and the check I ran were different checks; mine happened to route around the failure."*

**Bank the distinction, because the near-miss is the moment a control gets credited for free.** A
good outcome produced by a check aimed at a different question is **luck with a respectable
alibi** — and it is more dangerous than a plain miss, because it leaves a control looking validated.
The test: *would this check have fired if the failure had arrived by a different route?* Here it
would not have; it separated the freeze ranges and would have opened a window for a parcel that did
not exist.

### THE RECEIVING HALF: RESTATE A PEER'S CLAIM ABOUT THEIR OWN REPO AS ATTRIBUTED (2026-08-30, aeon's)

The asymmetry banked above — *a lane's claims about its own repo are the ones its peers can least
check and it is least likely to check itself* — names a sending-side failure and gives the sender a
control. Aeon supplied the half they own, and it is the practical one:

**The receiving discipline is NOT "audit your peer's tree", which no lane can do. It is: restate a
peer's claim about their own repo as ATTRIBUTED, never as fact.** Their board said *"Sigil's
alignment flip is ready and waits on my sequencing"* — in their own voice. Had it read *"sigil
reports R7 ready (their claim, unverified here)"*, the hub would have planned off it identically,
**but the staleness would have been visible the moment anyone looked.**

**One clause, and it is the half the receiver controls.** The sender cannot fix this from their end
— they are the party who has stopped looking. A restatement in the receiver's own voice launders an
unverified claim into a second apparent source; the same sentence with four words of attribution
stays exactly as strong as its origin and no stronger.

### "THE ALIGNMENT FLIP IS READY" IS FALSE, AND IT HAD REACHED TWO OTHER LANES' PLANS (2026-08-30)

**Caught because the hub asked me to tell aeon when the flip was ready to ride their next
byte-mover — which made me check a claim I had been restating for days.**

**The true state, read out of the source rather than the board.** `crates/sigil-harness/src/
section_align.rs` describes the flip under a heading that says **`── AFTER THE FLIP ──`**, in the
future tense: *"`required` becomes the packer's input: `align_up(running, required_for(section))`
replaces `align_up(running, packed_align_of(prov))` … That WILL move bytes — most sections require 2
and are being handed 16 today."* And `native.rs`'s live packer still documents its own rule as
*"`align_up(running, A)` with A = the largest power of two ≤ 16 dividing prov."*

**So what is ready is the PRECONDITION, not the parcel.** The declaration table and its gate landed
(`section_align.rs`, `section_alignment_declared.rs`, branch `parcel/declare-section-alignment` —
which is **0 commits ahead of master**, i.e. merged 75 commits ago). The byte-moving switch **is not
written.** A reader of "the flip is ready and waits on sequencing" concludes a parcel exists to
land. None does.

**The propagation is the finding, not the error.** This lane asserted it; **aeon's board restated it
as "Sigil's alignment flip is ready and waits on my sequencing"**; the hub then planned off it and
told me to hand it to aeon's item-1 build as the pairing byte-mover. **One unverified claim about my
own repo, in three lanes' plans**, and the cost lands on a peer: aeon opens a paired freeze window
for a parcel that does not exist, and a byte-mover slot burns.

**This is `own-repo-state-asserted-from-memory` at n=4, and the widening is where it travels.** The
first three instances were self-contained — a wrong rebase order, a claim my own lane-log refuted, a
stale published queue row. This one **left the lane**. Verification discipline points outward by
default: I check peers' SHAs, peers' mechanisms, peers' counts, and I check them well. My own
board's claims about my own tree arrive as CONTEXT rather than as claims, so nothing in the habit
fires on them — and a peer has no way to audit my tree, so they restate it faithfully and it
hardens.

**The cheap control, and it is the one I already apply to peers:** a readiness claim about your own
repo is a claim, so **name the artifact that would be landed and confirm it exists.** `git log
--oneline <branch>` and `rev-list --count` answered this in one command. A branch that is *behind
and zero ahead* is merged, not pending — and that is a spelling of "done" that reads exactly like a
spelling of "ready".

### A BASELINE CAN MOVE UNDER AN IN-FLIGHT AGENT, AND ITS REPORT WILL BE SELF-CONSISTENT AND STALE (2026-08-30)

**Live instance, caught before it landed.** An agent was dispatched with the four golden CRCs
written into its brief as *"the current goldens, which you should match"*, and told to report
BLOCKED if the bytes moved. While it worked, the aeon lane froze chain 189 and **two of those four
goldens moved** (`s4 6e2f9b22 -> 63451f96`, `s4.debug 6516fc68 -> 3aa7cb12`).

**Nothing about the agent's run becomes wrong, and that is exactly the hazard.** Its worktree
branched before the freeze, `provision-aeon-ref.sh` defaults to the aeon revision pinned by the LAST
entry of ITS `provenance.toml` — the pre-freeze one — so it builds the old revision, compares against
the old goldens, and correctly reports a match. **Self-consistent, correctly derived, and citing
numbers that are no longer current.** There is no failing check anywhere in that chain: not in the
agent, not in the wrapper, not in the goldens.

**So byte-neutrality proven against a baseline is a claim about THAT baseline, not about master.**
The parcel changes no bytes, so it *should* reproduce the new goldens at the new aeon revision — but
that is an INFERENCE, and the whole point of a byte gate is that inference is what it replaces.

**The rule: when the baseline moves under an in-flight agent, the merge is not a merge.** Merge
master into the branch and **re-run the four-shape build against the NEW goldens at the NEW aeon
revision** before landing. This is the one case where "re-verify on the merged tree" is load-bearing
rather than ceremony — normally it re-proves a combination nobody changed; here the baseline
genuinely moved, and the agent structurally could not see it.

**Do not fix this by messaging the agent mid-flight.** Its fixed baseline is what makes its report a
clean byte-neutrality proof; moving the target underneath it buys a re-run and loses the control.
The correction belongs at the merge, which is the controller's job and not the agent's.

### THE DRIFT KEY MISSES ON DOCS-ONLY COMMITS — RULED: KEEP THE KEY, MAKE THE MISS NAME ITSELF (2026-08-30)

**The engine lane found it in their own half and refused to patch it**, correctly, because the fix
they could see carries a bias in the one direction this record must not lean. Their measurement,
taken while their agent worked: aeon master moved `d27ceba6 -> 07a97317` in **three docs-only
commits — identical ROM bytes, `lookup-aeon` MISS**. The nightly job keys aeon on
`git rev-parse origin/master` raw, so the steady state is `unverified` most nights and **N never
advances while the watch appears to be running.**

**Why their non-fix was right.** Resolving the queried revision to a ROM-path closure revision
widens every expectation to cover revisions nobody built, and errs toward **more** chains counting
as evidence-bearing — in a decision about whether the byte-identity gate can be retired. Failing
safe is the correct default and they held it.

**MY RULING, and it is a third option neither of us had: do not widen the key. Make the MISS state
its reason.** The defect that matters is not the miss — a conservative miss is correct — it is that
"nothing is accumulating because the key is brittle" is **indistinguishable from** "nothing is
accumulating because nothing landed." A watch that looks like it is running while structurally
unable to advance is this lane's own landed-and-blind class, one level down, and the harness already
refuses to render quiet as a verdict for exactly this reason.

So the report gains a discriminator: **`MISS (key moved, no byte-producing path changed)`** separate
from **`MISS (no expectation for this pair)`**. The first is computable cheaply and the method is
**aeon's own, used in the direction they specified** — a `git diff --stat` over the byte-producing
paths, which they banked as *"a PREDICTOR, never the assertion"*. It is a **REPORTING** discriminator
only: it **must never mint an expectation**, must not advance N, and must not promote a chain into
the evidence-bearing population. The conservative bias is preserved intact; only the silence is
removed.

**And the disagreement stays the prize.** If the pre-filter says no byte-producing path moved and a
build says the bytes DID, that is nondeterminism or an environment leak — the same-pair-different-CRC
alarm arriving through a second door. Wire both and make the mismatch first class.

**THE HALF THAT IS NOT MINE.** Whether the resulting evidence population — only chains where the
pair genuinely matches — is **acceptable** for N is the owner's, and it interacts with `d-48`. My
ruling narrows what the job hides, not what counts. Do not fold the two.

### THE RECORD REFUSED TO CARRY MY OWN CRC, AND THAT IS THE BEST THING IN IT (2026-08-30, aeon's)

Their record declined to store assembler `85a5726c`'s CRC even though **the number is known and
correct** — it is the chain-188 golden. Their reason: it is *sigil's* artifact, and mirroring my
goldens into the record would **launder my own expectations back into the job built to check them.**
They ran an independent build instead and reproduced `s4_debug 6516fc68/736315` at a different
revision as the cross-check.

**Bank the shape, not the instance.** A vacuous gate is not usually built out of wrong numbers — it
is built out of **right numbers taken from the party being checked**, which is why it survives
review: every value in it verifies. The test is not "is this figure correct" but **"whose artifact
is it, and would the checker be re-reading its own claim."**

### THE STATUS CURL RUNS AFTER **EVERY** WRITE — now contract, and my slip is its second instance (2026-08-30)

`contract/LANE_STATUS.md` at empyrean `97c4f72`, verified reachable from `origin/main` and read
there: **the boot step's curl runs after ANY write to `docs/lane-status.json`, not once at boot.**
The boot curl validates the file written at boot and nothing after it, and a lane **cannot see its
own invisibility** — the console is the only thing that reports the rejection.

**It is oracle's finding and mine is the second instance, an hour apart, neither knowing.** Oracle
wrote `state: "done"` mid-session and its card was dark on the owner's console for about an hour
across several pushes with nothing on its side looking wrong; the hub found it by reading the
console, and *a lane with no peers up would never have*. I wrote `closed` the same night. **Both of
us had read the warning about the exact field, in the boot text, shortly before doing it.**

**The contract's own sentence is the one to keep: *a rule you have read does not fire; a curl
does*.** That is the general form of what the section below says about my own case, and it is why
that section is about the verify step rather than about the vocabulary. Reading is not a control.
The write-then-verify pair is, and it costs one line.

### A FINISHED QUEUE ROW HAS NO STATE WORD — IT LEAVES THE QUEUE (2026-08-30, my own, forty minutes after reading the warning)

`lane-status.json`'s `state` vocabulary is exactly `doing | next | open | blocked`. There is no
`done` and no `closed`. **One bad enum in one row rejects the WHOLE file**, so every true thing in
it is lost and the owner's card for this lane goes dark — silently to the writer, because nothing
in the write path complains.

I did it anyway, with `closed`, having read the boot doc's warning about three lanes writing
`done` in three days less than an hour earlier. **Knowing the vocabulary is not what protects you;
running the verify curl is** — it named the row, the value and the legal set in one line, and it is
the only step in the sequence that could have caught it.

**The modelling error under the typo, which is the part worth keeping.** I reached for a state word
because I was thinking of the queue as a RECORD of what this lane did. It is not: it is the list of
things a fresh session could pick up. A finished item is therefore not a row in a terminal state —
**it is not a row.** It leaves, and its findings live in `lane-log.jsonl` and in this file, which
are the append-only records built for exactly that. A queue that accumulates completed rows is
telling a reader to choose between items that cannot be chosen.

### THE HOME-PATHS CLASSIFICATION IS MEASURED — AND THE BUCKETS ARE BEHAVIOURS, NOT POPULATIONS (2026-08-30)

`docs/superpowers/notes/2026-08-30-absolute-path-classify.md`, merged `8a377311`. Nine poison
scenarios, per-scenario never aggregate, all six of aurora's method corrections applied.

**THE RESULT THAT CHANGES HOW THIS IS FIXED.** There is ONE reference-dependent population of
**398 rows**, and which of the four things a row does — names the input, misdirects, goes
silent, or fails on purpose — is a property of the **(row, missing-part) PAIR**, not of the row.
Of the 61 rows that misdirect somewhere, **all 61 are silent somewhere else**; of the 184 that
name the input somewhere, **all 184 are silent somewhere else**; 31 both misdirect under one
poison tree and name the input under another. **A per-row fix list therefore cannot be derived
from any single run**, which is a stronger claim than "run more scenarios" and is the thing to
carry into the fix parcel.

**Aurora's correction (1) reproduces sharply, in the direction they said.** Absent tree: **0
misdirected, 0 failed, exit 0**. Source-only: the same. The scenarios that find anything are the
PARTIAL ones — roms-only 29, no-games-data 28, markers 13 — and **no single scenario finds more
than 29 of the 61**. Deleting the input really does find the least.

**AND THE ORDINARY SUITE IS GREEN WITH NO REFERENCE TREE AT ALL — by design, with one guard.**
`if !aeon.exists()` skips in ordinary mode and panics under `SIGIL_STRICT_GATE` (392 rows silent
ordinary / 398 failing strict). That is the designed shape, so do NOT read the 392 as broken
rows. Read it as: **the only thing standing between "no reference tree" and a green suite is the
strict gate, and the strict gate runs at landings.** That is this lane's own silent-green class —
IMPORTANT and non-self-correcting — sitting in the guard rather than in the rows.

### THE `generated/` GROUP SPLIT LOOKS LIKE A WRITER RECORD AND IS NOT ONE (2026-08-30, aeon's control, re-verified here)

**Written down because it will be rediscovered.** `aeon/engine/sound/generated/` carries a
**9 `volence:uucp` / 10 `volence:volence` split at one identical timestamp**. That is exactly the
shape of a writer discriminator, and the next person to go looking for "who wrote these" will find
it and conclude something.

**It is not a discriminator, and this is settled by a CONTROL rather than by reasoning.** The aeon
lane ran a build whose writer is known — theirs — and **the split survived completely unchanged,
9/10 before and 9/10 after**, while all 19 mtimes moved to `03:39:52`. Both halves re-verified
here directly rather than taken from their report. Group ownership is fixed **at creation** and
preserved by in-place rewrites, so it records a file's **FIRST** writer and says nothing whatever
about its most recent one.

**One refinement, measured here, and it sharpens their point rather than softening it.** The
directory is **not setgid** (`drwxr-xr-x volence:uucp`), so a new file takes the *creating
process's primary group* rather than inheriting the directory's. The split therefore does record
something real — that two processes with different primary groups each created part of this set at
some point — it simply cannot answer the only question anybody asks it. A loose thread nobody needs
today: `uucp` as a primary group is unusual and unexplained.

**The general shape, which is the reason this is a section and not a footnote.** A candidate record
that is *sitting right there and looks probative* is more dangerous than no record at all, because
it converts "we cannot know" into a confident wrong answer. The move that settled it was cheap and
is the one to copy: **exercise the candidate record with a KNOWN writer and see whether it moves.**
An artifact that does not move under a known cause is not evidence about that cause.

### THE `AEON_DIR` LIVE-TREE DEFAULT: REFUSE — BUT ARGUE IT ON INVISIBILITY, NOT ON CORRUPTION (2026-08-30)

**The deflating half first, because the recommendation must not carry a scarier framing than the
evidence supports.** Aeon's build **regenerates all nineteen blobs unconditionally** — one
`DEBUG=1 ./build.sh` moved every mtime, verified here. So an aeon build is **self-healing against a
foreign write**, and the hazard is NOT "their tree is silently corrupted". It is the narrower
*"a process that READS those blobs without regenerating first gets whatever last wrote them."*
**Price the recommendation on that**, not on the raw 93 files / 113 occurrences.

**And no contamination occurred.** Aeon snapshotted all 19, rebuilt, and compared: byte-identical,
with `s4.debug.bin` reproducing at `6516fc68` — the figure they had already published tonight. They
checked *because* they had published it, which is the right trigger.

**The recommendation stands at REFUSE anyway, and the argument changes.** A fallback to a hardcoded
live checkout is the option that is **structurally incapable of announcing its own failure**, and
the gitignore removes the last surface where a reader could notice. A refusal turns every unset-env
caller into a named error at its own call site — loud, local, fixed in a minute. **Put it to the
owner as an invisibility argument, never as an imminent-corruption story.** The scarier framing is
also the falsifiable one, and it would deserve to lose.

### THE SUITE WRITES INTO `AEON_DIR`, AND THE WRITE FLIPS OTHER ROWS' GUARDS MID-RUN (2026-08-30)

Found while reconciling a scenario that would not reproduce; mechanism re-derived at source here
rather than taken from the report. `seam1::emit_sound_blob` (`seam1.rs:729`) opens with
`create_dir_all(&out_dir)` **as its first statement, before it reads anything**, and
`native::ensure_generated` (`native.rs:1149`) points it at `$AEON_DIR/engine/sound/generated`. So
the suite mkdirs inside a tree that does not exist, then panics on the missing source.

**The consequence is not cosmetic.** `contract_closure_corpus`'s guard probes the **ROOT**
(`if !aeon.exists()`). Once another row has mkdir'd a path under it the root exists, the guard
stops skipping, the corpus walk finds zero files, and the floor fires. Measured both directions
on one command: pristine absent **0 failed exit 0** -> after the suite's own mkdir **53 failed
exit 101** -> directory deleted **0 failed exit 0**.

**Two things follow, and the second is the general one.** The `absent` scenario is **not stable
across repeated runs in one session** — its second run is a different scenario, which is exactly
the shape that makes a poison result irreproducible for the next person. And a read-side analysis
gate is mutating the tree it reads, **while `contract_closure_corpus`'s own doc comment declines
to do that very thing for that very reason** ("generating one would be a WRITE into `AEON_DIR`,
racing any concurrent build"). One module states the rule and a neighbour breaks it
unconditionally. **Remedy is byte-adjacent — `emit_sound_blob` produces the sound blob — so it is
sequenced with the aeon lane, not taken solo.**

**CLOSED, AND BYTE-NEUTRAL — the answer aeon was waiting on before rebasing a byte-mover.**
`docs/superpowers/notes/2026-08-30-reference-tree-write-guard.md`. **All SEVEN emitters carried
the shape, not just the first** — enumerated, not assumed. Each now calls
`seam2::require_reference_tree(aeon)` before it creates anything, and moves its `create_dir_all`
to AFTER the bytes exist, so a failing emit leaves nothing behind even inside a present tree. The
guard's probe (`SOUND_PLACEMENT_MAP_REL`) is the same constant `bank_anchors` reads, so the
precondition and the input it guards cannot name different files. All four shapes rebuilt:
`s4.bin 6e2f9b22/719315`, `s4.debug.bin 6516fc68/736315`, `demo.bin 9223a60d/96450`,
`demo.debug.bin d30c3636/101333` — **all four MATCH THE GOLDEN**. Nothing in `golden/`,
`pins.rs` or `repin.toml` touched.

**The three-state instability is gone**, re-measured the same way: absent-pristine, the same
command again, and after a delete now read identically, because nothing conjures the root
between runs. The gate is `crates/sigil-harness/tests/reference_tree_write_guard.rs`, run by
every `cargo test --workspace` (so by `scripts/landing-run.sh`), never skipping because it needs
no reference tree — and its emitter set is PARSED OUT OF `ensure_generated`'s own body, so an
eighth emitter added there and not to the gate fails by name rather than shrinking the coverage
silently. It reports UNMEASURABLE rather than green when it cannot establish that it ran.

**Still open, and NOT this lane's to take:** whether `ensure_generated` should write into the
reference tree at all, and the hardcoded `AEON_DIR` default itself (93 `.rs` files / 113
non-comment occurrences; 29 of the 127 literal-or-helper files can reach the write). Enumerated
with a recommendation and blast radius in the note; deliberately unchanged.

### A PUBLISHED COUNT OF MINE DOES NOT REPRODUCE — `322/181/129` IS RETIRED, NOT RESTATED (2026-08-30)

This board published **322/181/129** as the size of the home-paths exposure. Under measurement it
**does not reproduce under any of the four units that were actually checked**: 110 non-comment
path literals, 319 helper call sites, 466 rows living in an aeon-reading file, 398 rows whose
behaviour actually changes. **Do not quote 322/181/129 again**, and do not quietly substitute one
of the four for it either — the replacement numbers are the agent's measurement and are recorded
as theirs.

**The unit rule that survives this, which is aurora's correction (3) landing on my own figure.**
Literal counts measure **editing effort**; rows gated measure **coverage exposure**. One file here
carries 2 literals and gates 28 rows. The two are not interchangeable and the risk must be priced
in **rows gated (398)**. This is the second time this lane has shipped a count that was defensible
by population and wrong by unit; the first is booked two sections down.

### A BOOKED ROW IS PROSE WITH A PROMISE ATTACHED, AND PROSE DOES NOT RUN — `scripts/landing-run.sh` (2026-08-30)

**The incident.** This board already carried the row *landing runs use a dedicated
`CARGO_TARGET_DIR`; the shared `target/` is poisoned by other checkouts* — it is two sections
below, dated 2026-08-27, and it prescribes exactly the right fix. The 2026-08-30 landing of
`cc2e71bc` + `8acee94a` + `ec4c368d` was then done with the shared directory anyway and
produced **36 failures across four crates**, every one a `read <file>: No such file or
directory` on a file demonstrably present, because cargo bakes the building worktree's
`CARGO_MANIFEST_DIR` into the cached rlib. All 36 cleared against a clean directory. The lane
log's own words for it: *"My own booked row predicted that failure and prescribed the fix;
nothing executes a booked row."*

**The diagnosis is NOT the target directory.** A better-worded row would have failed the same
way. The gap is that a landing run has **six** preconditions, each invisible when omitted, and
the only thing holding them together was an operator's memory. The evidence that this was
never working: **26 ad-hoc `.sigil-*-target` directories** exist under `~/sonic_hacks/`, each
one a different session hand-rolling the same rule.

**The remedy is a wrapper that a landing run cannot omit the steps of.** `scripts/landing-run.sh`
carries all six as one command: a dedicated on-disk target dir (never tmpfs); a **refusal** —
not a warning — when pointed at any checkout's default `target/` **or** at a dedicated dir a
different checkout already built into (an ownership marker, which is the half that catches two
worktrees sharing one private dir); `SIGIL_STRICT_GATE=1` inside the command span; `AEON_DIR` /
`SIGIL_BUILD` / `SIGIL_EMIT` resolved once, refused early **by name**, and passed explicitly to
the child; a log stamped with pwd, HEAD, branch, the reference tree **and its HEAD**, and the
UTC start; `CARGO_EXIT=` written from `PIPESTATUS[0]`; failures-first with **every** failing
name; a `skip:`/`skipping` count; and reconciliation against a caller-stated `--baseline`
printing `baseline + N new = observed`.

**Two things it refuses that the brief for it did not ask for, both because the run is
otherwise red for a reason that is not the code's.** A reference tree missing any of the four
built ROMs is refused for a full run (the port and golden gates read them directly, and
`build.sh` makes one shape per invocation, so half-built is the common shape). And a run given
a cargo filter without `--scoped` is refused, because a partial run recorded as a landing is
the same class of error one level up.

**WHAT IT DOES NOT DO, and this is the honest limit.** It reduces the omission surface from
"remember six things" to "remember one thing." **It does not make omission impossible, because
someone can still not run it.** Nothing invokes it but a human — no timer, no hook, and no gate
inside the suite notices a landing that bypassed it. Two follow-ups that would genuinely close
that, neither built:

- **A suite-side test** comparing each test binary's baked `env!("CARGO_MANIFEST_DIR")` against
  the tree the run is standing in. That turns the shared-target class from 36 confusing
  missing-file reads into one named failure **regardless of who ran the suite or how** — the
  only proposal here that does not depend on the operator. It costs a test-count change, so it
  ripples into every stated baseline and wants its own parcel.
- **Folding the target-dir and environment refusals into `refreeze --attest`**, which already
  runs the suite, already sets the strict flag, and already stamps its log — so the freeze path
  and the landing path would share one set of preconditions instead of two.

**RELATION TO `refreeze --attest`, which is NOT superseded and does not supersede this.**
`--attest` covers requirements 3 and 5 already, and better, for the freeze path. It is not
usable as a general landing runner because it is bound to the provenance chain: it refuses once
the tip records a strict run, requires the chain to hold, and appends `[entry.strict]` to
`provenance.toml`. **A merge that moves no ROM bytes has no chain entry to attest**, and that is
the run the wrapper is for. Where both apply, `--attest` is the one that leaves a record.

**A measured correction to the recipe this file repeats.** The suite does **not** read
`SIGIL_EMIT` or `SIGIL_BUILD` — `native.rs` emits the sound blob in-process, and the only Rust
readers are `repin`, `derive_offcanon`, `refreeze` and `capture_goldens.sh`. CI runs the whole
workspace with neither set. Those two variables are needed to **build the reference tree**, not
to run the suite against it, which is why the wrapper defaults them into its own target dir and
builds them rather than demanding them. (A `docs/readme-refresh` parcel reached the same finding
on 2026-08-22 and declined to document the variable as required; the recipe blocks in this file
still imply it is.)

### A PAIRED LANDING CITES TWO SHAs THAT ANSWER DIFFERENT QUESTIONS — LABEL WHICH IS WHICH (2026-08-29)

A freeze lands as a pair: the commit carrying the **goldens and the `pins.rs` evidence**, and
the commit carrying the **attestation**. They are not interchangeable and the tip is the wrong
default. Instance: chain 180 was handed here as *"sigil `6b3ef068` (freeze ball-seating + attest)"*
— one SHA doing the work of two claims. `6b3ef068` is the attest; the goldens are in
`fa0e6540`. A reader who `--stat`s the cited SHA finds no goldens in it and concludes the
freeze is empty.

**The form: `freeze fa0e6540 · attested 6b3ef068`.**

**And the half this lane owes: I went to `fa0e6540` because that is where the data was, and
never said the citation had been wrong.** A silent correction costs the sender nothing and
teaches nobody — the citation form stays broken, and the next reader without the instinct to
go looking pays the full price. **When you route around a bad citation, say that you did.**
The verification was right; the silence was the defect, and it was the aeon lane that had to
raise it against their own message for it to surface at all.

### Standing rules — independent of whether a row is active

**Anything that can relink counts**, not just `cargo build --release` — see *Guard the
artifact, not the subcommand*. `cargo test --release --workspace` relinks the identical file.
Agents in worktrees with their own `CARGO_TARGET_DIR` are unaffected; a row binds the **main
checkout**, and dispatching worktree queue work during one is correct, not a violation.

**The commits-to-master clause attaches to a row only when that row's own *Why* names a run
that reads HEAD.** A row protecting an *attest* covers commits, because a version gate compares
the binary's revision against HEAD and a docs commit reddens the run while relinking nothing. A
row protecting a build-dependent *parcel* does not. Read into the wrong row it silently stops
this lane landing anything, at no benefit to the asker.

### A COMMIT TO THIS MASTER CAN REDDEN A PAIRED ATTEST WITHOUT MOVING A BYTE (2026-08-29)

**The bound, so a successor can evaluate it rather than obey it:** while the aeon lane is between
`--freeze` and `--attest`, a commit to sigil `master` — **including a docs-only one** — can turn
their run red, because the version gate compares the built binary's revision against HEAD. It
relinks nothing and changes no byte; the redness is the gate doing its job on a moved HEAD.

**Therefore: hold ALL commits here, prose included, from the moment a paired freeze starts until
its two SHAs arrive.** It costs nothing — `docs/lane-status.json` is git-ignored, so the lane can
keep saying what it is doing without moving HEAD.

**Evaluate it, do not inherit it.** The row is dead the moment no paired attest is running. The
condition is checkable in one message to the aeon lane, and this lane must not let it decay into
a habit of never committing while aeon is awake.

**How it was nearly violated, which is why it is written as a bound.** This overseer was about to
commit tonight's rulings while chain 182's attest was in flight, holding **three** true
clearances — standing push approval, no active hold row, zero bytes — every one of which answers
*"may I commit?"* and none of which answers *"may I commit while someone else's run is reading my
HEAD?"*. **A permission is scoped to the hazards its author was thinking about.** The tell: every
clearance checked was about this lane's own rules and none was about the peer's.

### THE AUTHOR OF A CONSTRAINT IS NOT EXEMPT FROM IT (2026-08-29)

Distinct from the volatile-pointer class, and this lane's own instance. This overseer wrote the
aeon lane a rule about not moving ground under a peer, and then did not check whether the rule
just written bound this lane. **Rehearsal is not protection; writing the rule is the highest-risk
moment rather than the safest** *(aeon's formulation, six instances across the two lanes in one
night, every one produced by a session actively holding the relevant rule)*.

### BORROWING `--git-dir` BORROWS THAT REPO'S INDEX — A FALSE POSITIVE THAT ARRIVES ON CUE (2026-08-29)

`git --git-dir=<other>/.git --work-tree=<copy> diff --stat <rev>` **does not compare the copy
against the revision.** It uses `<other>`'s index, so another checkout's staged deletions are
reported as properties of the tree being examined. Measured: it named three band-parcel files as
deleted from a reference copy in which all three were present and byte-identical.

**What made it dangerous was the timing, not the error.** It arrived inside a peer warning that
predicted exactly that poisoning, so the wrong answer was the expected one. Had it been believed,
a clean reference would have been discarded and rebuilt for a defect that did not exist.

**The instrument that settled it uses no git plumbing at all:** `[ -e ]` on the path, then
`hash-object` on the file against `rev-parse <rev>:<path>`, over every tracked path. 1,203 paths,
0 differing. **Prefer a content witness to a cleverer comparison** — the same conclusion this file
already reached about `level_staleness.py`.

**Two smaller traps from the same ten minutes, both this lane's:**

- **`pgrep -f '<pattern>'` MATCHES THE SHELL RUNNING THE CHECK.** A wait loop built on it never
  exits, and a liveness probe reports RUNNING forever — this overseer read "still running" three
  times off a suite that had finished nearly an hour earlier. Filter by `/proc/<pid>/cwd` and
  `cmdline`, or check the log's mtime. *(Same family as the aeon lane's `pgrep … | head -1`
  capturing a transient wrapper, reported the same night.)*
- **`git ls-tree` escapes non-ASCII names** (`docs/research/parallax-\302\2474.6.md`), so an
  existence test over its output checks filenames that never existed. Use `-z` and read NUL-safely.

### A BOOKED KILL LIST IS A HYPOTHESIS, AND ITS EXAMPLE ROTS SEPARATELY FROM ITS RULE (2026-08-29)

`parcel/retire-pinnedbaked`, merged `5babb3ea`, pushed. The `SizeSource::PinnedBaked`
placement path is gone: 854 lines deleted, `pins.rs` and `golden/` diffs empty, `repin.toml`
touched on 37 lines of which **0 are non-comment**, suite 4156/0/2 on the merged tree against
a 4156/0/4 baseline — passed count unchanged, the whole delta being the two deleted
`#[ignore]`d proofs.

**The finding is not the deletion. It is that the booking would have deleted a live gate.**
The FIVE-REG ledger row enumerated eleven symbols to remove. One of them,
`sonic4_pinned_profile`, had since acquired a live, non-ignored caller —
`canonical_pins_agree_with_the_canonical_size_tables`, the ONLY place `pins.rs` and the
canonical size tables are checked against each other. Executed verbatim, the clause deletes
that gate and the suite goes green with one fewer instrument. The row was **correct when
written**; nothing executes a ledger row, so nothing could tell it when it stopped being.

**The dispatch instruction that saved it is the reusable part**, and it is one sentence:
*treat the kill clause as a HYPOTHESIS, not as authority; step 1 is to re-derive the death
claim from source.* Booked work lists are the same PROSE-BOUND class as a stale comment, with
one aggravating difference — **a stale comment misinforms, a stale booking is executed.**

**AND THE EXAMPLE ROTS ON A DIFFERENT CLOCK FROM THE RULE.** The row said the bootstrap cannot
place the present tree because `ojz_effects_editor_act1` is content-derived and has no region.
Running the flag rather than citing the row gives:
`section 'player_instashield' has no region in the map`. **The mechanism was right and the
witness was stale.** This is worth separating from ordinary drift, because of how it fails: a
reader who checks the named example finds it does not hold and concludes the RULE is wrong,
when only the illustration had moved. A document can be right about the rule and wrong about
the evidence it offers for it. **Cite mechanisms in bookings; re-measure witnesses at use.**

**What the parcel buys downstream, and it is the reason to have done it first.** Region
LENGTHS now have **no reader outside `crates/*/tests/`** — `derive_canonical_bootstrap_table`,
`emp_map_toml`, `ModuleSpec::len` and `::region` are all gone. That discharges the written
revival condition standing over the R6 region-end conversion wave (~80 pairs): there is no
bootstrap left to revive, and no path from a region length to an emitted byte. The wave now
needs nothing but each region's own port gate.

### A HAND-WRITTEN "RERUN THESE TESTS" HINT THAT NOTHING CHECKS — 190 of 412 (2026-08-29)

`repin.toml`'s per-symbol `tests = [...]` is read in exactly ONE place —
`repin.rs:167-186` — and only after a pin has already been found to have drifted. It feeds a
printed line:

    rerun hint (affected binaries first, full workspace once at the end):
      dma_queue_port

**That is its whole behaviour.** It gates nothing, selects nothing that runs, and does not
affect `pins.rs` generation. Nothing fails if it is incomplete, so it **cannot be wrong out
loud**.

**Measured:** 190 of the 412 symbol rows carrying a `tests` list omit at least one test binary
that carries the label. *Honest bound: the sweep matches a quoted symbol name anywhere under
`crates/sigil-cli/tests/`, so it is an UPPER bound and some hits will be comments or unrelated
uses. The mechanism and the `DMA_Overflow_Count` instance below are source-confirmed; the 190
is not.*

**The source-confirmed instance.** `DMA_Overflow_Count` declares `tests = ["dma_queue_port"]`
(`repin.toml:1888`) while three tests carry its label row off the one shared pins constant —
`dma_queue_port:139`, `dplc_port:471`, `bg_anim_port:404`. All three lower
`engine/system/dma_queue.emp` through a real `flip_lower_and_place`, verified by reading the
enclosing block rather than the grep line.

**The cost is deferred and lands on a person.** When such a pin drifts, repin names one binary
while three are affected; someone reruns the one, sees green, and two stale port tests are
never examined. The consuming lane's own words for why this reaches them: *"a rerun hint naming
one binary is read as a complete instruction, not as a hint."*

**Same unexecuted-prose class as the comment and stale-booking sweeps, arriving on the one
artifact shaped like help rather than like a claim.** Booked as `REPIN-TESTS-HINT-UNDERLISTED`.

### A CONTROL SAYS YOUR MODEL AND THE WORLD DISAGREE — NOT WHICH ONE IS WRONG (2026-08-29, aeon's formulation of their own error)

Found in the same exchange and worth more than the row. The aeon lane predicted
`DMA_Overflow_Count`'s single-test row should be red or wrong, ran the control, and found it
green. They concluded *the field must answer a different question* — when the truth was *the
field has no teeth*. They took the committed row as the fixed point **because it was committed,
shipped and green, which are the three properties an unfalsifiable field has for free.**

The step neither lane had run is the cheap one: **grep for what CONSUMES the field.** A
precedent is only evidence if something could have contradicted it.

**And the direction cuts against a bar both our files carry.** The
*convenient-result-is-a-trigger* instinct did NOT apply: the satisfying answer was the true
one, and deference to a committed artifact is what nearly lost it. **Suspicion of one's own
cleverness is not a general-purpose instrument** — it can be pointed the wrong way, and here it
was.

### THE PORT HARNESS DOES NOT RESOLVE RAM SYMBOLS BY REGION — enumerated for the next cross-seam parcel (2026-08-29)

Answering the question chains 182 and 184 each discovered at `--attest`, after the freeze was
committed and pushed. **Every cross-seam address symbol is HAND-LISTED by name** in each port
test's `addr_labels()` as a `Vec<(&str, u32)>`, and each row becomes its own synthetic one-byte
section — `cpu 68000 / phase $VMA / <name>: / dc.b 0` — pinned at a per-shape VMA from `pins::`.
**A name absent from that table is an unresolved external.** New names in an existing region get
nothing for free; symbol WIDTH is irrelevant, since the carrier emits `dc.b 0` whatever the real
type is.

**Three sites per new symbol:** a `[[symbol]]` block in `repin.toml`; a `pins.rs` constant that
repin resolves from aeon's listings (`debug_only = true` yields a bare `pub const NAME: u32`,
not a `Pin`, and belongs in the `if debug { }` branch); and a table row in **every** consuming
test.

**The enumeration that matters is which tests LOWER the module, not which tests name the
symbol.** Measured:

| module | tests that lower it |
|---|---|
| `engine/system/dma_queue.emp` | `dma_queue_port`, `dplc_port`, `bg_anim_port` |
| `engine/system/vblank.emp` | `vblank_port`, `game_loop_port`, `load_art_port` |

**Sequencing:** `repin.toml` rows can be written in advance; `pins.rs` VALUES cannot, because
repin resolves them out of aeon's listings — so the parcel must build and emit listings first.
A parcel that also GROWS a lowered routine hits region byte gates in every lowerer, not merely
unresolved externals — a byte-gate re-prove, not a table row.

### A PEER'S DERIVATION DIED WITH ITS SESSION — AND THE ONLY SURVIVOR IS THE FALSIFIER (2026-08-30)

The aeon session that produced a freeze prediction for `parcel/dplc-entry-instrument` was
cleared mid-thread; the successor inherited the same socket and name and **does not hold the
conversation**. It grepped its own tree for every part of the derivation and found nothing —
it was never banked, and it died with that context. This is `BANK-TWO-FRAMINGS` happening to
somebody else, in real time, on the same night that row was written.

**What was reported to this lane, recorded as an UNVERIFIED REPORT from a session that no
longer exists — not as an aeon finding, and not as a measurement of this lane's.** DEBUG shape,
vblank site: **+$60 (96 B) measured against +$5A (90 B) derived** from instruction encodings
(three sub-queue blocks at 30 bytes each); the re-stamped fixture moved `refresh_addr`
16036 → 16132, with the cut's byte string differing in exactly one displacement field
(`3ea4` → `3f04`), i.e. pure displacement, no instruction change. The 6-byte gap was flagged by
its author as unaccounted-for rather than smoothed. **Nothing here is re-derived by this lane
and none of it should be cited as aeon's.**

**The successor's refusal is the correct one and is worth copying:** it declined to reconstruct
the derivation from this lane's paraphrase and hand it back as its own, because *a lane's own
reasoning, quoted back to it by a peer, is not a record.* A summary re-imported as a finding
launders provenance in the direction nobody checks. It banked the thread at aeon `35f33786`
with every number marked as this lane's report.

**What actually survives a rotation is the FALSIFIER, because it is a rule and not a number:**
*any plain-shape movement at all indicts the parcel, not the harness* — the whole `DMA_Peak_*`
block sits inside `if DEBUG == 1 {` and is comptime-elided, so the plain shape must be
byte-identical. Cheap, and re-runnable by anyone with no context at all.

**The general form, which is the part to carry:** when a session is about to end, a derivation
is the perishable half and a falsifier is the durable half. Bank the falsifier first. A number
without its derivation is an orphan a successor cannot defend; a falsifier without its
derivation is still a working instrument.

### A BROKEN COMMAND THAT RETURNS THE RIGHT ANSWER (2026-08-30, seraph's, and it generalises)

`grep -rani "sigil" --include=*.rs …` with the glob **unquoted under zsh** makes grep never run,
and returns a clean empty result **indistinguishable from a true negative**. The seraph lane hit
this while establishing it consumes nothing sigil-built; the conclusion was right and the
command was broken, which is the combination that never gets caught.

Same family as this file's *background wrapper reports exit 0 for a command that never ran* and
*a check can be vacuous by construction*: **an empty result is only evidence if the instrument
could have produced a non-empty one.** Quote the globs — and pair a zero with a positive control
in the same pass, which is what the aeon lane did when confirming the relink (`PinnedBaked` = 0
beside `sigil` = 5407 hits in the same `strings` run), turning an absence into a measurement.

### I CONTRADICTED MY OWN CORRECT STATEMENT INSIDE ONE SESSION (2026-08-30, aeon's catch)

Told the aeon lane the DPLC repin ask was on two boards, *"pushed, so it survives both our
rotations."* True of the `OVERSEER.md` row. **False of the `lane-status.json` row**, which is
gitignored here (`.gitignore:13`), is not at `origin/master`, and cannot be verified by any peer
at the remote. Confirmed after their catch: `git show origin/master:docs/lane-status.json` →
*exists on disk, but not in `origin/master`*.

**The aggravating part is the sequence.** This session had already stated the fact CORRECTLY,
twenty minutes earlier and unprompted — *"lane-status.json is gitignored, so the board updated
without moving HEAD"* — and then asserted the opposite once it became a durability claim. The
error was not ignorance; it was **stating my own repo's state from memory at the moment it
became load-bearing**, which is the one moment it gets checked least.

Both halves of the intent were in fact met, by a different mechanism than the one stated: the
lane-status row survives a `/clear` because **the file persists on disk**, not because it was
pushed — so it does not survive a fresh clone. **Bar 10 shape: the verdict held and the stated
reason did not, and the reason is what a reader carries forward.** Offering two rows as equally
checkable when only one is at the remote is the defect, not the row.

### SOUND REASONING ON A PEER'S UNVERIFIABLE PREMISE RETURNS TO THEM AS CORROBORATION (2026-08-30, oracle's, and it is the sharpest thing from this exchange)

The oracle lane reported that this relink had exposed a stale rebuild recipe on their side. This
lane agreed and extended it — *a recipe in prose either names the revision it was true at or
degrades into a wrong instruction rather than a historical note.* **The rule is right. The
instance never existed:** their recipe already built sigil from a pinned worktree at `7b46f075`
into a scratch `CARGO_TARGET_DIR`, never touching the shared binary, and the document said so in
prose. They had asserted it from their own overseer file's summary phrase — the word *"pinned"*
sitting in the sentence they were reading — without opening the recipe it summarised.

**Their formulation, and it names a circuit neither lane can close alone:** *a wrong premise
came back to me wearing your confidence.* The claim was about THEIR tree, so this lane had no
way to check it; sound reasoning applied to it made the error look **corroborated rather than
caught.** Same circuit as a number of theirs returning as a peer's and outranking their own
measurement.

**The obligation is on the responder and it is cheap: hedge the PREMISE, not the reasoning.**
When agreeing with a peer's claim about a tree you cannot read, say that you are reasoning
conditionally and that the premise is theirs to verify. A rule endorsed unconditionally on an
unchecked instance hands the reporter false corroboration in exactly the direction they will not
re-examine. Note this is the mirror of *a stated MECHANISM absorbs rather than competes*: there
the controller's story overrode the agent's evidence; here the responder's confidence overrode
the reporter's own doubt.

### R6 PROVISIONING IS DONE AND THE POPULATION IS ENUMERATED (2026-08-30)

`scripts/provision-aeon-ref.sh` replaces the paragraph nobody should re-derive under time
pressure. Reference tree at aeon `def98ee5` (the `aeon_rev` pinned by the provenance tail,
verified reachable from `origin/master` with `ls-remote`, never a tracking ref), living
OUTSIDE both repos at `../.aeon-r6` so it cannot pollute either git status.

**The positive witness is `repin --check` printing `pins.rs unchanged`, and it passed — twice,
the second time by running the script's own printed instruction rather than trusting that the
instruction was right.** A wrongly provisioned tree CANNOT reproduce the pinned revision's
placement, so an unchanged pin file is a positive result, not an absence. All four reference
ROMs verified by CRC32+size against the provenance tail before use.

**What the ~200-failure trap actually looks like, measured rather than quoted:** it does NOT
present as divergence. The gate named its own missing inputs — `no module
`engine.compression_vectors`` plus five `[embed.not-found]` lines pointing at
`engine/debug/generated/`. Honest, once you run one targeted test instead of the whole suite.
The three generation steps the real `build.sh` uses are salvador (`make -C tools/salvador`),
`tools/gen_compression_vectors.py`, and `emit_sound_blob` for the sound tree; `repin` resolves
sound-ON and names `SIGIL_EMIT` itself when it is unset.

**THE POPULATION IS 50, NOT 51 AND NOT ~80 — and I got it wrong twice on the way here.** The
board said "~80". A `grep -c` said 51 and was published as a correction. The structured parse
says **50**: the 51st hit is `repin.toml:39`, a line in the manifest's own header COMMENT
explaining the syntax. **Counting a documentation line as a member of the population it
documents** is the same unexecuted-prose family as the rest of this file's sweep, arriving on
my own correction of a stale number. The reusable form: **a `grep -c` counts a spelling, not a
population; parse the structure when the number is going to be quoted.**

**The 50 split by what the last byte belongs to, which decides the work:**

- **47 straightforward** — the region's own section owns the last byte, so `end =
  "section:<name>"` is a pure re-spelling. Gaps run 2 to 32 bytes.
- **3 need care** — the last byte belongs to a DIFFERENT section: `objdefs` → `text`,
  `dust_spindash` → `ring_sparkle`, `player_climb` → `player_instashield`. `objdefs` → `text`
  is the ambiguous-ownership case R6 already ruled on: **an unresolvable owner is dropped,
  never attributed.**

**Sequencing note for whoever converts:** a conversion SHRINKS a pin length by the gap, so
`pins.rs` changes and `repin --check` goes red until repin rewrites it. That is expected and is
NOT a paired freeze: a pin length reaches no emitted byte, and since `5babb3ea` no region length
has any reader outside tests. The 82 warning lines cover these 50 regions across both shapes;
remember the meter is an ADDRESS comparison and is blind to the flush cases, so **do not treat
the warning list as the whole population** without the `section_label_owners` derivation.

### DECISIONS CLOSED OUT OF SHAPE — DO NOT REPAIR (listed once, per DECISIONS.md rule 8d)

Rule 8d (`answered` on the closing entry) is in force from **2026-08-30T01:58:05Z**, empyrean
`df8939b`, verified reachable before this list was written. The rule grandfathers everything
appended earlier and says so in as many words: *"Nothing in 8d is an instruction to touch an
existing line."* **These are listed, not fixed.**

**Closures out of shape — `d-4`, `d-5`, `d-6`, `d-8`, `d-10`, `d-12`, `d-13`, `d-16`.** Each
carries `supersedes` and the identical question/options/recommend that 8c requires, and none
carries `answered`, because the field did not exist when they were written. Every one of them is
settled the rule-9 way — the blocker was dropped — and `blockedOnOwner` is currently empty, so
none of them renders as an open card.

**`d-15` is NOT in that list and must not be counted as a closure.** It supersedes `d-14` for a
SCHEMA defect while the question stayed open: `d-14` carried a `state` field the contract does
not define, a `refs.commits` key it does not list, and a `refs.queue` id (`EMBED-BASE-RULE`) that
existed nowhere. The re-file changed nothing about the question, the options or the
recommendation. **A supersede is not evidence of an answer** — `supersedes` marks replacement,
and replacement has at least two causes. Counting every superseding entry as a closure would
record `d-14` as a question the owner decided, which he never saw.

The closure of `d-15` is **`d-16`**, appended 2026-08-30T01:14:44Z — 43 minutes before 8d came
into force, which is why it is on the grandfathered list rather than being the rule's first
customer. Its content, for anyone reading the record without the `answered` field to key on:
ruled by the **hub in the owner's place**, reversible, option **`remove`**; the `bless` option
was explicitly NOT ruled and stays reserved to the owner under `d-6`, because it would add
durable language surface.

**From here every closure carries `answered`.**

### A POISON MUST RESEMBLE THE FAILURE REALITY PRODUCES (2026-08-30, aurora's measurement)

This lane booked *"poison by making the path absent"* as the discriminator between an absent-input
failure that is silent and one that is loud. **Aurora measured that this finds the LEAST:**

| scenario | misdirected rows |
|---|---|
| reference tree **absent** | **0** of 306 |
| reference tree **present but empty** | **43** |

**The generalisation is bigger than paths. Deleting a thing is a CLEANER break than the world
delivers.** Real failures are partial — a half-clone, a wrong path, a stale mount, the case on
someone else's machine — and **a partial break is what routes a failure into the wrong
vocabulary.** A total break is loud and honest: the lookup fails at the top and names itself. A
partial one lets a lookup return a plausible nothing and surface the error somewhere unrelated.

That is the mechanism of this file's own loud-elsewhere instance: the listing was **missing from a
tree that otherwise existed and worked**. Had the whole reference tree been gone, the failure
would have named itself in the first line.

**So a poison designed for a clean absence tests a failure mode the system will rarely meet.**
Design the poison against the *partial* state, and run both — the total-absence run is still worth
having as the control that proves the poison is reaching anything at all.

### A COUNT CAN BE WRONG BY POPULATION OR BY UNIT, AND THIS FILE ONLY CARRIED THE FIRST (2026-08-30)

This file already says **a `grep -c` counts a spelling, not a population**. Aurora's O11 landing
supplies the other half: **even when the population is right, the UNIT can answer a different
question.**

Measured there: **one literal gated 51 rows.** So a count of literals measures **editing effort**;
a count of rows gated measures **coverage exposure**. Both are real populations of real things.
Only one prices the risk.

**This lane's own instance, published to the hub before the correction arrived:** the
`/home/volence` sweep here gave **322 occurrences / 181 files / 129 test-or-harness**, and the row
was scoped by those figures — including a caveat that *"129 is not 129 unknowns"*, which was
reasoning carefully **in the wrong unit**. The numbers stand for sizing an edit and are worthless
for pricing exposure; re-measure by rows gated before anyone prices that row.

**And a third shape neither count reaches: a WALK-UP PATH FINDER looks converted and still opens
the real tree.** No source read catches it — only an fs-level trace with the override pointed at an
absent directory. Check `SIGIL_ROOT` derivation in `scripts/` first.

### URGENT AND IMPORTANT ARE NOT A RANKING — THE TWO ABSENT-INPUT CLASSES (2026-08-30)

Sigil contributed a fourth class to the suite's baked-absolute-path audit, and it came back from
the hub as a **fix ordering with a justification this lane did not write**. Both halves are worth
keeping: the class, and the correction of the rule made from it.

**The fourth class — BAKED-AND-LOUD-ELSEWHERE.** The suite's audit sorted absent-input failures
into baked-and-silent, baked-and-loud, and legitimate coupling. This repo has one that is none of
them: `listing_symbol_addr(s4.lst, sym)` on a missing listing **returns `None` rather than
erroring**, no label is pushed, and the failure surfaces as `unresolved symbol RingSparkle_Spawn
for fixup in section rings` — in a test with nothing to do with listings. **A sweep hunting for
green-when-absent cannot find it: it is red, loudly, in the wrong place.** Measured cost here: one
false attribution and three needless reverts.

**The ordering that came back, and why its stated reason is wrong.** It was banked as *loud-in-the-
wrong-place before silent-green, because a misdirected failure costs reverts while a silent pass
costs nothing until trusted.* **The final clause is false. A green test in a suite is trusted
immediately and by construction** — that is what a suite is for. There is no interval in which a
silent pass sits un-relied-upon; every reader counts it as coverage the moment it is in the run.

**The two classes have different URGENCIES and ranking them by seriousness inverts the durable
risk:**

- **Loud-in-the-wrong-place is URGENT and SELF-LIMITING.** It costs reverts now, to whoever holds
  an unrelated parcel — and because it hurts, it gets fixed. This one was fixed within hours.
- **Silent-green is IMPORTANT and NEVER SELF-CORRECTS.** Its cost is invisible and unbounded, and
  **precisely because nothing hurts, nothing ever raises it.** It is the class that survives for
  years, and this file's own sweep is full of them: a rerun hint nothing checks, a gate defeasible
  by build order, a control comparing an artifact to a copy of itself.

**So the correct form: fix loud-in-the-wrong-place first because it is cheap and bleeding, not
because it is more serious.** A reader who takes the ordering as a seriousness ranking concludes
silent-green is the lesser defect — **which is the exact reading that lets it keep living, and is
therefore the mechanism by which it lives.**

**The process note, this lane's own.** A comparative remark of mine returned from a peer as a
settled priority ordering with a justification attached that I would not have written, on an
instance no lane outside sigil could check. That is the corroboration circuit this file already
names, running on my own contribution — **and the obligation is on the party who can check it to
say so**, which here is me.

### THREE WRONG DIAGNOSES OF ONE RED, AND THE STRUCTURAL CAUSE UNDER ALL THREE (2026-08-30)

**CORRECTION OF RECORD. Merge commit `2a53cbaf` and the lane-log entry of 03:34:57Z both state
that the aeon clamps parcel introduced `Cache_Fill_Resume_Col`. That is WRONG.** Those artifacts
are pushed and are not being rewritten; this section is the correction they point at.

**What actually happened, measured with correct quoting over both `section.emp` and
`plane_buffer.emp`:**

| commit | lines added carrying the symbol |
|---|---|
| `839d600d` — the d-45 canopy fix | **7** |
| `d3b3ab5a` (clamps) | **0** |
| `9ba11115` (clamps) | **0** |

The pickaxe over `def98ee5..ec6a4791` names `839d600d` and nothing else. It is an **ancestor of
the clamps merge** — already on master when that branch was cut — with **56 commits between the
two merges and no attest among them.**

**The three diagnoses, and note that each is a correct measurement wrongly scoped:**

1. **aeon: "latent condition exposed."** Read the endpoints as equal (4 before, 4 after) — but
   both counts were the *after* count, taken through the broken shell read below.
2. **This lane: "the clamps parcel introduced it."** Read the endpoints as unequal (0 → 7) and
   attributed the difference to the parcel being frozen. **The endpoints are right; the
   attribution is not.** An endpoint measurement is a fact about a SPAN and about no commit
   inside it.
3. **The agent's own output contained the answer and neither of us read it as one.** It ran
   `git log -S`, got `839d600d` back, wrote *"which is inside the span"* — and then concluded
   *"the parcel added"*. **The commit that settles the question was printed and passed over,**
   because it arrived as corroborating detail rather than as the subject.

**THE STRUCTURAL CAUSE, and it is aeon's — it outlives all three diagnoses: ATTEST DEBT ACCRUES
SILENTLY AND IS CHARGED TO A STRANGER.** A freeze attributes every strict failure to the parcel
being frozen, because that is the only parcel it knows about. The longer the gap between attests,
the more of someone else's debt the next freezer inherits — and **the natural reading of the red,
by every party including the lane that caused it, is that the current parcel did it.** Nothing in
the tooling can partition that population.

**The remedy, aeon's, cheap and structural: run `freeze_preflight.sh` at the START of a parcel,
not only before its freeze.** Red before you touch anything means the debt is not yours and the
diagnosis begins elsewhere. It partitions the population before the question can be framed
wrongly, which is why it beats "remember to run step two".

### `git show $rev:path` IS SILENTLY BROKEN IN ZSH AND RETURNS A PLAUSIBLE ZERO (2026-08-30)

`git show $rev:engine/level/section.emp` — unbraced — parses `$rev:e` as zsh's **history
modifier** for "extension", so git receives `ngine/level/section.emp`, dies with *"ambiguous
argument"*, and a piped `grep -c` prints **`0`**. Not an error: a plausible count.

**Four instances in one night across two lanes** — three of aeon's loops and one of this lane's —
and one of them produced the "4 before and 4 after" reading that became diagnosis 1.

**This lane's instance is the instructive one because the trap was a SECOND defect on top.** The
loop carried `2>/dev/null`, so the fatal error was discarded and the fabricated zero stood alone.
It appeared to refute this lane's own agent and was nearly sent to aeon as such. **Suppression did
not hide an error; it destroyed the only artifact that could have corrected the reading** — the
exact clause this file already carries, met while breaking it.

**Correctives:** brace both sides (`"${rev}:${path}"`), or `git cat-file -e` first; and never
`2>/dev/null` a command whose emptiness you are about to treat as a finding. **The braced re-run
is where every number in the table above comes from.**

### R2 IS GATED ON A NUMBER ONLY THE OWNER CAN SET (2026-08-30, aeon's ruling, read out of their plan)

The hub pushed R2 as the next SIGIL-DECOUPLE step once R6 landed. **It is not startable**, and
the reason is stronger than "step 4 has not happened yet".

Aeon settled it from `docs/superpowers/plans/2026-08-27-sigil-decouple-steps-1-4.md` §3, quoting
their own text rather than recalling it: *"So step 4 is gated on data from step 1, not on a
decision. N is unset deliberately; the owner sets it, and it is the one number in this plan
neither lane should choose."* Step 1's nightly non-blocking drift job is the instrument that
produces step 4's answer — run it for N chains, and either the engine stays byte-clean while
nothing blocks on it (the gate was spent) or reds appear that a landing would have caught (the
gate was load-bearing and earns a promotion instead of a retirement). §3's reason: *"Neither lane
can distinguish those from inside, and step 4 must not be written from either lane's intuition."*

**So step 4 is not merely undone, it is UNANSWERABLE until step 1 has run N chains, and N is the
owner's number.** Neither lane opens this by working harder. The unblocking move is to get N set,
which is why this is an owner decision and not a backlog row. **Aeon files that card, not this
lane** — they hold the plan and the firsthand text, and `DECISIONS.md` gives one cross-lane
blocker exactly one card, filed by the lane holding the measurement. Do not file a second.

**The method note worth keeping.** This lane's inventory row stated R2's gate correctly, and this
lane still could not settle it — the row is a summary of aeon's text, and a sequencing clause read
two hops from its source is exactly where a plan rots. Asking the party who owns the plan cost one
message and returned the governing sentence. **A booked sequencing clause is a hypothesis; the
lane that wrote the plan is the instrument.**

### THE FAR-SCRATCH SLOTS ALIAS ACROSS THE 24-BIT BUS, AND IT BEARS ON R2's TRADE (2026-08-30)

R2's inventory row prices the choice as two-sided: keeping the far-scratch means the assembler goes
on emulating a toolchain nobody runs, dropping it moves bytes. **There is a third term the row does
not carry**, found in `native.rs`'s own header while grounding a dispatch.

Never-pinned ROM sections measure at `0x70_0000 + k*0x10_0000` (`native.rs::lens_pinned`). Those
slots **wrap the 24-bit bus from k = 9**, verified here by arithmetic rather than taken from the
comment: k=9 gives `0x0100_0000`, masked to `0x00_0000`; k=41 gives `0x0300_0000`, also masked to
`0x00_0000`. `asl_width_rule` (`crates/sigil-ir/src/width.rs:34`, confirmed to exist as code and
not only as a name in comments) masks its argument to 24 bits and returns `W` for `a <= 0x7FFF`,
so **an aliased slot looks abs.w-reachable when the real base is not**. That is the exact shape of
the `player_sensors` catch the header records: twelve `lea` sites measured 4 B each where the real
base encodes 6.

**What is settled and what is not.** The header states the hazard is not a live measuring input
*because every FROZEN-labeled section now measures at a real base in every round*, and books it as
a ledger row for the next refreeze. That reasoning covers frozen sections. **It does not, on its
face, cover the never-pinned ones, which are precisely the sections that still take a scratch
slot.** Whether any never-pinned section reaches k >= 9 today is NOT measured here — the inventory
row names three (`replay`, `raster`, `page_cache`) behind an ellipsis, and an ellipsis is not a
count. **Do not quote a population from that row; derive it.**

**Why it matters for R2:** if the answer is yes, dropping the far-scratch stops being a pure
taste trade between two defensible options and starts also retiring a live width-selection hazard,
which is a correctness argument and outranks both. Settle the count before R2 is priced.

### THE R6 CONVERSIONS LANDED AT 45, AND THE THREE REFUSALS ARE THE FINDING (2026-08-30)

45 of the 50 regions converted from `end_measures = "allotment"` to `end = "section:<name>"`.
Suite **4156 passed / 0 failed / 2 ignored over 357 binaries**, matching the master baseline
exactly. **The falsifier stated before the parcel HELD at every repin: across 1186 pin fields,
every change was a LENGTH, every length SHRANK, and no BASE moved.** That is the property that
makes this parcel sigil-internal and not a paired freeze.

**The five that did not convert, and why each refusal is a different mechanism:**

- **`objdefs`, `dust_spindash`, `player_climb`** — the last byte belongs to a DIFFERENT section
  (`text`, `ring_sparkle`, `player_instashield`). `objdefs` is the ambiguous-owner case R6
  already ruled: an unresolvable owner is dropped, never attributed.
- **`entity_window` and `children`** — a **shared-anchor contract asserted in a PORT TEST, not
  in the manifest.** `children_port` requires `entity_window` to end exactly where `children`
  begins and `children` exactly where `load_object` begins. Their end IS deliberately the next
  placement, so the width is a true allotment rather than an accident.

**THE DISCRIMINATOR I SHIPPED WAS INSUFFICIENT AND THIS IS THE REUSABLE PART.** "Does the
region's own section own the last byte?" answers *can this region name its own end*. It does NOT
answer *is this end deliberate*. Deliberateness lived in an assertion in a test file, where no
manifest-level derivation could see it. **Ownership and intent are two questions and only one of
them is visible in the placement data.**

### A PROVISIONING GAP THAT PRESENTS AS A REGRESSION IN WHATEVER YOU ARE HOLDING (2026-08-30)

**My own script, published as proved, was missing the listings** — the step this file's recipe
had named in words (*"generate the `.lst` files from master's sigil BEFORE editing anything"*)
and which I then did not implement.

**How it fails, and it is nasty.** Several port gates resolve a cross-region symbol through
`listing_symbol_addr(s4.lst, sym)`. A missing listing does not error: the lookup returns `None`,
no label is pushed, and the failure surfaces later as `unresolved symbol RingSparkle_Spawn for
fixup in section rings` — **in a test with nothing to do with listings, and reading exactly like
a regression in the parcel you happen to be holding.** It cost a false attribution and three
needless reverts: `rings` was reverted for a defect it did not have, and `entity_window` and
`children` were reverted against a poisoned run before being re-tested properly.

**THE WITNESS I PUBLISHED IS NECESSARY AND NOT SUFFICIENT.** `repin --check` printing
`pins.rs unchanged` passed on the tree that had NO listings at all. It witnesses that placement
resolves; it is silent on every artifact the gates read afterwards. **A witness earns only the
scope of what it touches** — the same limit the aeon lane put on the off-canonical table, now
demonstrated against my own instrument.

**The stronger control, now in the script:** build both shapes in the provisioned tree and check
the freshly built ROM against the golden CRC32. Both matched at `def98ee5`. A rebuild landing on
the frozen CRC cannot be produced by a wrongly provisioned tree, and unlike `pins.rs unchanged`
it exercises the listings, the generated trees and the sound blob on the way.

**And the process lesson, which is the one I keep re-learning tonight: I had no BASELINE run.**
With no measurement of this tree before the parcel, every failure was attributable to the parcel
by default. One clean run on the unconverted tree would have shown `rings` red before I touched
anything.

### `AEON_DIR` NEEDS A PROVISIONED WORKTREE, NOT A BARE ONE — AND THE FAILURE LOOKS REAL (2026-08-29)

**Correcting this file's own standing advice.** "Use a plain detached worktree at the goldens'
SHA" is necessary and NOT sufficient. Verified firsthand on `.aeon-pinnedbaked` at
`def98ee5`: it carries **15 gitignored artifacts** that `git worktree add --detach` does not
produce — the seven reference ROMs (`s4`, `s4.debug`, `demo`, `demo.debug`, `config_a`,
`config_b`, `lean`), `s4.lst` / `s4.debug.lst`, the generated `engine/{debug,sound}/generated/`
trees, and `tools/salvador` built plus its compression vectors.

**Without them the suite reports ~200 failures that read exactly like golden divergence** —
the same signature as the shared-`CARGO_TARGET_DIR` trap, and equally unfalsifiable from the
log alone. Two provisioning rules that keep it honest:

- **The reference ROMs may be copied from sigil's own goldens** — but verify all seven
  CRC32+size against the provenance tail first, which proves they are that revision's
  artifacts. Non-circular: they are not built by the tree under test.
- **Generate the `.lst` files from master's sigil BEFORE editing anything**, so the same fixed
  oracle serves the baseline run and the post-change run.

### A BACKGROUND WRAPPER REPORTS EXIT 0 FOR A COMMAND THAT NEVER RAN (2026-08-29)

`… > $DIR/landing.log 2>&1` with `$DIR` not yet existing fails in the SHELL, before cargo is
reached — and the harness still reported **"completed (exit code 0)"**. A landing run that
never happened, announced as green. Caught only because the log was inspected rather than the
exit status believed.

**Therefore: `mkdir -p` the target dir in the same command, and never read a background
wrapper's exit code as the run's verdict.** Put the real one IN the log
(`echo "CARGO_EXIT=$?"`) and grep for it. Same family as *a suite log does not name its tree*:
the trustworthy witness is inside the artifact, not in the thing reporting on it.

### LANDING RECORD — `parcel/freeze-step-gap`, merged `ec870c3e`, pushed (2026-08-29)

Landed **under a hub ruling made in the owner's place**, not under his direct word, and that is
recorded here so it can be reversed cheaply. Authority verified at a committed revision before
acting: empyrean `3de429d`, reachable from their `origin/main`, transcribing the owner's
*"if anything's confused you can make decisions/fable can. goodnight!"* and *"(you're the
director/overseer)"*, scoped by the hub's own line to *"wherever a lane is blocked on a
decision."* **This lane did not witness the utterance.**

**Gate, with its own positive control rather than a green log:** 4,139 passed / 0 failed / 4
ignored over 358 test binaries on the merged tree; the parcel's own 20 gates named in the log by
`freeze_step_gap`; and — because `SIGIL_STRICT_GATE` only makes a *missing* reference fatal —
the strict path was proved live by pointing `AEON_DIR` at an empty directory and watching
`vblank_port` fail by name, then passing against the real reference. **A green run whose new
gates never executed is the artifact this bar exists to refuse.**

**Reference tree:** an exclusive `cp -a` of `.aeon-land-182` (detached at aeon `e99a2ca7`), never
aeon's shared checkout — which is deliberately behind `origin/master` pending the owner's `d-44`
and must not be read for artifacts or contract facts. Verified before use: 1,203 tracked paths
byte-identical to `e99a2ca7`, four ROM CRCs matching ledger entry 183.

### A NUMBER DERIVED FROM A REAL MEASUREMENT OF THE WRONG QUANTITY (2026-08-29, aeon's sharpening of this lane's finding)

**Worse than unexecuted prose, because it survives the check that catches unexecuted prose.** A
comment saying *"two is the floor"* reads as measured, and it IS measured — the derivation behind
it is `~354 KB of character art ≈ 2.7 × 128 KB, so at least two boundary-straddling entries exist
by construction`. **That is an argument about total ART VOLUME. It bounds EXISTENCE somewhere in
the ROM. The reserve has to cover how many can want a slot in ONE FRAME**, which the derivation
says nothing about. A lower bound on a different quantity, wearing the grammar of a floor on this
one.

**Why the prose sweep cannot catch it.** `PROSE-BOUND-SWEEP` looks for claims nothing executes.
This claim has evidence, arithmetic, and a source; asking *"is this backed by a measurement?"*
returns yes. **The question that catches it is "a measurement OF WHAT?"** — does the quantity
measured govern the decision the number is guarding?

**And the operational half, which is aeon's:** the closing condition is a measurement (count
straddling Important enqueues per frame on a real run), **with an explicit *do not raise the
reserve without it*** — raising an unmeasured number trades a possible drop for a certain cost.
**A wrong bound is not repaired by making it bigger.**

### WRITE THE `ensure` IN THE SAME COMMIT AS THE OPTIMISATION THAT DEPENDS ON IT (2026-08-29)

Adopted with aeon, and it is the discipline the alignment declaration's residual rests on. Any
optimisation depending on a base's low bits (a page-aligned index, a mask instead of a divide, a
`bankid()` assumption) must land with its `ensure` in the **same commit** — not as belt-and-braces
but because **the `ensure` is the only artifact that makes the requirement legible outside the
routine.** Without it the requirement exists solely inside an instruction encoding, `section_align`
declares the 68000 floor of 2, and the gate is green over a real constraint.

**The live near-miss, verified in aeon's source by both lanes:** `z80_sound_driver.emp:1034`
records a 256-byte page-aligned form **declined** because `ensure((DacSampleTable & $FF) == 0)`
fails at `$85AD`, with *"keep the ensure with it."* Taken without that line, the requirement would
have been invisible to every instrument either lane has.


### THE PROGRESS METER WAS THE DEFECT'S OWN BLIND SPOT (2026-08-29, R6's finding — bank 58, not 82)

The REPIN-END parcel left a population of regions borrowing a neighbour's padding, **warned and
ledgered**, and the warning became the progress meter. **That warning is an ADDRESS comparison.**
A successor's head label sitting *flush* against a region's last byte is, by address, identical to
a label the region owns. So the meter counted only the padded cases: **82 visible, 58 invisible,
real population 140.**

**This is the vacuity class arriving on the one artifact nobody audits: the burndown.** Every
other instance today was a gate that could not fail. This is a gate that could not fail *at
counting the thing it was created to drain* — the meter would have read zero with 58 live. **When
a check is promoted to a progress meter, re-derive the population by what DETERMINES the value,
never by what the check reports** (bar 8 aimed at your own burndown).

**The discriminator that fixed it is a second, independent derivation**: `native::section_label_owners`
answers *which section DEFINES the end label*, which an address cannot. Ambiguous ownership
(`text`, ~20 instances) is **dropped, never attributed** — an unresolvable owner is not a match.

**And the related history correction, which sharpens the remedy:** the `ACT_DESCRIPTOR` incident
was **not** silent. The port gate fired — it failed on length with every content byte matching.
The failure was **misattribution**, not absence. That is why R6's remedy is a message that NAMES
the neighbour rather than another comparison; a louder comparison would have reproduced the
original confusion.

### A PIN LENGTH IS SIGIL-SIDE; A PIN BASE IS NOT (2026-08-29, enumerated not assumed)

Sequencing fact for the remaining ~80 end conversions. **A pin BASE reaches emitted bytes**:
`seam1.rs:51` computes `blob_lma() = pins::BOOT_HEAD.{plain,debug}_base + 54`, and `seam1.rs:565`
assigns `sec.lma = blob_lma(debug) + cursor`. **A pin LENGTH does not**, on any live path: the only
non-test consumer is `ModuleSpec::len` (`native.rs:301`) → `emp_map_toml` → `build_emp`, reachable
only under `SizeSource::PinnedBaked`, and `build_native_rom_with_listing` (`native.rs:4113-4115`)
returns early to the chained builder because `sonic4_profile` constructs `SizeSource::Frozen`
unconditionally.

**So an `end` re-spelling can only move a length, never a base** (`repin.rs:840-843` derives bases
from `start`; `:884` derives lengths as `end − base`) — **the 80 conversions need no paired freeze.**

**The revival condition is GONE (2026-08-29, `parcel/retire-pinnedbaked`).** It used to read:
`derive_canonical_bootstrap_table` *does* read region lengths and mints a frozen boundary table,
so a wrong length there reaches bytes one step removed — re-check it if the PinnedBaked bootstrap
is ever revived. That parcel deleted the bootstrap, `derive_canonical_bootstrap_table`,
`emp_map_toml` and `ModuleSpec::len`/`region`, so nothing outside `crates/*/tests/` reads a region
LENGTH at all (203 readers, all in tests, measured after the deletion). **A conversion now needs
only its own port gate to pass; there is no bootstrap left to guard against.**

**Provenance worth keeping: the general form was asserted from the specific case measured**, and
the agent said so in those words when challenged. The narrowed claim is the one banked.


### CHAIN NUMBERS: 181 IS THE COLLISION RE-BAKE; THE VSRAM FIX MOVED TO 182 (2026-08-29)

Corrected in place because the booking below was written when the VSRAM fix was next in line
and has been wrong since aeon queued the re-bake ahead of it. **Chain 181 = the collision
re-bake** carrying the owner's 574-cell repaint into the generated tree (aeon's master was red
on two collision gates until it merged; the repaint was in the source of truth and absent from
the artifact). Everything below about the VSRAM fix is unchanged and true — **it is chain 182**.
A stale chain-number booking is the same unexecuted-prose defect as the rest of this file's
sweep: nothing runs a chain number, so nothing could contradict it.

**Chain 181's `[entry.strict].sigil_rev` is ORPHANED and is staying that way** — the rebase
between attest and push moved the freeze commit. It is measured, named and deliberately not
repaired; see the `PROVENANCE-REV-REACHABILITY` row in the Queue for what it is and why
re-attesting is refused.

### CHAIN 182, the left-edge VSRAM fix — BLOCKED ON THE OWNER

aeon ruled it lands **strictly after 180, never sharing a freeze range** (chain 179's
two-movers-in-one-range lesson, applied). **It is not a length-neutral parcel and pins WILL
move**, direction stated before the repin rather than at it: the DEBUG shape grows 4 bytes;
180 symbols slide +4 in release, and in debug the same 180 slide +4 and a further 282 slide
+16 as the region crosses a 16-byte placement boundary. It also touches
`engine/level/parallax.emp` and the scene DSL, so **it may move a port surface** — aeon reports
which before the repin.

**Currently stopped on the owner**: its cost came out materially larger than the decision card
he ruled on described, and aeon stopped rather than landed. A fresh prediction and a fresh
falsifier come with it when it unblocks; **the relink row re-arms at that point, not now.**

### ONE `refreeze` DEFECT THE CHAIN-180 FREEZE PAID FOR — the other was retracted (2026-08-29)

Reported as two by the aeon lane. One is real and is this lane's to fix; the other was a
correct measurement with a wrong cause attached, refuted here the same morning and left
standing below as the retraction, because a finding that is quietly deleted teaches nobody
and the wrong mechanism had already reached a hub ruling and a dispatch order.

**1. RETRACTED 2026-08-29 — the grep was right, the mechanism was wrong, and the queued fix
would have made things worse.** What was banked here: *"Without the override it operates on the
tree it was compiled in — the shared checkout ... nothing fails: it works, on the wrong tree.
Queued: give `refreeze` the flag."* That is false. `refreeze` resolves its root **from the
current working directory** — `main` calls `resolve_harness_root(&cwd, ROOT_OVERRIDE)`
(`src/bin/refreeze.rs:1037`), which runs `git rev-parse --show-toplevel` and resolves a linked
worktree to that worktree. Its own module doc says so in as many words: *"WHICH TREE IT WRITES
INTO is derived from where it is INVOKED, not from where it was built."*

**Verified behaviourally, not by re-reading the source** — the same prebuilt binary
(`target/release/refreeze`, md5 `05a1f3c0501c94638d53bfa5620503b5`), `--check` only, from two
working directories:

| cwd | operates on | chain it read |
|---|---|---|
| the shared checkout | `sigil/crates/sigil-harness` | tip `ball-seating`, len 180 |
| `.worktrees/lane-c` | `lane-c/crates/sigil-harness` | tip `migmask`, len 51 |

It read a *different tree's* provenance chain purely from where it was standing, and printed
`THIS BINARY WAS BUILT FROM A DIFFERENT TREE THAN IT IS OPERATING ON` with both paths. So the
failure mode banked above — silent, wrong-tree — is the one thing it is loud about.

**Why the grep misled.** `--harness-root` is absent from `refreeze` and present in `repin`
because that flag is the **parent→child protocol**, not a safety feature one tool lacks:
`refreeze` *derives* the root and *passes* it to `repin` via `harness_root::ROOT_FLAG` (see
`root_args` / `repin_invocation`) precisely so the child cannot resolve a different one. Adding
the flag to `refreeze` would be adding a way to tell the parent something it already knows.

**And the fallback nearly landed tonight was actively dangerous:** making `refreeze` refuse when
`SIGIL_HARNESS_ROOT` is unset would break the *normal, correct* invocation — standing in the
tree you mean to freeze — and would have bricked an unattended aeon freeze at 3am for a hazard
that does not exist.

**What survives from the report, in its true and narrower form.** `SIGIL_BUILD` does default to
`<root>/target/release/sigil`, which a fresh worktree does not have — so aiming a prebuilt
`refreeze` at a dedicated worktree does need it set. That is **friction, not a silent hazard**:
`capture_goldens.sh` tests `-x` and exits by name (`ERROR: sigil build binary not at …`) — cited
by its message rather than its line, which moves.
Separately, the *aeon-side* tree — the one the hub's ordering rationale was actually about — is
already enforced hard: `resolve_aeon_rev` refuses `--freeze` unless `AEON_DIR` is set, is a git
repo, resolves `HEAD`, and is **clean**. The two trees are different questions and only the
sigil-side one was ever in doubt.

**The lesson, which is the part worth keeping.** This is *a peer's measurement vs their
mechanism*: aeon's grep counts were exactly right and reproduced here, and the causal story
attached to them was not. A mechanism travels further and faster than a count, because it
explains — this one reached a queue row, a hub ruling, and a dispatch order inside a day. Take a
peer's numbers; re-derive the cause. And note which check settled it: not a closer reading of
the source, but **running the binary in two directories and looking at what it said**.

**HOW THE WRONG MECHANISM WAS PRODUCED — the aeon lane's own diagnosis, banked because it is
the reusable half and it is not "they were careless"** *(aeon `1506cf43`, verified reachable at
their `origin/master` before citing)*. Chain 180's freeze ran with the cwd **and**
`SIGIL_HARNESS_ROOT` pointed at the **same worktree**. The two candidate causes — *resolves from
the cwd* and *resolves from the override* — were therefore held equal **by construction**, so
that run could not discriminate between them at all. The variable that got credited was the one
that had been deliberately set, which is the one anybody credits.

**That is bar 5 arriving on a causal story instead of on a number, and that is why it was not
recognised.** Bar 5 is written about *a suspiciously clean constant across varied inputs* — a
measurement. Here the inputs only looked varied and the output was an *explanation*, so nothing
about it pattern-matched to the bar that covers it. **Read bar 5 as covering mechanisms too:
before crediting a cause, ask which run could have distinguished it from its rival, and whether
that run was ever made.** The two-directory `--check` above is the command that separates them,
takes seconds, and was never run by either lane until the retraction.

**STATUS, so a successor does not cite this back to empyrean as contract.** This is adopted
**lane-locally, here, now**; the shared protocol's bar 5 is **unchanged**. The generalization is
queued at empyrean as **Q-35** in their pending-protocol-bars list and lands with their next
batch, per the owner's 2026-08-22 ruling that how-we-work notes batch rather than trickle. So:
follow it in this lane, and **do not tell another lane "bar 5 says"** until empyrean's file
actually says it. Re-check Q-35's state before citing it outward — a queued bar and a landed
one read identically in prose, which is the whole reason this paragraph exists.

**The operational form, agreed with aeon and adopted here: report the measurement, and flag the
cause as the softer half, explicitly, every time.** The asymmetry is not in how carefully each
is checked — it is in **how far each travels unchecked**. A count sits there being a count; a
mechanism travels *as a fact and gets built on*. This one reached a queue row in this file, a
hub ruling, and an authorized fix inside a day, while the counts it rode in on stayed exactly
what they were.

**2. `refreeze` outlives a 10-minute foreground cap, and `timeout 1800` does not help because
the harness clamps.** aeon's first attempt was killed mid-capture holding **five half-written
goldens**. Recoverable with `git checkout -- .` *in a dedicated worktree*. **In this lane's
shared checkout the same kill leaves five modified goldens sitting under whatever runs next** —
and a later `git add -u` would commit them, which is why that command is already barred here.

`REFREEZE-PARTIAL-WRITE` — **ADDRESSED on `parcel/refreeze-atomic-capture`, awaiting the
overseer's landing** (it shares a path with aeon's freeze lane, so it is not self-merged).
`golden/atomic_freeze.sh` is a staged commit `capture_goldens.sh --write` now writes through:
every captured ROM lands in `golden/.staging` and the committed blobs are untouched until all
seven have been captured, at which point the set moves into place with one rename each.

**The guarantee, stated as narrowly as it is true — it is NOT full atomicity.** A kill during
capture or staging, the multi-minute stretch, leaves the complete old set: no committed blob is
open for writing at any point there, so none can be truncated. A kill inside the *commit loop*
still leaves a MIXTURE — seven renames are seven operations — but no truncated blob, and the
window is milliseconds rather than minutes. That residual is made loud rather than closed: the
staging area survives such a kill holding exactly the blobs that did NOT land, under a
`.committing` marker, and the next `--write` refuses over it by name instead of capturing a
second set on top of a mixture. A leftover WITHOUT that marker is abandoned capture output —
the committed set is provably the complete old one — and is discarded rather than refused.
Closing the residual properly wants a `renameat2(RENAME_EXCHANGE)` directory swap; it is
priced in the parcel's report and is not what tonight wanted.

`crates/sigil-harness/tests/golden_freeze_atomicity.rs` drives the library against stand-in
blobs, since a gate that could only run inside a real seven-target capture would never run. Two
of its nine gates are CONTROLS asserting the bare-`cp` path really does leave a mixed set and
a truncated blob — the staged gates are otherwise assertions never shown capable of failing.

*(The built-from/operating-on warning fired for them and did its job; they discharged it by
checking that the only two commits since that binary was linked touch `golden/` data and no
Rust source, rather than rebuilding. Correct, and the check is per-run, never a remembered
result.)*

**A shell hazard from the same run, theirs, and it lands on this lane's habits too:**
`cmd | tail; echo $?` reports **tail's** status, not `cmd`'s. They nearly recorded a
`repin --check` verdict from an exit code measuring the wrong process — on the single number the
falsifier turned on. This document already bars `grep | head` on test output for hiding FAILED
lines; **the same pipe hides the exit code**, which is worse because it looks like a check.
Verify a push or a gate by a **positive artifact** (`git ls-remote` against the remote, the
named test in the log), never by an exit code read through a pipe.

`REFREEZE-STEP-GAP` — **ADDRESSED on `parcel/freeze-step-gap`, awaiting the overseer's
landing** (held behind the same freeze lane as the row above). The staged capture closed the
half-write *inside* step 1; this closes nothing, and deliberately — it makes the three joints
BETWEEN `--freeze`'s four steps (capture → sizes → pins → ledger) self-describing.

**Why the joints are the worse exposure.** A half-written blob is obviously damaged. A run
killed between two steps leaves every artifact individually well-formed: fresh goldens beside a
size table, a `pins.rs` and a `provenance.toml` that all parse and all describe the PREVIOUS
freeze. And for a byte-neutral `--supersede-tip` freeze killed at the last joint, the tree is
**byte-identical to one where the freeze never ran** — no CRC moved, so `--check` sees nothing,
and the entry that should have been appended is simply absent.

**The mechanism is a completion journal, not a transaction.** `crates/sigil-harness/src/
freeze_journal.rs` writes `golden/.freeze-journal` (git-ignored) before step 1, records each
step as it completes, and removes it only when the run finishes — its ABSENCE is the only
statement that a freeze completed. `--check` and `--attest` REFUSE over a leftover and print
which artifacts are fresh, which are stale, the interrupted command verbatim, and the exact
`git checkout` naming only the paths that moved. **`--freeze` does NOT refuse**: it announces
the leftover and replaces it, because that run regenerates every artifact the leftover names,
so a refusal there would obstruct the recovery. Nothing here can fire on a completed run.

**Full staging across all four steps was considered and rejected on evidence, not cost.** The
step graph is not what it looks like: `repin` reads `repin.toml` and a native resolve of the
aeon tree only — it consumes NOTHING from steps 1 or 2 — but `derive_offcanon` DOES open each
golden blob, for the `golden_crc32` / `assembled_anchor` header of every size table. So staging
the goldens would require teaching a third tool a staged-golden-dir override whose misuse
writes a plausible-but-wrong provenance stamp — the exact defect class this row exists to
prevent. And it would not buy atomicity: the commit becomes 15 renames plus a ledger append,
a *wider* mixed window than today's 7.

**The honest guarantee.** The inconsistent state is unchanged; what changes is that it is
nameable. A kill can still leave fresh artifacts beside stale ones — and a kill in the instant
between a step returning and its record being written leaves the journal UNDERSTATING progress,
which calls a fresh artifact stale (conservative: recovery regenerates it either way). An
unreadable journal is `COULD NOT MEASURE`, never "completed". Residual holes, both stated in
the module: a `capture_goldens.sh --write` run BY HAND outside `refreeze` is unjournaled, and a
`SIGIL_KILL -9` between the last record and the removal leaves a completed-run journal, which
is reported as a note rather than a fault.

`GOLDEN-HAND-WRITE` — the first of those two, **ADDRESSED on `parcel/capture-write-journalled`,
HELD from merging.** The script's own WRITE GATE refuses `--write` unless `SIGIL_GOLDEN_WRITE`
says which kind of write it is: `refreeze`, which the tool sets on the child it spawns, or
`unjournalled`, the operator's acknowledgement — which appends to `golden/.unjournalled-write`
before anything is built and is REFUSED if that record cannot be written. The refusal names the
journalled command, names the no-write capture most hand runs actually want, and names the
override; it is consulted before `AEON_DIR` and `SIGIL_EMIT`, so it costs nothing and is
provable without a provisioned aeon tree. No shell gate can tell a forged caller from a real
one, and the trace's ABSENCE proves nothing — it is checkout-local and untracked. What closes
is the one-flag slip, not the possibility.

**Why it is held.** It changes the measuring instrument a paired freeze runs through, so
merging it while an aeon byte-moving parcel is out would move the instrument under someone
else's landing. Byte-neutrality does not exempt it; the precedent is
`fix/island-non-vacuous-arm`, held for this reason and no other. The merge is the hub's to
sequence with the aeon lane.

`crates/sigil-harness/tests/golden_write_gate.rs` — 6 gates, and half of them are the half that
gets skipped: a COMPLETE seven-target `--write` running to `freeze_commit` under the journalled
caller's environment, because a gate proven only on its refusal is indistinguishable from one
that refuses the ritual too. The plumbing gate never names the variable — it runs `refreeze`
far enough to spawn its capture step, diffs the child's environment against a control child
given the same inputs, and runs the REAL script under what `refreeze` added.

`crates/sigil-harness/tests/freeze_step_gap.rs` — 20 gates, **one per joint** rather than one
over the set, since the three leave different wreckage. Three of them are CONTROLS asserting
the unjournaled sequence really does leave a mixed set with **no residue of any kind** to read.
Expectations come from the disk: what the journal calls fresh is checked against which
artifacts actually hold new bytes, so a mis-attributed step table fails these gates instead of
defining them.

### History — PRIOR rows, already lifted. Not the row above.

**LIFTED 2026-08-28T21:34:58Z** *(the chain-176 freeze hold — a different row, now closed;
kept for the precedent, not because anything is in force).* The aeon lane reported the freeze wave complete: ONE freeze,
chain 176 (`debug-rings-gate`, aeon_rev `55e0858f`, sigil `c84a98c2` on `origin/master`), and
four zero-byte parcels after it. The three binaries were still at their raise-time md5s when the
hold came off — `sigil 85ba502f…`, `refreeze 8cf597eb…`, `repin 37657e41…` — so the hold did the
job it was raised for, verified from both sides: the aeon lane quoted the same `sigil` md5 back
as their `SIGIL_BUILD` pin.

**Relinking is free again, but the announcement is owed at the RELINK, not at the lift.** Other
lanes build against these binaries; nothing changes for them until the bytes actually move. Tell
every lane when you relink, not when the row comes off — announcing a lift that changes nothing
spends attention and teaches lanes to ignore the next one.


### A DISCARDED BAD RUN IS A FREE POSITIVE CONTROL FOR THE GOOD ONE THAT REPLACED IT (2026-08-27)

An agent's suite runs before a mid-run correction showed **54 phantom failures**; after the
correction, a clean set. Rather than simply discarding the bad log, it ran **the same
failing-name extraction against it** — 54 names came back, proving the query was live, so the
empty result on the clean run was **a real absence rather than a broken query**.

The absent-instrument problem is normally solved by planting a defect to prove the query
works. **A run you have just invalidated already contains real instances**, and it is sitting
there. **Before discarding a bad run, ask what it is now a control FOR.**


### NEVER POINT TWO CONCURRENT AGENTS AT ONE `AEON_DIR` (2026-08-29, this lane's own error)

The documented rule is *never share one `CARGO_TARGET_DIR` between two worktrees*. **The aeon
reference tree needs the same rule and did not have it.** Two agents were dispatched within
three minutes of each other, each with its own worktree and its own target dir — and both with
`AEON_DIR=/home/volence/sonic_hacks/.aeon-hole-wire`. **Builds WRITE into that tree**
(`build.sh` produces `s4.bin`, `demo.bin`, …, and rebuilds `rm`-first), so one agent's build
transiently deletes the ROMs the other agent's strict gates read.

It surfaced as one failure in an otherwise clean 4090-test run:
`SIGIL_STRICT_GATE set but reference missing: …/.aeon-hole-wire/demo.bin`, green on a re-run
seconds later. **The gate was right both times**; the tree moved underneath it.

**Two things about it that generalise past the fix.**

**1. The perturbation is not one-directional, so "it passed on a re-run" is not the end of it.**
A shared tree can make a result look green as easily as red — a gate that should have refused
can find a stale ROM from the other agent's build and pass. Red is the *visible* face of this
hazard and green is the invisible one, so a single re-run resolves the instance and says nothing
about the class. **Ask which artifacts a measurement depended on, not just whether it repeats.**

**2. The reporting agent attributed it to "a peer lane" and that was wrong.** The observation
was exact — a build running, the ROM briefly absent — and the cause was *the controller's own
second agent*. This is the same measurement-vs-mechanism split as the `refreeze` retraction
above, arriving twice in one night from opposite directions: there a peer's mechanism was
wrong, here an agent's was. **An agent has no way to see its controller's other dispatches**,
so it will reach for the only actor it knows about. The fix is on the dispatching side, not the
reporting side.

**The rule: one dedicated aeon reference worktree PER AGENT, named for that agent**, created by
the controller before dispatch, and stated in the brief as exclusively theirs. Cheap — a
detached worktree is seconds and the disk is not scarce. `AEON_DIR` is already unconditional in
every brief; **"unconditional" was doing the work of "exclusive" and those are different
claims.**

### A FRESH AEON WORKTREE IS SOURCE-ONLY — THE STRICT SUITE IS UNREACHABLE ON IT (2026-08-29)

Root cause of the shared-`AEON_DIR` collision above, and the more useful half. A
`git worktree add --detach` of aeon gives you **source with no reference ROMs**. The strict
suite reads `s4.bin` / `s4.debug.bin` / `demo.bin` / `demo.debug.bin` **from that tree**, and
`engine/debug/generated/` does not exist either, so `demo debug` cannot even resolve. Measured:
a first strict run on a freshly-created reference worktree returned **213 failures, every one
`SIGIL_STRICT_GATE set but reference missing`**. After building the four shapes, the same binary
over the same source returned exactly **4081 / 0 / 4**.

**So handing an agent a bare worktree does not hand it a reference tree — it hands it a
build job**, and the agent has no choice but to write into the thing the brief calls a
reference. Two agents given the same bare tree therefore *must* collide; the collision was
designed in at dispatch, not stumbled into at run time.

**Practice, and it is the controller's work rather than the agent's:**

1. **Prepare the tree once, before dispatching**: create the detached worktree at the
   provenance `aeon_rev` and build all four shapes in it yourself.
2. **Then give each agent its OWN copy** (`cp -a` of the prepared tree, or its own prepared
   worktree). A prepared tree is ~51 MB per the agent that measured it; disk is not the scarce
   thing here.
3. **Say in the brief which it is** — prepared-and-exclusive, or bare-and-yours-to-build. The
   agent cannot tell by looking, and "do not rebuild the aeon tree" is an impossible
   instruction against a bare one. That instruction was issued here mid-run and was already
   unfollowable when it arrived.

**The generalisation worth keeping: `AEON_DIR` names a path, and a path is not a state.** Every
brief in this file has said `AEON_DIR` is unconditional. None has ever said what must be TRUE of
the tree it points at. Unconditional, exclusive, and *prepared* are three different claims, and
the briefs have been making the first while relying on all three.

**Corollary for reading agent evidence:** an agent that had to build its own references cannot
offer "I deleted the ROMs first, so their existence proves my build ran" as a freshness witness
if any other process could have rebuilt them. Where the binaries are byte-neutral and the source
is pinned the CRCs are identical either way, so the conclusion survives — but it survives on
*that* argument, not on exclusivity. Make agents state which.

### STATE A RULE AS A BOUND, NOT AS A PROCEDURE (2026-08-29, aeon's formulation)

*"A rule stated as a **procedure** invites the reader to run it further than it goes; a rule
stated as a **bound** cannot be over-run."*

The chain-181 false R7 in one sentence: **the aeon lane executed this lane's procedure
correctly, past its cap.** *"The largest power of two dividing the frozen base"* is a
procedure — you can run it to 32, 64, 128, and nothing in the wording stops you.
*"`packed_align_of` only distinguishes residues mod 16"* (`2026-08-26-config-b-two-byte-growth.md:182`)
is a bound, and **the false conclusion is unreachable from it.** Both sentences describe the
same function; only one of them can be over-run.

**This subsumes the parenthetical lesson rather than sitting beside it.** A buried qualifier is
what a procedure-shaped rule needs in order to be correct — the cap had to be bolted on because
the headline stated a method instead of a limit. State the bound and there is no qualifier left
to bury, nothing for a summary to drop, and no hop at which the meaning can decay. **The fix for
"my qualifier got dropped" is usually to stop needing one.**

Applies well past this instance: prefer *"never exceeds N"* to *"computed by doing X"*, and
*"only these five values occur"* to *"derived from the address"*. Where a procedure genuinely
must be given (someone has to reimplement it), give the bound FIRST and the procedure after.

### AN ENUMERATION THAT COMES BACK MOSTLY CLEAN IS A RESULT, NOT A NULL (2026-08-29)

Same episode, the other half. One bad restatement of R7 was found by a peer; sweeping the
population **by what touches the value** (bar 8) found five more, **all correct**. The
conclusion — *one bad summary, not systemic drift* — is a finding, and it is the one that
governed what happened next.

**The failure mode this prevents is over-correction.** Had the sweep not run, the reasonable
response to "your doc misled me" is to distrust the whole document, re-audit everything, and
warn other lanes off it. That would have been wrong, expensive, and would have spent the
credibility of five accurate sites to pay for one inaccurate one. **Report the clean count out
loud** — "five other sites, all correct" is what makes "fix this one line" the proportionate
answer instead of a doc-wide rewrite.

**And it cuts the other way too:** a sweep that comes back mostly *dirty* converts a
one-line fix into a class problem, which is equally a result. Either way the number is the
deliverable. What is never acceptable is fixing the reported instance and not looking.

### PUSH-BEFORE-ATTEST: ritual NOW, refusal ON A CONDITION (aeon's ruling, 2026-08-29)

**A revision already in `origin/master` cannot be orphaned by a later rebase.** So pushing the
freeze commit *before* `refreeze --attest` **removes** the orphaned-`sigil_rev` failure mode
rather than detecting it. Chain 181 is the instance that lacked it. **The aeon lane adopted it
as ritual from chain 182 onward** — it costs one `git push` in an order they were performing
anyway.

**`AHEAD OF REMOTE` at attest time stays a WARNING, and the refusal has a condition rather than
a date.** Their reasoning, which is this lane's own precedent turned around: making it a refusal
today would refuse a state that is *currently normal*, in the middle of an unattended overnight
freeze — **exactly the shape of the `SIGIL_HARNESS_ROOT` refusal this lane was told to build and
refused to**, which would have guarded a hazard that did not exist and bricked a 3am freeze.

> **The condition: when three consecutive chains have attested with the freeze already pushed
> and the warning has not fired once, `AHEAD OF REMOTE` has stopped being a normal state and
> should become a refusal. If it fires inside that window, the window restarts and we find out
> why.**

**A refusal is safe exactly when the state it refuses has stopped being normal — and that is a
measurable fact, not a judgement.** Keep the refusal from arriving ahead of the practice it
enforces.

**GATED FURTHER 2026-08-29 at aeon `04fcac05`** (verified reachable at their `origin/master`),
after this lane objected that the condition was REMEMBERED rather than measured. Their
withdrawal, their words: *"I claimed the property that would have made it safe, and the claim
was the only thing supplying it."* Warnings go to stderr and nothing durable records that one
fired, so checking the condition needs somebody to count three chains **and recall** whether any
warned. **The refusal now does not flip at all until the ledger can answer the question by
query** — until `ATTEST-RECORDS-REACHABILITY` lands. Their rule: *do not flip the refusal on a
count anyone had to remember.* Push-before-attest stands regardless; it prevents rather than
detects.

**The deciding instance was live in this suite tonight:** the aeon session was rotated **while
holding exactly that class of state** — the mapping write nearly happened twice and the
no-re-attest reasoning nearly died with the context holding it, avoided only because the fresh
session verified firsthand at boot. **A remembered condition is a population of one, kept in the
place that gets cleared.**

**⚠ THE CONDITION IS CURRENTLY REMEMBERED, NOT DERIVED — and that is the weak joint.** Checking
it today means a human counting three chains and recalling whether a warning fired, across
sessions that get rotated mid-flight (aeon's was, tonight, holding exactly this kind of state).
**Queued here as `ATTEST-RECORDS-REACHABILITY`:** have `--attest` *record* the state it observed
into the entry, so "three consecutive clean chains" becomes computable from the ledger by the
same walk that reports orphans. That converts a condition somebody must remember into one the
tool can answer — the same preference that killed the exception list.

### READ THE REMOTE, NOT THE TRACKING REF — the shared-machine rule arriving on a REF (2026-08-29)

Measured during the reachability parcel: **aeon's `origin/master` moved four times inside one
parcel** (`ac20c424 → a0a5acff → 4d86f5db → 734ab392`, and twice more after) while this lane's
prepared reference worktree's tracking ref still read the first. **An implementation that
trusted the tracking ref would have judged the whole ledger against a twenty-minute-old branch
and reported confidently.**

`git ls-remote` names **the remote**. A tracking ref names **whenever you last fetched**. Only
the first is what *"reachable from origin"* means, and **the two produce identical-looking
output**, which is why the reachability check tests this explicitly
(`the_git_oracle_reads_the_remote_rather_than_the_local_tracking_ref` moves the origin behind
the clone's back and asserts the tool answers `COULD NOT MEASURE … git fetch` rather than
trusting the stale ref).

This is the *shared-machine* rule — already banked here for **directories**, where reading a
sibling's working tree silently measures somebody's mid-edit state — arriving on a **ref**. Same
failure, same silence, different object: **a path is not a state, and neither is a ref.**

### A CHECK CAN BE VACUOUS BY CONSTRUCTION IN THE ENVIRONMENT IT RUNS IN (2026-08-29, aeon's)

`aeon/tools/level_staleness.py` asks whether the generated tree is current by comparing
`newest mtime(editor sources) > newest mtime(generated tree)`. **In a fresh worktree every file
is written within the same second, so the comparison is false by construction and the check
passes on any tree, current or stale.** It answers a question about a *filesystem* when the
claim is about *content*. Found by the aeon lane the same night their master went red with a
clean `git status` and a stale generated half — the exact condition the tool exists to detect.

**The discriminator, and it is one question: what input would make this fail?** If the answer
depends on a property of the *environment* rather than of the *subject* — mtimes, ordering,
whether a directory happens to be fresh — the check is measuring the harness. This is bar 2's
absent-instrument problem one level out: bar 2 catches a matcher that matches the wrong thing;
this catches a comparison that cannot be false where it runs.

**The fix shape is a content witness**: re-run the producer into a scratch location and compare
byte-for-byte against the committed artifact. Identical proves currency; different names the
stale files. It cannot pass vacuously. Aeon has made that a required, separately-reported line
on their re-bake parcel — *"the bake ran"* and *"the bake is current"* are two claims and only
the second failed.

**This lane has open instances of the same class, and they should be read together rather than
as unrelated tidying:** `CENSUS-EATS-PARSE-ERRORS` (a file it cannot parse silently shortens an
authoritative list — a failure rendered as a shorter clean answer), `PIN-CHAIN-ONE-INSTRUMENT`
(the tool that writes the address file and the check that guards it ask the same resolver, so
the check cannot notice that resolver being wrong — vacuity by shared instrument rather than by
environment), and `REPIN-VERBOSE-GHOST` / `PROSE-BOUND-SWEEP` (prose asserting behaviour nothing
executes). **Ask the discriminator question of each before fixing any of them**, and prefer the
content-witness shape to a cleverer comparison.

## Boot

> You're the overseer for this repo. Read `docs/OVERSEER.md` first, then
> `empyrean/docs/OVERSEER-PROTOCOL.md` if you haven't. Work the queue. Peers may or
> may not be running — check `ListAgents`; coordinate if present, proceed solo if not.

**Read the protocol at a committed revision, never through the filesystem path:**

```sh
git -C ../empyrean fetch -q origin && \
git -C ../empyrean show origin/main:docs/OVERSEER-PROTOCOL.md
```

`../empyrean/docs/OVERSEER-PROTOCOL.md` is one peer's live working tree, so reading it
by path means booting from somebody's uncommitted directory. Carried here because this
is the only file upstream of that read; the empyrean copy governs on any disagreement.
Taken from empyrean `274d26d2`.

**Re-read it mid-session.** That file currently moves faster than a session lasts — on
2026-08-22 it went from `cea2e57c` to `274d26d2` and gained two numbered bars inside
ninety minutes, both of which landed on this lane's in-flight work. Boot-time is the only
read anybody performs unless you make yourself perform another; re-read when a peer cites
a bar you don't recognise, before dispatching a wave, and at any landing.

## Landing-lane division — THE rule for this repo

The aeon↔sigil landing lane has **one owner, and it is the aeon overseer** (owner
directive, 2026-08-19). Aeon-paired byte-movers — golden refreezes, the provenance
chain, pin updates — land through aeon's session. One session sequencing them is why
a 16-refreeze day (chain 134→149) had zero collisions; a parallel refreeze against
shared goldens is exactly the collision this rule prevents.

**The sigil overseer owns everything sigil-internal:**

- the `.emp` language work (Spec 2 — specs in `empyrean/docs/SIGIL_*.md`)
- the CLI (`crates/sigil-cli`), the frontends, the backends, the linker
- the port-test harness (`crates/sigil-harness`) and its test-support seams
- sigil-only parcels: anything whose landing does not move aeon-paired bytes

**COORDINATE with the aeon session before touching any of:**

- `crates/sigil-harness/golden/` (and `golden/provenance.toml`)
- `crates/sigil-harness/src/pins.rs`
- `crates/sigil-harness/repin.toml`

If a sigil-internal change would ripple into those files, message the aeon overseer
(find it via `ListAgents`; address by repo, not session name) and let that session
sequence the landing. A byte-changing parcel ripples past `pins.rs` into
`engine.inc` / `mixed_dac_rom.rs` / `repin_pins.rs` — the repin tool auto-updates
only `pins.rs`; the rest are hand-edited, and `repin.toml` changes only when a
region is added. That whole ripple belongs to the aeon-owned lane.

Provenance identity is **CRC32 + size**, never SHA1 — the campaign standard.

## The autonomy directive — and its scope, which is the part that matters

Every sigil overseer session operates on a banked owner directive: **on assembler
internals the owner defers to the implementer — make the best technical/design call and
proceed; checkpoint at milestone boundaries.** Provenance, stated because it is load-bearing
and was until now unbanked: it lives in this lane's session memory
(`user-defers-sigil-technical-calls`, written ~2026-07-05, origin session
`9ff8029c`), recording the owner's own words — the internals are "out of my wheelhouse",
"make the best decisions". It is a real granting act, not a status field promoted into a
ruling. But **`git log -S` confirms it has never appeared in any sigil doc**, so until this
paragraph every cold boot took the whole posture from a memory file no other seat can audit.

**The scope boundary is narrower than "sigil technical calls", and reading it broadly is the
live risk.** The directive names *technical/design forks and encoding minutiae* — fold-vs-fragment,
where a shared rule lives — and explicitly reserves **direction and priorities** to the owner.
So:

**RULED BY THE OWNER, 2026-08-24, in his own words — and it is none of the three options he
was offered, so read the words rather than a key.** Verbatim: *"The language is yours, but
let's discuss it first and let me agree or not for the most part"*. Put to him as decision
`d-3`; recorded as `d-6`.

**What it means operationally.** The `.emp` language is this lane's to design and drive —
that is a real grant and is broader than the previous parked reading, which sent every new
surface to him as a blocker. It is **not** a licence to land language surface silently. The
shape is **propose, discuss, then land**: sigil does the design work and forms the
recommendation, puts it to him in plain terms before it lands, and he agrees or not. The
authority to design sits here; the agreement is his and is genuine, not a notification.

- **Discuss before landing:** anything that adds or changes a word, spelling or construct
  the game's source will be written in. That is the durable surface — once the game is
  written using it, it is close to unremovable, which is the whole reason he wants a look.
- **Just do it:** everything behind that surface. Implementation strategy, diagnostic
  wording, gate design, which of two sound encodings to use, how the compiler achieves a
  construct that already exists. This is `d-2` territory and gets a lane-log note if the
  call is notable, no discussion required.
- **"For the most part" is load-bearing and cuts toward less ceremony, not more.** He did
  not ask to approve every spelling. A trivial, obvious or purely-mechanical surface change
  does not need a round trip; use judgement, and when in doubt spend the one message. What
  he is buying is a veto on the shape of the language, not a queue of rubber stamps.
- **Do not convert this into a blocker.** A discussion is a message and an answer, not a
  `blockedOnOwner` entry with work stopped behind it — park it as a blocker only if it is
  genuinely holding a landing and he has not answered.

Live consequences of this ruling, both previously stuck: the closure-edge import spelling
that kills the two `CORPUS_OPEN_FINDINGS` rows is now sigil's to design and put to him, and
`pad_to(N)` (queue item 4) is sigil's to draft and put to him rather than a thing that waits
for him to raise it. Neither lands without his nod.

**`pad(N)` / `pad_to(N)` — RULED YES BY THE OWNER, 2026-08-26, directly in this lane's
session.** Put to him as `d-12`; his answer was a bare **yes** against the recommendation, so
the ruling is the **`both`** option: `pad_to(N)` derives the filler width from the next field's
offset, AND the plain `pad(N)` reserve-N-bytes spelling stays for the case where the count is
the author's real intention, with a diagnostic that names the mistake and prints the exact line
to write when the deriving form was clearly meant. Closed as `d-13` (rule 8c supersession).
`(align: N)` stays mandatory and is not the derivation's casualty — that is §1.3 of the draft
and the argument the whole design rests on.

**This retires the provenance caveat the draft carries in its own header.** That document
records its authority as an *empyrean* overseer's ruling made under a delegation reported in
the owner's words, flagged there as *"this lane did not witness that utterance"*. It is now
witnessed firsthand here, so the construct no longer stands on a relayed grant. The draft text
itself is unaffected; only its authority line is.

**The spec text is LANDED and un-DRAFTed at empyrean `2000b5ca03592377ea1881671db53e03ad36f264`**
(reachable from their `origin/main`, verified here after their push, not from the local tracking
ref). D2.37 is `(align: N)`, D2.38 is `pad`/`pad_to`, §4.3.1 carries the construct text lifted
verbatim from this lane's draft. **Cite that SHA from the implementation parcel.**

**What the review of that text bought, and the debt it left here.** The spec had carried, from
this lane's own field-align packet §10, the claim that the six `(align: N)` diagnostic strings are
a cross-repo interface aeon fixtures assert on. **They are not: none of the six appears anywhere in
aeon** — not in the 44-case `tools/emp_expect_fail.py` negative-build lane, not in the poison
modules, nowhere. Checked here and independently by the hub. It was a name doing a behaviour's
work, and it would have frozen six strings against a consumer that does not exist while telling
every reader a gate protected the wording. The producer side is thinner than the spec implied too,
measured string by string at `db2dacce`: #1 well pinned; #2 only on `must be a power of two` (the
parenthetical free); #3 only on ``asserts its alignment with `(align: N)` `` (the leading clause
free); #4, #5 and #6 pinned by nothing in the workspace.
**Owed by this lane, in the `pad`/`pad_to` parcel:** tests for #4-#6, widen #2/#3 to the full
strings, and pin the Scope clause (true by construction — the check walks only this struct's own
fields — but unpinned, and marked as such in the spec). The hub strengthens that clause on a
message from here once it lands.

Sequencing, so nobody lands half of it: the spec text is **empyrean's to land** — sigil does not
land `.emp` language spec, `SIGIL_SPEC2_LANGUAGE.md` is their file — lifted verbatim from
`docs/superpowers/notes/2026-08-26-pad-to-spec-draft.md`. **Nothing is implemented**; no crate
was touched by the parcel that wrote the draft, and `pad_to_cycles` in `t40_cycles.rs` is an
unrelated cycle-padding construct that shares a prefix and nothing else. Implementation is a
separate parcel and is this lane's.

**RE-CONFIRMED BY THE OWNER, 2026-08-24, directly in this lane's session — not relayed.**
The 48-day-old grant was put to him as decision `d-2` in `docs/decisions.jsonl` and he chose
**keep it, and tell me the notable calls as they happen**. So the authority is unchanged and
a fresh boot does NOT need to re-ask it. What is new is the obligation attached to it, below.

**The obligation: a notable call gets written down for him WHEN IT IS MADE.** Before this
ruling nothing was written down for him at all, which meant the first he heard of an internal
design choice was when something built on it went wrong. The vehicle is `docs/lane-log.jsonl`
(`empyrean/contract/LANE_LOG.md`), which is already the owner-facing feed — do not invent a
second channel. The bar for "notable" is **a choice a reasonable person could have made the
other way and would care about having made**, not every fork in the road: a spelling that
will appear across the game's source, a tier decision (error vs warning), a soundness
trade, an option deliberately refused. Routine implementation strategy stays unwritten;
logging everything would restore exactly the volume the lane log exists to replace.
**Write it at the time, not at the landing** — a call reconstructed afterwards is the
confident-guess-wearing-a-record's-clothes defect that ruling forbids for the log generally.
The scope line itself (`d-3`) is still open and is the one that gates work.

**Do not fund work off the broad reading of the scope line while `d-3` is open.**

**PUSHING IS STANDING-APPROVED — owner ruling, 2026-08-24, direct in this lane's session.**
Put to him as decision `d-1` in `docs/decisions.jsonl`; he chose **send finished work up
whenever it is ready**. So a push needs no per-time approval and a fresh boot must not
re-ask. The reasoning he was given, which is also the scope: pushing only ever ADDS, and
until work is at the remote no other lane can reference it at all, so holding it back
protects nothing and only delays consumers. That is the whole grant — **it authorizes
fast-forward pushes of finished work, and nothing else.** A history rewrite (force-push,
rebase or squash of already-pushed commits) is a different act with a different blast
radius, it is not covered here, and it still goes to him. Verify every push against
`git ls-remote origin refs/heads/master`, never the local tracking ref, which is the only
check that distinguishes "pushed" from "looks pushed".

The history this replaces, kept because it is why the question existed: the 2026-08-22 push
to `a70e6644` was banked as "owner-approved" for that ONE push, and the preceding 80-commit
unpushed backlog was described by the aeon lane as "the owner's gate", i.e. deliberate. The
ambiguity was real, and it cost two sessions the same re-derivation before it was asked
plainly.

## Dispatch practice — a stated MECHANISM is more dangerous than a stated FACT

*(2026-08-22; the oracle lane's formulation, from an episode this lane caused.)* A brief's
factual claims compete with the agent's evidence and lose when wrong. A brief's
**explanations do not compete — they absorb.** An agent that measures something
inconsistent with its controller's stated mechanism will tend to reconcile the measurement
to the story rather than report the conflict, and **an agent has almost none of the standing
a peer overseer has to push back.** So label mechanisms in a brief as hypotheses, and say
outright that the agent's own command output outranks anything the brief asserts.

**The cheap mechanism, with this lane's measured hit rate.** Every dispatch's deliverable
section ends with a required line: *"and anything in this brief you concluded was wrong."*
On 2026-08-22 that produced a correction in **3 of 3** dispatches:

- `feat/field-align` overturned the brief's **central design ruling** — the auto-padding
  question was already settled by spec §4.3, dispatched as open because the controller had
  not read the spec.
- `fix/rom-sentinel-port-tests` refuted the brief's implied **cost** argument for excluding
  gates from the nightly lane (cost is not the obstacle; committed-artifact oracles are) and
  corrected the booked count from ten to eleven.
- `docs/readme-refresh` corrected a flat **factual** error (`SIGIL_BUILD`/`SIGIL_EMIT` do not
  live in `test_support.rs`) and declined to document an env var the brief implied was
  required, having verified it is not.

Pair it with an explicit invitation where the brief carries a design position — the
field-align brief said a contradicting delivery is worth more than a complying one, and got
the sharpest pushback of the three.

**Honest limits, so this is not read as a solved problem:** n=3, one day, one lane, and all
three briefs were checkable against a tree, which is the easy case. A self-report line
cannot surface what the agent never thought to question — it catches conflicts the agent
noticed and would otherwise have swallowed, which is a real but bounded win.

## Quality bars

**⚠ THE REFERENCE TREE IS WHATEVER `golden/provenance.toml`'s TIP PAIRS WITH — and every
CRC written into THIS file is a snapshot that a refreeze silently invalidates**
*(2026-08-26, and this overseer caused it at a dispatch)*. The bar paragraph below used to
carry four CRCs and an aeon SHA. A refreeze landed at `029868e5` (chain entry 165, paired
with aeon `b08b35c0`), moving the two s4 goldens — and nothing updated the paragraph, because
nothing can: a prose CRC has no trigger. The next dispatch verified `.aeon-landing`'s four
artifacts **against the numbers in this file**, found them matching, and declared the tree
good. It was a real check with a stale oracle, so it returned green while pointing the parcel
at the wrong aeon revision; the agent lost a run to **71 failures reading exactly like a
byte-moving regression** (`Ground_Move_Cap resolved to 0x10902, expected 0x10912`) and settled
it only by re-running the same gate against master's own sources and getting the identical
failure. Note the shape: this is the *derived-never-copied* bar (#1) aimed at the reference
tree rather than at a test expectation, and it is the failure mode this document already warns
about one section down — *"Read the TIP of `golden/provenance.toml`, never a number in this
file"* — which did not save the dispatch that wrote it.

**So: derive the reference tree, never read it off a note.** The demo shapes matched through
the whole incident, so a partial match is not a witness:

```sh
tail -40 crates/sigil-harness/golden/provenance.toml   # the TIP entry = the four expected CRC32+size
git -C <aeon-tree> rev-parse --short HEAD              # must be the SHA that tip entry pairs with
```

All four must match on **CRC32 + size** (never SHA1 — the campaign standard) before a run is
worth anything.

**Which reference worktree is current is NOT recorded here, deliberately — ask the disk.** This
paragraph twice carried a named worktree and a SHA, and both rotted at the next refreeze while
reading as fact. The candidates are `~/sonic_hacks/.aeon-landing` and
`~/sonic_hacks/.sigil-portfix-aeon`; which of them pairs with the tip changes every freeze, so
derive it:

```sh
for d in ~/sonic_hacks/.aeon-landing ~/sonic_hacks/.sigil-portfix-aeon; do
  printf '%s %s\n' "$d" "$(git -C "$d" rev-parse --short HEAD)"
done                                                   # then CRC32+size all four ROMs in it
```

`~/sonic_hacks/.aeon-sigil-gates` is never a candidate — it is source-only by construction and
deletes built ROMs, per the source-gate lane section.

**This is now gated, as of 2026-08-26 — and it was not the one-afternoon gate it looked
like.** Historically no gate witnessed that `AEON_DIR` matched the provenance tip, because
`provenance.toml` had **no field naming the aeon revision a freeze pairs with**: at chain 166
the schema was `name` / `ab` / `note` plus the per-target CRC rows, and the aeon SHA appeared
only inside the free text of `ab`/`note` (16 of the 166 entries carry no `note` at all). Every
check of the pairing to date was a human reading prose, and a parcel lost a run to exactly
that. What shipped:

- **`Entry.aeon_rev`** — a full 40-character SHA (never abbreviated), typed `Option<String>`
  with `#[serde(default)]`, so the 166 historical entries keep parsing; they are
  **deliberately not backfilled**, since a prose-derived SHA is a reconstructed record. The
  `Option` is load-bearing: `None` means the KEY IS ABSENT (an older `refreeze` wrote the
  entry) while `Some("")` means somebody blanked it, and only the first is legitimate.
- **`refreeze --freeze` refuses unless it can name the revision honestly** — `AEON_DIR` unset,
  not a directory, not a git repo, HEAD unresolvable, or the tree **DIRTY** all refuse, before
  anything is built, each naming the variable, the path and the fault. This is not new policy:
  the landing lane below already required freezing from a clean checkout of a committed SHA.
  It was simply unenforceable, and unset silently fell back to the owner's live tree via
  `capture_goldens.sh`'s `${AEON_DIR:-…/aeon}`. **`--check` is unaffected and still takes no
  aeon tree.**
- **`provenance_chain::aeon_dir_matches_the_provenance_tip`** — compares `AEON_DIR`'s HEAD
  against the tip's `aeon_rev`. **Hard under `SIGIL_STRICT_GATE=1`; a loud `notice:` line
  otherwise.** Deliberately not hard in both modes: aeon master routinely runs ahead of the
  frozen tip for byte-neutral reasons (at the time this shipped it was two commits ahead, both
  documentation-only, and four by the time it was reviewed), so an unconditional assertion
  would be red on trees that are byte-correct in every way. Strict is where the bar belongs —
  it *is* the pre-merge run.
- **AEON-REV-WELL-FORMED** in `provenance::check` — an entry carrying an `aeon_rev` at all must
  carry a full 40-char SHA, **wherever it sits in the chain**. Hard in every mode.
- **AEON-REV-MONOTONIC** in `provenance::check` — once any entry names its aeon revision, no
  later entry may omit it. Hard in every mode. These two are the rules with teeth today.

**⚠ THE BOUNDARY IS DERIVED FROM THE CHAIN, NEVER PINNED TO AN ENTRY NUMBER — and this was a
real defect caught in review, not a style preference.** The first implementation used a
`AEON_REV_FROM_ENTRY = 166` constant. The aeon lane refreezes by running sigil's `refreeze` out
of **sigil master**, so a byte-moving refreeze landing before the field shipped would append a
field-less entry #167 that was entirely legitimate when written — and the pinned rule then
turned master red on somebody else's correct work the moment the branch merged. Measured, not
reasoned: on a chain carrying exactly that entry the pinned form failed **two** tests
(`provenance_chain_holds` and the pairing gate) while the derived form passes the identical
file. That is the failure this whole mechanism exists to prevent, aimed at ourselves. If you
ever find yourself wanting a number here, that is the trap.

**The one tolerance, and its disarm condition:** while the tip carries no `aeon_rev` the
pairing gate prints a `ratchet:` line and passes *in both modes*. `refreeze` appends nothing
when nothing moved, so no parcel can force the field onto the tip; failing closed would put
master red under the full-suite bar indefinitely. **The ratchet disarms permanently at the
next refreeze that names a revision** — the condition is the field's absence, never an entry
number. It prints `ratchet:`, never `skip:` — the bar
below requires zero `skip:` lines and this is not a missing reference.

- **Full suite bar** — run it as `scripts/landing-run.sh --baseline <N> --aeon <clean>`, which
  carries every requirement below inside one command span and REFUSES rather than degrading
  when one is missing. The hand-spelled equivalent, which is what the wrapper runs:
  `SIGIL_STRICT_GATE=1 AEON_DIR=<clean> cargo test --release --workspace --no-fail-fast -- --nocapture`,
  with `AEON_DIR` a tree matching the provenance tip (derive it — see the warning above) —
  **3990 passed / 0 failed / 4 ignored** (3994 declared), **zero `skip:` lines**, exit 0,
  clippy `-D warnings` exit 0.
  **The env vars belong INSIDE the command span, and this paragraph had them outside it
  until 2026-08-27.** That is not formatting: a reader copies the backticked command and
  gets the prose-stated requirement only if they read two lines further. It is the exact
  defect that cost aeon two unverified chains from their own landing lane, found in their
  file and swept here immediately after.
  Last measured at master `2e284c4d` against aeon `ac57991d` (chain 171), 2026-08-27, by the
  aeon lane's landing run of the seam1 re-pin.
  **This paragraph said `3939 / 3943` for a day while the RELAYOUT-REVIEW section below
  recorded `3943 / 3947` from the same tree, and the stale half is the one a cold boot quotes
  — this overseer quoted it to a peer, who reconciled against `git grep -c '#\[test\]'` and
  refused it.** That is this document's own trigger-less-prose defect, one section below the
  paragraph warning about it. **Reconcile against the declaration, never against this
  sentence:** `git grep -c '#[test]' HEAD -- '*.rs'` summed is the count, and
  `passed + ignored` must equal it.
  **The run emits EXACTLY ONE `ratchet:` line and that is the correct state, as of the
strict-attestation landing (2026-08-27).** It reads *"no entry in this chain records a
strict run yet … the strict-attestation rule is not yet in force"*, and it disarms
permanently at the aeon lane's first `refreeze --attest`. Do NOT read it as the old
`aeon_rev` pairing ratchet returning — that one disarmed at chain 167 and its reappearance
would still be a defect. **Two self-disarming ratchets now exist and they say different
things; read the sentence, not the word.** Once the first attestation lands, this count
returns to zero and any `ratchet:` line is again worth investigating.

*(Superseded: "The run now emits ZERO `ratchet:` lines and that is the correct state")* A `ratchet:` line reappearing means a tip was written without the field, which
  `check`'s monotonic rule should already have refused; investigate rather than tolerate.
  **⚠ DO NOT COMMIT WHILE A LANDING RUN IS IN FLIGHT.**
  `version_reports_the_head_of_the_tree_it_was_built_from` compares the binary's baked-in
  revision against the checkout's HEAD *at assertion time*, so a commit landed mid-run fails
  it — the binary is honest, the tree moved. Cost here: one full re-run at a landing. The
  diagnostic names both causes and the command that separates them, and the log stamp is what
  makes it legible; without `head=` in the header it reads as a mysterious one-test red.
  **The count is the bar; the pairing is a
  timestamp, not an instruction** — a later freeze moves the aeon SHA and leaves the count
  alone, so reconcile a parcel's delta against `git grep -c '#\[test\]' HEAD -- '*.rs'` and
  read the pairing only as "this number was last seen on that pair".
  **The previously recorded bar of 3881/3885 was 4 LOW and had been for some time** — measured
  at `67575c3c`, master's own sources run 3885/0/4 against 3889 declared, and
  `git grep -c '#\[test\]'` agrees with the run, not with the old note. A parcel's delta is
  therefore reconciled against `git grep -c '#\[test\]' HEAD`, never against the number in this
  paragraph; the pad parcel's +43 matches the declared-count delta exactly, where the stale bar
  implied +47 and would have manufactured a four-test discrepancy that does not exist.
  **⚠ THE REFERENCE CHECKOUT MUST CARRY NO `sigil`/`skdisasm` SYMLINKS.** The aeon main tree
  has none, and adding them makes `section_row_fixture`'s tree mirror die with
  *"the source path is neither a regular file nor a symlink to a regular file"* — three gates
  red for a reason that has nothing to do with the parcel under test. The old worktree-seeding
  note that says a fresh aeon worktree needs them is STALE; `./build.sh` works without them.
  Ledgered: the mirror should skip or name a non-regular entry instead of `unwrap`ing.
  *(History 2026-08-24→26: 3844/5/4 → §4 → 3852/5/4 → closure 3854/3/4 → first real scene
  3852/5/4 → SECTION-ROW → CLOSURE-2 3866/3/4 → REPIN-END 3869/1/4 → FIVE-REG 3870/0/4 →
  NIGHTLY-GAP 3872/0/4 → aeon ring-sparkle 3870/2/4 → BGROOM-3 + RINGS-ENV 3881/0/4.)*
  **BGROOM-3 landed** (`fix/measure-at-packed-base`): every measuring round is exact at its own
  bases (`resolve_layout_measuring` in sigil-link, fixpoint; scratch/spread fallbacks deleted,
  which also removes the ~0x400 growth cap — 5000 B of growth now builds with drift warnings),
  and non-convergence names the width-flipping sites file:line with both encodings. **The aeon
  lane's reported MECHANISM was refuted while their measurement stood:** not "abs.w because the
  provisional base is unknown" (an unresolved operand is a hard error) but the collision-fallback
  scratch slot aliasing zero — `collision_data` at slot 41 = `0x300_0000`, masked to 24 bits by
  `asl_width_rule` → `0x0`, where `abs.w` is legitimate. Their `lea (X).l` pins are a SUPERSEDED
  WORKAROUND, not a style rule; they un-pin once the shared binary reports a master containing it.
  **CONFIRMED IN THE FIELD 2026-08-26** (aeon `bc95e32e` and its parent): they un-pinned
  `SolidityTable`/`AngleTable` back to unsized `lea` and all four shapes stayed byte-identical
  (`b96319e3` / `7be32302` / `bf2cdb42` / `62a0019e`), after verifying the shared binary
  themselves rather than taking a quoted SHA. Their own finding, kept because it is a trap for
  the next reader: the THIRD site, `movea.l #{ptable}`, was never a width pin — `({ptable}).l`
  does not parse in a template arg, so it was always an immediate, and `movea.l`-immediate vs
  `lea`-abs.l are both 6 bytes but different opcodes, so "finishing" the un-pin moves the CRC at
  identical size for no gain. It stays `movea.l` with a comment saying why.
  **RINGS-ENV landed** (`fix/rings-contract-env`): the port rigs bind the game's real contract via
  `test_support::game_contract_env_from_aeon` (interface + the profile's own `config/game.emp`,
  path derived from `GameProfile::game_root_rel`), replacing hand-written stub strings that could
  not see a new hook. `game_contract_env_coverage` parses the member COUNT from the contract, so
  the next hook cannot repeat it. `tranche5_negative_probes` keeps its synthetic stub by intent
  (it probes the binder over a define matrix no manifest spans); `camera_port` is split.

  **FIVE-REG landed:** the last red was `soundbankhead_pinned_bootstrap_lands_at_lma_not_vma`
  on the **PinnedBaked/registry** path (`build_emp(sonic4_pinned_profile)` → `emp_map_toml`
  mints one region per REGISTRY pin; a content-derived section declared by a `section:` row
  has no `pins::Region` by design and `repin` derives pins from the shipped resolve, so that
  path can never learn it). Every PinnedBaked reference enumerated: two `#[ignore]`d archaeology
  tests, `derive_offcanon --bootstrap-canonical` (documented unavailable since 2026-08-01), and
  an unreachable arm behind the Frozen early-return. The catch was RE-HOMED, not deleted:
  `soundbankhead_pin_is_the_lma_not_the_vma` asserts `pins::SOUNDBANKHEAD` bases == the shipped
  resolve's lma with a distinct vma present (literals removed, red-first on a sabotaged pin).
  Test-file `LOCK` is poison-tolerant (3/8 repeats had poisoned a sibling; 8/8 clean after).
  Ledgered: retire the whole PinnedBaked path (kill list in the gap ledger).

  **REPIN-END landed:** `repin.toml` `start`/`end` accept `section:<name>` (LMA / own
  `lma + image_len`, one derivation `native::section_end` shared with the `<Base>_End` sites);
  `act_descriptor` and `scene_registry` ends re-spelled; `ACT_DESCRIPTOR` plain_len 0x27C→0x27A,
  debug_len 0x280→0x27A, bases unchanged, no other constant moved, `repin --check` a fixed point.
  A bare `end` that sweeps placer pad is now WARNED per shape by `repin` — **79 region/shape
  pairs** carry pad by convention (boot … player_climb, 0x2..0x20), green under the align-pad
  tolerance, ledgered for region-by-region conversion. `scene_registry` was NOT affected (0xACE
  flush both shapes; the 0xB1C was pre-repoint history).

  **CORRECTED 2026-08-26 (aeon's closure agent, reproduced in worktrees; branch verified here as
  2 test files, +20/-1): "cause is aeon-side, the RESERVED SLOT" is WRONG for four of the five.**
  (A) `unknown function ojz_act1_act_default / ojz_act1_sec_scene` is a SIGIL HARNESS gap: the
  four act tests build the descriptor with single-file `lower_module` + a hand-listed
  `with_ambient` set that resolves no `use`; every other cross-seam name rides as an AS equ but
  these two are `pub comptime fn` and cannot. `sigil build` never saw it. Fix on sigil branch
  `parcel/sigil-red-closure` @ `75802f6a` (worktree `~/sonic_hacks/sigil-wt/sigil-red`, based on
  `7bc50e41`, local-only): parse the generated `effects_scenes.emp` and ride its items ambient.
  Queued as `CLOSURE-FIX`, lands after the §4 parcel (shares the landing checkout).
  (C) UNMASKED by (A): the `act_descriptor_port` pair then fail
  `section.bytes.len() == pins::ACT_DESCRIPTOR.plain_len` — emitted 0x27A, pinned 0x27C. The
  805370b1 refreeze put `OJZ_Sec0_Blocks` 2 bytes past the descriptor's end on an alignment
  boundary and `repin` measures start..next-label, so the SUCCESSOR'S PAD entered the pin.
  Same "gap between labels is an allotment" family as the bganim 2-byte slot, now in the pin
  generator. Queued as `REPIN-END`: repin measures the section's own end (preferred) or the
  gate tolerates trailing fill. `pins.rs` is not to be hand-edited for it.
  With (A) applied: 3846 / 3 / 4 (their run, clean aeon 415e0b6a); the three = the pair on (C)
  and `soundbankhead` on (B) below. Aeon CRCs unchanged in all four shapes.
  **Also: this correction has itself gone stale twice since it was written, which is the
  point.** It named a fresher tip in prose; two refreezes later (chains 165 and 166) that
  tip was wrong again, and a fresher-looking number reads as more trustworthy than the one
  it replaced. **No CRC is quoted here any more.** Read the TIP of `golden/provenance.toml`,
  never a number in this file — including a number in this file that presents itself as a
  correction to another number in this file.
  **Re-split 2026-08-26 with the aeon lane:** the two messages are two owners. `unknown
  function ojz_act1_act_default / ojz_act1_sec_scene` is aeon's (closure of their generated
  module under this harness). `section ojz_effects_editor_act1 has no region in the map`
  (`sigil-frontend-emp resolve/mod.rs:849`) cannot arise on the Frozen path `sigil build`
  takes — `emp_map_frozen` mints a region per present section — so it comes from whichever
  of the five tests walks the **PinnedBaked/registry** path, and is a SIGIL registry question
  (a registry row, or the test's synthetic entry not reaching the module). Aeon will NOT land
  a map row for it; do not ask them to. Queued here as `FIVE-REG`.
  **§3.5 confirmed live 2026-08-26:** aurora's first real scene save made `ojz_effects_editor_act1`
  emit, and `[map.order-undeclared]` named `EditorSceneBinding_OJZ_Act1_Sec0` — the BINDING TABLE,
  which changes with content. Aeon wrote that label as an interim `order` row (uncommitted, lands
  through their lane), to be replaced by the `section:ojz_effects_editor_act1` row. Queued as
  `SECTION-ROW` (S, after the §4 parcel); sigil specs it, aeon lands the map side.
  **The nightly source-gate lane does NOT cover these** — none of the five is in
  `SOURCE_GATES` (they are artifact-oracle gates, deliberately excluded), so the lane ran
  green at 05:17 while master was red. That exclusion was correct for byte-divergence red;
  it is wrong for a HARD ERROR like a missing map region, which no refreeze clears — check that, not just the
  totals: a reference gate that skips reports nothing and reads as coverage.
  **Reconcile the total against the tree, not against the last remembered bar:**
  `git grep -c '#\[test\]' HEAD -- '*.rs'` summed gives the declared count, and
  `passed + ignored` must equal it (3839 + 4 = 3843 here). Baseline arithmetic carried
  across branches measured on different reference trees does not reconcile and will
  invent a discrepancy that is not there. Never plain
  `cargo test`: without `--release` some gates are impractically slow, without
  `--workspace --no-fail-fast` a wedge or an early failure hides the rest of the
  result set. Report failures-first with explicit pass/fail counts; never
  `grep | head` test output (it buries FAILED lines).
  **SECTION-ROW LANDED both sides 2026-08-26:** sigil `8566f962` (`"section:<name>"` order rows;
  packet `2026-08-26-section-row-packet.md`), aeon `058ad606` (map.toml:118 now
  `"section:ojz_effects_editor_act1"`, byte-neutral, verified at aeon origin/master). The
  sigil fixture gate `both_spellings_of_the_section_row_build_the_same_rom` is direction-agnostic
  and ran 3/3 green against the landed map. `.aeon-landing` moved to `058ad606` (same ROM bytes
  as `0e34408d`).
- **THE PAIRING GATE IS ARMED — the ratchet disarmed at chain 167 and will not re-arm.**
  `aeon_dir_matches_the_provenance_tip` is now a live assertion in both modes, and its
  first real execution was exercised here both directions rather than assumed: pointed at
  `.aeon-landing` (aeon `893747f7`, the tip's own `aeon_rev`) it passes with **no ratchet
  line**; pointed at the genuinely stale `~/sonic_hacks/.sigil-portfix-aeon` (aeon
  `b08b35c0`) it FAILS, naming both revisions, the entry number, the parcel and the exact
  remedy. That second case is this morning's incident — a real check run against a stale
  reference tree, returning green — and it is now impossible to have silently. **A stale
  reference worktree is a hard failure from here, not a thing to notice**, so a run that
  refuses is telling you `AEON_DIR` is wrong, not that the parcel is.
- **`aeon_rev` answers WHICH TREE BUILT THESE BYTES — never WHICH PARCEL MOVED THEM.**
  `name` and `ab` are the attribution; `aeon_rev` is the provenance, and the two point at
  different commits whenever a freeze is retried on top of a later aeon revision. Chain
  entry #167 is the first where they diverge by construction: the showcase parcel moved the
  bytes at aeon `9dd52471`, but two freeze attempts failed on latent defects and the
  succeeding freeze runs from a zero-byte gate-fix commit stacked above it, which is what
  `aeon_rev` honestly records. **A reader who takes `aeon_rev` as attribution finds the
  bytes moving at a commit whose diff moves nothing**, and the record looks self-
  contradictory while being exactly right. The entry carries the attribution in words for
  that reason. Do not "simplify" these into one field, and do not backfill one from the
  other.
- **Pre-merge:** re-run the suite on the merged tree with `SIGIL_STRICT_GATE=1`, which
  turns reference-tree skips into failures so the port gates cannot silently skip.
- **A suite log does not name the tree it ran in — stamp it, and prove the landed code
  is IN it** *(2026-08-22, caught at a landing)*. Cargo prints no cwd, no branch and no
  HEAD, so a run launched from the wrong directory produces a log that is green,
  plausible, and about somebody else's branch. Precedent, this repo, this lane: a
  landing run of master reported **3857 passed / 0 failed / 2 ignored, exit 0** — a
  *higher* number than the bar, which reads as strictly better news. It was another
  agent's worktree: the log contained that agent's in-flight `m68k_capstone_differential`
  and contained **zero** occurrences of `version_provenance`, the parcel actually being
  landed. Nothing in the output said so.
  Two mechanisms, both cheap, and the second is the one that cannot be fooled:
  **(1)** stamp the log before cargo writes to it —
  `{ echo "### pwd=$(pwd)"; echo "### head=$(git rev-parse HEAD)"; echo "### branch=$(git branch --show-current)"; } > "$LOG"` then append the run;
  **(2)** `grep -c` the log for a test name **unique to the parcel being landed** and
  require ≥ 1. A landing whose own new tests do not appear in its own green log did not
  happen. This is why the reconcile-against-the-tree rule above is load-bearing rather
  than bookkeeping: `passed + ignored` equalling the declared count was the *only*
  signal that separated the bogus run from the real one (3857 + 2 = 3859 ≠ 3823; the
  correct re-run gave 3819 + 4 = 3823 exactly). Aggregate greens do not self-attribute.
  **Second precedent, 2026-08-22, and it is the nastier face: the numbers were EXACTLY
  RIGHT.** A parcel agent's first suite run came back `3821 / 0 / 4` — the bar of the day,
  to the test — stamped `branch=worktree-agent-a19f801c9ae459301 head=8884e255`. The work was
  still uncommitted and the worktree carried its auto-generated branch name, so the run had
  measured *master*, faithfully, and reported a number that reconciled perfectly against the
  declared count. Where the first precedent announced itself as suspiciously good news, this
  one announces nothing at all: a bar-matching green on a parcel that changes no test count
  is exactly what a correct run looks like. **The stamp is the only thing that caught it**,
  and it caught it on the `branch=` line, not the totals — which is the argument for stamping
  branch and head rather than just cwd. The agent renamed the branch, committed, and re-ran.
  Note the trigger: this is a hazard of the *deliver-on-a-named-branch* workflow itself, since
  an agent that has not yet renamed its worktree branch is in this state by default.
- **GENERAL-GROUNDS DOUBT IS NOT A FINDING — confidence is contagious and that is a
  defect, not calibration** *(2026-08-26; the aeon lane's own naming of something it did,
  kept because the honest half is the useful half)*. Having had two of its claims fail in
  one evening, that lane pre-emptively lowered its confidence in a **third, unrelated,
  sound** claim — "44 rows moved, 0 quantum changes" — on no evidence about that claim at
  all, and asked this lane not to spend an agent refuting it. The line was correct, and
  re-derivation upheld it. The cost of the discount is real and asymmetric: a *raised*
  doubt sends someone to re-measure something already true, and a lowered one lets a
  wrong thing through. **Discount a claim on evidence about THAT claim, never on the
  batting average of the claimant — including when the claimant is you.** The
  countermeasure is the same one that settled it: re-derive over a **different
  enumeration parameter** (here, sigil's own frozen tables vs. their measured deltas,
  each computed before seeing the other's number). Two derivations that share a parameter
  are one derivation run twice; two that do not are corroboration, and corroboration is
  what should move a confidence, not mood.
  **Its companion, from the same episode:** "every delta is a multiple of 16" is
  **strictly stronger** than "the addresses are 16-aligned", and the non-vacuity is
  carried entirely by the counterexamples (`Art_Tails` %16==10,
  `GameState_OJZScroll_Init` %16==4). A reader who skims it as *"well, everything is
  aligned anyway"* takes a proof for a truism. When a quantified claim's force lives in
  its counterexamples, state them beside it or the claim decays into a platitude the
  next time it is read.
- **Port work** follows the port loop (canonical:
  `docs/superpowers/notes/campaign-port-loop.md` — byte gate is step-1 only, then
  modernize/retrospect/back-prop/optimize until dry; dry is panel-adjudicated, not
  self-declared). New-era work takes the A/B/C lens panels
  (`docs/superpowers/notes/2026-08-03-era-lens-loop.md`).
- Unimplemented nice-to-haves go to `docs/superpowers/notes/campaign-gap-ledger.md`;
  twin scaffolding gets a kill condition in
  `docs/superpowers/notes/twin-scaffolding-kill-list.md`, same commit.
- Comments describe present-tense function, never change history. Commit with
  explicit paths only — never `git add -u` in this shared checkout — and check the
  branch before every commit.

### THE ZERO-`skip:` BAR CANNOT SEE 27 OF THIS REPO'S SKIPS (2026-08-27)

**RATIFIED AS PROTOCOL BAR 25** — *a green log and an absent run are the same artifact* —
at empyrean `dc0ebe7`, verified reachable from their `origin/main` here at read time. It is
aeon's finding with this lane's endorsement, and **`SKIP-TEXT-HOLE` is named in the bar as
sigil's to own**, so the queue item is now contract-visible rather than lane-local. Read the
bar as the authority; everything below is this lane's narrative of the same episode and is
the perishable half. Its two correctives are already in force here: the gate's own NAME must
appear in its green log, and a rule naming a suite spells the invocation INSIDE the command
span.

**Found by the aeon lane, off a stale pin they hit at their chain-171 landing.** The
full-suite bar above requires **zero `skip:` lines**. Twenty-seven early-return sites across
**nine** test files announce themselves as `eprintln!("skipping <gate> …")` instead, and
**`skipping` does not match `skip:`** — so those gates can no-op and still clear the bar,
which reads back as coverage. Measured, not reasoned: `grep -rn 'eprintln!("skipping' crates
--include='*.rs' | wc -l` = 27 across 9 files, against 146 sites using the `skip:` form.
Queued as `SKIP-TEXT-HOLE`; the fix is one spelling, and it is the bar that moves, not the
gates. **It is not only the hand-run bar: `scripts/nightly_source_gates.sh` gates on the same
blind `grep -q 'skip:'` and exits 2 on a hit**, so all 27 are invisible to the automated lane
as well — whose whole purpose is to notice what a byte-triggered ritual cannot.

**Their hypothesis for the specific incident was WRONG, and the correction matters more than
the hole.** They proposed the silent skip as the reason a stale `SFX_BODY_LEN` survived chain
170. It cannot be: `colinked_sfx_head_matches_the_reference_rom_slice_both_shapes` guards on
`if !strict_gate()`, and `strict_gate()` is `std::env::var("SIGIL_STRICT_GATE").is_ok()` —
under the landing bar that test **always runs and must have gone red**. So the skip text is a
real defect with a different victim, and this one needs its own answer.

**The actual answer, and it is a landing-lane gap rather than a false green: THIS LANE
PRODUCED NO CHAIN-170 GREEN.** No strict full-suite run was logged at chain 169 (`c8e87ecb`)
or chain 170 (`174f4300`); the newest suite log on this machine is
`~/sonic_hacks/.sigil-verify-d8748933.log`, stamped `head=d8748933`, which **predates both**.
Both chains were refreezes landed through the aeon-owned lane. Verified rather than inferred:
`SFX_BODY_LEN` was `2046` = `0x7FE` = `SFX_BANK_BLOB.plain_len` **at chain 169**, chain 170
grew that region to `0x8DA` (2266) and left the constant behind, and the assertion is
`out.body.len() == SFX_BODY_LEN` — 2266 against 2046, which no strict run survives.

**So the structural finding is: a refreeze can land in this repo without sigil's strict suite
ever running.** That is what let a pin go stale for a whole chain with nothing red anywhere.
Queued as `REFREEZE-NEEDS-STRICT`; aeon has accepted it as a standing commitment banked in
their own repo.

**CORRECTED the same night, by the aeon lane, and the corrected cause is sharper than the
one first written here.** This section originally said the landing rule named *who lands* but
not *that the bar runs*. **That was wrong: their rule did require the full sigil suite.** It
spelled the command as a bare `cargo test --release --workspace --no-fail-fast` with **no
`SIGIL_STRICT_GATE=1`** — so a session following it faithfully ran a suite in which every
`strict_gate()`-guarded port and co-link gate early-returns. The rule was present and the one
token that gave it force was absent. Fixed on their side at aeon `origin/master`, verified
here by reading their file at that revision rather than taking the report.
**The generalisation, and it is why this is banked rather than closed:** a procedure can be
complete in its steps and inert in its spelling, and the inert version passes every review a
reader gives it — nobody audits a command line for a missing env var. Note the two findings
compound: the missing run is why nothing was red, and the skip-text hole is why a future run
might not be red either.

**Their `+0x10` vs `+0x12` derivation was re-checked here against the pins themselves and
holds.** `SFX_BANK_BLOB` grew `0x12` while the assembled total moved `0x10`, because the
16-alignment pad before the next region absorbed 2: `0xA3B20 + 0x8DA = 0xA43FA` (6 to
`0xA4400`), `0xA3B20 + 0x8EC = 0xA440C` (4 to `0xA4410`). Corroborated over a different
parameter than their arithmetic — `EPILOGUE` moved `0xA5C80 → 0xA5C90`, exactly `+0x10`.


### THE ZERO-`skip:` BAR COULD NOT SEE A SKIP LINE AT ALL (2026-08-27)

**Measured here, one binary, same conditions both ways:**

```sh
cargo test --release -p sigil-cli --test seam2_dac_emit                 # 0 skip-shaped lines
cargo test --release -p sigil-cli --test seam2_dac_emit -- --nocapture  # 2
```

**libtest captures a PASSING test's output.** Every `skip:` line this repo's gates print
from a test that then passes is swallowed, so the landing bar's "zero `skip:` lines"
requirement has been **structurally incapable of failing** for its whole life. Every
hand-run landing that reported it — including two this session — measured nothing. The
bar command above now carries `-- --nocapture`; that token is the entire fix and it
belongs INSIDE the command span, per bar 25's corrective (2).

**This compounds with `SKIP-TEXT-HOLE` rather than duplicating it.** That defect was the
*spelling* (27 sites saying `skipping` where the grep wanted `skip:`); this is *capture*.
They are independent, and **closing only the spelling would have left the bar blind
anyway** — which is what the skip-text parcel's own green log did, until it was re-run
with `--nocapture`. Two blind spots, one artifact, and neither visible from the other.

**The nightly lane was NEVER blind to this half** — `scripts/nightly_source_gates.sh`
already passes `-- --nocapture` (find it by content near the `cargo test` invocation).
So the earlier statement that the automated backstop "shares the ritual's blind spot" is
true for the SPELLING and false for CAPTURE. The automated lane was the sound one; the
hand-run bar was the blind one. Do not carry the general claim forward — say which half.

### RECONCILE THE TEST COUNT AGAINST `-- --list`, NOT ONLY AGAINST A SOURCE GREP (2026-08-27)

*(aeon's finding, adapted: they hit the same class in pytest, where a fully green `-q` run
prints no gate names at all, so bar 25's corrective (1) — confirm the gate's name appears
in its own log — is unrunnable there. Their remedy is `pytest --collect-only`.)*

The cargo equivalent, measured here: **`cargo test --release --workspace -- --list`
enumerates 3953 test ids in 2 seconds**, and that figure agreed exactly with
`git grep -c '#[test]'` on the same tree.

Prefer it, and use both. They enumerate over genuinely different parameters — the grep
reads **source text** (and will count an attribute that is `cfg`-ed out), while `--list`
enumerates **what the built binaries will actually run**. Agreement between them is
corroboration in bar 19's sense; agreement of a grep with itself is not.

The deeper reason this is the better instrument: **a log grep can only ever see what a
passing run chose to print, while collection emits the population.** A gate that silently
stops being built or run cannot shrink the collected id set without the diff showing it,
whereas it can vanish from a log with nothing to notice. That is the generator-enumeration
bar (see the OFFCANON-ROT note) pointed at test existence rather than at constants.

### NEVER SHARE ONE `CARGO_TARGET_DIR` BETWEEN TWO WORKTREES OF THIS REPO (2026-08-27)

Cargo bakes the building worktree's `CARGO_MANIFEST_DIR` into the cached rlib. Point a
second worktree at the same target dir, then delete or move the first, and the suite
reports **284 failures that read exactly like golden divergence** — while the log's own
stamp truthfully names the *correct* tree, so the stamping discipline cannot catch it.
Sibling of the `.aeon-sigil-gates` hazard, and it defeats the one mechanism that exists to
detect a run measuring the wrong tree.

**THIS ROW WAS BOOKED AND THEN VIOLATED ON 2026-08-30, COSTING 36 FALSE FAILURES.** It is
correct and it was not enough: a row is prose with a promise attached, and prose does not run.
For a landing run, use `scripts/landing-run.sh`, which REFUSES a shared or default `target/`
instead of asking you to remember this paragraph — see the wrapper's section above for what it
still cannot prevent.

**Give every worktree its own target dir** (`.sigil-<name>-target`). Two side benefits,
both real: it keeps the shared `target/release/sigil` from being relinked underneath a
peer's in-flight A/B — the aeon lane pins that binary for freezes and needs to be told
before it moves — and it avoids the mid-build assembler swap recorded above.

### THE STRICT-ATTESTATION RATCHET — what the aeon lane must now do (landed 2026-08-27)

`REFREEZE-NEEDS-STRICT`, merged at `729cd642`. A refreeze can no longer be built on top of
an entry whose strict suite never ran — the defect that let chains 169 and 170 land with no
strict run behind them and a stale `SFX_BODY_LEN` ride through both.

**The landing ritual gains one step and one SUBSTITUTION.** Where the aeon lane previously
ran the strict suite by hand, it now runs through the tool:

1. `refreeze --freeze … --ab …` (unchanged), then **commit the freeze** — goldens,
   `pins.rs`, `provenance.toml`.
2. On that clean committed tree:
   `AEON_DIR=<clean checkout of the frozen aeon SHA> cargo run --release -p sigil-harness --bin refreeze -- --attest`
   This **replaces** the manual `SIGIL_STRICT_GATE=1 … cargo test …`. It runs the same
   suite, **sets the flag itself**, adds `--nocapture`, and streams to a log it stamps up
   front. `--expect-test <name>` (repeatable) refuses if a named test did not execute.
3. On success it appends `[entry.strict]` → commit `provenance.toml` only. On a red run it
   records `outcome = "failed"`, names the failing tests, and exits 1.

**⚠ PUSH THE FREEZE COMMIT BEFORE STEP 2.** `sigil_rev` names the tree the suite ran on, and
a commit that has not been pushed is a coordinate a later rebase can delete. Once it is in
`origin/master` nothing short of a force-push can orphan it. Step 2 now measures this and
says which state the revision is in — `AHEAD OF REMOTE` means it is still exposed — but it
does NOT refuse, because being unpushed at that moment is the honest state and refusing it
would refuse the correct case. Chain 181 is the instance that motivates the step: it was
attested, then rebased, then pushed, and its `sigil_rev` now reaches nothing.

**⚠ A REBASE IS A TREE-MOVE, AND "CLEAN COMMITTED TREE" DOES NOT IMPLY "REBUILT"**
*(aeon lane, 2026-08-27, caught in their own pre-launch check at the first real attestation)*.
Step 2's tree can be clean, committed and still carry a harness binary compiled at the
*previous* HEAD. `version_reports_the_head_of_the_tree_it_was_built_from` then goes red for a
reason with nothing to do with the freeze — the binary honest, the tree moved. This lane's
existing warning covers a commit landing mid-run; a rebase reaches the same state from the
other direction and **before** the run, so the mid-run rule does not catch it. **Rebuild after
any rebase, then launch.** The whole sigil-side reason to rebase at all is that the freeze's
parent has fallen behind `origin/master`, which happens whenever sigil lands anything —
including docs — between freeze and attest.

**And the freeze this needs from sigil is the REF, not the working tree.** The aeon lane
rebases onto `origin/master` and later pushes to it, so what must not move is that ref:
sigil holds master **and any parcel branch about to merge into it** — the in-flight merge is
the tip-move most likely to be forgotten, since it is not a deliberate act at the time it is
requested. A dirty sigil working tree is harmless to their run; a moved ref is not.

**⚠ THE GENERAL FORM, AND IT INVERTS THE USUAL ADVICE ABOUT BEING SPECIFIC** *(aeon's
formulation, 2026-08-27)*. **A freeze request asks a lane to suspend a set of FUTURE actions,
and the requester can only enumerate the ones they know about.** They asked for master; they
could not ask about a branch they did not know existed, and the holder would not naturally
count *"let the agent merge when it goes green"* as **doing something** — at the moment of the
request it is not an act, it is the absence of an intervention. **The party who can enumerate
the pending actions is never the party making the request.**

So a precise request is the one that fails: *"please hold master"* names what the requester can
see and implicitly licenses everything else. **The robust form is the vague one:** *"land
nothing that moves my rebase target, and tell me what that turns out to include."* Both seats
have a duty here — the requester asks for the OUTCOME rather than the action, and the holder
enumerates their own pending tip-moves and reports what the freeze turned out to cover.

**Two clocks, and say so IN THE ENTRY.** `[entry.strict].sigil_rev` is the revision the suite
RAN ON; the freeze's `Assembler:` banner is the revision that PRODUCED the frozen bytes. After
a rebase these differ **by construction, not by fault**, and a reader comparing them will
assume something broke. Name both quantities in the entry itself — not only in a lane log,
which the confused reader does not have open. Same defect shape as the version banner's
stuck-versus-stale field, found independently in a second instrument the same hour.

**Why the flag is set by the tool and not asked of the operator: that one missing token IS
the whole defect being closed.** A procedure can be complete in its steps and inert in its
spelling, and nobody audits a command line for a missing env var.

**The witness, and why it is not another aggregate.** `strict_bodies` counts the
strict-gated decision points a run actually reached with the flag on — written only on the
path that has *already* observed the flag set, so it is **structurally zero** when the flag
is unset. No pass count, exit code or `ignored` total can make that distinction, because a
non-strict suite is also fully green. Measured: 29, matching an independent static count of
`if !strict_gate() { skip; return }` sites — two derivations over different parameters.

**Do NOT substitute a log grep for it.** The aeon lane proposed harvesting `_port` hits from
the log (66) as a free witness. Refuted: **43 of 46 are cargo's own `Running tests/…` lines,
printed before any binary starts**, so the flag cannot affect them and a flag-off run
reproduces the same count. It was a constant wearing a witness's clothes.

**A failed freeze does not deadlock the chain, and this was a real defect in the first
design.** If a strict run legitimately goes red and the fix moves bytes, the entry can never
be attested. The next freeze passes `--supersede-tip "<why>"`, naming its successor.
**Abandonment additionally requires a recorded RED run** — you cannot abandon an entry you
never tested — which closes the serial-supersession evasion (freeze, abandon, freeze,
abandon, and the suite never runs again). Residual, stated rather than hidden: an operator
can still cycle red-run → supersede, but every cycle costs a real strict run that genuinely
came back red, which is the run the mechanism exists to force.

**Nothing is backfilled.** Entries 1–172 are untouched; a reconstructed record is a
confident guess wearing a record's clothes.

## The source-gate lane

`scripts/nightly_source_gates.sh`, fired by the `systemd --user` timer
`sigil-source-gates.timer` at 05:17 daily. It runs the **gates whose inputs are aeon
SOURCE** — the warn-tier corpus and its neighbours, named one per line in the
hand-maintained `SOURCE_GATES` array; read the count out of the array rather than out of
this sentence, which is how it went stale the first time — against detached master-tip
checkouts of *both* repos, at `~/sonic_hacks/.sigil-source-gates` and
`~/sonic_hacks/.aeon-sigil-gates`. Both live outside their repo roots: a worktree under
the aeon root double-counts every module in that repo's `tools/emp_helper_closure.py`
tree scan.

**Why it exists.** These gates read aeon source, but nothing *ran* them against a fresh
aeon tip except a refreeze — and a refreeze happens only when ROM bytes move. Six
consecutive zero-byte aeon parcels hid a real `layout.odd-field` finding for a day
(`docs/superpowers/notes/2026-08-22-warn-tier-drift-open.md`). A trigger keyed to byte
movement is structurally blind to a source-derived lint set moving, so this one is a
clock.

- It compares **no byte against a committed artifact**. The region-diff gates, the
  golden-CRC gates and `pins_rs_is_current` read a built ROM or compare against sigil's
  frozen goldens, and they already have the right trigger — aeon's byte-identity ritual,
  which fires exactly when bytes move. The exclusion is named in the script, not silent,
  and it is **counted**: the verdict line prints how many aeon-reading gates were skipped
  as artifact-lane (82 at `a886fd2b`; read the line, not this number) so "skipped" cannot
  be read as "green".
- It **does build every shipped shape from source**: `corpus_builds`
  (`crates/sigil-cli/tests/corpus_builds.rs`) is the lane's **brick witness**. For each
  shape in `native::shipped_shapes()` — the one table the byte gates enumerate — the entry
  `sigil build` reaches (`build_rom_chained_with_listing`) must return `Ok` with zero
  error-level diagnostics; no byte is compared to anything, so a byte-moving aeon parcel
  leaves it green while a BRICK (`[map.order-undeclared]`, `section … has no region in
  the map`, colliding pins, an unknown function in a reached module) turns it red naming
  every bricked shape. The two failure kinds the skipped artifact gates conflate — CRC
  drift (refreeze owns it) and a brick (nobody's ritual clears it) — are therefore split:
  the verdict line names `corpus_builds` as the brick witness, and a red whose output
  carries its phrase is announced as `BUILD BRICK`. Its second test injects the
  2026-08-26 live brick into a `shadow_aeon_tree` copy (the `section:ojz_effects_editor_act1`
  map row deleted) and requires the same checker to name the shape and section, so the
  detector is proven live on every run. Two other `SOURCE_GATES` entries
  (`m68k_roundtrip_stream`, `m68k_capstone_stream`) also build all seven shapes and would
  panic on a brick — at the first bricked shape, under a roundtrip test's name; the witness
  exists so the brick is measured as itself, on every shape, and read back by the verdict.
- **Every new `crates/*/tests/*.rs` that reads the aeon tree must be classified the day it
  lands.** `derived_layout` (master `4f303b0d`) was source-only, not in `SOURCE_GATES`, and
  named no artifact: the audit would have exited 2 — the whole lane dark — at the next
  05:17. Replaying the audit against the branch is one loop and it is the only thing that
  sees this before the timer does.
- Exit `1` = a gate failed, `2` = the lane could not run; both `notify-send`. "Could not
  run" includes fewer gates executing than were named, zero tests executed, and any
  `skip:` line surviving `SIGIL_STRICT_GATE=1`.
- `SIGIL_SOURCE_GATES_REF` / `AEON_SOURCE_GATES_REF` put a branch or an old SHA through
  the real lane. The timer never sets them.
- **`~/sonic_hacks/.aeon-sigil-gates` is source-only by construction — never point an
  artifact-dependent run at it.** Each lane run deletes the generated inputs *and* any
  built ROM or listing, so a full-suite run sharing that tree loses its ROMs mid-flight
  and reports ~127 `reference missing: …/s4.bin` failures that read exactly like a
  golden divergence. Build a separate checkout for the artifact gates.
- The units are committed at `scripts/systemd/` and installed by copying them to
  `~/.config/systemd/user/` — a `--user` unit lives outside every repo, so an
  enabled-but-uncommitted timer is invisible to every session that did not install it.
- Do **not** touch `aeon-effects-gates.{service,timer}`; that lane is aeon's, and it
  fires at 04:17 so the two do not contend.

**⚠ The lane's self-audit reads PROSE, and a doc comment can take the whole lane down.**
Before running, the script classifies every `crates/*/tests/*.rs` matching
`AEON_DIR|aeon_dir|reference_tree|--aeon` as either in `SOURCE_GATES` or derivably
artifact-dependent, and exits `2` — the entire nightly backstop dark, reporting nothing —
if any file is neither. That grep cannot tell a *use* from a *mention*: a new test whose
header says "takes no `AEON_DIR`" matches on the disclaimer and is unclassifiable by
construction. Caught on `feat/version-provenance` before landing (replaying the audit gave
`unclassified=1 [version_provenance]` at the first delivery, `0` after); the fix was to
describe those inputs without naming the identifiers, and to say why in the file so the
next author does not re-arm it. **Replay the audit against any branch adding a
`crates/*/tests/*.rs` file** — it costs one loop and the failure is otherwise invisible
until 05:17.
**⚠ `gates=N` is NOT a number this script prints — it is a reconstruction, and two lanes
reconstructed different quantities under the same label** *(caught 2026-08-22, at a landing)*.
The script emits no `gates=` line at all. This document recorded `gates=35 unclassified=0`
(the size of the `SOURCE_GATES` array); a parcel agent replaying the same audit reported
`gates=116 unclassified=0` (the count of `crates/*/tests/*.rs` files the selector SCANS).
**Both measurements were correct and they are not the same quantity** — on the merged tree:
`SOURCE_GATES=35 scanned=116 unclassified=0`. Nothing was wrong except the label, which is
precisely why it survived: two different numbers agreeing on the part that matters look like
a discrepancy in the part that does not. **The load-bearing figure is `unclassified`** — it
is the only one the script acts on (non-zero ⇒ exit 2, whole lane dark). When reporting an
audit replay, name the quantity: `SOURCE_GATES=<n> scanned=<n> unclassified=<n>`, never a
bare `gates=`.

**Left open, deliberately:** tightening the detector to match code uses rather than any
occurrence would disarm the trap, but it also loosens what the lane considers
unclassified, and a genuinely aeon-reading gate escaping silently is the failure this
audit exists to prevent. Soundness-reducing, so it is not a parcel-local call — it needs
its own ruling.

**Adjudicating a warn-tier firing.** A new firing goes into `CORPUS_OPEN_FINDINGS`
(`crates/sigil-cli/tests/warn_tier_corpus.rs`), **not** into `WARN_ID_BASELINE`. The
baseline admits an id everywhere and any number of times; the register pins
`(shape, id, file, symbol)` with a count, and requires an owner, an anchor and a kill
condition per row. Anchors are symbols and paths, never line numbers — a register entry
outlives what it points at, so a coordinate in one rots. A row leaves the register in
either direction: fixed, or ruled deliberate and promoted into the baseline citing the
ruling. Each row's age prints on every lane run.

### THE SHARED CHECKOUT'S BRANCH MOVES UNDER YOU — verify it IN the committing command (2026-08-27)

**This overseer committed onto the aeon lane's branch four minutes after they put the
shared main checkout on it.** They switched `~/sonic_hacks/sigil` from master to
`parcel/band-ceiling-16-pair` at 05:44:42Z to land chain 172; this session committed a
lane-log entry at 05:48:36Z without re-checking, so it landed on top of their unpushed
freeze. Repaired: cherry-picked to master (`f50713ed`, pushed, reachability verified),
their branch `git reset --hard` back to `521956f9` with the tracked tree confirmed clean
first, checkout left on their branch where they had it. Disclosed to them immediately,
with the verification commands, because **the owner of a contaminated branch cannot detect
this themselves.**

**The standing rule already existed and did not fire, which is the finding.** "Check the
branch before every commit" was banked in this lane's memory precisely because the checkout
is shared. It failed because the check had been performed — **at boot** — and nothing
prompted a second one. A peer landing in the shared main checkout **silently changes what
`git commit` means for every other session using that tree**, and it is invisible: no
fetch fails, no gate trips, and `git status` looks entirely normal because it is.

This is protocol bar 22's read-time-versus-send-time staleness aimed at a **git ref**
rather than at a peer's status file. The measured window there was four minutes; the
measured window here was **also four minutes**, which is not a coincidence — it is how long
a peer's landing takes.

**Corrective, and it is a mechanism rather than vigilance:** verify the branch in the SAME
command that commits, never at boot and never in a preceding call.

```sh
[ "$(git branch --show-current)" = master ] && git add <explicit paths> && git commit -m '…'
```

**What bounded the damage was the OTHER half of the rule.** The commit used explicit paths,
so it touched only `docs/lane-log.jsonl` and no file of theirs — `git add -u` would have
swept their entire in-flight freeze into a commit on their own branch. The two halves of
that memory row are not equally load-bearing: the branch check is the one that fails
silently, the explicit-paths rule is the one that limits the blast radius when it does.

**THE UPSTREAM HALF, and the aeon lane's correction to this section's first framing.**
As written above this reads as the committer's check failing. That is not the whole defect,
and putting it all on the committer lets the cheaper fix escape. **A session that repoints a
shared checkout onto a private branch changes what `git commit` means for every other session
using that tree — silently, and RETROACTIVELY, invalidating branch checks that were correct
when they were performed.** No amount of committer-side discipline prevents another lane doing
that to you; it only bounds the damage.

Two correctives, opposite ends, both load-bearing:

- **Committer side** (this section): never commit to master from the shared tree — use a
  worktree; and if you commit there anyway, verify the branch inside the same command.
- **Repointer side** (aeon's, executed 2026-08-27): **never leave a shared checkout on a
  private branch.** They moved `parcel/band-ceiling-16-pair` into its own worktree at
  `~/sonic_hacks/.sigil-pair-172` and returned the shared checkout to master, tracked-clean;
  freezes run that way from here. This repo's protocol already says worktrees are why a shared
  main tree never matters — repointing the main tree is the one act that makes it matter again.

The repointer prevents this most cheaply and is the only party who knows it is happening.

**Mechanical note for the committer-side rule: use `git worktree add --detach <path> master`,
not `git worktree add <path> master`.** The second form fails outright when the shared checkout
is *itself* on master (`'master' is already used by worktree at …`), which is the normal state
and therefore the state you will hit. Detach, commit, `git push origin HEAD:master`, remove the
worktree from OUTSIDE it — removing it while your shell is inside deletes your cwd and every
subsequent command in the chain fails with `Unable to read current working directory`.

**Corollary: do not check out master in the shared tree to get around this.** That yanks the
branch out from under whichever peer is landing. Commit to master from a throwaway worktree
(`git worktree add -q <path> master`, cherry-pick, remove) and leave the shared checkout on
whatever branch its current user put it on.

### A QUALIFIER PRINTED BESIDE A VALUE IS PART OF THE VALUE (2026-08-27)

This session reported a "mystery" to the aeon lane: golden files whose mtimes read
`01:32-01:40` while the tree demonstrably changed between `05:35Z` and `05:44Z`, which
looks like a tree changing at a time its own files deny. **There was no mystery. The
`stat` output printed `-0400` on the same line as the timestamp**, this box is EDT, and
`01:40 -0400` IS `05:40Z` — squarely inside the window. The disambiguating field was
already in the output and got skipped.

The transferable form is deliberately not "local and UTC are used side by side here",
which describes an environment. It is a **reading** failure and it recurs wherever a tool
prints a qualifier next to a value: a timezone offset, a units suffix, a base prefix, a
scale factor. **Read the qualifier as part of the value, or convert before comparing** —
never compare two numbers whose qualifiers you did not read.

Cost: a peer spent a reply refuting a phantom, and the report had already been sent as an
anomaly. Cheap here; the same skip over a base prefix or a units suffix is not.

### A CHECK WHOSE SUBJECT IS NOT YET IN PLAY RETURNS CLEAN, AND CLEAN CARRIES NO TELL (2026-08-30)

Three instances from one night, two of them this lane's, and they are one class:

- **Boot frames on a scroll clamp** *(aeon's, and it nearly passed their own parcel as inert)*.
  Two ROMs compared byte-identical at boot, **zero differing pixels** — because the background
  V-scroll is still 0 there and the divergence only exists once the parallax step has run. A
  reviewer comparing boot frames concludes the clamp does nothing. It broke only when they
  stopped comparing pictures and read `Vscroll_Factor` out of memory in both ROMs.
- **`d-15`, this lane's**: a schema repair checked against the contract prose, which it satisfied,
  while the reader that decides whether the card renders was never run.
- **`repin --check` printing `pins.rs unchanged`** on a reference tree with no listings at all,
  recorded above. It witnesses that placement resolves and is silent on every artifact the gates
  read afterwards.

**The common form: a clean result does not distinguish *the property holds* from *the property
was not yet in play*.** That is bar 16(d)'s absence surface arriving on a POSITIVE artifact — a
page of passes rather than an empty output — which is what makes it harder to suspect.

**The corrective is one question, asked of the witness rather than of the subject: does this
instrument TOUCH the thing I am claiming about?** Boot frames do not touch a scroll value that is
still zero. Contract prose does not touch a parser. A pin file does not touch a listing. **Where
the answer is no, the run is not weak evidence — it is no evidence, and it should be reported as
"not exercised" rather than as a pass.**

### A GATE DEFEASIBLE BY BUILD ORDER ALONE IS A FALSE GREEN WITH NO TELL (2026-08-30, aeon's)

`tools/demo_specialization_witness.py` reads `s4.debug.lst` **and** `demo.debug.lst`. A build
order that leaves either listing stale means nothing ever sees both fresh — all four shapes went
green before the pin was updated, and it surfaced only on the next build. The remedy applied
there was to re-run ending on `DEBUG=1 ./build.sh` so both debug listings are current.

**Why it is banked here rather than left as aeon's:** this lane's freeze runs lean on gates of
exactly that shape — multi-artifact readers whose inputs are produced by separate build
invocations. **A gate that can be defeated by ordering has no failing mode to observe**, so it
cannot be caught by making it fail; only by enumerating its inputs and asking which invocation
produces each. **Sweep sigil's own multi-listing gates for the same pattern rather than assuming
it is specific to that script.**

#### THE SWEEP, RUN — `docs/superpowers/notes/2026-08-30-build-order-gate-sweep.md` (2026-08-30)

360 carriers enumerated by the PRODUCT, not by the name: 123 read a build product, 84 read two
or more, 70 cross an invocation boundary, **14 are exposed**. Read the note for each one; four
results belong here because they change how this file's own witnesses should be read.

**aeon's exact defect is absent from sigil, and for a stateable reason.** The asl `.lst` parse
was retired from `repin` at Stage-3 P4c: it now resolves BOTH shapes in one process from source
and opens no listing file. The only two `listing_symbol_addr` call sites in the tree pair a
listing with the ROM from its own invocation. **No gate in sigil reads two listings.**

**The worst instance is a PRODUCER, not a gate.** `refreeze --freeze` calls `resolve_aeon_rev`
**once**, before step 1, and its four steps then re-read `$AEON_DIR` independently across a run
this file already records as outliving a ten-minute cap. A tree that moves mid-freeze yields
blobs from rev N, tables and pins from rev N+1, and a ledger naming rev N — and every downstream
gate is green, because step 4 derives its numbers from steps 1-3's OUTPUTS rather than from the
tree. **The remedy is a HEAD re-read, not a cleanliness re-check**: `git status --porcelain`
after step 3 would fire on the build's own writes, which is why the check is early;
`git rev-parse HEAD` would not.

**`refreeze --check` cannot be red about a stale tree, and it is quoted as a landing witness.**
Its two products are steps 1 and 4 of one freeze, and step 4 computes its CRCs from step 1's
bytes. Tip-match is a tautology over one invocation's own output. A tree where nobody has run
anything for months is permanently green. Correct for what it claims; not a freshness witness.

**The better witness this file already nominates has no gate behind it.** The section above
prefers `golden/offcanonical_sizes/s4.txt` to `pins.rs` for length-neutral parcels, on the
strength of its two CRC header lines. `derive_offcanon` writes `# golden_crc32=` and
`# assembled_anchor=` from the committed blob's bytes — and **grepped with a positive control,
both have zero readers anywhere in the workspace.** The repo's best freeze witness is checked by
a human reading a diff. Asserting it in `offcanon_assembled_bar` is two comparisons and lands
green today (verified 7/7 on the committed blobs at `9fd6607d`).

**And one witness in this file's own recipe is circular.** `provision-aeon-ref.sh` copies the
golden ROMs into the reference tree at `:76`, builds at `:118-122`, then prints
`REBUILD CONTROL … MATCHES THE GOLDEN` by comparing the files it placed against the goldens they
came from. It cannot distinguish a real rebuild from its own copy; the listing check beside it is
`[ -s ]`, i.e. presence. `capture_goldens.sh` carries the fix — an mtime marker — in the same
repo, and it is the only freshness assertion in the tree apart from its twin.

### THE SENDING-SIDE HALF: A QUALIFIER ONE LINE AWAY IS NOT BESIDE THE VALUE (2026-08-30)

The rule above is the READING direction — read the qualifier as part of the value. Its inverse
is the AUTHORING direction and neither lane's file had it: **when you print a number whose
meaning depends on a qualifier, put the qualifier in the number's own line, because headers do
not survive quotation.**

Instance, from the ledger-audit cross-check. Aeon's conformance tool prints a reason histogram
that COLLECTS every failing option field, while dominion's `parseOptions` returns on the first
(`796bc1e:server/src/decisions.ts:194`). So their `9 option missing name` and this lane's `3`
are both right about different questions — theirs describes the data, mine describes what the
reader emits. Their fix was to say so **in the section header**. That corrects the document and
does not correct the failure, because **a histogram is exactly the shape that gets pasted into a
message on its own** — which is how their original `31 reasons` figure reached this lane stripped
of its context, inside a relay that was careful in every other respect.

**The form that survives quotation puts the correction in the row:**

    9  option missing name   (fields in the data; the reader reports 3, one per line)

A reader quoting the row quotes the qualifier with it, and a reader quoting the bare `9` has to
DELETE text to be wrong. Same principle as this file's *a count whose elements are enumerated in
the same sentence cannot drift*: **a self-qualifying value beats a qualified section.**

**And the design call it settles, so it is not re-litigated as two modes.** The temptation is to
split a transcribing instrument into a strict mode and a repair-list mode. Refused: two modes are
two things to keep in sync, and they drift SILENTLY in the direction that matters — if the
upstream parser ever changes to collect, the analysis mode becomes accidentally correct and
nothing announces it; if it gains a rule, the repair list quietly lacks it. **One output where
every number carries its own authority** is less machinery and fewer ways to be wrong.

### TWO AGENT-FACING HAZARDS MEASURED 2026-08-27, both from one reference-tree build

**(1) AN AGENT TOLD TO "RE-DERIVE FROM THE FILE" WILL READ A PATH, AND A PATH IN THIS
CHECKOUT IS NOT A REVISION.** A reference-tree agent read
`crates/sigil-harness/golden/provenance.toml` three times over ~10 minutes and saw three
different files: chain 172 uncommitted (a peer's freeze mid-write), chain 172 committed,
then chain 171 — because the shared checkout's branch moved between reads. It concluded
**sigil master had been rewound and chain 172 retracted**, and tagged that for foreground
relay to the aeon lane. Both halves were false: `521956f9` is alive on two branches and in
its own worktree, master's reflog runs strictly forward, and `521956f9` was never an
ancestor of master — so the agent's test (`--is-ancestor 521956f9 HEAD`, HEAD on master)
could never have been true.

**Every number the agent measured was correct; only its causal story was wrong.** That is
the take-the-numbers-re-derive-the-cause rule with an agent as the source, and it is why
the episode cost a retraction message instead of a peer's hour.

The instruction that caused it was **this overseer's**: the brief said the provenance file
wins over the brief, which is right, and did not say to read it at a revision, which was
wrong in a checkout whose branch peers move. **Briefs that tell an agent to re-derive from a
file in the shared checkout must say `git show <rev>:<path>`.** Now standing in this lane's
dispatches.

**(2) A CONCURRENT `cargo test` RELINKS THE ASSEMBLER UNDERNEATH A MULTI-SHAPE BUILD.**
`sigil/target/release/sigil` was rebuilt by a parallel workspace test run *between shapes 2
and 3* of a four-shape build. The only tell was the build's own banner shifting from
`(dirty)` to `(revision+dirty)`. Undetected it yields **four shapes built by two different
assemblers, with nothing in the artifacts to reveal it** — and for a freeze, "which
assembler produced these bytes" is the entire point.

Corrective, which that agent invented and this lane has adopted: **copy the assembler to a
pinned tools directory and point every shape of a multi-shape build at the copy.** The
build then provably used one binary, and the directory is the provenance record — do not
delete it after the build. Kept for the 2026-08-27 trees at
`~/sonic_hacks/.sigil-ref-353aaa49-tools/` (`sigil 0.1.0`, built at `4b8347ac`).

Note this is the strongest evidence so far for the `GOLDEN-DIRTY-BANNER` queue item (booked
under queue item 1): the banner is noisy on every paired landing, and here it carried real
signal that only close human reading caught.

**And it is the argument AGAINST quieting the banner too far.** The classification landed
under that queue item makes the warning fire less; the tell in this episode was the banner
shifting from `(dirty)` to `(revision+dirty)` mid-build, and a closure-aware revision
comparison would not have shown that shift if the intervening commits missed the closure.
The corrective that actually covers this case is the pinned tools directory above, not the
warning — do not let a quieter banner be read as this hazard being closed.

**(3) The `level_staleness.py` hazard's stated MECHANISM is wrong, measured on two fresh
worktrees.** The note that a fresh checkout gives `project.json` a new mtime so the gate
hard-fails before any ROM is emitted **did not reproduce**: the gate compares
newest-mtime(editor sources) `>` newest-mtime(generated tree), and `git worktree add`
writes every file inside the same second, so `>` is false. It bites an **in-place
`git checkout`**, which rewrites only changed files and leaves `project.json` newer than an
untouched generated tree. So "fresh checkout" should read "checkout into an existing tree",
and running `regenerate-level.sh` prophylactically on a fresh worktree is pure cost plus
`DONOR_PROVENANCE.json` churn. That note is aeon's; flagged to them, not edited here.

**Reference trees standing as of 2026-08-27, both clean detached aeon worktrees with all
four shapes built and CRC32+size verified against the committed tip** (read via
`git show master:…provenance.toml`, not off the working file):
`~/sonic_hacks/.sigil-ref-ac57991d` (chain 171) and
`~/sonic_hacks/.sigil-ref-353aaa49` (chain 172). **Added 2026-08-27:
`~/sonic_hacks/.aeon-ref-a6a7c23d`, detached at `33d905b8`, all four shapes built and
CRC32+size verified against the chain-173 tip — kept rather than removed after the run that
built it, because a four-shape build is the expensive part of any artifact-dependent
verification and this one matches the CURRENT tip.** **Derive which one is current from the tip; do not
read that pairing off this paragraph** — the warning at the top of the quality-bar section
applies to these two names exactly as it applied to every predecessor.

### `git diff -- <path>` IS CWD-RELATIVE AND `git show <rev>:<path>` IS NOT (2026-08-27)

*(aeon's finding, against their own evidence; reproduced firsthand here before banking.)*

Mix the two forms after a `cd` and you get **an empty diff with exit 0 while the blobs
genuinely differ.** Reproduced in this repo:

```sh
git diff --stat HEAD~2 HEAD -- docs/superpowers/notes          # 109 lines changed
cd docs && git diff --stat HEAD~2 HEAD -- docs/superpowers/notes   # EMPTY, exit 0
cd docs && git diff --stat HEAD~2 HEAD -- ':(top)docs/superpowers/notes'  # 109 again
```

`git diff`'s pathspec is resolved against the **current directory**; `git show <rev>:<path>`
is always resolved against the **repository root**. An investigation that enumerates
something with `git show rev:path` and then diffs with the same string — having `cd`-ed
somewhere in between, which is an entirely reasonable earlier step — silently compares
nothing.

**This is bar 16(d) with `cd` as the manufacturing mechanism, and it is nastier than
`2>/dev/null` or the `eza` alias.** Nothing is suppressed. Nothing errors. The command is
correct, the shell is correct, the revisions are correct — only the *composition* of two
path conventions is wrong, and the result is a clean empty answer with a zero exit and
nothing to be suspicious of.

**The tell aeon nearly missed, and it is the general one:** the conclusion being checked
happened to be **TRUE**, so the empty diff confirmed what they already believed. It was
caught only because blob hashes from a different command contradicted it. An empty result
that agrees with your prior is the hardest possible case — bar 10 (a verdict and its stated
reason are separately checkable) aimed at your own hand-offered evidence rather than at a
gate's message.

**Two remedies, use either:** run diffs from the repository root, or prefix the pathspec
with `:(top)` (verified above to restore the correct output from a subdirectory). Prefer
`:(top)` in anything scripted, since a script cannot know which directory *inside the tree*
it will be invoked from.

**Boundary, stated because the rule above could otherwise make a script worse** *(aeon's
caveat, and it is right)*: `:(top)` buys **cwd-independence within one tree, not
location-independence.** A script run from *outside* the repo, or one that walks into a
worktree, fails differently — `not a git repository`, or a correct pathspec resolved against
the *wrong* repo — and those failures are **loud**, which makes them the better failure. The
whole reason to reach for `:(top)` is that the CWD-relative form fails **silently and
empty**. A script that must also be repo-agnostic wants `git -C <root>` in addition, not
instead.

## Worktree and environment quirks

- **Worktrees are agent-isolated but the registry is repo-global.** Every session's
  worktrees live under `.worktrees/` off this main checkout and share one
  `git worktree list`. Check the list before adding (names collide across sessions),
  and prune stale entries — ~18 accumulate fast on a busy day.
- **Port tests need the aeon tree.** `AEON_DIR` selects it
  (`crates/sigil-harness/src/test_support.rs::aeon_dir`, default
  `/home/volence/sonic_hacks/aeon`); when the referenced paths are absent the gates
  skip green unless `SIGIL_STRICT_GATE=1`. A worktree agent running port tests must
  be told which aeon tree to point at — and beware the aeon-side worktree trap: a
  fresh aeon worktree missing its gitignored `games/sonic4/data/editor/` builds a
  padded, wrong ROM.
- **The `*_port` cross-seam trap:** the standalone port oracles lower ONE aeon module
  against a hand-picked dep list, so they see neither the whole-program contract bind
  (which resolves `Game.MEMBER`) nor `scene_dsl.emp` (which declares the `CAP_*`
  bits). A new `Game.*` / `CAP_*` reference in an engine module therefore breaks ~9
  port tests silently (`unknown name Game.SCANLINE_CAPS` at lower time). The fix
  pattern is `crates/sigil-harness/src/test_support.rs` §4: synthesize the interface
  + `implement` and the `pub const CAP_*` block, both **derived from the aeon tree at
  test runtime** via `emp_const_rhs` / `emp_const_literal` — never a copied literal.
- **Never build in the scratchpad/tmp** — `/tmp` is tmpfs; a cargo build there wedges
  the shell. Set `CARGO_TARGET_DIR` to disk for any out-of-tree build.
- **A LANDING RUN AGAINST THE OWNER'S LIVE AEON TREE PRODUCES PHANTOM FAILURES, and this
  overseer proved it the expensive way (2026-08-24).** The rule below is not advice. A
  full-suite landing run pointed at `/home/volence/sonic_hacks/aeon` returned **six**
  failures where the parcel's own agent had reported five; the extra was
  `soundbankhead_matches_reference`, and the parcel had touched `seam1.rs` — the sound
  seam — so the coincidence looked damning. It reproduced twice, including with the test
  binary run alone. A control at unmodified master showed it passing, which read as
  attribution. **It was transient**: four consecutive re-runs on the merged tree passed,
  single-threaded included, and a clean re-run of the whole suite returned the agent's
  5 exactly. Nothing was wrong with the parcel; the tree had artifacts in flux underneath
  a run that takes minutes. **The cost was an hour and a nearly-shipped wrong attribution
  against an agent that had done nothing wrong.** Steady-state disagreement between two
  runs over the same tree is the tell — re-run before attributing, and prefer a clean
  worktree of a committed SHA, which is what the next rule already said.
- **The main aeon checkout carries the owner's live editor content edits** (collision
  bins, regenerated act-pool pages under `games/sonic4/data/`) for hours or days at a
  time — never gate a green on that tree cleaning up. A strict-gate or landing run
  that matters points `AEON_DIR` at a CLEAN WORKTREE of a committed aeon SHA, with all
  four shapes built there first (repin resolves but does not generate). File seeding is
  RETIRED (`aeon/tools/seed-worktree.sh` is a copies-nothing stub; the OJZ tree and
  collision tables are committed, generated dirs rebuild via build.sh). **A fresh aeon
  reference worktree needs NO `sigil`/`skdisasm` symlinks and no paired sigil worktree
  — and this paragraph asserted the opposite for some time while the quality-bar section
  above asserted the truth, so a cold boot could read either.** Settled in the field
  2026-08-26: the aeon lane rebuilt `.aeon-landing` at `c3f5cbe0` with no symlinks of any
  kind and all four shapes built and reproduced their chain-168 CRCs exactly. The symlink
  requirement is real only for a worktree that must resolve the emp-helper closure — an
  agent's *development* tree — never for a reference checkout that only has to build.
  Seeding them into a reference tree is actively harmful: it kills `section_row_fixture`'s
  tree mirror with *"the source path is neither a regular file nor a symlink to a regular
  file"*, three gates red for a reason unrelated to whatever is under test.
  Verify the built ROMs against `golden/provenance.toml` (CRC32+size) before trusting
  the worktree. Mid-brushstroke aeon
  edits flipping sigil port-gate results is environmental, not signal; the tell is
  broad `*_port` region-diff failures at embedded addresses plus
  `repin_pins::pins_rs_is_current` failing identically on sigil master.

### A RED CHECK SHOULD NAME BOTH ITS FAILURE MODES AND THE COMMAND THAT SEPARATES THEM (2026-08-27)

*(aeon's formulation, from watching one of this repo's gates do it by accident.)*

`version_reports_the_head_of_the_tree_it_was_built_from` went red during a landing and its
message said: *"Either build.rs did not re-run when HEAD moved (the rerun triggers are the
fix), or HEAD moved while the suite was running (re-run to distinguish)."* That turned a
plausible five-minute investigation into **one command**.

**Every two-valued result that cost this workspace real time would have been a non-event with
a message in that shape** — the empty commit range that reads identically for "no work" and
"already merged"; an unmodified `pins.rs` that reads identically for "nothing moved" and
"the tool died before writing"; an absent banner that reads identically for "old binary" and
"filtered log". In each case the failing artifact was one-valued-looking and two-valued.

So when writing a check: **do not stop at making it fail. Ask what ELSE produces this exact
failure, and put the discriminator in the message.** A gate that says only *what* is wrong
delegates the harder half to whoever reads it, usually under time pressure, usually without
the author's context.

### WHEN YOU GRANT A HOLD, ENUMERATE WHAT THE PROTECTED RUN MEASURES (2026-08-27)

*(aeon's, after this lane granted a hold scoped to the wrong thing.)*

A hold on `target/release/sigil` was granted as *"do not relink"*, because the binary was the
thing visibly moving. **The protected run also measures repository HEAD** — a check compares
the binary's baked revision against it — so a plain docs commit trips it while relinking
nothing. Neither lane had said so, and both had been careful.

**Enumerate what the protected run MEASURES, not what you happen to be about to change.** The
second list is the one you can see; the first is the one that matters.

**And state the hold's reason in a form that survives being checked.** This lane widened that
hold on the belief that a commit could leave the other lane with a permanently-recorded red.
It could not — `refreeze` refuses to record a run whose HEAD moved, returning before any
outcome is computed. The hold was still worth keeping, for the weaker true reason (it wastes
their run) rather than the stronger false one. **A rule kept for a reason that will not
survive checking gets discarded along with the reason.**

### HOLD STATE LIVES IN EXACTLY ONE PLACE — THIS FILE (2026-08-27)

*(aurora's, after making the same class of error three times in an afternoon.)*

A hold that lives only in a message does not survive a `/clear`. But a hold **copied** into
another lane's doc goes stale the moment this lane lifts it — and so does a *receipt* saying
it was lifted, and so does a statement that **no** hold is in force. All three are
present-tense claims about this lane's live state, and abstraction does not help: *"nothing is
in force"* is exactly as perishable as a hash list and harder to check.

**A fact that can only be correct in one place must live in one place, and every other place
points at it.** Other lanes carry the PROCEDURE — `git -C ../sigil show origin/master:docs/OVERSEER.md` —
and no state. **A receipt is the more dangerous artifact than a rule**: a hold announces itself
as a constraint a reader should evaluate, while a receipt reads as settled history and invites
no scrutiny at all.

### STANDING ARTIFACTS THIS LANE DEPENDS ON — declared here so a SWEEPING lane can find them

**A declaration only its author reads is not a declaration.** On 2026-08-27 the aeon lane
swept `~/sonic_hacks/` for merged, detached, branchless worktrees and removed eleven. One was
this lane's standing reference tree. It was an **aeon** worktree by construction — it holds
aeon shapes — so it matched every mechanical criterion, and **the only thing that would have
distinguished it was a line in this file, which a sweep of aeon's worktree list cannot see.**
*Ownership is not conferred by registration* (aeon's formulation): being in a repo's worktree
list is a fact about bookkeeping, not about who depends on it.

**THE AEON REFERENCE TREE.** This lane needs one aeon checkout with **all four shapes built**,
pinned to **the `aeon_rev` of the corpus tip** — the expensive half is the build, not the
clone. Byte gates point `AEON_DIR` at it; without one, they cannot run at all.

**Do not write its revision here.** Derive it, or this line rots into the trap it exists to
prevent — the declaration that was here on 2026-08-27 named `33d905b8`, two corpus revisions
stale, **so a lane that HAD read this file would still have rebuilt the wrong tree and believed
it had repaired the damage.** A stale declaration sitting on top of a missing one.

```sh
# the revision the reference tree must be pinned at, always:
grep -E '^aeon_rev' crates/sigil-harness/golden/provenance.toml | tail -1
```

**Verify before trusting it**: hash the four built ROMs against the tip entry's recorded
targets (CRC32 + size). **A tree existing at the right SHA is not a tree whose shapes are
right.**

### `--force` CONVERTS A LOUD STOP INTO A SILENT LOSS (2026-08-27)

*(aeon's line, against this lane's own sweep.)*

`git worktree remove` refuses when a worktree holds uncommitted work. **`--force` overrides
precisely that refusal.** This lane removed fourteen worktrees with `--force` without
inspecting any of them first, and **can no longer establish whether anything was lost, because
the evidence went with the directories.**

**An unrecoverable unknown is worse than a known loss** — a known loss can be repaired, and
nothing about an unrecoverable one ever announces itself. Inspect for uncommitted files, and
for a running process holding the directory (a peer's build died `spawn ./build.sh ENOENT`
the same afternoon), *before* reaching for `--force`.

### A PIN DIES WITH ITS PARCEL — UNLESS SOMEONE ELSE HAS ADOPTED IT (2026-08-27)

*(aeon's amendment to the hub's Q-28, proposed against their own correct behaviour.)*

Q-28 says a pin created for a parcel dies with the parcel. The aeon lane deleted
`.aeon-freeze-slope` under exactly that rule and **the rule was being followed correctly** —
it was their parcel pin and their parcel had finished. **But this lane had adopted it as its
gates' ground the moment `AEON_DIR` pointed at it, and that transition left no trace in either
tree.**

**The parcel's owner is structurally the party who cannot see the adoption.** Their bookkeeping
says *done* and nothing contradicts it. So: **whoever ADOPTS a pin declares it.** The
owner-declares rule above covers where a sweeper looks; this covers the adoption being recorded
at all. As written, Q-28 licenses this again, correctly, on the next lane that tidies up after
itself.

### `AEON_DIR` IN EVERY BRIEF: UNCONDITIONAL, **EXCLUSIVE**, AND **PREPARED** — three claims (2026-08-27, widened 2026-08-29)

`test_support.rs::aeon_dir` defaults to `/home/volence/sonic_hacks/aeon`, which is the
owner's live checkout. **So any suite run that does not set `AEON_DIR` reaches the tree he
is authoring in**, and `sigil build --aeon <tree>` **emits its ROM and listing INTO that
tree** — not merely reading it.

That happened: a brief here said *"a stable aeon tree, **if needed**"*, an agent ran the
workspace suite without setting it, and `s4.debug.bin` / `s4.debug.lst` in the owner's tree
were rewritten by a run neither he nor the aeon lane knew about, while he was testing live.
His authoring was untouched — no re-bake ran — but the debug ROM he would have loaded had a
provenance nobody intended.

**`"if needed"` is the whole defect, and it reads as helpful flexibility.** A default that
points at a person's working directory is one that must be overridden *every* time, and
every-time is what briefs are bad at. **Set it in the template, not per parcel.**

**WIDENED 2026-08-29, after this rule was followed and still produced a collision.** Setting
`AEON_DIR` unconditionally is necessary and is not sufficient. Both of that night's briefs set
it — and both set it to **the same tree**, which was **bare**. Two further claims were being
relied on and neither was stated:

- **EXCLUSIVE.** `build.sh` writes into the tree and rebuilds `rm`-first, so two agents sharing
  one `AEON_DIR` transiently delete each other's reference ROMs. It surfaced as one failure in
  an otherwise clean 4090-test run (`SIGIL_STRICT_GATE set but reference missing: …/demo.bin`),
  green on a re-run seconds later. **The gate was right both times; the tree moved under it.**
- **PREPARED.** A fresh `git worktree add --detach` of aeon is **source with no reference
  ROMs** — measured: 213 failures on first contact, every one `reference missing`, and
  `engine/debug/generated/` absent so `demo debug` could not even resolve. So a bare tree is
  not a reference tree; it is a **build job**, and the agent has no choice but to write into
  the thing the brief calls a reference. **The collision was designed in at dispatch, not
  stumbled into at run time**, and a mid-run instruction of mine not to rebuild the tree was
  already unfollowable when it arrived.

**The template line, therefore:** prepare one tree per agent yourself (detached at the
provenance `aeon_rev`, all four shapes built), then say in the brief **which** it is —
prepared-and-exclusive, or bare-and-yours-to-build. The agent cannot tell by looking.

**And the reading lesson, which outlives the fix:** *a path is not a state.* `AEON_DIR` names
a location; every claim that matters is about the **condition** of what is there. This is the
bound-vs-procedure rule one level up — the brief stated a *value to set* where what it needed
was a *property to hold*.

**Watch the perturbation direction, too.** A shared tree can manufacture a false **green** as
easily as a false red — a gate that should have refused finding a stale ROM another agent
built. Red is this hazard's visible face and green is its invisible one, so *"it passed on a
re-run"* closes the instance and says nothing about the class. Make agents state which
artifacts a measurement depended on, not merely whether it repeats.

The same agent's own numbers are the argument: its runs before the correction showed **54
phantom failures**, its runs after showed a clean set. This document already carried the
phantom-failure warning; it was not in the artifact that had to carry it. **Knowing a rule
and encoding it in the instruction that carries it are separate acts.**

### A GREP HIT THAT CONFIRMS WHAT YOU SET OUT TO FIND IS THE ONE YOU LEAST READ THE CONTEXT OF (2026-08-27)

*(aeon's, against their own error.)*

Protocol bar 11 says read the lines around a cited line before accepting what it proves.
This is sharper: **when the hit confirms your hypothesis, the confirmation is the reason you
stop reading** — so the risk is highest exactly where the evidence looks best, and **the
search itself has selected for agreement.**

Instance: that lane grepped a gate's name, read the hit, and reported the gate unarmed. The
hit was inside a *fast-mode skip banner* — a correct statement about what fast mode omits —
and the gate runs unconditionally on the canonical build, four lines further down. It went
into a plan, into a message, and was echoed back as a work item. **It happened on a day whose
running theme was vacuous gates, which is why the search produced what it was looking for.**

### GUARD THE ARTIFACT, NOT THE SUBCOMMAND — `target/release/sigil` (2026-08-27)

The aeon lane pins `target/release/sigil` for freezes and A/Bs. The rule everyone was
given was **"do not run `cargo build --release` in sigil"**. That rule is written at the
wrong level and does not work.

**`cargo test --release --workspace` is not `cargo build`, and it relinks the same
binary.** A landing run from this lane relinked it at 07:20:23 (`fbf60abd` → `52882e2e`
→ `537869e6`) while aeon's seven-ROM freeze was pinning it and while the aurora lane was
mid-build against it. Nobody broke the rule as stated. **Anyone honouring it exactly
would do the same thing**, because the rule names a subcommand and the thing that needs
protecting is an artifact.

**The rule, restated at the level that works:** any invocation that can relink
`target/release/sigil` — `build`, `test`, `run`, `clippy --fix`, anything cargo — is a
mutation of a shared artifact. Guard the file, not the verb.

**Announce a relink at the time, every time, to every lane — not just the one that asked.**
This lane's first version of this rule was agreed bilaterally with aeon, who had asked for
the hold. The aurora lane then reported being mid-build against the same binary, and was
never part of that conversation. **The holder cannot enumerate who depends on a shared
artifact**, so a bilateral promise silently excludes everyone it did not name. The
announcement is cheap and every recipient who does not need it discards it for free.

**A standing permission expires when the grantor's conditions change.** "Safe to relink"
was true when aeon said it, at a boundary with nothing measuring. They then dispatched a
parcel whose CRC baseline depended on the binary. Nothing was violated in words; the
conditions under the grant changed and neither lane re-read it. Treat any "go ahead" on a
shared artifact as expiring the moment the grantor dispatches work of their own.

**Pin the ARTIFACT with a hash, never with `revision:`** *(aurora's, 2026-08-27)*. A
revision names a property of the **source**, so it cannot detect a relink of the **file**
by a lane that has legitimately moved on — the pin stays "correct" while the binary under
it changes. An `md5`/`sha256` of the binary is a property of the file, changes exactly when
the file changes, and needs no cooperation from whoever relinks. Use that shape for any
hold-still arrangement.

**The cheap structural alternative to all of the above: give every worktree its own
`CARGO_TARGET_DIR`,** which is already this document's rule for a different reason. A run
that cannot reach the shared `target/` cannot relink the pinned binary, and then none of
the announcement discipline is load-bearing.

**THE ANNOUNCEMENT NAMES EVERY BINARY THE BUILD PRODUCES, DERIVED BY A COMMAND — NEVER A
HAND-PICKED LIST** *(2026-08-29; the aeon lane raised it, this lane ruled the shape)*.
`cargo build --release` relinks the whole workspace, so an announcement that names only the
binary someone happened to ask about leaves every other consumer with an unpinned input.

**The hand-list failed on its first outing, and it failed in the worst available way.** The
lifted hold above names three binaries by md5 — `sigil`, `refreeze`, `repin` — chosen by hand
at raise time. The one it omits is `emit_sound_blob`, and that is the only one of the four
that is a **hard build input**: it writes `z80_sound_blob.bin` / `z80_sound_blob_debug.bin`,
its own header declares the output byte-deterministic from the tracked `.emp` sources **and
the sigil toolchain version**, and aeon's `build.sh` invokes it as an ARTIFACT step that dies
loudly without it. So a silent relink of it can move ROM bytes by a path that never touches
`sigil`, and the resulting ROM is internally consistent and self-certifying. A hand-list that
drops the load-bearing member is not a list with a gap — it is evidence that hand-selection
cannot see load-bearingness, which is why extending it to four would have repeated the
mechanism rather than fixed it.

**The rule: emit an md5 for every executable in `target/release/`, from the command, at the
relink.** Nothing is choosing, so a new binary cannot be silently absent.

```sh
find target/release -maxdepth 1 -type f -executable -printf '%p\n' | sort | xargs md5sum
```

Twelve lines today. Twelve is cheap; picking the subset by hand is the step that failed.

**n=2, and the prior instance was banked in the consumer's own tree before either lane
looked.** aeon's `build.sh` carries an ASSEMBLER PROVENANCE banner recording that
`target/release/sigil` "sat three days behind while aeon builds invoked it, and nothing in
the pipeline was capable of noticing", with the reason stated outright one line above: *a
stale assembler and a current one emit byte-identical ROMs whenever the SOURCE has not
changed, so every artifact check we have is silent on which binary produced it.* That lane's
own framing of finding it is the useful sentence and is worth more than the second data
point: **we had seen this before and did not reach for it.** A prior instance sitting
unretrieved in one's own file is its own finding.

### TWO PARCELS INSIDE ONE A/B RANGE CANNOT BE SEPARATED BY DIFFERENCING ITS ENDS (2026-08-29)

Chain 179 carried the tilt AND the insta-shield between its endpoints. Both lanes
differenced 178→179 — the aeon lane to write the freeze's description, this lane to check
it — and **neither could attribute a byte to either parcel, because both were inside the
range.** Neither lane reached for the third revision, and the disagreement that followed
was entirely an artifact of that.

The resolving form is a revision BETWEEN them (`e1f412ed`, tilt-landed and shield-not):

```
CharacterDefs   b12c0141      e1f412ed        4ba7cb92
plain           0x11EA0  →   0x11F10 (+0x70) →  0x11F20 (+0x10)   = +0x80
debug           0x11FC0  →   0x12030 (+0x70) →  0x12030 (+0x00)   = +0x70
```

Endpoints verified here against this repo's own committed `pins.rs` at `13a6d3c8^` and
`13a6d3c8`; the middle column is the aeon lane's measurement.

**Before differencing an A/B range, count the byte-movers inside it.** More than one and the
delta is a sum you cannot decompose — every per-parcel attribution drawn from it is a guess
wearing arithmetic's clothes.

### A PIN FIELD MEASURES WHERE PINS ARE, NOT WHERE CODE IS (2026-08-29)

This lane measured that the insta-shield's `0x10` never enters the **debug** pin field —
correct, and reported to aeon as a question rather than a mechanism, which was the right
call. But it was **framed** as "the debug shape never receives the insta-shield's bytes",
which is a claim about the ROM inferred from the pin field. The bytes are there: aeon's gate
decodes the debug ROM's own bytes and runs 6,912 executions with `PSTATE_JUMP` /
`PSTATE_ROLLJUMP` firing, and the routine's immediate neighbours move `+14` in debug. The
growth propagates locally and is absorbed by slack before the next pin; plain has less slack
and alignment rounds `14` up to `0x10`.

**Slack between a section and the next pin makes the pin field blind to local growth**, so an
absence in that field is an absence in the INSTRUMENT. Same class as bar 16(d) — a failing
lookup tells you about the tree you ran it in, never about the object — arriving on a
positive, quantitative instrument instead of an empty grep, which is what made it hard to
see. When the pin field says nothing moved, the available witnesses are the listing symbol,
the neighbours, and a gate that decodes the ROM.

*(Also settled: `14` and `16` were both right and named different quantities — `16` is the
gate block, which is what plain's tier-3 delta measures; `14` is the routine's net growth,
48 → 62 bytes, the fix also letting `d1` carry the state byte into the roll-jump cancel that
used to re-read it. Neither number was wrong; neither said which it was.)*

## Queue

The standing sigil-native arc is the **`.emp` language work (Spec 2)** — specs in
`empyrean/docs/SIGIL_*.md`. The whole sound stack is sigil-native, the language round
+ §17 optimization arc + conversion tail are done, and the map drives the build.

### REPIN-TESTS-HINT-UNDERLISTED — a hint nothing can contradict, 190 of 412

`repin.toml`'s per-symbol `tests = [...]` feeds one printed rerun hint after a pin drifts
(`repin.rs:167-186`) and nothing else. It gates nothing, so an incomplete list cannot fail.
**190 of 412 rows omit at least one consuming test binary** — upper bound; mechanism and the
`DMA_Overflow_Count` instance are source-confirmed, the count is a name-anywhere grep and needs
narrowing to actual `addr_labels` rows before it is quoted as fact.

**The fix is not editing 190 rows by hand** — that is a population to maintain whose failure
mode is "green because nobody maintained it", which this file rejects twice elsewhere. Prefer
deriving the list, or gating it: the consuming set is mechanically discoverable (which tests
lower the module / carry the label), so the honest shape is a check that the declared list
matches the derived one, or dropping the field for a derived hint. Full detail and the two
method findings are in the dated section above.

### DPLC-ENTRY-INSTRUMENT REPIN — an ask that must outlive both sessions

`parcel/dplc-entry-instrument` (parked on aeon, 621 insertions across `dplc.emp`, `ram.emp`,
`dma_queue.emp`, `vblank.emp`) is the only live byte-mover on that lane and is the CANDIDATE
owner of the +$60-shaped delta above — flagged as candidate, explicitly **not** attribution, by
the session that flagged it. When it moves:

- four cross-seam symbols need the three-site treatment (`repin.toml`, `pins.rs`, addr_labels)
  documented above, with **full** `tests` lists: `DMA_Split_Reject_Count` →
  `dma_queue_port, dplc_port, bg_anim_port`; each `DMA_Peak_*` → `vblank_port, game_loop_port,
  load_art_port`;
- `game_loop_port` and `load_art_port` additionally face region byte gates, DEBUG shape only,
  because the parcel grows `VInt_Level` itself — a byte-gate re-prove, not a table row;
- run the plain-shape falsifier FIRST; it is one command and it indicts the parcel or clears it.

Written here rather than left in a thread because both sessions that held it were rotated the
same night.

### PROVENANCE-REV-REACHABILITY — LANDED. The ledger now judges the revisions it records

`provenance.toml` records `aeon_rev`, `strict.sigil_rev` and `strict.aeon_rev` to give an
attestation tree identity. Nothing validated that those revisions still EXIST: the only
check was `is_full_sha` — 40 hex characters — so **a well-formed orphan passed forever**.

`sigil_harness::rev_reachability` judges each recorded revision against its own remote
branch, read with `git ls-remote` at measurement time. Not a local tracking ref (a cached
answer that goes stale silently — measured here twice in one session: aeon's `origin/master`
moved from `ac20c424` to `a0a5acff` to `4d86f5db` while this parcel was being written, and
the tracking ref in the reference worktree still said `ac20c424`), and not some sibling
checkout's `HEAD`, which is one working tree's opinion rather than the branch.

**FOUR measured states, not two, because the remedies differ:**

| state | means | remedy |
|---|---|---|
| `REACHABLE` | an ancestor of the remote tip | — |
| `OBJECT ABSENT` | this clone has never seen it | `git fetch`. TRANSIENT |
| `AHEAD OF REMOTE` | present; the remote tip is its ancestor | `git push`. Not yet a defect |
| `DIVERGENT` | present; neither reaches the other | **none. PERMANENT** |
| `COULD NOT MEASURE` | no remote / no `AEON_DIR` / unfetched tip | named, never green |

**`AHEAD OF REMOTE` is the state a three-way split would have missed**, and it is the one
that matters operationally: between `--freeze` and `git push`, a recorded revision is
"present but not reachable from the remote branch" — structurally the same sentence as the
orphan, and a completely different situation. Collapsing them makes the report unreadable
during the very ritual it is meant to protect.

**Commands.** `refreeze --reachability` walks the whole ledger and exits 1 on an orphan or
an absent object, 2 on anything unmeasured. `refreeze --check` prints the same walk as a
standing, NON-FATAL report and its verdict is unchanged by it. `refreeze --attest` runs it
over the two revisions it is about to write, BEFORE the suite, and warns.

**WHY THE LEDGER IS REPORTED AND NOT GATED — the design call, stated so it is not
re-litigated as an oversight.** An exception list was rejected: it is a population to
maintain, and its failure mode is "green because nobody maintained it". A ratchet pinned at
entry 181 was rejected on two counts — this module's own doctrine rejects pinned boundaries
twice with a merge-race argument, and more decisively **the ratchet does not work**: entry
182's freeze commit is legitimately unpushed at `--check` time, so a hard rule over new
entries turns the gate red during the normal ritual. Nothing over an append-only ledger's
HISTORY can be a hard gate while chain 181 stands, and re-attesting 181 is refused (it would
record a different tree's run under 181's name). **So the teeth are at the WRITE site**,
where the operator can still act, and the report is unconditional so it cannot go quiet.

**THE LIVE INSTANCE — chain 181 `rebake-after-repaint`.** Its `[entry.strict].sigil_rev` is
`bfbedc11fb52183c08631034a0108be9df01f8bf`, which is **DIVERGENT**: present in the clone,
not reachable from `origin/master`, and `origin/master` does not reach it. The aeon lane
rebased between attesting and pushing; the freeze commit that landed is `16b83c63` and the
two trees differ (`ed5a25ac` vs `7ccd5d3f`), so **no reachable commit carries the tree that
run happened on**. Verified here from the object database, not taken from a brief.

**IT IS NOT BEING REPAIRED, and that is a ruling rather than a backlog item.** Re-attesting
would record a DIFFERENT tree's run under entry 181's name — trading a dangling anchor for a
resolvable wrong one, which is worse for the exact question the field exists to answer. The
entry stands, the report names it every run, and `--check` stays green.

**THE ONE-LINE RITUAL CHANGE THAT PREVENTS THE NEXT ONE — for the aeon lane.** Push the
freeze commit BEFORE `--attest`. A revision already in `origin/master` cannot be orphaned by
a later rebase, so `sigil_rev` names a coordinate that survives. Attesting first records a
commit that is merely `AHEAD OF REMOTE`, and every rebase between that run and the push is
another chance to orphan it — which is exactly how 181 happened. **This lane did not make
that a refusal**: `AHEAD OF REMOTE` is the honest mid-ritual state, and refusing it would
refuse the correct case. Turning the warning into a refusal is the aeon lane's call to make.

### SIGIL-DECOUPLE — the owner ruled 2026-08-26, in this lane's session: follow aeon's plan

Asked whether to adopt the shape aeon nominated, his answer was to follow it. **The
authoritative text is aeon's, not a paraphrase here**: `docs/DEFERRED_WORK.md` at aeon
`822c382a`, verified reachable from their `origin/master` at read time. Four steps in
order — (1) cut the golden cord, sigil vendors a PINNED aeon source snapshot as its corpus
and drift detection moves to a nightly non-blocking job; (2) placement authority comes home
to aeon, anchors declared in `map.toml` and everything else placed fresh; (3) retire
`repin`/`pins.rs` from the landing path into an internal regression tool over sigil's own
corpus; (4) archive the byte-identical certification as a dated historical result.
**Sequencing is theirs and it is step 2 FIRST**, after the showcase — which has landed
(chain 167).

**What the coupling actually buys, measured before the ruling rather than assumed**, because
step 4 will want it and because a future session will otherwise re-litigate this from scratch.
Two families, and only one of them is liveness:

- **The goldens (byte-identity) catch things nothing else does, and they SURVIVE step 1** —
  they simply describe a pinned corpus. Instance with teeth: the one-sided
  `Player_SensorPair` push/pop narrowing is a real runtime bug (wrong angle delivered) that
  left contract closure green, fired no warn tier, and is invisible to every static analysis
  here because nothing models 68k stack byte lanes; the ROM diverging from the frozen golden
  was the only signal. Also the `test_emitter` end-anchor error (caught as a window length
  mismatch) and the `boot.asm` −0xA base slide, which surfaced hardcoded-address fixtures the
  design-gate census had not listed.
- **Liveness catches exactly one family: aeon's newest source hitting a sigil capability or
  measurement gap** — a brick or a mis-measure, never a byte regression. All three 2026-08-26
  instances are that shape: the missing map region for `ojz_effects_editor_act1`; the `repin`
  pin defect where the successor's alignment pad entered `ACT_DESCRIPTOR` (0x27C pinned vs
  0x27A real); and BGROOM-3's `abs.w`, whose real cause was a collision-fallback scratch slot
  aliasing zero. **That family is already covered without the coupling** — the nightly
  source-gate lane and `corpus_builds`' brick witness build every shipped shape from live
  aeon tip and block nothing, which is the same shape step 1's nightly drift job takes.
- **And the coupling is not a general safety net, so do not price it as one:** the `×26`
  stride bug sat green because both twins carried the identical wrong shift decomposition
  (the gate proves twin-agreement, not correctness), and `[layout.odd-field]` drifted four
  days through six zero-byte parcels whose CRCs were verified at every landing.

**The residual cost, stated so nobody discovers it later:** byte coverage AGES. A construct
aeon writes after the snapshot has no golden, so mis-placing it surfaces as a brick rather
than a byte diff, until the snapshot is bumped on sigil's cadence. Aeon stated the matching
cost to the owner before his yes — assembler regressions surface nightly rather than at the
next aeon landing.

**The gate this lane holds on step 1, and it is aeon's own sentence rather than an addition:**
*"Every constraint the frozen tables encode today must be recaptured as an explicit rule
BEFORE the tables stop being authority, or it silently stops being enforced."* Treat that as a
precondition with a deliverable, not a caution. **Enumerate by what TOUCHES placement, never
by what the frozen table lists** (bar 8) — the table is the artifact whose authority is being
retired, so taking it as the enumeration is the shared-frame failure aimed at ourselves.
BGROOM-2's leftovers are the same territory and should be read as part of this, not beside it.

**STATE AS OF 2026-08-26 EVENING — a fresh boot inherits exactly this.**

- **The constraint inventory is COMPLETE and pushed**:
  `docs/superpowers/notes/2026-08-26-placement-constraint-inventory.md`. Eight rows, no
  row left named-but-unlocated. R7 is the one to read first — alignment is *inferred* from
  each pin's own address, **capped at 16**: `native::packed_align_of` returns the largest
  power of two **in {16, 8, 4, 2, 1}** dividing the frozen base, so it distinguishes only
  residues mod 16 and an address divisible by 32 or 64 still infers exactly 16. A repin can
  therefore change a section's alignment with no alignment code changing — bounded, but real
  — and only two labels are guarded against the silent-audio consequence.
  **This sentence was WRONG until 2026-08-29 and it is how the error travelled.** It read
  *"largest power of two dividing the frozen base"* with **no cap at all**, while the
  inventory it summarises had the cap present but buried in a parenthetical behind a
  contradicting headline. **Buried in the source, dropped in the summary, one hop apart, with
  nothing in between able to notice** — and the summary is what a reader of another lane's
  docs reaches first, so the dropped version is the one that propagated. The aeon lane
  reasoned faithfully from it during chain 181 and was about to record a false "R7 fired" in
  a permanent ledger; they found this line. Enumerated the rest at the same time (bar 8, by
  what touches the value, not by what defines it): five other doc restatements of the rule,
  all correct — `2026-08-27-constraint-recheck.md:217` and
  `2026-08-26-config-b-two-byte-growth.md:182` both state the mod-16 bound explicitly. So
  this was one bad summary, not systemic drift.
- **Aeon's `parcel/rom-relayout` LANDED 2026-08-26** — merged here as chain-168 (`6e4f2533`,
  pairing aeon `c3f5cbe0`); see the RELAYOUT-REVIEW section below for the review and the
  sweep that outranked it. **This row said `IN FLIGHT` for a day after it landed**, twenty
  lines from the section recording the landing, and a dispatched agent read this row rather
  than that section and repeated the stale blocker back in its delivery. A booking is prose:
  nothing executes it, so nothing can contradict it (see the absent-and-silent note's prose
  section). When an item lands, the row that BOOKED it is the one that has to move.
- **Island-order piece (1) LANDED** at `1a03c75c`: the MDDBG blob-end guard is proven to
  fire, red-first against two mutants. Bar moved 3939 → 3943 declared.
- **Island-order piece (2) is BUILT on `fix/island-non-vacuous-arm`** (`83b6610e`,
  `2247b0f2`), byte-neutral and touching none of `golden/`, `pins.rs` or `repin.toml`.
  It **is held from merging, and the reason has been corrected**: the aeon pair it was
  booked behind has landed, so that blocker is retired. What holds it now is the
  sprite-owner refreeze in flight in the aeon lane — this parcel changes `native.rs`, which
  the attest's own gates run through, so merging it mid-freeze would move the harness under
  a landing and change what that attest measures. Byte-neutrality does not exempt it: the
  hazard is the measuring instrument moving, not the bytes. Merge when the pair lands.
  - The fail-open is gone: `check_error_handler_is_last` takes its expectation as an
    argument and refuses BOTH directions of a mismatch — a declared island whose blob
    label never appeared, and an undeclared one whose label did.
  - **The expectation is `native::declares_error_handler_island`, which reads no build
    output.** It reconciles the profile's `debug || crash_report` axis with whether
    `profile.registry` carries `engine.debug.error_handler`, and refuses when the two
    disagree or when the registry has lost the exclusive island/`release_fault` split.
    The registry is the module list the build is HANDED, so it is upstream of every
    label the build produces — which is what keeps this off the circular reading, where
    an expectation taken from the listing would assert only that the listing agrees with
    itself.
  - **DEVIATION RATIFIED (bar 7): the dispatch brief was WRONG about where the authority
    lives, and the agent overturned it with evidence.** The brief nominated `map.toml`'s
    `order` list as a candidate. It cannot serve: sonic4's `order` is the UNION across all
    four sonic4-rooted targets and carries **both** `ReleaseFault` and `BusError`
    consecutively, so it cannot distinguish a shape at all — verified here firsthand at aeon
    `origin/master`, `games/sonic4/map.toml:143`, and the file's own header says so at :37
    while :32-33 carries a struck-through superseded ruling saying the opposite. An agent
    that had complied with the brief would have built a per-shape authority on a
    shape-invariant list, and the resulting gate would have been green and meaningless.
    Recorded because the invited-contradiction line in the dispatch is what produced it.
  - The CLI's appendix decision was a THIRD hand-maintained copy of the axis
    (`!matches!(opts.target, Lean)`); it reads the reconciled answer now.
  - **The per-shape set diff is `tests/error_handler_island_membership.rs`**, listed in
    `scripts/nightly_source_gates.sh` — which is not optional, since that lane audits its
    own list and refuses to run on an unclassified aeon-reading test file. Census over
    the seven shapes: **6 carry the island, `lean` is the one that does not**, and the
    gate fails loud if either bucket empties, because a one-sided population makes half
    the diff enforce a rule over nothing.
  - Red-first four ways: two mutants of the guard's match; a standing shadow-tree witness
    that renames the blob label throughout `error_handler.emp` (the build stays clean —
    only the name moves, which is exactly the defect the vacuous arm could not see); the
    reconciliation's three refusals on doctored profiles; and the whole gate driven red
    over real builds by a stale `ERROR_HANDLER_BLOB_LABEL`, naming all 6 carrying shapes.
  - **Bar moves +8 declared** (4019 → 4027 on this branch): +2 in
    `error_handler_island_order`, +2 in `native.rs`'s placement unit tests, +4 new.
  - Open and deliberately not closed: a change that removes the registry row AND clears
    `crash_report` together moves both records at once and stays green, correctly — that
    is a redeclaration of what the shape IS. What is refused is a change to one record
    alone.
  - **Strict suite GREEN: 4023 passed / 0 failed / 4 ignored, reconciling exactly
    against 4027 declared**, `SIGIL_STRICT_GATE=1`, zero skip lines, exit 0.
  - **BYTE-NEUTRAL, proven rather than argued:** the branch's own `sigil` rebuilt all
    four shipped ROMs from a clean aeon checkout at the provenance tip and reproduced
    entry #172's CRC/size for each — s4 `21cc1347`/718999, s4.debug `9732c56a`/735420,
    demo `3415e3ef`/96372, demo.debug `b6f9759f`/101080.

**THE STRICT RUN NEEDS A PAIRED AEON TREE, and the live one is not it.** A strict
`--workspace` run with `AEON_DIR` at `sonic_hacks/aeon` returns **58 failures across 41
binaries** — every `*_debug_region_matches_reference`, the golden full-file/anchor gates,
`pins_rs_is_current`, and the two provenance gates. None of it is a defect in the branch
under test: `provenance_chain::aeon_dir_matches_the_provenance_tip` says so in one line,
naming the tree's revision, the frozen one, and the fix. The recipe that turns the same
branch from 58 red to 0:

```
git -C …/aeon worktree add --detach …/.aeon-island-arm <provenance aeon_rev>
cd …/.aeon-island-arm
SIGIL_BUILD=<target>/release/sigil SIGIL_EMIT=<target>/release/emit_sound_blob ./build.sh
#   … and again with DEBUG=1, and again for `./build.sh demo`, both shapes — build.sh
#   makes ONE shape per invocation and the port gates read all four
SIGIL_STRICT_GATE=1 AEON_DIR=…/.aeon-island-arm cargo test --release --workspace --no-fail-fast
```

Read `aeon_dir_matches_the_provenance_tip` FIRST when a strict run comes back with a wall
of debug-shape byte diffs: it is the one failure that explains the other 57, and the wall
is otherwise easy to read as a regression in whatever branch happens to be checked out.
- **The 79 pad-sweeping region ends convert before the flip** and are this lane's to
  sequence; they touch the aeon-owned lane files, so they wait for the pair too.
- **Ruled jointly with aeon and NOT to be re-opened unilaterally:** when the rules are
  written, declare each section's alignment as what its CONTENT needs; never transcribe
  the accidental quanta. Anything that held only by accident stops holding once, visibly.

**Not started, and two triggers are not this lane's:** the hub declares the project id (it has
said one word from the owner is enough, the yes being banked at aeon `822c382a`), and step 2 is
aeon's ROM-RELAYOUT, which their queue holds for the owner's go.

**Current state (2026-08-22, master `8884e255`, pushed — `origin/master` verified equal by
`git ls-remote origin refs/heads/master`, not by the local tracking ref).** The two parcels
below merged at `cba0a0bc`, which master has since moved past; read this heading for where
master IS, and the per-parcel SHAs for where each landed. Both were
sigil-internal (neither touches `golden/`, `pins.rs` or `repin.toml`, so neither needed
aeon-lane sequencing):

- **`feat/game-defines`** — the aeon `emp_defines` ask, with both lens latents closed. A
  game-declared `[defines]` row now joins the blind-arm polarity net via
  `audit_game_declared_polarity` (exempted-and-named, with reason-less and stale
  exemptions both rejected), and `shape_defines`' NotFound tolerance is gone outright.
  Value-only enforcement at parse time was considered and **rejected**: it would refuse
  the one safe game-declared toggle (a row the two games declare at opposite values —
  exactly what the polarity net wants) while still missing `if CAPS == 20` on a
  non-boolean row. The shipped CLI's failure surface moves: `--report ram` and
  `--report contracts` now exit on a missing `games/<g>/map.toml` where they proceeded on
  built-in rows; a full `sigil build` was already fatal there, the diagnostic just moves
  earlier and names the config instead of the placement.
- **`feat/warn-tier-ungate`** — the source-gate lane (see that section above), the
  `CORPUS_OPEN_FINDINGS` register, and `sigil-source-gates.timer`.

**The reference tree for any artifact-dependent run is aeon master, built** — at the
revision the `provenance.toml` **tip** pairs with, derived at read time per the quality-bar
section, never from a CRC written in this file. A non-tip entry read as the tip once cost this
session a false "corroborated" in a review, and there are 166 entries to pick the wrong one
from. `~/sonic_hacks/.aeon-landing` is a built checkout at that
revision **(it was found ABSENT on 2026-08-26 and re-created as an aeon worktree at
`415e0b6a`. **BUILDING ONE — the corrected recipe; the version this paragraph carried until
2026-08-27 omitted a fatal step and prescribed an unnecessary one, while the correction to the
second already sat 300 lines above it in this same file.**
(a) **`SIGIL_EMIT` is REQUIRED and its absence is fatal, not degraded.** With only
`SIGIL_BUILD` set, both sonic4 shapes die at `ERROR: seam-1 needs the sigil emit_sound_blob
binary`, zero ROMs emitted — **and both demo shapes build fine without it**, which is exactly
the shape the build-all-four rule exists to catch: a half-green reference tree that looks
built. `cargo build --release --bin emit_sound_blob` and export it.
(b) `rm -f` all ROMs before EACH build — a build that stops at a gate leaves leftovers whose
CRCs match the pins perfectly, which is a green that means nothing.
(c) Export `AEON_SKDISASM_DIR`.
(d) **Do NOT run `tools/regenerate-level.sh` prophylactically.** The old note said a fresh
checkout hard-fails `level_staleness.py`; that mechanism is wrong and has now failed to
reproduce **three times** — see the measured correction earlier in this file, and a third
instance 2026-08-27 (`ok (generated … >= editor …)`, same-second whole-second compare). It
bites an in-place `git checkout`, not a `git worktree add`. Run it only if the gate actually
fires; it rewrites the level tree and churns `DONOR_PROVENANCE.json`.
(e) Verify the built ROMs by **CRC32 + size against the provenance TIP entry**, deriving the
tip by parsing the file, never by reading a revision off prose. Note the tip entry pins
**seven** targets; `build.sh` emits four and the suite produces the other three, so the
four-ROM check is the right bar but is not the whole entry.
**⚠ `scripts/nightly_source_gates.sh` has NO audit flag** (`--selftest-fail` is the only
one), and running it to "check the audit" is NOT read-only: it creates worktrees off `master`
in the shared checkouts, builds salvador and regenerates compression vectors. To read
`gates=N unclassified=N` safely, replicate its inline audit block (its own `SOURCE_GATES`
array plus the classification loop) against your own worktree)**; `~/sonic_hacks/.aeon-sigil-gates` is **source-only by construction** and must
never be pointed at an artifact-dependent run.

Previously landed the same day: **const-arity** — a
typed comptime `const` literal is now array-arity checked at elaboration (the length
check lived only in the byte-emission path, which a `const` never reaches, so a
wrong-shaped constant compiled clean); and the **Oracle listing gate's `ORACLE_DIR`
default**, which had gone stale at the oracle/oracle-old rename and made
`SIGIL_STRICT_GATE=1` — the pre-merge bar this very file names — unsatisfiable.

Also landed today: **`feat/m68k-roundtrip`** (an m68k decoder mirror, a round-trip pass
over every shipped shape's emitted stream, and an opcode sweep) — carrying the real ISA
fix it surfaced: `TST` takes `DATA_ALTERABLE` on the MC68000, not `DATA`, so the old row
admitted nine words the hardware traps. Latent, never emitted, derived four ways. Its
packet's soundness claim was corrected rather than kept — the sweep's oracle is
`encode()`, so it proves decoder-subset-of-encoder and **cannot see a defect both halves
share**; `TST` was exactly that, live.

**Front of the queue, in order:**

1. ~~**A provenance witness for the shared binary**~~ — **LANDED** at `9c08f2a5`
   (`feat/version-provenance`). `sigil --version` / `-V` reports the revision, branch,
   commit date, tree state and source dir the binary was built from. Verified firsthand at
   review: an empty commit touching zero source files re-runs `build.rs` and the binary
   reports the new revision (1.25s); a detached checkout reports `branch: detached`, not a
   branch called `HEAD`. Cargo tracks `HEAD`, `<common-dir>/refs` and `packed-refs` — the
   git-dir/common-dir split is load-bearing, since from a linked worktree cargo must be
   given the *main* checkout's common dir. **Tree dirtiness is NOT trackable** — cargo has
   no trigger for uncommitted edits, and the alternatives were priced and refused (a
   repo-root directory trigger never reaches a fixed point because `target/` is at the
   root; an unconditional rerun costs 13.2 s wall / 167 s CPU per invocation to refresh
   one boolean). The banner discloses that limit in place and names the command that
   settles it, and a test asserts the disclosure so it cannot be quietly tidied away.
   **The consumer side has since landed on aeon's `build.sh`** — it parses `revision:`,
   `source:` and `tree:`, flags a revision mismatch against `git rev-parse HEAD` in the
   source dir, flags dirt with the prefix test `[[ "$SIGIL_TREE" == dirty* ]]`, prints a
   banner and makes it fatal under `SIGIL_VERSION_STRICT=1`.
   Not covered by this witness at all: rustc version and build profile.

   **`VERSION-DIRT-CLASSIFY` / `GOLDEN-DIRTY-BANNER` — the warning fires permanently, so
   it tells nobody anything. Half fixed on this side; the other half is a consumer
   negotiation.** Both names are booked HERE; before this they existed only as prose
   references with no entry to read.

   - **What was wrong, measured rather than assumed.** The revision half keys on the
     repository tip, so any commit makes the assembler look stale: measured from the
     binary the aeon lane is using (`fbf60abd`) to master `a7073abe`, **19 commits, of
     which 5 touch anything the `sigil` binary is compiled from**. The dirt half keys on a
     count of changed files, which cannot tell a source edit from a note left in a
     documentation directory — and the recorded noise instance (a modified fixture under
     the harness package) is a *modified tracked file*, so no reading of the existing
     `N modified, M untracked` split can classify it. **The discriminating data was not in
     the banner**; it had to be derived.
   - **The derivation, and it is cargo's, not a hand-list.** `build.rs` runs
     `cargo metadata --no-deps`, walks the transitive non-dev path-dependency closure from
     `sigil-cli`, and narrows each package to what cargo declares it compiles: the whole
     package directory when it carries a build script (a build script may read any file in
     it), otherwise the manifest plus the directories of its non-dev targets' `src_path`s.
     Plus cargo's own fixed workspace inputs (`Cargo.toml`, `Cargo.lock`, `.cargo`,
     `rust-toolchain*`). Every closure manifest is now a rerun trigger, so the set cannot
     go stale behind a relink.
   - **The finding that refutes the obvious model: the closure is ALL 14 workspace
     packages.** Crate-level membership has zero discriminating power in this repo, so
     "is this crate in the closure" is not the question — file-level target layout is.
     `docs/`, lane logs and per-package `tests/` and fixture directories fall outside;
     that is where the 14-of-19 reduction comes from.
   - **Landed on this side (interface-safe):** the `tree:` DETAIL now separates the changes
     in the compiled sources from the rest. The state word is unchanged — every uncommitted
     change still yields `dirty`, so aeon's prefix test behaves exactly as before.
   - **NOT landed, needs the aeon conversation first — written and gated on
     `feat/version-drift-classify`, second commit, DO NOT MERGE that commit until the
     conversation happens.** It narrows the state word (non-source dirt reports
     `clean-sources`, which stops matching their `dirty*` test) and adds three fields:
     `closure:` (how many packages and paths, or NOT DERIVED and why),
     `closure-revision:` (the last commit that reached those paths) and `closure-paths:`
     (the pathspecs themselves, one line, so a consumer needs no cargo of its own).
     **The consumer-side patch it enables is one line** — replace
     `_src_head=$(git -C "$SIGIL_SRC" rev-parse HEAD)` with
     `git -C "$SIGIL_SRC" log -1 --format=%H HEAD -- $SIGIL_CLOSURE_PATHS` and compare it
     against the binary's `closure-revision:` instead of `revision:`. Their existing `sed`
     idiom reads the new fields unchanged and ignores what it does not name, so adding
     them breaks nothing on its own; the state-word narrowing is the half that changes
     their behaviour.
   - **Residual on the closure-revision comparison, stated rather than discovered later:**
     the path list is baked at build time, so after the closure GROWS the binary's list
     lags. It is still sound as a detector, because the commit that grows the closure must
     edit a manifest and every manifest is in the closure — but a lane that ignores that
     first warning is then under-covered until it rebuilds.
   - **What the classification proves, and the line it must not be read across:** it proves
     a change *cannot affect this binary*. It does NOT prove *the output did not change* —
     only a rebuild and a byte compare supports that, and no value derived here may be
     cited as having measured it. It over-approximates by design (build-script packages
     widen to a whole directory; a `#[cfg(test)]` body inside a source directory counts),
     and an underivable closure counts every change as material rather than reporting
     clean.
   - **A real bug the measurement caught**, worth keeping because it is the whole failure
     mode in one character: the status reader trimmed leading whitespace, and `git status
     --porcelain` renders an unstaged modification with a leading space — so the first
     entry's path arrived as `argo.lock`, matched no source region, and a genuine edit to a
     compiled file classified as harmless. A unit test now pins it.
2. ~~**A Capstone differential as a permanent gate**~~ — **LANDED** at `aafe612a`
   (`feat/capstone-differential`). Two gates: `m68k_capstone_differential` (the 65,536-word
   space, 0.5 s, no aeon dependency, default suite) and `m68k_capstone_stream` (all seven
   shipped shapes' emitted stream, default suite **and** the nightly source-gate lane).
   **Non-circularity verified firsthand, not asserted:** under MUTANT T — `Tst` widened to
   `DATA` on encoder *and* decoder together — the pre-existing `m68k_opcode_sweep` stays
   GREEN while the capstone gate goes RED naming `4A3A`/`4A7A`/`4ABA` by word. Loud on
   unmeasurable both directions: capstone absent skips and passes; capstone absent under
   `SIGIL_STRICT_GATE=1` fails naming the reason.
   Comparison is on a normalised abstract form (legality, consumed length, mnemonic family,
   size where capstone reports one, ordered canonical operands) — **never text**, which
   measures spelling. Capstone's *structured* operand detail is deliberately unused: in
   5.0.7 its m68k backend leaves `mem.base_reg` invalid for `(An)/(An)+/-(An)` and
   `mem.disp` zero for absolutes, so a structural compare would measure capstone's own bugs.
   Both named hypotheses re-derived and both turned out **broader than stated**: the `$FF`
   long-displacement escape is a 68020 addition capstone applies inside its 000 mode (16
   words), and the bit-op sizing class is *every* bit-op form, not just dynamic `btst` —
   capstone is wrong in both directions and it costs a length disagreement too. Five
   exclusions total, each value-aware, each asserting its derived class size exactly; that
   size assertion **fired twice during development and was right both times**. Residual
   after normalisation and exclusion: zero, on all three corpora.
   Three findings went to the gap ledger rather than being excluded or papered over — see
   its tail: `encode_bit`'s bit-number field (152 words, a defect **both halves share**,
   byte-neutral to fix and provably so), the odd `.s` branch displacement latent, and the
   unpinned oracle version.
3. ~~**An alignment attribute / even-offset assertion**~~ — **LANDED** at `6fae4d6a`
   (`feat/field-align`). Shipped as `sc_mask_raw: i16 (align: 2)` — error-tier, per-field,
   opt-in, `N` restricted to powers of two, `(align: 1)` the identity and a per-*field*
   `[layout.odd-field]` opt-out (finer than the existing module-wide `@allow`).
   **The spelling is `(align: N)` and NOT `@align(N)`, and this is the parcel's real
   finding:** `@align(N)` is already taken on `vars` region fields, where it *moves* the
   allocation cursor (`parser.rs::opt_align` → `lower/regions.rs::align_to`); D2.29 says so
   outright — *"`vars` regions keep `@align(N)` on fields (reserved space, no bytes) —
   different mechanism, deliberately different spelling."* One spelling with two opposite
   meanings (assert vs move) is a worse trap than the one being closed. `(align: N)` instead
   joins the paren-attribute family, every existing member of which verifies a placement
   rather than choosing one. `@align(N)` written on a struct field is refused **by name**
   with a teaching diagnostic, not mis-parsed as an offset expr opening with the identifier
   `align`.
   **The auto-padding tier was never an open call, and this overseer was wrong to dispatch
   it as one.** Spec 2 §4.3 already rules it, verbatim: *"The compiler never inserts
   alignment or padding — Aeon runs `padding off` globally and hand-pads; an auto-aligning
   struct would silently break byte-exact ports."* Shipping it is a spec amendment, not a
   feature. Read the spec before framing settled law as a design question.
   Verified at landing, not accepted: declared `#[test]` = **3839** on the merged tree,
   run gave **3835 passed / 0 failed / 4 ignored**, `3835 + 4 = 3839` reconciling exactly;
   zero `skip:` lines under `SIGIL_STRICT_GATE=1`; log stamped `head=6fae4d6a branch=master
   AEON_DIR=.aeon-landing@1ee8f8e6`; source-gate self-audit `gates=35 unclassified=0`.
   Delta vs the old 3821/0/4 bar is exactly +14, the parcel's own tests — note that only 13
   carry the `field_align` prefix; the 14th is `vars_form_align_on_a_struct_field_is_refused_by_name`,
   so a `grep -c field_align` under-counts by one and looks like a discrepancy that is not there.
   **The bar moves to 3835 / 0 / 4.**
   **Why error tier and not another warning — and this travels as a POST-MORTEM, not as
   foresight** *(framing at the aeon lane's explicit request, and they are right that it
   matters)*: `[layout.odd-field]` **did** fire across the 2026-08-18→08-22 drift and was
   swallowed by a warning baseline nobody re-read, through six consecutive zero-byte parcels
   whose CRCs were verified and cited at every landing. A lint that fires into a baseline is
   not a guard. That lesson was paid for by the lane that MISSED it, not spotted by the lane
   that saw it — record it that way, because the next person needs to know it is easy to miss.
   Its packet is `docs/superpowers/notes/2026-08-22-field-align-packet.md`; six new
   diagnostic strings are a cross-repo interface (aeon fixtures assert exact text),
   enumerated in its §10.
   The superseded queue text, kept because it is still the best statement of WHY: the
   class-level fix for the
   odd-field finding: today a struct wanting even-aligned members can only say so by
   hand-counting bytes into a pad, and the pad goes stale silently. Aeon's own fix has
   LANDED (`9a718f74`, `ensure(offsetof(Scene, sc_mask_raw) % 2 == 0, …)`), so this now
   has a live subject to retire rather than a hypothetical one.
4. ~~**`pad_to(N)` — a derived-width struct pad**~~ — **LANDED** at `ffa7bdb8`
   (`feat/struct-pad`), implementing Spec 2 §4.3.1 / D2.38 at empyrean `2000b5ca`. Both
   spellings ship per the owner's `both` ruling (`d-13`): `pad_to(N)` derives the filler width
   from the next field's target offset, `pad(N)` reserves exactly N, and
   `[layout.pad-hand-counted]` (default-on warning) names the mistake and prints the exact
   `pad_to` line when a fixed pad stands immediately before a field asserting `(align: N)`.
   **The design call:** pads are held BESIDE the fields (`StructDecl::pads`, each anchored by
   the index of the field it precedes), not interleaved into one member list. A pad has no
   name, so struct literals, `offsetof`, the emitted-shape check, `resolve_bare_window` and the
   harness's offset harvest all read `fields` unchanged and are correct **by construction**
   rather than by remembering to filter — and forgetting to filter is silent. Only the two
   consumers of a pad's WIDTH change: the layout walk, and the emission that follows its
   offsets. Emission derives the `$00` runs from the layout's offsets, never from what the
   fields emitted, so a field that lowers to nothing cannot slide the pad.
   **Byte-identity is by construction too, and worth stating so nobody re-derives it:** a gap
   between two consecutive fields can ONLY be a pad, because `@ offset` on a struct field is an
   *assertion* checked against the declaration-order offset (`check_struct_field_offsets`) and
   never a placement. No corpus struct carries a pad line, so zero ROM bytes moved in all four
   shapes; nothing under `golden/`, `pins.rs` or `repin.toml` was touched and the parcel stayed
   sigil-internal.
   **Four deviations from the spec's literal rendering, all RATIFIED by this overseer** (bar 7 —
   ruled explicitly, not absorbed): (1) the lint tag is inline in the message, because
   `sigil_span::Diagnostic` has no code field and every other tagged diagnostic here embeds it —
   the spec's sentence survives verbatim as a substring; (2) `[layout.pad-count]` tags both
   spellings, only the noun varying, per the derivation table; (3) the non-int case interpolates
   the value's type name, a rendering the spec does not spell; (4) **`[layout.pad-hand-counted]`
   withdraws when the neighbouring `(align:)` is itself refused** — the fix-it promises "the
   assertion still proves it" and a refused assertion proves nothing, so firing there is false
   advice, and the lint must evaluate the same expression `check_struct_field_align` does, which
   without the withdrawal reports an unresolvable name twice.
   **(4) IS NOW SPEC** — the hub appended it to §4.3.1's lint paragraph in this lane's reasoning
   and recorded the other three as implementation's, at empyrean `08ce4c1` (verified reachable
   from their `origin/main` here at read time, and the clause read back rather than taken).
   D2.38 is marked shipped at `ffa7bdb8`.
   **The owed align debt is CLOSED in the same parcel:** the six `(align: N)` coverage claims
   were re-measured at `67575c3c` rather than taken (all six held); #2 and #3 now assert the
   whole string, #4/#5/#6 gained tests, and §4.3's Scope clause gained
   `field_align_does_not_propagate_into_a_nested_structs_own_fields`, built so the nested struct
   is internally clean and lands at outer offset 1 — a propagating check must report, and must
   not. **`ALIGN-STRINGS` is CLOSED**: the hub verified the four test names on this repo's origin
   and re-grepped aeon for the three pad tags themselves rather than taking the report, then
   strengthened D2.37's "what actually pins them" clause to say all six strings are pinned
   (empyrean `08ce4c1`).
   **§4.3.1 shipped carrying the same false "cross-repo interface" claim D2.37 had just been
   corrected of** — the sentence rode in on a verbatim lift from this lane's draft. Now fixed to
   *"these strings are sigil's; no consumer outside sigil asserts them"*, with a parenthetical
   recording how it happened. **The durable lesson, and the hub named it against itself:** a
   verbatim lift is reviewed for FIDELITY TO THE SOURCE, and that review cannot see a claim that
   is false in the source. Both lanes read it twice and neither was reading for truth. When
   lifting text that carries factual claims, the claims need their own pass.
   **22 poisons, 22 red**, one of which caught a green leak in the parcel's OWN new assertion:
   `final_pad_to_below_the_cursor_names_the_end_of_the_struct` asserted the sentence but not the
   `[layout.pad-overflow]` tag, so a poison left it green while its sibling went red. That is bar
   2 firing on the parcel that wrote it.
   Packet: `docs/superpowers/notes/2026-08-26-pad-packet.md`. **The bar moves to 3928 / 0 / 4.**
   *(Superseded queue text, kept for the reasoning — NOT a queue item; the ratification
   history that led to the landing above.)*
   **RULED ADOPT, 2026-08-22 — but read the provenance before acting on it.** The ruling is the **empyrean overseer's**, made under a
   delegation from the owner they report in his words (*"Sure I'll go with what your decision
   is."*). **This lane did not witness that utterance**; it is a peer's report of an owner
   grant, recorded as such rather than as an owner ruling, and it is reversible by him or by
   evidence. Verdict: **adopt `pad_to(N)`, and `(align: N)` stays MANDATORY alongside it —
   never made redundant by the derived width.** The reasoning, which matters more than the
   verdict: the decisive objection is **silence, not authorship** — auto-derivation converts
   a *detected* defect class into an *absorbed* one, so the assertion is what makes derivation
   safe and therefore cannot be derivation's casualty. That argument bites `pad_to(N)` too and
   not only blanket auto-padding, which is why this is a **pair, not a replacement**.
   **Falsifier they stated, so it is not a preference:** if keeping both makes the common case
   actively worse to write — an author supplying an alignment *and* a pad marker where one
   number used to do — the ergonomics argument wins and the cost should be brought back.
   **SEQUENCING, and this lane is holding it: SPEC TEXT FIRST.** Not built yet, deliberately.
   `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` is the language contract and empyrean's to land;
   implementing before the text exists means the implementation *defines* the spec, which is
   the protocol's own "a spec ratifying whatever shipped" failure. Sigil has offered to draft
   the §4.3 text for them to land.
   **Scope note that outlives this item:** the ruling authorizes **this construct**, not a
   general licence to add `.emp` surface. The next one parks the same way — see the autonomy-
   scope section.
   *(Superseded framing, kept for the reasoning:)* ratified in
   principle by this overseer, not built. The `(align: N)` assertion above **guards** the
   stale constant; it does not remove it. `sc_pad_5D`'s width is still a function of every
   field above it and a human still re-counts it by hand each time the guard fires. The
   proposal (packet §4): give structs the `pad(N)` field form that `vars` region bodies
   already have (`parser.rs::region_field`), plus a `pad_to(N)` sibling whose width the
   engine computes. **This does not need a §4.3 amendment** — the bytes sit on a line the
   author wrote, in declaration order; the compiler sizes a declared pad rather than
   inserting an undeclared one. Nothing appears in a struct lacking such a line, so no
   existing struct changes size and byte-identity is untouched. It kills the staling
   constant outright, with `(align: N)` left as the independent proof that the derivation
   did what was intended. Needs spec text in `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` first.
   **The strongest argument for keeping it parked is not the one this overseer led with**
   (*empyrean lane, 2026-08-22*). "It would be the first construct moving bytes the author
   did not write" is true but second-order. The real cost: **auto-pad makes the defect class
   SILENT.** Today a stale hand-counted pad is *detected* — loudly, by the guard. After
   auto-pad a wrong pad is *absorbed*, the ROM changes size, and nothing notices. That is
   trading a loud failure for an invisible one, worth doing only once the layout rules are
   settled enough that the invisible case is genuinely impossible. Note this argument bites
   `pad_to(N)` too, not just blanket auto-padding — which is why `(align: N)` must stay
   mandatory alongside it rather than becoming redundant once the width is derived.
5. ~~**The remaining ROM-as-sentinel port tests**~~ — **LANDED** at `c75c2ffa`
   (`fix/rom-sentinel-port-tests`). Eleven files / 35 tests now sentinel on the source they
   actually read, via `test_support::reference_tree_for_profile` — derived from the
   `GameProfile` each gate builds (`profile.game_root_rel`, with the `map.toml` sibling by
   path arithmetic), not a copied path.
   **The booked count of ten was wrong, and HOW it was wrong is the finding.** The eleventh,
   `compression_selftest_port`, sentinels on **`s4.debug.bin`** — which an `s4\.bin` regex
   does not match, so re-running the original enumeration returns ten forever and looks
   complete. It was found by a third pass enumerating the presence-check **operation**
   (`.exists()`), which contains no filename at all. Neither of the first two passes was a
   superset of the other: five targets spell the check inline with no named guard (invisible
   to an identifier pass), and the name pass also throws false positives it cannot resolve
   (`golden("s4.bin")` reads *sigil's* golden dir, not aeon's). This is the enumeration-
   parameter bar with a live instance in this repo — **a count is only as good as the
   attribute it enumerated over, and re-running the same pass is not a second check.**
   Classification: **A = 71 files** where the ROM is the subject (untouched), **B = 11 files
   / 35 tests** (fixed), **C = 2** (ledgered — the tree-root `.exists()` probe shape, and the
   lane classifier's use-vs-mention weakness, whose kill condition is a declared
   `ARTIFACT_ORACLE_GATES` array).
   Proven three directions per test: source-only tree (35 pass, doing real work — one built
   five ROM shapes in 18s with no ROM in the tree; all 35 panic there on master); absent tree
   under strict (all 35 FAIL, exit 101, naming the missing path); full built tree (green).
   **Nothing was added to `SOURCE_GATES`, and the brief's implied reason was wrong** — cost
   is not the obstacle (all eleven run source-only in ~50s). The real one: each is measured
   against a *committed sigil artifact* (golden blob, `provenance.toml`, `pins.rs`), so
   between an aeon parcel that legitimately moves bytes and sigil's refreeze they are red by
   design. `repin_pins` argues hardest for inclusion (3.72s, wholly source-only now) and is
   excluded on exactly that ground.
   Verified at landing: **3835 / 0 / 4**, `3835 + 4 = 3839` = declared, zero `skip:` lines,
   log stamped `head=c75c2ffa branch=master`, source-gate audit `gates=35 unclassified=0`.
   The parcel adds no tests, so the bar is unchanged from item 3.
   Packet: `docs/superpowers/notes/2026-08-22-rom-sentinel-packet.md`.

**`feat/arity-cli-fixture` is LANDED, not queued** — `crates/sigil-cli/tests/const_arity_cli.rs`
is on master (added by `a24a1b4f`; the branch tip is an ancestor of master, confirmed by
`git log <branch>` rather than by an empty commit range — see the empty-range trap below).
It is a CLI-level regression test for const arity, driving the built binary via
`CARGO_BIN_EXE_sigil` over a committed poison/control pair, taking no `AEON_DIR` at all.
It exists because the enforcement was covered only at the frontend-unit level while aeon
invokes the **binary** — which is how a three-day-stale shared assembler went unnoticed.
Red-first on both arms: the poison arm against the enforcement reverted, the control arm
against the check made unconditional, because a reject-everything compiler satisfies the
poison arm and is caught only by the control. It also pins a cross-repo interface: aeon
fixtures assert on the exact diagnostic wording `array length mismatch: expected N
element(s), got M`, so rephrasing that diagnostic breaks them.

*(This paragraph sat under "front of the queue" opening with "And", which reads as a fifth
queue item. Prose adjacency is not queue membership — state landed/queued in the sentence
itself, since the next boot's only source is this file.)*

Queue items 1 and 2 both landed —
`feat/version-provenance` at `9c08f2a5`, `feat/capstone-differential` at `aafe612a`. The
merged tree is verified green at the bar above: **3821 passed / 0 failed / 4 ignored**,
`3821 + 4 = 3825` reconciling exactly against the declared `#[test]` count, zero `skip:`
lines under `SIGIL_STRICT_GATE=1`, log stamped and all three new test binaries confirmed
present in it by name. The nightly lane's self-audit is `gates=35 unclassified=0` on the
merged tree. Master is pushed — see the heading of the "current state" paragraph above for
the tip and how it was verified.

**In flight (2026-08-22, dispatched by this session).** Two worktree agents, both
sigil-internal, neither touching `golden/` / `pins.rs` / `repin.toml`:

- **`feat/field-align`** — LANDED at `6fae4d6a`, see queue item 3 above.
- **`fix/rom-sentinel-port-tests`** — LANDED at `c75c2ffa`, see queue item 5 above.

6. ~~**`pub equ` does not export**~~ and ~~**clippy fails on master**~~ — both **LANDED** at
   `1a984bd2` (`fix/pub-equ-export`). Reported by the aeon lane; ruled a **compiler bug, not a
   spec error**, on a fact neither lane had at report time: `parser.rs:546` accepts `pub` and
   threads it into `ast::EquDecl::is_pub`, a field whose own doc comment says it means
   exported, while `item_pub_name()` dropped it. **A modifier that parses and does nothing is
   worse than one that is rejected**, so §7.5 (*"`pub equ` adds module visibility like every
   other `pub` item"*, read at empyrean `origin/main`) stands and the compiler moved.
   **The one-line fix this was dispatched as would have been WORSE THAN THE BUG, and the
   agent measured that rather than arguing it.** Every other exported item renames to its
   module-qualified canonical; a `pub equ`'s definition keeps the **bare** name — the
   construct's purpose. Exporting without changing how imports bind makes `use m.a.{X}`
   resolve to a symbol nothing defines: applying exactly the specified change yields
   `unresolved symbol m.a.WIDTH`. Shipped fix also records which exports are equs and binds
   those to the bare name on the `use{}`, glob **and** prelude paths (all three called
   `canonical()` directly). Sound only because `build_program`'s `[equ.collision]` check
   already makes a `pub equ` name program-unique — its own diagnostic states that invariant.
   `collect_exported` needed no change (the `Vars` special case is for items exporting MANY
   names; an equ exports one), and `collect_defined` must NOT gain an `Equ` arm — that
   reintroduces the dangling symbol.
   **Still open, and worth knowing before writing any new consumer:** a QUALIFIED reference to
   an imported `pub equ` (`m.a.X`) still fails `unknown symbol` — `canonicalize_name`'s
   last-dot path assumes the canonical is `<module>.<item>`, which a pub equ's never is.
   **`equ` is now the only item kind whose bare and qualified spellings disagree.** Bare names
   and `use{}` imports both work. Ledgered OPEN with a kill condition.
   Verified at landing: **3839 / 0 / 4**, `3839 + 4 = 3843` = declared, zero `skip:` lines,
   log stamped `head=1a984bd2 branch=master`, all four new tests present by name, and
   `cargo clippy --workspace --all-targets -- -D warnings` **now exits 0** — that bar was
   failing on master before this parcel. **The bar moves to 3839 / 0 / 4.**
   Packet: `docs/superpowers/notes/2026-08-22-pub-equ-export-packet.md`.
   *(Superseded queue text, kept for the reasoning:)* clippy failed on master with one error,
   pre-existing, surfaced by the README pass (which is docs-only and did not cause it):
   `crates/sigil-frontend-emp/src/eval/const_arity.rs:153`, `clippy::collapsible_match` —
   *"this `if` can be collapsed into the outer `match`"*. Reproduced firsthand on master with
   `cargo clippy -p sigil-frontend-emp --all-targets -- -D warnings`; rustc 1.97.1.
   Almost certainly a lint that tightened under a toolchain bump rather than a regression.
   **Why it is queued rather than hand-fixed at the desk:** collapsing a match arm is a
   control-flow edit in shipping frontend code, and the const-arity enforcement it sits in
   landed only today — a "one-line" refactor there wants the suite behind it, and the CLI
   fixture (`const_arity_cli.rs`) pins a diagnostic aeon fixtures assert on by exact text.
   Small parcel, not a desk edit.

**README, landed** at `a4476e53` (`docs/readme-refresh`), per the owner's relayed directive
(*"let's quickly have everything update their readmes correctly. Doesn't have to be super in
depth."*). The finding worth keeping: **every CLI invocation the README documented was
broken, and none had ever worked as written** — `cargo run -p sigil-cli --` cannot run at all
(two binaries, no `default-run`), `sigil diff` does not exist, `parse`/`build` had the wrong
argument shapes, and a generator bin name was misspelled (`gen-snippet-vectors` vs the real
`gen_snippet_vectors`; the two `sigil-isa` generators ARE hyphenated, so the inconsistency is
in the tree, not the doc). Also corrected: `sigil-frontend-emp` was described as a *future*
track while being what the shipped ROM is lowered from; five workspace members were missing
from the crate table; three named harness gates no longer exist (`assemble_full_rom`,
`m0_regions`, `m1d_rom`/`m1d_debug_rom` — the last three survive only as names in
`repin.toml`/`pins.rs` comments); and the claim that the `convsym` deb2 appendix is out of
scope is false, `sigil build` produces it. **A README's commands are its most-used and
least-checked content — run them, do not read them.**

**Nothing in flight; no agents running.** Master `f02fb22b`, pushed — verified against
`git ls-remote origin refs/heads/master`, not the tracking ref. *(Superseded heading:
`ffa7bdb8`, the pad landing.)*

### RELAYOUT-REVIEW — LANDED 2026-08-26, and the sweep outranks the parcel

The aeon lane's cartridge re-layout merged here as chain-168 (`6e4f2533`, pairing aeon
`c3f5cbe0`). Their landing went red at 3941/2 on `boot_port.rs`'s hand-typed
`GameState_OJZScroll_Init`, which they re-typed. **`d8748933` derives it instead** —
`pins::OJZ_SCROLL_TEST.plain_base`/`.debug_base`, sound because `repin.toml` declares that
region's `start = "GameState_OJZScroll_Init"`, so the pin's base IS the symbol's address by
construction and `repin` regenerates it every freeze. The site stops being a ripple site
rather than becoming a sixth row in a doctrine. The aeon lane **withdrew their doc-row
proposal** in favour of this. Suite on the merged tree 3943/0/4 (3947 declared, reconciled),
zero `skip:`, zero `ratchet:`, clippy `-D warnings` exit 0, log
`~/sonic_hacks/.sigil-verify-d8748933.log`. Bar unchanged — the parcel adds no tests.

**Deriving does not weaken the gate, and the reasoning is worth keeping:** the value is an
INPUT to boot's imm-link, not the oracle. The oracle is the frozen golden window, sliced
from a *different* pin (`pins::BOOT`). A wrong `OJZ_SCROLL_TEST` now emits wrong bytes and
fails here — so the derivation ADDS an alarm. The old literal was never independent either;
its own comment said it was copied from the pin.

**THE FINDING OF RECORD is the sweep, not the fix:**
`docs/superpowers/notes/2026-08-26-hand-typed-rom-address-inventory.md`. Enumerated by the
PROPERTY (a literal address fed into or compared against shipped output) rather than by the
five known filenames — which is the enumeration that let this one hide, and a second pass
over the same five would have agreed with itself (bar 8, aimed at ourselves).

- **Nine region bases across `tranche{2,4,5,6}_negative_probes.rs` are ALREADY STALE** and
  rotted at some earlier re-layout with nothing noticing. The precise claim, measured and
  deliberately weaker than the first draft: repointing tranche4 at its current pin leaves
  all six probes green, so the base is **not load-bearing** for an `assert_ne!` — not
  "vacuous", not "red and unnoticed". The cost is lost intent and a lying comment, plus
  proof that **nothing in the tree can tell you when one of these rots.** Queued as
  `STALE-PROBE-BASES`; convert the `refrom`-slicing ones first, following
  `core_negative_probes.rs`. `tranche5:240` already uses `pins::GAME_LOOP.plain_len`, so
  the idiom is in the file.
- **Rotted independently of any doctrine:** all three off-canonical `assembled_len` values
  in `native.rs` disagree with their own generated frozen tables (config_b says `0x434d0`,
  its table `0x8b6f0`); `lib.rs:69 REGION_A_LMA` is a dead ROM-address const whose sibling
  was already deleted for that reason. Neither has a test reading it, which is why. Queued
  as `OFFCANON-ROT`.
- **`Z80_SOUND_SIZE` stays literal, ruled:** a derivation exists (`seam1::BLOB_LEN_*`) but
  `BLOB_LEN_*` is itself hand-pinned, so it trades two re-type sites for one.
- **`native_full_rom.rs:180` (`Ground_Move_Cap`) must NOT derive** — the file argues at
  `:70-75` that `repin` generates pins from the same listing `convsym` consumes, so a
  pin-derived expectation is circular. The agent listed it, tested the argument, and
  corrected itself. It is an ORACLE literal needing a doctrine entry, not a derivation.

### CONFIG-B +2 BYTES — ANSWERED 2026-08-26; R7 REFUTED HERE AND NARROWED

`docs/superpowers/notes/2026-08-26-config-b-two-byte-growth.md` (+ an explicitly
non-gate investigation artifact, `assets/deb2_appendix.py`, wired into no runner).

**The 2 bytes are not in the assembled image at all**, so the overseer's R7 hypothesis —
that a move silently changed an inferred alignment quantum — is refuted rather than
unproven. `anchor_end` is `0x8b6f0` in **both** chain 167 and 168 (verified here directly
in `provenance.toml`), and `EndOfRom` is where `convsym` writes the `de b2` magic, so the
assembled length did not move and there is no room in the image for a placement pad to have
acted. The bytes are **one 64 KB chunk-block header in the `deb2` debug-symbol appendix at
ROM offset `0x93d66`, value `00 0a`** — verified firsthand in `golden/config_b.bin`, and
the two records immediately following it are `1870`/`21b0`, the in-window halves of
`DPLC_Sonic` (`0x71870`) and `Art_Sonic` (`0x721b0`), the symbols that moved into a
`$70000–$7FFFF` window previously holding none. Non-empty chunks 7 → 8, 2 bytes of header
each. The size model derived for it predicts **all twelve** shipped appendices byte-exactly.

- **Aeon's "44 rows moved, 0 quantum changes" SURVIVES**, re-derived independently from
  sigil's own tables before comparing. It is a proof rather than a sample because
  `packed_align_of` is read only in the `labeled[i]` arm plus seam2's two frozen rows
  (frozen-table rows are the whole population), `derive_frozen_table` writes
  `s.lma + l.offset` so a base moves by exactly its row's delta, and **all 44 deltas are
  multiples of 16** — stronger than "the addresses are 16-aligned", which matters because
  several moved symbols are not (`Art_Tails` %16==10, `GameState_OJZScroll_Init` %16==4).
- **R7 is ONE-DIRECTIONAL, which narrows the inventory's row:** for a packed section
  `tb = align_up(running, a)` is divisible by `a` and the refreeze records `tb`, so an
  inferred quantum **ratchets up and can never silently fall**. The hazard is therefore not
  "needs 16, silently gets 8"; it is a quantum silently *rising*, inserting pad and
  invalidating structural pads. **Code-derived, not measured** — flagged as such in the note,
  and it wants a red-first probe that was deliberately NOT built inside this task (bar 6).
- **The appendix moves `full_size`, the `$1A4` header field and the checksum while being
  invisible to every layout-side gate** — which is exactly what made 2 bytes look alarming.
  The only guard is the coarse `min_appendix..=0x10000` band. `lean` carries no appendix at
  all. Whether that band is the right bar belongs to the appendix contract's owner.
- **The `0x2a3c0` exact landing is by construction, not coincidence:** zero symbols below it
  moved (appendix chunks 00/01 byte-identical), and `Map_Tails`' section took the order slot
  `HeightMaps` vacated, both taking `align_up(running, 16)` from an unchanged cursor.
- **TAGGED for foreground:** nothing here is runtime-confirmed; the "debugger-only data"
  claim is read off the code.

**This paragraph used to say `pad_to(N)` was PARKED ON THE OWNER and must not be
dispatched. That is no longer true and the contradiction was live for a day** — the
autonomy-scope section above recorded the owner's `d-6` ruling while this tail still told
a fresh boot to stop. A cold boot reads the tail. **The ruling governs: the `.emp`
language is this lane's to design and drive; the obligation is to put a new surface to
him before it lands, not to wait for him to raise it.**

**LANDED 2026-08-24 — the blank-import spelling (`use base._`).** Queue item `WARN-2`.
Pulls a module into the use closure while binding none of its names: the closure-edge
idiom that until now could only be spelled as a bare whole-path `use`, which
`[import.no-names]` cannot tell from an unfinished import. Owner-agreed spelling, put to
him with two alternatives (a `require` keyword; an `@allow` opt-out with no new surface)
and chosen by him. Merged at `4e262006`.
- **`_` stays an ordinary identifier to the lexer.** It is reserved BY NAME at the two
  places a module path is written — `use` and the `module` header — so one spelling never
  carries two meanings, and a module named `_` is impossible rather than merely
  unreachable. Reserving the token outright would have rippled into expressions and types
  for no gain: zero standalone `_` identifiers exist in the aeon corpus.
- **Byte-identical to the bare form, proven red-first** against a reached module that
  emits, so placement and relocation are in scope. Nothing under `golden/`, `pins.rs` or
  `repin.toml` moved, so this stayed sigil-internal and needed no aeon-lane sequencing.
- **The agent's first poison came back GREEN and it caught its own gate, not the guard.**
  A `use` binds names by TWO independent mechanisms — `resolve::ambient_from_uses` injects
  comptime names, `resolve::imports` builds the rename map for link names — and a gate on
  one says nothing about the other. This is the gate-every-consumer bar arriving in the
  frontend: the brief specified the proof as a single check and was wrong to.
- **The dispatch brief was wrong about the cross-repo interface.** It asserted aeon
  fixtures assert this diagnostic's exact text; they do not. `git grep "imports no names"`
  at aeon `origin/master` returns nothing — the id appears only as prose in four docs. The
  claim was over-generalised from the `array length mismatch` case, which IS asserted.
  The wording change is additive anyway, so every existing prefix matcher survives.
- **Still open, deliberately: aeon has not adopted it.** Sequencing — aeon writing `._`
  before sigil accepts it breaks their build. The two `CORPUS_OPEN_FINDINGS` rows stay,
  correctly, and now track the ADOPTION; their `kill` clauses and the register header were
  rewritten, because both asserted the edges "cannot be spelled another way", which is
  false as of this parcel.
- **Ledgered, not done:** `native.rs::synthetic_entry_src` writes ~88 bare `use` lines
  that are blank imports in every sense, suppressed today by a `SourceId` filter. Adopting
  `._` there would let that suppression narrow. Adjacent to the golden lane, so not taken
  incidentally.
- **Owed to empyrean:** `SIGIL_SPEC2_LANGUAGE.md` documents the `use` grammar and does not
  carry the fourth suffix. Sigil has offered to draft it; the spec is theirs to land.

A note on the `?? sigil` in `git status`: it is a self-symlink `sigil -> /home/volence/sonic_hacks/sigil`
created 2026-08-20, untracked and harmless. It is NOT a stray nested checkout — resolve it
before reading it as one.

## Standing cross-session obligations (2026-08-22)

The aeon session owes sigil two things, both triggered by sigil work rather than by
time — **ping them, don't assume they are watching**:

- **On the `game-defines` ship notice:** they re-run T8's three measured contexts
  (data-binding layout, struct harvest, RAM harvest) against a capability-derived
  define and confirm all three see it. Cheap, theirs to run.
- **When the alignment attribute lands:** they migrate the two `offsetof(Scene, …) % 2
  == 0` ensures to it and retire them. Their `ensure`s are a workaround for the missing
  language feature, not a fix for the class. Those ensures now EXIST — aeon landed them
  at `9a718f74` (merged `1a794ace`), live in `engine/level/scene_dsl.emp` (cited by symbol,
  not by line: the `:1025,1027` this row carried is exactly the coordinate-rot the protocol
  bars) — so this obligation has a concrete subject.
  **TRIGGERED 2026-08-22:** `(align: N)` landed at sigil `6fae4d6a`. The migration is
  `sc_mask_raw: i16 (align: 2)` / `sc_v_deform_shift_raw: i16 (align: 2)`, deleting both
  trailing `ensure`s. Two cautions for them: the spelling is **`(align: N)`, not
  `@align(N)`** — the latter is the `vars`-region cursor-mover and is refused by name on a
  struct field; and `sc_pad_5D`'s width is still hand-computed after the migration, since
  the assertion guards the constant rather than deriving it (that is queue item 4,
  `pad_to(N)`, parked). Push before citing `6fae4d6a` to them — verify against the remote.
  **CLOSED 2026-08-26:** aeon landed the migration (merge of `parcel/field-align` `1c3dd0cf`,
  verified reachable from aeon origin/master): both fields carry `(align: 2)`, both `ensure`s
  are gone, zero-byte on all four shapes. Their follow-up claim that the `@align(N)`-on-a-struct-
  field refusal had no test was REFUTED: `eval_layout.rs::vars_form_align_on_a_struct_field_is_refused_by_name`
  asserts on the second half of the message; they grepped the first half.

**Sigil owes aeon a warning before changing either of these** *(registered 2026-08-22)*:

- **`pub equ` is zero-byte and listing-visible — aeon depends on exactly that.** It reaches
  the `.lst` and **not** the deb2 appendix; aeon measured it (two added, ROM length
  unchanged) and is using the property as a reachability witness. So it is a contract, not
  an implementation detail: if a change would put equates into deb2, tell them **before**
  it lands rather than letting a ROM length move be their notification.
- **`[map.order-undeclared]` keys on BYTE-EMITTING sections** (`sigil-harness/src/native.rs`,
  verified by them firsthand). That scoping is load-bearing for aeon: it let them ship a
  zero-byte generated section with **no** `order` row, plus a guarantee the build stops by
  name the moment there is content to place. A row would have been inert *and* unverifiable —
  a gate that cannot measure its subject. A well-meant "require `order` rows everywhere"
  tightening would break the honest option; do not take it without talking to them.

**⚠ An empty commit range does not mean an empty branch.** Both overseers read
`git rev-list --count master..parcel/scene-even-align-guard` = 0, an empty three-dot
diff, and `--is-ancestor <tip> master` = true as proof the branch held no work. That
triple is the signature of a branch **already merged** — its commits are in master, so
the range is necessarily empty. The fix had landed hours earlier and the lint fired zero
times. This matters beyond the one incident: those are exactly the commands protocol bar
16 prescribes for converting a name into behaviour, so **the bar's mechanism is
necessary and not sufficient** — the output is two-valued and reads as one-valued.
Disambiguate with `git log <branch>` for its own history, or `--is-ancestor` on a commit
you expect the branch to CONTAIN. Two lanes cross-verified each other and shared the
frame, which is the one thing mutual verification cannot catch.

Their side is banked at aeon `1ee8f8e6` (handoff) and `ba189b40` (the `br_ext` unlock
row, cuttable cold) — both verified reachable from aeon's `origin/master`.

**Local-only anchors — RESOLVED 2026-08-22.** Sigil's `origin/master` sat at `40f862e2`
(2026-08-21) while local master ran 80 commits ahead, so every sigil SHA exchanged with
aeon that day was unreachable from origin, including the arity fixture their unlock row
cites. Master was pushed to `a70e6644` (owner-approved), fast-forward, no history
rewritten — verified by `git ls-remote origin refs/heads/master` against the remote
itself rather than by the local tracking ref, which is the only check that can distinguish
"pushed" from "looks pushed locally". Reachability from `origin/master` then re-verified
per SHA: `a24a1b4f` (the arity fixture), `cba0a0bc`, `5c75b5b6`, `9c08f2a5`, `aafe612a`,
`a70e6644` — all reachable.

**The same class with the sign flipped: a stale CAUTION is a false negative, and it wears
caution as a costume** *(aeon lane, 2026-08-22, from their own doc)*. Everything above
guards against a stale "this is reachable". The inverse is a stale **"you cannot cite
these"** — aeon's handoff carried, as standing hazard #1, that every sigil SHA was
local-only with `origin/master` at `40f862e2`. Once sigil pushed, that row made their lane
refuse a perfectly good anchor. **Nothing fails, nobody is told, and the lane looks rigorous
while being wrong** — which is why this direction outlives the positive one. Retracted on
their side at aeon `8ccef438`.
**Checked here, so the next boot does not re-check it:** this document carries **no**
mirrored row. `grep -n 'local-only\|unpushed\|not pushed'` returns exactly one hit, the
general two-directions rule immediately below, which is the durable form rather than a
snapshot. The rule generalises past reachability: **a caution is a claim, and it expires
like one.**

**The rule that outlives the incident, and it did NOT stay true by itself:** a note saying
"this is local-only" is true when written and rots on the next push, exactly as a note
saying "this is reachable" rots on the next rewrite. **Whoever acts on a cross-repo row
re-verifies reachability at read time rather than trusting either kind of note.** The
general form, which composes with the SHA-class rule: **a SHA has a class, a path has a
time, and a revision has a reachability.** Nothing about sigil being pushed today makes a
*future* citation fetchable — push before you cite, every time, and verify against the
remote.

For anything else read newest-first: the dated notes in `docs/superpowers/notes/`
(start with `2026-08-22-warn-tier-drift-open.md`), then
`docs/superpowers/notes/campaign-gap-ledger.md`, whose tail carries eleven rows added
2026-08-22 — refinement bounds unchecked on every binding form, interface `const`
members getting a shape-only check, the `emp_const_rhs` scraper that breaks on any
const gaining a type, a Capstone differential as the only non-circular ISA oracle,
and the `--extra-entry` liveness hazard (a red obtained there proves an assertion's
logic, never that it is reached).

For live next-work, read newest-first: the most recent dated `HANDOFF`/packet notes
in `docs/superpowers/notes/`, then `docs/superpowers/notes/campaign-gap-ledger.md`
for banked nice-to-haves. Keep this section's "current state" paragraph fresh when
landing an arc — a stale queue snapshot misleads the next boot more than no snapshot.

### THE EXPLICIT-PATH `git add` RULE PAID OFF FOR REAL (2026-08-29, this lane's near-miss)

**A `cd` into a sibling repo persisted across a compound command**, so a `git add docs/OVERSEER.md`
and a `git commit` intended for sigil executed **inside the aeon checkout** — the owner's live
tree, carrying **76 modified files** at that moment (his editor content plus another lane's
agent output) and one untracked file.

**Nothing happened, and the reason is the rule this file already bars.** `git add <explicit path>`
matched a file that was unmodified there, so nothing staged and the commit refused with *"no
changes added to commit"*. **`git add -A` or `git add -u` would have staged 76 files of somebody
else's uncommitted work and committed them to their master** — the exact incident that got
`git add -u` barred here in the first place, arriving from a direction nobody had modelled: not
carelessness inside your own tree, but *precision inside the wrong one*.

**Two things worth keeping.**

**1. The dangerous ingredient was the `cd`, not the `git`.** A compound command that changes
directory leaves the shell there for the *next* call. Every safety rule downstream — verify the
branch, enumerate the paths — was followed, and all of them were answered by the wrong
repository, confidently. `git branch --show-current` would have said `master`, truthfully, about
aeon. **Prefer `git -C <path>` to `cd`, and pass absolute paths to file-editing scripts**, so a
stale working directory cannot silently re-target a correct command.

**2. The lane that had spent the night cataloguing wrong-tree failures committed one.** The
refreeze retraction, the shared-`AEON_DIR` collision, the poisoned `target/`, the tracking-ref
rule — all four are *"the operation was right and the tree was wrong"*, and this is the fifth,
authored by the session banking the other four. **Knowing the class does not confer immunity;
only a mechanism does**, which is the same conclusion the protocol reached about typed SHAs.

## Decision-card reader audit — sigil's ledger is NOT clean (2026-08-30)

**3 of 16 lines in `docs/decisions.jsonl` do not render on the owner's Dominion console.**
The number was unmeasured before today, and "unmeasured" was being carried as if it were zero.

**Listed, not fixed.** Rule 8 forbids rewriting a ledger line and rule 8d states nothing is
rewritten, so this section records the defect and changes no byte of `docs/decisions.jsonl`.

### The measurement

`tools/decisions_reader_audit.py` transcribes `parseDecisions`/`parseEntry` from dominion
`server/src/decisions.ts` at `796bc1e`, plus the timestamp rule it imports from
`contractTime.ts`. Run it as `python3 tools/decisions_reader_audit.py docs/decisions.jsonl`.

```
lines: 16 total / 13 parse / 3 rejected

reasons:
     3  options[0] is missing "name"

rejected lines:
  line 14  d-14  - options[0] is missing "name"
  line 15  d-15  - options[0] is missing "name"
  line 16  d-16  - options[0] is missing "name"
```

**Lines 1-13 (`d-1` through `d-13`) all parse.** The defect is confined to one decision.

**The histogram undercounts the defect at field level.** `parseOptions` validates the list as a
whole and returns on the first failing member, so it names `options[0]` and stops. In fact **all
three options of all three lines lack `name`** — nine missing fields, reported as three. A
per-line count is what the console drops on, so three is the number that matters, but a lane
reading the histogram as a repair list would fix a third of it.

### THE 8c TRAP IS REAL HERE, AND IT COMPOUNDED TWICE

Rule 8c closes a decision by **appending** a restatement carrying the same question, options and
recommendation with `supersedes` set. **A faithful closure of a card that does not itself parse
reproduces the defect verbatim and creates a second rejected line.**

Sigil's ledger is that trap running twice:

| line | id | supersedes | options missing `name` |
|---|---|---|---|
| 14 | `d-14` | — | 0, 1, 2 |
| 15 | `d-15` | `d-14` | 0, 1, 2 |
| 16 | `d-16` | `d-15` | 0, 1, 2 |

**One malformed entry became three rejected lines by being closed correctly.** Fidelity is the
mechanism: 8c asks the closing entry to restate the original, and an entry that restates a
missing field is missing it too.

**The blast radius is the whole question, not one card.** `resolveSupersedes` only ever sees
entries that parsed, so a rejected superseding entry leaves the earlier one standing — but here
the earlier one is rejected as well. **`d-14`, `d-15` and `d-16` are the same embed-path question
(`EMBED-BASE-SKEW`), and none of the three reaches the owner's console.** The decision and both
of its closures are invisible there; the ledger on disk is the only record.


### THE SHARPEST HALF: `d-15` WAS A DELIBERATE SCHEMA REPAIR THAT STILL DID NOT CONFORM

The audit agent could not see this, because it needed this file's own record of *why* `d-15`
exists. `d-15` is not an ordinary 8c closure. It was filed **specifically to fix a schema
defect**: `d-14` carried a `state` field the contract does not define, a `refs.commits` key it
does not list, and a `refs.queue` id that existed nowhere. Confirmed against the ledger — `d-15`
does drop `state` (line 14 top-level keys carry `state`; lines 15 and 16 carry `supersedes`
instead). **It was a real repair and it worked on the fields it aimed at.**

**And it still does not render, because every option is still missing `name`.**

That is the finding, and it is a stronger one than the compounding. The repair was made against
the **contract prose**, which a person reads, rather than against the **reader**, which is what
actually decides whether the owner ever sees the card. Nothing in the loop could report the
difference: the contract text was satisfied, the entry looked conformant to its author, and the
only party able to refute it was a TypeScript function in another repo that nobody ran.

**The reusable form, and it is this file's standing class arriving on a fix rather than on a
stale comment:** *conforming to the written rule and conforming to the implementation that
enforces it are two different claims, and only one of them has a consumer.* A schema repair that
is not run against the parser is unexecuted prose with a repair's confidence attached — the same
family as the rerun hint nothing checks and the booking nothing executes, except that this one
was authored **as a correctness act**, which is exactly the moment the check gets skipped.

**CLOSED FORWARD, NOT REPAIRED (2026-08-30).** `d-16-answered` is appended: a conforming re-file
of `d-16` under the amended 8c, supplying `name` on each of the three options and rule 8d's
`answered` block (`by: hub`, `chose: remove`, `said: null` — the hub ruled, so there are no owner
words to quote). Question, options and recommendation are reproduced from `d-16` unchanged.
**The three malformed lines stay exactly as written** — verified append-only, one line added and
zero removed — and remain on the do-not-repair list above. Both instruments agree the ledger is
now **17 lines / 14 parse / 3 rejected**. The ruling taken in the owner's place is now visible to
the person it was made for, which is the whole point of it being reversible.

**Operationally, from here: `tools/decisions_reader_audit.py` runs before any decision entry is
appended, not after.** The tool existing changes nothing on its own; what closes this is running
it at the write site, which is the same conclusion this file reached about `refreeze`'s teeth
being at the write site rather than over the history.

### What this changes about writing a decision

**A decision entry is not durable until it parses.** The ledger is append-only and the console is
where the owner reads it, so a line that fails the reader is committed history nobody sees, and
8c will faithfully copy the failure forward every time that decision is restated. **Run the audit
before appending a closure**, so a restatement does not inherit a defect from the entry it
supersedes.

**Nothing in this repo runs this tool automatically.** It is operator-run: no CI job, no nightly
gate, no test invokes it. `.github/workflows/ci.yml` and `scripts/nightly_source_gates.sh` do not
reference it, and wiring it is an open call, not a done thing.
