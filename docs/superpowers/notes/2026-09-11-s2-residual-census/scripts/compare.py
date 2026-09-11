"""compare.py <sigil-image> <reference-rom> <asl-listing> <z80blob> <p-file>

Windowed byte compare of a sigil image against the reference ROM.

The ONLY excluded window is the driver hole [Snd_Driver, Snd_Driver+guess):
the reference holds the Saxman stream there, and stub B moved sigil's driver
out of the image. Its bounds are DERIVED (Snd_Driver = end of the 68000 record
run that precedes the Z80 record in asl's object file; guess read from
s2.constants.asm), PRINTED, and ASSERTED against an expected extent, so the
exclusion cannot silently grow. No fill-byte window: p2bin runs with -p=0 and
sigil's AS route flattens with 0x00, so gaps compare directly.

The positive control runs first: a copy of the reference with planted byte
changes (outside and inside the window) must report exactly the outside ones.
"""
import re, struct, sys

img_p, ref_p, lst_p, blob_p, pfile_p = sys.argv[1:6]
img = open(img_p, 'rb').read()
ref = open(ref_p, 'rb').read()
blob = open(blob_p, 'rb').read()

# --- derive the window from asl's object file and the constants file
d = open(pfile_p, 'rb').read(); i = 2; recs = []
while i < len(d):
    h = d[i]; i += 1
    if h == 0: break
    if h == 0x80: i += 4; continue
    if h == 0x81: cpu = d[i]; i += 3
    else: cpu = h
    s, n = struct.unpack_from('<IH', d, i); i += 6; recs.append((cpu, s, n)); i += n
snd = None
for k in range(1, len(recs)):
    if recs[k][0] != 1 and recs[k][1] == 0 and recs[k - 1][0] == 1:
        snd = recs[k - 1][1] + recs[k - 1][2]
const = open('/home/volence/sonic_hacks/s2disasm/s2.constants.asm', encoding='latin-1').read()
guess = int(re.search(r'Size_of_Snd_driver_guess\s*=\s*\$([0-9A-Fa-f]+)', const).group(1), 16)
W = [(snd, snd + guess)]
print('WINDOW driver hole [0x%X, 0x%X) len 0x%X  (Snd_Driver from the p-file, guess from s2.constants.asm)' % (snd, snd + guess, guess))
assert W == [(0xEC0E8, 0xED04C)], W
print('WINDOW asserted: exactly one window, extent 0x%X bytes = %.3f%% of the reference' % (guess, 100.0 * guess / len(ref)))

def inwin(x):
    return any(a <= x < b for a, b in W)

def diffruns(a, b, lim):
    runs = []; cur = None
    for x in range(lim):
        if a[x] != b[x] and not inwin(x):
            if cur and cur[1] == x: cur[1] = x + 1
            else:
                cur = [x, x + 1]; runs.append(cur)
    return runs

# --- positive control on the instrument itself
plant = bytearray(ref)
for x in (0x200, 0x7FFF0, 0xFFFFE):
    plant[x] ^= 0xFF
plant[0xEC100] ^= 0xFF   # inside the window: must NOT be reported
pr = diffruns(plant, ref, len(ref))
print('CONTROL planted outside at 0x200,0x7FFF0,0xFFFFE and inside at 0xEC100 -> reported', [(hex(a), b - a) for a, b in pr])
assert [(a, b) for a, b in pr] == [(0x200, 0x201), (0x7FFF0, 0x7FFF1), (0xFFFFE, 0xFFFFF)], pr
print('CONTROL passed: the instrument reports planted differences and only the window hides one')

# --- listing: address -> source line, 68000 context only
lines = open(lst_p, encoding='latin-1').read().split('\n')
pat = re.compile(r'^(?:\((\d+)\))?\s*(\d+)/\s*([0-9A-F]+) : (.*)$')
z80_ranges = []
start = None
for k, l in enumerate(lines):
    if 'include "s2.sounddriver.asm"' in l and start is None: start = k
    if start is not None and '!org (Snd_Driver+Size_of_Snd_driver_guess)' in l:
        z80_ranges.append((start, k)); start = None
zs = None
for k, l in enumerate(lines):
    if 'phase 0 ; pretend we\'re at address 0' in l: zs = k
    if zs is not None and 'dephase' in l and k > zs:
        z80_ranges.append((zs, k)); zs = None; break
entries = []
for k, l in enumerate(lines):
    if any(a <= k <= b for a, b in z80_ranges):
        continue
    m = pat.match(l)
    if not m: continue
    rest = m.group(4)
    if not re.match(r'[0-9A-F]{2}', rest): continue
    entries.append((int(m.group(3), 16), int(m.group(2)), m.group(1) or '0', k + 1, l.strip()[:150]))
entries.sort(key=lambda e: e[0])
addrs = [e[0] for e in entries]
import bisect
def where(x):
    j = bisect.bisect_right(addrs, x) - 1
    return entries[j] if j >= 0 else None

# --- the real compare
print('image size %d (0x%X), reference %d (0x%X)' % (len(img), len(img), len(ref), len(ref)))
lim = min(len(img), len(ref))
win_img = img[snd:snd + guess]
print('sigil bytes inside the window: all zero =', set(win_img) <= {0})
runs = diffruns(img, ref, lim)
total = sum(b - a for a, b in runs)
print('DIFF outside the window: %d bytes in %d runs' % (total, len(runs)))
for a, b in runs:
    e = where(a)
    print('  [0x%06X,0x%06X) %4d B  sigil %s  ref %s' % (a, b, b - a, img[a:min(b, a + 12)].hex(' '), ref[a:min(b, a + 12)].hex(' ')))
    if e:
        print('      listing line %d, include depth %s, source line %d, addr 0x%X: %s' % (e[3], e[2], e[1], e[0], e[4]))
# --- the driver, uncompressed, against asl's own Z80 record
ZB = 0x300000
if len(img) >= ZB + len(blob):
    zi = img[ZB:ZB + len(blob)]
    zd = [x for x in range(len(blob)) if zi[x] != blob[x]]
    print('DRIVER at 0x%X, %d bytes vs asl Z80 record: %d bytes differ' % (ZB, len(blob), len(zd)))
    zr = []
    for x in zd:
        if zr and zr[-1][1] == x: zr[-1][1] = x + 1
        else: zr.append([x, x + 1])
    for a, b in zr:
        print('  z80 [0x%04X,0x%04X) sigil %s  asl %s' % (a, b, zi[a:b].hex(' ')[:48], blob[a:b].hex(' ')[:48]))
    tail = img[ZB + len(blob):]
    print('image bytes after the driver: %d, nonzero %d' % (len(tail), sum(1 for c in tail if c)))
else:
    print('DRIVER: image too short to hold the driver at 0x%X' % ZB)
