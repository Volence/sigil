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
//! [provenance] 83 regions, 348 symbols, 7 offsets

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
pub const ASSEMBLED_LEN: usize = 0x7EA20;
/// Assembled (pre-convsym) ROM length, `__DEBUG__` shape. tests: m1d_rom, m1d_debug_rom, mixed_dac_rom
pub const DEBUG_ASSEMBLED_LEN: usize = 0x808D4;

// ── Regions (manifest order) ──

/// `Vectors` .. start + 0x100 plain / 0x100 debug (literal — no end symbol) — gate `SIGIL_EMP_VECTORS`. tests: vectors_port
pub const VECTORS: Region = Region { plain_base: 0x0, debug_base: 0x0, plain_len: 0x100, debug_len: 0x100 };

/// `GameHeader` .. `EntryPoint`. tests: header_port
pub const HEADER: Region = Region { plain_base: 0x100, debug_base: 0x100, plain_len: 0x100, debug_len: 0x100 };

/// `HeightMaps` .. start + 0x1C480 plain / 0x1C480 debug (literal — no end symbol). tests: collision_data_port
pub const COLLISION_DATA: Region = Region { plain_base: 0x265C8, debug_base: 0x26D60, plain_len: 0x1C480, debug_len: 0x1C480 };

/// `EntryPoint` .. `BootData` — gate `SIGIL_EMP_BOOT`. tests: boot_port
pub const BOOT: Region = Region { plain_base: 0x200, debug_base: 0x200, plain_len: 0x1A0, debug_len: 0x1B0 };

/// `BootData` .. `BootData_PostBlob`. tests: boot_data_port
pub const BOOT_HEAD: Region = Region { plain_base: 0x3A0, debug_base: 0x3B0, plain_len: 0x1850, debug_len: 0x18D0 };

/// `BootData_PostBlob` .. `BootData_End`. tests: boot_data_port
pub const BOOT_TAIL: Region = Region { plain_base: 0x1BF0, debug_base: 0x1C80, plain_len: 0xE, debug_len: 0xE };

/// `VDP_Shadow_Init` .. `Init_DMA_Queue` — gate `SIGIL_EMP_VDP_INIT`. tests: vdp_init_port
pub const VDP_INIT: Region = Region { plain_base: 0x1C00, debug_base: 0x1C8E, plain_len: 0x4A, debug_len: 0x52 };

/// `Init_DMA_Queue` .. `Init_SpriteTable` — gate `SIGIL_EMP_DMA_QUEUE`. tests: dma_queue_port
pub const DMA_QUEUE: Region = Region { plain_base: 0x1C4A, debug_base: 0x1CE0, plain_len: 0x336, debug_len: 0x338 };

/// `Init_SpriteTable` .. `VBlank_Handler` — gate `SIGIL_EMP_BUFFERS`. tests: buffers_port
pub const BUFFERS: Region = Region { plain_base: 0x1F80, debug_base: 0x2018, plain_len: 0x260, debug_len: 0x268 };

/// `VBlank_Handler` .. `HBlank_Install` — gate `SIGIL_EMP_VBLANK`. tests: vblank_port
pub const VBLANK: Region = Region { plain_base: 0x21E0, debug_base: 0x2280, plain_len: 0x1D0, debug_len: 0x1E0 };

/// `HBlank_Install` .. `Read_Controllers` — gate `SIGIL_EMP_HBLANK`. tests: hblank_port, m1c_vector_table
pub const HBLANK: Region = Region { plain_base: 0x23B0, debug_base: 0x2460, plain_len: 0x50, debug_len: 0x48 };

/// `Read_Controllers` .. `GameLoop` — gate `SIGIL_EMP_CONTROLLERS`. tests: controllers_port
pub const CONTROLLERS: Region = Region { plain_base: 0x2400, debug_base: 0x24A8, plain_len: 0x10E, debug_len: 0x118 };

/// `GameLoop` .. `Input_Tick` — gate `SIGIL_EMP_GAME_LOOP`. tests: game_loop_port, load_art_port
pub const GAME_LOOP: Region = Region { plain_base: 0x250E, debug_base: 0x25C0, plain_len: 0x1C, debug_len: 0x1C };

/// `Input_Tick` .. `S4LZ_DecompressDict`. tests: game_loop_port
pub const REPLAY: Region = Region { plain_base: 0x252A, debug_base: 0x25DC, plain_len: 0x146, debug_len: 0x1EC };

/// `S4LZ_DecompressDict` .. `ZX0R_Decompress` — gate `SIGIL_EMP_S4LZ`. tests: s4lz_port
pub const S4LZ: Region = Region { plain_base: 0x2670, debug_base: 0x27C8, plain_len: 0xF8, debug_len: 0x200 };

/// `ZX0R_Decompress` .. `GetSineCosine`.
pub const ZX0_RESUME: Region = Region { plain_base: 0x2768, debug_base: 0x29C8, plain_len: 0x78, debug_len: 0x78 };

/// `GetSineCosine` .. `Perform_DPLC` — gate `SIGIL_EMP_MATH`. tests: math_port
pub const MATH: Region = Region { plain_base: 0x27E0, debug_base: 0x2A40, plain_len: 0x298, debug_len: 0x298 };

/// `Perform_DPLC` .. `InitObjectRAM` — gate `SIGIL_EMP_DPLC`. tests: dplc_port
pub const DPLC: Region = Region { plain_base: 0x2A78, debug_base: 0x2CD8, plain_len: 0xA8, debug_len: 0xA8 };

/// `InitObjectRAM` .. `InitSpriteSystem` — gate `SIGIL_EMP_CORE`. tests: core_port
pub const CORE: Region = Region { plain_base: 0x2B20, debug_base: 0x2D80, plain_len: 0x300, debug_len: 0x750 };

/// `InitSpriteSystem` .. `AnimateSprite` — gate `SIGIL_EMP_SPRITES`. tests: sprites_port
pub const SPRITES: Region = Region { plain_base: 0x2E20, debug_base: 0x34D0, plain_len: 0x420, debug_len: 0x4EE };

/// `AnimateSprite` .. `TouchResponse` — gate `SIGIL_EMP_ANIMATE`. tests: animate_port, test_objects_port
pub const ANIMATE: Region = Region { plain_base: 0x3240, debug_base: 0x39BE, plain_len: 0x194, debug_len: 0x2B8 };

/// `TouchResponse` .. `RingBuffer_Add` — gate `SIGIL_EMP_COLLISION`. tests: collision_port
pub const COLLISION: Region = Region { plain_base: 0x33D4, debug_base: 0x3C76, plain_len: 0x200, debug_len: 0x208 };

/// `RingBuffer_Add` .. `Collected_Init` — gate `SIGIL_EMP_RINGS`. tests: rings_port
pub const RINGS: Region = Region { plain_base: 0x35D4, debug_base: 0x3E7E, plain_len: 0x1B8, debug_len: 0x214 };

/// `Collected_Init` .. `PopulateSpawnedPieceCount` — gate `SIGIL_EMP_ENTITY_WINDOW`. tests: entity_window_port
pub const ENTITY_WINDOW: Region = Region { plain_base: 0x378C, debug_base: 0x4092, plain_len: 0x8F4, debug_len: 0xD5E };

/// `PopulateSpawnedPieceCount` .. `Load_Object` — gate `SIGIL_EMP_CHILDREN`. tests: children_port
pub const CHILDREN: Region = Region { plain_base: 0x4080, debug_base: 0x4DF0, plain_len: 0x2F0, debug_len: 0x3A0 };

/// `Load_Object` .. `Plane_Buffer_Reset` — gate `SIGIL_EMP_LOAD_OBJECT`. tests: load_object_port, entity_window_port
pub const LOAD_OBJECT: Region = Region { plain_base: 0x4370, debug_base: 0x5190, plain_len: 0x88, debug_len: 0x88 };

/// `Plane_Buffer_Reset` .. `Tile_Cache_GetTile` — gate `SIGIL_EMP_PLANE_BUFFER`. tests: plane_buffer_port
pub const PLANE_BUFFER: Region = Region { plain_base: 0x43F8, debug_base: 0x5218, plain_len: 0x328, debug_len: 0x328 };

/// `Tile_Cache_GetTile` .. `Collision_GetType` — gate `SIGIL_EMP_TILE_CACHE`. tests: tile_cache_port
pub const TILE_CACHE: Region = Region { plain_base: 0x4720, debug_base: 0x5540, plain_len: 0xCE0, debug_len: 0xE40 };

/// `Collision_GetType` .. `Collision_ProbeDown` — gate `SIGIL_EMP_COLLISION_LOOKUP`. tests: collision_lookup_port
pub const COLLISION_LOOKUP: Region = Region { plain_base: 0x5400, debug_base: 0x6380, plain_len: 0x70, debug_len: 0x70 };

/// `Section_Init` .. `Camera_Init` — gate `SIGIL_EMP_SECTION`. tests: section_port
pub const SECTION: Region = Region { plain_base: 0x5964, debug_base: 0x68E4, plain_len: 0x42C, debug_len: 0x42C };

/// `Camera_Init` .. `Parallax_Init` — gate `SIGIL_EMP_CAMERA`. tests: camera_port
pub const CAMERA: Region = Region { plain_base: 0x5D90, debug_base: 0x6D10, plain_len: 0x1D0, debug_len: 0x1E0 };

/// `Parallax_Init` .. `Level_LoadArt` — gate `SIGIL_EMP_PARALLAX`. tests: parallax_port
pub const PARALLAX: Region = Region { plain_base: 0x5F60, debug_base: 0x6EF0, plain_len: 0x5EC, debug_len: 0x5EC };

/// `Level_LoadArt` .. `PageIn_Process` — gate `SIGIL_EMP_LOAD_ART`. tests: load_art_port
pub const LOAD_ART: Region = Region { plain_base: 0x654C, debug_base: 0x74DC, plain_len: 0x92, debug_len: 0x92 };

/// `PageIn_Process` .. `PageCache_Init`.
pub const PAGE_IN: Region = Region { plain_base: 0x65DE, debug_base: 0x756E, plain_len: 0x2EE, debug_len: 0x460 };

/// `PageCache_Init` .. `BG_Init`.
pub const PAGE_CACHE: Region = Region { plain_base: 0x68CC, debug_base: 0x79CE, plain_len: 0x474, debug_len: 0xB72 };

/// `BG_Init` .. `BgAnim_Init` — gate `SIGIL_EMP_BG`. tests: bg_port
pub const BG: Region = Region { plain_base: 0x6D40, debug_base: 0x8540, plain_len: 0xE0, debug_len: 0xE0 };

/// `BgAnim_Init` .. start + 0x9E plain / 0x158 debug (literal — no end symbol) — gate `SIGIL_EMP_BG_ANIM`. tests: bg_anim_port
pub const BG_ANIM: Region = Region { plain_base: 0x6E20, debug_base: 0x8620, plain_len: 0x9E, debug_len: 0x158 };

/// `CompressionSelfTest` .. `Sound_PostByte` (debug-only region; plain empty at `Sound_PostByte`) — gate `SIGIL_EMP_COMPRESSION_SELFTEST`. tests: compression_selftest_port
pub const COMPRESSION_SELFTEST: Region = Region { plain_base: 0x6EBE, debug_base: 0x8778, plain_len: 0x0, debug_len: 0xDE8 };

/// `Sound_PostByte` .. start + 0x2A8 plain / 0x452 debug (literal — no end symbol) — gate `SIGIL_EMP_SOUND_API`. tests: sound_api_port
pub const SOUND_API: Region = Region { plain_base: 0x6EBE, debug_base: 0x9560, plain_len: 0x2A8, debug_len: 0x452 };

/// `TestSolid_Init` .. `ObjDef_PathSwap` plain / `TestParticle` debug — gate `SIGIL_EMP_TEST_OBJECTS`. tests: test_objects_port
pub const TEST_SOLID: Region = Region { plain_base: 0x10F40, debug_base: 0x113AC, plain_len: 0x12, debug_len: 0x14 };

/// `TestParticle` .. `TestEmitter` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_OBJECTS`. tests: test_objects_port
pub const TEST_PARTICLE: Region = Region { plain_base: 0x10F52, debug_base: 0x113C0, plain_len: 0x0, debug_len: 0x58 };

/// `TestStatic_Main` .. `TestSolid_Init` plain / `TestAnimated` debug — gate `SIGIL_EMP_TEST_STATIC`. tests: test_g1_objects_port
pub const TEST_STATIC: Region = Region { plain_base: 0x10F30, debug_base: 0x11060, plain_len: 0x10, debug_len: 0x10 };

/// `TestAnimated` .. `TestPlayer` (debug-only region; plain empty at `TestSolid_Init`) — gate `SIGIL_EMP_TEST_ANIMATED`. tests: test_g1_objects_port
pub const TEST_ANIMATED: Region = Region { plain_base: 0x10F40, debug_base: 0x11070, plain_len: 0x0, debug_len: 0x60 };

/// `TestEmitter` .. `TestChildPart` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_EMITTER`. tests: test_g2_objects_port
pub const TEST_EMITTER: Region = Region { plain_base: 0x10F52, debug_base: 0x11418, plain_len: 0x0, debug_len: 0x5E };

/// `TestStressEmitter` .. `TestChurnObj` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_STRESS_EMITTER`. tests: test_g2_objects_port
pub const TEST_STRESS_EMITTER: Region = Region { plain_base: 0x10F52, debug_base: 0x115B0, plain_len: 0x0, debug_len: 0x60 };

/// `TestChurnObj` .. `ObjDef_PathSwap` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_CHURN`. tests: test_g2_objects_port
pub const TEST_CHURN: Region = Region { plain_base: 0x10F52, debug_base: 0x11610, plain_len: 0x0, debug_len: 0x7C };

/// `TestChildPart` .. `TestStressEmitter` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_PARENT`. tests: test_g3_objects_port
pub const TEST_PARENT: Region = Region { plain_base: 0x10F52, debug_base: 0x11476, plain_len: 0x0, debug_len: 0x13A };

/// `TestPlayer` .. `TestEnemy_Init` (debug-only region; plain empty at `TestSolid_Init`) — gate `SIGIL_EMP_TEST_PLAYER`. tests: test_g4_final_objects_port
pub const TEST_PLAYER: Region = Region { plain_base: 0x10F40, debug_base: 0x110D0, plain_len: 0x0, debug_len: 0x294 };

/// `TestEnemy_Init` .. `TestSolid_Init` (debug-only region; plain empty at `TestSolid_Init`) — gate `SIGIL_EMP_TEST_ENEMY`. tests: test_g4_final_objects_port
pub const TEST_ENEMY: Region = Region { plain_base: 0x10F40, debug_base: 0x11364, plain_len: 0x0, debug_len: 0x48 };

/// `ObjDef_PathSwap` .. `DeformTable_Zero` — gate `SIGIL_EMP_PATH_SWAP`. tests: test_g4_final_objects_port
pub const PATH_SWAP: Region = Region { plain_base: 0x10F52, debug_base: 0x1168C, plain_len: 0x92, debug_len: 0xFC };

/// `DeformTable_Zero` .. `ObjDef_Static` — gate `SIGIL_EMP_PARALLAX_CONFIGS`. tests: parallax_configs_port
pub const PARALLAX_CONFIGS: Region = Region { plain_base: 0x10FE4, debug_base: 0x11788, plain_len: 0xA8C, debug_len: 0xA8A };

/// `Map_TestObj` .. `Ani_Sonic` — gate `SIGIL_EMP_TEST_MAPPINGS`. tests: test_mappings_port
pub const TEST_MAPPINGS: Region = Region { plain_base: 0x26410, debug_base: 0x26B9A, plain_len: 0x30, debug_len: 0x30 };

/// `Ani_Sonic` .. `Ani_Sonic_End` — gate `SIGIL_EMP_SONIC_ANIMS`. tests: sonic_anims_port
pub const SONIC_ANIMS: Region = Region { plain_base: 0x26440, debug_base: 0x26BCA, plain_len: 0x86, debug_len: 0x86 };

/// `Ani_Tails` .. `Ani_Tails_End` — gate `SIGIL_EMP_TAILS_ANIMS`. tests: sonic_anims_port
pub const TAILS_ANIMS: Region = Region { plain_base: 0x264C6, debug_base: 0x26C50, plain_len: 0x102, debug_len: 0x102 };

/// `Map_Tails` .. `Map_Tails_End` — gate `SIGIL_EMP_TAILS_DATA`. tests: collision_data_port
pub const TAILS_DATA: Region = Region { plain_base: 0x5C258, debug_base: 0x5DCA8, plain_len: 0x20F5E, debug_len: 0x20F5E };

/// `Ani_Particle` .. `Ani_Particle_End` (debug-only region; plain empty at `HeightMaps`) — gate `SIGIL_EMP_PARTICLE_ANIMS`. tests: particle_anims_port, test_objects_port
pub const PARTICLE_ANIMS: Region = Region { plain_base: 0x265C8, debug_base: 0x26D58, plain_len: 0x0, debug_len: 0x8 };

/// `OJZ_Sec0_TypeTable` .. `OJZ_Act_Pool_Page0`. tests: ojz_run_a_port
pub const ENTITY_DATA: Region = Region { plain_base: 0x11AA8, debug_base: 0x12246, plain_len: 0x170, debug_len: 0x170 };

/// `OJZ_Act_Pool_Page0` .. `OJZ_Act1_Descriptor`. tests: ojz_run_a_port
pub const OJZ_ACT_POOL: Region = Region { plain_base: 0x11C18, debug_base: 0x123B6, plain_len: 0x2F76, debug_len: 0x2F76 };

/// `OJZ_Act1_Descriptor` .. `OJZ_Sec0_Blocks` — gate `SIGIL_EMP_ACT_DESCRIPTOR`. tests: act_descriptor_port
pub const ACT_DESCRIPTOR: Region = Region { plain_base: 0x14B8E, debug_base: 0x1532C, plain_len: 0x27A, debug_len: 0x284 };

/// `OJZ_Sec0_Blocks` .. `OJZ_Sec0_LocalMap`. tests: ojz_run_b_port
pub const SEC_BLOCK_BLOBS: Region = Region { plain_base: 0x14E08, debug_base: 0x155B0, plain_len: 0xB098, debug_len: 0xB08A };

/// `OJZ_Sec0_LocalMap` .. `OJZ_Palette`. tests: ojz_run_b_port
pub const SEC_LOCAL_MAPS: Region = Region { plain_base: 0x1FEA0, debug_base: 0x2063A, plain_len: 0xCE0, debug_len: 0xCDC };

/// `OJZ_Palette` .. `BgAnim_Table`. tests: ojz_run_b_port
pub const OJZ_ACT_ASSETS: Region = Region { plain_base: 0x20B80, debug_base: 0x21316, plain_len: 0x5882, debug_len: 0x5882 };

/// `BgAnim_Table` .. `Map_TestObj`. tests: ojz_run_b_port
pub const OJZ_BG_ANIM: Region = Region { plain_base: 0x26402, debug_base: 0x26B98, plain_len: 0xE, debug_len: 0x2 };

/// `ObjDef_Static` .. `OJZ_Sec0_TypeTable` — gate `SIGIL_EMP_OBJDEFS`. tests: objdef_port
pub const OBJDEFS: Region = Region { plain_base: 0x11A70, debug_base: 0x12212, plain_len: 0x38, debug_len: 0x34 };

/// `GameState_ObjectTest_Init` .. `GameState_OJZScroll_Init` (debug-only region; plain empty at `GameState_OJZScroll_Init`) — gate `SIGIL_EMP_OBJECT_TEST_STATE`. tests: test_t1_harness_states_port
pub const OBJECT_TEST_STATE: Region = Region { plain_base: 0x7D1C0, debug_base: 0x7EC10, plain_len: 0x0, debug_len: 0x394 };

/// `GameState_OJZScroll_Init` .. `Replay_OJZ_Fixture` — gate `SIGIL_EMP_OJZ_SCROLL_TEST`. tests: test_t1_harness_states_port
pub const OJZ_SCROLL_TEST: Region = Region { plain_base: 0x7D1C0, debug_base: 0x7EFA4, plain_len: 0x54C, debug_len: 0x61E };

/// `Replay_OJZ_Fixture` .. `BusError`.
pub const REPLAY_FIXTURE: Region = Region { plain_base: 0x7D70C, debug_base: 0x7F5C2, plain_len: 0x264, debug_len: 0x262 };

/// `BusError` .. `EndOfRom` — gate `SIGIL_EMP_ERROR_HANDLER`. tests: error_handler_port
pub const ERROR_HANDLER: Region = Region { plain_base: 0x7D970, debug_base: 0x7F824, plain_len: 0x10B0, debug_len: 0x10B0 };

/// `Dac_Temp_Blip` .. start + 0xF8BC plain / 0xF8BC debug (literal — no end symbol) — gate `SIGIL_EMP_DAC`. tests: dac_bank_port
pub const DAC_BANKS: Region = Region { plain_base: 0x48000, debug_base: 0x48000, plain_len: 0xF8BC, debug_len: 0xF8BC };

/// `Song_MovingTrucks` .. start + 0x34E8 plain / 0x4F38 debug (literal — no end symbol) — gate `SIGIL_EMP_MT`. tests: mt_bank_port
pub const MT_BANK_BLOB: Region = Region { plain_base: 0x58628, debug_base: 0x58628, plain_len: 0x34E8, debug_len: 0x4F38 };

/// `Sfx_33` .. start + 0x748 plain / 0x748 debug (literal — no end symbol) — gate `SIGIL_EMP_SFX`. tests: sfx_bank_port
pub const SFX_BANK_BLOB: Region = Region { plain_base: 0x5BB10, debug_base: 0x5D560, plain_len: 0x748, debug_len: 0x748 };

/// `SoundTablesZ80_Head` .. start + 0x628 plain / 0x628 debug (literal — no end symbol) — gate `SIGIL_EMP_SOUNDBANKHEAD`. tests: soundbankhead_port
pub const SOUNDBANKHEAD: Region = Region { plain_base: 0x58000, debug_base: 0x58000, plain_len: 0x628, debug_len: 0x628 };

/// `EndOfRom` .. start + 0x0 plain / 0x0 debug (literal — no end symbol) — gate `SIGIL_EMP_EPILOGUE`. tests: m1d_rom, m1d_debug_rom
pub const EPILOGUE: Region = Region { plain_base: 0x7EA20, debug_base: 0x808D4, plain_len: 0x0, debug_len: 0x0 };

/// `ObjCodeBase` .. start + 0x2 plain / 0x2 debug (literal — no end symbol) — gate `SIGIL_EMP_OBJCODEBASE`. tests: m1d_rom, m1d_debug_rom
pub const OBJCODEBASE: Region = Region { plain_base: 0x10000, debug_base: 0x10000, plain_len: 0x2, debug_len: 0x2 };

/// `Player_Init` .. `PState_Ground` — gate `SIGIL_EMP_PLAYER_COMMON`. tests: test_p1_player_port
pub const PLAYER_COMMON: Region = Region { plain_base: 0x10002, debug_base: 0x10002, plain_len: 0x4FE, debug_len: 0x61E };

/// `CharDef_Sonic` .. `CharDef_Tails` — gate `SIGIL_EMP_SONIC`. tests: test_p1_player_port
pub const SONIC: Region = Region { plain_base: 0x10E84, debug_base: 0x10FB0, plain_len: 0x30, debug_len: 0x30 };

/// `CharDef_Tails` .. `CharacterDefs` — gate `SIGIL_EMP_TAILS`. tests: test_p1_player_port
pub const TAILS: Region = Region { plain_base: 0x10EB4, debug_base: 0x10FE0, plain_len: 0x30, debug_len: 0x30 };

/// `CharacterDefs` .. `TestStatic_Main` — gate `SIGIL_EMP_CHARACTERS`. tests: test_p1_player_port
pub const CHARACTERS: Region = Region { plain_base: 0x10EE4, debug_base: 0x11010, plain_len: 0x4C, debug_len: 0x50 };

/// `PState_Ground` .. `PState_Air` — gate `SIGIL_EMP_PLAYER_GROUND`. tests: test_p2_player_states_port
pub const PLAYER_GROUND: Region = Region { plain_base: 0x10500, debug_base: 0x10620, plain_len: 0x490, debug_len: 0x490 };

/// `PState_Air` .. `PState_Spindash` — gate `SIGIL_EMP_PLAYER_AIR`. tests: test_p2_player_states_port
pub const PLAYER_AIR: Region = Region { plain_base: 0x10990, debug_base: 0x10AB0, plain_len: 0x350, debug_len: 0x350 };

/// `PState_Spindash` .. `PState_Fly` — gate `SIGIL_EMP_PLAYER_SPINDASH`. tests: test_p2_player_states_port
pub const PLAYER_SPINDASH: Region = Region { plain_base: 0x10CE0, debug_base: 0x10E00, plain_len: 0x9C, debug_len: 0x9C };

/// `PState_Fly` .. `CharDef_Sonic` — gate `SIGIL_EMP_PLAYER_FLY`. tests: test_p2_player_states_port
pub const PLAYER_FLY: Region = Region { plain_base: 0x10D7C, debug_base: 0x10E9C, plain_len: 0x108, debug_len: 0x114 };

/// `Collision_ProbeDown` .. `Section_Init` — gate `SIGIL_EMP_PLAYER_SENSORS`. tests: test_p4_player_sensors_port
pub const PLAYER_SENSORS: Region = Region { plain_base: 0x5470, debug_base: 0x63F0, plain_len: 0x4F4, debug_len: 0x4F4 };

// ── Symbols (manifest order) ──

/// `TestStatic_Main`. tests: objdef_port
pub const TEST_STATIC_MAIN: Pin = Pin { plain: 0x10F30, debug: 0x11060 };

/// `TestSolid_Init`. tests: objdef_port
pub const TEST_SOLID_INIT: Pin = Pin { plain: 0x10F40, debug: 0x113AC };

/// `TestEnemy_Init` — debug-shape consumer only (`debug_only`). tests: objdef_port
pub const TEST_ENEMY_INIT: u32 = 0x11364;

/// `TestParent` — debug-shape consumer only (`debug_only`). tests: objdef_port
pub const TEST_PARENT_LABEL: u32 = 0x11500;

/// `Map_TestObj`. tests: objdef_port
pub const MAP_TEST_OBJ: Pin = Pin { plain: 0x26410, debug: 0x26B9A };

/// `Map_Sonic`. tests: test_g1_objects_port
pub const MAP_SONIC: Pin = Pin { plain: 0x287C8, debug: 0x28F60 };

/// `DPLC_Sonic`. tests: test_g1_objects_port
pub const DPLC_SONIC: Pin = Pin { plain: 0x2A448, debug: 0x2ABE0 };

/// `Art_Sonic`. tests: test_g1_objects_port
pub const ART_SONIC: Pin = Pin { plain: 0x2AD88, debug: 0x2B520 };

/// `CreateEffect_Normal`. tests: test_g2_objects_port
pub const CREATE_EFFECT_NORMAL: Pin = Pin { plain: 0x42D6, debug: 0x50F6 };

/// `CreateChild_Normal`. tests: test_g3_objects_port
pub const CREATE_CHILD_NORMAL: Pin = Pin { plain: 0x40AC, debug: 0x4E1C };

/// `DeleteChildren`. tests: test_g3_objects_port
pub const DELETE_CHILDREN: Pin = Pin { plain: 0x42B8, debug: 0x50D8 };

/// `GetSineCosine`. tests: test_g3_objects_port
pub const GET_SINE_COSINE: Pin = Pin { plain: 0x27E0, debug: 0x2A40 };

/// `EntryPoint`. tests: m1c_vector_table
pub const ENTRY_POINT: Pin = Pin { plain: 0x200, debug: 0x200 };

/// `BusError` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const BUS_ERROR: u32 = 0x7F824;

/// `AddressError` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ADDRESS_ERROR: u32 = 0x7F83C;

/// `IllegalInstr` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ILLEGAL_INSTR: u32 = 0x7F858;

/// `ZeroDivide` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ZERO_DIVIDE: u32 = 0x7F87A;

/// `ChkInstr` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const CHK_INSTR: u32 = 0x7F894;

/// `TrapvInstr` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const TRAPV_INSTR: u32 = 0x7F8B2;

/// `PrivilegeViol` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const PRIVILEGE_VIOL: u32 = 0x7F8D2;

/// `Trace` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const TRACE: u32 = 0x7F8F4;

/// `Line1010Emu` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const LINE1010_EMU: u32 = 0x7F908;

/// `Line1111Emu` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const LINE1111_EMU: u32 = 0x7F928;

/// `ErrorExcept` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ERROR_EXCEPT: u32 = 0x7F948;

/// `ErrorTrap` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ERROR_TRAP: u32 = 0x7F966;

/// `VBlank_Handler`. tests: m1c_vector_table
pub const V_BLANK_HANDLER: Pin = Pin { plain: 0x21E0, debug: 0x2280 };

/// `HBlank_Vector_Slot`. tests: hblank_port, m1c_vector_table
pub const H_BLANK_VECTOR_SLOT: Pin = Pin { plain: 0xFFFFB0A4, debug: 0xFFFFB132 };

/// `VDP_Shadow_Table`. tests: vdp_init_port
pub const VDP_SHADOW_TABLE: Pin = Pin { plain: 0xFFFF800E, debug: 0xFFFF800E };

/// `VDP_Dirty_Mask`. tests: vdp_init_port
pub const VDP_DIRTY_MASK: Pin = Pin { plain: 0xFFFF8022, debug: 0xFFFF8022 };

/// `BootData_VDPRegs`. tests: vdp_init_port
pub const BOOT_DATA_VDP_REGS: Pin = Pin { plain: 0x3BA, debug: 0x3CA };

/// `Ctrl_1_Held`. tests: controllers_port
pub const CTRL_1_HELD: Pin = Pin { plain: 0xFFFF802C, debug: 0xFFFF802C };

/// `VSync_Wait`. tests: game_loop_port, load_art_port
pub const V_SYNC_WAIT: Pin = Pin { plain: 0x237E, debug: 0x2426 };

/// `Sound_DrainSfxRing`. tests: game_loop_port, load_art_port
pub const SOUND_DRAIN_SFX_RING: Pin = Pin { plain: 0x702A, debug: 0x9876 };

/// `Game_State`. tests: game_loop_port, load_art_port
pub const GAME_STATE: Pin = Pin { plain: 0xFFFF8008, debug: 0xFFFF8008 };

/// `Input_Tick`. tests: game_loop_port, game_debug_port
pub const INPUT_TICK: Pin = Pin { plain: 0x252A, debug: 0x25DC };

/// `Cache_Left_Col`. tests: collision_lookup_port, section_port
pub const CACHE_LEFT_COL: Pin = Pin { plain: 0xFFFFA852, debug: 0xFFFFA8E0 };

/// `Draw_TileColumn`. tests: section_port
pub const DRAW_TILE_COLUMN: Pin = Pin { plain: 0x4400, debug: 0x5220 };

/// `Draw_TileRow_FromCache`. tests: section_port
pub const DRAW_TILE_ROW_FROM_CACHE: Pin = Pin { plain: 0x4554, debug: 0x5374 };

/// `EntityWindow_Init`. tests: section_port
pub const ENTITY_WINDOW_INIT: Pin = Pin { plain: 0x3B4A, debug: 0x47CE };

/// `Section_Plane_Dirty`. tests: section_port
pub const SECTION_PLANE_DIRTY: Pin = Pin { plain: 0xFFFFA8C6, debug: 0xFFFFA954 };

/// `Section_Right_Col_Written`. tests: section_port
pub const SECTION_RIGHT_COL_WRITTEN: Pin = Pin { plain: 0xFFFFA8C8, debug: 0xFFFFA956 };

/// `Section_Left_Col_Written`. tests: section_port
pub const SECTION_LEFT_COL_WRITTEN: Pin = Pin { plain: 0xFFFFA8CA, debug: 0xFFFFA958 };

/// `Section_Top_Row_Written`. tests: section_port
pub const SECTION_TOP_ROW_WRITTEN: Pin = Pin { plain: 0xFFFFA8C2, debug: 0xFFFFA950 };

/// `Section_Bottom_Row_Written`. tests: section_port
pub const SECTION_BOTTOM_ROW_WRITTEN: Pin = Pin { plain: 0xFFFFA8C4, debug: 0xFFFFA952 };

/// `Cache_Head_Col`. tests: section_port
pub const CACHE_HEAD_COL: Pin = Pin { plain: 0xFFFFA854, debug: 0xFFFFA8E2 };

/// `Cache_Top_Row`. tests: section_port
pub const CACHE_TOP_ROW: Pin = Pin { plain: 0xFFFFA856, debug: 0xFFFFA8E4 };

/// `Cache_Bottom_Row`. tests: section_port
pub const CACHE_BOTTOM_ROW: Pin = Pin { plain: 0xFFFFA858, debug: 0xFFFFA8E6 };

/// `Cache_Origin_Col`. tests: section_port
pub const CACHE_ORIGIN_COL: Pin = Pin { plain: 0xFFFFA85A, debug: 0xFFFFA8E8 };

/// `Cache_Origin_Row`. tests: section_port
pub const CACHE_ORIGIN_ROW: Pin = Pin { plain: 0xFFFFA85C, debug: 0xFFFFA8EA };

/// `Plane_Buffer_Ptr`. tests: section_port
pub const PLANE_BUFFER_PTR: Pin = Pin { plain: 0xFFFFA73E, debug: 0xFFFFA7CC };

/// `Plane_Buffer`. tests: plane_buffer_port
pub const PLANE_BUFFER_BASE: Pin = Pin { plain: 0xFFFFA13E, debug: 0xFFFFA1CC };

/// `Tile_Cache_Nametable`. tests: section_port
pub const TILE_CACHE_NAMETABLE: Pin = Pin { plain: 0xFFFF0000, debug: 0xFFFF0000 };

/// `Tile_Cache_Collision`. tests: tile_cache_port, collision_lookup_port
pub const TILE_CACHE_COLLISION: Pin = Pin { plain: 0xFFFF2580, debug: 0xFFFF2580 };

/// `Frame_Counter`. tests: tile_cache_port
pub const FRAME_COUNTER: Pin = Pin { plain: 0xFFFF8002, debug: 0xFFFF8002 };

/// `Logic_Tick`. tests: game_loop_port, bg_anim_port
pub const LOGIC_TICK: Pin = Pin { plain: 0xFFFF8004, debug: 0xFFFF8004 };

/// `Block_Stage_Keys`. tests: tile_cache_port
pub const BLOCK_STAGE_KEYS: Pin = Pin { plain: 0xFFFFA880, debug: 0xFFFFA90E };

/// `Block_Stage_Next`. tests: tile_cache_port
pub const BLOCK_STAGE_NEXT: Pin = Pin { plain: 0xFFFFA8C0, debug: 0xFFFFA94E };

/// `Block_Stage_Buffers`. tests: tile_cache_port
pub const BLOCK_STAGE_BUFFERS: Pin = Pin { plain: 0xFFFF3842, debug: 0xFFFF3842 };

/// `Block_Stage_Ptrs`. tests: tile_cache_port
pub const BLOCK_STAGE_PTRS: Pin = Pin { plain: 0xFFFFB0AA, debug: 0xFFFFB138 };

/// `Block_Stage_ZeroPage`. tests: tile_cache_port
pub const BLOCK_STAGE_ZERO_PAGE: Pin = Pin { plain: 0xFFFFB12E, debug: 0xFFFFB1BC };

/// `Cache_Fill_Last_Frame`. tests: tile_cache_port
pub const CACHE_FILL_LAST_FRAME: Pin = Pin { plain: 0xFFFFA85E, debug: 0xFFFFA8EC };

/// `Cache_Fill_Budget`. tests: tile_cache_port
pub const CACHE_FILL_BUDGET: Pin = Pin { plain: 0xFFFFA868, debug: 0xFFFFA8F6 };

/// `Cache_Fill_Resume_Col`. tests: tile_cache_port
pub const CACHE_FILL_RESUME_COL: Pin = Pin { plain: 0xFFFFA860, debug: 0xFFFFA8EE };

/// `Cache_Fill_Resume_Row`. tests: tile_cache_port
pub const CACHE_FILL_RESUME_ROW: Pin = Pin { plain: 0xFFFFA862, debug: 0xFFFFA8F0 };

/// `Cache_Fill_RowResume_Row`. tests: tile_cache_port
pub const CACHE_FILL_ROW_RESUME_ROW: Pin = Pin { plain: 0xFFFFA86A, debug: 0xFFFFA8F8 };

/// `Cache_Fill_RowResume_Col`. tests: tile_cache_port
pub const CACHE_FILL_ROW_RESUME_COL: Pin = Pin { plain: 0xFFFFA86C, debug: 0xFFFFA8FA };

/// `Cache_Fill_Rows_Left`. tests: tile_cache_port
pub const CACHE_FILL_ROWS_LEFT: Pin = Pin { plain: 0xFFFFA86E, debug: 0xFFFFA8FC };

/// `Cache_Prev_Cam_Row`. tests: tile_cache_port
pub const CACHE_PREV_CAM_ROW: Pin = Pin { plain: 0xFFFFA870, debug: 0xFFFFA8FE };

/// `Cache_Prev_Cam_X`. tests: tile_cache_port
pub const CACHE_PREV_CAM_X: Pin = Pin { plain: 0xFFFFA872, debug: 0xFFFFA900 };

/// `Cache_H_Pfx_Dir`. tests: tile_cache_port
pub const CACHE_H_PFX_DIR: Pin = Pin { plain: 0xFFFFA874, debug: 0xFFFFA902 };

/// `Cache_H_Pfx_Accum`. tests: tile_cache_port
pub const CACHE_H_PFX_ACCUM: Pin = Pin { plain: 0xFFFFA876, debug: 0xFFFFA904 };

/// `Cache_Pfx_Row_Target`. tests: tile_cache_port
pub const CACHE_PFX_ROW_TARGET: Pin = Pin { plain: 0xFFFFA878, debug: 0xFFFFA906 };

/// `Cache_Pfx_Col_Target`. tests: tile_cache_port
pub const CACHE_PFX_COL_TARGET: Pin = Pin { plain: 0xFFFFA87A, debug: 0xFFFFA908 };

/// `Cache_Pfx_Skip_Armed`. tests: tile_cache_port
pub const CACHE_PFX_SKIP_ARMED: Pin = Pin { plain: 0xFFFFA87C, debug: 0xFFFFA90A };

/// `Cache_Pfx_Lag_Flag`. tests: tile_cache_port
pub const CACHE_PFX_LAG_FLAG: Pin = Pin { plain: 0xFFFFA87E, debug: 0xFFFFA90C };

/// `Block_Stage_Gen`. tests: tile_cache_port
pub const BLOCK_STAGE_GEN: Pin = Pin { plain: 0xFFFFB092, debug: 0xFFFFB120 };

/// `Pfx_Memo_Row`. tests: tile_cache_port
pub const PFX_MEMO_ROW: Pin = Pin { plain: 0xFFFFB094, debug: 0xFFFFB122 };

/// `Pfx_Memo_L`. tests: tile_cache_port
pub const PFX_MEMO_L: Pin = Pin { plain: 0xFFFFB096, debug: 0xFFFFB124 };

/// `Pfx_Memo_H`. tests: tile_cache_port
pub const PFX_MEMO_H: Pin = Pin { plain: 0xFFFFB098, debug: 0xFFFFB126 };

/// `Pfx_Memo_Gen`. tests: tile_cache_port
pub const PFX_MEMO_GEN: Pin = Pin { plain: 0xFFFFB09A, debug: 0xFFFFB128 };

/// `Cs_Memo_Col`. tests: tile_cache_port
pub const CS_MEMO_COL: Pin = Pin { plain: 0xFFFFB09C, debug: 0xFFFFB12A };

/// `Cs_Memo_T`. tests: tile_cache_port
pub const CS_MEMO_T: Pin = Pin { plain: 0xFFFFB09E, debug: 0xFFFFB12C };

/// `Cs_Memo_B`. tests: tile_cache_port
pub const CS_MEMO_B: Pin = Pin { plain: 0xFFFFB0A0, debug: 0xFFFFB12E };

/// `Cs_Memo_Gen`. tests: tile_cache_port
pub const CS_MEMO_GEN: Pin = Pin { plain: 0xFFFFB0A2, debug: 0xFFFFB130 };

/// `S4LZ_DecompressDict`. tests: tile_cache_port
pub const S4_LZ_DECOMPRESS_DICT: Pin = Pin { plain: 0x2670, debug: 0x27C8 };

/// `Player_1`. tests: collision_port, rings_port
pub const PLAYER_1: Pin = Pin { plain: 0xFFFF8A02, debug: 0xFFFF8A90 };

/// `Cheat_Flags`. tests: test_g4_final_objects_port, test_p1_player_port
pub const CHEAT_FLAGS: Pin = Pin { plain: 0xFFFFB492, debug: 0xFFFFDD28 };

/// `Dynamic_Slots`. tests: collision_port
pub const DYNAMIC_SLOTS: Pin = Pin { plain: 0xFFFF8AA2, debug: 0xFFFF8B30 };

/// `Ring_Buffer`. tests: rings_port
pub const RING_BUFFER: Pin = Pin { plain: 0xFFFFA934, debug: 0xFFFFA9C2 };

/// `Ring_Count`. tests: rings_port
pub const RING_COUNT: Pin = Pin { plain: 0xFFFFAC34, debug: 0xFFFFACC2 };

/// `Ring_HighWater`. tests: rings_port
pub const RING_HIGH_WATER: Pin = Pin { plain: 0xFFFFAC35, debug: 0xFFFFACC3 };

/// `Ring_Add_Dropped`. tests: rings_port
pub const RING_ADD_DROPPED: Pin = Pin { plain: 0xFFFFAC36, debug: 0xFFFFACC4 };

/// `Ring_Counter`. tests: rings_port
pub const RING_COUNTER: Pin = Pin { plain: 0xFFFFACA0, debug: 0xFFFFAD2E };

/// `Ring_Anim_Frame`. tests: rings_port
pub const RING_ANIM_FRAME: Pin = Pin { plain: 0xFFFFACA2, debug: 0xFFFFAD30 };

/// `Ring_Anim_Timer`. tests: rings_port
pub const RING_ANIM_TIMER: Pin = Pin { plain: 0xFFFFACA3, debug: 0xFFFFAD31 };

/// `Camera_X`. tests: rings_port, section_port, camera_port, bg_anim_port
pub const CAMERA_X: Pin = Pin { plain: 0xFFFFA130, debug: 0xFFFFA1BE };

/// `Camera_Y`. tests: rings_port, section_port, camera_port, bg_anim_port
pub const CAMERA_Y: Pin = Pin { plain: 0xFFFFA134, debug: 0xFFFFA1C2 };

/// `Camera_Target`. tests: camera_port, test_g4_final_objects_port, test_t1_harness_states_port
pub const CAMERA_TARGET: Pin = Pin { plain: 0xFFFFA84C, debug: 0xFFFFA8DA };

/// `Camera_Curl_Offset`. tests: camera_port, test_p1_player_port
pub const CAMERA_CURL_OFFSET: Pin = Pin { plain: 0xFFFFA84E, debug: 0xFFFFA8DC };

/// `Camera_Deadzone_Base`. tests: camera_port
pub const CAMERA_DEADZONE_BASE: Pin = Pin { plain: 0xFFFFA842, debug: 0xFFFFA8D0 };

/// `Camera_Pan_Offset`. tests: camera_port
pub const CAMERA_PAN_OFFSET: Pin = Pin { plain: 0xFFFFA846, debug: 0xFFFFA8D4 };

/// `Camera_Hold_Frames`. tests: camera_port
pub const CAMERA_HOLD_FRAMES: Pin = Pin { plain: 0xFFFFA850, debug: 0xFFFFA8DE };

/// `Camera_Art_Hold`. tests: camera_port, tile_cache_port
pub const CAMERA_ART_HOLD: Pin = Pin { plain: 0xFFFFA851, debug: 0xFFFFA8DF };

/// `Dbg_Cam_Clamp_Frames` — debug-shape consumer only (`debug_only`). tests: camera_port
pub const DBG_CAM_CLAMP_FRAMES: u32 = 0xFFFF8A36;

/// `Camera_X_Max`. tests: camera_port
pub const CAMERA_X_MAX: Pin = Pin { plain: 0xFFFFA848, debug: 0xFFFFA8D6 };

/// `Camera_Y_Max`. tests: camera_port
pub const CAMERA_Y_MAX: Pin = Pin { plain: 0xFFFFA84A, debug: 0xFFFFA8D8 };

/// `BgAnim_LastStep`. tests: bg_anim_port
pub const BG_ANIM_LAST_STEP: Pin = Pin { plain: 0xFFFF8998, debug: 0xFFFF8998 };

/// `BgAnim_Table`. tests: bg_anim_port
pub const BG_ANIM_TABLE: Pin = Pin { plain: 0x26402, debug: 0x26B98 };

/// `Camera_X_Biased`. tests: sprites_port
pub const CAMERA_X_BIASED: Pin = Pin { plain: 0xFFFFA138, debug: 0xFFFFA1C6 };

/// `Camera_Y_Biased`. tests: sprites_port
pub const CAMERA_Y_BIASED: Pin = Pin { plain: 0xFFFFA13A, debug: 0xFFFFA1C8 };

/// `Collected_MarkRing`. tests: rings_port
pub const COLLECTED_MARK_RING: Pin = Pin { plain: 0x380E, debug: 0x4176 };

/// `EntityWindow_EntryForSection`. tests: rings_port
pub const ENTITY_WINDOW_ENTRY_FOR_SECTION: Pin = Pin { plain: 0x3A2A, debug: 0x4658 };

/// `EntityLoaded_Clear`. tests: rings_port
pub const ENTITY_LOADED_CLEAR: Pin = Pin { plain: 0x3A16, debug: 0x45E2 };

/// `Sound_PlayRing`. tests: rings_port
pub const SOUND_PLAY_RING: Pin = Pin { plain: 0x707A, debug: 0x98C6 };

/// `MDDBG__ErrorHandler` — debug-shape consumer only (`debug_only`). tests: rings_port
pub const MDDBG_ERROR_HANDLER: u32 = 0x7F97E;

/// `MDDBG__ErrorHandler_PagesController` — debug-shape consumer only (`debug_only`). tests: rings_port
pub const MDDBG_ERROR_HANDLER_PAGES_CONTROLLER: u32 = 0x80744;

/// `DMA_Critical`. tests: dma_queue_port
pub const DMA_CRITICAL: Pin = Pin { plain: 0xFFFF804E, debug: 0xFFFF804E };

/// `DMA_Critical_End`. tests: dma_queue_port
pub const DMA_CRITICAL_END: Pin = Pin { plain: 0xFFFF80BE, debug: 0xFFFF80BE };

/// `DMA_Important`. tests: dma_queue_port
pub const DMA_IMPORTANT: Pin = Pin { plain: 0xFFFF80BE, debug: 0xFFFF80BE };

/// `DMA_Important_End`. tests: dma_queue_port
pub const DMA_IMPORTANT_END: Pin = Pin { plain: 0xFFFF8166, debug: 0xFFFF8166 };

/// `DMA_Deferrable`. tests: dma_queue_port
pub const DMA_DEFERRABLE: Pin = Pin { plain: 0xFFFF8166, debug: 0xFFFF8166 };

/// `DMA_Deferrable_End`. tests: dma_queue_port
pub const DMA_DEFERRABLE_END: Pin = Pin { plain: 0xFFFF820E, debug: 0xFFFF820E };

/// `DMA_Critical_Slot`. tests: dma_queue_port
pub const DMA_CRITICAL_SLOT: Pin = Pin { plain: 0xFFFF820E, debug: 0xFFFF820E };

/// `DMA_Important_Slot`. tests: dma_queue_port
pub const DMA_IMPORTANT_SLOT: Pin = Pin { plain: 0xFFFF8210, debug: 0xFFFF8210 };

/// `DMA_Deferrable_Slot`. tests: dma_queue_port
pub const DMA_DEFERRABLE_SLOT: Pin = Pin { plain: 0xFFFF8212, debug: 0xFFFF8212 };

/// `DMA_Budget_Remaining`. tests: dma_queue_port
pub const DMA_BUDGET_REMAINING: Pin = Pin { plain: 0xFFFF8216, debug: 0xFFFF8216 };

/// `DMA_Enq_Bytes_Frame`. tests: bg_anim_port, dma_queue_port, dplc_port, game_loop_port, load_art_port, vblank_port
pub const DMA_ENQ_BYTES_FRAME: Pin = Pin { plain: 0xFFFF8218, debug: 0xFFFF8218 };

/// `Act_Art_Budget`. tests: load_art_port
pub const ACT_ART_BUDGET: Pin = Pin { plain: 0xFFFFB48E, debug: 0xFFFFB51C };

/// `Art_Budget_Remaining`. tests: load_art_port
pub const ART_BUDGET_REMAINING: Pin = Pin { plain: 0xFFFFB490, debug: 0xFFFFB51E };

/// `PageIn_Pool_Pages`. tests: load_art_port
pub const PAGE_IN_POOL_PAGES: Pin = Pin { plain: 0xFFFFB482, debug: 0xFFFFB510 };

/// `PageIn_Bulk_Drain`. tests: load_art_port
pub const PAGE_IN_BULK_DRAIN: Pin = Pin { plain: 0xFFFFB47D, debug: 0xFFFFB50B };

/// `PageIn_Fully_Resident`. tests: load_art_port
pub const PAGE_IN_FULLY_RESIDENT: Pin = Pin { plain: 0xFFFFB484, debug: 0xFFFFB512 };

/// `Block_Stage_Maps`. tests: tile_cache_port
pub const BLOCK_STAGE_MAPS: Pin = Pin { plain: 0xFFFFB0EA, debug: 0xFFFFB178 };

/// `Cache_Cur_LocalMap`. tests: tile_cache_port
pub const CACHE_CUR_LOCAL_MAP: Pin = Pin { plain: 0xFFFFB12A, debug: 0xFFFFB1B8 };

/// `Dbg_DMA_Enq_Capped` — debug-shape consumer only (`debug_only`). tests: bg_anim_port, dma_queue_port, dplc_port
pub const DBG_DMA_ENQ_CAPPED: u32 = 0xFFFF8A0C;

/// `DMA_Overflow_Count` — debug-shape consumer only (`debug_only`). tests: dma_queue_port
pub const DMA_OVERFLOW_COUNT: u32 = 0xFFFF8A0A;

/// `Art_Staging_Buffer`. tests: load_art_port
pub const ART_STAGING_BUFFER: Pin = Pin { plain: 0xFFFF69DA, debug: 0xFFFF69DA };

/// `S4LZ_Decompress`. tests: load_art_port
pub const S4_LZ_DECOMPRESS: Pin = Pin { plain: 0x2674, debug: 0x2820 };

/// `QueueDMA_Critical`. tests: load_art_port
pub const QUEUE_DMA_CRITICAL: Pin = Pin { plain: 0x1D68, debug: 0x1DFE };

/// `BG_Init`. tests: load_art_port
pub const BG_INIT: Pin = Pin { plain: 0x6D40, debug: 0x8540 };

/// `QueueDMA_Important`. tests: dplc_port
pub const QUEUE_DMA_IMPORTANT: Pin = Pin { plain: 0x1D72, debug: 0x1E08 };

/// `QueueDMA_Deferrable`. tests: dplc_port
pub const QUEUE_DMA_DEFERRABLE: Pin = Pin { plain: 0x1D7C, debug: 0x1E12 };

/// `Object_RAM`. tests: core_port
pub const OBJECT_RAM: Pin = Pin { plain: 0xFFFF8A02, debug: 0xFFFF8A90 };

/// `System_Slots`. tests: core_port
pub const SYSTEM_SLOTS: Pin = Pin { plain: 0xFFFF9722, debug: 0xFFFF97B0 };

/// `Effect_Slots`. tests: core_port
pub const EFFECT_SLOTS: Pin = Pin { plain: 0xFFFF99A2, debug: 0xFFFF9A30 };

/// `Game_Paused`. tests: core_port
pub const GAME_PAUSED: Pin = Pin { plain: 0xFFFFA13C, debug: 0xFFFFA1CA };

/// `Object_RAM_End`. tests: core_port
pub const OBJECT_RAM_END: Pin = Pin { plain: 0xFFFF9EA2, debug: 0xFFFF9F30 };

/// `Dynamic_Free_Stack`. tests: core_port
pub const DYNAMIC_FREE_STACK: Pin = Pin { plain: 0xFFFF9EA2, debug: 0xFFFF9F30 };

/// `Dynamic_Free_SP`. tests: core_port
pub const DYNAMIC_FREE_SP: Pin = Pin { plain: 0xFFFF9EF2, debug: 0xFFFF9F80 };

/// `Effect_Free_Stack`. tests: core_port
pub const EFFECT_FREE_STACK: Pin = Pin { plain: 0xFFFF9EF4, debug: 0xFFFF9F82 };

/// `Effect_Free_SP`. tests: core_port
pub const EFFECT_FREE_SP: Pin = Pin { plain: 0xFFFF9F14, debug: 0xFFFF9FA2 };

/// `Dynamic_Live`. tests: core_port
pub const DYNAMIC_LIVE: Pin = Pin { plain: 0xFFFFB02C, debug: 0xFFFFB0BA };

/// `Dynamic_Live_Count`. tests: core_port
pub const DYNAMIC_LIVE_COUNT: Pin = Pin { plain: 0xFFFFB07C, debug: 0xFFFFB10A };

/// `Dynamic_Live_Dirty`. tests: core_port
pub const DYNAMIC_LIVE_DIRTY: Pin = Pin { plain: 0xFFFFB07E, debug: 0xFFFFB10C };

/// `Dynamic_Live_Walking` — debug-shape consumer only (`debug_only`). tests: core_port, collision_port, entity_window_port
pub const DYNAMIC_LIVE_WALKING: u32 = 0xFFFFB10D;

/// `Dynamic_Live_Pending`. tests: core_port
pub const DYNAMIC_LIVE_PENDING: Pin = Pin { plain: 0xFFFFB080, debug: 0xFFFFB10E };

/// `Dynamic_Live_Pending_Count`. tests: core_port
pub const DYNAMIC_LIVE_PENDING_COUNT: Pin = Pin { plain: 0xFFFFB090, debug: 0xFFFFB11E };

/// `DeleteObject`. tests: animate_port, children_port
pub const DELETE_OBJECT: Pin = Pin { plain: 0x2BF0, debug: 0x2E50 };

/// `DrawRings`. tests: sprites_port
pub const DRAW_RINGS: Pin = Pin { plain: 0x365A, debug: 0x3F60 };

/// `Sprite_Table_Buffer`. tests: sprites_port
pub const SPRITE_TABLE_BUFFER: Pin = Pin { plain: 0xFFFF829C, debug: 0xFFFF829C };

/// `Sprite_Table_Dirty`. tests: sprites_port
pub const SPRITE_TABLE_DIRTY: Pin = Pin { plain: 0xFFFF851C, debug: 0xFFFF851C };

/// `Sprite_Emit_Active`. tests: sprites_port, buffers_port
pub const SPRITE_EMIT_ACTIVE: Pin = Pin { plain: 0xFFFF851D, debug: 0xFFFF851D };

/// `Sprite_Bands`. tests: sprites_port
pub const SPRITE_BANDS: Pin = Pin { plain: 0xFFFF9F16, debug: 0xFFFF9FA4 };

/// `Sprite_Band_Counts`. tests: sprites_port
pub const SPRITE_BAND_COUNTS: Pin = Pin { plain: 0xFFFFA116, debug: 0xFFFFA1A4 };

/// `Sprites_Rendered`. tests: sprites_port
pub const SPRITES_RENDERED: Pin = Pin { plain: 0xFFFFA11E, debug: 0xFFFFA1AC };

/// `Sprite_Cycle_Counter`. tests: sprites_port
pub const SPRITE_CYCLE_COUNTER: Pin = Pin { plain: 0xFFFFA120, debug: 0xFFFFA1AE };

/// `SpriteMask_Y`. tests: sprites_port
pub const SPRITE_MASK_Y: Pin = Pin { plain: 0xFFFFA122, debug: 0xFFFFA1B0 };

/// `SpriteMask_Height`. tests: sprites_port
pub const SPRITE_MASK_HEIGHT: Pin = Pin { plain: 0xFFFFA124, debug: 0xFFFFA1B2 };

/// `SpriteMask_After_Band`. tests: sprites_port
pub const SPRITE_MASK_AFTER_BAND: Pin = Pin { plain: 0xFFFFA126, debug: 0xFFFFA1B4 };

/// `Scanline_Band_Sprites`. tests: sprites_port
pub const SCANLINE_BAND_SPRITES: Pin = Pin { plain: 0xFFFFA128, debug: 0xFFFFA1B6 };

/// `Sound_PlaySFX`. tests: animate_port
pub const SOUND_PLAY_SFX: Pin = Pin { plain: 0x6FE4, debug: 0x97EA };

/// `ObjectMoveX`. tests: test_g4_final_objects_port
pub const OBJECT_MOVE_X: Pin = Pin { plain: 0x2DFC, debug: 0x34A6 };

/// `ObjCodeBase`. tests: test_objects_port
pub const OBJ_CODE_BASE: Pin = Pin { plain: 0x10000, debug: 0x10000 };

/// `Draw_Sprite`. tests: test_objects_port
pub const DRAW_SPRITE: Pin = Pin { plain: 0x2E34, debug: 0x34E4 };

/// `ObjectMove`. tests: test_objects_port
pub const OBJECT_MOVE: Pin = Pin { plain: 0x2DE2, debug: 0x348C };

/// `Ring_Sfx_Speaker`. tests: sound_api_port
pub const RING_SFX_SPEAKER: Pin = Pin { plain: 0xFFFFAF70, debug: 0xFFFFAFFE };

/// `Sfx_Ring_Buf`. tests: sound_api_port
pub const SFX_RING_BUF: Pin = Pin { plain: 0xFFFFAF72, debug: 0xFFFFB000 };

/// `Sfx_Ring_Wr`. tests: sound_api_port
pub const SFX_RING_WR: Pin = Pin { plain: 0xFFFFAF7A, debug: 0xFFFFB008 };

/// `Sfx_Ring_Rd`. tests: sound_api_port
pub const SFX_RING_RD: Pin = Pin { plain: 0xFFFFAF7B, debug: 0xFFFFB009 };

/// `SongTable`. tests: sound_api_port
pub const SONG_TABLE: Pin = Pin { plain: 0x5BB08, debug: 0x5D548 };

/// `SongPatchTable`. tests: sound_api_port
pub const SONG_PATCH_TABLE: Pin = Pin { plain: 0x5BB0C, debug: 0x5D554 };

/// `OJZ_Palette`. tests: act_descriptor_port
pub const OJZ_PALETTE: Pin = Pin { plain: 0x20B80, debug: 0x21316 };

/// `OJZ_Act1_BG_Layout`. tests: act_descriptor_port
pub const OJZ_ACT1_BG_LAYOUT: Pin = Pin { plain: 0x20C00, debug: 0x21396 };

/// `OJZ_Act1_BG_Tiles`. tests: act_descriptor_port
pub const OJZ_ACT1_BG_TILES: Pin = Pin { plain: 0x22C00, debug: 0x23396 };

/// `ParallaxConfig_OJZ_Default`. tests: act_descriptor_port
pub const PARALLAX_CONFIG_OJZ_DEFAULT: Pin = Pin { plain: 0x110E4, debug: 0x11888 };

/// `OJZ_Act_Pool_PageTable`. tests: act_descriptor_port
pub const OJZ_ACT_POOL_PAGE_TABLE: Pin = Pin { plain: 0x14B3E, debug: 0x152DC };

/// `OJZ_Sec_LocalMaps`. tests: act_descriptor_port
pub const OJZ_SEC_LOCAL_MAPS: Pin = Pin { plain: 0x20B58, debug: 0x212F2 };

/// `OJZ_Sec0_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC0_BLOCKS: Pin = Pin { plain: 0x14E08, debug: 0x155B0 };

/// `OJZ_Sec1_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC1_BLOCKS: Pin = Pin { plain: 0x169F8, debug: 0x171A0 };

/// `OJZ_Sec2_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC2_BLOCKS: Pin = Pin { plain: 0x17D74, debug: 0x1851C };

/// `OJZ_Sec3_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC3_BLOCKS: Pin = Pin { plain: 0x1950C, debug: 0x19CB4 };

/// `OJZ_Sec4_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC4_BLOCKS: Pin = Pin { plain: 0x17D74, debug: 0x1851C };

/// `OJZ_Sec5_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC5_BLOCKS: Pin = Pin { plain: 0x1A658, debug: 0x1AE00 };

/// `OJZ_Sec6_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC6_BLOCKS: Pin = Pin { plain: 0x1B47E, debug: 0x1BC26 };

/// `OJZ_Sec7_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC7_BLOCKS: Pin = Pin { plain: 0x1D07E, debug: 0x1D826 };

/// `OJZ_Sec8_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC8_BLOCKS: Pin = Pin { plain: 0x1E2F2, debug: 0x1EA9A };

/// `OJZ_Sec0_Objects`. tests: act_descriptor_port
pub const OJZ_SEC0_OBJECTS: Pin = Pin { plain: 0x11AAE, debug: 0x1224C };

/// `OJZ_Sec0_Rings`. tests: act_descriptor_port
pub const OJZ_SEC0_RINGS: Pin = Pin { plain: 0x11AB6, debug: 0x12254 };

/// `OJZ_Sec0_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC0_TYPE_TABLE: Pin = Pin { plain: 0x11AA8, debug: 0x12246 };

/// `OJZ_Sec1_Objects`. tests: act_descriptor_port
pub const OJZ_SEC1_OBJECTS: Pin = Pin { plain: 0x11AE0, debug: 0x1227E };

/// `OJZ_Sec1_Rings`. tests: act_descriptor_port
pub const OJZ_SEC1_RINGS: Pin = Pin { plain: 0x11AF4, debug: 0x12292 };

/// `OJZ_Sec1_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC1_TYPE_TABLE: Pin = Pin { plain: 0x11AD6, debug: 0x12274 };

/// `OJZ_Sec2_Objects`. tests: act_descriptor_port
pub const OJZ_SEC2_OBJECTS: Pin = Pin { plain: 0x11B26, debug: 0x122C4 };

/// `OJZ_Sec2_Rings`. tests: act_descriptor_port
pub const OJZ_SEC2_RINGS: Pin = Pin { plain: 0x11B34, debug: 0x122D2 };

/// `OJZ_Sec2_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC2_TYPE_TABLE: Pin = Pin { plain: 0x11B1C, debug: 0x122BA };

/// `OJZ_Sec3_Objects`. tests: act_descriptor_port
pub const OJZ_SEC3_OBJECTS: Pin = Pin { plain: 0x11B6A, debug: 0x12308 };

/// `OJZ_Sec3_Rings`. tests: act_descriptor_port
pub const OJZ_SEC3_RINGS: Pin = Pin { plain: 0x11B6C, debug: 0x1230A };

/// `OJZ_Sec3_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC3_TYPE_TABLE: Pin = Pin { plain: 0x11B68, debug: 0x12306 };

/// `OJZ_Sec4_Objects`. tests: act_descriptor_port
pub const OJZ_SEC4_OBJECTS: Pin = Pin { plain: 0x11B72, debug: 0x12310 };

/// `OJZ_Sec4_Rings`. tests: act_descriptor_port
pub const OJZ_SEC4_RINGS: Pin = Pin { plain: 0x11B74, debug: 0x12312 };

/// `OJZ_Sec4_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC4_TYPE_TABLE: Pin = Pin { plain: 0x11B70, debug: 0x1230E };

/// `OJZ_Sec5_Objects`. tests: act_descriptor_port
pub const OJZ_SEC5_OBJECTS: Pin = Pin { plain: 0x11BAA, debug: 0x12348 };

/// `OJZ_Sec5_Rings`. tests: act_descriptor_port
pub const OJZ_SEC5_RINGS: Pin = Pin { plain: 0x11BAC, debug: 0x1234A };

/// `OJZ_Sec5_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC5_TYPE_TABLE: Pin = Pin { plain: 0x11BA8, debug: 0x12346 };

/// `OJZ_Sec6_Objects`. tests: act_descriptor_port
pub const OJZ_SEC6_OBJECTS: Pin = Pin { plain: 0x11BD2, debug: 0x12370 };

/// `OJZ_Sec6_Rings`. tests: act_descriptor_port
pub const OJZ_SEC6_RINGS: Pin = Pin { plain: 0x11BD4, debug: 0x12372 };

/// `OJZ_Sec6_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC6_TYPE_TABLE: Pin = Pin { plain: 0x11BD0, debug: 0x1236E };

/// `OJZ_Sec7_Objects`. tests: act_descriptor_port
pub const OJZ_SEC7_OBJECTS: Pin = Pin { plain: 0x11BDA, debug: 0x12378 };

/// `OJZ_Sec7_Rings`. tests: act_descriptor_port
pub const OJZ_SEC7_RINGS: Pin = Pin { plain: 0x11BDC, debug: 0x1237A };

/// `OJZ_Sec7_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC7_TYPE_TABLE: Pin = Pin { plain: 0x11BD8, debug: 0x12376 };

/// `OJZ_Sec8_Objects`. tests: act_descriptor_port
pub const OJZ_SEC8_OBJECTS: Pin = Pin { plain: 0x11C02, debug: 0x123A0 };

/// `OJZ_Sec8_Rings`. tests: act_descriptor_port
pub const OJZ_SEC8_RINGS: Pin = Pin { plain: 0x11C04, debug: 0x123A2 };

/// `OJZ_Sec8_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC8_TYPE_TABLE: Pin = Pin { plain: 0x11C00, debug: 0x1239E };

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
pub const CAMERA_Y_COARSE_PREV: Pin = Pin { plain: 0xFFFFADB0, debug: 0xFFFFAE3E };

/// `Current_Act_Ptr`. tests: entity_window_port, section_port
pub const CURRENT_ACT_PTR: Pin = Pin { plain: 0xFFFFAF6C, debug: 0xFFFFAFFA };

/// `Entity_Window_Active`. tests: entity_window_port
pub const ENTITY_WINDOW_ACTIVE: Pin = Pin { plain: 0xFFFFACA4, debug: 0xFFFFAD32 };

/// `Entity_Window_Anchor`. tests: entity_window_port
pub const ENTITY_WINDOW_ANCHOR: Pin = Pin { plain: 0xFFFFACA6, debug: 0xFFFFAD34 };

/// `Entity_Window_OriginX`. tests: entity_window_port
pub const ENTITY_WINDOW_ORIGIN_X: Pin = Pin { plain: 0xFFFFACA8, debug: 0xFFFFAD36 };

/// `Entity_Window_OriginY`. tests: entity_window_port
pub const ENTITY_WINDOW_ORIGIN_Y: Pin = Pin { plain: 0xFFFFACAA, debug: 0xFFFFAD38 };

/// `Entity_Window_Center_ID`. tests: entity_window_port
pub const ENTITY_WINDOW_CENTER_ID: Pin = Pin { plain: 0xFFFFACA5, debug: 0xFFFFAD33 };

/// `Entity_Scan_State`. tests: entity_window_port
pub const ENTITY_SCAN_STATE: Pin = Pin { plain: 0xFFFFAC38, debug: 0xFFFFACC6 };

/// `Entity_Loaded_Masks`. tests: entity_window_port
pub const ENTITY_LOADED_MASKS: Pin = Pin { plain: 0xFFFFACAC, debug: 0xFFFFAD3A };

/// `Entity_Mask_Scratch`. tests: entity_window_port
pub const ENTITY_MASK_SCRATCH: Pin = Pin { plain: 0xFFFFAD2C, debug: 0xFFFFADBA };

/// `Ring_Collected_Window`. tests: entity_window_port
pub const RING_COLLECTED_WINDOW: Pin = Pin { plain: 0xFFFFADB2, debug: 0xFFFFAE40 };

/// `Ring_Collected_Park`. tests: entity_window_port
pub const RING_COLLECTED_PARK: Pin = Pin { plain: 0xFFFFAEE6, debug: 0xFFFFAF74 };

/// `Collected_Park_Next`. tests: entity_window_port
pub const COLLECTED_PARK_NEXT: Pin = Pin { plain: 0xFFFFAF6A, debug: 0xFFFFAFF8 };

/// `RingBuffer_Clear`. tests: entity_window_port
pub const RING_BUFFER_CLEAR: Pin = Pin { plain: 0x364C, debug: 0x3F52 };

/// `RingBuffer_Remove`. tests: entity_window_port
pub const RING_BUFFER_REMOVE: Pin = Pin { plain: 0x3618, debug: 0x3F1E };

/// `Section_GetSecPtrXY`. tests: entity_window_port
pub const SECTION_GET_SEC_PTR_XY: Pin = Pin { plain: 0x59B4, debug: 0x6934 };

/// `Section_FlatIDXY`. tests: entity_window_port
pub const SECTION_FLAT_IDXY: Pin = Pin { plain: 0x599A, debug: 0x691A };

/// `AllocDynamic`. tests: load_object_port, children_port
pub const ALLOC_DYNAMIC: Pin = Pin { plain: 0x2B72, debug: 0x2DD2 };

/// `AllocEffect`. tests: children_port
pub const ALLOC_EFFECT: Pin = Pin { plain: 0x2BD6, debug: 0x2E36 };

/// `Palette_Buffer`. tests: buffers_port
pub const PALETTE_BUFFER: Pin = Pin { plain: 0xFFFF821A, debug: 0xFFFF821A };

/// `Hscroll_Buffer`. tests: buffers_port
pub const HSCROLL_BUFFER: Pin = Pin { plain: 0xFFFF851E, debug: 0xFFFF851E };

/// `Static_Pal_Line0`. tests: buffers_port
pub const STATIC_PAL_LINE0: Pin = Pin { plain: 0xFFFF89A0, debug: 0xFFFF89A0 };

/// `Static_Pal_Line1`. tests: buffers_port
pub const STATIC_PAL_LINE1: Pin = Pin { plain: 0xFFFF89AE, debug: 0xFFFF89AE };

/// `Static_Pal_Line2`. tests: buffers_port
pub const STATIC_PAL_LINE2: Pin = Pin { plain: 0xFFFF89BC, debug: 0xFFFF89BC };

/// `Static_Pal_Line3`. tests: buffers_port
pub const STATIC_PAL_LINE3: Pin = Pin { plain: 0xFFFF89CA, debug: 0xFFFF89CA };

/// `Static_Sprite_DMA`. tests: buffers_port
pub const STATIC_SPRITE_DMA: Pin = Pin { plain: 0xFFFF89D8, debug: 0xFFFF89D8 };

/// `Static_Hscroll_Cell`. tests: buffers_port
pub const STATIC_HSCROLL_CELL: Pin = Pin { plain: 0xFFFF89E6, debug: 0xFFFF89E6 };

/// `Static_Hscroll_Line`. tests: buffers_port
pub const STATIC_HSCROLL_LINE: Pin = Pin { plain: 0xFFFF89F4, debug: 0xFFFF89F4 };

/// `Palette_Dirty`. tests: buffers_port
pub const PALETTE_DIRTY: Pin = Pin { plain: 0xFFFF829A, debug: 0xFFFF829A };

/// `Parallax_Active_Config`. tests: buffers_port
pub const PARALLAX_ACTIVE_CONFIG: Pin = Pin { plain: 0x6050, debug: 0x6FE0 };

/// `VBlank_Ready`. tests: vblank_port
pub const V_BLANK_READY: Pin = Pin { plain: 0xFFFF804C, debug: 0xFFFF804C };

/// `VBlank_Flag`. tests: vblank_port
pub const V_BLANK_FLAG: Pin = Pin { plain: 0xFFFF8000, debug: 0xFFFF8000 };

/// `VInt_Ptr`. tests: vblank_port
pub const V_INT_PTR: Pin = Pin { plain: 0xFFFF8048, debug: 0xFFFF8048 };

/// `Ctrl_1_Press`. tests: vblank_port
pub const CTRL_1_PRESS: Pin = Pin { plain: 0xFFFF802D, debug: 0xFFFF802D };

/// `Ctrl_1_Press_Accum`. tests: vblank_port
pub const CTRL_1_PRESS_ACCUM: Pin = Pin { plain: 0xFFFF8030, debug: 0xFFFF8030 };

/// `Ctrl_2_Press`. tests: vblank_port
pub const CTRL_2_PRESS: Pin = Pin { plain: 0xFFFF802F, debug: 0xFFFF802F };

/// `Ctrl_2_Press_Accum`. tests: vblank_port
pub const CTRL_2_PRESS_ACCUM: Pin = Pin { plain: 0xFFFF8031, debug: 0xFFFF8031 };

/// `Ctrl_1_Ext_Press`. tests: vblank_port, game_loop_port
pub const CTRL_1_EXT_PRESS: Pin = Pin { plain: 0xFFFF8033, debug: 0xFFFF8033 };

/// `Ctrl_1_Ext_Press_Accum`. tests: vblank_port, game_loop_port
pub const CTRL_1_EXT_PRESS_ACCUM: Pin = Pin { plain: 0xFFFF8036, debug: 0xFFFF8036 };

/// `Ctrl_2_Ext_Press`. tests: vblank_port, game_loop_port
pub const CTRL_2_EXT_PRESS: Pin = Pin { plain: 0xFFFF8035, debug: 0xFFFF8035 };

/// `Ctrl_2_Ext_Press_Accum`. tests: vblank_port, game_loop_port
pub const CTRL_2_EXT_PRESS_ACCUM: Pin = Pin { plain: 0xFFFF8037, debug: 0xFFFF8037 };

/// `Parallax_State`. tests: parallax_port
pub const PARALLAX_STATE: Pin = Pin { plain: 0xFFFF88A4, debug: 0xFFFF88A4 };

/// `Vscroll_Factor`. tests: parallax_port
pub const VSCROLL_FACTOR: Pin = Pin { plain: 0xFFFF88A0, debug: 0xFFFF88A0 };

/// `DMA_Budget_Default`. tests: vblank_port
pub const DMA_BUDGET_DEFAULT: Pin = Pin { plain: 0xFFFF8214, debug: 0xFFFF8214 };

/// `Lag_Frame_Count` — debug-shape consumer only (`debug_only`). tests: vblank_port
pub const LAG_FRAME_COUNT: u32 = 0xFFFF8A0E;

/// `DMA_Bytes_ThisFrame` — debug-shape consumer only (`debug_only`). tests: vblank_port
pub const DMA_BYTES_THIS_FRAME: u32 = 0xFFFF8A02;

/// `PageIn_InFlight`. tests: game_loop_port
pub const PAGE_IN_IN_FLIGHT: Pin = Pin { plain: 0xFFFFB450, debug: 0xFFFFB4DE };

/// `PageIn_Saved_PC`. tests: game_loop_port
pub const PAGE_IN_SAVED_PC: Pin = Pin { plain: 0xFFFFB44A, debug: 0xFFFFB4D8 };

/// `PageIn_BankRegs`. tests: game_loop_port
pub const PAGE_IN_BANK_REGS: Pin = Pin { plain: 0x67B2, debug: 0x78A8 };

/// `Dbg_PageIn_Preempts` — debug-shape consumer only (`debug_only`). tests: game_loop_port
pub const DBG_PAGE_IN_PREEMPTS: u32 = 0xFFFF8A28;

/// `ZX0R_Decompress.__end`. tests: game_loop_port
pub const ZX0R_DECOMPRESS_END: Pin = Pin { plain: 0x27E0, debug: 0x2A40 };

/// `PageIn_Staging_Busy`. tests: game_loop_port, load_art_port
pub const PAGE_IN_STAGING_BUSY: Pin = Pin { plain: 0xFFFFB452, debug: 0xFFFFB4E0 };

/// `PageIn_Flush`. tests: load_art_port
pub const PAGE_IN_FLUSH: Pin = Pin { plain: 0x687A, debug: 0x7978 };

/// `PageIn_Enqueue`. tests: load_art_port
pub const PAGE_IN_ENQUEUE: Pin = Pin { plain: 0x683C, debug: 0x793A };

/// `PageIn_Pool_Table`. tests: load_art_port
pub const PAGE_IN_POOL_TABLE: Pin = Pin { plain: 0xFFFFB47E, debug: 0xFFFFB50C };

/// `PageIn_Queue_Count`. tests: load_art_port
pub const PAGE_IN_QUEUE_COUNT: Pin = Pin { plain: 0xFFFFB454, debug: 0xFFFFB4E2 };

/// `PageIn_Suspended`. tests: load_art_port
pub const PAGE_IN_SUSPENDED: Pin = Pin { plain: 0xFFFFB451, debug: 0xFFFFB4DF };

/// `PageIn_Land_Pending`. tests: load_art_port
pub const PAGE_IN_LAND_PENDING: Pin = Pin { plain: 0xFFFFB453, debug: 0xFFFFB4E1 };

/// `PageCache_Init`. tests: load_art_port
pub const PAGE_CACHE_INIT: Pin = Pin { plain: 0x68CC, debug: 0x79CE };

/// `PageCache_AllocFrame`. tests: load_art_port
pub const PAGE_CACHE_ALLOC_FRAME: Pin = Pin { plain: 0x697A, debug: 0x7ADE };

/// `PageCache_Publish`. tests: load_art_port
pub const PAGE_CACHE_PUBLISH: Pin = Pin { plain: 0x6A36, debug: 0x7C9E };

/// `PageCache_PatchRun_Seq`. tests: tile_cache_port
pub const PAGE_CACHE_PATCH_RUN_SEQ: Pin = Pin { plain: 0x6AA4, debug: 0x7D72 };

/// `PageCache_PatchRun_Col`. tests: tile_cache_port
pub const PAGE_CACHE_PATCH_RUN_COL: Pin = Pin { plain: 0x6B6C, debug: 0x7F04 };

/// `PageCache_Audit`. tests: tile_cache_port
pub const PAGE_CACHE_AUDIT: Pin = Pin { plain: 0x6D34, debug: 0x819A };

/// `Cache_Art_Stall`. tests: tile_cache_port
pub const CACHE_ART_STALL: Pin = Pin { plain: 0xFFFFA864, debug: 0xFFFFA8F2 };

/// `Page_Audit_Ticks` — debug-shape consumer only (`debug_only`). tests: tile_cache_port
pub const PAGE_AUDIT_TICKS: u32 = 0xFFFF8A3C;

/// `Cache_Stall_Watchdog` — debug-shape consumer only (`debug_only`). tests: tile_cache_port
pub const CACHE_STALL_WATCHDOG: u32 = 0xFFFF8A3A;

/// `Flush_VDP_Shadow`. tests: vblank_port
pub const FLUSH_VDP_SHADOW: Pin = Pin { plain: 0x1C16, debug: 0x1CA4 };

/// `VInt_DrawLevel`. tests: vblank_port
pub const V_INT_DRAW_LEVEL: Pin = Pin { plain: 0x46A6, debug: 0x54C6 };

/// `Vscroll_Write`. tests: vblank_port
pub const VSCROLL_WRITE: Pin = Pin { plain: 0x6062, debug: 0x6FF2 };

/// `Read_Controllers`. tests: vblank_port
pub const READ_CONTROLLERS: Pin = Pin { plain: 0x2400, debug: 0x24A8 };

/// `Process_DMA_Critical`. tests: vblank_port
pub const PROCESS_DMA_CRITICAL: Pin = Pin { plain: 0x1E42, debug: 0x1EE4 };

/// `Process_DMA_Important`. tests: vblank_port
pub const PROCESS_DMA_IMPORTANT: Pin = Pin { plain: 0x1F10, debug: 0x1FB2 };

/// `Process_DMA_Deferrable`. tests: vblank_port
pub const PROCESS_DMA_DEFERRABLE: Pin = Pin { plain: 0x1F24, debug: 0x1FC6 };

/// `Enqueue_Dirty_Buffers`. tests: vblank_port
pub const ENQUEUE_DIRTY_BUFFERS: Pin = Pin { plain: 0x207A, debug: 0x2112 };

/// `BootData`. tests: boot_port
pub const BOOT_DATA: Pin = Pin { plain: 0x3A0, debug: 0x3B0 };

/// `VInt_Level`. tests: boot_port
pub const V_INT_LEVEL: Pin = Pin { plain: 0x2228, debug: 0x22CC };

/// `BuildStaticDMA`. tests: boot_port
pub const BUILD_STATIC_DMA: Pin = Pin { plain: 0x1FA2, debug: 0x203A };

/// `Sound_Init`. tests: boot_port
pub const SOUND_INIT: Pin = Pin { plain: 0x6EE4, debug: 0x9586 };

/// `Hardware_Region`. tests: boot_port
pub const HARDWARE_REGION: Pin = Pin { plain: 0xFFFF802A, debug: 0xFFFF802A };

/// `Region_Flags`. tests: boot_port
pub const REGION_FLAGS: Pin = Pin { plain: 0xFFFF802B, debug: 0xFFFF802B };

/// `Game_State_ID`. tests: boot_port
pub const GAME_STATE_ID: Pin = Pin { plain: 0xFFFF800C, debug: 0xFFFF800C };

/// `Game_State_Init`. tests: boot_port
pub const GAME_STATE_INIT: Pin = Pin { plain: 0xFFFF800D, debug: 0xFFFF800D };

/// `RAM_Start`. tests: boot_port
pub const RAM_START: Pin = Pin { plain: 0xFFFF8000, debug: 0xFFFF8000 };

/// `PState_Ground`. tests: test_p1_player_port
pub const P_STATE_GROUND: Pin = Pin { plain: 0x10500, debug: 0x10620 };

/// `PState_Roll`. tests: test_p1_player_port
pub const P_STATE_ROLL: Pin = Pin { plain: 0x10662, debug: 0x10782 };

/// `PState_Spindash`. tests: test_p1_player_port
pub const P_STATE_SPINDASH: Pin = Pin { plain: 0x10CE0, debug: 0x10E00 };

/// `PState_Air`. tests: test_p1_player_port
pub const P_STATE_AIR: Pin = Pin { plain: 0x10990, debug: 0x10AB0 };

/// `PState_Jump`. tests: test_p1_player_port
pub const P_STATE_JUMP: Pin = Pin { plain: 0x10998, debug: 0x10AB8 };

/// `PState_RollJump`. tests: test_p1_player_port
pub const P_STATE_ROLL_JUMP: Pin = Pin { plain: 0x10994, debug: 0x10AB4 };

/// `PState_AirBall`. tests: test_p1_player_port
pub const P_STATE_AIR_BALL: Pin = Pin { plain: 0x10990, debug: 0x10AB0 };

/// `PState_Fly`. tests: test_p1_player_port
pub const P_STATE_FLY: Pin = Pin { plain: 0x10D7C, debug: 0x10E9C };

/// `Player_SensorFloor`. tests: test_p1_player_port
pub const PLAYER_SENSOR_FLOOR: Pin = Pin { plain: 0x57DC, debug: 0x675C };

/// `Player_AtLedgeEdge`. tests: test_p1_player_port
pub const PLAYER_AT_LEDGE_EDGE: Pin = Pin { plain: 0x58F6, debug: 0x6876 };

/// `Player_SetState`. tests: test_p2_player_states_port
pub const PLAYER_SET_STATE: Pin = Pin { plain: 0x102C8, debug: 0x1038C };

/// `Player_SnapToSurface`. tests: test_p2_player_states_port
pub const PLAYER_SNAP_TO_SURFACE: Pin = Pin { plain: 0x103B0, debug: 0x10474 };

/// `Player_SensorCeiling`. tests: test_p2_player_states_port
pub const PLAYER_SENSOR_CEILING: Pin = Pin { plain: 0x57F2, debug: 0x6772 };

/// `Player_SensorWallDir`. tests: test_p2_player_states_port
pub const PLAYER_SENSOR_WALL_DIR: Pin = Pin { plain: 0x58AC, debug: 0x682C };

/// `Player_SensorWallAt`. tests: test_p2_player_states_port
pub const PLAYER_SENSOR_WALL_AT: Pin = Pin { plain: 0x58A4, debug: 0x6824 };

/// `Collision_GetType`. tests: test_p4_player_sensors_port
pub const COLLISION_GET_TYPE: Pin = Pin { plain: 0x5400, debug: 0x6380 };

/// `SolidityTable`. tests: test_p4_player_sensors_port
pub const SOLIDITY_TABLE: Pin = Pin { plain: 0x286C8, debug: 0x28E60 };

/// `AngleTable`. tests: test_p4_player_sensors_port
pub const ANGLE_TABLE: Pin = Pin { plain: 0x285C8, debug: 0x28D60 };

/// `HeightMaps`. tests: test_p4_player_sensors_port
pub const HEIGHT_MAPS: Pin = Pin { plain: 0x265C8, debug: 0x26D60 };

/// `HeightMapsRot`. tests: test_p4_player_sensors_port
pub const HEIGHT_MAPS_ROT: Pin = Pin { plain: 0x275C8, debug: 0x27D60 };

/// `Character_ID`. tests: test_p1_player_port
pub const CHARACTER_ID: Pin = Pin { plain: 0xFFFFB494, debug: 0xFFFFDD2A };

/// `Player_Chardef`. tests: test_p1_player_port
pub const PLAYER_CHARDEF: Pin = Pin { plain: 0xFFFFB496, debug: 0xFFFFDD2C };

/// `Ability_None`. tests: test_p1_player_port
pub const ABILITY_NONE: Pin = Pin { plain: 0x10F2C, debug: 0x11058 };

/// `CharacterDefs`. tests: test_p1_player_port
pub const CHARACTER_DEFS: Pin = Pin { plain: 0x10EE4, debug: 0x11010 };

/// `Player_InitAssets`. tests: test_p1_player_port
pub const PLAYER_INIT_ASSETS: Pin = Pin { plain: 0x10EF0, debug: 0x1101C };

/// `Player_LoadArt`. tests: test_p1_player_port
pub const PLAYER_LOAD_ART: Pin = Pin { plain: 0x10F08, debug: 0x11034 };

/// `Player_Ability`. tests: test_p2_player_states_port
pub const PLAYER_ABILITY: Pin = Pin { plain: 0x10F22, debug: 0x1104E };

/// `PhysTable_Sonic`. tests: test_p1_player_port
pub const PHYS_TABLE_SONIC: Pin = Pin { plain: 0x10EA4, debug: 0x10FD0 };

/// `Player_Blocks`. tests: test_p1_player_port
pub const PLAYER_BLOCKS: Pin = Pin { plain: 0xFFFFB49A, debug: 0xFFFFDD30 };

/// `Player_Ring_Index`. tests: test_p1_player_port
pub const PLAYER_RING_INDEX: Pin = Pin { plain: 0xFFFFB800, debug: 0xFFFFE100 };

/// `Player_Pos_Ring`. tests: test_p1_player_port
pub const PLAYER_POS_RING: Pin = Pin { plain: 0xFFFFB600, debug: 0xFFFFDF00 };

/// `Player_Stat_Ring`. tests: test_p1_player_port
pub const PLAYER_STAT_RING: Pin = Pin { plain: 0xFFFFB700, debug: 0xFFFFE000 };

/// `Player_Death_Pending`. tests: test_p1_player_port
pub const PLAYER_DEATH_PENDING: Pin = Pin { plain: 0xFFFFB4BE, debug: 0xFFFFDD54 };

/// `Player_Bound_Right`. tests: test_p1_player_port
pub const PLAYER_BOUND_RIGHT: Pin = Pin { plain: 0xFFFFB4C0, debug: 0xFFFFDD56 };

/// `Player_Bound_Bottom`. tests: test_p1_player_port
pub const PLAYER_BOUND_BOTTOM: Pin = Pin { plain: 0xFFFFB4C2, debug: 0xFFFFDD58 };

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
pub const FLUSH_VDP_SHADOW_OFF: usize = 0x16;

/// `HBlank_Uninstall` − `hblank` start (shape-invariant, asserted at generation). tests: hblank_port
pub const HBLANK_UNINSTALL_OFF: usize = 0x2C;
