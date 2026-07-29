# Game-side census (games/sonic4/) — the durable re-derivation

**Author:** read-only recon agent (Fable-dispatched), 2026-07-29.
**Status:** written, NOT committed; no aeon edit; no build run (source + committed
pins/listings only — every claim needing a build is tagged **PORTER-VERIFY**).
**Supersedes:** the uncommitted 2026-07-28 game-side recon. Re-derives it against
the real trees; the 2026-07-29 t26-close-packet's three framing corrections and its
one OVERRULED census claim (the `$55A4` listing-echo trap) are folded in below.

**STATUS AMENDMENT (2026-07-29, t29 close — `2026-07-29-t29-close-packet.md`):**
G1 is PORTED (`test_static.emp` + `test_animated.emp`, merged; byte-delta zero,
strict 2691/0). TWO corrections this census owes:
1. **test_animated was UNDER-SCOPED** (§3a row): it is NOT "trivial … display +
   AnimateSprite" with 2 cross-seam calls — it carries a `vars DplcV: Sst.sst_custom`
   SST-OVERLAY (the corpus's FIRST `vars` overlay, grep-confirmed) + a THIRD cross-seam
   call `Perform_DPLC`. So test_animated — not test_parent (§3c/G3) — is the FIRST
   game-side SST-overlay-twin port. G3's brief reframes: test_parent is the 2nd/3rd
   overlay consumer (the shared-overlay-twin consolidation question opens there), not
   the debut.
2. **"shape-invariant" (§2) means BASES, not bytes:** G1 file BASES are shape-invariant
   (bank not slid), but CONTENT bytes track per-shape cross-seam operands → the
   COMPILE-TWICE class (the gate compiles once per shape), not "identical bytes plain/
   debug." Same for every canonical game-code file that calls shape-moving engine symbols.

**The $55A4 trap (never repeat it):** an asl listing ECHOES skipped-ifdef lines
with a frozen address and NO byte column. A listing address is never proof of
emission. Every emission claim here is grounded in the include graph
(`games/sonic4/main.asm`) + `if/ifdef` structure, never a bare listing address.
Anything that would need the byte column or the built ROM to settle is PORTER-VERIFY.

---

## 1. HEADLINE NUMBERS (files by class)

Counting every `.asm` under `games/sonic4/` (79 files) plus the game-owned
manifest. The porting target is the **code + config** surface; the data surface is
overwhelmingly not-portable-by-design or already-ported.

| Class | Count | Ported (.emp exists) | Remaining |
|---|---|---|---|
| **Game CODE** (objects / player / test-state) | 20 | 3 | **17** |
| **Game MANIFEST** (main.asm) | 1 | 0 | 1 (config-class, see §4) |
| **CONFIG** (constants/game/ram/sound_ids) | 4 | 0 | 4 (Spec-5 flip, blocked) |
| **DATA** (animations/parallax/sound/mappings/objdefs/levels/generated/editor/sprites) | 54 | 6 | ~48, but ~all NOT-portable-by-design or DSL-await |

**The prior "~20 code + 4 config" framing HOLDS.** Code total = 20 (objects 11 +
player 6 + test-state 2 + game_debug already counted under objects/debug = actually
11 objects incl. 2 ported + 6 player + 2 test + 1 debug = 20; main.asm is the 21st
but is manifest/config-class). Ported code: `test_solid.emp`, `test_particle.emp`,
`game_debug.emp` (t26). **17 code files remain** + the manifest + 4 config.

**The single most important census fact (corrects a likely mis-read):** the game
CODE backlog is **CANONICAL in BOTH shapes** (files are included unconditionally in
the bank/engine-block/state macros), so it uses the **standard windowed byte-gate
machinery already proven by `test_objects_port`** — NOT game_debug's off-canonical
oracle. game_debug was the exception (whole-file `ifdef SOUND_DEBUG_HOTKEYS` → zero
canonical bytes → needed the off-canonical AS-twin oracle). No other game code file
is empty-in-canonical. Only **3 code files carry `__DEBUG__` shape divergence**
(`path_swap`, `ojz_scroll_test`, `object_test_state`); all others are
shape-invariant (same bytes plain/debug), the cheapest byte-gate class.

---

## 2. EMISSION-SHAPE PRIMER (the two gate classes on the game side)

- **Canonical, shape-invariant** (the common case): file emits identical bytes in
  plain and debug. Byte gate = one windowed region, both shapes share the pin.
  Proof machinery = `test_objects_port` class (region pin `pins::TEST_SOLID` /
  `TEST_PARTICLE`, gate `SIGIL_EMP_*`, `org`-resume in main.asm). ~14 of the 17.
- **Canonical, shape-DEPENDENT** (`path_swap`, `object_test_state`,
  `ojz_scroll_test`): file has internal `ifdef __DEBUG__` blocks → debug shape is
  longer. Byte gate needs distinct plain/debug lengths (the `vblank`/`core`
  shape-dependent precedent). PORTER-VERIFY the per-shape byte deltas at step 1.
- **Off-canonical** (game_debug only, DONE): whole-file gated to a non-shipped
  define; proven by an AS-twin oracle at the enabling shape. No remaining game code
  is in this class.

---

## 3. PER-FILE CENSUS — GAME CODE

Placement context from `games/sonic4/main.asm`:
- `gameObjectBankIncludes` (org `$10000` = ObjCodeBase): player_common →
  player_ground → player_air → player_spindash → sonic, then the objects
  (test_static, test_animated, test_player, test_enemy, **[SIGIL_EMP_TEST_OBJECTS:
  test_solid, test_particle]**, test_emitter, test_parent, test_stress_emitter,
  test_churn, path_swap). Dispatched via `objroutine()` / `code_addr`.
- `gameEngineBlockIncludes` (engine region, NOT the object bank):
  **player_sensors** + game_debug. (main.asm:20-23; player_sensors "has no
  code_addr entry points".)
- `gameStatesIncludes` (game-state region): object_test_state, ojz_scroll_test.

### 3a. Objects (object bank) — the LEAN-friendly front

| File | Lines | Shape | Role / notes | Cross-seam calls |
|---|---|---|---|---|
| `objects/test_static.asm` | 11 | invariant | **PORTED (t29)** → test_static.emp (`pins::TEST_STATIC` $10C66, gate `SIGIL_EMP_TEST_STATIC`). One `jbra Draw_Sprite` proc. | Draw_Sprite (sprites.emp) |
| `objects/test_animated.asm` | 48 | invariant (overlay window-overflow check) | **PORTED (t29)** → test_animated.emp (`pins::TEST_ANIMATED` $10C6A, gate `SIGIL_EMP_TEST_ANIMATED`). **FIRST game-side SST-overlay port** — `vars DplcV: Sst.sst_custom` (corpus's first `vars` overlay); adopts vram_art, builds vram_bytes. | Draw_Sprite, AnimateSprite (animate.emp), **Perform_DPLC** (dplc.emp) |
| `objects/test_enemy.asm` | 63 | invariant | badnik-shaped test object. | Draw_Sprite, ObjectMove, TouchResponse |
| `objects/test_emitter.asm` | 50 | invariant | spawns effects. | CreateEffect_Normal/Simple (children.emp t24) |
| `objects/test_stress_emitter.asm` | 51 | invariant | effect-pool stress. | CreateEffect_* (children.emp) |
| `objects/test_churn.asm` | 85 | invariant | alloc/despawn churn test. | AllocDynamic, DeleteObject, CreateEffect |
| `objects/test_parent.asm` | 190 | invariant | **2 SST overlay structs** (TParentV, TOrbitChildV) + child lifecycle; GetSineCosine orbit. | CreateChild_Normal, DeleteChildren (children.emp), GetSineCosine |
| `objects/test_player.asm` | 293 | invariant (3 asserts) | test player-ish object. | Draw_Sprite, sensors, Sound_PlaySFX |
| `objects/path_swap.asm` | 132 | **DEBUG-divergent** (2 `__DEBUG__`) | path-swap object; debug-only blocks. | Draw_Sprite, section calls |
| `objects/test_solid.asm` | 22 | invariant | **PORTED** → test_solid.emp (`pins::TEST_SOLID` $10F7C). | Draw_Sprite |
| `objects/test_particle.asm` | 46 | invariant | **PORTED** → test_particle.emp (`pins::TEST_PARTICLE` $10F8A). | ObjectMove, AnimateSprite, Draw_Sprite |

Every engine callee the objects reach is **already .emp-ported** (sprites, animate,
children, core, sound_api) → module-to-module resolution, no new externs. The
object-shape template is fully worked (test_solid.emp / test_particle.emp):
`use engine.objects.sst.{Sst}`, `pub proc … (a0: *Sst) clobbers(…) falls_into …`,
`move.w #Foo_Main - ObjCodeBase, code_addr(a0)`, `jbra/jbsr` into engine.

**.emp-readiness for objects:** covered by existing capabilities. The per-object
SST **overlay-struct** idiom (test_parent's TParentV/TOrbitChildV — `struct` +
`_field = SST_sst_custom+…` equates) is the one recurring shape to name: it is the
file-local struct-twin class (row 25 EntityScanState precedent) — a game-side
`struct` in the .emp with `@`-asserted offsets over `SST_sst_custom`. No new grammar
demanded; PORTER-VERIFY the overlay-collision `if …_len > SST_interact-SST_sst_custom`
assert ports as an `ensure` (game_debug/const-guard precedent).

### 3b. Player cluster (object bank + engine block) — the campaign's largest game lump

~2689 lines across 6 files, **tightly coupled through player_common**:

| File | Lines | Placement | Role |
|---|---|---|---|
| `player/player_common.asm` | 770 | object bank (first) | **KEYSTONE.** Defines `PlayerV` struct (13-field SST overlay, `_pl_*` equates), the `(a4)` physics-table `PPHYS_*` offsets, and MACROS the state files expand: `setStandingSize`/`setBallSize`, `maskOpposingLR`, `distToFix`. Player_Init, Player_Main, dispatch, history rings, display tail. |
| `player/player_ground.asm` | 783 | object bank | ground state machine (Ground_Move, PState_Roll/Spindash) — biggest single file. |
| `player/player_air.asm` | 470 | object bank | air state machine (Air_*, wall/floor/ceiling probes). |
| `player/player_spindash.asm` | 119 | object bank | PState_Spindash. |
| `player/sonic.asm` | 54 | object bank | character assets/physics hooks (Sonic_InitAssets, Sonic_LoadArt). |
| `player/player_sensors.asm` | 493 | **engine block** | sensor primitives (Player_SensorFloor/Ceiling/WallAt/Pair); no code_addr entry points → different region than the bank files. |

**Coupling seam (internal, .asm↔.asm today):** player_common's `PlayerV` struct +
`_pl_*` / `PPHYS_*` equates + 3 macros are consumed by ground/air/spindash. Porting
player_common FIRST establishes: (1) a `PlayerV` **file-local (or shared) struct
twin** (row 11/25 class — the biggest new game-side struct twin, 13 fields); (2) the
3 macros as **comptime-fn templates** (macro-port rule — `maskOpposingLR`,
`distToFix` are the `reload_anim_timer` class; `set*Size` are field-writers). Until
player_common ports, ground/air can't see the .emp struct/macros → player_common is
a hard ordering root.

**Demanded features to flag (cite):** `maskOpposingLR` (player_common.asm:95-101)
uses a fixed internal label `.lr_masked` expanded once per global scope — a
comptime-fn splice template (hygienic-label class, `emit_piece_loop` precedent).
`distToFix` (player_common.asm:109-113) is the `swap`/`clr.w` pair =
**`pixels_to_coord`** which ALREADY EXISTS in `engine/coords.emp` (kill row 49) —
ADOPT, don't rebuild. Type-layer: `_pl_state` (PSTATE_*), ANIM_* ids, ground_speed
(Velocity/inertia) are step-2 item-6 newtype candidates (PSTATE could be a
`PlayerState` sum type; ANIM_* → AnimId item-13). PORTER-VERIFY the register-heavy
physics math against A4-i deferral (shift/add chains wait; moved+compared type now).

### 3c. Test-state harness (game-state region)

| File | Lines | Shape | Role / notes |
|---|---|---|---|
| `test/object_test_state.asm` | 365 | **DEBUG-divergent** (1 `__DEBUG__`) | GS_OBJECT_TEST harness: spawns test objects, runs the object loop, renders. |
| `test/ojz_scroll_test.asm` | 310 | **DEBUG-divergent** (5 gates incl `__DEBUG__`) | GS_OJZ_SCROLL_TEST — the game ENTRY state (`Game_Entry = GameState_OJZScroll_Init`, game.asm:46). Camera-scroll harness. **Carries kill-list row 35** (the per-frame mode-register force-write workaround, :234-273, a parallax-engine-gap compensation). |

These are `setVDPReg`/palette-copy/`jsr Level_LoadArt`-heavy harness code (VDP/DMA
facing) → the panel would run **lens C3** (hardware-timing) on them. Larger,
debug-divergent, and ojz_scroll_test is entangled with an OPEN engine-gap kill row →
**highest risk, port LAST** of the code surface. PORTER-VERIFY per-shape lengths.

### 3d. Manifest

| File | Lines | Class |
|---|---|---|
| `main.asm` | 354 | **Manifest / config-class — NOT a normal code port.** Defines the 7 game-contract include MACROS (`gameConfigIncludes`, `gameRamIncludes`, `gameEngineBlockIncludes`, `gameObjectBankIncludes`, `gameDataIncludes`, `gameSoundDataIncludes`, `gameStatesIncludes`) that `engine/engine.inc` invokes, PLUS raw BINCLUDE carriers (HeightMaps/AngleTable/Map_Sonic/DPLC_Sonic/Art_Sonic, :144-172) and ALL the sigil mixed-build `org` resume arms. This is the build orchestrator + the reverse side of every `SIGIL_EMP_*` seam. Ports only when the dual build dies (Spec 5) or a game-manifest .emp construct is designed. Treat as config-class, blocked. |

---

## 4. CONFIG (Spec-5 ownership-flip class — all BLOCKED on dual-build death)

All four emit **ZERO ROM bytes** (pure `=`/`equ` constants + macros + `ds` RAM
reservation). They are the AS-side OWNERS of values the .emp side currently MIRRORS
with drift guards. They cannot flip to .emp-owned until the gate-off AS build dies
(the .emp `const`/`equ` export can't be seen by the AS front-end yet — kill rows
4/54 stage 2 = "at Spec 5").

| File | Lines | Contents | Kill rows it would close |
|---|---|---|---|
| `config/constants.asm` | 86 | PSTATE_*, ANIM_* ids, SPINDASH_*, ring/collected caps, VRAM_TEST_* | rows 18/22 (game ring/collected mirrors) |
| `config/sound_ids.asm` | 94 | SONG_* (debug-gated), SFXID_* ladder, SFX_ID_BASE/COUNT/TABLE_LEN, SFXPRI_* | row 10 (SFXID typed mirrors), row 54 (game_debug SONG_*/SFXID_*) |
| `config/game.asm` | 75 | ROM header equs, GAME_* contract, `gameBootHook` + `gameDebugTick` MACROS | rows 9/45 (gameDebugTick/gameBootHook macro mirrors) |
| `config/ram.asm` | 60 | `phase Engine_RAM_End` RAM map (Player_Phys, history rings, Dbg_* debug-gated) | — (the game RAM `vars`-era story) |

**This is a class, not a tranche:** the whole 4-file config cluster flips together
when Spec-5 lands (or piecemeal as each drift-guard's kill condition trips). It is
NOT a "port a file" tranche in the current dual-build regime — porting it now would
break the gate-off AS build that still reads these values. Confirms the brief's
"config 4-file cluster is a Spec-5 ownership-flip class blocked on dual-build death."

---

## 5. DATA (NOT-portable-by-design + data-table classes — bulk)

54 `.asm`; 6 already have .emp twins. Per-directory role (representative citations):

- **GENERATED (NOT-portable-by-design)** — `data/generated/ojz/act1/*.asm` (6:
  bg_anim, entity_data, ojz_act_pool, ojz_act_pool_manifest, sec_block_blobs,
  sec_block_dicts). Header: "AUTO-GENERATED by tools/ojz_entity_gen.py — DO NOT EDIT"
  (entity_data.asm:1). Tool output; the .emp campaign does not own these.
- **EDITOR-OWNED (NOT-portable-by-design)** — `data/editor/ojz/act1/export/*.asm`
  (3: act_descriptor, entity_data, vram_bases). Raw `dc.w` exports authored by the
  editor pipeline (entity_data.asm:1 `ojz_Sec0_Rings: dc.w …`). Stays AS-side.
- **SOUND (mostly NOT-portable / BINCLUDE / bank-critical)** — `data/sound/*` (29).
  Includes the phased Z80 `phase 08000h` bank head, song streams, patch banks,
  pitch tables, the 18 `sfx/sfx_*` blob+patch files, `sfx_table`. Bank-alignment +
  no-straddle asserts are LOAD-BEARING (main.asm:201-343). Three data-tables ARE
  ported (`dac_samples.emp`, `mt_bank.emp`, `sfx/sfx_bank.emp` — t27 lanes). The
  rest are generator output / bank-placement carriers; DSL-await at best.
- **PARALLAX (data-table, portable or DSL-await)** — `data/parallax/*` (10: 2 configs
  + 4 effects + 4 scenes). Deform tables + `ParallaxConfig_*` records (haze.asm is a
  reusable-effect fixture). A **data-table DSL candidate** (the campaign's #1
  offset-table roadmap item, per t26 panel) but no port yet.
- **ANIMATIONS / MAPPINGS / OBJDEFS / LEVELS (data-table, partly ported)** —
  `data/animations/{particle,sonic}_anims.emp` PORTED; `data/objdefs/test_objects.emp`
  PORTED; `data/levels/ojz/act1/act_descriptor.emp` PORTED. Remaining:
  `data/mappings/test_mappings.asm` (mapping data-table),
  `data/sprites/pitcher_plant/anims.asm` (anim script data-table).

**Data is NOT the game-side code tranche target.** The portable-code campaign should
treat data as the separate data-table/DSL track (largely done or DSL-await).

---

## 6. SEAM MAP (what a game-code tranche must know)

- **All engine callees are already .emp** (§3 tables): sprites, animate, children,
  core, sound_api, coords, sensors-in-engine. Object/player ports resolve
  module-to-module; **no new `extern proc` decls needed** (the corpus's last extern,
  Sound_DebugMirror, is engine-side, row 42).
- **ObjCodeBase seam:** every object writes `#Foo_Main - ObjCodeBase, code_addr(a0)`
  — a `.w` link-time immediate against the cross-seam `.asm` symbol ObjCodeBase.
  Proven (test_solid.emp:22). Player_Main is the same dispatch.
- **Object-bank placement is CONDITIONAL, not structural** (main.asm:52-64): the
  bank's contents are `__DEBUG__`-invariant, but its code calls ENGINE symbols, so a
  debug-only engine growth that pushes a callee past `$8000` widens `jsr (Sym).w` →
  `abs.l` (+2) and slides the whole debug bank. This is the **$8000 abs.w/abs.l bar**
  (t24). Every new object/player region shares the ONE `org` resume only while the
  two banks coincide — PORTER-VERIFY per port that plain/debug bank pins still
  coincide (main.asm:60 records a +$14E debug growth that tripped this at t24).
- **PlayerV / TParentV / TOrbitChildV struct twins** (new game-side, rows 11/25
  class): each is an SST overlay `struct` + `_field = SST_sst_custom+…`. The .emp
  gets a file-local `struct` with `@`-offset ensures; the AS twin keeps its `struct`
  in lockstep until Spec 5.
- **Kill rows a game-code tranche would ADD/CLOSE:** objects/player add row-5 AS-twin
  scaffolding rows (gate-off body lockstep) + row-6 region pins per file. Closes:
  nothing on the current list directly, but shrinks the object-bank twin surface.
  ojz_scroll_test's port INTERACTS WITH open **row 35** (the harness force-write —
  its B2 sub-decision-(ii) kill condition; do not port ojz_scroll_test without
  reconciling row 35).
- **Config mirrors ride cross-seam** (rows 4/9/10/18/22/45/54): a game-code port that
  touches PSTATE_*/ANIM_*/SFXID_*/SONG_* consumes the config-owned values via the
  existing engine.constants.emp / game_debug.emp mirrors or new file-local mirrors +
  `ensure(extern(...))` drift guards (row 54 template).

---

## 7. TRANCHE ORDERING RECOMMENDATION (LEAN, 1-3 files each)

Ordered by (a) machinery already proven, (b) risk, (c) coupling roots. Byte movement
is **zero (canonical, shape-invariant)** unless noted; proof machinery = the
`test_objects_port` canonical windowed byte-gate class + a per-region pin + `org`
resume, EXCEPT where a shape-dependent length or new struct twin is called out.

**G1 — trivial display/animate objects** *(FIRST — highest machinery reuse, lowest risk)*
`test_static` + `test_animated` (+ optionally `test_enemy`).
- Why first: structurally identical to the shipped test_solid.emp/test_particle.emp;
  all callees ported; zero byte movement; shape-invariant; the object-bank pin +
  gate + org machinery is proven end-to-end. Establishes the game-object port rhythm
  before touching structs or debug shapes.
- Proof: canonical byte gate both shapes (shared pin); byte class = zero-slide.
- New scaffolding: 2-3 row-5 twin rows + region pins.

**G2 — effect/child-lifecycle objects** *(second — exercises the children.emp seam)*
`test_emitter` + `test_stress_emitter` + `test_churn`.
- Why: small, invariant, but drive CreateEffect/AllocDynamic/DeleteObject — proves
  the game→children.emp/core.emp effect seam at scale. Zero byte movement.
- Risk: low. Proof: canonical byte gate both shapes.

**G3 — struct-overlay object** *(third — introduces the game-side struct twin)*
`test_parent` (+ optionally `test_player`).
- Why: first game-side `struct` overlay twin (TParentV/TOrbitChildV, row-25 class) +
  child dispatch + GetSineCosine. Names the SST-overlay idiom every later object and
  the whole player cluster reuse. Zero byte movement; shape-invariant.
- Risk: low-med (struct-offset ensures, PORTER-VERIFY the overlay-collision assert →
  `ensure`).

**P1 — player keystone** *(fourth — the hard ordering root; its own mini-arc)*
`player_common` (+ `sonic`).
- Why here (not later): player_common defines the `PlayerV` struct + `_pl_*`/`PPHYS_*`
  equates + 3 macros that ground/air/spindash NEED as .emp before they can port. It
  is a hard root. Pairing tiny `sonic` (54 lines, asset hooks) keeps it LEAN.
- New machinery: the **PlayerV 13-field struct twin** (row-11/25 class); the 3 macros
  → comptime-fn templates under the macro-port rule (ADOPT coords.emp's
  `pixels_to_coord` for `distToFix` — do NOT rebuild). Type-layer walk on PSTATE_*/
  ground_speed (candidates, likely A4-i-deferred).
- Byte movement: zero if step-1 faithful; step-2 may relax branches (re-pin tax).
- Risk: MED-HIGH (largest new struct twin + macro redesign). Give it step-0 design.

**P2 / P3 — player state machines** *(fifth/sixth — after the keystone)*
`player_ground` (P2, 783 lines, solo — biggest file); then
`player_air` + `player_spindash` (P3).
- Why after P1: they consume P1's struct/macros. Invariant, canonical; hot-path →
  full step-5 + a Fable hot-path second look + panel lens C1/C2.
- Byte movement: zero at step 1; step-2/5 relaxations re-pin. Risk: MED (hot physics).

**P4 — sensors** *(engine-block region, separable)*
`player_sensors` (493 lines).
- Why separable: lives in `gameEngineBlockIncludes`, a DIFFERENT region than the
  bank; no code_addr entry points → its own pin, independent of the bank slide bar.
  Can port any time after P1 (it's a called primitive). Risk: MED (VDP-free math;
  panel C1/C2).

**T1 — harness states** *(LAST — debug-divergent + open kill row)*
`object_test_state`, then `ojz_scroll_test`.
- Why last: both are `__DEBUG__` **shape-dependent** (distinct plain/debug byte
  lengths — the vblank/core shape-dependent gate class, PORTER-VERIFY the deltas);
  VDP/DMA-facing (panel lens C3); and **ojz_scroll_test is entangled with OPEN kill
  row 35** — reconcile the harness force-write before porting.
- Proof: shape-DEPENDENT byte gate (separate plain/debug pins). Risk: HIGHEST.

**NOT tranched (blocked):** the **config 4-file cluster** (§4, Spec-5 flip) and
**main.asm** (§3d, manifest/config-class). They flip when the dual build dies, not on
a code tranche.

---

## 8. SURPRISES vs the prior census's framing

1. **The game code backlog is CANONICAL byte-gate work, not off-canonical.** A likely
   mis-read from t26 (game_debug's off-canonical oracle) is that game-side ports need
   the AS-twin oracle machinery. They do NOT — 16/17 remaining code files are
   canonical in both shapes and reuse the proven `test_objects_port` windowed gate.
   game_debug was the lone off-canonical exception (whole-file hotkeys ifdef). This
   makes the game-code front CHEAPER than t26 implied.
2. **The player cluster has a hard ordering ROOT** (player_common), not a free file
   set. Its struct + macros gate ground/air/spindash. Any tranche plan that ports a
   state file before the keystone is unbuildable.
3. **`distToFix` is already-built** (coords.emp `pixels_to_coord`, kill row 49) — a
   free adoption the player port must not re-hand-roll.
4. **ojz_scroll_test is not a clean leaf** — it is the game ENTRY state AND carries
   open kill row 35 (parallax engine-gap force-write). Higher risk than its line
   count suggests.
5. **"~20 code + 4 config" confirmed exactly** (20 code incl. 3 ported + manifest;
   4 zero-byte config). The prior "~10" is superseded, as the t26 packet recorded.
6. **Data is a separate track** — 54 data `.asm`, but ~all generated/editor/BINCLUDE
   (not-portable-by-design) or already-ported/DSL-await; not the code-tranche target.

---

**Census path:** `/home/volence/sonic_hacks/sigil/docs/superpowers/notes/2026-07-29-game-side-census.md`
</content>
</invoke>
