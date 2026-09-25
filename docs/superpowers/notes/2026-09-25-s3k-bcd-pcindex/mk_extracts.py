#!/usr/bin/env python3
"""mk_extracts.py: S3K's real sites, verbatim source lines from the census tree,
each phased at the address the reference build's listing gives it, plus the ROM
slice the reference build (buildSK.lua, md5 4ea493ea...) holds there."""
import os

S = "/home/volence/sonic_hacks/.scratch/s3k-bcd-pcindex"
SRC = S + "/trees/sk-gen/sonic3k.asm"
ROM = "/home/volence/sonic_hacks/.scratch/s3k-codepage/trees/sk-luaref/skbuilt.bin"
lines = open(SRC, encoding="latin-1").read().split("\n")
rom = open(ROM, "rb").read()

# (probe name, address from sonic3k.lst, first line, last line) ; 1-based, inclusive
EXTRACTS = [
    ("sk01", 0x8280C, 174875, 174926),  # both movem sites and the two velocity tables
    ("sk02", 0x2DE6A, 62886, 62888),    # abcd x3
    ("sk03", 0x4D252, 100959, 100959),  # subx.w d2,d0
    ("sk04", 0x5A036, 119898, 119898),  # subx.w d0,d2
    ("sk05", 0x5A05A, 119919, 119919),  # subx.w d0,d2
]
for name, addr, a, b in EXTRACTS:
    body = "\tcpu 68000\nx_vel = $18\ny_vel = $1A\n\tphase $%X\n" % addr
    body += "\n".join(lines[a - 1:b]) + "\n"
    with open(os.path.join(S, "probes", name + ".asm"), "w", encoding="latin-1") as f:
        f.write(body)
    print(name, "lines %d-%d at $%X" % (a, b, addr))

# The ROM slices, sized after the fact by the extract image length.
with open(os.path.join(S, "extract_addrs.txt"), "w") as f:
    for name, addr, a, b in EXTRACTS:
        f.write("%s %X\n" % (name, addr))
