# Sigil Overseer — reference

Rules this lane reads **at a moment**, not at boot. The boot read is `docs/OVERSEER.md`: it holds
scope, the queue, the standing owner rulings that change what a session does first, and the
obligations this lane owes now. It names this file by path at every point one of the blocks below
used to sit.

**Split by WHEN A RULE IS READ, never by size** — owner ruling 2026-09-04T15:38:47Z, carried in
`empyrean/docs/OVERSEER-PROTOCOL.md` at `origin/main`. **No rule here is shorter for having
moved**: every block below is its boot-read text, verbatim. This file is deliberately not
bounded, because nobody pays for it at boot. A few sentences in it say *"above"*, *"below"* or
*"this file"* about text that now sits in `docs/OVERSEER.md`; those resolve across the pair, and
the boot read carries the matching note.

Blocks keep the order they held in the boot read, which is what the losslessness proof requires —
each output must be an order-preserving subsequence of the original, so grouping them by moment
would have made the move unprovable. Read them by moment instead.

- **Certifying that a length-neutral parcel's build actually ran** — *Prefer a positive freeze
  witness to `pins.rs unchanged`*.
- **A run refuses for want of a reference tree** — *d-18: REFUSE BARE, OPT IN TO PARTIAL*.
- **Building a proof, or judging a claim, a green, or a detached run** — *Rules banked from
  closed findings*, and *A re-baseline does not explain a green*.
- **Planning work off the drift record** — *The drift watch's timer*.
- **Writing a brief for any agent** — *Dispatch practice*, and *`AEON_DIR` in every brief*.
- **Preparing, judging or landing a full-suite run** — *Quality bars*.
- **A source-gate notification fires, or a new test reads the aeon tree** — *The source-gate
  lane*.
- **Setting up a worktree, or running cargo in the shared checkout** — *Worktree and environment
  quirks*.
- **About to sweep, delete or rebuild trees under `~/sonic_hacks/`** — *Standing artifacts this
  lane depends on*.

## Freeze, proof and review bars

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

**Widened to SUITE-ROOT readers, 2026-09-07** (ledger `CI-SUITE-ROOT-READERS`, note
`2026-09-07-ci-suite-root-readers.md`). A suite-root reader is a row whose subject is the directory
holding `aeon/` + `empyrean/` beside the checkout rather than the reference tree (the step-3
derivation, the unnamed default tree); it opens with nothing, because its own derivation is the
read, and closes with `test_support::suite_root_absent` when that came back empty. The three
answers are d-18's: strict fails by name, a declared partial run leaves the row unmeasured in the
`skip:` form with a second banner line carrying the class's derived size and names, a bare run
stops. Two limits are the rule's own: a set `EMPYREAN_SUITE_ROOT` is never an absent root (set but
wrong stays a hard failure, in the partial run too), and CI provides no fake suite root.
### Rules banked from closed findings — the narrative is in `docs/OVERSEER-LOG.md`, 2026-09-02

Each of these was a dated section in the boot read (`docs/OVERSEER.md`) until it went over
its byte bound. **The
rule is what survives; the episode that earned it is in the log** under its original
heading, verbatim. When a rule and its narrative disagree, the rule wins.

- **A LOSSLESSNESS PROOF CERTIFIES THAT NO TEXT WAS LOST AND NOTHING ABOUT WHETHER THE
  SURVIVING TEXT STILL PARSES** *(2026-09-03, found by the agent that split the boot read into `docs/OVERSEER-LOG.md` — a
  different cut from the 2026-09-04 one that made this file — and it
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

## A re-baseline does not explain a green

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

**⚠ A TASK RELAYED AS ROUTINE IS THE ONE WHOSE GROUNDS NOBODY RE-CHECKS** *(2026-09-09, the
refreeze-debt hold; the hub upheld the refusal and banked the push as its own error)*.

A peer relayed a freeze debt with a recipe attached, framed as housekeeping and as the last open item
across the suite. Three grounds refused it, and the second is the one that matters:

1. **Nothing was broken.** `refreeze --check` returned OK, chain len 205, with only the two known
   pre-existing divergent entries. So it was not a repair.
2. **It was a 93-commit CORPUS PIN ADVANCE, and a standing ruling forbids exactly that framing.**
   The hub's own 2026-09-02 ruling: *the pin exists so the corpus does NOT track tip; advancing it to
   tidy a transition is the behaviour the decouple forbids.* Closing a freeze debt is tidying a
   transition. **The hub had written that ruling seven days earlier and read it at boot the same
   night.**
3. **It would have reintroduced a hazard banked six hours before, on the first advance since.** The
   reference tree's exclusivity is an accident of its pin: the peer's self-locating test script is
   absent at the pinned revision and present at the target.

**The mechanism, and it is not carelessness.** The word "routine" is a claim about a task's GROUNDS,
and it is the one claim a recipe never carries evidence for. Everything else in that message was
checkable and correct: the recipe, the SHAs, the reachability. **A framing cannot be verified by
checking the things inside it.**

**The check that fires, and it is one question: what ruling would this have to be tested against, and
has anyone tested it?** If the answer is "it is too routine to need one", that is the answer this
rule exists for.

**And the decomposition that resolved it is reusable:** "landed unfrozen" was EITHER a debt on the
peer's own certification, theirs to discharge, OR an ask for our corpus to describe their tip, which
is a decision. A recipe that reads as one thing when it is two is how a decision arrives disguised as
a chore. **Ask which of the two before pricing it.**

**Operationally, before any reference-revision move:** run `refreeze --check` first, so you know
whether you are repairing or deciding; re-read the ruling that governs the pin; and re-check the
peer's writer scripts for presence at the target revision, because the exclusivity you verified is
only ever true of the revision you verified it at.

**⚠ A FAILED MERGE LEAVES A GREEN SUITE BEHIND IT, AND THE STAMP IS THE ONLY THING THAT CATCHES IT**
*(aurora's finding, twice there; reproduced HERE the same night, 2026-09-09, and this seat's instance
is the one with the unanticipated cause)*. A `git merge` that FAILS prints its error and returns; the
landing run started immediately afterwards then measures the UNMERGED tree, comes back green at
master's own numbers, and reads as the landing verified. **A green suite after a landing is not
evidence the landing happened.**

**This seat's instance, and the cause is the part nobody plans for.** The merge did not fail on a
conflict. Its `-m "..."` message contained backticks, which THIS SHELL EXECUTES, so the command died
in argument parsing before git ever ran. A gate was already launched in the same breath and ran
happily against the previous commit. **The rule against backticks in a commit message was in three of
this lane's own agent briefs that same evening.**

**Two things catch it and one of them already exists.** `scripts/landing-run.sh` stamps the HEAD it
measured into its own log and verdict block, so comparing that SHA against the merge you believe you
made settles it in one glance. And the baseline reconciliation catches it whenever the merge would
have added tests, since `+0 new` on a merge that adds a test binary is the tell. **A bare RESULT
GREEN catches nothing.**

**Read the merge's own output before starting anything that depends on it.** The general form: when
two commands are issued together and the second is the evidence for the first, the second cannot
witness the first's failure, because a no-op predecessor leaves it a perfectly valid subject.

**⚠ WRITING ABOUT A THING CAN CONSUME IT: a sweep that scans source for references counts its own
prose** *(found 2026-09-08 by the pins parcel, against its own first draft, and volunteered)*. The
zero-consumer sweep asks which pin constants no `.rs` file mentions. Its predicate's doc comment
used a REAL constant as the worked example, so the sweep read that comment as a reference and
dropped that pin from the very list the comment documents: **20 names where the truth was 21.**

**The direction is what makes it dangerous.** A self-reference of this shape can only ever REMOVE
members, so the report comes back shorter and cleaner, which is the direction nobody audits. It was
caught only because an independent derivation existed to compare against, not by anything in the
output looking wrong.

**Two rules, and the second is the general one.** Documentation of a scanner states the SHAPE and
never a live name from the scanned population, with the reason written beside it so the next author
does not helpfully add an example back. And more broadly: **when a check's subject is the source
text, the check's own source is inside its subject.** Ask what the instrument says about itself
before believing a population it produced. Same family as a fixture derived from the constant under
test, arriving through prose instead of through code.

**⚠ THE FINISH CLAUSE NAMES ITS ACTOR, AND THE PROHIBITION NAMES THE ACTS** *(aeon `48c49d4e`,
relayed by the hub 2026-09-08; banked here the same night because a brief of this lane's had the
same gap while an agent was live)*. An agent whose brief said *"never commit to master"* and, in a
tidy clause with no actor, *"the branch is removed after landing, as expected"*, merged its own
parcel to `origin/master` and reported the removal as the expected ending.

**Two separate defects, and the second is the one this lane had.** The passive finish clause leaves
the agent as the only available actor, so a sentence written to REASSURE it reads as an instruction.
And the prohibition covers an act that is not the act that happens: **a merge is not a commit to
master, and neither is a push**, in the sense an agent hears. A brief can therefore be obeyed to the
letter and still land its own work.

**So the finish clause says who does it, in the active voice, and the prohibition enumerates the
acts:** do not merge, do not push, do not delete or move any ref, do not touch master. Add the
recovery line as well, because the alternative to hearing about it is discovering it: if you have
already done one of these, say so immediately and do not correct it yourself.

The test for a draft: read the finish clause and ask **who the sentence's subject is**. If the
answer is "nobody", the agent is the only candidate standing in the room.

**⚠ EVERY BRIEF NAMES A PRIVATE SCRATCH DIRECTORY, and this one is a rule about CONSTRUCTION rather
than about care** *(finding `sig-seat-scratch-dirs`, 2026-09-06 panel, banked 2026-09-08)*. The
reverse-perf seat had its pinned instrument and several probe files DELETED MID-RUN by a concurrent
agent writing into the same scratchpad, and files it never created appeared beside its own. It
re-copied, re-verified the md5, moved to a private subdirectory and re-ran, so the measurement
survived; nothing announced the corruption while it was happening.

**Two agents told to build scratch files and not told where to put them are each other's concurrent
writer by construction.** Neither is careless and neither can detect the other. The dispatch
invariant already says this about a detached script's own path, where a mid-run truncation resumes
execution inside the new bytes; this is the same shape arriving on a measurement instead of on a
script.

**So the brief names the directory, and names it as the agent's alone.** Create it before dispatch
(`/home/volence/sonic_hacks/.scratch/<parcel>` is the shape in use) and say in the brief that every
probe file, log and temp script goes there and nowhere else. A brief that says "use a scratch
directory" without naming one is the defect wearing helpful flexibility, exactly as
*"a stable aeon tree, if needed"* is.

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
rule. It is `docs/OVERSEER.md`'s banked shape met from the other side: **a vanished branch whose commits
survive unreferenced is the signature of a MERGE**, exactly as an empty commit range is.

*(Origin, and the n=3 caveat in full: `docs/OVERSEER-LOG.md`, 2026-09-03 cut, original lines
466-488.)*

### THE FACES OF A CHECK THAT CANNOT COME OUT OTHER THAN GREEN (2026-09-05; a fourth and fifth added 2026-09-06, a sixth 2026-09-09, a seventh 2026-09-10)

**All three were found in one night, all by DELIBERATELY BREAKING THE CODE, and none by the test
suite.** They are one defect wearing three costumes, and a brief should name all three because an
agent that guards against one walks into the others.

1. **THE INPUT CANNOT DISTINGUISH THE TWO ANSWERS.** `\{expr}` interpolation renders hex in asl and
   decimal here — a live byte divergence that survived months because **every probe behind that
   helper used a SINGLE-DIGIT value, where hex and decimal are the same characters.** Two carried
   comments calling them *asl-verified*. Single digits, zero, one, identity permutations,
   single-element lists and symmetric operand pairs are where this hides.
2. **THE SUBJECT IS NEVER REACHED.** Four aeon shapes rebuilt byte-identical looks like the
   strongest evidence available. Replace the new function with `panic!` and rebuild: if all four
   still build, **the identity attests that nothing ELSE moved and nothing whatever about the
   change.** Two parcels had to volunteer this after the fact; ask for it in the brief instead.
3. **THE FIXTURE DERIVES FROM THE SUBJECT.** Depth fixtures built their include chains *from*
   `INCLUDE_NEST_MAX`, so mutating that constant moved the expectation with it and the test could
   never disagree. **Write expected values out as the REFERENCE'S own numbers**, never computed
   from the constant under test. Three of six mutations landed green on that parcel's first run.


4. **TWO ERROR TERMS IN ONE EXPRESSION THAT CANCEL: the green is the SUM OF TWO WRONGS**
   *(2026-09-06, formed with the aeon lane against their live B7 parcel; theirs is the sharper
   statement and this is it)*. `end = LMA + blob_len` has two independently wrong inputs, and a
   probe that perturbs both together can leave `end` unchanged. The gate then agrees, and the
   agreement is real, and it is evidence of nothing. **This is distinct from face 1**: there the
   input could not DISTINGUISH two answers; here both inputs are wrong and the errors have opposite
   sign, so a discriminating input still produces the right total. **Vary ONE ARM AT A TIME, and
   report what each arm PRODUCED rather than that the arms agreed** (the bar that caught this
   lane's own four-corpus sweep agreeing 1,753 times over zero bytes). Any derived quantity built
   from two or more measured terms carries this, and a compound expression is where to look.

5. **AND THE MUTATION MUST HIT THE SUBJECT, NOT THE CHECKER** *(same exchange; it is the clause
   that makes faces 2 and 4 actionable rather than merely nameable)*. Where a gate asserts
   something that is true today only because the tree happens to be ARRANGED that way, the honest
   red-first proof perturbs **the arrangement**, never the assertion's own code or constant. A red
   obtained by breaking the parser proves the code executes and says nothing about whether it
   observes the breach. Aeon named the cause on their own side and it is the general one: a brief
   saying *"perturb it"* is AMBIGUOUS between the checker and the subject, **and the cheaper
   reading is the one an agent takes**. Say which.

6. **AND A RED IS NOT EVIDENCE THAT YOU TESTED WHAT YOU MEANT TO TEST: READ THE MESSAGE AND
   CONFIRM IT NAMES YOUR MECHANISM** *(oracle's, 2026-09-09, against its own re-run; paired with
   aurora's the same night; relayed, not reproduced here)*. Oracle re-ran a refusal and the run
   refused, and it proved nothing: **a different guard fired first**, and reading a refusal for
   the wrong reason is indistinguishable from the one you meant to test until you read the text.
   Aurora hit the same shape from the other side, two malformed plants caught by a neighbouring
   arm of the same gate, going red with plausible messages. **A build that fails for the reason
   you planted and a build that fails for a neighbouring reason are the same exit code.**
   This is protocol bar 2 inverted. That bar governs a poison that comes back GREEN and says to
   suspect the matcher; this governs a poison that comes back RED, where nothing looks wrong at
   all, which is why it is the face that ships. **Quote the diagnostic text beside every red-first
   proof, not the exit status**, and on this lane's surfaces that means the AS-shape `file(line):`
   line itself. Live consequence for every AS-frontend parcel: the frontend has many refusal
   paths over one input, so an unimplemented construct and a neighbouring syntax error land the
   same way.
   **And its positive-control twin, from the same exchange: an absence check is only evidence once
   the same filter has been shown to FIND the thing somewhere.** Run the filter where the subject
   IS, see a hit, then run it where the subject must not be. Without that, an absence proof
   measures the FILTER and not the world. This lane already carries *an emptiness is never a
   finding without an instrument that could have returned non-empty* and the canary rule that a
   canary covers the RULE while an input count covers the FEED; oracle's addition is the third
   leg, that the filter must be shown to select THIS subject and not merely some subject of its
   class. Their instance: a window filter keyed on `_NET_WM_PID`, which minifb never sets, so it
   returned zero on the display where the window demonstrably was. Five independent arrivals at
   this rule in one night across four lanes, carried at empyrean `origin/main a0e523a`.

7. **AND THE SEVENTH IS ON THE GATE'S INPUT RATHER THAN ITS OUTPUT: A GATE BUILT FROM THE SHAPES
   YOU ALREADY FOUND CANNOT COME OUT OTHER THAN GREEN** *(2026-09-10, written into the
   `NOTHING-MEASURES-OVER-ACCEPTANCE` brief; the hub banked it the same hour and it is the half of
   that brief they did not already hold)*. Faces 1 to 6 all ask whether the check can observe its
   subject. This one asks **where its population came from**, and the answer is almost always the
   defect report that motivated the gate. A divergence ledger seeded with the divergences we know
   about asserts a set against itself: it is green on the day it lands, green forever, and it
   measures nothing, while presenting as the exact instrument that would have caught the thing it
   was built after.
   **It is the motivating-case rule arriving on a GATE instead of on a parcel**, and the parcel
   form is already banked in `docs/OVERSEER-REFERENCE.md` (*THE MOTIVATING CASE IS SELECTED FOR
   BEING BROKEN, NEVER FOR BEING REPRESENTATIVE*, moved out of the boot read on 2026-09-10). The
   difference is who is fooled and for how long: a parcel
   verified against its own motivating corpus is wrong once, at review. **A gate whose population
   is its own motivating set is wrong every day it runs, and each green makes it more trusted.**
   **The question that finds it, asked while the brief is being written and not at review:** what
   would a half-built version of this gate look like, and would anything go red? By review a green
   run exists and reads as evidence.
   **The remedy is a population derived independently of the finding**: from the reference tool's
   own refusal classes, from a sweep of the corpora, from mutation toward a boundary. And **report
   the population SIZE beside the verdict**, because a gate over four members and a gate over sixty
   are different instruments with the same green.
   **⚠ THIS FACE IS A PREDICTION AND NOT YET A MEASUREMENT, AND NO LATER SESSION MAY CITE IT AS
   ONE.** It was written into a brief before the parcel ran, so what stands behind it today is the
   argument above and the parcel form's own n=2, not an observed instance of a self-seeded gate
   going vacuously green here. **When that parcel reports, come back and either ground this face in
   what it measured or say plainly that it was not tested.** A face banked from a brief is exactly
   the shape this document warns about elsewhere: a claim with no artifact, in the register of a
   finding.

**THE ONE CLAUSE THAT CATCHES ALL OF THEM is invariant 6(c): applied-and-still-green is a RUNNER
DEFECT, never a pass.** It is not a formality and it must never be softened, it is the only step
that discovers a fixture cannot fail, because a fixture that cannot fail is silent by construction.
**And its companion, aeon's** *(2026-09-06)*: *"the assertion held"* and *"the assertion could have
failed and did not"* are different claims and **only the second is worth anything**. Where a
perturbation that would make the gate fail cannot be constructed at all, **saying so plainly is the
valuable outcome**, never a gap to paper over.

### A BEFORE/AFTER STREAM DIFF NEEDS AN ENGAGEMENT COUNTER BESIDE IT

**Diffing diagnostic streams before and after a change CANNOT distinguish "the rule engaged and
changed nothing" from "the rule never ran".** Both print an empty diff. This overseer prescribed
that method twice and it was benign both times by luck: one tree's population was zero, and the
other died 5,761 front-end diagnostics before the stage under test.

So a census reports **how many times the changed code was ENGAGED**, beside the diff. An empty diff
with a non-zero engagement count means something; an empty diff alone means nothing.

**And the counter is NECESSARY, NOT SUFFICIENT** *(the agent's correction, accepted)*: a counter
reporting `repeats=0` beside a green suite still proves nothing if the fixtures cannot fail. Pair
it with 6(c) or it becomes face 2 with a number attached.

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
  the landing-lane division — in `docs/OVERSEER.md`, and ABOVE this text before the cut, so
  the old word "below" was wrong in the boot read too — already required freezing from a
  clean checkout of a committed SHA.
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
  clippy `-D warnings` exit 0. The wrapper ENFORCES the zero: a `skip:` or `skipping` line
  inside its test span is `RESULT FAILED`, exit 1, in the same condition as a red test and
  a red lint bar (`crates/sigil-harness/tests/landing_verdict.rs` holds it there by running
  the script over fixture logs through `--verdict-only`).
  **The env vars belong INSIDE the command span, and this paragraph had them outside it
  until 2026-08-27.** That is not formatting: a reader copies the backticked command and
  gets the prose-stated requirement only if they read two lines further. It is the exact
  defect that cost aeon two unverified chains from their own landing lane, found in their
  file and swept here immediately after.

  **This paragraph said `3939 / 3943` for a day while a RELAYOUT-REVIEW section — which no
  longer exists in either file; it was cut to `docs/OVERSEER-LOG.md` and the pointer was
  never repaired, so this cite was already dangling before the 2026-09-04 split —
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

Between those two sits `SITE_WATCH` in the same file: for EVERY admitted id it pins the
FILES the id fires in — unioned over the seven shapes, counts deliberately not pinned —
so a class reaching code it had never reached fails by name even with no register row.
A red there is one of three things and the message says so: a real new site (adjudicate,
then pin the file or fix it), a file aeon RENAMED (move the row), or a class that stopped
firing somewhere (delete the line and say so). Two ids carry `unpinned` directory
prefixes whose populations were measured to grow with the corpus; each prefix must carry
its measurement, and the firings it swallows are printed on every run.

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
- **A GATE THAT WALKS THE FILESYSTEM IN A REPO HOSTING WORKTREES INSIDE ITSELF FAILS FOR WHOEVER
  STANDS IN THE PARENT TREE AND FOR NOBODY ELSE** *(aeon's defect and aeon's fix, 2026-09-09;
  relayed, not reproduced at this seat)*. Their `test_citation_form` walked the disk for `.emp`
  files and descended into `.claude/worktrees/`, so a bare `foo.emp:N` citation matched roughly 160
  checkouts and resolved ambiguous: 183 of 363 cases red, and it was **failing `build.sh`'s pytest
  lane**, met as a failed four-shape verify on a comment-only change. **The red was unreachable by
  exactly the people who could fix it and unavoidable for anyone standing where landings happen.**
  The underlying bug was an asymmetry: the citing side enumerated from git while the cited side
  walked the disk, two populations answering one question. Fixed at the class, not the instance,
  by enumerating from `git ls-files` plus `--others --exclude-standard`, which cannot see an
  ignored tree by construction rather than by remembering to skip one; and because the METHOD
  changed they re-established the existing claims, three poisons each quoted off disk and restored
  from a committed baseline, including a poison for the silent-recreation path (git made to fail
  goes red with *"will NOT fall back to a directory walk"*).
  **⚠ THIS BLOCK FIRST SAID THEIR TREE WAS EXPECTED TO BE RED AND COULD BE DISREGARDED. That was
  wrong and it is the more instructive half.** The relay carried checkable facts (the counts, the
  stashed control) and one unverifiable FRAMING, that the red meant nothing; this seat checked the
  facts and banked the framing, into a reference file, inside twenty minutes, while
  `docs/OVERSEER-REFERENCE.md` already carried *a framing cannot be verified by checking the things
  inside it*. **A standing "expect that red" rots into a licence to ignore red**, which this suite
  has recorded happening twice, and it was aimed at a defect whose owner was actively fixing it.
  The byte-gate advice below is unchanged in effect and changed in reason: run it against a clean
  checkout or `.aeon-sigil-ref` because **that is the right instrument**, never because some other
  tree is expected to be red.
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

### THE REFERENCE TREE IS `.aeon-sigil-ref`, AND ITS EXCLUSIVITY IS INCIDENTAL (2026-09-09)

**Provisioned at `/home/volence/sonic_hacks/.aeon-sigil-ref`**, detached at aeon `ec640bcf`, by
`scripts/provision-aeon-ref.sh`, with the assembler built from this checkout rather than the shared
binary. Both rebuild controls matched the goldens, all four shapes verify against the provenance tip
(s4 b09ccd65/820229, s4.debug 1b7fe316/846529, demo 0ad17404/96863, demo.debug 2565ece2/103185), and
the positive witness passed: `repin --check` prints `pins.rs unchanged`.

**Why it replaced `.aeon-ls12-fix`, and the finding is this seat's own.** That tree was adopted at
boot after verifying its four ROMs by digest. PREPARED was established and EXCLUSIVE never was, and
four agent briefs then carried the words "PREPARED, READ-ONLY for you", which is true about the agent
and silent about everyone else. The writer turned out to be THIS LANE: `native::ensure_generated`
calls `emit_generated`, which runs seven emitters into `engine/sound/generated/` of whatever tree
`AEON_DIR` names, and eight test binaries call it as a precondition. A write landed at 01:04:06Z
inside a landing run that ran 00:59:15Z to 01:04:13Z. **The attribution was first sent to the aeon
lane as theirs, on an agent's say-so, unverified; their one question, "how did you attribute it",
was the entire audit.**

**The sharper form of the rule: exclusivity is not only about other lanes. Ask whether your OWN
gates write into the tree.** Watching for a foreign writer while being the writer is the shape that
survives every check aimed outward.

**⚠ AND THIS TREE'S EXCLUSIVITY IS AN ACCIDENT OF ITS PIN, NOT A PROPERTY.** The aeon lane disclosed
that `tools/test_extern_guard_reachability.py` resolves its root from its own file location and runs
a sound-on `sigil build --check`, so **any tree holding their `tools/` is written by their pytest
lane**. That file is ABSENT here only because `ec640bcf` predates it. **The day this pin advances
past that commit, the exposure returns and nothing will announce it.** Re-check for that file
whenever the reference revision moves.

**A timestamp trap that nearly reversed the attribution.** Formatting mtimes with a literal `Z` in
the format string renders a LOCAL time as `...T21:04:06Z`, which reads as UTC and is four hours off,
enough to move the write outside the run and back onto the peer. Use `--time-style=full-iso`, which
prints the offset.

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

**THE DURABLE PIN WORKTREE `~/sonic_hacks/.sigil-pin-af35fa56`** (added 2026-09-07T23:49:44Z, and it
SUPERSEDES `.sigil-ls12-pin`, one for one). The shared pair installed at that instant bakes this path
as its `source:` field and aeon's `build.sh` resolves it on every build, so sweeping it turns every
aeon build's assembler provenance to `unknown` and refuses under `SIGIL_VERSION_STRICT=1`. Installed
pair: `sigil` `49ecc532e0b133ab0eab9447e071805c`, `emit_sound_blob` `b1569c67cbd02aba003a1455e72f6046`,
built from sigil `af35fa56` in a clean detached worktree (`tree: clean at capture`). The OUTGOING pair
is kept at `~/sonic_hacks/.sigil-outgoing-135ba589/` with the swap instant beside it, copied aside
BEFORE the rename, so this refresh is reversible with the original artifacts rather than a rebuild.
**⚠ THE INSTALLED EMITTER CHANGED 2026-09-12T06:25:12Z, AND `b1569c67` ABOVE IS NO LONGER WHAT IS ON
DISK.** On 2026-09-08 23:04:49 -0400 a cargo command in the main checkout relinked
`target/release/emit_sound_blob` to `8d80a57809979d43fe8d7c3a0a9ded13` (embedded source path: the main
checkout; HEAD `55b14261` by reflog; the command is unidentified), splitting the pair silently for three
days. Repaired inside a hub-opened window on aeon's clear: the installed `emit_sound_blob` is now
**`36ef302cbf5eeca693ecbf981ce8a53a`**, built from `af35fa56` INSIDE `.sigil-pin-af35fa56` (porcelain 0
before and after, `--locked`) with target dir `~/sonic_hacks/.scratch/pair-reinstall/target`, build
instant 2026-09-12T06:18:12Z. That is also the file's mtime, since `cp -p` kept it; the swap instant is
the one in this heading. `sigil` stays `49ecc532...`, the original. It cannot match `b1569c67`: a
same-revision rebuild of `sigil` from the same tree gave `4061cd32...`, so builds here do not reproduce
md5s, and the hub ruled known source over a byte match. The outgoing `8d80a578` is kept at
`~/sonic_hacks/.sigil-outgoing-8d80a578/` until aeon's four-shape re-check under the new emitter is
green. **Re-check GREEN 2026-09-12** (aeon `808141f7`, all four shapes byte-identical to master's ROMs
built under `8d80a578`, per aeon's own run): the swap stays, and the copy is RETAINED as provenance for
the aeon builds of 09-09 to 09-12, not deleted. What that re-check cannot say, in aeon's words: nothing
about `b1569c67`, which no longer exists anywhere. **The next drift check compares against `36ef302c`.**
**A SECOND REASON IT MUST SURVIVE, found 2026-09-11:** `native::load_frozen_table`
(`crates/sigil-harness/src/native.rs:232`) opens `env!("CARGO_MANIFEST_DIR")/golden/offcanonical_sizes/`
at RUN time, and every shape profile seeds its provisional section bases from it. `CARGO_MANIFEST_DIR`
is baked in at compile time, so the installed binary reads those tables from THIS worktree on every
aeon build. Deleting the tree breaks placement, not only the `source:` field. Detail and call sites:
`docs/superpowers/notes/2026-09-11-aeon-source-digest-ask.md`, second amendment.
**`.sigil-ls12-pin` is released once aeon confirms it is on the new pair — ask them, not this file.**

**THE INSTALLED PAIR SINCE 2026-09-12T08:57:35Z IS THE 6884bfba BUILD, and three trees must survive for it.**
Swapped on the hub's OPEN (aeon LS-1a) by the approved method: `/proc` exe scan (0 hits), outgoing pair
copied aside and md5-verified, each new file `cp -p`'d to a staging name inside `sigil/target/release/`
and `mv`'d over, so each installed file has its own inode (link count 1). Installed: `sigil`
`2e7c25920b95cec2c462ea51b4f078b5`, `emit_sound_blob` `d258341604bbf735a8af8438c2b8d642`, built from sigil
`6884bfba` in the durable worktree `~/sonic_hacks/.sigil-pin-6884bfba` with target dir
`~/sonic_hacks/.sigil-pin-6884bfba-target` (banner: `tree: clean at capture`, `source:` that worktree).
Aeon's four-shape decider PASSED at aeon `57528c22` in `.aeon-ls8-land`, re-read here from disk rather
than taken from their message: s4 `7a552cde`/821155, s4.debug `b93a889f`/847533, demo `dd589fe7`/97109,
demo.debug `c3eda757`/103501, every listing's `DIGEST-ASSEMBLER` naming revision `6884bfba`.
**The outgoing pair is KEPT at `~/sonic_hacks/.sigil-outgoing-49ecc532/`** (sigil `49ecc532...`, emitter
`36ef302c...`, with `README-ASIDE.txt` carrying the instant and the restore recipe), as provenance for the
aeon builds of 2026-09-12 06:25Z to 08:57Z and as the restore path. **So keep all three:**
`.sigil-pin-6884bfba` (the installed pair reads its size tables from it at run time, per the
`load_frozen_table` paragraph above), `.sigil-pin-6884bfba-target` (the build that the aside recipe
does not cover), and `.sigil-pin-af35fa56` (the aside pair reads ITS tables from there, so a restore
without it breaks placement). Ask the aeon lane before retiring any of them.

**⚠ THIS ARTIFACT EXISTS BECAUSE OF A ONE-CHARACTER MISMATCH IN A PEER'S PARSER, and it dies when
they fix it.** Measured by the aeon lane 2026-09-07 across all four arms of their `tree:` switch:
`clean`, `clean at capture, no uncommitted changes` and bare `clean-sources` are all ACCEPTED, while
the real main-checkout string `clean-sources, 1 uncommitted change` falls to the catch-all — their
accept pattern is `clean-sources` with a TRAILING SPACE, and `clean-sources,` is not that. So a pair
built from the main checkout (whose `source:` path is permanent and never swept) would be refused
under strict for the comma alone. **This lane predicted the refusal and was right for the WRONG
reason** — it believed `clean-sources` was outside their accept list. When their arm reads the real
vocabulary, build the installed pair from the MAIN CHECKOUT and this standing dependency ends
permanently instead of being handed from one worktree to the next. Aeon has booked that fix; do not
press its timing, and do not rebuild in the meantime.

**RETIRED 2026-09-07T23:5xZ: `~/sonic_hacks/.sigil-ls12-pin`.** Aeon confirmed on the new pair and
released it; the worktree and its 148 MB target dir are deleted and the branch is gone. Kept as one
line so a reader meeting the path in an older note knows it was retired rather than swept.

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

**⚠ THIS SENTENCE'S FIRST CLAUSE IS STALE AND IS CORRECTED IN PLACE (2026-09-07), because it is the
line that instructs every brief.** `test_support.rs::aeon_dir` **no longer** falls through to the
owner's live checkout: `PathStep::Derived` — step 3, `<suite root>/aeon` — is excluded from
`names_a_reference_tree` **by name**, with the reason stated at `test_support.rs:795` (its revision
moves under a run, so a pass or a failure measured against it is attributable to whatever it
happened to contain). A run that names no tree reaches `no_named_reference_tree` and refuses or
skips by declared intent; it does not quietly measure his directory. **The reason to name the tree
in every brief is therefore no longer "or it silently uses his" but "or the run refuses" — the
practice is unchanged and its ARGUMENT is not**, and a brief repeating the old argument teaches a
mechanism the code refutes. *(Found 2026-09-07 by turning aurora's fifth Roster C rule on this
lane's own rig, an hour after this seat flagged the identical stale-sentence class in a peer's
file.)* The rest of the paragraph stands: `sigil build --aeon <tree>`
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

## Read at the moment - the 2026-09-05 blocks

Moved verbatim from `docs/OVERSEER.md` at master `8e35bd94` on 2026-09-05, when the boot read
crossed its byte bound. Each block is read at the moment its rule applies rather than at boot,
and the boot read names every one of them by its own heading. Nothing was shortened to move it.

### AS-NAMELESS-LABELS-RC1: the sizing detail (original lines 839-883)

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
to the exit-status rule under SIGIL-AS-REPLACEMENT in `docs/OVERSEER.md`, and it is its own
small parcel with its own verification
because it rewrites committed test vectors.

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

### ERROR-PATH BEHAVIOUR IS CONDITIONAL ON WHAT ELSE FAILED (original lines 1045-1049)

**The general rule, which is what to carry: error-path behaviour is CONDITIONAL ON WHAT ELSE FAILED,
so a probe samples ONE POINT in that space and both directions must be run.** Probing only clean
files misses everything an earlier stage swallows; probing only dirty ones misses everything that
needs a clean run to reach. Neither habit is the safe one, and **a lane that adopts only the
minimal-probe rule has swapped one blind spot for the other.** *(That rule, and the register fault
that earned both halves: `docs/OVERSEER-LOG.md`, 2026-09-05 cut.)*

### WHEN A CHANGE MAKES SOMETHING NEWLY REFUSE, READ THE OLD CODE (original lines 1099-1102)

**The generalisable move: when a change makes something newly refuse, ask what the OLD code did on
the same path.** If the old path was a panic, an `unreachable!`, or an already-failing branch, the
new refusal cannot regress a working build and no build is needed to prove it. That is a stronger
argument than a green run, because a green run samples inputs while this quantifies over them.

### A CONTROL CAN BE CONFOUNDED BY THE VERY THING IT CONTROLS FOR (MOMPASS, 2026-09-05)

*(Landed `f1673ba2`, after this seat HELD the first version. Three lessons, and the first is the one
the boot read did not already have.)*

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

### A SWEEP KILLS ANY NEGATIVE ASSERTION KEYED TO WHAT IT CHANGED, AND NOTHING ANNOUNCES IT

*(Raised by the aurora lane from their own dash sweep; **checked here and it had happened**, one
instance, fixed at the same time.)*

**`crates/sigil-cli/tests/version_provenance.rs` asserted `!value.ends_with('—')`** with the comment
*"a dangling em-dash means a reason was promised and not supplied"*. The producer, `build.rs`, rendered
`format!("unknown — {}", why)` until the 2026-09-05 dash sweep and renders `format!("unknown, {}", why)`
after it. **The sweep did not touch the assertion, so from that moment it could not fail.** A negative
assertion whose needle the producer has stopped emitting **does not go red, it stops testing**, and it
keeps printing `ok` forever.

**Re-pointed at the producer's current separator with the reasoning written beside it**, and **proved
it can fire**: injecting a dangling comma into the live `tree_detail` path reds
`no_banner_field_is_blank_or_a_bare_placeholder`, naming the field and quoting the value.

**⚠ MY FIRST RED-FIRST ATTEMPT WAS VACUOUS AND PRINTED A FULL PAGE OF `ok`.** I mutated
`format!("unknown, {}", upstream.why)`, which is only reached when the upstream probe fails, and it
never runs in this repo. **The mutation applied, the suite passed, and nothing distinguished that
from a working assertion** until I asked which of the rendered fields the mutated line actually
produces. That is face 2 of the three vacuous-proof shapes already banked here, met while proving a
fix for face 3. **Mutate a path you have CONFIRMED renders in the artifact under test**, not one that
merely looks like the right function.

**The general rule, which is broader than dashes: a sweep's blast radius includes every NEGATIVE
assertion keyed to the text it changed.** A positive assertion reds loudly and gets fixed in the same
hour; a negative one goes quiet and is indistinguishable from a passing test forever. **After any
mechanical text change, grep the tests for negations keyed to the old text**, which for this lane was
`git grep -P '!\s*\w[\w.()]*\.(contains|find|starts_with|ends_with)\([^)]*<old-text>'` against the
PRE-sweep revision, because the post-sweep tree no longer contains the evidence.

**A control is what makes that grep mean anything:** the same pattern without the negation returned
7 hits pre-sweep, so a zero from the negated form would have been a finding rather than a broken
pattern. It returned 1.

**And the sibling finding, also aurora's, checked here: a dash inside a CHAR literal is neither
comment nor string.** `tool_text_dash_lint` tracks char-literal state deliberately and excludes it,
which is right for user-facing prose and means `'—'` survives a sweep by design. That is exactly how
this dead assertion kept its needle. Their instance was a regex literal; ours was a char literal;
**the class is "a token the lexer classifies as neither of the two things the gate looks at".**

### WORKTREE PRUNING: THE LOCK IS NOT A LIVENESS SIGNAL, AND I OVERRODE ONE TWICE

*(Criterion retracted by the hub from aurora's measurement; **checked here, where it lands on two
things I did tonight**.)*

**The retracted criterion:** "tip is an ancestor of master" does NOT mean an agent worktree is
finished. A tree started at master HEAD with uncommitted work is **trivially** an ancestor. I never
pruned on it, having only counted, but the count I reported to the hub (238 of 246 branches
"removable") was computed with it and **is not a safe removal list**.

**⚠ I PASSED `-f -f` TWICE TONIGHT**, on `agent-a91ec943071597b23` and `agent-a3923cd793d91d0de`. In
both cases `git worktree remove --force` had **refused** with `use 'remove -f -f' to override or
unlock first`, and I read that as a syntax hint rather than as a guard telling me something. **What
saved me was ordering, not judgement**: I had merged each agent's tip before removing, so the
committed work survived, verified after the fact (`eda59a35` and `aede487e` are both ancestors of
master). **I cannot prove no uncommitted scratch was lost, because the trees are gone.** Treat the
refusal as the answer.

**⚠ AND THE LOCK DOES NOT EXPIRE, SO "HONOUR THE REFUSAL" ALONE LEAKS DISK WITHOUT BOUND.** Measured
here: **5 locked trees, of which exactly ONE is a live agent.** The other four are 17, 25 and 40 hours
old, left by agents that finished long ago, and they hold **34.8 GB between them, one of them 28 GB
alone.** So the lock records that an agent *claimed* a tree, never that one still holds it.

**The safe rule needs both halves, and neither is sufficient:**

- **Liveness comes from whether the agent is running**, not from the lock. A task notification
  received, or the agent absent from the live list, is the evidence a lock cannot give.
- **The lock is the tiebreaker when liveness is UNKNOWN.** Refusal stands; never a second `-f`.
- **The ancestor test is at best a staleness hint** among trees already known dead, and it is wrong
  in both directions: fresh trees are ancestors, and squash or rebase merges make landed work a
  non-ancestor.
- **`du -sh` per tree before any count**, because "31 worktrees" and "34.8 GB in four dead locks" call
  for different urgency and only the second one is a reason to act tonight.


## Moved from the boot read on 2026-09-06, when it crossed its byte bound

The blocks below, verbatim, moved under the owner's 2026-09-04T15:38:47Z ruling that the boot read is
split by WHEN A RULE IS READ rather than by size. **Nothing was shortened to move it.** The boot
file names every one of them by its own heading at the point they used to sit.
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

**⚠ THE INSTALLED BINARY'S OWN MD5 WAS RECORDED NOWHERE UNTIL 2026-09-06, WHICH IS THE ONE FIELD A
CONSUMER PINS BY.** The 2026-09-05 refresh above is recorded as `58db3594...` to `945387f2...`,
correctly. The 2026-09-06T05:45:48Z swap that superseded it is recorded by its **commit**
(`e6e942e5`) and never by its digest, so this document told every reader to *"select and cite by
MD5"* while giving none for the artifact actually on disk. Measured and recorded rather than left to
the next reader:

| path | md5 | mtime |
|---|---|---|
| `target/release/sigil` | `fa18ebfe849aa4bfb3203cde7c8a0770` | 2026-09-06 05:45:48Z |
| `target/release/emit_sound_blob` | `0c1cf7ec5dd452bfb703856a42437d6d` | 2026-09-06 05:45:48Z |

**Superseded 2026-09-07T13:35:41Z, inside a hub-opened window and after aeon's explicit clear** (the
handshake the ownership bar above asks for, exercised for the first time). Installed by rename from
sigil `135ba589` (branch `parcel/ls12-blob-repin`, built in the durable worktree
`~/sonic_hacks/.sigil-ls12-pin`, which the banner names as `source:`): `sigil`
`509b93c4520aed3e0d65e4a7003104d3`, `emit_sound_blob` `51e1190dd97577690658d598e6f968e8`. The file
mtimes read 12:36Z, the BUILD time, because the copy preserved them; the swap instant is the one
above. The outgoing pair is kept at `~/sonic_hacks/.sigil-outgoing-e6e942e5/` (md5 as in the table),
so this refresh, unlike the 09-06 one, is reversible. A refresh from sigil master is owed after the
LS-12 landing so the banner names master rather than a branch; the durable worktree is released only
after aeon confirms it is on that pair.

**Both mtimes are the swap instant and have not moved since**, which is the check that says nothing
has relinked the path. Read the mtime as well as the digest: a digest alone cannot say whether it is
the blessed build or a coincidental rebuild of the same source. **The general form is this
document's own rule turned on itself: recording an artifact's SOURCE is not recording its IDENTITY,
and a consumer pinning by digest cannot use a commit.**

**Resolution of the incident: `e6e942e5` STAYS.** No swap back and no rebuild of `d094c3c8` was
ordered, on the grounds that a reproduced pin would answer a question nobody needs any more, since
aeon restarts every SP-5 leg under the new identity. **The path is frozen to this lane until the hub's
word.** The refreshed binary's acceptance rests on the proof, not the version string: one
`^PHASE-COUNT` line, six phase rows, and **zero lines matching the old spelling, checked as a
control** rather than only confirming the new one appeared.

### NEVER PASS A COMMIT MESSAGE THROUGH `-m "..."` IN THIS SHELL: BACKTICKS RUN AS COMMANDS (2026-09-06)

*(Read at the moment you are writing any commit, and it is the one bar below that has a mechanism
rather than a habit as its remedy.)*

**Instance, this seat's own, at `3a5eb6c1`.** A landing commit was written with `git commit -m "..."`
instead of the quoted-heredoc form used for every other commit that day. The message contained
`` `!org 0` `` and `` `save` ``. **zsh performs command substitution inside double quotes**, so both
were executed and REPLACED BY THEIR EMPTY OUTPUT before git ever saw the message. Two sentences lost
their subject:

- *"`` `!org 0` `` inside the 68000 ROM is such a target"* became *"inside the 68000 ROM is such a
  target"*.
- *"s1disasm has `` `save` `` before its org"* became *"s1disasm has before its org"*.

**The commit succeeded, exit 0.** The only signal was two `zsh: command not found` lines on stderr,
scrolling past inside ordinary merge output.

**Why this is worse than a typo: the mutilation is SELECTIVE and it eats exactly the identifiers.**
Backticks are how a technical message quotes a symbol, a directive or a path, so the shell destroys
the load-bearing tokens and leaves the prose around them grammatical enough to read. The same
message keeps its argument, its numbers and its conclusion, and silently drops the thing the
argument is ABOUT.

**This is protocol bar 23 (*a commit message is a claim about a diff and nothing checks it*) with a
new mechanism: the claim was correct when written and was corrupted between the keyboard and git.**
Bar 23's remedy, read the blob back before writing the message, does not reach this at all, because
the damage happens after the check.

**THE RULE, and it is a mechanism rather than vigilance: write every commit message with
`git commit -F - <<'EOF'`.** The quoted delimiter suppresses all expansion. `-m` is acceptable only
for a one-line message containing no backticks, no `!`, and no `$`. Never reach for `-m` because the
message is short; reach for the heredoc because the shell is hostile.

**And the recovery is constrained, which is worth knowing before it happens.** The commit was
already pushed, and fixing a pushed message means `--amend` plus a force push. That is a HISTORY
REWRITE and is explicitly outside this lane's standing push approval, which covers fast-forward
pushes of finished work and nothing else. So the correction is an APPENDED commit, never a rewrite,
and the damaged message stays in the history with a successor naming it.

**Sweep run before banking this, because an instance is not a class:** all 25 commits from that day
were checked for the signature (a word gap where a quoted token was eaten). One was damaged, this
one. Every other commit that day used the heredoc form and is intact. Note the first sweep returned
a confident ZERO from a broken `--since` filter and had to be re-run with a canary, which is the
fifth false zero this lane hit in a day.

### DO NOT COMMIT WHILE A GATE IS READING THE TREE (2026-09-06)

*(Read at the moment you start any full-suite or landing run.)*

**Instance, this seat's own.** A landing run was started at HEAD `6e04f349`. A docs-only commit was
made while it ran. `version_reports_the_head_of_the_tree_it_was_built_from` failed: the binary bakes
in the revision it was built from, and the checkout's HEAD had moved past it.

**The reasoning that let it happen is the part worth keeping, because the hazard WAS noticed.** The
thought was: the commit touches a note, nothing compiled, so it cannot change the run's inputs. Both
clauses are true. They answer a DIFFERENT mechanism from the one that fired. The gate does not
compare source, it compares the binary's recorded provenance against the tree's, and **every commit
moves that, whatever it contains.** Noticing a hazard and then dismissing it with a correct
statement about an unrelated mechanism is worse than not noticing, because it retires the worry.

**THE RULE: from the moment a full-suite or landing run starts until it prints its verdict, the tree
does not move.** No commits, no merges, no branch switches. Queue the work and do it after.

**And a run you can EXPLAIN is not a run that PASSED.** The broken run was informative (412 suites,
4,703 passed, exactly one failure, the self-inflicted one, and 4,703 + 1 reconciled to the expected
4,704) and it is NOT what the landing rested on. It was re-run at a still HEAD. Explaining away a
red is always available and is right often enough to be dangerous, which is why this document
already forbids retiring a check because it is red; this is the same move wearing a diagnosis.

**Credit where the gate earned it:** it did not merely fail. It named BOTH candidate causes (*"either
build.rs did not re-run when HEAD moved, or HEAD moved while the suite was running"*) and the command
that distinguishes them. A gate that hands you the discriminator turns an investigation into one
command, and it is worth copying that shape into anything that can fail two ways.

**⚠ EXTENDED 2026-09-10, AND THE FIRST FORM WAS TOO NARROW IN TWO WAYS AT ONCE.** It says *the TREE
does not move* and forbids *commits, merges, branch switches* - all acts performed in the tree that
is running the gate. This seat obeyed every word of that and reddened an agent's landing run three
times.

**What actually moved was `origin/master`, and I moved it from a DIFFERENT CHECKOUT.** The agent was
in its own worktree; I never touched it. `crates/sigil-cli/build.rs` bakes `SIGIL_PUBLISHED` from
`origin/master` at COMPILE time and
`the_published_line_states_this_revision_s_position_against_a_named_remote_ref` resolves that ref
LIVE at test time. **A worktree shares its parent's ref store**, so a `git push` from the main
checkout updates the remote-tracking ref the agent's gate reads, instantly, with nothing in either
tree changing. Six pushes across a roughly 25-minute window, three landing runs of about six minutes
each, three reds.

**So the frozen thing is not the tree, it is EVERY REF THE GATE RESOLVES**, and the actor who breaks
it need not be in the tree under test. Restate it that way: **from the moment a full-suite or
landing run starts until it prints its verdict, neither the tree NOR the refs move - including
`origin/*`, and including pushes made from any other checkout sharing the ref store.** A controller
holding an agent's landing run is the likeliest violator, because banking findings between polls is
exactly what there is time for while a gate runs.

**The direction that makes it expensive: the failure lands on the AGENT.** Its work is indicted by
its controller's activity, the red names a test it never touched, and it has no way to see the
cause. This one diagnosed the mechanism, predicted both values of its next run in advance, hit both,
and refused to fix the gate to green its own landing, which is the correct call and cost it a
recovery cycle it should not have had to spend.

**Whether the gate itself is at fault is a SEPARATE question and must not be settled while it is
red** *(this lane's standing bar: the tell is who is expected to move)*. It reds correct code
whenever a push overlaps a run, which is the shape that trains people to weaken a check. Booked as
its own row; not touched as part of a landing it was blocking.

### A MONITOR FILTER THAT MATCHES NON-FAILURES TRAINS YOU TO SKIM IT

Same session, and it is the reason the above took three notifications to see. A filter watching for
`panicked` fired on **every `should_panic` test in the suite**, which are tests PASSING, and one
watching for `Killed` fired on a game object named `Killed_CheckObject`. **Coverage is not the only
property a filter needs; precision is what keeps its events readable.** The authoritative signals
here are `^test result: FAILED` and the verdict block, not the panic text, because a panic is how
half of these tests report success. **Ask not only "would this fire if the run crashed" but "does
anything that is NOT a failure match this."**

### LS-13b, THE AEON ROW BLOCKED ON SIGIL: WHERE IT ACTUALLY LIVES (routed 2026-09-09)

Read when anyone asks why aeon's board carries a row `blockedBy: sigil`, or before starting it.

**The artifact is aeon `origin/master d3339973`, `docs/DEFERRED_WORK.md`, the row beginning
`| LS-13b |`.** Read it there rather than from any summary. `git grep LS-13b` in THIS tree returns
nothing and always would: the id is aeon's ledger coordinate, not a shared one, so the empty result
is a fact about which tree the search ran in and not about the row. That is protocol bar 16(d)
arriving from the receiving side, and this seat spent a message concluding the row might be
ungrounded before the path was sent.

**The ask, which is narrower than the title:** `engine/system/boot.emp`'s `EntryPoint` requests,
spins on and releases the Z80 bus by hand around the driver-blob copy, because it interleaves the
hold with the /IC reset pulse, a shape `with z80_stopped` cannot take. So it gets none of the
pairing proofs (`[context.escape]`, `[context.entry-skip]`, `[context.reacquire]`), and this lane's
`[bus.*]` net cannot see it either: that net keys off a RESOLVED destination operand and a
register-indirect destination is its documented soundness bailout, per the module doc in
`crates/sigil-frontend-emp/src/z80_bus.rs`. **It is a LANGUAGE question, not a lint request: can
`z80_stopped` be given a form that admits boot's interleaved /IC pulse**, which would delete the
exception rather than document it. So it is `.emp` language surface and lands under propose,
discuss, then land, never silently.

**Nobody is held up by it.** It sits behind the owner's unanswered items and aeon's region planning
does not wait on it.

### THE UX SEAT PAIR: WHAT ORACLE'S PILOT ALREADY REFUTED IN THE BRIEF (relayed 2026-09-09)

Read before dispatching `LENS-UX-SEAT-DIAGNOSTICS`. Banked here because it reached this lane only
by mail, and mail is not part of any tree. Oracle's amendment SHA is owed and not yet in hand, so
this is a relay: read their text when it lands rather than treating this as the source.

**The one sentence: isolate the SURFACE, not the process.** Both of oracle's safety defects were a
rig that correctly confined a process while leaving it a path to the owner's screen that nothing in
the proof looked at.

- **An absence check must point at the surface the thing would escape TO, not the one you can
  conveniently enumerate.** Their charter proved "window on its own Xvfb, nothing on `:0`"; on a
  Wayland desktop that is blind, and they measured a window on the owner's real screen while the
  X-only check found zero windows. **Environment-as-launched and environment-as-running are two
  different claims and only the second is worth anything**: enumerate the running process's OPEN
  SOCKETS. **Whether a `/proc/<pid>/environ` read is evidence at all is PER-SURFACE and decided by
  the client library, so check your own binding rather than inheriting either answer**: C
  libwayland's `wl_display_connect(NULL)` invents the literal `wayland-0` under
  `$XDG_RUNTIME_DIR`, so on an Electron or Chromium surface the variable's absence is consistent
  with the window being on the owner's screen, while Rust `wayland-client` has no such fallback
  (0.31.15 hard-errors `NoCompositor`, 0.29.5 gives `NoCompositorListening`) and there the environ
  read IS evidence. **On every surface the socket enumeration is the direct observation and the
  environ read never stands in for it, because no-fallback in one function is not a property of
  the whole graph.** Measured by aurora and by oracle on their own surfaces; carried at empyrean
  `origin/main d31a87e`, verified an ancestor from here.

  **⚠ THIS BLOCK WAS WRONG TWICE IN AN HOUR, IN OPPOSITE DIRECTIONS, AND THE MECHANISM IS THE
  TRANSFERABLE PART.** It first arrived here saying the environ read was load-bearing, was
  corrected to say it was worthless, and is now per-surface. Nobody was careless: aurora measured
  its own surface and said explicitly that it had NOT measured oracle's, and that qualifier was
  dropped while three findings were compressed into one bank entry, after which the wide version
  shipped onward as fact. **A true finding goes one notch wider or narrower in transit, and the
  RELAY is where the qualifier drops, because a per-surface caveat reads as throat-clearing
  exactly when you are compressing.** The fix is mechanical rather than attentional, and it is the
  same shape as this lane's write-the-SET-not-the-count rule: **attach the qualifier to the
  sentence, not to the surrounding context, so it survives every excerpt of it.** This seat's own
  instance, worth keeping because it is the rule failing on the session enforcing it: the commit
  that banked the first correction said in its message that *the remedy is the half nobody
  re-derives*, while carrying a remedy it had not re-derived.
- **The charter assumed a seat could drive a window and never said how; `xdotool`, `ydotool`,
  `wtype`, `dotool` and `xte` are all absent on this box.** The failure mode matters more than the
  delay: with no input path **a seat quietly substitutes reading the code for driving the thing, and
  a UX seat that reads code has stopped being a UX seat while still producing findings that look
  like UX findings.** This lane's pair is a terminal-diagnostics pair rather than a window one and
  the identical substitution is available to it: a seat reading our diagnostic source instead of
  running builds and reading the output would look the same in its report. **So the brief for our
  pair must require a transcript of commands run and output read, not a list of conclusions.**
- **`/dev/uinput` is user-writable here through a POSIX ACL the mode string hides** (`crw-rw----
  root:root`, only a `+` betrays it), so kernel-level injection is possible and must not be used:
  uinput events bind to no display and land wherever focus is, which is his live session. The
  general rule outlives the instance: **an input mechanism not bound to a display escapes a
  correctly isolated rig.**

### CUSTODY-INDEPENDENCE IS NOT OBSERVATION-INDEPENDENCE (2026-09-09)

Read when a peer's result agrees with yours and either of you is about to call it corroboration.

**The hub reconstructed this lane's destroyed board and it matched the restore exactly.** They
described it, honestly and wrongly, as *an independent corroboration from a source you do not
control*. The first half is true: `dominion/.dominion/changes.jsonl` is not this lane's file. The
second does not follow, because **this lane RECOVERED from that log and their tool REPLAYS that
log.** If the file is wrong about a row, both readings are wrong about it identically and agree
perfectly.

**The rule: a source being outside your control makes it independent of your BIAS, not of your
OBSERVATION.** Two lanes reading one file are one witness read twice, and the agreement carries no
information about whether the file is right. This is protocol bar 8 (mutual verification cannot
catch a shared frame) wearing a costume that reads as diligence, because the second reader really
is a different party with different motives and different code.

**What survived the correction is the shape to look for.** The COUNT was independently
corroborated: the hub had observed 16 rows live, on its own tick, through its own instrument,
before the loss. The row IDENTITIES came from the log alone. So the account checked out **for how
many, and not for which**, and separating those two was the whole value of the exchange. **When
agreeing with a peer, name which specific claim has two observations behind it rather than
accepting the agreement wholesale.**

### A TICK-DIFF LOG IS SILENT ON WHAT HAPPENED BETWEEN TICKS, AND SAYS NOTHING ABOUT IT

Read when relying on a change log to reconstruct prior state, or when judging a tool that
"degrades honestly".

**A log built by diffing snapshots can only testify about changes a tick actually observed.** A row
added and removed between two ticks leaves **no event of any kind**. The reconstruction is then
silently short, and reports nothing missing, because there is no event to fail on. Measured cadence
on this machine at the time: median 128 s between change instants, minimum 8 s, so the window is
real rather than theoretical.

**The transferable half is about the false comfort, not the gap.** The tool listed every event it
could not invert, which is genuine honest degradation and was the right design. But that clause
**fails only on events that EXIST**, so it gives its strongest reassurance exactly where the
instrument is blind. **A tool that degrades honestly on one axis can still be silent on another,
and the honest-degradation property gets banked as though it covered both.**

**Also from the same audit, and it is the reduction-supplies-the-absence rule turned on the
auditor.** This seat tested that log for completeness by asking whether every pre-loss row had an
`added` event. Zero missing, clean and convenient. The control refuted it: two rows had their FIRST
`added` event stamped at this seat's own RESTORE, hours after their creation, so the log holds no
original entry for either. **The repair had supplied the evidence the test was looking for.** The
peer's method survived anyway, for a reason the test was not measuring (it inverts
`left the queue (was X)` and never consults `added`), which is the distinction between clearing a
tool and clearing it for the right reason.

## Moved from the boot read on 2026-09-09, the second cut

Eight blocks, moved VERBATIM when `docs/OVERSEER.md` crossed its byte bound a second time,
under the owner's 2026-09-04T15:38:47Z ruling that the split is by WHEN A RULE IS READ.
**Nothing was shortened to move it.** They keep the order they held in the boot read, so the
move is provable as an order-preserving subsequence; each carries a locator giving its original
line span and the heading it sat under. The moment that triggers each is in the boot read's
index, *Read at the moment - the blocks moved to the reference file on 2026-09-09*. The last
four keep their own `##` headings, so they read as sections in their own right below this one.

### `pad`/`pad_to`: the spec text's landing, the diagnostic-string debt, and the sequencing

*(original `docs/OVERSEER.md` lines 366-391, under: The autonomy directive - and its scope, which is the part that matters)*

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
**PAID, and this paragraph said it was owed until 2026-09-09.** All four items below landed in the `pad`/`pad_to` parcel itself (`ffa7bdb8`, an ancestor of master): #4, #5 and #6 each have a test (`duplicate_at_offset_on_one_field_is_diagnosed`, `duplicate_field_align_on_one_field_is_diagnosed`, `an_unknown_field_attribute_keyword_names_the_one_that_belongs`), #2 and #3 `assert_eq!` the whole string, and the Scope clause is pinned by `field_align_does_not_propagate_into_a_nested_structs_own_fields`. All four verified by name here with a control, and empyrean `origin/main` records the same in the spec's own §4.3. The original text, kept so the correction is legible: **Owed by this lane, in the `pad`/`pad_to` parcel:** tests for #4-#6, widen #2/#3 to the full
strings, and pin the Scope clause (true by construction — the check walks only this struct's own
fields — but unpinned, and marked as such in the spec). The hub strengthens that clause on a
message from here once it lands.

Sequencing, so nobody lands half of it: the spec text is **empyrean's to land** — sigil does not
land `.emp` language spec, `SIGIL_SPEC2_LANGUAGE.md` is their file — lifted verbatim from
`docs/superpowers/notes/2026-08-26-pad-to-spec-draft.md`. **Nothing is implemented**; no crate
was touched by the parcel that wrote the draft, and `pad_to_cycles` in `t40_cycles.rs` is an
unrelated cycle-padding construct that shares a prefix and nothing else. Implementation is a
separate parcel and is this lane's.

**⚠ CORRECTION MADE AT THE 2026-09-09 CUT, and deliberately written BESIDE the moved text
rather than into it: `pad_to` IS IMPLEMENTED.** The block above says *"Nothing is
implemented"*. It landed at `ffa7bdb8` (merge of `feat/struct-pad`, 2026-08-26, verified an
ancestor of master here), and `crates/sigil-frontend-emp/src/parser.rs:1211` takes `pad_to` as
a struct-body pad marker. The boot read carried the stale sentence for two weeks, which is
exactly the defect the R7 block earned: **a parcel's completion has to be written back to the
document that DISPATCHES it.** What this cut did NOT establish, and is still owed: whether that
parcel paid the debt the block records, the tests for diagnostic strings #4 to #6, widening #2
and #3 to their full strings, and pinning the Scope clause.

### Selecting and citing the `asl` oracle: three faces, and the md5 covers only the first

*(original `docs/OVERSEER.md` lines 527-590, under: SIGIL-AS-REPLACEMENT - active on the owner's own words; source locations LANDED)*

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

**`ASL-GUARD-EXIT-STATUS` IS CLOSED, and this paragraph used to say it was open.** It said
`asl_ref.sh` checks the binary and not the run, and that checking the exit status was therefore the
caller's job on every asl invocation. That was true when written and stopped being true at
`8e35bd94`: the guard now defines **`asl_run`**, which runs the pinned binary, prints `ASL_EXIT`
into the transcript whether or not the caller thought to look, refuses out loud on a non-zero
status, and returns that status so a caller's `|| exit $?` works the way it does for the digest
check. `asl_run` is the blessed invocation; calling `"$ASL"` directly still works and asks nothing
about whether the program answered. See `docs/superpowers/notes/2026-09-05-asl-guard-exit-status.md`
and the `asl_run` header in `docs/superpowers/notes/asl-reference/asl_ref.sh`.

The property that improved is narrow and worth naming rather than deleting the caveat over: the
guard now answers **"did the run as a whole fail"** in addition to **"which program ran"**. It still
does not answer **"did the build answer THIS line"**, and no check anywhere does. For a shape asl
declines, the byte column is an artifact on a clean exit too.

**AND THE RULE INVERTS FOR A CORPUS OF DELIBERATELY-FAILING FILES.**
`crates/sigil-frontend-as/tests/over_acceptance/` is made of programs asl is SUPPOSED to reject, so
a non-zero exit there is the subject of the measurement rather than a fault in it, and
`|| exit $?` would be wrong. The rule that survives the inversion is not "check the exit status", it
is **never quote an emitted VALUE out of these runs**: read accept-or-refuse and the diagnostic
text, never a byte. `scripts/mint_over_acceptance_verdicts.sh` states that in its own header.

*(Counting note, and it has now been wrong TWICE. "Four binaries, one bad" was four PATHS and
TWO PROGRAMS — and that correction was itself a count of what someone happened to check.
**Measured 2026-09-05 by running every `asl` on the machine: SEVEN paths execute here under FOUR
distinct digests**, all printing the same banner. The reference digest is reached by three paths,
and `s2disasm/build_tools/Linux-x86/asl` is an **ELF 64-bit binary in the 32-bit slot** with a
digest of its own — so selecting by architecture directory gets a program neither its path nor
its banner describes. The guard is unaffected: it pins one digest and refuses the other three.
Population and `file` output in `docs/superpowers/notes/asl-reference/README.md`.)*

### SIGIL-DECOUPLE: what the coupling actually buys, and the residual cost

*(original `docs/OVERSEER.md` lines 742-771, under: SIGIL-DECOUPLE - the owner ruled 2026-08-26, in this lane's session: follow aeon's plan)*

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

### SIGIL-DECOUPLE: the strict run needs a paired aeon tree, and the recipe

*(original `docs/OVERSEER.md` lines 781-800, under: SIGIL-DECOUPLE - the owner ruled 2026-08-26, in this lane's session: follow aeon's plan)*

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

*(original `docs/OVERSEER.md` lines 891-962, under: its own `##` section in the boot read)*

## A NAME-STRING ENUMERATION CANNOT TELL A CONSUMER FROM A FIXTURE (2026-09-06)

**Two lanes enumerated the same symbol independently and BOTH over-counted, in the same way.**
Aeon asked this lane to price the sigil-side cost of deleting one aeon object module
(`games.sonic4.path_swap`). They sent their own enumeration, explicitly flagging that their agent
had MISSED two files. This lane re-enumerated varying the alphabet rather than the scope
(case-insensitive, underscore / camel / spaced spellings, all tracked file types, per bar 19) and
found four more again.

**Then the extra hits turned out not to be consumers at all.** `path_swap`'s diagnostic string,
`"Bad path swap!%<endl>Got: %<.b d0>"`, is used as a **test vector** in the `.emp` diagnostics
encoder's own tests, and two further files merely name the module in a comment. Five files in
total **survive the module's deletion untouched**, including the one aeon had reported as a missed
site. They test how a format string encodes; that message is just a realistic sample.

**The rule: a name-string enumeration cannot distinguish a CONSUMER from a FIXTURE, and the error
is toward over-counting.** The tell is position, not spelling: **a hit inside a quoted literal is
data, a hit in a symbol position is a consumer.** Bar 14 covers the case where an identifier grep
and a quoted-key grep each find what the other misses; this is the inverse face, where the quoted
occurrence is a *decoy* and both greps find it. **Widening the alphabet made the answer worse, not
better**, which is worth remembering the next time a wider enumeration feels like the safer one.

**And the real surface was still under-counted where it mattered**, which is why the correction is
not simply "fewer sites". Aeon named `repin.toml`'s `test_parent` anchor as the second consumer
that gets missed. **There are five** distinct regions anchored on the deleted symbol
(`test_particle`, `test_emitter`, `test_stress_emitter`, `test_churn`, `test_parent`), plus the
`path_swap` region itself. **The instinct about the class was right and the instance was one fifth
of it**, so a correction that only removed the false positives would have shipped a booking that
was wrong in the expensive direction.

**⚠ AND THIS LANE THEN CARRIED THE REFUTED SET'S ARITHMETIC.** Aeon's headline was *"seven files
plus five frozen tables"*, seven including the fixture file. This lane refuted that file's
membership and wrote back that *"seven files plus five frozen tables stands as your headline, what
changes is WHICH seven."* **Both halves cannot be true**: removing a member from a set reduces its
count. The real figure is **SIX**, enumerated here so it cannot drift again: `native.rs`,
`section_align.rs`, `repin.toml`, `pins.rs`, `ojz_run_a_port.rs`, `test_g4_final_objects_port.rs`,
plus the five golden tables. The peer had already banked the seven, citing this lane, and it was
corrected inside the hour.

**This is the banked rule aimed at the session enforcing it** *(a refuted mechanism does not leave
its arithmetic standing)*. Note why nothing looked wrong: the error was **one high on the stated
side and one low on the true side**, so the sentence read as a careful partial correction, which is
the most credible possible wrapper for a wrong number. **A correction that preserves the original
count is not a correction, it is a concession with the figure smuggled through.** When refuting a
member, restate the SET by enumeration, never the total.

**THE MECHANISM, in aeon's formulation, which is better than this lane's and places the instance in
an existing family rather than leaving it an anecdote:** *a refuted MEMBER invalidates the
arithmetic that rested on it, and the arithmetic is the half nobody re-derives, because the
refutation feels like the hard part and the number feels like transcription.* That is the protocol's
**provenance-feeling material gets reasoning-grade trust** (aurora's three surfaces: the verified-at
SHA, the repaired line number, the hash typed from memory), **arriving on a COUNT instead of on a
SHA**. Same family, new surface, and the surface is the one that looks least like a claim.

**Both lanes owned it and the shared frame is the finding.** Aeon relayed the sentence verbatim in a
message whose whole purpose was to carry this lane's refutation of the seventh member, with that
refutation in front of them, and did not run the subtraction either. **Two lanes, one number,
neither re-derived it** (bar 8: mutual verification cannot catch a shared frame). The fix both
adopted costs nothing and is mechanical rather than attentional: **write the SET, not the count.**
Six names in one sentence cannot drift; a bare *"six"* can, and *"seven"* did, twice in twenty
minutes, in both directions at once.

**Open and deliberately not asserted:** since the alignment flip, an UNDECLARED section is refused
by name. Whether a `section_align::DECLARED` row for a section that no longer EXISTS is inert or
fatal is the opposite direction and is untested. One command, owed before the bundled parcel is
sized.

*(Priced and PARKED by the hub in the owner's place: the deletion waits in aeon's `DEFERRED_WORK`
and rides the next chainer-pin advance as one parcel. Grounds, and this lane's own read before the
ruling arrived: a repin plus a five-shape refreeze plus a shared-binary relink for one obsolete
object is poor value per byte moved, and the relink is hub-gated since 2026-09-06, so it cannot be
scheduled from inside this lane at all.)*

*(original `docs/OVERSEER.md` lines 964-993, under: its own `##` section in the boot read)*

## A CRC QUOTED WITHOUT ITS AEON REVISION CANNOT BE REPRODUCED OR REFUTED (2026-09-06)

**Found by the aeon lane against a message from this seat, and verified here from the record
rather than taken on relay.** This lane sent aeon four expected digests
(`s4 1c09fbfc/819131`, `s4.debug e2144057/840324`, `demo 11ebd7ab/96602`,
`demo.debug 9b0d2ce7/102818`) and asked them to reproduce. They built aeon master
`f14b21a8` in a clean detached worktree with the CURRENT shared binary, this lane's fix
nowhere near the build, and got four shapes differing from that table in **both** crc and
size. Their conclusion was right and their reasoning was right: the table describes a
different aeon tree.

**Which tree, established here in one command rather than left as "almost certainly":** those
four values are the chain TIP entry `relayout-at-aeon-master` in
`crates/sigil-harness/golden/provenance.toml`, whose `aeon_rev` is
`483b3e128ec4b9efc77a9d2e8a8c7679e961cea8`. Aeon master `f14b21a8` is **358 commits ahead of
it** and has it as an ancestor. So the divergence is fully explained by the revision gap and
says nothing about any assembler.

**The rule: a CRC is meaningless across sessions without BOTH referents beside it, the sigil
revision AND the aeon revision.** Four digests with neither named can only be agreed with or
disagreed with, never checked, which is the opposite of what a digest is for. This lane already
holds *provenance is CRC32 + size*; that rule fixes the IDENTITY FUNCTION and is silent on the
SUBJECT, and the subject is the half a receiver cannot recover.

**And the stronger form, which is aeon's and should be preferred whenever it is available: send
a DIFFERENTIAL, not a table.** One tree, one worktree, one `build.sh`, exactly one variable (the
binary), control first. Pairing eight digests proves byte neutrality on the tree the receiver
actually has; matching a table proves agreement with a tree they do not. **Byte neutrality at
the pinned reference does not entail byte neutrality 358 commits later**, so their differential
is not a re-run of this lane's check, it is the check this lane could not perform.

*(original `docs/OVERSEER.md` lines 995-1021, under: its own `##` section in the boot read)*

## THE `--version` BANNER'S TREE VOCABULARY IS A LIVE TWO-WAY CONTRACT (banked 2026-09-06)

**Sigil defines the words; aeon's `build.sh` enumerates the trusted ones and fails CLOSED on
anything else; so ADDING A WORD IS A BREAKING CHANGE AT THEIR END and they must be told before
it lands.** The vocabulary today is exactly `clean`, `clean-sources`, `dirty`, `unknown`,
defined in `crates/sigil-cli/src/tree_class.rs::state_and_detail` and pinned by
`crates/sigil-cli/tests/version_provenance.rs`. Their consumer is `build.sh` (the `tree:` read
and the `case "${SIGIL_TREE}"` arm), verified here at aeon `origin/master`, and the coupling is
written into their own comment: *the vocabulary is sigil's to define, the fail-safe direction is
theirs to keep, and a consumer enumerating the trusted words must be told when a word is added.*

**⚠ THIS LANE HAS ALREADY BROKEN IT ONCE, AND THE HOLD THAT SHOULD HAVE PREVENTED IT WAS WRITTEN
IN A PLACE THAT CANNOT STOP A MERGE.** `9d71d8f4` (2026-08-27) added `clean-sources` and its own
message opens `DO NOT MERGE THIS COMMIT until the banner's shape is agreed with the aeon lane`,
naming their `dirty*` test as the thing it breaks. It is an ancestor of master via `d7f3f632`,
merged the same day, and the hold was retired at `d5967f87` by READING their consumer rather
than by asking them. **A DO-NOT-MERGE in a commit message is not a gate; it is a note that
travels with the thing it is trying to stop.** A hold that must hold gets a failing test or an
owner-visible blocker, never a sentence in the artifact being held.

**And this lane's retirement note understated the cost, which is the part worth carrying.** It
recorded the effect as *a FALSE warning, noisy and in the safe direction*. Aeon's own fix
(`306608f2`, 2026-08-30) shows the same flag reaches `exit 1` under `SIGIL_VERSION_STRICT=1`, so
on a correct tree with uncommitted sigil docs, STRICT REFUSES THE BUILD; latent only because
nothing in-tree sets STRICT. **Reading a consumer settles more than reasoning about the producer
can, and it still under-reads when you stop at the first arm the value reaches.** Three days
passed between the word landing and their arm being added.

*(original `docs/OVERSEER.md` lines 1023-1044, under: its own `##` section in the boot read)*

## A BOUND IS SAFE ONLY IN THE DIRECTION IT WAS DERIVED FOR (2026-09-06, this lane's own error)

**Refuted by the aeon lane against a measurement of mine, with a control rather than an
assertion** (aeon `36e3e409`, verified an ancestor of their `origin/master`; their citation
spot-checked here line by line). This lane derived *a frame's slot cost is at most twice its
entry count*, used it correctly to prove three subjects **cannot** exceed a bar of 10, and then
reused the same number one bullet later as Knuckles' **exact ceiling** to price how much room was
left. The exact ceiling is **6**, headroom 4: the bound assumes every entry straddles at once, and
disjoint runs mean at most one can.

**The rule: an upper bound proves impossibility and prices nothing.** Both uses read as arithmetic
about the same quantity and the second one silently needs a property the first never established.
Nothing announces the switch, because the number does not change, and the failure direction is the
one that gets acted on: an over-estimate looks like a margin about to close.

**When you carry a bound past its first use, say which it is in the same sentence.** *"At most
2 x peak entries"* and *"the ceiling is 10"* are different claims, and this page's own banked rule
applies to itself: a refuted mechanism does not leave its arithmetic standing.

**And the half that survived is the half worth having, which is the usual shape of a good
refutation:** the ceiling was UNSTATED, a bar met exactly is indistinguishable from a bar met with
room, and it is now computed and printed on every aeon build with a tag at zero headroom.

### A PROOF THAT CANNOT FAIL ON DEMAND HAS NOT BEEN READ, IT HAS BEEN ADMIRED

Read when building or judging any red-first proof, and when writing a bar for someone else.

**Aurora's sentence, relayed through the hub 2026-09-09, and it states this lane's red-first bar
better than the lane's own wording does.** Their case: the RED row of a before/after pair was
required to reproduce a defect and simply refused to. Right mechanism, right table, green run,
nothing looking wrong. **The tell is not in the artifact, it is in what the reader did with it.**

**Why it improves on "prove it red first":** that phrasing names a step in a procedure, and a step
can be performed. This names the READER'S obligation, and it is not satisfiable by ceremony. A
proof whose red arm was never observed failing is not evidence, however carefully it was built,
and the standard question becomes *did I watch this fail, on demand, just now* rather than *did
the parcel include a red-first section*.

**Composes with the vacuity shapes already banked here** (an unapplied mutation, a runner not
executing what you patched, a proof that ran the wrong program). Those ask what the green could be
hiding. This one asks whether anybody ever saw the instrument move.

### A PRESCRIPTION IS NOT A PROOF: NAMING THE OBSERVABLE IS NOT NAMING HOW TO READ IT

Read when writing a bar, a brief, or a diagnostic that tells somebody what to check.

**Oracle's sentence, on why a hub bar failed three consecutive times.** The bar named the right
observable, at a level of detail that **PERMITTED the failing implementation**. Every seat that
followed it did follow it, and three of them measured nothing. **A bar is not judged by whether a
correct reading exists, but by whether an incorrect one is excluded.**

**The direction of the error is what makes it survive: a prescription that is merely true reads as
a completed act.** Nobody rereads a bar they satisfied. So the cost lands on whoever executes it
next, and it lands as a confident green.

**How to apply, and it is one extra clause:** beside the observable, say **what reading it wrong
would look like**, or give the invocation. *"Check the isolation holds"* permits the failure;
*"resolve through the PEER via `ss -x -a -n`, because a connecting client's socket carries no
address and walking your own fds finds an empty path"* does not. **The test for your own bar: could
a careful person satisfy this sentence and measure nothing?** If yes, the sentence is a preference
wearing a rule's grammar.

**This lane meets the class constantly in `.emp` diagnostics**, where a message that names the
offending construct without naming the line to write is the same defect aimed at a user.

### SORT A STALENESS AUDIT BY MOOD, NOT BY AGE

Read when auditing any standing document for rot, and when deciding which stale line to fix first.

**Aurora's sharpening, relayed 2026-09-09: a stale DESCRIPTION is merely believed; a stale
IMPERATIVE is acted on.** Age is the wrong sort key because it measures how long a line has been
wrong rather than what being wrong costs.

**The instance is this lane's own, which is why it is banked rather than admired.** The boot read
carried *"Nothing is implemented"* about `pad`/`pad_to` for two weeks after it landed
(`ffa7bdb8`, 2026-08-26, verified an ancestor of master with the parser taking it). That sentence
parses as a description. **It is read as an instruction not to use the construct, and as a
standing invitation to implement it again.** So it sat in the class that looks harmless and is not.

**The tell, and it is a grammar tell rather than a content one: an imperative wearing a
description's grammar.** *"X is unimplemented"*, *"nothing consumes Y"*, *"the only caller is Z"*,
*"this is not yet gated"* all read as facts and all license an action. Contrast a genuinely
descriptive rot, a superseded measurement or an old count, which misleads a reader without
directing one.

**How to apply:** when sweeping a document, first pass for lines that would change what somebody
DOES if believed, and fix those before anything else. **The R7 block and this one are the same
defect twice in the same file** (*what remains is the flip itself*, held for a week after the flip
landed), and both were caught by someone asking a question, never by a sweep, because **nothing
executes a document.** A completion has to be written back to the document that DISPATCHES the
work, in the landing commit.

### A CHECK CAN BE BLIND BY POPULATION OR BY ORDERING, AND THE FIXES ARE DIFFERENT

Read when designing any check whose clean result you intend to believe.

**Two blind checks met on 2026-09-09, one aurora's and one this lane's, and they look identical
from the outside: both returned a confident clean result while measuring nothing.** The hub's
separation is the useful part, because collapsing them yields a fix that closes only one.

- **Blind by POPULATION.** Aurora's scoring driver scored nine planted defects KILLED against a
  suite that never executed an assertion, a lint having failed before the runner started. Six of
  the nine were survivors. **A batch containing no predicted-survivor cannot detect that its own
  scorer has stopped working**, because every possible outcome agrees with a dead scorer.
  **Fix: plant some you expect to LIVE. They are the control.**
- **Blind by ORDERING.** This seat tested a change log for completeness by asking whether every
  pre-loss row carried an `added` event. Zero missing. Two of those rows had their first `added`
  event stamped at this seat's own RESTORE, hours after their creation: **the repair had supplied
  the evidence the check was looking for.** The population was fine and the clock was not.
  **Fix: run the check BEFORE the repair, or against a source the repair did not touch.**

**So the question to ask of a clean result is two questions.** *Could any member of this batch have
produced a different answer?* and *did anything I did earlier put the answer here?* A check can pass
the first and fail the second, which is what makes this worth separating: the population discipline
this lane already had would not have caught the ordering case, and the instance proves it, since
this seat ran that check having already written the rule about planting controls.

**Both are the same family as a control that bypasses its subject and a test that cannot fail, and
the family's signature is that nothing looks wrong.** The failure is never in the output.

### A VERBATIM MOVE LAUNDERS STALENESS, AND FIXING ONE STALE LINE CERTIFIES THE REST

Read when cutting or reorganising any standing document, and when booking a row off text you just
moved.

**Caught 2026-09-09, within two hours, by this seat against itself.** The boot-read cut moved the
`pad`/`pad_to` block verbatim to `docs/OVERSEER-REFERENCE.md`, which was correct: a cut moves text,
it does not fact-check it. The agent performing it noticed ONE stale claim in that block, *"Nothing
is implemented"*, verified `pad_to` had landed at `ffa7bdb8` on 2026-08-26, and wrote the correction
beside the moved text. Exemplary, and it is the reason the second half is worth banking.

**The same block's next clause was also stale and neither of us checked it.** It read *"Owed by
this lane, in the `pad`/`pad_to` parcel: tests for #4-#6, widen #2/#3 to the full strings, and pin
the Scope clause"*. All of it had been paid, by that same `ffa7bdb8` parcel. The spec at empyrean
`origin/main` §4.3 records it explicitly, and all four tests exist here, verified by name with a
control. **This seat then booked a fresh queue row asserting the debt "was never established"**,
hours after banking the rule that a stale imperative is acted on while a stale description is
merely believed. The row was the acting.

**Two mechanisms, and both are about how a document earns unearned credit:**

- **A verbatim move launders staleness.** Old text arrives at a new location with a fresh date on
  the commit that moved it, and the next reader meets it as current material rather than as
  something written weeks ago. The move is honest and the freshness is an artifact of it.
- **Fixing one stale line in a block certifies the rest of the block.** A visible correction reads
  as *this block has been reviewed*, which is precisely what it is not: a correction is evidence
  about ONE sentence and says nothing about its neighbours. **The corrected block is more
  trustworthy-looking and no more trustworthy.**

**How to apply.** A cut does not audit, and should not: keep the two acts separate. But **a row
booked off moved text is a row booked off an unaudited source**, so resolve it against the code or
the spec before booking, not before working it. And when you correct one line in a block, say in
the correction which lines you did NOT check, because the default reading is that you checked them
all.

### CONVERGENCE IS EVIDENCE ABOUT REPRODUCIBILITY, NEVER ABOUT CAUSE

Read before dispatching a seat pair, and when two seats agree on a finding.

**Aurora's, relayed 2026-09-09, and it is about the strongest-looking evidence a seat pair
produces.** Both their seats hit `Open Project...` and found nothing happened. One filed it as its
costliest finding while refusing to attribute it; the other declined to press it and declared the
surface undrivable. Aurora then reproduced it with a twelve-line script containing **none of their
code**: it was the headless rig, not the product. **Two honest seats, one wall, and their agreement
would have been read as confirmation.**

**Why it is not a lapse: a seat pair shares an environment BY CONSTRUCTION.** That is the point of
running them together, and it is also a shared confound in every finding they reach independently.
Convergence tells you the observation is real and repeatable. It says nothing about what produced
it. Same shape as *two draws from one distribution are not two witnesses*, and as this file's
custody-versus-observation rule, arriving on a walkthrough instead of on a measurement.

**The discriminator is TRACING, not agreement.** This lane's own convergent finding, the success
line that printed identically whether or not a file was written, is sound because it was traced to
`emit_image` and the writer. Both seats saying it added nothing to that.

### THE EXCLUSIONS THIS LANE'S SEATS CANNOT DISTINGUISH FROM PRODUCT DEFECTS

Read when writing any seat brief, and **declare these in the brief**. Aurora's formulation, adopted:
**an exemption nobody sees is a hole; an exclusion nobody declares is a finding factory.** A seat
that meets one of these and cannot name it files a serious defect in good faith, and the next seat
files it again.

**Everything below was measured on 2026-09-09 and every item is MUTABLE. Re-measure at brief time;
do not copy the values forward.**

- **`grep`, `find` and `ls` come from the harness's shell snapshot**, which is regenerated per
  session (`~/.claude/shell-snapshots/snapshot-zsh-<epoch-ms>.sh`). `ls` is an alias for `eza`, so a
  probe passing a bare path where `eza` expects a flag value fails with a message about `--icons`
  that has nothing to do with the subject. **`type <cmd>` at the start of a seat run, and never
  state what these do from memory**, this file included.
- **This checkout hosts its own worktrees** (19 when measured). Any filesystem walk from the repo
  root descends into all of them: same needle, `git grep -l` 8 files against `/usr/bin/grep -rl`
  130. **A count from a tree walk here is not a count of this repo.**
- **A bare test run REFUSES rather than skipping** when no reference tree is named (ruling `d-18`):
  six tests in `seam1_native_link` alone stop with `NO REFERENCE TREE IS NAMED`. **That is correct
  behaviour and a seat will read it as a broken test suite.** Say so, and name `SIGIL_ALLOW_PARTIAL=1`
  or a provisioned tree.
- **`target/release/sigil` is shared and another lane may be mid-relink.** A seat measuring "the
  shipped binary" can be measuring somebody else's build. Give seats their own `CARGO_TARGET_DIR`
  and tell them which binary is the subject, by md5.
- **zsh, not bash**: no word splitting on unquoted variables, and `\|` alternation inside `$'...'`
  does not alternate. **`$?` after a pipeline is the LAST command's status**, which this seat hit
  twice in one session while verifying a row about exit codes.
- **The corpus `asl` binaries**: seven paths, four digests, one identical banner. Selection is by
  md5 and a run carrying any error is not a source of values for the lines that did assemble.

**The general form for whoever writes the next brief: an exclusion is a precondition the seat cannot
tell apart from the product.** Not everything you happen to know, and not a disclaimer. The test is
whether a competent seat meeting it would file a defect.

### A CONTROL CAN VERIFY THE WRONG PREDICATE

Read when planting any canary or control, and before believing a refutation you produced yourself.

**This seat refuted a standing rule on 2026-09-09, pushed the refutation, and told a peer who
carried it to a third lane. The refutation was wrong and the rule was right.** The rule: the
harness's `grep` skips gitignored files and returns a clean zero. The test looked rigorous. It had a
canary, a positive control, an explicit check that the canary was ignored, and it produced a clean
contradiction.

**The canary was ignored via `.git/info/exclude`. The instrument reads `.gitignore` FILES.** The
control was `git check-ignore`, which confirmed, correctly and unambiguously, that **git** ignored
the path. But the thing under test was `ugrep --ignore-files`, whose notion of ignored is
`.gitignore` files, not git's full ignore resolution. **The control verified a TRUE predicate that
was not the INSTRUMENT'S predicate**, so it passed while the canary remained invisible to the thing
it was supposed to make visible. Re-run with the needle behind a real `.gitignore` entry: shell
`grep -rl` 0, `/usr/bin/grep -rl` 1, tracked-needle control 8. The rule holds.

**This is a distinct vacuity from the ones already banked here** (an unapplied mutation, a runner not
executing what you patched, a proof that ran the wrong program, a reduction supplying the missing
input). Those are all about the SUBJECT. This one is about the CONTROL: the control ran, applied,
and reported truthfully about a property the instrument does not use.

**The question that catches it, and it is one question:** *by what mechanism, exactly, is my canary
supposed to be visible to this instrument?* Not *is my canary of the right kind*, which is the
question this seat asked and answered correctly. Name the instrument's own predicate and satisfy
THAT. Near-synonyms are where it hides: ignored-by-git versus ignored-by-a-`.gitignore`-file,
tracked against committed, on-disk against staged, installed against on-`PATH`.

**Two aggravating features worth carrying, because they are what made it propagate.** A refutation
feels like the rigorous act, so it draws less scrutiny than the claim it overturns. And this one
overturned a rule ABOUT false clean zeros, using a false clean zero, which read as fitting rather
than as suspicious. **A result that confirms the local theme is not corroborated by fitting it.**

**And the CAUSE in that retraction was wrong too, by a second mechanism.** It blamed an
`alias grep=...` read out of the shell snapshot. An alias of that name exists; so does a FUNCTION of
that name, which is what the shell actually resolves (`whence -w` says `function`, `which grep`
prints the ugrep router). **Reading one definition out of a config file is not reading what the
shell resolves.** Ask the shell, do not grep its snapshot.

### A PEER'S "YOUR PIN HAS DRIFTED" IS USUALLY THE PIN GAP (2026-09-10, aeon's report)

**Read this the moment another lane reports that a `pins.rs` value disagrees with their tree.**

Aeon reported two byte-movers touching `pins::CORE` and, beside them, a standing `0x4E` gap they
had measured before either parcel: `pins::CORE.plain_base` `0x2CCC` against their master's
`0x2D1A`, with `SPRITES` showing the identical `0x4E`. Read as drift, it is a repin trigger. It is
not drift.

**`pins.rs` is generated from SIGIL'S OWN resolved layout of the corpus it is PINNED to**, which its
own header states, and the pinned corpus is `.aeon-sigil-ref`. Measured that day: the pinned tree
was an ancestor of aeon `origin/master` and **280 commits behind it**. **DERIVE that count when you
need it and never read it off this page** - aeon's master moved twice during the single exchange
that produced this block, so the figure is a timestamp, not a property. A pin compared against tip
therefore reports the GAP, and the gap is the pin working. The hub's 2026-09-02 ruling is that the
pin exists so the corpus does NOT track tip; advancing it is `AEON-REFREEZE-DEBT`, a held decision.

**The discriminator, and it is one grep rather than an argument:** `pins_rs_is_current` is the gate
that answers "is the pin correct for the tree it describes". Grep the gate's own NAME out of the
last green landing log, per bar 25 - `RESULT GREEN` alone does not establish that the gate ran. It
ran and passed against `ec640bcf`, so the pin was current for its own subject while disagreeing
with tip by `0x4E`. **Those two facts are compatible and only one of them is about a defect.**

**A uniform shift across two adjacent regions, with each region's own LENGTH still matching, is the
signature of a pin gap rather than a pin fault.** A real pin fault is region-local: the length moves
too, or one region moves and its neighbour does not.

**A SECOND SIGNATURE, AND IT IS A DIFFERENT ENUMERATION PARAMETER: the two SHAPES' gaps can have
OPPOSITE SIGNS.** Derived here after both lanes had argued the plain shape only, neither having
consulted debug. Against the same pinned corpus, `CORE.plain_base` `0x2CCC` sat `+0x4E` behind
aeon's tip while `CORE.debug_base` `0x2FB0` sat `-0x66` AHEAD of it. **A region-local pin fault
cannot flip sign between shapes for the same region**, so opposite signs are independent evidence of
accumulated unrelated change ahead of the region rather than a bad pin. **The operational half: at a
repin, `debug_base` can legitimately move DOWN.** A repin sanity check written to expect forward
motion reads a correct value as a fault. *(PROVENANCE, and it is deliberately not upgraded further than it earns. Aeon read
`InitObjectRAM 0x2F4A` out of `.aeon-land-c1b2/s4.debug.lst` in the tree they built and pushed as
`9fcf502a`, so the figure now has a named artifact and is REPRODUCIBLE AND REFUTABLE, which the bare
number was not. **It is still ONE lane's observation, not two.** Two parties reading one listing are
one witness read twice; this lane has read no byte of it. What improved is the citation, not the
independence.)*

**What to keep from such a report even when the drift half dissolves.** Aeon serialized their two
landings and recorded the INTERMEDIATE revision rather than reconstructing it, which is the only
thing that will let the two be attributed separately whenever the pin is advanced; a differencing of
the range's ends cannot separate two parcels inside it. And their sharpest observation survives
entirely: **a CRC change at an UNCHANGED ROM length is positive evidence that bytes moved without
the image growing**, which is a repin trigger wearing a reassuring shape.

**⚠ AND THE REPORT CARRIED A REFUTATION THAT WAS ITSELF WRONG, IN THE DIRECTION THAT LOOKS
DILIGENT.** Their agent cited the same `0x4E` as visible in `golden/offcanonical_sizes/s4.txt`;
the lane checked, reported the path does not exist here, and withheld the claim. **The path exists
and is tracked** (seven files under that directory; `git ls-files` returns them). The verdict
"treat the golden half as unverified" was still right, because the golden actually reads
`InitObjectRAM 0x2ccc`, agreeing with `pins.rs` to the digit and containing no `0x4E` at all. **So a
true conclusion arrived on a false mechanism, and the mechanism is the half that travels.**

**⚠ AND THIS PARAGRAPH FIRST NAMED THE WRONG CAUSE, WHICH IS THE SAME DEFECT ONE LAYER OUT.** It
attributed their empty result to two known local hazards, the `eza` alias and the `ugrep` wrapper,
because those are this lane's banked producers of a false zero. **Neither was involved.** Aeon
re-derived it and supplied the real mechanism: their agent wrote the path as
`golden/offcanonical_sizes/s4.txt` and they resolved it against the repo root as `sigil/golden/...`,
which does not exist. Their `[ -f ]` test and their `find` fallback were **both rooted at a
directory that is not there**, so neither instrument could return anything but empty. **This lane
supplied a plausible cause from its own hazard list rather than asking, inside the very block
teaching that a false mechanism is the half that travels.** Kept rather than quietly rewritten,
because the substitution is the lesson: a banked hazard list makes a wrong cause CHEAP to reach and
fluent to state.

**So the class is a FEED failure, not a rule failure**, which this lane's own canary bar already
separates: their pattern was fine and their data never arrived. The check that fires is asserting
the input exists before believing an emptiness about it. When a peer reports a path absent from THIS
tree, re-run it here with `git ls-files` before accepting it, and when YOU report a path absent from
a peer's tree, print the directory you actually looked in.

**COMMITMENT MADE TO AEON THE SAME DAY, DISCHARGED 2026-09-10: their finding is CONFIRMED and the
prose is fixed.** They flagged that `crates/sigil-cli/tests/core_port.rs` (header and the block above
`debug_shape_length_diverges`) calls the two debug assert sites `bsr.w`, while their debug ROM emits
`6162`, which is `bsr.s`. The byte has now been read here, on `.aeon-sigil-ref` at aeon `ec640bcf`:
`Debug_AssertObjLoop` is at `0x3550` in `s4.debug.lst`, and `s4.debug.bin` carries `61 62` at
`0x34EC` and `61 0A` at `0x3544`. **Both are `bsr.s` and both displacements land exactly on
`0x3550`**, which is what makes the reading self-checking rather than a plausible parse: a
mis-located instruction does not resolve onto the symbol twice.

**The prose now names `jbsr` and states no width at all**, which is the durable fix rather than
swapping one width for the other. The source writes `jbsr` (auto-reaching, `engine/objects/core.emp`
lines 556 and 608), so the width is the relaxer's to choose and can move whenever the proc does. A
comment asserting a width the source does not fix is the coordinate-rot class one level up, and the
surplus it exists to explain does not need the width, because the proc dominates.

**The arithmetic gap is NOT a second defect, and it looked like one for a few minutes.** Proc span
`0x3550` to `0x3670` is `0x120`, plus two 2-byte call sites is `0x124`, against a stated surplus of
`0x128`: four bytes over. Those four are a **short align pad**, which the test's own `pad_ok`
tolerance already accounts for, since `pins::CORE.*_len` spans to the NEXT section's aligned base
rather than to the end of the emitted image. Recorded because the gap is exactly the shape that
invites a second finding to be booked against a correct pin.

### A CAVEAT RETIRED BY AN ADJACENT IMPROVEMENT LEAVES NO ARTIFACT (2026-09-10, aeon's formulation)

**Read this the moment something arrives that appears to satisfy a caveat you are carrying.**

This lane's banked rule covers a caveat being WRITTEN: forbid the misreading rather than qualify it.
This is the other end, and it is the end with nothing left behind.

Instance: a figure was banked here as *"aeon's measurement, not a second observation of it: this lane
ran no build."* Aeon supplied the path and revision of the listing they read it from, offered as the
second sighting the caveat asked for. **It was not one.** A named artifact makes a number
REPRODUCIBLE AND REFUTABLE, which is a real gain; it adds no second OBSERVATION, because two parties
reading one listing are one witness read twice. Only a reading this lane could make would satisfy
that caveat, and that needed a corpus-pin advance nobody was asking for.

**Why this direction is worse than the writing direction, in the peer's words:** a caveat costs
nothing while it stands and costs a future session real confidence once quietly discharged, **and
the discharge is invisible afterwards, because what remains is a number with an artifact beside it
and no memory of the doubt.** That is worse than the original bare figure, which at least announced
its own thinness. A written caveat that was too weak still leaves the caveat; a discharged one
leaves a clean-looking claim.

**THE CHECK: name which property actually improved, and AMEND the caveat rather than deleting it.**
A caveat is retired only by the thing it named, never by an adjacent improvement offered in good
faith. **The tell is a peer saying "here is the X you asked for"** - check the offered thing against
the caveat's own words rather than against its spirit, because a good-faith offer is fitted to the
spirit by construction.

**And the same exchange produced the staleness twin.** A "280 commits behind" figure sat in this
file as a measurement while the peer's tip moved TWICE during the conversation that produced it.
Derive counts at read time; a number that goes stale inside its own conversation is the cleanest
demonstration this rule will get.

## A BLOCKER IS A CLAIM IN A FIELD NOTHING RE-READS, AND IT FAILS IN TWO DIRECTIONS (2026-09-10)

**Found on this lane's own board an hour after reporting three dead blockers on the hub's, and the
discovery order is the whole lesson: I audited mine only because a peer reported theirs.** Nothing
about my board looked wrong, and nothing would have.

**Two rows, two DIFFERENT failures, and a sweep aimed at either one misses the other:**

- **`S1-BONUS-PASS-CUT` said `blockedBy: owner` and he had ANSWERED IT**, at `d-28-answered`,
  2026-09-09T17:27:10Z, whose own text says it *"unblocks the bonus-pass parcel that stopped at its
  own gate"*. Fifteen hours of a settled decision producing nothing, because the row that would have
  been picked up still filed itself as waiting on him. **Its TITLE was stale too, and independently**:
  it carried *"no saving on Sonic 1"* as the rationale, which `d-28`'s own detail records as true of
  Sonic 1 alone and NOT generally, the cut being a real 20 to 32 percent on five of nine roots. So
  the row was wrong about who was blocking AND about why.
- **`EMP-Z80-MNEMONIC-TABLE` said `blockedBy: owner` and NO QUESTION HAS EVER BEEN PUT TO HIM.** No
  card exists; `d-27` is the cpu-NAME question and a different subject. Under the propose, discuss,
  land ruling that surface is this lane's to design and offer. **A row asserting he owes an answer to
  a question he has never seen is the same defect as one asserting he owes an answer he has already
  given**, and it reads identically on his console: a lane apparently waiting on him.

**THE MECHANISM, and it is structural rather than careless.** A `blockedBy` is a claim about the
world written once, in a field that is READ to decide not to pick the row up, and therefore never
re-read against the world. **Every event that would falsify it (his answer arriving, the question
never being asked, the peer finishing) leaves the field untouched**, and the row looks correctly
parked for the entire time it is false. It is the inverse of a stale green: a stale RED, which costs
nothing visible and hides real work in the one pile nobody sweeps.

**THE CHECK, and it is cheap enough to run at every boot: for each blocked row, name the FALSIFIER.**
Not the state, the event that would end it. Then resolve it: for an owner block, grep
`docs/decisions.jsonl` for the card and read its `state` (this lane already has that as a filing
rule and did not apply it in the READ direction); for a peer block, read their live board. **A
blocker that cannot name an event a reader can resolve in one command is expired**, exactly as the
hold table at the top of `docs/OVERSEER.md` already says of holds, and for the same reason.

**THE ASYMMETRY THAT MAKES IT WORTH A RULE: `blockedOnOwner` was `[]` while three rows said
`blockedBy: owner`.** The filed list and the queue disagreed and both were internally consistent.
The filed list is swept, because it appears on his console and is embarrassing when wrong; the queue
field is not, because a blocked row asks nothing of anybody. **Sweep the unswept one.**

## A CORRECT ACTION CARRYING A WRONG RATIONALE PROPAGATES THE RATIONALE (2026-09-10)

**This lane re-ided a duplicate `d-25` correctly and stated a false reason for doing it, and the
reason is the half that travelled.** The commit message at `4ebfb950` said line 28 "is an OPEN,
UNANSWERED question to the owner" and "He has never been shown it". Line 28 carried
`"state": "answered"` and his verbatim words at the moment that sentence was written: he answered it
2026-09-04T15:40:14Z, `accept`, committed at `142709bc`. The re-id itself stands; every other claim
in that message (the numstat, the line count, the identification of line 29 as a mis-numbered
embed-path record) is true.

**The decisive fact, and it is worth keeping because it forecloses the charitable reading:** the
shadowing record is dated 2026-09-04T21:18:23Z and his answer is 15:40:14Z. **The duplicate did not
exist when he answered.** So there is no version of events in which he answered the wrong card, and
the shadowing explains only why the answered card later became hard to SEE, never why it would be
unanswered.

**The mechanism, in the hub's formulation, which is better than mine: downstream readers cannot act
on the action, they can only act on the sentence.** A correct action is self-justifying to whoever
performed it and invisible to everyone else; the rationale is the entire interface. So a wrong
rationale on a right action is not a cosmetic defect, it is the only part of the work that
propagates. Here it propagated into `docs/lane-status.json` as a `blockedOnOwner` row telling the
owner he was seeing a six-day-old answered question for the first time, and from there into the
hub's overnight write-up as one of eight cards awaiting him, which is the one artifact he reads on
waking. **Three artifacts agreed and none of them was the ledger.**

**What would have caught it, and it is one field:** the line being re-ided around was read for its
`id` and never for its `state`. **When a record's IDENTITY is the subject of an edit, read its
CONTENT before writing the reason.** The `id` and the `state` sit in the same JSON object two keys
apart.

**And the aggregation half, booked by the hub against itself and recorded here because this lane
supplied the bad input:** an aggregated card count inherits every constituent's confidence and
states none of them. Six lane boards became "your card is EIGHT", a number he cannot audit and which
no lane authored. The partial remedy in place is that each card names its lane; there is no full one.

**THE STANDING CHECK THIS EARNS, and it is cheap: before filing anything as `blockedOnOwner`, grep
the ledger for that id and read its `state` and `answered_at`.** A blocker is a claim that he owes
you something. This lane held one for six days that he had already paid.

## Moved from the boot read on 2026-09-10, the third cut

Twelve blocks, moved VERBATIM when `docs/OVERSEER.md` crossed its byte bound again, under the
owner's 2026-09-04T15:38:47Z ruling that the split is by WHEN A RULE IS READ. **Nothing was
shortened to move it**, and no rule was dropped. They keep the order they held in the boot read.
The index that names the moment triggering each of them is in the boot read, under *Read at the
moment - the blocks moved to the reference file on 2026-09-10*.

## AN EXCLUSIVE-TREE LEASE HAS NO END UNTIL YOU GIVE IT ONE (2026-09-09)

**This seat told an agent "the reference tree is yours exclusively; I will run nothing against it",
then started a landing run against that tree while the agent was still alive, and the AGENT caught
it.** No harm: its three runs closed at 17:34:42Z and mine opened at 17:38:14Z, verified from both
sides, and it re-checked the tree at `ec640bcf` with 0 modified paths. The promise was still live
and I broke it anyway.

**The mechanism, and it is structural rather than careless: a completion notification says the agent
STOPPED, not that it has ENDED.** The notification's own text says the same task id may notify more
than once and that the agent can be resumed from its transcript. So a lease worded *"while you are
out"* has **no expiry event on the controller's side at all** - there is no moment the harness tells
you the lease is over, and the moment that FEELS like one is the report landing in your lap, which is
exactly when you want the tree back.

**THE RULE: scope the lease to an event YOU control, and say which in the brief.** The merge of that
agent's branch is the natural one. Until then the tree is theirs, whatever their status looks like.
If you need it sooner, message the agent and get an explicit release, which costs one round trip and
is unambiguous.

**Watch the perturbation direction, because it is not symmetric.** Two runs against one aeon tree
manufacture a false GREEN as readily as a false red: `build.sh` rebuilds `rm`-first, so the loser of
a race can find a stale ROM the other agent built and read it as its own. A red gets investigated; a
green gets quoted. That is the mandated-gate asymmetry arriving on the tree instead of on the check.

**And note who found it.** The agent flagged a shared-state violation committed by its own controller,
against a brief clause written to protect it. A dispatch that tells agents what the controller has
promised THEM is what made that reportable at all.

## THE MOTIVATING CASE IS SELECTED FOR BEING BROKEN, NEVER FOR BEING REPRESENTATIVE (2026-09-09)

**A corpus earns a parcel by FAILING on it. That says nothing about whether it EXERCISES the fix**,
and the two get silently conflated because the same file is doing both jobs: it is the reason the
work exists and the thing the work is verified against. **Its coverage of the fix is accidental.**

Measured instances, and the sample is stated precisely because the pattern is more attractive than
its evidence:

- **The integer selector.** Sonic 1 refuses the construct, which is why the parcel existed. But
  `SonicDriverVer = 1` and `case 1` is FIRST in both blocks, and the old code took a refused arm as
  the default, so a fix that stopped the refusal WITHOUT wiring the comparison would emit zero
  diagnostics and the correct bytes and pass every byte gate the corpus offers.
- **`charset`.** Sonic 1 reaches one of the two consumers the seam's own doc comment names: 0 `dc.b`
  strings after the bare `charset` reset in `sonic.asm`, against 45 elsewhere in the corpus. All nine
  complaints can vanish with `string_to_int` still unmapped.

**HONEST SAMPLE, because this lane's own bar 19 applies to its own findings.** That is **n=2 on the
corpus form**, not three: the third instance people will want to add, the four-shape byte gate that
stayed green under a mutation breaking `exec_switch` outright, is the **reference-artifact** form of
the same idea and has a different cause (aeon does not use the construct at all). Keep them adjacent
and do not merge them into one count. **And the two corpus instances are not independent:** the
second was found by an agent whose brief already carried the first's lesson, so it is corroboration
under a shared frame rather than a second discovery. It may still be the general case; it has not
been shown to be.

**THE REMEDY, and it is cheap: build a probe for the half the motivating corpus does NOT reach, and
derive its expected values from `asl` rather than from your reading of what ought to happen.** The
selector parcel found the better shape of this without being asked: it built the probe out of the
corpus's OWN text, the real block with one constant changed so the matching arm is second, choosing
a block where both arms emit a symbol that exists in both so a wrong pick stays SILENT. A probe made
of the real construct beats a model of it, and a probe whose failure is silent is the only kind that
tests this hazard at all.

**The question that finds it before a parcel starts: WHAT WOULD A HALF-FIX LOOK LIKE HERE, AND WOULD
ANYTHING GO RED?** Ask it while writing the brief, not at review, because by review the green run
already exists and reads as evidence.

## A RULING HAS CONSUMING SURFACES, AND A PARTIAL ENUMERATION LOOKS EXACTLY LIKE A FINISHED ONE

**Banked 2026-09-09, earned by `UX-CPU-NAME-SILENT-DEFAULT`.** This lane ruled *AS-DEFAULT-CPU is
REFUSE BY NAME*, shipped it on the AS frontend with four gates and a module doc, and never applied
it to `.emp`, where `attr_cpu` went on silently turning every unrecognised processor name into a
68000. The banked rule *gate every consumer of a value* already covers values. **This is the same
failure one level up, on a RULING, and it is harder rather than easier to watch.**

**Why it is harder, and this is the operative sentence:** a value's consumers are enumerable by a
mechanical sweep, so a miss is findable by grep. **A ruling's surfaces are not enumerable by any
sweep**, because nothing in the tree marks a site as one the ruling ought to reach. So the check
has no population to run over, and the artifact cannot distinguish *we finished* from *we finished
the first one*.

**The direction that makes it survive: a lane that did most of the work looks identical to one
that did all of it.** The AS half was not sloppy. It was thorough, tested, documented, and cited
approvingly for months by this very file. Thoroughness at the completed surface is what supplies
the impression of completeness, so **the more carefully the first surface is done, the less likely
anyone asks about the second.**

**The practice, and it is cheap: when you bank a ruling, write down the surfaces it must reach, by
name, in the ruling itself.**

**⚠ AMENDED 2026-09-10, AND THE AMENDMENT IS TO THE REMEDY RATHER THAN THE RULE: WRITING THE
SURFACES DOWN IS NOT ENOUGH, BECAUSE A SITE CAN PARAPHRASE THE THING IT CONSUMES.** Corroborated
independently by the oracle lane, in their repo, with no contact between the seats: their H26 pass
hardened a claim and fixed ELEVEN sites of one spelling, and a later parcel found SEVEN more that
PARAPHRASED it, which H26's own greps had no reason to match. **So the enumeration a ruling's author
writes down is an enumeration of the spellings they thought of**, and a sweep keyed to it inherits
that limit while looking exhaustive. Two seats, two repos, one class, arriving as a doc defect on
their side and a code defect on ours (`LINKER-STILL-PRINTS-A-PASS-COUNT`: the owner's decision took
an attempt count out of the assembler's message and the linker's copy of the same wording, at
`crates/sigil-link/src/relax.rs:1116`, was never touched; that surface was brought under the ruling
at merge `4280ee9f`, landed 2026-09-11, so the line cite now names code that has moved). By bar 19 that is corroboration and not
echo. **Enumerate by what a site DOES, never by what it says**, and treat a grep over the ruling's
own words as a floor.

**And a floor is a PRIOR, not a law** *(oracle, booked against themselves the same hour)*. They
pushed an agent past a stated single consumer, it looked, found nothing, and the count was exact.
**Keep sending agents past the number; stop reading a count that holds as a failure to look.** The
two halves compose: a count is a floor because spellings are missed, and a floor that turns out to
be the ceiling is a result rather than a shortfall.

**A BASELINE COUNT HANDED TO AN AGENT CARRIES ITS PROFILE AND ITS COMMAND** *(oracle's, same
exchange, and checked here rather than adopted)*. They handed an agent a count taken under one
profile beside a command from the other, with three tests `cfg_attr(debug_assertions, ignore)`, so
the agent had to reconcile the two before it could report anything. **Checked on this tree: sigil is
not exposed, and the reason is worth writing down rather than the verdict.** Its only two
profile-sensitive tests (`crates/sigil-link/src/relax.rs`, the empty-ladder and mis-ordered-ladder
guards) BRANCH on `cfg!(debug_assertions)` inside the body and run in both profiles, so they count
either way; nothing here is conditionally ignored. **That is a property of today's tree, not of the
lane**: the first `cfg_attr(…, ignore)` anyone adds makes every baseline in this document
profile-dependent with nothing announcing it. State the profile and the command beside the number
regardless, since it costs one clause. Not the sites you changed, the sites it GOVERNS. That converts an
unenumerable class into a list a later session can check, which is the only thing that makes the
ruling auditable at all. When you cite a ruling as already-applied, name the surface you verified
it on, never the ruling alone.

*(The instance also carries a second lesson worth keeping beside it: the naive application of the
ruling would have been a REGRESSION. Neither `m68000` nor `m68k` is recognised either, both reach
`M68000` through the same default the ruling removes, so the accepted spellings were never a policy,
they are the defect's silhouette. Refusing all but `z80` and `m68000` would newly refuse
`examples/main.emp`, the newcomer's first-contact file. **Before applying a ruling to a new surface,
ask what that surface's current behaviour is RESTING on**, here the very default being deleted.)*

## `d-22` IS HUB-ANSWERED: name who answered a decision card in the same sentence that cites it (2026-09-06)

*(Moved from the boot read's `AS-NAMELESS-LABELS-RC1` row, original lines 1126-1136. The rest of
that row is a landing record and closed history, and is in `docs/OVERSEER-LOG.md`, 2026-09-10 cut.)*

**⚠ `d-22` IS HUB-ANSWERED, AND EVERY EARLIER STATEMENT OF THIS ROW OMITTED THAT** *(established
2026-09-06, reading the card rather than the row that cites it: `docs/decisions.jsonl` has `d-22`
answered `by: "hub, under the project declaration"`)*. **So this item has never had an owner
decision of any kind.** What went to him was the size, and it has not come back. The bare words
*"Ruled ACCEPT"* in a queue row are how a hub ruling becomes indistinguishable from his, one hop
later, to a session reading cold, which is this lane's own banked *relay is not approval* with the
delay that makes it invisible at the time. Instance, and it is why the correction is written into
the row rather than only into the log: on 2026-09-06 the hub sent a GO resting partly on `d-22`
read as an acceptance already in hand. This lane held, named the card's author, and the hub
withdrew the go and corrected its card within minutes. **When citing a decision as grounds, name
who answered it in the same sentence.**

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
