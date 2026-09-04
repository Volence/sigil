# asl probes: two places we accepted silently what AS refuses

Everything below is read off `asl` 1.42 Beta Bld 212 (`s1disasm/build_tools/Linux-x86_64/asl`)
run with **Sonic 1's own flags**, taken from `s1disasm/build_tools/lua/common.lua:773`
rather than guessed:

```
asl -xx -n -q -A -L -U -E -i . [-c] <args> <root>.asm
```

The probes drop `-E` (which redirects the error listing to a file) and `-c` so the
diagnostics land on stdout; nothing else differs. Probe sources are `k*.asm`,
`a*.asm`, `b*.asm`, `c*.asm`, `d*.asm`.

**No expectation here is a reading of the semantics.** Each is a listing row.

---

## 1. Macro keyword / positional arguments (asl #1811, #1812, #320)

### How asl decides an argument is a KEYWORD argument

On **raw text**, before the argument means anything: the **first `=` outside
parentheses and outside a string literal** splits it, and whatever stands to its
left is the keyword's NAME. Probes `k4.asm`–`k7.asm`, macro `m macro px,py,pz`
over `dc.b px,py,pz`:

| written | asl's reading | listing row |
|---|---|---|
| `zz=2` | keyword named `zz` (#1811) | `dc.b 1,,` |
| `2=3` | keyword named `2` (#1811) | `dc.b 1,,` |
| `2+3=5` | keyword named `2+3` (#1811) | `dc.b 1,,` |
| `2==3` | keyword named `2` (#1811) — `==` is not special | `dc.b 1,,` |
| `=5` | keyword with an EMPTY name (#1811) | `dc.b 1,,` |
| `px =9` | keyword `px`, name trimmed (#320) | `dc.b 9,,` |
| `PX=1` | keyword `PX` (#1811) — case-sensitive under `-U` | `dc.b ,,` |
| `2<=3` | keyword named `2<` (#1811) | `dc.b 1,,` |
| `2>=3` | keyword named `2>` (#1811) | `dc.b 1,,` |
| `py=` | keyword `py` bound to EMPTY TEXT — no #1811 | `dc.b 1,,` |
| `(2=3)` | ordinary expression | `0100 04` |
| `((2=3))` | ordinary expression | `0100 04` |
| `[2=3]` | ordinary expression | `0104` |
| `"py=2"` | ordinary expression | `0170 793D 3203` |
| `'py=2'` | ordinary expression | `0170 793D 3204` |
| `2<>3` | ordinary expression — carries no `=` | `0101 04` |
| `<py=2>` | keyword named `<py` (#1811) — angle brackets do NOT protect | `dc.b 1,,` |

Parentheses and square brackets protect; angle brackets do not; both quote kinds
protect. There is no operator awareness whatsoever — `<=`, `>=` and `==` all
split, and `<>` survives only because it contains no `=`.

### The three diagnostics

**#1812 — `positional argument no longer allowed after keyword argument`.**
Fires **once per positional argument that follows the first keyword argument**,
whether or not that keyword was itself valid. Probe `k2.asm`/`k4.asm`:

```
> > > k2.asm(8): error #1812: positional argument no longer allowed after keyword argument
> > >  m py=$22,1,3
> > > k2.asm(8): error #1812: positional argument no longer allowed after keyword argument
> > >  m py=$22,1,3
   8/       6 : (MACRO)              	m	py=$22,1,3
   8/       6 :                             dc.b    ,$22,
> > > k2.asm(9): error #1812: positional argument no longer allowed after keyword argument
> > >  m 1,py=$22,3
   9/       6 : (MACRO)              	m	1,py=$22,3
   9/       6 :                             dc.b    1,$22,
```

Two on line 8 (`1` and `3` both follow the keyword), one on line 9 (only `3`
does — the leading `1` precedes it and is legal). `m PX=1,2,3` reports two.

**#1811 — `keyword argument not defined in macro`.** The name is not among the
declared parameters. Points at the argument's own column, quotes the name, and
**still arms #1812** for everything behind it:

```
> > > k3.asm(7):6: error #1811: keyword argument not defined in macro
> > > zz
> > >  m 1,zz=2,3
> > >      ~~~~
> > > k3.asm(7): error #1812: positional argument no longer allowed after keyword argument
   7/       0 : (MACRO)              	m	1,zz=2,3
   7/       0 :                             dc.b    1,,
```

**#320 (a WARNING) — `macro argument redefined`.** A keyword rebinding a slot a
positional already filled, or a duplicated keyword. The **keyword wins**, and
positionals bind **by position index**, not by "next unfilled slot":

```
> > > k3.asm(8): warning #320: macro argument redefined
   8/       0 : (MACRO)              	m	1,2,px=9
   8/       0 :                             dc.b    9,2,
   9/       0 : (MACRO)              	m	px=1,px=2,pz=3
   9/       0 :                             dc.b    2,,3
```

`m 1,2,px=9` gives `px=9` (keyword), `py=2` (the SECOND positional), `pz` empty
— the first positional's slot was taken, and the `2` did not slide into `py`'s
place by being next in line, it landed there by being second.

### RECOVERY — the part that makes this a wrong program, not a missing message

A refused argument — #1811's keyword, or #1812's positional — **binds NOTHING**.
The parameter keeps the empty text an omitted argument would have given it.
`dc.b` then reports `#2050: empty argument` and emits no bytes.

Assembly **continues** past all of these; the PC does not advance for the
refused line; and asl produces **no `.p` file at all** when any error occurred
(verified: `a1.p`, `a3.p`, `k2.p`, `k3.p` all absent), exit code 2.

### What this front end did before 2026-09-04

Accepted every row above without a word. For `m 1,py=$22,3` it bound the refused
`3` to `pz` and emitted `01 22 03`, exit 0 — asl emits nothing and refuses.

### What it does now, and the two gaps left open

Closed: #1811 and #1812, with asl's bind-nothing recovery, asl's once-per-
offending-argument count, asl's empty-value-is-still-a-keyword rule, and asl's
paren/string protection.

Open, and unreachable in the corpora rather than merely rare:

- **`2<=3` / `2>=3`.** asl splits these at the `=` into keyword names `2<` and
  `2>`. This lexer folds both characters into one `Le`/`Ge` token by maximal
  munch, so there is no `=` left to find and the argument stays positional.
  A census across s1disasm, s2disasm and all four aeon shapes found **zero**
  bare depth-0 `<=`/`>=` in any macro argument.
- **#320.** A warning; this front end has no warning channel, so a keyword
  rebinding a positional-filled slot is silent. It also binds differently (see
  the position-index rule above) — but every call that reaches that difference
  is one asl warns about, and the corpora contain none.

---

## 1a. THE SHARPEST ONE, and it is in neither filed row: both assemblers succeed and emit DIFFERENT BYTES

Found while minting the #320 rows above. Probe `w3.asm`, `m macro px,py` over
`dc.b px,py`, called `m 1,2,px=9`:

```
> > > w3.asm(6): warning #320: macro argument redefined
> > > px
> > >  m 1,2,px=9
      6/       0 : (MACRO)              	m	1,2,px=9
      6/       0 : 0902                        dc.b    9,2
```

- **asl: exit 0**, a WARNING only, `.p` produced, ROM bytes `09 02`.
- **this front end: exit 0**, no diagnostic of any kind, bytes `09 01`.

Every other divergence in this note has asl REFUSING, so a mistake is at least
stopped by the reference tool. This one asl accepts and ships. Two assemblers,
one source, two ROMs, and nothing anywhere says so.

The cause is the positional binding rule, not the diagnostic:

- **asl binds a positional argument by its POSITION INDEX.** Positional #1 goes
  to `px` and #2 to `py`, whether or not a keyword already claimed `px`. The
  keyword wins the slot and the positional written for it is simply lost.
- **this front end binds positionals into the next slot no keyword has taken.**
  `1` is pushed past the keyword-claimed `px` and lands in `py`, and `2` lands
  in whatever follows.

Confirmed on a second shape (probe `k3.asm` line 8, `m macro px,py,pz` called
`m 1,2,px=9`): asl lists `dc.b 9,2,` — `py` takes the SECOND positional and `pz`
is empty — where this front end produces `09 01 02`.

NOT CHANGED IN THIS PARCEL, deliberately. It is a byte divergence rather than a
silent acceptance of a refusal, and the binding rule it lives in is shared with
`ALLARGS`, `filled` and `shift`, each carrying its own asl-verified probe rows
that this parcel took no measurement of. Its population is zero — the census
found no macro argument carrying a depth-0 `=` anywhere in s1disasm, s2disasm or
the four aeon shapes, so no call reaches it — but it wants its own parcel with
its own `ALLARGS`/`shift` probes, not a same-day patch to reach a green count.

---

## 2. An assignment whose right-hand side names something undefined (asl #1010)

**The row this was filed under said we "bind zero and say nothing". We do not
bind zero.** What actually happens is below.

### asl's answer

`equ`, `=`, `set` and `:=` behave identically — error **#1010 `symbol undefined`**
pointing at the undefined name's column, the symbol listed as `=???`, and
assembly continues (probes `a1.asm`, `d2.asm`):

```
> > > d2.asm(3):5: error #1010: symbol undefined
> > > Undefined1
> > > X = Undefined1+1
> > >     ~~~~~~~~~~
      3/       0 : =???                 X	=	Undefined1+1
      4/       0 : =???                 Y	set	Undefined2
      5/       0 : =???                 Z	:=	Undefined3
      6/       0 : 4444                	dc.w	$4444
```

The name is left genuinely undefined, so a later **use** errors again — and the
using line **emits no bytes and does not advance the PC** (probe `a1.asm`: line
5's `dc.w Bad` and line 6's `dc.w Known` are both at PC 0):

```
> > > a1.asm(5):7: error #1010: symbol undefined
      5/       0 :                     	dc.w	Bad
      6/       0 : 0010                	dc.w	Known
      7/       2 : 4444                	dc.w	$4444
```

A **forward** reference is not this: `Fwd: equ Later+1` with `Later:` defined
below assembles clean, `=$21` (probe `a2.asm`).

### This front end's actual behaviour

`directive_equate`'s `eval_all`-failed branch emits the equate as a **deferred
symbolic `EquSym`** whenever the right-hand side parses and carries a symbol.
That is a deliberate, load-bearing mechanism for the mixed `.emp`/AS build
(`Game_Entry = GameState_OJZScroll_Init`, the `ErrorHandler`/`MDDBG__*` chain),
not an oversight.

Downstream of it there are **two link paths, and only one of them refuses**:

- `resolve_layout` → `link` — the FULL pipeline, which aeon's build uses.
  `sigil-link/src/relax.rs::fold_equ_syms` **already errors loudly** on an equ
  that never resolves, referenced or not.
- `link()` called directly — what `crates/sigil-cli/src/main.rs:107` does for a
  single `.asm` file, i.e. the `sigil <file>` / `--hex` / `-o` path the corpus
  measurements run through. Its Pass 1b takes `Fold::Poison => continue` and
  leaves the symbol undefined, **by an explicit and correct decision for a
  PARTIAL link**. An unreferenced dangling equate is silent there.

So `sigil a3.asm -o a3.bin` emits `44 44`, exit 0, where asl reports #1010.

### Why this was NOT changed here

The front end cannot tell a typo from a cross-seam `.emp` label; that ambiguity
is exactly why the deferral exists, and `Options` carries no "nothing will be
joined to this unit" flag to resolve it. Two candidate closes, neither taken in
this parcel:

1. Add such an `Options` flag and refuse in `directive_equate` when it is set.
2. Route the single-file CLI seam through `resolve_layout`, so the loud
   `fold_equ_syms` refusal that already exists actually runs on that path.
   (2) is plumbing rather than semantics, but it puts relaxation into a path
   that today has none, so it needs its own byte gate.

Neither would move a corpus number today: s1disasm and s2disasm both fail in the
FRONT END (65 and 6,035 diagnostics) and never reach any link at all.

### A sharper member of the same class, found while probing (asl #1820)

An `if` whose condition cannot be evaluated in the first pass is **refused** by
asl and **silently decided** here — and it is not equ-specific (probe `d1.asm`):

```
> > > d1.asm(3): error #1820: expression must be evaluatable in first pass
> > >  if Undefined1=0
      3/       0 : =>TRUE               	if Undefined1=0
      4/       0 : 1111                	dc.w	$1111
      5/       2 : =>FALSE              	else
```

asl refuses and then takes TRUE, emitting `11 11`. This front end takes the
ELSE branch and emits `22 22`, exit 0, no diagnostic. **Different code from the
same source, in silence.**

It cannot simply be closed, because asl's rule is FIRST-PASS evaluability, not
resolvability: a **forward-defined** name is refused too (probe `d3.asm`,
`if Later=0` with `Later: equ 0` below it → #1820), while this front end
resolves it on a later pass and gets the right answer. A faithful #1820 would
refuse sources that assemble correctly today. That is a design call about the
pass model, not a parcel fix.
