"""saxdec.py <rom> <offset> <comp_size> <expected_blob> : decode the Saxman stream
the way s2.asm's own DecompressSoundDriver does (lines 90763-90849), and compare
the output with asl's assembled Z80 record.

Mirrors the 68000 routine exactly, including its exit rule: SaxDec_GetByte
decrements the remaining count AFTER reading, and when it reaches zero it pops
the return address and leaves, so the byte that made it zero is never used."""
import sys
rom = open(sys.argv[1], 'rb').read()
off = int(sys.argv[2], 0)
size = int(sys.argv[3], 0)
want = open(sys.argv[4], 'rb').read()

class Done(Exception):
    pass

src = rom[off:off + size]
pos = 0
remaining = size

def getbyte():
    global pos, remaining
    b = src[pos]; pos += 1
    remaining -= 1
    if remaining == 0:
        raise Done
    return b

out = bytearray()
d6 = 0
try:
    while True:
        d6 = (d6 >> 1) & 0xFFFF
        if not (d6 & 0x100):
            d6 = (getbyte() | 0xFF00)
        if d6 & 1:
            out.append(getbyte())
            continue
        lo = getbyte()
        hi = getbyte()
        d3 = (hi & 0xF) + 2
        d4 = (lo + ((hi & 0xF0) << 4) + 0x12) & 0xFFF
        d5 = len(out)
        d4 = (d4 + (d5 & 0xF000)) & 0xFFFF
        if d5 >= d4:
            dict_ref = True
        else:
            d4 -= 0x1000
            dict_ref = d4 >= 0
        if dict_ref:
            for _ in range(d3 + 1):
                out.append(out[d4]); d4 += 1
        else:
            out.extend(b'\x00' * (d3 + 1))
except Done:
    pass
print('decoded %d bytes from %d compressed bytes at 0x%X (consumed %d)' % (len(out), size, off, pos))
print('expected %d bytes' % len(want))
if bytes(out) == want:
    print('DECODE_EQUALS_ASL_Z80_RECORD=yes')
else:
    n = min(len(out), len(want))
    first = next((i for i in range(n) if out[i] != want[i]), n)
    print('DECODE_EQUALS_ASL_Z80_RECORD=NO first difference at %d (len out %d, want %d)' % (first, len(out), len(want)))
print('bytes after the stream up to 0xED100:', rom[off + size:0xED100].hex()[:64], '... all zero =', set(rom[off + size:0xED100]) <= {0}, 'len', 0xED100 - off - size)
