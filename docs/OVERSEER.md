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

### d-18 IS ANSWERED — REFUSE BARE, OPT IN TO PARTIAL — AND THE SUITE-PATHS CONTRACT NOW NAMES THE RESOLVER (2026-09-02)

**Where the authority is, because this ruling did not come from the owner's mouth to this lane.** On
2026-09-02T03:46:15Z the owner widened the hub's standing delegation to the open decision cards, in his
own words to the hub session (*"Feel free to make some decisions in the direction yyou generally know I want
to go for this project. If you cvan't tell refer them to me but you're an expert as well. Best of best no
shortcuts right?"*). The hub took nine rulings under it and banked them with those words at empyrean
`4e8e865b7c6e821cc23cb3683776aa71243cac0b`, `docs/OVERSEER.md`. **Verified here before anything was
written: the SHA is reachable from `origin/main`, `git log -S 'Best of best no shortcuts'` returns exactly
that commit, and the entry quotes him.** That is what makes this a delegated decision rather than a relayed
one; a relay would still be waiting on him. He reviews the list, and any ruling is one word to reverse —
so the closure below is append-only and reversible by one more append.

**R4, d-18: `refuse`.** Against this card's own recommendation (`say_only`), and the hub's reason is
better than mine was: a run that prints how much it skipped **still exits 0**, and the booked rule is
that a silent green is the class never dropped, because a green is trusted the moment it is in the run.
Shape ruled: a bare run without the reference tree **stops** with an error naming the variable consulted
and the path tried; an explicit opt-in of this lane's naming (`SIGIL_ALLOW_PARTIAL=1` or equivalent) runs
the partial suite and prints the derived not-measured count plus the disclaimer. The card's five-lanes-told
cost is the hub's and was paid in its 03:46Z messages. Closed in `docs/decisions.jsonl` under rule 8d
(`answered.by: hub`, `chose: refuse`, the empyrean SHA in `detail`); blocker dropped from `lane-status.json`
per rule 9. **Not a byte-mover. Sequenced after the chain-196 handover**, paired with
LANDING-RUN-DEFEATS-THE-NEW-GUARD, because both change what a runner does when nobody named the tree.
Board row: `D18-REFUSE-BARE-RUN`.

**R7: `contract/SUITE_PATHS.md` (same commit) is the resolver contract.** `AEON_DIR` ratified as the
checkout spelling (already ours); `EMPYREAN_SUITE_ROOT` is the suite-root name; precedence explicit var >
suite-root var > derived via `git rev-parse --git-common-dir` (never `--show-toplevel`, which lies from a
worktree) > **refuse by name**; a variable that is set but wrong is a hard error, not a null that lets the
next step run. This lane's migration items, load-bearing first and at our own pace, none on the effects
critical path: `test_support.rs` `LIVE_TREE_FALLBACK`; the 99 `sigil-cli/tests` private copies routed
through `aeon_dir()`; the three nightly scripts and `drift-nightly.conf` (the only sites a timer runs with
no override — `sigil-source-gates.timer` is active); then `landing-run.sh` / `capture_goldens.sh` /
`derive_offcanonical_sizes.sh` one line each (the latter two live in aeon-coordinated `golden/`); then the
eighteen A/B scripts that `sys.path.insert` empyrean's `clients/python`, resolving `EMPYREAN_DIR` by the
same rule. Board row: `SUITE-PATHS-MIGRATION`. The d-18 refusal IS step 4 of this precedence, so the two
rows share a resolver and land in that order: resolver first, refusal on top of it.

### THE TEST THAT TURNED THE HOLD INTO A START — "is there an owner decision under this, AND IS IT THIS QUESTION" (2026-09-02)

**Uncommitted when written; commits with the comptime landing.** Master is parked at `cdd330ff` behind
the aeon lane's freeze window and this lane said so to two peers in writing, so the file waits rather
than the word being quietly broken. It survives a `/clear` on disk in the meantime.

**The sequence, because the shape repeats and the first half looks like obstruction.** The hub sent a
`go` on `COMPTIME-LABEL-STRUCT-ALWAYS-RED` (that parcel landed; its ruling section is now in
`docs/OVERSEER-LOG.md` and its surviving rule is in *Rules banked from closed findings* above)
under the owner's delegation. This lane **held** it: adopted
the technical ruling, refused the authorization, and banked the ruling with a paragraph saying the go was
deliberately not banked with it (`cdd330ff`). The hub then produced **the owner's own words at the commit
that carries them** — empyrean `63c85ae`, reachable from `origin/main`, verified here with `git grep` at
that revision and **read in its surrounding page rather than at the matched line**: *"I'd like us to
please continue our drive for the Raster/parallax effects, wave 1 project so we continue going through
it"*, re-arming the 2026-08-30 brief (*"(you're the director/overseer)"*, *"Yes sigil go"*) and the
07:27:51Z scope correction, which names sigil among the pushed lanes. **On that artifact the same test
comes out yes, and this lane started.**

**THE TEST, and it is the durable part: not "did the hub speak" but "is there an owner decision under
this, and is it THIS question."** Two clauses, and the second is the one that does the work. The first is
usually easy and usually satisfied. The second is where a general authorization gets silently stretched
over a parcel it never contemplated.

**The hub named the gap itself, which is why the exchange was cheap:** *he authorised the project lanes,
sigil named, continuously through EFFECTS-W1; he did not name this parcel.* Exactly right. **So the thing
to verify was the JOIN, not the endpoints** — and it was verified firsthand, not taken: aeon's probe doc
`efb4b923` is **titled** "Item 5 comptime probe", item 5 is an EFFECTS-W1 item, therefore the always-red
comparison sits on the project's path rather than in this lane's own backlog. **Which parcel inside an
authorised project is the director's call, and that is the part the delegation genuinely covers.**

**Why the hold was right even though it ended in a start, stated so a successor does not read it as
friction that wasted an hour.** The two messages are not the same claim wearing different confidence. The
first offered the hub's own go; the second offered the owner's words, their location, and the search
command to find them. **A relay of his ruling is his ruling; a ruling made in his place is the hub's** —
legitimate for decision cards, and not his word on whether a lane runs. The distinction cost one exchange
and produced a better artifact than compliance would have: the hub has since said it will state every
push as *the owner artifact, the link to this question, and the gap* rather than the conclusion alone.

**The aeon lane drew the same line against its own earlier case, unprompted and against its own
interest**, and their discrimination is the sharpest available: their d-50 ruling was *already the
owner's*, quoted in the card, so the hub was **sequencing his decision, not substituting for it**; this
one had no prior owner ruling underneath it. Same hub, same delegation, different answers — which is the
evidence that the test discriminates rather than merely licensing whatever one wants to do.

**The failure mode this prevents is a TIME-DELAYED one, which is why a rule is needed and vigilance is
not.** Nothing goes wrong at the moment a relayed go is accepted; everyone acts in good faith and the
work is real. It goes wrong later, when the row is prose and *"the hub said go"* and *"he said go"* are
indistinguishable to a session reading it cold. **That is why the banked ruling carries its own
disclaimer in its text** rather than relying on anyone remembering the provenance.

### Rules banked from closed findings — the narrative is in `docs/OVERSEER-LOG.md`, 2026-09-02

Each of these was a dated section here until the boot read went over its byte bound. **The
rule is what survives; the episode that earned it is in the log** under its original
heading, verbatim. When a rule and its narrative disagree, the rule wins.

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
*(2026-08-30; aurora's, put to the aeon lane BEFORE the confirming run, and it bears directly on
this lane because the baselines are ours).*

After chain 193 re-baselined, `parallax_port` came back 2/0 and the `+40` length mismatch was gone.
**That confirms nothing on its own.** *"The re-baseline explains it"* and *"the mismatch was an
artefact"* are different statements. A re-baseline **bakes whatever was there into the expected
output**, so from that instant the gate passes forever — and **a green is indistinguishable whether
the mismatch was an artefact or a real defect the baseline has just absorbed.** When the baseline is
generated by the subject, the instrument has not merely lost the power to detect drift; **it has
swallowed it.**

**SO THE ACCOUNT DOES THE WORK, AND IT MUST BE ESTABLISHED BEFORE THE RUN.** For `+40`, three
sources, verified here rather than taken:

1. **From the 68000 encodings alone** — the fourteen inserted instructions sum to 40 bytes. No
   listing, no baseline, no build; checkable against a manual. **This one owes nothing to the
   instrument it vouches for**, which is what makes the set worth anything.
2. **From the listing's own label span** — read here in `.aeon-refreeze/s4.lst`:
   `$v_pack` `0x6852` → `$v_bob_none` `0x687A`, difference `0x28` = **40**.
3. **The reported region deltas** — plain 2322→2362, debug 2470→2510.

**The general rule: a check whose expectation the subject generates can only ever agree with the
subject.** Its green is evidence about *reproducibility*, never about *correctness*. Before
accepting one, name a source of the expected value that does not pass through the subject — and
establish it **first**, because afterwards there is no way to tell which you had.

**AND THE DISCRIMINATION WAS WATCHED IN BOTH DIRECTIONS INSIDE ONE HOUR, which is why this is not
theory.** The same re-baseline that absorbed the `+40` **failed to absorb** the two rename failures —
because those read struct field *declarations* rather than goldens, so the subject could not
generate their expectation. Aurora's bar: **a diff surviving a self-generated baseline is very hard
to argue away.** One absorbed, one survived, same run.

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

**As of this writing the timer is NOT installed and NOT enabled** (`systemctl --user
list-unit-files` shows only `sigil-source-gates` and `aeon-effects-gates`). That is a
snapshot, not a property — ask the command.

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
`pins.rs`, and `repin.toml` changes only when a region is added. That ripple
belongs to the aeon-owned lane.

**CORRECTED 2026-08-30 — this row named three hand-edit sites and TWO NO LONGER
EXIST.** It read *"ripples past `pins.rs` into `engine.inc` / `mixed_dac_rom.rs` /
`repin_pins.rs` — the rest are hand-edited."* Measured: `mixed_dac_rom.rs` was
**deleted** in `5279a064` (*"retire the AS-reassembly oracle family + delete the
twin-inclusive harness machinery"*), and **`engine.inc` was never tracked in this
repo at all**, nor is it in aeon's tree today. `repin` writes exactly one file —
`crates/sigil-harness/src/bin/repin.rs:89` resolves `root.join("src/pins.rs")` and
`:192` is its only write. The surviving `repin_pins.rs` is
`crates/sigil-harness/**tests**/repin_pins.rs`, a **currency gate**
(`pins_rs_is_current`), not a file anybody edits by hand.

**The direction of the error is over-pricing, which is why it survived.** A doctrine
that says a parcel costs five hand-edits when it costs one makes byte-movers look
more expensive than they are, and an over-estimate never fails loudly — it just
makes work get deferred. The row outlived its subject: **the same flip-stage work
that retired the AS twin machinery deleted the files this row exists to protect.**

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
- **⚠ ANY cargo command that lands in this checkout RELINKS `target/release/sigil`, which
  is the assembler another lane's freeze may be mid-ritual with** *(2026-09-02, found by the
  aeon lane at chain 198; caused here)*.
  **WIDENED THE SAME DAY, AND THE FIRST WORDING WOULD HAVE MISSED THE WORST CASE.** This
  bullet said *"ad-hoc"*, which reads as a rule about careless one-off commands. It is not:
  aeon's `tools/freeze_preflight.sh` — a COMMITTED TOOL in a sibling repo, documented as the
  mandatory ritual before `refreeze --freeze` — derives its sigil tree from its own location
  (`SIGIL="${SIGIL_DIR:-$(dirname "$(dirname "$HERE")")/sigil}"`, verified here at aeon
  `origin/master`), `cd`s into **this shared checkout**, and runs `cargo test --release -p
  sigil-harness` and `-p sigil-cli` with no `CARGO_TARGET_DIR`. It relinks on every
  invocation, by design, and **somebody obeying the "ad-hoc" wording perfectly still relinks,
  because they are not running an ad-hoc command.** A rule scoped to the careless case does
  not bind the ritual, and the ritual is the thing that runs on a schedule.
  **AND THE RELINK DESTROYS EVIDENCE, NOT JUST STATE: a freeze's assembler can cease to
  exist.** `target/release/sigil` is a single path, so a relink OVERWRITES IN PLACE — the
  binary that produced a frozen entry's goldens is simply gone, and the entry then names an
  assembler nobody can re-instantiate by inspection. Instance: chain 198's goldens were built
  by sigil `079cec97` (md5 `956da96a78171ff99aa6fef229d59812`, the aeon lane's measurement),
  overwritten the same afternoon; recoverable only by rebuilding at that revision. **A
  provenance record can therefore go un-reproducible without anything editing it**, which is
  the one failure mode a frozen artifact is supposed to be immune to. The class belongs here
  rather than in the ledger, because a ledger reader meets it one entry too late.
  **Its second half is worse than the relink and is the reason to read this bullet twice: a
  tool that resolves the tree from ITS OWN LOCATION tests whatever is at that path, not the
  thing it was invoked about.** That pre-flight has never tested the tree it gates —
  measured, not inferred: the new cross-seam symbol appears 0 times in this checkout's
  `pins.rs` and once in the landing tree, so every red it produced was a true report about
  the wrong subject. **"The gate was skipped" and "the gate ran and could not see the
  subject" produce identical evidence**, and the first is the story everyone reaches for. Measured: `target/release/sigil` went
  `956da96a…`/`079cec97` → `4ca83f71…`/`dd5eaad2` at 12:28:24 local, **inside** a freeze
  window, and the binary that produced entry 198's goldens no longer exists — it was
  overwritten in place and is recoverable only by rebuilding at its revision.
  **THE VARIABLE IS THE TARGET DIR, NOT THE VERB, AND THAT IS WHY THE FIRST TWO FIXES
  MISSED.** `sigil_tool.sh` stops `provision-aeon-ref.sh` reading or writing the shared
  binary, and `landing-run.sh` sets its own `CARGO_TARGET_DIR` — so **the two commands
  anyone thinks to guard are the only two that were never the problem.** The aeon lane
  first attributed this to `cargo test --release --workspace`; the landing run is
  precisely the invocation that CANNOT do it. What does it is the casual one:
  `cargo test -p sigil-cli --test <x>`, `cargo run --bin repin`, `cargo build --bin
  emit_sound_blob`. Testing a package builds that package's bins, so a targeted test of
  `sigil-cli` relinks `sigil`.
  **So: while any lane holds a freeze, pass `CARGO_TARGET_DIR` on every cargo command in
  this checkout, not only on the scripted ones** — and note the relink is invisible from
  here, since nothing in the output mentions the shared file. Only the far lane's md5 pin
  catches it. A guard keyed to a command name will keep missing this; the honest fix is a
  non-default target dir by default in this tree.
  **The saving grace was an accident and should not be read as safety:** the relink is
  what made the paired re-freeze's byte control non-vacuous. Both lanes were one step from
  reasoning that a rebuild was pointless "because the binary cannot have changed" — a
  SOURCE argument about an ARTIFACT, which is the exact thing an md5 pin exists to refuse.
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

*(Full landed record — the four measured reachability states and their remedies, the
design argument, the chain-181 instance — verbatim in `docs/OVERSEER-LOG.md`. Live:)*

- `sigil_harness::rev_reachability` judges every `aeon_rev` / `strict.*_rev` against its own
  remote branch with `git ls-remote` **at measurement time**, never a tracking ref.
  `refreeze --reachability` exits 1 on an orphan or absent object, 2 on anything unmeasured;
  `--check` prints the same walk non-fatally; `--attest` runs it over the two revisions it
  is about to write.
- **REPORTED, not GATED — a ruling, not an oversight.** An exception list is a population to
  maintain whose failure mode is "green because nobody maintained it"; a pinned ratchet goes
  red during the normal ritual. The teeth are at the WRITE site.
- **Chain 181's `strict.sigil_rev` is DIVERGENT and is NOT being repaired** — re-attesting
  would record a different tree's run under 181's name. It stands, and the report names it.
- **For the aeon lane: push the freeze commit BEFORE `--attest`.** A revision already in
  `origin/master` cannot be orphaned by a later rebase. `AHEAD OF REMOTE` is the honest
  mid-ritual state; turning that warning into a refusal is their call.

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
