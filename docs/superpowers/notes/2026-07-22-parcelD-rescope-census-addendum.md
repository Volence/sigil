# Parcel D re-scope census — ADDENDUM: object-render items re-verdicted vs a real population

The base census parked sprites/core/animate for lack of an object-heavy scene. The overseer
overruled the "no scene" premise: `games/sonic4/test/object_test_state.asm` (GameState_ObjectTest,
~40-object population: player + 25 level objects + 8 emitters/particles) and `test_churn.asm`
(GameState_ObjectTestChurn, delete-churn variant) are live in the tree. Booted each via a **local
throwaway `Game_Entry` flip** at `config/game.asm:46` (oracle-soak technique — measured, then
reverted; git clean, both shapes rebuild canonical 748ca5ba/d5d8e163). **Measurement only, no
harness code.** Same oracle profiler + accumulator-reset method + ≳2% bar as the design gate; plain
build (the scene loop's `else`-arm — no in-scene profiling overhead).

## ObjectTest profile (~40 objects, 120-frame avg, % of 128k frame)

| routine | % | note |
|---|---|---|
| RunObjects | **34.8%** | core #1/#2 path |
| VSync_Wait | 29.6% | idle — **no lag even at 40 objects** |
| Render_Sprites | **27.4%** | sprites H1/H3 path |
| Emit_ObjectPieces | **7.8%** | sprites H2 (`emit_piece_loop`) |
| AnimateSprite | **3.1%** | animate A2/A3 |
| TouchResponse | 4.2% | — |

## Churn profile (delete variant, 120-frame avg)

| routine | % | note |
|---|---|---|
| VSync_Wait | 54.3% | idle — no lag |
| RunObjects | 23.9% | — |
| Render_Sprites | 12.6% | — |
| **DeleteObject** | **1.9%** | 3 deletes/frame steady — no ~20-delete storm, no lag |

## Re-verdicts (base census → addendum)

| Item | Base | Evidence (real population) | **Re-verdict** |
|---|---|---|---|
| **sprites H1** | PARK | Render_Sprites **27.4%** (resolve chain on the hot path) | **SURVIVE** |
| **sprites H2** | PARK | Emit_ObjectPieces **7.8%** (`emit_piece_loop`) | **SURVIVE** |
| **sprites H3** | PARK | Render_Sprites 27.4% CPU-hot, but H3 is a *DMA-budget* win and VBlank never binds (≤55% window) | **SURVIVE — rides the sprites ripple** (H1/H2), low priority |
| **core #1** | PARK | RunObjects **34.8%** (`.run_culled` on the hot path) | **SURVIVE** |
| **core #2** | PARK | DeleteObject **1.9%** in Churn — below bar, no delete-storm, no lag; review self-gates "only if storms show" | **PARK** (gating condition unobserved even in its own churn vehicle) |
| **animate A2** | PARK | AnimateSprite **3.1%** | **SURVIVE** |
| **animate A3** | PARK | AnimateSprite 3.1% (same routine as A2) | **SURVIVE — rides the animate ripple** |
| **rings R2** | PARK | ObjectTest/Churn populate **no rings** — RingCollision/DrawRings absent from both profiles | **PARK** (harness-caveat: building ring population is out of scope) |
| **rings R3** | PARK | same — no ring population to measure | **PARK** (harness-caveat) |

## Revised Parcel D queue

**SURVIVE (11 items, 5 parcels):**
- **entity_window** #1 (anchor) / #3 / #4 — *(base census)*
- **section** H1 (anchor) / H3 — *(base census)*
- **sprites** H1 / H2 (anchors) / H3 (rides) — *new, Render_Sprites 27.4% + Emit 7.8%*
- **core** #1 — *new, RunObjects 34.8%*
- **animate** A2 (anchor) / A3 (rides) — *new, AnimateSprite 3.1%*

**PARK (3 items):**
- **core #2** (DeleteObject O(1)) — measured 1.9% in Churn, no storm/lag; the review's own
  "only if delete storms show" gate is unmet even in the delete-churn scene. Post-t18, or revisit
  if a real delete-storm regime ever lags.
- **rings R2 / R3** — no ring population exists in ObjectTest/Churn; building one is out of scope
  (overseer). Harness-caveat park: likely hot in ring-dense gameplay (R3 ~3k/f @ 128 rings), needs
  a ring-populated scene to earn the ceremony.

## Note
The 40-object scene stays idle (VSync_Wait ≥30%) — none of these are *lag* levers, but all clear
the ≳2% *hotness* bar (opportunity). The overseer's overrule was correct: the object-render path
is genuinely hot once populated; the scroll-test scene simply never exercised it.
