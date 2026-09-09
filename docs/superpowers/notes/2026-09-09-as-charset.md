# `charset` measured: three consumers, a self-referential operand, and no scope at all

2026-09-09, branch `parcel/as-charset`. Every value below is a live reading from
`/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`,
`Macro Assembler 1.42 Beta [Bld 212] (x86_64-unknown-linux)`, md5
`61e672562465725a8c102288a7da9098`, invoked `-cpu 68000 -L -q -A`.

**Values are quoted only from runs that exited 0.** This build substitutes
stable-but-invented answers for a shape it declines, so a listing carrying any
error is not a source of values even for the lines that did assemble. Three
probes here were re-run for that reason, and one was rewritten outright because
its own construct confounded it.

## What `charset` is

The code page: the character-to-byte step, and nothing stacked on top of it.
`dc.b "AB"` maps each character and emits the mapped bytes; a string in an
expression maps each character and then packs the MAPPED bytes big-endian. The
packing does not move; only what goes into it does.

The index is the SOURCE character, so a mapping applies once and never chains:
after `charset 'A','X',$11`, `dc.b "A"` is `$11`, and `$11` is not looked up
again.

## The forms

```text
  charset                     reset the whole page to the identity
  charset SRC, TGT            TGT integer: map[SRC] = TGT
  charset SRC, "str"          map[SRC+i] = the RAW byte of str[i]
  charset LO, HI, BASE        map[LO+i] = (BASE+i) mod 256, i in 0..=HI-LO
```

**The brief that scoped this work described Sonic 1's sites as "eight `charset
lo,hi,base` forms". They are not.** `sonic.asm` 2616-2624 writes FIVE
two-operand forms and THREE three-operand ones:

```text
	charset ' ', $FF          2 operands
	charset '0','9',$00       3
	charset '$', $0A          2
	charset '-', $0B          2
	charset '=', $0C          2
	charset '>', $0D          2
	charset 'Y','Z',$0F       3
	charset 'A','X',$11       3
```

An implementation that read the brief and supported only the three-operand form
would have refused five of the eight.

## THE POPULATION IS THREE, NOT TWO

`expr.rs:73-83` said, of `string_to_int` and `eval.rs::directive_db`, that "the
two are the complete population a `charset` implementation must reach". **There
is a third: `lexer.rs`'s `'...'` character constant**, which packs its bytes
into an integer at LEX time with its own `ch as i64`. A page wired into the
other two leaves it on the identity.

asl maps it, and it is a packed INTEGER in every width rather than a character
sequence. Probe `p15.asm`, exit 0, `charset $49,$11` ('I' -> $11) live:

```text
       3/       0 :                     	charset $49,$11
       4/       0 : 114E 1154           	dc.l 'INIT'
       5/       4 : 203C 114E 1154      	move.l #'INIT',d0
       6/       A : 41                  	dc.b 'A'
       7/       B : 00                  <padding>
       7/       C : 114E                	dc.w 'IN'
```

The first attempt at this probe used `'INIT'` under `charset $41,$11` and read
`494E 4954` both before and after. That is not a finding, it is a confound:
`INIT` contains no `A`.

**Sonic 1 cannot detect the omission.** Its menu text is double-quoted, and its
own `charset` operands, though written as `'...'`, never index a character an
earlier `charset` moved. A build that leaves the lexer on the identity page
emits Sonic 1's 504 bytes correctly and says nothing.

asl has a FOURTH site that sigil does not: its wide data directives distribute a
string operand and translate each character. Probe `p11.asm`, `charset $41,$11`
live:

```text
       7/       0 : 0011 0042           	dc.w "AB"
       8/       4 : 0000 0011 0000      	dc.l "AB"
                A : 0042
```

sigil refuses that shape outright (`STRING_IN_WIDE_DATA`), so it is not a
consumer here. If it is ever implemented it is one on day one.

## The operand rule, which is the one to get wrong

`SRC`, `LO`, `HI` and `BASE` are ordinary integer expressions, so a character
literal written there is ITSELF translated through the page live at that moment.
Probe `p2b.asm`, exit 0:

```text
       3/       0 :                     	charset 'A',$11
       4/       0 : 11                  	dc.b 'A'
       5/       1 :                     	charset 'A',$20
       6/       1 : 11                  	dc.b 'A'
       7/       2 : 11                  	dc.b $11
```

The second `charset` does not move `A`. By then `'A'` evaluates to `$11`, so it
remaps index `$11`. The CONTROL, probe `p3.asm`, exit 0, is the same two lines
with a raw index:

```text
       4/       0 :                     	charset 'A',$11
       5/       0 :                     	charset $41,$20
       6/       0 : 20                  	dc.b 'A'
```

Without the control the first listing reads equally well as "a repeated
`charset` on the same character is ignored", a different rule with the same
byte. The same mechanism explains `charset 'A','C',$30` followed by
`charset 'B','B',$99` leaving `dc.b "ABC"` at `30 31 32`.

The string TARGET is the one place a character is NOT translated. Probe
`p8.asm`, exit 0:

```text
       5/       0 :                     	charset $7A,$05
       6/       0 :                     	charset 'a',"z"
       7/       0 : 7A                  	dc.b "a"
```

## Scope: there is none, measured four ways

| question | answer | probe |
|---|---|---|
| does the page reach into an `include`? | yes | `p4.asm` |
| does an include's `charset` leak back out? | yes | `p4.asm` |
| does a macro body's `charset` leak out? | yes | `p3.asm` |
| does `save`/`restore` bracket it? | **no** | `p7.asm` |
| is it reset at the start of each pass? | **yes** | `p4.asm`, `t3.asm` |

Include and macro, probes `p4.asm` and `p3.asm`, both exit 0:

```text
      10/       1 :                     	charset 'A',$11
      11/       1 :                     	include "p4inc.asm"
(1)    1/       1 : 11                  	dc.b "A"
(1)    2/       2 :                     	charset 'B',$22
(1)    3/       2 : 22                  	dc.b "B"
      12/       3 : 22                  	dc.b "B"

      23/       6 : 41                  	dc.b "A"
      24/       7 : (MACRO)              	mset
      24/       7 :                             charset 'A',$11
      24/       7 : 11                          dc.b "A"
      25/       8 : 11                  	dc.b "A"
```

`save`/`restore`, probe `p7.asm`, exit 0:

```text
       3/       0 :                     	charset $41,$11
       4/       0 : 11                  	dc.b "A"
       5/       1 :                     	save
       6/       1 :                     	charset $41,$44
       7/       1 : 44                  	dc.b "A"
       8/       2 : ALL                  	restore
       9/       2 : 44                  	dc.b "A"
```

**The raw `$41` index is load-bearing and the first attempt at this probe was
wrong because of it.** Written `charset 'A',$44`, the inner line is inert (by
then `'A'` is `$11`), the listing reads `11 / 11 / 11`, and it was written down
as "save/restore brackets the page". It does not; the probe was measuring its
own self-reference. This is why `p5.asm` was discarded and `p7.asm` written.

The pass boundary needs a probe where the two answers differ: the file must end
with a dirty page AND take more than one pass, so a page that carried across
would change the FIRST line of the last pass. Probe `p4.asm`, exit 0, `2
passes`, closing summary `STANDARD (2 changed characters)`:

```text
       7/       0 : 41                  	dc.b "A"
      10/       1 :                     	charset 'A',$11
```

`41`, with the page demonstrably still dirty at the end of the run.

## Range rules and refusals

`SRC`/`LO`/`HI` and an integer `TGT`/`BASE` must be `0..=255`; `LO > HI` is
refused; a string target running past `$FF` is refused. Probes `p10.asm`,
`p6.asm`, `p11.asm`, `p5.asm`, `p12.asm`:

```text
> > > p10.asm(3): error: range overflow          charset $100,$11
> > > p6.asm(18): error: range overflow          charset $41,$1FF
> > > p11.asm(11): error: range overflow         charset $41,-1
> > > p5.asm(12): error: range underflow         charset 'C','A',$70
> > > p12.asm(4): error: range overflow          charset $FE,"ABC"
> > > p8.asm(16): error: wrong number of operands charset 'A'
> > > p10.asm(4): error: wrong number of operands charset $61,$62,$63,$64
```

**Every refusal is ATOMIC**: no part of the mapping lands. Probe `p13.asm` is
the one that establishes this rather than assuming it:

```text
> > > p13.asm(4): error: range overflow
       4/       0 :                     	charset $FE,"ABC"
       5/       0 : FE                  	dc.b "\xfe"
```

Had asl applied the two in-range entries first, `$FE` would map to `A` and that
byte would read `41`.

Within an ACCEPTED range the target wraps: `charset $41,$43,$FE` gives
`dc.b "ABC"` = `FE FF 00` (probe `p7.asm`, exit 0).

## An operand that never resolves, and one that resolves late

Two answers from one construct, and conflating them costs either a silent wrong
byte or a refused legal source. Probes `p16.asm` (exit 2) and `p17.asm` (exit 0):

```text
> > > p16.asm(4):10: error: symbol undefined
       4/       0 :                     	charset NeverDefined,$11
       5/       0 : 41                  	dc.b "A"

       4/       0 :                     	charset Later,$11
       5/       0 : 11                  	dc.b "A"
       7/       1 : =$41                 Later	equ $41
```

**The first of these caught a real hole in this parcel's own implementation.**
`charset_index` folded through `eval_all`, and `eval_all` says NOTHING for an
operand that merely fails to resolve: its `Fold::Poison` arm names a register and
otherwise returns silently. So the first cut of this directive dropped the
mapping, emitted no diagnostic, exited 0, and handed back plain ASCII where the
source asked for the game's font. Measured on that binary: `41`, the same byte
asl prints, but at exit 0 where asl exits 2. A silent wrong byte, which is the one
outcome a code page must never produce. `charset_index` now reaches for its own
word, exactly as `align`, `ds` and the duplicate count do at the same choke point.

The forward reference must NOT be caught by that arm, and is not, because
diagnostics are returned from the converged pass alone: the pass-0 refusal is
superseded once the symbol has a value. sigil emits `11` at exit 0, as asl does.

## Where sigil and asl differ, stated rather than hidden

1. **`charset "AB",$11`.** asl says `wrong number of operands`; sigil packs the
   operand and says `charset operand 16706 out of range 0..=255`. Same line,
   same outcome (refused, nothing applied), different reason. asl evidently has
   a dedicated one-character-string path for operand 1 rather than routing it
   through the integer conversion, since `charset $4142,$11` on the same build
   draws `range overflow` instead.
2. **`dc.b "AB"+0`.** asl distributes the operator over the string's elements
   and emits `11 42` under a live page; sigil packs it to `$1142` and refuses it
   as out of range. **Pre-existing and unrelated to `charset`**: `expr.rs`
   already documents the rule and sigil's departure from it. It surfaced
   here and is recorded so the next reader does not re-derive it.

## The census

`scripts/s1-census.sh` over a pristine detached worktree of `s1disasm` at
`f6ece657`, generated includes READY 4/4, corpus dirty 0:

```text
before   9 diagnostics, all `charset` is not a recognized 68000 mnemonic
after    1 diagnostic

lines only in the NEW run:  1
  sonic.asm(81):3: error: sections `sec0` [0x0, 0x2CA) and `sec0#2` [0x0, 0x1BC6) overlap
lines only in the OLD run:  9
```

The 1 is the stage-boundary class the census predicted: `resolve_layout`'s
overlap at the Z80 phased block, which no front-end run could reach before. The
front end is now clean on Sonic 1 and the run gets to LAYOUT.

**The census's own re-derivation was needed.** The standing note recorded 17
diagnostics from two causes; by the time this parcel ran, the integer
`switch`/`case` cause had landed and the figure was 9. The note said to re-derive
rather than quote it, and it was right to.

The 504-byte figure re-derives from the listing: `LevelMenuText` at `$359E`, the
`charset` reset at `$3796`, `$3796 - $359E = $1F8 = 504`.


## WHICH GATE ACTUALLY SEES THIS, measured by breaking it

Four mutations, each one line, each applied and shown on disk with
`git diff --stat`, each restored from a commit:

| mutation | as_charset | census | four-shape ROM CRCs |
|---|---|---|---|
| M1 `directive_db` back to `c as u8` | 8 of 14 RED | **1, unchanged** | **12 of 12 green** |
| M2 `string_to_int` back to `c as u8` | **1** of 14 RED | **1, unchanged** | **12 of 12 green** |
| M3 lexer back to `ch as i64` | 2 of 14 RED | not run | not run |
| M4 `CodePage::reset` a no-op | 2 of 14 RED | not run | not run |

Each mutation ran a demonstrably different binary: baseline md5
`f49ae09f04397af7ac10cf9ab7383019`, M1 `e0ab40e1ac87488a08d50a8b80d4d5aa`, M2
`519ae4a5f2350d058702639f59bd5c13`.

**Neither of this repo's two corpus-wide instruments can see a broken code
page.** The four-shape ROM gate is blind structurally: aeon writes the token
`charset` 0 times across 205 `.asm`/`.emp`/`.inc` files, so no charset change can
move those bytes. The census is blind for a different and more interesting
reason: it counts REFUSALS, and a wrong byte is not one. Under M1, Sonic 1's own
menu text assembles to plain ASCII instead of the level-select font and the
census still reports the same single diagnostic.

So the four green CRCs reported for this parcel are a guard against incidental
damage from the 29 threaded call sites, and they are not evidence that `charset`
works. The evidence for that is `tests/as_charset.rs`, and for the expression
consumer specifically it is one test:
`a_string_in_an_expression_packs_the_mapped_bytes`, the only gate in the
workspace that goes red under M2.


## A correction to this parcel's own commit message

`eaf4ef29` says the compiler named "18 in the expression path and 11 in the
lexer". The expression half is right; **the lexer half is 13, and the total is
31, not 29.**

The 11 came from a regex that required the call's first argument to sit on the
same line as `self.state.cpu`, so it missed the multi-line call sites. A second
pass patched those without printing a count, and the first pass's number was
banked. Re-derived by counting occurrences rather than matching lines:

```
lexer sites       13
parse_expr sites  11
operands sites     7
TOTAL             31
```

Recorded rather than amended, because the point of the figure was that the
COMPILER did the enumeration and a reader did not, and that point survives the
wrong number. What did not survive is the number, and a later reader quoting 29
would be quoting a regex artifact.
