#!/usr/bin/env python3
"""Label-address diff of two sigil listings: first differing labels, and the net shift at the end."""
import re, sys

def load(p):
    rows = []
    for line in open(p, errors='replace'):
        m = re.match(r'\s*\(\d+\)\s+\d+/\s*([0-9A-F]+)\s*:\s+(\S+):\s*$', line)
        if m:
            rows.append((m.group(2), int(m.group(1), 16)))
    return rows

a = load(sys.argv[1])
b = dict(load(sys.argv[2]))
print(f'{len(a)} labels in A, {len(b)} in B')
prev = None
shown = 0
deltas = {}
for name, addr in a:
    if name not in b:
        continue
    d = b[name] - addr
    deltas[d] = deltas.get(d, 0) + 1
    if d == 0:
        if shown == 0:
            prev = (name, addr)
        continue
    if shown < 4:
        print(f'  DIFF {name} {addr:#x} -> {b[name]:#x} ({d:+d})')
        shown += 1
print('  last unchanged label before the first diff:', prev and (prev[0], hex(prev[1])))
print('  label count by delta:', dict(sorted(deltas.items())))
