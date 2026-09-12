#!/usr/bin/env python3
"""Round 5: take Sonic 1's signedToString apart, one construct per file, to
find which piece sigil refuses."""
import os

D = "/home/volence/sonic_hacks/.scratch/as-missing-builtins/probes5"
os.makedirs(D, exist_ok=True)
for f in os.listdir(D):
    os.remove(os.path.join(D, f))
M = "\tcpu 68000\n\tpadding off\n\torg 0\n"
END = "\tdc.b $EE\n\tend\n"
P = {
    "v_substr_sgn": '\tdc.b substr("-",0,-sgn(-5))\n',
    "v_substr_lit": '\tdc.b substr("-",0,1)\n',
    "v_concat": '\tdc.b "-"+"$\\{abs(-5)}"\n',
    "v_concat_lit": '\tdc.b "-"+"x"\n',
    "v_concat_substr": '\tdc.b substr("-",0,1)+"x"\n',
    "v_fn_str_const": 'f function number,"ab"\n\tdc.b f(1)\n',
    "v_fn_concat_const": 'f function number,"a"+"b"\n\tdc.b f(1)\n',
    "v_fn_interp_only": 'f function number,"$\\{abs(number)}"\n\tdc.b f(-5)\n',
    "v_fn_substr_lit": 'f function number,substr("-",0,1)\n\tdc.b f(-5)\n',
    "v_fn_substr_sgn": 'f function number,substr("-",0,-sgn(number))\n\tdc.b f(-5)\n',
    "v_fn_nosgn": 'f function number,substr("-",0,1)+"$\\{abs(number)}"\n\tdc.b f(-5)\n',
    "v_fn_full": 'f function number,substr("-",0,-sgn(number))+"$\\{abs(number)}"\n\tdc.b f(-5)\n',
    "v_fn_full_pos": 'f function number,substr("-",0,-sgn(number))+"$\\{abs(number)}"\n\tdc.b f(5)\n',
    "v_fn_full_zero": 'f function number,substr("-",0,-sgn(number))+"$\\{abs(number)}"\n\tdc.b f(0)\n',
    "v_fn_full_str": 'f function number,substr("-",0,-sgn(number))+"$\\{abs(number)}"\nS set f(-5)\n\tdc.b S\n',
}
for k, v in P.items():
    with open(os.path.join(D, k + ".asm"), "w") as fh:
        fh.write(M + v + END)
print(len(P), "probes")
