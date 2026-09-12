#!/usr/bin/env python3
"""Census probes for asl's integer-valued builtins, one construct per file.

Each value probe is written twice: `a_*` with nothing above the construct and
`b_*` with an accepted immediate and a word above it, so a value asl declines
and fills from its last computed result shows up as a disagreement."""
import os, re

D = "/home/volence/sonic_hacks/.scratch/as-missing-builtins/probes"
os.makedirs(D, exist_ok=True)
for f in os.listdir(D):
    os.remove(os.path.join(D, f))

M = "\tcpu 68000\n\tpadding off\n\torg 0\n"
PRE = "\tmove.w #$1234,d0\n\tdc.w $5678\n"
END = "\tdc.b $EE\n\tend\n"

FNS = ["sgn", "bitcnt", "firstbit", "lastbit", "bitpos", "toupper", "tolower", "abs"]
ARGS = [
    "0", "1", "2", "3", "5", "6", "7", "8", "12", "$80", "$FF", "$FFFEB",
    "-1", "-2", "-5", "-$80",
    "$7FFFFFFF", "$80000000", "$FFFFFFFF", "$100000000",
    "$7FFFFFFFFFFFFFFF", "-$7FFFFFFFFFFFFFFF-1",
    "'a'", "'A'", "'z'", "'0'", "$E4", "353",
]
FLOATS = ["5.0", "0.0", "-2.5", "0.5", "2.5"]

P = {}


def slug(s):
    s = s.replace("-", "m").replace("$", "h").replace("'", "q").replace(".", "p").replace("+", "pl")
    return re.sub(r"[^A-Za-z0-9]", "_", s)


def val(name, line):
    P["a_" + name] = M + line + END
    P["b_" + name] = M + PRE + line + END


for f in FNS:
    for a in ARGS:
        # the low 32 bits and the high 32 bits, as two separate files
        val(f"{f}_{slug(a)}_lo", f"\tdc.l ({f}({a}))&$FFFFFFFF\n")
        val(f"{f}_{slug(a)}_hi", f"\tdc.l ({f}({a}))>>32\n")
    for a in FLOATS:
        # the bare call (is the result an int or a float?) and under INT
        val(f"{f}_f{slug(a)}_bare", f"\tdc.l {f}({a})\n")
        val(f"{f}_f{slug(a)}_int", f"\tdc.l INT({f}({a}))\n")
    val(f"{f}_empty", f"\tdc.l {f}()\n")
    val(f"{f}_empty_int", f"\tdc.l INT({f}())\n")
    val(f"{f}_upper", f"\tdc.l {f.upper()}(5)\n")
    val(f"{f}_fwd", f"\tdc.l {f}(Later)\nLater equ 5\n")
    val(f"{f}_equ", f"X equ {f}(5)\n\tdc.l X\n")
    val(f"{f}_imm", f"\tmove.l #{f}(5),d0\n")
    val(f"{f}_string", f"\tdc.l {f}(\"A\")\n")
    val(f"{f}_string2", f"\tdc.l {f}(\"ab\")\n")
    val(f"{f}_twoarg", f"\tdc.l {f}(1,2)\n")
    val(f"{f}_undef", f"\tdc.l {f}(Nope)\n")
    val(f"{f}_space", f"\tdc.l {f}( 5 )\n")
    val(f"{f}_nested", f"\tdc.l {f}({f}(5))\n")
    val(f"{f}_floatsym", f"F set 2.5\n\tdc.l {f}(F)\n")
    val(f"{f}_emptyparen", f"\tdc.l {f}(())\n")

# The empty argument on every OTHER known builtin, for the census.
for f in ["int", "sin", "cos", "tan", "atan", "sqrt", "exp", "log", "ln", "asin", "acos", "sinh", "cosh",
          "tanh", "asinh", "acosh", "atanh"]:
    val(f"x_{f}_empty", f"\tdc.l INT({f}())\n")
    val(f"x_{f}_empty_bare", f"\tdc.l {f}()\n")

# Interpolation: the only corpus use of sgn (s1disasm MacroSetup.asm(221)).
SIG = 'signedToString function number,substr("-",0,-sgn(number))+"$\\{abs(number)}"\n'
val("i_signed_neg", SIG + '\tdc.b signedToString(-5)\n')
val("i_signed_pos", SIG + '\tdc.b signedToString(5)\n')
val("i_signed_zero", SIG + '\tdc.b signedToString(0)\n')
val("i_sgn_interp", '\tdc.b "\\{sgn(-5)}"\n')
val("i_sgn_interp_pos", '\tdc.b "\\{sgn(7)}"\n')
val("i_bitcnt_interp", '\tdc.b "\\{bitcnt(7)}"\n')
val("i_sgn_substr", '\tdc.b substr("-",0,-sgn(-5))\n')
val("i_sgn_substr0", '\tdc.b substr("xyz",0,sgn(5))\n')
val("i_sgn_if", "\tif sgn(-5)<0\n\tdc.b 1\n\telse\n\tdc.b 2\n\tendif\n")
val("i_toupper_str", '\tdc.b toupper("a")\n')

for k, v in P.items():
    with open(os.path.join(D, k + ".asm"), "w") as fh:
        fh.write(v)
print(len(P), "probes")
