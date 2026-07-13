# Occupancy Amendment A2 — overflow latch: implementation packet

**Date:** 2026-07-13. **Branch:** `occupancy-a2-latch` (aeon `264037b` + sigil
`fa02f91`, both off master). **NOT merged — Volence's gate.** Spec:
`docs/superpowers/specs/2026-07-11-object-pool-occupancy-design.md` §9 (RULED
2026-07-12). Evidence trail: the churn-first soak that rail-caught the hazard
(`notes/2026-07-12-churn-first-objecttest-a2-soak-packet.md`).

**Final state:** full workspace strict **2211/0**, clippy clean, core_port twin
byte-parity both shapes, repin idempotent. Plain `s4.bin` md5 `393dd0e3…`
(452500 B), debug `s4.debug.bin` md5 `0c1c6fab…` (460501 B).

---

## The ruling, implemented

AllocDynamic's full-count path (which the 2026-07-12 soak proved fires
`CompactDynamicLive` mid-walk under a held cursor — the stale-tail
double-dispatch hazard) now **latches** the popped slot instead of compacting.
The RunObjects frame-end tail drains the latch: one `CompactDynamicLive` (no
walk live), then append the latched entries in alloc order. 4+ compacts/frame →
1 frame-end compact.

## The 7 edits (both twins in lockstep unless noted)

1. **`NUM_DYNAMIC_PENDING = 8`** — `engine/constants.asm` + `constants.emp`
   (+drift-lock ensure). Latch capacity (byte-neutral equate).
2. **`Dynamic_Live_Pending` (8 words) + `_Count` (word)** — `engine/ram.asm`, at
   the RAM tail after the DEBUG-walk/pad byte. **RELEASE both shapes** (this is a
   release fix, not DEBUG-gated); placed so both shapes carry it at the same
   offset → `Engine_RAM_End` moves +18 identically. Ripples game RAM only.
3. **AllocDynamic latch** — at `count == NUM_DYNAMIC`: if latch full →
   `.latch_full` (roll back the pop: `addq.w #2,(Dynamic_Free_SP)` — the slot was
   already popped, so a bare alloc-fail would LEAK it) → `.full` alloc-fail; else
   append the popped slot to `Dynamic_Live_Pending[count++]` and return success
   (the slot IS allocated; it dispatches next frame). The old movem + `bsr
   CompactDynamicLive` is gone — AllocDynamic no longer mutates the live list
   mid-frame.
4. **DeleteObject pending-zero** — the dynamic arm's live-list scan+zero is
   followed by a symmetric scan+zero of `Dynamic_Live_Pending` (the `.dyn_zero_scan`
   flows into `.dyn_zero_pending`). Extends the A1 "exactly once" invariant to the
   latch: a slot latched, deleted, and re-latched the same frame would else be
   listed twice by the drain → permanent double-dispatch (the A1 class).
5. **CompactDynamicLive** — keeps the compact core + the walk-live rail + the
   `count ≤ NUM_DYNAMIC` assert; the §6-2/§6-3 post-state DEBUG block is REMOVED
   (moved to edit 6). Plain shape byte-UNCHANGED here (the removed block was
   DEBUG-only); header updated (called only from the frame-end reconcile now).
6. **`DrainDynamicPending`** (new proc) — appends each non-zero
   `Dynamic_Live_Pending` entry to `Dynamic_Live` in alloc order (a zeroed entry,
   deleted-while-latched, is dropped), clears the count, then runs the moved
   §6-2/§6-3 asserts on the FINAL reconciled list. Placed after CompactDynamicLive.
7. **RunObjects tail** — reconcile condition is now `Dirty` **OR** latch non-empty
   (latch-non-empty ⇒ dirty by construction, but the OR guards against ever
   leaking a latched object); `bsr CompactDynamicLive` then `bsr DrainDynamicPending`.

## The three beyond-spec decisions (pre-ruled by Volence 2026-07-12) — verified

1. **Pop-rollback on latch-full (edit 3).** The latch-full check is
   POST-pop, so alloc-fail must return the slot to the free stack or leak it.
   Verified airtight by the room proof: latch-full requires >8 saturated allocs
   in one frame; the rollback re-exposes the slot at the current SP. (A
   DEBUG-forced latch-full soak is available on demand; the room proof shows it's
   only reachable with a pathological >8-saturated-alloc-per-frame spawner.)
2. **Latch-side A1 duplicate guard (edits 4 + 6).** DeleteObject zeroes the latch
   entry; the drain null-guards zeroed entries. The §6-3 sweep assert (exactly
   once, no dup/missing) validates the combined invariant every reconcile frame —
   0 assert hits across the ~6800-frame soak.
3. **Room-after-compact — AIRTIGHT via the physical-slot bound** (Volence's
   challenge answered). NOT "compact reclaims ≥ latch" (which does NOT hold with
   latch-side deletes). Correct argument: only `NUM_DYNAMIC` physical slots exist;
   at drain every occupied slot has EXACTLY ONE live entry, in the live list OR
   the latch, never both (edit 4 + the A1 zero make deleted slots leave no live
   entry; once count hits `NUM_DYNAMIC` it stays there with no mid-frame compact,
   so a latched slot can only be re-latched). Hence
   `live_count_after_compact + (non-zero latch) = occupied ≤ NUM_DYNAMIC`, so
   `room ≥ latch entries`. The DEBUG assert is the post-drain `d0==0` sweep
   (count == occupied ≤ NUM_DYNAMIC); it documents + guards the bound.

## Verification (spec §9)

**Gates:** strict 2211/0, clippy clean, core_port 4/4 (twin byte-identical both
shapes). Re-pin wave: pins.rs regenerated (core +0x6A plain / +0x6E debug,
downstream uniform); 2 new RAM pins; engine.inc's 8 gate resume orgs re-derived
(else-arms only → real ROM unchanged; the 16 mixed-build resumes); repin.toml +
core_port/core_negative_probes symbol tables + repin_pins baseline + the
engine-constants guard count (49→50) all updated. `repin --check` clean.

**Churn soak (DEBUG, GameState_ObjectTestChurn, press-only): the assert does NOT
fire.** MDDBG__ErrorHandler = **0 hits over ~6800 frames** — the identical scene
fired the walk-live assert at ~frame 4 pre-A2. Pool healthy (count ~30–40, no
collapse). At a mid-soak CompactDynamicLive breakpoint: `Dynamic_Live_Walking =
0` (no walk live), call stack `RunObjects+62 ← GameState_ObjectTestChurn` (the
frame-end reconcile, NOT AllocDynamic), `Dynamic_Live_Pending_Count = 6` (the
latch engaged — 6 saturated-frame allocs deferred). The mid-walk-compact hazard
is eliminated.

**Profile (plain, 120-frame avg, jitter-check PASSED):**
`CompactDynamicLive` **8.1% → 0.7%** (4 calls/frame → 1 frame-end call);
`DrainDynamicPending` <0.4% (not in top-28); RunObjects 40.5% → 26.9%. The
frame-end reconcile (compact + drain) ≈ ~1% vs the old 8.1%. HONEST CAVEAT: the
emergent churn intensity is slightly lower post-A2 (count ~30 vs ~40; the latch
defers saturated-frame allocs, shifting steady state), so the share drop is not a
perfectly-controlled A/B — but the compact call-count 4→1 is the unambiguous
structural win, and the share drop is consistent with it.

**Spawn-order preservation.** Structural: the drain appends `Pending[0..N)` in
alloc order after compaction (which preserves relative order) → spawn-order
dispatch preserved; latched spawns run next frame. Continuously asserted: the
moved §6-3 sweep (post-drain count == full live sweep, no dup/missing) passed
every reconcile frame across the ~6800-frame soak. **A behaviorally-distinguishable
frame-locked A/B was NOT run** — the churn scene has identical anonymous churners,
so dispatch order isn't observable; the invariant is guaranteed structurally +
asserted every frame. (Gate note: if a labeled-object A/B is wanted, it needs a
new scene with distinguishable parents/children.)

## Same-frame TouchResponse tradeoff (documented, spec §9)

A latched spawn is not in the live list until the frame-end drain, so it misses
same-frame TouchResponse (collision begins next frame) — ONLY for spawns during
saturated frames; normal-append spawns keep today's semantics. Documented in the
AllocDynamic header (both twins).

## Files

- aeon `264037b`: constants.asm, constants.emp, ram.asm, core.emp, core.asm, engine.inc.
- sigil `fa02f91`: pins.rs, repin.toml, test_support.rs, core_port.rs,
  core_negative_probes.rs, repin_pins.rs.
- Scratch verification data: `scratchpad/a2-latch-verify.md`; plan:
  `scratchpad/a2-latch-plan.md`.

## Owed at merge (gate)

- PROVENANCE.md re-baseline (new plain/debug md5s above) + spec §9 status →
  SHIPPED. Coupled aeon+sigil dual-push (both masters together). The churn-profile
  A/B caveat + the labeled-object spawn-order A/B are the only soft spots — both
  are gate calls, not correctness gaps (the invariant is asserted every frame).
