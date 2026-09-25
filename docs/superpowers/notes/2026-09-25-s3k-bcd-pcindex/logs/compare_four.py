#!/usr/bin/env python3
"""compare_four.py: whole-file byte compare of each before/after ROM pair, and a
planted three-byte control per pair that the same compare must report exactly."""
import os
S = "/home/volence/sonic_hacks/.scratch/s3k-bcd-pcindex/aeon/"
for f in sorted(os.listdir(S + "before")):
    if not f.endswith(".bin"):
        continue
    a = open(S + "before/" + f, "rb").read()
    b = open(S + "after/" + f, "rb").read()
    diff = [i for i in range(max(len(a), len(b))) if i >= len(a) or i >= len(b) or a[i] != b[i]]
    p = bytearray(b)
    plant = [len(p) // 3, len(p) // 2, len(p) - 7]
    for i in plant:
        p[i] ^= 0x5A
    pd = [i for i in range(len(a)) if a[i] != p[i]]
    print("%-28s sizes %d/%d differing bytes %d; planted control found %s (planted %s)"
          % (f, len(a), len(b), len(diff), pd, plant))
