#!/usr/bin/env python3
"""Frame-boundary screenshots (paused at vblank via run_frames, not a mid-frame
breakpoint) for a phase-independent render cmp. usage: ab_wavec_shot.py <rom> <OLD|NEW>"""
import asyncio, os, sys
sys.path.insert(0, "/home/volence/sonic_hacks/empyrean/clients/python")
from aether import BusClient
OUT = os.path.dirname(os.path.abspath(__file__)); ROM, NAME = sys.argv[1], sys.argv[2]
async def call(bus, m, p=None):
    r = await bus.call(m, p or {})
    if isinstance(r, dict) and r.get("ok") is False: raise RuntimeError(f"{m}: {r}")
    return r
async def main():
    bus = BusClient(client_id="wavecsh", client_name="wavec-shot", client_version="1", want_events=False)
    await bus.connect()
    await call(bus, "breakpoint_clear", {"all": True})
    want = os.path.getsize(ROM); want += want % 2
    r = await call(bus, "read_memory", {"addr": "0x1A0", "len": 8})
    assert int(r["bytes"][8:], 16) + 1 in (want, want - 1), "wrong ROM booted"
    await call(bus, "reset", {"run": False})
    await call(bus, "run_frames", {"frames": 60})
    await call(bus, "hold", {"buttons": ["right"], "down": True})
    cur = 60
    for f in (200, 260, 320):
        await call(bus, "run_frames", {"frames": f - cur}); cur = f
        await call(bus, "screenshot", {"path": f"{OUT}/shot_{NAME}_f{f}.png"})
        print(f"{NAME} f{f} shot", flush=True)
    await call(bus, "hold", {"buttons": ["right"], "down": False})
    try: await bus.close()
    except Exception: pass
asyncio.run(main())
