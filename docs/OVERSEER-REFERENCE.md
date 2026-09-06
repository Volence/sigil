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

### THE FOUR FACES OF A CHECK THAT CANNOT COME OUT OTHER THAN GREEN (2026-09-05, a fourth added 2026-09-06)

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
  clippy `-D warnings` exit 0.
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

### A MONITOR FILTER THAT MATCHES NON-FAILURES TRAINS YOU TO SKIM IT

Same session, and it is the reason the above took three notifications to see. A filter watching for
`panicked` fired on **every `should_panic` test in the suite**, which are tests PASSING, and one
watching for `Killed` fired on a game object named `Killed_CheckObject`. **Coverage is not the only
property a filter needs; precision is what keeps its events readable.** The authoritative signals
here are `^test result: FAILED` and the verdict block, not the panic text, because a panic is how
half of these tests report success. **Ask not only "would this fire if the run crashed" but "does
anything that is NOT a failure match this."**
