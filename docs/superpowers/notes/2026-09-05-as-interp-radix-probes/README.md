# The radix of `\{expr}` interpolation, measured against asl — 2026-09-05

Reference assembler: `s2disasm/build_tools/Linux-x86_64/asl`, **AS 1.42 Beta
[Bld 212]**. `run.sh <stem>` runs one probe THREE TIMES and prints asl's stderr,
its exit code, and the `p2bin` image verbatim; every row below was stable across
all three runs. asl 1.42 is known to answer differently run-to-run for at least
one shape (a `function` call in an immediate whose argument is a register name),
so three runs is the bar for anything a gate is pinned to, not a formality.

## Why every value here is multi-digit

The divergence this directory measures survived for months because every probe
behind `interp_text` used a SINGLE-DIGIT value, where hex and decimal are the
same characters. Two code comments called such a probe *asl-verified*. Every
fixture here uses a value whose hex and decimal spellings differ, and `4660`
(`1234` hex) is multi-digit in both bases so a fixture that merely looked long
enough cannot pass by accident.

## The rule

**A `\{expr}` that folds to an integer renders as BARE UPPERCASE HEXADECIMAL** —
no `$`, no `0x`, no sign, no leading zeros — **and a negative value renders as
its 64-bit two's complement.**

| probe | source | asl says | what other answer it could have given |
|---|---|---|---|
| `r1` | `equ 42` / `255` / `4095` / `10` / `4660` | `2A` `FF` `FFF` `A` `1234` | `42` `255` `4095` `10` `4660` (decimal); `$2A`/`002A` (prefixed/padded) |
| `r2` | `equ -1` / `-42` / `-255`, and `\{0-1}` | `FFFFFFFFFFFFFFFF` `FFFFFFFFFFFFFFD6` `FFFFFFFFFFFFFF01` `FFFFFFFFFFFFFFFF` | `-1`/`-42` (signed decimal); `-2A` (signed hex); `FFFFFFFF` (32-bit); `FF` (byte-wide) |
| `r3` | `twice function x,x*2` → `\{twice(21)}`, `\{add10(245)}`, `\{twice(0-21)}` | `2A` `FF` `FFFFFFFFFFFFFFD6` | `42`/`255`/`-42`, had a function return been a separate value channel |
| `r12` | `equ 0`, and all four of `message`/`warning`/`error`/`fatal` with `equ 42` | `0`; `w=2A` `e=2A` `f=2A` | `0000000000000000` (fixed width); a per-directive radix |
| `r6` | a label 42 bytes in, `*` one byte later, `here+here` | `2A` `2B` `54` | `42`/`43`/`84`, had addresses taken a different path |
| `r8` | `strlen` of a 42-character string | `2A` | `42` |

`r12` verbatim, and it is the cell that proves the four directives share one
renderer:

```
> > > r12.asm(7): warning: w=2A
> > > r12.asm(8): error: e=2A
> > > r12.asm(9): error: f=2A
fatal error, assembly terminated
zero=0
```

## It reaches ROM BYTES

`interp_text` folds where a STRING SYMBOL is BOUND, so the rendering becomes the
symbol's characters.

`r11` — `n := 42`, `s := "\{n}"`, `dc.b s`, `dc.b $ff`:

```
image: 32 41 ff          (the characters "2A")
```

and `strlen(s)` is `2`. The same probe binds `sneg := "\{neg}"` with `neg := -1`
and reports `strlen(sneg)` = `10` hex — **sixteen** characters, which corroborates
the 64-bit width by counting rather than by re-reading the same digits.

`r13` re-verifies, with a distinguishing value, the two comments that were stale.
`n := 42` / `s := "\{n}"` / `n := 255` / `dc.b s`, and the `equ` spelling beside
it:

```
image: 32 41 ff 32 41 fe
```

So the fold happens at BIND time, in hex, in both the `set`/`:=` branch and the
`equ` branch. The earlier claim ("`n := 3` binds `"3"`") was true but could not
have been false: `3` is `3` in both radices, and a read-time fold would have
produced the same byte too, since the probe never reassigned `n`.

## The one place the hex rule does NOT apply

**`{expr}` symbol-name composition renders in DECIMAL**, in the same source file,
in the same assembler.

- `r9`: `n := 42` then `name_{n} equ $55`. Reading `name_2A` is
  `error: symbol undefined`, exit 2. Reading `name_42` gives `55`, exit 0.
- `r10`: `n := 42` then `name_{"\{n}"} equ $55` defines **`name_2A`** — a `\{}`
  inside a literal inside the group is the STRING construct nested, so it takes
  the hex rule.

`s2disasm` depends on both halves: `zone_id_{cur_zone_str}` composes off
`cur_zone_str := "\{cur_zone_id}"` (string arm, hex) and
`zoneanimcount_{"\{zoneanimcur}"}` off a string-valued symbol. Defining and
reading sides agree only because both go through the same arm.

## Cells measured and NOT adopted — sigil still diverges here

These are real, three-run-stable asl behaviours that this parcel did not
implement. They are recorded so the next reader does not have to re-measure.

Sigil's column was MEASURED, run against the branch build, not inferred from the
code.

| probe | source | asl says | sigil today |
|---|---|---|---|
| `r5` | `f equ 1.5` → `\{f}`; `g equ 42.0` → `\{g}` | `1.5`, `42` — floats render in **DECIMAL** | does not fold; the text `\{f}` is left verbatim |
| — | `\{n/1.0}` with `n equ 42` | `42` | does not fold; `\{n/1.0}` verbatim |
| `r4`/`r7` | `c equ 'z'` → `\{c}`; `\{c+0}`; `\{'z'+1}` | `z`, `z`, `{` — a character value stays a CHARACTER through arithmetic | `7A`, `7A`, `7B` — the integer, now in hex |
| `r4` | `s equ "abc"` → `\{s}` | `abc` | does not fold; `\{s}` verbatim |
| `r7` | `\{substr(e,1,2)}` with `e equ "abc"` | `bc` | does not fold, AND raises a spurious `trailing tokens in expression` error |

`fold_text` resolves through `eval_all`, which answers only for integers, so
every non-integer cell falls into the leave-it-verbatim arm. That arm predates
this parcel and is unchanged by it.

The float row is why the corpora write `\{tracenum/1.0}` and
`\{.cur_zone_id/1.0}`: dividing by `1.0` is the idiom for "print this in
decimal". Every such site in `s2disasm` and `s1disasm` is inside a `message`, and
sigil renders all of them verbatim today — so **this parcel neither fixed nor
regressed the decimal idiom**, which is worth saying out loud because "sigil used
to render decimal" makes it sound as though those sites used to come out right.
They never folded at all.

## Cells NOT reached

- The radix under `cpu z80` — every probe here is `cpu 68000`. asl's renderer is
  not obviously target-dependent, but that is a reading, not a measurement.
- A value wider than 64 bits, and the boundary at `$8000000000000000` where the
  two's-complement reading and a hypothetical "print the magnitude" reading part
  company for the last time.
- `\{}` inside a macro parameter substitution, and `\{}` inside an `irp` body.
- The radix of a value carried through `MOMPASS`-dependent forward references,
  i.e. whether pass 1's unresolved rendering differs from pass 2's.
