# The 2026-09-05 boot-read cut

`docs/OVERSEER.md` was 105,389 B against a 100,000 B bound and the suite gate on it was RED on
master. Cut back under the bound by WHEN A RULE IS READ, per the owner's ruling of
2026-09-04T15:38:47Z carried in `empyrean/docs/OVERSEER-PROTOCOL.md` at `origin/main`.

Branch `parcel/boot-doc-cut-2`, from master `8e35bd94`.

## Per-file byte counts, before and after

| file | before | after | delta |
|---|---|---|---|
| `docs/OVERSEER.md` (boot read) | 105,389 B / 1,477 L | 66,177 B / 943 L | **-39,212 B** |
| `docs/OVERSEER-LOG.md` | 446,869 B / 6,540 L | 474,366 B / 6,928 L | +27,497 B |
| `docs/OVERSEER-REFERENCE.md` | 60,875 B / 837 L | 80,750 B / 1,113 L | +19,875 B |
| total | 613,133 B / 8,854 L | 621,293 B / 8,984 L | +8,160 B |

The total grows because the cut ADDS text: pointers, the boot index, and the destination
headings. It deletes nothing.

**Headroom under the 100,000 B bound: 33,823 B.** No residual, and nothing was shaved to reach
it: the honest move cleared the bound by a wide margin on its own.

## Red first, from the committed baseline

```
pwd=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a97be5a4cf1ce39d7
HEAD=8e35bd94af664db9e8ebbd6d63521710743a492b
bytes=105389
test the_bound_actually_refuses_something ... ok
test the_boot_read_is_inside_its_byte_bound ... FAILED
  boot read .../docs/OVERSEER.md is 105389 B / 100000 B: OVER by 5389 B (1477 lines).
test result: FAILED. 1 passed; 1 failed
```

Green after, at the tip of this parcel:

```
pwd=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a97be5a4cf1ce39d7
HEAD=6051fa29916d98729e302d973d34f4bfd3f7c8df
bytes=66177
test the_bound_actually_refuses_something ... ok
test the_boot_read_is_inside_its_byte_bound ... ok
test result: ok. 2 passed; 0 failed
```

The mutation between the two is the working tree itself, not a `git checkout` restore, so the
staging trap the brief warns about does not apply here. The byte count printed beside each run is
the content check that would catch it if it did.

## The losslessness proof

Instrument: oracle `tools/prove_doc_split.py`, run from THIS repo with the script by absolute
path. `4826bf5` (the table-row false-positive fix) is an ancestor of oracle HEAD, so this
checkout postdates it.

### The gating run, with its provenance lines

```
============================================================================================
INPUTS
============================================================================================
  repo     : /home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a97be5a4cf1ce39d7   (--repo '.')
  git tree : /home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a97be5a4cf1ce39d7   (git rev-parse --show-toplevel)
  original : git master:docs/OVERSEER.md   (read from git tree /home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a97be5a4cf1ce39d7)
  output   : OVERSEER.md <- file .../docs/OVERSEER.md
  output   : log_frag.md <- file .../scratchpad/log_frag.md
  output   : ref_frag.md <- file .../scratchpad/ref_frag.md
  declared new : file .../scratchpad/new_lines.md

PROOF 1 - STRUCTURE
  original non-blank lines            : 1226
  OVERSEER.md non-blank lines : 779
  log_frag.md non-blank lines : 311
  ref_frag.md non-blank lines : 223
  declared NEW non-blank lines        : 87
  outputs - new  (must == original)   : 1226
  1a ORIGINAL lines ABSENT after split: 0
  1b lines present but NOT DECLARED   : 0
  1c declared NEW but already present : 0
  1d RESULT: PASS - every original line sits in exactly one output, in its original
     relative order.
        OVERSEER.md holds 710 original line(s) + 69 declared-new
        log_frag.md holds 300 original line(s) + 11 declared-new
        ref_frag.md holds 216 original line(s) +  7 declared-new

PROOF 2 - TOKENS
  original tokens                     : 16770
  outputs - new  (must == original)   : 16770
  DELTA vs original                   : 0
  tokens LOST                         : 0
  tokens GAINED undeclared            : 0

PROOF 3 - SENTENCES
  [heading-aware] flagged in original: 8    seams INTRODUCED by the split: 0    file-edge cuts: 0  <= GATE
  [heading-blind] flagged in original: 45   seams INTRODUCED by the split: 16   file-edge cuts: 0
  derived cut points: 60   failing the predicate: 0

VERDICT: PROVED - the split is lossless by all three proofs      (exit 0)
```

**The provenance line names 1,226 original non-blank lines, which is this lane's own count of
`master:docs/OVERSEER.md`** (1,477 lines, 1,226 non-blank), and the git tree it names is this
worktree. That is the check the brief asks for before reading the verdict, and it is the one that
distinguishes this transcript from the vacuous one another lane produced by running from oracle's
checkout.

### Both extras counts were read, and the heading-blind 16 were checked

`--headings` gates the verdict on the heading-aware number, which is **0**. The heading-blind
number is **16**, and the bar is that they are heading-adjacent. **All 16 are, and zero are
prose-to-prose.** Recomputed here by importing the tool's own predicates and running
`introduced_seams` with `heading_aware=False`: every one of the 16 is a heading THIS CUT WROTE
followed by its own body. There is no seam in the set where original prose meets original prose.

### PROOF 3 could not be measured in the three-output form, and that is not this cut's doing

The brief's prescribed invocation names all three real files as outputs. Run that way, with the
two siblings' pre-split content declared new:

```
  original : git master:docs/OVERSEER.md
  output   : OVERSEER.md / OVERSEER-LOG.md / OVERSEER-REFERENCE.md   (all three real files)
  declared new : git master:docs/OVERSEER-LOG.md
  declared new : git master:docs/OVERSEER-REFERENCE.md
  declared new : file .../scratchpad/new_lines.md

  PROOF 1  1a=0  1b=0  1c=0   1d PASS
           OVERSEER.md 710 original + 69 new
           OVERSEER-LOG.md 300 original + 5217 new
           OVERSEER-REFERENCE.md 216 original + 749 new
  PROOF 2  original 16770 -> outputs-new 16770, DELTA 0, 0 lost, 0 gained undeclared
  PROOF 3  seams introduced: heading-aware 182 / heading-blind 379
  VERDICT: DISPROVED - 1 failing check(s)                          (exit 1)
```

**PROOF 1 and PROOF 2 are IDENTICAL to the gating run** (0/0/0, the same 710/300/216 partition,
token delta 0). Only PROOF 3 differs, and the reason is structural rather than about this
document: `introduced_seams` scores every paragraph seam in each whole output file and subtracts
only the seams the ORIGINAL already had. The two siblings carry 6,035 non-blank lines that were
never in `docs/OVERSEER.md`, so every paragraph boundary among them is scored as "introduced".
**The instrument was built for a split that CREATES its destination; this cut APPENDS into two
destinations that already held six thousand lines.**

The gating run therefore takes the appended FRAGMENTS as its outputs. That is the same partition
of the same original with the pre-existing text excluded, and it is a stricter reading of PROOF 3,
not a weaker one: every seam it scores is one this cut actually made.

**The claim that the 182 is the siblings' pre-existing prose, and not this cut's, was measured
rather than asserted.** A control run puts the UNSPLIT state through the identical three-output
shape (`--output` the three master blobs, same `--new` declarations). Any seam it reports is by
construction not this cut's doing, because that state contains no cut.

```
  original : git master:docs/OVERSEER.md
  output   : o_boot.md / o_log.md / o_ref.md      (the three MASTER blobs, unsplit)
  declared new : git master:docs/OVERSEER-LOG.md
  declared new : git master:docs/OVERSEER-REFERENCE.md

  PROOF 1  1a=0 absent, 1b=0 undeclared, 1c=0 over-declared
  PROOF 3  [heading-aware] flagged in original: 8    seams INTRODUCED: 182   file-edge cuts: 0
           [heading-blind] flagged in original: 45   seams INTRODUCED: 363   file-edge cuts: 0
           derived cut points: 0   failing the predicate: 0
  VERDICT: DISPROVED - 1 failing check(s)
    FAILED: PROOF 3: 182 sentence seam(s) introduced by the split      (exit 1)
```

**The control reports 182, the same number, from a state with ZERO derived cut points.** There is
no cut in it to introduce a seam. So all 182 are the siblings' own pre-existing paragraph
boundaries, and this cut's contribution to that number is 0. The heading-blind figure differs
(363 against 379) by exactly the 16 heading-adjacent seams the cut's own new headings add, which
is the same 16 itemised above.

**And the append itself is verifiable without the tool at all**: the pre-split bytes of both
siblings are a byte-identical PREFIX of the new files. `cmp` on the first 446,869 bytes of
`OVERSEER-LOG.md` and the first 60,875 of `OVERSEER-REFERENCE.md` against their master blobs both
pass. Nothing already in either file was touched by the move commit.

## How the borderline sections were classified

The rule the cut ran on: a section headed like history that states a live standing rule STAYS; a
section headed like a rule that is entirely closed narrative MOVES. Judged by content, and the
line ranges below are from the original 1,477-line file.

### Split internally, because the rule and the episode were separable

| original | rule kept in boot | episode moved |
|---|---|---|
| 776-828 dash ruling | the owner's words and both halves in force (778-783, 794-798) | the scope gap and the measured counts (785-792, 800-828) -> log |
| 830-883 AS-NAMELESS-LABELS-RC1 | Size L, the 86.5 percent measurement, the `d-22` ACCEPT (832-837) | the six change sites, the risk, the landing condition, the prohibition (839-883) -> reference |
| 1012-1067 register fault | nothing; the general rule went to the reference file (1045-1049) | 1012-1043, 1051-1067 -> log |
| 1069-1107 the `if` refusal | nothing; the old-code rule went to the reference file (1099-1102) | 1069-1097, 1104-1107 -> log |
| 1154-1197 `grep -r` | what `grep` is here, the instrument-picking list, and AN EMPTINESS IS NEVER A FINDING (1159-1162, 1174-1181, 1192-1197) | the reproduction and this lane's zero audit (1164-1172, 1183-1190) -> log |
| 1199-1271 master's strict red | a partial run is not a landing gate, and an AS-frontend parcel gets a strict run (1267-1271) | 1199-1226, 1228-1265 -> log |
| 1324-1402 the landing-gate episodes | run `landing-run.sh` not a subset; copy its invocations; print `pwd` and `HEAD` (1343-1346, 1382-1385, 1399-1402) | 1326-1341, 1348-1380, 1387-1397 -> log |

Three of those retained fragments lost the heading they sat under, so the cut wrote three new
headings for them in the boot read: *A PARTIAL RUN IS NOT A LANDING GATE*, *PRINT `pwd` AND
`HEAD` BESIDE ANY VERDICT*, and the section wrapper *Rules banked 2026-09-05 - read at boot*.

### The calls a reasonable person could have made the other way

- **`grep -r` is a shell function (1154-1197): the rule STAYED in boot rather than joining
  *Worktree and environment quirks* in the reference file.** The 2026-09-04 cut sent environment
  quirks to the reference file, so consistency argued for moving it. It stayed because every
  session greps, and an instrument that returns a confident wrong zero changes what a session does
  from its first command, not at a recognisable moment. Its narrative went to the log; the boot
  keeps what a session must know before it looks anything up.
- **`MY OWN BISECTION WAS CONFOUNDED` (946-977) went to the LOG, not the reference file**, even
  though it earned probe-design rules. Both rules already exist in
  `docs/OVERSEER-REFERENCE.md`: "ask what OTHER answer the probe could have given" is face 1 of
  *THE THREE FACES OF A CHECK THAT CANNOT COME OUT OTHER THAN GREEN*, and "a stated MECHANISM is
  more dangerous than a stated FACT" is the opening of *Dispatch practice*. Moving it to the
  reference file would have duplicated two live rules; moving it to the log preserves the episode
  without a second copy.
- **`A CONTROL CAN BE CONFOUNDED BY THE VERY THING IT CONTROLS FOR` (1109-1152) went WHOLE to the
  reference file**, though it is framed as a dated MOMPASS episode. Its three lessons are
  control-design rules read when judging a green or a report's severity, and the reference file
  already holds the check-cannot-fail material they belong beside. Classified by content, against
  its heading.
- **`THREE THINGS THIS PARCEL TAUGHT` (919-944) went WHOLE to the reference file** rather than
  being split across the log and the reference. Item 2's rule is generalised by the emptiness rule
  that stayed in boot, so item 2 alone had a case for the log. It was kept with its siblings
  because splitting a numbered list across two files leaves both halves misnumbered, and the
  readability cost outweighed a marginal classification gain.
- **The `## MASTER'S STRICT RED` heading moved with its narrative**, which leaves the retained
  rule (1267-1271) under a heading written by this cut. That is deliberate: the section is
  explicitly RESOLVED at `34dad07c`, and a boot read should not open a section with a closure
  notice.

### What NOTHING did

Nothing outside lines 776-1477 was touched. The overflow is one day's banking and the cut is
confined to it, which is what "in one cut" is for. No rule was shortened, none deleted, and the
move commit `cd7d83db` modified no existing line: the diff is pure relocation plus 87 declared-new
non-blank lines of pointer, index and heading.

## The reference repairs, as their own commit

`6051fa29`, separate from the move so the proof could tell a move from a rewrite. Swept by
REFERENT and in both directions. Seven repairs, every one a pointer whose target changed file:

- **reference -> boot**: "the exit-status rule banked above" now names SIGIL-AS-REPLACEMENT in
  `docs/OVERSEER.md`.
- **reference -> log**: "a lane that adopts only the rule above" now names the minimal-probe rule
  and points at the log entry that holds it.
- **reference**: "this document did not already have" meant the boot read, not the reference file
  it now sits in.
- **log, four instances**: "this document" meant `docs/OVERSEER.md` in all four.

**And the boot file's own index, which no proof looks at.** "Six blocks that sat here" was the
2026-09-04 cut's count and read as a description of the whole reference file. It is now dated to
that cut, with a following paragraph saying ten more joined at the 2026-09-05 cut and naming
where they are indexed.

**Two live bookings would otherwise have existed only inside moved narrative.** The queue board
`docs/lane-status.json` is UNTRACKED and is not present in this worktree, so this seat cannot show
that a rotated session would still hold `PINS-GATE-MESSAGE-MISLEADS` or
`AS-IF-REFUSAL-DIAG-VECTOR`. Both are named in the boot index, the second marked CLOSED at
`34dad07c` so nobody re-opens it from the narrative alone. **This is a report, not a fix**: if the
board does hold them, the boot lines are harmless duplication; if it does not, they were about to
be lost.

## The landing run

`scripts/landing-run.sh` itself, with its own invocations rather than retyped ones, run TWICE:
once at the cut plus the repairs, and again at the final tip so the verdict is stamped with the
tree that actually ships.

```
=============================== LANDING RUN VERDICT ===============================
  tree            .../agent-a97be5a4cf1ce39d7 @ 8ebe18d4 (parcel/boot-doc-cut-2, clean)
  reference       /home/volence/sonic_hacks/.aeon-ref @ 483b3e12 (HEAD, clean), all four present
  target dir      .../agent-a97be5a4cf1ce39d7/.target-land
  started/ended   2026-09-06T00:25:38Z -> 2026-09-06T00:31:49Z (UTC)
  CARGO_EXIT      0
  CLIPPY_EXIT     0   (lint bar clean)
  suites          410
  passed          4650
  failed          0
  ignored         2
  skip lines      0
  reconciles      4650 baseline + 0 new = 4650 observed
  RESULT          GREEN
===================================================================================
LANDING2_EXIT=0
== END MARKER ==
```

**Failures first: there are none.** `^test .* FAILED` returns 0 and `test result: FAILED` returns
0, against a positive control of 410 `test result: ok` lines from the same file, so the zero is a
finding rather than a broken pattern. The three `panicked at` lines in the log are
`should_panic` tests naming themselves (`override_of_unknown_constant_panics`,
`ensure_generated_refuses_before_it_touches_an_absent_tree`, `compress_panics_on_error`).

**The count reconciles exactly with the brief's stated baseline.** 4,650 passed, 0 failed, which
is master's 4,649 plus the one this parcel turned green, and no other delta.

The first run (at `6051fa29`, before this note existed) was identical: 410 suites, 4,650 passed, 0
failed, clippy 0, RESULT GREEN, 00:19:17Z to 00:24:30Z. It is re-run rather than relied on because
this note was written into the tree while it was in flight, and a verdict stamped with a tree that
no longer exists is the shape this lane already banked.

`/home/volence/sonic_hacks/.aeon-ref` was read and never rebuilt; the script names it at
SUITE_PATHS step 1 and reports it clean at `483b3e12` on both runs.

## Anything in this brief you concluded was wrong

**1. The branch name was already taken, and by work that had ALREADY LANDED.** The brief says to
branch onto `parcel/boot-doc-cut`. That branch exists, is checked out in a locked worktree
(`agent-aeab6aa67300cf771`), and points at `8997715d`, *"docs: split the boot read by WHEN A RULE
IS READ, in one cut"*. `git merge-base --is-ancestor 8997715d 8e35bd94` returns true: it is the
2026-09-04 cut, landed at 18:29 the previous evening, and the ref and its worktree are stale
leftovers. Work is on **`parcel/boot-doc-cut-2`**. Worth flagging for two reasons beyond the name:
the stale locked worktree is exactly the class the boot read banked last night (a lock records
that an agent claimed a tree, never that one still holds it), and `git log --oneline
8e35bd94..8997715d` returning EMPTY is the already-merged signature, not an empty branch.

**2. The prescribed proof invocation cannot measure PROOF 3 for this cut**, and it does not fail
loudly in a way that says so: it exits 1 with `DISPROVED - 182 sentence seam(s) introduced by the
split`, which reads exactly like a real finding. The cause is that the brief's three-output form
assumes the destinations are CREATED by the split, as they were for the 2026-09-04 cut this
instrument's guidance was written from. Appending into siblings that already hold 6,035 lines
makes every one of their pre-existing paragraph seams score as introduced. A lane that took the
exit 1 at face value would start moving cut points to satisfy an instrument measuring somebody
else's prose, which is the failure the boot read calls bar 9. **The three-output form is still
worth running: PROOF 1 and PROOF 2 are fully valid in it, and they are the proofs that catch a lost
line or a lost token.** It is PROOF 3 alone that needs the fragment form. This is a candidate
amendment to the protocol's split section.

**3. It also takes several minutes and can look hung.** Declaring 6,035 sibling lines as `--new`
puts 33 lines that share text with the original (`` ``` ``, ` ```sh `, `|---|---|---|`) into the
alignment's AMBIGUOUS class, and the state closure is re-expanded at each of 1,226 levels. It
finishes, but a 2-minute timeout kills it first, and a killed run here prints nothing at all.

**4. The brief's baseline of 4,650 is one interpretation of master's count, and the run below
settles which.** Master sits at 4,649 passed / 1 failed. The failing test is
`the_boot_read_is_inside_its_byte_bound`, and its sibling `the_bound_actually_refuses_something`
passes, so the binary contributes 2 tests either way and the expected green total is 4,650. That
is what the brief says and this seat agrees; it is recorded here only because "baseline + the one
we fixed" and "baseline as stated" are the same number by coincidence of the arithmetic rather
than by the same reasoning, and a future reader deserves to see which was checked.

**5. One brief instruction was followed but is worth naming as a residual risk rather than a
correction.** The brief says the boot file keeps "the standing rulings that change what a session
does first". Three of the rules kept in boot (`landing-run.sh` is the gate, copy its invocations,
print `pwd` and `HEAD`) are read at a MOMENT, strictly speaking, and a purist reading of the
owner's ruling would send them to the reference file. They stayed because all three are about the
act of landing, which is what every parcel ends with, and because two of them were violated ten
times in one day by a session that had read the reference file. If the boot read comes under
pressure again, these three are the first honest candidates to move, and they should move together
with a pointer, not be trimmed.
