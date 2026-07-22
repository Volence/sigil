# Core #1 — RunObjects dispatch loop (mini design gate)

> **CLOSED 2026-07-22 — DISSOLVED AT STAGE-0 (dissolution #4, new variant: hotness-measurement
> artifact).** Overseer RULED after independently verifying stage-0 (own canonical rebuild
> `00222415`/421134 · `fffc0179`/429151, trees clean at `5c975af`/`4993b0b`, `core.emp:504-519`,
> the `0x1C4`/`0x2EC` shape-elision, `s4.lst` `$29EA`/`$2A2C`/`$2A42`, and re-derived all four
> self-time subtractions). **The hotspot MOVED, not evaporated:** the census "34.8%" was
> DEBUG-shape *inclusive* (dispatched object code + a per-slot `Debug_AssertObjLoop` jbsr that
> emits zero bytes in the shipping shape); the addressable dispatch/cull **machinery self-time is
> ≈5.75%** in a **54%-idle** plain-shape frame. **RULING: do NOT cut a byte-changing RunObjects
> parcel; do NOT cut a standalone byte-neutral parcel either.** The two surfaces below (the
> `declared∖effective` `.culled_loop` sweep and the branchless-abs cull-math edit with its `$8000`
> pin) are **BANKED as gap-ledger rows** — they ride a future `core.emp` touch or an elected
> ceremony; this note is their reference.
>
> **REOPEN CONDITION:** a **real-scene lag report** re-opens the evidence. The "54%-idle → no lag
> lever" finding is **scene-relative** — measured on the churn *stressor*, not gameplay. If a
> shipping/gameplay scene ever reports frame-lag with RunObjects material to it, re-run stage-0
> under the real population and re-adjudicate.
>
> _(original design deliverable preserved below, unchanged)_

---

## Headline finding (read this first)

**The hotspot MOVED, it did not evaporate.** The census "RunObjects 34.8%" is an
**inclusive** call-graph figure dominated by *dispatched object code* + (in DEBUG) per-slot
assert overhead that **does not ship**. On the shipping (plain) shape, the portion a
RunObjects *loop* change can actually touch — the dispatch/cull **machinery self-time** — is
**≈5.75% of frame**, in a frame that is **54% idle**. Render_Sprites carries *more* addressable
self-time than RunObjects's machinery. Per the standing re-scope ruling this is a **"stop and
re-rank at the gate"** result, not a green-light-to-cut. Recommendation in §6.

---

## 1. STAGE-0 EVIDENCE (fresh, on master `5c975af`)

**Method.** Throwaway `Game_Entry` flip (`games/sonic4/config/game.asm:46` →
`GameState_ObjectTestChurn_Init`, **never committed** — reverted, canonical CRCs reproduced
`00222415`/`fffc0179`, tree clean). Scene = the dynamic-pool **churn** stressor: fills all
`NUM_DYNAMIC=40` slots with self-replacing `TestChurnObj`, steady-state 40/40 pool, self-driving
(no controller input). Profiler = oracle CPU call-graph (`set_profiler`), **inclusive** cycles
per JSR/BSR/RTS frame-node, averaged over the stated window. Reset → resume → steady state →
profile. Budget 128000 cyc/frame (NTSC).

**The profiler is INCLUSIVE, and it brackets `jbsr`-to-local-label as child frames.** Verified
against `s4.lst`: `RunObjects`=`$29EA`, `.run_always`=`$2A2C` (**3 calls** = players/system/effects
✓), `.run_culled`=`$2A42` (**1 call** = dynamic pool ✓). So RunObjects's inclusive number =
its two dispatch sub-loops **plus every object routine they jsr into**.

### Plain shape (SHIPPING) — 120-frame average

| node | incl cyc | incl % | note |
|---|---|---|---|
| VSync_Wait | 69405 | **54.2%** | idle — waiting for VBlank |
| RunObjects (incl) | 30534 | 23.9% | inclusive of all dispatched object code |
| ├ `.run_culled` (incl) | 24506 | 19.2% | dynamic pool; **contains** TestChurnObj_Main etc. |
| ├ `.run_always` (incl) | 4912 | 3.8% | players/system/effects (3 calls) |
| └ CompactDynamicLive | 889 | 0.7% | frame-end reconcile (not per-slot) |
| Render_Sprites (incl) | 16095 | 12.6% | |
| TestChurnObj_Main | 15159 | 11.8% | **dispatched object code — inside run_culled** |
| TestChurnObj (init) | 3435 | 2.7% | dispatched (replacement inits) |
| TestPlayer_Main | 3684 | 2.9% | dispatched (inside run_always) |

**Self-time decomposition** (inclusive − direct children):
- RunObjects top-self ≈ 30534 − 24506 − 4912 − 889 ≈ **227 cyc (0.2%)**
- `.run_culled` self ≈ 24506 − 15159 (Main) − 3435 (init) ≈ **5912 cyc (4.6%)** — the cull math + list walk + dispatch prologue for ~40 checks / ~28 dispatches
- `.run_always` self ≈ 4912 − 3684 (TestPlayer) ≈ **1228 cyc (1.0%)** — system/effect slots mostly empty, near-zero
- **Addressable dispatch/cull machinery self ≈ 5912 + 1228 + 227 ≈ 7367 cyc ≈ 5.75%**
  (CompactDynamicLive's 0.7% is a distinct O(count) reconcile, not per-slot loop work.)

### DEBUG shape — 100-frame average (to locate the census number)

| node | incl % |
|---|---|
| RunObjects (incl) | **30.9%** |
| ├ `.run_culled` | 24.1% |
| ├ `.run_always` | 4.0% |
| **Debug_AssertObjLoop** (31 calls) | **3.6%** |
| VSync_Wait (idle) | 46.6% |

DEBUG inclusive RunObjects = 30.9%, in the census's 34.8% neighborhood. The delta from plain
(23.9%) is the **per-slot `Debug_AssertObjLoop` jbsr** (`if DEBUG==1` inside the dispatch loop) —
**zero bytes in the plain shape** (the proc self-elides). So the census headline was measured on
a shape that inflates the dispatch loop with overhead the shipping ROM never runs.

**Stage-0 verdict.** RunObjects still clears the ≳2% D-survival bar *if* measured by machinery
self-time (5.75%). But the 34.8% was inclusive; the addressable ceiling is ~5.75% of a **54%-idle
frame**, and even that is mostly branchy cull math whose realistic saving is ~1.5–1.8% of frame.
The queue's "hotness-descending, RunObjects first" order was set on *inclusive* numbers and is
**wrong by addressable self-time**: Render_Sprites self (≈8.5–12% plain, see §6) exceeds it.

---

## 2. CLAIM SURFACE (the only defensible byte-changing transform, if the gate wants one)

I am **not** recommending we cut this (see §6). But if the overseer elects a RunObjects touch,
the single defensible transform is a **behavior-preserving tightening of the `.culled_loop`
per-iteration cull math** — nothing structural, no contract change.

**Proposed edit.** Replace the two `bpl/neg.w` conditional-abs sequences in the X/Y distance
checks (`core.emp:504–519`) with **branchless abs**:
```
    move.w  Sst.x_pos(a0), d0
    sub.w   Camera_X, d0
    move.w  d0, d1
    add.w   d1, d1            ; C = sign bit  → (or smi/ext form)
    ...  eor/sub branchless abs ...
    cmpi.w  #CULL_DISTANCE_X, d0
```
(exact form chosen at implementation to minimise bytes/cycles; the point is *no conditional
branch* in the hot per-slot path). Same for Y.

### Invariant analysis (8b R1–R4 mold)

- **R1 — load-bearing property.** The *observable* the loop preserves is: **which slots dispatch,
  in which order, with a0/d7 intact.** The cull decision `abs(dx) > CULL_DISTANCE_X` must be
  **bit-identical** to the current `bpl/neg; cmpi; bhi`. Branchless abs computes the same
  magnitude for every input **except** `$8000` (INT16_MIN), where `neg.w $8000 = $8000` (overflow,
  stays negative) and the branchless `eor/sub` form yields `$8000` too — **must prove the two agree
  at the `$8000` boundary**, load-bearing, since `dx = x_pos − Camera_X` is a wrapping 16-bit
  subtract and can equal `$8000`. This is the one non-obvious correctness pin.
- **R2 — architectural equivalence at the join.** `.culled_next` / dispatch consume **only** d7
  (counter), a2 (cursor, saved across jsr), a0 (slot). The cull block clobbers d0/d1 only; both are
  dead at `.culled_next` and re-loaded next iteration. Must confirm the branchless form introduces
  no new live register and leaves **no CC flow** into dispatch (dispatch sets its own flags). Trace
  at implementation.
- **R3 — load-bearing vs belt-and-braces.** Load-bearing: (a) the `$8000`-boundary agreement;
  (b) the cull *threshold semantics* (`>` vs `>=`) unchanged. Belt-and-braces: the branch-count
  reduction itself (perf, not correctness).
- **R4 — regression guard that fails LOUDLY.** Add a DEBUG assert/`static_assert`-style pin (or a
  sigil-side test) that the cull predicate is computed identically — concretely, a
  `Debug`-shape parity check is hard here; the practical loud guard is the **A/B in §3 run at the
  `$8000` boundary position** plus the strict-gate liveness lints. If a future edit reintroduces a
  branch or changes the threshold, the frame-anchored Object_RAM A/B diverges. (Weaker guard than
  8b's R3 toucher-census — another reason §6 leans re-rank.)

**Value ceiling.** ~2 predicted branches removed × ~28 dispatched + 40 checked slots ≈ a few
hundred cycles ≈ **≤0.3–0.5% of frame**. This is the honest number. It is **not** worth a
byte-changing 5-site-ripple parcel.

---

## 3. A/B METHOD (argued before results)

RunObjects is object-path → A/B in **ObjectTest/Churn via the throwaway `Game_Entry` flip**, NOT
the OJZ scroll A/B (standing rule; the scroll scene does not exercise the dispatcher).

**Throughput caveat (8b lesson).** A cull-math change *does* alter throughput (fewer cycles). So
**input-anchored A/B is invalid** — but the churn scene takes **no input**, so that failure mode
(lag-frame shift making identical input diverge) reduces to a single question: **does the saving
create/remove a lag frame?** The frame is **54% idle** (69405 idle cyc); a ≤500-cyc saving cannot
cross the VBlank deadline, so **`Frame_Counter` advances lock-step** OLD vs NEW. Since
`TestChurnObj` lifetimes are seeded from `Frame_Counter`, lock-step `Frame_Counter` ⇒ identical
spawn/expiry schedule ⇒ the observable is well-defined.

**The observable = frame-anchored `Object_RAM` + `Sprite_Table`.** Method:
1. Reset → churn scene → run to a **fixed `Frame_Counter`** value N (deterministic from reset;
   `run_to`/frame-token anchored, NOT press-count).
2. Dump `Object_RAM` (all slots) + the sprite attribute buffer (`Sprite_Table`).
3. Compare OLD (`5c975af`) vs NEW byte-for-byte at **several** N (e.g. 60, 180, 300) **and** at a
   frame chosen so a churner sits at a cull-boundary `dx==$8000`/`==CULL_DISTANCE_X` position
   (the R1 boundary — a bug both twins *don't* share must show here).

**Determinism controls.** No controller input (self-driving); anchor to `Frame_Counter` not wall
time; identical reset state; same shape (compare plain-vs-plain and debug-vs-debug). This captures
the *specific* preserved observable (per-frame object + sprite state), and the idle-margin argument
is what licenses frame-anchoring despite the throughput change. **If a lag frame ever appears
(it won't at 54% idle), this method is void and we fall back to a cycle-count-neutral edit only.**

---

## 4. COORDINATION CHECK

Riders on record: **D7 must preserve `ess_*_left_idx`** (entity_window #1 reuses it); **section H3's
descriptor interacts with G5** (`Act.max_tile_col`).

- **Does a RunObjects loop change touch `ess_*_left_idx`?** **No.** That is entity-window scan
  state (`EntityWindow_Scan`/`DespawnObjects`), a separate proc. RunObjects's dispatch loop reads
  `Dynamic_Live`, `Camera_X/Y`, SST `x_pos/y_pos/code_addr`, and writes `Spawn_Count` — disjoint.
- **Does it touch the section descriptor / G5 (`Act.max_tile_col`)?** **No.** Section/column code is
  untouched by the dispatcher.
- **Ripple-sharing for a queue re-order?** RunObjects (`core.emp`, engine region) does **not** share
  a ripple region with entity_window, section, or sprites parcels — each is its own `.emp` and
  engine.inc org span. So re-ordering the D queue (my §6 recommendation) has **no ripple-sharing
  penalty**: moving RunObjects later or parking it does not disturb the ew#1/#3/#4 → section H1/H3
  → animate chain, and does not force any rider to re-ripple. Clean to re-rank.

---

## 5. SCOPE + CEREMONY PLAN (if cut)

- **One parcel**, `core.emp` `.culled_loop` cull math, both twins (`core.emp` + `core.asm`) in
  lockstep. **Byte-changing and length-changing** (branchless abs is a different instruction count)
  → **full 5-site ripple**: `repin` does pins.rs only; **hand-edit** engine.inc orgs,
  `mixed_dac_rom` tail-call disps if downstream shifts, `repin_pins`; `repin.toml` only if a region
  is added (it isn't). **PROVENANCE re-baseline** (crc32+size, both shapes). Sequential cross-repo
  merge queue (check `origin/master` first), merge + push at close.
- **Attack-the-diff** from clean state expected (touches the object dispatch hot path).
- **Estimated byte impact:** small (a handful of instruction words in each twin), but **non-zero →
  the whole ceremony fires** for a ≤0.5%-of-frame, no-user-visible-benefit gain in a 54%-idle frame.

This ceremony/value ratio is the crux of the recommendation.

---

## 6. RECOMMENDATION

**Do NOT cut a byte-changing RunObjects loop parcel now.** Bring to the gate:

1. **The hotspot moved.** Addressable machinery self ≈ **5.75%** (plain), not 34.8% (that was
   DEBUG-shape inclusive + dispatched object code). Frame is 54% idle → zero lag lever.
2. **Re-rank the D queue by addressable SELF-time, not inclusive.** On this scene **Render_Sprites
   self (≈8.5–12% plain) > RunObjects machinery (5.75%)**. The sprites parcel (H1/H2/H3) looks like
   the real #1. I did not fully decompose Render_Sprites/Emit_ObjectPieces this session — that is
   the natural next stage-0 (its own mini gate).
3. **If the gate still wants a RunObjects touch**, prefer a **byte-NEUTRAL / contract-precision**
   change (the Parcel-B pattern — no ripple, no PROVENANCE) over the speculative branchless-abs.
   The `.culled_loop` clobber set and the a2-save discipline are candidates for a
   `declared∖effective` sweep; that pays no ceremony and can't regress bytes.
4. The branchless-abs claim surface (§2) + A/B method (§3) are **banked and ready** should the gate
   choose to spend the ceremony anyway — the analysis is done, the `$8000` boundary is the one pin.

**No code until the overseer rules on re-rank vs cut.**
