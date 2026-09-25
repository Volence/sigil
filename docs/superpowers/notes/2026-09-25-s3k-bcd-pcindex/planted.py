#!/usr/bin/env python3
"""planted.py: positive control for the byte comparisons. Flip one byte of a copy of
each sigil image (the movem displacement byte of sk01, the abcd register byte of
sk02, the index byte of pcx01) and require every comparison used as evidence to see it."""
import filecmp
import shutil
S = "/home/volence/sonic_hacks/.scratch/s3k-bcd-pcindex/probes/"
ROM = open("/home/volence/sonic_hacks/.scratch/s3k-codepage/trees/sk-luaref/skbuilt.bin", "rb").read()
for name, off, base in (("sk01", 5, 0x8280C), ("sk02", 1, 0x2DE6A), ("pcx01", 2, None)):
    src = S + name + ".sigil.bin"
    dst = S + name + ".planted.bin"
    shutil.copy(src, dst)
    b = bytearray(open(dst, "rb").read())
    b[off] ^= 0x01
    open(dst, "wb").write(b)
    asl_same = filecmp.cmp(S + name + ".asl.bin", dst, shallow=False)
    rom_same = None if base is None else bytes(b) == ROM[base:base + len(b)]
    unplanted = filecmp.cmp(S + name + ".asl.bin", src, shallow=False)
    print("%s byte %d flipped: planted==asl %s, planted==rom %s; unplanted==asl %s"
          % (name, off, asl_same, rom_same, unplanted))
