# rept-nesting probes (NOT this row — a separate defect found while measuring it)

`nested.asm` is the minimal repro: a `set` that FOLLOWS an inner `rept … endr`
inside an outer `rept` body does not execute per outer iteration.

    asl:    21 21 22 22 23 23 24 24
    sigil:  21 21 21 21 21 21 21 21

`n2.asm` (the `set` moved BEFORE the inner `rept`) and `n3.asm` (the inner `rept`
replaced by an `if` block) are the two controls; sigil matches asl on both, which
is what localises the fault to `rept` body capture rather than to `set`.

Run:

    asl -xx -n -q -A -L -U -E -i . X.asm && p2bin -p=FF X.p X.asl.bin
    sigil X.asm -o X.sigil.bin

The reference `asl` is `s1disasm/build_tools/Linux-x86_64/asl`, md5
`61e672562465725a8c102288a7da9098`. Reached from the Sonic 1 corpus through
`Macros.asm:346`'s `range` macro, which is byte-wrong at four sites totalling 75
bytes and draws no diagnostic at any of them.
