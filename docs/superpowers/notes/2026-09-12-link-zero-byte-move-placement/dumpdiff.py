#!/usr/bin/env python3
"""Diff two ZBM_DUMP files pass by pass: per ROM section index, name, head, lma, img, pinned."""
import re, sys

def load(p):
    passes = []
    cur = None
    for line in open(p):
        if line.startswith('==='):
            cur = {'hdr': line.strip(), 'rows': {}}
            passes.append(cur)
            continue
        m = re.match(r'(\d+) (\S+) head=(\S+) lma=(0x[0-9a-f]+) img=(0x[0-9a-f]+) pinned=(\S+) nfrag=(\d+)', line)
        if m:
            cur['rows'][int(m.group(1))] = (m.group(2), m.group(3), int(m.group(4), 16),
                                            int(m.group(5), 16), m.group(6) == 'true', int(m.group(7)))
    return passes

def fmt(t):
    return t and (t[0], t[1], hex(t[2]), hex(t[3]), 'pin' if t[4] else 'scr', t[5])

c = load(sys.argv[1])
r = load(sys.argv[2])
limit = int(sys.argv[3]) if len(sys.argv) > 3 else 25
for i, (pc, pr) in enumerate(zip(c, r)):
    print('PASS', i, pc['hdr'], '|', pr['hdr'], 'nrom', len(pc['rows']), len(pr['rows']))
    diffs = 0
    for idx in sorted(set(pc['rows']) | set(pr['rows'])):
        a = pc['rows'].get(idx)
        b = pr['rows'].get(idx)
        if a != b:
            diffs += 1
            if diffs <= limit:
                print('  ', idx, 'CTL', fmt(a))
                print('  ', ' ' * len(str(idx)), 'RED', fmt(b))
    print('  total differing rows', diffs)
