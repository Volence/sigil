#!/usr/bin/env python3
"""Given ROM addresses, print the asl listing line that emits each one."""
import re
import sys

LST = "/home/volence/sonic_hacks/.s1recon-ref/sonic.lst"
targets = sorted(int(a, 16) for a in sys.argv[1:])
row = re.compile(r"^(?:\(\d+\))?\s*\d+/\s*([0-9A-F]+) :")

best = {t: (None, -1) for t in targets}
with open(LST, encoding="utf-8", errors="replace") as f:
    for ln in f:
        m = row.match(ln)
        if not m:
            continue
        addr = int(m.group(1), 16)
        for t in targets:
            if addr <= t and addr > best[t][1]:
                best[t] = (ln.rstrip(), addr)
for t in targets:
    ln, a = best[t]
    print(f"${t:06X}  <- listing @ ${a:06X}: {ln}")
