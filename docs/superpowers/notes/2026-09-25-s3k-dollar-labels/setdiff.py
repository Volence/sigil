#!/usr/bin/env python3
"""setdiff.py <old rows> <new rows>: multiset diff, classified per row."""
import collections, re, sys

def cls(row):
    if "`$` with no hex digits" in row:
        return "dollar-label"
    if "codepage" in row:
        return "codepage"
    if "`abcd`" in row or "`subx`" in row:
        return "abcd/subx"
    if "pc" in row.lower() and ("index" in row.lower() or "d8" in row.lower() or "(pc" in row.lower()):
        return "pc-indexed"
    return "OTHER: " + row

def load(p):
    return collections.Counter(l.rstrip("\n") for l in open(p) if l.strip())

old, new = load(sys.argv[1]), load(sys.argv[2])
left, entered, common = old - new, new - old, old & new
for name, ms in (("left", left), ("entered", entered), ("common", common)):
    per = collections.Counter()
    for r, n in ms.items():
        per[cls(r)] += n
    print(f"{name}: {sum(ms.values())}  " + ", ".join(f"{k}={v}" for k, v in sorted(per.items())))
    if name != "common":
        for r, n in sorted(ms.items()):
            print(f"  {name} x{n}: {r}")
tot = collections.Counter()
for r, n in new.items():
    tot[cls(r)] += n
print("new totals per class: " + ", ".join(f"{k}={v}" for k, v in sorted(tot.items())))
