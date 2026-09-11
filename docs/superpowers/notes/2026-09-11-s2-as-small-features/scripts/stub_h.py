#!/usr/bin/env python3
"""stub_h.py <src-corpus> <dst-corpus>

MEASUREMENT SCAFFOLD. The census's stub B with every class THIS parcel
implements left in its original spelling: only the Z80 half-register lines
(census class 3, asl's own listed bytes) and the driver placement (census
stub B's S10: `!org $300000` + `phase 0` + a trailing `dephase`) are rewritten.
Every replacement asserts the original text; the line count of s2.asm is
unchanged, the driver file gains the one appended `dephase`.
"""
import os, shutil, sys

src, dst = sys.argv[1], sys.argv[2]
if os.path.exists(dst):
    shutil.rmtree(dst)
shutil.copytree(src, dst, symlinks=True)
log = []

HALF = {1863: ('\tld\ta,iyl', '0FDh,7Dh'), 1866: ('\tadc\ta,iyu', '0FDh,8Ch'),
        1976: ('\tld\ta,iyl', '0FDh,7Dh'), 1979: ('\tadc\ta,iyu', '0FDh,8Ch'),
        2258: ('\tld\te,ixl', '0DDh,5Dh'), 2259: ('\tld\td,ixu', '0DDh,54h'),
        3475: ('\tld\ta,ixl', '0DDh,7Dh'), 3478: ('\tadc\ta,ixu', '0DDh,8Ch'),
        3650: ('\tld\te,ixl', '0DDh,5Dh'), 3651: ('\tld\td,ixu', '0DDh,54h'),
        3687: ('\tadd\ta,ixl', '0DDh,85h'), 3689: ('\tadc\ta,ixu', '0DDh,8Ch')}

p = os.path.join(dst, 's2.sounddriver.asm')
lines = open(p, encoding='latin-1').read().split('\n')
for n, (prefix, bytes_) in HALF.items():
    old = lines[n - 1]
    assert old.startswith(prefix), (n, old)
    lines[n - 1] = '\tdb\t%s\t; stub for:%s' % (bytes_, prefix.replace('\t', ' '))
    log.append('S7-ixiy %d: %r -> %r' % (n, old, lines[n - 1]))
old = '    !org 0 ; Z80 code starting at address 0 has special meaning to s2p2bin.exe'
assert lines[248 - 1] == old, lines[248 - 1]
lines[248 - 1] = '    !org $300000 ; STUB-H: driver outside the ROM image'
assert lines[249 - 1] == '', lines[249 - 1]
lines[249 - 1] = '    phase 0 ; STUB-H'
lines.append('\tdephase ; STUB-H')
log.append('S10-placement 248/249 + EOF dephase')
open(p, 'w', encoding='latin-1').write('\n'.join(lines))
open(os.path.join(os.path.dirname(dst), 'stub-H.log'), 'w').write('\n'.join(log) + '\n')
print('stub H edits', len(log))
