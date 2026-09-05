#!/usr/bin/env python3
"""Decompose two sigil diagnostic streams into message CLASSES and diff them.

A class is the diagnostic's message text with every backtick-quoted name, every
number and every quoted path replaced by a placeholder, so two reports of the
same rule about different symbols land in one row. Rows are printed with both
counts and the delta; a row present in only one run is marked APPEARED or GONE,
which is the reading the parcel actually needs (a falling total is noise
removal, not correctness).
"""
import re
import sys

LOC = re.compile(r"^(.*)\(([0-9]+)\): (error|warning): (.*)$")


def norm(msg):
    msg = re.sub(r"`[^`]*`", "`X`", msg)
    msg = re.sub(r"'[^']*'", "'X'", msg)
    msg = re.sub(r"\bat offset [0-9]+", "at offset N", msg)
    msg = re.sub(r"\bsection [A-Za-z0-9_.]+", "section S", msg)
    msg = re.sub(r"\b[0-9]+\b", "N", msg)
    return msg.strip()


def load(path):
    counts = {}
    lines = []
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.rstrip("\n")
        if not line:
            continue
        m = LOC.match(line)
        msg = m.group(4) if m else line
        level = m.group(3) if m else "error"
        key = (level, norm(msg))
        counts[key] = counts.get(key, 0) + 1
        lines.append((key, line))
    return counts, lines


def main():
    b, blines = load(sys.argv[1])
    a, alines = load(sys.argv[2])
    keys = sorted(set(b) | set(a), key=lambda k: (-max(b.get(k, 0), a.get(k, 0)), k))
    print("  %-7s %7s %7s %7s  %s" % ("level", "before", "after", "delta", "class"))
    for k in keys:
        bc, ac = b.get(k, 0), a.get(k, 0)
        mark = ""
        if bc == 0:
            mark = "  <== APPEARED"
        elif ac == 0:
            mark = "  <== GONE"
        elif ac > bc:
            mark = "  <== ROSE"
        print("  %-7s %7d %7d %+7d  %s%s" % (k[0], bc, ac, ac - bc, k[1][:110], mark))
    print("  %-7s %7d %7d %+7d  TOTAL" % ("", sum(b.values()), sum(a.values()),
                                          sum(a.values()) - sum(b.values())))
    # One concrete example per APPEARED/ROSE class, so a row is readable.
    amap = {}
    for k, line in alines:
        amap.setdefault(k, line)
    print()
    print("  examples for APPEARED / ROSE rows:")
    any_shown = False
    for k in keys:
        bc, ac = b.get(k, 0), a.get(k, 0)
        if ac > bc:
            any_shown = True
            print("    %s" % amap[k][:200])
    if not any_shown:
        print("    (none)")


if __name__ == "__main__":
    main()
