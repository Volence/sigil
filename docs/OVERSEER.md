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
>
> **Split by WHEN A RULE IS READ, never by size** — owner ruling 2026-09-04T15:38:47Z, carried in
> `empyrean/docs/OVERSEER-PROTOCOL.md` at `origin/main`. **No rule is shorter for having moved**;
> every block in the reference file is its boot-read text, verbatim. A few sentences in either
> file say *"above"*, *"below"* or *"this file"* about text that now sits in the other; those
> resolve across the pair, and the index names every block that moved.

## STANDING: DO NOT BOOT INTO A STOP WHILE SLEEP MODE IS ON (owner) — read this before waiting

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

**So a fresh sigil session does NOT stop for a go while sleep mode is on.** It takes the next queue
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

## STANDING: CUT THE CEREMONY (owner, 2026-09-02T18:20:19Z) — OUTRANKS every process bar here

**Read it at the artifact, not from this line, which deliberately does not restate it:** empyrean
`origin/main`, `docs/OVERSEER.md`, the bullet beginning *"2026-09-02T18:20:19Z — CUT THE CEREMONY"*,
carried by empyrean `90554f2` —
`git -C ../empyrean fetch -q origin && git -C ../empyrean show origin/main:docs/OVERSEER.md | grep -n "18:20:19Z"`.
It is in force while EFFECTS-W1 is open. It is what suspends the two `blocked` queue rows below, it
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

### THE TEST for a relayed authorization (2026-09-02)

**Not "did the hub speak" but "is there an owner decision under this, AND IS IT THIS QUESTION."**
Two clauses, and the second does the work: the first is usually easy and usually satisfied, while
the second is where a general authorization gets silently stretched over a parcel it never
contemplated. **So verify the JOIN, not the endpoints.**

**A relay of his ruling is his ruling; a ruling made in his place is the hub's** — legitimate for
decision cards, and not his word on whether a lane runs.

**The failure this prevents is TIME-DELAYED, which is why a rule is needed and vigilance is not.**
Nothing goes wrong at the moment a relayed go is accepted; everyone acts in good faith and the work
is real. It goes wrong later, when the row is prose and *"the hub said go"* and *"he said go"* are
indistinguishable to a session reading it cold. **That is why a banked ruling carries its own
provenance disclaimer in its text** rather than relying on anyone remembering it.

*(The episode that earned it: `docs/OVERSEER-LOG.md`, 2026-09-03 cut, original lines 124-151.)*
### Standing rules — independent of whether a row is active

**The R7 alignment flip: LANDED at `e2517405` (2026-08-30), verified an ancestor of master.**
*(Its pre-flip measurement, 81 to 91% of declared sections over-aligned with the dominant case
declaring 2 and handed 16, and the ratchet hazard under it, are in `docs/OVERSEER-LOG.md`,
2026-08-30, original lines 1342 to 1391.)*

The packing walk now reads `align_up(running, required_for(head label))`; the declaration is the
ONLY alignment input and a section with no declaration is **refused by name, never given a
default**. `packed_align_of` is gone from every Rust source file in the workspace and survives only
as prose inside `golden/provenance.toml` history. Frozen as chain 196, whose note records seven
shapes shrinking (s4 -62, s4_debug -46, demo -18, demo_debug -36, config_a -62, config_b -152,
lean -14).

**⚠ THIS BLOCK SAID *"what remains is the flip itself"* FOR A WEEK AFTER THE FLIP LANDED**, and it
is the boot read, so every session since 2026-08-30 booted holding a landed parcel as outstanding.
Nothing surfaced it because nothing executes this file, which is this document's own banked defect
(*a snapshot wearing the grammar of a standing fact*) aimed at itself. It was caught on 2026-09-06
only because the hub asked which decouple pieces were landed and the answer was verified in code
rather than read off this page. **The general form, and it is why the correction is written here
rather than quietly applied: a parcel's completion has to be written back to the document that
DISPATCHES it, and no gate anywhere makes that happen.** When you land something this file
describes as pending, edit this file in the landing commit.

**A RE-BASELINE DOES NOT EXPLAIN A GREEN — IT MANUFACTURES ONE** sat here and is now in
`docs/OVERSEER-REFERENCE.md`, beside the four ways a red-first proof goes vacuous. Read both at
the moment you are judging a green or building the witness that gates one — including the
red-first witness this parcel already has.

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
sequence the landing. **A byte-changing parcel costs ONE hand-edited file, not five.**
`repin` writes exactly one — `crates/sigil-harness/src/bin/repin.rs:89` resolves
`root.join("src/pins.rs")` and `:192` is its only write — and `repin.toml` changes only
when a region is added. `crates/sigil-harness/**tests**/repin_pins.rs` is a **currency
gate** (`pins_rs_is_current`), not a site anybody edits by hand, and neither
`mixed_dac_rom.rs` nor `engine.inc` is tracked in this repo (`git ls-files` returns
neither). **Over-pricing is the error direction that survives**, because an over-estimate
never fails loudly — it just makes byte-movers get deferred. That ripple belongs to the
aeon-owned lane.

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

## Read at the moment — `docs/OVERSEER-REFERENCE.md`

Six blocks that sat here at the 2026-09-04 cut are read at a specific moment rather than at
boot, so they live in `docs/OVERSEER-REFERENCE.md` and are read when that moment arrives.
**Nothing was shortened to move it.** Each entry below states the moment that triggers its block, because a rule whose
trigger a reader cannot recognise is a rule nobody looks up.

- **Writing a brief for any agent** — *Dispatch practice*: why a stated MECHANISM is more
  dangerous than a stated FACT, the required *"anything in this brief you concluded was wrong"*
  line, and telling every agent that its branch is deleted at the landing.
- **Writing a brief that names an aeon tree** — *`AEON_DIR` IN EVERY BRIEF*: unconditional,
  exclusive and prepared, plus the template line that carries all three.
- **Preparing, judging or landing a full-suite run** — *Quality bars*: deriving the reference
  tree by property rather than by name, the full-suite bar and its command span, how long a
  strict run takes and why a capped run still reads green, the pairing gate, reconciling the
  declared count, and the port loop.
- **A source-gate notification fires, or a new `crates/*/tests/*.rs` reads the aeon tree** —
  *The source-gate lane*: what it runs, its three buckets and its exit codes, and how a
  warn-tier firing is adjudicated.
- **Setting up a worktree, or running any cargo command in this shared checkout** — *Worktree
  and environment quirks*: including the relink of the shared `target/release/sigil` that
  overwrites the assembler another lane's freeze is pinned to, and the phantom failures a live
  aeon tree produces.
- **About to sweep, delete or rebuild trees under `~/sonic_hacks/`** — *STANDING ARTIFACTS THIS
  LANE DEPENDS ON*: the pinned assembler at `~/sonic_hacks/.pinned/` and the aeon reference
  tree, neither of which any other lane's worktree list can see.

Four further blocks moved from earlier in this file — the positive freeze witness, `d-18`, the
rules banked from closed findings, and the drift watch's timer — and each is named by path at
the point it used to sit.

**Ten more blocks joined them at the 2026-09-05 cut**, and they are indexed separately, under
*Read at the moment - the 2026-09-05 blocks, and the moment that triggers each*, in the
2026-09-05 section near the end of this file. The list above is the 2026-09-04 cut's and is not
the whole of `docs/OVERSEER-REFERENCE.md`.

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

**RULED HERE — the two diagnostic dialects STAY, and the split is deliberate.** `.emp` renders
`path:line:col:`; the AS surface renders `file(line):`. This is a divergence inside one binary
and it was flagged rather than committed silently. Ruled under the autonomy directive's
just-do-it clause, which names *diagnostic wording* explicitly, and logged because a reasonable
person could rule it the other way: **a compatibility surface's job is to be the thing it is
compatible with.** A user moving off AS reads the same shape in the same place, which is the
adoption argument the whole project rests on; `.emp` is a modern language and keeps the shape
modern tooling parses. **The failure mode this ruling is written to prevent is not the split —
it is an UNDOCUMENTED split**, which reads as a bug to the next person and gets "fixed" into
one dialect by someone who cannot tell which was intended.

**RULED HERE — the AS FRONTEND must fold case; whether `.emp` proper does is PARKED for the owner.**
*(This ruling said `@as_compat` until 2026-09-03. Wrong surface: `@as_compat` is an `.emp` module
attribute and the name appears nowhere in `sigil-frontend-as`. The subject is the frontend crate.)*
Same shape as `d-22` (nameless labels) and settled the same way, by evidence rather than taste:
the canonical community disassembly writes `CPU 68000`, `EQU`, `STRUCT` in capitals, so the
corpus is unassemblable without folding. **The RULING is live; its evidence figures are not
carried here** — they were a snapshot and this document has already misled one boot with them.
DERIVE the corpus counts when you need them (build a current `sigil`, run it on a pristine
corpus worktree, decompose by class). The `.emp` half is a language-surface call and is **not**
ruled here.

**⚠ THE ROOT IS WORSE THAN CASE AND MUST NOT BE FOLDED INTO IT.** `initial_cpu` defaults to
`Z80` (`Options`' default, honest for the Z80-only M0 build), `dispatch`'s directive match takes
`"cpu"` lower case only, and under `Cpu::Z80` the lexer's `b'$'` arm yields `Tok::Dollar` — the
program counter — not a hex prefix. So a 68000 disassembly assembles **as Z80**.
Case folding fixes this corpus *because s2disasm happens to carry a `CPU` line at all*. A 68000
source with no `cpu` directive still silently assembles as Z80, and **that is a separate defect
with its own fix** (the CLI's default for a general-purpose assembler is not the M0 build's
default). Booked separately so folding case does not look like it closed both.

**RULED — AS-DEFAULT-CPU is REFUSE BY NAME.** A source with no `cpu` directive is a hard error
naming what was not declared and printing the line to write; never a silent default of any
processor. Grounds are this lane's own `d-18` (now in `docs/OVERSEER-REFERENCE.md`; `R4` is the rule
number on empyrean card `4e8e865b`, not a marker in either file): a run that reports what it skipped **still
exits 0**, and a silent green is the class we never drop. Amends the hub's first form (empyrean
`802fdee`), whose acceptance named an asl oracle — episode in `docs/OVERSEER-LOG.md`,
2026-09-03. The RULING stands on the `d-18` grounds above and needs no oracle. **Pinning AS as a
dependency of the project that exists to replace it is a real cost and is the OWNER'S
alternative, not taken.**

**⚠ THE ORACLE EXISTS, AND THIS PARAGRAPH SAID IT DID NOT — corrected 2026-09-03 after four
parcels had been run without it.** A working **`asl` 1.42 Beta Bld 212** is committed in the
corpus repos. It runs. It is a **differential oracle for the entire AS-replacement project** —
the name-composition parcel used `asl -L` for every expected value in every new test, which is
why that delivery could quote listings instead of asserting semantics. **Use it.**

**⚠ BUT NOT THE COPY THIS PARAGRAPH USED TO NAME. It said
`s2disasm/build_tools/Linux-x86_64/asl`, which is the ONE BUILD HERE THAT ANSWERS
INCONSISTENTLY** *(measured 2026-09-05; this file was pointing every fresh session at it, by
path, with the words "Use it")*.

- **USE** `s1disasm/build_tools/Linux-x86_64/asl`, md5 **`61e672562465725a8c102288a7da9098`**.
  `skdisasm`'s copy is the identical binary.
- **REFUSE** `s2disasm/build_tools/…/asl`, md5 **`0dee1f98e6480a4783d27ffd8b90896f`**. For any
  operand it declined to give a value — an undefined symbol, a range-refused immediate — it
  returns a **different answer every run** with zero errors reported. The mechanism is an
  uninitialized read; it collapses to a constant under `setarch -R`.

**THE BANNER CANNOT DISCRIMINATE: both print `Macro Assembler 1.42 Beta [Bld 212]` verbatim.**
So a runner or a note that names the VERSION has not identified its instrument, and a path is
not an identity either. **Select and cite by MD5.** The shared guard is
`docs/superpowers/notes/asl-reference/asl_ref.sh`, whose own `selfcheck.sh` proves it refuses
the varying build rather than merely claiming to.

**⚠ AND THE PIN IDENTIFIES THE INSTRUMENT WITHOUT MAKING ITS ANSWER REAL — A STABLE VALUE IS
NOT AN ANSWER** *(measured 2026-09-05, reproduced firsthand at this seat on
`docs/superpowers/notes/2026-09-05-disp-or-call-probes/d9.asm`, three runs)*. For an operand it
declines to value, the **reference** build substitutes **the last value it computed**. Three
declined `#f(<register>)` immediates, each preceded by a successful call holding `$0111` /
`$0222` / `$0333`, come back `0111` / `0222` / `0333` — each echoing the line above it, **exit 0
and no diagnostic**. So its stability on this shape is a property of SOURCE ORDER, and the
uniform `0000` measured elsewhere was the initial state of that slot rather than a policy.

**This inverts which build is dangerous to freeze from.** The varying build's defect announces
itself on the second run; the reference build's agrees with itself forever and therefore reads
as a measurement — which is exactly the value that gets minted into a golden, a note's table or
a test's module doc. **Both builds are wrong on this shape and we match neither** (queue row
`ASL-SILENT-WRONG-ON-BOTH-BUILDS`). Before quoting any asl value, ask whether the shape is one
asl DECLINES; if it is, the number is an artifact under either digest, and the digest tells you
only which build produced the artifact. This does not weaken the md5 ruling above by one inch —
an unidentified instrument was always the bigger defect — but the ruling was banked with a
contrast (`303C 8000` "every run" against four random draws) whose stable half is itself a
carry-over, and **a right conclusion does not license the evidence that was offered for it.**

**⚠ AND A THIRD FACE, WHICH THE MD5 GUARD DOES NOT COVER AT ALL: AN ASL RUN CARRYING ANY ERROR IS
NOT A SOURCE OF VALUES FOR THE LINES THAT DID ASSEMBLE** *(found 2026-09-05 by the S2 decomposition
parcel, against its own probe; landed at sigil `49acd05d`)*. Its first probe file had one invalid
line (`bra.s /`, where `/` is definition only in AS). That single unrelated error **changed a value
elsewhere in the same file**: a macro expanded `beq.s +` came back `67FE`, a branch to itself,
instead of the correct `6702`. The listing looked complete.

**So the selection ritual is now two checks, not one.** The md5 says WHICH PROGRAM ran. The exit
status says WHETHER ITS ANSWERS MEAN ANYTHING. A session that pins the digest perfectly, reads a
listing that looks complete, and quotes a value out of a run that exited non zero has done
everything this document previously asked and still carried a fabricated number. Note the direction:
the corrupted value was plausible, in range, and of the right shape, which is why nothing announced
it.

**`asl_ref.sh` checks the binary and NOT the run** (verified here: its four `exit` sites are all
about selecting the digest). Until that is fixed, checking the exit status is the caller's job on
every asl invocation. Booked as `ASL-GUARD-EXIT-STATUS`.

*(Counting note, and it has now been wrong TWICE. "Four binaries, one bad" was four PATHS and
TWO PROGRAMS — and that correction was itself a count of what someone happened to check.
**Measured 2026-09-05 by running every `asl` on the machine: SEVEN paths execute here under FOUR
distinct digests**, all printing the same banner. The reference digest is reached by three paths,
and `s2disasm/build_tools/Linux-x86/asl` is an **ELF 64-bit binary in the 32-bit slot** with a
digest of its own — so selecting by architecture directory gets a program neither its path nor
its banner describes. The guard is unaffected: it pins one digest and refuses the other three.
Population and `file` output in `docs/superpowers/notes/asl-reference/README.md`.)*

**Why it survived four parcels:** `git grep asl` **in the sigil repo** returns the 68000 shift
mnemonic, so a true local finding hardened into a claim about the workspace. The rule is protocol
bar 16(d), verbatim there — an absence leaves nothing to be suspicious of. *(Episode:
`docs/OVERSEER-LOG.md`, 2026-09-04 cut.)*

**⚠ THE CASE FOLD IS IN AEON'S SHIPPING BUILD PATH — fold DIRECTIVES AND MNEMONICS ONLY, NEVER
SYMBOLS.** `sigil-frontend-as` is not a compatibility side-car: aeon's `build.sh` routes the
residual `.asm` DATA through it and three files still go that way (`engine/debug/debugger.asm`,
both `game_root.asm`). So this parcel can move aeon bytes and must prove it did not. Symbols
are deliberately case-sensitive (`lib.rs:31`, *"Names are case-sensitive"*) and `.emp` shares
its symbol namespace with those files — folding symbol case would collide the two.

**CLOSED — `landing-run.sh` runs clippy**, as precondition (7) in its header: `--release
--workspace --all-targets -- -D warnings` in its own command span, `CLIPPY_EXIT` and every lint
site in the verdict block, a red bar makes `RESULT` not-green, nothing skips it (`--scoped`
included). **A COUNT TAKEN UNDER `-D warnings` IS NOT THE POPULATION** — the first crate to fail
aborts and cargo stops scheduling the rest, so the visible count is how far the build got. This
one read 10, then 35 more, then 71 more: **116** sites in 7 files, all quoted asl listings whose
tabs are evidence. Size such work with clippy run *without* `-D warnings`, where nothing aborts.

**⚠ THE CORPUS DIAGNOSTIC COUNT IS THE PROJECT'S HEADLINE METRIC AND IT IS STRUCTURALLY BLIND
TO THE DEFECT CLASS THIS PROJECT KEEPS FINDING** *(2026-09-03, earned by the `{INTLABEL}` parcel;
this is the rule, and it should be read before any parcel is sized off a diagnostic count)*.

A complaint count measures **what the frontend refused**. It cannot see:

- **A silent wrong answer.** `zoneID macro zoneID,{INTLABEL}` bound every zone constant to a
  program counter and emitted **zero diagnostics**. The count was not merely unhelpful here — it
  was *maximally* reassuring about the worst case in the file.
- **Anything that fails at LINK.** A frontend-only corpus run never gets there. Seven
  ordinary-parameter sites (41 invocations) were broken and invisible for exactly this reason.

Both halves have now been met repeatedly: **five silent-wrong-answer faults closed in one day**,
of which the two largest were invisible to every measurement the project had. The `×26` stride
bug and `[layout.odd-field]` are the same shape one layer out, already banked in the
SIGIL-DECOUPLE section below (what the coupling buys) and in
`docs/OVERSEER-REFERENCE.md`'s source-gate lane.

**COMPARE THE SETS, NOT THE TOTALS — a summary statistic answers a question nobody asked.**
Two populations can differ by one member in each direction and total identically, so a count
difference is consistent with a change it cannot see. The assertion worth gating is therefore not
`debug = release + 1` but **`release` is CONTAINED IN `debug`** — `release \ debug` empty, with
`debug \ release` computed and REPORTED rather than asserted. A future release-only equate is a
genuine anomaly, and a count-difference check passes straight through it.

**⚠ THE MEASUREMENT THIS RULE ORIGINALLY CITED WAS WRONG IN BOTH HALVES, and the rule is stated
above WITHOUT it on purpose** *(refuted 2026-09-05 by the parcel that implemented it; verified
firsthand here on the reference tree at `483b3e12`)*. It read: *"across five aeon trees spanning
155 commits, the debug listing carries exactly one more equate than release"*, and prescribed
**strict** subset. Both fail on a SHIPPED SHAPE: **demo is 555/555, sets IDENTICAL** (sonic4 is
737/738). The cited pairs were five revisions of **sonic4 alone**, so one game's property was
written down as a property of the shapes.

**The consequence is the failure this document warns about elsewhere, aimed at itself:** a gate
built to this text — `debug == release + 1`, `debug > release`, or `release ⊂ debug` strictly —
would have been **RED ON DEMO the day it was written**, on correct code, and the remedy a
reasonable person reaches for is weakening the check. **A right conclusion does not launder the
evidence offered for it:** the set-comparison rule survives entirely, and it survives on its own
argument (two populations can differ by one member each way and total identically), never on the
figure that was attached to it. Assert containment; never strictness; and derive the shapes'
counts when you need them rather than reading any number off this page. The same shape is why
this project compares unresolved-symbol *name sets in both directions* rather than their sizes.

**So: a falling diagnostic count is evidence that noise was removed, never that correctness
improved.** The discriminators that DO see this class, and which a parcel should report beside
the count: the **per-class decomposition** (did any class rise, did a new one appear), the
**sorted unresolved-symbol sets** compared in both directions (newly-unresolved AND
newly-resolved, since a name that silently starts resolving to the wrong thing leaves the set),
byte identity against the aeon shapes, and — the one nothing else covers — **reaching link at
all**. Sizing a row off a diagnostic count alone systematically under-prices the dangerous work
and over-prices the loud work: this row was booked at 492 and delivered 5,381, and the
consequential half of it had no diagnostics at all.

### PER-PARCEL-TERM-FEED-CUT — SUSPENDED by the cut ruling; three rules survive it

*(Row suspended while EFFECTS-W1 is open. Full measurement:
`docs/superpowers/notes/2026-09-03-per-parcel-term-feed-cut.md`. Narrative and the closed red:
`docs/OVERSEER-LOG.md`, 2026-09-03 cut, original lines 1205-1256.)*

- **⚠ FILE SIZE IS NOT ASSEMBLED LENGTH, and the gap is large.** `file = assembled + appendix`.
  Taking file deltas as terms once gave a baseline 1,632 B wrong that read as nine measured numbers.
- **⚠ DO NOT RETIRE THE ASSERT TO CLEAR THE RED.** Its sibling was retired on reasoning that applies
  to this one word for word, which makes the move look pre-blessed. Retiring a check **while it is
  red, because it is red** is bar 9 with the causation hidden — the tell is that the conclusion
  requires work from nobody.
- **A "live state" bullet is a snapshot wearing the grammar of a standing fact.** A stale suite
  figure here was read at boot, believed, and written into a dispatch brief as a fact about the
  tree, telling an agent a pre-existing red existed when none did. **Derive the suite figure when
  you need it; never quote one from this document.**
### REPIN-TESTS-HINT-UNDERLISTED — a hint nothing can contradict

`repin.toml`'s per-symbol `tests = [...]` feeds one printed rerun hint and nothing else. It gates
nothing, so an incomplete list cannot fail, and a large share of rows omit at least one consuming
test binary. **The fix is not editing those rows by hand** — that is a population to maintain whose
failure mode is "green because nobody maintained it", which this file rejects twice elsewhere.
Prefer deriving the list or gating it: the consuming set is mechanically discoverable, so the honest
shape is a check that the declared list matches the derived one, or dropping the field for a derived
hint. The count is a name-anywhere grep and needs narrowing before it is quoted as fact.

*(The measurement, the mechanism, its instance and the two method findings: `docs/OVERSEER-LOG.md`,
2026-09-04 cut, original lines 1186-1202.)*
### DPLC-ENTRY-INSTRUMENT REPIN — an ask that must outlive both sessions

`parcel/dplc-entry-instrument` is parked on the aeon lane and is the CANDIDATE owner of a
`+$60`-shaped delta — flagged as candidate, explicitly **not** attribution, by the session that
flagged it. If it moves: four cross-seam symbols need the three-site treatment (`repin.toml`,
`pins.rs`, addr_labels) with **full** `tests` lists; `game_loop_port` and `load_art_port`
additionally face region byte gates, DEBUG shape only, because the parcel grows `VInt_Level`
itself — a byte-gate re-prove, not a table row; and the plain-shape falsifier runs FIRST, since it
is one command and it indicts the parcel or clears it. Written down rather than left in a thread
because both sessions that held it were rotated the same night.

*(The symbol names and their per-test mappings: `docs/OVERSEER-LOG.md`, 2026-09-04 cut, original
lines 1203-1220.)*
### PROVENANCE-REV-REACHABILITY — LANDED

`sigil_harness::rev_reachability` judges every `aeon_rev` / `strict.*_rev` against its own remote
branch with `git ls-remote` **at measurement time**, never a tracking ref. **REPORTED, not GATED —
a ruling, not an oversight:** an exception list is a population to maintain whose failure mode is
"green because nobody maintained it", and a pinned ratchet goes red during the normal ritual, so
the teeth are at the WRITE site. Chain 181's `strict.sigil_rev` is DIVERGENT and is **not** being
repaired — re-attesting would record a different tree's run under 181's name; it stands and the
report names it. For the aeon lane: **push the freeze commit BEFORE `--attest`**, since a revision
already in `origin/master` cannot be orphaned by a later rebase.

*(The four measured reachability states and their remedies, the design argument and the chain-181
instance: `docs/OVERSEER-LOG.md`, 2026-09-04 cut, original lines 1221-1239.)*
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

**⚠ An empty commit range does not mean an empty branch** — that triple is the signature of a branch ALREADY MERGED, and two lanes here read it as proof of no work. The rule is protocol bar 16(a); disambiguate with `git log <branch>`, or `--is-ancestor` on a commit you expect the branch to CONTAIN. *(Episode: `docs/OVERSEER-LOG.md`, 2026-09-04 cut.)*

Their side is banked at aeon `1ee8f8e6` (handoff) and `ba189b40` (the `br_ext` unlock
row, cuttable cold) — both verified reachable from aeon's `origin/master`.

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

## Rules banked 2026-09-05 - read at boot

The findings of 2026-09-05 that a fresh session needs before it acts. Their episodes are
not here: this section carries the rule and names where the narrative went, per the owner's
2026-09-04T15:38:47Z ruling that the boot read is split by WHEN A RULE IS READ.

### Read at the moment - the 2026-09-05 blocks, and the moment that triggers each

Nothing below was shortened to move it. Each row names the moment its block is read, because a
rule whose trigger a reader cannot recognise is a rule nobody looks up.

**In `docs/OVERSEER-REFERENCE.md`:**

- **Sizing or dispatching AS-NAMELESS-LABELS-RC1** - the six change sites, the L sizing, the
  arithmetic risk, the landing condition and the no-prediction prohibition.
- **Touching a committed generated artifact** - *the three golden-vector headers are NOT
  hand-edited*: why a hand-freshened generator header is indistinguishable from an adjusted
  measurement, and `GOLDEN-HEADER-UNGATED`.
- **Building a count, or writing a brief** - *THREE THINGS THIS PARCEL TAUGHT*: the lexer with
  no char-literal state, the control that made three wrong counts detectable, and why a brief
  that states a constraint in the abstract when exactly one value works has stated a preference
  and called it a rule.
- **Proving a mutation applied** - *`git checkout <rev> -- <path>` STAGES*, so plain
  `git diff --stat` reports nothing on an applied mutation.
- **Naming a path in a brief** - the `.target-land` instruction that caused a shared-state
  mutation because it named the value and dropped the tree.
- **Designing a probe for anything on an error path** - *error-path behaviour is CONDITIONAL ON
  WHAT ELSE FAILED*, so both directions must be run.
- **Making something newly refuse** - ask what the OLD code did on the same path; a panic or an
  `unreachable!` cannot regress a working build, and that quantifies where a green run samples.
- **Judging a control, or a report's own severity** - *A CONTROL CAN BE CONFOUNDED BY THE VERY
  THING IT CONTROLS FOR* (MOMPASS): a pre-existing claim needs its own instrument, fidelity is a
  cost rather than a goal, and a reporting change should be strictly additive.
- **Running any mechanical text change** - *a sweep kills any negative assertion keyed to what
  it changed*, and the grep that finds them runs against the PRE-sweep revision.
- **Pruning worktrees** - the lock is not a liveness signal, the ancestor test is wrong in both
  directions, and `du -sh` comes before any count.

**Two live bookings are named only in that narrative, so they are named here too**, because the
queue board `docs/lane-status.json` is untracked and a rotated session cannot be shown to hold
them: **`PINS-GATE-MESSAGE-MISLEADS`** (make the count and the verdict describe the same
comparison; the printed command was fixed at `81d92f80`) and **`AS-IF-REFUSAL-DIAG-VECTOR`**,
which is CLOSED at `34dad07c` and is listed so nobody re-opens it from the narrative alone.

**In `docs/OVERSEER-LOG.md`, 2026-09-05 cut:** the dash ruling's scope gap and its measured
counts; the ratification of the sweep's extension to shell and Python; the 518-block bisection
that this seat got wrong twice and an agent refuted; the register fault and its silent
acceptance; how the `if` refusal was cleared without an aeon build; master's strict red and the
enumeration error under it; the pins gate's two true halves; and the four landing-gate
episodes whose rules are kept below.

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

### AS-NAMELESS-LABELS-RC1: the sizing, asked for by the owner before dispatch

**Size: L.** The single highest-value row in SIGIL-AS-REPLACEMENT, measured rather than estimated:
one unimplemented feature is the sole root cause of **4,985 diagnostic rows across four message
classes, 86.5 percent of the entire Sonic 2 run** (landed measurement, `49acd05d`, note at
`docs/superpowers/notes/2026-09-05-s2-top-blocks-decompose.md`). Ruled **ACCEPT** by `d-22` on
2026-09-03 for the AS surface only; zero of 5,003 constructs are accepted today, so this is not
started rather than half done.

**The rest of this row is read when the parcel is DISPATCHED, not at boot**, so it is in
`docs/OVERSEER-REFERENCE.md` under this row's own name, verbatim: the six things that have to
change and where each is refused today, why the row is L and not XL, the arithmetic-regression
risk that decides the gate, the landing condition with its zero-population premise measured at
the consuming end, and the prohibition on quoting `5,761 - 4,985 = 776` as a post-fix
prediction.

### SHELL `grep -r` IS A FUNCTION HERE AND SKIPS GITIGNORED FILES, RETURNING A CLEAN ZERO

*(Reported by the aeon lane at aeon `56e42f00`, reproduced by aurora with a canary, and REPRODUCED
HERE before being banked. It had already put a false "symbol appears nowhere" into one lane's brief.)*

`type grep` on this machine returns **"grep is a shell function"**, sourced from the harness's shell
snapshot. That function's `-r` **skips gitignored paths and reports success with no output and no
error**. Everything this lane generates is gitignored: listings, ROMs, build directories, generated
corpus files, probe output.
*(The reproduction, including the first attempt that was VOID because the canary went into a
path that was not ignored, and this lane's audit of its own 2026-09-05 zeroes:
`docs/OVERSEER-LOG.md`, 2026-09-05 cut.)*

**Pick the instrument by what the subject IS:**

- **tracked source** (corpus `.asm`/`.inc`, our `.rs`, docs): `git grep`, which is also faster and
  respects the repo boundary;
- **ignored artifacts** (a listing, an output binary, anything under a build dir): `/usr/bin/grep -r`
  by absolute path, never bare `grep -r`;
- **either, when a zero would be a finding**: plant a canary of the same class, confirm it is ignored,
  and confirm the instrument finds it *before* believing the zero.

**And it is the FOURTH instrument that returned a confident wrong zero to this lane in one day**, all
different mechanisms: a `git grep` pathspec that matched no files; a hand-rolled lexer desynchronised
by a char literal; a peer's `\|` alternation inside `$'...'` under zsh; and now this. **The rule is
not "watch out for greps", it is that AN EMPTINESS IS NEVER A FINDING WITHOUT AN INSTRUMENT THAT
COULD HAVE RETURNED NON-EMPTY.** Same family as `cmd | sed ... || echo`, where the `||` binds to the
whole pipeline.

### A PARTIAL RUN IS NOT A LANDING GATE (2026-09-05)

*(The strict red this came out of is CLOSED. Four of its five failures were this lane's own,
the survivor was the pins gate, and the reasoning error that produced them is the transferable
part: `docs/OVERSEER-LOG.md`, 2026-09-05 cut.)*

**What this costs, stated rather than softened: a partial run is not a landing gate, and I treated it
as one six times today.** Every one of those parcels reported honestly that the byte gates had not
executed. The reporting was correct and the LANDING DECISION still went ahead on it. The rule this
lane needs is not better disclosure, which was already perfect, but that **a parcel touching the AS
frontend gets a strict run before it lands, or it lands knowing the gate is owed.**

### I RAN A SUBSET OF THE LANDING GATE ALL DAY AND IT COST ME TWICE (2026-09-05)

*(The two things that escaped through that gap, and the clippy attribution by `git log -L`:
`docs/OVERSEER-LOG.md`, 2026-09-05 cut.)*

**Standing rule for this lane, in force: run `scripts/landing-run.sh`, not a hand-assembled subset.**
If it cannot run (machine load killed two monolithic suites tonight), segment it, give each segment
its own end marker, and say in the landing which preconditions did not execute. Naming the gap is the
minimum; running the gate is the job.

*(Both measurements, and why the quoted `asl` tabs make a scoped `allow` the right remedy
rather than a reflow: `docs/OVERSEER-LOG.md`, 2026-09-05 cut.)*

**The lesson did not transfer because I applied it to the PROCEDURE and not to the COMMANDS INSIDE
IT.** Reading a gate's precondition and then re-typing it from memory is the same omission the wrapper
exists to prevent, one layer in. **Copy the invocation out of the script; never retype a gate's
command.**

### PRINT `pwd` AND `HEAD` BESIDE ANY VERDICT (2026-09-05)

*(The compound command that left the shell in `s2disasm` and produced a clippy exit 101 from
a tree with no `Cargo.toml`: `docs/OVERSEER-LOG.md`, 2026-09-05 cut.)*

**The rule: a verdict from a compound command inherits wherever the previous part left the shell, and
an exit code alone cannot tell a tool's failure from the tool never running.** Print `pwd` and `HEAD`
beside any verdict you intend to act on. This lane already stamps its suite logs that way; the same
discipline was missing from one-line checks.

### AN A/B WHOSE ARMS AGREE MAY HAVE MEASURED NOTHING: state what each arm PRODUCED

*(Aeon's finding against its own test, widened by them from a bar this seat had stated too narrowly.)*

Their first before/after run had **both arms fail on a missing positional argument**. The failure
output was identical, so **the pair read as "identical, therefore fine"**. Caught only by looking at
the exit codes.

**This seat then banked the narrow form, "check exit codes on both arms". Aeon widened it and the
wider version is the right one: THE EXIT CODE IS THE INSTANCE, NOT THE SHAPE.** The real requirement
is that **at least one arm must have PRODUCED something**, not merely that the two agree. That covers
every shared upstream failure an exit code may not even report: a wrong path, an absent input, a
stale cache, a subprocess that never ran, a tool that resolved its subject from its own location
rather than from the argument.

**Two arms agreeing perfectly while having measured nothing are the same family as a control that
bypasses its subject and a test that cannot fail** (the three vacuous-proof shapes already banked
here). The discipline is one line longer than the habit: report a positive artifact from the arms, a
row count, an extracted symbol and its value, a byte length, **beside** the agreement. Their corrected
run does exactly that: `s4budget` exit 0 both arms and byte-identical output, **plus** 1,654 symbol
rows either way and two named symbols extracted with their values.

**And the same lane bounded its own result before being asked**, which is the other half of the
practice: their unphased-listing test used a build with the sound driver off, whose listing still
carries 34 Z80 mentions, so they reported it as *a listing gaining a count-0 trailer where phased
content is at most minimal*, **not** as a listing with provably zero phased symbols. **A second
partial result offered as a discharge is exactly what the first correction had just caught**, and
they refused to let it stand as one.

### A DO-NOT-TOUCH RULE WITH NO NAMED OWNER FOR THE DELIBERATE TOUCH ROTS THE THING IT PROTECTS

*(Aeon's framing, adopted in their words because it is better than mine: **the protection and the
maintenance were the same action, so forbidding one forbade the other.**)*

The shared assembler every lane links against is `sigil/target/release/sigil`, this checkout's
DEFAULT target dir. This document's standing rule is that **no cargo command may land there**, because
a relink silently replaces the binary another lane's freeze pins by md5. That rule is right and was
obeyed all session, every build routed to `.target-land`.

**So it sat at `756c7efd` while master moved 20-odd commits past it**, and a peer lane's builds were
warning about the mismatch. **It went stale precisely BECAUSE the rule was working.** Nothing in the
rule says who performs the deliberate refresh or when, so the answer was nobody.

**Refreshed 2026-09-05 from master `d094c3c8`: md5 `58db3594...` to `945387f2...`, `emit_sound_blob`
rebuilt from the same tree.** Checked before overwriting that nothing live pinned the old digest (the
two hits were dated historical records, which stay true) and that the standing pinned artifact at
`~/sonic_hacks/.pinned/` is a separate file, untouched.

**THE GENERAL BAR, and aeon is right that it is not sigil-specific: read every "nobody may write
here" convention in this suite for whether it also forbids the UPDATE, and where it does, name who
performs it.** A protected artifact with no maintainer is a protected artifact rotting on schedule.

**And prove the refresh CHANGED BEHAVIOUR, not just the version string.** A version bump is not
evidence the new code is in there. Built a shape with the refreshed binary and read the new section
out of the listing. **My first check was a false positive**: a case-insensitive grep for `VMA|LMA`
matched `LocaLMAp` inside unrelated symbol names, and would have read as confirmation.

### AND "NOT WORTH CHASING" WAS WORTH CHASING: 29 AND 30 RECONCILED EXACTLY

Two lanes measured the same tool's error rate and got 29 and 30. The peer proposed it was "probably
the demo case counted differently" and not worth pursuing. **It took three minutes and reconciled
exactly**: the tool returns 36 names, 6 appear in sonic4's phase table and 1 in demo's, so 36-6=30
are absent from sonic4 (theirs) and 36-6-1=29 are absent from BOTH (mine). **Both numbers correct,
different populations, no disagreement.** The single name between them is `Z80_IdleProgram`.

**A plausible reconciliation offered in place of an actual one is a guess wearing an explanation's
clothes**, and the cost of settling it was three commands. The same run confirmed the second
direction the peer had rightly refused to accept on assertion:
`$engine.z80_init$Z80_IdleProgram$code_end` is in demo's phase table and is NOT returned by the tool.

### A DERIVATION THAT CAPTURES PARENTS AND DROPS LOCALS MISSES EXACTLY THE BOUNDARY MARKERS

*(Aeon's mechanism, found by checking their own tool rather than my listing, and it is worth more
than the counts either of us produced.)*

`vma_phased_symbol_names()` parses `section ... (vma: ...)` blocks and takes the **top-level** names.
The one symbol it misses is a **local inside** such a proc, mangling to `$module$Proc$local`. So the
misses are not scattered: **they are precisely the local labels inside phased procs.**

**And that is the worst class to miss, by the consumer's own logic.** The tool exists so a
boundary-inferring consumer never lets a phased symbol stand as an extent boundary. The single symbol
it misses is an **end-of-code marker**, a symbol whose entire purpose is to be a boundary, with three
`imm16` references to it. **So the 30 over-reports are harmless phantoms and the one miss is the exact
shape the tool was written to catch.**

**The transferable rule: when a derivation walks a structure and takes the named things at one level,
ask what lives at the level below, because in a symbol table that is where the boundary markers
are.** A count of how often such a derivation is wrong says nothing about this; only the mechanism
does. Both of us had measured the error rate and neither of us had it until the mechanism was found.

### THE FOUR-CORPUS SWEEP COMPARES ALMOST NO BYTES, AND THIS SEAT PRESCRIBED IT THREE TIMES

**Measured 2026-09-06 and verified independently here: 0 of 40 `s2disasm` `.asm` files emit ANY bytes
assembled standalone.** The corpus trees are include FRAGMENTS, not roots. Across all four trees both
arms accepted **2 of 1,753** files and emitted **zero** bytes.

**So the sweep's `DIFFER=0` is a statement about 43,862 DIAGNOSTIC lines and exit codes, and is
VACUOUS about bytes.** It has been reported in this lane's own words as comparing "emitted bytes,
every diagnostic, and the exit code", which is true and misleading: the first term is empty.

**This narrows a banked result rather than overturning it.** The strict-arguments null result rests on
the diagnostic half, which is real; its byte wording was not. **A conclusion can survive while the
sentence that carried it does not**, and the correction is owed to the sentence.

**What caught it was the bar added hours earlier from a peer's void test: state what each arm
PRODUCED, not merely that they agreed.** The bar caught an instrument THIS SEAT had prescribed three
times, which is the argument for stating bars as properties of evidence rather than as warnings about
a particular tool.

**`sweep.sh` now prints the byte census every run and refuses to let a zero-byte run read as byte
identity.** The real byte check is separate and was run: both aeon ROM shapes on both binaries,
identical, **1,659,455 bytes of actual emission**, written to scratch so the shared reference tree
was untouched.

**The general form: an instrument that compares X over a population where X is empty reports perfect
agreement forever.** Before believing any comparison, ask what the population actually contains of
the thing being compared. A file count is not a byte count.

### AND A ROW'S TITLE CAN BE THE NARROWEST TRUE STATEMENT OF ITS OWN DEFECT

`AS-SET-OPENS-SCOPE` named `set`. **Twelve spellings diverged** (`set`, `equ`, `=`, `:=`, `eval`, each
with or without the decorative colon, the comma-operand forms, string-valued binders) plus `enum`,
where the last member owns the scope: **24 of 28 matrix rows**. The two already-correct rows were the
controls that made the matrix worth building.

**And the row's cited five sites CANNOT diverge at all**: they are dotted binders, and a dotted binder
opens no scope in either assembler. I had measured that one of them did not diverge and framed it as
"at least one does not". **The true statement was stronger and simpler than my hedge** — a hedge
around a measurement is not automatically the safe direction, it can also be the imprecise one.

### CITE THE ARTEFACT THAT CAN BREAK, NOT THE SESSION THAT ASKED

*(Oracle's correction to a comment I wrote naming them. Small, and it generalises past attribution.)*

I recorded a cross-lane contract in source and credited the requesting session by its handle. **A
session handle stops existing.** In six weeks it names nothing, and the comment reads as a note from
a ghost, **which is worse than no attribution, because the next editor cannot tell whether the
dependency is still real** and will therefore either preserve a dead constraint or delete a live one.

**Name the REPO and the PARSER**: the thing that will still be there to break. Now cited as `oracle`'s
`SymbolTable::parse` at `crates/oracle-core/src/symbols.rs:553`, **verified present when written**,
since a path cited without checking is the defect this lane closes weekly.

**It is the promise-in-a-message problem one level down.** A guarantee given in chat has no owner and
rots invisibly; a guarantee given in a comment naming a session has an owner who evaporates. Only the
artefact persists.

### AND THE PIN FAILED IN THE EXACT WAY THE PIN EXISTED TO PREVENT

Worth recording as the sharpest instance of the night's theme. The contract's first pin asserted
**"every line in this section begins with `PHASE` at column 0"**. The count line was then renamed
`PHASE-COUNT`, and **`PHASE` is a prefix of `PHASE-COUNT`**, so the assertion would have gone on
passing while the consumer's two distinct keys silently collapsed into one. **A guard that fails the
way it exists to prevent, one level above where it was written.**

Caught by the consumer, not by me. **The corrective is in the test now**: the two shapes are asserted
SEPARATELY and required disjoint, and the trailing space in `^PHASE ` is documented as load-bearing
with the reason, because it is the trap a future consumer is most likely to repeat.

**And the detail worth copying into any guard: it REFUSES TO RUN over an empty row set.** A guard that
is green because it examined nothing survives every other precaution, and it is the one thing a
red-first proof cannot catch, since an unapplied mutation and an empty corpus both print `ok`.

### ATTRIBUTION, CORRECTED TWICE IN ONE EXCHANGE

*"Prefer a check that must CLOSE over a check that must AGREE"* is **not** the phrase's relayer's, and
not mine. It is **the dash-sweep agent's**, formed after it found its own cross-validation worthless:
two lexers in two languages, written hours apart and deliberately cross-checked, agreeing on 480.
**One author wrote both, so the agreement measured nothing.** What caught the error was a residual
that had to reach zero and did not.

**Three independent arrivals at that idea in one night**, the third being this lane's own sweep
agreeing 1,753 times over zero bytes. **Nearly every defect found today was a check that AGREED;
nearly every recovery was a check that had to CLOSE** — an accounted population, a control that had
to fire, a leg count that had to reconcile.

### ⚠ THE OWNERSHIP BAR I WROTE TODAY BROKE A PEER'S RUN THE FIRST TIME I EXERCISED IT

**2026-09-06 05:45:48Z I renamed over `sigil/target/release/sigil` while aeon's SP-5 agent was
mid-build with the previous binary's md5 pinned in its brief.** Four legs assembled by two different
assemblers, with **nothing in the output that would say so**: a false byte-identity, which is the
exact failure the pin existed to prevent.

**Hours earlier I banked: a shared artifact protected by a do-not-touch rule needs a NAMED OWNER for
the deliberate touch, or it rots.** I became that owner. **The first exercise of the ownership caused
a collision, because the bar named WHO MAY TOUCH and said nothing about WHEN.**

**A rule that assigns ownership without a coordination point does not remove the hazard; it relocates
it, from rot to collision.** That is the amendment, and it is the more important half:

- **The owner of a deliberate touch announces BEFORE, not after.** I announced after, as a completed
  refresh, which is a report rather than a coordination.
- **A refresh of a shared artifact is a SCHEDULED act, not a local one.** The artifact being shared
  makes the TIMING shared too. Content being correct settles nothing about when it may land.
- **Ask who is mid-run before touching**, not whether the content is right. I checked staleness,
  provenance, that nothing pinned the old digest in tracked files, and that the standing pinned copy
  was untouched. **Every check I ran was about the artifact. None was about the other lanes.**

**And the recovery is worse than the incident.** The previous binary is **unrecoverable**: no copy in
scratch, none in `~/sonic_hacks/.pinned/`. A rebuild of the same revision is a DIFFERENT artifact
answering a slightly different question, per this document's own rule, so it cannot be handed back as
the pinned one. **This repo already knows a rename-over destroys evidence and not merely state**, in
the words of its own freeze rule: `chmod` on a file does not stop a rename, because a rename is
governed by the DIRECTORY. I had that written down and applied none of it, because I was thinking
about whether the content was current rather than about who else was standing on it.

**Cheap prophylactic for next time, since the rule needs a mechanism and not vigilance: copy the
outgoing binary aside BEFORE the rename.** One `cp`, and the pin stays recoverable whatever else goes
wrong.

**⚠ AND MY OFFER TO PARTITION THE DAMAGE BY TIMESTAMP WAS WRONG, on a mechanism I verified here
rather than accepting.** I offered aeon that only legs straddling the swap instant were unattributable
and that timestamps could identify them. Aeon declined for the right reason: **a rename-over does not
disturb a process that has already opened the old inode.** Demonstrated at this seat with a running
script renamed over mid-execution: **the running process completed on the OLD contents while the path
already served the NEW ones.**

So a leg running across the instant is not half-and-half; it completes on the old binary. And a
"leg" is not one execution but many invocations over its life, so **which binary each invocation got
is not recoverable from the leg's start time.** Partitioning would have needed per-invocation timing
that nobody records, and my offer would have had aeon keep results it could not actually attribute.

**The general form, and it is the same family as the freeze rule this repo already carries: a rename
changes what a PATH resolves to, never what an already-open process is running.** That is what makes
a mid-run swap invisible from both ends: nothing fails, nothing warns, and the output of a leg that
straddled it looks exactly like the output of one that did not.

**RULED BY THE HUB 2026-09-06, and it supersedes my amendment because it names WHO COORDINATES rather
than merely when: the shared binary is refreshed ONLY INSIDE A WINDOW THE HUB OPENS, after asking
every lane that builds against that path, and the HUB announces the swap instant and the new identity
afterwards.** My version said "the owner announces before", which still leaves the owner deciding the
moment from inside one lane's view. **A lane cannot see who is mid-run; only the hub can.** That is
the whole reason the coordination point has to sit above the owner.

**Resolution of the incident: `e6e942e5` STAYS.** No swap back and no rebuild of `d094c3c8` was
ordered, on the grounds that a reproduced pin would answer a question nobody needs any more, since
aeon restarts every SP-5 leg under the new identity. **The path is frozen to this lane until the hub's
word.** The refreshed binary's acceptance rests on the proof, not the version string: one
`^PHASE-COUNT` line, six phase rows, and **zero lines matching the old spelling, checked as a
control** rather than only confirming the new one appeared.

### VARYING A FLAG IS NOT VARYING A ROUTE: I compared one path with itself and called it two

**The `AS-ASSIGN-UNRESOLVED` row said the divergence was between the LINKER path and a SINGLE-FILE
route. I reproduced it, ran the CLI with and without `--hex`, got identical behaviour, and wrote in
the dispatch brief that "both routes behave identically" and the row's framing was wrong.**

**The row was right and I was wrong.** Verified at `e6e942e5`: `main.rs:124` goes
`assemble_root_located_warned` then `sigil_link::link()` with **no `resolve_layout`**, while every
other seam (`:436`, `:789`, `:902`) runs `resolve_layout` then `link`. `resolve_layout`'s
`fold_equ_syms` already refused an unfoldable `equ` **without asking whether anything reads it**, and
the `.asm` CLI was the one final link that stepped over it.

**Only one CLI route takes a `.asm` file, so both my observations were of the same path.** `--hex`
changes the OUTPUT FORMAT, not the seam. **I compared a route with itself and reported agreement**,
which is the same family as a sweep that runs one binary twice, or a cross-validation whose two
implementations share an author.

**The rule: before claiming two paths agree, name what makes them DIFFERENT PATHS, and check that the
thing you varied is that.** A flag, an output format, a verbosity level and a file extension are all
things that feel like route selectors and usually are not. The cheap check is the one I skipped:
follow the call chain far enough to see where the two supposedly diverge.

**And the fix I proposed in that brief could not be taken at all.** "Evaluate the right-hand side
where it is written" would refuse the cross-seam equates the mixed AS plus `.emp` build depends on,
because the front end cannot distinguish a never-defined name from a `.emp` label the link is about to
supply. **A fix specified from the outside can be impossible for a reason the specifier cannot see**,
which is the argument for stating the DEFECT precisely and leaving the remedy to whoever can read the
seam.

### A CANARY PROVES THE PATTERN CAN FIRE, NOT THAT THE INPUT ARRIVED

**The sharpest refinement of the night, and it is a correction to a discipline this document already
teaches.** This lane has been demanding a canary before believing any zero. An agent then ran one
correctly and STILL shipped a false clean: its dash scanner used `\+` in an ERE, so it scanned **1
line instead of 1,365** and printed "clean". **The canary passed, because the canary proved the
PATTERN could match. It said nothing about whether the 1,365 lines ever reached the matcher.**

**So a canary covers the RULE and not the FEED.** Both need a control, and they are different
controls: plant a positive to prove the pattern fires, and **assert the input count** to prove the
data arrived. A zero is only meaningful when both are known. Every false-clean this lane hit tonight
splits cleanly by that test: the bad pathspec, the shell `grep -r`, the zsh alternation, the empty
corpus, the truncated scan. **Four were feed failures and only one was a rule failure**, and the
canary discipline as written only covered the rule.

### AND A REDUCTION CAN SUPPLY THE VERY THING WHOSE ABSENCE IS THE FAULT

I reduced a corpus failure to a small probe, both assemblers agreed, and I reported to the agent that
**my reduction does not reproduce it** and the isolation was still owed. Correct as far as it went and
wrong in a way I could not see: the real fault was a **missing build-generated include**, and my
reduction had supplied a value for the symbol whose absence IS the fault. **The probe tested a
program in which the defect cannot occur.**

**That is a distinct confound from the ones already banked here.** The others were probes that could
not distinguish two answers; this one removed the cause while preserving the shape. **When a
reduction fails to reproduce, ask what the original had that the reduction supplies**, not only what
the reduction lacks.

**It was the fourth confounded probe from this seat in one night**, which is the real headline: an
off-by-one index, a value too large for its destination, one route compared with itself, and now a
reduction that filled in the missing input. **All four looked like measurements and three of them
reached the dispatch brief.**

### THE RIGHT BASE COMPUTED THE WRONG WAY IS ONE BYTE AT ONE SITE

I warned the parcel that `asl`'s `log` is base 10, so a natural-log implementation answers 4 where the
reference answers 2. True, and **loud**: wrong by a whole digit, visible in any test.

**The quiet one is finer.** `ln(1000)/ln(10)` is `2.9999999999999996` in binary64, and `int()`
**floors**, so the right base computed the obvious way emits `7C02` where asl emits `7C03`. **A wrong
base is loud; a wrong SPELLING of the right base is one byte at one of six sites.** `asl`'s is an
exact `log10`. **When matching a reference's arithmetic, the function is the easy half and the
formulation is where the byte moves.**
