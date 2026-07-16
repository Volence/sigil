# Unified direction-aware block prefetch — merge packet (2026-07-16)

**Branch:** `feat/unified-prefetch` (aeon + sigil). **Design:** the B+ note (this dir,
`2026-07-16-unified-prefetch-design.md`, §10 = the implementation outcome). **Gate:**
byte gate 4/4 both shapes, strict suite 2252/0, clippy clean. **For Volence's
countersign at the merge gate + Fable's hot-path second look.**

## What shipped (six mechanisms, one file pair each shape)
- **H5** slots 12→16 (`BLOCK_STAGE_SLOTS`) — lap-rate model (design §5).
- **H1** column-scan k=1 prefetch — the row scan mirrored 90° (`Cache_Prev_Cam_X`
  direction, target block-col beyond `Cache_Head_Col` / before `Cache_Left_Col`,
  enumerate the spanned block-rows, stage first unstaged, leftover budget after row).
- **H3** horizontal direction hysteresis — latch (`Cache_H_Pfx_Dir`) flips only after
  ≥ `H_PFX_HYST` = 16 px net opposite motion (`Cache_H_Pfx_Accum`). Vertical = none.
- **H2** diagonal corner — when both axes cross, stage the (next-row × next-col) block
  last (`Cache_Pfx_Row/Col_Target`, priority row → col → corner).
- **H4** — **reworked from the design's beam-position gate.** See the correction below.
- **H6** FillColumn/CopyBlockColumn base-lea hoist (a5/a6, sentinels by displacement).

## The two design corrections (A/B-surfaced, ruled, executed)
1. **H4 was dead by design.** `Tile_Cache_Fill` runs once/frame INSIDE VBlank at
   V-counter ≈ 240 (measured), so the beam can never gauge frame load; the literal
   `V >= 200 → skip` killed all prefetch. **Reworked to a Frame_Counter-delta
   trailing-lag gate** (delta > 1 = a frame lagged → skip this frame's speculation,
   bounded ≤ 1 consecutive skip). Self-contained, release-safe. Verified FIRING on
   post-lag diagonal frames (rider 1: breakpoint on the skip branch @ 0x5606).
2. **Exit-criterion amendment (regime (a) NOT-MET-AS-WRITTEN).** Re-scoped to "zero
   VInt_Lag attributable to cold-crossing decompresses + no lag regression vs the
   pre-prefetch baseline." Both hold. **← Volence countersigns this at the gate.**

## Two DISTINCT lag numbers (do not blur)
- **Controlled A/B** (hash-verified ROMs, Frame_Counter-anchored, identical scripted
  drive): **OLD t16 44 lag / ≈224 VBlanks vs NEW 27 / ≈207** → ~40% cut on sustained-
  max-horizontal. Internally consistent (180+44, 180+27). The comparison of record.
- **Free-gameplay corroboration** (context only, NOT the comparison): 22/180 at ~14 px/f.

## Regimes (state-counter method; all six, none skipped) — detail in design §10
(a) PASS re-scoped (next-col tags build 1/frame). (b) PASS via A/B. (c') PASS — corner
block staged + corner path fires + warmup tags survive; 16-slot ruling holds (18-slot
fork not fired). (d) H_PFX_HYST=16 binds — latch flips on ±16 but 0 decompress churn
(slots absorb); vertical jump-arc 0 churn → shared-hysteresis fork not fired. (e)
sustained-max-diagonal ~42% (was ~76%); H4 gate fires. (f) down-only warmup confirmed
(no cold-side-column lag) → WarmupSideColumns not fired. H6 mid-scroll screenshot CLEAN.

## Ledger / doc-sync (this packet's obligations)
- campaign-gap-ledger: dossier row + slot-ruling row EXTENDED (built/proven, 16-slot
  re-test); NEW rows — "the horizontal Wave-1 that never happened" (copy/draw-bound
  residual, **domain split: Draw_TileColumn → plane_buffer; FillColumn → tile_cache**)
  and the VBlank-execution constraint (Phase-2 arbiter input).
- ENGINE_ARCHITECTURE §4.7 prefetch description rewritten; §9.7 user-mode drift FLAGGED
  (not fixed). CAM_MAX_Y_STEP comment + DEFERRED_WORK diagonal entry updated with the
  measured ~42% (was stale ~76%). Design note §3/H4 corrected + §10 outcome added.

## Provenance
Final: plain `453087`/`e9b3e9fa`, debug `461110`/`1e47bf0c`. Old-t16 A/B baseline:
debug `460661`/`1f93a71f`.

## Pass breakdown (step-3 vs step-5 framing)
This is not a port tranche (byte-CHANGING throughout), but for parity:
- **Mechanism/design work (the charter):** H1–H6 + the H4 rework — all new per-frame
  code; every tradeoff commented to the FillRow bar for Fable's second look.
- **Neither-bucket headline:** the H4 beam-gate was falsified by measurement, not
  review — the A/B is what caught that Fill runs in VBlank. The lesson (verify the
  gate can fire) was applied to the replacement (rider 1).

## Merge mechanics (rider 2 — for the gate)
`feat/unified-prefetch` carries the code + the CORRECTED design note (§3/H4 + §10). The
`design/unified-prefetch` branch holds the original (falsified beam-gate) note only.
**Proposed:** merge `feat/unified-prefetch` (note + code together) so master never holds
the falsified note beside the reworked code; fold/retire `design/unified-prefetch` into
it rather than merging the stale note separately. Confirm this topology at the gate.
Both t16 and this work are still LOCAL (unpushed) — push decision is Volence's.
