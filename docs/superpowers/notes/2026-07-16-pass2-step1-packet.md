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

## Step 1.2 — plane_buffer (b) (Draw_TileRow_FromCache segments) — DONE (2026-07-17)

The `.row_src_loop` 64× per-cell W-walk → two monotonic world-col legs (`A..R` and
`R-63..A-1`, `A = R & ~63`, `R = Section_Right_Col_Written` cache-clamped) each emitted
as a contiguous `move.w`/`dbf` cache run through a `jbsr .emit_row_run` helper (physical
col = `(W + Origin - Left) mod COLS`, affine in W → one wrap-split per leg at the cache
right edge; ≤4 runs, within the note's "≤5"). The **zero segment is DROPPED** (Probe (i)):
the `W < Cache_Left_Col` cols emit their stale physical-col cache word instead of a
per-cell `clr.w`. This CHANGES Plane_Buffer content by design (off-screen cols carry stale
data, not tile 0), so **identity compare is N/A** — replaced by the clamp rail below. Cell
count unchanged (64) → row header (32 longwords) and VInt_DrawLevel drain untouched
(VBlank-Δ = 0). `move.l`/unroll of the runs = the ledger-1092 rider (deferred). Twins
byte-identical (`.emp`/`.asm`, gate both shapes); region **+$22 both shapes**
(Draw_TileRow_FromCache $BA→$DC = 186→220 B). ROMs debug **f8ee99d9** / plain **364b3ed1**
(were e15b6ff7 / f7faf57c).

**Clamp rail (identity N/A — the drop changes off-screen buffer content by design).**
Method note: screenshot-cmp under `Debug_Scene_Freeze` is CONFOUNDED — the freeze skips
`Camera_Update`, so HSCROLL/VSCROLL stay at the pre-freeze value and the rendered window
does not track the poked camera (OLD rendered a red stale-scroll frame where NEW rendered
content, a false diff). Replaced by the SCROLL-INDEPENDENT **nametable-cell compare**:
`read_vram` the drawn NT row on each ROM, compare per-cell (canonical freeze + Camera-poke
anchors give byte-identical cache metadata OLD≡NEW, so any NT diff is purely the drop).
Two left-behind anchors (`R < Head` ⇒ `R-63 < Left` ⇒ dropped cols exist):
- **A1 Cam(512,640)** — Left=44 Head=123 Origin=44 R=104; dropped world cols 41-43 = NT
  41-43. NT row 10 (`0xC500`): cells 0-40 **and** 44-63 **byte-identical** OLD/NEW; cells
  41/42/43 = OLD `0000 0000 0000` → NEW `D005 D81C D005` (stale VALID cache tiles, not
  garbage). Visible window (cam world col 64) = NT 0-40 ⇒ dropped NT 41-43 off-screen-left.
- **A2 Cam(768,512)** — Left=76 Head=155 Origin=76 R=136 (path-independent, matched OLD≡NEW);
  dropped world cols 73-75 = NT 9-11. NT row 10: cells 0-8 **and** 12-63 byte-identical;
  cells 9/10/11 = OLD `0000 0000 0000` → NEW `58C4 58B0 50B5`. Visible (cam col 96) =
  NT 32-63,0-7 ⇒ dropped NT 9-11 off-screen.
- **Structural invariant + inheritance (the margin case, ratified):** dropped cols are
  `W < Cache_Left_Col` = definitionally LEFT of the cache = left of the viewport+margin
  window (the cache is 80 = 40 viewport + 20×2 margin, always covering the visible 64).
  OLD writes tile-0 to these EXACT cols and ships with no visible glitch ⇒ they are
  provably never visible ⇒ NEW's stale-in-the-same-cols is VISIBLY IDENTICAL. The max-H
  A/B profile (Draw_TileRow absent, FillColumn/CopyBlockColumn == baseline) confirms the
  change is isolated to the vertical producer.

**Ripple sweep (5 sites, Fable rider 1):** (a) **pins.rs** repin ✓ (b) **engine.inc** 4
resume orgs +$22 (tile_cache $4F5A/$5BE4, collision_lookup $4F7E/$5C08, section
$5856/$64E0, sound_api $62C8/$7CE2 plain/debug) ✓ (c) **mixed_dac_rom.rs** UNCHANGED —
all engine symbols slide uniformly +$22 (plane_buffer precedes every cross-seam disp), so
the pinned *relative* displacements are invariant (contrast 1.1b, which changed a
tile_cache-INTERNAL disp); verified by the green suite ✓ (d) **repin_pins.rs** SOUND_API
base +$22 (plain 0x60A0→0x60C2, debug 0x79C4→0x79E6; lens unchanged) ✓ (e) **repin.toml**
symbols stable (Plane_Buffer_Reset / Tile_Cache_GetTile) ✓ → **2262 passed / 0 failed**
(failures-first). Detection note: the stale `secondary_pin_classes` baseline (site d) FAILED
first (`left 24770 / right 24736`), fixed, re-ran clean — the rider-1 hole stays closed.

**Producer A/B (debug, 60-frame sustained-scroll avg, budget 128000; Lag 0 all regimes):**

| regime | OLD Draw_TileRow | NEW Draw_TileRow | Δ | anchor |
|---|---|---|---|---|
| max-V (↓) | 13416 (10.5%, 2 calls) | 4188 (3.3%, 2 calls) | **−9228 / −68.8%** | 45ca85d 13416 (exact) |
| diagonal (↓→) | 6708 (5.2%, 1 call) | 2110 (1.6%, 1 call) | **−4598 / −68.5%** | — |
| max-H (→) | — (0 calls) | — (0 calls) | vertical producer — no run | FillColumn/CopyBlockColumn == baseline |

Per-call ~6708 → ~2100 = **−68.7%** consistent across regimes (beats the note's ~3.4k/row
estimate). OLD max-V reconciles with the 45ca85d anchor to the cycle (method + shape
validated). max-V idle VSync_Wait 45.2% → 53.3% (**+10442 freed**, ≈ the producer cut).
VBlank-Δ = 0 (VInt_DrawLevel/drain untouched — cell count and header unchanged). Diagonal:
no shared-budget regression (DecompressBlock cap untouched; FillRow/FillColumn unmoved),
Lag stays the 0 floor.

**Per-pass (step-3 vs step-5):** step-3 (modernize) = the two-leg segment decomposition +
the `.emit_row_run` helper (idiomatic internal `jbsr`/`rts`, core.emp precedent). step-5
(engine optimize) = dropping the zero-writes (Probe (i)) — the ~69% cut. **Neither-bucket
headline:** the stale-scroll screenshot confound under freeze → the scroll-independent
nametable-cell method is a verification-technique refinement worth carrying forward (cf. the
dead press-count drive); logged as the clamp-rail vehicle for any future plane-buffer piece.

---

### (scouting archive — superseded by the DONE block above)

**Target: `Draw_TileRow_FromCache`** (`engine/level/plane_buffer.emp:226` / `.asm` twin).
Appends one 64-cell row to `Plane_Buffer` for the VBlank drain (row header bit15=0, then
`PLANE_H_CELLS`=64 cells, then a `0` terminator; advances `Plane_Buffer_Ptr`). The
`.row_src_loop` (emp:285) runs 64× with per-cell:
- **W-cursor walk**: `d0` = world col, starts at `A = R & ~63` (R = `Section_Right_Col_
  Written` clamped to `Cache_Head_Col`), increments; when `W > R` it wraps `−64` → walks
  `A, A+1, …, R, then R−63, …, A−1` (the 64 plane cols in NT order). Deterministic.
- **zero segment**: `W < Cache_Left_Col` → `clr.w (a2)+` (cols behind the streamed window).
- **data**: physical col = `W + (Cache_Origin_Col − Cache_Left_Col)`, wrap at `TILE_CACHE_
  COLS`, indexed `move.w (a0,d1.w),(a2)+` from the cache row base (a0, physical col 0).

**Planned restructure (design note §2A(b) + Probe (i)):** decompose the deterministic
W-walk into contiguous `move.w`/`dbf` runs (the note's "≤5 runs"; the `A..R` and
`R−63..A−1` legs are each monotonic, split ≤2 more by the physical-col wrap at
`TILE_CACHE_COLS`) and **DROP the zero segment** — `clr` the `< Cache_Left` cols is WASTE
(Probe (i) DISCHARGED 2026-07-16: those cells are structurally off-screen ring-back in
every reachable regime; design note tail). Emit DATA runs only, no trailing zero run.
~3.4k cy/row (Fable 2nd-review). *producer / V.* `move.l`/unroll is the ledger-1092 rider.

**THREE things to carry (Fable, park):**
1. **Bar (identity N/A by design — the clamp changes buffer content):** VRAM VISIBLE-
   WINDOW compare across the whole drive (multiple frames) — NOT a cache-RAM byte-identity
   (dropping the zero-writes leaves off-screen plane cols with STALE data, not tile 0, so
   Plane_Buffer content + off-screen VRAM differ by design; the VISIBLE result must be
   unchanged). Clamp rail MANDATORY: leading-edge mid-scroll screenshots + the up+left
   drive proving the dropped cells never reach the visible window. Canonical freeze+poke
   anchors (`Debug_Scene_Freeze` 0xFF8A10 + Camera poke), NOT press-count drives — see the
   CANONICAL identity bar section above.
2. **CENTRAL RISK (the step-0 WATCH line):** the clamp leaves STALE cells, not zeros, and
   Draw_TileRow's left-behind case (`< Cache_Left_Col`) is **MARGIN-based, not ring-back-
   based** — unlike FillRow's zero region. The leading-edge screenshot criterion re-checks
   exactly this. Design the segment shapes with the margin case in front of you; if a
   dropped cell CAN reach the visible window in any regime, the zero segment stays for those
   cols (clamp only the proven-off-screen tail). Verify with up+left AND leading-edge.
3. **Discipline from commit one (1.1b-era):** the 5-site RIPPLE SWEEP (pins.rs, engine.inc,
   mixed_dac_rom.rs, repin_pins.rs, repin.toml) run AFTER repin with failures-first suite
   counts pasted (`N passed / M failed`, never `grep 'test result' | head`); one candidate
   group per commit; twin lockstep + byte gate both shapes; 3-regime A/B vs the 45ca85d
   anchor (producer-Δ + VBlank-Δ, Lag stays 0). Baseline for the A/B: Draw_TileRow_FromCache
   = 13416 cy / 10.5% max-V (unchanged through 1.1a/1.1b — it's the plane_buffer producer,
   not tile_cache), 45ca85d anchor row.

## Step 1.3 — tile_cache #5 (CopyBlockColumn wrap-split) — DONE (2026-07-17)

CopyBlockColumn is the HORIZONTAL-path producer (max-H regime; 45ca85d anchor 11869/9.3%).
Its two per-column copy loops (`.copy_nt` nametable, `.copy_coll` collision both-planes)
each walked DOWN a cache column with a **per-row row-wrap test** (`cmpa/blo/suba` at the
circular buffer's row-59→0 / coll-row-29→0 boundary). Since one block column is ≤16 rows,
each run crosses the wrap **at most once** → a 1.1a-class wrap-split: compute rows-until-
wrap (`COLS - phys_row`), emit ≤2 straight `dbf` runs split there, no per-row branch.
**BYTE-PRESERVING** by design ⇒ full identity bar. Twins byte-identical both shapes; region
+$42 (CopyBlockColumn $9E→$E0 = 158→224 B). ROMs **debug 217224d3 / plain 8b71f0c5**
(sizes 461540 / 453519; were f8ee99d9 / 364b3ed1).
_(Provenance note: the pre-final-fix BUGGY build was debug 59157ab2 / plain df2f9b7e —
same sizes, different bytes; the `movea.l`→`suba.w` wrap fix + re-pin changed the bytes.
The committed 1.3 code assembles to 217224d3 / 8b71f0c5, verified by fresh rebuild. These
are the canonical CRCs for the R2 provenance re-baseline and the merge-night rebuild check —
NOT the buggy 59157ab2/df2f9b7e.)_

**⚠️ THE IDENTITY BAR CAUGHT A REAL BUG (which the emp==asm byte gate could NOT — both
twins shared it).** First pass used `movea.l a5,a2` / `movea.l a6,a2` to reset the dest on
wrap. WRONG: that resets to (phys row 0, **col 0**), but the vertical run must stay in its
own `cache_col` — the original `suba.w #SIZE,a2` wraps while PRESERVING the column offset
(a2 was at base+NT_SIZE+cache_col*2 → −NT_SIZE → base+cache_col*2). So seg2 wrote the
wrapped rows to column 0 and the real columns lost their data. Caught at anchor A2
Cam(768,512), Origin_Row=46 (which maps content into physical row 0 — the seg2 target):
OLD-1.2 phys row 0 had content (`5009 5016 4017 583C…`), buggy NEW had zeros + a stray
`C1E7` at col 0. Fixed to `suba.w #TILE_CACHE_NT_SIZE`/
`#TILE_CACHE_COLL_SIZE` (both loops) — restores the original's column-preserving wrap.
Post-fix +$4 (movea.l 2B → suba.w 4B ×2 sites), so the region grew +$3E→+$42; re-pinned.

**Identity (full, cache-RAM byte compare — canonical Debug_Scene_Freeze + Camera-poke;
metadata byte-identical OLD-1.2≡NEW-1.3 at each anchor since CopyBlockColumn changes no
metadata).** Anchor **A2 Cam(768,512)** (Origin_Row=46 → content maps to the wrap-seam
physical rows 0 & 59; settled 360 frames, RowResume + Resume_Col = $FFFF on both):
- **NT phys row 0** (0xFF0000, the seg2 wrap target, content-bearing): **byte-IDENTICAL**
  OLD-1.2 vs fixed NEW-1.3 (scripted cell diff, 0 diffs) — proves the column-preserving wrap.
- **NT phys row 59** (seg1's last pre-wrap row) + **collision row 23** (content): byte-
  identical. (Row 59 also matched with the BUG present — it's seg1 — which is exactly why
  the seam-target row 0 was the discriminating read.)
- Collision-seam-CONTENT compare not forcible here (collision is terrain-sparse; coll rows
  0/29 were zero at every reachable content anchor). Collision correctness rests on: the NT
  wrap-seam byte-identity (the bug locus, same wrap code), the IDENTICAL structural fix
  (`suba.w #TILE_CACHE_COLL_SIZE`), the coll-row-23 content match, and the strict
  collision_lookup_port / tile_cache_port gates.
- Resume contract: CopyBlockColumn is atomic per call (budget-out lands at block boundaries,
  never mid-column); the A2 fills were budget-limited and settled byte-identical.

**Ripple sweep (5 sites) → 2262/0.** (a) pins repin ✓ (b) engine.inc 4 orgs +$42
(tile_cache $4F9C/$5C26, collision_lookup $4FC0/$5C4A, section $5898/$6522, sound_api
$630A/$7D24) ✓ (c) **mixed_dac_rom.rs** Collision_GetType bra disp F3EC→**F3AA** plain /
F32C→**F2EA** debug (tile_cache-INTERNAL growth between the bra target and the bra, the
1.1b pattern; FAILED first at F3EC, fixed) ✓ (d) repin_pins SOUND_API base +$42
(0x60C2→0x6104 / 0x79E6→0x7A28) ✓ (e) repin.toml symbols stable ✓.

**Producer A/B (debug, 60f sustained-scroll avg, Lag 0 all regimes):**

| regime | OLD-1.2 | NEW-1.3 | Δ | anchor |
|---|---|---|---|---|
| max-H (→) | 11869 (9.3%, 6 calls, 1978/call) | 9829 (7.7%, 7 calls, **1404/call**) | **−29.0%/call** | 45ca85d 11869 (exact) |
| diagonal (↓→) | 6948 (5.4%, 4 calls, 1737/call) | 5694 (4.4%, 4 calls, 1424/call) | **−1254 / −18.0%** | — |
| max-V (↓) | — (0 calls) | — (0 calls) | horizontal producer — no run | — |

OLD max-H = 45ca85d anchor 11869 to the cycle. Per-call win scales with rows-copied-per-
column (max-H stages full-height columns → −29%; diagonal partial → −18%). VBlank-Δ = 0
(pure producer). Modest vs the FillRow #1 cuts — CopyBlockColumn is #5 — but real, Lag 0.

## R2 HOLD — all four restructures DONE on pass2-hv-streaming (packet to Fable)

**Consolidated producer A/B (debug, 60f sustained-scroll avg, Lag 0 EVERY row/regime;
each OLD reconciles with the 45ca85d anchor to the cycle):**

| piece | routine | regime | OLD | NEW | Δ |
|---|---|---|---|---|---|
| 1.1a | TileCache_FillRow (NT segs) | max-V | 45798 (35.8%) | 29894 (23.4%) | −34.7% |
| 1.1b | TileCache_FillRow (+coll segs) | max-V | 45774 (baseline) | 18023 (14.1%) | **−60.6% cum** |
| 1.2 | Draw_TileRow_FromCache (drop-zero segs) | max-V | 13416 (10.5%) | 4188 (3.3%) | **−68.8%** |
| 1.3 | TileCache_CopyBlockColumn (wrap-split) | max-H | 11869 (9.3%) | 1404/call (9829/7c) | **−29.0%/call** |

Net max-V producer (FillRow+Draw_TileRow): Tile_Cache_Fill 53555→~21k, idle VSync ~24%→~53%.

**R2 checklist:**
- [x] **1.2 belt-and-braces — DONE 2026-07-17, CLEAN.** Live UNFROZEN drive on the branch-tip
      SHIPPING build (plain 8b71f0c5): reset→start→run right (build cache + left-behind
      columns)→sustained LEFT + up-jumps at speed. 7 leading-edge screenshots
      (scratchpad/r2drive/drive_1..7.png) across the dirt field, foliage/trunk sections, the
      canopy top, and a fast-scroll grass line + rings. NEW-only glitch inspection: every
      frame coherent — no stale-tile flash, no garbage at the left/top leading edges, rings
      intact. The 1.2 dropped-zero cells never surface under real motion (motion confirmation
      of the static + structural clamp proof).
- [x] Completed A/B table across all four restructures (above).
- **Bar-worked note for Fable:** 1.3's FULL identity bar CAUGHT a real column-preserving-wrap
      bug the emp==asm byte gate structurally could not (both twins shared it). Vindicates the
      "byte-preserving ⇒ full identity, not screenshots" framing — the cache-RAM seam compare
      was the exact instrument that surfaced it.
- **Neither-bucket (provenance hygiene — how the CRC nearly slipped):** the 1.3 checkpoint
      cited the PRE-FIX buggy build's CRCs (debug 59157ab2 / plain df2f9b7e). Two things
      masked it: (1) the buggy and fixed builds had **IDENTICAL byte sizes** (the wrap fix
      changed bytes, not length), so the size half of the CRC+size pair gave NO signal; and
      (2) the CRC was hashed at the wrong moment — from a build that predated the fix+re-pin.
      The gate fired on the ratifier's own numbers (a deviation from the stated expectation),
      which is the system working. **Standing lesson:** hash a piece AFTER its final re-pin,
      as the LAST item of its checklist next to the suite count — never from a build that
      predates any part of its commit. Canonical FIXED CRCs = debug 217224d3 / plain 8b71f0c5.

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
