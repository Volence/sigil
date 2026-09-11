#!/usr/bin/env python3
"""check_listings.py <probe-dir> <sigil-binary> : for every probe whose asl run
exited 0 (read from <probe>.asl.out, the transcript probe.sh wrote), compare
asl's WHOLE byte stream (every listing byte column, continuation lines
included) with sigil's --hex output. Reports MATCH / MISMATCH per probe and
the refusal agreement for the probes asl refused. Bytes asl's listing does
not print (a ds reservation, an org gap) are not compared: such probes are
reported as SKIP-GAP when sigil's stream is longer than asl's printed one."""
import os, re, subprocess, sys

d, sig = sys.argv[1], sys.argv[2]
LINE = re.compile(r'^ +(\d+)/ +([0-9A-F]+) : ((?:[0-9A-F]{2,4} ?)+)')
CONT = re.compile(r'^ {15,}([0-9A-F]+) : ((?:[0-9A-F]{2,4} ?)+)\s*$')
res = {}
for f in sorted(os.listdir(d)):
    if not f.endswith('.asm'):
        continue
    b = f[:-4]
    out = os.path.join(d, b + '.asl.out')
    lst = os.path.join(d, b + '.lst')
    if not os.path.exists(out):
        continue
    m = re.search(r'ASL_EXIT=(\d+)', open(out).read())
    if not m:
        continue
    asl_exit = int(m.group(1))
    p = subprocess.run([sig, f, '--hex'], cwd=d, capture_output=True, text=True)
    sig_ok = p.returncode == 0
    if asl_exit != 0:
        res[f] = 'BOTH-REFUSE' if not sig_ok else 'ACCEPTS-WHAT-ASL-REFUSES'
        continue
    if not sig_ok:
        res[f] = 'MISMATCH (sigil refused)'
        continue
    # asl stream: address-keyed bytes from the listing
    mem = {}
    for line in open(lst, errors='replace'):
        mm = LINE.match(line) or CONT.match(line)
        if not mm:
            continue
        if mm.re is LINE:
            addr, cols = int(mm.group(2), 16), mm.group(3)
        else:
            addr, cols = int(mm.group(1), 16), mm.group(2)
        hexs = ''.join(cols.split())
        # Z80 listings group by byte, 68000 by word: both are just hex digits
        for i in range(0, len(hexs) - 1, 2):
            mem[addr + i // 2] = int(hexs[i:i + 2], 16)
    hexline = [l for l in p.stdout.splitlines() if re.fullmatch(r'([0-9A-F]{2} ?)+', l.strip() or 'x')]
    sb = bytes(int(x, 16) for x in ' '.join(hexline).split())
    if not mem:
        res[f] = 'MATCH' if not sb else 'MISMATCH (asl printed no bytes)'
        continue
    lo, hi = min(mem), max(mem) + 1
    gaps = [a for a in range(lo, hi) if a not in mem]
    want = bytes(mem.get(a, 0) for a in range(lo, hi))
    got = sb[lo:hi] if len(sb) >= hi else sb
    if got == want and len(sb) == hi:
        res[f] = 'MATCH' + (' (gap bytes not compared: %d)' % len(gaps) if gaps else '')
    elif got == want:
        res[f] = 'MATCH-PREFIX sigil has %d more bytes' % (len(sb) - hi)
    else:
        diff = [a for a in range(lo, min(hi, len(sb))) if a in mem and sb[a] != mem[a]]
        res[f] = 'MISMATCH at %s' % (['0x%X' % a for a in diff[:6]] or 'length %d vs %d' % (len(sb), hi))
for k, v in res.items():
    print('%-34s %s' % (k, v))
from collections import Counter
print(Counter(v.split(' ')[0] for v in res.values()))
