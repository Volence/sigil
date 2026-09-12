#!/usr/bin/env python3
"""fit.py : each candidate model against every row of asl's exit-0 tables."""
import json, re

T = json.load(open("/home/volence/sonic_hacks/.scratch/as-missing-builtins/tables.json"))
M64 = (1 << 64) - 1


def val(lit):
    lit = lit.strip()
    if lit == "(-$7FFFFFFFFFFFFFFF-1)":
        return -(1 << 63)
    neg = lit.startswith("-")
    v = int(lit.lstrip("-").lstrip("$"), 16)
    return -v if neg else v


def ctz(x):
    x &= M64
    return (x & -x).bit_length() - 1 if x else -1


def firstbit(x):
    if x & 1:
        x >>= 1
    return -1 if x == 0 else ctz(x)


models = {
    "t_firstbit": firstbit,
    "t_bitcnt": lambda x: bin(x & M64).count("1"),
    "t_lastbit": lambda x: (x & M64).bit_length() - 1,
    "t_sgn": lambda x: (x > 0) - (x < 0),
}
for t, f in models.items():
    bad = 0
    for src, hx in T[t].items():
        x = val(re.match(r"dc\.l \w+\((.*)\)$", src).group(1))
        if (f(x) & 0xFFFFFFFF) != int(hx.replace(" ", ""), 16):
            bad += 1
            if bad <= 5:
                print("  MISMATCH", t, src, hx, f(x))
    print(t, "n=", len(T[t]), "mismatches=", bad)

# the plain lowest-set-bit model, to show the table rejects it
bad = sum(1 for src, hx in T["t_firstbit"].items()
          if (ctz(val(re.match(r"dc\.l \w+\((.*)\)$", src).group(1))) & 0xFFFFFFFF) != int(hx.replace(" ", ""), 16))
print("t_firstbit under plain ctz: mismatches=", bad)

bad = 0
for src, hx in T["t_abs"].items():
    x = val(re.match(r"dc\.l abs\((.*?)\)>>32", src).group(1))
    want = int(hx.replace(" ", "")[:16], 16)
    a = abs(x) if x != -(1 << 63) else x
    if (a & M64) != want:
        bad += 1
        print("  abs MISMATCH", src, hx)
print("t_abs n=", len(T["t_abs"]), "mismatches=", bad)

bad = 0
for src, hx in T["t_bitpos"].items():
    x = val(re.match(r"dc\.l bitpos\((.*)\)$", src).group(1))
    if ctz(x) != int(hx.replace(" ", ""), 16):
        bad += 1
        print("  bitpos MISMATCH", src, hx)
print("t_bitpos n=", len(T["t_bitpos"]), "mismatches=", bad)

for t, f in [("t_toupper", lambda c: c - 32 if 97 <= c <= 122 else c),
             ("t_tolower", lambda c: c + 32 if 65 <= c <= 90 else c)]:
    bad = 0
    for src, hx in T[t].items():
        c = int(re.match(r"dc\.b \w+\((\d+)\)$", src).group(1))
        if f(c) != int(hx, 16):
            bad += 1
            print("  ", t, "MISMATCH", src, hx)
    print(t, "n=", len(T[t]), "mismatches=", bad)

for t in ["t_sgn_float", "t_sgn_floatexpr"]:
    for src, hx in T[t].items():
        print(" ", t, src, "->", hx)
