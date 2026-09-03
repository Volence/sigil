# `shift` walks a macro's arguments, so a body substitutes where it is read

2026-09-03 · branch `parcel/as-macro-shift` · sigil master base `1a921c2f`

Fifth in the AS-frontend arc for the public Sonic 2 disassembly
(`/home/volence/sonic_hacks/s2disasm`, git `e45ebf3`). The fourth parcel
scope-blocked `shift` with a sizing; this one implements it.

Ground truth throughout is `asl -L` (AS V1.42 Beta Bld 212,
`s2disasm/build_tools/Linux-x86_64/asl`). Every rule below is stated with the
listing row that establishes it, and every expected value in the eight new
tests is such a row rather than a value this implementation produced.

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
   > > > p5.asm(24) PW(5):14: error: invalid symbol name
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
   67/ 16 : 0A                  dc.b strlen("aaaa,bbbbb")
   67/ 17 : (MACRO-2)            ein qq,rrr
   67/ 17 : 03                  dc.b strlen("RRR")
   67/ 18 : 0A                  dc.b strlen("aaaa,bbbbb")
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
   77/ 19 : (MACRO-2)   zfin zzzzz
   77/ 19 : 08          dc.b strlen("BBB,CCCC")
```

`08` is the OUTER post-shift `ALLARGS` (`bbb,cccc`); the inner call's `zzzzz`
would be `05`.

A `rept`/`while` body is the same shape: substituted ONCE where the loop is
entered, then replayed. A `shift` inside it advances the frame — visible after
the loop — without rewriting the body's own text:

```
   55/ 12 : 0E     dc.b strlen("aaa,bbbb,ccccc")     ; before the rept
   55/ 13 : 0E     dc.b strlen("aaa,bbbb,ccccc")     ; iteration 1, after a shift
   55/ 14 : 0E     dc.b strlen("aaa,bbbb,ccccc")     ; iteration 2, after another
   55/ 15 : 05     dc.b strlen("CCCCC")              ; after the loop: two shifts landed
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
call with no arguments — is genuinely empty and none of this applies
(`e[AA][][][aa]` emits exactly its 13 characters; `if ""<>""` is `=>FALSE`).

sigil treats an emptied slot as empty everywhere. The corpus only ever tests
the never-bound case: each recursion is a FRESH call whose first parameter is
unbound once `ALLARGS` runs dry, which is what terminates
`zoneTableEntry`/`creditsPtrs`. Reproducing the placeholder would mean carrying
an AS storage artifact into emitted bytes for no reachable gain.

Two related divergences, same reasoning: asl upper-cases the argument values it
rebuilds `ALLARGS` from after a shift (`aa,bb,cc` → `BB,CC`), an artifact of
running case-insensitive; sigil preserves the text as written. And `ALLARGS`
before any shift is rendered from the whole argument token run rather than
re-joined from the groups — the two agree on every shape probed, and the
whole-run rendering is what aeon's byte-exact `%<…>` debugger strings already
depend on.

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

### Keyword arguments plus `shift`

`ALLARGS` after a shift renders the supplied parameter slots in PARAMETER
order, so `kw k2=aa,k1=bb` on params `k1,k2` shifts to `aa` — the value bound
to `k2`, not the second-written group. That is implemented and matches asl.
What is NOT settled is the shape where asl itself errors (`positional argument
no longer allowed after keyword argument`, probe 4c): asl drops the offending
argument and then renders one fewer group than the slot vector holds. No corpus
or aeon macro mixes keyword arguments with `shift`; the behaviour under an
already-diagnosed call is left unspecified rather than guessed.

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
* Each of the eight tests proven red by a mutation shown applied from disk
  (`git diff` quoting the changed line) and restored from the committed
  baseline between runs. Seven mutations, each redding a named set: suppressing
  `MacroFrame::shift` reds seven; freezing post-shift `ALLARGS` at the entry
  text reds five; re-windowing instead of empty-filling reds exactly the
  empty-fill test; disabling the loop-body capture reds exactly the `rept`
  test; reading the outermost frame instead of the innermost reds the two
  nesting tests; storing raw text in `capture_macro` reds exactly the
  nested-definition test; dropping the not-in-a-macro refusal reds exactly the
  outside-a-macro test.
