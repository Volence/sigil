# Tranche 16 step-5 Wave 2 — crossing-decompress amortization (design note)

**Status:** design deliverable, GATED before any code (Fable rules on the recommendation).
**Problem (from the Wave-1 A/B + f5801df positive control):** at 16 px/f the lagging
frames are cold block-row CROSSINGS. Wave 1 (FillRow hoist) cleared the warm-window
overrun (budget 103.9→100.0%) but the control window is still **107% ≈ ~9k over** —
the cold-crossing DecompressBlock spike. Wave 2 owns that ~9k.

## 1. Current mechanics (as-built)

**Fill** (`Tile_Cache_Fill`, per frame): H-fill columns then V-fill rows, both
budget-limited. `BLOCK_DECOMP_BUDGET = 6` decompresses/frame, SHARED across H-fill +
V-fill + prefetch. `VFILL_ROWS_PER_FRAME = 2` rows/frame cap. A **resume ladder**
(`Cache_Fill_Resume_Col/Row` for columns, `Cache_Fill_RowResume_Row/Col` for rows,
`Cache_Fill_Rows_Left`) stores mid-fill budget-out state; next frame's preamble
finishes the pending partial FIRST.

**Prefetch** (leftover-budget, after V-fill): if `Cache_Fill_Budget > 0`, direction-
sensed via `Cache_Prev_Cam_Row` delta (+down / −up), target = the block-row just
below cache bottom (down) / above cache top (up), **the single block under camera
CENTER X** (`Camera_X + 160`). `FindStagedBlock`; on miss, `DecompressBlock` (ONE
block) + `subq budget`. Grid-bound guards (sec_x < grid_w, sec_y < grid_h).

**Staging:** `BLOCK_STAGE_SLOTS = 12`, round-robin evict (`Block_Stage_Next`), keyed
`(sec_x|sec_y|block_index)`.

**THE GAP:** the prefetch re-probes the SAME center block every quiet frame — after
the first stage it's a `FindStagedBlock` hit (no decompress), so frames 2..8 of the
quiet window are WASTED. Only ~1 of a crossing's ~5-6 blocks is ever pre-warmed; the
other ~5 decompress AT the crossing (inside FillRow's block loop) → the spike.

## 2. Demand profile

- Cache width `TILE_CACHE_COLS = 80` tiles = 5 blocks; + horizontal margin overlap →
  **~5-6 blocks span a filled row.**
- At 16 px/f = 2 rows/frame, a block-row (16 rows) is crossed **every 8 frames**.
- **Crossing spike:** ~5 cold `DecompressBlock` (~1.8k inclusive each incl. S4LZ) ≈
  9-11k added to the crossing frame → ~14k total → 110.8% → VInt_Lag.
- **8-frame quiet window** between crossings: fill is cache-hits (warm) → leftover
  budget available → prefetch runs (but wastes it re-warming center).
- **Cold-start regime (= the control):** right after `Tile_Cache_Init`/`FillAll`
  (which warm ONLY the initial viewport window), the first scroll crossings are FULLY
  cold — the prefetch hasn't targeted the new blocks yet. Worst case; what f5801df hits.

## 3. Candidate mechanisms (each: quiet-frame cost + risk)

### (i) Staged-count-aware prefetch — "stage the next row's unstaged blocks, ≤k/frame"
Replace "re-probe the center block" with: enumerate the next block-row's ~5-6 blocks
(iterate block_x across the cache col span at the target block-row), and each quiet
frame stage the NEXT UNSTAGED one (FindStagedBlock per candidate, DecompressBlock the
first miss), ≤k/frame. Over the 8 quiet frames, all ~6 get staged → **crossing is
warm → no spike (PREVENTS it at source).**
- *Quiet-frame cost:* ≤k decompresses/frame (was ≤1-but-wasted). 6 blocks / 8 frames
  = 0.75/frame avg → **k=1 suffices** if it targets DIFFERENT blocks; k=2 clears faster
  with margin.
- *Risk:* new enumeration + staged-tracking logic (walk the block_x set, skip staged);
  SLOT PRESSURE (§4); eviction-order (round-robin must not evict a just-pre-staged
  next-row block before the crossing consumes it).

### (ii) Cold-start warmup — Init/FillAll also pre-stages the adjacent block-row
After the initial `FillAll`, decompress+stage the block-row just below the initial
window (the likely first scroll direction) so the FIRST crossing is warm.
- *Cost:* ~6 extra decompresses at Init — ONE-TIME, display-OFF (Init is already ~10
  synchronous frames; no lag concern).
- *Risk:* minimal (off-hot-path). But addresses ONLY the cold-start regime — steady
  crossings still spike unless combined with (i). Direction assumption (down); an
  up-scroll from spawn wouldn't benefit.

### (iii) Budget-cap + resume-ladder spread — cap crossing decompresses, spread as fill debt
Lower the per-frame decompress cap (`BLOCK_DECOMP_BUDGET` 6→~3) so a crossing frame
decompresses ≤3 blocks; the existing resume ladder defers the rest to next frame (the
row fills as a partial → resume). The spike SPREADS over ceil(6/k) frames instead of
one.
- *Cost:* the crossing's ~6 decompresses over 2 frames (3+3), each ≤ budget → each
  frame under deadline. The new row's late columns lag 1 frame (fill DEBT).
- *Risk:* deferred columns show stale tiles for 1 frame — but `MARGIN_V = 16` leads
  the visible edge by 16 rows / 8 frames, so the partial is OFF-SCREEN (the Wave-1
  correctness probe PROVED 2-row transient debt absorbs cleanly). Steady-state
  SUSTAINABLE: 6 blocks per 8-frame window at cap 3/frame = 24 capacity ≫ 6 needed;
  up-scroll symmetric. BUT the cap is SHARED with the column path (§5) → a fast
  diagonal scroll could starve H-fill; and the cap alone may not fully clear <100%
  (capping at 3 removes ~half the spike ≈ 5k off ~9k → ~103%, still over) → likely a
  COMPLEMENT, not a standalone fix.

## 4. Staging-slot pressure

12 slots. Current: a crossing needs ~6 (the new row). **(i)** deepens to ~12 (current
row 6 + next row 6 pre-staged) → **exactly fits 12, tight** — the round-robin evict
(`Block_Stage_Next`) must not evict a pre-staged next-row block before the crossing
consumes it (a pre-staged block sits in staging for up to 8 frames; 8 frames × the
current-row fill's own stagings must not lap it). This is the main (i) risk; may need
BLOCK_STAGE_SLOTS ↑ or eviction-order awareness. **(iii)** does NOT deepen staging
(same ~6, spread in time) → no slot pressure. **(ii)** stages +6 at init, evicted as
scroll proceeds → fine.

## 5. Column-path budget interaction

`BLOCK_DECOMP_BUDGET` is SHARED H-fill + V-fill + prefetch. **(iii)**'s lower cap
slows ALL three → a fast DIAGONAL (H+V) scroll could starve column fill (the H-fill
that keeps the horizontal edge current). **(i)** uses LEFTOVER budget (after fill), so
it doesn't starve fill — but competes for the leftover on diagonal frames (less
leftover → slower pre-staging → the crossing may still be partly cold). The diagonal
worst-case must be in the A/B.

## 6. Up-scroll symmetry

The prefetch already branches `.pfx_up`/`.pfx_go`. **(i)** enumerates the up block-row
the same way. **(iii)** cap is direction-agnostic. **(ii)** init warmup assumes down
(up-from-spawn uncovered — acceptable, spawn is typically ground-level).

## 7. A/B plan

- **Lag fix:** the f5801df control ($00900000 +16px/f, first-8-frames): **budget <100%
  + VInt_Lag → 0.**
- **No regression:** the warm 20-frame window (FillRow/Parallax/Section unchanged from
  Wave-1's 23815/24152/7645).
- **Correctness:** leading-edge mid-scroll screenshot + cache-keeps-up (Cache_Bottom
  tracks camera, no stale-tile corruption) — CRITICAL for (iii) (introduces debt).
- **Cold-start AND steady:** the control is cold-start; ALSO test a steady crossing
  (past warmup) — the lag must clear in both.
- **Diagonal (H+V) scroll:** the shared-budget starvation check (§5).

## 8. Recommendation (Fable rules)

**Primary: (i) staged-count-aware prefetch, complemented by (ii) cold-start warmup.**

Reasons:
1. **(i) PREVENTS the spike at source** — a warm crossing does 0 decompresses → the
   crossing frame ≈ a steady frame (100%), the most robust clear of the overrun. It
   directly fixes the identified gap (the prefetch wastes 7 of 8 quiet frames
   re-warming one block).
2. **It reuses the existing prefetch cadence** — a TARGETING refinement (which block
   to stage), not a new subsystem; the 8-frame quiet window × leftover budget already
   has the ~6-block capacity.
3. **(ii) covers what (i) structurally can't** — the first crossing, before any quiet
   frames have run (the cold-start / control regime). It's cheap and off-hot-path.

**(iii) is the fallback/complement**, not the primary: it only SPREADS the spike (may
leave ~103% residual) and shares the column budget (diagonal starvation risk) — but
it's the simplest and its debt-absorption is already proven, so it's the safety net if
(i)'s slot pressure (§4) proves intractable within 12 slots.

**Open question for the gate:** (i)'s 12-slot tightness — accept BLOCK_STAGE_SLOTS ↑
(RAM cost: +768 B/slot in `Block_Stage_Buffers`), or add eviction-order awareness, or
fall back to (i)-with-k=1-and-(iii)-cap as a hybrid. Fable's call before code.

---

## Wave-2 (i) BEHAVIOR VALIDATION — clean A/B, W2(i) half (2026-07-15)

**Method wall cleared.** Two profiler approaches (aggregate window, then per-frame
`get_profiler_frames(1)`) both failed on frame-advance non-determinism (ledger row: press
advances 1-2 game frames; profiler `calls` unreliable). The fix (Fable's primary metric):
read `Block_Stage_Next` ($FFA8A8, word, mod-12 — bumped once per `TileCache_DecompressBlock`
call, unconditional) and take its **delta between frame boundaries** = decompresses/frame.
State, not a sample → immune to the timing non-determinism. Frames delimited by
`run_to_scanline(224)`+wait; every sample indexed by the `Frame_Counter` VALUE read
(FC delta varied +1/+2 per step — harmless, indexing absorbs it).

**Harness (deterministic, replayable for W1):** reset → press start 8f → press start 120f
(→ `Game_State`=6 OJZScroll, FC=0x0230, tile cache live) → `Debug_Scene_Freeze`=1 (byte!)
→ per step: poke `Camera_Y += $00100000` (16 px, 16.16 fixed) → `run_to_scanline(224)` →
read `Block_Stage_Keys`+`Block_Stage_Next` (one 50-byte read @ $FFA878) + `Frame_Counter`.
ROM hash-verified W2 debug **2b57bf0c** (reload_rom explicit path) BEFORE measuring.

**W2(i) fingerprint** (2-frame buckets, Camera_Y in px → decompress-count via Next delta):
```
144→160  +1     Keys: (boot-staged 24,30-34,40-43,50)
160→176  +5     ← cold-start catch-up transient (first real motion off the frozen boot)
176→192  +1     Keys slot2: 24→51   (0x5x row staging begins)
192→208  +1     Keys slot3: 30→52
208→224  +1     Keys slot4: 31→53
224→240  +1     Keys slot5: 32→54   ← 0x5x row now COMPLETE (50,51,52,53,54)
240→256  +0     (quiet — 0x5x fully staged, camera inside cached window)
256→272  +0
272→288  +0
288→304  +1     Keys slot6: 33→60   (0x6x row staging begins — no spike)
```
Total = **11 decompresses over 160 px** (Camera_Y 144→304; Block_Stage_Next 8→7 mod-12).
That 160 px is **~1.25 block-rows** (a block-row is ~128 px), spanning **two row-onsets**
(0x5x at ~176, 0x6x at ~288) **plus the cold catch-up** at ~160. (Earlier prose "10 / 144 px
/ 9 block-rows / ~1 block per row" was wrong arithmetic — the table is authoritative; a
"row" here is the ~128 px vertical block-row the tags 0x5x/0x6x each represent, NOT a 16 px
step.)

**Reading:** this IS Fable's predicted "(i) working" fingerprint — **flat, low (0-1/bucket),
next-row tags accumulating ONE PER QUIET FRAME** (watched 0x5x build 50→51→52→53→54 one
block per step, then 0x6x start at 60). The Keys array is direct mechanistic evidence: the
k=1 staged-count-aware scan stages the next row ahead of demand, so row crossings cost 0-1,
NOT a 5-6 spike. The single +5 is a cold-start transient (first motion off frozen boot), not
a steady-state spike — no spikes recur at any of the 3 row crossings observed (0x5x, 0x6x
onsets both smooth). No tags observed vanishing before consumption → no lap/thrash.

**W1 half — DELIBERATELY SKIPPED (Fable ruling, 2026-07-15), not a silent omission.** The
W2(i) Keys timeline is standalone mechanistic proof (watching the next row build one tag per
quiet frame is stronger than the spike-contrast inference we originally planned). The
symmetric W1 run would confirm nothing contestable: the W1 spike is forced **by
construction** — the old prefetch warms ≤1 block/frame, a row crossing needs ~5-6 blocks, so
the remainder MUST decompress on demand (arithmetic, already corroborated by the R2 probe +
positive-control cycle data). Mechanism (i) is CLOSED on the Keys evidence.

The symmetric protocol stays on file (above) in case anyone wants the W1 run later: build W1
debug ROM (commit 371aedd) in a scratch worktree INSIDE .worktrees/ (seed → rm pre-built
ROMs → build → hash-verify **8e41c991**), replay the identical warmup+schedule.

**Still OWED (rides the 3-regime A/B on W2+(ii), same Block_Stage_Next/Keys method):** the
DIAGONAL lap/thrash run — vertical was the easy case; the slot ruling's condition was always
the diagonal (does a diagonal crossing evict tags before consumption / thrash the 12 slots).
NOT discharged by this vertical run.

**The state-counter harness is now the campaign's live-measurement template** (Fable):
read a monotonic engine-maintained STATE counter's delta between frame boundaries via
read_memory, index by Frame_Counter value — immune to the frame-advance non-determinism that
corrupts profiler-window sampling.

---

## Wave-2 (ii) IMPLEMENTED + MECHANISTIC PROOF (2026-07-15)

Shipped `TileCache_WarmupBelowRow` (aeon 7e11b17 / sigil 8ec4889). Called once from
`Tile_Cache_Init` after `FillAll`, display OFF; reuses the prefetch down-scroll target +
scan but stages the WHOLE below-row (no k=1 cap, no budget) and saves d6/d7 across
`DecompressBlock` (which clobbers d0-d7) since the scan continues past it. Init-only
(Reinit's scroll direction is unknown). Region grew +$76 both shapes (0x920→0x996 plain /
0x9E0→0xA56 debug); all downstream gate orgs + pins re-baselined uniformly +$76.
tile_cache_port gate 4/4, full strict 2252/0, clippy clean. Provenance (branch): plain
99fd3a55/452638, debug a48fb0df/460661.

**LIVE MECHANISTIC PROOF (oracle, breakpoint at TileCache_WarmupBelowRow on fresh boot):**
the proc IS called from Init (breakpoint fired), and step_out confirmed it RETURNS cleanly
(→ Tile_Cache_Init+100, no hang — which also independently proves termination, since a hung
Init never reaches Game_State 6). Keys diff across the call (Cache: Left_Col=0, Head_Col=79,
Bottom_Row=61 → target below-row = align(61)+16 = world row 64 = block_y 4, cols 0-4):

```
entry (post-FillAll):  22 23 24 30 31 32 33 34 13 14 20 21   Next=8
after WarmupBelowRow:  44 23 24 30 31 32 33 34 40 41 42 43   Next=1
                       ^^                      ^^ ^^ ^^ ^^
```
→ staged EXACTLY the below-row **0x40,0x41,0x42,0x43,0x44** (5 blocks, Next 8→1 mod-12 = 5
decompresses), round-robin evicting the oldest slots. Confirms: (a) correct target (the
below-row), (b) the whole row staged (not k=1), (c) the d6/d7 save/restore works — a
corrupted col cursor could not have produced the contiguous 0x40-0x44. So the first downward
crossing finds these cached → the cold-start +5 transient is pre-staged away at source.

**Still OWED (the wave exit criterion, declared next phase):** the 3-regime A/B cycle-level
confirmation — cold control (budget <100% + VInt_Lag → 0, measured via a BREAKPOINT ON
VInt_Lag run free: zero hits = proof by non-event; Fable rider 3), steady-state
no-regression, and the DIAGONAL lap/thrash run (the slot ruling's real condition). Then
loop-until-dry (×80 revisit + FillColumn-hoist) → step-6 → merge packet + Fable's FillRow
line-by-line gate.
