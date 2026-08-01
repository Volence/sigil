# Conversion-tail census — the remaining `.asm` in the aeon MAIN tree

Read-only analysis. Goal: 100% `.emp` for the aeon tree (sigil's AS-comprehension
frontend stays as a separate product). Inventory = 50 `.asm` files (find, minus
`.git`/`.worktrees`). Grounded in engine.inc / main.asm gate reads and the
campaign ledger, not guesses.

## How the tail is shaped (the three structural facts that drive the plan)

1. **The P5 constants flip is the proven template.** `engine/constants.asm` lost
   114 `=` to `engine/system/constants.emp` via harvest→inject guarded-defines,
   byte-neutral across all six ROMs (`2026-07-30-stage3-close-packet.md` §1). Any
   AS file that is a *constants/equate holder* flips the same way. Any AS file that
   is a *code twin already gated by `SIGIL_EMP_*`* (z80_init, player_common,
   test_player, test_enemy) flips by making the `.emp` canonical + deleting the
   `.asm` — its `.emp` twin already exists and is proven at its own oracle.

2. **The engine.inc / main.asm resume `org`s are now INERT** (packed placement
   B-0, `2026-07-31-waveb-b0-computed-placement.md`). The chainer derives every
   target's placement from frozen table + live sizes and ignores the ~35 `org`
   lines. They are dead comments awaiting the **residual-split capstone** (ledger
   line 1852, the Stage-2 companion; line 1859 lists "engine.inc org deletion" as
   remaining rows-6/58 work). This means the *skeleton* files (main.asm ×2,
   engine.inc) are not ported — they collapse when the last residual AS data island
   in them moves to `.emp`.

3. **RAM packing (B-0b) is the next §17 unblock** and `ram.asm` is its natural
   pair — RAM sections are still hand-pinned `Pin`s (`pins.rs`), the RAM analog of
   B-0's ROM packing (waveb close packet, "entity_window #1 and tile_cache #2 …
   need RAM-layout growth"). ram.asm's port and B-0b want the same information.

---

## THE CENSUS (50 files)

Effort S/M/L = small/medium/large. "BN" = byte-neutral-provable (fold-identity or
zero-byte equate/offset); "A/B" = needs oracle A/B + re-freeze.

### Engine definitions (no-ROM-output block of engine.inc)

| # | path | lines | what | class | eff | deps / blockers | notes |
|---|---|---|---|---|---|---|---|
| 1 | `engine/constants.asm` | 386 (147 `=`) | residual hardware addrs, VDP access, VRAM layout, art sizes — the half NOT flipped at P5 | OWNERSHIP-FLIP | M | the P5 harvest→inject mechanism (already built) | BN. `constants.emp` already owns 114; this extends the SAME flip to the residual `=`. Some entries (`SYSTEM_STACK`) are integer equs the harvest handles. |
| 2 | `engine/sound_constants.asm` | 1480 (321 defs) | shared 68k/Z80 sound equates (SND_* slots, banks, ids) — **+ 5 Z80 structs + derived Z80 RAM layout + ~40 assert walls** | ~~PORT (constants flip)~~ **CAPSTONE (structs + derived-RAM + flat fused)** | ~~L~~ **CAPSTONE** | P5 + struct-offset harvest exist; but resident-blob mirror = seam1's 399 hardcoded entries + seam2 carriers + 33 68k externs | **✅ DONE 2026-08-01 (sound-constants flip SHIPPED, spec `2026-08-01-sound-constants-flip-design.md`).** E1 (`2026-08-01-sound-e1-flip.md`): `sound_constants.asm` DELETED, `engine/sound/sound_constants.emp` sole author, 4 consumers `use`. E2 (`2026-08-01-sound-e2-mirror.md`): seam1's 399 hardcoded values + seam2's DAC carriers DISSOLVED — every resident-module `-D` value now flows from the authority via `eval_all_pub_consts`; `dac_sample_tab.emp`'s 2 externs → `use`. 15 Z80 blobs + 6 ROMs byte-identical; 2868/0/4. |
| 3 | `engine/structs.asm` | 258 | `struct…endstruct` generating SST/Act/Sec/DMAEntry/parallax_config/EntityScanState field-offset equs + VDP_Shadow_len | OWNERSHIP-FLIP | M | needs a STRUCT-OFFSET harvester (sibling of `eval_all_pub_consts`) | BN. Close-packet §4 "the structs flip (rows 7/8/11/15/25)". Retires the VDP_Shadow_len bridge. Its `.emp` overlays (sst.emp, engine.structs, act_descriptor.emp) already mirror with drift guards. |
| 4 | `engine/macros.asm` | 367 | AS `function`/`macro` comptime helpers (vdpComm, vdpReg, vram_art, vram_bytes, sprSize, clearRAM, DMA macros) | RESIDUAL-SKELETON (comptime-helper carrier) | M | its `.emp` twins (engine.vdp comptime fns) largely exist; dies when its last residual-AS consumer moves | BN. Still needed while residual AS data emits (boot_data's vdpComm, demo_data's vram_art, mappings' sprSize). Port the helpers into a shared `.emp` comptime module opportunistically; delete the `.asm` with the residual-split capstone. |
| 5 | `engine/ram.asm` | 524 (1 `=`, 201 `ds`) | engine RAM LAYOUT (`Name: ds.b N` runs, conditional debug fields, buffer-reuse overlays) | PORT (`vars` layout) | L | **B-0b RAM packing** (pins→computed); the `vars` construct; comptime reads served by injected defines | ✅ **DONE (item #7b, `engine/ram.emp`)** — the region form: two `region`s + two `pub vars`; the three `if/error` guards → region `limit`s + `w_addressable`; buffer-reuse → `alias()`+`ensure`; the two `ifdef __DEBUG__` → `if DEBUG == 1` groups. `.asm` deleted. Six-target byte-identical; repin zero-diff. |
| 6 | `engine/debug/debugger.asm` | 806 | vendored MD Debugger v2.6 (Vladikcomper) — assert/RaiseError/KDebug MACRO + definitions layer (no ROM output) | PORT (vendored macro layer) | L | `.emp` equivalents of the assert/KDebug macro semantics; the error_handler BLOB is already `error_handler.emp` | BN (definitions, zero bytes). **OPEN QUESTION: vendored third-party — port or keep-as-is?** Low priority; consumed by both `.asm` and `.emp` assert expansions. |

### Engine generated / derived carriers

| # | path | lines | what | class | eff | deps / blockers | notes |
|---|---|---|---|---|---|---|---|
| 7 | `engine/debug/mddbg_symbols.asm` | 59 | `MDDBG__* = ErrorHandler+off` equ table, zero bytes | OWNERSHIP-FLIP (derived-equate holder) | S | the "equ off link-external base" capability (already live — engine.inc:409 uses it for ErrorHandler alias) | BN. Debug-only reads. Straight flip to a `.emp` equate module. |
| 8 | `engine/debug/generated/vectors.asm` | 23 | golden compression self-test vectors, DEBUG-only | GENERATED | S | generator = `tools/gen_compression_vectors.py` | Emit `.emp` from the generator, or fold into the residual-split. Consumed cross-seam by compression_selftest.emp. |
| 9 | `engine/sound/generated/mt_syms.asm` | 4 | `SongTable`/`SongPatchTable` equs | GENERATED (sigil emit) | S | generator = `sigil emit_sound_blob` (seam-2 mt_bank) | ALREADY a sigil output. Convert = emit `.emp` syms (or fold the two labels via link). |
| 10 | `engine/sound/generated/mt_syms_debug.asm` | 4 | debug-shape of #9 | GENERATED (sigil emit) | S | same | same |
| 11 | `engine/sound/generated/z80_sound_syms.asm` | 59 | resident-blob exported-symbol contract (per shape) | GENERATED (sigil emit) | S | generator = `sigil emit_sound_blob` | ALREADY a sigil output. Same emit-`.emp` decision. |

### Engine system

| # | path | lines | what | class | eff | deps / blockers | notes |
|---|---|---|---|---|---|---|---|
| 12 | `engine/system/boot_data.asm` | 149 | `BootData` (a5)+ cursor table (movem preload, VDP reg bytes, DMA fill, PSG silence, post-DMA cmds) + z80_init include + layout-assert wall | PORT (data table) | M | `boot.emp` already reads BootData; the assert wall → `.emp` ensures; the z80_init include is #13 | BN. Data-DSL demand (ledger 1526, row-46 "boot's cursor protocol"). No `boot_data.emp` yet. |
| 13 | `engine/system/z80_init.asm` | 38 | Z80 idle program (no-sound builds) | OWNERSHIP-FLIP | S | twin `z80_init.emp` EXISTS, gated `SIGIL_EMP_Z80_INIT` | BN (both shipped shapes have sound → dead in canonical). Flip = make `.emp` unconditional, delete `.asm`. Oracle `z80_init_port` is its region proof. |

### games/demo (the "start here" template — a separate ROM target with its own goldens)

| # | path | lines | what | class | eff | deps / blockers | notes |
|---|---|---|---|---|---|---|---|
| 14 | `games/demo/config/constants.asm` | 14 | demo game constants + capacity contracts | PORT | S | reverse-seam (link-position) | BN vs demo/demod goldens. |
| 15 | `games/demo/config/game.asm` | 30 | demo game contract (header strings, hooks) | PORT | S | header string contract | BN. |
| 16 | `games/demo/config/ram.asm` | 9 | demo game RAM (empty stub) | PORT (`vars`) | S | `vars` construct | ✅ **DONE (item #7c, `games/demo/config/ram.emp`)** — `region game_ram @ after(upper_ram) .. SYSTEM_STACK, w_addressable` + an empty `pub vars` producing `Game_RAM_End`. `.asm` deleted; `gameRamIncludes` emptied. Six-target byte-identical. |
| 17 | `games/demo/data/demo_data.asm` | 39 | objdef/mapping/art/palette | PORT (data) | S | vram_art/sprSize helpers; `objdef` construct | BN. |
| 18 | `games/demo/demo_state.asm` | 36 | `GameState_Demo_Init` code | PORT (code) | S | DMA/object API externs | BN. |
| 19 | `games/demo/main.asm` | 43 | demo game manifest (include-order macros) | RESIDUAL-SKELETON | S | collapses when 14–20 are all `.emp` | The thin manifest; may survive as the minimal template or die with residual-split. |
| 20 | `games/demo/objects/demo_box.asm` | 3 | `DemoBox_Main: jmp Draw_Sprite` | PORT (code) | S | — | BN. Trivial. |

### games/sonic4/config

| # | path | lines | what | class | eff | deps / blockers | notes |
|---|---|---|---|---|---|---|---|
| 21 | `games/sonic4/config/constants.asm` | 86 | game tuning, PSTATE_* ids, test scaffold consts | PORT (P6, rows 54/62/65) | M | reverse-seam; typed ids to language round | ✅ **DONE (conv-f #21, 2026-08-01, `2026-08-01-conv-f-game-config.md`).** `constants.asm` DELETED → `games/sonic4/config/constants.emp` (`games.sonic4.constants`), the sole authority. Census "no guarded-define channel" premise CORRECTED: the game-SIDE `.emp` (player_*, sonic_anims, test_*) folds through `use` (reverse-seam holds), but the game-AGNOSTIC engine `.emp` (rings/entity_window/ram drift guards) DID need the per-game harvest (`harvest_game_constants`, the #7c precedent) to link-export the pub consts. 3 game-VARYING `-D` interface consts (MAX_RING_BUFFER / VRAM_RING_PLACEHOLDER / COLLECTED_WINDOW_SLOTS) stay native.rs `emp_defines` (codebase forbids declaring a `-D` name in `.emp`). Six-target byte-identical; strict 2868→2867 (−1 retired probe). |
| 22 | `games/sonic4/config/game.asm` | 83 | game contract (header strings, sound contract, hooks) | PORT (game contract) | M | Game_Entry already `.emp`-resolved; header contract | **NAMED REMAINDER (conv-f #22 scoped, conv-f2 shipped its one flip).** `SFXID_REV_LOOP = SFXID_SPINDASH` flipped to `config/sound_ids.emp` at F2. The rest PARKS (AS): gameBootHook/gameDebugTick (defined-not-invoked, game_loop lockstep row 9), header strings (`gameHeader` `dc.b` data), `Game_Entry`/`GAME_ENTRY_ID` (cross-seam label equalate; GAME_ENTRY_ID reads a `.emp` game const via harvest), `GAME_CAMERA_JUMP_LOCK` (a `-D`). BN. |
| 23 | `games/sonic4/config/ram.asm` | 60 | game RAM continuation (`phase Engine_RAM_End`) | PORT (`vars`) | M | **pairs with #5 ram.asm + B-0b** | ✅ **DONE (item #7c, `games/sonic4/config/ram.emp`)** — the cross-module chain `region game_ram @ after(upper_ram)` onto engine `upper_ram`; player/phys/history-ring fields; the `ifdef __DEBUG__` counters → `if DEBUG == 1 @shape_divergent`; `align 256` → `@align(256)` (AS in-phase align semantics). `.asm` deleted. Six-target byte-identical; repin zero-diff. |
| 24 | `games/sonic4/config/sound_ids.asm` | 94 | SONG_*/SFXID_* numeric ids + SFX priority ladder | PORT (P6, rows 62/65) | M | typed `SFXID_RING_*` STAY DEFERRED to language round (typed-extern grammar) | ✅ **DONE (conv-f2 #24, 2026-08-01, `2026-08-01-conv-f2-sound-ids.md`).** `sound_ids.asm` DELETED. Clean subset (SONG_*/SFXID_*/SFXPRI_* + SFXID_REV_LOOP from game.asm) → `config/sound_ids.emp` (`games.sonic4.sound_ids`), the #21 clone. SFX-bank counts (SFX_ID_BASE/COUNT/TABLE_LEN) DISSOLVED into `sfx_bank.emp` (their derived authority; the ruling's mechanical path — eval-on-a-data-module proven, quadruple mirror → one derivation). Typed `SFXID_RING_*`/`SONG_*: SfxId/SongId` mirrors STAY (pre-ruled newtype deferral). Six-target byte-identical + 15 blobs identical; strict 2867→2866 (−1 retired probe). |

### games/sonic4/data/editor (PARKED exports — NOT in the build)

| # | path | lines | what | class | eff | deps / blockers | notes |
|---|---|---|---|---|---|---|---|
| 25 | `data/editor/ojz/act1/export/act_descriptor.asm` | 217 | editor export (act descriptor) | GENERATED (editor) | M | **PARKED** — main.asm includes the `data/levels/` + `data/generated/` copies, NOT these | Ledger: "parked editor export whose object was never wired in (plantbadmaps class)." Conversion target = editor emits `.emp`. Confirm still wanted before spending effort; possible delete-candidate. |
| 26 | `data/editor/ojz/act1/export/entity_data.asm` | 159 | editor export (entity data) | GENERATED (editor) | S | PARKED | same |
| 27 | `data/editor/ojz/act1/export/vram_bases.asm` | 8 | editor export (VRAM base equates) | GENERATED (editor) | S | PARKED | same |

### games/sonic4/data/generated (auto-generated; mostly gitignored, some committed)

| # | path | lines | what | class | eff | deps / blockers | notes |
|---|---|---|---|---|---|---|---|
| 28 | `data/generated/ojz/act1/bg_anim.asm` | 4 | auto-gen "no BG animation" stub | GENERATED | S | generator = `tools/regenerate-level.sh` | Emit `.emp` from generator. |
| 29 | `data/generated/ojz/act1/entity_data.asm` | 265 | auto-gen X-sorted ring lists + object placements + per-section type tables (BUILT, main.asm:147) | GENERATED | M | generator = `tools/ojz_entity_gen.py` | The real conversion is `ojz_entity_gen.py` emitting `.emp`. Ledger: "generator-emits-.emp mechanics." |
| 30 | `data/generated/ojz/act1/ojz_act_pool.asm` | 14 | auto-gen art-pool page BINCLUDEs + address table | GENERATED | S | generator = `tools/regenerate-level.sh` / `ojz_strip_gen.py` | BINCLUDE wrapper. |
| 31 | `data/generated/ojz/act1/ojz_act_pool_manifest.asm` | 9 | auto-gen pool manifest equates | GENERATED | S | same | |
| 32 | `data/generated/ojz/act1/sec_block_blobs.asm` | 27 | auto-gen per-section block-blob BINCLUDEs (content-dedup) | GENERATED | S | generator = `tools/ojz_block_gen.py` | |
| 33 | `data/generated/ojz/act1/sec_block_dicts.asm` | 11 | auto-gen block-dictionary length equates | GENERATED | S | same | |

### games/sonic4/data/levels (committed, portable, BUILT)

| # | path | lines | what | class | eff | deps / blockers | notes |
|---|---|---|---|---|---|---|---|
| 34 | `data/levels/ojz/act1/act_descriptor.asm` | 268 | level descriptor + section grid tables (uses `Act`/`Sec` struct; `include`s the generated pool/dict/blob files) | PORT / DATA-DSL | M | **structs flip (#3)** for the struct twins; `[Act;N]`/`[Sec;N]` struct-array DSL; the generated includes (#30–33) | BN. `act_descriptor.emp` already exists partially (P5 consumer of SECTION_SIZE_SHIFT/EDGE_CLAMP). The struct-array form is the DATA-DSL end-state (ledger 1621 panel B1). |

### games/sonic4/data/mappings

| # | path | lines | what | class | eff | deps / blockers | notes |
|---|---|---|---|---|---|---|---|
| 35 | `data/mappings/test_mappings.asm` | 40 | test sprite mappings (frame header + piece records, word offset table) | DATA-DSL | S | offset-table + piece-record DSL; sprSize helper | BN. Offset-table roadmap #1 candidate (memory `emp-data-table-dsl-candidates`). |

### games/sonic4/data/parallax (config DSL — depends on parallax_macros.inc macro layer)

| # | path | lines | what | class | eff | deps / blockers | notes |
|---|---|---|---|---|---|---|---|
| 36 | `data/parallax/effects/haze.asm` | 119 | reusable parallax effect (deform_table + haze_fg/haze_bg macros + ParallaxConfig record) | DATA-DSL | M | `parallax_macros.inc` macro layer → `.emp` (deform_table_sine, parallax_config, band macros); `parallax_config` struct twin (#3) | BN. |
| 37 | `data/parallax/effects/perspective.asm` | 72 | parallax effect | DATA-DSL | S | same | BN. |
| 38 | `data/parallax/effects/rocking.asm` | 63 | parallax effect | DATA-DSL | S | same | BN. |
| 39 | `data/parallax/effects/shimmer.asm` | 101 | parallax effect | DATA-DSL | S | same | BN. |
| 40 | `data/parallax/ojz_default.asm` | 58 | default OJZ config (5-band table + ParallaxConfig, DeformTable_Zero) | DATA-DSL | S | same; DeformTable_Zero is referenced by effects | BN. Include-order sensitive (must precede effects). |
| 41 | `data/parallax/ojz_windy.asm` | 17 | windy config | DATA-DSL | S | same | BN. |
| 42 | `data/parallax/scenes/caves.asm` | 28 | composite scene config | DATA-DSL | S | same | BN. |
| 43 | `data/parallax/scenes/locked_clouds.asm` | 35 | composite scene config | DATA-DSL | S | same | BN. |
| 44 | `data/parallax/scenes/sky_haze.asm` | 21 | composite scene config | DATA-DSL | S | same | BN. |
| 45 | `data/parallax/scenes/windy_haze.asm` | 46 | composite scene config | DATA-DSL | S | same | BN. |

### games/sonic4/data/sprites

| # | path | lines | what | class | eff | deps / blockers | notes |
|---|---|---|---|---|---|---|---|
| 46 | `data/sprites/pitcher_plant/anims.asm` | 6 | `Ani_PitcherPlant` table | DATA-DSL (anim table) | S | **PARKED** — pitcher_plant object never wired in (ledger line ~459) | Convert with the anim-table DSL, or delete if the object stays unwired. Confirm intent. |

### games/sonic4/objects + player (gated code twins — `.emp` twin exists)

| # | path | lines | what | class | eff | deps / blockers | notes |
|---|---|---|---|---|---|---|---|
| 47 | `games/sonic4/objects/test_enemy.asm` | 76 | `TEnemyV` struct header + ENEMY_PATROL_SPEED + gated code | OWNERSHIP-FLIP | S | twin `test_enemy.emp` EXISTS (`SIGIL_EMP_TEST_ENEMY`); TEnemyV struct → structs flip; ENEMY_PATROL_SPEED read by test_objects.emp guard | BN. Header carrier + gated residual. |
| 48 | `games/sonic4/objects/test_player.asm` | 307 | `DplcV` struct header + `_dplc_ptr`/`_art_base` equates + gated code | OWNERSHIP-FLIP | M | twin `test_player.emp` EXISTS (`SIGIL_EMP_TEST_PLAYER`); DplcV struct read by test_animated.emp drift guards → structs flip | BN. Header carrier; the DplcV struct must move to `.emp` before the guard can. |
| 49 | `games/sonic4/player/player_common.asm` | 790 | `PlayerV` struct + overlay equates + macros (header) + gated frame code (Player_Main/Display/RefreshPhysics/States) | OWNERSHIP-FLIP | M | twin `player_common.emp` EXISTS (`SIGIL_EMP_PLAYER_COMMON`); PlayerV struct → structs flip; **the 3 player-state offset tables** (Player_States/EnterHooks/ExitHooks) → offsets-DSL dedicated parcel (ledger 1767, cross-module Ref, "dedicated parcel or Spec 5") | BN. |

### The skeletons (residual-split capstone material)

| # | path | lines | what | class | eff | deps / blockers | notes |
|---|---|---|---|---|---|---|---|
| 50 | `games/sonic4/main.asm` | 343 | game manifest: include-order macros, ~15 INERT resume `org`s, the collision-data BINCLUDE island (HeightMaps/AngleTable/SolidityTable/Map_Sonic/DPLC_Sonic/Art_Sonic), the sound-data BINCLUDE island (DAC/MT/SFX) | RESIDUAL-SKELETON | L (capstone) | the residual-split companion (ledger 1852) + all data islands moved to `.emp` | **CAPSTONE-SIZED.** The orgs are inert; the BINCLUDE islands are data. Dies via the ledgered residual-split. `engine/engine.inc` (`.inc`, not in the 50) is the engine half of the same capstone. |

---

## SEQUENCED PLAN

Ordering by (1) dependency, (2) §17 Wave-B synergy, (3) risk. All parcels except
the two flagged are **byte-neutral-provable** (fold-identity ×6 or zero-byte
equates/offsets); none need oracle A/B *for the port itself* — the gated code
twins (13, 47, 48, 49) already passed their oracle proofs when the `.emp` was
written. Re-freeze is only needed where a growth actually shifts placement, which
none of these ports introduce (they are ownership moves at unchanged bytes).

### Parcel A — the structs flip (unblocks the most) · #3
The single highest-leverage dependency: structs.asm feeds the struct twins that
Parcels D, E, G, H all need. Build the struct-offset harvester (sibling of
`eval_all_pub_consts`), delete structs.asm, retire the per-field drift walls and
the VDP_Shadow_len bridge. **BN** (offsets unchanged, close-packet §4 template).
Effort M. → files: **3**.

### Parcel B — the engine residual-constants tail · #1
Extend the P5 constants flip to the residual 147 `=` in constants.asm (mechanism
already built). **BN.** Effort M. → files: **1**. Can run parallel to A.

### Parcel C — RAM packing (B-0b) + the RAM ports · #5, #23  [§17 SYNERGY]
Build B-0b (RAM analog of B-0 packing, same fold-identity bar) FIRST — it is
already the named prerequisite for Wave-B increment 2 (entity_window #1 /
tile_cache #2). Then port `engine/ram.asm` + `games/sonic4/config/ram.asm` +
`games/demo/config/ram.asm` (#16) to the `vars` construct on the computed RAM
layout. **BN** (fold-identity). Effort L. → files: **5, 23, 16**. *This is the
parcel that rides the active arc — sequence it right behind A/B so B-0b lands
before entity_window/tile_cache need it.*

> **PARCEL C OUTCOME (2026-08-01, `2026-08-01-conv-c-ram-ports.md`):** SPLIT.
> **Half (a) B-0b = DONE** (2026-08-01 B-0b note: RAM's B-0 analog is AS
> `phase`-from-symbol, already live; `ram_packing_invariants_{plain,debug}` guards
> committed). **Half (b) the three ram.asm ports = BLOCKED, files 5/23/16 PARKED.**
> The census premise ("port to the `vars` construct") does not hold: the region-form
> `vars` (`vars upper_ram { … }`) **parses then is INERT** — zero bytes, no address
> allocation, no labels, none of the §4.6 checks — by a RECORDED decision (item-#7
> OUT-list, "recorded so nobody creeps",
> `specs/2026-07-07-spec2-plan7-item6-overlay-dispatch-design.md:105`; three inert
> lowering sites `lower/mod.rs:394/583`, `eval/mod.rs:661`). Building the port is a
> capstone-scale LANGUAGE FEATURE (9 pieces: region map-file base authority,
> region-allocation lowering, reserve-`@align`, conditional `vars` fields
> [gap-ledger:153 "the port BLOCKS on this"], cross-region base chaining `phase
> Engine_RAM_End`, buffer-reuse overlay, the reserved compiler checks, the
> layout-stability lint, the repin/port-gate ripple) — NOT effort-L, NOT a mechanical
> port, and NO completable subset exists (even the 9-line demo stub #16 needs region
> allocation for its `Game_RAM_End` label). Effort reclass: **L → CAPSTONE**.
> Overseer decision at the packet §10: (1) build item #7 as its own spec+plan+parcel,
> (2) accept AS-authored RAM as the standing mechanism (nothing is broken — the
> honest "100% .emp" exception, like the vendored debugger), or (3) byte-neutral
> pre-port hygiene (move both `ifdef __DEBUG__` blocks to region tails) now,
> feature later.

> **PARCEL C RESOLVED — path (1) taken, ALL THREE ports DONE (2026-08-01, item #7).**
> The overseer/Volence ruled path (1): build the region-`vars` feature as its own
> spec (`specs/2026-08-01-item7-ram-regions-design.md`) + three parcels, then port.
> **#7a** built the region form (allocation lowering, `@align`, conditional groups,
> the diagnostics); **#7b** ported `engine/ram.asm` → `engine/ram.emp`
> (`2026-08-01-item7b-engine-ram-port.md`); **#7c** ported both game RAM files
> (`2026-08-01-item7c-game-ram-ports.md`), adding cross-module `after(..)` resolution
> + the game-RAM harvest extension. **Rows 5/23/16 all ✅ DONE.** All six targets
> byte-identical to chain-9; repin RAM-cell zero-diff both shapes. The "capstone
> LANGUAGE FEATURE" is shipped. (A spec-vs-reality catch closed en route: region
> `@align` had to adopt AS's IN-PHASE align semantics `round_up(cursor + n, n)` — the
> asl 1.42 quirk that places `Player_Pos_Ring` at `$FFFFB500`, not `$FFFFB400` — for
> byte-identity; regions are RAM-only so the phased regime always applies.)

### Parcel D — the gated code twins (make `.emp` canonical, delete `.asm`) · #13, #47, #48, #49
Needs Parcel A (their struct headers move with the structs flip). Flip
z80_init, test_enemy, test_player, player_common: drop the `SIGIL_EMP_*` gate,
make the `.emp` unconditional, delete the `.asm` header+twin. player_common
carries a sub-dependency: the 3 player-state **offset tables** want the offsets
DSL cross-module Ref path (ledger 1767) — either adopt in this parcel (feature-
scale) or keep the current `extern-difference` form (byte-identical) and defer
the DSL adoption. **BN.** Effort M. → files: **13, 47, 48, 49**.

### Parcel E — the sound-constants flip · #2  (row-59)
Independent of A–D; the P5 mechanism serves it. Harvest `sound_constants.emp`,
inject, retire the 5-consumer mirrors. Dissolves the SND_* comptime-source
circularity (ledger 1619). **BN.** Effort L. → files: **2**.

> **PARCEL E OUTCOME (2026-08-01, `2026-08-01-conv-e-sound-constants.md`):
> INSPECTION STOP — reclass L → CAPSTONE.** Three premise corrections, numbers-
> grounded: (1) the file is NOT a flat-equate holder — 1481 ln = 321 `=` (many
> struct-derived aliases) + **5 Z80 structs** (DacSample/FmPatch/SfxHeader/SfxChannel/
> SeqChannel, with a 13-field SeqChannel↔SfxChannel shared-prefix invariant) + a
> **derived Z80 RAM layout** (~40 addrs mixing flat bases × `sizeof(struct)`) + ~40
> `error`/`fatal` walls + 5 comptime fns; it is the structs-flip + derived-RAM +
> flat-equate classes FUSED, not "constants flip" (Parcel A deliberately left the 5
> SOUND structs here). (2) "5-consumer mirrors" understates a TRIPLE unguarded mirror:
> `seam1.rs` **399 hardcoded `(name,value)` entries** across `driver/sequencer/sfx/fm/
> psg_consts` (fed to the resident Z80 blob build) + seam2 pinned DAC-head carriers +
> 33 68k-`.emp` link-externs (sound_api 24 / sound_debug 7 / dac_sample_tab 2); the AS
> residual 68k `.asm` reads it ~0×. (3) the cited circularity **ledger 1619 is already
> SETTLED** (ledger line 1877, 2026-07-30) — a different file (`sound_tables_z80`); the
> still-live hazard is the seam1 399-entry mirror. The mechanism EXISTS (harvest +
> struct-offset + `sizeof`-in-eval all proven — NOT a missing-feature blocker), but the
> WORK is the **item-#7 (RAM-regions) shape**: new `sound_constants.emp` with the first
> Z80 struct-offset + derived-RAM-from-`sizeof` composition, a `harvest_sound_constants`
> extension feeding BOTH the AS residual AND the resident Z80 blob (retiring all 5 seam1
> tables + seam2 carriers), the 33 externs → `use`, `.asm` deleted, six-ROM **+ Z80-blob**
> byte-identity. No clean sub-seam (derived RAM mixes flat + sizeof; seam1 tables
> interleave all classes). RULE NEEDED: build as its own spec + 2–3 parcels (path 1),
> or accept AS-authored `sound_constants.asm` as the standing "100% .emp" exception
> (path 2, like #6 vendored debugger). No code changed; branches clean.

### Parcel F — the game-config / P6 module · #21, #22, #24
The untyped half folds through the reverse-seam (link-position) — likely no
guarded-define channel needed (close-packet §4). Port sonic4 config constants,
game contract, sound ids (untyped half). Typed `SFXID_RING_*` STAY DEFERRED to
the language round. **BN.** Effort M. → files: **21, 22, 24**.

> **✅ PARCEL F COMPLETE — #21 (conv-f), #24 + #22's one flip (conv-f2, `2026-08-01-conv-f2-sound-ids.md`).**
> **#24 sound_ids.asm — FLIPPED + DELETED (conv-f2).** The clean subset (SFXID_*/SONG_*/
> SFXPRI_* + SFXID_REV_LOOP from game.asm) → `config/sound_ids.emp` (`games.sonic4.sound_ids`),
> the #21 clone. Premise correction on remainder (b): the SFX_ID_BASE family is NOT entangled
> beyond the mechanical shape — a probe proved `eval_all_pub_consts(sfx_bank.emp)` resolves
> `SfxTable.min_key/.count/.len` standalone (0 errors), so per the overseer's ruling the family
> DISSOLVED into `sfx_bank.emp` (their derived authority): the quadruple mirror (sound_ids.asm +
> seam1 + seam2 + sfx_bank derivation) → the one derivation. Remainder (a) typed `SFXID_RING_*`
> stayed deferred as ruled. `#22 game.asm`'s one flip (`SFXID_REV_LOOP`) shipped with #24; the
> rest is the named remainder below. Six-target byte-identical + 15 blobs; strict 2867→2866.
>
> **PARCEL F #21 DONE; #22/#24 SCOPED (2026-08-01, `2026-08-01-conv-f-game-config.md`).**
> **#21 constants.asm — FLIPPED + verified** (see the row-21 note above): six-target
> byte-identical, strict 2867/0/4. Premise correction: the census "no guarded-define
> channel needed" is HALF-right — game-side consumers `use` the new
> `games.sonic4.constants` (reverse-seam holds), but the game-agnostic engine's extern
> drift guards needed the per-game harvest (`harvest_game_constants`), and 3
> game-VARYING `-D` interface constants stay native.rs `emp_defines` (a `-D` name
> can never be declared in `.emp`). Effort was **L, not M** (new `.emp` module + harvest
> fn + ~18 aeon consumer edits + heavy sigil test ripple: 7 port tests + m1c root).
>
> **#24 sound_ids.asm — SCOPED, NOT YET FLIPPED.** SPLIT verdict: `SFXID_*` (untyped) /
> `SONG_*` (`if DEBUG`) / `SFXPRI_*` flip cleanly to a `games.sonic4.sound_ids` `.emp`
> the SAME way #21 did (consumers: sound_api/sound_sfx/player_ground/player_spindash/
> game_debug/mt_bank — most via drift-guard mirrors + the harvest). **REMAINDERS:** (a)
> typed `SFXID_RING_*` PRE-RULED deferred (SfxId newtype, language round) — sound_api.emp
> keeps its typed mirror; (b) `SFX_ID_BASE`/`SFX_COUNT`/`SFX_TABLE_LEN` are read at
> AS-time by `engine/sound/sound_bank.inc` (`if …<> SFX_TABLE_LEN*2 fatal`) AND are
> UNCHECKED MIRRORS of `sfx_bank.emp` + hardcoded in `seam1.rs`/`seam2.rs` — a
> triple-mirror entangled with the sound seam (Parcel-E-adjacent); flip only after
> deciding the sfx_bank.emp/seam authority, else park with a reason. The clean subset
> (SFXID_*/SONG_*/SFXPRI_*) is a direct #21 clone.
>
> **#22 game.asm — SCOPED, mostly a documented REMAINDER.** Empirically: gameBootHook /
> gameDebugTick macros are DEFINED-but-never-INVOKED in the residual (boot.emp /
> game_loop.emp expand the `.emp` mirrors) — so they force no AS reads, but their bodies
> are the game_loop combo-matrix LOCKSTEP reference (kill row 9) → PARK (AS). Header
> strings (GAME_CONSOLE…) are read by the invoked `gameHeader` (header.inc) → PARK (AS
> ROM-header data). `Game_Entry = GameState_OJZScroll_Init` is a cross-seam label
> equalate (main.asm `move.l #Game_Entry`) → PARK. `GAME_CAMERA_JUMP_LOCK` is a `-D`
> (emp_defines) → cannot move to `.emp`. `GAME_ENTRY_ID = GS_OJZ_SCROLL_TEST` reads a
> now-`.emp` game const via the harvest → stays AS. Only flip candidate: `SFXID_REV_LOOP
> = SFXID_SPINDASH` (a sound const → belongs with #24). So game.asm ships as a named
> remainder (header/macros/Game_Entry/-D gates), NOT a flip.

### Parcel G — the parallax config DSL · #36–45 (+ the macro layer)  [DONE 2026-08-01, conv-g branch — see notes/2026-08-01-conv-g-parallax.md]
> **DONE (unmerged).** `parallax_macros.inc` → `engine/level/parallax_dsl.emp`
> (comptime helper: FACTOR encoding + deform generators); the 10 config files →
> `games/sonic4/data/parallax/configs.emp` (native, `PARALLAX_CONFIGS` pin). Six
> targets byte-identical; strict 2868/0/4. Design gate: existing surfaces sufficed
> (Appendix A ratified), NO new grammar — nested `ParallaxCfgN` wrapper struct is the
> lowering-reality shape (relocations flow through the data-item type, so the spec's
> `Struct ++ Array` sketch became a typed nested struct). #36–45 + macro layer all retired.

Sequence AFTER Wave C (row-35 parallax hardening, ledger 1825) so the port lands
on the hardened engine, not the harness-force-write shape. Express
`parallax_macros.inc` (deform_table_sine, parallax_config, band macros) as `.emp`
comptime + a parallax-config construct (needs the `parallax_config` struct twin
from Parcel A), then port the 10 config files. **BN.** Effort M–L. → files:
**36, 37, 38, 39, 40, 41, 42, 43, 44, 45**.

### Parcel H — the remaining game data · #12, #17, #18, #20, #34, #35, #46
Boot data table (#12, needs the data-DSL cursor construct or a straight `.emp`
data section), the demo data/state/box (#17, #18, #20), the level descriptor
(#34, struct-array DSL — needs Parcel A), test mappings (#35, offset-table DSL),
pitcher_plant anims (#46, or delete). **BN.** Effort M. Groups naturally with F
for the demo files (14, 15 config go here too).

### Parcel I — the generators emit `.emp` · #8, #28–33, #29 + the sigil-emitted syms #9–11
The conversion target is the GENERATOR, not the `.asm`. Two sub-tracks:
- **sigil-emitted syms** (9, 10, 11): `sigil emit_sound_blob` already owns these
  — decide emit-`.emp`-syms vs fold-via-link. Cheap, do alongside E.
- **Python generators** (8 via gen_compression_vectors.py; 28/30/31 via
  regenerate-level.sh/ojz_strip_gen.py; 29 via ojz_entity_gen.py; 32/33 via
  ojz_block_gen.py): each generator emits `.emp` instead of `.asm`. Effort S–M
  each; mechanical once one is done.
- **mddbg_symbols** (7): straight equate-flip using the link-external-base
  capability. **BN.** → files: **7, 8, 9, 10, 11, 28, 29, 30, 31, 32, 33**.

### Parcel J — the parked editor exports · #25, #26, #27, (#46)  [DECISION FIRST]
NOT in the build. Either wire the editor to emit `.emp` or delete as stale.
**Ask Volence** before spending effort (see open questions). → files: **25, 26, 27**.

### Parcel K (CAPSTONE) — the residual-split + skeleton deletion · #50 + engine.inc, #19, #4
The last parcel. Once every data island above has moved to `.emp`, the residual
AS collapses: split the residual at declared boundaries, migrate placement
pins→map, declare + `emit_rom`-enforce the object-bank region + section ordering,
compute the resume `org`s from placement (kills rows 6/58). Deletes main.asm ×2
skeletons (#50, #19), engine.inc's inert orgs, and macros.asm (#4, its last
consumer gone). **This is the byte-neutral-but-large capstone** already ledgered
(line 1852, Stage-2 companion; line 1859 lists it as remaining). Effort L
(capstone). → files: **50, 19, 4** (+ engine.inc).

### Sequencing summary
```
A structs flip ─┬─> D gated twins (needs A)
                ├─> G parallax  (needs A + Wave C)
B const tail    ├─> H game data (needs A for #34)
                └─> Parcel A also unblocks E/F only loosely
C RAM (B-0b) ── rides the ACTIVE arc (Wave-B inc 2) — do right after A/B
E sound consts  (independent)
F game config   (independent)
I generators    (independent, cheap, do opportunistically)
J editor parked (DECISION gate)
K CAPSTONE      (LAST — needs every data island moved)
```

### Byte-neutral vs A/B ceremony
- **Every port parcel here is byte-neutral-provable** (fold-identity ×6, or
  zero-byte equates/offsets). None *introduce* a byte change.
- **Re-freeze / A/B is needed only if a port is bundled with an optimization**
  (e.g. adopting the offsets DSL in player_common if it changed lowering — it
  does not; `extern-difference` is byte-identical). Keep ports pure; let the §17
  opt-sweep own the byte-changers.
- **The two large/risky items are placement-mechanism work, not ports:** B-0b
  (Parcel C prerequisite) and the residual-split capstone (Parcel K). Both have
  the fold-identity bar as their proof, both already ledgered.

### Capstone-sized flags
- **Parcel K** (residual-split + skeleton deletion + engine.inc) — explicitly the
  rows-6/58 capstone remainder (ledger 1852/1859).
- **Parcel C's B-0b** — partial-capstone (RAM packing), but it is *pulled forward*
  because the active arc needs it; scoped by the waveb close packet.
- **Parcel G's macro layer** — the parallax config construct is a language-feature
  build, not a mechanical port (medium-capstone within G).

---

## OPEN QUESTIONS FOR VOLENCE

1. **Vendored debugger (#6, engine/debug/debugger.asm, 806 lines).** It is
   upstream third-party (Vladikcomper MD Debugger v2.6) — a macro/definitions
   layer, zero ROM output. Port to `.emp` (large, and diverges from upstream) or
   keep vendored `.asm` as an accepted exception to "100% .emp"? Recommendation:
   keep-as-vendored unless you want the assert/KDebug macros in `.emp`.

2. **Parked editor exports (#25/#26/#27) and pitcher_plant anims (#46).** These
   are NOT in the build (main.asm includes the `data/levels/` + `data/generated/`
   copies instead; pitcher_plant's object was never wired). Delete as stale, or
   wire the editor/object and convert? Recommendation: delete #25–27 (the editor
   is the source of truth); rule on #46 with whether pitcher_plant ships.

3. **Generator outputs (#8–11, #28–33) — emit `.emp` or fold at link?** The sigil-
   emitted syms (#9–11) could stop being `.asm` files entirely if the labels fold
   through the link (like ErrorHandler does). The Python generators (#28–33) are
   "generator-emits-.emp" mechanics. Confirm you want the generators changed vs.
   the files kept as a tolerated generated-artifact class (they never drift by
   hand). This is the "generated-file ownership class" the ledger (1619) already
   flagged as a toolchain decision.

4. **The offsets-DSL adoption in player_common (#49).** The 3 player-state offset
   tables can adopt the `offsets` construct (cross-module Ref path) during Parcel
   D, or stay in the byte-identical `extern-difference` form and defer the DSL to
   its dedicated parcel / Spec 5 (ledger 1767 new kill). Recommendation: keep the
   byte-identical form in the port, let the DSL parcel own the adoption — no
   correctness gap in the interim.

5. **Does "100% .emp" include the demo tree as a deliverable, or is it a template?**
   The demo (#14–20) is the "engine-boots-without-Sonic" template. Converting it
   proves the new-game `.emp` path end-to-end (worth doing early, low risk, its
   own goldens). Confirm it's in scope vs. left as an AS "start here" reference.

## VOLENCE RULINGS (2026-07-31, all five open questions closed)

1. **debugger.asm: KEEP VENDORED** — the plan is to move off it entirely once our own
   debugging lands; porting a to-be-replaced vendored file is waste.
2. **Parked editor exports + pitcher_plant anims: DELETE AS STALE** (small, not in the
   build, the editor is the source of truth).
3. **Generators: LEARN TO EMIT `.emp`** — generated `.asm` is not a tolerated class.
4. **player_common offsets: WAIT for the full conversion parcel** (keep the
   byte-identical extern-difference form until then).
5. **Demo tree: IN SCOPE, convert** — it proves the new-game pure-`.emp` path
   end-to-end (the sonic4 game code is already ~all `.emp`; demo's remainder is the
   small demo_state/demo_box pair + config/data).

**The standing intent, in Volence's words: everything converted — done correctly.**
The 11-parcel sequence above executes under that bar (fold-identity or the full
re-freeze ceremony per parcel, never a bulk mechanical sweep).
