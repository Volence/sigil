# Parcel D (Tier-B object/render) — re-scope census

Stage-0 over all 13 Tier-B items against the overseer's survival bar. An item **survives** only if:
(i) it still exists as described in current code, AND (ii) it is measured-hot (≳2% in some regime
of the design-gate profile) OR byte-neutral/contract-only OR rides an already-paid ripple alongside
another survivor, AND (iii) it isn't superseded by a pending mechanism or by t18. **Default = PARK
to post-t18**, one logged row each; an item must earn the byte-changing ceremony. Overseer adjudicates.

**Existence (i): all 13 still present, none applied** (Explore agent verified each site in current
`.emp`). **Byte-neutral escape (ii-b): NONE** — every one is a code restructure (byte-changing); no
item is contract-only or DEBUG-assert-only. So survival rests on **hotness** or **riding a paid ripple**.

## Profile reference (design-gate, 3 regimes, % of 128k frame)

| routine | diagonal | max-H | max-V |
|---|---|---|---|
| Section_UpdateColumns | 2.4% | **6.3%** | **4.0%** |
| EntityWindow_Scan | 0.4% | **2.9%** | **2.8%** |
| RunObjects | 0.4% | — | — |
| Render_Sprites / emit | 0.3% | — | — |
| DrawRings / RingCollision | — | — | — |
| AnimateSprite | — | — | — |
| DespawnRings / DespawnObjects | — | — | — |

> **CAVEAT — the profile scene is object-SPARSE.** The design-gate profile is the OJZ scroll-test
> scene: a placeholder "player" box, **no rings, no badniks, no real object population**. It exercises
> the scroll/stream path (Section_UpdateColumns, EntityWindow_Scan run every frame) but barely touches
> the object-render path. So **sprites / rings / core / animate are structurally UNDER-measured** — they
> read cold here not because they're cheap but because nothing populates them. There is no gameplay
> scene available to measure them (the engine's only runnable scene is this scroll test). By the *letter*
> of the survival bar they all PARK; the *spirit* question — "are object-render opts measurable at all
> pre-gameplay?" — is the overseer's to weigh (§ Recommendation).

## Per-item verdicts

| Item | Site | Est saving | Hotness (profile) | Verdict (recommend) |
|---|---|---|---|---|
| **entity_window #1** | section loop `entity_window.emp:901`; walkers :1030/:1237 | ~500-650c/f steady | **Scan 2.9% max-H — HOT** | **SURVIVE** (anchor) |
| **entity_window #3** | `DespawnRings` :1385 | ~1-1.4k/f @ 20 rings | DespawnRings cold* | **SURVIVE** — rides #1's entity_window ripple |
| **entity_window #4** | `DespawnObjects` :1500 | ~300-700c/f | DespawnObjects cold* | **SURVIVE** — rides #1's entity_window ripple |
| **section H1** | `Section_UpdateColumns` idle early-out :481 | ~450-550c idle frames | **UpdateColumns 6.3% max-H — HOT** | **SURVIVE** (anchor) |
| **section H3** | `Section_UpdateColumns` clamp :507 | ~45-55c/f | same routine as H1 | **SURVIVE** — rides H1's section ripple |
| sprites H1 | Draw_Sprite/Render_Sprites resolve `sprites.emp:79/:275` | ~1.5-2k/f @ 30 obj | Render_Sprites 0.3%* | **PARK** (cold, no same-file survivor) |
| sprites H2 | `emit_piece_loop` :594 | ~1-1.9k/f @ 50-80 pieces | 0.3%* | **PARK** |
| sprites H3 | Critical-DMA length patch `Render_Sprites.done` :436 / `buffers.asm:71` | ~480 B Critical VBlank DMA | VBlank ≤55% window — **not binding** | **PARK** (DMA budget win, but VBlank never binds — design gate) |
| rings R2 | `DrawRings` fold :178 | ~2.5k/f @ full buffer | absent* | **PARK** |
| rings R3 | `RingCollision` loop-invariant hoist :285 | ~3k/f @ 128 rings | absent* | **PARK** (but see caveat — ring-heavy gameplay) |
| core #1 | `RunObjects.run_culled` :504 | ~200-350c/f @ 20-25 obj | RunObjects 0.4%* | **PARK** |
| core #2 | `DeleteObject` O(1) backpointer :250 | delete-storm dependent | not measured; review self-gates "only if delete storms show" | **PARK** (gated on unobserved condition) |
| animate A2 | `.set_frame` dirty-check `animate.emp:111` | ~60c when frame unchanged | absent* | **PARK** |
| animate A3 | `.set_frame` jbsr+rts → jbra :113 | ~24c/frame-advance | absent* | **PARK** |

\* under-measured — see the object-sparse-scene caveat.

## Supersession check (iii)
- **t18 (parallax port):** rewrites parallax code only — **none** of the 13 are t18-superseded. (The review's #1-overall item, "parallax H1," is t18 territory and is NOT in this Tier-B list.)
- **Pending mechanisms:** entity_window #1 **reuses** the dead `ess_ring_left_idx`/`ess_obj_left_idx`
  fields — a coordination note with the **D7 dead-code batch (Phase 2.5)**: D7 must NOT delete fields #1
  repurposes (flag when both are scheduled). section H3's `Act.max_tile_col` descriptor is data-format
  work adjacent to G5/act_descriptor (Phase 3) — not superseded, but worth a G5 coordination note. No
  item is fully superseded.

## Summary
- **SURVIVE (5): entity_window #1/#3/#4** (one parcel, #1 the hot anchor, #3/#4 ride its ripple) +
  **section H1/H3** (one parcel, H1 the hot anchor, H3 rides). These are the **scroll-path** items —
  the only routines the available scene measures hot.
- **PARK (8): all sprites (H1/H2/H3), all rings (R2/R3), all core (#1/#2), all animate (A2/A3)** — the
  **object-render path**, structurally cold in the object-sparse scroll scene, byte-changing, no same-
  file survivor to ride. Post-t18, one logged row each.

## Recommendation
1. **Cut the 2 scroll-path parcels** (entity_window #1/#3/#4 · section H1/H3) — measured-hot anchors
   with real per-frame savings, each with a mini design gate + A/B before commit.
2. **PARK the 8 object-render items** to post-t18 — but flag the *reason*: they can't clear the
   "measured-hot" bar because **no object-heavy scene exists to measure them**, not because they're
   proven cold. If object-render perf becomes a priority, it needs a gameplay/object-populated test
   harness first (there is none today). rings R3 (~3k/f @ 128 rings) and sprites H1/H2 (~1.5-2k/f @ 30
   objects) are the items most likely to matter once such a scene exists.
3. **Coordination flags:** D7 (Phase 2.5) must preserve the `ess_*_left_idx` fields entity_window #1
   repurposes; section H3's descriptor coordinates with G5 (Phase 3).
