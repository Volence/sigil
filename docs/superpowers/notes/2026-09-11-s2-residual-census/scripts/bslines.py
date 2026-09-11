"""bslines.py : every string literal carrying a backslash in the s2disasm sources,
classified by the statement it sits in."""
import os, re
D = '/home/volence/sonic_hacks/s2disasm/'
files = ['s2.asm', 's2.constants.asm', 's2.macros.asm', 's2.macrosetup.asm',
         's2.sounddriver.asm', 'sound/_smps2asm_inc.asm']
cls = {}
for f in files:
    for n, l in enumerate(open(D + f, encoding='latin-1').read().split('\n'), 1):
        code = l
        # strip a trailing comment that is outside quotes
        out, q = '', None
        for ch in l:
            if q:
                out += ch
                if ch == q: q = None
            elif ch in '"\'':
                q = ch; out += ch
            elif ch == ';':
                break
            else:
                out += ch
        strs = re.findall(r'"([^"]*)"', out)
        if not any('\\' in s for s in strs):
            continue
        head = out.strip().split()
        word = next((w for w in head if not w.endswith(':')), '')
        wl = word.lower()
        if wl == 'charset':
            k = 'charset-mapping'
        elif wl in ('message', 'error', 'fatal', 'warning'):
            k = 'diagnostic-text:' + wl
        elif wl in ('dc.b', 'db'):
            k = 'EMITTING-' + wl
        else:
            k = 'other:' + word
        cls.setdefault(k, []).append('%s:%d: %s' % (f, n, l.strip()[:110]))
for k in sorted(cls):
    print('==', k, len(cls[k]))
    for x in cls[k]:
        print('   ', x)
