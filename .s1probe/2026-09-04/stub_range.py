#!/usr/bin/env python3
"""Replace every `range` call site with the literal `dc.b` list AS would emit, so the
stub corpus stays SIZE- and BYTE-faithful there (the two `rept` stubs inside the macro
were the only size-changing edits in the whole stub set)."""
import re
import os

ROOT = "/home/volence/sonic_hacks/.s1recon-corpus"
SITES = [
    "_incObj/1A, 53 Collapsing Ledges and Floors.asm",
    "_incObj/2F, 35 MZ Large Grassy Platforms and Burning Grass.asm",
    "_incObj/5E SLZ Seesaw.asm",
]
NUM = re.compile(r"^[+-]?\$?[0-9A-Fa-f]+$")


def val(t):
    t = t.strip()
    neg = t.startswith("-")
    t = t.lstrip("+-")
    v = int(t[1:], 16) if t.startswith("$") else int(t, 10)
    return -v if neg else v


for rel in SITES:
    p = os.path.join(ROOT, rel)
    out = []
    for ln in open(p, encoding="utf-8", errors="surrogateescape"):
        m = re.match(r"^(\s*)range\s+(.*?)(\s*;.*)?$", ln.rstrip("\n"))
        if not m:
            out.append(ln.rstrip("\n"))
            continue
        ind, args = m.group(1), [a.strip() for a in m.group(2).split(",")]
        first, last, step = val(args[0]), val(args[1]), val(args[2])
        rep = val(args[3]) if len(args) > 3 and args[3] else 1
        n = 1 + abs(first - last) // abs(step)
        vals = []
        v = first
        for _ in range(n):
            vals.extend([v & 0xFF] * rep)
            v += step
        for i in range(0, len(vals), 8):
            out.append(ind + "dc.b\t" + ",".join(f"${x:02X}" for x in vals[i:i + 8]) + "\t;STUB range")
        print(f"{rel}: range {args} -> {len(vals)} bytes")
    open(p, "w", encoding="utf-8", errors="surrogateescape").write("\n".join(out) + "\n")
