#!/usr/bin/env python3
"""Vblank-aligned screenshots: run to frame N, then run_to_scanline into vblank
(visible lines 0-223 fully rendered, beam past the visible area) → the captured
framebuffer is a complete, phase-stable frame. usage: ab_wavec_vshot.py <rom> <OLD|NEW>"""
import asyncio, os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from suite_paths import add_empyrean_clients  # noqa: E402
add_empyrean_clients()
from aether import BusClient
OUT = os.path.dirname(os.path.abspath(__file__)); ROM, NAME = sys.argv[1], sys.argv[2]
async def call(bus, m, p=None):
    r = await bus.call(m, p or {})
    if isinstance(r, dict) and r.get("ok") is False: raise RuntimeError(f"{m}: {r}")
    return r
async def main():
    bus = BusClient(client_id="wavecvs", client_name="wavec-vshot", client_version="1", want_events=False)
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
        await call(bus, "run_to_scanline", {"line": 240})
        await call(bus, "screenshot", {"path": f"{OUT}/vshot_{NAME}_f{f}.png"})
        print(f"{NAME} f{f} vshot", flush=True)
    await call(bus, "hold", {"buttons": ["right"], "down": False})
    try: await bus.close()
    except Exception: pass
asyncio.run(main())
