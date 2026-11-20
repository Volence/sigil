#!/usr/bin/env python3
"""BlastEm boot witness for the walk-measure-in-place parcel (headless, xvfb, GDB RSP stub).

Boots ROM on BlastEm 0.6.2 from power-on with one breakpoint at `Input_Tick` (resolved
from the ROM's own listing), continues for N hits, and records (Frame_Counter, Logic_Tick)
at every hit. A ROM that does not reach Input_Tick N times did not boot to its game loop.
Prints one JSON line. Exit 0 booted, 1 did not reach N hits, 2 could not run.
"""
import json
import re
import sys
import time

sys.path.insert(0, "/home/volence/sonic_hacks/oracle/tools/blastem-differential")
import rsp  # noqa: E402

rom, lst, n = sys.argv[1], sys.argv[2], int(sys.argv[3])
syms = {}
for line in open(lst, errors="replace"):
    m = re.match(r"^[ *](\S+) : ([0-9A-F]+) C \|$", line)
    if m:
        syms[m.group(1)] = int(m.group(2), 16)
try:
    tick, fc, lt = syms["Input_Tick"], syms["Frame_Counter"] & 0xFFFFFF, syms["Logic_Tick"] & 0xFFFFFF
except KeyError as e:
    print(json.dumps({"rom": rom, "status": "COULD NOT RUN", "reason": f"missing symbol {e}"}))
    sys.exit(2)
rsp.watchdog(300)
t0 = time.monotonic()
stub = rsp.RSP(rom)
rows = []
try:
    stub.wait_ready()
    stub.bp(tick)
    for _ in range(n):
        rep, timed_out = stub.cont_or_interrupt(10.0)
        if timed_out or rep is None:
            break
        regs = stub.read_regs()
        if regs["pc"] & 0xFFFFFF != tick:
            break
        f = stub.read_mem(fc, 2)
        g = stub.read_mem(lt, 2)
        rows.append((int.from_bytes(f, "big"), int.from_bytes(g, "big")))
finally:
    stub.close()
ok = len(rows) == n
print(json.dumps({"rom": rom, "status": "BOOTED" if ok else "DID NOT REACH N HITS", "hits": len(rows),
                  "want": n, "first": rows[:3], "last": rows[-1] if rows else None,
                  "rows": rows, "wall_s": round(time.monotonic() - t0, 1)}))
sys.exit(0 if ok else 1)
