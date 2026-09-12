#!/usr/bin/env python3
"""Diff relaxable-site lengths of watched sections between two ZBM_DUMP files at one pass.

Maps each differing site's span start to a line of a candidate source file (argv[4:], tried in
order; the first whose text at that offset looks like an instruction line wins).
"""
import re, sys

def load(p, want):
    passno = -1
    sites = {}
    for line in open(p):
        if line.startswith('==='):
            passno += 1
            continue
        if passno != want:
            continue
        m = re.match(r'\s+SITE (\S+) src=(\d+) start=(\d+) len=(\d+)', line)
        if m:
            sites.setdefault(m.group(1), []).append((int(m.group(2)), int(m.group(3)), int(m.group(4))))
    return sites

a = load(sys.argv[1], int(sys.argv[3]))
b = load(sys.argv[2], int(sys.argv[3]))
cands = sys.argv[4:]
texts = {c: open(c, 'rb').read() for c in cands}

def where(start):
    out = []
    for c, t in texts.items():
        if start < len(t):
            ln = t.count(b'\n', 0, start) + 1
            s = t.rfind(b'\n', 0, start) + 1
            e = t.find(b'\n', start)
            out.append(f"{c.rsplit('/', 1)[-1]}:{ln}: {t[s:e].decode(errors='replace').strip()[:90]}")
    return out

for sec in sorted(set(a) | set(b)):
    sa, sb = a.get(sec, []), b.get(sec, [])
    if len(sa) != len(sb):
        print(sec, 'site count differs', len(sa), len(sb))
        continue
    diffs = [(x, y) for x, y in zip(sa, sb) if x[2] != y[2]]
    print(f'{sec}: {len(sa)} relaxable sites, {len(diffs)} change length (CTL -> RED), net {sum(y[2]-x[2] for x, y in diffs):+d} B')
    for x, y in diffs:
        print(f'   src={x[0]}/{y[0]} start={x[1]} len {x[2]} -> {y[2]}')
        for w in where(x[1]):
            print('      candidate', w)
