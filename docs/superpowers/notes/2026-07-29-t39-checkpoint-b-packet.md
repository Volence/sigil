# t39 checkpoint (b) — the FINAL three objects (test_enemy + test_player + path_swap)

Porter: Opus subagent (direct-dispatch). Overseer: Fable. Masters: sigil `ea38f7c` /
aeon `fa474cd`. Branches `port-tranche39` BOTH repos, worktrees `.worktrees/port-tranche39`.
**Checkpoint (a) COUNTERSIGNED** (overseer own-run: plain 4b66cace EXACT · debug
1c256b3b EXACT · strict 2862/0(1 ignored), zero regressions). This is the checkpoint-(b)
packet: the loop ran to DRY (panel round returned no cycle-reopener) and the close-packet
duties are staged. **After t39 the object bank is ALL-.EMP.**

## 0. Bars — HELD

- CANONICAL-BYTES tranche: step-1 delta **ZERO** both shapes. plain **4b66cace/421041** ·
  debug **1c256b3b/429102** UNCHANGED (verified: gates-OFF AS rebuild both shapes).
- Own strict (`SIGIL_STRICT_GATE=1 AEON_DIR=<t39 aeon worktree> --workspace --release
  --no-fail-fast`): **2862 / 0 / 1 ignored** (baseline 2856 + 6 new t39 gate tests).
  Re-run pending after the packet commit; the last full run before the panel byte-neutral
  fixes was 2862/0/1, and every gate re-run since (g4 ×4, mixed_tranche39 ×2,
  contract_closure_corpus ×6) is green.
- **Zero extern proc — HELD.** None in the 3 new files; a whole-`games/` grep finds zero
  `extern proc` (only a comment in game_debug.emp). Player_SensorFloor resolves
  module-to-module via bare `jbsr`. The 68k game side stays at ZERO extern proc.

## 1. Evidence block (gates + counts)

- **Windowed byte gate** `test_g4_final_objects_port` (4/4 strict, both shapes): region
  match test_player ($270) + test_enemy ($48) + path_swap **shape-dependent** ($92 plain /
  $FA debug) + the DEBUG-only raise_error seam (MDDBG__ErrorHandler entry points); negative
  probe + positive control on path_swap's shape-dependent window; drift-guard counts
  (player 30+twin+4, enemy 30+twin, swap 30+twin+1); outbound `objroutine(TestEnemy_Init)` +
  `dc.l ObjDef_PathSwap`.
- **Whole-ROM** `mixed_tranche39` (2/2 strict): plain == `s4.bin` (4b66cace), debug ==
  `s4.debug.bin` (1c256b3b) through convsym; per-shape path_swap resume proof
  (DeformTable_Zero `$1128C`/`$112F4`); internal-gate resume proof (TestSolid_Init `$10F4A`);
  outbound proofs.
- **Closure gate** `contract_closure_corpus` (6/6): honest-contract residue empty — the
  TestPlayer d7 whole-register clobber closes the one firing.
- **$8000 / t24 control:** TEST_PLAYER + TEST_ENEMY pins are shape-invariant
  (plain_base == debug_base) → the debug object bank did NOT slide; path_swap's per-shape
  resume is the sole shape divergence (the object bank's FIRST shape-dependent gate).
- **Contract/bless counts:** 7 `pub proc` + 1 `pub data`; 7 `clobbers` (TestPlayer/
  TestPlayer_Main honest `d0-d7/a1-a3`), 2 `falls_into`, **5 `ensure` drift guards**
  (test_player 4: VRAM_TEST_SONIC + _dplc_ptr/_art_base/_debug_flag; path_swap 1:
  VRAM_TEST_OBJ; test_enemy 0 unguarded), **0 `as`-bless** (no domain-newtype construction
  sites — the `code_addr` label-diff stores follow the test_solid/test_particle no-bless
  precedent).

## 2. Design calls (ratified at (a); bases restated for the gate)

- **Internal gates on test_player AND test_enemy** (t34 keystone). Their zero-byte headers
  stay AS-visible because surviving consumers read them: test_animated.emp's
  `_dplc_ptr`/`_art_base` guards, test_objects.emp's `ENEMY_PATROL_SPEED` objdef guard,
  object_test_state.asm's `STUB_FLOOR_Y` (×7). C2 re-derived this SOUND (headers emit zero
  bytes — `objvarsCheck` is a pure `if/fatal`; resume orgs `$10F02`/`$10F4A` are the next
  reference labels). test_enemy's internal-gate need (ENEMY_PATROL_SPEED) was discovered
  IN step 1, extending the recon note.
- **path_swap: whole-file gate, SHAPE-DEPENDENT, PER-SHAPE resume orgs** (`$1128C` plain /
  `$112F4` debug). Its `ObjDef_PathSwap` label is `.emp`-exported (the `dc.l` consumers
  flip). NEW ripple surface: a wave moving path_swap's bytes must re-derive BOTH orgs
  (gap-ledger rider; wave-ripple checklist instance addendum).
- **TestPlayer clobbers(d7) — whole-register honest contract.** Player_SensorFloor does
  `moveq #0,d7` (full 32-bit clear); the `move.w d7` bracket preserves only d7's low word
  (the RunObjects dbf counter), so the high word is genuinely clobbered. This is the
  IDENTICAL already-ratified t34 Player_Main pattern — **gap-ledger lines 1760/1768** (the
  `.w` partial-save verifier-gap, DEMAND 1). C2 confirmed.
- **C3 flagged-INACTIVE for path_swap, NAMED basis** (gate-accepted at (a)): path_swap
  touches no VDP/DMA/bus and its ported prose makes only coordinate-handling BEHAVIORAL
  claims (teleport rebase, side-tracking), no hardware/timing/bank/section/camera claims.
- **C1 flagged-INACTIVE, NAMED basis:** all three are TEST objects on human-timescale /
  code_addr-dispatched paths, not per-frame hot paths — no cycle-lens cuts warranted. The
  tree agrees (no hot-loop bodies). Reversible at the gate.

## 3. Loop (steps 2 → 3→4→5 → panel) — per-pass breakdown

**Step 2 (house format, items 1-11):** the ports were written in modern form at step 1
(bare Bcc / jbra / jbsr — byte-identical to the twins' `.s`/jmp/jsr; brace-indent;
`Sst.field`/overlay access; `/`-grouped 68k contracts, `falls_into` last; bare abs-EA for
Ctrl/Player_1). Item-6 type-layer: LineSide/Facing, LayerId, PixelExtent candidates LOGGED
(T2/A4-i gated, log-don't-adopt); 0 `as`-bless. Item-8 ladder: all bare-link /
`extern()`-const-guard, no extern proc. Item-9 honest contracts (the d7 fix). No byte
movement (verification pass).

**Loop pass 1 — step 3 findings** (byte-neutral, applied): (3b magic-number) site-comment
the `$A0FA` debug art_tile + the `#$F` subtype mask; (3b name/dead-code) drop test_enemy's
dead `.draw:` label (test_particle.emp precedent). **step 4:** no constructs to adopt/build
(objdef/vram_art/vars all already adopted; no repeated in-tranche shape). **step 5:** test
objects, C1-inactive named basis → no optimizations (not-applicable-because per line).

**Loop pass 2 — step 3/4/5 all EMPTY** → DRY claim → dry panel.

**Dry panel (A1 · B1 · C2; C3 flagged-inactive, C1 flagged-inactive):**
- **C2 (correctness, highest weight): CLEAN.** All 3 mandated re-derivations SOUND (§2).
  Full gate-blind sweep clean (CC-clobber, overlay strides == AS equates, contract unions,
  save/restore). One minor comment finding — already fixed via A1.
- **A1 (cold reader): reads-wrong fixes APPLIED** (byte-neutral): TestPlayer_Debug clobbers
  comment `d0-d1`→`d0-d3`; test_enemy "direction sets sign"/"2× range each way" corrected.
  **ADOPT-NOW applied:** `BUTTON_{LEFT,RIGHT,UP,DOWN}_BIT` at test_player's 6 d-pad `btst`
  (t35 discharge — consts free, no new mirror). Candidates LEDGERED: object-header init
  DSL, guarded-overlay drift-check construct, "mirror external symbol" declaration ask,
  art_tile composite constructor; Button/AnimId/Subtype-bitfield item-13 newtypes.
- **B1 (corpus pattern): constructs consistent.** Candidates LEDGERED (build-vs-ledger =
  gate/step-6 call, t36/t37 precedent): `perform_dplc_from(<overlay>,<tile>)` (2 sites +
  sonic variant, byte-neutral; build touches the merged test_animated.emp), overlay-prefix
  composition (TPlayerV extends DplcV), sign-mirror clones (byte-changing → master backlog).
- **FLAGGED to Volence (step-4(d), deliberate dead code — NOT cut):** TestPlayer_Debug's
  A-turbo branch is a no-op (`DEBUG_FLY_SPEED_FAST == DEBUG_FLY_SPEED`); forward-scaffolding,
  comment discloses the camera-clamp reason, byte-faithful → his disposition.

**Panel outcome:** no finding re-opens the cycle byte-changingly or on correctness; the
byte-neutral reads-wrong/adopt items were applied, the rest ledgered → **DRY** (one panel
round per the cost-bounded rule).

**Step 6 corpus sweep:** NOT triggered — no construct was BUILT this tranche (perform_dplc_from
et al. are ledgered, not built). If the gate rules `perform_dplc_from` a build, its sweep
(test_animated.emp + sonic.emp + kill row) rides that decision.

## 4. Close-packet duties — STAGED

- **CENSUS AMENDMENT (applied):** `2026-07-29-game-side-census.md` gains the t39 STATUS
  AMENDMENT — **THE OBJECT BANK IS ALL-.EMP** — with the 5-item corrections list and the
  0-for-5 census-pattern note (every t3x tranche has overturned census rows; the recon step
  earns its keep; the POST-TWIN retrospect inherits a census-refresh duty).
- **ROW-74 READER-SET UPDATE (re-derived per field):** the brief's premise — "test_player.asm's
  port removes an AS `_pl_*` reader set" — is **FALSE** (census correction #2): test_player
  reads NO `_pl_*` / PlayerV fields; it uses its own DplcV/TPlayerV overlay. So t39 removes
  ZERO `_pl_*` readers; the `_pl_*` reader set (the player cluster) is UNTOUCHED by t39.
  What test_player's port actually does per field:
  - `_dplc_ptr` / `_art_base`: the AS-CODE consumer (TestPlayer_Main) moves to `.emp`; the
    AS-HEADER definitions SURVIVE (internal gate), so test_animated.emp's guards keep
    resolving. AS readers remaining: test_player.asm header (def) + test_animated.asm header
    (gated twin) — both lockstep, die at Spec 5.
  - `_debug_flag`: AS-CODE consumer moves to `.emp`; AS-HEADER def survives (internal gate);
    single-consumer, no external reader.
  - `ENEMY_PATROL_SPEED` (test_enemy): AS-CODE stops referencing it; the AS-HEADER def
    survives (internal gate) for test_objects.emp's objdef guard.
  - `STUB_FLOOR_Y` (test_player): NOT consumed by test_player at all — a header equate for
    object_test_state.asm (surviving T1 file); the internal gate keeps it visible.
- **KILL ROWS (applied, same-commit): 84 / 85 / 86** in `twin-scaffolding-kill-list.md` —
  test_player internal-gate scaffolding, test_enemy internal-gate scaffolding, path_swap
  gate-off body + PER-SHAPE org arm. All kill = Spec 5.
- **LEDGER (applied):** the t39 step-2 type-layer candidates + per-shape-org ripple rider +
  the full panel-findings rows (C2 clean / A1 applied+ledgered / B1 ledgered / dead-turbo
  flag) in `campaign-gap-ledger.md`.

## 5. STOP — checkpoint (b)

Loop dry, panel clean, close-packet duties staged. Gate (c), the rebase, and the merge are
the overseer's (t40 runs parallel — merge order ruled at the gates). No push, no merge, no
rebase. Branch tips: sigil `port-tranche39`, aeon `port-tranche39` (see the final report for
the exact SHAs).
