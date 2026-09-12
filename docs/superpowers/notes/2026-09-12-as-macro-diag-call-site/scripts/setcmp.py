#!/usr/bin/env python3
"""setcmp.py BEFORE.stderr AFTER.stderr : compare two diagnostic streams.

Rows are `location: level: message`. Prints:
  - per-class decomposition (level x message-with-numbers-kept) with counts
  - the sorted multiset of (level, message) with the location stripped, diffed
    in both directions
  - how many rows changed location, and the in-expansion rows (a trail frame
    after the file(line) head) before and after
"""
import re, sys
from collections import Counter

ROW = re.compile(r"^(?P<loc>.*?): (?P<level>error|warning|note): (?P<msg>.*)$")
BARE = re.compile(r"^(?P<level>error|warning|note): (?P<msg>.*)$")

def load(path):
    rows = []
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.rstrip("\n")
        m = BARE.match(line)
        if m:
            rows.append(("<none>", m["level"], m["msg"]))
            continue
        m = ROW.match(line)
        if m and "(" in m["loc"]:
            rows.append((m["loc"], m["level"], m["msg"]))
    return rows

def trail(loc):
    # file(line) FRAME...:col  -> True when anything follows the first `)` before `:col`
    head = re.sub(r":\d+$", "", loc)
    i = head.find(")")
    return i >= 0 and head[i + 1:].strip() != ""

b, a = load(sys.argv[1]), load(sys.argv[2])
print(f"rows before: {len(b)}   rows after: {len(a)}")
mb = Counter((lv, msg) for _, lv, msg in b)
ma = Counter((lv, msg) for _, lv, msg in a)
gone = mb - ma
came = ma - mb
print(f"(level, message) multiset: {'IDENTICAL' if mb == ma else 'DIFFERS'}")
print(f"  in before, not after: {sum(gone.values())}")
for k, n in sorted(gone.items()):
    print(f"    -{n} {k}")
print(f"  in after, not before: {sum(came.values())}")
for k, n in sorted(came.items()):
    print(f"    +{n} {k}")

def cls(msg):
    return re.sub(r"`[^`]*`", "`_`", re.sub(r"\b\d+\b", "N", msg))
print("per-class decomposition (level, message class): before -> after")
cb = Counter((lv, cls(m)) for _, lv, m in b)
ca = Counter((lv, cls(m)) for _, lv, m in a)
for k in sorted(set(cb) | set(ca)):
    print(f"  {cb[k]:4d} -> {ca[k]:4d}  {k[0]}: {k[1]}")

# Same order on both sides? Pair rows positionally when the message sequences agree.
same_seq = [(lv, m) for _, lv, m in b] == [(lv, m) for _, lv, m in a]
print(f"row order and messages identical position by position: {same_seq}")
if same_seq:
    moved = [(x[0], y[0], y[2]) for x, y in zip(b, a) if x[0] != y[0]]
    print(f"rows whose location changed: {len(moved)}")
    for before, after, msg in moved:
        print(f"    {before}  ->  {after}   [{msg[:60]}]")
print(f"rows with a trail before: {sum(trail(l) for l, _, _ in b)}   after: {sum(trail(l) for l, _, _ in a)}")
print(f"distinct locations before: {len(set(l for l, _, _ in b))}   after: {len(set(l for l, _, _ in a))}")
