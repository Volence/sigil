#!/usr/bin/env python3
"""mutate.py <file> <old-text-file> <new-text-file>

Replace the ONE occurrence of old text with new text, then quote the mutated
lines back FROM DISK so the log shows the patch landed. Refuses (exit 3) when
the old text occurs zero times or more than once.
"""
import sys

path, oldf, newf = sys.argv[1:4]
old = open(oldf).read()
new = open(newf).read()
src = open(path).read()
n = src.count(old)
if n != 1:
    print(f"MUTATION REFUSED: old text occurs {n} times in {path}")
    sys.exit(3)
src = src.replace(old, new)
open(path, "w").write(src)
back = open(path).read()
if new not in back or old in back and old != new:
    print("MUTATION DID NOT LAND")
    sys.exit(4)
start = back.index(new)
line0 = back[:start].count("\n") + 1
nlines = new.count("\n") + (0 if new.endswith("\n") else 1)
lines = back.split("\n")
print(f"APPLIED to {path}; mutated region read back from disk:")
for i in range(line0, line0 + max(nlines, 1)):
    print(f"  {path}:{i}  {lines[i-1]}")
