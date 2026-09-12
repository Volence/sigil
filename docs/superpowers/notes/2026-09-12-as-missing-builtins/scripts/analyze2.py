#!/usr/bin/env python3
"""analyze2.py : round 2. For each t_* table: asl exit/diag, and a JSON map
source-line -> listing bytes (only written when the run exited 0). For each o_*
probe: asl exit, construct bytes, first error; sigil result."""
import json, os, re

D = "/home/volence/sonic_hacks/.scratch/as-missing-builtins/probes2"


def asl_info(b):
    out = open(b + ".asl.out", errors="replace").read()
    rc = int(re.search(r"ASL_EXIT=(\d+)", out).group(1))
    diag = (re.search(r"ASL_DIAG=(\w+)", out) or [None, "?"])[1]
    errs = re.findall(r"(?:error|warning) #\d+: [^\n]*", out)
    rows = {}
    if os.path.exists(b + ".lst"):
        for l in open(b + ".lst", errors="replace"):
            m = re.match(r"^ +(\d+)/ +([0-9A-F]+) : (.{20})", l.rstrip("\n"))
            if m and m.group(3).strip():
                rows.setdefault(int(m.group(1)), []).append(m.group(3).strip())
    return rc, diag, errs, rows


def sigil_info(b):
    t = open(b + ".sigil.out", errors="replace").read()
    errs = [l for l in t.splitlines() if "error" in l]
    hexs = "".join(l.strip() for l in t.splitlines() if "error" not in l and "warning" not in l)
    return ("ERR " + errs[0].split(": error: ", 1)[-1][:70]) if errs else hexs[:60]


tables = {}
for f in sorted(os.listdir(D)):
    if not f.endswith(".asm"):
        continue
    b = os.path.join(D, f[:-4])
    rc, diag, errs, rows = asl_info(b)
    src = open(b + ".asm").read().split("\n")
    if f.startswith("t_"):
        print(f"{f:22} asl exit={rc} {diag} errors={len(errs)} rows={len(rows)} {errs[:1]}")
        if rc == 0:
            tables[f[:-4]] = {src[n - 1].strip(): " ".join(v) for n, v in rows.items()
                              if not re.match(r"\s*(cpu|padding|org|end|dc\.b \$EE)", src[n - 1])}
    else:
        n = next((i for i, l in enumerate(src, 1) if l.strip() and not re.match(
            r"\s*(cpu|padding|org|end\b|dc\.b \$EE|db 0EEh|Q equ|X equ|Later equ)", l)), None)
        print(f"{f[:-4]:26} asl exit={rc} {diag[:4]} [{' | '.join(rows.get(n, []))}] {'; '.join(errs[:2])[:90]}")
        print(f"{'':26}   sigil {sigil_info(b)}")
json.dump(tables, open("/home/volence/sonic_hacks/.scratch/as-missing-builtins/tables.json", "w"), indent=0)
print("tables:", {k: len(v) for k, v in tables.items()})
