# asl can fail completely and never have looked for a whole class of defect

2026-09-05 · branch `parcel/asl-pass-loop-swallows-diagnostics` · sigil master
base `d094c3c8`

**ASL-PASS-LOOP-SWALLOWS-DIAGNOSTICS.** Detection, and the sweep the
`2026-09-05-asl-silent-decline-regime` parcel booked and did not run.

Reference assembler: `s1disasm/build_tools/Linux-x86_64/asl`, md5
`61e672562465725a8c102288a7da9098`, exit status checked on every invocation.
`s2disasm`'s copy (`0dee1f98…`) is refused by the guard.

```sh
cd docs/superpowers/notes/2026-09-05-asl-pass-loop-probes
./run.sh                     # the reproduction table below
./sweep.sh                   # the population, one TSV row per tracked .asm
cd ../asl-reference && ./selfcheck.sh    # 15 cases, cases 10 to 14 are new
```

---

## THE SHAPE, and why the guard shipped hours earlier does not cover it

asl assembles in a **pass loop**. A forward reference is legal, so an undefined
symbol is a provisional value on pass 1 and becomes `error #1010: symbol
undefined` only when a later pass finds it still undefined. **Any error stops the
loop, and the pass that would have found them never runs.**

`asl_run` already refused a non-zero exit. That check cannot see this, and the
reason is worth stating rather than assuming: **here the exit is legitimately
non-zero.** The run really did fail. The defect is a different shape, *the run
failed for reason X and reported nothing whatever about reason Y*, and a check
keyed to "did it fail" has both arms of the pair in the same bucket. **The exit
check is not partial coverage of this.**

## The reproduction, with cases that can tell the answers apart

Six probes in `2026-09-05-asl-pass-loop-probes/`, all through `asl_run`. A file
with one error and one undefined symbol could not separate *suppressed* from
*never had one*, so the count and the position both vary, and two controls fence
the check on the other side.

| probe | contents | passes | errors | `#1010` reported | exit | `ASL_DIAG` |
|---|---|---|---|---|---|---|
| `three_undef_alone.asm` | 3 undefined, nothing else | 2 | 3 | **3** | 2 | complete |
| `error_first.asm` | 1 unrelated error **above** the same 3 | 1 | 1 | **0** | 2 | INCOMPLETE |
| `error_last.asm` | 1 unrelated error **below** the same 3 | 1 | 1 | **0** | 2 | INCOMPLETE |
| `error_only.asm` | 1 error, no forward reference at all | 1 | 1 | 0 | 2 | complete |
| `remote_error_placeholder.asm` | 1 error on line 3, macro below | 1 | 1 | 0 | 2 | INCOMPLETE |
| `remote_error_control.asm` | the same file without that line | 2 | 0 | 0 | 0 | complete |

**Why these discriminate.**

- `three_undef_alone` against `error_first` varies the *count*: 3 against 0 for
  the same three symbols. One-against-zero would be consistent with the file
  simply not containing them.
- `error_first` against `error_last` varies the *position*, and they agree. The
  committed reading was "an error found **earlier** suppresses every **later**
  report"; that is narrower than the rule. What stops is the loop, not the
  reading of the file.
- **`error_only` is the case the check has to get right to be worth having.** It
  fails. Exit 2, one error, one pass, and its diagnostics are **complete**, so it
  reads `complete`. Set beside `error_first` it is 1 pass, 1 error, exit 2 as
  well: the two differ in nothing a runner sees except one prose line in the
  listing footer.
- `remote_error_placeholder` against its control is a *byte* measurement rather
  than a diagnostic one: `beq.s +` reads `67FE` (a branch to itself) in the
  first and `6702` (the correct forward branch) in the second.

**Neither arm of any pair here is a silent no-op.** Each produced a named,
different value: an error count, a `#1010` count, a pass count, a two-byte
encoding.

## The mechanism, in asl's own words, where no runner reads it

```text
      1 pass
        Additional necessary passes not started due to
        errors, listing possibly incorrect.
```

**That line appears in the `-L` listing footer and NOWHERE ELSE.** The console
summary for `error_first.asm` and for `error_only.asm` is byte-identical:

```text
      1 pass
      1 error
      0 warnings
```

So without `-L` the two cases are indistinguishable, and detection is not
possible at all. That is a hard constraint on where detection can live, not a
preference.

## A correction to committed prose: same arithmetic, different mechanism

`asl_ref.sh`, `asl-reference/README.md` and `partial_failure.asm` all said the
`bra.s /` error **changed** the macro's `beq.s +` from `6702` to `67FE`. It did
not. `67FE` is the **pass-1 placeholder** for a forward reference, still there
because the loop stopped before pass 2 could resolve it.

The two readings are distinguishable, and `remote_error_placeholder.asm`
distinguishes them: an unknown-instruction error on line 3, above the macro
definition, with no `bra.s /` anywhere and nothing sharing an operand with the
branch, still yields `67FE`. Any error, at any position, related or not, leaves
**every** forward reference in the file at its placeholder. All three files are
corrected. The rule they were justifying does not move; only its reason does.

---

## WHERE THE DETECTION WENT, AND WHAT IT COSTS

**In the shared guard, `asl-reference/asl_ref.sh`, not in the runners.** The
prior parcel had put the same grep in three runners by hand; that is a
hand-maintained population, and the twenty-odd other runners in this tree were
not in it.

- `asl_diag_state <lst>` classifies a listing.
- `asl_lst_for <argv>` derives the listing asl would have written.
- `asl_run` writes `ASL_DIAG=<state>` to stderr beside `ASL_EXIT`, for every
  caller, without any of them being edited.

**THREE STATES, NOT TWO,** and the third is the part a one-line grep gets wrong:

| state | meaning |
|---|---|
| `complete` | footer present and lacking the warning: asl ran every pass it wanted |
| `INCOMPLETE` | asl refused a needed pass; later-pass diagnostics never looked for |
| `unknown` | no footer (a fatal or a crash), or no listing: **completeness unknowable** |

`unknown` exists because **absence of the warning is not evidence of
completeness**. A fatal error or a crash writes a listing with no footer at all,
and greping that for the warning finds nothing and reads as clean. **18 of the
348 tracked `.asm` files in this tree are exactly that shape.**

**THE HONEST LIMIT, and it is the whole shape of the thing: this makes the
incompleteness VISIBLE. It does not make the missing diagnostics APPEAR.** A
listing carrying `INCOMPLETE` is not a smaller diagnostic set to be topped up; it
is a set of **unknown size**. The only way to learn what was suppressed is to fix
the reported error and assemble again, which is what this sweep did for every hit
it could not clear by reading.

**What it costs.**

- **It reports and never gates.** `ASL_DIAG` does not touch `asl_run`'s return
  status (selfcheck case 12). Many probes here are supposed to fail and several
  read a non-zero exit as *the answer*; a state that moved the status would break
  them silently. The cost is that nothing forces a reader to act on it.
- **It cannot fire on correct input.** The warning says *due to errors*, and an
  error is exit 2, so a clean run cannot produce it. That is the always-red bar,
  and the sweep puts a number on how badly a failure-keyed check would have
  missed it: **115 tracked probes fail; only 24 of them swallow anything.**
- **It still cannot make itself get used.** Five scripts in this tree call
  `asl_run`: `scripts/z80_byte_sweep.sh`, the two probe `run.sh`es that already
  did (`2026-09-05-as-logical-precedence-probes`,
  `2026-09-05-as-macro-body-label-probes`), this parcel's `run.sh` and `sweep.sh`.
  Everything else calls `"$ASL"` directly and gets none of this. Unchanged from
  the exit-status parcel, and unchanged for the same reason: an adoption lint
  would fire on the runners that deliberately run a second build, which is the
  always-red shape.
- **`selfcheck.sh` is not in the cargo suite.** The landing run's 4658 tests do
  not include any of its 15 cases, and never did. Running it is a hand step, so
  a regression in `asl_diag_state` would not redden the gate. Pre-existing for
  cases 0 to 9 and not made worse here, but it is why this parcel's landing delta
  is 0 rather than 5.
- **It needs `-L`.** Without a listing the honest answer is `unknown`, and
  `z80_byte_sweep.sh`, which passes no `-L`, now gets `unknown` on any failing
  run. That is the true state, not a regression, but it is not free.

### The second defect, found by the check failing to work

Wiring the check into the probe runner produced `ASL_DIAG=unknown` on every file,
including ones known to swallow. The derivation was wrong, and it was wrong in
the way asl is wrong. Measured:

```text
swal.asm                                 ->  swal.lst
a.b/p.asm                                ->  a.lst          (in the PARENT dir)
./p2.asm                                 ->  .lst           (a hidden file)
<root>/sigil/.claude/worktrees/x/p.asm   ->  <root>/sigil/.lst
```

**asl truncates the listing name at the FIRST dot in the path it is handed.** A
runner inside a `.claude/worktrees/...` checkout that passes an **absolute**
source path writes its listing to the repository root, outside the worktree, and
finds none where it looked. The untracked `.lst` and `.log` at sigil's root are
that. `asl_lst_for` uses `${src%%.*}` to reproduce asl's rule exactly, and
selfcheck case 14 asks the filesystem where the listing went rather than
believing the rule.

### Red-first evidence

`selfcheck.sh`, 15 cases, 0 fail. The new ones:

- **case 10** measures the pair with no check of ours in the loop, and requires
  **both arms to exit non-zero** and the two `#1010` counts to differ (3 against
  0). Two arms failing the same way upstream are indistinguishable from two arms
  passing, so the case fails if either arm goes green or the counts converge.
- **case 11** requires `INCOMPLETE` on the subject **and `complete` on the
  control**, where the control fails with exit 2 and three real errors.
- **case 12** requires `asl_run`'s status to equal raw asl's on the subject.
- **case 13** is the mutation. It stubs the footer literal to an unmatchable
  string, **verifies the stub applied** (the guard carries the line before and
  the stub does not carry it after, because a stub that fails to apply runs the
  original program and produces the same answer as a working one), and requires
  the subject to come back `complete`. It does. Third stub point in this file:
  case 5 disables the digest comparison, case 9 the exit propagation, case 13 the
  footer match, and a stub of any one says nothing about the others.
- **case 14** assembles `d.d/p.asm`, finds the listing on disk, and requires
  `asl_lst_for` to name that same path.

**A mutation caught a defect before it was committed, which is the point of
running one.** The first version of `asl_diag_state` was an unanchored
`grep -q 'Additional necessary passes not started'`. A listing **echoes its
source**, `swallowed_undef_control.asm`'s comments explain that its footer does
*not* carry the warning, and the check read that sentence and called the control
`INCOMPLETE`. Case 11 went red. The match is now anchored to
`^[[:space:]]+`, which separates the footer from any mention of it, and the
fixtures keep their comments so the anchoring stays load-bearing.

---

## THE SWEEP

`./sweep.sh` assembles every tracked `.asm` standalone from a copy of its own
directory, with the blessed flags, and classifies the listing. 348 tracked
`.asm` at base `d094c3c8`, 297 of them under `docs/superpowers/notes/`.

Run twice: once by an ad-hoc script with the classification inlined, once by the
committed `sweep.sh` calling the guard's `asl_diag_state`. They agree file for
file. **That checks that the guard's classifier and a hand-written grep make the
same call, which is worth having; it is not two independent derivations, because
the second was written from the first.**

| | exit 0 | exit 2 | exit 3 / 139 |
|---|---|---|---|
| `complete` | 215 | **91** | 0 |
| `INCOMPLETE` | 0 | **24** | 0 |
| `unknown` (no footer) | 0 | 0 | **18** |

**91 probes fail with a complete diagnostic set.** That row is the reason the
detector is not keyed to failure: a failure-keyed check would fire on 115 files
where 24 are affected.

**Re-running it after this parcel lands returns 354 files, 28 `INCOMPLETE`,
310 `complete`, 18 `unknown`.** The eight-file difference is this parcel's own
fixtures joining the corpus, classified as designed: `swallowed_undef.asm`,
`error_first.asm`, `error_last.asm` and `remote_error_placeholder.asm`
`INCOMPLETE`, their four controls `complete`. Every other row is unchanged, which
is a live positive control on the sweep: a version that had stopped detecting
would show them green.

**The sweep's own limit.** Each file is run standalone. That is what most of this
tree's runners do, but a probe that needs an include path into one of the
disassembly corpora, or that is an include *fragment* rather than a probe, fails
here for a reason its own runner would not produce. Three rows are that, and they
are named as artifacts below rather than counted as findings.

### The 24 with a suppressed diagnostic set, by name

**An exposure is not a defect.** For each: does the note's recorded conclusion
*depend* on what was suppressed, or merely *coexist* with it? Two depend. The
other twenty-two are exposed and their conclusions stand, and saying so is the
finding.

| probe | reported | verdict |
|---|---|---|
| `2026-09-03-as-struct-probes/q8.asm` | 1 x #2040 | **DEPENDS** |
| `2026-09-03-irp-irpc-probes/p2.asm` | 6 x #1107 | **DEPENDS** |
| `2026-09-04-as-enum-probes/q10.asm` | 1 x #1820 | coexists |
| `2026-09-04-as-silent-acceptance-probes/b2.asm` | 1 x #1820 | coexists |
| `2026-09-04-as-silent-acceptance-probes/d1.asm` | 1 x #1820 | coexists |
| `2026-09-04-as-silent-acceptance-probes/d1a.asm` | 1 x #1820 | coexists |
| `2026-09-04-as-silent-acceptance-probes/d1b.asm` | 1 x #1820 | coexists |
| `2026-09-04-as-silent-acceptance-probes/d3.asm` | 1 x #1820 | coexists |
| `2026-09-04-as-symbol-class-probes/m6.asm` | 2 x #2030, 1 x #2035 | coexists |
| `2026-09-04-as-symbol-class-probes/m17.asm` | 2 x #1010, **after 2 passes** | coexists |
| `2026-09-05-as-macro-body-label-probes/p3.asm` | 2 x #1010, **after 2 passes** | coexists |
| `2026-09-05-as-macro-body-label-probes/p5.asm` | 2 x #1010, **after 2 passes** | coexists |
| `2026-09-05-as-undefined-sym-panic-and-silent-if-probes/elseif_undef.asm` | 1 x #1820 | coexists |
| `…/fwd_equ.asm` | 1 x #1820 | coexists |
| `…/fwd_include.asm` | 1 x #1820 | coexists |
| `…/fwd_label.asm` | 1 x #1820 | coexists |
| `…/fwd_set.asm` | 1 x #1820 | coexists |
| `…/if_undef.asm` | 1 x #1820 | coexists |
| `2026-09-05-asl-nondeterminism-sweep-probes/n_equ_ctx.asm` | 1 x #1860 | coexists |
| `2026-09-05-asl-nondeterminism-sweep-probes/n_nonreg_name.asm` | 1 x #1860 | coexists |
| `2026-09-05-asl-nondeterminism-sweep-probes/n_z80_reg.asm` | #1020, #1860 | coexists |
| `2026-09-05-asl-silent-decline-regime-probes/r08_other_arg_kinds.asm` | 5 errors | coexists, already booked |
| `2026-09-05-asl-silent-decline-regime-probes/r11_earlier_error_swallows_undef.asm` | 1 x #1133 | coexists, exists to demonstrate it |
| `asl-reference/partial_failure.asm` | 1 x #1110 | coexists as to its finding; its **mechanism** prose was wrong and is corrected above |

**A shape worth naming, because it defeats the obvious reading of the flag.**
`m17`, `p3` and `p5` ran **two** passes, reported `#1010 symbol undefined`, and
*then* wanted a third and refused. So "undefined symbols were reported" is not
evidence that the set is complete, and "1 pass" is not the tell. The footer line
is.

### The two that depend

**`q8.asm`, and `2026-09-03-as-struct-dots.md`.** The note said `endstruct C`
"leaves `C.len` undefined, yet asl then resolves `C.len` to the current PC and
exits 0, which is its own silent-wrong-answer". Both halves are wrong. asl exits
**2** on `q8.asm`, and the `100A` printed for `dc.w C.a,C.len` is the pass-1
placeholder. `q8.asm` carries an unrelated `#2040 structure name missing` on its
last line, which stopped the loop. Delete only that line:

```text
q8n.asm(15):11: error #1010: symbol undefined
C.len
```

with **no byte column at all** for that line. asl is loud about `C.len`. There is
no silent wrong answer, only a suppressed diagnostic. Corrected in the note.

**`p2.asm`, and `2026-09-03-irp-irpc-argcount.md`.** The note recorded a
behavioural divergence: "`ARGCOUNT` outside a macro: asl resolves it to something
(13 in `p2.asm`), sigil reports an unresolved symbol." `p2.asm` is a five-shape
file whose `three` macro raises six `#1107` errors above the
`dc.b $BB,ARGCOUNT` line. The `BB0D` in that listing is a pass-1 artifact. The
same construct alone in a file assembles in **2 passes**, `ASL_DIAG=complete`,
and says `error #1010: symbol undefined  ARGCOUNT`, emitting no byte. **The two
front ends agree; the divergence does not exist.** Corrected in the note.

Both were verified at this seat after a subagent proposed them, by rebuilding
each pair and reading the listings, not by taking the report.

### The 18 with no footer, by name

For these the flag cannot speak: the run died before writing a footer, so
`complete` is unavailable and `INCOMPLETE` is unavailable with it.

**Deliberate `fatal`, where dying IS the recorded conclusion (11)**:
`2026-09-05-as-interp-radix-probes/r12.asm`;
`2026-09-05-as-mompass-probes/` `co_fatal.asm`, `co_fatal_gt.asm`,
`co_fatal_plain.asm`, `co_map.asm`, `co_map2.asm`, `ctrl2_ifndef.asm`,
`ctrl3_relax.asm`, `ctrl_fatal.asm`, `m_fatal.asm`. All coexist.

**A fatal that is the finding (3)**: `2026-09-05-as-include-repeat-probes/`
`p2.asm`, `p3.asm`, `p8.asm`, all `#10008 INCLUDE nested too deeply`, which is
what those probes are for. Coexist.

**asl crashes (2)**: `2026-09-04-as-warning-exitm-probes/e8.asm` and `e11.asm`,
`exitm` inside `irp`, SIGSEGV, exit 139. That directory's README already records
this as "no listing" and as a cell with no reference answer at any nesting. The
listing file does exist and is zero bytes, which is why `asl_diag_state` returns
`unknown` for it rather than `missing`. Coexist.

**Sweep artifacts, not findings (3)**:
`2026-09-03-as-struct-probes/q5.asm` and
`2026-09-05-as-mompass-probes/songprobe.asm` both `include` a file from a
disassembly corpus that a standalone run cannot see, and
`2026-09-05-as-mompass-probes/inc/c.asm` is an include fragment, not a probe.
Their own runners supply what this sweep does not. **Not classified here.**

---

## WHAT IS LEFT OPEN

- **`ASL_DIAG` reaches four scripts.** `z80_byte_sweep.sh` and the two
  `run.sh`es that already call `asl_run`, plus this parcel's. Every other runner
  in this tree calls `"$ASL"` directly. Migration, not enforcement, for the
  reason the exit-status parcel gave.
- **The three runners that grep for the footer by hand** still do, and now print
  the line twice. Left alone: they are the committed evidence of the parcel that
  found this, and de-duplicating them is a cosmetic edit to a note's artifact.
- **The absolute-path listing hazard is documented, not prevented.** A runner
  that hands asl an absolute path still writes a stray `.lst` at the first
  dot-directory in its path. `asl_run` says so when it finds no listing, which is
  after the fact.
- **The sweep is a snapshot at `d094c3c8`.** It is one command and it is
  committed, so re-deriving it costs nothing, but nothing re-runs it.
- **`m6.asm` hides a real diagnostic that nothing currently reads.** With its
  three class-conflict errors neutralised, asl reports `#1010` three times on the
  `rept`'s self-referential `Cm set Cm+1`. The note's own "cells NOT probed"
  section defers every `rept`-redefinition question, so no conclusion moves, but
  the answer is there and unrecorded.
- **`b2.asm` and `elseif_undef.asm` are cited by no note at all.** They exist,
  they are exposed, and nothing rests on them. Whether an uncited probe should
  stay is not this parcel's call.

## ANYTHING IN THIS BRIEF I CONCLUDED WAS WRONG

- **"The row says it stops reporting after its first error."** The mechanism was
  not new to this tree at all. `2026-09-05-asl-silent-decline-regime.md`, landed
  hours earlier on this same day, already names the footer line, already quotes
  it verbatim, already has a matched `r11`/`r11b` pair, and already wired the
  grep into three runners. It also explicitly booked this sweep: *"Any probe
  corpus in this tree that puts several deliberate errors in one file has this
  defect and does not know it. Not swept here; booked."* This parcel is the
  execution of that booking, not a discovery. The genuinely new parts are the
  position result, the anchoring, the third state, the placement in the shared
  guard, and the swept population.
- **"Position is irrelevant" is right, and it is a correction to that note, not
  to the brief.** The committed reading was "an error found EARLIER suppresses
  every LATER report". The brief had this correct; the note did not, and it is
  widened.
- **The brief's table said the footer reads `1 error` in the suppressed cases and
  implied that is the whole tell.** The footer is where the tell is, but the
  **console** is where it is not: with `-q` or without `-L` there is no listing
  and the two cases are byte-identical on the console. That constraint decided
  where detection could live and the brief did not name it.
- **"A run whose footer says additional passes were not started has incomplete
  diagnostics"** is true but is only half a predicate. The brief's implied
  converse, that a run without the line is complete, is false: 18 tracked probes
  produce no footer at all, and an unanchored grep of those reads as clean. The
  check needed a third state the brief did not ask for.
- **The brief warned that adding a test file could red the source gate.** No
  `crates/*/tests/*.rs` file was added; the whole parcel is under
  `docs/superpowers/notes/`, so that hazard did not arise.
- **My own first implementation was the defect it was written to catch.** An
  unanchored grep read a fixture's comment about the warning as the warning. It
  is in the red-first section above because a mutation test that only ever
  confirms is not doing anything.
- **And my own first write-up of the sweep overstated its corroboration.** It
  said the population was derived by "two independently written scripts". The
  second was written from the first and calls the guard's classifier; the
  agreement is real and worth having, but it is a classifier-against-grep check,
  not two independent derivations. Corrected above and stated here rather than
  quietly softened.

## THE LANDING RUN

```text
tree        .claude/worktrees/agent-aee518212984133ae @ 419d1278
            (parcel/asl-pass-loop-swallows-diagnostics, DIRTY: untracked scratch only)
reference   /home/volence/sonic_hacks/.aeon-ref @ 483b3e12 (clean), all four present
CARGO_EXIT  0      CLIPPY_EXIT 0 (lint bar clean)
suites 411   passed 4658   FAILED 0   ignored 2   skip lines 0
reconciles  4658 baseline + 0 new = 4658 observed
RESULT      GREEN
```

**No failures, so there are no names to lead with.** The delta is 0 and that is
correct: this parcel adds no `crates/*/tests/*.rs`, so the source-gate
classification hazard the brief warned about did not arise, and the five new
checks live in `selfcheck.sh`, which the cargo suite does not run.
