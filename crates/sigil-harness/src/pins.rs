//! GENERATED FILE — DO NOT EDIT BY HAND.
//!
//! Emitted by `cargo run -p sigil-harness --bin repin` from `repin.toml`
//! + SIGIL'S OWN resolved layout (Stage-3 P4c; the asl-`.lst` parse retired).
//! Edit the MANIFEST, then regenerate; `tests/repin_pins.rs::
//! pins_rs_is_current` guards staleness. All values are per-shape VMAs/lengths
//! from sigil's native canonical resolve (plain + `__DEBUG__`).
//!
//! [provenance] plain: sigil-native canonical resolve (plain)
//! [provenance] debug: sigil-native canonical resolve (debug)
//! [provenance] 95 regions, 377 symbols, 7 offsets

/// A per-shape address pin: one cross-seam symbol's VMA in each shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pin {
    pub plain: u32,
    pub debug: u32,
}

/// A gated region's geometry. Slice as `base..base + len` — the lens are
/// computed `end − start` at generation, PER SHAPE (core's debug len ≠
/// plain len), so the slice-end bug class is unwritable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub plain_base: u32,
    pub debug_base: u32,
    pub plain_len: usize,
    pub debug_len: usize,
}

/// A region-relative offset that is genuinely shape-DEPENDENT (the
/// invariant ones emit a bare `usize`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeOffset {
    pub plain: usize,
    pub debug: usize,
}

// ── ROM end (the listing `END` line address, per shape) ──

/// Assembled (pre-convsym) ROM length, plain shape. tests: m1d_rom, m1d_debug_rom, mixed_dac_rom
pub const ASSEMBLED_LEN: usize = 0xA11E0;
/// Assembled (pre-convsym) ROM length, `__DEBUG__` shape. tests: m1d_rom, m1d_debug_rom, mixed_dac_rom
pub const DEBUG_ASSEMBLED_LEN: usize = 0xA3090;

// ── Regions (manifest order) ──

/// `Vectors` .. start + 0x100 plain / 0x100 debug (literal — no end symbol) — gate `SIGIL_EMP_VECTORS`. tests: vectors_port
pub const VECTORS: Region = Region { plain_base: 0x0, debug_base: 0x0, plain_len: 0x100, debug_len: 0x100 };

/// `GameHeader` .. `EntryPoint`. tests: header_port
pub const HEADER: Region = Region { plain_base: 0x100, debug_base: 0x100, plain_len: 0x100, debug_len: 0x100 };

/// `HeightMaps` .. start + 0x1C480 plain / 0x1C480 debug (literal — no end symbol). tests: collision_data_port
pub const COLLISION_DATA: Region = Region { plain_base: 0x28330, debug_base: 0x28B60, plain_len: 0x1C480, debug_len: 0x1C480 };

/// `EntryPoint` .. `BootData` — gate `SIGIL_EMP_BOOT`. tests: boot_port
pub const BOOT: Region = Region { plain_base: 0x200, debug_base: 0x200, plain_len: 0x1A0, debug_len: 0x1A0 };

/// `BootData` .. `BootData_PostBlob`. tests: boot_data_port
pub const BOOT_HEAD: Region = Region { plain_base: 0x3A0, debug_base: 0x3A0, plain_len: 0x1850, debug_len: 0x18D0 };

/// `BootData_PostBlob` .. `BootData_End`. tests: boot_data_port
pub const BOOT_TAIL: Region = Region { plain_base: 0x1BF0, debug_base: 0x1C70, plain_len: 0xE, debug_len: 0xE };

/// `VDP_Shadow_Init` .. `Init_DMA_Queue` — gate `SIGIL_EMP_VDP_INIT`. tests: vdp_init_port
pub const VDP_INIT: Region = Region { plain_base: 0x1C00, debug_base: 0x1C7E, plain_len: 0x3A, debug_len: 0x92 };

/// `Init_DMA_Queue` .. `Init_SpriteTable` — gate `SIGIL_EMP_DMA_QUEUE`. tests: dma_queue_port
pub const DMA_QUEUE: Region = Region { plain_base: 0x1C3A, debug_base: 0x1D10, plain_len: 0x336, debug_len: 0x338 };

/// `Init_SpriteTable` .. `VBlank_Handler` — gate `SIGIL_EMP_BUFFERS`. tests: buffers_port
pub const BUFFERS: Region = Region { plain_base: 0x1F70, debug_base: 0x2048, plain_len: 0x260, debug_len: 0x268 };

/// `VBlank_Handler` .. `HBlank_Install` — gate `SIGIL_EMP_VBLANK`. tests: vblank_port
pub const VBLANK: Region = Region { plain_base: 0x21D0, debug_base: 0x22B0, plain_len: 0x1F0, debug_len: 0x1F0 };

/// `HBlank_Install` .. `Read_Controllers` — gate `SIGIL_EMP_HBLANK`. tests: hblank_port, m1c_vector_table
pub const HBLANK: Region = Region { plain_base: 0x23C0, debug_base: 0x24A0, plain_len: 0x30, debug_len: 0x30 };

/// `Read_Controllers` .. `GameLoop` — gate `SIGIL_EMP_CONTROLLERS`. tests: controllers_port
pub const CONTROLLERS: Region = Region { plain_base: 0x23F0, debug_base: 0x24D0, plain_len: 0x10E, debug_len: 0x110 };

/// `GameLoop` .. `Input_Tick` — gate `SIGIL_EMP_GAME_LOOP`. tests: game_loop_port, load_art_port
pub const GAME_LOOP: Region = Region { plain_base: 0x24FE, debug_base: 0x25E0, plain_len: 0x22, debug_len: 0x22 };

/// `Input_Tick` .. `S4LZ_DecompressDict`. tests: game_loop_port
pub const REPLAY: Region = Region { plain_base: 0x2520, debug_base: 0x2602, plain_len: 0x150, debug_len: 0x1FE };

/// `S4LZ_DecompressDict` .. `ZX0R_Decompress` — gate `SIGIL_EMP_S4LZ`. tests: s4lz_port
pub const S4LZ: Region = Region { plain_base: 0x2670, debug_base: 0x2800, plain_len: 0xF8, debug_len: 0x200 };

/// `ZX0R_Decompress` .. `GetSineCosine`.
pub const ZX0_RESUME: Region = Region { plain_base: 0x2768, debug_base: 0x2A00, plain_len: 0x78, debug_len: 0x80 };

/// `GetSineCosine` .. `Perform_DPLC` — gate `SIGIL_EMP_MATH`. tests: math_port
pub const MATH: Region = Region { plain_base: 0x27E0, debug_base: 0x2A80, plain_len: 0x3F8, debug_len: 0x3F8 };

/// `Perform_DPLC` .. `InitObjectRAM` — gate `SIGIL_EMP_DPLC`. tests: dplc_port
pub const DPLC: Region = Region { plain_base: 0x2BD8, debug_base: 0x2E78, plain_len: 0xA8, debug_len: 0xA8 };

/// `InitObjectRAM` .. `InitSpriteSystem` — gate `SIGIL_EMP_CORE`. tests: core_port
pub const CORE: Region = Region { plain_base: 0x2C80, debug_base: 0x2F20, plain_len: 0x300, debug_len: 0x750 };

/// `InitSpriteSystem` .. `AnimateSprite` — gate `SIGIL_EMP_SPRITES`. tests: sprites_port
pub const SPRITES: Region = Region { plain_base: 0x2F80, debug_base: 0x3670, plain_len: 0x420, debug_len: 0x4EE };

/// `AnimateSprite` .. `TouchResponse` — gate `SIGIL_EMP_ANIMATE`. tests: animate_port, test_objects_port
pub const ANIMATE: Region = Region { plain_base: 0x33A0, debug_base: 0x3B5E, plain_len: 0x194, debug_len: 0x2B8 };

/// `TouchResponse` .. `RingBuffer_Add` — gate `SIGIL_EMP_COLLISION`. tests: collision_port
pub const COLLISION: Region = Region { plain_base: 0x3534, debug_base: 0x3E16, plain_len: 0x200, debug_len: 0x208 };

/// `RingBuffer_Add` .. `Collected_Init` — gate `SIGIL_EMP_RINGS`. tests: rings_port
pub const RINGS: Region = Region { plain_base: 0x3734, debug_base: 0x401E, plain_len: 0x1B8, debug_len: 0x214 };

/// `Collected_Init` .. `PopulateSpawnedPieceCount` — gate `SIGIL_EMP_ENTITY_WINDOW`. tests: entity_window_port
pub const ENTITY_WINDOW: Region = Region { plain_base: 0x38EC, debug_base: 0x4232, plain_len: 0x8F4, debug_len: 0xD5E };

/// `PopulateSpawnedPieceCount` .. `Load_Object` — gate `SIGIL_EMP_CHILDREN`. tests: children_port
pub const CHILDREN: Region = Region { plain_base: 0x41E0, debug_base: 0x4F90, plain_len: 0x2F0, debug_len: 0x3A0 };

/// `Load_Object` .. `Plane_Buffer_Reset` — gate `SIGIL_EMP_LOAD_OBJECT`. tests: load_object_port, entity_window_port
pub const LOAD_OBJECT: Region = Region { plain_base: 0x44D0, debug_base: 0x5330, plain_len: 0x88, debug_len: 0x88 };

/// `Plane_Buffer_Reset` .. `Tile_Cache_GetTile` — gate `SIGIL_EMP_PLANE_BUFFER`. tests: plane_buffer_port
pub const PLANE_BUFFER: Region = Region { plain_base: 0x4558, debug_base: 0x53B8, plain_len: 0x328, debug_len: 0x378 };

/// `Tile_Cache_GetTile` .. `Collision_GetType` — gate `SIGIL_EMP_TILE_CACHE`. tests: tile_cache_port
pub const TILE_CACHE: Region = Region { plain_base: 0x4880, debug_base: 0x5730, plain_len: 0xCE0, debug_len: 0xE30 };

/// `Collision_GetType` .. `Collision_ProbeDown` — gate `SIGIL_EMP_COLLISION_LOOKUP`. tests: collision_lookup_port
pub const COLLISION_LOOKUP: Region = Region { plain_base: 0x5560, debug_base: 0x6560, plain_len: 0x70, debug_len: 0x70 };

/// `Section_Init` .. `Camera_Init` — gate `SIGIL_EMP_SECTION`. tests: section_port
pub const SECTION: Region = Region { plain_base: 0x5AC4, debug_base: 0x6AC4, plain_len: 0x42C, debug_len: 0x48C };

/// `Camera_Init` .. `Parallax_Init` — gate `SIGIL_EMP_CAMERA`. tests: camera_port
pub const CAMERA: Region = Region { plain_base: 0x5EF0, debug_base: 0x6F50, plain_len: 0x1D0, debug_len: 0x1E0 };

/// `Parallax_Init` .. `Raster_Install` — gate `SIGIL_EMP_PARALLAX`. tests: parallax_port
pub const PARALLAX: Region = Region { plain_base: 0x60C0, debug_base: 0x7130, plain_len: 0x5EE, debug_len: 0x5EE };

/// `Raster_Install` .. `Palette_LoadSection` — gate `SIGIL_EMP_RASTER`. tests: raster_port
pub const RASTER: Region = Region { plain_base: 0x66AE, debug_base: 0x771E, plain_len: 0x1E4, debug_len: 0x1E4 };

/// `Palette_LoadSection` .. `Level_LoadArt` — gate `SIGIL_EMP_PALETTE`. tests: palette_port
pub const PALETTE: Region = Region { plain_base: 0x6892, debug_base: 0x7902, plain_len: 0x4CE, debug_len: 0x4CE };

/// `Level_LoadArt` .. `PageIn_Process` — gate `SIGIL_EMP_LOAD_ART`. tests: load_art_port
pub const LOAD_ART: Region = Region { plain_base: 0x6D60, debug_base: 0x7DD0, plain_len: 0x90, debug_len: 0x90 };

/// `PageIn_Process` .. `PageCache_Init`.
pub const PAGE_IN: Region = Region { plain_base: 0x6DF0, debug_base: 0x7E60, plain_len: 0x2E4, debug_len: 0x456 };

/// `PageCache_Init` .. `BG_Init`.
pub const PAGE_CACHE: Region = Region { plain_base: 0x70D4, debug_base: 0x82B6, plain_len: 0x47C, debug_len: 0xB7A };

/// `BG_Init` .. `BgAnim_Init` — gate `SIGIL_EMP_BG`. tests: bg_port
pub const BG: Region = Region { plain_base: 0x7550, debug_base: 0x8E30, plain_len: 0xE0, debug_len: 0x140 };

/// `BgAnim_Init` .. start + 0x9E plain / 0x158 debug (literal — no end symbol) — gate `SIGIL_EMP_BG_ANIM`. tests: bg_anim_port
pub const BG_ANIM: Region = Region { plain_base: 0x7630, debug_base: 0x8F70, plain_len: 0x9E, debug_len: 0x158 };

/// `CompressionSelfTest` .. `Sound_PostByte` (debug-only region; plain empty at `Sound_PostByte`) — gate `SIGIL_EMP_COMPRESSION_SELFTEST`. tests: compression_selftest_port
pub const COMPRESSION_SELFTEST: Region = Region { plain_base: 0x76CE, debug_base: 0x90C8, plain_len: 0x0, debug_len: 0xDE8 };

/// `Sound_PostByte` .. start + 0x2A8 plain / 0x452 debug (literal — no end symbol) — gate `SIGIL_EMP_SOUND_API`. tests: sound_api_port
pub const SOUND_API: Region = Region { plain_base: 0x76CE, debug_base: 0x9EB0, plain_len: 0x2A8, debug_len: 0x452 };

/// `TestSolid_Init` .. `ObjDef_PathSwap` plain / `TestParticle` debug — gate `SIGIL_EMP_TEST_OBJECTS`. tests: test_objects_port
pub const TEST_SOLID: Region = Region { plain_base: 0x118E0, debug_base: 0x11DEC, plain_len: 0x12, debug_len: 0x14 };

/// `TestParticle` .. `TestEmitter` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_OBJECTS`. tests: test_objects_port
pub const TEST_PARTICLE: Region = Region { plain_base: 0x118F2, debug_base: 0x11E00, plain_len: 0x0, debug_len: 0x58 };

/// `TestStatic_Main` .. `TestSolid_Init` plain / `TestAnimated` debug — gate `SIGIL_EMP_TEST_STATIC`. tests: test_g1_objects_port
pub const TEST_STATIC: Region = Region { plain_base: 0x118D0, debug_base: 0x11AA0, plain_len: 0x10, debug_len: 0x10 };

/// `TestAnimated` .. `TestPlayer` (debug-only region; plain empty at `TestSolid_Init`) — gate `SIGIL_EMP_TEST_ANIMATED`. tests: test_g1_objects_port
pub const TEST_ANIMATED: Region = Region { plain_base: 0x118E0, debug_base: 0x11AB0, plain_len: 0x0, debug_len: 0x60 };

/// `TestEmitter` .. `TestChildPart` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_EMITTER`. tests: test_g2_objects_port
pub const TEST_EMITTER: Region = Region { plain_base: 0x118F2, debug_base: 0x11E58, plain_len: 0x0, debug_len: 0x5E };

/// `TestStressEmitter` .. `TestChurnObj` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_STRESS_EMITTER`. tests: test_g2_objects_port
pub const TEST_STRESS_EMITTER: Region = Region { plain_base: 0x118F2, debug_base: 0x11FF0, plain_len: 0x0, debug_len: 0x60 };

/// `TestChurnObj` .. `ObjDef_PathSwap` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_CHURN`. tests: test_g2_objects_port
pub const TEST_CHURN: Region = Region { plain_base: 0x118F2, debug_base: 0x12050, plain_len: 0x0, debug_len: 0x7C };

/// `TestChildPart` .. `TestStressEmitter` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_PARENT`. tests: test_g3_objects_port
pub const TEST_PARENT: Region = Region { plain_base: 0x118F2, debug_base: 0x11EB6, plain_len: 0x0, debug_len: 0x13A };

/// `TestPlayer` .. `TestEnemy_Init` (debug-only region; plain empty at `TestSolid_Init`) — gate `SIGIL_EMP_TEST_PLAYER`. tests: test_g4_final_objects_port
pub const TEST_PLAYER: Region = Region { plain_base: 0x118E0, debug_base: 0x11B10, plain_len: 0x0, debug_len: 0x294 };

/// `TestEnemy_Init` .. `TestSolid_Init` (debug-only region; plain empty at `TestSolid_Init`) — gate `SIGIL_EMP_TEST_ENEMY`. tests: test_g4_final_objects_port
pub const TEST_ENEMY: Region = Region { plain_base: 0x118E0, debug_base: 0x11DA4, plain_len: 0x0, debug_len: 0x48 };

/// `ObjDef_PathSwap` .. `DeformTable_Zero` — gate `SIGIL_EMP_PATH_SWAP`. tests: test_g4_final_objects_port
pub const PATH_SWAP: Region = Region { plain_base: 0x118F2, debug_base: 0x120CC, plain_len: 0x92, debug_len: 0xFC };

/// `DeformTable_Zero` .. `ObjDef_Static` — gate `SIGIL_EMP_PARALLAX_CONFIGS`. tests: parallax_configs_port
pub const PARALLAX_CONFIGS: Region = Region { plain_base: 0x11984, debug_base: 0x121C8, plain_len: 0xDDC, debug_len: 0xDD0 };

/// `Map_TestObj` .. `Map_DustSpindash` — gate `SIGIL_EMP_TEST_MAPPINGS`. tests: test_mappings_port
pub const TEST_MAPPINGS: Region = Region { plain_base: 0x272D0, debug_base: 0x27AF2, plain_len: 0x30, debug_len: 0x30 };

/// `Map_DustSpindash` .. `Map_DustSpindash_End` — gate `SIGIL_EMP_DUST_DATA`.
pub const DUST_DATA: Region = Region { plain_base: 0x27300, debug_base: 0x27B22, plain_len: 0xBDA, debug_len: 0xBDA };

/// `Ani_Sonic` .. `Ani_Sonic_End` — gate `SIGIL_EMP_SONIC_ANIMS`. tests: sonic_anims_port
pub const SONIC_ANIMS: Region = Region { plain_base: 0x27EE0, debug_base: 0x286FC, plain_len: 0x10A, debug_len: 0x10A };

/// `Ani_Tails` .. `Ani_Tails_End` — gate `SIGIL_EMP_TAILS_ANIMS`. tests: sonic_anims_port
pub const TAILS_ANIMS: Region = Region { plain_base: 0x27FEA, debug_base: 0x28810, plain_len: 0x1BC, debug_len: 0x1BC };

/// `Ani_Knuckles` .. `Ani_Knuckles_End` — gate `SIGIL_EMP_KNUCKLES_ANIMS`. tests: sonic_anims_port
pub const KNUCKLES_ANIMS: Region = Region { plain_base: 0x281A6, debug_base: 0x289CC, plain_len: 0x16C, debug_len: 0x16C };

/// `Map_Tails` .. `Map_Tails_End` — gate `SIGIL_EMP_TAILS_DATA`. tests: collision_data_port
pub const TAILS_DATA: Region = Region { plain_base: 0x5C320, debug_base: 0x5DD70, plain_len: 0x20F5E, debug_len: 0x20F5E };

/// `Map_Knuckles` .. `Map_Knuckles_End` — gate `SIGIL_EMP_KNUCKLES_DATA`. tests: collision_data_port
pub const KNUCKLES_DATA: Region = Region { plain_base: 0x7D27E, debug_base: 0x7ECCE, plain_len: 0x226C8, debug_len: 0x226C8 };

/// `Ani_Particle` .. `Ani_Particle_End` (debug-only region; plain empty at `Ani_DustSpindash`) — gate `SIGIL_EMP_PARTICLE_ANIMS`. tests: particle_anims_port, test_objects_port
pub const PARTICLE_ANIMS: Region = Region { plain_base: 0x28312, debug_base: 0x28B38, plain_len: 0x0, debug_len: 0x8 };

/// `Ani_DustSpindash` .. `Ani_DustSpindash_End` — gate `SIGIL_EMP_DUST_ANIMS`.
pub const DUST_ANIMS: Region = Region { plain_base: 0x28312, debug_base: 0x28B40, plain_len: 0x14, debug_len: 0x14 };

/// `OJZ_Sec0_TypeTable` .. `OJZ_Act_Pool_Page0`. tests: ojz_run_a_port
pub const ENTITY_DATA: Region = Region { plain_base: 0x12798, debug_base: 0x12FD0, plain_len: 0x170, debug_len: 0x170 };

/// `OJZ_Act_Pool_Page0` .. `OJZ_Act1_Descriptor`. tests: ojz_run_a_port
pub const OJZ_ACT_POOL: Region = Region { plain_base: 0x12908, debug_base: 0x13140, plain_len: 0x2F16, debug_len: 0x2F20 };

/// `OJZ_Act1_Descriptor` .. `OJZ_Sec0_Blocks` — gate `SIGIL_EMP_ACT_DESCRIPTOR`. tests: act_descriptor_port
pub const ACT_DESCRIPTOR: Region = Region { plain_base: 0x1581E, debug_base: 0x16060, plain_len: 0x27A, debug_len: 0x280 };

/// `OJZ_Sec0_Blocks` .. `OJZ_Sec0_LocalMap`. tests: ojz_run_b_port
pub const SEC_BLOCK_BLOBS: Region = Region { plain_base: 0x15A98, debug_base: 0x162E0, plain_len: 0xB2D8, debug_len: 0xB2CC };

/// `OJZ_Sec0_LocalMap` .. `OJZ_Palette`. tests: ojz_run_b_port
pub const SEC_LOCAL_MAPS: Region = Region { plain_base: 0x20D70, debug_base: 0x215AC, plain_len: 0xCD0, debug_len: 0xCC2 };

/// `OJZ_Palette` .. `BgAnim_Table`. tests: ojz_run_b_port
pub const OJZ_ACT_ASSETS: Region = Region { plain_base: 0x21A40, debug_base: 0x2226E, plain_len: 0x5882, debug_len: 0x5882 };

/// `BgAnim_Table` .. `Map_TestObj`. tests: ojz_run_b_port
pub const OJZ_BG_ANIM: Region = Region { plain_base: 0x272C2, debug_base: 0x27AF0, plain_len: 0xE, debug_len: 0x2 };

/// `ObjDef_Static` .. `OJZ_Sec0_TypeTable` — gate `SIGIL_EMP_OBJDEFS`. tests: objdef_port
pub const OBJDEFS: Region = Region { plain_base: 0x12760, debug_base: 0x12F98, plain_len: 0x38, debug_len: 0x38 };

/// `GameState_ObjectTest_Init` .. `GameState_OJZScroll_Init` (debug-only region; plain empty at `GameState_OJZScroll_Init`) — gate `SIGIL_EMP_OBJECT_TEST_STATE`. tests: test_t1_harness_states_port
pub const OBJECT_TEST_STATE: Region = Region { plain_base: 0x9F950, debug_base: 0xA13A0, plain_len: 0x0, debug_len: 0x384 };

/// `GameState_OJZScroll_Init` .. `Replay_OJZ_Fixture` — gate `SIGIL_EMP_OJZ_SCROLL_TEST`. tests: test_t1_harness_states_port
pub const OJZ_SCROLL_TEST: Region = Region { plain_base: 0x9F950, debug_base: 0xA1724, plain_len: 0x580, debug_len: 0x65C };

/// `Replay_OJZ_Fixture` .. `BusError`.
pub const REPLAY_FIXTURE: Region = Region { plain_base: 0x9FED0, debug_base: 0xA1D80, plain_len: 0x260, debug_len: 0x260 };

/// `BusError` .. `EndOfRom` — gate `SIGIL_EMP_ERROR_HANDLER`. tests: error_handler_port
pub const ERROR_HANDLER: Region = Region { plain_base: 0xA0130, debug_base: 0xA1FE0, plain_len: 0x10B0, debug_len: 0x10B0 };

/// `Dac_Temp_Blip` .. start + 0xF8BC plain / 0xF8BC debug (literal — no end symbol) — gate `SIGIL_EMP_DAC`. tests: dac_bank_port
pub const DAC_BANKS: Region = Region { plain_base: 0x48000, debug_base: 0x48000, plain_len: 0xF8BC, debug_len: 0xF8BC };

/// `Song_MovingTrucks` .. start + 0x34E8 plain / 0x4F38 debug (literal — no end symbol) — gate `SIGIL_EMP_MT`. tests: mt_bank_port
pub const MT_BANK_BLOB: Region = Region { plain_base: 0x58630, debug_base: 0x58630, plain_len: 0x34E8, debug_len: 0x4F38 };

/// `Sfx_33` .. start + 0x7FE plain / 0x7FE debug (literal — no end symbol) — gate `SIGIL_EMP_SFX`. tests: sfx_bank_port
pub const SFX_BANK_BLOB: Region = Region { plain_base: 0x5BB20, debug_base: 0x5D570, plain_len: 0x7FE, debug_len: 0x7FE };

/// `SoundTablesZ80_Head` .. start + 0x630 plain / 0x630 debug (literal — no end symbol) — gate `SIGIL_EMP_SOUNDBANKHEAD`. tests: soundbankhead_port
pub const SOUNDBANKHEAD: Region = Region { plain_base: 0x58000, debug_base: 0x58000, plain_len: 0x630, debug_len: 0x630 };

/// `EndOfRom` .. start + 0x0 plain / 0x0 debug (literal — no end symbol) — gate `SIGIL_EMP_EPILOGUE`. tests: m1d_rom, m1d_debug_rom
pub const EPILOGUE: Region = Region { plain_base: 0xA11E0, debug_base: 0xA3090, plain_len: 0x0, debug_len: 0x0 };

/// `ObjCodeBase` .. start + 0x2 plain / 0x2 debug (literal — no end symbol) — gate `SIGIL_EMP_OBJCODEBASE`. tests: m1d_rom, m1d_debug_rom
pub const OBJCODEBASE: Region = Region { plain_base: 0x10000, debug_base: 0x10000, plain_len: 0x2, debug_len: 0x2 };

/// `Player_Init` .. `PState_Ground` — gate `SIGIL_EMP_PLAYER_COMMON`. tests: test_p1_player_port
pub const PLAYER_COMMON: Region = Region { plain_base: 0x10002, debug_base: 0x10002, plain_len: 0x60E, debug_len: 0x71E };

/// `CharDef_Sonic` .. `CharDef_Tails` — gate `SIGIL_EMP_SONIC`. tests: test_p1_player_port
pub const SONIC: Region = Region { plain_base: 0x11570, debug_base: 0x11680, plain_len: 0x40, debug_len: 0x40 };

/// `CharDef_Tails` .. `CharDef_Knuckles` — gate `SIGIL_EMP_TAILS`. tests: test_p1_player_port
pub const TAILS: Region = Region { plain_base: 0x115B0, debug_base: 0x116C0, plain_len: 0x36, debug_len: 0x36 };

/// `CharDef_Knuckles` .. `CharacterDefs` — gate `SIGIL_EMP_KNUCKLES`. tests: test_p1_player_port
pub const KNUCKLES: Region = Region { plain_base: 0x115E6, debug_base: 0x116F6, plain_len: 0x3A, debug_len: 0x3A };

/// `CharacterDefs` .. `TailsAppendage_Refresh` — gate `SIGIL_EMP_CHARACTERS`. tests: test_p1_player_port
pub const CHARACTERS: Region = Region { plain_base: 0x11620, debug_base: 0x11730, plain_len: 0x4A, debug_len: 0xB0 };

/// `TailsAppendage_Refresh` .. `DustPuff_Spawn` — gate `SIGIL_EMP_TAILS_APPENDAGE`. tests: test_p1_player_port
pub const TAILS_APPENDAGE: Region = Region { plain_base: 0x1166A, debug_base: 0x117E0, plain_len: 0x11C, debug_len: 0x174 };

/// `DustPuff_Spawn` .. `Dust_Tick` — gate `SIGIL_EMP_DUST_PUFF`.
pub const DUST_PUFF: Region = Region { plain_base: 0x11786, debug_base: 0x11954, plain_len: 0x46, debug_len: 0x46 };

/// `Dust_Tick` .. `TestStatic_Main` — gate `SIGIL_EMP_DUST_SPINDASH`.
pub const DUST_SPINDASH: Region = Region { plain_base: 0x117CC, debug_base: 0x1199A, plain_len: 0x104, debug_len: 0x106 };

/// `PState_Ground` .. `PState_Air` — gate `SIGIL_EMP_PLAYER_GROUND`. tests: test_p2_player_states_port
pub const PLAYER_GROUND: Region = Region { plain_base: 0x10610, debug_base: 0x10720, plain_len: 0x490, debug_len: 0x490 };

/// `PState_Air` .. `PState_Spindash` — gate `SIGIL_EMP_PLAYER_AIR`. tests: test_p2_player_states_port
pub const PLAYER_AIR: Region = Region { plain_base: 0x10AA0, debug_base: 0x10BB0, plain_len: 0x350, debug_len: 0x350 };

/// `PState_Spindash` .. `PState_Fly` — gate `SIGIL_EMP_PLAYER_SPINDASH`. tests: test_p2_player_states_port
pub const PLAYER_SPINDASH: Region = Region { plain_base: 0x10DF0, debug_base: 0x10F00, plain_len: 0x9C, debug_len: 0x9C };

/// `PState_Fly` .. `PState_Glide` — gate `SIGIL_EMP_PLAYER_FLY`. tests: test_p2_player_states_port
pub const PLAYER_FLY: Region = Region { plain_base: 0x10E8C, debug_base: 0x10F9C, plain_len: 0x132, debug_len: 0x134 };

/// `PState_Glide` .. `Climb_WallDist` — gate `SIGIL_EMP_PLAYER_GLIDE`. tests: test_p2_player_states_port
pub const PLAYER_GLIDE: Region = Region { plain_base: 0x10FBE, debug_base: 0x110D0, plain_len: 0x2B0, debug_len: 0x2B0 };

/// `Climb_WallDist` .. `CharDef_Sonic` — gate `SIGIL_EMP_PLAYER_CLIMB`. tests: test_p2_player_states_port
pub const PLAYER_CLIMB: Region = Region { plain_base: 0x1126E, debug_base: 0x11380, plain_len: 0x302, debug_len: 0x300 };

/// `Collision_ProbeDown` .. `Section_Init` — gate `SIGIL_EMP_PLAYER_SENSORS`. tests: test_p4_player_sensors_port
pub const PLAYER_SENSORS: Region = Region { plain_base: 0x55D0, debug_base: 0x65D0, plain_len: 0x4F4, debug_len: 0x4F4 };

// ── Symbols (manifest order) ──

/// `TestStatic_Main`. tests: objdef_port
pub const TEST_STATIC_MAIN: Pin = Pin { plain: 0x118D0, debug: 0x11AA0 };

/// `TestSolid_Init`. tests: objdef_port
pub const TEST_SOLID_INIT: Pin = Pin { plain: 0x118E0, debug: 0x11DEC };

/// `TestEnemy_Init` — debug-shape consumer only (`debug_only`). tests: objdef_port
pub const TEST_ENEMY_INIT: u32 = 0x11DA4;

/// `TestParent` — debug-shape consumer only (`debug_only`). tests: objdef_port
pub const TEST_PARENT_LABEL: u32 = 0x11F40;

/// `Map_TestObj`. tests: objdef_port
pub const MAP_TEST_OBJ: Pin = Pin { plain: 0x272D0, debug: 0x27AF2 };

/// `Map_Sonic`. tests: test_g1_objects_port
pub const MAP_SONIC: Pin = Pin { plain: 0x2A530, debug: 0x2AD60 };

/// `DPLC_Sonic`. tests: test_g1_objects_port
pub const DPLC_SONIC: Pin = Pin { plain: 0x2C1B0, debug: 0x2C9E0 };

/// `Art_Sonic`. tests: test_g1_objects_port
pub const ART_SONIC: Pin = Pin { plain: 0x2CAF0, debug: 0x2D320 };

/// `CreateEffect_Normal`. tests: test_g2_objects_port
pub const CREATE_EFFECT_NORMAL: Pin = Pin { plain: 0x4436, debug: 0x5296 };

/// `CreateChild_Normal`. tests: test_g3_objects_port
pub const CREATE_CHILD_NORMAL: Pin = Pin { plain: 0x420C, debug: 0x4FBC };

/// `DeleteChildren`. tests: test_g3_objects_port
pub const DELETE_CHILDREN: Pin = Pin { plain: 0x4418, debug: 0x5278 };

/// `GetSineCosine`. tests: test_g3_objects_port
pub const GET_SINE_COSINE: Pin = Pin { plain: 0x27E0, debug: 0x2A80 };

/// `EntryPoint`. tests: m1c_vector_table
pub const ENTRY_POINT: Pin = Pin { plain: 0x200, debug: 0x200 };

/// `BusError` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const BUS_ERROR: u32 = 0xA1FE0;

/// `AddressError` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ADDRESS_ERROR: u32 = 0xA1FF8;

/// `IllegalInstr` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ILLEGAL_INSTR: u32 = 0xA2014;

/// `ZeroDivide` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ZERO_DIVIDE: u32 = 0xA2036;

/// `ChkInstr` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const CHK_INSTR: u32 = 0xA2050;

/// `TrapvInstr` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const TRAPV_INSTR: u32 = 0xA206E;

/// `PrivilegeViol` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const PRIVILEGE_VIOL: u32 = 0xA208E;

/// `Trace` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const TRACE: u32 = 0xA20B0;

/// `Line1010Emu` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const LINE1010_EMU: u32 = 0xA20C4;

/// `Line1111Emu` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const LINE1111_EMU: u32 = 0xA20E4;

/// `ErrorExcept` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ERROR_EXCEPT: u32 = 0xA2104;

/// `ErrorTrap` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ERROR_TRAP: u32 = 0xA2122;

/// `VBlank_Handler`. tests: m1c_vector_table
pub const V_BLANK_HANDLER: Pin = Pin { plain: 0x21D0, debug: 0x22B0 };

/// `HBlank_Vector_Slot`. tests: hblank_port, m1c_vector_table
pub const H_BLANK_VECTOR_SLOT: Pin = Pin { plain: 0xFFFFB3A2, debug: 0xFFFFB430 };

/// `VDP_Shadow_Table`. tests: vdp_init_port
pub const VDP_SHADOW_TABLE: Pin = Pin { plain: 0xFFFF800E, debug: 0xFFFF800E };

/// `BootData_VDPRegs`. tests: vdp_init_port
pub const BOOT_DATA_VDP_REGS: Pin = Pin { plain: 0x3BA, debug: 0x3BA };

/// `Ctrl_1_Held`. tests: controllers_port
pub const CTRL_1_HELD: Pin = Pin { plain: 0xFFFF8028, debug: 0xFFFF8028 };

/// `Ctrl_1_Held_Raw`. tests: controllers_port
pub const CTRL_1_HELD_RAW: Pin = Pin { plain: 0xFFFFB790, debug: 0xFFFFB81E };

/// `Ctrl_2_Held`. tests: vblank_port
pub const CTRL_2_HELD: Pin = Pin { plain: 0xFFFF802A, debug: 0xFFFF802A };

/// `Ctrl_1_Ext_Held`. tests: vblank_port
pub const CTRL_1_EXT_HELD: Pin = Pin { plain: 0xFFFF802E, debug: 0xFFFF802E };

/// `Ctrl_2_Ext_Held`. tests: vblank_port
pub const CTRL_2_EXT_HELD: Pin = Pin { plain: 0xFFFF8030, debug: 0xFFFF8030 };

/// `Ctrl_2_Held_Raw`. tests: vblank_port
pub const CTRL_2_HELD_RAW: Pin = Pin { plain: 0xFFFFB791, debug: 0xFFFFB81F };

/// `Ctrl_1_Ext_Held_Raw`. tests: vblank_port
pub const CTRL_1_EXT_HELD_RAW: Pin = Pin { plain: 0xFFFFB792, debug: 0xFFFFB820 };

/// `Ctrl_2_Ext_Held_Raw`. tests: vblank_port
pub const CTRL_2_EXT_HELD_RAW: Pin = Pin { plain: 0xFFFFB793, debug: 0xFFFFB821 };

/// `VSync_Wait`. tests: game_loop_port, load_art_port
pub const V_SYNC_WAIT: Pin = Pin { plain: 0x2386, debug: 0x246E };

/// `Sound_DrainSfxRing`. tests: game_loop_port, load_art_port
pub const SOUND_DRAIN_SFX_RING: Pin = Pin { plain: 0x783A, debug: 0xA1C6 };

/// `Game_State`. tests: game_loop_port, load_art_port
pub const GAME_STATE: Pin = Pin { plain: 0xFFFF8008, debug: 0xFFFF8008 };

/// `Input_Tick`. tests: game_loop_port, game_debug_port
pub const INPUT_TICK: Pin = Pin { plain: 0x2520, debug: 0x2602 };

/// `Cache_Left_Col`. tests: collision_lookup_port, section_port
pub const CACHE_LEFT_COL: Pin = Pin { plain: 0xFFFFAB50, debug: 0xFFFFABDE };

/// `Draw_TileColumn`. tests: section_port
pub const DRAW_TILE_COLUMN: Pin = Pin { plain: 0x4560, debug: 0x53C0 };

/// `Draw_TileRow_FromCache`. tests: section_port
pub const DRAW_TILE_ROW_FROM_CACHE: Pin = Pin { plain: 0x46B4, debug: 0x5514 };

/// `EntityWindow_Init`. tests: section_port
pub const ENTITY_WINDOW_INIT: Pin = Pin { plain: 0x3CAA, debug: 0x496E };

/// `Section_Plane_Dirty`. tests: section_port
pub const SECTION_PLANE_DIRTY: Pin = Pin { plain: 0xFFFFABC4, debug: 0xFFFFAC52 };

/// `Section_Right_Col_Written`. tests: section_port
pub const SECTION_RIGHT_COL_WRITTEN: Pin = Pin { plain: 0xFFFFABC6, debug: 0xFFFFAC54 };

/// `Section_Left_Col_Written`. tests: section_port
pub const SECTION_LEFT_COL_WRITTEN: Pin = Pin { plain: 0xFFFFABC8, debug: 0xFFFFAC56 };

/// `Section_Top_Row_Written`. tests: section_port
pub const SECTION_TOP_ROW_WRITTEN: Pin = Pin { plain: 0xFFFFABC0, debug: 0xFFFFAC4E };

/// `Section_Bottom_Row_Written`. tests: section_port
pub const SECTION_BOTTOM_ROW_WRITTEN: Pin = Pin { plain: 0xFFFFABC2, debug: 0xFFFFAC50 };

/// `Cache_Head_Col`. tests: section_port
pub const CACHE_HEAD_COL: Pin = Pin { plain: 0xFFFFAB52, debug: 0xFFFFABE0 };

/// `Cache_Top_Row`. tests: section_port
pub const CACHE_TOP_ROW: Pin = Pin { plain: 0xFFFFAB54, debug: 0xFFFFABE2 };

/// `Cache_Bottom_Row`. tests: section_port
pub const CACHE_BOTTOM_ROW: Pin = Pin { plain: 0xFFFFAB56, debug: 0xFFFFABE4 };

/// `Cache_Origin_Col`. tests: section_port
pub const CACHE_ORIGIN_COL: Pin = Pin { plain: 0xFFFFAB58, debug: 0xFFFFABE6 };

/// `Cache_Origin_Row`. tests: section_port
pub const CACHE_ORIGIN_ROW: Pin = Pin { plain: 0xFFFFAB5A, debug: 0xFFFFABE8 };

/// `Plane_Buffer_Ptr`. tests: section_port
pub const PLANE_BUFFER_PTR: Pin = Pin { plain: 0xFFFFAA3C, debug: 0xFFFFAACA };

/// `Plane_Buffer`. tests: plane_buffer_port
pub const PLANE_BUFFER_BASE: Pin = Pin { plain: 0xFFFFA43C, debug: 0xFFFFA4CA };

/// `Tile_Cache_Nametable`. tests: section_port
pub const TILE_CACHE_NAMETABLE: Pin = Pin { plain: 0xFFFF0000, debug: 0xFFFF0000 };

/// `Tile_Cache_Collision`. tests: tile_cache_port, collision_lookup_port
pub const TILE_CACHE_COLLISION: Pin = Pin { plain: 0xFFFF2580, debug: 0xFFFF2580 };

/// `Frame_Counter`. tests: tile_cache_port
pub const FRAME_COUNTER: Pin = Pin { plain: 0xFFFF8002, debug: 0xFFFF8002 };

/// `Logic_Tick`. tests: game_loop_port, bg_anim_port
pub const LOGIC_TICK: Pin = Pin { plain: 0xFFFF8004, debug: 0xFFFF8004 };

/// `Block_Stage_Keys`. tests: tile_cache_port
pub const BLOCK_STAGE_KEYS: Pin = Pin { plain: 0xFFFFAB7E, debug: 0xFFFFAC0C };

/// `Block_Stage_Next`. tests: tile_cache_port
pub const BLOCK_STAGE_NEXT: Pin = Pin { plain: 0xFFFFABBE, debug: 0xFFFFAC4C };

/// `Block_Stage_Buffers`. tests: tile_cache_port
pub const BLOCK_STAGE_BUFFERS: Pin = Pin { plain: 0xFFFF3842, debug: 0xFFFF3842 };

/// `Block_Stage_Ptrs`. tests: tile_cache_port
pub const BLOCK_STAGE_PTRS: Pin = Pin { plain: 0xFFFFB3A8, debug: 0xFFFFB436 };

/// `Block_Stage_ZeroPage`. tests: tile_cache_port
pub const BLOCK_STAGE_ZERO_PAGE: Pin = Pin { plain: 0xFFFFB42C, debug: 0xFFFFB4BA };

/// `Cache_Fill_Last_Frame`. tests: tile_cache_port
pub const CACHE_FILL_LAST_FRAME: Pin = Pin { plain: 0xFFFFAB5C, debug: 0xFFFFABEA };

/// `Cache_Fill_Budget`. tests: tile_cache_port
pub const CACHE_FILL_BUDGET: Pin = Pin { plain: 0xFFFFAB66, debug: 0xFFFFABF4 };

/// `Cache_Fill_Resume_Col`. tests: tile_cache_port
pub const CACHE_FILL_RESUME_COL: Pin = Pin { plain: 0xFFFFAB5E, debug: 0xFFFFABEC };

/// `Cache_Fill_Resume_Row`. tests: tile_cache_port
pub const CACHE_FILL_RESUME_ROW: Pin = Pin { plain: 0xFFFFAB60, debug: 0xFFFFABEE };

/// `Cache_Fill_RowResume_Row`. tests: tile_cache_port
pub const CACHE_FILL_ROW_RESUME_ROW: Pin = Pin { plain: 0xFFFFAB68, debug: 0xFFFFABF6 };

/// `Cache_Fill_RowResume_Col`. tests: tile_cache_port
pub const CACHE_FILL_ROW_RESUME_COL: Pin = Pin { plain: 0xFFFFAB6A, debug: 0xFFFFABF8 };

/// `Cache_Fill_Rows_Left`. tests: tile_cache_port
pub const CACHE_FILL_ROWS_LEFT: Pin = Pin { plain: 0xFFFFAB6C, debug: 0xFFFFABFA };

/// `Cache_Prev_Cam_Row`. tests: tile_cache_port
pub const CACHE_PREV_CAM_ROW: Pin = Pin { plain: 0xFFFFAB6E, debug: 0xFFFFABFC };

/// `Cache_Prev_Cam_X`. tests: tile_cache_port
pub const CACHE_PREV_CAM_X: Pin = Pin { plain: 0xFFFFAB70, debug: 0xFFFFABFE };

/// `Cache_H_Pfx_Dir`. tests: tile_cache_port
pub const CACHE_H_PFX_DIR: Pin = Pin { plain: 0xFFFFAB72, debug: 0xFFFFAC00 };

/// `Cache_H_Pfx_Accum`. tests: tile_cache_port
pub const CACHE_H_PFX_ACCUM: Pin = Pin { plain: 0xFFFFAB74, debug: 0xFFFFAC02 };

/// `Cache_Pfx_Row_Target`. tests: tile_cache_port
pub const CACHE_PFX_ROW_TARGET: Pin = Pin { plain: 0xFFFFAB76, debug: 0xFFFFAC04 };

/// `Cache_Pfx_Col_Target`. tests: tile_cache_port
pub const CACHE_PFX_COL_TARGET: Pin = Pin { plain: 0xFFFFAB78, debug: 0xFFFFAC06 };

/// `Cache_Pfx_Skip_Armed`. tests: tile_cache_port
pub const CACHE_PFX_SKIP_ARMED: Pin = Pin { plain: 0xFFFFAB7A, debug: 0xFFFFAC08 };

/// `Cache_Pfx_Lag_Flag`. tests: tile_cache_port
pub const CACHE_PFX_LAG_FLAG: Pin = Pin { plain: 0xFFFFAB7C, debug: 0xFFFFAC0A };

/// `Block_Stage_Gen`. tests: tile_cache_port
pub const BLOCK_STAGE_GEN: Pin = Pin { plain: 0xFFFFB390, debug: 0xFFFFB41E };

/// `Pfx_Memo_Row`. tests: tile_cache_port
pub const PFX_MEMO_ROW: Pin = Pin { plain: 0xFFFFB392, debug: 0xFFFFB420 };

/// `Pfx_Memo_L`. tests: tile_cache_port
pub const PFX_MEMO_L: Pin = Pin { plain: 0xFFFFB394, debug: 0xFFFFB422 };

/// `Pfx_Memo_H`. tests: tile_cache_port
pub const PFX_MEMO_H: Pin = Pin { plain: 0xFFFFB396, debug: 0xFFFFB424 };

/// `Pfx_Memo_Gen`. tests: tile_cache_port
pub const PFX_MEMO_GEN: Pin = Pin { plain: 0xFFFFB398, debug: 0xFFFFB426 };

/// `Cs_Memo_Col`. tests: tile_cache_port
pub const CS_MEMO_COL: Pin = Pin { plain: 0xFFFFB39A, debug: 0xFFFFB428 };

/// `Cs_Memo_T`. tests: tile_cache_port
pub const CS_MEMO_T: Pin = Pin { plain: 0xFFFFB39C, debug: 0xFFFFB42A };

/// `Cs_Memo_B`. tests: tile_cache_port
pub const CS_MEMO_B: Pin = Pin { plain: 0xFFFFB39E, debug: 0xFFFFB42C };

/// `Cs_Memo_Gen`. tests: tile_cache_port
pub const CS_MEMO_GEN: Pin = Pin { plain: 0xFFFFB3A0, debug: 0xFFFFB42E };

/// `S4LZ_DecompressDict`. tests: tile_cache_port
pub const S4_LZ_DECOMPRESS_DICT: Pin = Pin { plain: 0x2670, debug: 0x2800 };

/// `Player_1`. tests: collision_port, rings_port
pub const PLAYER_1: Pin = Pin { plain: 0xFFFF8D00, debug: 0xFFFF8D8E };

/// `Cheat_Flags`. tests: test_g4_final_objects_port, test_p1_player_port
pub const CHEAT_FLAGS: Pin = Pin { plain: 0xFFFFB794, debug: 0xFFFFE02A };

/// `Dynamic_Slots`. tests: collision_port
pub const DYNAMIC_SLOTS: Pin = Pin { plain: 0xFFFF8DA0, debug: 0xFFFF8E2E };

/// `Ring_Buffer`. tests: rings_port
pub const RING_BUFFER: Pin = Pin { plain: 0xFFFFAC32, debug: 0xFFFFACC0 };

/// `Ring_Count`. tests: rings_port
pub const RING_COUNT: Pin = Pin { plain: 0xFFFFAF32, debug: 0xFFFFAFC0 };

/// `Ring_HighWater`. tests: rings_port
pub const RING_HIGH_WATER: Pin = Pin { plain: 0xFFFFAF33, debug: 0xFFFFAFC1 };

/// `Ring_Add_Dropped`. tests: rings_port
pub const RING_ADD_DROPPED: Pin = Pin { plain: 0xFFFFAF34, debug: 0xFFFFAFC2 };

/// `Ring_Counter`. tests: rings_port
pub const RING_COUNTER: Pin = Pin { plain: 0xFFFFAF9E, debug: 0xFFFFB02C };

/// `Ring_Anim_Frame`. tests: rings_port
pub const RING_ANIM_FRAME: Pin = Pin { plain: 0xFFFFAFA0, debug: 0xFFFFB02E };

/// `Ring_Anim_Timer`. tests: rings_port
pub const RING_ANIM_TIMER: Pin = Pin { plain: 0xFFFFAFA1, debug: 0xFFFFB02F };

/// `Camera_X`. tests: rings_port, section_port, camera_port, bg_anim_port
pub const CAMERA_X: Pin = Pin { plain: 0xFFFFA42E, debug: 0xFFFFA4BC };

/// `Camera_Y`. tests: rings_port, section_port, camera_port, bg_anim_port
pub const CAMERA_Y: Pin = Pin { plain: 0xFFFFA432, debug: 0xFFFFA4C0 };

/// `Camera_Target`. tests: camera_port, test_g4_final_objects_port, test_t1_harness_states_port
pub const CAMERA_TARGET: Pin = Pin { plain: 0xFFFFAB4A, debug: 0xFFFFABD8 };

/// `Camera_Curl_Offset`. tests: camera_port, test_p1_player_port
pub const CAMERA_CURL_OFFSET: Pin = Pin { plain: 0xFFFFAB4C, debug: 0xFFFFABDA };

/// `Camera_Deadzone_Base`. tests: camera_port
pub const CAMERA_DEADZONE_BASE: Pin = Pin { plain: 0xFFFFAB40, debug: 0xFFFFABCE };

/// `Camera_Pan_Offset`. tests: camera_port
pub const CAMERA_PAN_OFFSET: Pin = Pin { plain: 0xFFFFAB44, debug: 0xFFFFABD2 };

/// `Camera_Hold_Frames`. tests: camera_port
pub const CAMERA_HOLD_FRAMES: Pin = Pin { plain: 0xFFFFAB4E, debug: 0xFFFFABDC };

/// `Camera_Art_Hold`. tests: camera_port, tile_cache_port
pub const CAMERA_ART_HOLD: Pin = Pin { plain: 0xFFFFAB4F, debug: 0xFFFFABDD };

/// `Dbg_Cam_Clamp_Frames` — debug-shape consumer only (`debug_only`). tests: camera_port
pub const DBG_CAM_CLAMP_FRAMES: u32 = 0xFFFF8D34;

/// `Camera_X_Max`. tests: camera_port
pub const CAMERA_X_MAX: Pin = Pin { plain: 0xFFFFAB46, debug: 0xFFFFABD4 };

/// `Camera_Y_Max`. tests: camera_port
pub const CAMERA_Y_MAX: Pin = Pin { plain: 0xFFFFAB48, debug: 0xFFFFABD6 };

/// `BgAnim_LastStep`. tests: bg_anim_port
pub const BG_ANIM_LAST_STEP: Pin = Pin { plain: 0xFFFF8C96, debug: 0xFFFF8C96 };

/// `BgAnim_Table`. tests: bg_anim_port
pub const BG_ANIM_TABLE: Pin = Pin { plain: 0x272C2, debug: 0x27AF0 };

/// `Camera_X_Biased`. tests: sprites_port
pub const CAMERA_X_BIASED: Pin = Pin { plain: 0xFFFFA436, debug: 0xFFFFA4C4 };

/// `Camera_Y_Biased`. tests: sprites_port
pub const CAMERA_Y_BIASED: Pin = Pin { plain: 0xFFFFA438, debug: 0xFFFFA4C6 };

/// `Collected_MarkRing`. tests: rings_port
pub const COLLECTED_MARK_RING: Pin = Pin { plain: 0x396E, debug: 0x4316 };

/// `EntityWindow_EntryForSection`. tests: rings_port
pub const ENTITY_WINDOW_ENTRY_FOR_SECTION: Pin = Pin { plain: 0x3B8A, debug: 0x47F8 };

/// `EntityLoaded_Clear`. tests: rings_port
pub const ENTITY_LOADED_CLEAR: Pin = Pin { plain: 0x3B76, debug: 0x4782 };

/// `Sound_PlayRing`. tests: rings_port
pub const SOUND_PLAY_RING: Pin = Pin { plain: 0x788A, debug: 0xA216 };

/// `MDDBG__ErrorHandler` — debug-shape consumer only (`debug_only`). tests: rings_port
pub const MDDBG_ERROR_HANDLER: u32 = 0xA213A;

/// `MDDBG__ErrorHandler_PagesController` — debug-shape consumer only (`debug_only`). tests: rings_port
pub const MDDBG_ERROR_HANDLER_PAGES_CONTROLLER: u32 = 0xA2F00;

/// `DMA_Critical`. tests: dma_queue_port
pub const DMA_CRITICAL: Pin = Pin { plain: 0xFFFF804A, debug: 0xFFFF804A };

/// `DMA_Critical_End`. tests: dma_queue_port
pub const DMA_CRITICAL_END: Pin = Pin { plain: 0xFFFF80BA, debug: 0xFFFF80BA };

/// `DMA_Important`. tests: dma_queue_port
pub const DMA_IMPORTANT: Pin = Pin { plain: 0xFFFF80BA, debug: 0xFFFF80BA };

/// `DMA_Important_End`. tests: dma_queue_port
pub const DMA_IMPORTANT_END: Pin = Pin { plain: 0xFFFF8162, debug: 0xFFFF8162 };

/// `DMA_Deferrable`. tests: dma_queue_port
pub const DMA_DEFERRABLE: Pin = Pin { plain: 0xFFFF8162, debug: 0xFFFF8162 };

/// `DMA_Deferrable_End`. tests: dma_queue_port
pub const DMA_DEFERRABLE_END: Pin = Pin { plain: 0xFFFF820A, debug: 0xFFFF820A };

/// `DMA_Critical_Slot`. tests: dma_queue_port
pub const DMA_CRITICAL_SLOT: Pin = Pin { plain: 0xFFFF820A, debug: 0xFFFF820A };

/// `DMA_Important_Slot`. tests: dma_queue_port
pub const DMA_IMPORTANT_SLOT: Pin = Pin { plain: 0xFFFF820C, debug: 0xFFFF820C };

/// `DMA_Deferrable_Slot`. tests: dma_queue_port
pub const DMA_DEFERRABLE_SLOT: Pin = Pin { plain: 0xFFFF820E, debug: 0xFFFF820E };

/// `DMA_Budget_Remaining`. tests: dma_queue_port
pub const DMA_BUDGET_REMAINING: Pin = Pin { plain: 0xFFFF8212, debug: 0xFFFF8212 };

/// `DMA_Enq_Bytes_Frame`. tests: bg_anim_port, dma_queue_port, dplc_port, game_loop_port, load_art_port, vblank_port
pub const DMA_ENQ_BYTES_FRAME: Pin = Pin { plain: 0xFFFF8214, debug: 0xFFFF8214 };

/// `Act_Art_Budget`. tests: load_art_port
pub const ACT_ART_BUDGET: Pin = Pin { plain: 0xFFFFB78C, debug: 0xFFFFB81A };

/// `Art_Budget_Remaining`. tests: load_art_port
pub const ART_BUDGET_REMAINING: Pin = Pin { plain: 0xFFFFB78E, debug: 0xFFFFB81C };

/// `PageIn_Pool_Pages`. tests: load_art_port
pub const PAGE_IN_POOL_PAGES: Pin = Pin { plain: 0xFFFFB780, debug: 0xFFFFB80E };

/// `PageIn_Bulk_Drain`. tests: load_art_port
pub const PAGE_IN_BULK_DRAIN: Pin = Pin { plain: 0xFFFFB77B, debug: 0xFFFFB809 };

/// `PageIn_Fully_Resident`. tests: load_art_port
pub const PAGE_IN_FULLY_RESIDENT: Pin = Pin { plain: 0xFFFFB782, debug: 0xFFFFB810 };

/// `Block_Stage_Maps`. tests: tile_cache_port
pub const BLOCK_STAGE_MAPS: Pin = Pin { plain: 0xFFFFB3E8, debug: 0xFFFFB476 };

/// `Cache_Cur_LocalMap`. tests: tile_cache_port
pub const CACHE_CUR_LOCAL_MAP: Pin = Pin { plain: 0xFFFFB428, debug: 0xFFFFB4B6 };

/// `Dbg_DMA_Enq_Capped` — debug-shape consumer only (`debug_only`). tests: bg_anim_port, dma_queue_port, dplc_port
pub const DBG_DMA_ENQ_CAPPED: u32 = 0xFFFF8D0A;

/// `DMA_Overflow_Count` — debug-shape consumer only (`debug_only`). tests: dma_queue_port
pub const DMA_OVERFLOW_COUNT: u32 = 0xFFFF8D08;

/// `Art_Staging_Buffer`. tests: load_art_port
pub const ART_STAGING_BUFFER: Pin = Pin { plain: 0xFFFF69DA, debug: 0xFFFF69DA };

/// `S4LZ_Decompress`. tests: load_art_port
pub const S4_LZ_DECOMPRESS: Pin = Pin { plain: 0x2674, debug: 0x2858 };

/// `QueueDMA_Critical`. tests: load_art_port
pub const QUEUE_DMA_CRITICAL: Pin = Pin { plain: 0x1D58, debug: 0x1E2E };

/// `BG_Init`. tests: load_art_port
pub const BG_INIT: Pin = Pin { plain: 0x7550, debug: 0x8E30 };

/// `QueueDMA_Important`. tests: dplc_port
pub const QUEUE_DMA_IMPORTANT: Pin = Pin { plain: 0x1D62, debug: 0x1E38 };

/// `QueueDMA_Deferrable`. tests: dplc_port
pub const QUEUE_DMA_DEFERRABLE: Pin = Pin { plain: 0x1D6C, debug: 0x1E42 };

/// `Object_RAM`. tests: core_port
pub const OBJECT_RAM: Pin = Pin { plain: 0xFFFF8D00, debug: 0xFFFF8D8E };

/// `System_Slots`. tests: core_port
pub const SYSTEM_SLOTS: Pin = Pin { plain: 0xFFFF9A20, debug: 0xFFFF9AAE };

/// `Effect_Slots`. tests: core_port
pub const EFFECT_SLOTS: Pin = Pin { plain: 0xFFFF9CA0, debug: 0xFFFF9D2E };

/// `Game_Paused`. tests: core_port
pub const GAME_PAUSED: Pin = Pin { plain: 0xFFFFA43A, debug: 0xFFFFA4C8 };

/// `Object_RAM_End`. tests: core_port
pub const OBJECT_RAM_END: Pin = Pin { plain: 0xFFFFA1A0, debug: 0xFFFFA22E };

/// `Dynamic_Free_Stack`. tests: core_port
pub const DYNAMIC_FREE_STACK: Pin = Pin { plain: 0xFFFFA1A0, debug: 0xFFFFA22E };

/// `Dynamic_Free_SP`. tests: core_port
pub const DYNAMIC_FREE_SP: Pin = Pin { plain: 0xFFFFA1F0, debug: 0xFFFFA27E };

/// `Effect_Free_Stack`. tests: core_port
pub const EFFECT_FREE_STACK: Pin = Pin { plain: 0xFFFFA1F2, debug: 0xFFFFA280 };

/// `Effect_Free_SP`. tests: core_port
pub const EFFECT_FREE_SP: Pin = Pin { plain: 0xFFFFA212, debug: 0xFFFFA2A0 };

/// `Dynamic_Live`. tests: core_port
pub const DYNAMIC_LIVE: Pin = Pin { plain: 0xFFFFB32A, debug: 0xFFFFB3B8 };

/// `Dynamic_Live_Count`. tests: core_port
pub const DYNAMIC_LIVE_COUNT: Pin = Pin { plain: 0xFFFFB37A, debug: 0xFFFFB408 };

/// `Dynamic_Live_Dirty`. tests: core_port
pub const DYNAMIC_LIVE_DIRTY: Pin = Pin { plain: 0xFFFFB37C, debug: 0xFFFFB40A };

/// `Dynamic_Live_Walking` — debug-shape consumer only (`debug_only`). tests: core_port, collision_port, entity_window_port
pub const DYNAMIC_LIVE_WALKING: u32 = 0xFFFFB40B;

/// `Dynamic_Live_Pending`. tests: core_port
pub const DYNAMIC_LIVE_PENDING: Pin = Pin { plain: 0xFFFFB37E, debug: 0xFFFFB40C };

/// `Dynamic_Live_Pending_Count`. tests: core_port
pub const DYNAMIC_LIVE_PENDING_COUNT: Pin = Pin { plain: 0xFFFFB38E, debug: 0xFFFFB41C };

/// `DeleteObject`. tests: animate_port, children_port
pub const DELETE_OBJECT: Pin = Pin { plain: 0x2D50, debug: 0x2FF0 };

/// `DrawRings`. tests: sprites_port
pub const DRAW_RINGS: Pin = Pin { plain: 0x37BA, debug: 0x4100 };

/// `Sprite_Table_Buffer`. tests: sprites_port
pub const SPRITE_TABLE_BUFFER: Pin = Pin { plain: 0xFFFF8298, debug: 0xFFFF8298 };

/// `Sprite_Table_Dirty`. tests: sprites_port
pub const SPRITE_TABLE_DIRTY: Pin = Pin { plain: 0xFFFF8518, debug: 0xFFFF8518 };

/// `Sprite_Emit_Active`. tests: sprites_port, buffers_port
pub const SPRITE_EMIT_ACTIVE: Pin = Pin { plain: 0xFFFF8519, debug: 0xFFFF8519 };

/// `Sprite_Bands`. tests: sprites_port
pub const SPRITE_BANDS: Pin = Pin { plain: 0xFFFFA214, debug: 0xFFFFA2A2 };

/// `Sprite_Band_Counts`. tests: sprites_port
pub const SPRITE_BAND_COUNTS: Pin = Pin { plain: 0xFFFFA414, debug: 0xFFFFA4A2 };

/// `Sprites_Rendered`. tests: sprites_port
pub const SPRITES_RENDERED: Pin = Pin { plain: 0xFFFFA41C, debug: 0xFFFFA4AA };

/// `Sprite_Cycle_Counter`. tests: sprites_port
pub const SPRITE_CYCLE_COUNTER: Pin = Pin { plain: 0xFFFFA41E, debug: 0xFFFFA4AC };

/// `SpriteMask_Y`. tests: sprites_port
pub const SPRITE_MASK_Y: Pin = Pin { plain: 0xFFFFA420, debug: 0xFFFFA4AE };

/// `SpriteMask_Height`. tests: sprites_port
pub const SPRITE_MASK_HEIGHT: Pin = Pin { plain: 0xFFFFA422, debug: 0xFFFFA4B0 };

/// `SpriteMask_After_Band`. tests: sprites_port
pub const SPRITE_MASK_AFTER_BAND: Pin = Pin { plain: 0xFFFFA424, debug: 0xFFFFA4B2 };

/// `Scanline_Band_Sprites`. tests: sprites_port
pub const SCANLINE_BAND_SPRITES: Pin = Pin { plain: 0xFFFFA426, debug: 0xFFFFA4B4 };

/// `Sound_PlaySFX`. tests: animate_port
pub const SOUND_PLAY_SFX: Pin = Pin { plain: 0x77F4, debug: 0xA13A };

/// `ObjectMoveX`. tests: test_g4_final_objects_port
pub const OBJECT_MOVE_X: Pin = Pin { plain: 0x2F5C, debug: 0x3646 };

/// `ObjCodeBase`. tests: test_objects_port
pub const OBJ_CODE_BASE: Pin = Pin { plain: 0x10000, debug: 0x10000 };

/// `Draw_Sprite`. tests: test_objects_port
pub const DRAW_SPRITE: Pin = Pin { plain: 0x2F94, debug: 0x3684 };

/// `ObjectMove`. tests: test_objects_port
pub const OBJECT_MOVE: Pin = Pin { plain: 0x2F42, debug: 0x362C };

/// `Ring_Sfx_Speaker`. tests: sound_api_port
pub const RING_SFX_SPEAKER: Pin = Pin { plain: 0xFFFFB26E, debug: 0xFFFFB2FC };

/// `Sfx_Ring_Buf`. tests: sound_api_port
pub const SFX_RING_BUF: Pin = Pin { plain: 0xFFFFB270, debug: 0xFFFFB2FE };

/// `Sfx_Ring_Wr`. tests: sound_api_port
pub const SFX_RING_WR: Pin = Pin { plain: 0xFFFFB278, debug: 0xFFFFB306 };

/// `Sfx_Ring_Rd`. tests: sound_api_port
pub const SFX_RING_RD: Pin = Pin { plain: 0xFFFFB279, debug: 0xFFFFB307 };

/// `SongTable`. tests: sound_api_port
pub const SONG_TABLE: Pin = Pin { plain: 0x5BB10, debug: 0x5D550 };

/// `SongPatchTable`. tests: sound_api_port
pub const SONG_PATCH_TABLE: Pin = Pin { plain: 0x5BB14, debug: 0x5D55C };

/// `OJZ_Palette`. tests: act_descriptor_port
pub const OJZ_PALETTE: Pin = Pin { plain: 0x21A40, debug: 0x2226E };

/// `OJZ_Act1_BG_Layout`. tests: act_descriptor_port
pub const OJZ_ACT1_BG_LAYOUT: Pin = Pin { plain: 0x21AC0, debug: 0x222EE };

/// `OJZ_Act1_BG_Tiles`. tests: act_descriptor_port
pub const OJZ_ACT1_BG_TILES: Pin = Pin { plain: 0x23AC0, debug: 0x242EE };

/// `ParallaxConfig_OJZ_Default`. tests: act_descriptor_port
pub const PARALLAX_CONFIG_OJZ_DEFAULT: Pin = Pin { plain: 0x11A84, debug: 0x122C8 };

/// `OJZ_Act_Pool_PageTable`. tests: act_descriptor_port
pub const OJZ_ACT_POOL_PAGE_TABLE: Pin = Pin { plain: 0x157CE, debug: 0x16006 };

/// `OJZ_Sec_LocalMaps`. tests: act_descriptor_port
pub const OJZ_SEC_LOCAL_MAPS: Pin = Pin { plain: 0x21A0E, debug: 0x2224A };

/// `OJZ_Sec0_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC0_BLOCKS: Pin = Pin { plain: 0x15A98, debug: 0x162E0 };

/// `OJZ_Sec1_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC1_BLOCKS: Pin = Pin { plain: 0x178CA, debug: 0x18112 };

/// `OJZ_Sec2_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC2_BLOCKS: Pin = Pin { plain: 0x18C46, debug: 0x1948E };

/// `OJZ_Sec3_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC3_BLOCKS: Pin = Pin { plain: 0x1A3DE, debug: 0x1AC26 };

/// `OJZ_Sec4_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC4_BLOCKS: Pin = Pin { plain: 0x18C46, debug: 0x1948E };

/// `OJZ_Sec5_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC5_BLOCKS: Pin = Pin { plain: 0x1B52A, debug: 0x1BD72 };

/// `OJZ_Sec6_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC6_BLOCKS: Pin = Pin { plain: 0x1C350, debug: 0x1CB98 };

/// `OJZ_Sec7_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC7_BLOCKS: Pin = Pin { plain: 0x1DF50, debug: 0x1E798 };

/// `OJZ_Sec8_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC8_BLOCKS: Pin = Pin { plain: 0x1F1C4, debug: 0x1FA0C };

/// `OJZ_Sec0_Objects`. tests: act_descriptor_port
pub const OJZ_SEC0_OBJECTS: Pin = Pin { plain: 0x1279E, debug: 0x12FD6 };

/// `OJZ_Sec0_Rings`. tests: act_descriptor_port
pub const OJZ_SEC0_RINGS: Pin = Pin { plain: 0x127A6, debug: 0x12FDE };

/// `OJZ_Sec0_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC0_TYPE_TABLE: Pin = Pin { plain: 0x12798, debug: 0x12FD0 };

/// `OJZ_Sec1_Objects`. tests: act_descriptor_port
pub const OJZ_SEC1_OBJECTS: Pin = Pin { plain: 0x127D0, debug: 0x13008 };

/// `OJZ_Sec1_Rings`. tests: act_descriptor_port
pub const OJZ_SEC1_RINGS: Pin = Pin { plain: 0x127E4, debug: 0x1301C };

/// `OJZ_Sec1_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC1_TYPE_TABLE: Pin = Pin { plain: 0x127C6, debug: 0x12FFE };

/// `OJZ_Sec2_Objects`. tests: act_descriptor_port
pub const OJZ_SEC2_OBJECTS: Pin = Pin { plain: 0x12816, debug: 0x1304E };

/// `OJZ_Sec2_Rings`. tests: act_descriptor_port
pub const OJZ_SEC2_RINGS: Pin = Pin { plain: 0x12824, debug: 0x1305C };

/// `OJZ_Sec2_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC2_TYPE_TABLE: Pin = Pin { plain: 0x1280C, debug: 0x13044 };

/// `OJZ_Sec3_Objects`. tests: act_descriptor_port
pub const OJZ_SEC3_OBJECTS: Pin = Pin { plain: 0x1285A, debug: 0x13092 };

/// `OJZ_Sec3_Rings`. tests: act_descriptor_port
pub const OJZ_SEC3_RINGS: Pin = Pin { plain: 0x1285C, debug: 0x13094 };

/// `OJZ_Sec3_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC3_TYPE_TABLE: Pin = Pin { plain: 0x12858, debug: 0x13090 };

/// `OJZ_Sec4_Objects`. tests: act_descriptor_port
pub const OJZ_SEC4_OBJECTS: Pin = Pin { plain: 0x12862, debug: 0x1309A };

/// `OJZ_Sec4_Rings`. tests: act_descriptor_port
pub const OJZ_SEC4_RINGS: Pin = Pin { plain: 0x12864, debug: 0x1309C };

/// `OJZ_Sec4_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC4_TYPE_TABLE: Pin = Pin { plain: 0x12860, debug: 0x13098 };

/// `OJZ_Sec5_Objects`. tests: act_descriptor_port
pub const OJZ_SEC5_OBJECTS: Pin = Pin { plain: 0x1289A, debug: 0x130D2 };

/// `OJZ_Sec5_Rings`. tests: act_descriptor_port
pub const OJZ_SEC5_RINGS: Pin = Pin { plain: 0x1289C, debug: 0x130D4 };

/// `OJZ_Sec5_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC5_TYPE_TABLE: Pin = Pin { plain: 0x12898, debug: 0x130D0 };

/// `OJZ_Sec6_Objects`. tests: act_descriptor_port
pub const OJZ_SEC6_OBJECTS: Pin = Pin { plain: 0x128C2, debug: 0x130FA };

/// `OJZ_Sec6_Rings`. tests: act_descriptor_port
pub const OJZ_SEC6_RINGS: Pin = Pin { plain: 0x128C4, debug: 0x130FC };

/// `OJZ_Sec6_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC6_TYPE_TABLE: Pin = Pin { plain: 0x128C0, debug: 0x130F8 };

/// `OJZ_Sec7_Objects`. tests: act_descriptor_port
pub const OJZ_SEC7_OBJECTS: Pin = Pin { plain: 0x128CA, debug: 0x13102 };

/// `OJZ_Sec7_Rings`. tests: act_descriptor_port
pub const OJZ_SEC7_RINGS: Pin = Pin { plain: 0x128CC, debug: 0x13104 };

/// `OJZ_Sec7_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC7_TYPE_TABLE: Pin = Pin { plain: 0x128C8, debug: 0x13100 };

/// `OJZ_Sec8_Objects`. tests: act_descriptor_port
pub const OJZ_SEC8_OBJECTS: Pin = Pin { plain: 0x128F2, debug: 0x1312A };

/// `OJZ_Sec8_Rings`. tests: act_descriptor_port
pub const OJZ_SEC8_RINGS: Pin = Pin { plain: 0x128F4, debug: 0x1312C };

/// `OJZ_Sec8_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC8_TYPE_TABLE: Pin = Pin { plain: 0x128F0, debug: 0x13128 };

/// `BLOCK_INDEX_SIZE`. tests: act_descriptor_port
pub const BLOCK_INDEX_SIZE: Pin = Pin { plain: 0x400, debug: 0x400 };

/// `EDGE_CLAMP`. tests: act_descriptor_port
pub const EDGE_CLAMP: Pin = Pin { plain: 0x0, debug: 0x0 };

/// `MAX_ACT_SECTIONS`. tests: act_descriptor_port
pub const MAX_ACT_SECTIONS: Pin = Pin { plain: 0x30, debug: 0x30 };

/// `SECTION_SIZE_SHIFT`. tests: act_descriptor_port
pub const SECTION_SIZE_SHIFT: Pin = Pin { plain: 0xB, debug: 0xB };

/// `Act_len`. tests: act_descriptor_port
pub const ACT_LEN: Pin = Pin { plain: 0x28, debug: 0x28 };

/// `Sec_len`. tests: act_descriptor_port
pub const SEC_LEN: Pin = Pin { plain: 0x42, debug: 0x42 };

/// `Camera_Y_Coarse_Prev`. tests: entity_window_port
pub const CAMERA_Y_COARSE_PREV: Pin = Pin { plain: 0xFFFFB0AE, debug: 0xFFFFB13C };

/// `Current_Act_Ptr`. tests: entity_window_port, section_port
pub const CURRENT_ACT_PTR: Pin = Pin { plain: 0xFFFFB26A, debug: 0xFFFFB2F8 };

/// `Entity_Window_Active`. tests: entity_window_port
pub const ENTITY_WINDOW_ACTIVE: Pin = Pin { plain: 0xFFFFAFA2, debug: 0xFFFFB030 };

/// `Entity_Window_Anchor`. tests: entity_window_port
pub const ENTITY_WINDOW_ANCHOR: Pin = Pin { plain: 0xFFFFAFA4, debug: 0xFFFFB032 };

/// `Entity_Window_OriginX`. tests: entity_window_port
pub const ENTITY_WINDOW_ORIGIN_X: Pin = Pin { plain: 0xFFFFAFA6, debug: 0xFFFFB034 };

/// `Entity_Window_OriginY`. tests: entity_window_port
pub const ENTITY_WINDOW_ORIGIN_Y: Pin = Pin { plain: 0xFFFFAFA8, debug: 0xFFFFB036 };

/// `Entity_Window_Center_ID`. tests: entity_window_port
pub const ENTITY_WINDOW_CENTER_ID: Pin = Pin { plain: 0xFFFFAFA3, debug: 0xFFFFB031 };

/// `Entity_Scan_State`. tests: entity_window_port
pub const ENTITY_SCAN_STATE: Pin = Pin { plain: 0xFFFFAF36, debug: 0xFFFFAFC4 };

/// `Entity_Loaded_Masks`. tests: entity_window_port
pub const ENTITY_LOADED_MASKS: Pin = Pin { plain: 0xFFFFAFAA, debug: 0xFFFFB038 };

/// `Entity_Mask_Scratch`. tests: entity_window_port
pub const ENTITY_MASK_SCRATCH: Pin = Pin { plain: 0xFFFFB02A, debug: 0xFFFFB0B8 };

/// `Ring_Collected_Window`. tests: entity_window_port
pub const RING_COLLECTED_WINDOW: Pin = Pin { plain: 0xFFFFB0B0, debug: 0xFFFFB13E };

/// `Ring_Collected_Park`. tests: entity_window_port
pub const RING_COLLECTED_PARK: Pin = Pin { plain: 0xFFFFB1E4, debug: 0xFFFFB272 };

/// `Collected_Park_Next`. tests: entity_window_port
pub const COLLECTED_PARK_NEXT: Pin = Pin { plain: 0xFFFFB268, debug: 0xFFFFB2F6 };

/// `RingBuffer_Clear`. tests: entity_window_port
pub const RING_BUFFER_CLEAR: Pin = Pin { plain: 0x37AC, debug: 0x40F2 };

/// `RingBuffer_Remove`. tests: entity_window_port
pub const RING_BUFFER_REMOVE: Pin = Pin { plain: 0x3778, debug: 0x40BE };

/// `Section_GetSecPtrXY`. tests: entity_window_port
pub const SECTION_GET_SEC_PTR_XY: Pin = Pin { plain: 0x5B14, debug: 0x6B14 };

/// `Section_FlatIDXY`. tests: entity_window_port
pub const SECTION_FLAT_IDXY: Pin = Pin { plain: 0x5AFA, debug: 0x6AFA };

/// `AllocDynamic`. tests: load_object_port, children_port
pub const ALLOC_DYNAMIC: Pin = Pin { plain: 0x2CD2, debug: 0x2F72 };

/// `AllocEffect`. tests: children_port
pub const ALLOC_EFFECT: Pin = Pin { plain: 0x2D36, debug: 0x2FD6 };

/// `Palette_Buffer`. tests: buffers_port
pub const PALETTE_BUFFER: Pin = Pin { plain: 0xFFFF8216, debug: 0xFFFF8216 };

/// `Hscroll_Buffer`. tests: buffers_port
pub const HSCROLL_BUFFER: Pin = Pin { plain: 0xFFFF851A, debug: 0xFFFF851A };

/// `Static_Pal_Line0`. tests: buffers_port
pub const STATIC_PAL_LINE0: Pin = Pin { plain: 0xFFFF8C9E, debug: 0xFFFF8C9E };

/// `Static_Pal_Line1`. tests: buffers_port
pub const STATIC_PAL_LINE1: Pin = Pin { plain: 0xFFFF8CAC, debug: 0xFFFF8CAC };

/// `Static_Pal_Line2`. tests: buffers_port
pub const STATIC_PAL_LINE2: Pin = Pin { plain: 0xFFFF8CBA, debug: 0xFFFF8CBA };

/// `Static_Pal_Line3`. tests: buffers_port
pub const STATIC_PAL_LINE3: Pin = Pin { plain: 0xFFFF8CC8, debug: 0xFFFF8CC8 };

/// `Static_Sprite_DMA`. tests: buffers_port
pub const STATIC_SPRITE_DMA: Pin = Pin { plain: 0xFFFF8CD6, debug: 0xFFFF8CD6 };

/// `Static_Hscroll_Cell`. tests: buffers_port
pub const STATIC_HSCROLL_CELL: Pin = Pin { plain: 0xFFFF8CE4, debug: 0xFFFF8CE4 };

/// `Static_Hscroll_Line`. tests: buffers_port
pub const STATIC_HSCROLL_LINE: Pin = Pin { plain: 0xFFFF8CF2, debug: 0xFFFF8CF2 };

/// `Palette_Dirty`. tests: buffers_port, palette_port
pub const PALETTE_DIRTY: Pin = Pin { plain: 0xFFFF8296, debug: 0xFFFF8296 };

/// `Parallax_Active_Config`. tests: buffers_port
pub const PARALLAX_ACTIVE_CONFIG: Pin = Pin { plain: 0x61B4, debug: 0x7224 };

/// `VBlank_Ready`. tests: vblank_port
pub const V_BLANK_READY: Pin = Pin { plain: 0xFFFF8048, debug: 0xFFFF8048 };

/// `VBlank_Flag`. tests: vblank_port
pub const V_BLANK_FLAG: Pin = Pin { plain: 0xFFFF8000, debug: 0xFFFF8000 };

/// `VInt_Ptr`. tests: vblank_port
pub const V_INT_PTR: Pin = Pin { plain: 0xFFFF8044, debug: 0xFFFF8044 };

/// `Ctrl_1_Press`. tests: vblank_port
pub const CTRL_1_PRESS: Pin = Pin { plain: 0xFFFF8029, debug: 0xFFFF8029 };

/// `Ctrl_1_Press_Accum`. tests: vblank_port
pub const CTRL_1_PRESS_ACCUM: Pin = Pin { plain: 0xFFFF802C, debug: 0xFFFF802C };

/// `Ctrl_2_Press`. tests: vblank_port
pub const CTRL_2_PRESS: Pin = Pin { plain: 0xFFFF802B, debug: 0xFFFF802B };

/// `Ctrl_2_Press_Accum`. tests: vblank_port
pub const CTRL_2_PRESS_ACCUM: Pin = Pin { plain: 0xFFFF802D, debug: 0xFFFF802D };

/// `Ctrl_1_Ext_Press`. tests: vblank_port, game_loop_port
pub const CTRL_1_EXT_PRESS: Pin = Pin { plain: 0xFFFF802F, debug: 0xFFFF802F };

/// `Ctrl_1_Ext_Press_Accum`. tests: vblank_port, game_loop_port
pub const CTRL_1_EXT_PRESS_ACCUM: Pin = Pin { plain: 0xFFFF8032, debug: 0xFFFF8032 };

/// `Ctrl_2_Ext_Press`. tests: vblank_port, game_loop_port
pub const CTRL_2_EXT_PRESS: Pin = Pin { plain: 0xFFFF8031, debug: 0xFFFF8031 };

/// `Ctrl_2_Ext_Press_Accum`. tests: vblank_port, game_loop_port
pub const CTRL_2_EXT_PRESS_ACCUM: Pin = Pin { plain: 0xFFFF8033, debug: 0xFFFF8033 };

/// `Parallax_State`. tests: parallax_port
pub const PARALLAX_STATE: Pin = Pin { plain: 0xFFFF88A0, debug: 0xFFFF88A0 };

/// `Vscroll_Factor`. tests: parallax_port
pub const VSCROLL_FACTOR: Pin = Pin { plain: 0xFFFF889C, debug: 0xFFFF889C };

/// `DMA_Budget_Default`. tests: vblank_port
pub const DMA_BUDGET_DEFAULT: Pin = Pin { plain: 0xFFFF8210, debug: 0xFFFF8210 };

/// `Lag_Frame_Count` — debug-shape consumer only (`debug_only`). tests: vblank_port
pub const LAG_FRAME_COUNT: u32 = 0xFFFF8D0C;

/// `DMA_Bytes_ThisFrame` — debug-shape consumer only (`debug_only`). tests: vblank_port
pub const DMA_BYTES_THIS_FRAME: u32 = 0xFFFF8D00;

/// `PageIn_InFlight`. tests: game_loop_port
pub const PAGE_IN_IN_FLIGHT: Pin = Pin { plain: 0xFFFFB74E, debug: 0xFFFFB7DC };

/// `PageIn_Saved_PC`. tests: game_loop_port
pub const PAGE_IN_SAVED_PC: Pin = Pin { plain: 0xFFFFB748, debug: 0xFFFFB7D6 };

/// `PageIn_BankRegs`. tests: game_loop_port
pub const PAGE_IN_BANK_REGS: Pin = Pin { plain: 0x6FC4, debug: 0x819C };

/// `Dbg_PageIn_Preempts` — debug-shape consumer only (`debug_only`). tests: game_loop_port
pub const DBG_PAGE_IN_PREEMPTS: u32 = 0xFFFF8D26;

/// `ZX0R_Decompress.__end`. tests: game_loop_port
pub const ZX0R_DECOMPRESS_END: Pin = Pin { plain: 0x27E0, debug: 0x2A78 };

/// `PageIn_Staging_Busy`. tests: game_loop_port, load_art_port
pub const PAGE_IN_STAGING_BUSY: Pin = Pin { plain: 0xFFFFB750, debug: 0xFFFFB7DE };

/// `PageIn_Flush`. tests: load_art_port
pub const PAGE_IN_FLUSH: Pin = Pin { plain: 0x708C, debug: 0x826C };

/// `PageIn_Enqueue`. tests: load_art_port
pub const PAGE_IN_ENQUEUE: Pin = Pin { plain: 0x704E, debug: 0x822E };

/// `PageIn_Pool_Table`. tests: load_art_port
pub const PAGE_IN_POOL_TABLE: Pin = Pin { plain: 0xFFFFB77C, debug: 0xFFFFB80A };

/// `PageIn_Queue_Count`. tests: load_art_port
pub const PAGE_IN_QUEUE_COUNT: Pin = Pin { plain: 0xFFFFB752, debug: 0xFFFFB7E0 };

/// `PageIn_Suspended`. tests: load_art_port
pub const PAGE_IN_SUSPENDED: Pin = Pin { plain: 0xFFFFB74F, debug: 0xFFFFB7DD };

/// `PageIn_Land_Pending`. tests: load_art_port
pub const PAGE_IN_LAND_PENDING: Pin = Pin { plain: 0xFFFFB751, debug: 0xFFFFB7DF };

/// `PageCache_Init`. tests: load_art_port
pub const PAGE_CACHE_INIT: Pin = Pin { plain: 0x70D4, debug: 0x82B6 };

/// `PageCache_AllocFrame`. tests: load_art_port
pub const PAGE_CACHE_ALLOC_FRAME: Pin = Pin { plain: 0x7180, debug: 0x83C4 };

/// `PageCache_Publish`. tests: load_art_port
pub const PAGE_CACHE_PUBLISH: Pin = Pin { plain: 0x723C, debug: 0x8584 };

/// `PageCache_PatchRun_Seq`. tests: tile_cache_port
pub const PAGE_CACHE_PATCH_RUN_SEQ: Pin = Pin { plain: 0x72AA, debug: 0x8658 };

/// `PageCache_PatchRun_Col`. tests: tile_cache_port
pub const PAGE_CACHE_PATCH_RUN_COL: Pin = Pin { plain: 0x7372, debug: 0x87EA };

/// `PageCache_Audit`. tests: tile_cache_port
pub const PAGE_CACHE_AUDIT: Pin = Pin { plain: 0x753A, debug: 0x8A80 };

/// `Cache_Art_Stall`. tests: tile_cache_port
pub const CACHE_ART_STALL: Pin = Pin { plain: 0xFFFFAB62, debug: 0xFFFFABF0 };

/// `Page_Audit_Ticks` — debug-shape consumer only (`debug_only`). tests: tile_cache_port
pub const PAGE_AUDIT_TICKS: u32 = 0xFFFF8D3A;

/// `Cache_Stall_Watchdog` — debug-shape consumer only (`debug_only`). tests: tile_cache_port
pub const CACHE_STALL_WATCHDOG: u32 = 0xFFFF8D38;

/// `Flush_VDP_Shadow`. tests: vblank_port
pub const FLUSH_VDP_SHADOW: Pin = Pin { plain: 0x1C12, debug: 0x1C90 };

/// `VInt_DrawLevel`. tests: vblank_port
pub const V_INT_DRAW_LEVEL: Pin = Pin { plain: 0x4806, debug: 0x5666 };

/// `Vscroll_Write`. tests: vblank_port
pub const VSCROLL_WRITE: Pin = Pin { plain: 0x61C6, debug: 0x7236 };

/// `Read_Controllers`. tests: vblank_port
pub const READ_CONTROLLERS: Pin = Pin { plain: 0x23F0, debug: 0x24D0 };

/// `Process_DMA_Critical`. tests: vblank_port
pub const PROCESS_DMA_CRITICAL: Pin = Pin { plain: 0x1E32, debug: 0x1F14 };

/// `Process_DMA_Important`. tests: vblank_port
pub const PROCESS_DMA_IMPORTANT: Pin = Pin { plain: 0x1F00, debug: 0x1FE2 };

/// `Process_DMA_Deferrable`. tests: vblank_port
pub const PROCESS_DMA_DEFERRABLE: Pin = Pin { plain: 0x1F14, debug: 0x1FF6 };

/// `Enqueue_Dirty_Buffers`. tests: vblank_port
pub const ENQUEUE_DIRTY_BUFFERS: Pin = Pin { plain: 0x206A, debug: 0x2142 };

/// `BootData`. tests: boot_port
pub const BOOT_DATA: Pin = Pin { plain: 0x3A0, debug: 0x3A0 };

/// `VInt_Level`. tests: boot_port
pub const V_INT_LEVEL: Pin = Pin { plain: 0x2218, debug: 0x22FC };

/// `BuildStaticDMA`. tests: boot_port
pub const BUILD_STATIC_DMA: Pin = Pin { plain: 0x1F92, debug: 0x206A };

/// `Sound_Init`. tests: boot_port
pub const SOUND_INIT: Pin = Pin { plain: 0x76F4, debug: 0x9ED6 };

/// `Hardware_Region`. tests: boot_port
pub const HARDWARE_REGION: Pin = Pin { plain: 0xFFFF8026, debug: 0xFFFF8026 };

/// `Region_Flags`. tests: boot_port
pub const REGION_FLAGS: Pin = Pin { plain: 0xFFFF8027, debug: 0xFFFF8027 };

/// `Game_State_ID`. tests: boot_port
pub const GAME_STATE_ID: Pin = Pin { plain: 0xFFFF800C, debug: 0xFFFF800C };

/// `Game_State_Init`. tests: boot_port
pub const GAME_STATE_INIT: Pin = Pin { plain: 0xFFFF800D, debug: 0xFFFF800D };

/// `RAM_Start`. tests: boot_port
pub const RAM_START: Pin = Pin { plain: 0xFFFF8000, debug: 0xFFFF8000 };

/// `PState_Ground`. tests: test_p1_player_port
pub const P_STATE_GROUND: Pin = Pin { plain: 0x10610, debug: 0x10720 };

/// `PState_Roll`. tests: test_p1_player_port
pub const P_STATE_ROLL: Pin = Pin { plain: 0x10772, debug: 0x10882 };

/// `PState_Spindash`. tests: test_p1_player_port
pub const P_STATE_SPINDASH: Pin = Pin { plain: 0x10DF0, debug: 0x10F00 };

/// `PState_Air`. tests: test_p1_player_port
pub const P_STATE_AIR: Pin = Pin { plain: 0x10AA0, debug: 0x10BB0 };

/// `PState_Jump`. tests: test_p1_player_port
pub const P_STATE_JUMP: Pin = Pin { plain: 0x10AA8, debug: 0x10BB8 };

/// `PState_RollJump`. tests: test_p1_player_port
pub const P_STATE_ROLL_JUMP: Pin = Pin { plain: 0x10AA4, debug: 0x10BB4 };

/// `PState_AirBall`. tests: test_p1_player_port
pub const P_STATE_AIR_BALL: Pin = Pin { plain: 0x10AA0, debug: 0x10BB0 };

/// `PState_Fly`. tests: test_p1_player_port
pub const P_STATE_FLY: Pin = Pin { plain: 0x10E8C, debug: 0x10F9C };

/// `PState_Glide`. tests: test_p1_player_port
pub const P_STATE_GLIDE: Pin = Pin { plain: 0x10FBE, debug: 0x110D0 };

/// `PState_GlideFall`. tests: test_p1_player_port
pub const P_STATE_GLIDE_FALL: Pin = Pin { plain: 0x1114E, debug: 0x11260 };

/// `PState_Slide`. tests: test_p1_player_port
pub const P_STATE_SLIDE: Pin = Pin { plain: 0x11192, debug: 0x112A4 };

/// `PState_Climb`. tests: test_p1_player_port
pub const P_STATE_CLIMB: Pin = Pin { plain: 0x112C8, debug: 0x113DA };

/// `PState_Ledge`. tests: test_p1_player_port
pub const P_STATE_LEDGE: Pin = Pin { plain: 0x11474, debug: 0x11586 };

/// `Player_SensorFloor`. tests: test_p1_player_port
pub const PLAYER_SENSOR_FLOOR: Pin = Pin { plain: 0x593C, debug: 0x693C };

/// `Player_AtLedgeEdge`. tests: test_p1_player_port
pub const PLAYER_AT_LEDGE_EDGE: Pin = Pin { plain: 0x5A56, debug: 0x6A56 };

/// `Player_SetState`. tests: test_p2_player_states_port
pub const PLAYER_SET_STATE: Pin = Pin { plain: 0x10370, debug: 0x10434 };

/// `Player_SnapToSurface`. tests: test_p2_player_states_port
pub const PLAYER_SNAP_TO_SURFACE: Pin = Pin { plain: 0x104B6, debug: 0x1057A };

/// `Player_SensorCeiling`. tests: test_p2_player_states_port
pub const PLAYER_SENSOR_CEILING: Pin = Pin { plain: 0x5952, debug: 0x6952 };

/// `Player_SensorWallDir`. tests: test_p2_player_states_port
pub const PLAYER_SENSOR_WALL_DIR: Pin = Pin { plain: 0x5A0C, debug: 0x6A0C };

/// `Player_SensorWallAt`. tests: test_p2_player_states_port
pub const PLAYER_SENSOR_WALL_AT: Pin = Pin { plain: 0x5A04, debug: 0x6A04 };

/// `Collision_GetType`. tests: test_p4_player_sensors_port
pub const COLLISION_GET_TYPE: Pin = Pin { plain: 0x5560, debug: 0x6560 };

/// `SolidityTable`. tests: test_p4_player_sensors_port
pub const SOLIDITY_TABLE: Pin = Pin { plain: 0x2A430, debug: 0x2AC60 };

/// `AngleTable`. tests: test_p4_player_sensors_port
pub const ANGLE_TABLE: Pin = Pin { plain: 0x2A330, debug: 0x2AB60 };

/// `HeightMaps`. tests: test_p4_player_sensors_port
pub const HEIGHT_MAPS: Pin = Pin { plain: 0x28330, debug: 0x28B60 };

/// `HeightMapsRot`. tests: test_p4_player_sensors_port
pub const HEIGHT_MAPS_ROT: Pin = Pin { plain: 0x29330, debug: 0x29B60 };

/// `Character_ID`. tests: test_p1_player_port
pub const CHARACTER_ID: Pin = Pin { plain: 0xFFFFB796, debug: 0xFFFFE02C };

/// `Player_Chardef`. tests: test_p1_player_port
pub const PLAYER_CHARDEF: Pin = Pin { plain: 0xFFFFB798, debug: 0xFFFFE02E };

/// `Ability_None`. tests: test_p1_player_port
pub const ABILITY_NONE: Pin = Pin { plain: 0x11668, debug: 0x117DE };

/// `CharacterDefs`. tests: test_p1_player_port
pub const CHARACTER_DEFS: Pin = Pin { plain: 0x11620, debug: 0x11730 };

/// `Player_InitAssets`. tests: test_p1_player_port
pub const PLAYER_INIT_ASSETS: Pin = Pin { plain: 0x1162C, debug: 0x1173C };

/// `Player_LoadArt`. tests: test_p1_player_port
pub const PLAYER_LOAD_ART: Pin = Pin { plain: 0x11644, debug: 0x11754 };

/// `Player_Ability`. tests: test_p2_player_states_port
pub const PLAYER_ABILITY: Pin = Pin { plain: 0x1165E, debug: 0x1176E };

/// `PhysTable_Sonic`. tests: test_p1_player_port
pub const PHYS_TABLE_SONIC: Pin = Pin { plain: 0x11596, debug: 0x116A6 };

/// `Pal_SonicTails`. tests: test_p1_player_port
pub const PAL_SONIC_TAILS: Pin = Pin { plain: 0x9F906, debug: 0xA1356 };

/// `OJZ_TestRaster`. tests: act_descriptor_port
pub const OJZ_TEST_RASTER: Pin = Pin { plain: 0x1240E, debug: 0x12C52 };

/// `OJZ_TestPal`. tests: act_descriptor_port
pub const OJZ_TEST_PAL: Pin = Pin { plain: 0x1242E, debug: 0x12C72 };

/// `OJZ_TestGradient`. tests: act_descriptor_port
pub const OJZ_TEST_GRADIENT: Pin = Pin { plain: 0x126F6, debug: 0x12F3A };

/// `OJZ_ShimmerCycle`. tests: act_descriptor_port
pub const OJZ_SHIMMER_CYCLE: Pin = Pin { plain: 0x1248E, debug: 0x12CD2 };

/// `OJZ_TestVsram`. tests: act_descriptor_port
pub const OJZ_TEST_VSRAM: Pin = Pin { plain: 0x12714, debug: 0x12F58 };

/// `OJZ_TestRamp`. tests: act_descriptor_port
pub const OJZ_TEST_RAMP: Pin = Pin { plain: 0x12730, debug: 0x12F74 };

/// `Raster_Program`. tests: raster_port
pub const RASTER_PROGRAM: Pin = Pin { plain: 0xFFFF8994, debug: 0xFFFF8994 };

/// `Raster_Cursor`. tests: raster_port
pub const RASTER_CURSOR: Pin = Pin { plain: 0xFFFF8998, debug: 0xFFFF8998 };

/// `Raster_Pending`. tests: raster_port
pub const RASTER_PENDING: Pin = Pin { plain: 0xFFFF899C, debug: 0xFFFF899C };

/// `Raster_Buf_A`. tests: raster_port
pub const RASTER_BUF_A: Pin = Pin { plain: 0xFFFF89A2, debug: 0xFFFF89A2 };

/// `Raster_Active_Buf`. tests: raster_port
pub const RASTER_ACTIVE_BUF: Pin = Pin { plain: 0xFFFF8AA2, debug: 0xFFFF8AA2 };

/// `Raster_VBlank`. tests: game_loop_port, vblank_port, load_art_port, boot_port
pub const RASTER_V_BLANK: Pin = Pin { plain: 0x66BC, debug: 0x772C };

/// `Palette_LoadSection`. tests: parallax_port
pub const PALETTE_LOAD_SECTION: Pin = Pin { plain: 0x6892, debug: 0x7902 };

/// `Raster_InstallSection`. tests: parallax_port
pub const RASTER_INSTALL_SECTION: Pin = Pin { plain: 0x6818, debug: 0x7888 };

/// `Palette_InstallCycleSection`. tests: parallax_port
pub const PALETTE_INSTALL_CYCLE_SECTION: Pin = Pin { plain: 0x694E, debug: 0x79BE };

/// `Palette_Compose`. tests: game_loop_port
pub const PALETTE_COMPOSE: Pin = Pin { plain: 0x6984, debug: 0x79F4 };

/// `Player_Blocks`. tests: test_p1_player_port
pub const PLAYER_BLOCKS: Pin = Pin { plain: 0xFFFFB79C, debug: 0xFFFFE032 };

/// `Player_Ring_Index`. tests: test_p1_player_port
pub const PLAYER_RING_INDEX: Pin = Pin { plain: 0xFFFFBB00, debug: 0xFFFFE400 };

/// `Player_Pos_Ring`. tests: test_p1_player_port
pub const PLAYER_POS_RING: Pin = Pin { plain: 0xFFFFB900, debug: 0xFFFFE200 };

/// `Player_Stat_Ring`. tests: test_p1_player_port
pub const PLAYER_STAT_RING: Pin = Pin { plain: 0xFFFFBA00, debug: 0xFFFFE300 };

/// `Player_Death_Pending`. tests: test_p1_player_port
pub const PLAYER_DEATH_PENDING: Pin = Pin { plain: 0xFFFFB7C4, debug: 0xFFFFE05A };

/// `Player_Bound_Right`. tests: test_p1_player_port
pub const PLAYER_BOUND_RIGHT: Pin = Pin { plain: 0xFFFFB7C6, debug: 0xFFFFE05C };

/// `Player_Bound_Bottom`. tests: test_p1_player_port
pub const PLAYER_BOUND_BOTTOM: Pin = Pin { plain: 0xFFFFB7C8, debug: 0xFFFFE05E };

/// `DustSpindash_Spawn`. tests: test_p1_player_port
pub const DUST_SPINDASH_SPAWN: Pin = Pin { plain: 0x11824, debug: 0x119F2 };

// ── Region-relative offsets (manifest order) ──

/// `AnimateSprite.cc_delete` − `animate` start (per-shape). tests: animate_port
pub const CC_DELETE_OFF: ShapeOffset = ShapeOffset { plain: 0x104, debug: 0x15E };

/// `RefreshSpritePieceCount` − `animate` start (per-shape). tests: animate_port
pub const REFRESH_OFF: ShapeOffset = ShapeOffset { plain: 0x16C, debug: 0x290 };

/// `RingCollision` − `rings` start (per-shape). tests: rings_port
pub const RINGCOL_OFF: ShapeOffset = ShapeOffset { plain: 0x116, debug: 0x172 };

/// `Sound_PlaySFX` − `sound_api` start (per-shape). tests: sound_api_port
pub const SOUND_PLAY_SFX_OFF: ShapeOffset = ShapeOffset { plain: 0x126, debug: 0x28A };

/// `Sine_Table` − `math` start (shape-invariant, asserted at generation). tests: math_port
pub const SINE_TABLE_OFF: usize = 0x18;

/// `Flush_VDP_Shadow` − `vdp_init` start (shape-invariant, asserted at generation). tests: vdp_init_port
pub const FLUSH_VDP_SHADOW_OFF: usize = 0x12;

/// `HBlank_Uninstall` − `hblank` start (shape-invariant, asserted at generation). tests: raster_port
pub const HBLANK_UNINSTALL_OFF: usize = 0x1C;
