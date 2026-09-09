# S1-EXPR-CHAR-LITERAL-AND-DEFINED

Two unimplemented AS expression features, 8 of the 46 diagnostics sigil emitted on
Sonic 1. A string literal in an expression is an integer, and `DEFINED(NAME)` is a
predicate about assembly state at a point in a pass.

Branch `s1-expr-char-literal-and-defined`, off master `bd7fce7b`.

## Instruments, and what each of them can and cannot say

**Reference assembler.** `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`,
md5 `61e672562465725a8c102288a7da9098`, verified before the first run. Invoked
`-xx -n -q -A -L -U -i .` through `docs/superpowers/notes/asl-reference/asl_ref.sh`,
which pins the digest and reports `ASL_EXIT` and `ASL_DIAG` on every call. Every
probe below quotes its exit status; a run with a non-zero status is treated as
carrying no values at all, including for the lines that assembled. The other build
in this workspace (md5 `0dee1f98e6480a4783d27ffd8b90896f`) was not run for any
value here.

**A zero exit is not sufficient either**, because this build substitutes the last
value it computed for an operand it declines to value. The packing figures were
therefore re-run with the probe lines reordered and with an unrelated
`move.l #$1234,d0` moved above and below them (probes `sp`, `sq`, both exit 0):
`"SW"` reads `5357` and `"GO"` reads `474F` in every position and neither ever
echoes the `1234`. These are answers, not artifacts.

**Corpus baselines.** `scripts/corpus-baseline.sh`, which derives generated-include
readiness from the corpus's own sources and refuses an unprepared tree.

**sigil binaries.** Before: `1d20a3eccefe033e01cb6cc76dd1acaa`, built in this
worktree from `bd7fce7b`'s `expr.rs`/`eval.rs`. After:
`9403d9f74b8a67098f8dc24dd5cfaa48`. Both built with
`CARGO_TARGET_DIR` off the shared `target/`, so the assembler another lane may have
frozen was never relinked.

## The corpus, before and after

s1disasm at `f6ece657`, entry `sonic.asm`, generated set READY (4/4). The before
column reproduces the controller's own baseline line for line.

```
  level    before   after   delta  class
  error        18      18      +0  unexpected character
  error        11      11      +0  `X` is not a recognized N mnemonic
  error         6       0      -6  bad immediate expression
  error         6       6      +0  case needs a string literal
  error         2       2      +0  switch needs a string expression
  error         2       0      -2  unresolved if condition: ...
  error         1       1      +0  unknown directive or mnemonic `X`
               46      38      -8  TOTAL

  lines only in the NEW run:  0
  lines only in the OLD run:  8
```

**Zero new lines and zero new classes.** Nothing was hiding behind the eight rows,
which is worth stating plainly because the expected shape of progress on this
project is the opposite: accepting a construct usually exposes whatever sat behind
it. Here it exposed nothing.

Sonic 1 still does not assemble. Four root causes remain (`listing`/`page`,
`charset`, `unexpected character`, `switch`/`case`), all other parcels.

### The other two corpora, checked for regression

The change is in the shared expression layer, so both were measured with the same
two binaries.

- **s2disasm** (unprepared, so the absolute number is not a baseline; the DELTA
  over one tree with two binaries is): 5227 to 5227, every class `+0`, and the
  diagnostic streams are identical, **zero lines in either direction**.
- **skdisasm** (prepared, 50/50, a real baseline): 2417 to 2407, `-10` on
  `bad immediate expression`, every other class `+0`, zero new lines. Those ten
  are string immediates too, and their bytes are pinned below.

## WHICH MECHANISM WAS ACTUALLY BROKEN

A red proves a refusal happened, not that the thing you meant to test caused it.
`unresolved if condition` does not say why, and the corpus expression
`if (MOMPASS=1)&&(DEFINED(loc))` has three candidate failure points: `MOMPASS`,
`DEFINED`, and the substitution of the macro parameter `loc`. `bad immediate
expression` is likewise one message over many ways an immediate can be bad. Each
candidate was separated, and the diagnostic text is quoted rather than the exit
status.

Probes in `.scratch/s1-expr/discrim/`, run through both binaries.

| probe | what it contains | BEFORE (`1d20a3ec`) | AFTER (`9403d9f7`) |
|---|---|---|---|
| `p1_corpus_shape` | macro + `MOMPASS` + `DEFINED` + `loc` | `p1_corpus_shape.asm(4): error: unresolved if condition: it does not evaluate, so it cannot decide whether the code it guards is assembled` | exit 0, no diagnostic |
| `p2_no_defined` | macro + `MOMPASS`, no `DEFINED` | `p2_no_defined.asm(5): error: guard fired` | `p2_no_defined.asm(5): error: guard fired` |
| `p5_loc_in_condition` | macro parameter `loc` used in the condition | exit 0, no diagnostic | exit 0, no diagnostic |
| `p3_no_macro` | `MOMPASS` + `DEFINED`, no macro at all | `p3_no_macro.asm(3): error: unresolved if condition: ...` | exit 0, no diagnostic |
| `p4_defined_alone` | `DEFINED` in a `dc.b`, no `if`, no `MOMPASS` | `p4_defined_alone.asm(4): error: bad byte expression` | exit 0, no diagnostic |
| `q1_corpus_shape` | `#"SW",SEgg_ChildCmd(a0)` | `q1_corpus_shape.asm(4): error: bad immediate expression` | exit 0, no diagnostic |
| `q2_no_string` | `#$5357,SEgg_ChildCmd(a0)`, same EA | exit 0, no diagnostic | exit 0, no diagnostic |
| `q3_string_simple_ea` | `#"SW",d0`, simplest EA | `q3_string_simple_ea.asm(3): error: bad immediate expression` | exit 0, no diagnostic |

Reading it. `p2` shows `MOMPASS` and a macro-body `if` ALREADY WORKED before the
change: the condition evaluated and executed the `fatal` it guards, which is the
correct answer for `(MOMPASS=1)&&(1)` on the first pass. `p5` shows parameter
substitution into a condition already worked. `p3` removes the macro and the
substitution entirely and still refuses, and `p4` removes the `if` and `MOMPASS`
too and still refuses. So `DEFINED` was the sole cause of both rows, and neither
of the other two candidates was implicated.

`p2` is also the control that MUST STILL BE RED, and is: its message is unchanged
across the two binaries. Without it, "every probe went quiet" would be equally
consistent with having broken the refusal machinery.

For Group A, `q2` holds the addressing mode, the displacement and the `equ` fixed
and removes only the string: it passed before. `q3` keeps the string and reduces
the operand to a register: it failed before. The string literal is the cause.

## The string-to-integer rule, as established

Each row is one asl probe, exit status quoted.

| probe | source | asl | exit |
|---|---|---|---|
| `sa` | `move.l #"A",d0` | `203C 0000 0041` | 0 |
| `sa` | `move.l #"AB",d0` | `203C 0000 4142` | 0 |
| `sa` | `move.l #"ABC",d0` | `203C 0041 4243` | 0 |
| `sa` | `move.l #"ABCD",d0` | `203C 4142 4344` | 0 |
| `sa` | `move.w #"SW",d0` | `303C 5357` | 0 |
| `sa` | `move.w #"AB"+1,d0` | `303C 4143` | 0 |
| `sa` | `move.l #"AB"*2,d0` | `203C 0000 8284` | 0 |
| `sc` | `move.l #"",d0` | `error #1141: expected integer, but got string` | 2 |
| `sb` | `move.l #"ABCDE",d0` | `error #1141` | 2 |
| `sf` | `move.b #"AB",d0` | `error #1320: range overflow` | 2 |
| `sg` | `K equ "ABCDE"` then `move.l #K,d0` | `error #1141` at the USE | 2 |
| `sd` | `move.l #"\xff",d0` | `203C 0000 00FF` | 0 |
| `sd` | `move.l #"\xff\xff",d0` | `203C 0000 FFFF` | 0 |
| `sd` | `move.w #"\x80\x01",d0` | `303C 8001` | 0 |
| `sl` | `move.l #"\xff\xff"+1,d0` | `203C 0001 0000` | 0 |
| `sl` | `move.l #("\xff\xff"<0),d0` | `203C 0000 0000` | 0 |
| `sl` | `move.l #"\xff\xff\xff"+1,d0` | `203C 0100 0000` | 0 |
| `sl` | `move.l #"\x80"+0,d0` | `203C 0000 0080` | 0 |
| `si` | Z80 `ld hl,"AB"` | `21 42 41` | 0 |
| `sm` | Z80 `ld hl,"ABCD">>16` | `21 42 41` | 0 |
| `sm` | Z80 `ld hl,"ABC">>8` | `21 42 41` | 0 |
| `ss` | Z80 `ld hl,"ABCDE"` | `error #1141` | 2 |

**The rule.** One to four characters, packed big-endian, each character
contributing its UNSIGNED byte; the result is an ordinary non-negative integer.
Zero characters or five or more is refused. The four-character limit belongs to
asl's integer type and not to the target word size, since it is four on the Z80
too. The conversion does not consult the target's width: `move.b #"AB"` converts
to `$4142` and THEN draws a range complaint.

**Two of these corrected my first reading.** I initially read the value as signed,
on `"\xff\xff\xff\xff"+1` coming back `0000 0000`. `sl` refutes it: `"\xff\xff"+1`
is `0001 0000`, not `0000 0000`, and `("\xff\xff"<0)` is 0. And I initially took
the wrap as a general property of asl's arithmetic on this target; `sh` refutes
that too, since `move.l #$FFFFFFFF+1,d0` is `error #1320` at exit 2 while the
identically-valued string form is `0000 0000` at exit 0.

**That last pair is a real divergence and is not modelled.** At exactly four
characters with every bit set, asl's overflow domain does something sigil does
not reproduce. It is not the substitution artifact: with `move.l #$1234,d0`
immediately above it the answer is still `0000 0000` and not `1234` (probes `so`,
`sp`, `sq`), so asl computed it. sigil treats the packed value as the plain
integer 4294967295, so `+1` is 4294967296 and the immediate's own range check
refuses it, which agrees with asl on the literal spelling and is LOUDER than asl
on the string one. No site in any of the four trees writes a four-character string
literal; the longest is two.

### A `dc`-family directive is not an expression

Probe `sr`, exit 0:

```text
       3/       0 : 0041 0042           	dc.w	"AB"
       4/       4 : 0041 0042           	dc.w	"AB"+0
       5/       8 : 4142                	dc.b	"AB"
       6/       A : 4142                	dc.b	"AB"+0
       7/       C : 0041 0042 0043 0044 	dc.w	"ABCD"
```

In a data directive a string is a CHARACTER SEQUENCE, one element per character at
the directive's width, and an operator DISTRIBUTES OVER THE ELEMENTS rather than
over a packed value. `dc.w "AB"+0` is `0041 0042`, never the packed `4142`. This
is the opposite of the expression rule, and it is where this change could have
been silently destructive: `dc.b` already consumed the shape before the expression
parser, but `dc.w`/`dc.l`/`dw` did not, and letting a string arrive at the packer
there would write one word where asl writes two. Those three now refuse a string
operand outright, preserving the loud behaviour they already had.

## The bytes at all sixteen newly-accepted sites

Not merely "the complaints went away". A little-endian packer would have removed
every one of these complaints and written the wrong word, and no diagnostic count
can see that.

**The six Sonic 1 sites** (`_incObj/82, 83 SBZ Eggman Cutscene and Crumbling
Floor.asm` lines 133, 163, 185, 255, 293, 312, with that file's
`SEgg_ChildCmd equ obSubtype` and `_Constants.asm`'s `obSubtype equ $28`). asl
probe `sites.asm`, exit 0, `1 pass`, `0 errors`:

```text
       8/       0 : 317C 5357 0028      		move.w	#"SW",SEgg_ChildCmd(a0)
       9/       6 : 337C 474F 0028      		move.w	#"GO",SEgg_ChildCmd(a1)
      10/       C : 0C69 5357 0028      		cmpi.w	#"SW",SEgg_ChildCmd(a1)
      11/      12 : 0C68 474F 0028      		cmpi.w	#"GO",SEgg_ChildCmd(a0)
      12/      18 : 337C 474F 0028      		move.w	#"GO",SEgg_ChildCmd(a1)
      13/      1E : 0C68 474F 0028      		cmpi.w	#"GO",SEgg_ChildCmd(a0)
```

**The ten skdisasm sites** (`sonic3k.asm` 100498 and 121694 to 121711). asl probe
`sksites.asm`, exit 0, `1 pass`, `0 errors`:

```text
       6/       0 : 0C79 4547 0020 0114 	cmpi.w	#"EG",(SSMagic_TestLoc_200114).l
       7/       8 : 0C01 0020           	cmpi.b	#" ",d1
       8/       C : 0C01 003F           	cmpi.b	#"?",d1
       9/      10 : 0C01 0021           	cmpi.b	#"!",d1
      10/      14 : 0C01 0026           	cmpi.b	#"&",d1
      11/      18 : 0C01 0029           	cmpi.b	#")",d1
      12/      1C : 0C01 0028           	cmpi.b	#"(",d1
      13/      20 : 0C01 002E           	cmpi.b	#".",d1
      14/      24 : 0C01 0049           	cmpi.b	#"I",d1
      15/      28 : 0401 0041           	subi.b	#"A",d1
```

sigil emits both sequences byte for byte, asserted in
`tests/as_string_literal_integer.rs`.

## What sigil's pass model can honestly answer for DEFINED

This was flagged as a possible BLOCKED. It is not, and the reason is worth writing
down: the answer sigil can honestly give turns out to be exactly asl's.

**asl carries a symbol's VALUE across passes and does not carry its DEFINEDNESS.**
Probe `db.asm`, exit 0, `2 passes`, `0 errors`. The `jmp Later` forces the second
pass and resolves on it; the query two lines below still reads 0:

```text
      12/       0 : 4EF8 0006           	jmp	Later
      13/       4 : 00                  	dc.b	DEFINED(Later)
      14/       5 : 00                  	dc.b	DEFINED(LaterEqu)
      15/       6 :                    Later:
```

and its guarded `message` arms print `pass1 Later=0` then `pass2 Later=0`.

So the question cannot be asked of sigil's `env`, which IS seeded from the previous
pass (`one_pass` does `asm.env = seed_env.clone()`) precisely so a forward reference
gets a value. Asking `env` would report every symbol in the program as defined from
the first line of every pass after the first. That is the naive implementation, and
on this corpus it would have looked right: the guard is `(MOMPASS=1)&&(DEFINED(loc))`,
and `MOMPASS=1` is false on every pass where the naive answer differs, so the eight
rows would have cleared either way while the general answer was wrong.

The honest answer needed no new architecture, only a second store: a
`defined_this_pass` set that is never seeded, written by a single
`define_sym` wrapper through which all eight symbol-defining sites (label,
`label` directive, `equ`, `set`, enum member, struct member, struct
instantiation, pad absorption) now pass. `env` says what the value is; the new set
says whether this pass has reached the definition yet. They answer different
questions and must be written together or they drift apart silently, which is what
the single writer is for.

### The rest of DEFINED, measured

| probe | source | asl | exit |
|---|---|---|---|
| `da` | `DEFINED(Early)`, defined above | `01` | 0 |
| `da` | `DEFINED(Later)`, defined below | `00` | 0 |
| `da` | `DEFINED(Nowhere)` | `00` | 0 |
| `da` | `defined(Early)` / `Defined(Nowhere)` | `01` / `00` | 0 |
| `dc` | `DEFINED(V)` after `V set 1` | `01` | 0 |
| `dc` | `DEFINED(E)` after `E equ 2` | `01` | 0 |
| `dc` | `DEFINED(MOMCPU)`, `DEFINED(TRUE)` | `01`, `01` | 0 |
| `dc` | `DEFINED(Mac)` after `Mac macro loc` | `00` | 0 |
| `dc` | `DEFINED(loc)` via macro parameter | argument substituted, then asked | 0 |
| `dc` | `DEFINED(E)+DEFINED(Nowhere)*4` | `01` | 0 |
| `dd` | `DEFINED((E))` | `00` | 0 |
| `dd` | `DEFINED( E )` with spaces | `00` | 0 |
| `dd` | `DEFINED(E.x)` | `00` | 0 |
| `de` | `DEFINED(1)` | `00` | 0 |
| `dg` | `DEFINED(E+1)` | `00` | 0 |
| `df` | `DEFINED equ 2` then `dc.b DEFINED` | `02` | 0 |
| `dh` | `DEFINED function x,x+100` then `DEFINED(E)` | `66` | 0 |

Four of these are worth naming. The FUNCTION NAME folds case while the ARGUMENT
does not (asl's symbol table prints `CASESENSITIVE : 1`). The argument is read as
a NAME and never evaluated, so anything that is not a bare name is 0 and not an
error. A bare `DEFINED` with no parenthesis is an ordinary symbol a program may
define. And a USER `function DEFINED` WINS over the builtin: `$66` is 102, which
is `E`'s 2 plus the function's 100. That last one decides the implementation's
arm order, and it was measured rather than guessed.

### One divergence, in the loud direction

`DEFINED( E )` with whitespace is 0 in asl even though `DEFINED(E)` is 1: the rule
is textual to the point that a space defeats it. sigil asks about TOKENS and the
lexer has already dropped the spaces, so the spaced form answers the same as the
unspaced one.

Reproducing the wart means asserting that the parenthesis and the name are adjacent
in the SOURCE TEXT. The spans do not survive macro expansion intact (an expanded
line is re-lexed against a base offset), and inside a macro body is exactly where
the corpus uses this, so the check would be unreliable precisely where it matters.
Taken deliberately: every site in all three disassemblies spends `DEFINED` on a
`fatal` guard, so answering 1 where asl answers 0 stops the build and says so,
rather than quietly changing bytes.

## Populations, and an instrument that was lying

Three claims here are absences, and an absence is only evidence once the same
filter has been shown to select THIS subject somewhere.

**A filter I used earlier could not fire at all.** `[ \t]` in an ERE bracket
expression is not a tab: it matches a space, a backslash or the letter `t`. Every
corpus line here is tab-separated, so `^[^;]*\b(dc\.w|dc\.l|dw)[ \t]+[^;]*"`
matched nothing anywhere, and its zeros read exactly like a finding. It was caught
by planting `dc.w "AB"` in a file and watching the filter miss it. The corrected
filter uses `[[:blank:]]` and fires on that same plant. **Every population below
was re-established with the corrected filter**; the earlier numbers are withdrawn,
including a `dc.b` count that had partially worked (it was matching only the
space-separated lines and I had read its partial hit as proof the filter worked).

| subject | filter | shown to fire on | s1disasm | s2disasm | skdisasm | aeon |
|---|---|---|---|---|---|---|
| `dc.w`/`dc.l`/`dw` with a string | `^[^;]*(dc\.w\|dc\.l\|dw)[[:blank:]]+[^;]*"` | a planted `dc.w "AB"` | 0 | 0 | 0 | 0 |
| `DEFINED(` | `\bdefined[[:blank:]]*\(` | the two real s1disasm sites | 2 | 2 | 2 | 0 |
| `dc.b`/`db` with a string | `^[^;]*(dc\.b\|db)[[:blank:]]+[^;]*"` | a planted `dc.b "hi"` | 56 | | | 0 |

Files scanned: 459 / 332 / 959 / 525 `.asm` and `.inc`. The `DEFINED` filter is
shown selecting this subject rather than merely running, by finding exactly the two
real sites in s1disasm before being run over aeon's 32,088 files.

**One more filter was unsound and its number is withdrawn rather than corrected.**
I counted `#[[:blank:]]*"` to size string immediates and got 648 for aeon. They are
worktree copies of one file, and the filter was matching the `#` INSIDE the string
literal `"#"` in `case "#"` and `substr(OPERAND,0,1)="#"`. No aeon string-immediate
count is asserted here; the construction argument below carries that claim instead.

## Aeon

**The byte gate was NOT run by me, and is owed at landing.** The brief forbids
touching any aeon tree or setting `AEON_DIR`, and it was not set: aeon's working
tree is as I found it and no build ran there.

What can be said without it, by construction rather than by measurement. Aeon
routes exactly three tracked `.asm` files through `sigil-frontend-as`
(`engine/debug/debugger.asm`, `games/demo/game_root.asm`,
`games/sonic4/game_root.asm`).

1. Before this change, a `Tok::Str` reaching `parse_atom` returned `None`, and
   every one of the 21 `parse_expr` call sites turns `None` into a diagnostic.
   Aeon builds green today, so **no aeon line reaches `parse_atom` with a string**.
   Any line the packer now folds would have been a build failure before.
2. The only dispatch order this change alters is `dc.w`/`dc.l`/`dw`, where a
   refusal is added ahead of `parse_expr`. Zero sites in aeon (table above).
3. `DEFINED(` appears in no aeon file (table above), so the new builtin arm is
   never entered.
4. `define_sym` adds a set insertion beside an existing `env.define`; nothing
   reads the set except `DEFINED`.

The 151 string-literal lines in those three files were classified by context:
`case`, `if`/`elseif`/`while`, `switch`, comments, `set`, `!error`, `include`,
`dc.b`, and one `strstr(...,"%<")` builtin argument. Every one of those is consumed
by `eval_str`, `expand_str_builtins`, `expand_str_comparisons` or `directive_db`
BEFORE `parse_expr`, and none of those orderings was touched.

## Verification

**Red-first, five mutations, each shown applied on disk and restored from the
committed baseline `0d79e449`.** The assertions are byte equalities, not message
matches, except where noted.

| mutation | must fail | observed |
|---|---|---|
| big-endian pack becomes little-endian | the corpus-site bytes | `left [.. 87, 83 ..]` vs `right [.. 83, 87 ..]`, `WS` for `SW`; 3 tests red |
| `c as u8` becomes `c as u8 as i8` | the unsigned property | `left [.. FF, FF, FF, FF ..]` vs `right [.. 00, 00, 00, FF ..]`; 1 test red |
| the wide-data guard removed from all 3 sites | the dc.w refusal | `expected a refusal, got bytes: [41, 42]`, a packed word where asl writes `0041 0042`; 1 test red |
| `DEFINED` answers from the carried `env` | the pass property | `left [1, 1]` vs `right [0, 0]`; 2 tests red |
| `define_label` bypasses `define_sym` | the single-writer property | `left [0, 0, 0, 0]` vs `right [0, 1, 1, 0]`; 2 tests red |

The third is the only message-based assertion, and it matches
`"string operand in a data directive"`, text unique to the constant this parcel
introduces.

**Suite.** 435 suites. Strict (no `SIGIL_ALLOW_PARTIAL`): 4519 passed, 382 failed,
2 ignored, exit 101. **All 382 are one cause** and none is a golden divergence: 379
are `test_support.rs:1341` "NO REFERENCE TREE IS NAMED", the provisioning refusal
for a run that has not been given an aeon reference tree, and the remaining 3 are
`PoisonError` from a mutex a sibling test in the same binary poisoned when it hit
that refusal. Declared partial (`SIGIL_ALLOW_PARTIAL=1`): 4901 passed, 0 failed, 2
ignored, exit 0, **of which 379 reference-dependent rows were not measured**, so
that 4901 is not a full green and is not reported as one. Clippy exit 0, 19
warnings, all pre-existing C vendor warnings, none naming a file this parcel
touches.

New tests: `crates/sigil-frontend-as/tests/as_string_literal_integer.rs` (9) and
`as_defined_builtin.rs` (9).

## Still open

- The `charset` seam. asl's `charset` remaps the code page, which is the
  CHARACTER-TO-BYTE step and nothing else; the packing sits on top of whatever
  byte a character maps to. `expr.rs::string_to_int` isolates that step (`c as u8`,
  the identity page, the `STANDARD (0 changed characters)` asl reports), so
  `charset` composes by supplying a mapping there rather than undoing the packing.
  The identical step lives in `eval.rs::directive_db`, and those two are the
  complete population a `charset` parcel must reach. A code page is assembler
  STATE and `expr.rs` is deliberately stateless, so that parcel will have to thread
  state in; the seam is which step it replaces, and it is one step, not two.
- The character-sequence form of `dc.w`/`dc.l`/`dw` is refused rather than
  implemented. Zero sites in four trees.
- asl's four-character overflow quirk (above) is recorded and not modelled.
- asl's `DEFINED( E )` whitespace wart (above) is recorded and not modelled.
- The aeon byte gate, owed at landing.
