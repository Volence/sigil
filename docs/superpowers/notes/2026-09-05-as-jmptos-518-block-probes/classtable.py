"""Class table for one joined.tsv, and the before/after comparison.

Usage: classtable.py <before.tsv> <after.tsv>

Prints, for each message class, rows and distinct sites in both runs and the
delta, so a class that ROSE or APPEARED is visible rather than being netted out
against a class that fell.
"""
import collections
import re
import sys


def key(msg):
    k = re.sub(r'`[^`]*`', '`X`', msg)
    k = re.sub(r'cannot include .*', 'cannot include <file>: no such file', k)
    k = re.sub(r'unsupported form: .*', 'unsupported form: <insn>', k)
    return k


def load(path):
    h = collections.defaultdict(lambda: [0, set()])
    sites = set()
    for line in open(path):
        f, ln, msg, _src = line.rstrip('\n').split('\t', 3)
        k = key(msg)
        h[k][0] += 1
        h[k][1].add((f, ln))
        sites.add((f, ln))
    return h, sites


b, bs = load(sys.argv[1])
a, ass = load(sys.argv[2])

print("| rows before | sites before | rows after | sites after | row delta | class |")
print("|---|---|---|---|---|---|")
for k in sorted(set(b) | set(a), key=lambda k: -(b.get(k, [0, set()])[0])):
    br, bsi = (b[k][0], len(b[k][1])) if k in b else (0, 0)
    ar, asi = (a[k][0], len(a[k][1])) if k in a else (0, 0)
    mark = ""
    if ar > br:
        mark = "  <-- ROSE"
    if br == 0:
        mark = "  <-- APPEARED"
    if ar == 0:
        mark = "  <-- GONE"
    print(f"| {br} | {bsi} | {ar} | {asi} | {ar - br:+d} | `{k}`{mark} |")
print(f"| **{sum(v[0] for v in b.values())}** | **{len(bs)}** | "
      f"**{sum(v[0] for v in a.values())}** | **{len(ass)}** | "
      f"**{sum(v[0] for v in a.values()) - sum(v[0] for v in b.values()):+d}** | **TOTAL** |")
