# AS's nameless temporary labels: the rules, the probes, and what the byte gate could not see

**Parcel** `parcel/as-nameless-labels`. **Subject** the bare `+`, `++`, `-`, `/`
an AS source writes in column 1 and branches to with a bare `+`/`++`/`-`
operand. Decision `d-22` accepted them for the AS compatibility surface only;
nothing here touches the `.emp` language, which is the owner's call under `d-6`
and is not ruled.

## Provenance

* Oracle: `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`,
  `Macro Assembler 1.42 Beta [Bld 212]`, md5
  `61e672562465725a8c102288a7da9098`, invoked `-q -A -L -U`.
  (Note that `s2disasm/build_tools/Linux-x86_64/asl` is a DIFFERENT binary,
  md5 `0dee1f98e6480a4783d27ffd8b90896f`; only the first is the reference.)
* Corpus: a private `rsync --exclude .git` copy of `s2disasm` at `e45ebf33`.
  The shared checkout was never read during a measured run and never written.
* Reference tree: `/home/volence/sonic_hacks/.aeon-sigil-ref`, detached at
  `ec640bcf`, `git status --porcelain` empty before and after every build here.
* Baseline sigil `17ce67d5` (master `6abe9488`); final sigil `2b0d1703`.

**Every listing quoted below is from a run that exited 0.** For this construct
that is not a formality. asl STOPS ITERATING ITS PASSES once a line errors, so
a listing from an errored run prints unconverged forward branches -- `60FE`, a
branch to itself -- for lines that are perfectly fine. The first draft of these
rules was written off a probe whose LAST line was refused, and it had the
forward direction backwards: all three forward references read as `60FE` and
looked like a rule. Deleting the one bad line turned the same file into `exit 0`
and three distinct correct answers.

## The rules

| | |
|---|---|
| definition `+` × m | forward counter += m; define the slot it lands on |
| definition `-` | backward counter += 1; define the slot it lands on |
| definition `/` | both counters += 1; define BOTH slots |
| reference `+` × k | forward slot `fwd + k` |
| reference `-` × k | backward slot `bwd - k + 1` |

Column 1 only. `--` and `//` in column 1 are refused; `++` is not.

### Ordinals, and `/` counting for both (`ord2.asm`)

```text
      4/    1000 : 6006                bra.s  +      ; -> $1008, the `+`
      5/    1002 : 6008                bra.s  ++     ; -> $100C, the `/`
      6/    1004 : 6008                bra.s  +++    ; -> $100E, the `+`
      8/    1008 :                     +
     10/    100A :                     -
     12/    100C :                     /
     14/    100E :                     +
     17/    1010 : 60FA                bra.s  -      ; -> $100C, the `/`
     18/    1012 : 60F6                bra.s  --     ; -> $100A, the `-`
```

The interleaving is the point: a `+` definition is forward-only (the `-` at
`$100A` is invisible to `++`), a `-` definition backward-only, and the `/` is
what `++` and `-` both land on from opposite sides.

### A `+` × m definition consumes m slots (`q8.asm`)

```text
      4/    1000 : 6004                bra.s  +      ; -> $1006, slot 1
      5/    1002 : 6004                bra.s  +++    ; -> $1008, slot 3
      7/    1006 :                     +
      9/    1008 :                     ++
```

This is the case that tells the two candidate models apart. Under "a `++`
definition is another spelling of `+`" the `++` would be slot 2. asl resolves
`+++` to it, so it is slot 3, and slot 2 is then never defined by anything --
which is why a `bra.s ++` in that same file is `error: symbol undefined`
(`q2.asm`).

The multi-character form is a `+` privilege: `--` (`q6.asm:6`) and `//`
(`q5.asm:6`) are both `error: invalid symbol name`.

### The same-line case, both directions (`q3.asm`, `q4.asm`)

```text
      6/    1004 : 60FE                -  bra.s  -   ; -> $1004, its OWN label
      4/    1000 : 6004                +  bra.s  +   ; -> $1006, the NEXT one
```

The asymmetry needs no rule of its own: a column-1 definition has already
advanced its counter by the time the rest of that line dispatches, so a
backward `-` (slot `bwd`) lands on the label the line just defined and a
forward `+` (slot `fwd + 1`) lands one past it. The backward half is 18 of the
corpus's references (`-\tdbf\td0,-`).

### The column rule (`q1.asm`)

```text
      4/    1000 : 60FE                bra.s  +
      6/    1004 :                       +          ; indented
      > > > q1.asm(6):2: error: unknown instruction
```

An indented `+` is not a label and defines nothing.

## The disambiguation, which is why the row was sized L

`+` and `-` are also operators. AS splits an expression at the RIGHTMOST
operator of the loosest precedence tier present, so in a leading run of `+`/`-`
the LAST one is the binary operator and everything before it is the left-hand
side -- which, being a bare run, is the nameless reference. Only when nothing an
operand could apply to follows the run is the whole run the reference.

A 32-expression battery was run one expression per file (an errored run poisons
every value in it). With `-` = `$1000`, `Base` = `$1020`, `+` = `$1030`:

```text
  dc.l -Base    -> FFFF EFE0   unary negation: one `-`, nothing to its left
  dc.l --Base   -> FFFF FFE0   (-) - Base: the LAST `-` is the operator
  dc.l +-Base   -> 0000 0010   (+) - Base, the offsetTableEntry shape
  dc.l +        -> 0000 1030   a whole run with nothing after it
  dc.l (+)      -> 0000 1030   parenthesised, one level down, same rule
  dc.l 1+(+)    -> 0000 1031
  dc.l --1      -> 0000 0FFF   (-) - 1

  dc.l 1+-2     -> error: wrong number of operands
  dc.l Base-+   -> error: wrong number of operands
  dc.l Base++   -> error: wrong number of operands
  dc.l +*2      -> error: wrong number of operands
  dc.l ~+       -> error: wrong number of operands
```

The refusals are as informative as the values. `1+-2` is the row that rules out
"a `-` after a binary operator is unary": AS splits it at the rightmost `-`, is
left with `1+`, and refuses that. Every refusal in the battery is predicted by
the rightmost-at-loosest-tier model, and so is every value; the model fits all
32 cases with no exceptions.

**sigil's Pratt parser is the same rule from the other side.** `parse_atom` runs
ONLY where an operand is expected, so reaching it with a `+`/`-` already means
"no left-hand side" -- AS's own condition, with no lookbehind and no
statement-level pre-pass. In a run of n leading `+`/`-`, if a token that can
begin an operand follows the run, the reference is the first n-1 and the last is
left in the stream for the infix loop; otherwise the reference is all n. The
reference part must be a pure run of one kind, else it is refused, which is
AS's answer too (`+--Base`).

### What that changes for expressions that already worked

Exactly one shape: `-` × m for m >= 2 before an operand. sigil folded it as m
nested negations (`--X` = `X`) and now reads it as `(nameless) - X`. Everything
else is either strictly new acceptance (`+` in atom position always failed
before; a run with nothing after it always failed) or bit-for-bit the old path
(one `-` before an operand takes the same unary arm).

Population of the changed shape:

* aeon's AS closure -- the three tracked `.asm` files, which are exactly the
  include closure of both `build.sh` routes, 899 lines: **zero**, measured with
  two independent instruments (a comment-stripping Python census and a grep),
  after the first grep was found vacuous (`[^...\]]` closes the bracket
  expression at the first `]`, so the pattern required a literal `]` and matched
  nothing; its canary did not fire and said so).
* Sonic 2 corpus: 14 lines, every one of them a nameless reference
  (`dbf d2,--`, `bra.s --`, one `dbf d5,---`).

## THE BYTE GATE IS BLIND HERE, and the dispatching brief said it was not

The brief for this parcel said: *"aeon's three `.asm` files on this route contain
ordinary arithmetic that goes through the very expression parser you are
changing, so a regression there can and should move ROM bytes"*, and asked for
that to be verified by breaking the change on purpose rather than taken on
trust. It was verified, and **it is false**.

Procedure: build all four shapes with `sigil build --aeon . --native --game
<g> [--debug]` into a scratch directory (never over the tree's own ROMs) and
compare crc32/size against the committed pins in
`crates/sigil-harness/golden/provenance.toml` at `aeon_rev ec640bcf`.

| run | s4 | s4_debug | demo | demo_debug |
|---|---|---|---|---|
| pins | `b09ccd65`/820229 | `1b7fe316`/846529 | `0ad17404`/96863 | `2565ece2`/103185 |
| CONTROL, baseline sigil `17ce67d5` | match | match | match | match |
| this parcel | match | match | match | match |
| **mutation A** ref_len = run (disambiguation broken) | match | match | match | match |
| **mutation B** unary negation DELETED from `parse_atom` | match | match | match | match |
| **mutation C** every integer literal +1 | BUILD FAILED | BUILD FAILED | match | match |
| **mutation D** `-` in atom position refused outright | match | match | match | match |

The control row is what makes the rest readable: the procedure reproduces all
four pins from the committed baseline, so a row that matches is a real match and
not a broken build script.

**Mutation D is the decisive one.** With a `-` in atom position made a hard
parse failure, all four shapes still build and still match their pins -- so the
arm this parcel changes is never reached by any AS input on aeon's build path.
The mechanism is not a mystery: the only `-` characters in the aeon closure that
are not comment banners are `-(sp)` predecrements, and `operands.rs::classify`
consumes that as an exact four-token shape before the expression parser is
reached.

**Mutation C says the route is live but narrow.** The AS frontend does run on
the sonic4 shapes -- `seam2.rs` assembles a synthetic carrier source, and bumping
its integer literal broke the build with a downstream `.emp` span guard. Note
what that row does NOT show: a silent byte MOVE. It shows a loud failure. The
demo shapes were byte-identical under mutation C, which means neither
`game_root.asm` nor `debugger.asm` contributes an integer-literal-derived byte
to that ROM at all.

So the honest statement of the gate's reach here is: **the four-shape CRC check
is structurally blind to the operand-parser arm this parcel touches, the same
way it was for `switch` and `charset`.** It was still worth running -- it costs
eight seconds and it is the only thing standing between an arithmetic regression
and aeon -- but it is not evidence that this parcel preserved arithmetic. The
evidence for that is the `arith.asm` differential below.

### What the evidence for arithmetic preservation actually is

`arith.asm`: every `+`/`-` expression shape asl ACCEPTS, plus the addressing
modes that spell a `+` or `-` and are not nameless labels, assembled in one file
and compared byte for byte against asl's listing. 102 bytes, IDENTICAL, with
three distinct reference values (`-` = `$1000`, `Base` = `$1020`, `+` = `$1084`)
so a wrong reading cannot coincide with a right one:

```text
  dc.l Base     0000 1020      dc.l (+)         0000 1084
  dc.l $1234    0000 1234      dc.l (-)         0000 1000
  dc.l +        0000 1084      dc.l (+)-Base    0000 0064
  dc.l -        0000 1000      dc.l Base-(+)    FFFF FF9C
  dc.l +-Base   0000 0064      dc.l 1+(+)       0000 1085
  dc.l -Base    FFFF EFE0      dc.l (+)+1       0000 1085
  dc.l --Base   FFFF FFE0      dc.l + -Base     0000 0064
  dc.l 1+2      0000 0003      dc.l -  Base     FFFF EFE0
  dc.l 1-2      FFFF FFFF      dc.l -(SIZE*2)   FFFF FF80
  dc.l -1       FFFF FFFF      move.b (a5)+,d0  101D
  dc.l --1      0000 0FFF      move.b -(a5),d0  1025
                               move.w #-1,d0    303C FFFF
                               move.l -4(a5),d0 202D FFFC
```

Nine probes were run through the same differential (`ord2 q2b q3 q4 q7 q8 q9
known arith`); all nine are byte-identical to asl.

**The differ itself was wrong once and said so.** Its first version concatenated
the listing's hex column without regard to addresses, so a four-byte `ds.b` gap
shifted everything after it and reported a divergence that did not exist. It now
places each byte at its listed address. The lesson is the ordinary one: a
comparison instrument that reports a difference is as suspect as one that
reports a match.

## The corpus effect, both directions

Private copy of `s2disasm` at `e45ebf33`, exact-line multiset diff:

```text
INPUT rows  before=5136   after=149
REMOVED   4,987 rows over 4,917 distinct coordinates
ADDED     0 rows, 0 distinct
UNCHANGED 149

removed by class:
   2624  bad operand expression
   2309  expected mnemonic, directive, or label
     49  bad word expression
      3  bad displacement expression in `X`
      2  `endm` is not a recognized 68000 mnemonic
```

* **Both runs REACH LINK.** `ROM size is $F9198 bytes` in each. Padding falls
  from `$B543` to `$989C`, which is the assembler emitting bytes for code it
  used to abandon.
* **Unresolved-symbol NAME sets are identical in both directions** -- the same
  eight names with the same counts (`zAbsVar.1upPlaying` ×6, `ixu`/`ixl` ×4,
  `zVar.1upPlaying` ×3, `.loop_counter` ×2, `iyu`/`iyl` ×2, `Snd_Sega.size` ×1).
  Nothing started resolving that should not have, and nothing stopped.
* **No slot name appears anywhere in the run.** A nameless reference that failed
  to resolve would surface as `unresolved symbol ` nameless+#N``; the count is 0
  (with a canary confirming the pattern fires).

**The count is reported, not predicted.** The sizing note's prohibition holds:
`5,761 - 4,985 = 776` was never a prediction and the number above is a
measurement of a different baseline (5,136 rows at `6abe9488`) anyway.

### The ADDED side was not empty on the first attempt, and that is the finding

The first implementation took the run to 153 rows, and a total would have called
that finished. The set diff showed two ADDED rows:

```text
   s2.asm(9132):5:  error: `rept` is not a recognized 68000 mnemonic
   s2.asm(21429):5: error: `rept` is not a recognized 68000 mnemonic
```

Both are `-\trept N` … `endm` … `dbf d0,-`: a nameless label sharing a line with
a BLOCK OPENER. Block structure is decided by the head alone, and
`head_of_tokens` returned `None` for a line whose first token is not an
identifier, so the line never reached `exec_rept`. Before this parcel the line
died on its `-` and the orphaned `endm` was the diagnostic; after it, the
orphaned `rept` was. These are gap (g) of the sizing note -- "references that
emit no row at all because the definition on their line already failed" --
arriving exactly as promised.

The fix peels the nameless run in `head_of_tokens` and binds it in
`bind_head_label`. Both halves are needed: peeling alone routes the line
correctly and leaves its label undefined, so the `dbf d0,-` underneath would
count one definition too few -- a branch to the wrong address with no diagnostic,
strictly worse than the error it replaced. asl assembles the shape and sigil now
matches it byte for byte (`q9.asm`: `7001 4e71 4e71 4e71 51c8 fff8`).

## The one call made on instinct, measured afterwards: pad absorption

A LONE label on its own line takes the address AFTER a pad the next line
inserts (`absorb_pad_into_lone_label`). The nameless definition was wired into
that machinery because that is what a named label does, which is a reason and
not evidence, so it was measured. The named twin sits in the SAME probe file, so
the two answers are comparable rather than two separate runs (`q12.asm`,
exit 0):

```text
      5/    1000 : 11                  dc.b  $11
      6/    1001 :                     Lone
      7/    1001 : 00                  <padding>
      7/    1002 : 2233                dc.w  $2233
      8/    1004 : 44                  dc.b  $44
      9/    1005 :                     -
     10/    1005 : 00                  <padding>
     10/    1006 : 5566                dc.w  $5566
     11/    1008 : 60F8                bra.s Lone   ; -8 -> $1002, PAST the pad
     12/    100A : 60FA                bra.s -      ; -6 -> $1006, PAST the pad
```

Both labels moved; sigil is byte-identical (`110022334400556660f860fa`). The
call was right.

**The first probe of this could not have told me that.** `q10`/`q11` used an
explicit `align 2`, and there NEITHER label moves: both stay before the pad, so
named and nameless agree for a reason that has nothing to do with the rule. A
green from that pair would have looked exactly like the green above and meant
nothing.

## Measured and deliberately NOT implemented: macro-body scoping

asl scopes a nameless definition made INSIDE A MACRO BODY to that expansion. A
`+` defined in a body is invisible to a reference outside it, and -- measured,
`mb.asm` -- it does not merely fail to satisfy that reference, it makes the
reference `error: symbol undefined` even though a later definition outside the
macro would otherwise have served it:

```text
      8/    1000 :                     	bra.s	+
     10/    1002 : (MACRO)              	defplus       ; body defines a `+`
     12/    1006 :                     +                 ; and one outside, too
      > > > mb.asm(8):8: error: symbol undefined
```

The counters here are global to the pass, so sigil resolves that reference to
the outer definition instead. The construct has **zero** occurrences in the
corpus this feature exists for: the census over all five `s2disasm` sources
(98,672 lines) finds 2,339 column-1 definitions -- `+` 2,010, `-` 315, `/` 14 --
and none of them inside a macro body. The dispatching brief's item 5 said
"scoping is per expansion instance", which is true of asl and has no consumer;
what the corpus actually needs is references arriving through macro ARGUMENTS
and inside macro BODIES, and both work (shapes 2 and 3 of `known.asm`).

Recorded rather than implemented, in `src/nameless.rs` where the next reader
will be standing.

## Things in the dispatching brief that turned out wrong

1. **"The byte gate is live for this parcel."** Refuted by mutation D above.
   It is blind to the arm this parcel changes, and blind to every integer
   literal in the AS inputs for the demo shapes.
2. **Two anchors, not one, and a third.** The brief's freshly-checked anchors
   (`eval.rs:3634`, `operands.rs:130` and `:391`) were all correct. But the
   definition side needed a change at a THIRD site the brief does not mention --
   `head_of_tokens`, the block-structure scanner -- and that site is invisible
   from the diagnostic stream, because the rows it produces are attributed to
   `endm`.
3. **Item 1's "a run of `+`, `-` or `/` in column 1"** is right for `+` and
   wrong for the other two: `--` and `//` are `invalid symbol name` to asl.
4. **Item 4's "`++` is the second following definition"** is right for
   REFERENCES and does not describe DEFINITIONS, where a `++` consumes two
   slots and leaves a gap.
5. **Item 6's "18 references go live the moment definitions parse"** is right,
   and two of them went live as new diagnostics rather than as working code.
   The brief predicted the direction; it is worth recording that the honest
   ADDED count was 2 and then 0, and only a both-directions set diff could tell
   the difference.
