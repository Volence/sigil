#!/usr/bin/env python3
"""G9 d7-high-word witness — break at Ground_Move_Cap's probe-direction decode
on the collision drive's grounded-moving approach frames and record D7/D2.

OLD (chain-8): the decode `move.b (a1,d2.w), d7` at 0x1079C — D7's high word
must be observed CLEAN naturally (benign-under-current-dispatch made concrete).
NEW (G9): the decode at 0x1079E, after the inserted `moveq #0, d7` — D7 == 0
by construction. No register injection (ledger row 21): the guard is reached
by the drive alone.

usage: ab_g9_witness.py <rom> <bp_addr_hex> <label>   (SKIP_RELOAD flow)
"""
import asyncio, json, os, sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from suite_paths import add_empyrean_clients  # noqa: E402
add_empyrean_clients()
from aether import BusClient

ROM, BP, LABEL = sys.argv[1], int(sys.argv[2], 16), sys.argv[3]
OUT = os.path.dirname(os.path.abspath(__file__))

async def call(bus, m, p=None):
    r = await bus.call(m, p or {})
    if isinstance(r, dict) and r.get("ok") is False:
        raise RuntimeError(f"{m}: {r}")
    return r

async def main():
    bus = BusClient(client_id="g9wit", client_name="g9-witness", client_version="1", want_events=False)
    await bus.connect()
    await call(bus, "breakpoint_clear", {"all": True})
    want = os.path.getsize(ROM); want += want % 2
    r = await call(bus, "read_memory", {"addr": "0x1A0", "len": 8})
    end = int(r["bytes"][8:], 16)
    assert end + 1 in (want, want - 1), f"booted ROM end {end:#x} != target size {want:#x} — swap ab_current.bin"
    await call(bus, "reset", {"run": False})
    await call(bus, "run_frames", {"frames": 60})
    await call(bus, "hold", {"buttons": ["b"], "down": True})
    await call(bus, "run_frames", {"frames": 2})
    await call(bus, "hold", {"buttons": ["b"], "down": False})
    await call(bus, "run_frames", {"frames": 120})          # fall + land (~182)
    await call(bus, "hold", {"buttons": ["right"], "down": True})
    await call(bus, "run_frames", {"frames": 3})            # grounded-moving
    hits = []
    await call(bus, "breakpoint_add", {"addr": f"0x{BP:X}"})
    for _ in range(8):
        await call(bus, "resume", {})
        await call(bus, "wait_for_break", {})
        regs = await call(bus, "registers", {})
        d7 = int(str(regs.get("d7", regs.get("D7"))), 0) if not isinstance(regs.get("d7", regs.get("D7")), int) else regs.get("d7", regs.get("D7"))
        d2 = regs.get("d2", regs.get("D2"))
        pc = regs.get("pc", regs.get("PC"))
        hits.append({"pc": pc, "d7": d7, "d7_hi": (d7 >> 16) & 0xFFFF, "d2": d2})
        await call(bus, "step", {})                          # off the bp before re-resume
    await call(bus, "breakpoint_clear", {"all": True})
    await call(bus, "hold", {"buttons": ["right"], "down": False})
    rec = {"label": LABEL, "bp": f"0x{BP:X}", "hits": hits,
           "all_high_words_clean": all(h["d7_hi"] == 0 for h in hits)}
    with open(f"{OUT}/witness_{LABEL}.json", "w") as f:
        json.dump(rec, f, indent=1)
    print(json.dumps(rec, indent=1))

asyncio.run(main())
