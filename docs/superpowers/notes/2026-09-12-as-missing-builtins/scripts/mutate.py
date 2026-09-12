#!/usr/bin/env python3
"""mutate.py <file> <old.txt> <new.txt> : replace the old text with the new,
refusing unless the old text occurs exactly once, then quote the mutated lines
back FROM DISK (re-read after the write), with their line numbers."""
import sys

path, old_p, new_p = sys.argv[1:4]
src = open(path).read()
old = open(old_p).read()
new = open(new_p).read()
n = src.count(old)
if n != 1:
    print(f"REFUSED: old text occurs {n} times in {path}")
    sys.exit(5)
open(path, "w").write(src.replace(old, new))
disk = open(path).read()
at = disk.index(new)
first = disk[:at].count("\n") + 1
lines = disk.split("\n")
print(f"MUTATED {path}, lines {first}..{first + new.count(chr(10))} now read:")
for k in range(first, first + max(1, new.rstrip(chr(10)).count("\n") + 1)):
    print(f"  {k}: {lines[k - 1]}")
