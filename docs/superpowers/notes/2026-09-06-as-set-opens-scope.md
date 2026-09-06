# A value binder opens a local-label scope, and it was never only `set`

2026-09-06 · branch `parcel/as-set-opens-scope` · sigil master base `fabeaeea`

Queue row **AS-SET-OPENS-SCOPE**. Every number below was produced by a command,
and each is named at the number. Reference assembler throughout:
`/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5
`61e672562465725a8c102288a7da9098`, flags `-xx -n -q -A -L -U -i .`, run through
`asl_run` so every invocation reports its exit status AND its `ASL_DIAG`
completeness. `s2disasm`'s build (md5 `0dee1f98e6480a4783d27ffd8b90896f`) was
never used.

---

## THE REPRODUCTION, BOTH DIRECTIONS, WITH THE VALUES EXTRACTED

Probes in `2026-09-06-as-set-opens-scope-probes/`. One source, two references:

```asm
	cpu	68000
	padding	off
	org	$1000
Parent:
	nop
Var	set	5
.lq:
	nop
	dc.l	<Parent.lq | Var.lq>
```

`lq` occurs exactly once in the file and `Parent` and `Var` are distinct names,
so the spelling that resolves names the parent outright. **A local that existed
under both candidate parents could not have told them apart, which is why this
probe defines it once rather than twice.**

| reference | asl (`d1`/`d2`) | sigil `fabeaeea` | sigil now |
|---|---|---|---|
| `dc.l Parent.lq` | `error #1010: symbol undefined`, exit 2 | **assembles, `00 00 10 02`, exit 0** | `unresolved symbol \`Parent.lq\``, exit 1 |
| `dc.l Var.lq` | assembles, `00 00 10 02`, exit 0 | `unresolved symbol \`Var.lq\``, exit 1 | assembles, `00 00 10 02`, exit 0 |

Both asl runs report `ASL_DIAG=complete`, so the pass loop finished and
`symbol undefined` was actually LOOKED FOR in the row where it does not appear.

**The bolded cell is the whole reason this is worst-class**: sigil returned
`$00001002` -- a plausible, in-range, correctly shaped answer -- for a symbol the
reference assembler says does not exist.

### The same fact without a reference at all

A probe that references the local cannot both exit 0 and produce the answer:
one of the two spellings is undefined, that is an error, and an error stops
asl's pass loop. So `s01` reads asl's OWN SYMBOL TABLE instead, exit 0,
`ASL_DIAG=complete`:

```text
*Parent :  1000 C |  *Var : 5 - |  *Var.lq : 1002 C
```

There is no `Parent.lq` row.

### The control that keeps "d1 assembles" from having two readings

`d3_absent.asm` references `Parent.nothere`, defined nowhere. sigil refuses it
(`unresolved symbol \`Parent.nothere\``, exit 1) on both binaries. Without that,
`d1` assembling could have been a fallback path rather than a resolved symbol.

---

## IT WAS NEVER ONLY `set`: TWELVE SPELLINGS, PLUS `enum`

`matrix.sh` generates two probes per form and puts both assemblers to both.
**24 of 28 rows diverged.** Every row's asl side is `ASL_DIAG=complete`.

| form | asl `Anchor.zz` | asl `Bn.zz` | sigil before | sigil now |
|---|---|---|---|---|
| `Bn set 5` | undefined | `$00001002` | inverted | matches |
| `Bn: set 5` | undefined | `$00001002` | inverted | matches |
| `Bn equ 5` | undefined | `$00001002` | inverted | matches |
| `Bn: equ 5` | undefined | `$00001002` | inverted | matches |
| `Bn = 5` | undefined | `$00001002` | inverted | matches |
| `Bn: = 5` | undefined | `$00001002` | inverted | matches |
| `Bn := 5` | undefined | `$00001002` | inverted | matches |
| `Bn eval 5` | undefined | `$00001002` | inverted | matches |
| `set Bn,5` | undefined | `$00001002` | inverted | matches |
| `eval Bn,5` | undefined | `$00001002` | inverted | matches |
| `Bn set "ab"` | undefined | `$00001002` | inverted | matches |
| `Bn equ "ab"` | undefined | `$00001002` | inverted | matches |
| `Bn label *` | undefined | `$00001002` | **matches** | matches |
| `Bn:` | undefined | `$00001002` | **matches** | matches |

"inverted" is the exact symmetry of the table above: sigil resolved what asl
called undefined and refused what asl resolved. The two `label` rows are the
controls; they were already right, which is what makes the other twelve a
defect and not a design choice.

`s11` adds `enum`: after `enum En1,En2` the following `.e1` lists as `En2.e1`
-- the **last** member owns the scope, with no `En1.e1` and no `Anchor.e1`.

---

## THE FIVE SUB-RULES, EACH MEASURED

| probe | question | asl's answer |
|---|---|---|
| `s03` | does a DOTTED binder open a scope? | **No.** `.b set 5` under `Outer:` leaves the next local at `Outer.zz`, not `Outer.b.zz` |
| `s04` | is the binder's own RHS read in the new scope? | **No.** `Vr set .prev` binds `Parent.prev`'s value; only the NEXT line resolves against `Vr` |
| `s06` | does a REFUSED declaration still open one? | **Yes.** `Kc equ 9` / `Kc set 10` is `#2030` and the next local still lists as `Kc.rr` |
| `s08` | does a binder whose RHS does not evaluate? | **No.** the table reads `Anchor.tt`, not `Bd.tt` |
| `s09` | inside a macro, whose scope? | **the CALLER's.** `Ms.uu : 1002 C` |

And one asl quirk worth recording, `s10`: with a FORWARD-referencing RHS
(`Fw set Later`) the symbol table carries **both** `Anchor.vv` and `Fw.vv`, at
the same address. Pass 1 files the local under the old parent because `Later` is
not yet resolved, pass 2 under the binder. asl is pass-unstable here and
tolerates it, because both names end up defined and references resolve on the
final pass.

---

## THE POPULATION, AND WHY THE ROW'S FIVE SITES ARE NOT IN IT

`scan.py`, over four trees. s2disasm read from a **detached worktree at
`e45ebf33`** under a run-unique path; the owner's checkout was never written to.

| tree | git | files | S1 global binders | S2 with a `.local` | S3 straddle | S4 qualified |
|---|---|---|---|---|---|---|
| `s2disasm` | `e45ebf33` | 332 | 1,599 | 4 | 1 | 0 |
| `s1disasm` | `f6ece65` | 459 | 1,228 | 1 | 0 | 0 |
| `skdisasm` | `2fcd861` | 959 | 518 | 4 | 0 | 0 |
| `aeon` | `483b3e12` | 3 | 37 | 0 | 0 | 0 |
| **TOTAL** | | **1,753** | **3,382** | **9** | **1** | **0** |

A canary of the exact divergent shape was planted in the s2 worktree first and
the scan reported it, then removed. **The zeros are a measurement, not a walker
that reached nothing.**

### The row's five sites, tested in their real surrounding context

The brief names `s2.asm:9486,9487`, `s2.asm:33509,33510` and
`s2.macros.asm:170`, and asks for a per-site verdict. Read in context, **all
five are DOTTED binders**:

| site | the line | verdict |
|---|---|---|
| `s2.asm:9486` | `.a	set	Object_RAM` | **cannot diverge** -- dotted, `s03` |
| `s2.asm:9487` | `.b	set	SS_Dynamic_Object_RAM_End` | **cannot diverge** -- dotted |
| `s2.asm:33509` | `.a	set	Dynamic_Object_RAM` | **cannot diverge** -- dotted |
| `s2.asm:33510` | `.b	set	Dynamic_Object_RAM_End` | **cannot diverge** -- dotted |
| `s2.macros.asm:170` | `.zone_table_name := "__LABEL__"` | **cannot diverge** -- dotted |

The brief was right that the shape it reduced does not diverge, and right to
flag "zero population" as ambiguous. The honest reading is stronger than either:
**none of the five can diverge, because a dotted binder opens no scope at all**,
which is a property of the whole class rather than a fact about one reduction.

### The nine sites the shape actually has

`sites.py` prints the lines behind each count so the verdict is checkable.

| # | site | binder | verdict |
|---|---|---|---|
| 1 | `s2.macros.asm:18` | `DMA = %100111` | **no divergence.** The `.l`/`.w` hits are operand SIZES. The three real `.loop` labels sit inside macro BODIES, which take the expansion's own scope in both assemblers, not `DMA`'s |
| 2 | `s2.macros.asm:114` | `DebugSoundbanks := 0` | **no divergence.** Every `.skip` is inside a macro body |
| 3 | `s2.macrosetup.asm:245` | `last_btst_converted := …` | **no divergence.** Both hits are `(…).l` sizes; no local in the window |
| 4 | `s2.sounddriver.asm:242` | `SFX_PSG_TRACK_COUNT = …` | **no divergence.** `.cnt` is defined and read entirely inside one macro body |
| 5 | `s1disasm/MacroSetup.asm:113` | `tracenum := 0` | **no divergence.** `.start`/`.end` defined and read inside one macro body |
| 6 | `skdisasm/Sound/Z80 Sound Driver.asm:245` | `zID_SongLimit = 0Ch` | **no divergence.** `.bankloop` and `.cnt` are macro-body locals |
| 7 | `skdisasm/s3.constants.asm:96` | `Ref_Checksum_String := 'init'` | **no divergence.** `.check2` is defined at line 100 and read at line 101, both inside the same window, so both assemblers resolve it consistently under their own parent |
| 8 | `skdisasm/sonic3k.constants.asm:996` | `Ref_Checksum_String := 'SM&K'` | **no divergence.** Same shape, `.check` |
| 9 | `skdisasm/sonic3k.macros.asm:20` | `DMA = %100111` | **no divergence.** Site 1's twin |

**Nine of nine do not diverge, and it changes what shipping this is worth.**
This fix moved no corpus byte and closed no corpus bug. What it closed is a
silent wrong answer the corpus does not currently ask for -- a program that
writes `dc.l Parent.lq` after a global `set` got a value from sigil and a
refusal from asl, and nothing announced the difference. That is worth having
because the class is silent, not because anything was broken today, and saying
otherwise would overstate it.

The single S3 straddle is site 3, on a local named `.l`, and it is the size
suffix, not a local.

---

## THE BYTE SWEEP, ITS START GUARD, AND WHAT THE ARMS PRODUCED

`sweep.sh <old> <new> <four trees>`, comparing per file: emitted bytes, every
diagnostic, and the exit code.

**THE START GUARD FIRED FIRST.** The sweep refuses to run unless the two
binaries answer a WITNESS differently -- the old one must ACCEPT `dc.l Parent.lq`
after a `set` and the new one must REFUSE it:

```text
OLD md5 b96afb88c7c78e0dbbfaa911023a1718   (master fabeaeea, via `git archive`)
NEW md5 e3192147939a3b56ff2380ae5326372e
witness: OLD rc=0  NEW rc=1  (dc.l Parent.lq after a `set`)
witness: OLD accepts the symbol asl calls undefined, NEW refuses it -- two distinct tools
```

The guard is written against the DEFECT and not against a digest, because two
binaries with different md5s can still be the same assembler.

```text
s2disasm (detached worktree @e45ebf33)  files=332   differ=0
s1disasm @f6ece65                       files=459   differ=0
skdisasm @2fcd861                       files=959   differ=0
aeon-ref @483b3e12                      files=3     differ=0
TOTAL files=1753  identical=1753  DIFFER=0
```

### And what each arm PRODUCED, which is where this nearly went wrong

```text
OLD accepted 2 of 1753, emitting 0 bytes
NEW accepted 2 of 1753, emitting 0 bytes
diagnostic lines compared on the refused files: 43862
^ NO BYTES WERE COMPARED.
```

**1,751 of 1,753 files are refused by BOTH arms and the two that are accepted
emit nothing.** These trees are include fragments; assembled standalone they do
not stand up. So this sweep compared **43,862 diagnostic lines** and not one
byte of image. That IS a real comparison -- a diagnostic names the symbol it
could not resolve, and moving a local to a different parent renames it -- but it
is not byte identity, and reading `DIFFER=0` as byte identity would be reading
two arms failing identically upstream as two arms passing.

`sweep.sh` now prints that census on every run and refuses to let a zero-byte
run be read as a byte result. The instrument carries the correction, not just
this note.

### The byte half, run separately, and real

Both aeon ROM shapes built with each binary against the same reference tree,
output to a scratch path so `.aeon-ref`'s own ROMs were never rebuilt:

```text
OLD plain  crc=1c09fbfc len=819131     NEW plain  crc=1c09fbfc len=819131
OLD debug  crc=e2144057 len=840324     NEW debug  crc=e2144057 len=840324
cmp old.bin       new.bin        IDENTICAL
cmp old.debug.bin new.debug.bin  IDENTICAL
```

1,659,455 bytes of real emission compared across both shapes, and both CRCs are
the goldens.

---

## WHAT CHANGED

`crates/sigil-frontend-as/src/eval.rs`, and nothing else in the assembler.

- **`open_binder_scope`** is the rule and holds its two measured conditions: a
  DOTTED name opens nothing (`s03`), and it is called only where a value
  ACTUALLY BOUND (`s08`). It routes through the existing `open_scope`, so a
  binder inside a macro lands in the caller (`s09`) with no second mechanism.
- Called from **`directive_set`** and **`directive_equate`**, in each of their
  three binding branches (string, float, integer), and from **`enum_members`**.
  `directive_set_comma` reaches it through `directive_set`, which is why the two
  operand-field spellings are covered without a call of their own.
- **Every call site sits AFTER the operand has been read.** The binder's own RHS
  is evaluated in the PREVIOUS scope (`s04`), and both integer branches run
  `qualify_expr` on that same line for the symbolic-equ / relocation snapshot --
  opening the scope first would qualify the line's own locals against the name
  it is defining.
- The `equ` arm where `eval_all` FAILED (the deferred cross-seam `equ_sym`) does
  NOT open one, and carries a comment saying that is `s08` rather than an
  omission: a promise to the linker is not a value bound at that line.

**No emission path changed.** The helper writes one `Option<String>` and returns.

### The one measured divergence deliberately left standing

`s06`: a binder whose DECLARATION asl refuses as a class crossing still opens a
scope there. Both directives return before `open_binder_scope` on that path, so
sigil leaves the scope where it was. Reaching asl's answer means evaluating the
right-hand side on a path that has already reported -- double-reporting one line,
to fix the spelling of a local inside a program **both assemblers refuse**.
Corpus population zero. Booked here rather than done.

---

## RED-FIRST, WITH THE MUTATION SHOWN ON DISK

`crates/sigil-frontend-as/tests/as_binder_opens_scope.rs` was committed ALONE,
in `0341314d` and then `31fcdb03`. At `31fcdb03`,
`git diff fabeaeea --name-only -- crates/` named **that file and nothing else**,
so the run below is master's assembler and not a mutation that failed to apply.
The final red/green pair was then run by stashing and restoring only
`crates/sigil-frontend-as/src/eval.rs`, with `git diff --stat` printed at each
step to show which files were actually in the tree.

```text
fix STASHED    (git diff HEAD --stat: the test file only)
  test result: FAILED. 5 passed; 7 failed
    FAILED  a_set_binds_the_local_to_the_binder
    FAILED  an_equ_binds_the_local_to_the_binder
    FAILED  the_comma_operand_form_binds_the_local_to_the_binder
    FAILED  a_string_set_binds_the_local_to_the_binder
    FAILED  a_set_inside_a_macro_opens_the_scope_in_the_caller
    FAILED  the_preceding_label_is_no_longer_the_parent_after_a_set
    FAILED  the_preceding_label_is_no_longer_the_parent_after_an_equ
    ok      a_dotted_binder_opens_no_scope
    ok      a_binder_whose_value_does_not_resolve_opens_no_scope
    ok      the_right_hand_side_is_evaluated_in_the_previous_scope
    ok      a_plain_label_still_opens_a_scope
    ok      the_label_directive_still_opens_a_scope

fix RESTORED   (git diff --stat: eval.rs + the test file)
  test result: ok. 12 passed; 0 failed
```

**The five passing rows are as load-bearing as the seven failing ones.** They
are what an over-broad fix breaks: a scope opened for a dotted binder, or opened
before the binder's own operand was read, or opened by a line that bound
nothing, fails one of them -- and all three of those shapes are in the corpus
while the divergent one is not.

Two of the seven fail in the OTHER direction: `_is_no_longer_the_parent`
ASSEMBLES on master and emits `$00000002` for asl's undefined symbol. That pair
is the silent half.

---

## LANDING RUN

`scripts/landing-run.sh --baseline 4658 --aeon /home/volence/sonic_hacks/.aeon-ref`

**FAILURES FIRST: zero.** `grep -c FAILED` over the whole log is `0`.

Run twice: once at `9585ac92` and once at the final tip after the dash pass,
with the same verdict block both times. The one below is the final tip's.

```text
pwd             /home/volence/sonic_hacks/sigil/.claude/worktrees/agent-a7ddc7ed61b24bb4c
HEAD            295e67b1 (parcel/as-set-opens-scope, clean)
reference       /home/volence/sonic_hacks/.aeon-ref @ 483b3e12 (clean), all four present
target dir      <worktree>/.target-land
CARGO_EXIT      0
CLIPPY_EXIT     0   (lint bar clean)
suites          412
passed          4670
failed          0
ignored         2
skip lines      0
reconciles      4658 baseline + 12 new = 4670 observed
RESULT          GREEN
```

The log was grepped for this parcel's own test binary rather than trusted to be
the right tree: `Running tests/as_binder_opens_scope.rs` is present, followed by
all twelve rows `... ok`, and the log's own stamp names this worktree, this
branch and HEAD `295e67b158abddee3ab355643071c29eff3e5a0a`. The delta is
exactly the twelve tests this parcel adds and nothing else.

The source-gate classification lane did not red: the new test file names none of
the identifiers that lane selects on, so it is outside the population that has
to be classified. `every_selected_test_file_is_classified` passed in the run
above.

---

## ANYTHING IN THIS BRIEF YOU CONCLUDED WAS WRONG

**1. "Declaring a variable with `set` opens a local scope" is true and far too
narrow, and the row is filed under the wrong name.** Twelve spellings do it --
`set`, `equ`, `=`, `:=`, `eval`, each with or without the decorative colon, both
operand-field forms, and the string-valued `set`/`equ` -- plus `enum`. Filing it
as a `set` defect would have closed one of twelve and left eleven, and the
brief's own reduction would still have passed.

**2. The five corpus sites are not "5 sites of the shape, at least one of which
does not diverge". All five are DOTTED binders, and a dotted binder opens no
scope at all.** So none of them CAN diverge, for a reason that is a property of
the class rather than a fact about one reduction. The brief was right to distrust
the row's wording and right that its reduction did not diverge; the true
statement is stronger and simpler than the careful one it offered.

**3. "Test ALL FIVE, in their real surrounding context" was the right
instruction pointed at the wrong five.** The five are inert. The nine sites the
shape actually has are elsewhere, in four files the brief does not name, and
they are the ones with verdicts above. Nine of nine also do not diverge.

**4. The four-corpus byte sweep, run exactly as specified, measured no bytes.**
The start guard is sound and it fired, but it answers "are these two binaries
different programs", not "were they put to anything". Both arms accepted 2 of
1,753 files and emitted zero bytes. `DIFFER=0` there is a statement about 43,862
diagnostic lines. The brief's own bar 3 is what caught this, and it caught it
because it asked for what each arm PRODUCED rather than whether they matched --
without that line I would have reported a byte sweep that compared no bytes. The
byte measurement is the aeon ROM A/B, and it had to be run separately.

**5. One thing in my own first cut was wrong in the same family.** My first
red-first run reported `the_preceding_label_is_no_longer_the_parent_after_a_set`
still FAILING after the fix, which read as the fix not working. It was the test
harness: `dc.l` of an unknown name is not a front-end error, it is a symbolic
fixup the LINKER refuses, so a `refusal()` that stopped at `assemble` reported
"the source assembled" for a source that does not link. The instrument was
answering a different question from the one under test.

## WHAT IS LEFT OPEN

- **`s06`**, the refused-declaration path, above. Population zero.
- **`Var set nosuchsymbol` is silently accepted by sigil** and is
  `#1010 symbol undefined`, exit 2, on asl. Found while writing the `s08` test,
  unrelated to scopes, not touched here, and named in that test's doc rather
  than absorbed into it.
- **asl's pass-unstable qualification** (`s10`) is reproduced rather than fixed:
  sigil's `eval_all` also returns `None` for a forward reference on an early
  pass, so a forward-referencing binder opens its scope only from the pass that
  resolves it. asl does the same and tolerates it; the landing run and the ROM
  A/B say it converges here too, but nothing pins it, and a program that
  DEPENDED on the early-pass spelling would be relying on an accident in both
  assemblers.
