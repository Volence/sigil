# Tranche-11 sprites.emp — Fable review addendum (post-step-5 second look)

2026-07-11 · Fable, at Volence's request, after the tranche's own step-5
returned "no changes." These are the work list for the follow-up wave
(t11 is MERGED — all items are post-merge work on fresh branches).
Line refs = sprites.emp at merge.

## A. Step-5 optimization — TAKE (byte-changing, own wave)

**A1 — Camera-bias fold.** Every piece, all four Emit_ObjectPieces
variants, pays `addi.w #VDP_SPRITE_Y_OFFSET` + `addi.w
#VDP_SPRITE_X_OFFSET` (lines 474/487 + clones) — 16 cycles/piece. The
bias is frame-constant: compute `Camera_X_Biased = Camera_X -
VDP_SPRITE_X_OFFSET` (and Y) once per frame (frame-loop or
InitSpriteSystem — pick where Camera_* is final), subtract the BIASED
values in the per-object setup (lines 266/268 and the sibling clone
344/346), delete the per-piece addis from all four loops. `.screen_pos`
paths (273-274, 349-350) compensate with explicit addis (rare path).
The flip variants ride along (bias lives in d2/d3; neg/width math never
touches it). Expected: ~16 cyc × pieces-on-screen per frame (~500-800
at typical load) + ~32 bytes. Behavior-identical but byte-changing:
lockstep twin edit + re-pin + oracle screenshot-compare (same scene
pre/post, pixel-identical) + profiler delta recorded in the packet.

## B. Decide-and-document (cheap, can ride any touch)

**B1 — Scanline budget consistency.** The budget block's early-out
(281-282, `d5 < SCANLINE_SPRITE_LIMIT → .budget_ok`) skips the COMMIT
too, so the first 20 sprites are never charged to band counters (budget
lenient ~2× per band). Multi-sprite children (sibling walk 315-363)
bypass the budget entirely. EITHER always-commit + charge children
(costs ~20 cyc/object under the limit) OR keep as-is with a comment
declaring the budget a soft heuristic that undercounts by design.
Volence's call at review; default = comment (the hardware drops excess
per-line sprites anyway; the budget only shapes WHICH drop).

**B2 — Silent-tradeoff comments** (one line each, no behavior change):
- Draw_Sprite cascade-down (125-130): overflowing high-priority sprite
  renders BEHIND lower band rather than dropping — CHOSEN.
- `.band_limit_pop` (413): skips DrawRings; equivalent today only
  because DrawRings can't emit at cap — comment guards the coincidence.
- Link-order cycling (207-216) flips same-band overlap z-order every
  frame even without overflow — CHOSEN (fairness beats stable z).
- Draw_Sprite `.offscreen` for multisprite children (60): rendered
  children keep RF_ONSCREEN CLEAR — nothing may ever key culling on
  that flag for children.
- InsertSpriteMasks height rounds UP to 32-scanline multiples (653-655)
  — if intended, say so at the site.

## C. Oracle probes (name the outcome in the next packet)

**C1 — X=0 mask first-sprite-on-line exemption.** The VDP may not mask
when no earlier-linked sprite touches the masked scanline (the classic
quirk). InsertSpriteMasks inserts at a band boundary, so coverage
depends on scene content. Probe on oracle/BlastEm: empty high band +
mask configured → does masking hold? If not, the feature needs a
guaranteed leading sprite on the masked lines (or a documented scene
contract).

## D. Protect (do NOT "optimize" away)

**D1 — The per-piece `cmpi.b #MAX_VDP_SPRITES / dbeq` net** (493-494 +
clones) is the ONLY overflow guard for objects with stale/zero
sprite_piece_count (the pre-checks at 231-240/334-338 trust the cache).
Load-bearing. A future unification (the {code}-splice emit_piece_loop
retrofit) must keep it in the skeleton.

## Process outcome

Step 5's text was an open question and got an anchored answer ("already
hand-unrolled → nothing to do"); every item above is STATIC inspection —
no profiler required. The loop now carries the step-5 interrogation
(invariant ladder / counter audit / guard coverage / hardware
cross-check / tradeoff comments) + the hot-path Fable second look, both
added same-commit as this note.
