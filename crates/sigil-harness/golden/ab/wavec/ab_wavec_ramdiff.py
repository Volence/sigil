#!/usr/bin/env python3
"""Wave-C RAM pinpoint, dump full 64K RAM at the two anchors whose crc diverged
(render fc=340, soak cam_x=576), so OLD vs NEW can be byte-diffed locally to
classify the diff (expected: moved return-address bytes in the stack region below
SSP 0xFFFF00). Saves <NAME>.render340.ram.bin and <NAME>.soak576.ram.bin.
usage: ab_wavec_ramdiff.py <rom> <OLD|NEW>"""
import asyncio, os, sys, zlib
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from suite_paths import add_empyrean_clients  # noqa: E402
add_empyrean_clients()
from aether import BusClient
OUT = os.path.dirname(os.path.abspath(__file__))
ROM, NAME = sys.argv[1], sys.argv[2]
FRAME_COUNTER, DEBUG_SCENE_FREEZE = 0xFF8002, 0xFF8A10
CAMERA_X, CAMERA_Y, UPDATE_ENTRY = 0xFFA140, 0xFFA144, 0x5E42C

async def call(bus, m, p=None):
    r = await bus.call(m, p or {})
    if isinstance(r, dict) and r.get("ok") is False:
        raise RuntimeError(f"{m}: {r}")
    return r
async def read_block(bus, addr, length):
    out = bytearray()
    while length:
        n = min(4096, length)
        r = await call(bus, "read_memory", {"addr": f"0x{addr:X}", "len": n})
        b = bytes.fromhex(r["bytes"]); assert len(b) == n
        out += b; addr += n; length -= n
    return bytes(out)
async def dump(bus, tag):
    ram = await read_block(bus, 0xFF0000, 0x10000)
    with open(f"{OUT}/{NAME}.{tag}.ram.bin", "wb") as f:
        f.write(ram)
    print(f"{NAME} {tag} ram_crc={zlib.crc32(ram)&0xffffffff:08x}", flush=True)

async def main():
    bus = BusClient(client_id="wavecrd", client_name="wavec-ramdiff", client_version="1", want_events=False)
    await bus.connect()
    await call(bus, "breakpoint_clear", {"all": True})
    want = os.path.getsize(ROM); want += want % 2
    r = await call(bus, "read_memory", {"addr": "0x1A0", "len": 8})
    assert int(r["bytes"][8:], 16) + 1 in (want, want - 1), "wrong ROM booted"
    # render fc=340
    await call(bus, "reset", {"run": False})
    await call(bus, "run_frames", {"frames": 60})
    await call(bus, "hold", {"buttons": ["right"], "down": True})
    await call(bus, "run_frames", {"frames": 278})
    await call(bus, "breakpoint_add", {"addr": f"0x{UPDATE_ENTRY:X}"})
    while True:
        await call(bus, "resume", {}); await call(bus, "wait_for_break", {})
        if int.from_bytes(await read_block(bus, FRAME_COUNTER, 2), "big") >= 340:
            break
        await call(bus, "step", {})
    await call(bus, "breakpoint_clear", {"all": True})
    await dump(bus, "render340")
    await call(bus, "hold", {"buttons": ["right"], "down": False})
    # soak cam_x=576
    await call(bus, "reset", {"run": False})
    await call(bus, "run_frames", {"frames": 60})
    await call(bus, "write_memory", {"addr": f"0x{DEBUG_SCENE_FREEZE:X}", "value": 1, "width": 1})
    await call(bus, "write_memory", {"addr": f"0x{CAMERA_Y:X}", "value": 0, "width": 4})
    for cx in [0x0040, 0x00C0, 0x0140, 0x01C0, 0x0240]:
        await call(bus, "write_memory", {"addr": f"0x{CAMERA_X:X}", "value": cx << 16, "width": 4})
        await call(bus, "run_frames", {"frames": 6})
    await dump(bus, "soak576")
    await call(bus, "write_memory", {"addr": f"0x{DEBUG_SCENE_FREEZE:X}", "value": 0, "width": 1})
    try: await bus.close()
    except Exception: pass
asyncio.run(main())
