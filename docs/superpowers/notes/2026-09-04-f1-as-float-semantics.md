# AS floating point: the specification, minted from asl

Parcel **F1**. Every row below is a byte column read out of an `asl` listing, not a
reading of the documentation or of the semantics. Where asl's behaviour is not
observable from either corpus it is said so in place.

## Provenance

| | |
|---|---|
| oracle | `s1disasm/build_tools/Linux-x86_64/asl`, `Macro Assembler 1.42 Beta [Bld 212] (x86_64-unknown-linux)`, md5 `61e672562465725a8c102288a7da9098` |
| flags | `-xx -n -q -A -L -U -i .` — Sonic 1's own, read from `build.lua` via `common.assemble_file` (`-E` dropped so errors reach the console) |
| probes | `.f1probe/f0.asm` … `f9.asm`, committed. `.f1probe/cmp.sh <file.asm>` runs both assemblers on one input and prints both. |
| sigil | branch `parcel/as-float-int`, built into a dedicated on-disk target dir |
| reference ROM | `lua build.lua` in a pristine `s1disasm` worktree at `f6ece657`: 524,288 bytes, md5 `09dadb5071eb35050067a32462e39c5f`, `build_tools/chkbitperfect.lua` says `ROM is bit-perfect with REV01` |
| wall clock | 2026-09-04, 05:29–06:20 local, machine `up 9 days, 21:1x` throughout |

## The one sentence

**AS's expression evaluator is typed — integer XOR float XOR string — and the type
is byte-visible.** Everything else follows from that.

## The table

### `INT()`

| written | asl | listing |
|---|---|---|
| `INT(3.7)` | `3` | `f1.asm(4)` `0000 0003` |
| `INT(3.2)` | `3` | `f1.asm(5)` `0000 0003` |
| `INT(-3.7)` | `-4` | `f1.asm(6)` `FFFF FFFC` |
| `INT(-3.2)` | `-4` | `f1.asm(7)` `FFFF FFFC` |
| `INT(3.0)` | `3` | `f1.asm(8)` `0000 0003` |
| `INT(-3.0)` | `-3` | `f1.asm(9)` `FFFF FFFD` |
| `int(3.7)` | `3` | `f1.asm(10)` — lower case |
| `Int(3.7)`, `iNt(3.7)` | `3` | `f5.asm(3)` — mixed case |
| `INT(7)` | `7` | `f1.asm(11)` — an INTEGER argument passes through |

`INT()` is **floor**, not truncation toward zero: `-3.2` goes to `-4`, and `-3.0`
stays `-3`. Its builtin NAME is matched **case-insensitively**, even under `-U`,
which makes user symbols case-sensitive (`CASESENSITIVE : 1` in every listing's
symbol table). asl reports an unknown builtin uppercased — `error #1860: unknown
function MIN` for a written `min(` — which is the same lookup seen from the
failing side.

### Arithmetic is typed

| written | asl | listing |
|---|---|---|
| `7/2` | `3` | `f1.asm(21)` `0000 0003` |
| `-7/2` | `-3` | `f1.asm(23)` `FFFF FFFD` |
| `INT(-7/2)` | `-3` | `f2.asm(4)` `FFFF FFFD` |
| `INT(1/3*3)` | `0` | `f2.asm(5)` `0000 0000` |
| `INT(3/2*2)` | `2` | `f2.asm(6)` `0000 0002` |
| `INT(7.0/2)` | `3` | `f1.asm(26)` `0000 0003` |

`/` between two INTEGERS is **truncating integer division**, and stays so inside
`INT(...)`. An evaluator that promotes to f64 on sight answers `floor(-3.5)` = -4
for row three and `floor(1.5*2)` = 3 for row five — wrong bytes out of a program
that assembles clean. This is the single most consequential row in the table.

Float arithmetic takes over as soon as either operand is a float.

### The float type is IEEE binary64

| written | asl | listing |
|---|---|---|
| `INT(1e17+1-1e17)` | `0` | `f2.asm(24)` `0000 0000` |
| `INT(123456789012345678.0/1000000000)` | `123456789` | `f2.asm(25)` `075B CD15` |

`1e17+1` is not representable in binary64 and rounds back to `1e17`, so the
difference is 0. An 80-bit x87 extended (64-bit mantissa) represents it exactly
and would answer 1. **This is a decisive discriminator, not an inference from the
values matching.**

### Where a float may and may not go

| written | asl |
|---|---|
| `dc.l 3.7` / `dc.w 3.7` / `dc.b 3.7` | `error #1133: expected integer or string, but got floating point number` (`f1.asm(17-19)`) |
| `dc.l fx` after `fx = 3.7` | `error #1133`, same (`f3.asm(5)`) |
| `dc.w fx*2` after `fx := 3.7` | `error #1133`, same (`f5.asm(5)`) |
| `move.l #3.7,d0` | `error #1133` (`f3.asm(8)`) |
| `dc.l 3.5<4` | `0000 0001` (`f2.asm(16)`) |
| `INT(1.5<2)`, `INT(2.5>2)` | `1`, `1` (`f2.asm(14,15)`) |
| `if 3.5>2` | takes the true arm (`f3.asm(22)` `=>TRUE`) |
| `rept INT(2.9)` | 2 iterations (`f3.asm(25)`) |
| `INT(7.5&3)`, `INT(7.5<<1)`, `INT(7.5!3)` | `error #1134: expected integer, but got floating point number` (`f3.asm(18-20)`) |
| `INT(7.5%2)` | `error #1020: invalid symbol name` — `%` is asl's binary-literal prefix, so this never reaches the type layer (`f3.asm(17)`) |

So a float **leaf** does not make an operand invalid; a float **result** in an
integer context does. Comparisons take floats and yield integers, which is why
the rule sigil implements is "the operand must reduce to an integer" rather than
"no float token may appear" — the blunt reading refuses `dc.l 3.5<4`, which asl
accepts.

### Symbols may hold floats

| written | asl |
|---|---|
| `fx = 3.7` | listing `=3.7`, symbol table `fx : 3.7` (`f2.asm(8)`) |
| `fy equ 2.5` | listing `=2.5`, symbol table `fy : 2.5` (`f2.asm(11)`) |
| `INT(fx)`, `INT(fx+1)`, `INT(fy*2)` | `3`, `4`, `5` (`f2.asm(9,10,12)`) |
| `sc := 1.5` then `sc := 7` then `dc.b sc` | `07` — the integer reassignment WINS (`f5.asm(6-8)`) |

The last row is the one a two-map implementation can get wrong: a stale float
binding left behind by an integer reassignment would win the lookup.

### Float literal forms

| written | asl |
|---|---|
| `1e3`, `1E3` | 1000 (`f3.asm(10,11)`) |
| `1.5e2` | 150 (`f3.asm(12)`) |
| `.5+3` | 3.5 (`f3.asm(13)`) |
| `1.` | 1 (`f3.asm(14)`) |
| `2.5e-1` | **`error #1020: invalid symbol name`** — a SIGNED exponent is refused (`f3.asm(15)`) |

**sigil does not implement any of these forms** — see "Left open" below. The
corpus writes only plain `15.39`-style literals.

### Builtins seen but not implemented

`abs()`, `sgn()`, `sqrt()`, `log()` and the builtin constant `CONSTPI` all
evaluate in asl (`f2.asm(18-21)`: `INT(abs(-3.7))` = 3, `INT(sgn(-3.7))` = -1,
`INT(sqrt(2.0)*1000)` = 1414, `INT(CONSTPI*1000)` = 3141). sigil has none of
them. `abs`/`sgn` belong to cause **F7** and are deliberately left to it; `log`
is a class of its own, and it is the whole of S2's remaining six `int()`
diagnostics at `s2.asm(87677)`, `.loop_counter = int(log(number))`.

### `min` is not a builtin

`min function a,b,b!((a!b)&(-(a<b)))` in `MacroSetup.asm:219` is the corpus's
OWN function; asl answers `error #1860: unknown function MIN` for a `min(` it
did not define (`f0.asm(11)`). It is built from `!`, `&`, `<` and unary `-` —
all integer-only — so `MakePSGFrequency`'s `roundFloatToInteger(...)` must have
collapsed to an integer before `min` ever sees it. That composition is what the
`PSGFrequencies` byte match proves end to end.

## What this buys

`FM_Notes` at `$072790` (192 bytes) and `PSGFrequencies` at `$0729CE` (138
bytes) come out **byte-identical to the retail REV01 cartridge**, with the
instrument proven non-vacuous first (reference vs itself: 0 differing;
reference vs one planted byte: exactly 1, at the planted offset).

With F1 as the ONLY unstubbed cause — the other twelve source-side stubs from
the 2026-09-04 reconnaissance still applied — sigil assembles Sonic 1 to a
524,288-byte ROM at **exit 0 with zero diagnostics**, and the whole-image
divergence map drops from seven regions / 7,285 bytes to **five regions / 7,003
bytes**. Both F1 regions leave the map.

## What it measures, against the post-F4 master (`6ac271da`)

Both corpora, both binaries, same run:

| | master `6ac271da` | branch | delta |
|---|---|---|---|
| Sonic 1 diagnostics | 259 | **93** | −166 |
| Sonic 2 diagnostics | 9,432 | **9,266** | −166 |
| sigil suite | 4,395 pass / 0 fail / 381 suites | **4,409 pass / 0 fail / 382 suites** | +14, +1 file |

**One class moves and no class rises.** `bad word expression` goes 166→0 in
S1 and 215→49 in S2; every other class is unchanged to the count, in both
corpora. The diagnostic SETS were compared in both directions: 0 newly
appearing, 2 disappearing in each corpus — `s1.sounddriver.asm(1796)` and
`(2052)`, `s2.sounddriver.asm(1056)` and `(1388)`, which are the four
`MakeFMFrequency`/`MakePSGFrequency` lines and nothing else.

The **unresolved-symbol NAME sets** were compared in both directions too, since
a count difference passes straight through a name that silently starts
resolving to the wrong thing: S1 is empty on both sides; S2 holds the same 200
names on both sides, with **0 newly unresolved and 0 newly resolved**.

Neither corpus reaches link unstubbed — `assemble_root_located` returns
`Failure`, which carries no partial module — so those symbol sets are the
front end's, not the linker's.

Aeon's four shapes are byte-identical across the change, `SIGIL_BUILD` pointed
at each binary in turn against a detached aeon worktree at `4f5ad5a1`:

| shape | before (CRC32/size) | after |
|---|---|---|
| `s4.bin` | `14ee2440`/719700 | `14ee2440`/719700 |
| `s4.debug.bin` | `142294b3`/737683 | `142294b3`/737683 |
| `demo.bin` | `0c456778`/96474 | `0c456778`/96474 |
| `demo.debug.bin` | `2e603d53`/101339 | `2e603d53`/101339 |

## Left open, deliberately

1. **Float literal exponent and bare-point forms** (`1e3`, `1.5e2`, `.5`, `1.`).
   sigil's lexer answers `malformed number (hex needs a trailing \`h\`)` for
   `1e3`/`1E3` — the Z80 hex heuristic firing on an exponent — and does not lex
   `.5` or `1.` as floats at all, so `INT(.5+3)` comes out as
   `int(): could not evaluate float expression`. No corpus site writes any of
   them, and the fix collides with that heuristic under `cpu z80`, so it is a
   separate parcel rather than a quiet edit here. asl's own `2.5e-1` refusal
   must be preserved by whoever takes it.
2. **`int(...)` in an INSTRUCTION operand.** `move.l #INT(3.7),d0` assembles in
   asl (`f3.asm(7)` `203C 0000 0003`) and does not in sigil: `lower_m68k` runs
   `expand_calls` and stops, exactly the asymmetry this parcel removed among the
   `dc` widths. Wiring it is one line; it was not taken because instruction
   operands are aeon's hottest shipping path and no corpus site needs it. The
   asymmetry is real and should be closed by whoever next touches that path.
3. **A float leaf beside a LINK-DEFERRED name.** A surviving float leaf sends
   the whole operand to the typed evaluator, which resolves symbols out of the
   front-end env only. A label defined ANYWHERE in the AS unit is fine — it
   resolves on a later pass, and `dc.w (1.5<2)+Lbl` matches asl byte for byte
   with the label ahead of the reference (`f11.asm`: `00 03` from both) and
   behind it (`f12.asm`: `00 01` from both). What would be refused rather than
   deferred is a float leaf beside a name only the LINKER can resolve — a
   cross-seam `.emp` symbol. Zero sites in either corpus and none in aeon; noted
   because it is the one place this design gives up a deferral the integer path
   has.

   **I predicted this would fail and it did not.** The prediction went into this
   note before the probe, and the probe refuted it: multi-pass convergence
   already covers every in-unit label.
4. **`abs`/`sgn`/`sqrt`/`log`/`CONSTPI`**, as above.
