#!/usr/bin/env python3
"""Measure the 68000 abs.w ceiling margin in each shipped ROM shape.

WHY THIS IS A FILE AND NOT A ROW. `ABSW-CEILING-INVARIANT` rotted three times while its
measurement lived in prose: a stored coordinate went stale twice, and the replacement --
`grep -nE 'SoundTablesZ80_Head|Sound_PlaySFX' s4.lst` -- reads a symbol whose 0x8000 is a
Z80 section VMA (`sound_tables_z80.emp:26`), a different address space from the rule it
claims to witness. It got the right number for the wrong reason, and its stated safe
direction ended up inverted.

WHAT IT MEASURES, so nobody has to trust a sentence: it decodes the four absolute transfer
encodings -- 4EB8/4EB9 jsr, 4EF8/4EF9 jmp -- out of the ROM bytes and reports, per shape,
the highest abs.w target (how close code has come to the 0x8000 ceiling) and any abs.l
target inside [0x8000, 0x10000) (whether the boundary cuts through live code).

READING IT. A shape with no abs.l in that band is ONE-SIDED: growth pushes its top abs.w
symbol over the ceiling and widens every site reaching it, while shrink has no exposure at
all. A shape that straddles has code on both sides and a byte-identity gate over it is
asserting the width decision in both directions.

ITS LIMIT, stated because the scan is naive: a byte pair inside data can look like one of
these opcodes. Validate a boundary address against the listing before acting on it -- a
real transfer target lands on a named symbol, and all three current ones do.
"""
import sys, zlib, struct, os
# 68000 absolute jmp/jsr: 4EB8 jsr.w, 4EB9 jsr.l, 4EF8 jmp.w, 4EF9 jmp.l
OPS = {0x4EB8:('jsr','w'), 0x4EB9:('jsr','l'), 0x4EF8:('jmp','w'), 0x4EF9:('jmp','l')}
def scan(p):
    d = open(p,'rb').read()
    w_t, l_t = [], []
    i = 0
    while i < len(d)-5:
        op = int.from_bytes(d[i:i+2],'big')
        k = OPS.get(op)
        if k:
            if k[1]=='w':
                t = int.from_bytes(d[i+2:i+4],'big')
                if t & 0x8000: t -= 0x10000          # sign-extended abs.w
                w_t.append(t); i += 4; continue
            else:
                t = int.from_bytes(d[i+2:i+6],'big')
                l_t.append(t); i += 6; continue
        i += 2
    return d, w_t, l_t
for p in sys.argv[1:]:
    if not os.path.exists(p): print(f"{os.path.basename(p):16} ABSENT"); continue
    d,w,l = scan(p)
    crc = format(zlib.crc32(d)&0xffffffff,'08x')
    maxw = max([t for t in w if t>=0], default=None)
    band = sorted(t for t in l if 0x8000 <= t < 0x10000)
    print(f"{os.path.basename(p):16} crc {crc} {len(d):>7}B  "
          f"max abs.w {maxw:#07x}  slack {0x8000-maxw:>6}B  "
          f"abs.l in [0x8000,0x10000): {len(band)}"
          + (f" min {band[0]:#07x}  STRADDLES" if band else "  none"))
