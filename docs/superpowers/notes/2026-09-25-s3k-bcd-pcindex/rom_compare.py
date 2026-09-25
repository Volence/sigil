#!/usr/bin/env python3
"""rom_compare.py: for each S3K extract, compare the asl image and the sigil image
(each assembled with phase at the real address, emitted from 0 by p2bin/sigil) to
the reference ROM slice at that address, of the same length."""
import os
import zlib

S = "/home/volence/sonic_hacks/.scratch/s3k-bcd-pcindex"
ROM = "/home/volence/sonic_hacks/.scratch/s3k-codepage/trees/sk-luaref/skbuilt.bin"
rom = open(ROM, "rb").read()
print("rom crc32 %08x size %d" % (zlib.crc32(rom) & 0xFFFFFFFF, len(rom)))
for line in open(os.path.join(S, "extract_addrs.txt")):
    name, addr = line.split()
    addr = int(addr, 16)
    out = []
    for side in ("asl", "sigil"):
        p = os.path.join(S, "probes", "%s.%s.bin" % (name, side))
        if not os.path.exists(p):
            out.append("%s: (no image)" % side)
            continue
        img = open(p, "rb").read()
        sl = rom[addr:addr + len(img)]
        out.append("%s %d bytes %s rom" % (side, len(img), "==" if img == sl else "!="))
    print(name, "$%X" % addr, rom[addr:addr + 12].hex(), "|", "; ".join(out))
