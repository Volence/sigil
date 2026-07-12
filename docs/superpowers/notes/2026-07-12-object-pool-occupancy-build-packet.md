# Object-pool occupancy — build packet

Branch: `object-pool-occupancy` (both sigil + aeon, off post-diagnostics masters
sigil 764002b / aeon 573f7c1). Spec: sigil
`docs/superpowers/specs/2026-07-11-object-pool-occupancy-design.md`. §3 ruled by
Volence: **spawn-order dispatch** (append at spawn, deferred frame-end compaction
preserving relative order; slot-order is the documented fallback only if live
verification surfaces a real order dependency).

## Decisions taken at recon (surfaced to Volence)

1. **C-B1 free-stack comment is ALREADY FIXED.** Volence corrected it himself in
   aeon `b8df29f` (2026-07-12, the retro-fix batch — the master this branch is
   off). core.emp:43-46 / core.asm:19-22 now read correctly ("the LAST-pushed
   (highest) slot is allocated first; the pool fills downward toward slot 2"),
   matching the code (pushes slot 2→41, LIFO pops 41 first). No edit made — the
   instructed fix is stale; re-editing a correct comment would only risk breaking
   it. **Surfaced, not actioned.**

2. **RAM placement = tail, not beside the free stacks.** `Dynamic_Live` block
   goes after `Sound_Dbg_Mirror`, before `Engine_RAM_End` (both shape-independent,
   the RAM tail). This ripples ZERO existing RAM addresses, so the layout change
   leaves every ported module's byte gate untouched — only the code edits
   (core/collision/entity_window) change bytes. Placing it beside the free stacks
   (semantically nicer) would shift every RAM symbol from Sprite_Bands down,
   forcing a corpus-wide re-pin of unrelated modules for zero runtime benefit
   (Genesis RAM has no locality cost). A site comment explains the placement.

3. **Oracle profiler caching bug FIXED 2026-07-12** (oracle `linux-port`, pending
   merge; gap-ledger:978). Numeric verification (§7, vs the t10 11,841-cycle pin)
   needs it. When reached, confirm the RUNNING oracle has it via a same-ROM jitter
   check (two profiler reads of one ROM must differ frame-to-frame); if the loaded
   oracle still serves stale data, the numeric packet becomes a pending follow-up
   rather than trusting the numbers.

## Facts

- NUM_PLAYERS 2 · NUM_DYNAMIC 40 · NUM_SYSTEM 8 · NUM_EFFECTS 16 · total 66.
  Live list = 40 words + count word + dirty byte + pad = **84 bytes** RAM.
- Twins: core.emp↔core.asm, collision.emp↔collision.asm (lockstep + re-pin on
  byte change). entity_window.asm / children.asm / load_object.asm are UNPORTED
  (single-side .asm, no twin).
- children.asm (×4 alloc, ×1 delete) + load_object.asm (×1 alloc) all route
  through `jsr AllocDynamic` / `jsr DeleteObject` → inherit maintenance for free,
  **no edits**. Only entity_window.asm has its own walk loop → retrofit.
- Reference-ROM refresh: `DEBUG=1 ./build.sh && cp s4.bin s4.debug.bin && ./build.sh`.
- Port gates: `SIGIL_STRICT_GATE=1 cargo test -p sigil-cli --test core_port --test collision_port`.
  Re-pin: `cargo run -p sigil-harness --bin repin` (updates region len + cross-seam
  label VMAs incl. the 3 new Dynamic_Live* labels once added to the Shape.labels).

## Verification harness — determinism method (learned Step 1)

The soak scene is `GameState_ObjectTest` (reached by the TEMP Game_Entry flip):
40 dynamic + 16 effect slots, emitter/child churn. Two findings that shape ALL
live verification:
- **`reset` + `press N` is NOT frame-deterministic**: boot length varies a few
  frames, and `press N` frames ≠ N `Frame_Counter` ticks (vblank.asm increments
  `Frame_Counter` at TWO sites, 99 + 153). So the same reset+press gives
  different absolute object positions run-to-run.
- **Per-tick logic IS deterministic in `Frame_Counter`** ($FF8002): e.g. TestEnemy
  slot32 x = `Frame_Counter − 186` exactly across runs. So the frame-lock anchor
  is a target `Frame_Counter`, NOT a press count. To A/B two ROMs: reach the SAME
  `Frame_Counter` on each (press to approach, then single-step frames to land),
  then compare object state / SAT / framebuffer. Always reload FRESH symbols
  (`s4.lst`) after every rebuild — stale symbols mis-resolve (`Lag_Frame_Count`
  → wrong addr).

## Build order (spec §8) — status

- [x] **0. Baseline** — ObjectTest stress scene identified; baseline ROM stashed
      (`s4.baseline-objtest.bin`, md5 29d3dcf4). Determinism method established.
- [x] **1. Structure + maintenance** (behavior-NEUTRAL) — DONE + VERIFIED.
      RAM block at tail (zero-ripple); InitObjectRAM zero count+dirty; AllocDynamic
      long-preserved-a0 append; DeleteObject dynamic arm `st` dirty. Both twins
      lockstep, byte-identical BOTH shapes. Core region grew +0x22 both shapes.
      Cascade re-pin: repin.toml +3 symbols; engine.inc 7 gate resume-orgs +0x22;
      mixed_dac_rom tranche5 game_loop disp $3A32→$3A54 / $4E60→$4E82;
      repin_pins hand-typed baselines (ANIMATE/RINGS/CORE/SOUND_API/DELETE_OBJECT).
      **Strict 2208/0, clippy clean, all port gates byte-identical both shapes.**
      LIVE: `Dynamic_Live_Count`=40 (matches occupancy), `Dynamic_Live[]` = exact
      spawn-order slot addresses (perfect descending 96BE→8A8E = slots 41→2, each
      −0x50), `Dynamic_Live_Dirty`=0 (effect churn correctly untouched). List
      maintained but unread → behavior neutral by construction (walkers byte-identical).
      POSITIVE-DIRTY check (forced parent-17 life_timer→1, self-destructed via
      DeleteChildren+DeleteObject): `Dynamic_Live_Dirty` 0x00→0xFF (the dynamic
      `st` executed), slot 17 code_addr cleared, `Count` stayed 40 — confirming the
      deferred-compaction over-approximation (delete flags dirty, leaves the list
      untouched; compaction reconciles in Step 6). Maintenance matrix fully green:
      append + negative-dirty + positive-dirty.
- [x] **2. Walker: RunObjects .run_culled → live-list loop** — DONE + VERIFIED.
      New loop: `lea Dynamic_Live,a2 / move.w Count,d7 / beq .culled_done /
      subq #1,d7 / .culled_loop: movea.w (a2)+,a0 / tst.w (a0) guard / …cull… /
      dispatch (a2 saved across jsr — object code may clobber it; only a0/d7 are
      preserved) / dbf`. Caller drops the dead `lea Dynamic_Slots/move.w #NUM-1`.
      d7 snapshots the count (mid-walk child spawn runs next frame). Twins lockstep
      byte-identical both shapes; the growth pushed the plain-shape
      `bne RunObjects_Frozen` past .s → twin now unconditional `bne.w` (bare-Bcc
      relaxes, twin follows). Core +0x8 plain / +0x6 debug; full re-pin cascade
      (engine.inc orgs, mixed tranche5 disp $3A54→$3A5C / $4E82→$4E88, repin_pins
      ANIMATE/RINGS/SOUND_API/CORE-len). **Strict 2208/0, clippy clean, core_port
      byte-identical both shapes.**
      LIVE (frame-locked, FC-anchored): RUN-ORDER FLIP proven — first dynamic
      dispatch a0 = slot 2 (BEFORE, slot sweep) → slot 41 (AFTER: `movea.w (a2)+`
      loads Dynamic_Live[0]=0x96BE, cursor→0xAFF2). Dispatch order = list forward
      = slots 41→2 (spawn order). PARENTS-BEFORE-CHILDREN: parents (slots 17-19)
      at list indices 22-24, children (slots 2-8) at 33-39 → every child dispatches
      after all parents. SOUNDNESS: scene renders + runs correctly; forced a
      dynamic delete (parent self-destruct → 4 dead entries) + 150-frame soak —
      no crash, dead-but-uncompacted entries safely skipped by the tst.w guard
      (§6 hazard), Count held 40 (deferred compaction, no overflow).
      CAVEAT (pre-compaction transient): between Steps 2-5 the walker reads the
      list but nothing compacts, so a DELETE+REALLOC cycle grows Count unbounded
      (past NUM_DYNAMIC). Not triggered in ObjectTest (pool fills to 40, then
      AllocDynamic returns full — no realloc). Step 6 compaction resolves it;
      Step 7 asserts catch count ≤ NUM_DYNAMIC. Verify each interim walker in a
      non-realloc scene.
- [ ] **3. Walker: RunObjects_Frozen** dynamic segment → live list. Live-verify.
- [ ] **4. Walker: TouchResponse** dynamic inner walk → live list + fixed
      system+effect sweep. Live-verify.
- [ ] **5. Walker: EntityWindow_DespawnObjects** → live list (.asm-only). Live-verify.
- [x] **A1. Same-frame delete+realloc duplicate fix** (spec amendment 05ae564,
      caught at the Step-2 checkpoint) — DONE + VERIFIED. Touches Steps 1+2:
      · DeleteObject dynamic arm: after the free-stack push, scan Dynamic_Live[0..
        Count) for the slot's low word and `clr.w` that entry in place (d1 saved;
        ≤40-word scan, delete-rare; zeroing moves nothing → cursor-safe). Keeps dirty.
      · .run_culled walker: null-guard the entry — `move.w (a2)+,d0 / beq skip /
        movea.w d0,a0 / tst.w (a0)` (don't dereference a zero entry).
      · AllocDynamic: capacity-guard — at Count==NUM_DYNAMIC, `jbsr
        CompactDynamicLive` first (movem-save d1/a0-a2 around it; room guaranteed
        since the free stack was nonempty). Count ≤ capacity by construction.
      · NEW proc CompactDynamicLive: keep nonzero + live-code_addr entries in place,
        drop zeroed + dead ones, recount, `sf` dirty. Runs only when no walk is live.
      Twins lockstep byte-identical both shapes; core +0x6A both; full re-pin cascade
      (engine.inc orgs, mixed tranche5 $3A5C→$3AC6 / $4E88→$4EF2, repin_pins
      CORE-len/ANIMATE/RINGS/SOUND_API/DELETE_OBJECT). **Strict 2208/0, clippy clean.**
      LIVE (OJZScroll entity-window — the exact scrolling despawn+respawn scene the
      amendment names; ObjectTest structurally can't produce it): scrolled + oscillated
      across section boundaries (continuous LIFO delete+realloc). Dynamic_Live held
      DUPLICATE-FREE throughout — e.g. `0000 0000 0000 0000 0000 95CE 96BE 0000`
      (Count 8): 6 ZEROED entries (A1 DeleteObject zeroing) + slots 38 & 41 each
      EXACTLY ONCE despite repeated recycling. Walker ran clean with the zero entries
      present (null-guard skips them — no crash from dereferencing 0). Count stayed
      ≤ NUM_DYNAMIC. OWED: CompactDynamicLive's own execution — neither available
      scene triggers AllocDynamic-at-full (OJZScroll peaks ~8; ObjectTest fills to 40
      but never reallocs). Its MAIN path is Step-6's frame-end wiring (runs every
      dirty frame), so verify the compact loop THERE (watch Count reconcile + zeros
      drain per frame); the compact-on-full guard is a rare safety valve, byte-verified.
- [ ] **6. Compaction** at RunObjects tail (if dirty) — CompactDynamicLive built in
      A1; Step 6 wires the frame-end call + live-verifies the compact loop. Live-verify.
- [ ] **7. DEBUG asserts** (§6, three one-liners via `assert`): count ≤ NUM_DYNAMIC;
      every entry in-pool; post-compact count == full tst.w sweep live count.
- [ ] **8. Profile packet** vs t10 11,841-cycle pin (needs oracle profiler fix).

Each behavior-affecting step: run-order trace + spawn/despawn soak, frame-locked
comparison (A1 precedent). Loop invariant: list may over-approximate, never
under-approximate; `code_addr` decides; compaction only runs when no walk is live.
