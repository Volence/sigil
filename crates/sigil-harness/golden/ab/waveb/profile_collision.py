#!/usr/bin/env python3
"""Wave-B collision-heavy profiler drive — deterministic grounded run on s4.debug.bin.

The OJZ scroll test boots Player_1 in DEBUG-FLY (yellow square, no collision), so
the max-H camera-scroll drive never exercises the sensor lever (Player_Main ~668
cyc/f, Collision_GetType absent). This drive presses B once to drop into PHYSICS,
lets Sonic fall onto the OJZ terrain, then holds right so the floor/wall/ceiling
sensor pairs probe real collision every frame — the BEFORE the collision_lookup
parcel needs.

Sequence (deterministic from reset, frame-anchored):
  reset -> 60 boot frames -> B press edge (drop to physics) -> 120 fall/land frames
  -> hold right -> 120 settle frames -> profile 150-frame steady window.

Reports the collision path routines (Collision_GetType, Collision_Probe*,
Player_Sensor*, Player_Main) + Lag_Frame_Count + Camera_X advance + a player-state
snapshot (grounded? running?) so the drive can be validated before it is trusted.

usage: SKIP_RELOAD=1 profile_collision.py <rom> <label>
  (the GUI must already have booted <rom> via ab_current.bin / an MCP reload)
"""
import asyncio, json, os, sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from suite_paths import add_empyrean_clients, debug_listing  # noqa: E402
add_empyrean_clients()
from aether import BusClient

ROM = sys.argv[1]
LABEL = sys.argv[2]
LST = debug_listing()
OUT = os.path.dirname(os.path.abspath(__file__))

# s4.debug shape RAM addresses
CAMERA_X = 0xFFA140
LAG      = 0xFF89F8
PLAYER_1 = 0xFF8A12
# SST field offsets (engine/structs.asm): x_pos +$02 (16.16), y_pos +$06,
# x_vel +$0A (8.8), angle +$1F. PlayerV lives in sst_custom; player_state /
# ground_speed read via the debug _pl_* equs below.

async def call(bus, m, p=None):
    r = await bus.call(m, p or {})
    if isinstance(r, dict) and r.get("ok") is False:
        raise RuntimeError(f"{m}: {r}")
    return r

async def rd(bus, addr, n):
    r = await call(bus, "read_memory", {"addr": f"0x{addr:X}", "len": n})
    return int(r["bytes"], 16)

def s16(v):
    return v - 0x10000 if v >= 0x8000 else v

async def main():
    bus = BusClient(client_id="wbcoll", client_name="waveb-collision-profiler",
                    client_version="1", want_events=False)
    await bus.connect()
    await call(bus, "breakpoint_clear", {"all": True})
    # SKIP_RELOAD: verify the booted ROM identity via the in-ROM end pointer ($1A0)
    want = os.path.getsize(ROM); want += want % 2
    r = await call(bus, "read_memory", {"addr": "0x1A0", "len": 8})
    end = int(r["bytes"][8:], 16)
    assert end + 1 in (want, want - 1), f"booted ROM end {end:#x} != target size {want:#x}"
    await call(bus, "load_symbols", {"path": LST})

    HOLD = os.environ.get("HOLD_DIR", "right")   # movement button held through the run
    EARLY = os.environ.get("HOLD_EARLY")         # hold the movement button during the fall too

    await call(bus, "reset", {"run": False})
    await call(bus, "run_frames", {"frames": 60})
    # --- B press edge: drop debug-fly -> physics ---
    await call(bus, "hold", {"buttons": ["b"], "down": True})
    await call(bus, "run_frames", {"frames": 2})
    await call(bus, "hold", {"buttons": ["b"], "down": False})
    if EARLY:
        await call(bus, "hold", {"buttons": [HOLD], "down": True})
    # --- let Sonic fall and land on the OJZ terrain ---
    await call(bus, "run_frames", {"frames": 120})
    ypos_land = s16(await rd(bus, PLAYER_1 + 0x06, 2))
    # --- hold movement button: run over terrain ---
    if not EARLY:
        await call(bus, "hold", {"buttons": [HOLD], "down": True})
    await call(bus, "run_frames", {"frames": 120})

    cam0 = await rd(bus, CAMERA_X, 2)
    lag0 = await rd(bus, LAG, 4)
    xvel0 = s16(await rd(bus, PLAYER_1 + 0x0A, 2))
    await call(bus, "set_profiler", {"enabled": True})
    await call(bus, "run_frames", {"frames": 150})
    prof = await call(bus, "get_profiler_frames", {"frames": 150, "top": 60})
    await call(bus, "set_profiler", {"enabled": False})
    cam1 = await rd(bus, CAMERA_X, 2)
    lag1 = await rd(bus, LAG, 4)
    ypos1 = s16(await rd(bus, PLAYER_1 + 0x06, 2))
    xvel1 = s16(await rd(bus, PLAYER_1 + 0x0A, 2))
    angle1 = await rd(bus, PLAYER_1 + 0x1F, 1)
    await call(bus, "hold", {"buttons": [HOLD], "down": False})

    rec = {"label": LABEL, "rom": ROM,
           "camera_x_start": cam0, "camera_x_end": cam1,
           "px_per_frame": (cam1 - cam0) / 150.0,
           "lag_frames_start": lag0, "lag_frames_end": lag1,
           "lag_in_window": lag1 - lag0,
           "ypos_after_land": ypos_land, "ypos_end": ypos1,
           "xvel_before_window": xvel0, "xvel_end": xvel1, "angle_end": angle1,
           "profile": prof}
    with open(os.path.join(OUT, f"profile_{LABEL}.json"), "w") as f:
        json.dump(rec, f, indent=1)
    print(json.dumps({k: v for k, v in rec.items() if k != "profile"}, indent=1))

asyncio.run(main())
