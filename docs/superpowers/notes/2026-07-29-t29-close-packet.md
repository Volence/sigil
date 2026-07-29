# 2026-07-29 — t29 close packet (game-side G1: test_static + test_animated)

Porter: Opus subagent (Fable-dispatched, direct). Brief:
`2026-07-29-t29-g1-trivial-objects-brief.md`. Design: `2026-07-29-t29-step0-design.md`.

**Outcome:** the game-side census's recommended first tranche ported. Two object
modules `.emp`-owned behind per-file gates; **byte-delta ZERO throughout** (step-1
faithful compiled byte-identical on the FIRST link; step-2 house flips
jmp/jsr→jbra/jbra byte-identical because every callee is out of `.w` range →
abs.w). Strict **2685 → 2691** (+4 windowed `test_g1_objects_port`, +2 whole-ROM
`mixed_tranche29`). Gate-off dual build EXACT canonical (c51342d0/992d9e7d).

Branch tips at close: aeon `58fa852`, sigil `60c3af5` (pre-merge).

## Region derivation (both shapes; bases shape-invariant, bank not slid)

| Lane | region | base (both) | end anchor | len | org resume |
|---|---|---|---|---|---|
| A test_static | TestStatic_Main | `$10C66` | TestAnimated | `$4` | `org $10C6A` |
| B test_animated | TestAnimated | `$10C6A` | TestPlayer | `$5A` | `org $10CC4` |

Shared anchor `TestAnimated` (static's end / animated's start). Content bytes
track cross-seam operands per shape (Draw_Sprite/AnimateSprite/Perform_DPLC move)
→ compile-twice class, NOT identical-bytes-both-shapes.

## What each pass added

**Step 1 (demanded features / neither-bucket):**
- The `vars DplcV: Sst.sst_custom` overlay — FIRST aeon-corpus `vars` consumer;
  its always-on `[overlay.window-overflow]` check subsumes the AS `objvarsCheck`.
- `vram_art` ADOPTED (objdef.emp comptime fn, ambient-prepended per rings_port).
- `vram_bytes(VramTile) -> VramAddr` BUILT file-local (macros.asm counterpart).
- `VRAM_TEST_SONIC: VramTile` game-config mirror + drift guard.
- Zero externs (Draw_Sprite/AnimateSprite/Perform_DPLC + 4 Sonic pointers are
  bare link symbols). First-linking-compile byte-identity, both shapes.

**Loop pass 1 — Step 3 (reads-wrong):** the comment-claim audit caught the
PORTER'S OWN step-1 false claim — test_static.emp's comment said the AS header
"under-declared" clobbers; the AS `test_static.asm` header already declared
`d0-d3/a1` exactly (Draw_Sprite's set). Corrected (byte-neutral). CREDIT: the
step-3(b) comment-claim audit (the loop catching the porter).

**Loop pass 1 — Step 4:** LOG-only. vram_bytes was step-1 demanded; vram_art
adopted; no clone worth a template (the init field-writes are linear, not a
repeated pattern). Nothing built.

**Loop pass 1 — Step 5 (optimize):** no changes. **C1 INACTIVE (named-site
justification):** the only per-frame per-object work in TestAnimated_Main is three
irreducible engine calls — AnimateSprite / Perform_DPLC / Draw_Sprite — which ARE
the object's required behavior; Perform_DPLC self-gates internally (streams only on
a mapping-frame change). The surrounding code (two SST pointer loads → a2/a3, one
constant `move.w #$7800, d1`) is minimal and byte-identical to the hand-written AS.
No cycle-relevant site to derive, no algorithmic improvement available. Debug-growth
/ $8000 bar: N/A (byte-identical both shapes; bank did not slide). Recorded inactive,
not run silently.

**Panel round (A1+B1+C2, synchronous, read-only):**
- **B1 (corpus-pattern): nothing new** — vram_art adoption, vram_bytes 1st-consumer,
  the `vars` overlay as the sanctioned mechanism, drift-guard/code_addr spellings all
  match the shipped siblings.
- **C2 (correctness-hazard): nothing new** — independently verified TestAnimated_Main
  `clobbers(d0-d4/a1-a3)` = callee union ∪ locally-written a3; TestStatic_Main
  `d0-d3/a1`; DplcV offsets `$2E`/`$32`; `falls_into` genuine; `vram_bytes tile<<5` =
  ×32; no CC/stride/save-restore hazards.
- **A1 (cold-reader): 3 findings** — Finding 3 (roadmap/hoist narration in comments)
  ACCEPTED + fixed (trimmed the vram_bytes hoist plan, DplcV gating lifecycle,
  VRAM_TEST_SONIC kill condition into the kill-list rows; byte-neutral). Findings 1
  (objroutine typed-constructor asymmetry) + 2 ($FF sentinel const) LEDGERED as
  corpus-wide item-13 candidates (match the shipped siblings; a t29-local change would
  reverse the C1 inline ruling / sweep every object module).

**Loop pass 2: DRY** — step-3 re-audit clean, step-4 empty, step-5 empty. One panel
round per the dry-panel rule.

## Neither-bucket headlines

- **First-linking-compile byte-identity, both shapes, zero wave** — the whole
  canonical-bytes tranche moved zero bytes through step 2 (the step-2 branch flips are
  byte-identical at these distances). No re-pin, no 5-site ripple, no $8000 bar.
- **The mixed whole-ROM drift guards resolve against the REAL AS tree** — `_dplc_ptr`/
  `_art_base` (surviving test_player.asm) + `VRAM_TEST_SONIC` (config/constants.asm)
  resolve globally in `mixed_tranche29`, not just via synthetic equs. Confirms the
  DplcV twin + game-config mirror seams are real-build-viable.

## Corrections list

1. **CENSUS UNDER-SCOPE:** test_animated is the FIRST game-side SST-overlay-twin port —
   the census G1 row (§3a) called it "trivial … display + AnimateSprite" (2 cross-seam
   calls) and assigned the overlay "first" to test_parent (G3, §3c). The real file
   carries a `vars DplcV` overlay + a THIRD cross-seam call (Perform_DPLC). Byte-neutral;
   the correction reframes G3 (test_parent is the 2nd overlay, not the debut).
2. **SHAPE-INVARIANT CLARIFICATION:** G1 BASES are shape-invariant; CONTENT bytes track
   cross-seam operands (compile-twice class), not "identical bytes both shapes" (census §2).
3. **PORTER'S OWN STEP-1 FALSE CLAIM** (credited to the step-3b comment-claim audit): the
   "AS header under-declared clobbers" note in test_static.emp was false (the AS header was
   accurate). Caught by the loop, fixed in pass 1.

## Standing pattern (overseer ruling 1)

For future MULTI-FILE OBJECT tranches: **per-file gates** (`SIGIL_EMP_<OBJ>` each),
**one shared test file per tranche** (`test_g1_objects_port.rs`), **one mixed-build fn
per tranche** (`build_mixed_tranche29_rom` + assemble_mixed_tranche29_as_side). The
combined-gate template (`SIGIL_EMP_TEST_OBJECTS`, one gate/two files) is superseded for
multi-file tranches by per-lane gates (cleaner per-lane region pins).

## Step-6 corpus sweep outcomes (overseer-ordered)

1. **vram_bytes / vram_art consumer census.** `vram_bytes`: exactly ONE `.emp` consumer
   (test_animated.emp — def + single use; the types.emp:58 / objdef.emp:33 hits are
   comments). Hoist trigger (2nd consumer) NOT reached → kill row 63 stays open, correctly.
   `vram_art`: 3 adopters (rings.emp `RING_ART_ATTR`, test_objects.emp objdef data ×4,
   test_animated.emp) off objdef.emp's `pub comptime fn` def — adopter list current.
2. **vars-overlay census (grep-grounded).** `grep -rnE '\bvars\b' --include=*.emp`:
   exactly ONE overlay in the whole corpus — `test_animated.emp:24 vars DplcV:
   Sst.sst_custom`. Confirms B1. SEEDS the G3/test_parent brief (test_parent's
   TParentV/TOrbitChildV will be the 2nd/3rd overlay consumers; the shared-overlay-twin
   consolidation question opens then).
3. **objroutine-inline census.** `grep -rn '- ObjCodeBase' --include=*.emp`: THREE
   `code_addr` store sites — test_solid.emp:22, test_particle.emp:35, test_animated.emp:46
   (test_particle.emp:15 is a comment). **Demand number = 3** for the item-13
   `objroutine(l) -> ObjRoutine` typed-constructor candidate (a corpus-wide reversal of
   the C1 inline ruling would touch all 3). Attached to the ledger row.
4. **$FF sentinel census.** `#$FF, prev_anim/prev_frame`: FOUR sites / TWO files —
   test_particle.emp:27,28 + test_animated.emp:43,44 (prev_anim + prev_frame each).
   **Demand number = 4** for the item-13 AnimId/MappingFrame "none/refresh" sentinel-const
   candidate (low priority; near-ceremony per A1). Attached to the ledger row.

## Kill-list rows (added same-commit) + ledger

- Rows 60-63: gate-off body twins + org arms (60); DplcV overlay twin, truth =
  test_player.asm (61); VRAM_TEST_SONIC game-config mirror (62); vram_bytes comptime-fn
  twin (63). All Spec-5 / 2nd-consumer kill conditions.
- Ledger: t29 section (census corrections, VramAddr-on-Perform_DPLC-d1 candidate,
  vram_art/vram_bytes shared-home hoist) + the t29-panel section (objroutine typed
  constructor demand=3, $FF sentinel demand=4).

## Test artifacts (all green by name, SIGIL_STRICT_GATE=1)

Windowed: `test_g1_objects_port::{g1_objects_regions_match_reference,
g1_objects_debug_regions_match_reference, g1_undoctored_compile_equals_the_reference_window,
g1_doctored_reference_diverges}`. Whole-ROM: `mixed_dac_rom::{mixed_tranche29_rom_matches_assembled_reference,
mixed_tranche29_debug_rom_matches_assembled_reference}`.
