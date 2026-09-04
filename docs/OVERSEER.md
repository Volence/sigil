# Sigil Overseer

How a Claude session runs **sigil** as its overseer. The shared role — solo-first
posture, delegation discipline, review bars, peer protocol — lives in
`empyrean/docs/OVERSEER-PROTOCOL.md`; read it once, then this file for what is
sigil-specific: the landing-lane division, the worktree/test quirks, and the queue.

> **This file is the boot read and is bounded** (protocol: *The boot read is bounded*,
> `empyrean/docs/OVERSEER-PROTOCOL.md` at `origin/main`). Dated entries — every night's
> findings, landing records and closed parcel narrative — live in `docs/OVERSEER-LOG.md`,
> append-only, newest last, **not read at boot**; reach for it with `tail`/`grep` when a
> particular night or a cited section is in question. A ruling that must survive a rotation
> is written HERE as well as there, never only into the log. A few sentences below still say
> *"the section above/below"* about a dated section that has moved; those resolve in the log,
> which carries every entry under its original line span.

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
row that is not itself waiting on another lane or an owner call; when the effects row is owner-blocked
— which it is, on `d-24` — the named fallback is **SIGIL-AS-REPLACEMENT**. The `overseer` skill's
boot stop is real and is overridden HERE, by him: it names an exception for a standing instruction
from the owner, and this is one. **Two sigil sessions booted into that stop on 2026-09-04 alone.**

**The gate is a condition to MEASURE.** Check *"aren't waiting on them"* against the other lanes'
live rows — and a row naming sigil settles nothing either way: on 2026-09-04 three named sigil while
the thing all three waited on was finished and pushed. **Ask whether sigil could release it, not
whether sigil is mentioned.**

**Provenance disclaimer, carried deliberately:** this reached the lane through the hub. A relay of
his ruling is his ruling — but only because the two commands above were run here and the join
checked. It authorizes taking the next unblocked row and nothing further; it does not authorize
landing anything he has parked, and `d-24` stays parked.

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

**PREFER A POSITIVE FREEZE WITNESS TO `pins.rs unchanged`** *(the aeon lane's finding, in one of
this repo's own artifacts. Episode — the chain-180 discharged falsifier and chain 181:
`docs/OVERSEER-LOG.md`, 2026-09-04 cut, original lines 97-118.)*

`pins.rs unchanged` is an **absence**: equally consistent with a correct length-neutral parcel and
with a build that never ran. `golden/offcanonical_sizes/s4.txt` is a **positive** witness — a table
of unmoved labels beside two changed CRC headers cannot be produced by a build that did not run.
**Two limits, so it is not adopted wider than it earns.** (1) It witnesses that a build ran and
produced these labels; it is **silent on which source that build used**, so it composes with the
assembler md5 rather than replacing it. (2) It is positive **only for a length-neutral parcel** — on
a parcel that moves lengths the table moving is expected, and it reverts to something to reconcile.

**And record a pre-agreed prediction that comes true as a DISCHARGED prediction, never as a green
run** — it is worth something only if it is written down that it was agreed first.

### d-18: REFUSE BARE, OPT IN TO PARTIAL — and `contract/SUITE_PATHS.md` names the resolver

**The rule.** A bare run without the reference tree **stops**, with an error naming the variable
consulted and the path tried. An explicit opt-in of this lane's naming (`SIGIL_ALLOW_PARTIAL=1`)
runs the partial suite and prints the derived not-measured count plus the disclaimer. Grounds: a
run that prints how much it skipped **still exits 0**, and a silent green is the class we never
drop.

**The resolver precedence** (`contract/SUITE_PATHS.md`, empyrean): explicit var > suite-root var >
derived via `git rev-parse --git-common-dir` (**never** `--show-toplevel`, which lies from a
worktree) > **refuse by name**. A variable that is set but wrong is a hard error, not a null that
lets the next step run. `AEON_DIR` is the ratified checkout spelling; `EMPYREAN_SUITE_ROOT` is the
suite-root name. The d-18 refusal IS step 4 of that precedence, so resolver lands first and refusal
on top of it.

*(The delegation it was ruled under, its authority trail, and this lane's migration list in order:
`docs/OVERSEER-LOG.md`, 2026-09-03 cut, original lines 86-123.)*
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
### Rules banked from closed findings — the narrative is in `docs/OVERSEER-LOG.md`, 2026-09-02

Each of these was a dated section here until the boot read went over its byte bound. **The
rule is what survives; the episode that earned it is in the log** under its original
heading, verbatim. When a rule and its narrative disagree, the rule wins.

- **A LOSSLESSNESS PROOF CERTIFIES THAT NO TEXT WAS LOST AND NOTHING ABOUT WHETHER THE
  SURVIVING TEXT STILL PARSES** *(2026-09-03, found by the agent that split this file, and it
  is the failure its own proof could not see)*. A set-difference over lines is blind to a
  **dangling reference**: when a paragraph moves to the log, a sentence left behind whose
  antecedent went with it survives the proof intact — every line accounted for, and the head
  simultaneously lossless and less legible. Instance: *"`"if needed"` is the whole defect"*
  passed cleanly while the episode defining *"if needed"* had moved. **Reading every seam is a
  separate step from the proof**, and it is the step that finds this class. Generalises past
  document splits: any mechanical completeness check answers the question it enumerates and
  silently declines the one it does not.
  **ITS SECOND BLIND SPOT, measured on the 2026-09-03 cut itself: the proof cannot distinguish
  text that MOVED from text DELIBERATELY REWRITTEN in the same pass.** Repairing one dangling
  sentence in place made the proof report a loss that was not one — and the tempting fix, quietly
  whitelisting the line, would have made the proof unable to see a real loss with the same shape.
  **Declare every in-pass repair by hand, in the proof's own output**, so the exclusion is visible
  rather than absorbed.
- **A RED-FIRST PROOF HAS FOUR KNOWN WAYS TO GO VACUOUS, and they share one artifact: a
  green run over a change you can see in the tree.** (1) `git checkout --` restoring a
  dirty tree, so later mutations patch an already-restored file; (2) a mutation that never
  applied; (3) Python reading a stale bytecode cache — invalidated on `(mtime, size)`, so a
  size-preserving edit runs the module's OLD self, and `-B`/`PYTHONDONTWRITEBYTECODE` do
  NOT help because they stop the cache being written, not read; (4) the binary that ran was
  built from other source. **So a proof must show the mutation LANDED and must state what
  the run MUST FAIL.** Invariant 8(c) — applied-but-still-green is a runner defect, never a
  pass — is the only clause that catches all four, and it was written before (3) was known.
  *Python subjects specifically:* set `PYTHONPYCACHEPREFIX` to a fresh per-run temp dir AND
  have the runner count the bytecode files it wrote, refusing on zero. This tree's `.py`
  exposure was nil when measured (27 tracked files, none used as a red-first bed) — that is
  a conditional clearance, not a standing one, and it expires the first time anyone mutates
  a `.py` file on disk and re-runs it.
- **PIN THE TOOL, AND MAKE THE LOG NAME IT.** `SIGIL_BIN`/`SIGIL_BUILD` left to default
  select whatever a shared checkout last relinked, which once certified a byte-neutrality
  proof against a pre-merge compiler while printing four `MATCHES THE GOLDEN` lines. Pass
  the binary the parcel built, head the log with the tool's own `--version`, and state the
  predicted red. **The correspondence check is `closure-revision`, never `revision` vs
  `git rev-parse HEAD`** — `revision` moves on every commit including ones no compilation
  can see, so the naive form fires on a correct binary, and always-red is not the safe
  direction. `SIGIL_BIN_CLOSURE` is the hatch for a legitimate off-tree run and is not a
  silencer: it must equal what the binary reports.
- **BEFORE CALLING ANY RESULT A REFUTATION, SAY WHAT OBSERVATION THE HYPOTHESIS FORBIDS.**
  If the result you got is one the hypothesis PREDICTED, the word is confirmation, whatever
  it feels like. Earned by getting it backwards in a sentence a peer then banked in their
  own tree, in this lane's phrasing, before the run — a convenient error that travelled.
- **DETACH THE WORK, BUT DO NOT WATCH IT WITH A HARNESS BACKGROUND TASK: the watcher is
  what gets reclaimed.** Prefer the ARTIFACT question (*did the log get its `finished=`
  line*), which anyone can answer later, over the PROCESS question (*is it alive*), which
  the harness can answer wrongly out from under both asker and answer. **And never poll a
  pattern the poller's own command line contains** — `pgrep -f <script>` matches the waiter
  itself and waits forever, which is indistinguishable from a job still running. Cost half
  an hour on a critical path once, in each of two lanes, on one day.
- **`==` REFUSES ACROSS COMPARISON CLASSES** (`[eq.cross-type]`, naming both types and the
  constant it stuck at) and is defined WITHIN one. Two cross-kind pairs stay DEFINED because
  the corpus depends on them and neither is a mistake: a newtype beside a bare int (§8.3
  erasure) and a label beside `0` (the empty-pointer-slot spelling). Both are always false,
  which is exactly the case that must be written down — it is in
  `docs/EMP_PITFALLS_EQUALITY.md`. **The principle, which outlives the parcel: a comparison
  or an annotation that cannot be meaningfully evaluated must REFUSE rather than produce a
  value, and an always-RED check is worse than an always-green one** — it fires on correct
  code, and the remedy a reasonable person reaches for is deleting the guard.

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

**A RE-BASELINE DOES NOT EXPLAIN A GREEN — IT MANUFACTURES ONE, AND THE TWO LOOK IDENTICAL**
*(2026-08-30, aurora's; the baselines are ours, so it lands here. Episode:
`docs/OVERSEER-LOG.md`, 2026-09-04 cut, original lines 201-232.)*

**A check whose expectation the subject generates can only ever agree with the subject.** Its green
is evidence about *reproducibility*, never about *correctness* — a re-baseline bakes whatever was
there into the expected output, so from that instant the gate passes forever, and the green is
identical whether the mismatch was an artefact or a real defect the baseline has just absorbed.
When the baseline is generated by the subject, the instrument has not lost the power to detect
drift; **it has swallowed it.**

**So name a source of the expected value that does not pass through the subject, and establish it
BEFORE the run** — afterwards there is no way to tell which you had. The best member of such a set
owes nothing to the instrument it vouches for (for the `+40`: the fourteen inserted instructions
summing to 40 bytes from the 68000 encodings alone, no listing, no baseline, no build).
**And the discrimination is real rather than theoretical**: the same re-baseline that absorbed the
`+40` failed to absorb two rename failures, because those read struct field *declarations* rather
than goldens, so the subject could not generate their expectation. Aurora's bar: **a diff surviving
a self-generated baseline is very hard to argue away.**

## The drift watch's timer — the unit name and how to switch it off

Written here so the owner can turn it off without asking anyone, and so a successor does
not have to reconstruct the unit name from the scripts.

```sh
systemctl --user status  sigil-ref-drift.timer     # is it armed, and when does it next fire
systemctl --user disable --now sigil-ref-drift.timer   # off, immediately and across reboots
systemctl --user enable  --now sigil-ref-drift.timer   # on
journalctl --user -u sigil-ref-drift.service -n 50     # what the last firing said
```

The unit files are committed at `scripts/systemd/`; a `--user` unit lives outside every
repo, so **installing them is a copy to `~/.config/systemd/user/` and nothing in any repo
can tell you whether that happened** — ask `systemctl`, never a doc. The job's own record
is `~/.local/state/sigil-ref-drift/observations.jsonl` (append-only) with its run log
beside it at `nightly.log`.

**⚠ AND ARMED IS NOT THE SAME AS PRODUCING EVIDENCE — the more important half.** The job
**cannot accumulate a record**; it is built so it cannot (*"holds NO expectation of its own
and is built so it cannot acquire one"*). Expectations enter only through
`aeon/tools/drift_record.jsonl`, which aeon's `docs/DRIFT_RECORD.md` updates by a **manual**
append-and-commit. That record has held **two** entries, both at old aeon revisions, since at
least 2026-09-02 — so every firing against a moved aeon returns `unattributable-both-moved`,
which is correct behaviour and is **not evidence**. The verdict `SIGIL-DECOUPLE` step 4 needs
is `quiet-sigil-moved`, and it is unreachable unless the aeon revision is IN the record.
**Waiting for the nightly to "build up a record" therefore accumulates nothing** — a queue row
in this lane promised exactly that for days. Full measurement:
`docs/superpowers/notes/2026-09-03-per-parcel-term-feed-cut.md`, instance 2.

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

## Dispatch practice — a stated MECHANISM is more dangerous than a stated FACT

A brief's factual claims compete with the agent's evidence and lose when wrong. A brief's
**explanations do not compete — they absorb.** An agent that measures something inconsistent with
its controller's stated mechanism tends to reconcile the measurement to the story rather than
report the conflict, and **it has almost none of the standing a peer overseer has to push back.**
**So label mechanisms in a brief as hypotheses, and say outright that the agent's own command
output outranks anything the brief asserts.**

**Every dispatch's deliverable section ends with a required line:** *"and anything in this brief you
concluded was wrong."* Measured hit rate when introduced: a correction in **3 of 3** dispatches.
Pair it with an explicit invitation wherever the brief carries a design position.

**Its honest limit:** a self-report line cannot surface what the agent never thought to question. It
catches conflicts the agent noticed and would otherwise have swallowed — a real but bounded win.

**⚠ TELL EVERY AGENT WHAT HAPPENS TO ITS BRANCH AT THE LANDING, because this lane deletes it.**
Briefs here tell an agent to commit early so a death costs only the run — teaching it that its
commits are precious — and then the controller merges, pushes and **deletes the branch and worktree
those commits hung from**, with nothing saying that is the normal ending. It cost one agent a full
recovery cycle. **We taught the fear and not the ending.** Carry this until it lands in the shared
dispatch block (dominion `50bb5e9`, invariant 12):

> *When your parcel lands, the controller merges your branch to master and then DELETES the branch
> and its worktree. A missing branch after a landing is the expected end state, not lost work.
> **Record your tip SHA while a ref still exists** — after the tidy it is the only handle. Then
> `git merge-base --is-ancestor <tip> master`: **ancestor PROVES it landed; non-ancestor proves
> NOTHING**, because a squash or rebase merge rewrites the commits and the tip stops being an
> ancestor of anything while the change sits in master. On a non-ancestor, confirm by content.*

**The asymmetry is dominion's correction to this rule and it matters here specifically:** this
controller uses real merges, so the positive check works — but the day a lane moves to squash or
rebase merges, the check silently stops detecting its own instance and the caveat becomes the whole
rule. It is this file's banked shape met from the other side: **a vanished branch whose commits
survive unreferenced is the signature of a MERGE**, exactly as an empty commit range is.

*(Origin, and the n=3 caveat in full: `docs/OVERSEER-LOG.md`, 2026-09-03 cut, original lines
466-488.)*
## Quality bars

**⚠ THE REFERENCE TREE IS WHATEVER `golden/provenance.toml`'s TIP PAIRS WITH — and every
CRC written into THIS file is a snapshot that a refreeze silently invalidates**

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
reading as fact.

**So derive by PROPERTY, not by name.** A reference tree is *a worktree of aeon whose HEAD is the
provenance tip's `aeon_rev`* — that sentence is the definition, and this finds every tree matching
it however anyone chose to name it:

```sh
TIP=$(grep -E '^aeon_rev' crates/sigil-harness/golden/provenance.toml | tail -1 | sed 's/.*"\(.*\)"/\1/')
MAIN=$(git -C ../aeon rev-parse --path-format=absolute --git-common-dir | xargs dirname)
git -C ../aeon worktree list --porcelain \
  | awk '/^worktree /{w=$2} /^HEAD /{if ($2=="'"$TIP"'") print w}' \
  | grep -v "^$MAIN\$" | grep -v '/\.claude/worktrees/'
```

**The two exclusions are not tidying and were found by running the command as written.** Without
them it returns the OWNER'S LIVE CHECKOUT (which is at the tip whenever he has not moved, and is
the one tree every rule here forbids as a reference — he authors in it, `sigil build` writes into
it) and agent worktrees under `.claude/worktrees/`, which are somebody's in-flight development
trees. The first draft of this block had neither exclusion and would have pointed a cold session
straight at his tree. *A derivation is not automatically safer than a list; it just fails
differently, and it has to be run before it is written down.*

It can still return more than one, and that is correct rather than ambiguous: **the disambiguator
is the artifact, not the name** — CRC32+size all four ROMs against the tip entry and take the tree
that matches. A tree at the right revision whose shapes are wrong is not a reference tree (an
in-flight one mid-rebuild will have three of four), which is why the revision alone was never
sufficient.

`~/sonic_hacks/.aeon-sigil-gates` is never a candidate — it is source-only by construction and
deletes built ROMs, per the source-gate lane section. It will not appear above anyway unless it
happens to sit at the tip, which is the point of deriving rather than listing.

**The `AEON_DIR`-matches-the-provenance-tip pairing is gated, as of 2026-08-26.** What shipped:

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

- **HOW LONG THE STRICT LANDING RUN TAKES, AND WHY THE NUMBER MATTERS.** The harness caps a
  foreground `Bash` call at ten minutes, and **a capped run is a KILLED run whose log still
  aggregates clean** — no `FAILED` lines, a plausible total, and nothing saying it stopped
  early. So the gate closest to that cap is the one that can silently turn into a green.

  **Do not read a duration off this page — every run stamps its own.** `landing-run.sh` writes
  `# started (UTC)` before cargo and `# finished (UTC)` after it (`:428`), so any log answers
  this for the tree and box it actually ran on:

  ```sh
  grep -E 'started \(UTC\)|finished \(UTC\)' "$LOG"
  ```

  **Measured 2026-09-03 across three independent runs on an otherwise busy box: 3m40s, 3m56s,
  4m03s** — about 2.5× headroom under the cap, comfortable today and not guaranteed on a
  loaded machine or a larger suite. Treat that as a snapshot with a known direction of travel:
  the suite only grows.

  **The completeness check is POSITIVE, and log-stamping does not provide it.** A stamp answers
  *which tree*; it does not answer *whether the run finished*. Assert the **suite count** and the
  runner's **own exit line** (`CARGO_EXIT`), never infer a pass from the absence of failures —
  those two are what separate "everything passed" from "it was killed a third of the way in",
  and an aggregate total is equally consistent with both.

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

  **This paragraph said `3939 / 3943` for a day while the RELAYOUT-REVIEW section below
  recorded `3943 / 3947` from the same tree, and the stale half is the one a cold boot quotes
  — this overseer quoted it to a peer, who reconciled against `git grep -c '#\[test\]'` and
  refused it.** That is this document's own trigger-less-prose defect, one section below the
  paragraph warning about it. **Reconcile against the declaration, never against this
  sentence** — by the `git grep -c '#[test]'` method below, which is an APPROXIMATION and
  not the identity this line used to assert.
  **DERIVE the expected `ratchet:` count from the chain; do not read a number off this
  page.** This paragraph has now carried three different fixed counts (zero, then exactly
  one, then zero again), each true when written, and a fixed count here is the
  snapshot-in-standing-fact-grammar defect this file warns about elsewhere. The rule that
  does not rot:

  ```sh
  # does any entry record a strict run? if yes, the strict-attestation ratchet is disarmed
  grep -c '^\[entry\.strict\]' crates/sigil-harness/golden/provenance.toml
  ```

  Non-zero means the strict-attestation rule is IN FORCE and the expected `ratchet:` count
  is **zero** — any line is then worth investigating. Zero means no entry records a strict
  run yet, the rule is not in force, and the run emits **exactly one** ratchet line reading
  *"no entry in this chain records a strict run yet"*. **Measured 2026-09-03: the chain
  carries `[entry.strict]` tables with `sigil_rev` set, so the attestation has landed and a
  strict landing run correctly emits ZERO ratchet lines** — observed on the shift parcel's
  own run (376 suites, 4273/0/2, zero `skip:`, zero `ratchet:`).

  Two self-disarming ratchets exist and they say different things, so **read the sentence,
  not the word.** The old `aeon_rev` pairing ratchet disarmed at chain 167 and its
  reappearance would still be a defect: a `ratchet:` line about a missing `aeon_rev` means
  a tip was written without the field, which `check`'s monotonic rule should already have
  refused. Investigate rather than tolerate.
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

  **⚠ THE REFERENCE CHECKOUT MUST CARRY NO `sigil`/`skdisasm` SYMLINKS.** The aeon main tree
  has none, and adding them makes `section_row_fixture`'s tree mirror die with
  *"the source path is neither a regular file nor a symlink to a regular file"* — three gates
  red for a reason that has nothing to do with the parcel under test. The old worktree-seeding
  note that says a fresh aeon worktree needs them is STALE; `./build.sh` works without them.
  Ledgered: the mirror should skip or name a non-regular entry instead of `unwrap`ing.

  **Reconcile the total against the tree, not against the last remembered bar:**
  `git grep -c '#\[test\]' HEAD -- '*.rs'` summed approximates the declared count, and
  `passed + ignored` should land on it.
  **⚠ IT IS AN APPROXIMATION, NOT AN IDENTITY, AND THIS PARAGRAPH CLAIMED THE IDENTITY**
  *(measured 2026-09-02)*. The grep counts LINES, so a `#[test]` inside a `macro_rules!`
  body is counted once however many times the macro is invoked.
  `crates/sigil-harness/tests/freeze_step_gap.rs` is the instance and currently the only
  one (`git grep -ln 'macro_rules' HEAD -- '*/tests/*.rs'` returns exactly it): grep says
  **12**, the binary runs **21**, a 9-test undercount in one file.
  **And the residual is NOT explained, which is the honest state:** master's summed grep
  is 4233 while the item-5 branch measured 4230 passed + 2 ignored against a branch grep of
  4231 — a net discrepancy of 1, not 9, so something over-counts by 8 that this lane has
  not identified. Do not report the difference as "the macro file" until that is measured;
  one known error and an unexplained offset is two findings, not one.
  **What it is still good for, and why it stays:** it caught the wrong-tree landing run
  (3857 + 2 = 3859 against a declared 3823), and an error of single digits does not hide a
  36-test gap. Use it as a coarse tripwire — a difference of a few is unexplained
  bookkeeping, a difference of tens is a different tree. **The exact number is what the
  suite reports; the grep never overrides it.** Baseline arithmetic carried
  across branches measured on different reference trees does not reconcile and will
  invent a discrepancy that is not there. Never plain
  `cargo test`: without `--release` some gates are impractically slow, without
  `--workspace --no-fail-fast` a wedge or an early failure hides the rest of the
  result set. Report failures-first with explicit pass/fail counts; never
  `grep | head` test output (it buries FAILED lines).

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

**⚠ The lane's self-audit reads PROSE, and a doc comment used to take the whole lane
down.** Before running, the script classifies every `crates/*/tests/*.rs` matching
`AEON_DIR|aeon_dir|reference_tree|--aeon`, and exits `2` — the entire nightly backstop
dark, reporting nothing — if any file lands in none of its buckets. That grep cannot tell
a *use* from a *mention*: a test whose header says "takes no `AEON_DIR`" matches on the
disclaimer. Caught on `feat/version-provenance` before landing (replaying the audit gave
`unclassified=1 [version_provenance]` at the first delivery, `0` after); the fix there was
to describe those inputs without naming the identifiers, and to say why in the file so the
next author does not re-arm it — **still the right thing to do in a new file**, because a
file that stays out of the selected population needs no bucket at all.

**The buckets are now THREE, and the third is derived** (`fix/source-gate-third-bucket`,
2026-08-30 — see `docs/superpowers/notes/2026-08-30-source-gate-third-bucket.md`). A file
that names the reference tree without ever OBTAINING one is bucketed `no-reference`,
counted in the verdict, and is not a defect: this lane has nothing to run for it and the
workspace suite already runs it on every invocation. Membership is decided per file, from
content — *does it call an accessor that yields the tree, or read the environment variable
itself?* — and the accessor set is closed over `test_support.rs`'s own public functions
from the one that reads that variable, so a new accessor spelled there joins the rule with
no edit to the script. **It is not a roster**: the fifth file in this shape
(`reference_tree_named_write`, landed by a concurrent branch mid-parcel) was classified
correctly with no change to the rule. The question is asked only *after* the two
established ones, so it can speak for a file that used to fall through and for no other.
A rule that cannot be derived **refuses** — an unreadable `test_support.rs`, an
unextractable variable name, an empty accessor set each exit `2` naming what could not be
measured, because an empty accessor set would otherwise make every file look like it reads
nothing and the lane would go green over a population it never classified.

**`--audit` is the read-only way to ask.** It runs the classification alone against the
checkout the script lives in (or `$2`), creates no worktree, builds nothing, and never
reaches `note`, so it sends no notification. `crates/sigil-harness/tests/source_gate_classification.rs`
invokes it on every `cargo test --workspace`, which is what `scripts/landing-run.sh` and CI
run — so **a landing run now fails on an unclassified file**, and the second test there
reconciles the four bucket sizes against the scanned population so a classifier that
silently dropped files cannot report `unclassified=0` and be believed. Replaying the audit
by hand against a branch is still worth one loop; it is no longer the only thing that
looks.

**Left open, deliberately:** the SELECTOR is still any occurrence, and that is the safe
direction — narrowing it to code uses would let a genuinely aeon-reading gate escape the
audit entirely, which is the failure the audit exists to prevent. The third bucket does
not narrow it: every file the selector matches is still classified, and the ones that read
nothing are answered rather than dropped.

**Also left open, and it is the bigger question:** `SOURCE_GATES` is still a hand-kept run
list, and the three files that darkened the lane alongside `reference_tree_write_guard`
were genuine source gates nobody had added. The derived rule already computes the property
that decides it — *reads the tree, names no committed artifact* — so the lane could DERIVE
its run list and a new source-only gate would join automatically instead of refusing.
**That is not a parcel-local call.** It trades a refusal for an auto-enrolment: strictly
better than dark, but a third-shape gate (source inputs, oracle'd on a golden or on
`pins.rs`) that stopped naming its artifact would then join and be red through every
refreeze window, and nightly criticals nobody can clear are how a lane gets ignored. It
needs its own ruling; the mechanism to implement it is in place either way.

**Adjudicating a warn-tier firing.** A new firing goes into `CORPUS_OPEN_FINDINGS`
(`crates/sigil-cli/tests/warn_tier_corpus.rs`), **not** into `WARN_ID_BASELINE`. The
baseline admits an id everywhere and any number of times; the register pins
`(shape, id, file, symbol)` with a count, and requires an owner, an anchor and a kill
condition per row. Anchors are symbols and paths, never line numbers — a register entry
outlives what it points at, so a coordinate in one rots. A row leaves the register in
either direction: fixed, or ruled deliberate and promoted into the baseline citing the
ruling. Each row's age prints on every lane run.

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
- **⚠ ANY cargo command that lands in this checkout RELINKS `target/release/sigil`, which is the
  assembler another lane's freeze may be mid-ritual with.**
  **THE VARIABLE IS THE TARGET DIR, NOT THE VERB, and that is why the first two fixes missed.** The
  first wording said *"ad-hoc"*, which reads as a rule about careless one-off commands — but a
  COMMITTED ritual tool in a sibling repo, documented as mandatory before a freeze, derives its
  sigil tree from its own location, `cd`s into this shared checkout and runs `cargo test` with no
  `CARGO_TARGET_DIR`. **Somebody obeying the "ad-hoc" wording perfectly still relinks**, because
  they are not running an ad-hoc command. Meanwhile the two commands anyone thinks to guard —
  `landing-run.sh` and the provisioner — are precisely the two that were never the problem, because
  they set their own target dir. What does it is the casual one: `cargo test -p sigil-cli --test
  <x>`, `cargo run --bin repin`, `cargo build --bin emit_sound_blob`. **Testing a package builds
  that package's bins, so a targeted test of `sigil-cli` relinks `sigil`.**
  **AND THE RELINK DESTROYS EVIDENCE, NOT JUST STATE.** `target/release/sigil` is a single path, so
  a relink OVERWRITES IN PLACE: the binary that produced a frozen entry's goldens is simply gone,
  and the entry then names an assembler nobody can re-instantiate by inspection. **A provenance
  record can go un-reproducible without anything editing it** — the one failure mode a frozen
  artifact is supposed to be immune to.
  **Its second half is worse: a tool that resolves the tree from ITS OWN LOCATION tests whatever is
  at that path, not the thing it was invoked about.** That pre-flight has never tested the tree it
  gates, so every red it produced was a true report about the wrong subject. **"The gate was
  skipped" and "the gate ran and could not see the subject" produce identical evidence**, and the
  first is the story everyone reaches for.
  **So: while any lane holds a freeze, pass `CARGO_TARGET_DIR` on every cargo command in this
  checkout, not only on the scripted ones.** The relink is invisible from here — nothing in the
  output mentions the shared file, and only the far lane's md5 pin catches it. A guard keyed to a
  command name will keep missing this; the honest fix is a non-default target dir by default.
  *(The chain-198 instance, its md5s and timestamps, and the accident that made a byte control
  non-vacuous: `docs/OVERSEER-LOG.md`, 2026-09-03 cut, original lines 886-935.)*
- **A LANDING RUN AGAINST THE OWNER'S LIVE AEON TREE PRODUCES PHANTOM FAILURES, proved here the
  expensive way.** A full-suite run pointed at `/home/volence/sonic_hacks/aeon` returned an extra
  failure the parcel's own agent had not seen, in a gate that plausibly matched what the parcel had
  touched. It reproduced twice, including with the test binary run alone, and a control at
  unmodified master passed — which read as attribution. **It was transient**: four consecutive
  re-runs on the merged tree passed. Nothing was wrong with the parcel; the tree had artifacts in
  flux underneath a run that takes minutes. Cost: an hour and a nearly-shipped wrong attribution
  against an agent that had done nothing wrong.
  **The tell is STEADY-STATE DISAGREEMENT between two runs over the same tree — re-run before
  attributing**, and prefer a clean worktree of a committed SHA, which is what the next rule already
  says. *(Full episode: `docs/OVERSEER-LOG.md`, 2026-09-03 cut, original lines 936-949.)*
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

### STANDING ARTIFACTS THIS LANE DEPENDS ON — declared here so a SWEEPING lane can find them

**A declaration only its author reads is not a declaration.** On 2026-08-27 the aeon lane
swept `~/sonic_hacks/` for merged, detached, branchless worktrees and removed eleven. One was
this lane's standing reference tree. It was an **aeon** worktree by construction — it holds
aeon shapes — so it matched every mechanical criterion, and **the only thing that would have
distinguished it was a line in this file, which a sweep of aeon's worktree list cannot see.**
*Ownership is not conferred by registration* (aeon's formulation): being in a repo's worktree
list is a fact about bookkeeping, not about who depends on it.

**THE PINNED ASSEMBLER `~/sonic_hacks/.pinned/`.** A second standing artifact, requested
permanently by the AEON lane 2026-09-04: the sigil binary at `0a58f2ec`, copied out of the
shared `target/release/sigil` before that path could be relinked. **It is the original, not a
rebuild** — a fresh build of the same revision is a different artifact answering a slightly
different question. Two aeon landings pin it by revision AND md5, and a pin whose referent has
been deleted is a citation to nothing; this repo has already lost one chain's assembler that
way. File and directory are both read-only, verified by observing a same-device `mv` refuse.
**It ends when the aeon lane says so — ask them, not this file.** The reason, the md5 and the
lift command live in `README-STANDING-ARTIFACT.txt` beside it, where whoever trips over it will
be standing.

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

### `AEON_DIR` IN EVERY BRIEF: UNCONDITIONAL, **EXCLUSIVE**, and **PREPARED**

`test_support.rs::aeon_dir` defaults to the owner's live checkout, and `sigil build --aeon <tree>`
**writes into** that tree rather than only reading it. A default pointing at a person's working
directory must be overridden *every* time, and every-time is what briefs are bad at. **Set it in
the template, not per parcel** — *"a stable aeon tree, if needed"* is the whole defect wearing
helpful flexibility.

Unconditional is necessary and **not sufficient**. Two further claims must be stated or they are
silently relied on:

- **EXCLUSIVE** — `build.sh` rebuilds `rm`-first, so two agents sharing one tree transiently delete
  each other's reference ROMs. It surfaces as one failure in an otherwise clean run, green seconds
  later. **The gate was right both times; the tree moved under it.**
- **PREPARED** — a fresh `git worktree add --detach` of aeon is source with **no reference ROMs**
  (measured: 213 failures on first contact, all `reference missing`). A bare tree is not a reference
  tree, it is a **build job**, and the agent has no choice but to write into the thing the brief
  calls a reference.

**The template line:** prepare one tree per agent yourself (detached at the provenance `aeon_rev`,
all four shapes built), then say in the brief **which** it is — prepared-and-exclusive, or
bare-and-yours-to-build. The agent cannot tell by looking.

**The reading lesson, which outlives the fix: *a path is not a state.*** `AEON_DIR` names a
location; every claim that matters is about the **condition** of what is there.

**Watch the perturbation direction.** A shared tree manufactures a false **green** as easily as a
false red — a gate that should have refused, finding a stale ROM another agent built. Red is this
hazard's visible face and green its invisible one, so *"it passed on a re-run"* closes the instance
and says nothing about the class.

*(The two collisions, their measured failure counts, and "knowing a rule and encoding it in the
instruction that carries it are separate acts": `docs/OVERSEER-LOG.md`, 2026-09-03 cut, original
lines 1038-1086.)*
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
processor. Grounds are this lane's own `d-18` R4: a run that reports what it skipped **still
exits 0**, and a silent green is the class we never drop. Amends the hub's first form (empyrean
`802fdee`), whose acceptance named an asl oracle — episode in `docs/OVERSEER-LOG.md`,
2026-09-03. The RULING stands on the `d-18` grounds above and needs no oracle. **Pinning AS as a
dependency of the project that exists to replace it is a real cost and is the OWNER'S
alternative, not taken.**

**⚠ THE ORACLE EXISTS, AND THIS PARAGRAPH SAID IT DID NOT — corrected 2026-09-03 after four
parcels had been run without it.** A working **`asl` 1.42 Beta Bld 212** is committed in the
corpus repo at `s2disasm/build_tools/Linux-x86_64/asl` (with `as.msg` beside it, and `p2bin` /
`saxman`). It runs. It is a **differential oracle for the entire AS-replacement project** — the
name-composition parcel used `asl -L` for every expected value in every new test, which is why
that delivery could quote listings instead of asserting semantics. **Use it.**

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

**BOOKED — `landing-run.sh` does not run clippy.** See the lane-log entry at `526ae99e`. Two
lint errors stood on master and the wrapper printed `RESULT GREEN` over them, because the bar
lists `clippy -D warnings exit 0` and the script contains no clippy invocation. Hand-running it
is the same maintenance model that failed, so the fix is wiring it into the wrapper's own
command span with its exit code in the verdict block — **and `--all-targets`**, since one of the
two errors was reachable only through test targets.

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
bug and `[layout.odd-field]` are the same shape one layer out, already banked above.

**COMPARE THE SETS, NOT THE TOTALS — a summary statistic answers a question nobody asked.**
Two populations can differ by one member in each direction and total identically, so a count
difference is consistent with a change it cannot see. Measured instance (2026-09-04, jointly with
the aeon lane): across five aeon trees spanning 155 commits, the debug listing carries exactly one
more equate than release — but the assertion worth gating is not `debug = release + 1`, it is
**`release` is a strict SUBSET of `debug`**, reverse set empty. A future release-only equate is a
genuine anomaly, and a count-difference check passes straight through it. The same shape is why
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
