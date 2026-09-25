#!/usr/bin/env python3
"""md_table.py <run.tsv>: render a run_probes.sh table as markdown, one row per
probe, with the probe's description from probes/INDEX.tsv."""
import os, sys
here = os.path.dirname(os.path.abspath(__file__))
desc = dict(l.rstrip("\n").split("\t", 1) for l in open(os.path.join(here, "probes", "INDEX.tsv")))
print("| probe | shape | asl | sigil | verdict |")
print("|---|---|---|---|---|")
for line in open(sys.argv[1]):
    n, arc, amsg, ab, src, smsg, sb, v, _w = line.rstrip("\n").split("\t")
    a = f"exit {arc}, `` {amsg} ``" if arc != "0" else f"exit 0, `{ab}`"
    s = smsg.split(": error: ", 1)[-1] if src != "0" else sb
    s = f"exit {src}, `` {s[:90]} ``" if src != "0" else f"exit 0, `{s}`"
    print(f"| `{n}` | {desc.get(n, '')} | {a} | {s} | {v} |")
