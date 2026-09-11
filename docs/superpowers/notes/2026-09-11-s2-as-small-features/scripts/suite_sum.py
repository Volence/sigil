#!/usr/bin/env python3
"""suite_sum.py <log> : totals over every `test result:` line, failing test
names first, and the partial-run (unmeasured) banner lines."""
import re, sys

t = open(sys.argv[1], errors="replace").read()
res = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored", t)
p = sum(int(r[1]) for r in res)
f = sum(int(r[2]) for r in res)
i = sum(int(r[3]) for r in res)
fails = sorted(set(re.findall(r"^test (\S+) \.\.\. FAILED", t, re.M)))
panics = re.findall(r"^---- (\S+) stdout ----", t, re.M)
print(f"FAILED tests ({len(fails)}):")
for x in fails:
    print("  ", x)
print(f"binaries={len(res)} passed={p} failed={f} ignored={i}")
for line in t.splitlines():
    if "UNMEASURED" in line or "partial" in line.lower() and "run" in line.lower() and "binar" in line.lower():
        print("banner:", line.strip()[:200])
        break
print([l for l in t.splitlines() if l.startswith(("head=", "pwd=", "tracked-changes=", "SUITE_END"))])
