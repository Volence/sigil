"""corpus_escapes.py <tree> [<tree> ...] : every string or character literal
carrying a backslash, in every .asm/.inc file of each tree (.git skipped).

Literal scanning follows asl's measured rule: inside a literal a backslash
consumes the next character, so an escaped quote does not close it. A `'` that
follows an identifier character is not an opener (the Z80 `af'` shadow form).
A `;` outside a literal starts a comment.

Each hit is classified by the statement it sits in and by escape kind:
`interp` for `\\{`, `escape` for every other backslash form. Lines, literals
and escape occurrences are counted separately.
"""
import os, re, sys
from collections import Counter

def literals(line):
    out, i, n = [], 0, len(line)
    while i < n:
        c = line[i]
        if c == ';':
            break
        if c in '"\'':
            if c == "'" and i > 0 and (line[i - 1].isalnum() or line[i - 1] in '_.'):
                i += 1
                continue
            j = i + 1
            while j < n and line[j] != c:
                j += 2 if line[j] == '\\' else 1
            out.append((c, line[i + 1:min(j, n)], j >= n))
            i = j + 1
            continue
        i += 1
    return out

def head(line):
    t = line.split(';')[0].strip()
    if not t:
        return ''
    w = t.split()
    k = 0
    if w[0].endswith(':') and len(w) > 1:
        k = 1
    elif len(w) > 1 and w[1].lower() in ('equ', 'set', ':=', '=', 'macro', 'function'):
        return w[1].lower()
    return w[k].lower()

def kinds(body):
    out, i = [], 0
    while i < len(body):
        if body[i] == '\\':
            nxt = body[i + 1] if i + 1 < len(body) else ''
            out.append('interp' if nxt == '{' else 'escape:' + (nxt if not nxt.isdigit() else 'digit'))
            i += 2
            continue
        i += 1
    return out

for tree in sys.argv[1:]:
    lines_hit = 0
    by_head = Counter()
    by_kind = Counter()
    lits = 0
    examples = {}
    unterminated = 0
    for dp, dn, fn in os.walk(tree):
        dn[:] = [d for d in dn if d != '.git']
        for f in sorted(fn):
            if not f.lower().endswith(('.asm', '.inc')):
                continue
            p = os.path.join(dp, f)
            for ln, l in enumerate(open(p, encoding='latin-1', errors='replace'), 1):
                l = l.rstrip('\n')
                hit = [(q, b, u) for q, b, u in literals(l) if '\\' in b]
                if not hit:
                    continue
                lines_hit += 1
                h = head(l)
                ks = [k for _, b, _ in hit for k in kinds(b)]
                cls = ('interp-only' if all(k == 'interp' for k in ks) else
                       'escape-only' if all(k != 'interp' for k in ks) else 'both')
                key = '%s | %s | %s' % (h, 'char' if any(q == "'" for q, _, _ in hit) else 'string', cls)
                by_head[key] += 1
                examples.setdefault(key, '%s:%d: %s' % (os.path.relpath(p, tree), ln, l.strip()[:110]))
                for k in ks:
                    by_kind[k] += 1
                lits += len(hit)
                unterminated += sum(1 for _, _, u in hit if u)
    print('== %s' % tree)
    print('   lines with a backslash in a literal: %d, literals: %d, unterminated: %d' % (lines_hit, lits, unterminated))
    for k, v in sorted(by_head.items(), key=lambda kv: (-kv[1], kv[0])):
        print('   %5d  %-45s e.g. %s' % (v, k, examples[k]))
    print('   escape occurrences by kind:', dict(sorted(by_kind.items())))
