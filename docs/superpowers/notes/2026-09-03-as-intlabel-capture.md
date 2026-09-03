# A macro can take the label written on the line that invoked it

2026-09-03 · branch `parcel/as-intlabel` · sigil master base `6446259c`

Seventh in the AS-frontend arc for the public Sonic 2 disassembly
(`/home/volence/sonic_hacks/s2disasm`, git `e45ebf3`). The sixth parcel settled
how a `.`-local's scope is decided and left three spellings standing together as
the largest remaining block: `{INTLABEL}`, `__LABEL__`, and the `label`
directive.

Ground truth throughout is an `asl -L` listing (AS V1.42 Beta Bld 212,
`s2disasm/build_tools/Linux-x86_64/asl`), invoked with the Sonic 2 build's own
flags minus the two that only redirect output: `asl -xx -n -q -A -L -U -i .`
(the build adds `-E` to send errors to a log and `-c` for a shared header;
`common.lua:773`). **`-U` forces case-sensitivity** and every row below carries
it. Every rule is stated with the row that establishes it, and every expected
value in the twelve tests is such a row.

## The three constructs

`{INTLABEL}` in a parameter list says: the label on the invocation line is
**mine**. The assembler does not define it; the macro reads it as `__LABEL__`
and places it — usually with `label`, which binds a name to an expression as an
address.

```text
   8/    1000 : (MACRO)              Table:	offsetTable
   8/    1000 : =$1000               current_offset_table := Table
   8/    1000 : =$1000               Table label *
   9/    1000 : 0002                	dc.w Target-Table
  12/    1004 : 1000                	dc.w current_offset_table
```

That is `s2.macros.asm:155`, the anchor site, whole. The listing shows the
SUBSTITUTED text — `Table label *`, not `__LABEL__ label *` — which is the first
thing it tells you about what `__LABEL__` is.

### The capture SUPPRESSES the definition; it does not duplicate it

Two macros differing only in the group, under identical labels:

```text
  10/    1000 : (MACRO)              LabA:	sup
  10/    1000 : 4E71                        nop
  11/    1002 : (MACRO)              LabB:	nosup
  11/    1002 : 4E71                        nop
  12/    1004 : 1002                	dc.w LabB
```

`LabB : 1002 C` is in the symbol table. **`LabA` is not there at all.** A
capture the body drops leaves no symbol behind — so the label is not "defined
and also captured", it is captured INSTEAD of defined.

### It consumes no argument position, wherever it is written

Three macros differing only in where the group sits, all called `11,22`:

```text
  13/    1000 : (MACRO)              L1:	m 11,22        ; m macro {INTLABEL},pp,qq
  13/    1000 : 0B16                        dc.b 11,22
  14/    1002 : (MACRO)              L2:	n 11,22        ; n macro pp,{INTLABEL},qq
  14/    1002 : 0B16                        dc.b 11,22
  15/    1004 : (MACRO)              L3:	o 11,22        ; o macro pp,qq,{INTLABEL}
  15/    1004 : 0B16                        dc.b 11,22
```

`pp`/`qq` bind identically in all three. Declaring the group TWICE is not an
error and means what declaring it once means. Both the group and the reference
FOLD CASE even under `-U` — they are AS keywords, not symbols:

```text
  10/    1000 : 3C41 613E 3C41              dc.b "<Aa><Aa>"     ; lo macro {intlabel}, body "<__LABEL__><__label__>"
  11/    1008 : 3C5F 5F4C 4142              dc.b "<__LABEL__>"  ; nd macro pp — no declaration, no substitution
```

A macro that does not declare the group leaves `__LABEL__` as the nine ordinary
characters it is written with. So the capture is not a global name: it exists
only where it was asked for.

### An absent label is the EMPTY text, and the bare `label` line is inert

```text
  10/    1000 : 5B46 6F6F 5D                dc.b "[Foo]"
  10/    1005 : =>TRUE                       if "Foo"<>""
  10/    1005 : =$1005               Foo label *
  11/    1005 : 5B5D                        dc.b "[]"
  11/    1007 : =>FALSE                      if ""<>""
  11/    1007 :                      label *
```

This is what makes `s2.sounddriver.asm:284`'s `if "__LABEL__"<>""` a guard. Note
the second `label *`: a `label` line whose name field is empty defines nothing,
and is **not** a diagnostic. Note also `Foo : 1005`, not `1000` — the capture is
placed where the body puts it, not where the call sits.

## `__LABEL__` is a substitution, not a binding

It is a macro parameter in every respect but how it is bound. That is the whole
design, and it is what makes it compose:

```text
  11/    1000 : =$1000               Tbl label *
  11/    1000 : 7A6F 6E65 616E              dc.b "zoneanimcount_Tbl"
  11/    1011 : =$1011               Prefix_Tbl: label *
  11/    1013 : =$1013               Tbl_End label *
  12/    1013 : 1000 1011 1013      	dc.w Tbl,Prefix_Tbl,Tbl_End
```

Suffix inside a string, interior segment, prefix of a colon label — all four
corpus idioms, one mechanism. It also passes THROUGH a call: `__LABEL__ inner aa`
substitutes to `Tbl inner 3`, and `inner`'s own `{INTLABEL}` captures `Tbl`
(`s2.macros.asm:180`'s `zoneOrderedOffsetTable`).

### The boundary rule, which turned out to be one rule for four names

`__LABEL___End` composing at all requires a substitution to survive a trailing
`_`. The frontend's rule was "the character before AND after must both be
non-identifier", with `_` an identifier character — under which
`__LABEL___End` cannot compose. Nine positions, run once for a parameter and
once for the capture:

```text
  10/ 1000 : dc.b "1[_Zz] 2[1pp] 3[Xpp] 4[.Zz] 5[ppX] 6[pp1] 7[Zz_] 8[(Zz)] 9[__Zz__] A[Foo_Zz_Bar] B[xALLARGSx] C[_Zz_]"
  11/ 1065 : dc.b "1[_Qq] 2[1__LABEL__] 3[X__LABEL__] 4[.Qq] 5[__LABEL__X] 6[__LABEL__1] 7[Qq_] 8[(Qq)] A[Foo_Qq_Bar]"
```

Position by position, the parameter and the capture answer the same. The rule
the rows state:

> A candidate is rejected when an ALPHANUMERIC character abuts an edge of it
> that could continue an identifier. `_` is an identifier character and is NOT
> alphanumeric, so it never blocks.

`_pp` and `pp_` substitute; `Xpp` and `ppX` do not. `ALLARGS` is in the same
class (`xALLARGSx` verbatim, `_ALLARGS_` → `_Zz_`) — the frontend had it as an
UNBOUNDED match, which is a third answer neither asl nor the parameter rule
gives.

`.ATTRIBUTE` differs from the other three ONLY through the per-edge test. Its
first character is `.`, which cannot continue an identifier, so no leading check
applies — which is exactly why the glued-mnemonic use works:

```text
   8/ 1002 : 505B 6D6F 7665     dc.b "P[move.w] Q[x.w] R[.ATTRIBUTEx]"
   8/ 1002 : 3001                       move.w d1,d0
```

`move.ATTRIBUTE` and `x.ATTRIBUTE` both substitute; `.ATTRIBUTEx` does not. The
frontend's comment argued from `move.ATTRIBUTE` that the match must be unbounded
in BOTH directions. Only the leading half follows; the trailing half is the
ordinary rule. Asserted directly — `strlen("x.ATTRIBUTEy")` — asl says twelve:

```text
   8/ 1000 : 0C          dc.b strlen("x.ATTRIBUTEy")
   8/ 1001 : 05          dc.b strlen("x.ATTRIBUTE y")
```

**Under `-U` the three built-ins fold case
and a parameter name does not** (`cm macro Pp` called `cm.w Zz`:
`a[Zz] b[pp] c[PP] d[Zz] e[Zz] f[.w]` for `a[Pp] b[pp] c[PP] d[allargs]
e[ALLARGS] f[.attribute]`), so the fold follows the keyword/symbol split rather
than the flag.

Because the boundary rule is uniform and `__LABEL__` obeys it, the capture needs
no special case in the substituter beyond being a fourth candidate.

It also lands on the right side of the OTHER text layer. A `{expr}` name
composition group is resolved after the frame substitution, so a capture pasted
into a name that also carries a group composes with it:

```text
   8/ 1000 : (MACRO)              Qq:	cmp
   8/ 1000 : =$9                  zzz_{n}_Qq = 9
   9/ 1000 : 09                  	dc.b zzz_3_Qq
```

The capture pastes first and the group evaluates into the result, not the other
way round.

## `label`

```text
   4/ 1000 : =$1000               A	label *
   5/ 1000 : =$2000               B	label $2000
   6/ 1000 : =$1004               C:	label *+4
   8/ 1002 : =$1002               D	label *
   9/ 1002 : 1000 2000 1004      	dc.w A,B,C,D
```

Any expression, and the decorative colon is tolerated exactly as on an equate.
Every one of those rows lists with a trailing `C` in asl's symbol table where a
`:=` row lists `-`: the symbol carries the segment. It is therefore neither
`equ` (no segment) nor a plain macro-body label (which does not escape the
expansion at all — `__LABEL___Blocks:` written as a plain label inside a
`{INTLABEL}` body is `error #1010` when read from outside, while the `label`
line beside it is not).

It also OPENS the scope it names, in the CALLER, and **the scope outlives the
expansion that opened it**:

```text
  16/ 1000 : (MACRO)              Tbl	outer 3
  16/ 1000 :  (MACRO-2)           Tbl inner 3
  16/ 1000 : =$1000               Tbl label *
  16/ 1001 : =$7                  .cnt := 7
  18/ 1003 : 07                  	dc.b Tbl.cnt
  19/ 1004 : (MACRO)              .loc	dot
  19/ 1006 : =$1006               .loc label *
```

Two frames deep, and `.cnt` — written in `outer`'s body AFTER the nested call —
still lands in `Tbl`. A colon-less invocation label is captured, and so is a
dotted one, with its dot.

Both halves of that are load-bearing for `zoneOrderedTable`. Inside the
expansion, `label` has to open the scope the CALLER sees, or the six
`.zone_*` accumulators bound on the lines below it land in the expansion's own
unspellable scope. After it returns, the scope has to still be open, because the
sibling `zoneTableEntry` reads those same accumulators from top level on every
later row of the table. The scope cannot come from defining the column-0 label
on the invocation line, because that definition is exactly what the capture
suppresses — so the directive has to open it, and the expansion has to hand it
back out. A capture implemented without moving the scope with it takes the
accumulators out from under the table.

Only a PC-valued `label` is a PLACED label, and only that case is handed to the
builder, so the symbol relocates with its section like any other label. Any
other value binds the name and opens the scope but claims no position, because
it has none to claim. It does not reuse the plain-label path wholesale: that
path qualifies a `.`-local against the EXPANSION and opens the expansion's
scope, which are both the opposite of what a value-binding form does.

## The design, and the one thing it is not

`{INTLABEL}` is not a parameter and `__LABEL__` is not a symbol. The capture is
a fourth substitution candidate alongside `.ATTRIBUTE`, `ALLARGS` and the
parameters, bound implicitly at the call from the label field rather than
positionally from the argument list. Everything else falls out: composition,
passing through a nested call, immunity to `shift` (it is not an argument —
`sm macro {INTLABEL},pp` called `Lb2: sm 5,6` after a `shift` emits
`<6> <Lb2> <>`), and the absence of any substitution where the group was not
declared.

The group never reached the parameter list to begin with: the lexer already
swallows a `{…}` group without emitting a token, so "consumes no argument
position" was true by construction and the test locks that in rather than
implementing it.

## Corpus movement

Same command both times, `sigil s2.asm` from `s2disasm`. The `before` column is
master `6446259c` built in its own worktree with its own target directory and
measured with the same script, not a quoted figure.

| | before | after |
|---|---|---|
| diagnostics | 21,003 | **15,622** |
| distinct classes (file × normalized message) | 163 | 138 |
| distinct unresolved symbols | 293 | 293 |
| distinct source lines carrying a diagnostic | 8,683 | 8,661 |

The fall is 5,381 and it is almost all one class — `label` read as a mnemonic:

| class | before | after |
|---|---|---|
| `` `…` is not a recognized 68000 mnemonic `` (`mappings/MapMacros.asm`) | 4,568 | **0** |
| `` `…` is not a recognized 68000 mnemonic `` (`s2.macros.asm`) | 801 | 273 |
| `` `…` is not a recognized 68000 mnemonic `` (`s2.asm`) | 512 | 255 |
| `operand … out of range` (`s2.asm`; 91 distinct spellings, 24 of them fall to zero) | 91 | 67 |
| `unknown directive or mnemonic `…`` (`s2.sounddriver.asm`) | 14 | 10 |
| **every other class** (135 of them) | — | **unchanged, to the count** |

**No class rose and no new class appeared.** Every one of the 21,003 before-rows
and 15,622 after-rows parses into the table above, so there is no unclassified
remainder hiding a rise. The unresolved-symbol SETS were compared sorted, not
just their sizes: **zero newly unresolved, and zero newly resolved** — the block
never was an unresolved-symbol problem.

The sixth parcel's own sites are the ones a scope change would break first, and
they held: `s2.macros.asm:191`, `:197`, `:208` and `:227` — `zoneTableEntry`'s
`!org` and `shift`, 1,315 diagnostics before that parcel — are still at zero.

Per line, every site of the three spellings went to zero: `MapMacros.asm:13`
and `:21` (1,774 each), `s2.macros.asm:157` — the anchor — 492, `MapMacros.asm:63`
and `:71` (398 each), `MapMacros.asm:3` 224, `s2.asm:88688` 59, `:48390` 39,
`:3895`/`:3900` 33 each, `:83542`/`:83543` 24 each, `s2.macros.asm:168` 23,
`s2.asm:90909` 22, `:24369` 12, `s2.macros.asm:239` 11, `s2.asm:88195` 11,
`:90910` 9, `:86228` 9, `:87676` 6, `s2.sounddriver.asm:285` 4,
`s2.macros.asm:109` 2.

The three largest remaining sites are all in `s2.macrosetup.asm` and none is
this row: `:104` (996, `strlen(): could not evaluate string builtin`), `:59` and
`:62` (749 each, `` `$` with no hex digits ``, on `if ($)&1`). The largest in
`s2.macros.asm` is `:289` (272), which is `irpc`.

`mappings/MapMacros.asm` is a FIFTH file carrying these spellings; the
dispatching brief named four, and that file alone is 85% of the fall.

### The silent half of the block

`s2.constants.asm:373` carried **zero diagnostics before and zero after**, and
it was wrong the whole time:

```
zoneID macro zoneID,{INTLABEL}
__LABEL__ = zoneID
```

Every `emerald_hill_zone zoneID $00` line is a bare column-0 label followed by a
macro, so the label was defined at the CURRENT ADDRESS and the body's
`__LABEL__ = zoneID` bound a literal symbol spelt `__LABEL__`. Reduced to one
file the two definitions collide and it is loud; in the corpus they do not, and
nothing said anything. asl and sigil now agree:

```text
  11/ 1002 : =$0                  emerald_hill_zone = $00
  12/ 1002 : =$2                  wood_zone = $02
  13/ 1002 : 0000 0002           	dc.w emerald_hill_zone,wood_zone
```

### The boundary rule is a defect on its own, and also silent

Seven of the sites the boundary rule moves in the corpus are ORDINARY
parameters, with no `{INTLABEL}` anywhere near them — `palptr macro ptr,lineno`
writing `bytesToLcnt(ptr_End-ptr)` (41 invocations), `TeleportTableEntry macro
addressA,addressB` writing `.sizeA := addressA_End-addressA` (17), and
`zoneAnimals macro first,second` writing `Obj28_Properties_first`. Between them
they exercise BOTH halves of the rule with no capture involved: `ptr_End` needs
the trailing `_` not to block, `Obj28_Properties_first` needs the leading one
not to. Reduced to one file:

```text
  10/ 1004 : (MACRO)              	palptr Pal_SEGA
  10/ 1004 : 0004                        dc.w (Pal_SEGA_End-Pal_SEGA)
```

asl `0004`; sigil before, `unresolved target expression (dangling symbol(s)
`ptr_End`) for fixup`; sigil after, `00 04`. The corpus run never shows it,
because that refusal is the LINKER's and `sigil s2.asm` on this corpus stops at
the front end — which is why `ptr_End` and `addressA_End` appear zero times in
both diagnostic dumps.

Of the 34 sites the rule moves across `s2disasm/**/*.asm`, 27 are the capture's
own composition idiom and 7 are these. None of the 34 is in aeon: a scan of
every macro body in `.aeon-as-fold` finds **zero** sites where the new rule
substitutes and the old did not, so the four-shape byte-identity below is a
regression gate for everything else in this parcel and NOT evidence for the
boundary rule. The corpus is the evidence for that.

A diagnostic count is not a measure of either silent half. It is the reason the
unresolved-symbol set comparison mattered even though it came back empty.

## Verification

* Twelve tests, every expected value a listing row quoted at the test. One
  existing test changed value rather than being added — the `.ATTRIBUTE` string
  row above.
* Thirteen mutations, each shown applied from disk by a `git diff --stat` naming
  the file, each restored from the COMMITTED baseline between runs, each redding
  a named set: defining the captured label anyway; substituting `__LABEL__`
  without the declaration; restoring the underscore-blocks boundary; dropping
  either boundary half; removing either `label` intercept; restoring the entry
  scope on the way out; making the declaration case-sensitive; making the
  built-in names case-sensitive; letting the `{…}` group reach the parameter
  list; dropping the builder label; and taking the parked capture AFTER the
  recursion cap's early return instead of before it — which reds with the
  literal wrong answer, `captured <D>` where asl runs the guard FALSE.
* **The builder-label mutation was applied and stayed GREEN across the whole
  crate**, and
  chasing it found real uncovered ground rather than a false alarm. Neither
  `image` nor `linked_image` can see the difference: the front end folds the
  symbol out of its own env on the converged pass, so a `label` binding only a
  CONSTANT of equal value produces identical bytes — including through a `bra.w`
  whose displacement the linker fills. What separates them is whether the
  SECTION carries the label, which is what the linker's symbol table is built
  from. The gate now asserts that directly, against the plain-label twin so the
  expected offset is derived from the equivalent program rather than copied.
* aeon four shapes rebuilt from `/home/volence/sonic_hacks/.aeon-as-fold`
  (detached at aeon `4f5ad5a1`), all four artifacts DELETED first, one shape per
  invocation, all exit 0 under `SIGIL_VERSION_STRICT=1`, every log stamped
  `Assembler: sigil 788648db5759 (clean at capture)`. CRC32+size unchanged on
  every shape: s4 `14ee2440`/719700, s4.debug `142294b3`/737683, demo
  `0c456778`/96474, demo.debug `2e603d53`/101339. mtimes moved on all four.
  aeon writes none of the three spellings anywhere in the tree.
* Full suite, `scripts/landing-run.sh` against that same reference tree, from
  this worktree at `788648db`: **376 suites, 4302 passed, 0 failed, 2 ignored**,
  `CARGO_EXIT=0`, GREEN. The same wrapper at master `6446259c` in its own
  worktree and its own target directory: 376 suites, **4290 passed**, 0 failed,
  2 ignored, `CARGO_EXIT=0`. The difference is 12, which is the count of
  `#[test]` this branch adds. Each of the thirteen touched test names appears in
  the branch log as `... ok`, and the log carries the run's pwd, HEAD, branch and
  reference tree above cargo's first byte.
* `cargo clippy --release --workspace --all-targets -- -D warnings`, exit 0.

## What stays open

### `irpc`

`s2.macros.asm:289` (272 diagnostics) is now the largest single site in that
file. It is a character-iteration loop (`irpc btn,"buttons"`), unrelated to this
parcel.

### The plain-label export row

The separately-booked row is UNMOVED and this parcel did not trade against it: a
plain `__LABEL___Blocks:` inside a macro body still does not export, which is
asl's behaviour (`error #1010` on the reference from outside), and sigil agrees.
The `label` directive is the spelling that DOES export, which is why the corpus
uses it and not a plain label.

### A non-PC `label` is a constant, not a placed symbol

`B label $2000` binds the value and opens the scope but claims no position,
because it has none to claim. asl types it as an address either way. No corpus
or aeon site writes it, so the divergence is unreachable today; closing it means
deciding what a label at an address the section does not cover even means.
