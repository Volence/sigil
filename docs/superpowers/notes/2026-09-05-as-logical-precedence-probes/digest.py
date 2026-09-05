#!/usr/bin/env python3
"""Digest ladder.lst into probe/candidate triples and rule each relation."""
import re, sys
lines = open(sys.argv[1], errors="replace").read().splitlines()
cur = None
rows = []   # (probe, [(src, byte), ...])
for L in lines:
    m = re.match(r"\s*\d+/\s*[0-9A-F]+ :\s*([0-9A-F]{2})?\s*(.*)$", L)
    if not m:
        continue
    byte, src = m.group(1), m.group(2).rstrip()
    if src.startswith("; PROBE "):
        cur = src[len("; PROBE "):]
        rows.append((cur, []))
        continue
    if "dc.b" in src and rows:
        rows[-1][1].append((src.split("dc.b", 1)[1].strip(), byte))
print("%-24s %-22s %-4s %-24s %-24s %s" % ("PROBE","BARE","VAL","LEFT-GROUPED","RIGHT-GROUPED","VERDICT"))
for name, items in rows:
    if len(items) != 3:
        print("%-24s MALFORMED %r" % (name, items)); continue
    (be, bv), (le, lv), (re_, rv) = items
    if lv == rv:
        verdict = "CONFOUNDED (candidates agree: %s)" % lv
    elif bv == lv:
        verdict = "LEFT  op1 binds tighter"
    elif bv == rv:
        verdict = "RIGHT op2 binds tighter"
    else:
        verdict = "NEITHER (bare=%s)" % bv
    print("%-24s %-22s %-4s %-24s %-24s %s" % (name, be, bv, "%s=%s"%(le,lv), "%s=%s"%(re_,rv), verdict))
