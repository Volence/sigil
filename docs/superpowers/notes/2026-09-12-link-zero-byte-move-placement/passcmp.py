#!/usr/bin/env python3
"""Compare two passes WITHIN one ZBM_DUMP: per ROM section, img at pass A vs pass B (and lma)."""
import re, sys

p, a, b = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
passes = []
for line in open(p):
    if line.startswith('==='):
        passes.append({'hdr': line.strip(), 'rows': {}, 'sites': {}})
        continue
    m = re.match(r'(\d+) (\S+) head=(\S+) lma=(0x[0-9a-f]+) img=(0x[0-9a-f]+) pinned=(\S+)', line)
    if m:
        passes[-1]['rows'][int(m.group(1))] = (m.group(2), int(m.group(4), 16), int(m.group(5), 16), m.group(6))
print('A:', passes[a]['hdr'], ' B:', passes[b]['hdr'])
n = 0
for idx in sorted(passes[a]['rows']):
    ra = passes[a]['rows'][idx]
    rb = passes[b]['rows'].get(idx)
    if rb is None or ra[2] != rb[2]:
        n += 1
        print(f'  idx={idx} {ra[0]:<24} A lma={ra[1]:#x} img={ra[2]:#x} ({ra[3]})  B lma={rb[1]:#x} img={rb[2]:#x} ({rb[3]})' if rb else f'  idx={idx} missing in B')
print('sections whose img differs between the two passes:', n)
