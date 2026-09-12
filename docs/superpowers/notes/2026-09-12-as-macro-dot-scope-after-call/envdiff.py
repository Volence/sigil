#!/usr/bin/env python3
"""Diff the LAST pass's symbol environment of two SIGIL_DUMP_ENV runs.
   envdiff.py <prefix-a> <prefix-b>
Prints each run's pass files, the key counts, keys only in one side, keys whose
value differs, and a total, so an empty diff is a printed zero over printed
populations."""
import glob, re, sys


def last(prefix):
    files = glob.glob(prefix + ".pass*")
    if not files:
        sys.exit(f"no dump files for {prefix}")
    files.sort(key=lambda f: int(re.search(r"\.pass(-?\d+)$", f).group(1)))
    return files, files[-1]


def load(path):
    out = {}
    for ln in open(path):
        k, v = ln.rstrip("\n").split("\t", 1)
        out[k] = v
    return out


fa, la = last(sys.argv[1])
fb, lb = last(sys.argv[2])
print("A passes:", [f.rsplit(".", 1)[1] for f in fa], "last", la)
print("B passes:", [f.rsplit(".", 1)[1] for f in fb], "last", lb)
a, b = load(la), load(lb)
only_a = sorted(set(a) - set(b))
only_b = sorted(set(b) - set(a))
diff = sorted(k for k in set(a) & set(b) if a[k] != b[k])
print(f"keys A={len(a)} B={len(b)} only-A={len(only_a)} only-B={len(only_b)} value-differs={len(diff)}")
for k in only_a[:40]:
    print("  only-A", k, a[k])
for k in only_b[:40]:
    print("  only-B", k, b[k])
for k in diff[:40]:
    print("  differs", k, a[k], "->", b[k])
