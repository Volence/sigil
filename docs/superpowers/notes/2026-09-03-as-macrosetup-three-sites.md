# The three largest sites are two faults, and the one that mattered emitted nothing

2026-09-03 · branch `parcel/as-macrosetup-sites` · sigil master base `8d9ff98b`

Eighth in the AS-frontend arc for the public Sonic 2 disassembly
(`/home/volence/sonic_hacks/s2disasm`, git `e45ebf3`). The seventh parcel left
the three largest remaining sites all in `s2.macrosetup.asm` — 996, 749 and 749
diagnostics — with nobody having looked at them.

Ground truth throughout is an `asl -L` listing (AS V1.42 Beta Bld 212,
`s2disasm/build_tools/Linux-x86_64/asl`) invoked with the Sonic 2 build's own
flags minus the two that only redirect output: `asl -xx -n -q -A -L -U -i .`
(`build_tools/lua/common.lua:773`). **`-U` forces case-sensitivity** and every
row below carries it. Every rule is stated with the row that establishes it, and
every expected value in the eight tests is such a row.

## The diagnosis: two causes, not three — and a third fault under them

The three sites are `:104` (996), `:59` (749) and `:62` (749).

| site | source | class |
|---|---|---|
| `:104` | `chkop function op,ref,(substr(lowstring(op),0,strlen(ref))<>ref)` | `strlen(): could not evaluate string builtin` |
| `:59` | `if ($)&1` — the Z80 arm of `even` | `` `$` with no hex digits `` |
| `:62` | `endif` — `even`'s outer closer | `` `endif` is not a recognized 68000 mnemonic `` |

**`:59` and `:62` are ONE cause.** The equal counts are the first sign and the
mechanism is the confirmation: the `if` at `:59` is lost, so the `endif` at
`:61` closes the OUTER conditional and the one at `:62` has nothing left to
close. Two diagnostics per invocation, 749 invocations.

**Neither of the two is what the sites look like.** `:59` is not about `$` and
`:104` is not about `strlen`. And underneath both sits a third fault that
emitted **zero** diagnostics in the entire corpus.

## Cause A — a head survives an operand that does not lex

A line's OPERAND may fail to tokenise while the token that decides what the line
IS sits at the head. `dispatch_head` lexed with `lex_line(...).ok()?`, so such a
line was invisible to block-structure scanning.

asl counts a nested `if`/`endif` inside a branch it never evaluates. The `[n]`
column says which line each closer closes:

```text
       2/       0 : =>FALSE                 if 0
       3/       0 :                             if ($)&1
       4/       0 :                                     dc.b 0
       5/       0 : [3]                         endif
       6/       0 :                             if Undefined_Symbol
       7/       0 :                                     dc.b 1
       8/       0 : [6]                         endif
       9/       0 : [2]                     endif
      10/       0 : 07                      dc.b 7
```

`[3]` closes line 3, `[6]` closes line 6, `[2]` closes line 2. asl evaluates
neither `($)&1` nor `Undefined_Symbol`; it only needs the head. Lose the head
and the inner `endif` pops the OUTER frame: the conditional ends early, whatever
followed inside it escapes into the emitted image, and the real `endif` is left
over as a diagnostic pointing at a line that is not the fault.

The byte witness, master against the branch on the same file:

```text
       3/       1 : =>FALSE                 if 0
       4/       1 :                             if ($)&1
       6/       1 : [4]                         endif
       7/       1 :                             dc.b $55
       8/       1 : [3]                     endif
       9/       1 : 22                      dc.b $22
```

asl `11 22`. Branch `11 22`. Master: no image at all, plus a spurious
`` `endif` is not a recognized 68000 mnemonic `` at line 8.

`lex_line_recover` returns the tokens lexed before the failure ALONGSIDE the
diagnostic; `lex_line` keeps its all-or-nothing contract for every caller that
needs a complete stream to mean anything.

### The other side of the recovery, which is where a recovery goes wrong

A head recovered from a partly-lexed line may be COUNTED, but its truncated
arguments must never be EVALUATED: folding them answers a question the source
did not ask, and it does it silently. `line_kw_args_checked` raises the lex
diagnostic and withholds the keyword, so `exec_if` declines the arm loudly. The
diagnostic quality improves at the same time — master reported the `$` AND a
cascade at the orphaned `endif`; the branch reports only the line that carries
the fault.

### What did NOT reproduce, and why it is worth saying

A truncated `macro` body is the byte-silent shape of this fault, so it was
probed directly: a `macro` whose body holds `if 0 / rept ($) / … / endm / endif`
— the `endm` that would close the invisible `rept` is also the MACRO's closer.

```text
      11/       0 : (MACRO)                 m
      11/       0 : 11                          dc.b $11
      11/       1 : =>FALSE                      if 0
      11/       1 :                                     rept ($)
      11/       1 : [11]                         endif
      11/       1 : 22                          dc.b $22
      12/       2 : 33                      dc.b $33
```

asl `11 22 33`. **Master also gives `11 22 33`.** `find_block_end` carries a
STACK of expected-closer SETS rather than a depth count, so the `if`'s frame
expects only `endif` and the stray `endm` is ignored rather than mistaken for
the macro's. The design already in place absorbs it. A desync therefore always
consumes a CLOSER and always leaves an unmatched one behind — so cause A is
loud, at the wrong line, and never silent.

## Cause B — parentheses around a string expression

Not a fact about functions. asl folds all eight of these:

```text
       2/       0 : 03                      dc.b strlen(("abc"))
       3/       1 : 03                      dc.b strlen((("abc")))
       4/       2 : 02                      dc.b strstr(("hello"),("ll"))
       5/       3 : 02                      dc.b strlen(substr(("hello"),0,2))
       6/       4 : 04                      dc.b strlen(lowstring(("ABCD")))
       7/       5 : 00                      dc.b (("he"))<>"he"
       8/       6 : 01                      dc.b ("he")<>("hf")
       9/       7 : 02                      dc.b strlen(( "ab" ))
```

Master failed all eight. Parentheses are transparent around a string expression
exactly as they are around a numeric one, in three places: `eval_str` peels a
balanced enclosing pair, a comparison's RHS accepts a group holding a string,
and `trailing_str_expr_len` accepts a bare group as a LHS candidate (with
`eval_str` still deciding whether it IS a string, so an ordinary numeric
`(a+b)=…` is untouched).

That is what `:104` needs, because `expand_calls` substitutes every user
`function` argument PARENTHESISED — so `chkop`'s own `strlen(ref)` is handed a
`("0(")`:

```text
       5/       0 : 03                      dc.b slen("abc")
       6/       1 : 00                      dc.b chkop("0(a0)","0(")
       7/       2 : 01                      dc.b chkop("d0","0(")
       8/       3 : 02                      dc.b strlen("0(")
       9/       4 : 00                      dc.b sub2("hello",2)<>"he"
```

`strlen` on a LITERAL always worked (`:8`). It is the parameter that could not
reach it.

## The fault underneath: `MOMCPU`, `TRUE` and `FALSE`

`MOMCPU` is the selected CPU as an integer, and asl reports it in hex:

```text
       2/       0 : 0006 8000               dc.l MOMCPU       ; cpu 68000
       2/       0 : 80 00                   dw MOMCPU         ; cpu z80
       2/       0 : 0100                    dc.b TRUE,FALSE
```

`$68000` and `$80`; `TRUE` 1 and `FALSE` 0. None of the three was defined. A
builtin outranks the symbol table, which is asl's own rule rather than a
simplification — it refuses `TRUE = 7` and `MOMCPU = 9`
(`error #2035: variables cannot be redefined as constants`) and goes on
reporting 1 and `$68000`.

`s2.macrosetup.asm:14` is
`notZ80 function cpu,(cpu<>128)&&(cpu<>32988)`, and `notZ80(MOMCPU)` picks the
arm of `org` (`:18`), `cnop` (`:38`), `align` (`:47`), `even` (`:52`) and `ds`
(`:66`). `:76` is `if TRUE`. Undefined, **every one of them read FALSE**:

```text
       3/       0 : =>TRUE                  if notZ80(MOMCPU)
       4/       0 : 11                          dc.b $11
       5/       1 : =>FALSE                 else
       8/       1 : =>TRUE                  if TRUE
       9/       1 : 33                          dc.b $33
```

asl `11 33`. Master `22 44`, **exit code 0, not one diagnostic**. The branch
`11 33`.

### How it stayed invisible, and what found it

The master corpus run names `MOMCPU` **zero** times and `TRUE`/`FALSE` **zero**
times. Grep is the whole search, and the absence is the finding: a name consumed
by an `if` condition never reaches an operand, so it never becomes an
unresolved-symbol diagnostic — the condition just reads false and the wrong arm
assembles.

What surfaced it was **reaching link**. `sigil s2.asm` stops at the front end;
a four-line probe assembled and linked says immediately

```text
error: unresolved symbol `MOMCPU` for fixup in section sec0 at offset 0
```

The eight tests use `linked_image`, not `image`, for exactly this reason: an
undefined `MOMCPU`/`TRUE` survives the front end as a deferred fixup, and a
front-end-only assertion about it would be vacuous.

Adding `TRUE`/`FALSE` moved the corpus by **zero** diagnostics — the before and
after multisets are identical. What it changed is which block `:76` assembles.
That is stated rather than hidden: a fix worth making is not always a fix a
count can see.

## Corpus movement

Same command both times, `sigil s2.asm` from `s2disasm`. `before` is master
`8d9ff98b` built in its own worktree with its own target directory and measured
with the same script.

| | before | after |
|---|---|---|
| diagnostics | 15,622 | **13,830** |
| distinct classes (file × normalized message) | 72 | 71 |
| distinct unresolved symbols | 293 | 291 |
| distinct source lines carrying a diagnostic | 8,661 | 8,698 |

Every class that moved, with the sums reconciling to the totals — no
unclassified remainder:

| class | before | after |
|---|---|---|
| `strlen(): could not evaluate string builtin` (`macrosetup`) | 996 | **0** |
| `` `…` with no hex digits `` (`macrosetup`) | 752 | **0** |
| `` `…` is not a recognized 68000 mnemonic `` (`macrosetup`) | 751 | **2** |
| `bad operand expression` (`macrosetup`) | 114 | 14 |
| `absolute address operand … needs an explicit width suffix` (`macrosetup`) | 32 | **0** |
| `unsupported form: Add requires a Dn operand` (`macrosetup`) | 2 | **0** |
| `unresolved symbol … in operand` (`macrosetup`) | 84 | 202 |
| `` `…` is not a recognized 68000 mnemonic `` (`s2.macros.asm`) | 273 | 333 |
| `macro `…` expansion too deep` (`macrosetup`) | 0 | 626 |
| `operand -N out of range` (`s2.asm`) | 0 | 34 |
| `org needs a constant expression` (`macrosetup`) | 0 | 1 |
| **every other class** (66 of them) | — | **unchanged, to the count** |

The unresolved-symbol SETS were compared sorted, in both directions: **zero
newly unresolved**, and two newly RESOLVED — `d0` and `d4`, register names that
`insn2op` had been feeding to the symbol resolver.

### Five classes rose, and every one of them is new ground

The `MOMCPU` fix restores the 68000 arm of `org`/`cnop`/`align`/`even`/`ds`, so
the corpus is being laid out at addresses at all. The frontend gets further, and
what it reaches next is the rise. Each was checked individually:

- **`macrosetup:68` — 626.** `!ds.ATTRIBUTE ALLARGS`. `!name` forces AS's
  builtin over a same-named user macro; sigil strips the `!` and dispatches,
  which re-enters the `ds` MACRO and recurses. The code says so itself: *"Core
  carries no macro that shadows a builtin, so the escape reduces to: strip the
  `!` and dispatch."* This corpus has one. **Now the largest site in the corpus.**
- **`macrosetup` unresolved-in-operand — +118**, at `:127` (98), `:141` (54),
  `:135` (1). These are `insn2op`'s `!oper x,1+y` arms, unreachable until
  `chkop` folded. `1+4(a1)` alone assembles correctly; the corpus operands do
  not.
- **`s2.macros.asm:62`/`:63` — 30 each.** `warning` and `exitm`, in `clearRAM`'s
  `elseif startaddr==endaddr` arm, unreachable until that conditional evaluated.
- **`s2.asm` out-of-range — 34**, one per line, all `.w` immediates naming a
  `$FFFF…` RAM label (`cmpa.w #VDP_Command_Buffer_Slot,a1`,
  `subi.w #MainCharacter,d5`). asl processes these lines — its own run reaches
  line 90887 before it stops on a missing generated include — and reports
  nothing at any of them. A real divergence, and whether it is the range check
  or the symbol's value is open.
- **`macrosetup:40` — 1.** `org needs a constant expression`, in `cnop`.

## What stays open, with measured sizes

| row | size | what it is |
|---|---|---|
| `AS-MACRO-BANG-SHADOW` | **626** | `!name` must bypass a user macro of that name, not merely strip the `!`. `macrosetup:68`. |
| `AS-MACRO-ARGCOUNT-IRP` | 272 | already booked. `s2.macros.asm:289` (`irpc`). |
| `AS-INSN2OP-DISP-OPERAND` | 162 | `insn2op`'s `1+y` arms. `macrosetup:127`/`:141`/`:135`/`:148`/`:115`. |
| `AS-WARNING-EXITM` | 60 | the `warning` and `exitm` directives. `s2.macros.asm:62`/`:63`. |
| `AS-WORD-IMM-RAM-LABEL` | 34 | a `.w` immediate naming a `$FFFF…` label, which asl accepts and sigil refuses. |
| `AS-ATTRIBUTE-BANG-ADDI` | 40 | `!addi.ATTRIBUTE`/`!subi.ATTRIBUTE`. `macrosetup:224`/`:227`. Pre-existing, unmoved. |
| `AS-CNOP-ORG-CONST` | 1 | `macrosetup:40`. |

`sound/_smps2asm_inc.asm` remains the largest single FILE at 3,520 before and
after, spread over many lines at 67 each; it is untouched by this parcel and is its own arc.

## Verification

- Eight tests, every expected value a listing row above:
  `momcpu_and_true_false_are_builtin_values`,
  `momcpu_and_true_select_the_arm_asl_selects`,
  `even_macro_takes_the_68000_arm_and_pads`,
  `unlexable_operand_does_not_break_conditional_nesting`,
  `arm_head_that_does_not_lex_is_loud_not_guessed`,
  `parentheses_around_a_string_expression_are_transparent`,
  `function_parameters_reach_the_string_builtins`,
  `lex_line_recover_keeps_the_head_before_a_bad_operand`.
- Red-first, three mutations, each applied and read back from disk with
  `git diff`, each restored from the committed baseline: reverting the lex
  recovery reddens 3, reverting the paren peel reddens 2, reverting
  `MOMCPU`/`TRUE`/`FALSE` reddens 3. Baseline and restored runs both green.
  The lexer's own test is not covered by a mutation — removing
  `lex_line_recover` fails to COMPILE rather than failing red.
- Landing run GREEN: 4310 passed, 0 failed, 2 ignored, 376 suites, cargo exit 0,
  reconciling 4302 (master baseline, its own worktree and target directory) + 8.
- Clippy `--release --workspace --all-targets -- -D warnings` exit 0.
- Aeon byte-neutral: all four artifacts deleted, each shape rebuilt in its own
  invocation with `SIGIL_VERSION_STRICT=1`, all four CRC32+size identical —
  s4 `14ee2440`/719700, s4.debug `142294b3`/737683, demo `0c456778`/96474,
  demo.debug `2e603d53`/101339.

## What the dispatching brief got wrong

The macros it named — `insn1op`/`insn2op` at `:111`/`:122`, `lea_`/`_btst`/
`_beq`/`_bne` at `:236`-`:262`, `jsrto`/`jmpto`/`jmpTosInternal` at
`:284`-`:320` — carry **none** of the three sites. The brief said its line
numbers were a reading rather than a measurement, and they were.

The prior note's hypothesis — that what still fails here is *"`ARGCOUNT`, `irp`
and the Z80 `$` program counter"* — **did not survive**. `ARGCOUNT`/`irp` are
not in the top three at all, and `$`-as-Z80-PC already works: sigil lexes bare
`$` as the program counter under `cpu z80` and always did. `:59`'s `$` error is
correct 68000 behaviour (asl refuses bare `$` there too,
`error #1020: invalid symbol name`) reported from a line that should never have
been reached.
