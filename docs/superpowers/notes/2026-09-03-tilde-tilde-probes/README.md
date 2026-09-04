# `~~` probes

`~~` is asl's LOGICAL not, one greedy token. sigil lexed it as two `~` and
folded `!!x`, which cancels — the operand came back unchanged, silently.

Every probe here is committed so the next reader MEASURES rather than inherits.

| probe | what it establishes |
|---|---|
| `p1.asm` | the operator: `~~`, `~`, `~~~`, `~~~~`, `~ ~`, `~-1`, precedence, `\|\|`/`&&`, comparisons |
| `p2.asm` | the type questions: do `\|\|`/`&&`/`if` take plain integers, does `~~` take a float, does the result behave as a boolean |
| `p3.asm` | the whole grid as one byte image, plus both corpus `if` shapes — the differential |
| `p4.asm` | `_btst`/`_beq`/`_bne` with the six operand shapes the corpus writes, plus one it never does |
| `p4a.asm` | the six real shapes ALONE — byte-identical even with the DEFECT, which is the point |
| `p5.asm` / `p5b.asm` | `jmpTos`/`jmpTos0`; `p5b` adds content after the last `align` to sidestep an unrelated trailing-align gap |
| `p6.asm` | that trailing-align gap in isolation: `rts` + `align 4`, differs with BOTH binaries, nothing to do with `~~` |
| `p7.asm` | `~~` inside a macro BODY |
| `p8.asm` | `~~` inside a macro ARGUMENT, including a `"[v]"` string embed — the only shape that reaches `punct_str` |

## Running them

```sh
./run.sh <path-to-asl> p1.asm out          # listing + log, for reading byte columns
ASL=… P2BIN=… SIG=… ./diff_bytes.sh p3.asm # asl vs sigil, CRC32/size
./mutate.sh                                 # the five red-first proofs
```

Run every probe through BOTH shipped `asl` builds. They agree here on every byte
column and every error line, which is what licenses using either one — but that
agreement is a measurement, not an assumption, and it is cheap to re-take.

`PREDICTIONS.md` is the Sonic 1 / aeon / Sonic 2 predictions as written BEFORE
any run, kept so the record shows which held and which did not.
