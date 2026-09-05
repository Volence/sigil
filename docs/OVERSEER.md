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

**The R7 alignment flip.** *(Its measurement — 81–91% of declared sections over-aligned,
the dominant case declaring 2 and handed 16 — and the ratchet hazard under it are in
`docs/OVERSEER-LOG.md`, 2026-08-30, original lines 1342–1391. The byte effect of the flip
is UNMEASURED and can only be measured by running it; report the counts, never a byte
figure.)*

**STATE OF THE PARCEL: writable, and the precondition really did land.** `DECLARED` carries **107
rows**, `required_for` is live, and `section_alignment_declared.rs` already gates it **with a
red-first witness** that doctors `Sfx_33`'s frozen row by +4 and requires the build to refuse by
name. What remains is the flip itself — `align_up(running, required_for(section))` replacing
`align_up(running, packed_align_of(prov))`, deleting `packed_align_of` and the provisional bases it
reads. **Its own paired freeze, never riding another parcel's range** (the chain-179 lesson).

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

Six blocks that sat here are read at a specific moment rather than at boot, so they live in
`docs/OVERSEER-REFERENCE.md` and are read when that moment arrives. **Nothing was shortened to
move it.** Each entry below states the moment that triggers its block, because a rule whose
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

### NO EM OR EN DASHES IN TOOL TEXT (owner ruling, 2026-09-05): booked, and the count is measured

**Verified firsthand at empyrean `24cdd17`, reachable from their `origin/main`**, a 9-line addition to
`design/CHROME_SPEC.md` under "Text in the tools". The owner, verbatim in the spec: *"Can we add no
emdashess to the design list, like no emdashes in an of our tools."* Relayed to this lane with his
broader words: *"get rid of all current emdashes and update so no more emdashes to all the tool
agents"*. No U+2014 and no U+2013 in any text a tool shows a person: diagnostics, warnings, panic
messages, generated help. Use a comma, a colon, a period, or parentheses.

**SCOPE IS SETTLED IN THE SPEC ITSELF, so do not re-litigate it.** At `24cdd17` the spec named only
Oracle, Aurora and Seraph, and the extension to this lane arrived through the relay carrying his
broader sentence. That gap was flagged to the hub and CLOSED at empyrean `f9fbfc9` (verified here
reachable from their `origin/main`, and the line read back): the rule now names every tool in the
suite, with **Sigil (the assembler's diagnostics, warnings and help)** explicit. Recorded because
the fix is the interesting half: the ruling bound this lane either way, but a cold session reading
a spec that does not name its own tool reads scope as a limit, which is the failure where a ruling
reaches only the sessions alive when it was sent.

**Half (b) is in force NOW, for everything anyone writes here**: strings, docs, lane logs, commit
messages, peer messages. This section is written under it.

**Half (a), the sweep, is booked as `TOOLTEXT-DASH-SWEEP` and takes its turn AFTER the S2
decomposition.** Sequenced by what a person reads first.

**The count, measured here 2026-09-05 over tracked files at master `297dcd8f`, with a lexer that
tracks Rust string, raw-string, line-comment and block-comment state rather than a line grep:**

| population | occurrences | note |
|---|---|---|
| `crates` src STRING literals | **595** across 64 files | the producer side, the actual subject |
| `crates/*/tests` STRING literals | **556** | the CONSUMING end, see below |
| `crates` comments | 10,840 | NOT in scope by the ruling's own words; touching them is churn |
| `docs` | 28,938 | out of scope for the sweep; half (b) governs new writing |

**⚠ THE SWEEP IS NOT 595 EDITS. It is 595 producer edits that must move in lockstep with 556
consumer assertions**, and this lane's own bar says a wording change is exactly how a matcher starts
passing for the wrong reason. Both directions need enumerating before anything is edited: a test
that asserts a full string goes RED (loud, fine), a test that matches a substring on the untouched
half goes GREEN while no longer testing what it names (silent, the whole problem). And
`docs/OVERSEER.md` already records that several `(align: N)` diagnostic strings are pinned by
**nothing at all**, so part of the producer side has no consumer to red at all. Enumerate the
consuming end, per the standing rule; never size this off the producer count.

**Correction to my own first figure, kept rather than silently patched:** this table said **582** for two
hours. That count took only files under a `/src/` path and dropped `build.rs` and `src/bin/`. The
figure is **595**. Same defect family as the zero below, one layer milder: a pathspec that answers a
narrower question than the one asked, with nothing in the output saying so.

**⚠ AND THE FIRST MEASUREMENT OF THIS RETURNED ZERO.** `git grep -- 'crates/*/src'` matched no paths
and reported 0 occurrences in 0 files, which reads exactly like a clean tool. It was caught only by
running a positive control (the same pattern found 28,938 in `docs`, so the pattern worked and the
PATHSPEC did not). Same family as the zsh word-split instance already banked: an empty result read
as a pass. Any re-measure of this row carries a positive control.

### AS-NAMELESS-LABELS-RC1: the sizing, asked for by the owner before dispatch

**Size: L.** The single highest-value row in SIGIL-AS-REPLACEMENT, measured rather than estimated:
one unimplemented feature is the sole root cause of **4,985 diagnostic rows across four message
classes, 86.5 percent of the entire Sonic 2 run** (landed measurement, `49acd05d`, note at
`docs/superpowers/notes/2026-09-05-s2-top-blocks-decompose.md`). Ruled **ACCEPT** by `d-22` on
2026-09-03 for the AS surface only; zero of 5,003 constructs are accepted today, so this is not
started rather than half done.

**What has to change, all in `crates/sigil-frontend-as/`:**

1. **Definition side.** A run of `+`, `-` or `/` in column 1 becomes a label. Today it is refused at
   `eval.rs:2697`, "expected mnemonic, directive, or label". 2,309 sites.
2. **Reference side.** A bare `+`, `++`, `-` as a branch operand. Today refused at
   `operands.rs:345`, "bad operand expression". About 2,600 sites. **This is the hard part and the
   reason the row is L**: `+` and `-` are also arithmetic operators, so this is context sensitive
   disambiguation inside the operand parser, not a token added to a table.
3. **The parenthesized forms `(+)` and `(++)` occur**, as `offsetTableEntry` arguments, so it cannot
   be a bare-token special case. The measurement note records that a regex over bare tokens misses
   them and they were nearly lost from the count.
4. **Ordinal resolution.** `+` is the next following definition, `++` the second, `-` the nearest
   preceding. Position ordered, and it has to stay stable across the existing multi-pass convergence
   loop, where a forward reference is resolved on a later pass.
5. **Macro interaction, already measured rather than anticipated.** References arrive through macro
   ARGUMENTS (14 calls at 2 sites, plus 49 through `offsetTableEntry`) and inside macro BODIES (11
   calls at 1 site). Scoping is per expansion instance, the same shape the plain-label half already
   has.
6. **The reference population is LARGER than the diagnostics show.** 18 references emit no row at all
   today because the definition on their line already failed. They become live the moment
   definitions parse, so a count taken from the diagnostic stream understates the work.

**Why L and not XL:** the machinery this needs mostly exists. `sym_key`, `plain_label_scope` and
`expansion_labels` (`eval.rs:1160-1210`) already implement "reader and writer agree about where a
name lives", per expansion instance, with the walk outward and the stop at the live stack both
measured against asl. The ordinal half is new; the scoping half has a working model to follow.

**THE RISK THAT DECIDES THE GATE, and it is not the feature.** Making `+` and `-` label-capable in
operand position can silently change how existing ARITHMETIC parses. That is a change to code that
is correct today, in a crate that sits in aeon's shipping build path, and it would not announce
itself. Every currently-correct expression must be proven unchanged, by artifact and not by argument.

**Landing condition (hub's, and its premise is now ASSERTED here rather than assumed):** the four
aeon shapes stay CRC32 and size identical. **Population in aeon is ZERO, measured at the CONSUMING
end** and not merely in a file listing: aeon tracks exactly three `.asm` files, which are exactly the
three `build.sh` routes through this frontend, and their include closure is self-contained (both
`game_root.asm` include only `engine/debug/debugger.asm`, which includes nothing). Zero definitions,
zero references. So the CRC condition should hold trivially, **which is precisely why it is worth
gating**: it costs nothing when the feature is clean and it is the tripwire for the arithmetic
regression above.

**⚠ PROHIBITION, carried into this row from the measurement: `5,761 - 4,985 = 776` is NOT a
post-fix prediction and must not be quoted as one.** Closing this resolves symbols that are
currently unresolved and lets the assembler reach code it currently abandons, which can remove rows
and add them. The only way to know the number after is to measure after.

### RULED: the three golden-vector headers are NOT hand-edited, and "wait for a regeneration" is not the ruling either

The dash sweep flagged one item for a ruling: three committed golden-vector files
(`crates/sigil-isa/tests/{z80,m68k}_golden_vectors.txt`,
`crates/sigil-frontend-as/tests/snippets_golden.txt`) carry a stale copy of the generated
`PREAMBLE` header, dashes included.

**Ruled: do not hand-edit them.** The agent's reasoning is this lane's own standing rule arriving
from the outside, and it is right: a regenerated header that no generator produced is a hand-written
artefact wearing a generator's name, and it would sit directly above rows that are real digest
measurements. At read time an edit that freshened the header is indistinguishable from an edit that
adjusted a measurement. That is the maintenance act being the vulnerability.

**But "they lose the dashes next time the generators run" is NOT an acceptable resting place, and
the agent named the reason itself: nothing schedules those generators.** An unbounded wait with no
owner is a state, not a plan, and it reads as closed while nothing is closing it.

**The defect underneath is bigger than the dashes and is the real booking. `PREAMBLE` has exactly
one reader, and NOTHING compares the committed header against it.** So the committed artefacts can
drift from their own generator silently, forever, and the dash ruling is merely what made one
instance visible. Booked as `GOLDEN-HEADER-UNGATED`: close it by REGENERATING (never by hand), and
add the missing comparison so the next drift is loud. Regeneration invokes `asl`, so it is subject
to the exit-status rule banked above, and it is its own small parcel with its own verification
because it rewrites committed test vectors.

### RATIFIED: the sweep's scope extension to shell and Python

The brief drew scope at "string literals in Rust source". A Rust test asserting on **shell** output
went red and proved the line had a hole wherever a tool in one language is tested from another. The
agent extended to 121 shell and Python edits and closed the CLASS rather than the instance.
**Explicitly ratified, not merely tolerated.** The residual it leaves is stated in its note and is
real: the regression gate covers Rust string literals only, so a shell or Python tool can still grow
a dash without reddening anything.

### THREE THINGS THIS PARCEL TAUGHT THAT OUTLIVE IT

**1. MY DASH COUNTS WERE WRONG THREE TIMES, and the third time had a mechanism worth keeping.** I
reported 582, then 595. Both were wrong; the true occurrence count was **1,031**, and the agent
refused my figure rather than reconciling to it. **The cause: a hand-rolled Rust lexer with no
CHAR-LITERAL state.** `crates/sigil-frontend-as/src/eval.rs:2208` is `b'"'`, a byte-char literal
holding a double quote, so the lexer entered string state there and stayed desynchronized for the
rest of the file, counting hundreds of COMMENT dashes as string dashes. The error is **not one
directional**: after a desync, real strings are read as code and their dashes are MISSED too. A
lexer written to count something in a language that lexes text will meet its own constructs in that
text. **When a count needs a lexer, either use a real one or prove it on a file whose answer is known
by a second method.**

**2. A CONTROL IS WHAT MAKES A COUNT MEAN ANYTHING, and it caught all three.** The first measurement
returned **zero** (bad pathspec). The second and third were inflated (the lexer above). Every one
looked like a clean confident answer. What settled it was running a corrected lexer over BOTH trees:
**1,031 on master, 0 on the swept branch**, matching the agent's independently-implemented count
exactly. Two implementations agreeing at a non-trivial number is worth more than either one's care.

**3. MY OWN BRIEF CARRIED A CONTRADICTORY INSTRUCTION, and it cost the agent two suite runs.** I
wrote "set `CARGO_TARGET_DIR` to a path inside your own worktree". Inside the repo root,
`target-agent` reds `scripts_name_their_tree.rs:58` and `<root>/target` reds
`shared_target_defaults.rs:375`. **Only `.target-land`, which is `landing-run.sh`'s own default,
satisfies both.** The general shape: a brief that states a constraint in the abstract ("somewhere
under your worktree") when the tree actually admits exactly ONE value has stated a preference and
called it a rule. **Name the value.**

### MY OWN BISECTION WAS CONFOUNDED, AND THE AGENT REFUTED IT: the 518 block, 2026-09-05

**Both diagnoses I dispatched were wrong, and both refutations are verified firsthand here** (landed
`c325c7a2`). Recorded at this length because the failure was in the OVERSEER's measurement, handed
down to an agent as ground truth, and the agent's willingness to refute it is the only thing that
caught it.

**Defect 1, refuted outright. MY PROBE COULD NOT DISTINGUISH THE TWO ANSWERS.** I claimed `val()`
accepts a string literal but not a computed string, and showed `val(substr("JmpTo_Foo", 7, 3))`
failing. **AS `substr` is 0-based**, so that expression is `val("oo")`, which fails correctly because
no symbol `oo` exists. At offset 6 it assembles on master. Verified here: 7 fails, 6 exits 0. The
real fault was one level deeper, in which arguments `fold_const` expands.

**This is the standing bar aimed at its own author.** A probe whose failure is equally explained by
the mechanism you propose and by a trivial error in the probe is not evidence for either. **Ask what
OTHER answer the probe could have given**, and for any probe built on an index, an offset or a
boundary, verify the intermediate value before building the conclusion on it: one `dc.b substr(...)`
would have shown me `"oo"` in five seconds.

**Defect 2, real but materially wider than I wrote, which is the more dangerous error.** I said the
long absolute address operand cannot hold a string builtin. In fact **no 68000 instruction operand
ran any builtin layer at all**: `move.l #int(3.7),d0` fails on master with nothing string-shaped in
it, while `dc.l int(3.7)` assembles (verified here). **A narrow diagnosis produces a narrow patch
that closes the visible rows and leaves the rest of the class broken**, and the corpus count would
have gone to zero and certified it. The agent widened the fix to the operand layer.

**A BRIEF'S STATED MECHANISM IS THE DANGEROUS PART, AND THIS IS WHY.** The dispatch rule says label
mechanisms as hypotheses because an agent tends to reconcile its measurement to the controller's
story. I labelled mine as measurements, which was honest and correct (I had run them) and made them
harder to refute. **Measured and wrong is a real category.** The line that saved it was the required
"anything in this brief you concluded was wrong" section: it has now produced a correction in 4 of 4
dispatches that carried it, and this time the correction was the entire diagnosis.

### `git checkout <rev> -- <path>` STAGES, so `git diff --stat` is EMPTY on an applied mutation

*(Found by the 518-block parcel, reproduced here at the landing seat.)* The standing rule is that a
red-first proof must show the mutation applied on disk, because an unapplied mutation and a correct
restore are the same artefact. **The obvious command for that is `git diff --stat`, and it reports
NOTHING here** while `git diff HEAD --stat` reports 86 changed lines and the file on disk is plainly
reverted.

Direction, stated so the risk is not over-read: this yields a false **"the mutation did not apply"**,
not a false pass, so it wastes a cycle rather than certifying a vacuous gate. It still belongs
written down, because the reasonable response to a proof that keeps saying "not applied" is to reach
for a weaker proof method. **Use `git diff HEAD --stat`, or a content check (`grep -c` for a symbol
the fix introduces), which is what this seat used.**

### AND MY OWN INSTRUCTION CAUSED A SHARED-STATE MUTATION, in the field I had just corrected

I told the 518-block agent that `CARGO_TARGET_DIR` **must be `.target-land`** because the previous
parcel had measured that only that name satisfies both tree guards. I did not say **whose**
`.target-land`. Its first build therefore used the MAIN checkout's, relinking
`.target-land/release/{sigil,emp_census}`.

**Impact, assessed rather than assumed: none.** `.target-land` is a gitignored BUILD directory that
every landing run rebuilds, and the standing pinned assembler is `~/sonic_hacks/.pinned/sigil-0a58f2ec`,
read-only and untouched. `target/release/sigil` was not touched either. **The agent was still right
to report it**, since it had no way to know that, and an unexpected mutation of shared state is
exactly what a report is for.

**The lesson is about the instruction, not the agent. I fixed an ambiguity and introduced a
different one in the same field, one parcel apart.** The first brief named a constraint too
abstractly ("somewhere under your worktree") when exactly one value worked; the correction named the
value and dropped the tree. **A path instruction needs BOTH halves, and the template line is: the
`.target-land` inside YOUR OWN worktree, never the main checkout's.**

### A DIAGNOSTIC RESIDUE ROW WAS HIDING A SILENT ACCEPTANCE: the register fault, 2026-09-05

**Landed `4b6e0378`. Booked as a wording defect, dispatched by me as a wording defect, and the worst
of its seven outcomes was NO DIAGNOSTIC AT ALL.** Recorded because the row, the brief and the
reproduction at this seat all understated it in the same direction, and the thing that found it was
an agent testing the REALISTIC case rather than the minimal one.

**The row said two stories. My dispatch brief, after reproducing, said three. It was SEVEN**,
including two panics (`jsr <undefined>` exits 101) and one silent pass (`if <undefined>` is quietly
false at exit 0). Both of those are general, not register specific, and are booked separately.

**⚠ THE PART THAT MATTERS: `dc.l a0` IS SILENTLY ACCEPTED IN ANY FILE THAT ALREADY HAS AN ERROR.**
Verified firsthand here, not taken from the report. Injecting `dc.l a0` at line 86 of the corpus
`s2.asm`, the pre-parcel binary produced **5,243 rows and not one naming line 86**; the post-parcel
binary produces 5,244 with the sorted stderr diff exactly one added line. The mechanism: the register
defers as a fixup, the front end returns `Err` for unrelated reasons, and the link stage that would
have refused it is never reached.

**Why every earlier look missed it, and this is the transferable part. A MINIMAL PROBE IS A
DIFFERENT PROGRAM FROM A REAL ONE.** In isolation `dc.l a0` DOES produce a message, the ugly
locationless link-stage one, which is what the row recorded and what I reproduced. The silence only
appears once something else in the file has already failed, because that is what stops the run
reaching the stage that would refuse it. **A one-line probe systematically cannot see any defect
whose trigger is another defect**, and a diagnostic residue row is exactly where such a trigger is
guaranteed to be present in the field and absent in the probe. Where a fault is about error
REPORTING, probe it inside a program that is already failing.

**⚠ AND THE RULE HAS A SECOND DIRECTION, measured the same day on the two faults that parcel booked.
A fault can be VISIBLE ONLY WHEN EVERYTHING ELSE IS CORRECT.** `jsr <undefined>` panics at exit 101
on an otherwise clean file, and in a file carrying any unrelated error the front end returns `Err`
first and **the panic never happens**. Its sibling `if <undefined>` is silent in both. So of two
faults booked together, one is hidden BY other errors and the other is hidden by their ABSENCE.

**The general rule, which is what to carry: error-path behaviour is CONDITIONAL ON WHAT ELSE FAILED,
so a probe samples ONE POINT in that space and both directions must be run.** Probing only clean
files misses everything an earlier stage swallows; probing only dirty ones misses everything that
needs a clean run to reach. Neither habit is the safe one, and **a lane that adopts only the rule
above has swapped one blind spot for the other.**

**Root cause, one rather than several: a register name is not in the symbol table, so an expression
holding one folds to `Poison`, and POISON IS THE SHAPE OF A FORWARD REFERENCE.** Every consumer did
what a forward reference deserves, defer it or call it unresolved. The discriminator is cheap and
was simply never applied: no later pass defines `a0`. Fixed at all 15 consumers, at the point of use,
with that line's span. **The bar this instance illustrates is the standing one: a property verified
at the PRODUCER is not a property of the CONSUMERS, and the population to enumerate is always the
consuming end.** One bespoke check at one producer was landed on 2026-09-05 and made exactly one of
seven variants read correctly, which is what a producer-side fix buys.

**The corpus table for this parcel reads NOTHING MOVED, and that is INERTNESS rather than a
measurement**, which the agent stated rather than letting the zero speak. The corpus contains no
instance of the fault and cannot. The injection is the engagement witness, and without it the table
reads identically whether the code ran and agreed or never ran at all.

**Over-firing is the dangerous direction for a new refusal and was checked here on both binaries**: a
symbol legitimately named `a0` still assembles, real addressing modes are untouched, and a genuine
forward reference still defers instead of being called a register.

### THE `if` REFUSAL: how a new refusal on the shipping path was cleared without an aeon build

*(Landed `d9f00a3e`. Recorded because the clearing argument is reusable and because the agent
escalated correctly rather than pushing through.)*

Fault 1 made the assembler **refuse** what it had accepted, which is the direction that reds correct
code. The agent answered the design question with evidence and it **inverted the assumption in the
brief**: **sigil is the MORE permissive assembler here.** It resolves a forward reference in an `if`
by iterating passes to a fixpoint; `asl` refuses a forward `equ`, a forward label, a forward `set`
and an include-after-the-`if`, all four at exit 2. **So keying the refusal on CONVERGENCE rather than
on asl's first-pass rule is strictly weaker than the reference and cannot red anything asl accepts.**
Keyed on asl's own rule it would have red four legitimate shapes. That is a BLOCKED question answered
rather than assumed, and the answer chose the design.

**The agent TAGGED an aeon risk it could not close and named the settling command (a strict landing
run on a provisioned tree). It was closed here two cheaper ways, and both are reusable:**

- **By MEASUREMENT, on the whole population rather than a sample.** Aeon's AS-routed surface is three
  files and **its include closure was verified self-contained earlier the same day** (both
  `game_root.asm` include only `engine/debug/debugger.asm`, which includes nothing). The new binary
  over all three gives **zero firings**, and both game roots assemble at **exit 0 with zero rows**,
  which is conclusive for the third file because each of them includes it. **The direction is sound
  too:** a standalone run defines FEWER symbols than the real build, so it can only OVER-report; a
  clean standalone result therefore implies a clean real one. State that direction, because a
  standalone run is otherwise easy to dismiss as unrepresentative.
- **By CONSTRUCTION, from the code being replaced.** All three width-deferred arms in the old
  `link()` were `unreachable!`, so **any build reaching them PANICKED**. The change is strictly panic
  to diagnostic and **cannot turn a green build red**. Reading the OLD code settled in one command
  what a build would have cost hours to demonstrate.

**The generalisable move: when a change makes something newly refuse, ask what the OLD code did on
the same path.** If the old path was a panic, an `unreachable!`, or an already-failing branch, the
new refusal cannot regress a working build and no build is needed to prove it. That is a stronger
argument than a green run, because a green run samples inputs while this quantifies over them.

**Two rows opened rather than closed, both silent-wrong-answer class:** `MOMPASS` is unimplemented
and was silently reading FALSE and dropping its block at 7 of the corpus's 11 firing sites; and `&&`
binds tighter than `=` in asl and looser here, so `(K*2)=6&&(J<>3)` is `0` there and `1` here, with
near-zero corpus population only because the disassemblies parenthesise.

### A CONTROL CAN BE CONFOUNDED BY THE VERY THING IT CONTROLS FOR (MOMPASS, 2026-09-05)

*(Landed `f1673ba2`, after this seat HELD the first version. Three lessons, and the first is the one
this document did not already have.)*

**1. "Pre-existing" is a CLAIM, and it needs its own control, which can itself be confounded.** The
parcel booked a discarded `fatal` as a pre-existing fault it merely widened, supported by a control
file whose before/after behaviour was identical. **The control was confounded by the fault it
existed to isolate:** its `fatal` sat behind a condition that folded to `Poison` on the first
iteration, so the arm was skipped and **the `fatal` never executed at all**. Identical before and
after was perfectly real and meant nothing.

Re-controlled three other ways, the truth was the opposite and sharper: **the parcel did not widen
that fault, it CREATED THE ONLY POPULATION THERE IS.** The structural reason is worth keeping: a
`fatal` aborts its pass, which truncates the environment, so whatever would flip its condition never
runs and it re-fires on every pass. Anything preceding it has the same value every iteration.
`MOMPASS` is the sole exception, because it flips for a reason internal to the assembler rather than
to the program. **A "this was already broken" finding is exactly as falsifiable as a "this is newly
broken" one and deserves the same instrument.**

**2. FIDELITY IS A COST, NOT A GOAL, AND THE COST IS MEASURABLE.** `asl` literally terminates on a
`fatal`, so the faithful hard stop is the obvious implementation and was written first. Measured, it
**cut s1disasm from 50 located diagnostics to 1 and skdisasm from 2132 to 1**, the survivor in each
being a line already printed. Matching the reference exactly would have destroyed the tool's own
diagnostic output. The shipped design CARRIES the diagnostic instead, deduped: **strictly additive,
louder never quieter**, which is the property to reach for whenever a change touches reporting.
**Implement the faithful reading first BECAUSE it is cheap to measure, then let the measurement
choose.**

**3. HOLDING A PARCEL OVER A LOUDNESS REGRESSION COST ONE MESSAGE AND WAS RIGHT.** The first version
was silent at exit 0 on a shape where master was loud at exit 1 and asl loud at exit 3. The hold
named the shape, gave the three measurements, offered three acceptable outcomes including "your
reading is wrong", and refused to design the fix. What came back was a narrow fix, a refutation of
the parcel's own framing, **and a defect in the fix itself found on disk rather than in theory**: ids
are handed out in splice order, so a carried span reported a `fatal` written in `inc/c.asm` as
`inc/b.asm(1)`, a real file and a real line and the wrong one. **Do not accept a report's own
severity classification when the landing consequence is checkable in three commands.**

**Both remaining divergences are pinned AS NAMED TESTS** (`one_pass_asl_file_is_a_known_divergence`,
`mompass_eq_two_guarding_an_emission_diverges_from_asl`) so they read as decisions rather than as
oversights a later session might "fix" without knowing they were chosen. `warning` and `message` on a
non-final pass are still dropped, deliberately: unlike `fatal`, asl keeps assembling past a `warning`
and prints it once per pass, so a later pass genuinely does supersede it. Booked with a kill
condition rather than left implicit.

### SHELL `grep -r` IS A FUNCTION HERE AND SKIPS GITIGNORED FILES, RETURNING A CLEAN ZERO

*(Reported by the aeon lane at aeon `56e42f00`, reproduced by aurora with a canary, and REPRODUCED
HERE before being banked. It had already put a false "symbol appears nowhere" into one lane's brief.)*

`type grep` on this machine returns **"grep is a shell function"**, sourced from the harness's shell
snapshot. That function's `-r` **skips gitignored paths and reports success with no output and no
error**. Everything this lane generates is gitignored: listings, ROMs, build directories, generated
corpus files, probe output.

**Reproduced here, and the first attempt was VOID, which is the part worth carrying.** The canary
went into a file that turned out not to be gitignored, so both greps found it and the test could not
have come out any other way. Redone with `git check-ignore -v` confirming the path first:

```
canary in a verified-ignored path
  shell `grep -r CANARY .`      ->  0 matches, exit 0, no error
  /usr/bin/grep -r CANARY .     ->  FOUND IT
```

**Pick the instrument by what the subject IS:**

- **tracked source** (corpus `.asm`/`.inc`, our `.rs`, docs): `git grep`, which is also faster and
  respects the repo boundary;
- **ignored artifacts** (a listing, an output binary, anything under a build dir): `/usr/bin/grep -r`
  by absolute path, never bare `grep -r`;
- **either, when a zero would be a finding**: plant a canary of the same class, confirm it is ignored,
  and confirm the instrument finds it *before* believing the zero.

**AUDIT OF THIS LANE'S 2026-09-05 MEASUREMENTS, run rather than asserted.** No reported zero came
through this path. The dash counts used `git grep -l` plus a Rust-aware lexer; the aeon
nameless-label and MOMPASS populations used `git ls-files` and `git grep`; the corpus logical-operator
count used `git grep`; the aeon three-root assertions ran the assembler over named files rather than
searching text; and the one bare `grep -r` over a directory (`crates/sigil-frontend-emp/src/`)
returned a NON-zero result and holds only tracked source anyway. **This audit is worth the five
minutes precisely because the failure is invisible: a lane that had been bitten would look exactly
like a lane that had not.**

**And it is the FOURTH instrument that returned a confident wrong zero to this lane in one day**, all
different mechanisms: a `git grep` pathspec that matched no files; a hand-rolled lexer desynchronised
by a char literal; a peer's `\|` alternation inside `$'...'` under zsh; and now this. **The rule is
not "watch out for greps", it is that AN EMPTINESS IS NEVER A FINDING WITHOUT AN INSTRUMENT THAT
COULD HAVE RETURNED NON-EMPTY.** Same family as `cmd | sed ... || echo`, where the `||` binds to the
whole pipeline.

## MASTER'S STRICT RED: FOUR OF FIVE WERE MINE AND ARE NOW CLOSED (2026-09-05)

**RESOLVED at `34dad07c`. Merged-tree strict gate: 4,616 passed, 1 failed**, the survivor being the
pre-booked `pins_rs_is_current` row. The four `diag_assert_vector` failures are green. The history
below is kept because the reasoning error that caused them is the transferable part, and because the
fix changed what the failure MEANT.

**The refusal was an OVER-FIRE and the underlying defect PREDATES the parcel by two months**: the
`pos < 0` guard came from `d59bab36` (2026-07-04) and is present at `742c7366`, where the tests
passed. `d9f00a3e` converted an accidentally-right SILENT answer into a visibly-wrong LOUD one, since
the silent-false read produced the same verdict asl gives on that site. **It exposed a latent bug
rather than creating one. That does NOT retract the enumeration error below, which stands on its own
terms and was a real mistake in reasoning.**

**⚠ AND THE FOUR RED TESTS COULD NOT HAVE DISTINGUISHED A CORRECT FIX FROM A WRONG ONE** *(the
agent's finding, and the most important thing in the parcel)*. `diag_assert_vector`'s "AS reference"
is **sigil's own AS front end**, not `asl`, so clamping the position to 0 would have turned all four
green while disagreeing with asl's actual length law. **Their green proves the regression is gone and
proves nothing about correctness**; the grounding lives entirely in three new asl-derived unit tests.
A red test going green is evidence about the regression, never about the fix, whenever the test's
oracle is the tool under repair.

**asl has no SEMANTICS for a negative position, only BEHAVIOUR.** It does not clamp; it reads below
the buffer, so `strlen(substr("wxyz",-4,0))` is **8**, longer than its source, which is what makes
that probe discriminating (no clamping model can exceed the input). `substr("a",-1000000,0)`
**segfaults asl, exit 139**. There was never a correct value to copy: the fix reproduces the length
law and models the unreadable prefix as NUL, which decides every comparison as asl does because an AS
string literal cannot contain a NUL.

### The original entry, kept for the reasoning error


**Read this before landing anything or reading a green partial run as safety.** `SIGIL_STRICT_GATE=1`
with a verified `AEON_DIR` returns **5 failures** at `33aeee96`. Every parcel today ran
`SIGIL_ALLOW_PARTIAL=1`, which skips 127 reference-dependent binaries, so none of them could see this.

| failure | attribution | state |
|---|---|---|
| `diag_assert_vector` x4 | **MY OWN if-condition parcel `d9f00a3e`.** Bisected: **15 passed / 0 failed at its parent `742c7366`** | booked `AS-IF-REFUSAL-DIAG-VECTOR`, top of queue |
| `repin_pins::pins_rs_is_current` | fails on master too; **declares STALE while reporting ZERO changed pins** | booked `PINS-GATE-CONTRADICTS-ITSELF` |

**The four are the hazard I warned an agent about, landed by me.** The `if` parcel makes the
assembler refuse a condition it cannot decide. I cleared its blast radius with this argument:
*"aeon's AS-routed surface is three files, zero firings, and a standalone run defines FEWER symbols
than the real build so it can only OVER-report; a clean standalone result therefore implies a clean
real one."*

**The argument was wrong, and the flaw is exactly the bar this document enforces on everyone else:
THE POPULATION TO ENUMERATE IS THE CONSUMING END.** I enumerated aeon's three roots as the consumers
of `debugger.asm`. **Sigil's own test harness is a fourth consumer**, and it does not assemble that
file either standalone or as aeon builds it: `diag_assert_vector` SYNTHESIZES a third context, its
own header plus stub `equ`s, and assembles the real `debugger.asm` inside it. My two-case reasoning
("standalone" versus "the real build") did not contain that case, so the implication never held.

**And "fewer symbols can only over-report" is itself false in general:** a synthesized context does
not define a SUBSET of the real one, it defines a DIFFERENT one, and a stub set to a particular value
can make a condition undecidable that would decide either way elsewhere.

**The failing site is `debugger.asm:572`,
`elseif (strlen(OPERAND)>4)&&(substr(OPERAND, strlen(OPERAND)-4, 4)="(pc)")`. Do not diagnose it from
that line alone**: reduced to a standalone macro with the same condition, sigil and asl AGREE
(`AA BB`), so the fault is context-specific and is NOT the obvious nested-string-builtin reading.
**Whether the refusal is CORRECT (the synthesized context genuinely cannot decide it, and the test's
golden was being produced by the silent-false bug this parcel removed) or an OVER-FIRE is the first
question of that parcel, and it decides whether the fix is in the test or in the assembler.** If it
is the former, re-baselining the test is the move this document forbids twice over; the expectations
would have to be re-derived from `asl`.

**What this costs, stated rather than softened: a partial run is not a landing gate, and I treated it
as one six times today.** Every one of those parcels reported honestly that the byte gates had not
executed. The reporting was correct and the LANDING DECISION still went ahead on it. The rule this
lane needs is not better disclosure, which was already perfect, but that **a parcel touching the AS
frontend gets a strict run before it lands, or it lands knowing the gate is owed.**

### THE PINS GATE WAS RIGHT AND ITS MESSAGE WAS NOT: one root cause, one loud artifact, one silent one

*(Closed 2026-09-05. The gate was booked here as self-contradicting. It was not contradicting itself;
it was reporting two true things whose juxtaposition reads as a contradiction.)*

**`pins_rs_is_current` failed with `src/pins.rs is STALE against the live listings (0 changed pin(s))`.**
The verdict comes from a WHOLE-TEXT comparison of committed against generated; the count comes from a
PIN-LEVEL differ. The file genuinely was stale and genuinely no pin had moved, so both halves were
true and the message was unreadable.

**THE CAUSE WAS MY OWN DASH SWEEP, and the mechanism is worth keeping.** `f6618ec9` correctly swept
STRING LITERALS and correctly left COMMENTS alone. But 38 lines of `crates/sigil-harness/src/repin.rs`
are string literals that RENDER comments into a generated file. So the generator started emitting
`GENERATED FILE, DO NOT EDIT BY HAND` while the committed `pins.rs` still carried the em-dash form in
its 108 comment lines. **A sweep scoped by what a token IS in the source can still move what a
generator PRODUCES, and the generated artifact does not update itself.**

**⚠ THE SAME ROOT CAUSE HIT A SECOND ARTIFACT SILENTLY, AND THAT PAIRING IS THE LESSON.** The three
golden-vector headers booked as `GOLDEN-HEADER-UNGATED` are the same defect: swept generator, stale
generated file. **`pins.rs` had a gate, so it went red within hours. The vector headers have none, so
they drifted with nothing to notice.** One cause, two artifacts, and the only difference in how it
surfaced was whether somebody had written a currency check. **After ANY sweep, the population is not
"files matching the pattern", it is "files matching the pattern PLUS every artifact generated by
one".**

**Closed by REGENERATING, never by hand**, which is this document's own standing ruling on generated
artifacts, applied to itself. Verified before landing with the sweep's own discipline: **all 108
changed lines are punctuation-only after stripping dashes and punctuation, and NOT ONE hex constant
appears on one side and not the other.** `repin` reported `0 pin(s) changed` and the diff proves it.
Gate green after.

**Also true and worth fixing separately: the remediation the failing gate PRINTS does not work as
printed.** `run: cargo run -p sigil-harness --bin repin` bails out with a hint unless `SIGIL_EMIT` is
also set, doing nothing and reporting no error. A gate whose own advice is incomplete sends its reader
in a circle. Booked `PINS-GATE-MESSAGE-MISLEADS` with both halves: make the count and the verdict
describe the same comparison, and make the printed command the command that works.
