# Unified direction-aware block prefetch + streaming/art-loading review (design note)

**Status:** design deliverable, GATED before any code (Volence/Fable rule on the
recommendation, §9).
**Charter:** the H-column dossier (campaign-gap-ledger row 1063) + Volence's widened
scope (2026-07-15): best-in-class review of the whole streaming/art-loading design,
not just the mirrored fix. Forked from port-tranche16 tip dd949c2 so all Wave-2
state cites cleanly.
**Problem:** Wave 2 (i)+(ii) cleared the VERTICAL crossing spike (3-regime A/B:
0 VInt_Lag on a sentinel-validated detector). The horizontal twin is unfixed and
measured: regime (c) (+16/+16 diagonal) logged 2 lag frames, each a fresh block_x
column cold-filled by `TileCache_FillColumn` (~5 decompresses in one frame — e.g.
X=256/Y=512: keys 25,35,45,55,65 staged in one frame, `Block_Stage_Next` 9→2).
The donor never had horizontal prefetch; the spike predates t16.

**Research provenance (2026-07-15/16, three-agent campaign + prior art):**
Sonic-family (s2disasm, skdisasm, S.C.E., sonic_hack — KosM bookmark mechanism
sonic3k.asm:2818-2966, S2 PLC 2-tier count budget s2.asm:2205/2218, sonic_hack
hysteresis latch section_streaming.asm:1104-1186); B&R corpus (B&R dual-cap
cost-normalized budget main_loop.asm:22-25/194-211/3604 + reserve floor
interrupts.asm:640-642 + cost normalization interrupts.asm:300-309; Vectorman
transactional dual cap disasm.asm:6288-6341 — CORRECTION for Phase-2: the length
cap is 2880 WORDS = 5,760 bytes, not 2,880 bytes, and admission is
snapshot/rollback transactional; Ristar yieldable decompressor ANALYSIS.md:330-337);
online/modern (SGDK map.c — no runtime decode in the scroll path, structure
RAM-resident; Tanglewood 128-px chunks + per-column decode cadence; Sunset
Overdrive GDC 2015 — neighbor-ring prefetch, derived deadlines, pinned eviction
classes; prefetch-aggressiveness theory: with cheap waste + expensive stalls,
err aggressive). Full reports in the session transcript; load-bearing citations
inlined below where a decision leans on them.

---

## 1. Demand envelope (design question 2)

| Axis | Sustained | Transient worst | Clamp | Crossing period (128-px block) |
|---|---|---|---|---|
| Vertical | 16 px/f (terminal fall) | 16 | `CAM_MAX_Y_STEP = 16` | every 8 frames |
| Horizontal | **6 px/f** (`PHYS_TOP_SPEED = $600`) | 16 (`PHYS_GSP_CAP = $1000`: slopes, rolling, springs) | `CAM_MAX_X_STEP = 16` | every ~21 f typical, every 8 f clamped |

No speed-shoes constant exists in aeon yet; if added at the classic $C00 =
12 px/f it stays under the clamp and inside this envelope. **Read:** the
horizontal case is ~2.7× gentler than vertical in normal play (more quiet frames
per crossing), but the adversarial case (rolling downhill, spring launches,
camera-clamped) is IDENTICAL to vertical. Design for the clamp, enjoy the slack.
Diagonal worst case = both axes at 16 px/f → a row crossing AND a column crossing
every 8 frames each. This is the design case, not the corner case (it is exactly
the regime-(c) lag and the DEFERRED_WORK "~76% lag at sustained max diagonal"
entry's regime).

## 2. Current mechanics delta (as-built at aeon port-tranche16 / sigil dd949c2)

Vertical prefetch (Wave-2 (i)): leftover-budget, `Cache_Prev_Cam_Row` delta
direction sense, next block-row enumeration left-to-right, k=1 stage-first-unstaged
(`Tile_Cache_Fill` .pfx_go, tile_cache.asm:897-989). Init warmup (Wave-2 (ii)):
`TileCache_WarmupBelowRow`, whole below-row, display off. Staging: 12 slots ×
768 B round-robin (`Block_Stage_Next`), keys `sec_x|sec_y|block_index`.
`BLOCK_DECOMP_BUDGET = 6` shared H-fill + V-fill + prefetch, fill first.
**The gap:** the prefetch is ROW-ONLY — no `.pfx` column branch exists; a fresh
block-column's ~5 blocks all decompress inside `FillColumn` on the crossing frame.

## 3. Mechanisms (each: quiet-frame cost + risk)

### (H1) Column-scan staged-count-aware prefetch — the (i) mirror, 90°
New `Cache_Prev_Cam_Col` (word) beside `Cache_Prev_Cam_Row`. Horizontal camera
tile-col delta signs +right/−left. Target = the block-col just beyond
`Cache_Head_Col` (right) / just before `Cache_Left_Col` (left), aligned like
`.pfx_go` does for rows. Enumerate the block-rows the cache spans (aligned
`Cache_Top_Row`..`Cache_Bottom_Row` step 16 — ≤5 candidates), `FindStagedBlock`
each, `DecompressBlock` the FIRST miss, **k=1**, leftover budget, ordered AFTER
the existing row scan. Grid guards mirror the row scan's: sec_x guarded once at
entry (fixed for the scan), sec_y guarded per step. Adopts `decompose_block()`
(5th consumer site).
- *Quiet-frame cost:* ≤1 decompress + ≤5 probes (probes ~380 cyc worst at 16
  slots, §H5). At the clamp: 5 blocks / 8 quiet frames — k=1 suffices with 3
  frames of margin; at top speed, 5 / ~21 — trivial.
- *Risk:* slot pressure (§5); reversal thrash (owned by H3); shares leftover
  budget with the row scan on diagonals (§6 shows it fits).

### (H2) Diagonal corner block
When BOTH axis latches are active (§H3) the (next-row × next-col) corner block is
in NEITHER enumeration — it cold-decompresses at whichever crossing consumes it
first. After both axis scans run (or report fully-staged), probe/stage the corner,
on remaining leftover budget. Sunset Overdrive's neighbor-ring is the precedent:
adjacency defines eligibility, direction only orders priority (row → col → corner).
- *Quiet-frame cost:* ≤1 probe, ≤1 decompress, diagonal-only frames.
- *Risk:* none beyond +1 to the live set (counted in §5). A wasted corner stage on
  a direction change costs one 768-B decode — the aggressiveness trade the
  prefetch literature endorses when stalls are expensive and waste is cheap.

### (H3) Horizontal direction hysteresis latch
The hazard vertical never had: players oscillate in X constantly. A naive
sign-of-delta retarget (the vertical pattern) makes a player dithering on a
column seam alternately stage the left and right neighbor columns — slot churn
with zero payoff. Precedent: sonic_hack's proto-section one-shot latch +
re-arm-on-retreat (section_streaming.asm:1112-1186 — fire once at a threshold,
re-arm only after retreating past a different threshold).
Mechanism: latch the H prefetch direction (byte); flip only after ≥ `H_PFX_HYST`
px of NET motion opposite the latch (accumulator word). `H_PFX_HYST` initial
value 16 px (2 tile-cols), tunable; A/B regime (d) binds it.
- *Quiet-frame cost:* a compare + occasionally an add (~10 cyc).
- *Risk:* a genuine reversal serves the stale target for ≤ the hysteresis window
  (≤1 wasted decompress) before flipping — bounded, self-healing, and strictly
  better than churn. NOTE: vertical has the same hazard milder (jump arcs
  reverse Y sign every ~40 f); vertical hysteresis is NOT taken now (arcs are
  short, slots absorb it, no measured thrash) — regime (d) also watches the
  vertical tags for churn and escalates only on evidence.

### (H4) Late-frame prefetch gate (the Option-C pull-forward)
> **CORRECTED at implementation — see §10.** The beam-position premise below is
> FALSE: `Tile_Cache_Fill` runs inside VBlank at V≈240, so the V-counter can't
> gauge frame load. H4 shipped as a `Frame_Counter`-delta trailing-lag gate instead.
"Leftover budget" is counted in SLOTS, not TIME: on a frame where parallax ate
the CPU but the cache was warm (fill spent 0), the counter still reads 6 and the
prefetch happily speculates — potentially pushing a ~98% frame over. The
references' arrow (S2's 2-tier count → KosM's pure time-budget; B&R's reserve
floor) says budget speculation by time. Cheapest correct form: before the
prefetch tail, read the VDP V-counter ($C00008 high byte); if the beam is past
`PFX_DEADLINE_LINE` (initial ~200 of 224 active), skip ALL prefetch this frame
(row + col + corner). Demand fill is untouched — it must run regardless.
- *Quiet-frame cost:* ~3-5 instructions, one VDP read.
- *Risk:* none — prefetch is pure speculation; skipping it on a late frame is
  always correct. This gate is ALSO the Phase-2 arbiter's block-tier hook (§7):
  when the cost-denominated arbiter lands, the gate's test becomes an arbiter
  call at the same seam.

### (H5) `BLOCK_STAGE_SLOTS` 12 → 16
Justified by the lap-rate arithmetic in §5. Mechanical footprint: buffers
+3,072 B lower RAM; `Block_Stage_Keys` +16 B upper RAM; `BlockStage_PtrTable`
+16 B ROM; `FindStagedBlock` probe worst case 12→16 slots ≈ +56 cyc (measured
~289/call at 12 → ~380 est; called ≤6×/frame steady — acceptable, and probes
only run on fill/prefetch paths). `InvalidateStaging` loop constant follows.
- *Risk:* RAM budget interaction with Phase 2 (§5 shows it closes); nothing else
  — round-robin, keys, and eviction semantics unchanged.

### (H6) Bundled: FillColumn/CopyBlockColumn base-lea hoist (dossier rider)
Per gap-ledger row 1063 (Fable-requested analysis, deferred to this effort):
`CopyBlockColumn` reloads 4 loop-invariant base leas per BLOCK (nametable base,
its wrap sentinel, collision base, its sentinel — tile_cache.emp
CopyBlockColumn:408/411/440/444), ~4-5 blocks per column fill. Hoist to
FillColumn scope like Wave 1 hoisted FillRow's row bases. Register-pressure
constraint: only a5/a6 survive `DecompressBlock`'s clobber license → hold the 2
BASES in a5/a6 and derive the 2 sentinels by `adda` inside CopyBlockColumn (the
sentinels are base + build-time constants), or partial-hoist. Byte-changing
(re-pin + live-verify rides this effort's A/B).
- *Cost saved:* ~4 × 12-cyc abs.l leas × ~5 blocks ≈ 240 cyc per column fill —
  modest; bundled because it is the same file, same hot path, same live-verify.

### Rejected / not-taken (with reasons)
- **Velocity-proportional block-splitting** — REJECTED, stays rejected. The
  ratified count-aware scan self-regulates (the faster axis crosses sooner, its
  scan finds unstaged blocks more often), and the research CONFIRMS the shape:
  Sunset's prefetch is adjacency+deadline-derived, not velocity-predicted;
  proportionality emerging from staging state is the same mechanism.
- **Eviction-order awareness (pin bits / protected sets / partitioned slots)** —
  NOT taken. Taxes every decompress and adds a failure mode for a problem the
  empirical ruling (ledger row 1064: tags survived every diagonal lag frame)
  shows we don't have at current cadence. §5's slot raise buys margin instead.
  Escalation trigger unchanged (Wave-2b condition): pre-staged tags vanish
  before consumption AND crossings lag — now evaluated at 16 slots.
- **Resumable S4LZ block decode (Ristar/KosM mid-stream bookmark at block
  granularity)** — NOT taken. Blocks are 768 B ≈ already KosM-granule-sized;
  the spike is 5-6 whole blocks in one frame, not one block too big for a frame.
  Prevention (warm crossings) beats suspension here. The bookmark stays where
  it is banked: the Phase-2 art-page tier.
- **SGDK-style resident map structure** — NOT adoptable. SGDK unpacks all map
  structure to RAM at init (no decode in the scroll path at all) — that is the
  S2 model (42 KB of 64 KB RAM on map data). Aeon's 23.6 KB tile-cache complex
  + per-section S4LZ dictionaries IS the RAM-affordable middle; going resident
  would cost the RAM Phase 2 and the game need. A pinned "hot blocks" set is
  likewise NOT taken — warm steady decompress rate is ~1.5/frame average,
  already cheap; revisit only if A/B shows recurring re-decompress hot spots.
- **Full budget re-architecture now (Option C)** — DEFERRED to Phase 2, §7. For
  the block tier alone a cost-denominated budget is a NO-OP: blocks are uniform
  768-B granules, so count ≡ cost until variable-size items (art pages) join
  the pool. The retrofit seam is narrow and named (§7). (H4) pulls forward the
  one piece with present-tense value.

## 4. Init warmup symmetry (design question 3): down-only STANDS

Argument for no side-column warmup:
1. The horizontal margins are pre-filled: `FillAll` covers camera ±160 px
   (20 tile-cols each side). The first FRESH block-col demand arrives only after
   the cache edge advances past the initial window.
2. Horizontal speed is earned, not free: from spawn standstill, reaching top
   speed takes ~85 frames ($C accel); even an instant-16-px/f spring gives ≥10
   quiet frames of camera motion before fresh-column demand — and (H1)'s k=1
   stages the ≤5 column blocks in ≤5 frames once direction is sensed. Gravity
   is the only free instant-max axis; that is exactly why down-only was right.
3. The residual is a bounded one-time catch-up transient (the H analog of the
   vertical +5 at 144→160 px that (ii) erased): ≤5 decompresses spread over
   demand-fill frames at ≤2 tile-cols/f — inside budget, no lag predicted.
Fallback IF regime (f) falsifies the prediction: `TileCache_WarmupSideColumns`
mirroring (ii) (~10 decompresses at init, display off, cheap) — but it doubles
the init-staged live set and is NOT taken speculatively. Spawn-into-instant-max
is rare by level design (spawns are rest points); the A/B decides, not taste.

## 5. Slot pressure (design question 1 — the central fork): the lap-rate model

**Live-set arithmetic (the brief's "compute the real number"):** the cache spans
≤6 block-cols (80 tiles unaligned) × ≤5 block-rows (60 tiles unaligned).
Peak protected set just before a diagonal double-crossing:

| Component | Blocks |
|---|---|
| Current row working set (re-probed for 16 tile-rows) | ≤6 |
| Current column working set (re-probed for 16 tile-cols) | ≤5, −1 shared with row |
| Pre-staged next row | ≤6 |
| Pre-staged next col | ≤5 |
| Corner | 1 |
| **Peak** | **≤22** |

22 > 12 > any affordable slot count — so "protect everything" is the wrong
frame. Staging protection is for PERFORMANCE, not correctness (an evicted block
re-decompresses on demand, ~2-3k cyc). The binding metric is the **lap rate**:
round-robin lap period = SLOTS ÷ claims-per-frame, and a pre-staged tag must
survive from staging to consumption ≈ up to one 8-frame crossing window at the
clamp.

**Claims per 8-frame window, steady WARM max diagonal (fill claims ≈ 0):**
next-row ≤6 + next-col ≤5 + corner 1 ≈ **12**. At 12 slots the lap period is
12 ÷ (12/8) = **8 frames — exactly the survival requirement. Zero margin.**
This is consistent with, not contradicted by, the row-1064 empirical ruling:
regime (c) ran ROW-ONLY prefetch (≈6-7 claims/window → lap ≥14 f ≫ 8), and the
H-fill's crossing bursts took stale slots — the ruling closed the eviction
question at THAT cadence; the unified cadence reopens it, and the arithmetic
says marginal.

**At 16 slots: lap = 10.7 f vs 8 needed (1.33× margin). At 18: 12 f (1.5×).**

**RAM budget (jointly with Phase 2, the contended resource):** lower-RAM slack
today 9,150 B. +4 slots = 3,072 B → 6,078 B remaining; the banked Phase-2 spec
earmarks ~2.5-3 KB (2 KB page staging + page table + refcount/LRU) → **~3.0-3.5
KB uncommitted after both**. 18 slots (+4,608 B) would leave ~1.5-2 KB post-
Phase-2 — crowds it for a margin the arithmetic doesn't demand. Upper RAM
(19,198 B free above `Game_RAM_End` $FFFFB402) is untouched by this design
(+16 B keys only).

**RECOMMENDATION: 16 slots.** Eviction-order awareness stays rejected (§3);
18 is the pre-approved fallback IF regime (c') shows tag laps at 16.

## 6. Budget interaction + up/left symmetry (design question 4)

Ordering unchanged and load-bearing: **fill first, prefetch on leftover** — the
prefetch structurally cannot starve `FillColumn`/`FillRow` (the row-1063/1064
mechanism: it stages at most what fill left unspent). Diagonal worst case,
steady warm: fill decompresses ≈ 0 (everything pre-staged), budget 6 intact →
row k=1 + col k=1 + corner ≤1 = **≤3 of 6** — fits with headroom. Cold/catch-up
regimes: fill consumes the budget first and prefetch degrades to zero — correct
by construction (demand > speculation). (H4) additionally bounds speculation on
time-heavy frames (the count-vs-time gap, §3).

Up/left symmetry: the column scan mirrors `.pfx_up` exactly — left target =
block-col before `Cache_Left_Col`, world-edge guard at col 0, sec_x ≥ grid_w
skip on the right. No asymmetries beyond the direction latch (H3).

## 7. Phase-2 composition contract (the re-architecture half)

The full-review finding: the best-in-class END STATE is already mostly banked
(Phase-2 spec 2026-07-02: bookmark resumable ZX0, page-frame residency,
refcount/LRU/pin, B&R per-act budget word, Vectorman dual cap, camera
soft-clamp). What was missing is the SEAM between the block tier and that
future — specified here so Phase 2 implements against an agreed contract:

1. **Target budget economy (build at Phase 2, not now):** one cost-denominated
   slack arbiter for all deferred work. Per-item costs: block ≈ constant
   (uniform 768 B — the reason count ≡ cost today); art page = per-page cost
   word in the manifest (B&R precomputed-cost pattern, main_loop.asm:830-832).
   Priority: demand fill (guaranteed floor, B&R interrupts.asm:640-642) →
   block prefetch → art-page decode. Transactional admission for speculative
   items (Vectorman rollback). Dual cap on the DMA side as banked.
2. **The block tier's adoption seam (named, ~10 lines):** `Cache_Fill_Budget`
   reset/consume in `Tile_Cache_Fill` + the (H4) gate test. The fill machinery
   never learns what unit the budget is in.
3. **Evidence-based sequencing flag on banked Phase-2 §3 (bookmark):** the
   corpus norm is KosM-style PRE-CHUNKING — pages sized so one decodes inside
   worst-case slack, ≤1 in flight, no mid-stream checkpoint (page boundaries
   reset the ZX0 window anyway; S3K/S.C.E. shipped exactly this shape at $1000
   granule). Recommendation: P2a runs the page-size sweep FIRST; the bookmark
   (banked, still justified for consuming ALL idle) lands only if small pages
   miss the latency target. Sequencing, not relitigation.
4. **Vectorman citation correction for Phase-2 implementation:** the dual cap's
   length budget is 2,880 WORDS (5,760 B), admission is snapshot/rollback
   (disasm.asm:6288-6341); the local ANALYSIS.md "double-buffered" gloss is
   wrong — single queue, transactional.
5. **Mega-act/corridors:** S3K's act transitions stream art+blocks+chunks into
   HAND-PLANNED static overlay regions mid-gameplay (sonic3k.asm:106283-106321)
   — the corridor design should prefer build-time pinned corridor pages over
   runtime cache-fighting (matches the build-time-computation ethos).
6. **Doc-sync flags (for the aeon docs pass, post-gate):** ARCH §9.7 still
   carries the REJECTED user-mode design (Phase-2 spec §3 says "rewritten to
   the bookmark" — the rewrite never landed); the `CAM_MAX_Y_STEP` comment's
   "~76% lag" figure and the DEFERRED_WORK diagonal entry predate Wave 1/2 —
   regime (e) re-measures both.

Block size stays 16×16 tiles / 128 px — validated by corpus convergence (SGDK
blocks and Tanglewood chunks are the same 128-px granule; KosM's $1000 module
≈ 1.3 blocks).

## 8. A/B plan (design question 5)

Method = the campaign's live-measurement template (probe note + ledger 1062):
ROM hash-verified via explicit-path reload BEFORE measuring; scene =
OJZScroll + `Debug_Scene_Freeze=1` (byte!) + Camera pokes; frames delimited by
`run_to_scanline(224)`+wait; decompresses = `Block_Stage_Next` delta,
tag timeline = `Block_Stage_Keys` (one ~50-70 B read), every sample indexed by
the `Frame_Counter` VALUE; lag = breakpoint at `VInt_Lag`, with the
sentinel-invalidation positive control (0xFF the keys → force a cold crossing →
detector MUST fire) re-run per regime batch. The profiler is
documented-unreliable for per-frame work — state counters only.

| Regime | Schedule | Binds | Pass |
|---|---|---|---|
| (a) H cold control | Camera_X +16 px/f from spawn region (mirror of f5801df) | (H1) mechanism + §4's no-side-warmup call | VInt_Lag hits = 0; keys show next-col tags building 1/frame (the 90° analog of the 50→54 build) |
| (b) H steady | warm 20-frame window, same schedule class as W1's | no-regression | FillRow/FillColumn/Parallax cycle counts unchanged from the Wave-1/2 baselines |
| (c') Diagonal | Camera_X + Camera_Y both +16 px/f — **the exit criterion** (was NOT-TAKEN-WITH-REASON) | (H1)+(H2)+(H5) jointly; the reopened slot ruling at 16 slots | VInt_Lag hits = 0; corner tag observed staged before the double-crossing; NO pre-staged tag vanishes before consumption |
| (d) Reversal thrash | oscillate Camera_X ±16 px/f across a column seam ×N | (H3) hysteresis constant | decompress count bounded (≤1 per genuine flip); no alternating-target churn signature in keys; vertical tags watched for the jump-arc analog |
| (e) Sustained max diagonal, long run | the DEFERRED_WORK stress | the ~76%-lag ledger figure, post-W1/W2/H | new lag-frame % recorded; DEFERRED_WORK entry + CAM_MAX_Y_STEP comment updated with the measured number |
| (f) Spawn + instant max H | teleport/spring-sim then +16 px/f immediately | §4's fallback trigger | no lag → down-only warmup CONFIRMED; lag → build `WarmupSideColumns` |

Plus, per the standing verification discipline: a gameplay-speed
`Lag_Frame_Count` run (lag counter is ground truth, not the profiler) and
mid-scroll leading-edge screenshots DURING motion for the (H6) hoist
correctness (same bar as Wave 1's).

## 9. Recommendation (Volence relays; Fable rules)

**Ship B+: (H1) column scan + (H2) corner + (H3) hysteresis + (H4) deadline
gate + (H5) 16 slots + (H6) bundled hoist. Init warmup stays down-only (§4).
Count budget + round-robin retained; the cost-denominated arbiter is specified
as the Phase-2 contract (§7), adopted at its seam when art streaming lands.**

Cost summary: +3,088 B RAM (3,072 lower + 16 upper), +16 B ROM table, est.
+150-250 B code; quiet-frame cost ≤3 decompresses on leftover budget behind a
time gate; no change to fill machinery, eviction semantics, or the budget unit.
Open parameters the A/B binds: `H_PFX_HYST` (initial 16 px),
`PFX_DEADLINE_LINE` (initial ~200), and the 18-slot fallback trigger (tag laps
in regime (c')).

Escalation forks (checkpoint before deciding unilaterally): regime (c') tag
laps at 16 slots → 18 slots vs pin-bit (pre-decided: 18 first); regime (f)
lag → side warmup; regime (d) vertical jump-arc churn → shared hysteresis.

---

## 10. IMPLEMENTATION OUTCOME (2026-07-16) — built, proven, Fable+design-reviewer ruled

Shipped B+ on `feat/unified-prefetch` (aeon + sigil), byte gate 4/4 both shapes,
strict suite 2252/0, clippy clean. Two design corrections surfaced by the A/B,
both ruled and executed:

### §3/H4 CORRECTION — the beam-position deadline gate was dead by design
The A/B (oracle, OJZScroll) found **`Tile_Cache_Fill` runs once per frame INSIDE
VBlank at V-counter ≈ 240** — it is called from the main-loop scene update, early,
in its fixed slot, with a full active frame of budget ahead. The §3/H4 premise
("read the VDP V-counter; if the beam is past line ~200 of 224 active, skip") was
FALSE: the literal `V >= 200 → skip` fires every frame (240 ≥ 200) and kills all
prefetch; and because Fill's V-position is ~240 every frame regardless of load, a
beam read can NEVER gauge frame load here. **H4 was reworked to a trailing-lag
gate:** `Frame_Counter` ticks once per VBlank on BOTH the normal (VInt_Level) and
lag (VInt_Lag) paths, so at the frame gate a delta > 1 since the last fill means a
frame lagged in between (`Cache_Pfx_Lag_Flag`). The tail then skips this frame's
speculation if the previous frame lagged — bounded to **≤ 1 consecutive skip**
(`Cache_Pfx_Skip_Armed`) so a sustained-lag run keeps pre-warming and cold columns
can't cascade back (demand fill is never gated; bounded worst case = pre-prefetch).
Self-contained in tile_cache (no VInt_Lag edit), release-safe (Lag_Frame_Count is
DEBUG-only). This constraint is load-bearing for the Phase-2 §7 arbiter: any
deferred-work gate must use a trailing indicator, not the beam.

### §8 EXIT-CRITERION AMENDMENT (for Volence's countersign at the merge gate)
Regime (a) as written ("0 VInt_Lag") is recorded **NOT-MET-AS-WRITTEN** and
re-scoped to: **"zero VInt_Lag attributable to cold-crossing decompresses"**
(Keys / `Block_Stage_Next` evidence) **+ "no lag regression vs the pre-prefetch
baseline"** (the A/B). Both hold. The residual is copy/draw-bound, not decompress,
and worse in the old code (§10 dossier).

### Two DISTINCT lag measurements (never blur them)
1. **Controlled A/B pair** (hash-verified ROMs, Frame_Counter-anchored, identical
   scripted drive reset→start×8→start×120→right×180): **OLD t16 44 lag / ≈224
   VBlanks (19.6%) vs NEW 27 lag / ≈207 VBlanks (13.0%)** — the H prefetch cuts
   sustained-max-horizontal lag ~40%; implied totals 180+44 and 180+27 are exactly
   what lag frames predict (internally consistent). Confirms the pre-existing branch.
2. **Corroborating free-gameplay run** (context only): hold-right 180 frames →
   22/180 lag at ~14 px/f. Not the controlled comparison; do not quote as such.

### Regime results (state-counter method, all six, none skipped)
- **(a) H cold control** — PASS (re-scoped). Next-col tags build 1/frame top-to-
  bottom (block_x=5: 0x05→0x15→0x25→0x35, the 90° analog of the V build); 0
  decompress-attributable lag; crossing lag is the pre-existing copy/draw residual.
- **(b) no regression** — PASS via the A/B (NEW 27 < OLD 44).
- **(c') diagonal / corner** — PASS. Both axes pre-stage (row 0x50 + col block_x=5);
  the **corner block 0x55 is staged and the corner code path executes** (breakpoint
  at the corner FindStagedBlock fired on the diagonal); pre-staged warmup tags
  0x40–0x44 SURVIVED (no vanish); no lag. The 16-slot ruling holds at the unified
  cadence — no tag lap, so the 18-slot fork does NOT fire.
- **(d) reversal thrash** — H_PFX_HYST=16 BINDS. The latch flips on a single 16 px
  reversal frame, but **decompresses = 0** over the whole ±16 px oscillation (both
  neighbor columns stay warm in the 16 slots → a flip re-probes an already-staged
  column, ~5 cheap probes, no decompress). No churn signature. Vertical jump-arc
  oscillation: also 0 decompress churn → the **shared-hysteresis fork does NOT
  fire**; vertical hysteresis stays not-taken.
- **(e) sustained max diagonal** — **~42% lag** (8 / 19 diagonal frames, post-
  prefetch), down from the pre-prefetch DEFERRED_WORK "~76%". The trailing-lag H4
  gate **demonstrably fires** here (execution hit the skip branch on post-lag
  diagonal frames — the lesson that killed the original H4, applied to its
  replacement), bounded ≤1 consecutive by construction.
- **(f) spawn + instant-max H** — down-only warmup CONFIRMED. The spawn+immediate-
  +16-px/f-horizontal case (regime a) shows the col prefetch staging fresh side-
  columns 1/frame with 0 quiet-frame lag; no cold-side-column lag → the
  WarmupSideColumns fallback does NOT fire.
- **H6 verify-during-motion** — mid-scroll leading-edge screenshot CLEAN (coherent
  foliage/ground/objects, no stale-tile garbage, no torn seam).

### Provenance (CRC32; sizes)
Final shipped tip: plain `453087`/`b335bdc6`, debug `461110`/`827e18c4`. Old-t16 A/B
baseline: debug `460661`/`a48fb0df`. (Hashes are CRC32 — the campaign standard;
earlier drafts of this note quoted SHA1 prefixes of the SAME files.)

### RE-ANCHORED on clean builds (2026-07-16, Fable merge-gate hold cleared)
The head-to-head A/B above (44 vs 27) used the INTERIM H4 ROM (beam-exclusion,
CRC32 `a4ed3b68`) as NEW. Re-run on the SHIPPED trailing-gate tip (`827e18c4`) vs
clean t16 master (`a48fb0df`), identical scripted drive: **OLD 39 lag / 200 frames
vs NEW 24 / 192 (~38% cut)** — the shipped gate confirmed better, same effect.
Regime (a) VInt_Lag free-run on the shipped tip: **0 breakpoint hits** on the quiet
cold-control frames, next-col tags build 1/frame (0x05->0x15->0x25->0x35). Regimes
(c')/(d)/(e) + the rider-1 skip-branch evidence all ran on `827e18c4`. The drive was
purely oracle-side (pokes/reset/press, no ROM patch), so the ROMs are the unpatched
builds — the hashes never differed by content, only by algorithm.
