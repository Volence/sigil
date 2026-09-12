#!/usr/bin/env python3
"""List the unpinned (scratch-slot) ROM sections of pass N of a ZBM_DUMP, with slot ordinal k."""
import re, sys

p, want = sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 0
passno = -1
k = 0
for line in open(p):
    if line.startswith('==='):
        passno += 1
        k = 0
        if passno == want:
            print(line.strip())
        continue
    if passno != want:
        continue
    m = re.match(r'(\d+) (\S+) head=(\S+) lma=(0x[0-9a-f]+) img=(0x[0-9a-f]+) pinned=(\S+)', line)
    if m and m.group(6) == 'false':
        lma = int(m.group(4), 16)
        masked = lma & 0xFFFFFF
        alias = ' ABS.W-ALIAS' if masked < 0x8000 else ''
        print(f'k={k:2d} idx={m.group(1):>3} {m.group(2):<28} head={m.group(3):<34} lma={lma:#x} masked={masked:#x} img={m.group(5)}{alias}')
        k += 1
