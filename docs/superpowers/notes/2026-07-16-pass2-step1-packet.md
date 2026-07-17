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

## Step 1.1a — tile_cache #1 FillRow NAMETABLE segments — DONE (2026-07-17)

FillRow's per-tile `.fr_col_loop` → a per-block valid-run `[ic_lo, ic_hi)` +
≤2 `move.w`/`dbf` segment copies for the nametable, split at the Origin_Col wrap.
COLLISION stays per-cell (phase 2, verbatim; 1.1b segments it). No zero segment —
FillRow always SKIPPED out-of-range cols (never zeroed), so the clamp is inherent.
Twins byte-identical (`.emp`/`.asm`), re-pinned (region +$88; engine.inc resume
orgs bumped). **CORRECTION (Fable rider 1, reconciled by rebuild): the 1.1a commit's
tree was NOT strict-clean — `mixed_dac_rom.rs` (mixed_tranche3, both shapes) and
`repin_pins.rs` (secondary_pin_classes) were RED at 1.1a (their hardcoded pins were
stale by 0x88), and the original 1.1a report ("full strict suite green") was WRONG:
the failures were buried by a `grep "test result|…" | head -50` that truncated FAILED
lines among the passing ones. They were surfaced and FIXED as part of 1.1b's ripple
work (F442→F3EC etc.). Detection-hole fix in the bar below.** Region grew, ROMs:
debug 04d352ab / plain bb6f194a (were b1f82f9a / 824d4f2e).

**Identity (Debug_Scene_Freeze=1 + Camera poke → deterministic controlled fill;
the packet's press-count anchor drive is NON-reproducible, replaced by matched-
geometry — gap-ledger d57091c).** Anchor Cam(512,640) → Left=44 Top=64
Origin_Col=44 (**wrap-split exercised**) Origin_Row=2, reached IDENTICALLY OLD+NEW:
- nametable visible window screenshots PIXEL-IDENTICAL; cache-RAM windows byte-identical.
- collision plane A (content) md5 IDENTICAL — untouched confirmed.
- resume contract: budget-limited fills (2 rows/f × 5 blocks > BLOCK_DECOMP_BUDGET 6
  → 2nd-row budget-out) reached byte-identical settled cache vs OLD; deep scroll
  settles clean, no orphaned rows.

**Producer A/B (max-V, sustained down-scroll, 60-frame avg, same drive both ROMs):**

| routine | OLD | NEW | Δ | 45ca85d anchor |
|---|---|---|---|---|
| TileCache_FillRow | 45798 (35.8%) | 29894 (23.4%) | **−15904 / −34.7%** | 45903 (35.9%) |
| Tile_Cache_Fill (incl) | 53435 (41.7%) | 37675 (29.4%) | −15760 | 53555 (41.8%) |
| Draw_TileRow_FromCache | 13416 | 13416 | 0 (untouched) | 13416 |
| VSync_Wait (idle) | 31373 (24.5%) | 47147 (36.8%) | +15774 freed | 30937 |
| Lag_Frame_Count | 0 | 0 | 0 | 0 |

OLD reconciles with 45ca85d to ~100 cy (validates method+anchor). VBlank-Δ = 0
(1.1a is pure producer; VInt_DrawLevel/drain untouched). Diagonal: FillRow reduced
(17416 vs 20756 anchor), Lag=0 — no shared-budget regression (decompress cap
untouched). max-H: FillRow doesn't run (FillColumn regime = later step). Win lands
entirely as freed producer headroom, Lag stays the 0 floor.

## Step 1.1b — tile_cache #1 FillRow COLLISION segments — DONE (2026-07-17)

Phase-2's per-cell collision loop → segmented `move.b`×2-plane runs over the SAME valid
run `[ic_lo, ic_hi)` with the SAME Origin_Col wrap-split. **Additive**: the range+split are
re-derived independently, so phase 1 (nametable) stays byte-for-byte as 1.1a — a fault here
can only touch collision (clean bisect). `move.w`/`move.l` byte-pairing deferred to the
ledgered step-5 rider (row 1092). Twins byte-identical (gate both shapes); region +$56.
Ripple: pins re-pinned, engine.inc 4 resume orgs, **mixed_dac_rom.rs** Collision_GetType
`bra` disp (F4CA→F3EC plain / F40A→F32C debug), **repin_pins.rs** SOUND_API base +0xDE.
Full strict suite 2262/0. ROMs debug e15b6ff7 / plain f7faf57c.

**Identity (canonical Debug_Scene_Freeze method; OLD = baseline b1f82f9a, whose per-cell
collision output == 1.1a's).** BOTH collision planes full byte-compare (md5 + `wc -c` length
assert, pipeline rule — no hand-transcription) at TWO anchors varying the wrap-split:
- A1 Cam(512,640) Left=44 **Origin_Col=44** (run-1 long): plane A md5 `3dab4323…` OLD==NEW.
- A2 Cam(768,640) Left=76 **Origin_Col=76** (run-1 only 4 cols, run-2 long): plane A md5
  `05b25a27…` OLD==NEW (different terrain).
- plane B all-zero both; nametable window byte-identical (phase-1 untouched); geometry matched.
- **Closes 1.1a plane-B debt**: baseline plane B == 1.1b plane B == 1.1a plane B (same bytes).
- **Plane-B sentinel test (Fable rider 2) — PASS.** The 2-anchor plane-B compare above was
  zero-over-zero (no discriminating power). So: `write_memory 0xEE` across the FULL plane-B
  dest cache (`0xFF2EE0`+2400 B) on OLD and NEW, controlled fill at the matched anchor
  (Cam 512→640, FillRow rewrites 16 rows over the sentinel), read back. Result IDENTICAL on
  both ROMs: the fill wrote `0x00` (this terrain's plane-B data) into exactly the recycled
  cells, boundaries at **byte 80 (00→EE)** and **byte 1839 (EE→00)** on BOTH — a wrong
  plane-B dest (a6) would shift these; and the `0xEE` sentinel SURVIVED in the entire
  un-refilled band (mid-window witness all-EE) — proving the clamp writes NOTHING outside
  the valid window (the strongest clamp artifact). Boundary md5s 6fee342f / 947a2c49 match.
  (Tooling note: the full-plane md5-to-file was blocked by the agent's output-token limit on
  emitting 2400-byte hex — reads are free, large WRITES are not; verified via read-confirmed
  boundary offsets instead. Reinforces gap-ledger 1091's `emulator_memory_hash` ask.)
- Resume: budget-out lands at a BLOCK boundary (before `.fr_have_block`), so a collision
  segment is always atomic — never split mid-segment; both anchor drives were budget-limited
  (2 rows/f × 5 blocks > BLOCK_DECOMP_BUDGET 6 → mid-row resume) and matched baseline byte-
  identically, verifying the resumed blocks' collision segments.

**Producer A/B (max-V, same drive all ROMs, 60-frame avg, Lag 0):**

| routine | baseline | 1.1a | 1.1b | Δ vs 1.1a | Δ vs baseline |
|---|---|---|---|---|---|
| TileCache_FillRow | 45774 (35.8%) | 29894 (23.4%) | 18023 (14.1%) | −11871 / −39.7% | −27751 / −60.6% |
| Tile_Cache_Fill (incl) | 53555 (41.8%) | 37675 | 25472 (19.9%) | −12203 | −28083 |
| VSync_Wait (idle) | 31169 (24.3%) | 47147 | 59339 (46.4%) | +12192 | +28170 |
| Lag_Frame_Count | 0 | 0 | 0 | 0 | 0 |

The collision segmentation cut another ~11.9k cy; the two FillRow restructures together cut
FillRow **~60%** (max-V), freeing ~half the frame to idle. VBlank-Δ = 0 (pure producer).

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

### CANONICAL identity bar (Debug_Scene_Freeze matched-geometry — 1.1a/1.1b/1.3)

The A1–A4 **press-count anchor drive below is DEAD** — reset+press is NOT frame-
deterministic on this oracle (boot jitter + lag → same input lands at different camera
positions; observed `Origin_Col` 0/2/74 for identical `start300;right200`; the vertical
free-camera falls chaotically and won't idle-stabilize). Byte-identity of a streamed
cache across two ROMs needs a MATCHED camera position, which input cannot deliver.

**The canonical method (proven 1.1a):** `Debug_Scene_Freeze` (`0xFF8A10`, __DEBUG__)
= 1 makes `GameState_OJZScroll_Update` skip `Camera_Update` + `EntityWindow_Scan`, so a
poked `Camera_X`/`Camera_Y` PERSISTS while `Tile_Cache_Fill` still runs — the cache
streams to exactly the poked camera. Identical pokes on OLD and NEW ⇒ identical FillRow/
FillColumn input ⇒ byte-identical cache. `Camera_X/Y` are LONGs (hi word = px, lo =
subpixel): poke `.l`, e.g. 512 px = `0x02000000`. Steady cache is a pure fn of position:
`Origin_Col = Left mod 80`, `Origin_Row = (Top−2) mod 60`, `Left = Cam_X/8 − 20`,
`Top = Cam_Y/8 − 16`. Recipe: `reset; start 300` → freeze=1 → poke Camera → step
`mode` (neutral) frames to settle (`RowResume` 0xFFA870 = `$FFFF`); poke Camera_Y DOWN
in ≤16-row increments (settling between) to exercise FillRow `.v_bottom_fill`.

**Comparison via the pipeline, NOT hand-transcribed hex (gap-ledger row 1091 rule):**
`emulator_screenshot path=…` writes the PNG to disk (transcription-free) → `cmp` the
visible plane; collision / off-screen cache via `md5sum` of a `read_memory` dump file
with a `wc -c` == 2×len assert immediately after each Write; prefer ≤256 B fresh reads
compared in-context over 4 KB pastes.

**Coverage argument (why one wrap-exercising full fill suffices — an argument, not an
anecdote).** One deterministic full fill at `Origin_Col ≠ 0` (1.1a used Cam(512,640) →
Left=44 Top=64 **Origin_Col=44**) crosses EVERY nametable emit path in a single anchor:
- **pre-wrap-only run** (blocks whose `cache_col + Origin < 80` throughout → run-2 empty),
- **already-wrapped single run** (a block entered with `phys_start ≥ 80` → the
  subtract-COLS branch, run-2 empty),
- **the split** (a block straddling `cache_col = 80 − Origin` → run-1 + run-2), which is
  guaranteed to occur once per full row because a 60+-row × 80-col fill sweeps the wrap
  boundary at every row,
plus the **Left/Head partial blocks** (first block clamped by `ic_lo = Left−B`, last by
`ic_hi = Head−B+1`) at the cache edges, and BOTH **row parities** (collision on odd rows
via phase 2, skipped on even). So a single matched anchor is exhaustive path coverage,
not a shortcut — additional anchors only re-roll the same paths at different offsets.

**1.1b (collision) specifics:** 2–3 poked anchors varying `Origin_Col` AND row parity;
BOTH planes full byte-compare (md5 + length assert); plus a budget-out resume that lands
INSIDE a collision segment (not just at a block boundary). **Plane-B debt from 1.1a:**
1.1a hashed plane A only (collision was verbatim) — capture 1.1b's OLD baseline on the
CURRENT NEW-1.1a ROM reading BOTH planes, which retroactively closes plane B for 1.1a at
zero extra cost.

### Byte-changing RIPPLE SWEEP — MANDATORY per piece (Fable rider 1)
A byte-changing tile_cache edit ripples past pins.rs into **FIVE** sites. 1.1a touched
two and nothing flagged the other three — the 1.1a report rode green on a stale hardcoded
pin. Every piece (1.1b/1.2/1.3/…) MUST, in order:
1. build DEBUG then plain (repin-trap: build.sh writes s4.bin regardless of DEBUG — build
   debug first + `cp` to s4.debug.*, plain last), `repin` (writes pins.rs), `repin --check`.
2. Sweep all five ripple sites, ticking each: **(a) pins.rs** (repin), **(b) engine.inc**
   resume orgs, **(c) mixed_dac_rom.rs** hardcoded cross-seam disps/pins, **(d) repin_pins.rs**
   hand-typed baseline, **(e) repin.toml** region start/end symbols (only if a region's
   boundary symbols changed — usually untouched, but VERIFY, don't assume).
3. Run the FULL strict suite AFTER repin, and paste the count PER PIECE as
   `N passed / M failed` — with a **failures-first** command so a red line can NEVER be
   truncated behind passing lines:
   `cargo test --workspace 2>&1 | tee suite.log; grep -E 'FAILED|panicked' suite.log;
    echo passed=$(grep -oE '[0-9]+ passed' suite.log|awk '{s+=$1}END{print s}')
    failed=$(grep -oE '[0-9]+ failed' suite.log|awk '{s+=$1}END{print s}')`.
   NEVER `grep "test result" | head -N` (the 1.1a detection hole — it buries FAILED among
   the ~160 `ok` lines). The packet row records `passed/failed` + the five ticks.
1.1b sweep (this piece): (a) pins ✓ (b) engine.inc 4 orgs ✓ (c) mixed_dac_rom disp F3EC/F32C ✓
(d) repin_pins SOUND_API +0xDE ✓ (e) repin.toml unchanged (tile_cache start/end symbols
stable) ✓ → **2262 passed / 0 failed**.

### Resume protocol (cache population — still useful)
The emulator holds probe residue; short boots don't clear it. `reset → start 300 →
sustained drive` repopulates the cache; confirm `Tile_Cache_Nametable` (0xFF0000) = real
art_tile words (e.g. `C807 C014…`), NOT zeros. Cache RAM map: `Tile_Cache_Nametable`
0xFF0000 (9600 B), `Tile_Cache_Collision` 0xFF2580 (plane A 2400 B) + 0xFF2EE0 (plane B
2400 B). Note OJZ foreground content is BANDED — deep scroll exits it (empty), Left past
the H-extent is empty; verify the cache isn't all-zeros before trusting a compare.

### DEAD press-count drive (kept for provenance only — DO NOT USE)
~~reset → start 300 → right 200 [A1] → down 90 [A2] → up 60 [A3] → down+right 90 [A4]~~
— non-reproducible (see above); superseded by the canonical method.
