# Tranche 16 — tile_cache oracle probe (R2, pre-step-5 worst-case)

**Run:** oracle, FOREGROUND, worktree `s4.debug.bin` (loaded by explicit path;
hash = my fresh build **b0ceca0b** — rider 4 satisfied). Scene = `GameState_OJZScroll`
(the game boots straight into it; `Game_Entry = GameState_OJZScroll_Init`), pinned
via `Debug_Scene_Freeze=1` and driven by poking `Camera_Y` a fixed +Δ/frame then
advancing exactly 1 frame — exact px/f control. Symbols from `s4.debug.lst` (941).

## Numbers

### 16 px/f (2 rows/frame — the worst case; VInt_Lag already fired at 8 in t15), 20-frame avg spanning ~2 block-row crossings
Profiler cycles are INCLUSIVE (self+callees); exclusive = inclusive − named callees.

| Routine | Inclusive | Exclusive | % frame |
|---|---|---|---|
| `Tile_Cache_Fill` | 34581 | ~1.6k (frame gate + budget + edge calc + prefetch) | 25.0% |
| **`TileCache_FillRow`** | **32490** | **~27.9k (its own `.fr_col_loop`)** | **23.5%** |
| `TileCache_DecompressBlock` | 3414 | ~2.8k (−S4LZ_DecompressDict 607) | 2.5% |
| `TileCache_FindStagedBlock` | 1736 (6 calls) | 1736 (leaf) | 1.3% |
| `TileCache_CopyBlockColumn` | **0 — NOT on the vertical path** | 0 | 0% |

Context (not tile_cache-owned): `Parallax_Update` 25178 (18.2%, co-equal, separate
domain), `Section_UpdateColumns` 8798 (6.4%), `Draw_TileRow_FromCache` 7716 (5.6%,
the VInt-side NT draw). **`VInt_Lag` = 476 (0.3%) — NONZERO**; frame budget 108.1%;
`VSync_Wait` idle 37.1% avg (cheap frames idle big, crossing frames overrun — the
average hides the per-crossing spike).

### 8 px/f (1 row/frame), 10-frame avg
`Tile_Cache_Fill` 21827 (14.7%), `TileCache_FillRow` 20069 (13.6%),
`DecompressBlock` below top-12 (≈0 — few crossings), `Draw_TileRow_FromCache`
5368 (3.6%). **No `VInt_Lag`**, `VSync_Wait` idle 46.9% — comfortable headroom.

### Correctness at 16 px/f (deliverable #3)
Cache KEEPS UP: over 20 frames Cache_Bottom tracked the camera at ~2 rows/frame
(69→99 vs +32 ideal = ~2 rows transient debt, never accumulating). Mid-scroll
screenshot at the leading edge = CLEAN (proper ground/foliage, no stale-tile
garbage, no torn seam). The `VFILL_ROWS_PER_FRAME=2` cap + `MARGIN_V=16` absorb
the 2-rows/frame demand; no fill-debt corruption.

## DECISION (the step-5 scope input)

**FillRow's own per-cell loop dominates (~27.9k excl at 16 px/f) — DecompressBlock
(~2.8k excl) is an ORDER OF MAGNITUDE smaller.** The prefetch + round-robin staging
cache already amortize decompress well (FindStagedBlock hits; DecompressBlock only
at crossings, cheap). CopyBlockColumn is not on the vertical path at all.

Per the ratified criterion (Fable/Volence): DecompressBlock does NOT dominate →
the lever is **NOT architectural** (no pre-staging/amortization redesign). It is
the **invariant hoist + strength-reduction in `TileCache_FillRow`'s `.fr_col_loop`**
— the three loop-invariant `lea (base).l` reloaded per cell (donor :1129/:1140/:1147,
a1 repurposed mid-cell for plane-B) + the per-cell circular column wrap. **Mine to
take at step 5**, under the target: **2 rows/frame with zero VInt_Lag** (clear the
108% / VInt_Lag=476 marginal crossing-frame overrun).

## Caveats / jots
- Exclusive = inclusive − NAMED callees; a small unnamed remainder is possible, but
  the FillRow-≫-DecompressBlock conclusion is robust to it (10× gap).
- `Parallax_Update` (18–22%) is co-equal to the fill but a SEPARATE domain (OJZ
  parallax, not tile_cache) — ledger jot as the next lag lever after tile_cache,
  NOT t16 scope.
- The t15 pre-recon jot (row 1057) cited FillRow 48.9k/38.2%; this measures 32.5k/
  23.5% at 16 px/f. The t15 figure was a heavier/colder or differently-driven
  sample; the RELATIVE conclusion (FillRow's own loop is the lever) stands and is
  now measured exclusive-vs-inclusive. Supersede row 1057's absolute with these.

## Refreshed baseline — post-step-2 ROM (Rider 3, before any FillRow edit)

Re-probed the SAME scene/method on the post-step-2 ROM (worktree s4.debug.bin =
canonical **88354f87**, verified by explicit-path reload; steps 3/4 were .emp-only
byte-neutral so gate-off behavior = post-step-2). Identical Camera_Y schedule
($00900000 start, +16 px/frame × 20) so the A/B against the post-step-5 re-probe
is apples-to-apples (same crossing positions).

**20-frame 16 px/f:** `TileCache_FillRow` **26219 (19.7%)**, `Tile_Cache_Fill`
**27785 (20.9%)**, `Parallax_Update` 23961 (18.0%), `Draw_TileRow_FromCache` 6710
(5.0%), `Section_UpdateColumns` 7645 (5.7%). budget **103.9%**, `VSync_Wait` idle
**43.6%**, **`VInt_Lag` absent (< top-14)**, `DecompressBlock`/`FindStagedBlock`
also < top-14 (this window's crossings hit the staging cache — the leftover-budget
prefetch amortized them; contrast R2's heavier window with DecompressBlock 3414 +
VInt_Lag 476).

**Read for step 5:** FillRow's OWN per-cell loop (the hoist+SR target, crossing-
independent) ≈ 25-26k cyc/f — consistent with R2's ~27.9k; the inclusive/window
variance is whether a cold DecompressBlock crossing folds in. The step-2 branch
relaxations shaved a touch (budget 108→104% vs R2). **This is the step-5 accounting
baseline.** NOTE on the exit criterion: VInt_Lag firing is INTERMITTENT — it hits
only when the prefetch falls behind on a crossing frame (R2 caught one; this window
didn't). So verifying "zero VInt_Lag at 16 px/f" post-optimization means re-running
the worst case (and if this schedule won't reliably lag, the pre-sanctioned debug
scroll-speed knob / a longer sustained run stresses it). The hoist+SR lowers EVERY
frame's FillRow ~6-8k → more headroom for the crossing spikes regardless.
