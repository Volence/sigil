#!/usr/bin/env python3
"""Per-class before/after decomposition of a sigil diagnostic log pair.

A class is the message text with the `file(line):` prefix stripped and
backticked identifiers / bare integers normalised, so one defect that fires
under many names still lands in one row.  Both totals are printed and both are
reconciled against the sum of the rows, so a remainder cannot hide.
"""
import re, sys, collections

def classify(path):
    c = collections.Counter()
    n = 0
    for line in open(path, encoding='utf-8', errors='replace'):
        line = line.rstrip('\n')
        if not line:
            continue
        n += 1
        m = re.match(r'^.*?\(\d+\): (error|warning|note): (.*)$', line)
        if not m:
            c['<<UNCLASSIFIED>> ' + line[:60]] += 1
            continue
        msg = re.sub(r'`[^`]*`', '`X`', m.group(2))
        msg = re.sub(r'-?\d+', 'N', msg)
        c[f'{m.group(1)}: {msg}'] += 1
    return c, n

b, bn = classify(sys.argv[1])
a, an = classify(sys.argv[2])
keys = sorted(set(b) | set(a), key=lambda k: (-(a[k] - b[k]), k))
print(f"{'before':>7} {'after':>7} {'delta':>7}  class")
for k in keys:
    d = a[k] - b[k]
    print(f"{b[k]:>7} {a[k]:>7} {d:>+7}  {k}")
print(f"{sum(b.values()):>7} {sum(a.values()):>7} {sum(a.values())-sum(b.values()):>+7}  == SUM OF ROWS")
print(f"{bn:>7} {an:>7} {an-bn:>+7}  == RAW LINE COUNTS")
assert sum(b.values()) == bn and sum(a.values()) == an, "remainder!"
print("reconciled: no remainder")
