# Pass 2 — H+V tile-streaming perf pass (step-0 design note)

**Status:** design deliverable, **GATED before any code** (Fable rules on the menu +
sequencing + the open questions). Scopes ledger row 1074's charter (plane_buffer a–f)
PLUS the review's tile_cache / section / collision-fusion candidates into one pass.
Structured like the t16 Wave-2 note: mechanisms → predicted deltas → measurement plan
with binding classes → exit criteria. **All cycle figures are the review's static
estimates (68000 timing tables); magnitude is provisional, attribution is binding.**

## 0. Problem & baseline

The unified-prefetch A/B (row 1066) removed the H-crossing DECOMPRESS spike, but sustained-
max scroll still lags: **real unfrozen max-H ~13% (27/~207 VBlanks), max-diagonal ~42%** —
a COPY/DRAW-bound residual, not decompress. Probe A (row 1074; frozen camera-poke, so the
RELATIVE split is robust but ABSOLUTE lag is understated — the frozen drive skips
EntityWindow_Scan, `Lag_Frame_Count` stayed 0 with 19.4% headroom) attributes the streaming
cost across two axes and two budgets:

| routine | %frame | budget | axis |
|---|---|---|---|
| **Tile_Cache_Fill (inclusive)** | **37.6** | producer | — |
| · TileCache_FillColumn (incl. CopyBlockColumn 21.4) | 35.0 | producer | **H** (copy) |
| · TileCache_DecompressBlock + S4LZ_DecompressDict | 10.7 | producer | shared |
| **Draw_TileColumn** | **7.5** (9587 cy, ~4800/call) | producer | **H** (draw) |
| VInt_DrawLevel drain | 2.0 | **VBlank** | H+V |

**Reading:** the DOMINANT lever is tile_cache's copy half (FillColumn/CopyBlockColumn,
~35% producer, H); Draw_TileColumn is real but ~5× secondary; the VBlank drain is small but
is the ONLY VBlank item and the one that corrupts (torn drain) if mishandled. The V-axis
(row producers/copy) is Wave-1's target mirrored — never done for the row/column streaming
path. **Cross-file review ranking:** #1 tile_cache #1 (10–25k/V-frame, the single biggest),
#2 tile_cache #2, #3 plane_buffer restructures + drain, #4 collision fusion, #9 section
H1–H4, #14 section M1/M2.

**Two axes, two budgets — the attribution law for every candidate:**
- **H axis** = column streaming: Draw_TileColumn (producer draw), FillColumn/CopyBlockColumn
  (producer copy — the dominant), the col-entry drain (VBlank).
- **V axis** = row streaming: Draw_TileRow_FromCache (producer draw), FillRow (producer
  copy), the row-entry drain (VBlank).
- **Producer budget** = main-loop frame time. **VBlank budget** = the ~18,500-cy NTSC
  window (VInt_DrawLevel). Every candidate is attributed to the budget it LANDS in; one
  that moves work producer↔VBlank is measured on BOTH.

## 1. Current mechanics (as-built)

**Producer (game tick):** `Tile_Cache_Fill` H-fills columns then V-fills rows into the 2D
`Tile_Cache_Nametable` (budget-limited: `BLOCK_DECOMP_BUDGET`/`VFILL_ROWS_PER_FRAME` caps +
a resume ladder). Separately the section producer appends DRAW entries to `Plane_Buffer` via
`Draw_TileColumn` (64-cell column) / `Draw_TileRow_FromCache` (64-cell row) — each walks the
2D cache with per-cell clamp/wrap checks + an indexed load, appending a header + 64 words.

**Consumer (VBlank):** `VInt_DrawLevel` drains `Plane_Buffer` — row entries (`bit15=0`,
autoinc $02) via `move.l (a0)+,(a6)` (already efficient); column entries (`bit15=1`, autoinc
$80) via `move.w (a0)+,(a6)` × count. Each entry pays an addr→VDP-command shuffle
(`lsl.l #2 / addq #1 / ror.w #2 / swap`) at its drain head, DUPLICATED in the row+col arms.

**The b96c861 invariant (binding on §2A(d)):** `VInt_Lag` NEVER drains (a lag frame is mid-
fill; the drain runs only on complete frames, `VBlank_Ready=1`). The long-`0` terminator
(VRAM addr 0 = "end") is unambiguous — no valid VDP command longword is 0. `Plane_Buffer_Ptr`
reset ordering vs the drain is what keeps a torn buffer from ever draining.

## 2. Candidate menu

Each: **mechanism · predicted delta · budget/axis · risk/verifier.**

### 2A. plane_buffer (row-1074 charter a–f)

**(b) `Draw_TileRow_FromCache` segment restructure — V producer, charter TOP.**
The `.row_src_loop` runs 64× with THREE per-cell checks (left-clamp; physical-col wrap at
`TILE_CACHE_COLS`; W-cursor R-wrap at −64) + one indexed `move.w (a0,d1.w),(a2)+`. The
W-walk is deterministic (`A..R` then `R−63..A−1`, both monotonic; the `<Cache_Left` zero-
region is a prefix of the second run; the physical wrap splits each run ≤2). Decompose into
**≤5 contiguous runs + one zero segment**, each a straight `move.w`/`move.l`/unrolled copy.
**~3.4k cy/row** (Fable 2nd-review). *producer/V.* Verifier: seam arithmetic (`Origin−Left`
adjust, off-by-one at `TILE_CACHE_COLS`); the twin-candidate case (R-clamp to `Cache_Head_
Col` picks the correct twin); `move.l` dest alignment. **The H-mirror of Wave-1's FillRow
hoist, applied to the row producer.**

**(a) `Draw_TileColumn` Part-A/B wrap-split (+2–4× unroll) — H producer.**
Per-word circular-wrap check (`cmpa.l a1,a0/blo/suba.w #NT_SIZE,a0`) below its scope (physical
row 59→0 wraps ≤ once/column). Split at the wrap; unroll each run. **~19% of Draw_TileColumn
≈ ~1.4%/f** (~1.8k cy). *producer/H.* Verifier: rows-until-wrap from the start row; the
Plane-B displacement trick holds inside each run.

**(c) `VInt_DrawLevel` `.drain_col` as `move.l` pairs + odd-word remainder — VBlank/H.**
Col autoinc is $80, so `move.l` writes TWO cells. Replace `move.w`×count with `move.l` pairs
+ trailing `move.w` for odd counts. **~300 cy/column drain ≈ ~0.5%/f VBlank.** *VBlank/H.*
Verifier — THE test vector: an extra word past NT row 63 lands in **Plane B at `$E000`**
(autoinc $80 from `$DF80`); odd-count handling EXACT. (Row drain already `move.l`.)

**(d) Producers store the READY VDP-command longword in the entry header — producer↔VBlank.**
The addr→command shuffle runs per entry in VBlank, duplicated row+col. Store the ready
command longword (4-B header vs 2-B addr) → each drain head collapses to one
`move.l (a0)+,(a5)`; count stores bare (drops `andi #$7FFF`). **Moves ~26–40 cy/entry OUT of
VBlank** (net VBlank win; producer ≈ neutral). Buffer +2 B/entry (negligible vs 1536).
*producer→VBlank.* Verifier — BINDING: **entry-format change ⇒ re-prove the b96c861 tear
invariant** (long-0 terminator stays unambiguous; re-walk reset-order/complete-frame) AND
update section.emp's reserve consts (cross-file, twin lockstep). Interacts with the
`vdp_comm_reg` shared-module consolidation.

**(e) DMA-drain — MEASURE-FIRST, its own sub-note, NOT a directive.** Resident art pool →
idle runtime DMA budget; a 128-B column via DMA ≈ ~100 cy setup + ~300 cy bus-frozen vs
~700–1,150 cy CPU drain. *VBlank.* Verifier: queue-slot pressure, small-entry setup
dominance, whether total VBlank WALL-TIME actually improves.

**(f) zero-fill + peephole micros.** `clr.w`(RMW 12c)→`move.w` from zeroed reg (pairs);
`move.w #imm`→`moveq`; stack-pair→free reg; unrolls. Per-site; batch each with the
restructure that touches its site — never standalone.

### 2B. tile_cache — the DOMINANT copy half + steady-state trims

**#1 FillRow / FillColumn per-tile loop → precomputed contiguous segments — THE lever.**
The `.fr_col_loop` runs PER TILE, re-doing world-col reconstruction, Head check, Left/width
clip, circular wrap, indexed read, stack-relative row-offset reload, double-indexed writes
≈ **180–250 cy/tile → 14–20k/row → ~30–40k/V-frame** (VFILL_ROWS_PER_FRAME=2). Everything is
deterministic once the block is known: compute `[col_start,col_end)` at block entry; the
wrap splits dest into ≤2 runs; source tiles are contiguous (`a0+2*col`). Restructure into
`move.w (a0)+,(a2)+ /dbf` (~22c/tile), `move.l` pairs (~13c/tile) when long-aligned, and
**two loop VARIANTS selected once per row** (collision vs no-collision) instead of the per-
tile `2(sp)` test; collision bytes contiguous both sides → `move.b` runs. **~5–8× on the
copy → 10–25k cy/V-frame.** *producer.* The V-axis is FillRow; **the H-axis DOMINANT
(FillColumn, Probe-A 35%) is the SAME restructure applied to the column-fill loop** — do
both. Verifier: wrap-split vs `Cache_Origin_Col` (seam off-by-one); the `Head−Left<COLS−1`
left-fill transient; `move.l` alignment; the collision odd-row gate (`btst #0` at :1329)
still selects the right variant after a mid-row RESUME (budget-out is at `.fr_block_loop`
head, so the `Cache_Fill_RowResume_Col` contract is unaffected by intra-block segmenting).

**#5 CopyBlockColumn per-iteration wrap-split — H copy inner loop.**
Each iter pays `cmpa.l a3,a2`(6)+`blo`(10, common branch TAKEN) on a wrap that happens ≤once.
Split into ≤2 `dbf` segments; 2×/4× unroll (rows even by contract). **~380c/call, ~1.5–2k
per newly-filled column.** *producer/H.* Verifier: seam-row math (59→0 NT, 29→0 coll); the
Plane-B displacement trick is position-independent (holds inside each segment).

**#2 empty/raw block-pointer indirection — decompress-adjacent.**
`.empty_block` writes 768 B via `clr.l (a0)+`×192 ≈ **5.8k/empty**; raw-direct does a 24-burst
movem ≈ **4.0k/raw** — both recur at world edges where max-scroll runs. Hold a per-slot data
pointer in RAM (16×4 B) at claim time: empty→shared zero ROM block (~0), raw→ROM-direct
(deletes the copy), compressed still decompress into the slot. Up to `BLOCK_DECOMP_BUDGET`(6)
×/frame. *producer.* Verifier: nothing WRITES through a staged pointer (CopyBlockColumn/
FillRow read-only); `FindStagedBlock:213-214` switches ROM-table→RAM-array; slot reuse
overwrites unconditionally; ROM block `even`. **Fallback if rejected:** pre-zeroed `movem.l`
bursts (~3.5k/empty, + clears the §2.5 `clr.l` violation).

**#3 FindStagedBlock memoize — steady-state probe.**
Probe ≈ 250c hit / 390c miss; steady prefetch re-probes every block col/row EVERY frame even
when fully staged ≈ **1.5–2.5k/frame pure probing**. Memoize completed scan targets
(target + edge coords + staging-generation; skip while memo matches; bump generation in
`DecompressBlock`'s claim). Saves ~the whole steady probe for ~30c/check. *producer.*
Verifier: generation bumps on EVERY claim (incl. empty/raw); memo dies on `InvalidateStaging`
and on any Left/Head/Top/Bottom move. (Alt: direct-mapped staging — O(1) but thrash risk,
needs the lag A/B.)

**#6 Tile_Cache_GetCollision arithmetic** — folded into §2D (it IS the terrain-sensor leaf's
tail; the ×80 row-table + cached-bias cuts are the same edit as collision_lookup #2/#3).

**Micros (batch with the restructure touching the site):** `cmpi #$FFFF`→`bmi` (~8c, sites
including the per-tile one #1 deletes); round-robin wrap→`andq` mask (~10c/decompress, +
power-of-two ensure); `Tile_Cache_Fill` double camera swap+shift (~24c/frame each axis);
`FillRow` redundant width check :1429 (DEBUG-assert the `Head−Left≤COLS−1` invariant, then
remove, ~16c/tile). **Init-only (not lag):** `FillAll` `clr.l`×3600 ≈108k, `InvalidateStaging`
`move.l #-1`×16 — `movem.l` bursts, conventions-compliance.

**Bugs/mismatches surfaced (triage, not perf):** first-fill spurious lag-skip
(`Init:496` comment vs `Fill:721-735` $FFFF-sentinel recompute — harmless, comment/gate
wrong); no out-of-window DEBUG guard on GetTile/GetCollision (§7.7); **dead export
`Tile_Cache_GetTile` — zero call sites** (delete); the a5/a6-survive-DecompressBlock hoist
(:1345) is one decompressor-swap from silent corruption (endorse the checked-clobbers lint).

### 2C. section — per-frame idle early-out + init-stall hoist

**H1 idle early-out for `Section_UpdateColumns` — ~500–600c/frame on the MAJORITY of frames.**
On a sub-tile-scroll/idle frame the routine still runs the movem + camera loads + clamp
chains + loop-entry checks doing nothing. Add a convergence gate (`Section_Stream_Converged`,
set after a pass where all four `Section_*_Written` trackers == needed; cleared on camera-
tile change, tile_cache Head/Left/Top/Bottom commit, or `Section_Plane_Dirty`). *producer.*
Verifier — CRITICAL: a **camera-tile-only compare is INSUFFICIENT** (a pass that exited early
on a buffer-full check with the camera then stopping would stall streaming forever);
**convergence must be trackers==needed**; teleport/rebase must dirty the gate. Zero on max-
scroll frames (buys headroom, doesn't move the lag counter) — but it's the CHEAPEST high-
frequency win.

**H2 delete the contract-contradicting movem pair — ~180c/frame, EVERY frame incl. lag.**
`.not_dirty` saves `d2-d7/a0-a3` the proc's own `clobbers` already declares dead (a3 saved,
never used). Delete it. *producer.* Verifier: the only per-frame caller
(`ojz_scroll_test.asm:187`) has no liveness; dirty/not-dirty register-state consistency.
**This one DOES land on lag frames.**

**H3 build-time act-boundary constant — ~50c/frame + kills a drift trap.**
Act-boundary clamp recomputed per frame → a build-time `Act.max_tile_col` (or RAM-cache at
Init); also removes the hardcoded `lsl.w #8` (B3, SECTION_SIZE_SHIFT−3 unguarded). *producer.*
Verifier: Act struct consumers (structs wall); teleport/act-switch re-reads.

**H4 (rider) hoist the per-cell wrap out of `Section_RedrawPlanes` — ~57k cy off the INIT/
RECOVERY stall, NOT a per-frame item.** ~14c × 4096 cell writes; the wrap point is constant
across all 64 columns. Hoist to two straight `move.w/lea/dbf` segments + optional unroll.
*neither per-frame budget — a synchronous stall at level init + cache recovery (user-visible
hitch).* **Verify by redraw-triggered SCREENSHOT DIFF, not the lag counter.** Rider because
it's off the sustained-scroll hot path (different budget class).

**M1/M2 (medium):** M1 build-time per-act row-pointer table for GetSecPtrXY/FlatIDXY (hoist
grid_w / `Act.row_pitch` / O(1) table; ~50–200c/frame, grows with mega-act) — verifier keeps
FlatIDXY's d2/d3/a2-preserved contract + the out-of-grid Z protocol (entity_window relies on
both). M2 drop the double caller/callee bound checks between section and the Draw_Tile*
producers (~100–250c on max-scroll frames — the lag-measured ones); **keep the CALLER's
check, drop the CALLEE's** (the d5 tracker desyncs the other way).

### 2D. collision — fixed-sweep skip + terrain-lookup fusion

**collision #1 fixed-sweep skip counter — ~1.5–1.9k/frame, biggest guaranteed win / lowest
risk.** The system+effect fixed sweep scans 24 slots × 2 players even when zero are
collidable (~40c/empty slot). Maintain `Fixed_Collidable_Count` (inc/dec at spawn/delete of
system/effect slots with nonzero `collision_resp` — cold paths); `tst.w`+`beq` skip when
zero. *producer.* Verifier: EVERY spawn/delete/`collision_resp`-mutation path updates the
counter (incl. `Object_ClearAll` resets); no in-place `collision_resp` clear without
bookkeeping.

**collision_lookup #1–3 FUSED terrain-sensor leaf rewrite — ~30%/call, ~600–2,000c/frame.**
Baseline ≈322c/lookup (114 GetType + 208 the `Tile_Cache_GetCollision` tail), 6–20 calls/f.
- **#1 span-check fusion:** the four-compare bounds check re-reads what the tail re-derives.
  Replace with the unsigned-span trick (`lsr #3 / sub Cache_Left_Col / cmp Cache_Col_Span /
  bhs .air`) producing the window-relative coord as a side effect → straight into wrap+index
  with d0/d1 already relative. Kills ~40c compares + ~24c dup subs + 10c jbra.
- **#2 cached halved-origin bias** (`Cache_Origin_Coll_Row = origin>>1`): ~12c/call, frees d2.
- **#3 ×80 row-offset table** (`add d1,d1 / move.w Row80Tbl(pc,d1.w),d1`, 18c, 120 B ROM
  shared with GetTile): ~22c/call; removes the scratch-reg requirement. **(= tile_cache #6.)**
*producer; per-sensor, H+V agnostic.* Verifier: `Cache_Col_Span`/`Cache_Row_Span` (and the
cached biases) written at EVERY edge-var commit site; `Tile_Cache_GetCollision` single-caller
(re-verify grep); CopyBlockColumn's inline ×80 is loop-SETUP — leave it; twins + byte gates
+ DEBUG-boot self-tests; the `Cache_Top_Row/Origin_Row` evenness invariant (load-bearing,
DEBUG-assert at write sites).

## 3. Cross-cutting concerns

- **THE SHARED EDGE-COMMIT-SITE AUDIT (single, covers three subsystems):** section H1's `sf`
  gate hooks, collision_lookup #1's span-var writers, and tile_cache #2/#3/#6's pointer/
  memo/bias updates ALL hook the same tile_cache edge-commit sites (`tile_cache.emp:844,
  873,944,985` + `Init` + H/VSlide/VSlideUp + left-fill origin retreat `:874`). One audit of
  "every edge-var commit site" discharges the verifier for all three — do it ONCE, up front.
- **Shared `BLOCK_DECOMP_BUDGET`** (H-fill+V-fill+prefetch): the copy-segment restructures
  (#1, b) reduce cost WITHOUT touching the decompress cap → no diagonal starvation (the t16
  slot ruling's binding case); #2 touches the staging pointer, not the cap.
- **Entry-format changes (d) are the ONLY b96c861-binding item** — everything else is cycle-
  for-cycle behavior-preserving within a producer or consumer.
- **Two-budget bookkeeping:** producer wins (a/b, tile_cache #1/#2/#3/#5, section H1–H3/M,
  collision) cut main-loop lag; VBlank wins (c) + the (d) shuffle-move cut the drain; (e)
  trades between them; **H4 is neither** (a synchronous init stall — screenshot-verified). A
  producer win that just relocates the bottleneck to VBlank is NOT a win.

## 4. Measurement plan (binding classes)

**Method = the state-counter template (ledger 1062):** frame-delimit with
`run_to_scanline(224)`+`wait_for_break`; read a monotonic engine-maintained STATE counter's
delta between frame boundaries (immune to the press frame-advance non-determinism that
corrupts profiler-window `calls`); index by `Frame_Counter` value; ROM hash-verified before
every measure. For the copy-cost levers where no state counter exists, use the recovered-
steady profiler window (Probe-A style) for the RELATIVE producer split + the lag counter for
the absolute clear.

**Two NAMED PROBES (row 1074):**
- **Probe (i) — zero-fill NECESSITY (sentinel-overwrite).** Before dropping/restructuring any
  zero-fill segment (b, tile_cache #1, f), pre-fill the would-be-zeroed cells with a
  SENTINEL, run a vertical-transition scroll, inspect: load-bearing stale-tile clearing, or
  waste? Waste → clamp entry counts to the cached rows (a real reduction); load-bearing → the
  zero segment stays (only its `clr`→`move` micro applies). **Binding on:** every candidate
  that would drop or restructure a zero segment.
- **Probe (ii) — unfrozen max-H AND max-V A/B drives.** The frozen Probe A can't lag (skips
  EntityWindow_Scan). Real drives, three regimes: sustained-max-H, sustained-max-V,
  sustained-diagonal. Per regime, per candidate: producer-Δ (main-loop) AND VBlank-Δ (drain)
  AND `Lag_Frame_Count` Δ. **Binding on:** every byte-changing candidate. A candidate that
  helps one axis but regresses the other (or the diagonal shared budget) does not ship as-is.

**Binding-class table (which probe/regime gates which candidate):**

| candidate | budget/axis | Probe (i) | Probe (ii) gating regime |
|---|---|---|---|
| tile_cache #1 (FillRow+FillColumn segments) | producer H+V | YES | max-V + max-H (the dominant) |
| (b) Draw_TileRow_FromCache segments | producer V | YES | max-V (+ diagonal no-regress) |
| tile_cache #5 (CopyBlockColumn split) | producer H | — | max-H |
| (a) Draw_TileColumn wrap-split | producer H | — | max-H |
| tile_cache #2 (block pointers) | producer | — | max-H/V edge regions (world edges) |
| tile_cache #3 (memoize) | producer | — | steady (no-regression) + max |
| section H1 (idle early-out) | producer | — | idle/sub-tile (MAJORITY) + max no-regress |
| section H2/H3/M2 | producer | — | max/diagonal (lag frames) |
| section H4 (redraw hoist) | init stall | — | **screenshot diff, NOT lag** |
| collision #1 + lookup #1–3 | producer | — | any (per-sensor steady) |
| (c) move.l col drain | VBlank H | — | max-H VBlank + the $E000 edge UNIT TEST |
| (d) ready-command header | producer→VBlank | — | max-H+V VBlank + **b96c861 re-proof** |

## 5. Exit criteria

1. **Lag clears on the real drives:** max-H and max-V `Lag_Frame_Count` measurably down from
   the 27/~207 (H) baseline; the diagonal (shared-budget worst case) does NOT regress.
2. **No warm-window regression:** steady (non-crossing) FillRow/Parallax/Section costs
   unchanged (the t16 warm-window control).
3. **Correctness:** leading-edge mid-scroll screenshot + cache-keeps-up (Cache_Bottom/Head
   track camera, no stale-tile corruption) — CRITICAL for any zero-fill or entry-format
   change; **H4 verified by redraw screenshot diff** (its own regime).
4. **b96c861 re-proven** for (d) if taken (sentinel-drain-during-lag NON-EVENT, like t16's
   sentinel-invalidation artifact).
5. **Every win attributed to its budget** (§3 two-budget law); each packet row records
   producer-Δ AND VBlank-Δ, never a single number.

## 6. Recommendation & sequencing (Fable rules)

Provisional order — biggest, lowest-risk, behavior-preserving first; b96c861-binding last:

1. **The two DOMINANT copy-segment restructures — tile_cache #1 (H FillColumn + V FillRow,
   ~35% producer / 10–25k/V-frame) and plane_buffer (b) (V draw, ~3.4k/row).** Biggest levers,
   behavior-preserving (no b96c861 exposure, no shared-budget touch), both carry Probe (i).
   Pair with tile_cache #5 (CopyBlockColumn split) since it's the same H copy inner loop.
2. **section H1 idle early-out** — cheap, hits the MAJORITY of frames, buys headroom for the
   rest; do the SHARED edge-commit-site audit (§3) here since H1's gate hooks need it and it
   also unlocks collision_lookup #1 and tile_cache #2/#3.
3. **collision #1 + collision_lookup #1–3 fused** (incl. tile_cache #6) — ~30%/sensor, one
   coordinated leaf rewrite riding the same edge-commit audit; section H2/H3 fold in (H2 lands
   on lag frames).
4. **The drain wins together — (c) move.l col drain + (d) ready-command header** — the VBlank
   budget half; (d) is the ONE b96c861-binding change, gated on its re-proof + the vdp_comm_reg
   consolidation ordering. tile_cache #2/#3 + (a) + micros + M1/M2 fold in opportunistically.
5. **H4** on its own (init-stall regime, screenshot-verified) whenever RedrawPlanes is touched.

**Open questions for the gate:**
- **(d) vs the `vdp_comm_reg` shared-module consolidation** — do (d) first (producers emit the
  ready command) then consolidate, or consolidate first? (d) changes the buffer entry format
  cross-file (section reserve consts); sequence to re-prove b96c861 exactly once.
- **(e) DMA-drain** — commission the measure-first sub-note, or defer past this pass? (It's the
  only candidate whose sign is unknown without measurement.)
- **tile_cache #2 indirection vs the movem-burst fallback** — the RAM per-slot pointer is the
  bigger win but adds a pointer-array invariant to the edge-commit audit; accept it, or ship
  the fallback (pre-zeroed bursts, ~3.5k/empty) this pass and defer the indirection?
- **tile_cache #3 memoize vs direct-mapped staging** — memoize is behavior-preserving;
  direct-mapped is O(1) but carries thrash risk needing its own diagonal A/B. Memoize this
  pass, direct-mapped never?

---
**Constraints (binding, restated):** twin lockstep + re-pin per byte-changing commit; the
b96c861 tear invariant re-proven by ANY entry-format change; two-budget attribution
(producer cy = main loop, drain cy = VBlank) on every A/B; the SHARED edge-commit-site audit
done ONCE up front (§3).

---

## Probe (i) — zero-fill NECESSITY — DISCHARGED (2026-07-16). RESULT: WASTE → CLAMP.

**R1 first act, run before any segment code (the answer shapes the segment shapes).**
Geometry: cache = 60 rows (`TILE_CACHE_ROWS`; 28 visible + 16 margin ×2), plane = 64
(`PLANE_V_CELLS`) → the zero-fill covers plane rows [60,64), the ring's FAR side, ≥16 rows
from the visible 28-row window whenever the cache covers it. Cols side symmetric
(`.row_src_zero`, cols < `Cache_Left_Col`).

**Method — sentinel-overwrite (throwaway DEBUG build, reverted):** patched all four zero-fill
sites (`plane_buffer.asm` `.pA_zero`/`.pB_zfill`/`.pB_az` in Draw_TileColumn + `.row_src_zero`
in Draw_TileRow_FromCache) `clr.w (a2)+` → `move.w #$E1FF,(a2)+` — a bright priority /
palette-3 tile ($1FF). If any zeroed cell reaches the visible plane, the sentinel renders as
an obvious anomaly. ROM hash-loaded into oracle.

**Regimes driven, all CLEAN (no sentinel on screen):**
1. sustained player-driven down+right (400f) — clean; VRAM read confirms the sentinels are
   CONFINED to nametable rows 60-63 (`$DE00-$DFFF`), the off-screen ring-back.
2. reversed player-driven up+left (300f, flips the V-fill direction) — clean.
3. gross camera-poke overrun (Debug_Scene_Freeze + Camera_Y +80–96 px/f, 5–6× the 2-row/f
   fill) — clean, but this TRIGGERS RedrawPlanes (full-plane rewrite), which bypasses the
   incremental zero-fill entirely.
4. **sustained INCREMENTAL overrun (Camera_Y +40→48 px/f × 6 frames, cache falling ~4–6
   rows/f behind → >24 rows behind, EXHAUSTING the 16-row margin, staying under the redraw
   threshold)** — STILL clean: the leading edge shows REAL streamed tiles (the fill
   prioritizes the leading edge), the zero rows stay behind it, off-screen.

**Conclusion:** the zero-fill cells are structurally off-screen in every reachable regime —
normal scroll (cache covers visible+margin), gross overrun (redraw bypasses it), and
sustained incremental overrun (leading edge stays real-tiled; zeros stay on the ring-back).
The "load-bearing during vertical transitions" hypothesis is DISPROVEN empirically + by
geometry.

**BINDING on the segment restructures (b, tile_cache #1, #5):** DROP the zero segment —
**clamp the entry count to the cached rows/cols** (`min(rows, TILE_CACHE_ROWS)` /
`min(cols, span)`). Real reduction on BOTH budgets: fewer producer word-writes AND fewer
VBlank drain words per column/row entry, on top of the per-cell→segment copy win. The ≤5-run
decomposition emits the DATA runs only — no trailing zero run. (Correctness rail for the
segment code: the clamp must never SHORTEN a run that the visible window can reach — it
clamps only the proven-off-screen tail; the leading-edge screenshot exit criterion re-checks
this after the restructure.)
