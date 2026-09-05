#!/usr/bin/env python3
"""Join sigil diagnostics to their source text.

Usage: join_source.py <diag-file> <corpus-root> > joined.tsv
Emits: file \t line \t message \t source-text
Lines whose (file,line) cannot be resolved get source-text "<<UNRESOLVED>>".
"""
import sys, re, os

diag_path, root = sys.argv[1], sys.argv[2]
pat = re.compile(r'^(.*)\((\d+)\): (?:error|warning): (.*)$')
cache = {}

def get(f):
    if f not in cache:
        p = os.path.join(root, f)
        try:
            with open(p, 'rb') as fh:
                cache[f] = fh.read().decode('utf-8', 'replace').split('\n')
        except OSError:
            cache[f] = None
    return cache[f]

n_bad = 0
for raw in open(diag_path, encoding='utf-8', errors='replace'):
    raw = raw.rstrip('\n')
    m = pat.match(raw)
    if not m:
        sys.stderr.write("UNPARSED: %s\n" % raw)
        n_bad += 1
        continue
    f, ln, msg = m.group(1), int(m.group(2)), m.group(3)
    lines = get(f)
    if lines is None or ln < 1 or ln > len(lines):
        src = "<<UNRESOLVED>>"
    else:
        src = lines[ln-1].rstrip('\r')
    print("%s\t%d\t%s\t%s" % (f, ln, msg, src))
sys.stderr.write("unparsed lines: %d\n" % n_bad)
