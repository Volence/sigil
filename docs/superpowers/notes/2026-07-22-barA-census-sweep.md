# Bar-A census sweep — remaining Parcel-D queue (measurement only)

> **STATUS: RULED-ACCEPTED 2026-07-22 — ALL 10 ROWS DISSOLVE; PHASE-2 D QUEUE → CLOSED-EARLY.**
> Overseer accepted in full (own canonical rebuild, `$40C4` + ew child symbols vs `s4.lst`, every
> decomposition re-derived). One session, plain-shape addressable SELF-time (inclusive − children,
> symbols mapped vs `s4.lst`) for every remaining D row + the two census-based parks. Masters aeon
> `5c975af` / sigil `4993b0b`. Canonical plain `00222415`/421134 reproduced; no flip left in tree
> (scroll-path rows measured on **canonical, no flip**; object rows reuse this session's plain churn
> captures). Both trees clean.
>
> **RULINGS FOLDED IN:**
> - **core #2** — reused churn capture ACCEPTED, no re-measure: the churn scene is
>   deterministic-from-reset with **no input** on a byte-identical canonical ROM (reproducible by
>   construction), and the dissolve is robust to ±0.5% since churn is the delete *stressor*. Reopen =
>   an **observed sustained >4 deletes/frame** storm.
> - **rings R2/R3** — harness-vs-close ruled **CLOSE BOTH** (recommendation adopted; no harness on
>   spec). Reopen = **ring-heavy level content approaching buffer saturation** (a headroom-for-content
>   condition); the **X=0-mask-after-conversion hazard rider travels with any reopen**.

## Scenes used

| scene | how | rows it exercises | Ring_Count | idle |
|---|---|---|---|---|
| **OJZ scroll** (canonical default `Game_Entry`) | drive `right` 150f held (max-H regime — worst case for scroll/stream/window) | section H1/H3, entity_window #1/#3/#4, **rings R2/R3** (scene has 13 live rings + RingCollision + DrawRings) | 13 | 48.8% |
| **churn** (`GameState_ObjectTestChurn_Init`, this session's plain capture, reverted) | 40/40 self-replacing pool, ~3 deletes/f | core #2 (delete-storm), animate A2/A3 | — | 54.3% |

Scene note: the census's "object-sparse scroll scene" caveat is **stale** — the current OJZ scroll
scene populates 13 rings + objects (RunObjects 2.1%, RingCollision, DrawRings, DespawnRings all live),
so rings/objects ARE measurable here at a realistic on-screen population.

## The table

All cycles = 120-frame average, budget 128000 cyc/frame (NTSC). Self = inclusive − direct children
(children mapped to `s4.lst` addresses). **Bar = ≳2% addressable self with a plausible transform.**

| Row | Target (site) | Scene | Inclusive | **Addressable SELF** | Transform value ceiling | Scene-bias (per lever) | Verdict |
|---|---|---|---|---|---|---|---|
| **section H1** | `Section_UpdateColumns` idle early-out :481 (`$57FC`) | OJZ max-H | 8036c / 6.3% | **1092c / 0.85%** (= 8036 − Draw_TileColumn `$40C4` 6944c[2 calls, the 2 fill sites]; RedrawPlanes/Draw_TileRow_FromCache ≈0 on horizontal scroll) | idle-frame subset ~450-550c (~0.4%) | max-H is WORST case (columns update every frame) → over-counts; gameplay ≤ this | **DISSOLVE** — the 6.3% "hot" was inclusive of the fill (Draw_TileColumn), which H1 doesn't touch |
| **section H3** | `Section_UpdateColumns` clamp :507 | OJZ max-H | (same routine) | **~50c / 0.04%** (structural: one clamp) | ~45-55c | scene-neutral micro-op | **DISSOLVE** (structural bound; no H1 ripple to ride) |
| **entity_window #1** | `EntityWindow_Scan` :901 scan loop (`$3892`) | OJZ max-H | 3695c / 2.9% | **849c / 0.66%** (= 3695 − DeriveWindow 148 − ScanRingsRight 574 − ScanObjectsRight 527 − DespawnRings 1597; RescanY/Slide/DespawnObjects ≈0) | ~500-650c (~0.4-0.5%) | max-H slides the window every frame → worst case, over-counts | **DISSOLVE** — the 2.9% was the whole window subsystem inclusive; the scan-loop lever is 0.66% |
| **entity_window #3** | `DespawnRings` hoist :1385 (`$3B34`) | OJZ max-H, 13 rings | 1597c / 1.2% | **≤1.2%** (loop-invariant hoist removes a fraction) | hoist ~0.3-0.4% | over-counts (max-H, ring-active) | **DISSOLVE** — sub-bar; no #1 ripple to ride (anchor dissolved) |
| **entity_window #4** | `DespawnObjects` :1500 (`$3BC0`) | OJZ max-H | ~0 (no in-window despawns this scene) | **<0.5%** (census ceiling ~300-700c) | ~300-700c | window-despawn dependent | **DISSOLVE** — cold + sub-ceiling; no ripple to ride |
| **core #2** | `DeleteObject` O(1) backpointer :250 (`$28EE`, LEAF — `.dyn_zero_scan` O(count) loop) | churn (delete-storm) | 2370c/3calls / 1.9% | **1.9% (all self — leaf)** | O(1) backpointer removes most of the O(count) scan (~most of 1.9%) | churn IS the delete-storm stressor (~3 deletes/f sustained) → gameplay ≤ this; over-counts | **DISSOLVE** — 1.9% in its own stress vehicle is sub-bar. **Closest call.** REOPEN if a sustained >~4-delete/frame storm is ever observed (census self-gate condition) |
| **animate A2** | `.set_frame` dirty-check `animate.emp:111` (`AnimateSprite $2F28`) | churn | 423c / 0.3% (AnimateSprite incl) | **~60c / 0.05%** (skip re-emit when frame unchanged) | ~60c | scene-neutral micro-op | **DISSOLVE** (structural bound) |
| **animate A3** | `.set_frame` jbsr+rts → jbra :113 | churn | (within AnimateSprite) | **~24c / 0.02%** (tail-call) | ~24c/advance | scene-neutral micro-op | **DISSOLVE** (structural bound) |
| **rings R2** | `DrawRings` fold :178 (`$3338`) | OJZ max-H, 13 rings | 947c / 0.7% | **0.7% self @ 13 rings** | fold removes a fraction (~0.2%); **structural ceiling @ ~32-ring full buffer ~2%** | 13 rings realistic on-screen; ceiling needs buffer saturation (under-counts vs theoretical max) | **DISSOLVE @ realistic pop** + **HARNESS-VS-CLOSE FLAG** (structural max could touch the bar only at ring-buffer saturation) |
| **rings R3** | `RingCollision` loop-invariant hoist :285 (`$33C8`) | OJZ max-H, 13 rings | 1020c / 0.8% | **0.8% self @ 13 rings** | hoist removes a fraction; **structural ceiling @ 128 rings ~2.3%** | 128 on-screen rings is not a real gameplay condition (VDP/scene caps) → theoretical only | **DISSOLVE @ realistic pop** + **HARNESS-VS-CLOSE FLAG** (128-ring ceiling; recommend close — no realistic scene reaches it) |

\* Review-board rider carried per instruction: the sprites **X=0-mask-after-conversion hazard** note
travels with the rings rows (mask insertion interacts with ring SAT emission) — relevant only if
rings R2/R3 ever reopen for a design gate.

## Verdict summary

**All 10 rows DISSOLVE under Bar A.** Every census "hot" number was **inclusive**; the addressable
self-time / transform-lever is sub-2% in every case:
- **section/ew** ("hot anchors" at 6.3%/2.9% inclusive) → self **0.85%/0.66%** — the inclusive figures
  were dominated by the tile-fill (`Draw_TileColumn`) and the window walkers, which the H1/#1 levers
  don't touch. Same inclusive-artifact as core #1 and sprites.
- **core #2** is the closest (1.9% self, leaf) but sub-bar **in its own delete-storm vehicle**; gameplay
  is lighter. Reopen only on an observed sustained storm.
- **animate A2/A3, section H3** close on **structural bounds** (~24-60c, orders of magnitude sub-bar) —
  no measurement scene needed, per the pre-ruling.
- **rings R2/R3** measure sub-bar at a realistic 13-ring population; their structural ceilings touch the
  bar only at ring-buffer saturation (R2 ~32 rings) / 128 on-screen rings (R3) — **the two harness-vs-close
  questions for the gate.** Recommendation: close both — no runnable or realistic scene reaches those
  populations, and even at the ceiling the *transform* (fold / hoist) removes only a fraction. Do not
  build a ring-saturation harness on spec.

**This is the "Phase-2 D queue closes early" shape the gate predicted** — the pass-2 / unified-prefetch /
free-lunch arcs already ate the lag (VSync idle 48.8% max-H, 54.3% churn; Lag=0 all regimes). No
object-render or scroll-path row clears the addressable-self bar.

## Recommendation to the gate

1. **Close all 10 rows with numbers** (dissolve). Bank each transform surface to the gap-ledger with its
   measured self + reopen condition (core #2 = delete-storm; rings = ring-saturation harness question;
   the rest = real-scene lag / elected headroom).
2. **Rings R2/R3 harness-vs-close:** recommend **CLOSE** (no realistic scene; fractional transform even
   at the ceiling). If the gate wants the ceiling proven, that is a *separate* harness-build decision —
   not this sweep.
3. **Phase-2 D queue → CLOSED-EARLY.** Milestone checkpoint to Volence with next-arc options: t18
   parallax port · H-streaming successor charter (rows 1066/1074) · VDP shared-module micro-batch
   (row 1073).
