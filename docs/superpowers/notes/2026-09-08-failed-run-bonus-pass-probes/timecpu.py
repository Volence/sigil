#!/usr/bin/env python3
"""Interleaved CPU-time comparison of two sigil binaries over every corpus root.

Wall clock on this machine moves by more than 4x with load, so the figure that
is compared is child CPU (user+sys) from getrusage(RUSAGE_CHILDREN), differenced
across one subprocess call. Wall and the 1-minute load average are recorded
beside every rep so the contamination stays visible.
"""
import json
import os
import resource
import statistics
import subprocess
import sys
import time

S = "/home/volence/sonic_hacks/.scratch/bonus-pass"
C = S + "/corpus"
ROOTS = [
    ("s1", C + "/s1disasm", "sonic.asm"),
    ("s2", C + "/s2disasm", "s2.asm"),
    ("s2mompass", C + "/s2disasm-mompass-clean", "s2.asm"),
    ("sk", C + "/skdisasm", "sonic3k.asm"),
    ("s3", C + "/skdisasm", "s3.asm"),
    ("s4legacy", C + "/sonic_hack", "S4.asm"),
    ("sce", C + "/Sonic-Clean-Engine-S.C.E.-", "Engine/Includes.asm"),
    ("batman", C + "/batman", "batman.asm"),
    ("mdos", C + "/MD-OS", "Source.asm"),
]
BINS = [("base", S + "/bin/sigil-base"), ("cut", S + "/bin/sigil-cut")]
REPS = int(sys.argv[1]) if len(sys.argv) > 1 else 7


def load1():
    with open("/proc/loadavg") as f:
        return float(f.read().split()[0])


def one(binpath, cwd, root):
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    t0 = time.perf_counter()
    p = subprocess.run(
        [binpath, root], cwd=cwd,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    wall = time.perf_counter() - t0
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = (after.ru_utime - before.ru_utime) + (after.ru_stime - before.ru_stime)
    return cpu, wall, p.returncode


# warm the page cache once per root so no rep pays a cold read
for name, cwd, root in ROOTS:
    one(BINS[0][1], cwd, root)

res = {}
loads = []
for rep in range(REPS):
    for name, cwd, root in ROOTS:
        for tag, binpath in BINS:
            loads.append(load1())
            cpu, wall, rc = one(binpath, cwd, root)
            res.setdefault((name, tag), []).append((cpu, wall, rc))

print(f"reps={REPS}  load1 min={min(loads):.2f} median={statistics.median(loads):.2f} max={max(loads):.2f}")
print(f"{'root':<10} {'base cpu med':>13} {'cut cpu med':>12} {'cut/base':>9} "
      f"{'base cpu rng':>18} {'cut cpu rng':>18} {'base wall med':>14} {'cut wall med':>13}")
out = {}
for name, _, _ in ROOTS:
    b = [c for c, w, r in res[(name, "base")]]
    k = [c for c, w, r in res[(name, "cut")]]
    bw = [w for c, w, r in res[(name, "base")]]
    kw = [w for c, w, r in res[(name, "cut")]]
    mb, mk = statistics.median(b), statistics.median(k)
    print(f"{name:<10} {mb:13.4f} {mk:12.4f} {mk/mb:9.3f} "
          f"{min(b):8.4f}-{max(b):<9.4f} {min(k):8.4f}-{max(k):<9.4f} "
          f"{statistics.median(bw):14.4f} {statistics.median(kw):13.4f}")
    out[name] = dict(base_cpu=b, cut_cpu=k, base_wall=bw, cut_wall=kw)
out["_load1"] = loads
with open(S + "/logs/timing.json", "w") as f:
    json.dump(out, f)
