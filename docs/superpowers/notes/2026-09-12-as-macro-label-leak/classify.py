#!/usr/bin/env python3
"""Classify a matrix SUMMARY.tsv, one row per shape.

MATCH        both refuse, or both accept with identical bytes
LEAK         asl refuses, sigil accepts            (silent: builds here, wrong there)
OVER-REFUSE  asl accepts, sigil refuses            (loud)
VALUE-DIFF   both accept, bytes differ             (silent)
"""
import sys

def rows(path):
    with open(path) as f:
        lines = f.read().splitlines()
    head = lines[0]
    out = []
    for ln in lines[2:]:
        if ln.startswith("SHAPES_RUN") or ln.startswith("MATRIX_END"):
            continue
        c = ln.split("\t")
        name, arc, diag, aerr, ahex, src, serr, shex = c[:8]
        shex = shex.split("built:")[0].lower()
        a_ok, s_ok = arc == "0", src == "0"
        if a_ok and s_ok:
            cls = "MATCH" if ahex.lower() == shex else "VALUE-DIFF"
        elif not a_ok and not s_ok:
            cls = "MATCH"
        elif s_ok:
            cls = "LEAK"
        else:
            cls = "OVER-REFUSE"
        ae = aerr.split(": ", 1)[-1] if aerr != "-" else "-"
        se = serr.split("error: ", 1)[-1] if serr != "-" else "-"
        tail = lambda h: h[-16:] if h and h != "-" else "-"
        out.append((name, arc, ae, tail(ahex), src, se, tail(shex), cls))
    return head, out

head, out = rows(sys.argv[1])
print(head)
counts = {}
for r in out:
    counts[r[7]] = counts.get(r[7], 0) + 1
    print(f"{r[0]:<40} asl={r[1]} {r[2][:32]:<32} {r[3]:<16} | sigil={r[4]} {r[5][:44]:<44} {r[6]:<16} {r[7]}")
print("TOTAL", len(out), " ".join(f"{k}={v}" for k, v in sorted(counts.items())))
