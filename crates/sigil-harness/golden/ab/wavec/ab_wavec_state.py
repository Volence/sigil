#!/usr/bin/env python3
"""Wave-C parallax row-35 A/B state capture, two scenes per ROM (debug shape).

The parcel moves the per-frame VDP reg $0B (Mode Set 3) re-assert from the ojz
harness force-write into Parallax_Update (shadow + direct hardware write). These
two scenes are the t41 named checks:

Scene RENDER (check a), "engine writes the same mode; no render regression on
  the ojz boot scene." Boot 60f, hold RIGHT (debug-fly camera scroll → parallax
  runs + the mode write fires every frame; Debug_Scene_Freeze=0 so camera/entity
  are normal). Code-point anchors at GameState_OJZScroll_Update entry (0x5E42C,
  identical OLD/NEW) at frames 220/280/340. Captures state_hash
  (vram/cram/vsram/regs/fb) + full-RAM crc + a screenshot. VRAM/CRAM/VSRAM = the
  render INPUTS; the VDP register file (regs) contains reg $0B itself, so an
  OLD==NEW regs hash IS the "same mode" proof.

Scene SOAK (check b), "the extra per-frame write does not perturb the
  deterministic Debug_Scene_Freeze cache-fill soak." Boot 60f, set
  Debug_Scene_Freeze=1 (0xFF8A10) then poke Camera_X to a fixed ascending
  sequence, running frames at each stop to drive Tile_Cache_Fill deterministically
  (frozen camera → Camera_Update skipped, poked value persists). Captures full-RAM
  crc + state_hash after each stop. The mode write touches neither tile-cache RAM
  nor VRAM tiles, so OLD==NEW byte-identity is the expectation.

Each scene runs twice per ROM (determinism, fb excluded). SKIP_RELOAD: the GUI
booted the target from disk (ab_current.bin); identity is verified via the in-ROM
end pointer, the flaky loader stays out of the loop.

usage: ab_wavec_state.py <rom> <OLD|NEW>
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

FRAME_COUNTER      = 0xFF8002
DEBUG_SCENE_FREEZE = 0xFF8A10
CAMERA_X           = 0xFFA140      # 16.16; high word = integer px
CAMERA_Y           = 0xFFA144
UPDATE_ENTRY       = 0x5E42C       # GameState_OJZScroll_Update (debug) — same OLD/NEW
RENDER_ANCHORS     = [220, 280, 340]
SOAK_CAM_X         = [0x0040, 0x00C0, 0x0140, 0x01C0, 0x0240]  # ascending px stops

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

async def ram_crc(bus):
    ram = await read_block(bus, 0xFF0000, 0x10000)
    return f"{zlib.crc32(ram) & 0xffffffff:08x}", ram

async def hashes(bus, fb=False):
    sh = await call(bus, "state_hash", {"includeFramebuffer": fb})
    rec = {"vram": sh["vram"], "cram": sh["cram"], "vsram": sh["vsram"], "regs": sh["regs"]}
    if fb:
        rec["fb"] = sh.get("framebuffer")
    return rec

async def render_run(bus, idx):
    d = f"{OUT}/wavec_{NAME}_run{idx}"
    os.makedirs(d, exist_ok=True)
    await call(bus, "breakpoint_clear", {"all": True})
    await call(bus, "reset", {"run": False})
    await call(bus, "run_frames", {"frames": 60})
    await call(bus, "hold", {"buttons": ["right"], "down": True})
    await call(bus, "run_frames", {"frames": 120})
    recs, cur = [], 180
    for a in RENDER_ANCHORS:
        if a - 2 > cur:
            await call(bus, "run_frames", {"frames": (a - 2) - cur}); cur = a - 2
        await call(bus, "breakpoint_add", {"addr": f"0x{UPDATE_ENTRY:X}"})
        while True:
            await call(bus, "resume", {})
            await call(bus, "wait_for_break", {})
            fc = int.from_bytes(await read_block(bus, FRAME_COUNTER, 2), "big")
            if fc >= a:
                break
            await call(bus, "step", {})   # step off the bp PC or resume re-fires
        await call(bus, "breakpoint_clear", {"all": True})
        cur = fc
        h = await hashes(bus, fb=True)
        crc, _ = await ram_crc(bus)
        png = f"{d}/render_f{a}.png"
        await call(bus, "screenshot", {"path": png})
        rec = {"scene": "render", "anchor": a, "fc": fc, "ram_crc": crc, **h}
        recs.append(rec)
        print(json.dumps({"run": f"{NAME}_r{idx}", **{k: rec[k] for k in ("scene","anchor","fc","ram_crc","vram","cram","vsram","regs")}}), flush=True)
    await call(bus, "breakpoint_clear", {"all": True})
    await call(bus, "hold", {"buttons": ["right"], "down": False})
    return recs

async def soak_run(bus, idx):
    await call(bus, "breakpoint_clear", {"all": True})
    await call(bus, "reset", {"run": False})
    await call(bus, "run_frames", {"frames": 60})
    await call(bus, "write_memory", {"addr": f"0x{DEBUG_SCENE_FREEZE:X}", "value": 1, "width": 1})
    await call(bus, "write_memory", {"addr": f"0x{CAMERA_Y:X}", "value": 0, "width": 4})
    recs = []
    for cx in SOAK_CAM_X:
        await call(bus, "write_memory", {"addr": f"0x{CAMERA_X:X}", "value": cx << 16, "width": 4})
        await call(bus, "run_frames", {"frames": 6})
        h = await hashes(bus, fb=False)
        crc, _ = await ram_crc(bus)
        rec = {"scene": "soak", "cam_x": cx, "ram_crc": crc, **h}
        recs.append(rec)
        print(json.dumps({"run": f"{NAME}_r{idx}", **rec}), flush=True)
    await call(bus, "write_memory", {"addr": f"0x{DEBUG_SCENE_FREEZE:X}", "value": 0, "width": 1})
    return recs

async def main():
    bus = BusClient(client_id="wavec", client_name="wavec-state-ab", client_version="1", want_events=False)
    await bus.connect()
    await call(bus, "breakpoint_clear", {"all": True})
    want = os.path.getsize(ROM); want += want % 2
    r = await call(bus, "read_memory", {"addr": "0x1A0", "len": 8})
    end = int(r["bytes"][8:], 16)
    assert end + 1 in (want, want - 1), f"booted ROM end {end:#x} != target size {want:#x} — swap ab_current.bin"
    await call(bus, "load_symbols", {"path": LST})
    m = {}
    for i in (1, 2):
        m[f"render_run{i}"] = await render_run(bus, i)
        m[f"soak_run{i}"] = await soak_run(bus, i)
    with open(f"{OUT}/manifest_wavec_{NAME}.json", "w") as f:
        json.dump(m, f, indent=1)
    def key(recs):
        return [{k: r.get(k) for k in ("scene","anchor","cam_x","fc","ram_crc","vram","cram","vsram","regs")} for r in recs]
    det_render = key(m["render_run1"]) == key(m["render_run2"])
    det_soak = key(m["soak_run1"]) == key(m["soak_run2"])
    print(f"{NAME} DETERMINISM render:", "OK" if det_render else "BROKEN", "| soak:", "OK" if det_soak else "BROKEN", flush=True)
    try:
        await bus.close()
    except Exception:
        pass

asyncio.run(main())
