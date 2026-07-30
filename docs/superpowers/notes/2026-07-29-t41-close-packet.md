# 2026-07-29 — t41 close packet (T1 harness states) — CHECKPOINT (b), staged for the overseer

Porter: t41 (Opus, direct-dispatch). Overseer: Fable. STOP at checkpoint (b) — gate
(c), the rebase-if-masters-moved, and the merge are the overseer's.

## HEADLINE — THE 68k GAME SIDE IS CODE-COMPLETE (census amendment)

`object_test_state.emp` + `ojz_scroll_test.emp` are the LAST game-side code files.
Every game-side `.asm` code file now carries a `SIGIL_EMP_*` gate with an `.emp`
twin (verified: main.asm's every `games/sonic4/{objects,test,player,debug}/*.asm`
include sits inside an `ifndef SIGIL_EMP_*` arm). What REMAINS on the game side is
`main.asm` (the manifest) + the config cluster (`config/*.asm`) + `gameDataIncludes`
(parallax/objdefs/act_descriptor/levels DATA, largely ported) — i.e. the Spec-5 flip
itself. After t41: the seams + the generator + the flip ARE the remaining campaign.

## Tips
- aeon `port-tranche41` @ `26cb20a` (off master `597ce06`)
- sigil `port-tranche41` @ `5a43308` (off the t41 step-0 plan `a98c8a9`)

## Evidence block

- **Gate-off dual rebuild (canonical-EXACT both shapes):** plain `./build.sh` →
  `4b66cace`/421041 · debug `DEBUG=1 ./build.sh` → `1c256b3b`/429102. The
  main.asm/config/game.asm gate scaffolding is byte-neutral gates-off.
- **Whole-ROM mixed byte gates (both shapes):**
  `mixed_tranche41_rom_matches_assembled_reference` (plain) +
  `mixed_tranche41_debug_rom_matches_assembled_reference` (debug) — the full
  assembled ROM byte-identical with both `.emp` regions spliced.
- **Windowed per-file gates (both shapes) + t24 controls** (`test_t1_harness_states_port`,
  cross-refs sourced from the real AS seam so every address is the reference ROM's):
  `t1_regions_match_reference` (plain: object_test_state $5C230/$5BC + ojz $5C7EC/$2C2),
  `t1_debug_regions_match_reference` (debug: $5DC82/$658 + $5E2DA/$2CE),
  `t1_undoctored_compile_equals_the_reference_window` (positive control),
  `t1_doctored_reference_diverges` (negative probe).
- **Strict (own-run, --no-fail-fast):** `2894 passed / 0 failed / 1 ignored`
  (baseline 2888 + 6: 2 mixed whole-ROM + 4 standalone). Corpus nets green:
  `contract_closure_corpus` 6/6, `slot_type_corpus` 3/3.
- **Contract / bless / drift counts:** 8 own drift-guard `ensure`s (object_test_state
  4: VRAM_TEST_OBJ, STUB_FLOOR_Y, VDP_HV_COUNTER, RingArt.len; ojz 4: VRAM_TEST_OBJ,
  VRAM_TEST_MARKER, CAM_SCREEN_HALF_W, CAM_SCREEN_HALF_H) + all ambient guards PASS
  against the real AS tree. 3 `as`-blesses (GridX ×1, GridY ×2 at the
  Section_GetSecPtrXY/FlatIDXY call sites). 3 `@discards(dropped)` (the corpus's first
  `@discards` uses — the ignored QueueDMA_Critical carry, test-scene-acceptable). 6
  `pixels_to_coord` adoptions (step-4).
- **Embed repo-root proof:** object_test_state's `TestArt` embeds
  `games/sonic4/test/ring_art.bin` + `art/palettes/sonic.bin` resolved from the AEON
  REPO ROOT (`include_root = embed_base = repo root`, the BINCLUDE path model per the
  overseer's ruling); the embedded bytes gate byte-identical to the twin's BINCLUDE
  both shapes; `ensure(RingArt.len == 512)` drift-guards the blob.

## Loop record (steps 2→5, dry by panel)

- **Step 2 (house format):** brace-indented the `if DEBUG` blocks to the corpus
  8-space scheme (byte-neutral). Comment/module-header/twin-note review clean bar the
  dry-panel fixes below.
- **Step 4 (back-prop / typed constructs):** adopted `engine.coords.pixels_to_coord`
  at all 6 spawn-promotion sites (byte-neutral; the template's own doc names "the
  game's spawn code" — children.emp:165 the identical precedent). GridX/GridY blesses
  at the 3 section-coord call sites.
- **Step 5 (corpus nets + adoption debt):** honest transitive clobbers (Load_ObjectList
  adds a3 to GameState_ObjectTest_Init — the .asm header under-stated a0-a2);
  `@discards(dropped)` on the ignored DMA carry; slot-type GridX/GridY. Adoption debt:
  `pixels_to_coord` BUILT (self-contained); `VDP_HV_COUNTER` hoist + the `set_vdp_reg`
  helper DEFERRED (both touch merged engine.vdp/parallax.emp — build-vs-ledger defers
  merged-file sweeps out of a byte-neutral tranche; the set_vdp_reg 3-site trigger IS
  crossed = its discharge condition, argued both ways in the ledger).
- **DRY PANEL (A ceremony · B corpus-pattern · C correctness+C3, gate-adjudicated):**
  - Lens C: all 3 C2 re-derivations (row-35 compensation semantics, embed
    byte-equivalence, Game_Entry gate-aware equ) + all 4 C3 hazard claims (VBlank-masked
    marker copy faithful spelling, Z80-bus wrap, HV-counter profiling, the two
    Debug_Scene_Freeze skips) CONFIRMED — no discrepancy.
  - Lens B: substantively CLEAN — no hand-rolled substitution for an existing corpus
    construct; every repeated shape (struct-list+`_term` sentinel, longword-copy dbf
    loop, VBlank-masked VDP copy, the parallax mode-3 derivation clone) is a
    corpus-tracked/deferred candidate (gap-ledger / kill row 35), the port consistent
    with each interim idiom. One within-file annotation-density gap (the DEBUG profiling
    block's absolutes lacked twin-notes) → resolved with one block-level width note (the
    corpus itself sanctions bare absolutes — entity_window.emp — so no per-operand
    mandate).
  - Lens A: (1) the profiling-clobbers comment restated to the honest transitive
    a0-a3 — its suggested revert-to-a0-a2 was a FALSE POSITIVE (Load_ObjectList's
    `clobbers(d0-d3/a0-a3)` genuinely leaks a3; the corpus closure REQUIRES a0-a3);
    (2) "Ported from skdisasm" → "Source: skdisasm" present-tense provenance.
  - Verdict: no finding re-opened a byte-changing cycle or revealed a correctness/hazard
    defect; all findings cosmetic/comment-tier or a self-correcting false positive → DRY.

## Kill rows (88–90, verified this checkpoint)

- **88** object_test_state.asm gate-off body + PER-SHAPE org arm (`$5E2DA`/`$5C7EC`).
- **89** ojz_scroll_test.asm gate-off body + PER-SHAPE org arm (`$5E5A8`/`$5CAAE` =
  NullInterrupt); **kill row 35 STAYS OPEN** (row-35 force-write carried verbatim,
  overseer-adjudicated CARRY-AS-IS).
- **90** config/game.asm gate-aware `Game_Entry` numeric equ (the error_handler
  ErrorHandler-equ class; the SECOND instance of demand-1 link-time-equ-off-external-base).

## Corrections list (up-chain + porter)

1. **Plan §5 deviation (up-chain):** `#TestArt_End - TestArt` (faithful label-difference
   immediate) is NOT an expressible EA — `[lower.imm-link]` refuses it. This is the
   SIBLING of t38's `dc.w <label> - <label>` DATA-arm blocker (`c79fa0e`, solved by
   `Cell::RelOffset`): same semantic, different position (an immediate, not a `dc` cell).
   Re-grounded byte-identically as a pure-int const `TEST_ART_LEN = 128+128+512` with a
   rooted `ensure` drift-guarding the blob. A real fix EXTENDS the t38 `RelOffset` feature
   to the immediate path.
2. **Plan §3 deviation (up-chain):** `VDP_HV_COUNTER` as a bare abs-EA (no bare-non-const-
   hardware-register precedent; read from a proc body — see finding 3) re-grounded as a
   local const mirror + drift guard (the VDP_CTRL/VDP_DATA-in-engine.vdp class).
3. **Feature-gap finding:** a comptime `embed(...)` reached through a PROC-BODY immediate
   hits `[sandbox.no-root]` — `lower_module` threads `include_root` to data/`ensure`/`equ`
   sites but NOT `ProcCtx`. Worked around (blob bytes emit from a `data` block, length via
   a rooted `ensure` + pure-int const). Ledgered as a clean feature-gap demand.
4. **Game_Entry equ-off-extern = demand-1 instance #2** (error_handler `ErrorHandler` #1);
   worked around with the gate-aware numeric equ (kill row 90). Instance count now 2 —
   strengthens the link-time-equ-off-external-base capability ask.
5. **Lens A false-positive adjudicated:** the suggested clobbers revert (a0-a3 → a0-a2)
   would re-break the corpus closure — a3 is a genuine transitive clobber; the COMMENT
   was stale, not the declaration.

STOP at checkpoint (b). Gate (c) / rebase / merge are the overseer's.
