"""compare.py <reference> <candidate> [--control]
Whole-image byte compare, no windows. Reports the number of differing bytes and
the number of differing runs, with the first few runs shown. With --control a
copy of the candidate is mutated at three planted offsets first, and the run
must report exactly those three, which is the proof the comparer can see a
difference at all."""
import sys, zlib

def ident(p):
    d = open(p, 'rb').read()
    return d, '%08x' % (zlib.crc32(d) & 0xffffffff), len(d)

def diff(a, b):
    n = min(len(a), len(b)); runs = []; i = 0; total = 0
    while i < n:
        if a[i] != b[i]:
            j = i
            while j < n and a[j] != b[j]:
                j += 1
            runs.append((i, j)); total += j - i; i = j
        else:
            i += 1
    return total, runs

ref, cand = sys.argv[1], sys.argv[2]
a, ca, la = ident(ref); b, cb, lb = ident(cand)
print('REFERENCE %s crc32=%s size=%d' % (ref, ca, la))
print('CANDIDATE %s crc32=%s size=%d' % (cand, cb, lb))
print('SIZE %s' % ('EQUAL' if la == lb else 'DIFFER'))
total, runs = diff(a, b)
print('DIFF %d bytes in %d runs' % (total, len(runs)))
for (i, j) in runs[:8]:
    print('  [0x%06X,0x%06X) ref %s cand %s' % (i, j, a[i:j][:8].hex(' '), b[i:j][:8].hex(' ')))
if '--control' in sys.argv:
    planted = [0x200, la // 2, la - 2]
    m = bytearray(b)
    for o in planted:
        m[o] ^= 0xFF
    t2, r2 = diff(a, bytes(m))
    got = [hex(i) for (i, j) in r2]
    want = [hex(o) for o in planted]
    ok = t2 == total + len(planted) and all(w in got for w in want)
    print('CONTROL planted at %s -> %d bytes in %d runs, offsets %s' % (want, t2, len(r2), got[:8]))
    print('CONTROL %s' % ('PASSED: the comparer reports planted differences' if ok else 'FAILED'))
    sys.exit(0 if ok else 3)
