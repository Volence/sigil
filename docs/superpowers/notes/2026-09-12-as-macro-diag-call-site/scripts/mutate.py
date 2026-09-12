#!/usr/bin/env python3
"""mutate.py FILE OLD NEW : replace OLD with NEW in FILE.

Refuses unless OLD occurs exactly once. After writing, reads the file back from
disk and proves the mutation landed: OLD no longer occurs, NEW occurs, and the
mutated line(s) are quoted with their line numbers."""
import sys

path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
src = open(path).read()
n = src.count(old)
if n != 1:
    print(f"REFUSED: `{old}` occurs {n} times in {path}")
    sys.exit(1)
at = src.index(old)
open(path, "w").write(src.replace(old, new))
disk = open(path).read()
assert old not in disk or old in new, "the old text is still on disk"
assert new in disk, "the new text is not on disk"
first = disk[:at].count("\n") + 1
lines = disk.splitlines()
span = max(1, new.count("\n") + 1)
print(f"MUTATED {path}")
for i in range(first, first + span):
    print(f"  {i}: {lines[i - 1]}")
