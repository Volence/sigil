#!/usr/bin/env python3
"""Region map of where sigil's (stubbed) S1 image diverges from the retail-identical
asl+p2bin build. Prints contiguous runs of difference, merged across gaps < 64 bytes."""
import sys

a = open("/home/volence/sonic_hacks/.s1recon-ref/s1built.bin", "rb").read()
b = open("/home/volence/sonic_hacks/.s1recon-out/s1stub.bin", "rb").read()
n = min(len(a), len(b))
print(f"ref {len(a)} bytes, sigil {len(b)} bytes, comparing {n}")

diff = [i for i in range(n) if a[i] != b[i]]
print(f"differing bytes in the common prefix: {len(diff)}  ({100.0*len(diff)/n:.1f}%)")
if not diff:
    sys.exit(0)

runs = []
start = prev = diff[0]
for i in diff[1:]:
    if i - prev > 64:
        runs.append((start, prev))
        start = i
    prev = i
runs.append((start, prev))
print(f"contiguous divergent regions (gap>64): {len(runs)}")
for s, e in runs[:60]:
    print(f"  ${s:06X}-${e:06X}  ({e - s + 1} bytes)")
if len(runs) > 60:
    print(f"  ... {len(runs) - 60} more")
