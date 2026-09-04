# F4 probes — macro default parameters, and an empty operand's span

Run each with the corpus's own oracle and flags; `../cmp.sh` does both sides at once.

    asl -xx -n -q -A -L -U -i . <file>.asm     # then read <file>.lst

| file | what it pins |
|---|---|
| `d1.asm` | the four call shapes of the `locVRAM` construct — omitted, supplied, keyword, explicitly empty |
| `d2.asm` | `ARGCOUNT` / `ALLARGS` / bindings across six calls of `ac macro p1,p2=DEF2,p3` |
| `d3.asm` | `shift` beside a default; default boundaries — parenthesised comma, interior whitespace, a string default |
| `d4.asm` | `shift` with the default on the last parameter, on the first, and absent |
| `d5.asm` | the shifted-`ALLARGS` rows the parcel could NOT pin (see the note) |
| `d7.asm` | the same over four parameters, with names that cannot collide with `dc.b` |
| `d11.asm` | a default's own identifiers are not parameters |
| `d12.asm` | a `{INTLABEL}` group consumes no slot beside a default |
| `sp1.asm` | a trailing empty operand — asl reads it as absolute address zero |
| `sp2.asm` | a leading empty operand — likewise |
| `f4b.asm` | the four call shapes as BYTES, for a direct asl-vs-sigil compare |

`d6.asm` is deliberately absent: it used parameter names `a,b,c,d`, and AS substituted
`c` inside `dc.b`. `d7.asm` is its uncontaminated replacement.
