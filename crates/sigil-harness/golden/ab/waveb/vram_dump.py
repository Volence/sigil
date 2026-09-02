#!/usr/bin/env python3
"""Dump full VRAM at quantum anchor 244 for one ROM (argv: OLD|NEW). No breakpoints."""
import asyncio, os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from suite_paths import add_empyrean_clients  # noqa: E402
add_empyrean_clients()
from aether import BusClient
OUT = os.path.dirname(os.path.abspath(__file__))
ROMS = {"OLD": f"{OUT}/s4_OLD.bin", "NEW": f"{OUT}/s4_NEW.bin"}

async def call(bus, m, p=None):
    r = await bus.call(m, p or {})
    if isinstance(r, dict) and r.get("ok") is False:
        raise RuntimeError(f"{m}: {r}")
    return r

async def main():
    name = sys.argv[1]
    bus = BusClient(client_id="vrdump", client_name="vram-dump", client_version="1", want_events=False)
    await bus.connect()
    r = await call(bus, "reload_rom", {"path": ROMS[name], "reset": True, "wait": True})
    assert r.get("reloaded"), r
    await call(bus, "reset", {"run": False})
    await call(bus, "run_frames", {"frames": 60})
    await call(bus, "hold", {"buttons": ["right"], "down": True})
    await call(bus, "run_frames", {"frames": 184})
    v = bytearray()
    for a in range(0, 0x10000, 0x1000):
        r = await call(bus, "read_vram", {"addr": a, "len": 0x1000})
        v += bytes.fromhex(r["bytes"])
    open(f"{OUT}/vram244_{name}.bin", "wb").write(bytes(v))
    await call(bus, "hold", {"buttons": ["right"], "down": False})
    print(f"{name}: vram dumped, {len(v)} bytes")

asyncio.run(main())
