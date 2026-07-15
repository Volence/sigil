# SST layout assessment — field order, the reorder question, and the usability queue

**Date:** 2026-07-15 (rides the t14 merge — the ObjDef ensure-chain is what makes
this assessment worth recording now).
**Status:** ASSESSMENT + STANDING RULING. No code change. Fable-reviewed with
Volence at the t14 merge gate, prompted by "are we happy with the SST order?"

## Verdict: the layout is right — keep it

The half-remembered optimization ("position right after the object pointer")
is REAL and ALREADY IN the layout: `x_pos` ($02) / `y_pos` ($06) sit
immediately after `code_addr` ($00), so `RunObjects.run_culled`
(core.emp `.culled_loop`) touches ONLY the first 8 bytes of each live
object — `tst.w (a0)` truth guard, X cull read, Y cull read, dispatch read.
That adjacency shipped with the occupancy work and is deliberate structure.

## The three ways field order matters on the 68000 (all satisfied)

Plain `d16(An)` field access costs the SAME at every offset — $02 and $4E
are identical. Order matters in exactly three ways:

1. **Offset 0 is the only cheap seat.** `(a0)` saves 4 cycles over
   `d16(a0)`. `code_addr` — the most-read field (executor dispatch, truth
   guards, occupancy) — correctly owns it. No cycle win exists from
   shuffling the middle fields.
2. **Pairs read as one wider access.** `x_pos`/`y_pos` are inherently-paired
   16.16 longs (`add.l SST_x_pos` in children.asm); `x_vel`:`y_vel` are
   adjacent if a `move.l` pair read is ever wanted; the
   `prev_anim..mapping_frame` run is what enables load_object's
   `#$FF000000` one-instruction runtime init.
3. **The template block ($0A-$1F) must be contiguous** for the spawn burst
   copy — machine-checked since t14 by the ObjDef↔Sst ensure-chain
   (sst.emp), break-verified (SHIFT mutation → all 14 field errors).

## Standing ruling: reorder freedom is BOUGHT — do not SPEND it

Reordering SST fields changes emitted displacement bytes in every object
file on both sides: a full-ROM byte change, a total re-pin wave, and a
provenance re-baseline — for zero measured cycles (see the three ways
above; there is no fourth). So:

- **Do not reorder without a profiler-measured win.** "Cleaner grouping"
  is not a reason; the guard structure documents the grouping.
- **If a measured win ever appears, reordering is now SAFE**: the sst.emp
  twin's extern drift guards, structs.asm's own template-start/size
  errors, the ObjDef ensure-chain, and the per-region byte gates mean any
  missed consumer fails loudly at build or link, never silently. t14's
  payoff is exactly this freedom. Reorder-day checklist: structs.asm +
  sst.emp lockstep, ObjDef field order in lockstep (the chain enforces
  it), full re-pin (`cargo run -p sigil-harness --bin repin`), provenance
  re-baseline, live-verify one spawn/cull/draw frame in oracle.

## The "make SST nicer" queue (all have homes; ranked by payoff)

1. **Overlay-write syntax** `Sst.prev_anim:l(a1)` — gap-ledger row 1023.
   Kills the `offsetof()` escape hatch at the one deliberate multi-field
   write (load_object's runtime init).
2. **`offsetof` in ABSOLUTE-EA position** — gap-ledger row 1005. The big
   usability item: unblocks the EntityScanState struct-twin AND lets
   static slots read as `(Player_1+offsetof(Sst,x_pos)).w` with the field
   NAMED instead of a magic sum. Natural t15 (section.asm) driver — the
   demand lives in entity/section code.
3. **Newtype enrichment** — CollisionType, RenderFlags, RomPtr (t14 jots,
   [[emp-sonic-newtype-candidates]]). Related thought recorded here:
   `render_flags`/`status` are u8s carrying seven-line bit comments; the
   language HAS bitfield types (Plan 3) — a named-bits type is a future
   candidate, gated by adoption-over-cleverness (asm-level code
   manipulates raw bits regardless; the win would be comptime-layer
   construction/documentation, not instruction-level).
4. **`SstPtr16` newtype** for `parent_ptr`/`sibling_ptr` (bare u16 RAM
   addresses with a `movea.w` convention) — minor, jot-grade.

Non-items, checked and dismissed: `movem`-reading the cull header (wash at
32 cycles either way, burns registers); width:height packed word read (aabb
consumes them separately); any hot-path displacement-size concern (d16(An)
is flat-cost).
