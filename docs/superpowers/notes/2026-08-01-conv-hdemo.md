# 2026-08-01 — CONV-H-DEMO: the demo tree native flip + the demo native-placement path

Status: **Checkpoint for the overseer's countersign + merge.** Branch pair
`conv-hdemo` (aeon + sigil). The demo game tree (#14/#17/#18/#20) flipped to
native `.emp`; the demo native game-module placement PATH built (the conv-h §5
blocker retired); #15 confirmed a documented AS remainder. NOT merged.

## §0 — THE HEADLINE

- **The demo is now a pure-`.emp` game** (the "new-game pure-.emp path" Volence
  wanted proven): its config, data, state, and object are ALL `.emp`, placed
  natively. Only the thin AS manifest `main.asm` + the game-contract residual
  `config/game.asm` survive AS-side.
- **The demo native game-module placement path was BUILT** — the conv-h finding
  ("the demo profile has ZERO native game modules; demo_registry is engine-only")
  is retired. It reused the sonic4 machinery WHOLESALE: no new harness code, only
  demo-profile DATA (registry rows + gates + frozen anchors) + `game_constants_rel`.
- **Four `.asm` twins DELETED**, `.emp` sole source, no scaffolding introduced.
- **Six-ROM identity:** s4 + both config pairs FULLY byte-identical (CRC unchanged);
  the demo pair's ASSEMBLED ANCHOR is byte-identical (`e2dc9207`/`b7d81931`), the
  full-file re-froze **appendix-only** (3 new symbols) — the flagged H precedent.
- **Strict 2877 / 0 / 4** (baseline 2875 + 2 new `demo_native_port` tests).

## §1 — Six-target CRC set (post-parcel)

| target | full CRC32 / size | assembled anchor | vs CHAIN-10 |
|---|---|---|---|
| s4.bin        | ff9037f2 / 412127 | 658c623b | UNCHANGED (full + anchor) |
| s4.debug.bin  | 06680f0b / 421958 | b137c411 | UNCHANGED (full + anchor) |
| config_a      | 2485eab3 / 422297 | 19b793ec | UNCHANGED (full + anchor) |
| config_b      | d6d23298 / 303501 | 8b490cfb | UNCHANGED (full + anchor) |
| demo.bin      | **4e446a64** / 90524 | **e2dc9207 (UNCHANGED)** | full moved appendix-only (was 9bb8c993/90506) |
| demo.debug.bin| **949e9215** / 93022 | **b7d81931 (UNCHANGED)** | full moved appendix-only (was bc7678d0/93006) |

**Appendix-only re-freeze evidence (the demo pair):** the ONLY byte differences in
the assembled prefix `[0, EndOfRom=0x11224)` are at `$18E-$18F` (the Sega header
checksum) and `$1A7` (the ROM-end pointer) — both header fields excluded by
`assembled_anchor_crc`. Masking them, the header-neutral anchor is byte-identical
old↔new (`e2dc9207`/`b7d81931`, matching each frozen table's committed
`assembled_anchor`). The full-file CRC moved because the deb2 appendix grew by 18
bytes (3 new symbols the `.emp` modules expose: `DemoBox_Main`, `ObjDef_DemoBox`,
`GameState_Demo_Init`, plus the `offsets`/struct internal labels, net +18 vs the
`.asm` twins' symbol set). Re-freeze via `refreeze --freeze conv-hdemo` (chain len
**11**); it detected the demo anchors did NOT move (so the `--ab` discipline was
satisfied with no A/B — the sanctioned appendix-only path) and left s4/config as a
FIXPOINT (their golden blobs untouched — verified: CRCs above unchanged).

## §2 — The infrastructure (the demo native game-module placement path)

The sonic4 side got this incrementally (conv-d/parallax/conv-h #35); the demo got
it in one parcel, reusing every mechanism:

- **`native.rs demo_registry()`** — 3 new `ModuleSpec`s: `games.demo.demo_box`
  (section `demo_box`), `games.demo.data.demo_data` (`demo_data`),
  `games.demo.demo_state` (`demo_state`). Regions are `DUMMY_REGION` — COSMETIC
  under `SizeSource::Frozen` (the chainer packs from the frozen tables' anchors);
  only the module id + section name are load-bearing. `require_one_text` stays OFF
  (demo_data lands its `pub data` in the named `demo_data` section, not `text`).
- **`native.rs demo_code_gates()`** — 3 new gates `SIGIL_EMP_DEMO_{BOX,DATA,STATE}`
  seeded as AS `-D`. (Vestigial after the twin deletions — nothing `ifdef`-checks
  them now; kept per the sonic4 deleted-twin-gate convention.)
- **`demo.txt` / `demo_debug.txt`** — 3 new head anchors (`DemoBox_Main` 0x10002,
  `ObjDef_DemoBox` 0x10006, `GameState_Demo_Init` 0x10100). Shape-invariant (the
  demo has no DEBUG blocks). `derive_offcanon` reads them back at the same
  addresses (the placement fixpoint IS the proof). No `_End` markers needed —
  demo_state ends at NullInterrupt (0x10172), already an anchor.
- **NO demo pins in `pins.rs`** — `repin` resolves only the canonical sonic4 shape
  (demo symbols are absent from it), so demo modules cannot carry a repin-generated
  pin. `DUMMY_REGION` + the frozen anchors are the placement authority; the
  `demo_native_port` gate reads window addresses from the demo listing.
- **`game_constants_rel`** wired (#14) → `harvest_game_constants` (already built).

Adding this empty-capability-plus-data moved ZERO assembled bytes (the six-ROM
identity above proves it — the demo anchor is byte-identical).

## §3 — Per-file port census

| # | file | disposition | proof |
|---|---|---|---|
| 14 | `config/constants.asm` → `config/constants.emp` (`games.demo.constants`) | PORTED (harvested) | byte-NEUTRAL — six-ROM identity held BEFORE the module flip (increment 1) |
| 15 | `config/game.asm` | **AS-RESIDUAL remainder** (sonic4 #22 shape) | header strings + gameBootHook/gameDebugTick (defined-not-invoked) + Game_Entry/GAME_ENTRY_ID + GAME_CAMERA_JUMP_LOCK `-D` |
| 17 | `data/demo_data.asm` → `data/demo_data.emp` | PORTED (native) | `demo_native_port` window `[0x10006,0x10100)` byte-identical, plain+debug |
| 18 | `demo_state.asm` → `demo_state.emp` | PORTED (native) | window `[0x10100,0x10172)` byte-identical |
| 20 | `objects/demo_box.asm` → `objects/demo_box.emp` | PORTED (native) | window `[0x10002,0x10006)` byte-identical |

(#16 `config/ram.emp` was already done at item #7c — RAM-only, no ROM placement,
which is why it landed before this infrastructure.)

**#14 constants.emp** declares GS_DEMO, VRAM_DEMO_OBJ, RING_BUFFER_ENTRY_SIZE,
RING_WIDTH, COLLECTED_SLOT_SIZE, COLLECTED_PARK_SLOTS, COLLECTED_PARK_ENTRY_SIZE —
the demo's constants.asm MINUS the 3 engine-VARYING `-D`s (MAX_RING_BUFFER /
VRAM_RING_PLACEHOLDER / COLLECTED_WINDOW_SLOTS), which stay `emp_defines` per the
`-D`-not-in-`.emp` rule (the sonic4 constants.emp shape). The 5 engine
`ensure(extern(..))` guards (RING_BUFFER_ENTRY_SIZE / RING_WIDTH /
COLLECTED_SLOT_SIZE / COLLECTED_PARK_SLOTS / COLLECTED_PARK_ENTRY_SIZE) resolve
against the harvested EquSyms — enumerated + all present.

**#17 demo_data.emp** — `objdef()` emitter (ObjDef_DemoBox), the `offsets` mapping
construct (Map_DemoBox, one `MapFrame1 = centered(...)` frame; the test_mappings
shape with local MapPiece/MapFrame1/spr_size/centered structs), a struct-typed
spawn list (DemoObjectList + terminator), art (`[u32; 40]`), palette (`[u16; 16]`),
BgAnim_Table header. `use games.demo.constants.{VRAM_DEMO_OBJ}`.

**#18 demo_state.emp** — GameState_Demo_Init / GameState_Demo procs. Cross-seam
externs (Palette_Buffer, Palette_Dirty, Camera_X/Y, Game_State, QueueDMA_Critical,
InitObjectRAM, Init_SpriteTable, Load_ObjectList, InitSpriteSystem, RunObjects,
TouchResponse, Render_Sprites, VDP_Shadow_Table, VDP_Dirty_Mask + the demo_data
labels) fold at the joint link; `setVDPReg` → the VDP_MODE2_OFF shadow-write +
dirty-bit expansion (the object_test_state.emp pattern); `DEMO_ART_LEN` const = 160
(the `DemoArt_End - DemoArt` immediate).

**#20 demo_box.emp** — `jbra Draw_Sprite` (the test_static.emp shape; jbra emits the
4-byte `jmp (Draw_Sprite).w`, Draw_Sprite < $8000 out of bra.w range).

## §4 — Retirements + re-homes

- **Retired (deleted):** `games/demo/config/constants.asm`, `data/demo_data.asm`,
  `demo_state.asm`, `objects/demo_box.asm`.
- **Re-homes (sigil-side):** `demo_registry` (3 ModuleSpecs) + `demo_code_gates`
  (3 gates) + `demo_profile.game_constants_rel`; `demo.txt`/`demo_debug.txt` (3
  head anchors each); `provenance.toml` (appendix-only entry, chain len 11);
  `demo.bin`/`demo.debug.bin` golden blobs re-frozen. `pins.rs` region stamp
  bumped 57→58 by the refreeze's repin (a PRE-EXISTING stale count; `0 pins
  changed` — metadata only).
- **main.asm:** the three game macros' includes → ONE unconditional `org` each
  (the sonic4 deleted-twin shape); `gameConfigIncludes` drops the constants.asm
  include (keeps game.asm).

## §5 — step-3 (retrospect) / step-5 (engine)

- **step-3:** the port needed NO new language or harness construct — `objdef()`,
  `offsets`, structs, `jbra`, cross-seam externs, and `harvest_game_constants` all
  pre-existed. The one genuine gap is a SHARED mapping-frame home (the
  MapPiece/MapFrame1/centered structs are duplicated demo↔sonic4) — gap-ledgered.
  Premise correction: the census scoped #14 as needing "a demo
  `harvest_game_constants` extension" — but the mechanism was already built
  (conv-f #21); only the per-profile WIRING (`game_constants_rel = Some(...)`) was
  missing. No extension.
- **step-5:** none — pure ownership move at unchanged assembled bytes. The §17
  opt-sweep owns byte-changers; this parcel is placement/authorship only.

## §6 — GAP-LEDGER (→ campaign-gap-ledger.md)

1. The conv-h "demo profile has NO native placement path" row is **RESOLVED** (path
   built this parcel; reused sonic4 machinery, no new harness code).
2. **Shared mapping-DSL home** — MapPiece/MapFrame1/spr_size/centered are
   duplicated between test_mappings.emp and demo_data.emp; an
   `engine.objects.mappings` comptime module would let both `use` it.

## §7 — KILL-LIST (→ twin-scaffolding-kill-list.md, row 101)

- #14/#17/#18/#20: the four demo `.asm` twins DELETED; `.emp` sole source, no
  scaffolding survives. The SIGIL_EMP_DEMO_* gates stay as vestigial `-D` markers
  (sonic4 deleted-twin-gate convention). #15 game.asm stays AS-residual (the
  documented #22-shape remainder).

## §8 — Comment hygiene (fixed this parcel)

The deleted demo `.asm` files were named as illustrative examples in surviving
build/harness sources — swept: `native.rs` (harvest_engine_ram_addresses example #1,
STRUCT_OFFSET_TWINS doc, the emp_defines duplicate comment, the `game_constants_rel`
doc) and `engine/structs.emp` (the VdpShadow harvest note) now reference the native
`.emp` successors instead of the deleted twins. (Byte-neutral — verified.)
