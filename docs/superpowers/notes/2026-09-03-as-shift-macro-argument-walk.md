# `shift` walks a macro's arguments, so a body substitutes where it is read

2026-09-03 · branch `parcel/as-macro-shift` · sigil master base `1a921c2f`

Fifth in the AS-frontend arc for the public Sonic 2 disassembly
(`/home/volence/sonic_hacks/s2disasm`, git `e45ebf3`). The fourth parcel
scope-blocked `shift` with a sizing; this one implements it.

Ground truth throughout is an `asl -L` listing (AS V1.42 Beta Bld 212,
`s2disasm/build_tools/Linux-x86_64/asl`). Every rule below is stated with the
row that establishes it, and every expected value in the twelve tests is such a
row rather than a value this implementation produced.

**The invocation carries `-U`**, which forces case-sensitivity — the namespace
this front-end implements, the flag the Sonic 2 build passes, and the flag every
`asl` vector generator in this repo passes. Without it asl folds every
identifier and the rows come back upper-cased, describing a different
assembler. See *Argument case* below; a few rows in the first two sections are
from the earlier `-U`-less runs and are marked where the difference shows.

## What `shift` does

AS keeps **two** vectors per expansion, and `shift` advances both. Probe,
`zt 1,2,3,4` on params `pp,qq,rr`, emitting `<pp|qq|rr|ALLARGS>` between shifts:

```
   19/  3 : (MACRO)   zt 1,2,3,4
   19/  3 :           dc.b "<1|2|3|1,2,3,4>"
   19/  C :           shift
   19/  C :           dc.b "<2|3||2,3,4>"
   19/ 14 :           shift
   19/ 14 :           dc.b "<3|||3,4>"
   19/ 2D :           shift
   19/ 2D :           dc.b "<|||4>"
   19/ 39 :           shift
   19/ 39 :           dc.b "<|||>"
   19/ 44 :           shift
   19/ 44 :           dc.b "<|||>"
```

| after | pp | qq | rr | ALLARGS |
|---|---|---|---|---|
| entry | 1 | 2 | 3 | `1,2,3,4` |
| shift | 2 | 3 | *(empty)* | `2,3,4` |
| shift | 3 | *(empty)* | *(empty)* | `3,4` |
| shift | *(empty)* | *(empty)* | *(empty)* | `4` |
| shift | *(empty)* | *(empty)* | *(empty)* | *(empty)* |
| shift | — no-op — | | | |

The parameter vector is **not a window sliding along the argument list**. It has
one slot per DECLARED parameter and empty-fills behind the shift, so the fourth
argument never reaches the third parameter even while `ALLARGS` still carries
it. The sharpest form, free of any listing-rendering question — two parameters,
four arguments, emitting `q1` after each shift:

```
   24/  3 : 05                  dc.b 5
   24/  4 : 06                  dc.b 6
   > > > p5.asm(24) pw(5):14: error: invalid symbol name
   24/  5 :                     dc.b
```

`q1` stops at the second argument and then holds nothing; the third argument is
reachable only through `ALLARGS`. A shift past exhaustion is a no-op.

### `shift` is an ordinary executable statement

Inside `if 0` it does not run (`<7|7,8,9>` — unshifted); inside `if 1` it does
(`<|8,9>`). Outside any macro asl refuses it, through the not-in-a-macro check
it shares with `EXITM`: `p4.asm(6): error: EXITM not called from within macro`.

### Shift state belongs to ONE expansion

An inner macro's shift consumes the inner call's arguments and leaves the
caller untouched:

```
   43/  A : 0A                  dc.b strlen("aaaa,bbbbb")
   43/  B : (MACRO-2)            ein qq,rrr
   43/  B : 03                  dc.b strlen("rrr")
   43/  C : 0A                  dc.b strlen("aaaa,bbbbb")
```

## Why the body cannot be substituted up front

All three corpus uses read `ALLARGS` on a line AFTER the `shift`, inside the
same `if` arm (`s2.macros.asm:197` `zoneTableEntry`, `s2.macrosetup.asm:320`
`jmpTosInternal`, `s2.asm:14427` `creditsPtrs`). Pasting the binding into the
whole body before executing it therefore cannot express `shift` at all.

An expansion now pushes a `MacroFrame` and executes the RAW body.
`.ATTRIBUTE`, `ALLARGS` and the parameters are substituted where a line's text
is **consumed**: `exec_one`, `dispatch_head` (hence every block scan and every
`if`/`switch`/`rept`/`while` head), `def_function`, `capture_macro`,
`capture_struct`, `parse_struct_field`. Only the innermost frame is ever read —
an outer expansion's parameters are already baked into anything an inner one
can see.

### The capture constructs go the OTHER way, and asl says so

The fourth parcel's sizing listed `rept`, `while`, `capture_macro`,
`capture_struct` and `def_function` among the sites that must become lazy, and
named `capture_macro` the semantically sharp one because it "captures text with
the outer expansion's parameters already baked in". **That baking is correct**,
and lazy capture would have been the defect. A macro defined inside an
expanding body freezes the outer `ALLARGS` at the shift state in force at
capture, and its own arguments do not rebind it:

```
   44/  D : (MACRO-2)   zfin zzzzz
   44/  D : 08          dc.b strlen("bbb,cccc")
```

`08` is the OUTER post-shift `ALLARGS` (`bbb,cccc`); the inner call's `zzzzz`
would be `05`.

A `rept`/`while` body is the same shape: substituted ONCE where the loop is
entered, then replayed. A `shift` inside it advances the frame — visible after
the loop — without rewriting the body's own text:

```
   42/  6 : 0E     dc.b strlen("aaa,bbbb,ccccc")     ; before the rept
   42/  7 : 0E     dc.b strlen("aaa,bbbb,ccccc")     ; iteration 1, after a shift
   42/  8 : 0E     dc.b strlen("aaa,bbbb,ccccc")     ; iteration 2, after another
   42/  9 : 05     dc.b strlen("ccccc")              ; after the loop: two shifts landed
```

`MacroFrame::suspend` implements exactly this: the loop body is materialized
against the frame, the frame's substitution is suspended for the replay, and a
nested invocation inside the body pushes its own unsuspended frame.

So the change is **six substitution sites plus a loop-capture rule**, not the
ten lazy sites the sizing projected, and the two constructs the sizing worried
most about needed no change in behaviour at all.

## Deliberately NOT replicated: AS's empty-parameter placeholder

AS stores a macro body with `\001\00N` placeholders for parameters. A slot
emptied BY A SHIFT keeps those two bytes, and they leak wherever the parameter
is stringified:

* `dc.b strlen("p3")` on a shift-emptied `p3` yields **2**, not 0.
* `if "p3"<>""` on a shift-emptied `p3` is **TRUE** (`=>TRUE  if ""<>""` — the
  listing renders the slot empty while the comparison sees the placeholder).
* `dc.b "…p3…"` emits the two bytes into the data.

A parameter that was never bound at all — more parameters than arguments, or a
call with no arguments — is genuinely empty and none of this applies. Both
states in one expansion, `ph aa,bb` on params `p1,p2,p3,p4`, after one shift:

```text
   34/ 104E : 00                          dc.b    strlen("")
   34/ 104F : =>FALSE                      if ""<>""
   34/ 104F : 655B 6262 5D5B              dc.b    "e[bb][][][]"
      1055 : 5D5B 5D5B 0105 5D
```

`p3` was never bound, so its `strlen` is 0 and the guard is FALSE. The only
placeholder in the emitted bytes is the `0105` for `p4` — the slot the SHIFT
vacated.

sigil treats an emptied slot as empty everywhere. The corpus only ever tests
the never-bound case: each recursion is a FRESH call whose first parameter is
unbound once `ALLARGS` runs dry, which is what terminates
`zoneTableEntry`/`creditsPtrs`. Reproducing the placeholder would mean carrying
an AS storage artifact into emitted bytes for no reachable gain.

This is the ONE divergence in the argument walk. The two others recorded here
earlier are not divergences.

## Argument case: there is nothing to diverge from

`-U` sets asl's `CASESENSITIVE` to 1 — it appears in every listing's symbol
table, so a listing states which assembler produced it. The Sonic 2 build
passes it (`common.lua`: `asl -xx -n -q -A -L -U -E -i .`, commented "forces
case-sensitivity"), and so does every `asl` vector generator in this repo.

Under `-U` asl applies NO case transformation to a macro argument at any point.
Same source, the two invocations side by side — params `pp,qq`, called
`pcase aa,bb,cc`, emitting `ALLARGS` and both parameters at entry and after
each shift:

```text
                    ; asl (no -U)                 ; asl -U
   13/ 1000 : E[aa,bb,cc][AA][BB]          E[aa,bb,cc][aa][bb]
   13/ 1013 : S[BB,CC][BB][]               S[bb,cc][bb][]
   13/ 1023 : T[CC][][]                    T[cc][][]
```

The fold is not in the `ALLARGS` rebuild. It is asl's global identifier fold,
applied to every argument VALUE when the parameter is bound — visible at ENTRY,
in `[AA][BB]`, before any shift has run. `ALLARGS` at entry is the invocation's
own text and so is the one place the unfolded spelling shows through, which is
what makes the post-shift rebuild *look* like the transform's origin.

So a post-shift `ALLARGS` cannot reach a context where case changes the result,
in either direction that would matter:

* **A composed symbol name.** asl `-U` composes the name from the caller's
  spelling and resolves that symbol; so does sigil. `Mix_Ss equ $77` with
  `pick zz,Ss` on param `qq` shifting once gives `dc.w Mix_{"Ss"}` → `0077` in
  both, and the folded call `pick zz,SS` gives asl `error #1010: symbol
  undefined` on `Mix_{"SS"}` and sigil `unresolved symbol Mix_SS`. Neither
  assembler can silently pick the other symbol, because neither one folds.
* **Text emitted into data.** `dc.b "E<ALLARGS>"`/`"S<ALLARGS>"` on
  `ws aa, bb , cc` emits `E<aa,bb,cc>S<bb,cc>` under asl `-U`, byte for byte
  what sigil emits.

The property that makes this so is a property of this implementation, not of
the corpus: **the AS front-end case-folds no identifier and no argument text.**
There is no `to_uppercase`/`to_ascii_uppercase` anywhere in it, and its three
lowercasing sites are all value-level (`lowstring()`, directive/mnemonic
normalization, an attribute-suffix split) — none touches a symbol name. Two
tests pin it against `-U` rows:
`shift_carries_argument_case_into_emitted_bytes` and
`a_composed_name_from_a_post_shift_allargs_keeps_the_arguments_case`, the
second carrying the negative half so a fold cannot pass by resolving both
spellings. Making `all_args()` upper-case its post-shift join — exactly the
behaviour this section used to attribute to asl — reds both.

## `ALLARGS` before a shift is the invocation's text, and a keyword call proves it

The whole-run rendering and a re-join of the argument groups agree on every
positional shape, whitespace included: asl `-U` normalizes the separators, so
the written `ws aa, bb , cc` renders `E<aa,bb,cc>`, as does sigil. A keyword
call splits them, and asl sides with the written text:

```text
   11/ 1000 : (MACRO)              	kw	k2=aa,k1=bb
   11/ 1000 : 453C 6B32 3D61              dc.b    "E<k2=aa,k1=bb>"
   11/ 100E : 533C 6161 3E                dc.b    "S<aa>"
```

Entry `ALLARGS` keeps the keyword syntax and the written order; the post-shift
render is the supplied slots in PARAMETER order, so `S<aa>` is the value bound
to `k2`. A re-join could not produce the first line. Pinned by
`a_keyword_calls_allargs_is_written_text_before_a_shift_and_parameter_order_after`.

## One substitution pass, because AS's is not a pass at all

AS resolves a body's parameter references to `\001\00N` placeholders when the
macro is CAPTURED. Text pasted in at expansion time therefore holds no
placeholders and cannot acquire any: what a binding pastes in is inert. Any
implementation that substitutes by successive whole-text replaces loses that,
because a value pasted for one name is still in the buffer when the next name
is scanned for. asl `-U`, `mm macro pp,qq`:

```text
   11/ 1000 : (MACRO)              	mm	qq,zz
   11/ 1000 : 453C 7171 2C7A              dc.b    "E<qq,zz>"
   12/ 100D : (MACRO)              	mm	xx,pp,yy
   12/ 100D : 453C 7878 2C70              dc.b    "E<xx,pp,yy>"
```

The `qq` the caller wrote stays `qq` even though `qq` is the callee's second
parameter. `subst_frame_text` walks the line ONCE, matching `.ATTRIBUTE`,
`ALLARGS` and the parameters in AS's precedence and never rescanning what it
appends. Pinned by `pasted_argument_text_is_not_rescanned_for_parameter_names`.

Neither live consumer exercised it, which is why the change is byte-neutral: no
invocation in the Sonic 2 disassembly passes an argument whose text spells one
of the callee's own parameter names (the eleven that come close all pass an
OUTER parameter of the same name, already substituted to its value by the time
the inner call is made), and aeon has one `ALLARGS` site — `ifdebug`, which
declares no parameters.

## Where post-shift argument text is CONSUMED

The corpus has three `shift` macros and four `shift` statements (`creditsPtrs`
shifts twice), and that is the complete list for every `.asm` under
`s2disasm`. What each does with the text after the shift:

| site | post-shift text reaches |
|---|---|
| `s2.macros.asm:197` `zoneTableEntry` | a recursive call's argument, then `dc.ATTRIBUTE value` — a symbol REFERENCE |
| `s2.asm:14427-14428` `creditsPtrs` | a recursive call's argument, then `dc.l addr` and `dc.w vram_pnt + pos` |
| `s2.macrosetup.asm:320` `jmpTosInternal` | `jmpTosInternal2 ALLARGS` → `irp op,ALLARGS` → `op label *`, a symbol DEFINITION, and `extractJmpToName("op")`, a string |

`jmpTosInternal` is the sharpest consumer — post-shift text becomes both a
defined symbol name and string-function input — and it is `ARGCOUNT`/`irp`
blocked below rather than case-blocked.

aeon consumes none of it. Its three residual `.asm` files hold one `ALLARGS`
(`engine/debug/debugger.asm:146`, the parameterless `ifdebug` pass-through) and
no `shift` at all; the byte-exact `%<…>` assert strings interpolate the
`src`/`cond`/`dest` PARAMETERS, not `ALLARGS`. `shift`/`ALLARGS` are
AS-front-end constructs with no `.emp` spelling, so nothing reaches them from
that side either.

## BLOCKED

### `ARGCOUNT` (`s2.macrosetup.asm:301`)

Unimplemented in sigil, and its interaction with `shift` is not a rule I can
state. asl drops it from 3 to **0** across one shift of a one-parameter macro
called with three arguments (`e[3]` → `s[0]`), which fits neither vector's
length. The corpus never combines them — `jmpTosInternal2` takes no parameters,
so its `ARGCOUNT` is just its own argument count — so nothing here forces the
question. Implementing `ARGCOUNT` belongs with `irp`/`irpc` (the same macro,
`s2.macros.asm:289`, 272 diagnostics), and that parcel owes its own probe of
the shift interaction before it writes the rule down. Sizing: one parcel with
`irp`/`irpc`, aeon-byte-neutral (aeon uses neither).

### A positional argument after a keyword argument

The well-formed keyword shapes are settled above and pinned. The one that stays
open is the shape asl REFUSES. asl `-U`, `kw k1=aa,bb` on params `k1,k2`:

```text
  > > > k1.asm(12): error #1812: positional argument no longer allowed after keyword argument
   12/ 1013 : 453C 6B31 3D61              dc.b    "E<k1=aa,bb>"
   12/ 101E : 533C 3E                     dc.b    "S<>"
```

asl DROPS the offending argument — one group survives, and one shift empties it
(`S<>`). sigil keeps it as a surplus positional and renders `S<bb>`.

The gap underneath is that sigil raises no diagnostic here at all: the
rendering difference is downstream of a call asl declines to assemble, so
matching it means implementing the refusal, not the rendering. Until then the
post-refusal render is unspecified rather than guessed. No corpus or aeon macro
mixes keyword arguments with `shift`. Sizing: the refusal is a check in the
argument binder, one parcel with whatever else lands in `bind`.

## Corpus movement

Same command both times, `sigil s2.asm` from `s2disasm`. The `before` column is
master `1a921c2f` rebuilt from a clean archive and measured with the same
script, not a quoted figure.

| | before | after |
|---|---|---|
| diagnostics | 121,505 | **22,328** |
| distinct unresolved symbols in `s2.asm` | 207 | 207 |
| distinct unresolved symbols, all files | 298 | 298 |
| distinct source lines carrying a diagnostic | 8,701 | 8,696 |

The fall is 99,177, and it decomposes exactly:

| class | before | after |
|---|---|---|
| `` `shift` is not a recognized 68000 mnemonic `` (inside the mnemonic class) | 35,167 | 0 |
| `org needs a constant expression` | 32,408 | 669 |
| `` `{…}` in a symbol name did not resolve `` | 32,385 | 646 |
| `macro … expansion too deep` | 532 | 0 |
| **every other class** | — | **unchanged, to the count** |

**No new error class appeared, and no individual class rose.** The unresolved-
symbol SETS are identical — not merely the same size: zero newly unresolved,
zero newly resolved. The runaway was purely multiplicative: it re-emitted
existing errors ~48× and hid nothing underneath. A count fall of this size
would normally be the moment to look for what it uncovered; here the honest
answer is that it uncovered nothing, and the value of the fix is that the
remaining 22,328 are now one-per-site.

`creditsPtrs` (`s2.asm:14422`) went from **2,838 diagnostics to zero** — the
construct assembles. `zoneTableEntry` still fails, at its true rate: 612 `org`
+ 612 brace diagnostics at `s2.macros.asm:191` (and 34 + 34 at `:208` for
`zoneTableBinEntry`), all of them the ONE remaining cause the fourth parcel
blocked — `.cur_zone_str` is `set` in `zoneOrderedTable`'s expansion and read
in `zoneTableEntry`'s, and sigil scopes `.`-locals per expansion where AS
scopes a `set` one to the caller. `s2.macrosetup.asm` is unchanged at 2,731:
its `shift` now succeeds, and everything still failing there is `ARGCOUNT`,
`irp` and the Z80 `$` program counter.

## Verification

* aeon four shapes rebuilt from `/home/volence/sonic_hacks/.aeon-as-fold`
  (detached at aeon `4f5ad5a1`), all four artifacts DELETED first and all four
  builds exit 0 under `SIGIL_VERSION_STRICT=1` (so a stale assembler would have
  been fatal, not a warning — the binary reports its revision as this branch's
  HEAD). CRC32+size unchanged on every shape: s4 `14ee2440`/719700, s4.debug
  `142294b3`/737683, demo `0c456778`/96474, demo.debug `2e603d53`/101339.
  `demo` is positional (`./build.sh demo`); `s4` additionally needs
  `SIGIL_EMIT`. mtimes moved on all four.
* Each of the first eight tests proven red by a mutation shown applied from disk
  (`git diff` quoting the changed line) and restored from the committed
  baseline between runs. Seven mutations, each redding a named set: suppressing
  `MacroFrame::shift` reds seven; freezing post-shift `ALLARGS` at the entry
  text reds five; re-windowing instead of empty-filling reds exactly the
  empty-fill test; disabling the loop-body capture reds exactly the `rept`
  test; reading the outermost frame instead of the innermost reds the two
  nesting tests; storing raw text in `capture_macro` reds exactly the
  nested-definition test; dropping the not-in-a-macro refusal reds exactly the
  outside-a-macro test.
* The four case/rendering tests proven red the same way. Making `all_args()`
  upper-case its post-shift join reds all four — the case pair, the
  no-rescan test and the keyword test. Restoring the sequential-replace
  substitution reds the no-rescan test and the keyword test. Rendering entry
  `ALLARGS` from a re-join instead of the invocation text reds exactly the
  keyword test.
* Byte neutrality of the one-pass substitution, both live consumers: aeon four
  shapes with all four artifacts deleted first, each shape one invocation under
  `SIGIL_VERSION_STRICT=1`, all exit 0, every build log stamped
  `Assembler: sigil 620f8b7dcfca (clean at capture)` — CRC32+size unchanged on
  all four. And `sigil s2.asm` over the corpus: 22,328 diagnostics before and
  after, with the SETS identical (`diff` of the sorted outputs is empty), the
  baseline measured with a binary built from master `d37c1738` in its own
  worktree.

## Found while settling this, not acted on

Two things the probes turned up that belong to other parcels:

* **A plain label in a macro body does not export.** asl `-U`, a macro whose
  body carries `lbl_static:` — the label appears in the expansion listing and
  then `dc.w lbl_static` outside the macro is `error #1010: symbol undefined`,
  and `lbl_static` is absent from the symbol table. Only the `label` directive
  form exports (`lbl_viadir label *` → `lbl_viadir : 2 C` in the table), which
  is why `jmpTosInternal2` uses `op label *` for symbols the whole program
  references. sigil resolves the plain form from outside. Unrelated to `shift`,
  reachable through any macro, and changing it moves whatever depends on it —
  it needs its own parcel and its own aeon gate. sigil also has no `label`
  directive (`` `label` is not a recognized 68000 mnemonic ``), so the shape
  the corpus actually relies on is the one still missing.
* **Surplus positional arguments skip `bind_macro_arg`.** `all.extend(pos_iter)`
  appends the arguments the parameter list could not hold as raw text, so they
  miss the caller-scope qualification every bound argument gets. A bare
  `.`-local passed beyond the last parameter and then read through `ALLARGS`
  would name nothing. No corpus or aeon call does that; noted because the
  asymmetry is invisible at the call site.
