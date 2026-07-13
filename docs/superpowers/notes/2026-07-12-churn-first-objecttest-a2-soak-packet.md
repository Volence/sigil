# Churn-first ObjectTest — A2 soak + entity-window churn profile

**Date:** 2026-07-12. **Branch:** `churn-first-objecttest-a2` (aeon `835967d` +
sigil, both off master). **NOT merged — Volence's gate.** Two deliverables in one
oracle session:

1. **A2 soak** — does AllocDynamic's compact-on-full fire mid-walk under genuine
   DYNAMIC-pool churn? (The prior soak, notes/2026-07-12-retro-fix-batch-packet.md
   §A2, could not reach the trigger: ObjectTest churns the EFFECT pool, dynamic
   pool stays static 40/40 — "not reached, not proven safe.")
2. **Churn profile** — frame shares of the entity-window walkers + RunObjects +
   TouchResponse under the churn (the carried churn-profile debt).

**HEADLINE: the A2 assert FIRED.** Under genuine dynamic-pool churn the mid-walk
compact-on-full hazard is REACHABLE — the walk-live rail (retro-fix item 1)
caught it exactly as designed. This is the missing evidence for the A2 design
ruling: the hazard is real, not merely theoretical. STOP per brief — no fix
designed or implemented; this is Volence's ruling input.

---

## The scene (test/content only — NO engine changes)

`games/sonic4/objects/test_churn.asm` — **TestChurnObj**, a self-replacing
dynamic-pool stressor. Each frame it counts down a staggered lifetime; on expiry
it `AllocDynamic`s a REPLACEMENT dynamic child (code_addr set immediately, per
AllocDynamic's caller invariant) and `DeleteObject`s ITSELF. Deletes zero the
live-list entry but leave `Dynamic_Live_Count` unchanged until frame-end
compaction, so the pool rides at `Dynamic_Live_Count == NUM_DYNAMIC` across a
frame while deletions free stack slots mid-walk.

`GameState_ObjectTestChurn` (added to `games/sonic4/test/object_test_state.asm`,
alongside the untouched `GameState_ObjectTest`) fills the dynamic pool to exactly
`NUM_DYNAMIC` (40) churners on a screen grid, plus a `TestPlayer` (TouchResponse
target). Per frame: `InitSpriteSystem → RunObjects → TouchResponse →
EntityWindow_Scan → Render_Sprites`. `EntityWindow_Scan` is called with the
window left inactive (`Entity_Window_Active=0`, `RingBuffer_Clear`) so it
early-outs to `DespawnRings` (no-op) + `DespawnObjects` — the fourth live-list
walker — over the churn pool (churners are UNTAGGED → walked and skipped, never
wrongly deleted). Entered at runtime by writing `GameState_ObjectTestChurn_Init`
to `Game_State` (the OJZ scroll test owns `Game_Entry`; the other agent's ojz
files were not touched).

**Why this reaches the trigger the effect-pool scene can't:** two churners
expiring in the same `.run_culled` walk at saturation — the first's replacement
alloc finds the free stack empty (saturated) and fails, but its self-delete
frees a slot; the second's alloc then finds `count == NUM_DYNAMIC` WITH a free
slot → `AllocDynamic` runs `CompactDynamicLive` mid-dispatch, holding a live-list
cursor. Exactly the A2 precondition.

---

## Deliverable 1 — A2 soak (DEBUG `s4.debug.bin`)

**Build:** `DEBUG=1 ./build.sh` → cp `s4.bin`/`s4.lst` → `s4.debug.bin`/`.lst` →
plain `./build.sh`. Verified distinct (debug md5 `f6049832…`, plain `c6b8f439…`,
`cmp` DIFFER), churn symbols in both listings, timestamps aligned (22:51).
**Driving:** press-only (`['start']`), never a bare `resume`, never `step_out` —
per ledger row 990. Breakpoint on `MDDBG__ErrorHandler`.

**Result: assert FIRED ~4 churn frames after scene entry** (Frame_Counter=0x2B=43,
= CHURN_MIN_LIFE, the first expiry band). Scene setup verified first:
`Game_State = GameState_ObjectTestChurn`, `Dynamic_Live_Count = 0x28 = 40`
(pool saturated) before the fire.

### Fire forensics (Volence's ruling input)

Faulting PC **`0x2B9E` = CompactDynamicLive+14** = the `assert.b d0, eq, #0`
walk-live rail (item 1). Call stack (bottom → top):

```
GameState_ObjectTestChurn+8   (0x065EAE)
RunObjects+26                 (0x002DD6)   ← .run_culled dispatch (jsr a1)
TestChurnObj_Main+12          (0x01120E)   ← object code AllocDynamic mid-dispatch
AllocDynamic+40               (0x0029BC)   ← compact-on-full capacity guard
CompactDynamicLive+32         (0x002BB0)   ← assert fired at +14
```

Machine state at the fire:

| symbol | value | meaning |
|---|---|---|
| `Dynamic_Live_Walking` | **0xFF (SET)** | a dynamic live-list walk in progress (`.run_culled`) |
| `Dynamic_Live_Count`   | **0x28 = 40 = NUM_DYNAMIC** | pool saturated → compact-on-full condition |
| `Dynamic_Live_Dirty`   | **0xFF (SET)** | a dynamic deletion happened this frame (freed the slot the alloc consumed) |
| a2 (walk cursor)       | 0xFFB01E = `Dynamic_Live`+12 | list index 6 — mid-walk |
| d7                     | 0x22 = 34 | walk loop counter remaining (snapshot count-1 = 39; ~6 walked) |

This is the textbook A2 hazard: a churner self-deleted (`Dirty=0xFF`, freed a
stack slot, zeroed its live entry, count stayed 40); a later churner in the SAME
`.run_culled` walk called `AllocDynamic`; free stack non-empty + count ==
NUM_DYNAMIC → the compact-on-full guard ran `CompactDynamicLive` while `a2` held
a live cursor → the walk-live assert fired. The rail works; the hazard is real.

**Interpretation vs the prior soak.** The prior ObjectTest soak reported
`Dynamic_Live_Dirty` NEVER set (no dynamic deletions — churn was in the effect
pool) and `CompactDynamicLive` never naturally invoked. Here `Dirty=0xFF` at the
fire is the crucial delta: genuine dynamic-pool deletion+alloc churn. The A2
trigger the prior soak called "not reached" IS reached the moment the churn moves
to the dynamic pool.

**Churn was genuine, not contrived-static** (verified on the plain build, which
does not halt): `Dynamic_Live_Count` oscillates in the 38–40 band; `object_list`
shows 38–40 live `TestChurnObj` in the dynamic pool across a 300-frame soak
(stable, no crash, no collapse); the profiler measured ~4 Alloc/Compact/Delete
calls per frame — sustained near-capacity dynamic churn.

**Recommendation for the A2 ruling (evidence only — no fix proposed):** the
hazard is now demonstrated REACHABLE with a minimal, realistic pattern (object
code spawning a dynamic child mid-dispatch while the pool is saturated — exactly
what `children.asm` does). "Not reached" is upgraded to "reachable and caught."
The occupancy-amendment-A2 design fix (hole-fill append / alloc-fail /
frame-end latch) is now warranted by live evidence, not just static tracing.
The rail stays regardless — it did its job.

---

## Deliverable 2 — churn profile (plain `s4.bin`)

**Profiler status:** oracle main **carries the stale-after-reload fix**
(`8871a17`, "flush accumulation buffers on ROM reload and profiler enable" —
ledger row 984, already CLOSED). **Jitter check PASSED:** captures move across
runs — single-frame (GameState 60.8% / RunObjects 34.5%) vs 60-frame-avg (65.4% /
40.6%) vs a second 60-frame-avg (65.2% / 40.4%), TestChurnObj_Main 34↔37
calls/frame, CompactDynamicLive 8.0%↔8.2%. Live data, not stale. ROM was reloaded
BEFORE enabling the profiler (no post-enable reload). 120-frame average, under
sustained ~4-replacement/frame churn at 38–40/40 occupancy (`Entity_Window_Active
= 0`, `Dynamic_Live_Count = 39`):

| Routine (task target in **bold**) | % frame | cyc | calls/frame | notes |
|---|---|---|---|---|
| GameState_ObjectTestChurn (whole state) | 65.3 | 83568 | 1 | inclusive |
| **RunObjects** | **40.5** | 51838 | 1 | inclusive (walk + churn machinery) |
| &nbsp;&nbsp;`.run_culled` (RunObjects+78) | 32.5 | 41604 | 1 | dynamic-pool walk |
| &nbsp;&nbsp;&nbsp;&nbsp;TestChurnObj_Main | 22.7 | 29063 | 34 | churner logic dispatched |
| &nbsp;&nbsp;&nbsp;&nbsp;CompactDynamicLive | **8.1** | 10357 | 4 | mid-walk + frame-end |
| &nbsp;&nbsp;&nbsp;&nbsp;AllocDynamic | 7.5 | 9611 | 4 | |
| &nbsp;&nbsp;&nbsp;&nbsp;Emit_ObjectPieces | 5.0 | 6442 | 35 | |
| &nbsp;&nbsp;&nbsp;&nbsp;DeleteObject | 2.5 | 3180 | 4 | |
| &nbsp;&nbsp;`.run_always` (RunObjects+56) | 6.2 | 7966 | 3 | player/system/effect sweeps |
| Render_Sprites | 17.5 | 22418 | 1 | |
| TestPlayer_Main | 5.3 | 6738 | 1 | |
| **TouchResponse** | **4.9** | 6316 | 1 | |
| **EntityWindow_Scan** | **2.2** | 2800 | 1 | ≈ DespawnObjects (Active=0 early-out) |
| **EntityWindow_DespawnObjects** | **~2.2** | ~2800 | (bra-tail) | dynamic-live-list walk, 39 untagged, ~72 cyc/entry |
| **EntityWindow_RescanY** | **0.0** | — | 0 | NOT exercised (see below) |
| VSync_Wait (idle headroom) | 28.3 | 36248 | 1 | scene NOT lagging (~28% idle) |
| VBlank/VInt (HINT) | 6.3 | 8011 | — | |

Budget 128000 cyc/frame; total 128004 (~100%, ~28% of it idle VSync).

### Notes on the entity-window targets

- **EntityWindow_DespawnObjects is entered by `bra.w` from Scan's tail** (Scan
  line 815), so the JSR/BSR/RTS profiler folds its cycles into
  `EntityWindow_Scan` rather than giving it a separate frame. With
  `Entity_Window_Active = 0` Scan does only: early-out check + `DespawnRings`
  (empty ring buffer, below the top-26 noise floor) + `DespawnObjects`. So the
  **2.2% / 2800 cyc `EntityWindow_Scan` figure IS essentially the DespawnObjects
  walk** — arithmetic confirms it: ~39 live entries × ~72 cyc/entry (load entry,
  null-guard, deref code_addr, tag-check, skip untagged) ≈ 2800 cyc. This is the
  fourth walker's cost under near-capacity churn: **O(live), bounded, ~2.2% of
  frame** — cheap, as the live-list design intended (vs a fixed 40-slot sweep it
  replaced).
- **EntityWindow_Scan-proper (spawn/scan) and EntityWindow_RescanY are 0%** —
  they require an ACTIVE streamed window (`Entity_Window_Active != 0`, set only by
  the full `Section_Init`/`BuildEntries` path) and, for RescanY, vertical camera
  motion across a 128px coarse row. A static-camera object-test scene structurally
  cannot drive them. That portion of the entity-window churn profile belongs to a
  streaming scene (the OJZ scroll state — the other agent's domain, out of scope
  here). The profiler is now confirmed working (fix present + jitter-verified), so
  that measurement is unblocked whenever a streaming-churn scene is available.

### Profile headlines

1. **CompactDynamicLive = 8.1% of the frame budget** under this churn (4
   calls/frame, ~2590 cyc each) — the A2 cost is not negligible: compaction is
   expensive AND, per deliverable 1, some of it runs mid-dispatch. Feeds the A2
   design ruling (a hole-fill/latch scheme also removes this mid-frame cost).
2. `.run_culled` dominates the frame (32.5%): churner dispatch + the
   alloc/compact/delete machinery. RunObjects total 40.5%.
3. DespawnObjects live-list walk ~2.2% — the "high-churn DespawnObjects" number
   the churn-profile debt asked for, at near-capacity occupancy.
4. The scene is NOT VBlank-bound (~28% idle VSync) — clean measurement, not a
   lag-saturated frame.

---

## Ledger rows closed / updated

- **`churn-first ObjectTest variant owed` (row ~1011, OPEN → CLOSED-with-evidence)**
  — the variant is built (aeon `835967d`) and the A2 assert FIRED under genuine
  dynamic churn. The A2 mid-walk-compact hazard is REACHABLE; "not reached" is
  resolved to "reachable and rail-caught". The design-fix decision
  (occupancy amendment A2) is now backed by live evidence — Volence's ruling.
- **`retro-audit A2 rider` (row ~1005)** — the DespawnObjects walk-live hook
  (item 12) is now soak-EXERCISED under churn (not just installed): the assert
  path is validated by a real fire from the `.run_culled` sibling walker; the
  rail is TOTAL and correct.
- **entity-window churn-profile debt (packet-tracked, retro-fix packet §A2)** —
  PARTIALLY closed: the **DespawnObjects live-list-walk cost** under near-capacity
  dynamic churn is measured (~2.2%, ~72 cyc/entry, O(live), bounded). The
  **Scan-proper + RescanY entity-STREAMING** profile stays OPEN — it needs an
  active streamed window (streaming scene, OJZ domain); the profiler is now
  confirmed live so it is unblocked, not blocked.
- **oracle profiler stale-after-reload (row 984)** — re-confirmed CLOSED: the
  running instance carries `8871a17`; this session's jitter check passed on
  changing data. (Independent corroboration of the row's "profile half can trust
  the tool".)

---

## What each pass produced

- **A2 evidence (the headline):** a minimal, realistic churn scene that reaches
  the compact-on-full-mid-walk state the prior soak could not — the assert fires
  in ~4 frames, with full forensics (PC, call stack, Walking/Count/Dirty). Live
  verification the design ruling needed.
- **Profile:** RunObjects 40.5% / TouchResponse 4.9% / EntityWindow_Scan(≈
  DespawnObjects) 2.2% / RescanY 0% (scene-limited) under churn; the standout is
  CompactDynamicLive at 8.1% (the A2 cost, quantified).
- **Neither-bucket:** the churn scene doubles as the definitive A2 stress the
  retro-fix packet asked for ("A churn-first ObjectTest variant would be the
  definitive stress") and as the vehicle that quantified the fourth walker's
  cost.

## Files

- `aeon/games/sonic4/objects/test_churn.asm` (new) — TestChurnObj stressor.
- `aeon/games/sonic4/test/object_test_state.asm` — +GameState_ObjectTestChurn(_Init).
- `aeon/games/sonic4/main.asm` — +test_churn.asm include (object bank).
- aeon branch `churn-first-objecttest-a2` @ `835967d`. NOT merged.
