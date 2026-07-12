# Object-pool occupancy — profile packet (Step 8)

Deliverable for Volence's gate. Measures the occupancy branch against the
tranche-10 pinned baseline (spec §7, gap-ledger:954). Emulator: oracle (plain
`s4.bin`, md5 388a4d86…, freshly reloaded + symbols from `s4.lst`).

## Profiler trust — caching-bug jitter check (MANDATORY, per gap-ledger:978)

The oracle profiler caching bug (served byte-identical data across ROMs / frames)
was fixed 2026-07-12 (oracle `linux-port`). Confirmed the RUNNING oracle has the
fix before trusting any number: two `get_profiler_frames` reads across advancing
frames must DIFFER.

| Routine | read A (30-frame avg) | read B (after +25 frames) | differs? |
|---|---|---|---|
| GameState_OJZScroll_Update | 54,268 | 53,348 | ✓ |
| VBlank_Handler | 37,738 | 37,750 | ✓ |
| VInt_Level | 36,840 | 36,420 | ✓ |
| VSync_Wait | 27,237 | 31,270 | ✓ |

Live data confirmed — the measurements below are trustworthy.

## Scene

`GameState_OJZScroll` (the real boot, no flip), scrolled to the light steady
state: **Player + 2 dynamic objects** (PhysTable slot 39 + TestSolid slot 41) —
matched to the t10 pin's "Player + 2 TestSolid" 3-object scene. NTSC frame
budget 128,000 cycles.

## Headline — RunObjects

| | cycles | % of NTSC frame | dispatch mechanism |
|---|---|---|---|
| **t10 pin (before)** | **11,841** | **9.3%** | fixed 66-slot sweep; dispatch loops $0028B6 ×3 = 9,677 cyc of empty-slot tax |
| **occupancy (after)** | **2,428** | **1.9%** | live-list walk — visits only the ~3 live slots |

**−9,413 cycles (−79.5%).** Stable across reads (2,381 / 2,428 in two windows).

The saving (**9,413 cyc**) tracks the pin's own note that the three fixed
dispatch loops cost **9,677 cyc** — i.e. the win IS the empty-slot tax
elimination, exactly as designed. Spec §7 predicted "drop to roughly a third"
(~3,900 cyc); the actual drop to ~**one fifth** beats that, because the
empty-slot tax was the dominant term, not a third.

## Secondary — TouchResponse (step-4 walker)

TouchResponse (collision, step 4's live-list retrofit) = **1,002 cyc (0.8%)** in
the same scene — also lean (was previously part of the 128-slot-per-frame
collision sweep tax). Not separately pinned at t10, logged here for the record.

EntityWindow_DespawnObjects (step 5) does not surface as its own line at this
occupancy (nested; the entity-window scan total EntityWindow_Scan = 2,341 cyc is
dominated by section/mask work, not the dynamic despawn walk).

## Frame context (why RunObjects is now a minor line)

The light OJZScroll scene is tile-cache / parallax / DMA bound:
GameState_OJZScroll_Update 42%, VBlank_Handler 30%, Tile_Cache_Fill 16%,
Parallax_Update 16%. RunObjects at 1.9% is now a minor contributor — the
occupancy work moved it off the hot path. Further RunObjects wins would be
per-live-object work (real work), not structural.

## Caveats

- The t10 11,841 pin was measured on the pre-branch codebase; exact camera/object
  placement may differ by a few cycles from this scene. The ~5× reduction is far
  larger than any such drift, so the conclusion is robust.
- Measured on the PLAIN shape (shipping). The DEBUG shape carries the §6 asserts
  (step 7) and is not the perf target.
- Scene is a live scroll (spawn/despawn churn), so RunObjects varies ±~50 cyc
  with the exact live count (2–4 objects); 2,428 is representative of the ~3-object
  light scene.

## Verdict

The empty-slot tax is eliminated. RunObjects 9.3% → 1.9% (−79.5%), matching the
spec §7 ceiling and beating its estimate. Ship.
