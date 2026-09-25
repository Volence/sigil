#!/usr/bin/env python3
"""summ.py <probe log>: one row per probe: asl exit, sigil exit, verdict.
Verdicts: same (both 0, bytes equal), DIFFER, both-refuse, OVER-ACCEPT
(asl non-zero, sigil 0), OVER-REFUSE (asl 0, sigil non-zero)."""
import re, sys
rows, cur = [], None
for l in open(sys.argv[1]):
    l = l.rstrip("\n")
    m = re.match(r"=== (\S+)", l)
    if m:
        cur = {"n": m.group(1), "asl": None, "sig": None, "ab": "", "sb": "", "cmp": "", "aerr": "", "serr": ""}
        rows.append(cur); continue
    if cur is None: continue
    if l.startswith("ASL_EXIT="): cur["asl"] = int(l.split("=")[1])
    elif l.startswith("SIGIL_EXIT="): cur["sig"] = int(l.split("=")[1])
    elif l.startswith("asl   bytes: ") and cur["asl"] == 0: cur["ab"] = l[13:]
    elif l.startswith("sigil bytes: "): cur["sb"] = l[13:]
    elif l in ("MATCH", "DIFFER"): cur["cmp"] = l
    elif "error #" in l and not cur["aerr"]: cur["aerr"] = l.split("error ")[1][:60]
    elif re.search(r": error: ", l) and not cur["serr"]: cur["serr"] = l.split("error: ", 1)[1][:90]
for r in rows:
    if r["asl"] == 0 and r["sig"] == 0:
        v = "same" if r["cmp"] == "MATCH" else "DIFFER"
    elif r["asl"] != 0 and r["sig"] != 0:
        v = "both-refuse"
    elif r["asl"] != 0:
        v = "OVER-ACCEPT"
    else:
        v = "OVER-REFUSE"
    print(f"{r['n']}\tasl={r['asl']}\tsigil={r['sig']}\t{v}\t{r['ab'] if r['asl']==0 else r['aerr']}\t{r['sb'] if r['sig']==0 else r['serr']}")
