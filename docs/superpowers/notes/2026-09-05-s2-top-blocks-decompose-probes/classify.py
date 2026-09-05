#!/usr/bin/env python3
"""Classify joined sigil diagnostics into root causes.

Usage: classify.py joined.tsv <class-message>
Prints a cause histogram (rows and distinct sites) and writes per-cause files
next to the input as <input>.<cause>.tsv so the residual can be read directly.
"""
import sys, re, os
from collections import defaultdict

path, cls = sys.argv[1], sys.argv[2]
rows = []
for raw in open(path, encoding='utf-8', errors='replace'):
    f, ln, msg, src = raw.rstrip('\n').split('\t', 3)
    if msg != cls:
        continue
    rows.append((f, int(ln), src))

# strip a trailing comment (AS: ';' outside quotes)
def strip_comment(s):
    out, q = [], None
    for ch in s:
        if q:
            out.append(ch)
            if ch == q:
                q = None
        elif ch in '"\'':
            q = ch; out.append(ch)
        elif ch == ';':
            break
        else:
            out.append(ch)
    return ''.join(out).rstrip()

NAMELESS = re.compile(r'^(\++|-+|/+)$')

def classify(src):
    body = strip_comment(src)
    stripped = body.strip()
    if not stripped:
        return 'blank-after-comment-strip'
    # definition side: line begins in column 1 with a nameless label token
    if body[:1] in '+-/' :
        head = body.split()[0] if body.split() else body
        if NAMELESS.match(head):
            return 'nameless-label-DEFINITION'
    # reference side: last comma-separated operand is a bare nameless token
    parts = stripped.split(None, 1)
    if len(parts) == 2:
        mnem, ops = parts
        last = ops.split(',')[-1].strip()
        if NAMELESS.match(last):
            return 'nameless-label-REFERENCE'
        if NAMELESS.match(ops.strip()):
            return 'nameless-label-REFERENCE'
    return 'OTHER'

hist_rows = defaultdict(int)
hist_sites = defaultdict(set)
buckets = defaultdict(list)
for f, ln, src in rows:
    c = classify(src)
    hist_rows[c] += 1
    hist_sites[c].add((f, ln))
    buckets[c].append((f, ln, src))

print("class: %s   rows=%d  distinct-sites=%d" % (cls, len(rows), len({(f,l) for f,l,_ in rows})))
for c in sorted(hist_rows, key=lambda k: -hist_rows[k]):
    print("  %-32s rows=%-6d sites=%d" % (c, hist_rows[c], len(hist_sites[c])))

base = os.path.splitext(path)[0]
for c, items in buckets.items():
    with open("%s.%s.tsv" % (base, c), 'w') as fh:
        for f, ln, src in items:
            fh.write("%s\t%d\t%s\n" % (f, ln, src))
