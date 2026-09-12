#!/usr/bin/env python3
"""Per-SECTION move table between two listings: for every boundary label of the shape's frozen
size table (the section heads the chainer keys on), the address in A and in B and the delta.
usage: sectdiff.py <frozen-table.txt> <A.lst> <B.lst>"""
import re, sys

def listing(p):
    out = {}
    for line in open(p, errors='replace'):
        m = re.match(r'\s*\(\d+\)\s+\d+/\s*([0-9A-F]+)\s*:\s+(\S+):\s*$', line)
        if m:
            out.setdefault(m.group(2), int(m.group(1), 16))
    return out

heads = []
for line in open(sys.argv[1]):
    line = line.strip()
    if line and not line.startswith('#'):
        name = line.rsplit(' ', 1)[0].strip()
        heads.append(name)
a, b = listing(sys.argv[2]), listing(sys.argv[3])
rows = [(a[h], h, b[h] - a[h]) for h in heads if h in a and h in b]
rows.sort()
moved = [(addr, h, d) for addr, h, d in rows if d != 0]
print(f'{len(rows)} boundary labels compared, {len(moved)} moved')
for addr, h, d in moved:
    print(f'  {h:<34} {addr:#08x} -> {addr + d:#08x} ({d:+d})')
missing = [h for h in heads if h not in a or h not in b]
if missing:
    print('  not in both listings:', ', '.join(missing[:12]), '...' if len(missing) > 12 else '')
