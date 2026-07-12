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
- [ ] **2. Walker: RunObjects .run_culled** → live-list loop. Live-verify.
- [ ] **3. Walker: RunObjects_Frozen** dynamic segment → live list. Live-verify.
- [ ] **4. Walker: TouchResponse** dynamic inner walk → live list + fixed
      system+effect sweep. Live-verify.
- [ ] **5. Walker: EntityWindow_DespawnObjects** → live list (.asm-only). Live-verify.
- [ ] **6. Compaction** at RunObjects tail (if dirty). Live-verify.
- [ ] **7. DEBUG asserts** (§6, three one-liners via `assert`): count ≤ NUM_DYNAMIC;
      every entry in-pool; post-compact count == full tst.w sweep live count.
- [ ] **8. Profile packet** vs t10 11,841-cycle pin (needs oracle profiler fix).

Each behavior-affecting step: run-order trace + spawn/despawn soak, frame-locked
comparison (A1 precedent). Loop invariant: list may over-approximate, never
under-approximate; `code_addr` decides; compaction only runs when no walk is live.
