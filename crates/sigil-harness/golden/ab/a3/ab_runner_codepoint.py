#!/usr/bin/env python3
"""A/B runner v3 — fc-exact code-point capture at Frame_Counter==470 (closes the
v2 anchor-480 one-frame misalignment). Loops run_to(GameState_ObjectTest) until
Frame_Counter reaches the target, so OLD and NEW are captured at the identical
PC AND identical Frame_Counter."""
import asyncio, json, os, sys, zlib

sys.path.insert(0, "/home/volence/sonic_hacks/empyrean/clients/python")
from aether import BusClient

OUT = os.path.dirname(os.path.abspath(__file__))
ROMS = {"OLD": os.path.join(OUT, "s4_OLD.bin"), "NEW": os.path.join(OUT, "s4_NEW.bin")}
GAME_STATE, FRAME_COUNTER = 0xFF8004, 0xFF8002
OBJECT_TEST_INIT, OBJECT_TEST_LOOP = 0x0005C230, 0x0005C2F0
TARGET_FC = 470

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

async def one_run(bus, name, rom, idx):
    await call(bus, "breakpoint_clear", {"all": True})
    r = await call(bus, "reload_rom", {"path": rom, "reset": True, "wait": True})
    assert r.get("reloaded")
    await call(bus, "reset", {"run": False})
    await call(bus, "run_frames", {"frames": 60})
    await call(bus, "write_memory", {"addr": f"0x{GAME_STATE:X}", "value": OBJECT_TEST_INIT, "width": 4})
    await call(bus, "run_frames", {"frames": 400})  # near fc ~454
    fc = 0
    while fc < TARGET_FC:
        await call(bus, "run_to", {"addr": f"0x{OBJECT_TEST_LOOP:X}"})
        br = await call(bus, "wait_for_break", {"timeout_ms": 20000})
        assert not br.get("timeout_reached"), br
        if br.get("pc") != f"0x{OBJECT_TEST_LOOP:08X}":
            continue  # spurious stale-transient break: re-arm and keep driving
        fc = int.from_bytes(await read_block(bus, FRAME_COUNTER, 2), "big")
    assert fc == TARGET_FC, f"overshot: fc={fc}"
    sh = await call(bus, "state_hash", {})
    ram = await read_block(bus, 0xFF0000, 0x10000)
    rec = {"tag": f"{name}_run{idx}", "fc": fc,
           "ram_crc32": f"{zlib.crc32(ram) & 0xffffffff:08x}",
           "vdp": sh["combined"], "regs": sh["regs"], "vram": sh["vram"],
           "cram": sh["cram"], "vsram": sh["vsram"]}
    print(json.dumps(rec))
    return rec

async def main():
    bus = BusClient(client_id="a3abv3", client_name="a3-ab-runner-v3", client_version="1", want_events=False)
    await bus.connect()
    out = [await one_run(bus, n, r, i) for n, r in ROMS.items() for i in (1, 2)]
    with open(os.path.join(OUT, "manifest_v4.json"), "w") as f:
        json.dump(out, f, indent=1)
    print("DONE")

asyncio.run(main())
