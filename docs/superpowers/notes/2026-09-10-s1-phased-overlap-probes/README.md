# the `+`-signed-literal probes (NOT the S1-LAYOUT-OVERLAP row)

`nested.asm` was filed as a `rept` nesting defect. **It is not one.** `rept` body
capture is correct; the trigger is the `+(+1)` in the `set` line, and `rept` is a
bystander.

    nested.asm  set .val, .val+(+1)   asl 21 21 22 22 23 23 24 24   sigil(pre-fix) 21 21 21 21 21 21 21 21
    flat.asm    the same `set`, NO rept anywhere at all
                                      asl 21 22 23                  sigil(pre-fix) 21 21 21

## the controls, and why the first two were confounded

`n2.asm` (the `set` moved BEFORE the inner `rept`) and `n3.asm` (the inner `rept`
replaced by an `if`) agree with asl — but each of them ALSO rewrote the `set` line
from `.val+(+1)` to `.val+1`, so each changed two variables and neither isolates
anything. `n4.asm` is the missing control: `nested.asm` with only the `+(+1)`
changed to `+1`. It agrees with asl pre-fix, which is what puts the fault in the
expression, not in the nesting. `flat.asm` then removes `rept` entirely and the
defect survives, which settles it.

## the fault

asl has no unary-plus OPERATOR. A leading `+` is a value only when the
(sub)expression asl scans is, entire, one signed number: asl tries the whole
string as a literal first and otherwise splits it at the rightmost operator of
the loosest tier present, where a leading `+` is a binary add with an empty left
operand (`error #1110`). Unary MINUS is a real operator there — the asymmetry is
asl's. `parse_atom` had the minus arm and no plus arm, so `+1` did not parse.

The SILENCE is a second, independent defect and it belongs to `set`:
`directive_set` ends at `defer_unresolved_assign`, whose first statement returns
without a word when the right-hand side does not PARSE, leaving the symbol's
previous value standing. `dc.b .val+(+1)` refuses loudly; `V equ 1+` refuses
loudly; `set .v, 1+` is exit 0 with no diagnostic and a stale value.

## the corpus reach

`Macros.asm(346)`'s `range` macro ends its loop body `set .val, .val+(step)`, so
a `+`-signed step substitutes into exactly this shape. Four call sites — the
ascending ones; the `-`-stepped calls beside them always worked:

    _incObj/2F, 35 MZ Large Grassy Platforms and Burning Grass.asm:322  range $21,$2F,+1
    the same file:336                                                   range $21,$3F,+1
    _incObj/1A, 53 Collapsing Ledges and Floors.asm:441                 range $21,$2F,+1,2
    _incObj/5E SLZ Seesaw.asm:310                                       range $26,$2C,+2

80 bytes emitted, 75 of them wrong (each range's first byte is right — it is the
increment that never happened), exit 0, not one diagnostic at any site.

`abs(step)` in the same macro's `rept` count reaches a DIFFERENT parser
(`eval.rs::parse_num_atom`), which was given its own unary-plus arm in an earlier
parcel. That is why the iteration count was right while the increment was dead.

## running them

    asl -xx -n -q -A -L -U -E -i . X.asm && p2bin -p=FF X.p X.asl.bin
    sigil X.asm -o X.sigil.bin

The reference `asl` is `s1disasm/build_tools/Linux-x86_64/asl`, md5
`61e672562465725a8c102288a7da9098`.
