#!/usr/bin/env python3
"""Compare asl's and sigil's locations per probe, row by row, column stripped.
Reads the asl transcripts (logs/asl-*.txt, rows `> > > loc: error...`) and a sigil
probe log; prints MATCH / DIFF per row and a total."""
import re, sys, glob
S = "/home/volence/sonic_hacks/.scratch/as-macro-diag/logs/"

def rows(path, asl):
    out, cur = {}, None
    for line in open(path):
        line = line.rstrip("\n")
        m = re.match(r"=== (\S+)\.asm", line)
        if m:
            cur = m.group(1); out.setdefault(cur, []); continue
        if cur is None:
            continue
        if asl:
            m = re.match(r"> > > (.*?)(?::\d+)?: (error|warning)", line)
        else:
            m = re.match(r"(.*?)(?::\d+)?: (error|warning)", line)
        if m:
            out[cur].append(m.group(1))
    return out

asl = {}
for p in sorted(glob.glob(S + "asl-*.txt")) + [S + "asl-round1.txt"]:
    try:
        asl.update({k: v for k, v in rows(p, True).items() if v})
    except FileNotFoundError:
        pass
sig = rows(sys.argv[1], False)
match = diff = 0
for probe in sorted(asl):
    a = asl[probe]; s = sig.get(probe)
    if s is None:
        print(f"NOT-RUN  {probe}")
        continue
    ok = a == s
    match += ok; diff += (not ok)
    print(f"{'MATCH' if ok else 'DIFF '}  {probe}")
    if not ok:
        print(f"   asl:   {a}\n   sigil: {s}")
print(f"TOTAL probes compared: {match + diff}, MATCH {match}, DIFF {diff}")
