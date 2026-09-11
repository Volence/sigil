"""classify_rows.py <rows>... : message-class and source-file census of sigil
diagnostic rows, plus the unresolved-symbol name multiset. Prints full sets."""
import collections, re, sys
for p in sys.argv[1:]:
    rows = [l for l in open(p).read().split('\n') if l.strip()]
    print('==', p, 'rows', len(rows))
    cls = collections.Counter(); files = collections.Counter(); names = collections.Counter()
    fc = collections.Counter()
    for r in rows:
        m = re.match(r'^([^(]+)\((\d+)\):\d+: error: (.*)$', r)
        if not m:
            cls['UNLOCATED: ' + r] += 1; continue
        f, msg = m.group(1), m.group(3)
        u = re.match(r'unresolved symbol `([^`]*)` in operand', msg)
        if u:
            names[u.group(1)] += 1
        k = msg
        k = re.sub(r'cannot include (sound/[A-Za-z]+/generated/).*', r'cannot include \1<file>', k)
        k = re.sub(r'`[^`]*`', '`X`', k)
        k = re.sub(r'\b\d{3,}\b', 'N', k)
        k = re.sub(r'\d+ / 0', 'N / 0', k)
        cls[k] += 1; files[f] += 1; fc[(f, k)] += 1
    for k, v in cls.most_common():
        print('  %4d  %s' % (v, k))
    print('  -- by file')
    for k, v in sorted(files.items()):
        print('  %4d  %s' % (v, k))
    print('  -- by file and class')
    for (f, k), v in sorted(fc.items()):
        print('  %4d  %-22s %s' % (v, f, k))
    print('  -- unresolved-symbol names (%d rows, %d names)' % (sum(names.values()), len(names)))
    for k, v in sorted(names.items(), key=lambda x: (-x[1], x[0])):
        print('  %4d  %s' % (v, k))
