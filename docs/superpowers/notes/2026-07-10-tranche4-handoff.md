# Tranche 4 handoff — data quick-wins (written 2026-07-10, post tranche-3 merge)

For the next session. Tranche 3 is MERGED AND PUSHED on both sides (sigil master `130ad40`,
aeon master `d0b8062`); post-merge validation: sigil strict **1895/0 workspace + 215/0
reference-gated**, clippy clean; aeon pins verified (plain `8ce6dd7e…`, debug `13c7b063…`).
The `pitcher_plant` rename (ex-`plantbadmaps`) landed on aeon master with pins re-verified —
per-sprite bundle dirs are now named for the ENTITY with generic member names.

## The loop is now FIVE steps (Volence-ratified at the tranche-3 packet review)

1 transcribe → 2 modernize (standing checklist) → 3 retrospect → 4 implement →
**Volence checkpoint/merge** → **5 optimize** (post-merge, byte-changing, each commit
re-gated against a REBUILT reference; later retrospects can send step-5 work back to
already-ported files). Full loop text + the standing step-2 checklist:
`docs/superpowers/notes/campaign-gap-ledger.md` (top).

## FIRST: step 5 for tranche 3 (the reads-wrong list)

Each its own commit; each changes bytes → rebuild aeon, re-baseline the PROVENANCE pins,
re-run the eight-module mixed gates. From `notes/2026-07-09-port3-tranche-complete.md`:

1. **Collision_GetType: delete the stack push of the world column** (shift Y in place in
   d1; drops `move.w d1,d2` + push/pop + `.cgt_air_pop`; clobbers shrink to d0/d1) — pairs
   with the `jbsr`→`jbra` tail call.
2. **vdp_init: `clr.l VDP_Dirty_Mask`** ×2 (RAM operand — the I/O clr hazard doesn't apply).
3. *(marginal, skip unless VBlank headroom matters)* Flush early-exit shift-out loop;
   controllers P1/P2 pointer-loop dedup.

NOTE: these live in `.emp` files whose bytes feed the MIXED gates, and the AS twins
(`collision_lookup.asm`/`vdp_init.asm`) must change in lockstep or gate-off/gate-on
diverge — the first step-5 exercise settles the mechanics (change BOTH, rebuild, re-pin).

## THEN: tranche 4 opening build (ratified, unbuilt)

**Comptime `Data` indexing + typed Data views** — one work item (Volence ratified both):
`embed(...)[i]` (+ `.len`) in comptime exprs, and `data X: [i16; 320] = embed(...)`
element-typed views (big-endian byte-identity). A-Spec2.3 decision record rides the build
(next number: D2.33). First consumers: sine-table content asserts (`ensure(embed(...)[0]
== 0)`), and `sonic_anims`' word tables.

## Tranche 4 targets (CORRECTED list — plantbadmaps is NOT in the build)

| File | What it exercises first |
|---|---|
| `engine/vram_bases.asm`(*) | THE REVERSE SEAM: `.emp` equ export → AS reads (ram.asm's eventual port needs this proven) |
| `ojz_act_pool.asm`(*) | `align 2` ×3 between BINCLUDEs + dc.l pointer table (dac_samples shape) |
| `games/sonic4/data/animations/particle_anims.asm` | `offsets` inline bodies, first real consumer |
| `games/sonic4/data/animations/sonic_anims.asm` | 15-member offsets table ordered by ANIM_* ids (the ordinals-replace-hand-synced-constants story); replaces the dropped plantbadmaps target |

(*) locate exact paths at recon (`find`/grep the include sites in main.asm/engine.inc);
derive pins from both shapes' `.lst` symbol tables as usual (the tranche-3 files show the
method — grep `X :` in `s4.lst`/`s4.debug.lst`, bounds from the neighboring includes).

## Standing discipline (unchanged)

Worktree branch `port-tranche4` in sigil (`.worktrees/`), aeon branch in the MAIN tree
(worktrees miss untracked generated state), TDD, per-item two-stage reviews + the
two-prong whole-branch review (adversarial + code-sense), byte gates both shapes, negative
probes, gate-off neutrality sha256 ×3, Volence checkpoint before merge. **Run every
build/test command with an explicit `cd`** — the session cwd resets between commands and
bit twice (a test ran against the wrong tree; a docs commit landed on main-tree master and
needed a reset).

## Open items / watch list

- **Empyrean spec working tree STILL UNCOMMITTED** (Volence's docs cadence): the
  2026-07-09c (D2.31) + 2026-07-09d (D2.32 + struct-equ addendum) amendment stack.
- Construct walks pending their triggers: production prelude (walk #1), `vars` vs ram.asm
  (#2), Sonic newtypes incl. the confirmed-working `d0: Angle` param typing (#3, Volence
  drives).
- Ledger watch: stale `demo.bin`/`demo.lst`; the reproducibility "own session" (generator
  outputs); dbcc clobber-lint blind spot (S2-D6); struct-equ export surplus symbols (jot);
  S2-D17 patch/bind demotion decision at campaign end.
- Tranche 5 candidate: `game_loop.asm` (the SOUND_DRIVER_ENABLED ifdef + gameDebugTick
  game-macro hazard class — design against it deliberately).
