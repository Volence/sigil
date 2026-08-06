# Hygiene lane packet (2026-08-06)

Three byte-neutral items: the closure-preserver tail-class adoption (aeon contract
text), the three define-free corpus gates relocated-and-widened (sigil tests), and
the `@budget` census close (sigil ledger). Seed proven byte-identical ×7 before any
edit (chain 48); byte bar held ×7 after; strict 3447 passed / 0 failed / 4 ignored
= 3451 == branch `#[test]` total; refreeze `--check` OK (chain 48); warn-tier lines
identical ×7 seed→post; pins/goldens/repin untouched.

## Item 1 — `preserves(a0)` on the Draw_Sprite/DeleteObject tail class (ledger row 2088)

The deferred CLOSURE-preserver census ran. `preserves(a0)` adopted on all 15
remaining tail-class Mains; every one VERIFIES through the 68k preserves-through-tail
closure credit (zero `[proc.preserves-unverifiable]` refusals). With
`TestChurnObj_Main`'s precedent the class is now 16/16.

Per-Main adoption outcome (all PROVED a0):

- `DemoBox_Main` (games/demo/objects/demo_box.emp) — `jbra Draw_Sprite`
- `Player_DebugMove` (games/sonic4/player/player_common.emp) — `jbra Draw_Sprite`
- `Sonic_LoadArt` (games/sonic4/player/sonic.emp) — `jbsr Perform_DPLC` + `jbra Draw_Sprite`
- `TestEnemy_Main` (test_enemy.emp) — `jbra Draw_Sprite`
- `TestParticle_Main` (test_particle.emp) — `jbra Draw_Sprite`
- `TestEmitter_Main` (test_emitter.emp) — `jbra Draw_Sprite`
- `TestStressEmitter_Main` (test_stress_emitter.emp) — `jbra Draw_Sprite`
- `TestStatic_Main` (test_static.emp) — `jbra Draw_Sprite`
- `TestPlayer_Main` (test_player.emp) — `jbra Draw_Sprite`
- `TestPlayer_Debug` (test_player.emp) — `jbra Draw_Sprite`
- `TestAnimated_Main` (test_animated.emp) — `jbra Draw_Sprite`
- `PathSwap_Main` (path_swap.emp) — `jbra Draw_Sprite`
- `TestSolid_Main` (test_solid.emp) — `jbra Draw_Sprite`
- `TestChildPart_Main` (test_parent.emp) — two tails `jbra Draw_Sprite` / `jbra DeleteObject`; a0 PROVES
- `TestParent_Main` (test_parent.emp) — two tails `jbra DeleteObject` / `jbra Draw_Sprite`; a0 PROVES

REFUSALS: none for a0. Per the row's caveat, d2 was NOT attempted on
`TestChildPart_Main` — its second `jbra Draw_Sprite` tail clobbers d2, so a `d2`
claim would refuse; the adoption stayed inside what the closure proves (a0 only).

Draw_Sprite preserves a0 by omission (a0 is its `*Sst` input, absent from its
`clobbers(d0-d3/a1)`); DeleteObject declares `preserves(d2, a0)`. Both builds
(sonic4 + demo) compile clean — the checker is the verifier.

## Item 2 — the three define-free gates relocated-and-widened (ledger row 2140)

`movem_restore_guard_corpus.rs`, `preserves_corpus.rs`, `dead_save_corpus.rs` moved
from `crates/sigil-frontend-emp/tests/` (which cannot depend on `sigil-harness`, the
owner of the `-D` profiles) to `crates/sigil-cli/tests/`, and re-run PER shipped
shape under `native::shape_defines` + the shape's bound `InterfaceEnv`
(`bind_corpus_interfaces`). No structural wall — sigil-cli already depends on both
sigil-harness and sigil-frontend-emp.

Before → after scanned-site counts (the non-vacuity bar):

- `movem_restore_guard` — GREW. Define-free baseline 32 restores → widest shape 33.
  The +1 is a `SOUND_DRIVER_ENABLED`-gated `movem (sp)+` restore invisible to
  no-define lowering (sonic4 plain/debug, config_a, lean = 33; the sound-off
  config_b + both demo shapes = 32). The (sp)+ exemption tripwire re-proven under
  every shape.
- `dead_save` — IDENTICAL with DEBUG arms counted. Define-free baseline 3 firings →
  3 in all seven shapes (the three known customers: TestChurnObj_Main/A0,
  TileCache_FillColumn/D7, TileCache_WarmupBelowRow/D7). No gated dead-save was
  hiding.
- `preserves` — WIDENED coverage, shape-STABLE. 6 define-free evals → 42 (6 residue
  procs × 7 shapes); every status matches its prediction, proving the checkpoint is
  not an artifact of the empty define set (3 DEBUG shapes + 4 non-DEBUG all agree).

## Item 3 — `@budget` census close (ledger, new row)

Census: exactly ONE live `@budget` (`Process_DMA_Critical`,
engine/system/dma_queue.emp:270, `@budget(cycles: 670)`, prose/table/walk meet at
670), ONE blocked candidate (`SfxDispatch`, engine/sound/sound_sfx.emp:611 — Z80
T-state demand subset per row 2118, and loop-bearing), zero `@cycles_exact`. No other
corpus proc carries a prose/derivation cycle-count customer. The "52 measurable
procs" figure was a misattribution — real structural-boundability = 235 (row 2115);
52 = row 2069's `proc.sr-undeclared` warn-tier count. Queue item DONE by census;
zero-customer adoption is ceremony.

## Per-pass findings

STEP 3 (retrospect / language asks): none new. Item 1 is the fruit of the
2026-08-06 preserves-through-tail credit; the census it closes was already the
row-2088 worklist. Item 2's relocation exposed no new language gap — the harness
`shape_defines`/`bind_corpus_interfaces` seam is exactly the injection point the row
2140 called for.

STEP 5 (perf / hazard / soundness): the one soundness-relevant find is that the
`movem_restore_guard` exemption tripwire was UNGUARDED over the `SOUND_DRIVER_ENABLED`
arm (its earlier no-define home never lowered that `movem (sp)+` restore); the
relocation now scans it, +1 restore. No hazard surfaced — the restore is a matched
save/restore round-trip, property holds.

NEITHER-BUCKET HEADLINE: the census-by-verification pattern — add the claim, let the
`[proc.preserves-unverifiable]` error tier adjudicate, keep only what compiles — made
Item 1 a mechanical 15-for-15 with zero manual proof reading. All three items are
byte-neutral (contract text is comptime; test/doc edits emit no ROM byte).

## Panel round (2026-08-06)

Zero MUST-FIX. All 15 `preserves(a0)` claims were independently traced and HOLD —
Lens C refutation attempts failed on every path, including DeleteObject's
slot-zero/pointer-restore and Sonic_LoadArt's Perform_DPLC closure. Adjudicated
fixups, all in sigil `hyg` (aeon branch untouched):

1. The three relocated tests' doc comments reworded from relocation-narration
   ("its earlier home… here it re-runs") to present-tense contract facts — the
   layering rationale kept (sigil-cli depends on both crates; per-shape `-D` lowering
   is the point), the old-location contrast dropped.
2. `movem_restore_guard`'s stale "26+" figure (comment + panic message) corrected to
   the measured 32 define-free / 33 widest.
3. `movem_restore_guard` gained a WIDENING pin: the widest shape's visited-restore
   count must STRICTLY EXCEED the define-free baseline (33 > 32 today). Without it a
   sound-gated restore that stops lowering leaves the gate quietly green. The `>= 20`
   floor stays.
4. `dead_save`'s non-vacuity floor raised from `> 0` to `>= 3` (tolerant, not an
   exact pin), giving the ledger's census-3 claim teeth.
5. The `aeon_dir()`/`SIGIL_STRICT_GATE` idiom unified — `dead_save` and `preserves`
   now use the `fn aeon_dir() -> Option<PathBuf>` helper form `movem` already had;
   `preserves`' `6 * 7` derived as `cases.len() * shipped_shapes().len()`.
6. Ledger: the budget-census close carries its date (— CLOSED 2026-08-06); a new
   step-3 row records the shared corpus-test-harness helper ask (the copy-pasted
   `emp_files` / `aeon_dir` / `shipped_shapes`→`shape_defines`→`bind_corpus_interfaces`
   loop across ~7 tests, whose symptoms were this round's stale-count and
   helper-drift catches).

Declined (recorded, no action): whole-corpus `@noreturn` note (already exact-matched),
dead_save not pinning the firing set (dump-by-design).

Re-gate after fixups: strict 3447 passed / 0 failed / 4 ignored = 3451 == branch
`#[test]` total; byte bar ×7 identical to committed goldens; refreeze `--check` OK
(chain 48); clippy `-D warnings` clean on the changed tests. Fixups are test/doc only
— byte-neutral.
