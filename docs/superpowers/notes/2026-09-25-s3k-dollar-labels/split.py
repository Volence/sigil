#!/usr/bin/env python3
"""split.py <bundle>: write each `### name` section of <bundle> to probes/<name>.asm."""
import os, sys

out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "probes")
name, lines = None, []
def flush():
    if name:
        with open(os.path.join(out, name + ".asm"), "w") as f:
            f.write("".join(lines))
for l in open(sys.argv[1]):
    if l.startswith("### "):
        flush()
        name, lines = l[4:].strip(), []
    else:
        lines.append(l)
flush()
