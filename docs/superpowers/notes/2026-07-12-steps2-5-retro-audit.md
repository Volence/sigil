# Steps 2–5 retro-audit — every port before the checklists existed

**Opened 2026-07-12 (Volence + Fable).** Trigger: an advisor pass over
`dplc.emp` found that tranche 10's step 5 covered core's RunObjects only —
dplc got no logged interrogation, and a fresh look found two real items
(below). The step-3(a)/3(b)/5 interrogation checklists were only added to
the loop on **2026-07-11** (t11), so **every file ported before sprites.emp
never faced them**. This doc walks steps 2–5 back over the ported corpus
at today's standard.

**This is NOT "t10 was done badly" in general** — the packet record shows
real step-5 work in most tranches (t6 −8 B live-verified; t7 the honest
collision review; t8 a hot-loop win; t9 an explicit not-taken record).
The systemic gaps are:

1. **Cold-file skips in multi-file tranches** — step 5 gravitates to the
   hot file; the cold file gets no line at all (t10: core profiled,
   dplc silent).
2. **Pre-checklist files** — t4–t10 ran step 3/5 as judgment, not
   checklist; t0–t3 ran under the OLD 4-step loop (no step 5 existed).
3. **Post-shipped-construct retrofits** — `assert`/`table`/`let`-reg/
   asm-splice all shipped AFTER most files were ported; comment-claims
   that could now be checked invariants are sitting as prose.

## Method (per file)

Run the ratified checklists from `campaign-port-loop.md` against the file
AS IT STANDS on master (post-consolidation, post-occupancy):

- **Step 2 residue** — idiom sweep (jbra/jbsr, widths, brace style, house
  comments). Expect clean; consolidation already swept most.
- **Step 3(a)** — ceremony scan, comment-as-compensation, escape-hatch
  census, domain-type scan.
- **Step 3(b)** — comment-claim audit, contract audit, name audit,
  magic-number audit, cold-reader test.
- **Step 4** — construct retrofit: would a shipped construct
  (`assert`, `table`, comptime-fn skeleton, `let` reg) collapse a shape
  here? (Corpus-sweep precedent: perform_dplc dedup, sfx_bank table.)
- **Step 5** — invariant ladder, counter/cache audit, guard-coverage
  audit, hardware cross-check, silent-tradeoff comments. Per hot proc.
  Oracle probes where static reading can't settle it (press-only soaks —
  see ledger row 990 for the step_out/resume/press wedge).

**Finding buckets:** `[PROC]` process gap (no logged line — record only),
`[OPT]` missed optimization, `[HAZ]` latent engine hazard, `[RETRO]`
shipped-construct retrofit, `[CERT]` audited clean. Fixes ship as
retro-fix batches (precedent: 2026-07-11-retro-fixes-CA1-RA1), never
inline in this doc's commits.

## Corpus status table

| File | Tranche | Packet step-5 evidence for THIS file | Audit |
|---|---|---|---|
| engine/objects/dplc.emp | t10 | none (core only) | **DONE 2026-07-12** — 3 findings below |
| engine/objects/core.emp | t10 | RunObjects profiled; 2 not-taken recorded | pending (occupancy + diagnostics both reworked it since — re-audit vs today's file) |
| engine/objects/sprites.emp | t11 | full checklist + Fable second look (A1 etc.) | **exempt** — set the standard |
| engine/objects/animate.emp | t9 | "no opt taken, reasons recorded" | pending (pre-checklist not-taken = anchoring risk; hot path) |
| engine/objects/rings.emp | t8 | RingCollision win, live-verified + not-taken record | pending (strongest pre-checklist record; also reworked by S3K ring art + colours since) |
| engine/objects/collision.emp | t7 | step-5 review (Volence's AABB, honest review) | pending |
| engine/objects/aabb.emp | t7 | same tranche | pending (asm-splice lead_move touched it since) |
| engine/system/game_loop.emp | t5 | one yield (SR-mask hazard comment) | pending |
| games/sonic4 object bank (test_particle/test_solid, act_descriptor) | t6 | −8 B live-verified | pending |
| data: particle_anims / sonic_anims | t4 | step-5 queue (pads) — executed? verify | pending (data files — 3(a)/3(b) focus) |
| engine/level/collision_lookup.emp | t3 (OLD loop) | n/a — step 5 didn't exist | pending |
| engine/system/vdp_init.emp | t3 (OLD loop) | n/a | pending |
| engine/system/controllers.emp | t2 (OLD loop; note says "complete through step 2") | n/a | pending — confirm steps 3+ ever ran |
| engine/system/math.emp | t2 (OLD loop) | n/a | pending |
| engine/sound/sound_api.emp, engine/system/hblank.emp | pre-loop (pin exact provenance during audit) | n/a | pending |
| data/sound: mt_bank, sfx_bank, dac_samples | pre-campaign (M1.D/Plan-6/7 era); sfx_bank got the `table` retrofit | n/a | pending — low priority, data-shaped |
| support twins: sst.emp, constants.emp, types.emp | grown across tranches | n/a | audit as cross-cutting pass at the end |

## Findings

### dplc.emp (t10) — audited 2026-07-12, advisor pass

1. **[PROC]** t10 step 5 never touched dplc; no logged skip. (Norm
   post-2026-07-11: per-file line even when "nothing taken".)
2. **[RETRO] Single-entry invariant is prose, could be a checked
   invariant.** Header claims "contiguous art layout → exactly 1 DPLC
   entry per frame — guaranteed single DMA". Nothing checks it. With the
   diagnostics construct shipped: DEBUG `assert` that entry count == 1
   (d4 == 0 after the `subq`) — byte-neutral in release, converts the
   claim comment into a checked invariant. Cheap; candidate for the next
   retro-fix batch. (If the invariant is REAL corpus-wide, the entry
   loop + 5-reg movem around the queue call is dead generality — a
   follow-on [OPT] specialization, gated on Volence confirming the
   build-tool guarantee.)
3. **[HAZ] `prev_frame` committed before the DMA enqueue; the queue can
   silently drop.** `QueueDMATransfer.full` (dma_queue.asm) bumps a
   DEBUG counter and rts — no carry, no retry. dplc writes
   `Sst.prev_frame` BEFORE enqueueing, so on a full queue the object
   believes its art loaded and shows the previous frame's tiles until
   `mapping_frame` changes again — for rarely-changing frames (monitor
   break, signpost settle) that's wrong art indefinitely. The
   "Deferrable — can slip one frame" doc line is inaccurate: there is no
   slip, only permanent drop for that frame value. Sibling: the 128 KB
   split path with one slot left queues the first half and silently
   drops the second (partial stale art). Fix candidates (engine work,
   Volence's call): commit `prev_frame` only after successful enqueue,
   or return carry from QueueDMA (RingBuffer_Add precedent) and skip the
   commit on failure. Contract change also touches bg_anim.asm.
   Predates the port — inherited, not introduced.

## Proposed audit order

1. **animate.emp** (t9) — hot path, pre-checklist "not taken" verdict is
   exactly the anchoring class the checklist was written for.
2. **core.emp** (t10) — hot path; heavily reworked since port
   (occupancy, diagnostics) so the packet record is stale anyway.
3. **collision.emp + aabb.emp** (t7) — hot path; splice touched aabb.
4. **rings.emp** (t8) — hot path but strongest existing record; also
   re-touched twice since.
5. **game_loop.emp** (t5), object bank (t6).
6. OLD-loop files (t2/t3 + pre-loop): controllers, math,
   collision_lookup, vdp_init, hblank, sound_api — never had ANY step 5.
7. Data files (t4 anims; sound banks) — 3(a)/3(b) focus.
8. Cross-cutting: sst/constants/types twins.

One file per sitting, findings appended here, fixes batched. Hot-path
files get the oracle where static reading can't settle a counter/guard
question (press-only driving).
