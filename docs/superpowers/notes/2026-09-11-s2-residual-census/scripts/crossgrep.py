"""crossgrep.py : do Sonic 1 and Sonic 3&K write the constructs behind the S2
residual? Counts code (comment-stripped) lines per construct per corpus, over
every .asm under each tree except .git. Includes s2disasm as the positive
control: every construct must be found there at least once."""
import os, re
PAT = {
    'lastbit(': re.compile(r'\blastbit\s*\(', re.I),
    'pushv/popv': re.compile(r'^\s*(pushv|popv)\b', re.I),
    'shared': re.compile(r'^\s*shared\s', re.I),
    'ix/iy half register': re.compile(r'\b(ixl|ixu|iyl|iyu|ixh|iyh)\b', re.I),
    'backslash escape in a string (not \\{})': re.compile(r'"[^"]*\\[xX0-9AaHh][^"]*"'),
    'charset with a string': re.compile(r'^\s*charset\s+[^;]*"', re.I),
    'memory shift, no size': re.compile(r'^\s+(asl|asr|lsl|lsr|rol|ror|roxl|roxr)\s+[^,#;]*\(', re.I),
    'digit-led struct member': re.compile(r'^\s+[0-9][A-Za-z_]\w*:'),
    'empty parens ()': re.compile(r'\(\s*\)'),
    'music_metadata-style (FLAGS)': re.compile(r'\(FLAGS\)'),
    'CONTROL move.w (must be >0 in every tree)': re.compile(r'\bmove\.w\b', re.I),
    'CONTROL charset directive of any form': re.compile(r'^\s*charset\b', re.I),
}
def strip(l):
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
    return out
for tree in ('s2disasm', 's1disasm', 'skdisasm'):
    root = '/home/volence/sonic_hacks/' + tree
    cnt = {k: 0 for k in PAT}; ex = {k: None for k in PAT}
    for dp, dn, fn in os.walk(root):
        dn[:] = [d for d in dn if d != '.git']
        for f in fn:
            if not f.lower().endswith('.asm'): continue
            p = os.path.join(dp, f)
            for n, l in enumerate(open(p, encoding='latin-1', errors='replace'), 1):
                c = strip(l.rstrip('\n'))
                for k, r in PAT.items():
                    if r.search(c):
                        cnt[k] += 1
                        if ex[k] is None: ex[k] = '%s:%d: %s' % (os.path.relpath(p, root), n, c.strip()[:80])
    print('==', tree)
    for k in PAT:
        print('  %5d  %-42s %s' % (cnt[k], k, ex[k] or ''))
