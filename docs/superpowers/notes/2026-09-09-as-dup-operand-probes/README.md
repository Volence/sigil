# AS `[count]value` duplicate operands, and the `listing`/`page` controls

The reference probes behind `crates/sigil-frontend-as/tests/as_dup_operand_and_listing.rs`.

Assembler: `asl` 1.42 Beta [Bld 212], `x86_64-unknown-linux`, md5
`61e672562465725a8c102288a7da9098`, the build Sonic 1's own `build.lua` runs.
Selected by digest through `../asl-reference/asl_ref.sh`; the banner and the
path identify nothing, because several `asl` binaries here print the same banner
and are not the same program.

Run one with `./run.sh dN.asm`. It sources the guard, uses `asl_run` (which
refuses a non-zero exit out loud and returns that status), prints
`asl_diag_state`'s completeness verdict, and then the listing. **A run carrying
any error is not a source of values for the lines that did assemble**, so a
listing printed under a non-zero status is there to read the DIAGNOSTIC from,
never the byte column.

One construct per file, deliberately: an error anywhere in a file stops asl's
pass loop, which both corrupts unresolved forward references elsewhere in the
listing and suppresses the diagnostics a later pass would have raised.

## What each probe established

| probe | shape | asl exit | result |
|---|---|---|---|
| `d1` | `dc.b [3]$FF`, `dc.w [2]$1234`, `dc.l [2]$AABBCCDD` | 0 | `FF FF FF` / `1234 1234` / the long twice; `$` advances by the whole run (labels land at +3, +8, +16) |
| `d2` | `dc.b $01,[3]$FF,$02` and friends | 0 | `01 FF FF FF 02`: the group is ONE operand's prefix |
| `d3` | `[1+2]`, `[(2*2)]`, `[n]`, `[n-1]` with `n equ 4` | 0 | any constant expression, including an earlier symbol |
| `d4` | `[ 3 ]$AA`, `[3] $BB`, `[ 2 ] $CC` | 0 | whitespace inside and after the group is immaterial |
| `d5` | `dc.b [0]$FF` | 0 | emits nothing, `$` does not move |
| `d6` | count defined BELOW its use | 2 | `error #1820: expression must be evaluatable in first pass` |
| `d7` | `dc.b [2]"ab"` | 0 | `61 62 61 62`: the string repeats whole |
| `d8` | `dcb.b 3,$FF` | 2 | `error #1200: unknown instruction DCB`. **This build has no `dcb` builtin at all**; Sonic 1's `dcb` is purely its own macro and shadows nothing |
| `d9` | `dc.b [2][3]$FF` | 2 | `error #1010: symbol undefined` naming `[3]$FF`: only a LEADING bracket is a count, the rest is expression text |
| `d10` | `dc.b [-1]$FF` | 2 | `error #1920: code overflow`: the count is read unsigned |
| `d11` | `ds.b [2]3` | 2 | `error #1820`: `ds` does not take the group |
| `d12` | `move.w #[2]1,d0` | 2 | `error #1010` naming `[2]1`: no meaning in an instruction operand |
| `d13` | Sonic 1's `dcb` macro body, `dc.ATTRIBUTE [count]value` | 0 | expands to `dc.b [3]$FF` etc. and assembles. **`.ATTRIBUTE` substitution was already working**; the bracket was the only missing half |
| `d14` | `db [3]0FFh` / `dw [2]1234h` under `cpu z80` | 2 | `error #1010`: the group is a property of the 68k `dc.*` family, not of `db`/`dw` |
| `d15` | `dc.b [3]` with no value | 0 | `00 00 00` |
| `d16` | `db`/`dw` under `cpu 68000` | 2 | `error #1200`: those spellings do not exist there, so d14's refusal cannot be attributed to the CPU alone |
| `d17` | `[1024]`, `[1025]`, `[1596]`, `[$62A]` on one line each | 0 | all assemble. `MacroSetup.asm`'s "AS can only generate 1 kb of code on a single line" does not bind this build, and `sonic.asm` writes `dcb.b $62A,$FF` regardless |
| `l1` | `listing purecode` / `page 0` under `cpu 68000` | 0 | accepted, emits nothing |
| `l2` | `listing zqp_bogus` | 2 | `error #1520: only ON/OFF allowed`, although `purecode` is accepted: the message understates the set |
| `l3` | a bare `listing`, a bare `page` | 2 | `error #1110`: one argument, and one or two, respectively |
| `l4` | `listing purecode` / `page 0` under `cpu z80` | 0 | accepted on both surfaces |

## Where sigil diverges, deliberately

- `dc.b [3]` with no value: asl emits zeros (d15), sigil refuses by name. An
  empty operand meaning zero is a rule `dc` does not implement here, and
  inheriting it through the bracket path would let a truncated line assemble.
- A forward-referenced count: asl refuses (d6), sigil resolves it, because sigil
  assembles to convergence rather than in one pass.
- The `listing` argument vocabulary: asl validates it (l2), sigil does not. The
  accepted set is wider than asl's own message claims, nothing downstream reads
  the value, and a guessed vocabulary would refuse working source.
- `db`/`dc.b` are one directive in sigil on both CPUs, so sigil takes the group
  on either spelling where asl gates the whole spelling by CPU (d14, d16). The
  alias predates this work; a bracket-only gate would imitate half a rule sigil
  does not implement.
