#!/usr/bin/env python3
"""collision_lookup #1 (fusion) PS state-identity capture, the collision-heavy drive.

Captures ONE ROM (already booted via ab_current.bin content-swap or an MCP reload;
SKIP_RELOAD identity-checks the booted cart against the target size). Writes
manifest_coll_<name>.json. Run once for OLD, once for NEW, then diff the manifests.

Scene (deterministic from reset): reset-paused -> 60 boot frames -> B press edge
(drop debug-fly -> physics) -> 120 fall/land frames -> hold right -> CODE-POINT
anchors at the GameState_OJZScroll_Update entry (a deterministic PC, kills the
mid-VBlank / interrupted-pc capture-alias class) at settled frames 220/280/340.
Each ROM runs twice (determinism) via reset (no reload between passes).

usage: ab_collision_state.py <rom> <OLD|NEW>
"""
import asyncio, json, os, sys, zlib

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from suite_paths import add_empyrean_clients, debug_listing  # noqa: E402
add_empyrean_clients()
from aether import BusClient

OUT = os.path.dirname(os.path.abspath(__file__))
ROM = sys.argv[1]
NAME = sys.argv[2]
LST = debug_listing()
FRAME_COUNTER = 0xFF8002
UPDATE_ENTRY = 0x5E42C          # GameState_OJZScroll_Update (debug) — code-point anchor
ANCHORS = [220, 280, 340]       # settled wall-push frames (landing ~182)

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

async def one_run(bus, idx):
    d = f"{OUT}/coll_{NAME}_run{idx}"
    os.makedirs(d, exist_ok=True)
    await call(bus, "breakpoint_clear", {"all": True})
    await call(bus, "reset", {"run": False})
    await call(bus, "run_frames", {"frames": 60})
    await call(bus, "hold", {"buttons": ["b"], "down": True})
    await call(bus, "run_frames", {"frames": 2})
    await call(bus, "hold", {"buttons": ["b"], "down": False})
    await call(bus, "run_frames", {"frames": 120})
    await call(bus, "hold", {"buttons": ["right"], "down": True})
    recs, cur = [], 182
    for a in ANCHORS:
        # free-run to just before the target frame, then land on the update-entry
        # ONCE (code-point anchor) — deterministic pc, no per-frame breakpoint loop.
        if a - 2 > cur:
            await call(bus, "run_frames", {"frames": (a - 2) - cur})
            cur = a - 2
        await call(bus, "breakpoint_add", {"addr": f"0x{UPDATE_ENTRY:X}"})
        while True:
            await call(bus, "resume", {})
            await call(bus, "wait_for_break", {})
            fc = int.from_bytes(await read_block(bus, FRAME_COUNTER, 2), "big")
            if fc >= a:
                break
            # A paused PC sitting ON the breakpoint re-fires without executing;
            # single-step off it so the next resume actually runs a frame.
            await call(bus, "step", {})
        await call(bus, "breakpoint_clear", {"all": True})
        cur = fc
        sh = await call(bus, "state_hash", {"includeFramebuffer": True})
        rec = {"anchor": a, "fc": fc,
               "vram": sh["vram"], "cram": sh["cram"], "vsram": sh["vsram"],
               "regs": sh["regs"], "fb": sh.get("framebuffer")}
        if os.environ.get("READ_RAM"):
            ram = await read_block(bus, 0xFF0000, 0x10000)
            with open(f"{d}/ram_f{a}.bin", "wb") as f:
                f.write(ram)
            rec["ram_crc"] = f"{zlib.crc32(ram) & 0xffffffff:08x}"
        recs.append(rec)
        print(json.dumps({"run": f"{NAME}_run{idx}", **recs[-1]}), flush=True)
    await call(bus, "breakpoint_clear", {"all": True})
    await call(bus, "hold", {"buttons": ["right"], "down": False})
    return recs

async def main():
    bus = BusClient(client_id="collab", client_name="coll-state-ab", client_version="1", want_events=False)
    await bus.connect()
    await call(bus, "breakpoint_clear", {"all": True})
    want = os.path.getsize(ROM); want += want % 2
    r = await call(bus, "read_memory", {"addr": "0x1A0", "len": 8})
    end = int(r["bytes"][8:], 16)
    assert end + 1 in (want, want - 1), f"booted ROM end {end:#x} != target size {want:#x} — swap ab_current.bin"
    await call(bus, "load_symbols", {"path": LST})
    m = {f"run{i}": await one_run(bus, i) for i in (1, 2)}
    with open(f"{OUT}/manifest_coll_{NAME}.json", "w") as f:
        json.dump(m, f, indent=1)
    def key(recs):
        return [{k: r[k] for k in ("anchor", "fc", "ram_crc", "vram", "cram", "vsram", "regs")} for r in recs]
    det = key(m["run1"]) == key(m["run2"])
    print(f"{NAME} DETERMINISM (fb excluded):", "OK" if det else "BROKEN", flush=True)
    try:
        await bus.close()
    except Exception:
        pass

asyncio.run(main())
