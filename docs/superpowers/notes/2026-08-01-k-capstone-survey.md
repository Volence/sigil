# Parcel K capstone survey — the residual AS layer, complete inventory

Read-only survey for the Parcel-K spec owner (Fable). Grounds every claim in the
aeon tree at master `b21d01e` and the sigil tree at `k-survey` (from master
`8f9cd698`, conv-i8-vectors tip). The K spec dissolves the residual AS skeleton;
this note is the exhaustive inventory it designs against.

Numbers are grep-proof (command + count stated). Where a fact contradicts the
Parcel-K census rows (`2026-07-31-conversion-tail-census.md`) or the conv-h/h2/i
notes, the discrepancy is called out in **§11**.

---

## §0. Headline numbers

- **Residual `.asm` in the aeon build tree: 17 files, 2723 lines.**
  (`find . -name '*.asm' -not -path './.git/*' -not -path './.worktrees/*'` → 17)
- **Residual `.inc`: 4 files, 656 lines** — of which **`engine/objects/aabb.inc`
  (50 lines) is a DEAD ORPHAN** (§10, §11-D); the live three are `engine.inc`,
  `sound_bank.inc`, `header.inc`.
- **Total residual AS lines (asm+inc): 3379.**
- **Residual `org` directives: 106 lines** (engine.inc 66, sonic4/main.asm 34,
  demo/main.asm 3, act_descriptor.asm 2, boot_data.asm 1). **All 106 are INERT as
  placement VALUES** — every shipped target is `SizeSource::Frozen` (§6); the
  chainer discards the baked org and recomputes the base. The ONE org that still
  expresses a placement RELATIONSHIP is boot_data's `org $3FE` mid-image hole (§7).
- **BINCLUDE sites in sonic4/main.asm: 18** (collision island 7 + DAC island 2 +
  MT/SFX island, shape-armed).
- **Six identity targets:** `s4` / `s4_debug` / `demo` / `demo_debug` /
  `config_a` / `config_b` (one frozen table each, `golden/offcanonical_sizes/`).
  demo + demo_debug build **sound-OFF** (`games/demo/build.conf`
  `SOUND_DRIVER_ENABLED:=0`); config_b is the silent-sonic4 no-sound whole-ROM proof.

### The K work, by class (17 asm + 3 live inc)

| Class | Files | K action |
|---|---|---|
| Skeleton-delete | `games/sonic4/main.asm` (352), `games/demo/main.asm` (59), `engine/engine.inc` (438) | split residual → migrate placement → delete the manifest |
| Contract-macro remainder | `engine/macros.asm` (367), `engine/system/header.inc` (55), `engine/sound/sound_bank.inc` (113) | dies when its last residual consumer moves; header.inc emits the $100 header data |
| Config `-D` / header-string carrier | `games/sonic4/config/game.asm` (83), `games/demo/config/game.asm` (30) | named remainders (header data + `-D` gates + Game_Entry equalate) |
| Org-hole / interior-island | `engine/system/boot_data.asm` (130), `games/sonic4/data/levels/ojz/act1/act_descriptor.asm` (61) | boot_data = the STOP org-hole (§7); act_descriptor = the interior-island wrapper (§2) |
| Generated (committed, OJZ) | `entity_data.asm` (265), `ojz_act_pool.asm` (14), `ojz_act_pool_manifest.asm` (9), `sec_block_blobs.asm` (27), `sec_block_dicts.asm` (11), `bg_anim.asm` (4) | generator-emits-`.emp` (Parcel I), not a K deletion — but they are the interior-island DATA the skeleton wraps |
| Generated (gitignored, sound) | `mt_syms.asm` (4), `mt_syms_debug.asm` (4) | sigil-emit syms; `z80_sound_syms.asm` (59) is STALE (§10) |
| Vendored (keep) | `engine/debug/debugger.asm` (806) | Volence ruling 1: KEEP VENDORED |
| Dead orphan | `engine/objects/aabb.inc` (50) | delete now — no consumer survives (§10) |

---

## §1. The residual AS census (all 21 files)

`find` result, with line count · includer · consuming targets · K class.
Line counts: `wc -l`.

### `.asm` (17)

| path | ln | included by | targets | K class |
|---|---|---|---|---|
| `engine/debug/debugger.asm` | 806 | engine.inc:79 | all 6 | VENDORED-KEEP (ruling 1) |
| `engine/macros.asm` | 367 | engine.inc:69 | all 6 | contract-macro remainder (§4) |
| `engine/sound/generated/mt_syms.asm` | 4 | main.asm:290 | s4 (sound-on) | generated sym (§I) |
| `engine/sound/generated/mt_syms_debug.asm` | 4 | main.asm:287 | s4_debug | generated sym (§I) |
| `engine/sound/generated/z80_sound_syms.asm` | 59 | **NOBODY** | none | STALE ORPHAN (§10) |
| `engine/system/boot_data.asm` | 130 | engine.inc:102 | all 6 | ORG-HOLE, STOP (§7) |
| `games/demo/config/game.asm` | 30 | demo/main.asm:20 | demo, demo_debug | config remainder (§8) |
| `games/demo/main.asm` | 59 | build.sh (root) | demo, demo_debug | SKELETON (§2) |
| `games/sonic4/config/game.asm` | 83 | sonic4/main.asm:20 | s4, s4_debug, cfgA, cfgB | config remainder (§8) |
| `.../generated/ojz/act1/bg_anim.asm` | 4 | act_descriptor:56 | s4 family | generated (§I) |
| `.../generated/ojz/act1/entity_data.asm` | 265 | main.asm:156 | s4 family | generated, interior-island (§2) |
| `.../generated/ojz/act1/ojz_act_pool.asm` | 14 | act_descriptor:9 | s4 family | generated (§I) |
| `.../generated/ojz/act1/ojz_act_pool_manifest.asm` | 9 | act_descriptor:8 | s4 family | generated (§I) |
| `.../generated/ojz/act1/sec_block_blobs.asm` | 27 | act_descriptor:40 | s4 family | generated (§I) |
| `.../generated/ojz/act1/sec_block_dicts.asm` | 11 | act_descriptor:19 | s4 family | generated (§I) |
| `.../levels/ojz/act1/act_descriptor.asm` | 61 | main.asm:157 | s4 family | interior-island wrapper (§2) |
| `games/sonic4/main.asm` | 352 | build.sh (root) | s4, s4_debug, cfgA, cfgB | SKELETON (§2) |

### `.inc` (4)

| path | ln | included by | K class |
|---|---|---|---|
| `engine/engine.inc` | 438 | main.asm ×2 (root include) | SKELETON — the ROM-layout owner (§3) |
| `engine/system/header.inc` | 55 | engine.inc:82 | contract-macro, EMITS $100 header data (§8) |
| `engine/sound/sound_bank.inc` | 113 | engine.inc:72 | contract-macro, sound-head data island + fatal walls (§8) |
| `engine/objects/aabb.inc` | 50 | **NOBODY** | DEAD ORPHAN (§10) |

**Ownership / gitignore classes** (`.gitignore`):
- `engine/sound/generated/` — **gitignored** (`.gitignore:11`); regenerated by
  `build.sh`'s `$SIGIL_EMIT --out-dir engine/sound/generated`. (mt_syms{,_debug},
  the `.bin` blobs.)
- `engine/debug/generated/` — **gitignored** (`.gitignore:75`); now `vectors.emp`.
- `games/sonic4/data/generated/**/*.asm` — **COMMITTED** (un-ignored via
  `.gitignore:83-85` `!` negation); the OJZ level tree ships in-repo.

---

## §2. The skeletons — main.asm ×2

Both are thin manifests: define the 8 contract macros, set
`PAD_TO_POWER_OF_TWO=1`, `include "engine/engine.inc"`, `END`.

### `games/sonic4/main.asm` (352 ln, 34 org lines, 18 BINCLUDE)

Macro-by-macro (all `macro {GLOBALSYMBOLS}`):

- **`gameConfigIncludes`** (10-21): `include "games/sonic4/config/game.asm"`. (Comment
  documents that constants/sound_ids are now `.emp`+harvested.)
- **`gameRamIncludes`** (23-29): EMPTY (comment only) — RAM is `ram.emp`.
- **`gameEngineBlockIncludes`** (31-53): **3 org lines** — an `ifdef __DEBUG__`
  pair (`$633C` / `$55A4`, per-shape base for player_sensors) + one
  `ifdef SIGIL_EMP_GAME_DEBUG` `org $6408` (Config-A only, canonically zero bytes).
- **`gameObjectBankIncludes`** (55-138): **13 org lines** — 12 flat resume orgs
  ($10448…$111FA, the player/test-object bank) + one `ifdef __DEBUG__` pair
  ($112F4 / $1128C, path_swap). All land on `.emp`-native labels.
- **`gameDataIncludes`** (140-205): **6 org lines** (3 `ifdef` pairs:
  $11DE6/$11D7E, $257B2/$2574A, $257BA/$25752) + **`include entity_data.asm`
  (156)** + **`include act_descriptor.asm` (157)** + the **collision BINCLUDE
  island** (177-205): `HeightMaps` / `HeightMapsRot` / `AngleTable` /
  `SolidityTable` / `Map_Sonic` / `DPLC_Sonic` / `Art_Sonic` (7 BINCLUDEs +
  `align 2` + two `if …>$7FFF error` word-offset walls).
- **`gameSoundDataIncludes`** (207-322): the DAC/MT/SFX island — the most
  conditional-heavy region. Arms: `SIGIL_EMP_DAC_BODY_STUB` (org $58000 harness
  skip) vs the real DAC BINCLUDE pair (`dac_blip_bank.bin` @ align$8000,
  `dac_shared_bank.bin`); the `SND_ENGINE_TABLE_BANK`/`SFX_BLOB_BANK` equates
  (250-258); the `save/cpu z80/phase 08000h/soundBankHead/dephase/restore` head
  bracket (259-264); `SIGIL_EMP_MT_BODY_STUB` (org pair $5D53A/$5BAE8) vs the MT
  BINCLUDE pair + `include mt_syms{,_debug}.asm` (285-291); `SIGIL_EMP_SFX_BODY_STUB`
  (org pair $5DC82/$5C230) vs the SFX BINCLUDE pair (316-320).
- **`gameStatesIncludes`** (327-349): **2 org lines** (2 `ifdef` pairs
  $5E2DA/$5C7EC, $5E5A8/$5CAAE) — object_test_state + ojz_scroll_test resume orgs.

### `games/demo/main.asm` (59 ln, 3 org lines, 0 BINCLUDE)

The minimal template. Six macros are 1-org-or-empty:
`gameConfigIncludes`→`include game.asm`; `gameRamIncludes`/`gameEngineBlockIncludes`/
`gameSoundDataIncludes` EMPTY; `gameObjectBankIncludes`=`org $10006`;
`gameDataIncludes`=`org $10100`; `gameStatesIncludes`=`org $10172`. Each org holes
out ONE `.emp`-native twin (demo_box / demo_data / demo_state). This is the
"start here" file a new game author reads first — the K spec must preserve its
readability as the template even as it deletes the sonic4 skeleton.

### act_descriptor.asm — the interior-island structure (61 ln, 2 org lines)

Included at main.asm:157, INSIDE `gameDataIncludes`, right after entity_data.asm.
Post conv-h #34 it is a **wrapper, not data** — the descriptor+sections literal is
`act_descriptor.emp` (the typed `Act`/`[Sec;9]`). What remains:
- `include ojz_act_pool_manifest.asm` (8), `include ojz_act_pool.asm` (9) — the
  art-pool equates + page BINCLUDEs.
- `if OJZ_ACT_POOL_TILES > POOL_TILE_CEILING error` (13-15) — a ceiling wall.
- `include sec_block_dicts.asm` (19) — dict-length equates.
- **The org pair** (30-34): `ifdef __DEBUG__ / org $14E06 / else / org $14D9E` —
  resumes AS placement past the `.emp` descriptor block for the block/palette/BG
  data below.
- `include sec_block_blobs.asm` (40) — per-section block BINCLUDEs (with equ
  aliasing: `OJZ_Sec4_Blocks equ OJZ_Sec2_Blocks`).
- 4 direct BINCLUDEs: `OJZ_Palette` (ojz_palette.bin), `BGND_Palette`
  (SonicAndTails.bin), `OJZ_Act1_BG_Layout` (zone_bg.bin), `OJZ_Act1_BG_Tiles`
  (bg_tiles.bin), each `align 2`.
- `include bg_anim.asm` (56) + `align 2`.

So the "interior island" the K spec must relocate = **entity_data.asm** +
**act_descriptor.asm** (which nests the 5 generated OJZ includes + 6 BINCLUDEs).
Its boundary in the ROM is the org pair at act_descriptor:30-34 ($14D9E/$14E06)
downward through the BINCLUDEs; the sec_block_blobs + palettes + BG are the
DATA that follows the `.emp` descriptor and stays AS-side today.

### The SIGIL_EMP_* gate defines seen in the skeletons

Placement-stub gates (harness-only, canonically unset): `SIGIL_EMP_DAC_BODY_STUB`,
`SIGIL_EMP_MT_BODY_STUB`, `SIGIL_EMP_SFX_BODY_STUB` (sonic4/main.asm);
`SIGIL_EMP_GAME_DEBUG` (sonic4/main.asm:43). Engine gates live in engine.inc (§3).

---

## §3. engine.inc (438 ln, 66 org lines, 107 ifdef/else/endif tokens)

The ROM-layout owner. Structure top-to-bottom:

1. **GAME CONTRACT doc block** (1-58): the required-symbols + required-macros
   contract a game manifest must satisfy. Pure comment — but it is the AUTHORITATIVE
   spec of the manifest interface the K capstone replaces.
2. **cpu/padding/supmode** (60-62).
3. **Definitions, no ROM output** (64-79):
   `include "engine/macros.asm"` (69), `include "engine/sound/sound_bank.inc"` (72),
   `gameConfigIncludes` (73), `gameRamIncludes` (78),
   `include "engine/debug/debugger.asm"` (79).
4. **org 0 image** (81-88): `include "engine/system/header.inc"` (82) → `org $100`
   → `gameHeader` (88). The vectors ($000-$0FF) are `vectors.emp`; `org $100` is the
   FIXED-SIZE resume; `gameHeader` emits $100-$1FF (§8, header.inc).
5. **Engine code block** (90-361): `__BUDGET_ENGINE:` label (93), then **~30
   resume-org sites** (mostly `ifdef __DEBUG__`/else pairs — 66 org lines total),
   each holing out one `.emp`-native engine region (boot_data at 102, VDP shadow,
   DMA queue, sprites, VBlank, HBlank, controllers, GameLoop, S4LZ/ZX0/math,
   DPLC, objects, entity_window, tile_cache, collision, camera, parallax, art, BG,
   compression_selftest, sound_api …). `gameEngineBlockIncludes` at 282. The two
   trailing gated sites: `SOUND_DRIVER_ENABLED` sound_api org (337-348) +
   `SIGIL_EMP_SOUND_DEBUG` org $827C (349-361).
6. **Object code bank** (363-378): `org $10000` / `ObjCodeBase:` / `rts` /
   `__BUDGET_OBJBANK:` / `gameObjectBankIncludes` (373). The 64KB overflow guard
   is now the MAP-owned `check_object_bank_budget` (comment 375-378).
7. **Data region** (380-384): `__BUDGET_DATA:` / `gameDataIncludes`.
8. **Sound-data region** (386-392): `ifdef SOUND_DRIVER_ENABLED /
   gameSoundDataIncludes / endif`.
9. **Game states** (394-395): `gameStatesIncludes` (outside the sound conditional).
10. **Epilogue** (397-438): `NullInterrupt: rte`; the error_handler/MDDBG comment;
    the EndOfRom resume org pair ($5F65A/$5DB60); `EndOfRom: align 2`; three walls
    (`EndOfRom & 1`, `EndOfRom > $3FFFFF`, `PLANE_H_CELLS*PLANE_V_CELLS > 4096`).

Engine gates present: `SOUND_DRIVER_ENABLED`, `__DEBUG__`, `SIGIL_EMP_SOUND_DEBUG`.
`__BUDGET_*` markers: **`__BUDGET_ENGINE` (93), `__BUDGET_OBJBANK` (371),
`__BUDGET_DATA` (383)** — all three survive; OBJBANK/DATA bracket the object bank
for `check_object_bank_budget` (§6); ENGINE has no live consumer (§9).

---

## §4. macros.asm (367 ln) — mostly already dead in AS

Defines **25 helpers**: 11 `function`s (`vdpComm`, `vdpReg`, `vram_art`,
`vram_bytes`, `sprSize`, `bytesToLcnt`, `vdpCommDelta`, `planeLoc`, `dmaSource`,
`dmaLength`, `objroutine`) + 14 `macro`s (`objvarsCheck`, `objdef`, `objentry`,
`objend`, `stopZ80`, `startZ80`, `disableInts`, `enableInts`, `setVDPReg`,
`vdpCommReg`, `queueStaticDMA`, `clearLoadedRing`, `clearLoadedObj`,
`collSrcRowBase`).

**Included by exactly one file: engine.inc:69.**

**Live residual consumers = 5 helpers across 2 files** (grep for each name in
`*.asm`/`*.inc`, minus macros.asm's own defs, minus comment-only hits):
- `vdpComm`, `vdpReg`, `bytesToLcnt` → **boot_data.asm** (real invocations).
- `objentry`, `objend` → **entity_data.asm** (15 `objentry` + `objend`).

`objdef`/`objroutine` appear only in COMMENTS now (sonic4/main.asm:98/108,
57/109/114; engine.inc:366/381) — no live expansion (demo_data/test_objects are
`.emp`). **The other 20 helpers have ZERO residual-AS consumers** — their `.emp`
comptime-fn twins (engine.vdp, aabb.emp, sound_api.emp's stop_z80/start_z80,
etc.) carry the load. Several are kill-list-tracked lockstep twins that survive
only "until sound_api.asm/the twins retire" (row 24 stopZ80/startZ80) — but those
`.asm` twins are already gone, so those macros are dead too.

**Consequence for K:** macros.asm dies the moment **boot_data.asm** and
**entity_data.asm** stop being AS. It is NOT a broad "port the macro library" job
— it is "remove the last 5 uses, delete the file." The 20 dead helpers can be
deleted independently *today* (byte-neutral — no expansion), which would shrink
the file to its 5-helper live core before the capstone.

---

## §5. boot_data.asm (130 ln, 1 org) — the org-hole STOP (census #12)

`BootData:` table walked by a single `(a5)+` cursor. Contents:
- movem preload (3 words d5-d7 + 5 longs a0-a4), `BootData_VDPRegs` (24 `dc.b`
  VDP regs $00-$17), a `dc.l vdpComm(0,VRAM,DMA)` fill command.
- **The Z80-program region, shape-armed** (47-74):
  - `ifdef SOUND_DRIVER_ENABLED`: `Z80_Sound_Start:` + BINCLUDE pair
    (`z80_sound_blob{,_debug}.bin`) + `Z80_Sound_End:` + `Z80_SOUND_SIZE` equ.
  - **`else` (no-sound)**: `Z80_IDLE_SIZE = $3FE-$3D8` + **`org $3FE`** — THE HOLE.
    The 38-byte idle program is `z80_init.emp`, chained separately at 0x3d8; the
    `org $3FE` skips the hole so the post-hole AS data (PSG silence, post-DMA cmds)
    resumes correctly.
- `align 2`, PSG silence (`dc.b $9F,$BF,$DF,$FF`), post-DMA cmds (`vdpReg($0F,$02)`,
  CRAM/VSRAM `vdpComm`), `BootData_End:`.
- **The layout-assert wall** (87-130): 4 `if…fatal` geometry locks (movem head =
  26; sound arm: Z80 blob at +54, `Z80_SOUND_SIZE` even, total =
  `54+SIZE+4+10`; no-sound arm: `Z80_IDLE_SIZE` even).

**Why it STOPPED (conv-h2):** `.emp` has `align`/byte-`pad` but no absolute-`org`
hole surface, and the hole is filled by a *separately-chained* module overlaid
between a pre-hole and post-hole split of the SAME data table. Reproducing it
natively needs a conditional two-module split (pre-hole / z80_init overlay /
post-hole) — a per-shape native-registry restructure. The hole is LIVE for the
sound-off shapes (demo, demo_debug; proven whole-ROM by
`mixed_offcanonical_rom::mixed_z80_init_config_b`). **This is the single hardest K
sub-problem** and the reason boot_data can't ride the mechanical-port path.

---

## §6. The placement machinery as-built (sigil, read-only)

`crates/sigil-harness/src/native.rs` (2741 ln). The production build path
(`build.sh` → `sigil build --native`) is `build_rom_chained_with_listing`
(1955-2018). Flow someone can design against:

```
1. ensure_generated()                 // if sound_on: run sigil emit → *.bin blobs
2. as_side  = assemble_as_side()      // asl on games/<g>/main.asm with SIGIL_EMP_*
                                       //   gates DEFINED → native modules holed out;
                                       //   AS residual sections carry labels at
                                       //   their asl offsets (the orgs feed asl here)
3. emp      = build_emp()             // lower every .emp module → Sections
4. sections = as_side ++ emp
5. true_bases_by_index(sections, size_source)   // the DECLARED-ORDER authority:
      - PinnedBaked (BOOTSTRAP ONLY): true base = baked asl lma
      - Frozen (ALL 6 SHIPPED):       true base = frozen[label] − label.offset;
                                      label-less blob = contiguity from neighbour;
                                      phase bank (vma≥0x8000) = baked org
6. declared_spans_by_index()          // exact live-measured span per section
7. apply_declared_chain()             // sort ROM by ascending true base; each org
      island / phase-bank head = Pinned ANCHOR (keeps base); every other section =
      Chained, lma←0 (base COMPUTED, baked resume-org DISCARDED). packed_true_bases
      = the §17 B-0 walk (islands anchor, the rest packs from live lengths)
8. resolve_layout → check_link_asserts (drift partition: real Value(0) = hard fail;
      gated-off unresolved externs = inapplicable allowlist)
9. link → load sigil.map.toml → check_object_bank_budget → emit_rom (checksum folded)
```

**The org lines never set a base in any shipped build.** `size_source` is
`Frozen` for all six (`sonic4_profile`=Frozen s4.txt:347; demo/config likewise).
`PinnedBaked` exists only in `sonic4_pinned_profile`, which mints the frozen
tables and never runs a shipped build. The orgs survive purely so asl's cursor
advances and the AS residual sections carry the right LABELS at measurable
offsets — their VALUES are recomputed away at step 7 (`s.lma = 0`).

**Genuine anchors that survive step 7** (`apply_declared_chain` /
`phase_region_mask` 1895-1920): (a) the run head + any org-island whose prov base
exceeds `running + ANCHOR_GAP`; (b) phase banks — `vma_base` ≥ 0x8000 and ≠ lma
(the DAC/MT/SFX hard-org sound banks) — which keep their baked absolute image and
never pack; (c) label-less data tails inside a phase run (contiguity). The
object-bank base ($10000) is ALSO map-declared (below).

### The map (`sigil.map.toml`) — geometry only

3 regions: `rom` (lma 0, size 0x400000), `object_bank` (lma 0x10000, size
0x10000, the `__BUDGET_DATA` cursor check), `z80_moving_trucks_bank` (lma 0x60000,
size 0x8000, vma 0x8000). Its own SCOPE NOTE (OQ-E) states the split explicitly:
**per-game section ORDER + per-region SIZES live in the frozen tables +
`pins.rs`, NOT the map** — folding them in "is the capstone-class full-pins
retirement (ledgered), not this parcel." That capstone IS Parcel K's "pins→map".

`load_map` usages: `native.rs`, `seam2.rs`, `map_load.rs`, `lib.rs`, and
`map_toml` fixtures in **~68 sigil-cli port tests** (each port test builds a
one-region-per-section nominal map). K's pins→map migration ripples every one.

### pins.rs / repin.toml — what pins→map must replace

- `repin.toml`: **58 `[[region]]` entries** + the `[rom]` end-symbol; the
  declarative manifest (engine.inc ladder order). `cargo run --bin repin`
  resolves each against both listings → regenerates `pins.rs`.
- `pins.rs`: **1172 lines** — the per-shape resolved bases/lens the port gates and
  the frozen-chain anchoring consume.

**"pins→map" must replace, enumerated** (from the map SCOPE NOTE + the census K
row): (1) per-game **section ORDER** (today: frozen-table label order +
`apply_declared_chain` sort) → a declared **§3.3 ordering manifest**; (2)
per-region **declared SIZES** (today: `golden/offcanonical_sizes/*.txt` × 6 +
`pins.rs`) → map-owned or computed-from-placement; (3) the **resume-org anchors**
(today: 106 inert org lines feeding asl + the frozen `island` bases) → computed
from placement; (4) the **object-bank/section-region ordering** enforcement
(today: `check_object_bank_budget` + the `__BUDGET_*` bracket labels) →
`emit_rom`-enforced region+ordering; (5) the **68 port-test `map_toml` fixtures**
+ `repin.toml`/`pins.rs`/the 6 frozen tables — the whole pin apparatus retires
together or not at all.

---

## §7. The org-hole problem, precisely

Two absolute-`org` sites in the residual AS express a real placement RELATIONSHIP
(vs the 104 inert resume orgs):

1. **boot_data.asm:73 `org $3FE`** (no-sound arm) — the MID-IMAGE HOLE. Filled
   from `z80_init.emp` (the 38-byte idle), chained by the frozen chainer at 0x3d8;
   `Z80_IDLE_SIZE = $3FE-$3D8` (=$26) is the numeric mirror the assert wall needs
   (the BootData-relative waypoints are link-external, so a comptime `if` cannot
   fold them — the "row-52 wall"). LIVE for demo/demo_debug (§5). **STOP finding**:
   needs the conditional two-module split. No completable subset.
2. **engine.inc:368 `org $10000` / `ObjCodeBase:`** — the object-bank base. This is
   a GENUINE anchor, but it is ALSO map-declared (`object_bank` lma 0x10000), so
   the K capstone can drop the org and let the map own it (the budget check already
   references `__BUDGET_OBJBANK`/`__BUDGET_DATA`, not the org).
3. **engine.inc:81 `org 0`** — the header/vectors base; structural, expressed by
   the `rom` region + the FIXED-SIZE vectors region.

All other 103 org lines are RESUME orgs (base recomputed). The phase-bank
`align $8000` hard-orgs (DAC $48000/$50000, MT/SFX $58000) are not `org` lines —
they are `align $8000` + `vma_base` phase brackets, which the chainer keeps as
anchors (§6b); K leaves those in whatever form emits the banked image.

---

## §8. Contract-macro remainders — the game.asm pair + header.inc + sound_bank.inc

### games/sonic4/config/game.asm (83 ln) — census #22 named remainder

Content, none of which flips: **9 `GAME_*` header strings** (`equ`, consumed by
`gameHeader`); a sound-contract COMMENT block (14-41, no assignment — SFX_BLOB_BANK
moved to main.asm, SFX_ID_BASE family derived in sfx_bank.emp); **`GAME_CAMERA_JUMP_LOCK
= 1`** (a `-D` interface const — cannot live in `.emp`); **`Game_Entry =
GameState_OJZScroll_Init`** + **`GAME_ENTRY_ID = GS_OJZ_SCROLL_TEST`** (cross-seam
label equalates); **`gameBootHook`** macro (60-70, DEFINED-not-INVOKED — boot.emp
expands the `.emp` mirror; kill row 45); **`gameDebugTick`** macro (77-83, likewise
— game_loop.emp mirrors it; kill row 9).

### games/demo/config/game.asm (30 ln) — census #15, the identical shape

9 `GAME_*` strings, `GAME_CAMERA_JUMP_LOCK = 0`, `Game_Entry = GameState_Demo_Init`,
`GAME_ENTRY_ID = GS_DEMO`, empty `gameBootHook`/`gameDebugTick`.

### engine/system/header.inc (55 ln) — census-MISSED, EMITS DATA

`gameHeader macro`, invoked at engine.inc:88. **Emits the $100-$1FF ROM header**
(9 `dc.b` string fields + `Checksum:` + ROM/RAM range longs `EndOfRom-1`), each
field guarded by a `strlen…fatal` width wall (9 walls). This is the ONE
contract-macro that produces ROM bytes — the K spec must decide how the header
(9 game-declared strings → 256 data bytes) is authored natively (a `.emp` data
section reading harvested `GAME_*` strings, or a header construct). The `EndOfRom-1`
back-reference makes it order-sensitive with the epilogue.

### engine/sound/sound_bank.inc (113 ln) — census-MISSED, DATA ISLAND + walls

`soundBankHead macro`, invoked at sonic4/main.asm:262 inside the
`phase 08000h` bracket. Emits the engine-table bank head as **6 BINCLUDE'd sigil
artifacts** at fixed VMAs: `SoundTablesZ80_Head` ($8000, wall `<>855`),
`SndDefaultPitchTable`/`MovingTrucks_PitchTable` ($8357, wall `<>2*PITCHTAB_COUNT`),
`SfxBlobWinTab` ($845F, shape-armed, wall `<>SFX_TABLE_LEN*2`), `SeqOpcodeTable`
($856D, shape-armed, wall `<>32*2`), `DacSampleTable` ($85AD, wall
`<>DAC_SAMPLE_COUNT*DacSample_len`). All labels precede their BINCLUDE (the
resident driver's banked carriers depend on the exact $8000-window addresses).
This is a demo-referenced contract (demo has no sound bank yet — the "TODO for
whoever grows this into a real game"). K must preserve this as the sound-head
authoring vocabulary; the 5 span-walls read `.emp`-harvested equates
(SFX_TABLE_LEN, PITCHTAB_COUNT, DAC_SAMPLE_COUNT, DacSample_len).

**Kill row 9 (the game_loop combo-matrix constraint):** `game_loop_port.rs`
re-extracts sonic4 `gameDebugTick`'s body from the REAL `game.asm` every run and
byte-diffs all four `SOUND_DEBUG_HOTKEYS × SOUND_DRIVER_ENABLED` define combos
against game_loop.emp's mirror. So game.asm's `gameDebugTick` (and `gameBootHook`,
kill row 45 via boot_port) is the LOCKSTEP source of a live sigil test — K cannot
delete these macro bodies without a ratified `.emp` game-contract-hook mechanism
(extern-macro / link-time hook, Spec-5 neighborhood). This is the combo-matrix
machinery the K spec must not break.

---

## §9. Ordering constraints the §3.3 ordering-manifest must own

Every place the residual AS encodes "X before Y":

- **engine.inc include order** (69-79): macros.asm → sound_bank.inc → gameConfig →
  gameRam → debugger.asm (definitions, no output — order sets symbol visibility).
- **org 0 → header** (81-88): vectors ($000-$0FF) then header ($100-$1FF).
- **Engine code ladder** (93-361): ~30 regions in a fixed sequence; each resume
  org encodes "this `.emp` region ends here, next AS/`.emp` resumes there."
- **Object bank** (368): `ObjCodeBase` = $10000, `objroutine()` offsets from it;
  `__BUDGET_OBJBANK`/`__BUDGET_DATA` bracket the 64KB budget.
- **gameSoundDataIncludes bank alignment** (main.asm 207-322): DAC banks
  (align $8000 × 2) → MT streamed bank (align $8000, `SND_ENGINE_TABLE_BANK =
  MovingTrucks_Bank_Start>>15`) → SFX block; the soundBankHead head-table VMAs
  ($8000/$8357/$845F/$856D/$85AD) are fixed-order within the bank.
- **Collision island order** (main.asm 177-205): HeightMaps → … → Art_Sonic, with
  `Map_Sonic`/`DPLC_Sonic` word-offset walls (`>$7FFF`).
- **act_descriptor internal order** (act_descriptor 8-56): pool_manifest → pool →
  dicts → [org] → block_blobs → palettes → BG → bg_anim.
- **sec_block_blobs equ aliasing** (`OJZ_Sec4_Blocks equ OJZ_Sec2_Blocks`) — a
  content-dedup order dependency.
- **EndOfRom back-reference** (header.inc:49 `EndOfRom-1`; engine.inc:414-422) —
  the header's ROM-end field depends on the epilogue label.

**`__BUDGET_*` markers surviving: 3** — `__BUDGET_ENGINE` (engine.inc:93, no live
consumer found — grep hit is the label + comment only; a cleanup/ordering-marker
candidate), `__BUDGET_OBJBANK` (371) + `__BUDGET_DATA` (383) (both consumed by
`check_object_bank_budget`). No `__BUDGET_*` in main.asm.

---

## §10. Census-missed / newly-dead files (item 10)

1. **`engine/objects/aabb.inc` (50 ln) — DEAD ORPHAN.** Nothing includes it
   (`grep -rn 'aabb.inc'` across asm/inc/emp/sh/rs/toml → zero includes; the only
   hits are two comment lines in `aabb.emp`). Its documented consumers —
   `collision.asm` + `rings.asm`, the gate-off AS twins — **no longer exist**
   (`find engine -name collision.asm -o -name rings.asm` → empty). The `.emp`
   consumers (`collision.emp:6`, `rings.emp:13`) `use engine.objects.aabb` — the
   `.emp` twin `aabb.emp`, NOT the `.inc`. Kill-list row 5 tracks it as "dies with
   the twins at Spec 5"; the twins already died, so it is **deletable now,
   byte-neutral**, ahead of K. Not in the 50-file census.

2. **`engine/sound/generated/z80_sound_syms.asm` (59 ln) — STALE, NO LONGER
   EMITTED, UNCONSUMED.** Census #11 classifies it as "GENERATED (sigil emit) —
   ALREADY a sigil output." Reality: `seam1.rs:446` states the file "is no longer
   emitted — the banked seq table is native and resolves those VMAs in-link, and no
   AS consumer of them survives; kill-list row 92." Nothing includes it (grep across
   all file types → zero). It is a gitignored (`engine/sound/generated/`) leftover
   from a prior build sitting untracked on disk. Not K-material — it drops out the
   moment `engine/sound/generated/` is next cleaned/regenerated. Cleanup, not port.

3. **`engine/system/header.inc` + `engine/sound/sound_bank.inc`** — both are live,
   data-emitting contract-macro `.inc`s (§8) that the 50-file census (which
   enumerated `.asm` only, plus engine.inc by name) did not list. They ARE
   K-material (the header data + the sound-head data island).

---

## §11. Discrepancies vs the census and conv-h/h2/i notes

- **A. Line-count drift** (census → current): sonic4/main.asm 343→**352**;
  demo/main.asm 43→**59**; act_descriptor.asm 268→**61** (gutted at conv-h #34, as
  the census row-34 note records); boot_data.asm 131→**130**; engine.inc **438**
  (census called it "not in the 50" — correct, but no count given). Effect: the
  census's "~15 INERT resume orgs" for main.asm undercounts — reality is **34 org
  lines** in sonic4/main.asm and **66** in engine.inc (**106** residual org lines
  total, mostly per-shape pairs).

- **B. macros.asm consumer reality.** Census #4 says it "dies when its last
  residual-AS consumer moves" and lists boot_data/demo_data/mappings as consumers.
  Current: demo_data + mappings are `.emp`; the live surface is **5 helpers in 2
  files** (boot_data + entity_data); **20 of 25 helpers have zero AS consumers**
  and are deletable today (§4). The file is far closer to death than the census row
  implies.

- **C. sound_bank.inc + header.inc missing from the census.** Both are live
  data-emitting `.inc`s consumed by engine.inc; neither appears in the 50-file
  census. They belong in the K material (§8).

- **D. aabb.inc's kill condition already fired.** Census/kill-list row 5+13 treat
  it as "dies with the gate-off twins at Spec 5." The twins are already deleted, so
  it is a present dead orphan, not a future one (§10.1).

- **E. z80_sound_syms.asm no longer generated.** Census #11 treats it as a live
  sigil output to "emit-.emp vs fold." It is neither emitted nor consumed (§10.2);
  the census row is stale vs `seam1.rs`.

- **F. mt_syms{,_debug}.asm are the surviving generated syms.** Census #9/#10.
  Confirmed live: `include`d at main.asm:290/287, supplying `SongTable`/
  `SongPatchTable` to sound_api's cross-seam `movea.l`. gitignored, regenerated by
  `$SIGIL_EMIT`. These are the real "emit-.emp-syms vs fold-via-link" decision the
  census #9-11 flagged — z80_sound_syms already resolved itself (fold).

- **G. The census "pins→map" framing is confirmed by the map's own SCOPE NOTE.**
  Not a discrepancy — corroboration: `sigil.map.toml` explicitly reserves order +
  sizes for the "capstone-class full-pins retirement," i.e. Parcel K (§6).

---

## §12. Hazards the spec must not miss (stop-findings)

1. **boot_data's org-$3FE hole is the one non-mechanical sub-problem.** It needs a
   conditional two-module split (pre-hole / z80_init overlay / post-hole) and a
   native absolute-org-hole surface `.emp` does not have (conv-h2 STOP). It is LIVE
   for demo/demo_debug and whole-ROM-proven only by
   `mixed_offcanonical_rom::mixed_z80_init_config_b`. Do not scope K assuming the
   hole "just becomes a pad" — the fill is a separately-chained module, and the
   assert wall's BootData-relative waypoints are link-external (numeric-mirror
   `Z80_IDLE_SIZE` required).

2. **The kill-row-9 combo matrix reads the REAL game.asm every run.**
   `game_loop_port.rs` (gameDebugTick) + `boot_port.rs` (gameBootHook, row 45)
   byte-diff the AS macro EXPANSION across define combos. Deleting or moving those
   macro bodies without the ratified game-contract-hook mechanism breaks live
   sigil tests. game.asm's `gameBootHook`/`gameDebugTick` bodies + the 9 `GAME_*`
   strings + `GAME_CAMERA_JUMP_LOCK` (`-D`) + `Game_Entry`/`GAME_ENTRY_ID`
   equalates are the IRREDUCIBLE game-contract remainder — plan game.asm as a
   surviving named remainder, not a deletion, unless the hook mechanism ships in K.

3. **`-D` interface names can never move to `.emp`** (codebase rule, conv-f).
   `GAME_CAMERA_JUMP_LOCK`, `MAX_RING_BUFFER`, `VRAM_RING_PLACEHOLDER`,
   `COLLECTED_WINDOW_SLOTS` stay `native.rs` `emp_defines`. The header/game
   contract cannot be "100% .emp" while these are `-D` gates.

4. **header.inc + sound_bank.inc emit ROM DATA and carry 14 fatal/span walls**
   (9 header strlen walls + 5 sound-head span walls reading harvested equates).
   These are not zero-byte definition layers — the header ($100-$1FF, with the
   `EndOfRom-1` back-reference) and the $8000-window bank head are load-bearing
   image bytes. K must author them natively, not just delete them.

5. **All 106 resume orgs are inert VALUES but asl still needs them to place the AS
   residual's LABELS.** They cannot be bulk-deleted while any AS residual section
   remains — asl would collapse the residual into the wrong offsets and the frozen
   `true_bases` (which key off those labels) would misresolve. The orgs and the AS
   residual die TOGETHER, region by region; the last one out deletes the skeleton.

6. **The pins/map/frozen-table/port-test apparatus retires as a unit.** pins→map
   touches `repin.toml` (58 regions), `pins.rs` (1172 ln), 6 frozen tables, and
   **~68 sigil-cli `map_toml` port-test fixtures**. There is no partial pins→map
   that leaves the port gates green — scope it as the single capstone the map's
   SCOPE NOTE already names.

7. **macros.asm has a free byte-neutral pre-shrink** (delete the 20 dead helpers
   now, §4) — but keep the 5 live ones (vdpComm/vdpReg/bytesToLcnt/objentry/objend)
   until boot_data + entity_data are native, and keep `objdef`/`objroutine`
   comptime semantics documented (they still describe the object archetype +
   ObjCodeBase-offset contract the `.emp` side implements).

8. **aabb.inc and z80_sound_syms.asm are deletable ahead of K, byte-neutral** (§10)
   — separating them from the capstone shrinks its surface and removes two stale
   census rows before the spec is written.
