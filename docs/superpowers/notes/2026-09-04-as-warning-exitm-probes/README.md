# `warning` and `exitm` measured against asl — 2026-09-04

Reference assembler: `s1disasm/build_tools/Linux-x86_64/asl`, md5
`61e672562465725a8c102288a7da9098`, invoked `asl -U -q <probe>.asm` then
`p2bin <probe>.p <probe>.bin -k` with `AS_MSGPATH` pointed at the same
directory. Every `image:` line below is the `p2bin` output read back with
`od -An -tx1`; every diagnostic is asl's own stderr, verbatim.

**The digest, not the banner, is the identity — and the rows below were taken
with a DIFFERENT build that prints the same one.** `run.sh` and `img.sh` named
`s2disasm/build_tools/Linux-x86_64/asl` (md5 `0dee1f98e6480a4783d27ffd8b90896f`)
when this table was written; both print **AS 1.42 Beta [Bld 212]** verbatim, so
nothing in the output said which had run. The runners now select by digest
(`../asl-reference/`) and refuse anything else.

The rows are NOT re-taken, and they do not need to be: all 33 probes in this
directory were run under both builds and the emitted code lines are identical
under every one of them. Two probes in the whole sweep of four repinned corpora
differ, and both are in `2026-09-04-as-end-probes`, not here.

## `warning`

| probe | source shape | asl exit | asl says | image |
|---|---|---|---|---|
| `w1` | `warning "hello from warning"` between two `dc.b` | **0** | `> > > w1.asm(5): warning: hello from warning` | `11 22` |
| `w2` | `warning "val is \{val} here"`, `val equ 42` | 0 | `warning: val is 2A here` | `11` |
| `w3` | warning + `exitm` inside a macro, called three times | 0 | `> > > w3.asm(12) mw(2): warning: zero seen` | `01 02` |
| `w4` | a warning above a forward reference (two passes) | 0 | the warning line printed **TWICE**, the summary still counts `1 warning` | `00 00 00 04 11` |
| `w5` | a warning inside a FALSE `if` arm, plus a live one | 0 | only the live one fires | `11` |
| `w6` | `warning bareword` | **2** | `warning: 0` then `symbol undefined` then `error: invalid string` | — |
| `w7` | `warning` with no operand | **2** | `error: wrong number of operands` | — |
| `w8` | `warning "a","b"` | **2** | `error: wrong number of operands` | — |

So: exactly one string operand; `\{expr}` interpolates; the message is arbitrary
author text; a warning does **not** fail the assembly and does not move a byte.

## `exitm`

`exitm` ends the **innermost running expansion** and nothing outside it. What
counts as an expansion, measured:

| probe | shape | image | reading |
|---|---|---|---|
| `e1` | `exitm` under a live `if` in a macro, called with the guard false then true | `11 A0 B0 22 A0 33` | ends the macro |
| `e2` | macro calls macro; the CALLEE exits | `B0 A0 B1 FF` | only the callee — the caller's next line still lands |
| `e3` | `rept 3` inside a macro | `A0 C0 A1 FF` | ends the REPT; the macro continues |
| `e4` | inside a FALSE `if` arm | `A0 A1 FF` | not reached, nothing happens |
| `e5` | at top level, no macro, no loop | *no image*, exit **2** | `error: EXITM not called from within macro` — and asl KEEPS GOING (`dc.b $22` after it still listed) |
| `e6` | a bare TOP-LEVEL `rept 3`, no macro anywhere | `11 C0 22` | **accepted** — a `rept` is enough; the message's word "macro" is not the condition |
| `e7` | `while` inside a macro | `A0 C0 A1 FF` | ends the WHILE; the macro continues |
| `e13` | `rept 2` containing `rept 2` inside a macro | `A0 D0 C0 D1 D0 C0 D1 A1 FF` | pops exactly ONE level — the outer rept still runs both iterations |
| `e14` | `exitm` in an INCLUDED file, the `include` inside a macro | *no image*, exit **2** | `e14inc.asm(2): error: EXITM not called from within macro` — an `include` is not an expansion and HIDES the ones around it; neither the include nor the macro stops |
| `e15` | macro called from inside a `rept` in another macro | `A0 C0 B0 C1 C0 B0 C1 C0 B0 C1 A1 FF` | the callee truncates, the rept runs all three |
| `e16` | inside a `case` arm of a `switch` in a macro | `A0 A0 C2 A1 FF` | `switch` is NOT a frame — it ends the whole macro |
| `e17` | macro exits, called from a TOP-LEVEL `rept 2` | `11 C0 A0 C1 C0 A0 C1 22` | one level again |
| `e18` | a label on the `endif` closing an exitm'd arm, referenced later | *no image*, exit 2 | `symbol undefined` — asl never READ the closer, so its label binds nothing |
| `e19` | the same without the closer label; a label AFTER the loop | `C0 FF 00 00 00 01` | everything after the loop is normal |
| `e10` | `exitm 1,2,junk` | *no image*, exit 2 | `error: wrong number of operands`, and the exitm does **not** take effect (`A1` still listed) |

### Cells NOT reached

- **`exitm` inside `irp` / `irpc` — asl SEGFAULTS.** Probe `e8` (an `irp` inside a
  macro) and probe `e11` (a bare top-level `irp`) both die with
  `Segmentation fault (core dumped)`, exit **139**, no listing. There is no
  reference answer for this cell at any nesting. Sigil treats `irp`/`irpc` as the
  frame `rept`/`while` are, which is the only reading consistent with its
  siblings, but that is a **choice with no oracle behind it**, not a measured
  behaviour.
- A label on the `exitm` LINE itself (`e9`) is inconclusive: the control (`e12`,
  an ordinary `dc.b` line carrying a label in the same macro body) is *also*
  `symbol undefined`, so `e9` measures asl's macro-body label scoping, not
  anything about `exitm`.
- `exitm` under `cpu z80`, inside a `function` body, and interacting with `end`
  in the same expansion: not probed.

## Out-of-parcel finding: `\{expr}` renders in HEX, sigil renders DECIMAL

`x1`/`x2` are not about either directive. asl folds `\{expr}` to the value in
**hexadecimal** — probe `x1` with `v equ 42` prints `2A`, in a `message` and a
`warning` alike. Sigil's `interp_text` renders decimal and its doc says so.

This reaches BYTES, not just message text: `x2` binds `s := "\{n}"` with `n := 42`
and emits `dc.b s`.

```
asl:    32 41 ff      ("2A")
sigil:  34 32 FF      ("42")
```

Every existing probe behind that code path used a single-digit value, where hex
and decimal agree — which is how it survived. NOT fixed here: `interp_text` also
feeds `str_env`, so changing it is byte-changing and needs its own aeon byte
re-proof.

**FIXED 2026-09-05** by `parcel/as-interp-radix`. Sigil renders hex now, negatives
as 64-bit two's complement, and the four author diagnostics moved together. The
full radix measurement — including the cell this note did not reach, that `{expr}`
SYMBOL-NAME composition renders in DECIMAL in the same assembler — is in
`../2026-09-05-as-interp-radix-probes/`.

## Corpus effect of implementing them: ZERO, and the reason is worth writing down

Both disassembly corpora walked with a MASTER-built `sigil` (`c97385f0`, exported with
`git archive` into a scratch tree and built into its own target dir) and a BRANCH-built
one, and the diagnostic SETS compared in both directions:

| corpus | HEAD | diagnostics OLD | NEW | newly resolved | newly unresolved |
|---|---|---|---|---|---|
| `s2disasm` (`s2.asm`) | `e45ebf3` | 6035 | 6035 | **0** | **0** |
| `s1disasm` (`sonic.asm`) | `f6ece65` | 57 | 57 | **0** | **0** |

Zero is a convenient answer, so here is how it was produced rather than the number alone.
The OLD walks contain **no `warning` or `exitm` refusal at all** — not one line of either
corpus's eleven sites was ever EXECUTED. They are all inside macro bodies or `if` arms
that a correct build does not take: `s2.macros.asm`'s `clearRAM` writes them under
`elseif startaddr==endaddr`, and every one of `s2.asm`'s ~30 `clearRAM` calls passes a
non-empty range.

The walk demonstrably REACHES those calls — `s2.asm` alone accounts for 5180 of the 6035
diagnostics, spread past line 4267 — so this is "the guard is false", not "the walk
stopped short of them".

**Do not read the 6035/57 totals as complete-walk figures.** They name only five and six
distinct files respectively, well short of the committed baselines
(`2026-09-03-s1-corpus-baseline.md`: 9,739 over 444 files, at an older sigil). What the
table above supports is the DELTA — identical binaries-apart runs, identical sets — and
nothing about absolute corpus coverage.
