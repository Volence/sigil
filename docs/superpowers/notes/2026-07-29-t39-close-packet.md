# 2026-07-29 — t39 close packet (the FINAL three objects — THE OBJECT BANK IS ALL-.EMP)

Porter: t39 (Opus subagent, direct-dispatch). Overseer: Fable. Checkpoints (a)/(b)
countersigned from the porter's tips; gate (c) OPEN, **t39 merges FIRST** (bases still
equal the masters — no rebase; t40 takes the rebase at its gate).

**Tips**: aeon `a498b39` · sigil `<this commit>` (branch `port-tranche39` both). CRCs,
overseer own-build both shapes: plain **`4b66cace`** EXACT · debug **`1c256b3b`** EXACT —
**UNCHANGED from canonical**. Strict (`--no-fail-fast`, AEON_DIR = worktree):
**2862 / 0 / 1 ignored** (baseline 2856 + 6 new t39 gate tests). No push · no rebase
(masters aeon `fa474cd` / sigil `ea38f7c` — the overseer merges at the gate).

## 0. Bars (overseer-countersigned at a + b)

- CANONICAL-BYTES tranche: step-1 delta ZERO both shapes; every step-2/3/panel movement
  was byte-neutral (comments, a dead label, a const adoption that folds identically). The
  whole tranche is byte-neutral.
- Baseline strict 2856/0/1 at dispatch → 2862/0/1 at close (+6: mixed_tranche39 ×2 +
  test_g4_final_objects_port ×4; the closure-residue firing that rode the step-1 d7
  contract cleared with the whole-register fix).
- **Zero extern proc — HELD.** None in the 3 new files; a whole-`games/` grep finds zero
  `extern proc`. Player_SensorFloor resolves module-to-module via bare `jbsr`.

## 1. THE HEADLINE — THE OBJECT BANK IS ALL-.EMP

`test_enemy.emp` + `test_player.emp` + `path_swap.emp` land. All 11 objects in
`gameObjectBankIncludes` (test_static/animated/enemy/player/solid/particle/emitter/parent/
stress_emitter/churn/path_swap) + the 6-file player cluster are now `.emp`. **The 68k game
side is down to T1** (object_test_state + ojz_scroll_test — the debug-divergent harness
states, ojz_scroll_test entangled with open kill row 35) **+ main.asm** (manifest) **+ the
4-file config cluster** (Spec-5 flip). No new language/frontend feature was demanded — the
object template (test_solid/test_particle/test_animated/test_parent) covered all three.

## 2. Byte-delta table — ZERO

| region | plain Δ | debug Δ |
|---|---|---|
| TEST_PLAYER ($10C92, $270) | 0 | 0 |
| TEST_ENEMY ($10F02, $48) | 0 | 0 |
| PATH_SWAP ($111FA, $92 plain / $FA debug) | 0 | 0 |
| (whole ROM) | 0 (4b66cace) | 0 (1c256b3b) |

path_swap is the object bank's FIRST shape-dependent region (debug **+$68**: the reserved-
bit `raise_error` guard block + the debug `jmp Draw_Sprite` vs release `rts` tail).

## 3. THE DESIGN CALLS (ratified at a/b; survival evidence)

### 3.1 Internal gates ×2 (test_player + test_enemy) — the t34 keystone applied twice
Whole-file gating either file would break a `.emp`/AS drift guard. So both AS twins keep
their ZERO-BYTE headers ALWAYS-emitted, gating only the CODE, with an `else org` resume:
- **test_player.asm** — DplcV `struct` (`ifndef _dplc_ptr`) + TPlayerV `struct` +
  `_debug_flag`/physics/`STUB_FLOOR_Y` equates + `objvarsCheck`; resume `org $10F02`
  (TestEnemy_Init). Survival evidence: **test_animated.emp's `_dplc_ptr`/`_art_base` drift
  guards** read these equates cross-seam, and **object_test_state.asm reads STUB_FLOOR_Y**
  (×7 sites, C2-confirmed).
- **test_enemy.asm** — TEnemyV `struct` + `_enemy_*` + `ENEMY_PATROL_SPEED`/`_RANGE` +
  `objvarsCheck`; resume `org $10F4A` (TestSolid_Init). Survival evidence:
  **test_objects.emp's `ensure(extern("ENEMY_PATROL_SPEED"))` objdef guard** reads it. This
  consumer was **discovered IN step 1**, extending the step-0 recon note (the right
  behavior — the recon named test_player's DplcV dependency, step 1 found test_enemy's).
- C2 re-derived both SOUND: `objvarsCheck` is a pure `if/fatal` (zero `dc`), so the header
  emits nothing and the resume org lands the next reference label; byte-identity confirms.

### 3.2 path_swap — whole-file gate, SHAPE-DEPENDENT, PER-SHAPE resume orgs
path_swap's `ObjDef_PathSwap` objdef label is `.emp`-EXPORTED, so the `dc.l ObjDef_PathSwap`
AS consumers (act_descriptor / objdef type table) flip to the `.emp` label — no header need
survive → whole-file gate. But the 2 `__DEBUG__` blocks make it shape-divergent, so
main.asm carries BOTH resume arms: `ifdef __DEBUG__ org $112F4 / else org $1128C`
(DeformTable_Zero, gameDataIncludes' first label = the next placement). **NEW WAVE-RIPPLE
SURFACE** (§8): a wave moving path_swap's bytes re-derives BOTH per-shape orgs — the
shape-invariant single-org assumption every prior object gate relied on does NOT hold here.

### 3.3 TestPlayer clobbers(d7) — whole-register honest contract
Step 1's faithful contract (`clobbers(d0-d6/a1-a3)`, d7 "preserved") FAILED the closure
gate. Root: Player_SensorFloor does `moveq #0,d7` (full 32-bit clear); TestPlayer_Main's
`move.w d7,-(sp) … move.w (sp)+,d7` bracket restores only d7's LOW word (the RunObjects dbf
counter) — the high word is genuinely clobbered. At whole-register granularity d7 IS
clobbered → `clobbers(d0-d7/a1-a3)` is the honest superset (a0 preserved per the RunObjects
contract, a4-a6 untouched). The IDENTICAL already-ratified t34 Player_Main pattern
(**gap-ledger lines 1760/1768**, the `.w` partial-save verifier-gap, DEMAND 1). C2
confirmed; byte-invisible fix.

## 4. What each pass added (step-3 vs step-5 + neither-bucket)

**Neither-bucket (step-1 demanded features / probes / live findings):** NO demanded feature
(template covered it). The whole-register d7 contract catch (§3.3) — a step-1 closure-gate
finding. The three census corrections + the internal-gate-×2 discovery (§6). Windowed +
whole-ROM byte gates both shapes; negative probe + positive control on path_swap's
shape-dependent window.

**Loop pass 1 — step 3 (byte-neutral, applied):** (3b magic-number) site-comment the
`$A0FA` debug art_tile + the `#$F` subtype mask; (3b name/dead-code) drop test_enemy's dead
`.draw:` label (test_particle.emp precedent). (3a) ceremony/escape-hatch/domain-type scans
→ type-layer candidates LOGGED (LineSide/LayerId/PixelExtent — T2/A4-i gated).
**Loop pass 1 — step 4:** no construct to adopt/build (objdef/vram_art/vram_bytes/vars all
already adopted; no repeated in-tranche shape). **Loop pass 1 — step 5:** C1-inactive named
basis (test objects, code_addr-dispatched, not per-frame hot paths) → each interrogation
line not-applicable-because; no optimizations.
**Loop pass 2 — step 3/4/5 ALL EMPTY → DRY.**

## 5. Panel outcomes (A1 · B1 · C2; C3 + C1 flagged-INACTIVE, named bases)

- **C3 flagged-INACTIVE, named basis:** path_swap touches no VDP/DMA/bus; its prose makes
  only coordinate-handling BEHAVIORAL claims (teleport rebase, side-tracking), no
  hardware/timing/bank/section/camera claims. (Gate-accepted at (a).)
- **C1 flagged-INACTIVE, named basis:** all three are TEST objects on human-timescale /
  code_addr-dispatched paths, not per-frame hot paths — no cycle-lens cuts. Tree agrees.
- **C2 (correctness, highest weight): CLEAN.** All 3 mandated re-derivations SOUND
  (§3.1 internal-gate survival, §3.3 d7, path_swap raise_error seam: +$68 sole divergence,
  debug-gated never-fires-in-release, release `rts` correct, no leak). Full gate-blind
  sweep clean (CC-clobber, overlay strides == AS equates, contract unions, save/restore).
  One minor comment finding — already fixed via A1.
- **A1 (cold reader): reads-wrong FIXES applied** (byte-neutral): TestPlayer_Debug clobbers
  comment `d0-d1`→`d0-d3`; test_enemy "direction sets sign" (neg.w does) + "2× range each
  way" (edge-to-edge) corrected. **ADOPT-NOW applied:** `BUTTON_{LEFT,RIGHT,UP,DOWN}_BIT`
  at test_player's 6 d-pad `btst` (t35 discharge — consts free from engine.constants, no
  new mirror; A/B/C stay raw, no `_BIT` const). Candidates ledgered (§7).
- **B1 (corpus pattern): constructs consistent.** Candidates LEDGERED (§7).
- **Panel outcome:** no byte-changing / correctness finding → DRY (one panel round, per the
  cost-bounded rule). **Step 6 corpus sweep NOT triggered** (no construct BUILT).

## 6. Corrections list (BOTH directions — the census 0-for-5 pattern)

The census is now **0-for-5 t3x tranches on being fully right** — every tranche has
overturned rows. This is the recon step earning its keep; the POST-TWIN retrospect inherits
a census-refresh duty (recorded in the census amendment).

**The census's 3 (down-chain):**
1. §3a test_enemy cross-seam "Draw_Sprite, ObjectMove, TouchResponse" → actually
   `ObjectMoveX` + `Draw_Sprite` (no TouchResponse, no ObjectMove).
2. §3a test_player "3 asserts → ensure … Sound_PlaySFX" → ZERO asserts, ZERO Sound_PlaySFX;
   uses its own DplcV/TPlayerV overlay (NOT PlayerV).
3. §3a path_swap "Draw_Sprite, section calls" → NO section calls (Draw_Sprite + Player_1
   RAM + raise_error); no hardware claims → C3 flagged-inactive.

**The brief's 1 (up-chain — the record owns its errors up):**
4. The brief's row-74 premise "test_player.asm's port removes an AS `_pl_*` reader set" is
   **FALSE** — test_player reads NO `_pl_*`/PlayerV fields. See §9.

**The recon-extension discovery (in-step-1, the right behavior):**
5. test_enemy's `ENEMY_PATROL_SPEED` consumer (test_objects.emp objdef guard) forced a
   SECOND internal gate the recon note did not name — found in step 1, note extended.

## 7. Type-layer + construct candidates — LEDGERED (nothing built)

- **Type-layer (item-6, T2/A4-i gated, log-don't-adopt):** `LineSide`/`Facing` (path_swap
  `prev_side` + test_enemy `direction` — same "which side/way" space, cross-mix hazard),
  `LayerId` (0/1), `PixelExtent` (half_height/steps_remaining), + A1's `Button` (A/B/C bits
  lack `_BIT` consts), `AnimId` (raw 0/1/2, no `.emp` ANIM_* const), `Subtype` bitfield. 0
  `as`-bless this tranche (no domain-newtype construction sites; `code_addr` label-diff
  stores follow the test_solid/test_particle no-bless precedent).
- **Constructs (B1, build-vs-ledger = gate/step-6 call):** `perform_dplc_from(<overlay>,
  <tile>)` (test_player + test_animated identical 4-liner + sonic variant; byte-neutral;
  build touches the MERGED test_animated.emp → step-6 sweep + kill row); overlay-prefix
  composition (TPlayerV extends DplcV); the object-header init DSL; the guarded-overlay
  drift-check construct; the "mirror external symbol" declaration ask; an art_tile composite
  constructor (`$A0FA`); sign-mirror clones (byte-changing → master backlog).

## 8. Retirement / kill rows + the new ripple surface

- **Kill rows 84 / 85 / 86** (`twin-scaffolding-kill-list.md`, same-commit): test_player
  internal-gate scaffolding (header ALWAYS + gate-off body + `org $10F02`), test_enemy
  internal-gate scaffolding (`org $10F4A`), path_swap gate-off body + PER-SHAPE org arm.
  All kill = **Spec 5** (the gate-off AS build dies; equates/objdef/overlays flip to
  `.emp`-owned). Also: test_player mirrors VRAM_TEST_SONIC (row-62 class); path_swap mirrors
  VRAM_TEST_OBJ (row-65 class, now ×4).
- **NEW WAVE-RIPPLE SURFACE (ledgered):** path_swap's PER-SHAPE resume orgs. Any wave that
  moves path_swap's bytes (or shifts the bank upstream of it) must re-derive BOTH
  `org $1128C` (plain) AND `org $112F4` (debug). Added as a checklist rider for any
  t39-touching wave.

## 9. Row-74 reader-set update (re-derived per field)

t39 removes ZERO `_pl_*` readers — the player-cluster `_pl_*` set is UNTOUCHED (§6 item 4).
Per field:
- `_dplc_ptr` / `_art_base`: the AS-CODE consumer (TestPlayer_Main) moves to `.emp`; the
  AS-HEADER definitions SURVIVE (internal gate) → test_animated.emp's guards keep resolving.
  Remaining AS readers: test_player.asm header (def) + test_animated.asm header (gated
  twin) — lockstep, die at Spec 5.
- `_debug_flag`: AS-CODE consumer → `.emp`; AS-HEADER def survives; single-consumer.
- `ENEMY_PATROL_SPEED`: AS-CODE stops referencing it; AS-HEADER def survives for
  test_objects.emp's objdef guard.
- `STUB_FLOOR_Y`: not consumed by test_player at all — a header equate for
  object_test_state.asm (surviving T1 file); the internal gate keeps it visible.

## 10. Volence-flagged item (default = keep)

**TestPlayer_Debug A-turbo no-op** — the `btst #6,d0 / beq / moveq #DEBUG_FLY_SPEED_FAST,d1`
branch does nothing today (`DEBUG_FLY_SPEED_FAST == DEBUG_FLY_SPEED == 16`). Deliberate
forward-scaffolding (turbo slot wired, clamped to base pending camera-follow tuning); the
comment discloses the reason; byte-faithful to the AS twin. Per step-4(d) FLAGGED to
Volence, **default = keep-as-scaffolding** (the Camera_Pan_Offset precedent). The overseer
is surfacing it; any override relays at/after the merge.

## 11. Gate state

Loop dry, panel clean, close-packet duties done. **The merge, provenance, roadmap, and
sweep are the overseer's.** t39 merges first (no rebase). t40 (Z80 rung-4) runs parallel — I
touched no sound file.
