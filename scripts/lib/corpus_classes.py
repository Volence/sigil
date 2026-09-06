#!/usr/bin/env python3
"""Decompose sigil diagnostic streams into message CLASSES.

A class is the diagnostic's message text with every backtick-quoted name, every
single-quoted string, every double-quoted path and every number replaced by a
placeholder, so two reports of the same rule about different symbols land in one
row.

With one stream the output is a class table plus the population it was computed
over. With two streams the rows carry both counts and the delta, and a row
present in only one run is marked APPEARED or GONE: a falling total is noise
removal, never evidence that correctness improved.

Usage:
    corpus_classes.py <stream>
    corpus_classes.py <before> <after>
"""
import re
import sys

LOC = re.compile(r"^(.*)\(([0-9]+)\): (error|warning): (.*)$")


def norm(msg):
    msg = re.sub(r"`[^`]*`", "`X`", msg)
    msg = re.sub(r"'[^']*'", "'X'", msg)
    msg = re.sub(r'"[^"]*"', '"X"', msg)
    msg = re.sub(r"\bat offset [0-9]+", "at offset N", msg)
    msg = re.sub(r"\bsection [A-Za-z0-9_.]+", "section S", msg)
    msg = re.sub(r"\b[0-9]+\b", "N", msg)
    return msg.strip()


def load(path):
    """Return (class counts, lines read, lines that parsed as located)."""
    counts = {}
    read = 0
    located = 0
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.rstrip("\n")
        if not line:
            continue
        read += 1
        m = LOC.match(line)
        if m:
            located += 1
        msg = m.group(4) if m else line
        level = m.group(3) if m else "error"
        key = (level, norm(msg))
        counts[key] = counts.get(key, 0) + 1
    return counts, read, located


def one(path):
    counts, read, located = load(path)
    total = sum(counts.values())
    print("  population: %d non-empty line(s) read, %d parsed as file(line): level:"
          % (read, located))
    if read == 0:
        print("  REFUSED: the stream is EMPTY, so every count below would be a zero")
        print("           that says nothing. An empty diagnostic stream is either a")
        print("           clean run or a run that never happened; this classifier")
        print("           cannot tell those apart and does not report either as 0.")
        return 4
    print("  %-7s %7s  %s" % ("level", "count", "class"))
    for k in sorted(counts, key=lambda k: (-counts[k], k)):
        print("  %-7s %7d  %s" % (k[0], counts[k], k[1][:120]))
    print("  %-7s %7d  TOTAL" % ("", total))
    return 0


def two(before, after):
    b, bread, _ = load(before)
    a, aread, _ = load(after)
    print("  population: before %d non-empty line(s), after %d" % (bread, aread))
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
    return 0


def main():
    if len(sys.argv) == 2:
        return one(sys.argv[1])
    if len(sys.argv) == 3:
        return two(sys.argv[1], sys.argv[2])
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main())
