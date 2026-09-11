"""check_probes.py <probes-dir> : every probe's sigil answer against asl's.

asl exit 0: the expected bytes are EVERY byte in the listing (continuation lines
included), sentinel too, in listing order; sigil's --hex first line must equal
them. For a probe whose construct is `message`/`warning` text, the text is
compared instead (asl stdout / warning line against sigil's).
asl non-zero: no byte is a value; sigil must refuse too (exit non-zero), or the
row is reported ACCEPTS-WHAT-ASL-REFUSES.
"""
import os, re, sys
D = sys.argv[1]
res = {}
for f in sorted(os.listdir(D)):
    if not (f.startswith('p_') and f.endswith('.asm')):
        continue
    b = f[:-4]
    out = open(os.path.join(D, b + '.asl.out'), encoding='latin-1').read()
    rc = int(re.search(r'ASL_EXIT=(\d+)', out).group(1))
    sig = open(os.path.join(D, b + '.sigil.out'), encoding='latin-1').read()
    sig_lines = sig.strip().split('\n')
    sig_err = bool(re.search(r'error', sig)) and not re.match(r'^[0-9A-F ]+$', sig_lines[0])
    src = open(os.path.join(D, f), encoding='latin-1').read()
    if rc != 0:
        verdict = 'BOTH-REFUSE' if sig_err else 'ACCEPTS-WHAT-ASL-REFUSES'
        res[b] = (verdict, '', sig_lines[0][:80])
        continue
    if '\tmessage ' in src or '\twarning ' in src:
        if '\tmessage ' in src:
            exp = [l for l in out.split('\n') if l and not l.startswith(('ASL_', '> > >', 'REFUSED', '  '))]
            got = [l for l in sig_lines if not re.match(r'^[0-9A-F ]+$', l) and not l.startswith('built:')]
        else:
            exp = [m.group(1) for m in re.finditer(r'warning: (.*)', out)]
            got = [m.group(1) for m in re.finditer(r'\[as\.warning\] (.*)', sig)]
        verdict = 'MATCH' if exp == got else 'MISMATCH'
        res[b] = (verdict, ' | '.join(exp), ' | '.join(got))
        continue
    hexs = []
    for l in open(os.path.join(D, b + '.lst'), encoding='latin-1').read().split('\n'):
        mm = re.match(r'^\s*\d+/\s*[0-9A-F]+ : ((?:[0-9A-F]{2,4} ?)*)', l)
        mc = re.match(r'^\s+[0-9A-F]+ : ((?:[0-9A-F]{2,4} ?)+)\s*$', l)
        g = (mm or mc)
        if g and g.group(1).strip():
            h = g.group(1).replace(' ', '')
            hexs += [h[i:i + 2] for i in range(0, len(h), 2)]
    exp = ' '.join(hexs)
    got = sig_lines[0] if not sig_err else 'ERR ' + sig_lines[0]
    res[b] = ('MATCH' if got == exp else 'MISMATCH', exp, got[:80])
from collections import Counter
c = Counter(v[0] for v in res.values())
for k, (v, e, g) in res.items():
    if v not in ('MATCH', 'BOTH-REFUSE'):
        print('%-28s %-26s asl=[%s] sigil=[%s]' % (k[2:], v, e, g))
print('TOTALS', dict(sorted(c.items())), 'of', len(res))
