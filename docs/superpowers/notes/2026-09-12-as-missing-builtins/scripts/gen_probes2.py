#!/usr/bin/env python3
"""Round 2: exhaustive value TABLES (one file per function, every line one
call; a table is a value source only when its whole run exits 0), plus the
contexts round 1 could not answer. Written into probes2/."""
import os, random

D = "/home/volence/sonic_hacks/.scratch/as-missing-builtins/probes2"
os.makedirs(D, exist_ok=True)
for f in os.listdir(D):
    os.remove(os.path.join(D, f))

M = "\tcpu 68000\n\tpadding off\n\torg 0\n"
END = "\tdc.b $EE\n\tend\n"
P = {}


def lit(v):
    """asl spelling of a 64-bit signed value."""
    if v == -(1 << 63):
        return "(-$7FFFFFFFFFFFFFFF-1)"
    return f"-${-v:X}" if v < 0 else f"${v:X}"


rng = random.Random(20260912)
VALS = list(range(0, 1024))
VALS += [-v for v in range(1, 257)]
for k in range(64):
    VALS += [(1 << k) if k < 63 else -(1 << 63)]
    VALS += [((1 << k) - 1) if k < 64 else 0]
    VALS += [((1 << k) + 1) if k < 63 else -(1 << 63) + 1]
    VALS += [((1 << k) * 3) if k < 62 else 0]
for _ in range(300):
    v = rng.getrandbits(64)
    if v >= 1 << 63:
        v -= 1 << 64
    VALS.append(v)
for _ in range(100):
    # sparse patterns: few bits, some with bit 0 set
    v = 0
    for _ in range(rng.randint(1, 4)):
        v |= 1 << rng.randint(0, 63)
    if v >= 1 << 63:
        v -= 1 << 64
    VALS.append(v)
VALS = [v for v in dict.fromkeys(VALS) if -(1 << 63) <= v < (1 << 63)]
with open(os.path.join(D, "VALS.txt"), "w") as fh:
    fh.write("\n".join(str(v) for v in VALS) + "\n")

for f in ["firstbit", "bitcnt", "lastbit", "sgn"]:
    P[f"t_{f}"] = M + "".join(f"\tdc.l {f}({lit(v)})\n" for v in VALS) + END
# abs: the full 64 bits, two longs per value
P["t_abs"] = M + "".join(f"\tdc.l abs({lit(v)})>>32,abs({lit(v)})&$FFFFFFFF\n" for v in VALS) + END
# bitpos: the one-bit values it accepts
P["t_bitpos"] = M + "".join(f"\tdc.l bitpos({lit(1 << k)})\n" for k in range(63)) + END
P["t_toupper"] = M + "".join(f"\tdc.b toupper({v})\n" for v in range(256)) + END
P["t_tolower"] = M + "".join(f"\tdc.b tolower({v})\n" for v in range(256)) + END
# sgn over floats
FL = ["0.0", "-0.0", "0.5", "-0.5", "1.0", "-1.0", "2.5", "-2.5", "1000000.0", "-1000000.0", "0.000001", "-0.000001"]
P["t_sgn_float"] = M + "".join(f"\tdc.l sgn({x})\n" for x in FL) + END
P["t_sgn_floatexpr"] = M + "\tdc.l sgn(1.5-1.5)\n\tdc.l sgn(0.1-0.2)\n\tdc.l sgn(sqrt(2))\n\tdc.l sgn(log(0.5))\n" + END

# ---- one construct per file below ----------------------------------------
one = {}
for f in ["bitpos"]:
    one[f"{f}_upper"] = f"\tdc.l BITPOS(8)\n"
    one[f"{f}_fwd"] = f"\tdc.l bitpos(Later)\nLater equ 8\n"
    one[f"{f}_equ"] = f"X equ bitpos(8)\n\tdc.l X\n"
    one[f"{f}_imm"] = f"\tmove.l #bitpos(8),d0\n"
    one[f"{f}_space"] = f"\tdc.l bitpos( 8 )\n"
    one[f"{f}_nested"] = f"\tdc.l bitpos(bitpos(4))\n"
    one[f"{f}_min"] = f"\tdc.l bitpos(-$7FFFFFFFFFFFFFFF-1)\n"
    one[f"{f}_neg1"] = f"\tdc.l bitpos(-1)\n"
for f in ["toupper", "tolower"]:
    one[f"{f}_97"] = f"\tdc.b {f}(97)\n"
    one[f"{f}_65"] = f"\tdc.b {f}(65)\n"
    one[f"{f}_256"] = f"\tdc.b {f}(256)\n"
    one[f"{f}_m1"] = f"\tdc.b {f}(-1)\n"
    one[f"{f}_255"] = f"\tdc.b {f}(255)\n"
    one[f"{f}_neg_expr"] = f"\tdc.b {f}(0-1+1)\n"
    one[f"{f}_word"] = f"\tdc.w {f}($61)\n"
# character literals as an argument, and in an expression argument
for f in ["lastbit", "abs", "sgn", "bitcnt", "firstbit", "toupper", "int", "sin"]:
    one[f"c_{f}_q"] = f"\tdc.l {f}('a')\n"
    one[f"c_{f}_qplus"] = f"\tdc.l {f}('a'+0)\n"
    one[f"c_{f}_qparen"] = f"\tdc.l {f}(('a'))\n"
    one[f"c_{f}_qsym"] = f"Q equ 'a'\n\tdc.l {f}(Q)\n"
    one[f"c_{f}_dq"] = f"\tdc.l {f}(\"a\"+0)\n"
one["c_plain_q"] = "\tdc.l 'a'\n"
one["c_plain_qplus"] = "\tdc.l 'a'+1\n"
one["c_int_str"] = "\tdc.l int(\"a\")\n"
# the empty argument in more contexts
one["e_abs_empty_b"] = "\tdc.b abs()\n"
one["e_abs_empty_imm"] = "\tmove.l #abs(),d0\n"
one["e_abs_empty_equ"] = "X equ abs()\n\tdc.l X\n"
one["e_abs_empty_sp"] = "\tdc.l abs( )\n"
one["e_sgn_empty_expr"] = "\tdc.l sgn()+5\n"
one["e_firstbit_empty_b"] = "\tdc.b firstbit()\n"
one["e_int_empty_b"] = "\tdc.b int()\n"
one["e_sin_empty_int"] = "\tdc.b int(sin())\n"
one["e_cos_empty_int"] = "\tdc.b int(cos())\n"
one["e_abs_empty_z80"] = None  # z80 below
# z80 and interpolation of a negative
Z = "\tcpu z80\n\torg 0\n"
ZEND = "\tdb 0EEh\n\tend\n"
del one["e_abs_empty_z80"]
for k, v in one.items():
    P["o_" + k] = M + v + END
P["o_z80_family"] = Z + "\tdb sgn(-5)\n\tdb bitcnt(7)\n\tdb firstbit(12)\n\tld a,toupper(97)\n\tdb abs()\n" + ZEND
P["o_interp_m1"] = M + '\tdc.b "\\{-1}"\n' + END
P["o_interp_m1_sgn"] = M + '\tdc.b "\\{sgn(-1)}"\n' + END
P["o_interp_firstbit"] = M + '\tdc.b "\\{firstbit(12)}"\n' + END
P["o_interp_toupper"] = M + '\tdc.b "\\{toupper(97)}"\n' + END

for k, v in P.items():
    with open(os.path.join(D, k + ".asm"), "w") as fh:
        fh.write(v)
print(len(P), "probes,", len(VALS), "table values")
