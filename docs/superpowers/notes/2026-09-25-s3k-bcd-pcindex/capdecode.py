#!/usr/bin/env python3
"""capdecode.py <name>...: decode probes/<name>.sigil.bin sequentially with capstone
(CS_ARCH_M68K, CS_MODE_M68K_000), one row per instruction, next to the source line
and asl's bytes at the same offset. Stops at the first label-only data line."""
import sys
from capstone import Cs, CS_ARCH_M68K, CS_MODE_M68K_000, __version__

S = "/home/volence/sonic_hacks/.scratch/s3k-bcd-pcindex/probes/"
md = Cs(CS_ARCH_M68K, CS_MODE_M68K_000)
print("capstone", __version__)
for name in sys.argv[1:]:
    sig = open(S + name + ".sigil.bin", "rb").read()
    asl = open(S + name + ".asl.bin", "rb").read()
    src = [l.strip() for l in open(S + name + ".asm") if l.strip() and not l.strip().startswith("cpu")]
    print("##", name, "sigil==asl:", sig == asl)
    off = 0
    for line in src:
        if "dc.w" in line:
            # data: skip its bytes (count words in the dc.w list)
            n = 2 * len(line.split("dc.w", 1)[1].split(";")[0].split(","))
            off += n
            continue
        if line.endswith(":") or line.startswith("phase") or "=" in line.split(";")[0]:
            continue
        ins = next(md.disasm(sig[off:off + 14], off), None)
        if ins is None:
            print("| `%s` | %s | (capstone: no decode) |" % (line, sig[off:off + 2].hex()))
            off += 2
            continue
        b = sig[off:off + ins.size]
        print("| `%s` | %s | %s | `%s %s` |" % (line, asl[off:off + ins.size].hex(), b.hex(), ins.mnemonic, ins.op_str))
        off += ins.size
