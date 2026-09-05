#!/usr/bin/env python3
"""Census of every `if`/`elseif` CONDITION in a corpus tree.

The refusal this parcel adds fires on one thing only: an `if`/`elseif` whose
condition still does not fold to a number after the assembler has converged.
So the population that matters is not "how many `if` lines exist" but "what
SHAPES do their conditions take", because a shape the evaluator cannot fold at
all would be refused where it is silently false today.

Classes, in the order they are tested:

  ifdef/ifndef  the definedness directives, which never fold a number and are
                untouched by the refusal
  strcmp        a condition with a quoted string on either side (the
                `MOMCPUNAME="Z80"` family), folded as a string comparison
                before any numeric fold is attempted
  empty         no condition tokens at all
  numeric       everything else: the population the refusal can reach

Run: if_census.py <tree> [<tree> ...]
"""
import os
import re
import sys

# The mnemonic column of an AS line: optional label, then the directive.
COND = re.compile(r"^(?:[A-Za-z_.@][\w.@]*:?)?\s+(if|elseif|ifdef|ifndef)\b(.*)$", re.I)
COL0 = re.compile(r"^(if|elseif|ifdef|ifndef)\b(.*)$", re.I)


def strip_comment(s):
    out = []
    in_str = False
    for ch in s:
        if ch == '"':
            in_str = not in_str
        if ch == ";" and not in_str:
            break
        out.append(ch)
    return "".join(out)


def classify(kw, arg):
    kw = kw.lower()
    if kw in ("ifdef", "ifndef"):
        return "ifdef/ifndef"
    arg = strip_comment(arg).strip()
    if not arg:
        return "empty"
    if '"' in arg or "'" in arg:
        return "strcmp"
    return "numeric"


def walk(tree):
    counts = {}
    samples = {}
    files = 0
    for root, dirs, names in os.walk(tree):
        dirs[:] = [d for d in dirs if d != ".git"]
        for n in names:
            if not n.lower().endswith((".asm", ".inc", ".s")):
                continue
            files += 1
            p = os.path.join(root, n)
            with open(p, encoding="utf-8", errors="replace") as fh:
                for ln, line in enumerate(fh, 1):
                    line = line.rstrip("\n")
                    m = COND.match(line) or COL0.match(line)
                    if not m:
                        continue
                    cls = classify(m.group(1), m.group(2))
                    counts[cls] = counts.get(cls, 0) + 1
                    samples.setdefault(cls, []).append(
                        "%s:%d: %s" % (os.path.relpath(p, tree), ln, line.strip())
                    )
    return files, counts, samples


def main():
    for tree in sys.argv[1:]:
        files, counts, samples = walk(tree)
        total = sum(counts.values())
        print("== %s ==" % tree)
        print("   source files scanned: %d" % files)
        print("   if/elseif/ifdef/ifndef lines: %d" % total)
        for cls in ("numeric", "strcmp", "ifdef/ifndef", "empty"):
            print("     %-14s %d" % (cls, counts.get(cls, 0)))
        for cls in ("strcmp", "empty"):
            for s in samples.get(cls, [])[:12]:
                print("       [%s] %s" % (cls, s))
        print()


if __name__ == "__main__":
    main()
