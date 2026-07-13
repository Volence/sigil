# Tranche 13 — load_object.asm — merge-gate packet (2026-07-13)

Branches `port-tranche13` (aeon / sigil), NOT merged — Volence's gate.
Design note: `2026-07-13-tranche13-load-object-design.md`.

## Summary

Ported `aeon/engine/objects/load_object.asm` (107 lines, `Load_Object` +
`Load_ObjectList`) — the spawn-seam callee of entity_window's TrySpawnObject.
The cleanest region since dplc: shape-INVARIANT length, no asserts, no
`__DEBUG__`. Loop ran TWO retrospect passes to dryness (pass 1 = the a3 fix +
redundant-save removal; pass 2 caught a shared-idiom the first pass missed —
see Pass 2 below).

**Final region:** plain `$3FDC..$4074`, debug `$4BA6..$4C3E`, len **`$98`**
both shapes (started `$9E`; step-2 −2 + step-5 −4).

**Verification:** full strict workspace **2213/0** (2211 baseline + 2 new
load_object_port gates), clippy clean, repin --check clean. Byte gate green
both shapes at every step.

## Commits

| Step | aeon | sigil |
|------|------|-------|
| 1 transcribe | `19b1424` | `7f19aa7` |
| 2 modernize | `d5860f6` | `7a3298f` |
| 3-5 audit+opt | `f3e2b31` | `289f8c1` |

## Steps

- **Step 0** — recon + design note. Region bounds, hazard sweep, and the
  ObjDef/offsetof revision (this file burst-copies the template opaquely; the
  ObjDef struct-twin + offsetof workout belong to t14, the objdef data file).
- **Step 1** — byte-exact transcription. Demanded-features: none new (`jsr sym`
  → abs-word via linker width-select already exists; zero-offset field opt
  already mirrors AS). 3 language frictions surfaced → step-3(a) asks (below).
- **Step 2** — modernize: `jsr`→`jbsr` ×2 (size-neutral), `bra.s`→`jbra`, bare
  Bcc; `bne.w .alloc_fail`→bare relaxes to `bne.s` (−2). `.asm` twin hand-set in
  lockstep (`bsr.w` ×2, `bne.s`). Downstream collision_lookup/sound_api −2.
- **Steps 3-5** — see per-pass below. a3-clobber fix (byte-neutral) +
  redundant-a0-save removal (−4). Downstream −4.
- **Step 6** — corpus sweep: NO trigger. t13 built no construct, added no
  generalizable optimization; the clobber-drift pattern was already swept
  corpus-wide in retro-fix-batch-2 (load_object was unported then, now handled
  in-port). Nothing prior files could adopt.

## What each pass added

### Pass 1 (steps 3-5, single dry loop)

**Step-3(a) — language / format asks** (filled interrogation):
- *Ceremony scan*: the 26-byte template burst copy is 3 movem pairs — dense,
  not ceremonious; range comments carry it. → step-4 (unique idiom, no build).
- *Comment-as-compensation*: the overlay-write comment compensates for the
  missing overlay-write syntax → **ASK 1: typed field overlay-write form**
  (`offsetof(Sst,f)(an)` escape shipped; `Sst.f:l(an)` owed). Ledgered.
- *Escape-hatch census*: 1 escape — `offsetof(Sst,prev_anim)(a1)` (the overlay
  write). 0 `extern()` manual drift-locks (uses shared sst/constants twins).
  0 transliteration blocks. → ASK 1.
- *Domain-type scan*: placement word d2 (OEF flips + subtype) could be a
  `PlacementWord` newtype; low value (1 use, bit-manip is local). Not taken.
- **ASK 2** (contract lint, from step 1): `proc.clobber-undeclared` FPs on
  individual-push preservation across a branch (d4); `preserves()` accepts only
  movem pairs. Inexpressible today; 1 residual warning. Ledgered (S2-D6).
- **ASK 3** (contract lint, from step 1): `out()` can't verify a callee-sourced
  output (a1 from AllocDynamic → `out-unwritten`). Matched .asm `clobbers(a1)`.
  Ledgered (S2-D6).

**Step-3(b) — reads-wrong / audits** (filled interrogation):
- *Comment-claim audit*: "preserve d4 — caller reads it" ✓ (entity_window:1179
  relies on it); "alloc failures silently skipped" ✓ (loop ignores Z).
- *Contract audit*: Load_Object clobbers(d0-d3,a1,a2,a3) ✓. **Load_ObjectList
  under-declared a3** (transitively clobbered via Load_Object tail-jsr — the
  retro-audit clobber-drift class) → **FIXED** (added a3, byte-neutral, both
  twins).
- *Name audit*: `.alloc_fail`/`.no_piece_count`/`.loop`/`.done` clear. ✓
- *Magic-number audit*: `rol.w #4`, `#$FF000000`, `#$FF`, movem `8/16(a3)` all
  carry range/intent comments. ✓
- *Cold-reader test*: one spawn traces cleanly from headers + range comments. ✓

**Kill-list / ledger**: no new twin mirrors (uses existing sst/constants twins,
no kill row). 4 ledger rows (ASKs 1-3 + the unique burst-copy idiom).

### Pass 1 — Step-5 (engine optimize, filled interrogation)

**Load_Object** (spawn-path — per object spawn, NOT per-frame-per-object):
- *Invariant ladder*: straight-line, no loop. movem block-copy already optimal. n/a.
- *Counter/cache audit*: no counters/caches/budgets. n/a.
- *Guard-coverage*: `beq .no_piece_count` guards mappings==0 (null-deref) — sole
  path, LOAD-BEARING. named.
- *Hardware cross-check*: no VDP/DMA/hardware (pure RAM setup). n/a.
- *Silent-tradeoff*: none.
- **Verdict: no change** — spawn-path not hot; block-copy optimal; guard load-bearing.

**Load_ObjectList** (level-load / section-stream time; zero callers today):
- *Invariant ladder*: list loop, all per-entry — nothing hoistable. But the
  `move.l a0,-(sp)` / `movea.l (sp)+,a0` around `Load_Object` is **REDUNDANT** —
  Load_Object (and its callee AllocDynamic) provably never touch a0 (verified
  both paths; entity_window already relies on a0-preservation) → **TAKEN:
  removed** (−4, provably behavior-identical; byte gate + static proof suffice —
  no callers exist to oracle-verify anyway).
- *Counter/cache audit*: none. n/a.
- *Guard-coverage*: `beq .done` guards the 0-terminator — sole path, load-bearing. named.
- *Hardware cross-check*: none. n/a.
- *Silent-tradeoff*: "alloc failures silently skipped" is the documented
  contract (Out: none) — commented in the header. named.
- **Verdict: 1 optimization taken** (redundant a0 save/restore removed).

### Pass 2 (second retrospect — the loop-until-dry pass)

Pass 1 asserted dryness on a light check; a proper re-run of the step-3
interrogation on the post-step-5 code surfaced two items the first pass glossed
(both marginal; neither changed shipped bytes):

- **Comment-as-compensation (the miss)**: the mappings→frame-0 piece-count read
  is NOT unique — `animate.emp:276` does the same `move.w FRAME_PIECE_COUNT(base,
  off.w), dest` with a byte-identical "+4 bbox bytes" comment. A duplicated
  what-comment across two files is the exact signal step-3(a) exists to catch;
  pass 1 (reading load_object in isolation) walked past it.
  - **Attempted** (Volence-directed) a `frame_piece_count` `pub comptime fn` in a
    new helper-only module. Interim MIS-DIAGNOSIS (committed then corrected): I
    first reported a "Gap B" that plain procs can't invoke Code helpers — FALSE;
    that was a syntax error on my part (brace splice `{helper()}` in a proc body
    instead of a bare call `helper()`; `TouchResponse` calls
    `touch_test_target(.dyn_next)` bare). **The real, single gap:** an asm-template
    EA can't take a spliced INDEX register — `map_an_indexed` (asm.rs:1345)
    resolves the index only from a literal `Path`, while the base register already
    accepts a `{splice}`. Two mechanical sub-fixes (parser `.w` after a spliced
    index + eval the index-slot splice, base path is the template); byte-neutral.
    Reverted the half-built helper to green step-5. Ledgered as a worth-building
    step-3(a) ask (indexed-addressing helper class; frame_piece_count is the
    first consumer across load_object + animate).
- **Magic-number (hidden coupling)**: `rol.w #4` silently encodes
  `(RF_XFLIP - OEF_XFLIP) & 15`. Drift-safe rewrite needs OEF_XFLIP in the
  constants twin (a ripple to `engine_constant_equs` + guard count across many
  port tests) and reads more cleverly than `#4`+comment → **deferred, ledgered**.

Pass 3 retrospect: empty → **dry**. Net shipped delta from pass 2: none (both
items ledgered); the value is the two language gaps.

## Merge integration checklist (for Volence's gate)

- `--no-ff` merge both sides + push coupled.
- Wire `SIGIL_EMP_LOAD_OBJECT` gate into engine.inc:294 (currently ungated
  unconditional include) — resume org plain `$4074` / debug `$4C3E`.
- Rebuild master s4.bin/s4.debug.bin + re-baseline provenance (engine-block
  content changed; EndOfRom org-$10000-shielded, ROM lengths unchanged).
- Paired-state re-verify: full sigil strict with AEON_DIR → this branch tree.
