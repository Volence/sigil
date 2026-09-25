#!/usr/bin/env python3
"""header.py <image>: what fix_header would write (end-of-ROM at 0x1A4, the 16-bit sum
of big-endian words from 0x200 at 0x18E) against what the image holds. Then a liveness
control: plant one word after 0x200 in a copy and show the required checksum moves."""
import sys, struct
d = bytearray(open(sys.argv[1], "rb").read())

def want(b):
    s = 0
    for i in range(0x200, len(b) - 1, 2):
        s += (b[i] << 8) | b[i + 1]
    if (len(b) - 0x200) % 2:
        s += b[-1] << 8
    return s & 0xFFFF, len(b) - 1

cks, end = want(d)
have_cks = struct.unpack(">H", d[0x18E:0x190])[0]
have_end = struct.unpack(">I", d[0x1A4:0x1A8])[0]
print(f"checksum at 0x18E: image {have_cks:04X}, fix_header computes {cks:04X}: {'same' if cks == have_cks else 'DIFFERS'}")
print(f"end-of-ROM at 0x1A4: image {have_end:08X}, fix_header computes {end:08X}: {'same' if end == have_end else 'DIFFERS'}")
p = bytearray(d)
p[0x10000] ^= 0x01
cks2, _ = want(p)
print(f"control: one bit flipped at 0x10000, required checksum becomes {cks2:04X} ({'moves' if cks2 != cks else 'DOES NOT MOVE: control failed'})")
