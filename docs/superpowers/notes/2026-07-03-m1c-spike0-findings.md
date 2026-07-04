# Sigil M1.C — Task 0 Spike Findings (2026-07-03)

Reference toolchain source: `/home/volence/sonic_hacks/aeon` (ROM: `s4.bin`, 458666 bytes).
Goldens extracted to: `crates/sigil-frontend-as/tests/vectors/sine_goldens/`.

## 1. Sine goldens

| File | A (amplitude) | P (period) | offset | size | byte[0] |
|---|---|---|---|---|---|
| `ojz_calm_a96_p64.bin`   | 96 | 64 | 0x11402 | 256 | 0x00 |
| `rocking_a20_p64.bin`    | 20 | 64 | 0x118D2 | 256 | 0x00 |
| `haze_a16_p64.bin`       | 16 | 64 | 0x1169A | 256 | 0x00 |
| `shimmer_a8_p32.bin`     |  8 | 32 | 0x11528 | 256 | 0x00 |

All four extracted files are exactly 256 bytes; index 0 of every table is `0x00`. Confirmed.

## 2. `int()` truncation mode — CONFIRMED: `floor`, not `trunc`

The plan's hypothesis was `trunc` (round toward zero). That hypothesis is **wrong**. The mode that
matches all 4 golden tables byte-for-byte is **`math.floor`** (round toward negative infinity).
`trunc`, `round_half`, and `round_even` all fail on every table.

```
ojz_calm_a96_p64.bin     trunc      differ
ojz_calm_a96_p64.bin     floor      MATCH
ojz_calm_a96_p64.bin     round_half differ
ojz_calm_a96_p64.bin     round_even differ
rocking_a20_p64.bin      trunc      differ
rocking_a20_p64.bin      floor      MATCH
rocking_a20_p64.bin      round_half differ
rocking_a20_p64.bin      round_even differ
haze_a16_p64.bin         trunc      differ
haze_a16_p64.bin         floor      MATCH
haze_a16_p64.bin         round_half differ
haze_a16_p64.bin         round_even differ
shimmer_a8_p32.bin       trunc      differ
shimmer_a8_p32.bin       floor      MATCH
shimmer_a8_p32.bin       round_half differ
shimmer_a8_p32.bin       round_even differ
```

Divergence is confined to negative arguments (as expected: floor and trunc agree for x >= 0, and
differ by exactly 1 whenever x has a nonzero fractional part and x < 0). Example
(`ojz_calm_a96_p64.bin`, index 33): `x = -9.4096...`, `trunc = -9`, `floor = -10`, gold byte = `-10`
(0xF6). 123 of 256 indices in this table diverge between `trunc` and `floor`; `floor` matches the
ROM at every one of them.

**T8 implication**: a single, simple mode (`floor`) reproduces the reference ROM exactly across all
4 sampled amplitude/period combinations using plain `f64` + libm `sin`. Bit-match with `asl` looks
feasible via `x.floor() as i64` (or equivalent) in Rust — **no evidence of an FP/libm mismatch**, so
the D2 source-cure fallback is not indicated by this spike. T8 should implement `int(x) = floor(x)`
(not `trunc`) and validate against these 4 goldens plus additional amplitude/period combos if time
allows.

## 3. AS surface enumeration

Enumerated across `/home/volence/sonic_hacks/aeon/{games,engine}` (`*.asm`, `*.inc`). Raw grep
counts (verbatim, occurrence-based via `-o`) are given first per the plan's exact commands, followed
by a manual pass that strips comments/string-literal false positives — the second pass is what
actually matters for scoping, since a large fraction of the raw hits turned out to be English prose
in comments or literal text inside `error`/`fatal` message strings, not real operator/directive use.

### Operators

Raw (`-o`, plan's exact command):

```
     70 <>
     63 mod
     33 !=
     20 ||
      5 &&
```

After stripping comment-only and string-literal-only lines (real, assembler-evaluated usage only):

| Operator | Real usages | Files | AS semantics |
|---|---|---|---|
| `<>`  | 70 | many (`main.asm`, `sonic.asm`, sfx patch files, `debugger.asm`, ...) | not-equal, used in `if (...)` compile-time asserts and string-token comparisons (e.g. `if "opts"<>""`) |
| `mod` | **0** | — | *(all 57 raw hits are English prose in comments — "mod 8", "tempo mod", "mod-0" — the `mod` operator is not actually used anywhere in this corpus)* |
| `!=`  | **0** | — | *(all 6 non-comment hits are literal text inside `error "... != ..."` / `fatal "..."` message strings being string-interpolated, not the assembler comparison operator)* |
| `\|\|` | 5 | `engine/debug/debugger.asm` (2), `games/sonic4/data/sound/dac_samples.asm` (3) | logical OR, yields 0/-1, short-circuits `if (...)` guards |
| `&&`  | 4 | `engine/debug/debugger.asm` (all 4) | logical AND, yields 0/-1 |
| `~` (bitwise-not) | 0 | — | *(all 76 raw `~` hits are the prose glyph "~5" / "~90°" meaning "approximately", in comments — no genuine unary bitwise-not use)* |
| `~~` (boolean-not) | 0 | — | not used anywhere |
| `!` (bitwise-or) | **3** | `engine/debug/debugger.asm` lines 247, 262, 532 | genuine bitwise-OR operator, pattern `((EXPR&1)!1)*offset` (align-padding trick); distinct from the `!name` escape prefix and from `!=` |

**T2 scope confirmation**: implement `<>` (heavily used, real), `\|\|`/`&&` (real but low-volume,
concentrated in `debugger.asm` + one sound-data file), and `!` bitwise-or (real, 3 sites, all in
`debugger.asm`). **De-prioritize `mod` and `!=`** — neither has a single genuine call site in the
current corpus; supporting them is "complete AS compat" nice-to-have, not corpus-driven necessity.
`~` and `~~` have zero genuine usage — do not spend budget chasing them beyond basic support.

### Builtins

Raw (`-o`, plan's exact command, paren-anchored):

```
     32 substr
     18 strlen
     15 sin
     11 strstr
      5 lowstring
      3 cos
      1 int
```

After stripping comment-only false positives (sin/cos have many comment mentions describing runtime
math, not assembler-time calls) and adding builtins the plan's grep list omitted (`val`, plus
negative checks for `sqrt`/`upstring`/`charfromstr`/`abs`/`sgn`/`atan`/`tan`/`exp`/`log`/`strchr`,
all **0** genuine hits):

| Builtin | Real usages | Files |
|---|---|---|
| `substr`    | 32 | `engine/debug/debugger.asm` (100%) |
| `strlen`    | 18 | `engine/debug/debugger.asm` (100%) |
| `strstr`    | 11 | `engine/debug/debugger.asm` (100%) |
| `lowstring` |  5 | `engine/debug/debugger.asm` (100%), always as `switch lowstring(...)` |
| `val`       |  5 | `engine/debug/debugger.asm` (100%) — **not in the plan's grep list; found by broadening the search** |
| `sin`       |  1 | `engine/parallax_macros.inc:223` only — every other "sin(" hit is a comment |
| `int`       |  1 | `engine/parallax_macros.inc:223`, same line as the `sin` call above — this one line is the generator for all 4 sine goldens |
| `cos`       |  **0** | *(all 3 raw hits are comments describing the runtime `GetSineCosine` routine, e.g. "cos(angle) = SineTable[angle+$40]" — no assembler-time `cos()` call anywhere)* |
| `sqrt`, `upstring`, `charfromstr`, `abs`, `sgn`, `atan`, `tan`, `exp`, `log`, `strchr` | 0 | — |

Related directive: `switch` — 20 raw hits, but only **5** are the real `switch`/`case` directive
(all in `engine/debug/debugger.asm`, always paired with `lowstring(...)`); the rest are English
prose ("bank switch", "switch to DRAINING_TAIL", etc.).

**T3 scope confirmation — this is the headline finding of the spike**: the entire string-builtin
surface (`substr`, `strlen`, `strstr`, `lowstring`, `val`, `switch`) is used **exclusively** inside
one file, `engine/debug/debugger.asm` (the assert/debug-console macro library) — none of it appears
in game/engine code proper. The only arithmetic builtins with a genuine call site anywhere are
`sin` and `int`, and both only fire on the single line `engine/parallax_macros.inc:223`
(`dc.b int(AMPLITUDE * sin(6.283185307179586 * deform_sine_i / PERIOD))`) that generates the sine
goldens this spike extracted. `cos` has zero real uses. This means T3 can be split cleanly:
a small, high-value arithmetic slice (`sin`, `int`, plus float arithmetic in expressions) is needed
for parallax/mainline builds to assemble at all; the much larger string-builtin slice is only
needed if/when `engine/debug/debugger.asm` itself is brought under sigil (a debug-only, low-priority
target) — it can be deferred without blocking non-debug builds.

### Data / reserve / align directives

Raw (`-o`, plan's exact command):

```
   1515 dc.b
    623 dc.l
    347 dc.w
    108 align
     65 even
     19 ds.b
      9 ds.w
      4 org
      4 ds.l
```

After stripping comment-only false positives:

| Directive | Real usages | Notes |
|---|---|---|
| `dc.b` | 1487 | genuine data |
| `dc.l` | 610  | genuine data |
| `dc.w` | 287  | genuine data (raw count included `w` sub-match noise; re-verified against `dc.w` directly) |
| `align`| 87   | almost all `align 2`; a few `align $8000` (bank boundaries in `main.asm`, `dac_samples.asm`) |
| `ds.b` | 17   | genuine reserve |
| `ds.w` | 9    | genuine reserve |
| `org`  | 4    | genuine |
| `ds.l` | 4    | genuine reserve |
| `even` | **0** | *every one of the 62 raw hits is the English word "even" in a comment ("odd or even", "a0 ends even") — the `even` directive itself is never actually invoked anywhere in this corpus* |

**T6 scope confirmation**: `dc.b`/`dc.w`/`dc.l`/`ds.b`/`ds.w`/`ds.l`/`org`/`align` all need real,
corpus-validated support. `align` matters (bank-boundary alignment is load-bearing for the sound
bank layout). `even` has **zero validation corpus** in this codebase — support it for AS-syntax
completeness, but there is no real usage to write a golden/regression test against.

### `!name` escapes / `.ATTRIBUTE` / `ALLARGS` / `MOMCPUNAME`

All `!name` escape sites are confined to `engine/debug/debugger.asm`, and only two escape names are
used:

```
     10 !align
      4 !error
```

(e.g. `!error "Unknown condition cond"`, `!align 2`). No other `!name` forms appear anywhere.

`.ATTRIBUTE` (AS's "current instruction size suffix" macro-parameter substitution) is used
extensively — ~30 sites — but **all confined to `engine/debug/debugger.asm`** (patterns like
`move.ATTRIBUTE d0,DEST`, `cmp.ATTRIBUTE src,dest`, `_Console.ATTRIBUTE`).

`ALLARGS` and `MOMCPUNAME` are used together as a dual-target (Z80 vs 68k) blob-emission idiom:
`if MOMCPUNAME="Z80" / db ALLARGS / else / dc.b ALLARGS`. Real usage sites:

- `ALLARGS`: 9 files — `engine/debug/debugger.asm` plus 8 `*_patches.asm` files
  (`games/sonic4/data/sound/movingtrucks_patches.asm`, `sfx_33_patches.asm`, `sfx_34_patches.asm`,
  `sfx_35_patches.asm`, `sfx_3C_patches.asm`, `sfx_AB_patches.asm`, `sfx_B6_patches.asm`,
  `sfx_B9_patches.asm`).
- `MOMCPUNAME`: the same 8 `*_patches.asm` files (not `debugger.asm`).

**T7/T9 scope confirmation**: `!align`/`!error` and `.ATTRIBUTE` are debug-console-only (T7 can
scope to just those two escape names, and note `.ATTRIBUTE`'s entire footprint is one file).
`ALLARGS`/`MOMCPUNAME` are a real, separate, non-debug idiom used by 8 SFX-patch data files for
Z80/68k dual-target blobs — T9 must support this pattern since it's load-bearing for the sound
patch files, independent of whether `debugger.asm` itself is ever ported.

## 4. Per-task scope confirmation (T2–T9)

- **T2 (operators)**: implement `<>` (real, 70 sites, broad), `||`/`&&` (real, low-volume,
  concentrated in `debugger.asm` + `dac_samples.asm`), `!` bitwise-or (real, 3 sites, all in
  `debugger.asm`). De-prioritize `mod` and `!=` — zero genuine call sites in this corpus (all hits
  are comment prose or literal text in error-message strings). `~`/`~~` — zero genuine usage,
  minimal effort only.
- **T3 (builtins)**: `sin`+`int` are the load-bearing pair (1 call site, generates the sine
  goldens) — needed for non-debug builds. `substr`/`strlen`/`strstr`/`lowstring`/`val`/`switch` are
  100% confined to `engine/debug/debugger.asm` — defer unless/until that file is in scope. `cos`,
  `sqrt`, `upstring`, `charfromstr`, and everything else checked have zero genuine usage.
- **T4–T5** (not separately re-enumerated by this spike; no new findings beyond what's implied by
  the operator/builtin split above).
- **T6 (data/reserve/align)**: `dc.b/.w/.l`, `ds.b/.w/.l`, `org`, `align` are all real and
  corpus-validated; `align $8000` (bank boundaries) is a real, load-bearing case beyond plain
  `align 2`. `even` has zero real usage anywhere in the corpus — implement for spec completeness
  only, no golden available.
- **T7 (`!name` escapes)**: exactly two names in use, `!align` (10) and `!error` (4), both confined
  to `engine/debug/debugger.asm`.
- **T8 (`int()` truncation)**: confirmed mode is `floor` (round toward -inf), **not** `trunc`. Plain
  `f64`/libm `sin` + `floor` bit-matches the reference ROM on all 4 sampled tables — no evidence the
  D2 source-cure fallback is needed.
- **T9 (`.ATTRIBUTE`/`ALLARGS`/`MOMCPUNAME`)**: `.ATTRIBUTE` confined to `debugger.asm` (~30 sites).
  `ALLARGS`/`MOMCPUNAME` are a real, non-debug idiom spanning 8 `*_patches.asm` sound files — must
  be supported independent of `debugger.asm` scope.

## 5. Methodology note

The plan's Step 3 grep commands are a good first pass but overcount real usage because AS source in
this codebase is comment-heavy and several keywords (`mod`, `even`, `switch`, `sin`, `cos`, `~`,
`!=`-as-substring-of-error-text) double as ordinary English words or appear inside string literals.
Every count in this note that says "real"/"genuine" was obtained by stripping the trailing `;`
comment (and, for `!=`, recognizing that all hits were inside quoted `error`/`fatal` strings) before
re-matching the pattern, then spot-checking samples. Raw counts are kept in this note for
traceability; treat the "real usages" columns as authoritative for scoping T2/T3/T6/T7/T9.
