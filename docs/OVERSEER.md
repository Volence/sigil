# Sigil Overseer

How a Claude session runs **sigil** as its overseer. The shared role — solo-first
posture, delegation discipline, review bars, peer protocol — lives in
`empyrean/docs/OVERSEER-PROTOCOL.md`; read it once, then this file for what is
sigil-specific: the landing-lane division, the standing rulings, and the queue. The
worktree/test quirks are NOT here any more — they and every other read-at-a-moment rule
are in `docs/OVERSEER-REFERENCE.md`.

> **This file is the boot read and is bounded** (protocol: *The boot read is bounded*,
> `empyrean/docs/OVERSEER-PROTOCOL.md` at `origin/main`). Dated entries — every night's
> findings, landing records and closed parcel narrative — live in `docs/OVERSEER-LOG.md`,
> append-only, newest last, **not read at boot**; reach for it with `tail`/`grep` when a
> particular night or a cited section is in question. A ruling that must survive a rotation
> is written HERE as well as there, never only into the log. A few sentences below still say
> *"the section above/below"* about a dated section that has moved; those resolve in the log,
> which carries every entry under its original line span.
>
> **Rules that are read at a MOMENT rather than at boot live in `docs/OVERSEER-REFERENCE.md`** —
> how to dispatch, the review and proof bars, the full-suite and landing bars, the nightly
> source-gate lane, the worktree and environment quirks, and the standing artifacts this lane
> depends on. That file is not read at boot; it is read at the moment its rule applies, and this
> file names it by path at every point one of its blocks used to sit. The index is *Read at the
> moment* below, and each row states the moment that triggers its block.
> Since the 2026-09-13 cut it also holds the working detail of each queue project (the rows stay
> here), the ledger gate, and the commitments and obligations between this lane and aeon. There is
> one index, *Read at the moment: the index to `docs/OVERSEER-REFERENCE.md`*, with a row per block.
>
> **Split by WHEN A RULE IS READ, never by size** — owner ruling 2026-09-04T15:38:47Z, carried in
> `empyrean/docs/OVERSEER-PROTOCOL.md` at `origin/main`. **No rule is shorter for having moved**;
> every block in the reference file is its boot-read text, verbatim. A few sentences in either
> file say *"above"*, *"below"* or *"this file"* about text that now sits in the other; those
> resolve across the pair, and the index names every block that moved.

## `docs/lane-status.json` IS EDITED, NEVER REGENERATED: an orienting read is not a basis for a rewrite

**This seat destroyed nine live queue rows and two of the owner's open questions on 2026-09-09, at
boot, in one write.** The mechanism is worth more than the incident because nothing about it looked
careless at the time.

The boot read of the file was `head -60`. That is the RIGHT instrument for orienting: it shows the
focus, the blockers and the front of the queue, which is what a booting session needs. The defect
was carrying its output into a **different act**. Later, writing the status, this seat composed a
COMPLETE REPLACEMENT of the file out of what the partial read had shown it. Everything past line 60
ceased to exist.

**Three properties made it silent, and all three are structural rather than attentional:**

- **The file is gitignored**, so there is no diff, no history and nothing to notice it against.
- **A file written from scratch is internally consistent by construction.** Nothing in the output
  is malformed; the console read it `ok` and the card rendered cleanly with nine fewer rows.
- **The loss is invisible to the obvious check.** The hub verified that a specific named row had
  survived the restructure, and it had. **Confirming that a named item survived cannot see what
  left beside it, because the survivor is the only thing in the frame.**

**THE RULE: this file is EDITED, never regenerated.** Load it, mutate the fields that changed, write
it back. A partial read composes safely with an edit and catastrophically with a replacement. Do not
restate this as "read the whole file first", which is advice nobody follows under time pressure and
which puts the burden in the wrong place: the durable property is that the WRITE preserves what the
read never saw.

**And the reason it matters more than the row count, which is the hub's formulation:** a decision
card carries the QUESTION, a queue row carries the WORK. `AS-NAMELESS-LABELS-RC1` is the row the
answer to `d-29` lands on (`d-24` is a different question, answered by the owner 2026-09-04; this
line said `d-24` until 2026-09-10 and was one of three places naming three different cards for one row). Destroying the row while keeping the card means his go arrives at
a board with nothing to catch it, and the failure appears days later as a decision that seemingly
produced nothing.

*(Recovery, if it happens again: Dominion's `dominion/.dominion/changes.jsonl` carries per-row
`added` / `left the queue` entries with the row's title text and prior state, and the
`no longer waiting on you` entries carry dropped `blockedOnOwner` text. It holds titles and states;
it does not hold sizes. Leave an unknown size ABSENT rather than guessing it.)*

## `decisions.jsonl`: FOUR IDS REWRITTEN IN PLACE, DO NOT REPAIR (contract rule 8f)

**Listed once here as rule 8f requires, and deliberately not repaired.** `d-23`, `d-24`, `d-27` and
`d-29` were rewritten in place by the owner-directed audit session at `32f26b02` (+4/-4 in an
append-only file); the pre-rewrite state is at **`b64a8af8`**. 8f's discriminator is *repair when a
GATE IS RED and the repair clears it; otherwise list and leave* - **no sigil gate is red**, the
answers render correctly, and an append would add a shadowing record without removing the violated
line. Anchor: empyrean `2d8ab09`, `contract/DECISIONS.md` rules 8e and 8f, verified reachable from
their `origin/main`.

**Read at the moment, not here.** *A CORRECT ACTION CARRYING A WRONG RATIONALE PROPAGATES THE
RATIONALE* sat at this point and is in `docs/OVERSEER-REFERENCE.md`: the d-25 instance, the rule
that downstream readers can act only on the sentence, and the standing check that a
`blockedOnOwner` id is grepped in the ledger for its `state` before it is filed. Read it when you
are writing the REASON for an edit to a record, or filing anything as blocked on the owner.

## STANDING: DO NOT BOOT INTO A STOP (owner): sleep mode is NOT the condition. Read this before waiting

**CORRECTED 2026-09-12, on the hub's catch, verified here at empyrean `origin/main` (`ec553e2` an
ancestor).** This heading said *"while sleep mode is on"* until then, and a boot that day measured
Dominion's `/ws` sleep state to decide whether to stop. **The licence does not depend on sleep mode.**
Its ground is empyrean `docs/OVERSEER.md`: the still-live *"Do not boot into a stop and wait for a
pick"* and *"a lane rebooted mid-project does NOT stop at its boot stop waiting for a pick - its pick
is its own `next` row"*, under his 2026-09-11T18:23:49Z standing instruction. Find them with
`git -C ../empyrean show origin/main:docs/OVERSEER.md | command grep -n -e 'Do not boot into a stop' -e 'does NOT stop at its boot stop'`.
Sleep state matters only for ROTATION, which fires when the mode is armed AND inside its window;
`enabled:true, asleep:false` is armed and outside it. The two rulings below are the history of the
same licence, and their gate still applies: take a row not waiting on another lane or an owner call.

**Two owner rulings, verified here firsthand at commits reachable from empyrean `origin/main` rather
than taken from the relay.** Both are his verbatim words in empyrean's `docs/OVERSEER.md`:

```sh
git -C ../empyrean show cdb72e9b:docs/OVERSEER.md | grep -n -A3 '2026-09-02T17:17:18Z'
git -C ../empyrean show 61dfcaa8:docs/OVERSEER.md | grep -n -A6 'Turning on sleep mode'
```

`cdb72e9b`, 2026-09-02T17:17:18Z: *"If something stops we have it work on the next item to get
through that list please (unless it's waiting on something else)"*. `61dfcaa8`, 2026-09-03T04:24:58Z,
sleep mode armed: *"If anything besides seraph runs out of tasks from the lists you can give it some
other stuff to work on (sigil can start looking to replacing AS in the github disassembly …) as long
as other agents aren't waiting on them for anything"*.

**So a fresh sigil session does NOT stop for a go** (this said *"while sleep mode is on"* until
2026-09-12; see the correction above). It takes the next queue
row that is not itself waiting on another lane or an owner call; the named fallback is
**SIGIL-AS-REPLACEMENT**. The `overseer` skill's
boot stop is real and is overridden HERE, by him: it names an exception for a standing instruction
from the owner, and this is one. **Two sigil sessions booted into that stop on 2026-09-04 alone.**

**The gate is a condition to MEASURE.** Check *"aren't waiting on them"* against the other lanes'
live rows — and a row naming sigil settles nothing either way: on 2026-09-04 three named sigil while
the thing all three waited on was finished and pushed. **Ask whether sigil could release it, not
whether sigil is mentioned.**

**Provenance disclaimer, carried deliberately:** this reached the lane through the hub. A relay of
his ruling is his ruling — but only because the two commands above were run here and the join
checked. It authorizes taking the next unblocked row and nothing further; it does not authorize
landing anything he has parked. **Derive the parked set from `docs/decisions.jsonl`, not from this
paragraph** — it named `d-24` until 2026-09-04, when he answered it and the work landed.

## STANDING: REPORT TO THE HUB WHENEVER YOU FINISH OR STOP (owner, 2026-09-03)

**Owner ruling, all lanes.** Verified here firsthand rather than taken from the relay:
`f04afe3` is reachable from empyrean `origin/main` and
`git -C ../empyrean show f04afe3:docs/OVERSEER.md | grep -n 'loosk like aeon'` returns it at
line 55. His words: *"tell the agents any time theyy finish work or stop to report to you
please, loosk like aeon's stopped right now"*.

**Send the hub (`empyrean-01`) one message whenever anything leaves this lane with nothing
running** — a landing, a boundary, a block, an owner question, a dispatched agent returning.
Say what landed (**SHA from git output, never typed from memory**) or why you stopped, and what
you need. **Going quiet without a message is the state he named**, and note what prompted it:
he could see a lane had stopped and could not see why. `lane-status.json` is not a substitute —
it is a pull, and this ruling is a push.

## STANDING: SAY WHEN YOU NEED A CONTEXT CLEAR (owner, relayed 2026-09-09) - the sibling of the report-when-you-stop rule

**Owner ruling, all lanes, transcribed by the hub rather than summarised:** *"I'd like to keep going
through lenses and fixing the items they found for whatever's left. Remind the agents to let us know
when they need a clear."*

**Do not silently run down to a compaction or drift through one.** When a clear would help, say so in
the message to the hub AND put it in `docs/lane-status.json`'s `awaiting`, with what a fresh session
needs to resume. **The anchor is a file at a committed SHA, never a summary** - this lane's whole
boot design rests on that and a handoff written as prose is the one thing a rotation cannot carry.

**PROVENANCE, and it is the reason this paragraph exists rather than a bare rule.** This reached the
lane BY RELAY and was **not in empyrean's tree when it was banked**: checked at their `origin/main`
`1530dd3`, `docs/OVERSEER.md` carries neither phrase. So it is the hub's transcription of his words,
recorded here under this lane's own standing practice that a banked ruling carries its provenance in
its text, and under protocol bar 20, mail is not part of any tree. **A later session finding it in
empyrean should prefer that copy and may delete this disclaimer; a later session finding it nowhere
else should treat it as a relay, which is his ruling, and re-verify the join before leaning on it for
anything costly.**

**AMENDED TWICE THE SAME DAY, and both amendments tighten it. Report a MEASUREMENT, not a feeling
(oracle), and be LOUD ON UNMEASURABLE (aurora).**

Oracle's half: the risk is not a session refusing to ask out of stubbornness, it is that **a session
near its limit is the least able to judge that it is.** So report the figure and let him decide, the
same way `updatedAt` comes from the clock and never from your own sense of the time. A lane that says
*"I feel fine"* has produced the one artifact nobody can check.

Aurora's half, which closes the hole in oracle's: **the only counter a session can see is not
necessarily monotonic.** Aurora measured theirs RESET UPWARD mid-session, roughly 13.65M back to
15.0M, so a percentage derived from it has a confident shape and no meaning, which is the exact
artifact this rule exists to prevent. **So: report the figure if you have one AND can name what you
read it from; if you cannot, say `unmeasurable` and name why, and never substitute the feeling it
would have replaced.**

**The second half is sufficient on its own, and it is the half that actually protects the work:
what is UNBANKED, and the resume anchor.** A lane with nothing unbanked can be cleared at any moment
at zero cost, whatever its counter says, which is why "everything is committed and pushed" is a
better answer to this rule than any percentage.

**The half that is easy to drop: it is a rule about ASKING, not about lasting.** Running long is not
a virtue here and a lane that clears early loses nothing, because everything load-bearing is in the
tree by construction. The failure this prevents is the silent one, a session degrading through a
compaction while its board still reads current.

## STANDING: A LANE SAYS WHEN IT CAN BE CLEARED (owner, 2026-09-11T23:19:31Z)

**His words, heard directly by the hub and verified here at empyrean `9911fd6`, reachable from their
`origin/main`** (`git -C ../empyrean show 9911fd6:docs/OVERSEER.md | grep -n "A LANE SAYS WHEN IT CAN BE CLEARED"`):
*"the overseer should say when theyy can be cleared, I think theyy should tell you then you just clear them if
yyou're automating them, and clear yourself if you're able to"*, answering the hub's report that no lane had been
cleared since the 18:17Z launch. At 23:20:23Z he agreed to the rest of the hub's proposal: *"every half hour is fine
but I agree with the rest of your #3"*.

**What this lane does:** past about 200k tokens (the hub's figure, which his agreement adopted), reach the next landing
boundary WITHOUT dispatching the next wave. At that boundary set `atBoundary: true` with an empty `inFlight`, tell the
hub it is clearable with its size as a measured figure (and name what the figure was read from), and end the turn. A
session cannot clear itself and the hub cannot clear a lane; he presses Clear+Reboot, or sleep mode does it on a
`rotate` verdict.

**And the traffic changed with it:** message the hub only when the lane STOPS or NEEDS something, not after every
landing. Landings go in `docs/lane-log.jsonl`, which his changelog reads. This supersedes the per-landing reading of the
2026-09-03 report-when-you-stop rule below; that rule's "stop" half stands.

## STANDING: CUT THE CEREMONY (owner, 2026-09-02T18:20:19Z) — OUTRANKS every process bar here

**Read it at the artifact, not from this line, which deliberately does not restate it:** empyrean
`origin/main`, `docs/OVERSEER.md`, the bullet beginning *"2026-09-02T18:20:19Z — CUT THE CEREMONY"*,
carried by empyrean `90554f2` —
`git -C ../empyrean fetch -q origin && git -C ../empyrean show origin/main:docs/OVERSEER.md | grep -n "18:20:19Z"`.
**⚠ ITS CONDITION IS SPENT, 2026-09-10, AND THE CLAUSES DO NOT ALL FALL WITH IT. TEST IT, DO NOT
TRUST THIS LINE:** `EFFECTS-W1` reads `done` in the hub's committed `contract/projects.json` at
empyrean `origin/main` (since 2026-09-06), so **the OUTRANKING has lapsed** and clause 2's
moratorium, which carried the same condition inside its own sentence, is spent with it.
**Clauses 1, 3, 4 and 5 carry no end condition and STAND; they merely stopped outranking.**
So the paired aeon+sigil freeze stays ENDED (clause 1) and nothing here re-imposes it.
**And his own words under the clauses are undated and were never scoped to a project**, *"cut
anything that's arbitrarily slowing us down without an actual good reason"*, **so the
PREFERENCE OUTLIVES THE APPLICATION** and "the moratorium lapsed" invites exactly the wrong
inference. Read at empyrean `origin/main`, `docs/OVERSEER.md`, the 2026-09-02T18:20:19Z bullet,
which carries the clause-by-clause split; the hub found the same spent condition in its own copy
the same day, and this lane's copy had it too. **What governs what this lane WORKS ON tonight is
his later 2026-09-10 05:45Z instruction, not clause 3**, which is 8 days older and narrower.
It is what suspends the two `blocked` queue rows below, it
ended the paired aeon+sigil freeze (so this lane's nightly drift observer is the net, after the fact,
never a gate on an aeon landing), and **it is the ruling that forbids hand-trimming this document** for
its own size gate — over the bound, history moves out in one cut, nobody shaves a ruling to hit a number.

**Why it is written here at all:** it reached this lane only through mail, and mail is not part of any
tree, so a rotated session booted without it while two queue rows cited it. The trace that existed —
inside `decisions.jsonl`'s *answered* card `d-21`, as supporting ground for a different question — is
the shape that reads as coverage and is not: `decisions.jsonl` is not the boot read.

## NO ACTIVE HOLD — read this before running anything that builds

**A hold that lives only in a chat message does not survive a `/clear`** *(aurora's)*. An
announcement reaches the sessions that exist; only a committed artifact reaches the ones that
do not exist yet. **A committed hold has the opposite failure: it outlives its reason and
nothing announces that either.** So every row carries its date, who to ask, and what ends it —
a successor EVALUATES it rather than obeying it, and a row that cannot be evaluated is expired.

| Raised | Artifact | Why | Ends when | Ask |
|---|---|---|---|---|
| *(none)* | — | — | — | — |

**Read at the moment, not here.** Two blocks that sat at this point are in
`docs/OVERSEER-REFERENCE.md`: **the positive freeze witness** — why `pins.rs unchanged` is an
absence, why `golden/offcanonical_sizes/s4.txt` is a positive witness, and the two limits that
keep it from being adopted wider than it earns — read when certifying that a length-neutral
parcel's build actually ran; and **`d-18`: REFUSE BARE, OPT IN TO PARTIAL** with the
`contract/SUITE_PATHS.md` resolver precedence, read when a run refuses for want of a reference
tree.

## Read at the moment — the nightly drift watch

The drift watch's timer — the unit name, the commands that arm and disarm it, and the measured
reason **armed is not the same as producing evidence** — is in `docs/OVERSEER-REFERENCE.md`. The
job holds no expectation of its own, so waiting for it to build up a record accumulates nothing.
Read it before planning any work off the drift record; `SIGIL-DECOUPLE` step 4 is the queue row
that would.

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

**Re-read it mid-session** — boot-time is the only read anybody performs unless you make yourself
perform another. Triggers: a peer cites a bar you don't recognise, before dispatching a wave, at any
landing. (The measured drift that earned it: `docs/OVERSEER-LOG.md`, 2026-09-04 cut.)

## Landing-lane division — THE rule for this repo

The aeon↔sigil landing lane has **one owner, and it is the aeon overseer** (owner
directive, 2026-08-19). Aeon-paired byte-movers — golden refreezes, the provenance
chain, pin updates — land through aeon's session. One session sequencing them is why
a 16-refreeze day (chain 134→149) had zero collisions; a parallel refreeze against
shared goldens is exactly the collision this rule prevents.

**⚠ THE FREEZE HALF OF THIS IS SUPERSEDED — read the CUT ruling at the top of this file before
acting on the paragraph above.** The owner's 2026-09-02T18:20:19Z cut **ended the paired aeon+sigil
freeze**: aeon certifies alone with its own gates, sigil is the only writer of its own chain, and a
sigil landing gates nothing of aeon's. What survives here is the **sequencing** rationale and the
coordinate-before-touching list below. What does NOT survive is "a byte-moving sigil parcel must
land through aeon's session" — that is no longer true, and **this overseer acted on the stale
reading as recently as 2026-09-03** before checking the ruling. What aeon actually needs from a
byte-relevant sigil landing is **one line: did the four shapes move, and which** — so their next
run does not read a delta as their own parcel's doing.

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
sequence the landing. **A byte-changing parcel is `refreeze --freeze` plus ONE hand-edited
file.** The freeze regenerates the goldens, the size tables, `pins.rs` (through `repin`) and the
provenance entry in one step, and `repin.toml` changes only when a region is added. The hand-edited
file is `crates/sigil-harness/tests/repin_pins.rs`, which holds TWO different things: the currency
gate `pins_rs_is_current`, which the freeze satisfies, and `generated_pins_match_the_hand_typed_baseline`,
hand-typed literals that countersign the freeze and that the freeze deliberately does NOT regenerate.
**Every byte-moving refreeze brief carries the resync step:** derive each new literal from the
parcel's own moved-section record, only then compare it with the regenerated `pins.rs` (a
disagreement is a finding, never a value to copy), and put each literal's cause in the commit body,
not in the test comment. Precedents: `c14fca39`, `cd7551dd`, `3097dd0d`. *(Until 2026-09-12 this
sentence called the file a currency gate nobody edits by hand. That conflated the two tests, and
the placement landing went red on the second one because its brief trusted this line.)* Neither
`mixed_dac_rom.rs` nor `engine.inc` is tracked in this repo (`git ls-files` returns
neither). **Over-pricing is the error direction that survives**, because an over-estimate
never fails loudly — it just makes byte-movers get deferred. That ripple belongs to the
aeon-owned lane.

Provenance identity is **CRC32 + size**, never SHA1 — the campaign standard, and the implementation is
**IEEE/zlib CRC-32 rendered as 8 hex digits**. Verified 2026-09-16 in both directions rather than
assumed: `crates/sigil-harness/src/native.rs:4445` documents itself as *"CRC-32 (IEEE, the campaign
provenance standard alongside byte-size)"*, and `golden/provenance.toml` carries `s4 =
"91c46c94/820209"` while `python3 zlib.crc32` over a freshly built sonic4 plain gives `0x91c46c94` at
820209 B. **This AGREES with the wire contract** (empyrean `origin/main`, `contract/protocol.md:1170`:
`crc32` is IEEE/zlib CRC-32, 8-hex, `$defs/hash32`; line 5845 pins it to zlib over the file), so there
is one convention here, not two, and nothing to reconcile at a lane boundary.

**⚠ A DECIMAL CRC FIGURE IN THIS LANE IS A FOREIGN CONVENTION AND IS THE TELL.** `cksum` is a
DIFFERENT checksum, not a different rendering: on that same file it gives 559008248 where zlib gives
2445569172 (`0x91c46c94`). Every figure in `provenance.toml`, the chain and the lane log is 8 hex
digits, so **a decimal checksum anywhere in this lane came from somewhere else and does not compare to
the chain.**

**THIS SEAT COMMITTED THE SPURIOUS-AGREEMENT FAILURE WITH IT, WITHIN AN HOUR OF THE HUB NAMING THAT
CLASS, AND THAT IS THE ENTRY.** A subagent reported its 7-shape table in `cksum`. Checking its figure,
this seat reproduced 559008248 **with `cksum`** and recorded that the agent's number "reproduces
exactly" — **verifying a foreign number with the foreign tool and reading the match as corroboration.**
The agreement was real and meant nothing about the lane's chain, which is the hub's own point made
flesh: *a spurious DISAGREEMENT prompts an investigation, while a spurious AGREEMENT is
indistinguishable from real corroboration.* The near-miss was that the lane's own hex convention was
sitting in `provenance.toml` the whole time and one `grep` separated them.
**THE BAR: check a peer's or an agent's figure with THIS LANE'S instrument, never with theirs.**
Reproducing someone's number with their own tool tests transcription and nothing else.
*(What survives unharmed: the byte-NEUTRALITY conclusion, which was established by building both arms
and comparing directly, plus sizes that match `provenance.toml`. Identical is identical under any
checksum. Only the quoted figures were in the wrong currency.)*
Suite rule, adopted 2026-09-16 at empyrean `6f91aa4`: a checksum written into a record as evidence
names its implementation.

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
**⚠ `d-3` IS NOT OPEN AND HAS NOT BEEN SINCE 2026-08-24, and this paragraph said it was for
seventeen days while the paragraph ABOVE it recorded the answer.** `docs/decisions.jsonl` carries
`d-3` at 01:38:39Z and `d-6` at 02:00:22Z the same night: *"Settled, and you answered in your own
words rather than picking one of the three I offered."* **So the broad reading IS the ruling**,
propose, discuss, land, and the clause that used to sit here told every cold session not to fund
work off it. Found 2026-09-10 by grepping this file for the SHAPE of a conditional rule rather
than for a clause already known wrong (the hub's sweep; four lanes, four hits).
**The general form is this document's own defect aimed at itself: a rule whose CONDITION names
something outside the rule is never re-tested, because the condition is read to decide not to act
and therefore never read again.** Grep the shape, not the clause.

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

## Read at the moment: the index to `docs/OVERSEER-REFERENCE.md`

Every block in `docs/OVERSEER-REFERENCE.md`, written `ref` below, is read when its moment arrives and
never at boot. Each row gives the moment in bold, then the block by its heading and the file it lives
in; a long heading is quoted by its opening words, which is enough to grep for. There is one row per
block, whichever cut moved it, including the blocks banked straight into `ref`. The six per-cut
indexes this one replaced, with each cut's own description of its blocks, are in `ref` under *The
per-cut indexes, as each cut wrote them*. Each cut's closed history is in `docs/OVERSEER-LOG.md`
under that cut's heading.

### Writing a brief, or dispatching an agent or a seat pair

- **Writing a brief for any agent**:
  - *Dispatch practice* (ref).
  - *THE FACES OF A CHECK* (ref, inside *Dispatch practice*).
  - *A BEFORE/AFTER STREAM DIFF* (ref, inside *Dispatch practice*).
- **Writing a brief that names an aeon tree**: *`AEON_DIR` IN EVERY BRIEF* (ref).
- **Building a count, or writing a brief**: *THREE THINGS THIS PARCEL TAUGHT* (ref).
- **Naming a path in a brief**: *AND MY OWN INSTRUCTION CAUSED* (ref).
- **Handing an agent a tree, or promising a peer exclusive use of one**: *AN EXCLUSIVE-TREE LEASE* (ref).
- **Choosing the corpus a parcel is verified against, or sizing one off it**: *THE MOTIVATING CASE*
  (ref).
- **Writing a bar, a brief, or a diagnostic that tells somebody what to check**: *A PRESCRIPTION IS
  NOT A PROOF* (ref).
- **Writing any seat brief**: *THE EXCLUSIONS THIS LANE'S SEATS* (ref).
- **Dispatching the UX seat pair**: *THE UX SEAT PAIR* (ref).
- **Dispatching a seat pair, or two seats agree on a finding**: *CONVERGENCE IS EVIDENCE* (ref).

### Building a proof, planting a control, or judging a green or a zero

- **Building a proof, or judging a claim, a green, or a detached run**:
  - *Rules banked from closed findings* (ref).
  - *A re-baseline does not explain a green* (ref).
- **Certifying that a length-neutral parcel's build actually ran**: *Freeze, proof and review bars*,
  which opens with *PREFER A POSITIVE FREEZE WITNESS* (ref).
- **Building or judging any red-first proof, or writing a bar for someone else**: *A PROOF THAT
  CANNOT FAIL ON DEMAND* (ref).
- **Proving a mutation applied**: *`git checkout <rev> -- <path>` STAGES* (ref).
- **Designing a probe for anything on an error path**: *ERROR-PATH BEHAVIOUR* (ref).
- **Making something newly refuse**: *WHEN A CHANGE MAKES SOMETHING NEWLY REFUSE* (ref).
- **Judging a control, or a report's own severity**: *A CONTROL CAN BE CONFOUNDED* (ref).
- **Planting any canary or control, or believing a refutation you produced yourself**: *A CONTROL CAN
  VERIFY THE WRONG PREDICATE* (ref).
- **Designing any check whose clean result you intend to believe**: *A CHECK CAN BE BLIND* (ref).
- **Believing a zero after planting a canary**: *A CANARY PROVES* (ref).
- **About to believe a zero from `grep`, or choosing the instrument for a search**: *SHELL `grep -r`
  SKIPS GITIGNORED FILES* (ref).
- **Judging an instrument, or believing a comparison**: *THE FOUR-CORPUS SWEEP* (ref).
- **Judging an A/B, or any pair whose arms agree**: *AN A/B WHOSE ARMS AGREE* (ref).
- **About to claim two code paths agree**: *VARYING A FLAG IS NOT VARYING A ROUTE* (ref).
- **A reduction fails to reproduce the fault**: *AND A REDUCTION CAN SUPPLY* (ref).
- **Judging a derivation that walks a structure**: *A DERIVATION THAT CAPTURES PARENTS* (ref).
- **Matching a reference implementation's arithmetic**: *THE RIGHT BASE COMPUTED THE WRONG WAY* (ref).
- **Carrying a bound past its first use**: *A BOUND IS SAFE ONLY IN THE DIRECTION* (ref).
- **Building a guard**: *AND THE PIN FAILED* (ref).
- **Acting on a verdict from a one-line or compound command**: *PRINT `pwd` AND `HEAD`* (ref).
- **Judging any claim whose ground is a comment**: *A COMMENT BESIDE A DECLARATION* (ref).
- **A peer's result agrees with yours**: *CUSTODY-INDEPENDENCE* (ref).
- **Relying on a change log to reconstruct prior state**: *A TICK-DIFF LOG* (ref).
- **Running any mechanical text change**: *A SWEEP KILLS ANY NEGATIVE ASSERTION* (ref).

### Landing, committing, and running the suite or a gate

- **Preparing, judging or landing a full-suite run**: *Quality bars* (ref).
- **Starting any full-suite or landing run**:
  - *DO NOT COMMIT WHILE A GATE IS READING THE TREE* (ref).
  - *A MONITOR FILTER* (ref).
- **Writing any commit**: *NEVER PASS A COMMIT MESSAGE THROUGH `-m "..."`* (ref).
- **Landing anything that touches the AS frontend**: *A PARTIAL RUN IS NOT A LANDING GATE* (ref).
- **Assembling a gate run by hand, or retyping a command out of one**: *I RAN A SUBSET OF THE LANDING
  GATE* (ref).
- **Touching section alignment or the packing walk, landing anything this file describes as
  pending, or judging a green**: *Standing rules*, independent of whether a row is active (ref).
- **A strict `--workspace` run comes back with a wall of byte diffs**: *SIGIL-DECOUPLE: the strict
  run needs a paired aeon tree* (ref).
- **A run refuses for want of a reference tree**: *d-18: REFUSE BARE, OPT IN TO PARTIAL* (ref).
- **A source-gate notification fires, or a new `crates/*/tests/*.rs` reads the aeon tree**: *The
  source-gate lane* (ref).

### Trees, worktrees and the shared binary

- **Setting up a worktree, or running any cargo command in this shared checkout**:
  - *Worktree and environment quirks* (ref).
  - *THE REFERENCE TREE IS `.aeon-sigil-ref`* (ref, inside *Worktree and environment quirks*).
- **About to sweep, delete or rebuild trees under `~/sonic_hacks/`**: *STANDING ARTIFACTS THIS LANE
  DEPENDS ON* (ref).
- **Pruning worktrees**: *WORKTREE PRUNING* (ref).
- **About to touch the shared `target/release/sigil`**:
  - *A DO-NOT-TOUCH RULE WITH NO NAMED OWNER* (ref).
  - *THE OWNERSHIP BAR I WROTE TODAY* (ref).
- **Planning any work off the drift record**: *The drift watch's timer* (ref).

### Invoking `asl`, generated artifacts, and the version banner

- **About to invoke `asl`, or to quote a value out of one of its listings**: *Selecting and citing the
  `asl` oracle* (ref).
- **Touching a committed generated artifact**: *RULED: the three golden-vector headers* (ref).
- **Changing the `--version` banner's tree vocabulary**: *THE `--version` BANNER'S TREE VOCABULARY*
  (ref).

### Working a named queue project

- **Working SIGIL-AS-REPLACEMENT or dispatching a parcel under it, or sizing any parcel off
  a diagnostic count**: *SIGIL-AS-REPLACEMENT: the rulings and measurement rules* (ref).
- **Sizing or dispatching AS-NAMELESS-LABELS-RC1**: *AS-NAMELESS-LABELS-RC1: the sizing detail* (ref).
- **Working PER-PARCEL-TERM-FEED-CUT, retiring an assert while it is red, or quoting a
  suite figure**: *PER-PARCEL-TERM-FEED-CUT: the three rules* (ref).
- **Working SIGIL-DECOUPLE step 1 or step 4**: *SIGIL-DECOUPLE: what the coupling actually buys* (ref).
- **Working SIGIL-DECOUPLE step 1, or answering the hub on its scope**: *SIGIL-DECOUPLE: the step-1
  gate* (ref).
- **Dispatching or implementing the `pad`/`pad_to` parcel**: *`pad`/`pad_to`: the spec text's
  landing* (ref).
- **Working the `pad`/`pad_to` surface**: *`pad(N)` / `pad_to(N)`: the owner's ruling* (ref).
- **Asked why aeon's board says `blockedBy: sigil`**: *LS-13b, THE AEON ROW BLOCKED ON SIGIL* (ref).
- **Aeon's `parcel/dplc-entry-instrument` lands or moves bytes, or a `+$60`-shaped delta appears**:
  *DPLC-ENTRY-INSTRUMENT REPIN* (ref).
- **Attesting a freeze, or reading the rev-reachability report**: *PROVENANCE-REV-REACHABILITY* (ref).

### Aeon and the other lanes

- **Acting on a go, a ruling or an authorization that reached this lane by relay**: *THE TEST for a
  relayed authorization* (ref).
- **Swapping the shared sigil pair, exchanging byte predictions with aeon, landing the align
  parcel, or comparing a peer's `dac_shared_bank` figure**: *COMMITMENTS MADE TO AEON 2026-09-07* (ref).
- **Shipping `game-defines`, changing `pub equ` visibility or `[map.order-undeclared]` scoping,
  reading an empty commit range, or acting on a cross-repo row**: *Standing cross-session
  obligations* (ref).
- **Briefing a parcel that adds a `mark`, a listing-visible `pub equ`, or anything else the
  deb2 appendix records**: *A `mark` IS NOT ZERO-BYTE* (ref).
- **Another lane reports that a `pins.rs` value disagrees with their tree**: *A PEER'S "YOUR PIN HAS
  DRIFTED"* (ref).
- **Sending or quoting a CRC to another lane**: *A CRC QUOTED WITHOUT ITS AEON REVISION* (ref).
- **Reconciling two lanes' numbers**: *AND "NOT WORTH CHASING"* (ref).
- **Enumerating a symbol's consumers, or pricing a deletion**: *A NAME-STRING ENUMERATION* (ref).
- **Marking a queue row `routed`, or handing a peer a routed finding**: *A ROUTED ROW MUST NAME ITS
  ARTIFACT* (ref).
- **Something arrives that appears to satisfy a caveat you are carrying**: *A CAVEAT RETIRED BY AN
  ADJACENT IMPROVEMENT* (ref).
- **Attributing a finding**: *ATTRIBUTION, CORRECTED TWICE* (ref).

### Queue rows, records, rulings and standing documents

- **Writing a queue row's title**: *AND A ROW'S TITLE* (ref).
- **Filing a row as blocked, or reading a row that says it is**: *A BLOCKER IS A CLAIM* (ref).
- **Writing the reason for an edit to a record, or filing anything as blocked on the owner**: *A
  CORRECT ACTION CARRYING A WRONG RATIONALE* (ref).
- **Citing a decision card as grounds for anything**: *`d-22` IS HUB-ANSWERED* (ref).
- **Touching the ledger gate, or wiring an id-uniqueness check on `docs/decisions.jsonl`**: *The
  ledger gate and rule 8f* (ref).
- **Banking a ruling, or citing one as already-applied**: *A RULING HAS CONSUMING SURFACES* (ref).
- **Writing a citation into source**: *CITE THE ARTEFACT THAT CAN BREAK* (ref).
- **Auditing any standing document for rot, or deciding which stale line to fix first**: *SORT A
  STALENESS AUDIT BY MOOD* (ref).
- **Cutting or reorganising any standing document, or booking a row off text you just moved**: *A
  VERBATIM MOVE LAUNDERS STALENESS* (ref).

## Queue

The standing sigil-native arc is the **`.emp` language work (Spec 2)** — specs in
`empyrean/docs/SIGIL_*.md`. The whole sound stack is sigil-native, the language round
+ §17 optimization arc + conversion tail are done, and the map drives the build.

### SIGIL-AS-REPLACEMENT — active on the owner's own words; source locations LANDED

Project row: empyrean `contract/projects.json`, `state: active`, lanes `["sigil","empyrean"]` —
**verified from the pushed object, not a peer's working tree**: the hub first cited `02e8881`
while it was local-only and it is now reachable from their `origin/main`. Owner artifact is
empyrean `17d0776`, `docs/OVERSEER.md:46`, 2026-09-03T04:24:58Z, naming this lane and this
project as the overnight fallback *"as long as other agents aren't waiting on them for
anything"* — the condition re-derived here over `/api/suite`, 0 queue rows blocked on sigil.

*(Everything else under this row, its standing rulings and the rules for measuring it, is read when
you work the project, not at boot: `docs/OVERSEER-REFERENCE.md`, *SIGIL-AS-REPLACEMENT: the rulings
and measurement rules read while working it*.)*

### PER-PARCEL-TERM-FEED-CUT — SUSPENDED by the cut ruling; three rules survive it

*(**The suspension's condition is SPENT: `EFFECTS-W1` is `done` since 2026-09-06.** The row is no
longer suspended by it; whether it is worth doing is now an ordinary queue question. Full measurement:
`docs/superpowers/notes/2026-09-03-per-parcel-term-feed-cut.md`. Narrative and the closed red:
`docs/OVERSEER-LOG.md`, 2026-09-03 cut, original lines 1205-1256.)*

*(The three rules that survive it are read when you work this row: `docs/OVERSEER-REFERENCE.md`,
*PER-PARCEL-TERM-FEED-CUT: the three rules that survive the suspension*.)*

### REPIN-TESTS-HINT-UNDERLISTED: LANDED

*(The landing record: what the `tests = [...]` hint fed and why deriving it deleted the
population rather than fixing it, the parse-error gate at `53bc85de` with its positive control,
and the 512-not-412 correction with the mechanism that carried the wrong figure:
`docs/OVERSEER-LOG.md`, 2026-09-09 cut, original lines 683-702.)*
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

*(The step-1 gate, the alignment rule ruled jointly with aeon, and the position sent to the hub on
2026-09-06 are read when you work step 1 or answer the hub on this row:
`docs/OVERSEER-REFERENCE.md`, *SIGIL-DECOUPLE: the step-1 gate and the position sent to the hub*.)*

**Not started, and two triggers are not this lane's:** the hub declares the project id (it has
said one word from the owner is enough, the yes being banked at aeon `822c382a`), and step 2 is
aeon's ROM-RELAYOUT, which their queue holds for the owner's go.

### Where the rest of the queue is read from

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

## Rules banked 2026-09-05 - read at boot

The findings of 2026-09-05 that a fresh session needs before it acts. Their episodes are
not here: this section carries the rule and names where the narrative went, per the owner's
2026-09-04T15:38:47Z ruling that the boot read is split by WHEN A RULE IS READ.
The narrative is in `docs/OVERSEER-LOG.md`, 2026-09-05 cut.

**Two live bookings are named only in that narrative, so they are named here too**, because the
queue board `docs/lane-status.json` is untracked and a rotated session cannot be shown to hold
them: **`PINS-GATE-MESSAGE-MISLEADS`** (make the count and the verdict describe the same
comparison; the printed command was fixed at `81d92f80`) and **`AS-IF-REFUSAL-DIAG-VECTOR`**,
which is CLOSED at `34dad07c` and is listed so nobody re-opens it from the narrative alone.

### NO EM OR EN DASHES IN TOOL TEXT (owner ruling, 2026-09-05): booked, and the count is measured

**Verified firsthand at empyrean `24cdd17`, reachable from their `origin/main`**, a 9-line addition to
`design/CHROME_SPEC.md` under "Text in the tools". The owner, verbatim in the spec: *"Can we add no
emdashess to the design list, like no emdashes in an of our tools."* Relayed to this lane with his
broader words: *"get rid of all current emdashes and update so no more emdashes to all the tool
agents"*. No U+2014 and no U+2013 in any text a tool shows a person: diagnostics, warnings, panic
messages, generated help. Use a comma, a colon, a period, or parentheses.

*(How that scope gap was found and closed at empyrean `f9fbfc9`, kept because the fix is the
interesting half: `docs/OVERSEER-LOG.md`, 2026-09-05 cut.)*

**Half (b) is in force NOW, for everything anyone writes here**: strings, docs, lane logs, commit
messages, peer messages. This section is written under it.

**Half (a), the sweep, is booked as `TOOLTEXT-DASH-SWEEP` and takes its turn AFTER the S2
decomposition.** Sequenced by what a person reads first.

**THE MEASURED POPULATIONS ARE DELIBERATELY NOT ON THIS PAGE.** A count read off the boot
read is the snapshot-wearing-the-grammar-of-a-standing-fact defect this file already bans, and
these particular figures were wrong three times before they were right. The producer and
consumer tables, the two corrections they went through, and the pathspec that first returned a
confident zero are in `docs/OVERSEER-LOG.md`, 2026-09-05 cut. **Re-measure with a positive
control and size the sweep off the CONSUMING end**, never off the producer count. One residual
is live and belongs here rather than in the narrative: the shipped regression gate covers Rust
string literals only, so a shell or Python tool can still grow a dash without reddening
anything.
