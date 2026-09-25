#!/usr/bin/env python3
"""diffranges.py <a> <b>: differing byte ranges (count, offset, size, bytes of each)."""
import sys
a = open(sys.argv[1], "rb").read()
b = open(sys.argv[2], "rb").read()
runs, start = [], None
for i in range(min(len(a), len(b))):
    if a[i] != b[i]:
        if start is None:
            start = i
    elif start is not None:
        runs.append((start, i - start)); start = None
if start is not None:
    runs.append((start, min(len(a), len(b)) - start))
print(f"ranges: {len(runs)}, bytes: {sum(n for _, n in runs)}, lengths {len(a)} vs {len(b)}")
for off, n in runs[:40]:
    print(f"  0x{off:06X} +{n}: {a[off:off+n].hex()} vs {b[off:off+n].hex()}")
