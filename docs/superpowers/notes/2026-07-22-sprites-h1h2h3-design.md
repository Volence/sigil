# Sprites H1/H2/H3 (mini design gate — stage-0 + claim surfaces)

> **CLOSED 2026-07-22 — DISSOLVED AT STAGE-0 (dissolution #5, value-vs-ceremony).** Overseer RULED
> (b) after independently verifying stage-0 (own canonical rebuild `00222415`/`fffc0179`, `s4.lst`
> `$2DDA`/`$2EFA`/`$3338`, both resolve sites instruction-identical, PB1 terminator, SAT DMA 640B at
> `system/buffers.asm:67-72`, arithmetic re-derived). **Ranking CONFIRMED (Render_Sprites self 8.9% >
> RunObjects machinery 5.75%)** — sprites-first was correct — **but no parcel cut.**
>
> **SCENE-BIAS CORRECTION (overseer, adopted):** churn's conservatism is **per-path**. It
> *under*-counts the inclusive per-piece / rings / masks paths (H2/H3), but it **OVER-counts H1's
> per-object lever** — the 40/40 churn pool ≥ typical gameplay object counts, so **H1's ~0.5-1% is a
> CEILING, not a floor.** That puts H1 in exactly the core-#1 branchless-abs decline shape: no lag
> lever (54.3% idle), sub-1% real value, no already-paid ripple to ride, **survival bar not cleared
> standalone.** Consistency decides it. H2 = evaporated (already comptime-unrolled). H3 = non-binding
> VBlank DMA (PB1 dep satisfied). **All three BANKED as gap-ledger rows.**
>
> **REOPEN CONDITIONS:** a real-scene lag report, OR a deliberately-elected headroom pass. On reopen,
> H1's **FIRST GATE ITEM** = the R1 `mapping_frame`-drift trace as a **corpus-wide writer sweep**
> (twins + game code, NOT a reasoning argument) — a cross-object writer between `Draw_Sprite` and
> `Render_Sprites` makes resolve-once non-behavior-preserving, and the churn A/B cannot see it.
>
> _(original stage-0 deliverable preserved below; §1 numbers stand, the §1 "conservative floor" note
> is corrected per the scene-bias ruling above — CEILING for H1, floor for H2/H3.)_

---

## Headline

**Sprites-first re-rank CONFIRMED by measurement.** `Render_Sprites` self ≈ **8.9%** (plain,
churn) — larger than RunObjects's dispatch machinery (5.75%) — and, unlike core #1, the churn scene
**under**-represents sprite cost (single-piece objects, no children/rings/masks), so this is a
**conservative** floor, not an inflated headline. **But the frame is still 54.3% idle → opportunity,
not lag.** H1 (resolve-once) is the one genuine lever; **H2 has largely evaporated** (emit_piece_loop
is already comptime-unrolled, zero JSR/piece); **H3 is a non-binding VBlank-DMA win** (PB1 dep
satisfied). Recommendation in §6.

---

## 1. STAGE-0 UNDER BAR A (fresh plain-shape self-time)

**Method.** Same discipline as core #1: throwaway `Game_Entry` flip →
`GameState_ObjectTestChurn_Init` (40/40 dynamic pool, self-driving, no input), **reverted**;
canonical CRCs reproduced `00222415`/`fffc0179`, both trees clean. Plain shape. oracle inclusive
call-graph profiler, **100-frame average**, budget 128000 cyc/frame. `Render_Sprites`'s loop labels
(`.band_loop`/`.object_loop`/`.sibling_loop`) are branches, **not** `jbsr` — so it is ONE inclusive
node; its only `jbsr` children (code-confirmed at `sprites.emp:345/352/400/421/429`) are
`Emit_ObjectPieces`, `InsertSpriteMasks`, `DrawRings`.

### Decomposition (every child named + mapped to s4.lst)

| node | s4.lst addr | incl cyc | incl % | calls | role |
|---|---|---|---|---|---|
| VSync_Wait | `$22D0` | 69411 | **54.3%** | 1 | idle |
| **Render_Sprites** | `$2BDE` | 16120 | 12.6% | 1 | (inclusive) |
| ├ Emit_ObjectPieces | `$2DDA` | 4552 | 3.6% | 25 | child (leaf) |
| ├ InsertSpriteMasks | `$2EFA` | **0** | 0 | 0 | child — absent this scene (`SpriteMask_Y=0` → early-out) |
| └ DrawRings | `$3338` | 170 | 0.1% | 1 | child — no rings → early-out |
| Perform_DPLC | `$2776` | 708 | 0.6% | 1 | **NOT** a Render_Sprites child (DPLC art path) |
| Draw_Sprite | `$2B1C` | <44 | — | — | absent from top-40 (cheap registration) |

**SELF-time (inclusive − direct children):**
- **`Render_Sprites` self = 16120 − 4552 − 0 − 170 = 11398 cyc ≈ 8.9%** — the band walk + per-object
  frame-resolve + SAT build. **[H1 territory]**
- **`Emit_ObjectPieces` self = 4552 cyc ≈ 3.6%** — it is a **leaf** (`emit_piece_loop` is inlined
  via comptime, zero JSR/piece; code `sprites.emp:594/635-652`), so inclusive = self. **[H2]**
- **Combined sprite-render self ≈ 12.5%.**

**Hypothesis CONFIRMED.** The first-pass ≈8.5–12% (Render_Sprites incl 12.6% minus children) resolves
to **8.9% self**. Sprites-first stands: 8.9% > RunObjects machinery 5.75%.

### Scene-relativity (opposite sign to core #1 — this is CONSERVATIVE)

Core #1's churn *over*-stated its target (cheap stub objects). Sprites is the reverse: churn objects
are **single-piece 16×16** test objects, **no multi-sprite children, no rings, no masks**. So:
- **H2** (`emit_piece_loop`, per-*piece*) scales with total pieces/frame — churn ≈25 pieces; a busy
  gameplay frame approaches the 80-piece SAT cap → measured 3.6% is a **floor**.
- **H1/Render_Sprites self** scales with on-screen object count (bounded by MAX_VDP_SPRITES=80) and
  gains the multi-sprite sibling-walk cost that churn never exercises.

So real gameplay has **≥** this sprite cost — the ranking is robust, not fragile. **Still, VSync_Wait
54.3% idle ⇒ no lag lever** (same as core #1); the ceremony/value tension is per-item.

---

## 2. H1 — resolve-once (the real lever)

**The redundancy.** The `(mapping_frame → frame-data pointer)` resolution runs **twice per on-screen
single-sprite object per frame**, from identical inputs:
- `Draw_Sprite:79-84` — resolves to read the frame's **bbox** for the exact cull.
- `Render_Sprites:275-285` — resolves **again** to read the frame's **pieces** for emit.

Same `a1`=mappings base, same `mapping_frame`, same 5-instruction sequence (`moveq/move.b/add.w/
move.w/lea`). The second resolve is pure repeat.

**Transform.** Cache the resolved frame-data pointer (or piece-list pointer + count) in a per-SST
render-scratch field during `Draw_Sprite`; `Render_Sprites` reads it instead of re-resolving.
Precedent: `Sst.sprite_piece_count` already caches per-SST render scratch
(`PopulateSpawnedPieceCount`/`RefreshSpritePieceCount`).

### Invariant analysis (8b R1–R4 mold)

- **R1 — LOAD-BEARING invariant (the one to close before build):** `mapping_frame`/`mappings` must
  not change between `Draw_Sprite` (called during the object's RunObjects dispatch) and
  `Render_Sprites` (after all dispatch). Plausible by convention — an object animates then registers,
  and nothing mutates its frame post-dispatch — **but this must be TRACED** against the object
  contract (does any object call `Draw_Sprite` *before* a later `AnimateSprite`? does any post-dispatch
  pass touch `mapping_frame`?). If it can drift, the cache is stale and the SAT corrupts. **This trace
  is the gate item for H1** (analogous to 8b's "prove the 2 gen sites cover every claim").
- **R2 — multi-sprite carve-out:** `Draw_Sprite` **skips** multi-sprite children (offscreen-flagged,
  :62-63) so no cache is written for them; `Render_Sprites` resolves children with the **parent's**
  `mapping_frame` on the **child's** mappings (:361-372) — a *different* resolution the cache doesn't
  cover. So the cache is **single-sprite-object only**; children keep the inline resolve. The
  cache-read must be gated (non-child + cache-valid), else it reads a stale/never-written field.
- **R3 — load-bearing vs belt-and-braces:** load-bearing = the frame-stability invariant (R1) + the
  child gating (R2). Belt-and-braces = a fallback to inline-resolve when the cache is marked invalid.
- **R4 — LOUD regression guard:** DEBUG-shape assert in `Render_Sprites` that re-resolves inline and
  `assert.l eq` against the cached pointer. If a future object ever drifts `mapping_frame` between the
  two passes, it fires at the source (self-elides to zero bytes in plain, rings.emp precedent).

**Value & ceremony.** Saves ~5 instr (~24-30 cyc) per on-screen single-sprite object in Render_Sprites
self — churn ~25 objects ≈ 600-750 cyc ≈ **0.5%**, scaling with object count (higher in gameplay).
Adds a per-SST scratch field → **RAM-layout ripple** (SST grows, or reuse an existing scratch slot) +
Draw_Sprite store + Render_Sprites load/branch → **byte- and possibly RAM-changing → full 5-site
ripple + PROVENANCE + attack-the-diff.** Modest value, real redundancy.

**A/B (§3 rules):** ObjectTest/Churn, frame-anchored on `Frame_Counter`; observable =
`Sprite_Table_Buffer` (the SAT output) + `Object_RAM`, byte-compared at fixed frames. Resolve-once
must produce a **bit-identical SAT** every frame. **Bar B:** record the lag-frame counter both sides.

---

## 3. H2 — emit_piece_loop (LARGELY EVAPORATED)

`emit_piece_loop` (`sprites.emp:594`) is **already** comptime-unrolled into four flip-variants,
**zero JSR per piece**, with the `MAX_VDP_SPRITES` cap-check folded into the `dbeq`. This is the
pass-2 sprite work; there is little addressable residual. The per-piece body is 5 loads + 4 flip
`{term}` splices + `cmpi` + `dbeq` — near-optimal.

The only candidate micro-op is the **4-way flip dispatch prologue** (`Emit_ObjectPieces:637-643`, 3
`cmpi/beq` per *call*) → a jump table: ~10 cyc × 25 calls ≈ 250 cyc ≈ **0.2%**, per-call not per-piece.
Marginal.

**Verdict: H2 substantially evaporated (Parcel-C pattern) — the review-doc target (`~1-1.9k/f @
50-80 pieces`) was against the pre-unroll loop.** BANK it; do not cut. If a specific per-piece win is
later identified, it rides H1's ripple (same file).

---

## 4. H3 — Critical-DMA length shrink (VBlank DMA, NOT CPU self-time)

`buffers.asm:71` DMAs a **fixed 640-byte** SAT to VRAM every frame; H3 shrinks the DMA length to
`Sprites_Rendered`×8 (up to ~480 B saved). **This is a VBlank DMA-bandwidth item — the profiler's CPU
self-time does not measure it.** Census: VBlank ≤55% window → **not binding**, so H3 saves
non-scarce bandwidth with **zero current lag benefit**.

- **PB1 dependency: CONFIRMED SATISFIED in current code.** The had-sprites→none edge terminator
  (`sprites.emp:440-453`, edge-triggered zero-sprite terminator) + `Sprites_Rendered` persistence
  (`:38`, deliberately not reset) are present — PB1 shipped in the wave-2 bugfix batch. A shrunk DMA
  relies on both, and both are in place.
- **Edge care:** shrinking to `Sprites_Rendered`×8 must still DMA ≥8 bytes on the had-sprites→none
  frame (the terminator write), and the static DMA entry (`Static_Sprite_DMA`) becomes dynamically
  length-patched → byte-changing DMA-build path.

**Verdict: PARK (as the census had it) — non-binding DMA budget, no lag lever.** Only earns a cut if a
DMA-bound scene appears, OR it rides H1's ripple for free *and* the overseer wants the headroom banked.

---

## 5. COORDINATION

- **vs D7 `ess_*_left_idx`:** sprite code (`Draw_Sprite`/`Render_Sprites`/`Emit_ObjectPieces`) does not
  touch entity-window scan indices. **Disjoint.**
- **vs section-H3 / G5 (`Act.max_tile_col`):** sprite render does not touch section descriptors.
  **Disjoint.**
- **Sprites-internal (H1/H2/H3 all in `sprites.emp` = ONE shared ripple region):** unlike core #1 this
  is a real design question. If H1 is cut (byte/RAM-changing), it ripples the sprites region; H3 (if
  elected) and any H2 micro-op would ride the **same** ripple + PROVENANCE (one ceremony, 8b-style
  fold). **Bundling plan:** H1 anchor; H3 rides only if it earns headroom; H2 banked. One sprites
  parcel, one ripple.

---

## 6. RECOMMENDATION

1. **Sprites-first re-rank CONFIRMED** — Render_Sprites self 8.9% > RunObjects 5.75%, and churn is
   conservative for sprites. The provisional roadmap order holds; no re-rank needed.
2. **The no-lag reality persists** (54.3% idle). By the core #1 precedent, this alone can justify
   **DISSOLVE-AT-STAGE-0 + bank the surfaces**. The overseer's call: is 8.9% self (scene-conservative,
   the largest object-render lever) enough to spend one sprites-parcel ceremony, given zero lag?
3. **If we cut:** **H1 (resolve-once) is the sole anchor.** It is a genuine redundancy (double
   resolve). Its gate item is the **R1 mapping_frame-stability trace** — close that first; if it holds,
   H1 proceeds with the R4 loud DEBUG guard + frame-anchored SAT A/B (bar B). H2 is **banked**
   (evaporated); H3 is **parked** (non-binding DMA) unless it rides H1's ripple for banked headroom.
4. **If we dissolve:** bank H1 (resolve-once + R1 trace + R4 guard), H2 (flip-dispatch jump table),
   H3 (DMA-length shrink + PB1-satisfied note) as gap-ledger rows, reopen on a real-scene lag report —
   exactly the core #1 shape.

**No code until the overseer rules: cut H1 (with the R1 trace as the build gate) vs dissolve-and-bank.**
