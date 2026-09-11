"""summarize.py <probes-dir> : one row per probe from the files run_probes.sh left.

Row: name | asl exit | asl first error | asl bytes per construct line | sigil exit | sigil hex
Only a probe whose asl run exited 0 has a byte column; a failed run prints
REFUSED and its first diagnostic, never bytes.
"""
import os, re, sys
D = sys.argv[1]
rows = []
for f in sorted(os.listdir(D)):
    if not (f.startswith('p_') and f.endswith('.asm')):
        continue
    b = f[:-4]
    out = open(os.path.join(D, b + '.asl.out'), encoding='latin-1').read()
    m = re.search(r'ASL_EXIT=(\d+)', out)
    rc = int(m.group(1)) if m else -1
    errs = re.findall(r'> > > [^\n]*?(error #\d+: [^\n]*|warning #\d+: [^\n]*)', out)
    msgs = [l for l in out.split('\n') if l and not l.startswith(('ASL_', 'REFUSED', '  ', '> > >'))]
    lst = os.path.join(D, b + '.lst')
    cols = []
    if rc == 0 and os.path.exists(lst):
        cur = None
        for l in open(lst, encoding='latin-1').read().split('\n'):
            mm = re.match(r'^\s*(\d+)/\s*([0-9A-F]+) : ((?:[0-9A-F]{2,4} ?)*)\s*(.*)$', l)
            mc = re.match(r'^\s+([0-9A-F]+) : ((?:[0-9A-F]{2,4} ?)+)\s*$', l)
            if mm and int(mm.group(1)) >= 3:
                hexs = mm.group(3).replace(' ', '')
                src = mm.group(4).strip()
                if src.startswith('dc.b $EE') or src.startswith('db 0EEh'):
                    break
                if hexs:
                    cur = [src, hexs]
                    cols.append(cur)
                else:
                    cur = None
            elif mc and cur is not None:
                cur[1] += mc.group(2).replace(' ', '')
    sig = open(os.path.join(D, b + '.sigil.out'), encoding='latin-1').read().strip().split('\n')
    sig_hex = sig[0] if sig else ''
    sig_ok = 'error' not in sig_hex
    rows.append((b[2:], rc, errs[0] if errs else '', cols, sig_hex if sig_ok else 'ERR ' + sig_hex[:90], msgs))
for name, rc, err, cols, sig, msgs in rows:
    if rc == 0:
        c = ' ; '.join('%s => %s' % (s[:40], ' '.join(h[i:i+2] for i in range(0, len(h), 2))) for s, h in cols)
        print('%-26s asl=0  %s' % (name, c))
        for mline in msgs:
            print('%-26s        asl-stdout: %s' % ('', mline[:100]))
    else:
        print('%-26s asl=%d REFUSED %s' % (name, rc, err[:70]))
    print('%-26s        sigil: %s' % ('', sig[:100]))
