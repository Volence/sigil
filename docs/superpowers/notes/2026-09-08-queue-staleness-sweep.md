# Queue staleness sweep, 2026-09-08

Every row in `docs/QUEUE.md` judged against the tree, clause by clause, on the question the
controller set: is this row describing work still outstanding, or work already done. No row was
edited. Retiring a row is a judgement about what this lane owes and belongs to the controller;
this note is the evidence he rules on.

Base: branch `sweep/queue-staleness` from master `bfd34618`. Master moved to `59bc7b16` during the
sweep, one commit, which corrects `AS-MESSAGE-AND-INTERPOLATION` in place. Every verdict below is
stated against `59bc7b16` and the QUEUE.md text was re-read there.

Sibling trees read read-only: aeon at `df5be38a`, oracle at `e6c432b`. Nothing was written to
either.

## The verdicts

22 rows: 7 LANDED with nothing outstanding, 3 PARTLY LANDED, 11 STILL OPEN, 1 that nobody outside
its author can judge. The three PARTLY rows are the ones where the row is closed or half closed but
a clause of it is still live, and they are called out rather than rounded to LANDED, because
rounding is how the residue gets lost.

| Row | Verdict | Established by |
|---|---|---|
| AS-NAMELESS-LABELS-RC1 | STILL OPEN | `git grep -n "'+' =>" crates/sigil-frontend-as/src/lexer.rs` |
| AS-MESSAGE-AND-INTERPOLATION | LANDED `67fdea97` | `git grep -n "pub messages" crates/sigil-frontend-as/src/lib.rs` |
| EMP-Z80-MNEMONIC-TABLE | STILL OPEN | `sed -n '2106,2156p' crates/sigil-frontend-emp/src/lower/code.rs` |
| ROWREMAP-HEAD-LABEL-RULED | LANDED (discharged at aeon) | `git -C aeon grep -n ojz_effects_editor_act1 -- games/sonic4/map.toml` |
| AEON-TOOL-DEFECTS-NAME-AS-POSITION | LANDED (discharged at aeon `36e3e409`) | `git -C aeon log --oneline -1 36e3e409` |
| STABILITY-RUNNER-MISSING-WHERE-CLAIMED | UNGROUNDED-IN-PRACTICE | `git grep -rln STABILITY-RUNNER -- docs/` |
| S1-BUILD-PROFILE | STILL OPEN | `git grep -rln S1-BUILD-PROFILE -- docs/` |
| AS-LEADING-DIGIT-IDENTIFIER | STILL OPEN | `git grep -n digit crates/sigil-frontend-as/src/eval.rs` |
| EMP-USP-CCR-SURFACE | PARTLY LANDED, name is wrong | `git grep -rn "Usp" -- crates/sigil-frontend-emp/` |
| EMP-ALIGN-SHARED-RULE | LANDED `3ed600d4` / merge `4f7fc54f` | `git grep -n asl_align_pad -- crates/sigil-frontend-emp/src/lower/` |
| CLOBBER-UNEXERCISED | STILL OPEN | `docs/decisions.jsonl` d-26-answered |
| SIGIL-DECOUPLE | STILL OPEN | `git -C aeon log --oneline --grep=relayout` |
| AGENT-WORKTREE-TARGET-DIRS | PARTLY LANDED, figures rotten | `git worktree list \| wc -l`, `git branch --list \| wc -l` |
| RESOLVER-FALLS-THROUGH-TO-SHARED-TREE | STILL OPEN | `sed -n '49,54p' crates/sigil-harness/tests/m1b_gate.rs` |
| LENS-UX-SEAT-DIAGNOSTICS | STILL OPEN, blocker holds | oracle `docs/lane-status.json`, LENS-UX-PAIR state `open` |
| S4BUDGET-STALE-ASSUMPTION | LANDED (discharged at aeon `c834cb66`) | `git -C aeon grep -A18 "def load_vram_layout" -- tools/s4budget.py` |
| CLOBBER-PAYOFF-MEASURE | STILL OPEN, BLOCKER CLEARED | `git merge-base --is-ancestor ee136317 master` |
| EMP-ORG-TWIN-CHECK | LANDED (answered, closed on the board) | commit message of `c8f67966` |
| PROSE-STATED-BOUNDS | STILL OPEN | `git grep -rln "stated in prose" -- docs/` |
| LENS-PASS | PARTLY LANDED, handoff text stale | `docs/lens-findings.jsonl` tally: 32 findings, 29 fixed |
| PINS-DEAD-TESTS-FIELD-AND-ORPHANS | LANDED `53bc85de` | `grep -cE "^\s*tests\s*=" crates/sigil-harness/repin.toml` |
| INDIRECT-COST-REPORT-UNCOVERED | STILL OPEN, in flight | `git grep -n indirect-cost -- crates/*/tests/` |

## The actionable half: rows that are not work any more

### PINS-DEAD-TESTS-FIELD-AND-ORPHANS is fully landed and is still marked `next`

This is the worst instance in the file, because `next` is the state that dispatches. The row was
written at `fa9d1e1d` (2026-09-08 19:29:47) and the work landed thirty minutes later at `53bc85de`
(2026-09-08 19:59:01), which is a descendant of the row commit and an ancestor of master. Both
halves landed, and the landing implements the row's own ruling rather than the obvious fix the row
warned against:

```
$ /usr/bin/grep -cE "^\s*tests\s*=" crates/sigil-harness/repin.toml
0
$ /usr/bin/grep -n "THERE IS NO" crates/sigil-harness/repin.toml
9:# THERE IS NO `tests` FIELD. A row that spells one is a PARSE ERROR, because every
$ git grep -n "zero_consumer_report" -- crates/sigil-harness/src/repin.rs
1558:pub fn zero_consumer_report(orphans: &[String], scanned: usize) -> String {
$ git grep -n "sig-orphan-pins" -- crates/sigil-harness/src/bin/repin.rs
158:    // sig-orphan-pins: which pins nothing imports, by NAME, on EVERY run, before the
$ git merge-base --is-ancestor 53bc85de master && echo ancestor
ancestor
```

The row's "412 dead `tests` lines" are gone, the field is a parse error against the four
`deny_unknown_fields` spec structs, and the zero-consumer set is reported by name on every repin
run and gated by nothing, which is what the row ruled. Nothing in the row is outstanding.

**This row is also a duplicate of two closed lens findings.** `docs/lens-findings.jsonl` carries
`sig-pins-tests-field` and `sig-orphan-pins` twice each: once `open`, once `fixed` at `53bc85de`.
The lens ledger knows; the queue does not.

### EMP-ALIGN-SHARED-RULE: the controller's spot-check is established, not just plausible

The row says the .emp align "still rounds up plainly". It does not. Both .emp align paths call the
shared signed rule, and the wiring is pinned:

```
$ git grep -n asl_align_pad -- crates/sigil-frontend-emp/src/lower/
crates/sigil-frontend-emp/src/lower/mod.rs:1034:    let pad = sigil_ir::asl_align_pad(pos, n);
crates/sigil-frontend-emp/src/lower/regions.rs:565:        let pad = sigil_ir::asl_align_pad(self.cursor, align);
```

`crates/sigil-cli/tests/align_as_parity.rs:138`,
`a_negative_vma_align_follows_the_shared_signed_rule`, derives its expectation by calling
`sigil_ir::asl_align_pad` rather than transcribing a number, and asserts a plain unsigned round-up
would answer 0 where the shared rule answers 256. Added at `3ed600d4`, merged at `4f7fc54f`, both
ancestors of master. Also a duplicate of lens finding `sig-emp-align-fill`, recorded fixed at
`4f7fc54f`.

The row's second clause, that the divergence moves no byte today because nothing aligns on the RAM
side, survives and is restated in the test's own doc comment. That clause was never the work.

### CLOBBER-PAYOFF-MEASURE is not landed, but its stated blocker has cleared

The row is `blockedBy: AS-S1-DRIVER-SIZE-60X: ruled to run after it, not beside it`. That work
landed:

```
$ git merge-base --is-ancestor ee136317 master && echo ancestor
ancestor
$ git log --oneline -1 ee136317
ee136317 land: org sets the counter absolutely, so a backward target is a mechanism and not a refusal
```

`3a5eb6c1` is the investigation that stopped correctly (no file under `crates/` changed) and
`ee136317` is the landing that closed it. The d-26 ordering condition is therefore satisfied and
this S row is runnable now. Nothing has measured the payoff: `docs/lane-log.jsonl` carries seven
`clobber` entries, none later than 2026-09-06, and none reporting a bytes-and-cycles figure.

This is the one row in the file whose staleness makes work AVAILABLE rather than wasted.

### S4BUDGET-STALE-ASSUMPTION is fully discharged, including at the receiving lane

The row's own text already says grounded and routed, retire at the next boundary. The receiving
lane acted:

```
$ git -C /home/volence/sonic_hacks/aeon log --oneline -1 c834cb66
c834cb66 fix(s4budget): 'a sigil listing emits no constants' is false, in four places
```

`tools/s4budget.py:660-671` now reads: *"that premise was FALSE and is corrected here 2026-09-07
(sigil's finding, re-derived in this tree): s4.lst carries 780 `EQU` rows, 32 of them `VRAM_*`"*.
The 32 is this lane's enumeration and the conclusion aeon kept (read `vram.toml`) is the one the
row predicted would survive on the union / `overlay_with` argument. Both halves of the row's
prediction held.

### ROWREMAP-HEAD-LABEL-RULED is fully discharged at aeon

The row rules that the order row must key by section name, not head label, and routes the fix to
the engine lane. It is done:

```
$ git -C /home/volence/sonic_hacks/aeon grep -n ojz_effects_editor_act1 -- games/sonic4/map.toml
115:  # The editor-scene block (`ojz_effects_editor_act1`, generated by tools/effects_gen.py
127:  "section:ojz_effects_editor_act1",
```

`map.toml:118` states *"The row is keyed by SECTION NAME (sigil ..."*, naming the source of the
ruling. The row's own guess that it would be a one-line move held.

### AEON-TOOL-DEFECTS-NAME-AS-POSITION is discharged

```
$ git -C /home/volence/sonic_hacks/aeon log --oneline -1 36e3e409
36e3e409 merge(S1S3S9): four of five worked, one refutes the lane that routed it, one declined with evidence
```

`docs/2026-09-06-sigil-routed-findings.md` exists in aeon at 337 lines. The row's own point, that
routing rather than recording was the failure, is closed by that file existing in their tree.

### EMP-ORG-TWIN-CHECK is answered and its own text says to close it

No new evidence needed beyond what the row states: the .emp frontend has no `org`, so the defect is
structurally impossible rather than absent. `lower/mod.rs` opening with `switch_section_lma` is
still true. Closed on the board at `c8f67966`.

### LENS-PASS is closed, and its handoff paragraph is now stale too

The heading says closed. But the body still says *"What remains is the FIX ORDER, which is yours: 4
byte-changing, 4 silent-acceptance or abort, 5 gates that cannot fail, 3 structural, 3
measure-first"*, which is nineteen items. Tallying the ledger by last state per id:

```
$ python3 -c "... last state per id in docs/lens-findings.jsonl ..."
distinct findings: 32 {'fixed': 29, 'open': 1, 'holds': 1, 'decided': 1}
  NOT FIXED: sig-extra-traversal | open
  NOT FIXED: sig-z80-codegen | holds
  NOT FIXED: sig-probe-content-snapshot | decided
```

One finding is open, and it is the one the board's `LENS-FIX-ARC` is already doing. The fix order
the row hands to the owner has been consumed without the row noticing.

### AGENT-WORKTREE-TARGET-DIRS is closed, and every figure in it and in its closing note is rotten

The row's residue is *"12 trees hold something ... 7 branches are unmerged"* and its closing
paragraph says *"6 registered worktrees of mine"*. Today:

```
$ git worktree list | wc -l
27
$ git branch --list | wc -l
39
```

The sweep did not undo itself; the population regrew through normal agent work. But a reader
arriving at either figure now reads a number that is wrong by a factor of four, and neither number
carries a measurement date. This is the count-in-a-row failure the lane already has a rule about,
sitting in a row that is otherwise closed.

## The structural defect underneath four of the seven

Four rows carry `state at archive: open` at their own heading while their CLOSED paragraph sits at
the bottom of a DIFFERENT row's body. The file was created at `c8f67966`, whose message says *"four
rows closed"* and names exactly these four: the org twin check, the five findings routed to aeon,
the root-level directory sweep, and the row-remap ruling.

```
$ git log --oneline --diff-filter=A -- docs/QUEUE.md
c8f67966 board: row text moves to docs/QUEUE.md, four rows closed
$ /usr/bin/grep -n "^\*\*CLOSED" docs/QUEUE.md
172:**CLOSED 2026-09-06 as far as this lane goes: ruled, no sigil change, sent to aeon ...
174:**CLOSED 2026-09-06. The aeon lane worked all five at aeon 36e3e409 ...
176:**CLOSED 2026-09-06 at the root level by the cleanup the owner asked for ...
178:**CLOSED 2026-09-06. Answered by the org parcel ...
$ /usr/bin/grep -n '^## ' docs/QUEUE.md | sed -n '19,20p'
165:## PROSE-STATED-BOUNDS
180:## LENS-PASS (closed 2026-09-06, dropped from the board)
```

All four `**CLOSED ...**` paragraphs are at lines 172 to 178, between the `PROSE-STATED-BOUNDS`
heading at 165 and the `LENS-PASS` heading at 180, which is where the file ended when it was
written. They belong, in order, to
ROWREMAP-HEAD-LABEL-RULED, AEON-TOOL-DEFECTS-NAME-AS-POSITION, AGENT-WORKTREE-TARGET-DIRS and
EMP-ORG-TWIN-CHECK. Every one of them is under the heading of a row that is genuinely open.

Two consequences, both live:

1. Four closed rows read as open at their headings.
2. `PROSE-STATED-BOUNDS`, which IS open and IS on the board, appears to carry four closure notices.
   A reader scanning headings and stopping at the first bold CLOSED will retire the wrong row.

The failure is not that a row went stale. It is that the file has no place to put a closing reason,
so the closing reason went where the cursor was. `LENS-PASS` solved the same problem by putting the
state in the heading, and it is the only row in the file a reader cannot misread.

## The board and this file disagree, in both directions

`docs/lane-status.json` (untracked, `updatedAt` 2026-09-09T00:42:16Z) carries eleven queue rows.
This file carries twenty-two headings. The two sets do not nest.

**On the board, absent from this file** (so the argument for them lives nowhere the file promises
it does): `LENS-FIX-ARC`, `SIG-EXTRA-TRAVERSAL`, `WARN-TIER-UNPINNED-PREFIXES`. The file's own
header says *"The board keeps a short title and this file keeps the argument"*, and for these three
it does not.

**In this file marked open, absent from the board**: AS-LEADING-DIGIT-IDENTIFIER,
EMP-USP-CCR-SURFACE, EMP-ALIGN-SHARED-RULE, CLOBBER-UNEXERCISED, CLOBBER-PAYOFF-MEASURE,
RESOLVER-FALLS-THROUGH-TO-SHARED-TREE, S4BUDGET-STALE-ASSUMPTION,
PINS-DEAD-TESTS-FIELD-AND-ORPHANS, INDIRECT-COST-REPORT-UNCOVERED, plus the four closed ones.

Absence from the board is not evidence of closure. Three of that list are LANDED, but
`RESOLVER-FALLS-THROUGH-TO-SHARED-TREE` is an open defect in shipped test code with no board row at
all, and both CLOBBER rows are unmeasured with no closure record anywhere. A row can fall off the
board without being finished, and this file is the only thing that remembers it existed.

One row disagrees the other way: the board still shows `AS-MESSAGE-AND-INTERPOLATION` as `open`,
size M. The board snapshot is 00:42:16Z; the correction commit `59bc7b16` is 2026-09-08 20:45:05
-0400, which is 00:45:05Z. The board is three minutes behind, not wrong on the merits. Noted so a
later reader does not treat it as a second instance. `lane-status.json` is untracked, so this is a
read-time observation with no history behind it and cannot be checked later.

## Rows that are open, with what remains stated precisely

**AS-NAMELESS-LABELS-RC1.** Unimplemented, as claimed. The lexer maps the character to an operator
token and there is no path from it to a label definition:

```
$ git grep -n "b'+' =>" crates/sigil-frontend-as/src/lexer.rs
crates/sigil-frontend-as/src/lexer.rs:333:        b'+' => Plus,
```

`git log --all -i --grep=nameless` returns eight commits, all probes, notes, sizing or unrelated;
none implements the feature. The probe fixture
`docs/superpowers/notes/2026-09-05-s2-top-blocks-decompose-probes/nameless_shapes.asm` holds one
instance of each of the four corpus shapes and the note records the two error texts. The owner
decision the row is waiting on has not arrived; the row's own account of that is accurate.

**EMP-Z80-MNEMONIC-TABLE.** Open, exactly as stated. `z80_mnemonic` at
`crates/sigil-frontend-emp/src/lower/code.rs:2106` ends at `"ldir" => Ldir`. The AS side has the
rest:

```
$ git grep -n '"otir"\|"rrd"\|"reti"\|"indr"' -- crates/sigil-frontend-as/src/
crates/sigil-frontend-as/src/eval.rs:9616:        "indr" => Indr,
crates/sigil-frontend-as/src/eval.rs:9619:        "otir" => Otir,
crates/sigil-frontend-as/src/eval.rs:9623:        "reti" => Reti,
crates/sigil-frontend-as/src/eval.rs:9625:        "rrd" => Rrd,
$ git grep -n '"otir"\|"rrd"\|"reti"\|"indr"\|"lddr"\|"cpir"' -- crates/sigil-frontend-emp/src/lower/code.rs
$ echo $?
1
```

Positive control for that zero: `git grep -c '"ldir"' -- crates/sigil-frontend-emp/src/lower/code.rs`
returns 1, so the instrument reaches the file and the empty result is a real absence.

**EMP-USP-CCR-SURFACE. Open, but the row's NAME is wrong and would mislead whoever takes it.**
CCR is already exposed in .emp:

```
$ git grep -rn "Ccr" -- crates/sigil-frontend-emp/ | head -5
crates/sigil-frontend-emp/src/branch_const.rs:90:                if !matches!(dst, CodeOperand::Ccr | CodeOperand::Sr) =>
crates/sigil-frontend-emp/src/eval/asm.rs:3019:                    return Some(CodeOperand::Ccr);
crates/sigil-frontend-emp/src/flag_check.rs:200:    matches!(ops.last(), Some(CodeOperand::Ccr) | Some(CodeOperand::Sr))
crates/sigil-frontend-emp/src/lower/code.rs:1409:        CodeOperand::Ccr => Ok(M68kOperand::Ccr),
crates/sigil-frontend-emp/src/lower/code.rs:1739:        (Andi, [_, M68kOperand::Ccr]) => AndiCcr,
$ git grep -rn "Usp\|\"usp\"" -- crates/sigil-frontend-emp/ ; echo "exit=$?"
exit=1
```

USP is what is missing, in both the operand kind and the two mnemonics. The AS side has
`M68kOperand::Usp` and refines `(Move, [_, Usp]) => MoveToUsp` at `eval.rs:10047-10048`. So the
work is real and the row's substance is right; only the CCR half of its title is false, and a row
named for two things where one is done is the shape this sweep exists to catch.

**AS-LEADING-DIGIT-IDENTIFIER.** Open. The row says the one-byte-short defect is closed and what
remains is a compatibility call and a language call. The lexer behaviour the note describes is
still there (`eval.rs:5452`, *"A processor name that begins with a digit reaches the lexer as an
..."*), no probe of what the reference accepts has been committed, and no ruling is recorded in
`docs/decisions.jsonl`. Both calls are still unmade.

**RESOLVER-FALLS-THROUGH-TO-SHARED-TREE.** Open, unchanged, and the exact line the row cites is
still the line:

```
$ sed -n '49,54p' crates/sigil-harness/tests/m1b_gate.rs
fn oracle_gui_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("ORACLE_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/oracle-old".into()),
    )
    .join("linux-port/gui")
}
```

The row's unaudited residue, `refreeze.rs:785`'s `AEON_DIR` default, is also still unaudited: this
sweep did not open it either, and says so rather than leaving the reader to assume.

**CLOBBER-UNEXERCISED.** Open. `docs/decisions.jsonl` carries `d-26-answered`, hub-ruled in the
owner's place and overturnable by him, ordering the payoff measurement first. Nothing has
overturned it and nothing has measured. The row's scope rider (an aeon-side landing, 89 aeon .emp
files against 3 in sigil) was not re-verified here and is not claimed current.

**SIGIL-DECOUPLE.** Open and correctly blocked. The engine-side relayout it waits on is itself
re-opened at aeon (`abce4643 ledger: relayout-magnitude is re-opened, and I had dropped it off the
owner's card`), and aeon's `67a5998c confirm(item 14)` re-found every aeon-side anchor in sigil's
45 rows. The row's verdict of NOT YET holds.

**LENS-UX-SEAT-DIAGNOSTICS.** Open, blocker holds, and the row's known-good floor still resolves.
Oracle's board carries `LENS-UX-PAIR` in state `open`, so the pilot has not run and the row's
"DO NOT START" clause is live. The floor the row names is present at aeon master:

```
$ git -C aeon grep -c "Roster C" -- docs/superpowers/LENS_PROTOCOL.md
4
$ git -C aeon grep -n "WHICH VALUE WAS IN EFFECT\|absent from the shared default" -- docs/superpowers/LENS_PROTOCOL.md
111:  explicitly, and the seat proves once WHICH VALUE WAS IN EFFECT, read back from what the
113:  under the seat's own location, absent from the shared default), and a demonstrated
```

Nothing has gone backwards. This is the one long row in the file that is entirely current, and the
reason is visible in its own text: it pins no revision and tells the reader to resolve at run time.

**PROSE-STATED-BOUNDS.** Open and unmeasured here, as its text says. No sweep artifact exists
(`git grep -rln "stated in prose" -- docs/` returns only `OVERSEER-REFERENCE.md` and `QUEUE.md`).
Its only defect is the four foreign CLOSED paragraphs glued to the end of its body.

**S1-BUILD-PROFILE.** Open. No profiling artifact exists beyond the one leg the row states.
Worth flagging: the board's `SIG-EXTRA-TRAVERSAL` ("the assembler reads the whole source one more
time than the old one does") is plausibly the second leg of the same question, is currently in
flight on `measure/extra-traversal`, and neither row names the other. Not a duplicate today, but
the two will answer each other and only one of them will get the answer written into it.

**INDIRECT-COST-REPORT-UNCOVERED.** Open, and in flight on `parcel/cost-report-shape-test`. The
gap is still real in master:

```
$ git grep -n "indirect-cost\|indirect_cost" -- crates/*/tests/
crates/sigil-frontend-emp/tests/contract_closure.rs:610:// with `sigil build --aeon <tree> --report indirect-cost` rather than recording it
```

One comment citing the command, no call. The row's second subject, `scripts/s8_seam_size.sh`, is
present, executable, and referenced only by documentation:

```
$ git grep -rn "s8_seam_size" -- . | grep -v QUEUE.md
```

returns `lane-log.jsonl`, `lens-findings.jsonl`, three review or note documents, and the gap
ledger. No runner. Both halves of the row stand.

## The one row nobody can close from outside

**STABILITY-RUNNER-MISSING-WHERE-CLAIMED** is judged UNGROUNDED-IN-PRACTICE, which is a narrower
claim than "matches no artifact". The class it describes is real and one instance is in the tree:
`docs/superpowers/notes/2026-09-05-disp-or-call-probes/README.md:3-9` says that note *"took its
rows on the s2disasm build and cited it by version banner; its probe files ... were never
committed"*. But that instance was already repaired at `460d46c0` on 2026-09-05, a day BEFORE this
row was archived, so it is not one of the five.

The row says "five notes" and names none of them. The result is that no one but its author can
tell whether it is done. I could not construct an instrument that returns the five, and I am not
going to guess a population and then declare it swept, because a sweep whose population was
invented by the sweeper is the failure mode the lane already books.

Concretely: `git grep -rn "version banner" -- docs/superpowers/notes/` returns twelve hits, eleven
of which are the GOOD practice (probe scripts saying *"The assembler is selected by MD5, not by
path and not by version banner"*). The bad population is the notes that do not say that, which no
grep reaches. That is the same third-population problem `PROSE-STATED-BOUNDS` describes, arriving
in a different file.

**What the row needs before anyone works it: its five notes by path.** Until then it is not
dispatchable, and its state should say so rather than `open`.

## What this sweep did not do

- Did not build. No cargo invocation, no `CARGO_TARGET_DIR` use, and the shared
  `target/release/sigil` binary was neither run nor relinked. Verdicts about behaviour are read
  from source at master, not from a binary of unknown currency: the installed one is dated
  2026-09-07 19:47 and master has moved since.
- Did not audit `refreeze.rs:785`, which `RESOLVER-FALLS-THROUGH-TO-SHARED-TREE` names as its own
  unaudited residue. That row's gap is unchanged, not narrowed.
- Did not enumerate the five notes behind `STABILITY-RUNNER-MISSING-WHERE-CLAIMED`, for the reason
  above.
- Did not re-verify `CLOBBER-UNEXERCISED`'s "89 aeon .emp files against 3 in sigil" scope rider.
- Read aeon and oracle read-only via `git --no-optional-locks -C`. Neither `build.sh` nor anything
  that writes into `AEON_DIR` was run; `AEON_DIR` itself was not needed and was not touched.

## What in the controller's brief turned out to be wrong

Two things, one small and one worth the correction.

**His reading of EMP-ALIGN-SHARED-RULE was right, and is now established rather than spot-checked.**
He asked to be checked and he holds. The test at `align_as_parity.rs:138` exists, is named as he
said, and landed at merge `4f7fc54f`; the part he did not state, and which is what actually settles
it, is that the .emp lowering calls `sigil_ir::asl_align_pad` at two sites, so the test is pinning a
real wiring rather than passing on a coincidence.

**"Two of two spot-checks were stale" understates it by one, and the third was more dangerous.**
`PINS-DEAD-TESTS-FIELD-AND-ORPHANS` is not merely stale but stale in state `next`, which is the
dispatch position, and it landed thirty minutes after the row describing it as outstanding was
written, on the same evening, in this same repo. The brief's model of the failure (a row goes stale
over days and nobody notices) is too slow by two orders of magnitude. Here the row and its own
completion were written within one session of each other and the row still says the work is next.

**The brief's expectation that some row would prove UNGROUNDED because its subject lives in a
sibling repo did not reproduce.** Every row matched an artifact somewhere. Three rows
(`ROWREMAP-HEAD-LABEL-RULED`, `AEON-TOOL-DEFECTS-NAME-AS-POSITION`, `S4BUDGET-STALE-ASSUMPTION`)
have their subject in aeon, and all three were found there and all three were DONE there. The trap
the brief warned about is real, and the correct move against it, looking in the sibling before
concluding absence, turned three would-be non-matches into three closures.
