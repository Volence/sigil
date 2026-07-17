# Pass 2 — step-1 packet (the two dominant copy-segment restructures + #5)

**Status:** IN PROGRESS. Step-0 baseline DISCHARGED (below); restructures pending.
HOLD-for-Fable at R2 after all three commits + the after-numbers reconciled.

## Step 0 — Probe-(ii) A/B BASELINE (canonical `b1f82f9a` / `824d4f2e`)

**Method:** unfrozen player-driven drive (deterministic from reset → start 240f →
hold direction 150f), profiler averaged over the last 60 frames; the state-counter
lag = `Lag_Frame_Count` ($FF89F8). Unfrozen ⇒ real load (EntityWindow_Scan /
RunObjects present, unlike the frozen Probe-A). Reproducible: same input sequence
reproduces the numbers. ROM hash-verified `b1f82f9a` before measuring. NOTE: the
player-driven window AVERAGES crossing + non-crossing frames, so the copy-lever %s
are lower than Probe-A's sustained-crossing 35% — the ANCHOR is this drive's
before/after delta, not the absolute %.

**Baseline table (cycles/frame, 60-frame avg; budget 128000 cy/frame):**

| routine | max-H (→) | max-V (↓) | diagonal (↓→) | budget/axis |
|---|---|---|---|---|
| Tile_Cache_Fill (inclusive) | 18.7% / 23953 | **41.8% / 53555** | 16.6% / 21238 | producer |
| **TileCache_FillRow** | — | **35.9% / 45903** (2) | 16.2% / 20756 (1) | producer **V** — #1 |
| **TileCache_FillColumn** | **13.4% / 17184** | — | 7.8% / 9953 | producer **H** — #1 |
| **TileCache_CopyBlockColumn** | 9.3% / 11869 (6) | — | 4.7% / 5999 (4) | producer **H** — #5 |
| **Draw_TileRow_FromCache** | — | 10.5% / 13416 (2) | 4.7% / 5999 (1) | producer **V** — (b) |
| Draw_TileColumn | 4.7% / 5978 | — | 2.5% / 3150 | producer H — (a) |
| VInt_DrawLevel | 2.3% / 2985 | 1.7% / 2236 | ~2% (unrec.) | **VBlank** — clamp/(c)/(d) |
| Section_UpdateColumns | 5.3% / 6740 | 11.3% / 14498 | 7.5% / 9595 | producer — H1 (later) |
| TileCache_DecompressBlock | 3.0% / 3882 | 3.1% / 3924 | 9.9% / 12680 (2) | producer — prefetch |
| VSync_Wait (idle headroom) | 25.2% / 32311 | 24.2% / 30937 | 22.2% / 28434 | — |
| **Lag_Frame_Count** | **0** | **0** | **0** | — |
| Camera reached | X $04→$09C0 (2496px) | Y $04→$0CD0 (3280px) | both | — |

**Reading — what step 1 targets:**
- **max-V is the heaviest regime** (Tile_Cache_Fill 41.8%, only 24% idle) — and it's
  where the two V-restructures live: **FillRow 35.9%** (#1 V) + **Draw_TileRow
  10.5%** ((b)). The single biggest lever in the whole pass.
- **max-H**: FillColumn 13.4% (#1 H) + CopyBlockColumn 9.3% (#5) + Draw_TileColumn
  4.7% ((a), step 4).
- **diagonal** spreads the load across both axes (FillRow 16.2% + FillColumn 7.8%)
  and adds a DecompressBlock spike (9.9%, both axes crossing) — the shared-budget
  case; the restructures must not regress it (they cut copy cost, don't touch the
  decompress cap).
- **All Lag=0** — the scene has 22-25% idle headroom, so step-1 wins register as
  PRODUCER-cycle reduction (FillRow/FillColumn/CopyBlockColumn) + VBlank reduction
  (VInt_DrawLevel, from the Probe-(i) clamp dropping the zero words) + freed
  VSync_Wait headroom, NOT a lag-counter drop. The absolute lag clear is a
  no-regression check (stays 0); the win is measured in cycles + headroom.

**Watch item (Fable, standing):** Draw_TileRow's LEFT-BEHIND cells (cols <
Cache_Left_Col) are margin-based, not ring-back — the leading-edge mid-scroll
screenshot + the up+left drive stay in EVERY after-A/B regime set.

---

## Step 1.1 — tile_cache #1 (FillColumn + FillRow segments) — PENDING
## Step 1.2 — plane_buffer (b) (Draw_TileRow_FromCache segments) — PENDING
## Step 1.3 — tile_cache #5 (CopyBlockColumn wrap-split) — PENDING

_Each: twin lockstep (.emp + .asm), re-pin, byte gates both shapes, per-group A/B
(the 3-regime table above re-measured, producer-Δ AND VBlank-Δ per row), resume-
contract check (#1's budget-out at `.fr_block_loop` head survives intra-block
segmenting), clamp correctness rail (leading-edge screenshot + up+left drive)._

---

## Verification setup (Fable's amended bar, 2026-07-17) + resume protocol

**Split ruling:** 1.1a (FillRow NAMETABLE-copy) and 1.1b (FillRow COLLISION-copy) are
separate commits (bisectable pieces on the riskiest change). Order: 1.1a → 1.1b →
1.2 → 1.3, R2 HOLD after 1.3.

**Amended identity bar (fitted to oracle's tooling — no memory-hash/dump/set-PC, see
gap-ledger d57091c):**
- **Collision (1.1b):** FULL cache-RAM byte compare, BOTH planes (2 reads × 2400 B/ROM
  per anchor) at 2–3 anchor frames — NON-NEGOTIABLE (collision never reaches VRAM; no
  screenshot/VRAM compare can see it; player-doesn't-fall-through is too coarse).
- **Nametable (1.1a/1.3):** `read_vram` visible-window identity across the WHOLE drive
  at multiple frames (scrolling → transitive coverage) PLUS targeted cache-RAM reads at
  the WRAP SEAM (`Cache_Origin_Row/Col` ±2) and the leading edge, same anchors. Bounded
  (few hundred B/anchor, not 14400).
- All pieces: screenshots both directions + resume-contract artifact (poke
  `Cache_Fill_Budget` mid-row → identical completion) + 3-regime profiler A/B vs the
  45ca85d anchor. **1.2** keeps its own bar (VRAM visible-window; identity N/A — the
  clamp changes buffer content by design).

**Resume protocol (the stale-session fix — confirmed working):** the emulator holds
probe residue; short boots don't clear it. **reset → press start 300 → sustained drive
(≥~200f)** repopulates the cache. Confirm before measuring with one read at
`Tile_Cache_Nametable` (0xFF0000) = real art_tile words (e.g. `C807 C014…`), NOT zeros.

**Confirmed geometry (anchor A1 = reset→start 300→right 200, canonical b1f82f9a):**
`Cache_Left_Col=352 Head=431 Top_Row=2 Bottom_Row=61 Origin_Col=32 Origin_Row=0`.
Cache RAM: `Tile_Cache_Nametable` 0xFF0000 (9600 B), `Tile_Cache_Collision` 0xFF2580
(plane A 2400 B) + 0xFF2EE0 (plane B 2400 B). VRAM Plane A window 0xC000 (`read_vram`).
Wrap seam this anchor: physical col 32 (Left maps there), physical row 0.

**Deterministic anchor drive (replay on OLD + NEW):** reset → start 300 → right 200
[A1: H] → down 90 [A2: V + crossings] → up 60 [A3: direction flip] → down+right 90
[A4: diagonal]. Capture identity data at A1–A4.
