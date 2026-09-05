#!/usr/bin/env python3
"""A/B runner, animate A3 tail-call (Wave-A pathfinder), PS state-identity bar.

Drives the oracle over the Aether bus with a byte-identical deterministic script
per ROM, capturing at fixed Frame_Counter anchors:

  reload(reset) -> reset(run=false) -> run_frames(60) -> poke Game_State to
  GameState_ObjectTest_Init -> run_frames to anchors 240/480/900 -> capture.

Per anchor: full 64KB work RAM raw dump, VDP state_hash (+framebuffer hash),
screenshot PNG, Frame_Counter + SP + Game_State reads. Each ROM runs TWICE
(determinism self-check) before OLD/NEW comparison.
"""
import asyncio, json, os, sys, zlib

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from suite_paths import add_empyrean_clients  # noqa: E402
add_empyrean_clients()
from aether import BusClient

OUT = os.path.dirname(os.path.abspath(__file__))
ROMS = {
    "OLD": os.path.join(OUT, "s4_OLD.bin"),
    "NEW": os.path.join(OUT, "s4_NEW.bin"),
}
GAME_STATE = 0xFF8004          # Game_State pointer (long)
FRAME_COUNTER = 0xFF8002       # word
OBJECT_TEST_INIT = 0x0005C230  # GameState_ObjectTest_Init
BOOT_FRAMES = 60
ANCHORS = [240, 480, 900]      # Frame_Counter targets (frames since power-on)

async def call(bus, method, params=None):
    r = await bus.call(method, params or {})
    if isinstance(r, dict) and r.get("ok") is False:
        raise RuntimeError(f"{method} failed: {r}")
    return r

async def read_block(bus, addr, length):
    out = bytearray()
    while length:
        n = min(4096, length)
        r = await call(bus, "read_memory", {"addr": f"0x{addr:X}", "len": n})
        b = bytes.fromhex(r["bytes"])
        assert len(b) == n, f"short read at 0x{addr:X}: {len(b)} != {n}"
        out += b
        addr += n
        length -= n
    return bytes(out)

async def capture(bus, tag, run_dir, frame_target):
    os.makedirs(run_dir, exist_ok=True)
    fc = int.from_bytes(await read_block(bus, FRAME_COUNTER, 2), "big")
    gs = int.from_bytes(await read_block(bus, GAME_STATE, 4), "big")
    regs = await call(bus, "registers")
    sh = await call(bus, "state_hash", {"includeFramebuffer": True})
    ram = await read_block(bus, 0xFF0000, 0x10000)
    ram_path = os.path.join(run_dir, f"ram_f{frame_target}.bin")
    with open(ram_path, "wb") as f:
        f.write(ram)
    png = os.path.join(run_dir, f"frame_f{frame_target}.png")
    await call(bus, "screenshot", {"path": png})
    rec = {
        "tag": tag, "frame_target": frame_target, "frame_counter": fc,
        "game_state": f"0x{gs:08X}", "sp": regs.get("sp") or regs.get("a7"),
        "state_hash": {k: sh[k] for k in ("vram", "cram", "vsram", "regs", "combined", "framebuffer") if k in sh},
        "ram_crc32": f"{zlib.crc32(ram) & 0xffffffff:08x}", "ram_len": len(ram),
        "ram_file": ram_path, "png": png,
    }
    print(json.dumps(rec))
    return rec

async def one_run(bus, name, rom, run_idx):
    run_dir = os.path.join(OUT, f"{name}_run{run_idx}")
    r = await call(bus, "reload_rom", {"path": rom, "reset": True, "wait": True})
    assert r.get("reloaded"), f"reload failed: {r}"
    await call(bus, "reset", {"run": False})
    await call(bus, "run_frames", {"frames": BOOT_FRAMES})
    fc = int.from_bytes(await read_block(bus, FRAME_COUNTER, 2), "big")
    gs = int.from_bytes(await read_block(bus, GAME_STATE, 4), "big")
    print(json.dumps({"tag": f"{name}_run{run_idx}", "post_boot_frame_counter": fc,
                      "post_boot_game_state": f"0x{gs:08X}"}))
    await call(bus, "write_memory", {"addr": f"0x{GAME_STATE:X}",
                                     "value": OBJECT_TEST_INIT, "width": 4})
    recs, cur = [], BOOT_FRAMES
    for a in ANCHORS:
        await call(bus, "run_frames", {"frames": a - cur})
        cur = a
        recs.append(await capture(bus, f"{name}_run{run_idx}", run_dir, a))
    return recs

async def main():
    bus = BusClient(client_id="a3ab", client_name="a3-ab-runner", client_version="1",
                    want_events=False)
    await bus.connect()
    manifest = {}
    for name, rom in ROMS.items():
        for run_idx in (1, 2):
            manifest[f"{name}_run{run_idx}"] = await one_run(bus, name, rom, run_idx)
    with open(os.path.join(OUT, "manifest.json"), "w") as f:
        json.dump(manifest, f, indent=1)
    print("DONE")

asyncio.run(main())
