#!/usr/bin/env python3
"""compare.py <reference> <candidate>: whole-image byte compare, then a positive
control that plants three bytes into a copy of the candidate and requires the
comparer to report exactly those three offsets."""
import sys, zlib

def diff(a, b):
    out = [i for i in range(min(len(a), len(b))) if a[i] != b[i]]
    return out, len(a) != len(b)

ref = open(sys.argv[1], "rb").read()
cand = open(sys.argv[2], "rb").read()
for name, d in (("reference", ref), ("candidate", cand)):
    print(f"{name}: {len(d)} bytes, crc32 {zlib.crc32(d) & 0xffffffff:08x}")
offs, lenmiss = diff(ref, cand)
print(f"compare: {len(offs)} differing bytes, length mismatch {lenmiss}")
planted = bytearray(cand)
want = sorted({0, len(cand) // 2, len(cand) - 1})
for o in want:
    planted[o] ^= 0xFF
got, _ = diff(cand, bytes(planted))
print(f"control: planted at {want}, comparer reported {got}: {'OK' if got == want else 'CONTROL FAILED'}")
got2, _ = diff(ref, bytes(planted))
print(f"control vs reference: {len(got2)} differing bytes (must be {len(want)} when the compare is clean)")
sys.exit(0 if (not offs and not lenmiss and got == want and len(got2) == len(want)) else 1)
