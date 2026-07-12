# Object-pool occupancy (the "run objects list") — design

2026-07-11 · Fable · status: DRAFT awaiting Volence gate (engine-arch,
behavior-affecting — the tranche-9 PerFrame-deletion class).
Promotes the gap-ledger DESIGN-READY entry (tranche-7 close) to a full
spec, grounded in the tranche-10 profile and a fresh full-corpus site map.
Supersedes the ledger row's sketch; the row gets a pointer here at the
next ledger-touching wave (not edited now — tranche 11 has the file hot).

## 1. Problem — the empty-slot tax, with numbers

The object pool is 66 fixed slots × $50 bytes (2 player + 40 dynamic + 8
system + 16 effect), and emptiness is encoded ONLY as `code_addr == 0`
(sst.emp:20 — no status bit, no list). Every per-frame walker therefore
visits ALL slots and tests each:

| Walker | Slots visited/frame | Site |
|---|---|---|
| RunObjects .run_culled (dynamic) | 40 | core.emp:191-261 |
| RunObjects .run_always ×3 (player/system/effect) | 26 | core.emp:186-224 |
| RunObjects_Frozen (paused) | 66 | core.emp:346-357 |
| TouchResponse (per player ×2) | 128 (64×2) | collision.emp:41-125 |
| EntityWindow_DespawnObjects | 40 | entity_window.asm:1324-1371 |

Tranche-10 oracle profile (live, 3 objects): **RunObjects alone = 11,841
cycles = 9.3% of the 128k NTSC frame**, dominated by the fixed 66-slot
iteration (~63 empty). Ledger estimate across all four sites: ~7-8k
cycles/frame of pure empty-slot tax at typical occupancy. Rings escaped
this pattern at birth (dense count-tracked buffer, rings.emp) — that's
the model.

## 2. The structure

**One live-list per DYNAMIC pool only** (rationale §5): a word-ADDRESS
array + count, following the engine's existing idiom (the free stacks and
Sprite_Bands already store word slot-addresses — `movea.w (a1)+, a0` is
the established 8-cycle "next slot" step; byte indices would re-buy a ×$50
multiply per visit):

```
Dynamic_Live:       ds.w NUM_DYNAMIC   ; word addresses of live slots, in SPAWN ORDER
Dynamic_Live_Count: ds.w 1             ; live entries
Dynamic_Live_Dirty: ds.b 1             ; a deletion happened; compact at frame end
```

RAM cost: 83 bytes. `code_addr == 0` REMAINS the single source of truth;
the live list is a conservative over-approximation (may briefly contain
dead/zeroed entries, never misses a live one) **in which each live slot
appears EXACTLY ONCE** (uniqueness clause — amendment A1, 2026-07-12; see
§6). Walkers null-guard the ENTRY (load to dN, `beq` skip, then `movea`
— the sprites band-walk pattern) and keep the `tst.w` slot guard; the
pair is what makes every mutation case safe (§6).

### Maintenance (rare events, not per-frame)
- **AllocDynamic** (core.emp:77): append the popped slot's address —
  `move.w a1, offset(Dynamic_Live + 2×count)`, `addq.w #1, count`. O(1)
  in the common case. **Amendment A1: append is capacity-guarded** — at
  `count == NUM_DYNAMIC`, run the compaction pass inline before
  appending (room is guaranteed: the free stack being nonempty at full
  count implies zeroed entries exist). Rare, O(NUM_DYNAMIC); Count can
  never exceed capacity by design.
- **DeleteObject** dynamic path (core.emp:126): existing pool-detection
  already branches per pool; the dynamic arm sets `Dynamic_Live_Dirty`
  AND — **amendment A1 — scans `Dynamic_Live[0..count)` for the slot's
  address and ZEROES that entry in place** (deletes are rare; a ≤40-word
  scan is trivial; zeroing moves nothing, so a live walker's cursor
  stays valid). The slot clear (`code_addr = 0`) still kills the object;
  the entry-zero is what keeps the list DUPLICATE-FREE when the LIFO
  free stack recycles the slot later the same frame (§6). No
  back-pointer field in the SST.
- **Compaction**: at RunObjects tail (after all walks), if dirty: one pass
  copying entries whose `code_addr != 0` down over dead ones, recount,
  clear dirty. O(live), runs only on frames with a deletion.
- **InitObjectRAM** (core.emp:34): zero count + dirty with the pool clear.

## 3. The ONE semantic decision — dispatch order (Volence's call)

Today objects run in SLOT order (ascending RAM). The free stack pops
high-slot-first (InitObjectRAM primes 0..39 so slot 39 allocates first),
so today's order is already an allocation-history artifact, not a designed
contract. The live list makes order explicit. Two options:

**(a) SPAWN order (append + compact — RECOMMENDED).** Iteration order =
allocation order, stable under deletion (compaction preserves relative
order). Deterministic, and it yields the genuinely useful invariant
**parents always run before their children** (children are allocated
after). O(1) mutations. Delta from today: order changes once at cutover;
verified live (§7).

**(b) SLOT order (insertion-sorted list).** Preserves today's exact order;
mutations become O(n) shifts (still cheap — spawn/despawn are rare). Take
this only if some behavior is discovered to depend on slot order during
§7 verification.

New spawns during the frame: walkers snapshot the count at loop entry, so
a child spawned mid-walk first runs NEXT frame — deterministic, replacing
today's it-depends-which-slot-was-free behavior (research confirmed
same-frame child dispatch is already allocation-position-dependent, so no
stable behavior is lost).

## 4. Consumers — the four retrofits

**(a) RunObjects .run_culled** (core.emp:191-261) — becomes:

```
    lea     Dynamic_Live, a2
    move.w  Dynamic_Live_Count, d7
    jbeq    .culled_done
    subq.w  #1, d7
.culled_loop:
    move.w  (a2)+, d0               // entry null-guard (A1): zeroed by a
    jbeq    .culled_next            // same-frame delete — skip
    movea.w d0, a0
    tst.w   (a0)                    // truth guard: dead-but-uncompacted
    jbeq    .culled_next
    // unchanged: X/Y cull vs CULL_DISTANCE, then bank dispatch
    ...
.culled_next:
    dbf     d7, .culled_loop
```

(Sketch amended per the Step-2 build: the list cursor a2 must be saved
across the `jsr` — object code preserves only a0/d7 — and d7 snapshots
the count per §3. The committed loop is the reference, not this sketch.)
Empty slots cost ZERO; dead-uncompacted or zeroed entries cost one
load+branch until the frame-end compact. The `moveq #OBJ_CODE_BANK; swap` prefix-rebuild
(t10's declined micro-hoist) now only runs per LIVE object — the hoist
question dissolves.

**(b) RunObjects_Frozen** (core.emp:346-357): player+system+effect keep
their small fixed sweeps (26 slots); the dynamic segment walks the live
list. Paused-frame cost drops the same way.

**(c) TouchResponse** (collision.emp:41-125): the 64-slot inner walk
becomes live-list (dynamic) + the two small fixed sweeps (system+effect,
24 slots), per player. The per-slot `collision_resp != 0` gate stays as-is
(it's dynamic state, cheap, and filtering it into a second registered
list is future work — this same structure IS the participants list that
object-vs-object collision will filter, per the ledger's one-build-two-
features intent).

**(d) EntityWindow_DespawnObjects** (entity_window.asm:1324-1371): walks
the live list instead of 40 slots; its DeleteObject calls just set the
dirty flag — the walk itself stays valid because entries don't move until
compaction. (This file is unported .asm — single-side edit, no twin tax.)

Non-consumers, confirmed: BuildSprites (already walks per-band lists of
live SSTs — the existing proof this pattern works in this engine), rings
(own counted buffer), the entity-window bitmask loops (window state, not
pool walks).

## 5. Scope rationale — dynamic pool only
- Player (2) and system (8): fixed sweeps are already near-free; a list
  adds maintenance for nothing. EXEMPT.
- Effect (16): highest churn (CreateEffect_Simple spawns N per call,
  AF_DELETE kills per-frame); ledger's call stands — the sweep is 16
  slots and the maintenance would run constantly. EXEMPT (revisit only if
  a profile shows the effect sweep mattering).
- Dynamic (40): the tax lives here (40 of 66 slots, typically ~3 live),
  and three of the four walkers sweep exactly this pool. IN SCOPE.

## 6. Hazard analysis (from the full mutation-site map)
Every allocation/deletion site was enumerated (load_object.asm:29,
children.asm:50/123/199/294/396/458, animate.emp:169 AF_DELETE,
children.asm:367 DeleteChildren, entity_window.asm:1365):
- **Self-deletion during dispatch** (AF_DELETE): entry stays in the list,
  `code_addr=0`, truth guard skips it; compaction collects it at frame
  end. Safe.
- **DeleteChildren during dispatch**: kills OTHER slots mid-walk — same:
  entries persist, guards skip, compact later. Safe. (This is why
  deferred compaction beats in-place swap-remove: swap-remove would move
  an unvisited entry into a visited position mid-walk — a skipped object.)
- **Child/effect spawn during dispatch**: append past the snapshotted
  count; runs next frame. Safe, deterministic (§3).
- **DespawnObjects deletions**: before RunObjects, same deferred rule;
  its own walk tolerates the dead entries it just made. Safe.
- **Freeze**: separate path, same lists, no interaction. Safe.
**Amendment A1 (2026-07-12, caught at the Step-2 build checkpoint) —
the same-frame delete+realloc duplicate.** The original design relied on
`code_addr == 0` alone to mark dead entries. That breaks in one common
sequence: DeleteObject frees slot N (entry stays, "dead by code_addr");
LATER THE SAME FRAME an allocation occurs, and the LIFO free stack hands
back slot N preferentially; AllocDynamic appends N again. N is now
listed TWICE with a live code_addr — the old entry is no longer
skippable, and a code_addr-based compaction KEEPS BOTH: the slot
dispatches twice per frame, permanently. This is not exotic — it is the
entity window's STEADY STATE while scrolling (despawn one edge + spawn
the other, same frame, LIFO recycling the just-freed slot). The pinned
stress scene (pool full, no realloc) structurally cannot produce it.
Fix: DeleteObject zeroes its entry (uniqueness by construction),
walkers null-guard entries, AllocDynamic compacts-on-full; compaction
drops zero entries and dead-code_addr entries alike. VERIFICATION RIDER:
every walker soak from Step 2 onward must include a forced
delete → same-frame-realloc cycle, confirming exactly one entry for the
recycled slot.

The invariant, stated once: **the list may over-approximate, never
under-approximate; each live slot appears exactly once; `code_addr` and
the entry-zero decide; compaction only runs when no walk is live.** DEBUG builds enforce it with the new `assert` construct
(count ≤ NUM_DYNAMIC; every entry in-pool; post-compact count == a full
tst.w sweep's live count — three one-liners once the diagnostics
construct ships; sequence them AFTER it).

## 7. Expected win + verification plan
- Ceiling: the empty-slot tax only (~7-8k cycles/frame, ~6% NTSC at
  typical occupancy; per-live work is real work and stays). RunObjects'
  9.3% should drop to roughly a third of itself in the 3-object scene.
- **This is behavior-affecting (order, timing) — full step-5 live
  treatment**: oracle profiler before/after on the same scene
  (emulator_get_profiler; compare against t10's pinned 11,841); object
  run-order trace via emulator_object_list across spawn/despawn-heavy
  play; pause/unpause soak; despawn-band scroll test. Bytes change on
  both sides of two ported twins (core, collision) → lockstep .asm edits
  + re-pin + gates, PLUS the live verification. entity_window/children/
  load_object are .asm-only edits.
- Rollback story: the structure is additive; the old fixed sweeps remain
  trivially restorable per-walker (each retrofit is an independent
  commit).

## 8. Sequencing
Engine-arch project, its own branch off both masters after tranche 11
merges (it touches core/collision, which t11's sprites work neighbors).
Recommended order: diagnostics construct first (its asserts are this
design's DEBUG rail), then occupancy. Build order within: structure +
maintenance (behavior-neutral, lists maintained but unread — verifiable
alone) → walker retrofits one at a time, live-verified each → compaction
→ DEBUG asserts → profile packet to Volence.
