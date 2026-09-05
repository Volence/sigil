#!/usr/bin/env python3
"""Wave-B profiler drive, deterministic max-H scroll on s4.debug.bin.

Boot 60 frames -> hold right -> 400 frames to reach steady max-speed scroll ->
profile a 150-frame steady window -> report top routines + Lag_Frame_Count +
Camera_X advance rate. Identical drive for BEFORE and AFTER builds.

usage: profile_drive.py <rom> <label> [extra_hold_button]
"""
import asyncio, json, os, sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from suite_paths import add_empyrean_clients, debug_listing  # noqa: E402
add_empyrean_clients()
from aether import BusClient

ROM = sys.argv[1]
LABEL = sys.argv[2]
EXTRA = sys.argv[3:] if len(sys.argv) > 3 else []
LST = debug_listing()
CAMERA_X, LAG = 0xFFA140, 0xFF89F8  # s4.debug shape
OUT = os.path.dirname(os.path.abspath(__file__))

async def call(bus, m, p=None):
    r = await bus.call(m, p or {})
    if isinstance(r, dict) and r.get("ok") is False:
        raise RuntimeError(f"{m}: {r}")
    return r

async def rd(bus, addr, n):
    r = await call(bus, "read_memory", {"addr": f"0x{addr:X}", "len": n})
    return int(r["bytes"], 16)

async def main():
    bus = BusClient(client_id="wbprof", client_name="waveb-profiler", client_version="1",
                    want_events=False)
    await bus.connect()
    await call(bus, "breakpoint_clear", {"all": True})
    import os as _os
    if _os.environ.get("SKIP_RELOAD"):
        # The GUI booted the ROM from disk (LastRomPath); verify identity via the
        # in-ROM end pointer instead of exercising the flaky loader at all.
        want = _os.path.getsize(ROM); want += want % 2
        r = await call(bus, "read_memory", {"addr": "0x1A0", "len": 8})
        end = int(r["bytes"][8:], 16)
        assert end + 1 in (want, want - 1), f"booted ROM end {end:#x} != target size {want:#x}"
        await call(bus, "load_symbols", {"path": LST})
        await run_body(bus)
        return
    want = _os.path.getsize(ROM)
    want += want % 2
    ok = False
    for attempt in range(4):
        r = await call(bus, "reload_rom", {"path": ROM, "reset": True, "wait": True})
        d = str(r.get("diagnostic", ""))
        if r.get("reloaded") and f"-> {want};" in d.replace("size:", "size:").replace(f"-> {want}", f"-> {want}"):
            ok = True; break
        if f"size: {want} -> {want}" in d or (r.get("reloaded") and str(want) in d.split("size:")[-1].split(";")[0].split("->")[-1]):
            ok = True; break  # already the target ROM
        print(f"reload attempt {attempt+1} rejected: {r} — retrying in 6s")
        await asyncio.sleep(6)
    assert ok, "reload never installed the target ROM"
    await call(bus, "load_symbols", {"path": LST})
    await run_body(bus)

async def run_body(bus):
    await call(bus, "reset", {"run": False})
    await call(bus, "run_frames", {"frames": 60})
    await call(bus, "hold", {"buttons": ["right"] + EXTRA, "down": True})
    await call(bus, "run_frames", {"frames": 120})
    cam0 = await rd(bus, CAMERA_X, 2)
    lag0 = await rd(bus, LAG, 4)
    await call(bus, "set_profiler", {"enabled": True})
    await call(bus, "run_frames", {"frames": 150})
    prof = await call(bus, "get_profiler_frames", {"frames": 150, "top": 40})
    await call(bus, "set_profiler", {"enabled": False})
    cam1 = await rd(bus, CAMERA_X, 2)
    lag1 = await rd(bus, LAG, 4)
    await call(bus, "hold", {"buttons": ["right"] + EXTRA, "down": False})
    rec = {"label": LABEL, "rom": ROM, "camera_x_start": cam0, "camera_x_end": cam1,
           "px_per_frame": (cam1 - cam0) / 150.0,
           "lag_frames_start": lag0, "lag_frames_end": lag1,
           "lag_in_window": lag1 - lag0, "profile": prof}
    with open(os.path.join(OUT, f"profile_{LABEL}.json"), "w") as f:
        json.dump(rec, f, indent=1)
    print(json.dumps({k: v for k, v in rec.items() if k != "profile"}, indent=1))

asyncio.run(main())
