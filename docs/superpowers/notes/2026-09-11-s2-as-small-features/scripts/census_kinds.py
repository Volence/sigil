#!/usr/bin/env python3
"""census_kinds.py <census.err>... : classify every argument whose text as
written differs from its re-rendered tokens (the pre-parcel paste).

NUM      a numeric literal re-spelled with the same value ($0F for 15, 007 for 7)
SPACE    the same text once blanks are removed (only the paste's spacing moved)
PARTIAL  the call's operand did not lex before this parcel (rendered side short)
OTHER    anything else, printed in full
A call is counted once per distinct (site, rendered, raw) line."""
import ast, re, sys
from collections import Counter

NUM = re.compile(r'^-?(\$[0-9A-Fa-f]+|%[01]+|[0-9]+|[0-9][0-9A-Fa-f]*[hH])$')


def value(t):
    neg = t.startswith('-')
    t = t[1:] if neg else t
    if t.startswith('$'):
        v = int(t[1:], 16)
    elif t.startswith('%'):
        v = int(t[1:], 2)
    elif t[-1] in 'hH':
        v = int(t[:-1], 16)
    else:
        v = int(t, 10)
    return -v if neg else v


for path in sys.argv[1:]:
    kinds, other, sites = Counter(), [], set()
    for line in open(path, errors='replace'):
        if not line.startswith('SIGIL-ARGTEXT'):
            continue
        if line in sites:
            continue
        sites.add(line)
        _, site, rendered, raw = line.rstrip('\n').split('\t')
        rendered, raw = ast.literal_eval(rendered), ast.literal_eval(raw)
        if len(rendered) != len(raw):
            kinds['PARTIAL'] += 1
            other.append(('PARTIAL', site, rendered, raw))
            continue
        for r, w in zip(rendered, raw):
            if r == w:
                continue
            if NUM.match(w) and re.fullmatch(r'-?[0-9]+', r) and value(w) == int(r):
                kinds['NUM'] += 1
            elif re.sub(r'\s+', '', w) == re.sub(r'\s+', '', r):
                kinds['SPACE'] += 1
                other.append(('SPACE', site, r, w))
            else:
                kinds['OTHER'] += 1
                other.append(('OTHER', site, r, w))
    print('==', path, dict(kinds))
    seen = Counter()
    for k, site, r, w in other:
        key = (k, str(r), str(w))
        seen[key] += 1
    for (k, r, w), n in seen.most_common(40):
        print('  %-7s x%-4d %r => %r' % (k, n, r, w))
