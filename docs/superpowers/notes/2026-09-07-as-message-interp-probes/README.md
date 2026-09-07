# `message` and `\{expr}` interpolation probes

Reference build md5 `61e672562465725a8c102288a7da9098` through
`../asl-reference/asl_ref.sh`; `./run.sh p1b p2s p3 p4 p5b p6b p7s p8`
re-runs every probe three times and prints its stdout with `cat -A`.
`<probe>.asl.stdout` and `<probe>.asl.stderr` are the captured first run of
each; all three runs were byte-identical for every probe.

| probe | what it measures | exit | usable |
|---|---|---|---|
| `p1` | integer shapes, including `\{$}` under `cpu 68000` | 2 | NO: `$` is not a symbol under 68000 (`error #1020: invalid symbol name`), so the run carries an error |
| `p1b` | `p1` without the `\{$}` line | 0 | yes |
| `p2s` | float shapes: division, literals, symbols, exponents | 0 | yes |
| `p3` | string symbols (`equ` and `:=`) beside numeric interpolations | 0 | yes |
| `p4` | pass behaviour: a forward reference, `MOMPASS` | 0 | yes; prints every line TWICE, once per pass |
| `p5b` | the s2disasm ROM-size line and the trace-macro line | 0 | yes; `MOMLINE` renders on asl and stays verbatim on sigil |
| `p6` | `\{$}` under `cpu z80` with `padding off` | 2 | NO: `padding` is not a Z80 directive on asl |
| `p6b` | `p6` without `padding off` | 0 | yes |
| `p7s` | the `FloatString` rule: length cut, exponent thresholds | 0 | yes |
| `p8` | three cells the test table needed that `p2s` had not measured | 0 | yes |

The exponent-form literals (`1e17`) of the first drafts (`p2`, `p7`) were
respelled with a point (`1.0e17`) because sigil's lexer reads a float only
from `digits.digits`; the respelled probes render identically on asl.
