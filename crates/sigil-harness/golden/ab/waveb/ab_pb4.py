#!/usr/bin/env python3
"""pb#4 PS state-identity A/B, quantum captures on the SCROLL drive (plain shape).

Scene: reset-paused -> 60 boot frames -> hold right -> anchors at frames 240/420/700
(two mid-scroll, one post-clamp). OLD = the committed golden, NEW = the pb#4 build.
Each ROM runs twice (determinism). Layout-shift classification: a differing aligned
RAM long where both values are ROM addresses with 0 < delta <= 0x20 is a relocated
ROM pointer (the +18 section shift); anything else fails the bar. VDP state + the
hashed framebuffer must be identical outright (tile indexes, not addresses).
"""
import asyncio, json, os, sys, zlib

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from suite_paths import add_empyrean_clients  # noqa: E402
add_empyrean_clients()
from aether import BusClient

OUT = os.path.dirname(os.path.abspath(__file__))
ROMS = {"OLD": f"{OUT}/s4_OLD.bin", "NEW": f"{OUT}/s4_NEW.bin"}
FRAME_COUNTER = 0xFF8002
ANCHORS = [240, 420, 700]
ROM_TOP = 0x70000

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
    d = f"{OUT}/{name}_run{idx}"
    os.makedirs(d, exist_ok=True)
    await call(bus, "breakpoint_clear", {"all": True})
    r = await call(bus, "reload_rom", {"path": rom, "reset": True, "wait": True})
    assert r.get("reloaded")
    await call(bus, "reset", {"run": False})
    await call(bus, "run_frames", {"frames": 60})
    await call(bus, "hold", {"buttons": ["right"], "down": True})
    recs, cur = [], 60
    for a in ANCHORS:
        await call(bus, "run_frames", {"frames": a - cur}); cur = a
        fc = int.from_bytes(await read_block(bus, FRAME_COUNTER, 2), "big")
        sh = await call(bus, "state_hash", {"includeFramebuffer": True})
        ram = await read_block(bus, 0xFF0000, 0x10000)
        with open(f"{d}/ram_f{a}.bin", "wb") as f:
            f.write(ram)
        recs.append({"anchor": a, "fc": fc,
                     "ram_crc": f"{zlib.crc32(ram) & 0xffffffff:08x}",
                     "vram": sh["vram"], "cram": sh["cram"], "vsram": sh["vsram"],
                     "regs": sh["regs"], "fb": sh.get("framebuffer")})
        print(json.dumps({"run": f"{name}_run{idx}", **recs[-1]}))
    await call(bus, "hold", {"buttons": ["right"], "down": False})
    return recs

def classify(old, new):
    """Return (relocated_ptr_count, bad_offsets)."""
    reloc, bad = 0, []
    i = 0
    while i < len(old):
        if old[i] != new[i]:
            base = i & ~3
            ov = int.from_bytes(old[base:base+4], "big")
            nv = int.from_bytes(new[base:base+4], "big")
            if ov != nv and ov < ROM_TOP and nv < ROM_TOP and 0 < nv - ov <= 0x20:
                reloc += 1
                i = base + 4
                continue
            # word-granular ROM-address check (pushed PCs are longs; but allow
            # a word pair straddle miss -> report offset)
            bad.append(base)
            i = base + 4
            continue
        i += 1
    return reloc, bad

async def main():
    bus = BusClient(client_id="pb4ab", client_name="pb4-ab", client_version="1", want_events=False)
    await bus.connect()
    m = {}
    for name, rom in ROMS.items():
        for idx in (1, 2):
            m[f"{name}_run{idx}"] = await one_run(bus, name, rom, idx)
    with open(f"{OUT}/manifest_pb4.json", "w") as f:
        json.dump(m, f, indent=1)
    # determinism
    for n in ("OLD", "NEW"):
        assert m[f"{n}_run1"] == m[f"{n}_run2"], f"{n} runs differ — determinism broken"
    print("DETERMINISM OK")
    # A/B verdicts
    for i, a in enumerate(ANCHORS):
        o, nrec = m["OLD_run1"][i], m["NEW_run1"][i]
        assert o["fc"] == nrec["fc"], f"anchor {a}: fc mismatch"
        for k in ("vram", "cram", "vsram", "fb"):
            print(f"anchor {a} {k}: {'IDENTICAL' if o[k]==nrec[k] else 'DIFFER'}")
        old = open(f"{OUT}/OLD_run1/ram_f{a}.bin", "rb").read()
        new = open(f"{OUT}/NEW_run1/ram_f{a}.bin", "rb").read()
        if old == new:
            print(f"anchor {a} RAM: IDENTICAL")
        else:
            reloc, bad = classify(old, new)
            print(f"anchor {a} RAM: {reloc} relocated ROM ptrs, {len(bad)} UNCLASSIFIED"
                  + (f" at {['0x%04X'%b for b in bad[:8]]}" if bad else ""))
    print("DONE")

asyncio.run(main())
