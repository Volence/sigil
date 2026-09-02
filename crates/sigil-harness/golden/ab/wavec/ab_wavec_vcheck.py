#!/usr/bin/env python3
"""At the vshot capture point (run_frames -> run_to_scanline 240), read vram/cram/
vsram/regs state_hash + Camera_X, to reconcile the screenshot diff against the
render-input identity. usage: ab_wavec_vcheck.py <rom> <OLD|NEW>"""
import asyncio, os, sys, json
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from suite_paths import add_empyrean_clients  # noqa: E402
add_empyrean_clients()
from aether import BusClient
OUT = os.path.dirname(os.path.abspath(__file__)); ROM, NAME = sys.argv[1], sys.argv[2]
CAMERA_X, FRAME_COUNTER = 0xFFA140, 0xFF8002
async def call(bus, m, p=None):
    r = await bus.call(m, p or {})
    if isinstance(r, dict) and r.get("ok") is False: raise RuntimeError(f"{m}: {r}")
    return r
async def rd(bus, a, n):
    r = await call(bus, "read_memory", {"addr": f"0x{a:X}", "len": n}); return r["bytes"]
async def main():
    bus = BusClient(client_id="wavecvc", client_name="wavec-vcheck", client_version="1", want_events=False)
    await bus.connect()
    await call(bus, "breakpoint_clear", {"all": True})
    want = os.path.getsize(ROM); want += want % 2
    r = await call(bus, "read_memory", {"addr": "0x1A0", "len": 8})
    assert int(r["bytes"][8:], 16) + 1 in (want, want - 1), "wrong ROM booted"
    await call(bus, "reset", {"run": False})
    await call(bus, "run_frames", {"frames": 60})
    await call(bus, "hold", {"buttons": ["right"], "down": True})
    cur, out = 60, []
    for f in (200, 260, 320):
        await call(bus, "run_frames", {"frames": f - cur}); cur = f
        await call(bus, "run_to_scanline", {"line": 240})
        sh = await call(bus, "state_hash", {"includeFramebuffer": True})
        rec = {"f": f, "fc": int(await rd(bus, FRAME_COUNTER, 2), 16), "cam_x": await rd(bus, CAMERA_X, 4),
               "vram": sh["vram"], "cram": sh["cram"], "vsram": sh["vsram"], "regs": sh["regs"], "fb": sh.get("framebuffer")}
        out.append(rec); print(json.dumps({"NAME": NAME, **rec}), flush=True)
    await call(bus, "hold", {"buttons": ["right"], "down": False})
    json.dump(out, open(f"{OUT}/vcheck_{NAME}.json", "w"), indent=1)
    try: await bus.close()
    except Exception: pass
asyncio.run(main())
