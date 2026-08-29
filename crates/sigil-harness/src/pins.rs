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
//! [provenance] 97 regions, 412 symbols, 7 offsets

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
pub const ASSEMBLED_LEN: usize = 0xA5C90;
/// Assembled (pre-convsym) ROM length, `__DEBUG__` shape. tests: m1d_rom, m1d_debug_rom, mixed_dac_rom
pub const DEBUG_ASSEMBLED_LEN: usize = 0xA7F40;

// ── Regions (manifest order) ──

/// `Vectors` .. start + 0x100 plain / 0x100 debug (literal — no end symbol) — gate `SIGIL_EMP_VECTORS`. tests: vectors_port
pub const VECTORS: Region = Region { plain_base: 0x0, debug_base: 0x0, plain_len: 0x100, debug_len: 0x100 };

/// `GameHeader` .. `section:header`. tests: header_port
pub const HEADER: Region = Region { plain_base: 0x100, debug_base: 0x100, plain_len: 0x100, debug_len: 0x100 };

/// `HeightMaps` .. start + 0x1C480 plain / 0x1C480 debug (literal — no end symbol). tests: collision_data_port
pub const COLLISION_DATA: Region = Region { plain_base: 0x6DD10, debug_base: 0x6E5E0, plain_len: 0x1C480, debug_len: 0x1C480 };

/// `EntryPoint` .. `BootData` — gate `SIGIL_EMP_BOOT`. tests: boot_port
pub const BOOT: Region = Region { plain_base: 0x200, debug_base: 0x200, plain_len: 0x1A0, debug_len: 0x1A0 };

/// `BootData` .. `BootData_PostBlob`. tests: boot_data_port
pub const BOOT_HEAD: Region = Region { plain_base: 0x3A0, debug_base: 0x3A0, plain_len: 0x1850, debug_len: 0x18D0 };

/// `BootData_PostBlob` .. `section:boot_tail`. tests: boot_data_port
pub const BOOT_TAIL: Region = Region { plain_base: 0x1BF0, debug_base: 0x1C70, plain_len: 0xE, debug_len: 0xE };

/// `VDP_Shadow_Init` .. `Init_DMA_Queue` — gate `SIGIL_EMP_VDP_INIT`. tests: vdp_init_port
pub const VDP_INIT: Region = Region { plain_base: 0x1C00, debug_base: 0x1C7E, plain_len: 0x3A, debug_len: 0x92 };

/// `Init_DMA_Queue` .. `Init_SpriteTable` — gate `SIGIL_EMP_DMA_QUEUE`. tests: dma_queue_port
pub const DMA_QUEUE: Region = Region { plain_base: 0x1C3A, debug_base: 0x1D10, plain_len: 0x336, debug_len: 0x338 };

/// `Init_SpriteTable` .. `VBlank_Handler` — gate `SIGIL_EMP_BUFFERS`. tests: buffers_port
pub const BUFFERS: Region = Region { plain_base: 0x1F70, debug_base: 0x2048, plain_len: 0x2E0, debug_len: 0x2D8 };

/// `VBlank_Handler` .. `HBlank_Install` — gate `SIGIL_EMP_VBLANK`. tests: vblank_port
pub const VBLANK: Region = Region { plain_base: 0x2250, debug_base: 0x2320, plain_len: 0x1E0, debug_len: 0x1F0 };

/// `HBlank_Install` .. `section:hblank` — gate `SIGIL_EMP_HBLANK`. tests: hblank_port, m1c_vector_table
pub const HBLANK: Region = Region { plain_base: 0x2430, debug_base: 0x2510, plain_len: 0x30, debug_len: 0x30 };

/// `Read_Controllers` .. `GameLoop` — gate `SIGIL_EMP_CONTROLLERS`. tests: controllers_port
pub const CONTROLLERS: Region = Region { plain_base: 0x2460, debug_base: 0x2540, plain_len: 0x10E, debug_len: 0x110 };

/// `GameLoop` .. `Input_Tick` — gate `SIGIL_EMP_GAME_LOOP`. tests: game_loop_port, load_art_port
pub const GAME_LOOP: Region = Region { plain_base: 0x256E, debug_base: 0x2650, plain_len: 0x22, debug_len: 0x24 };

/// `Input_Tick` .. `S4LZ_DecompressDict`. tests: game_loop_port
pub const REPLAY: Region = Region { plain_base: 0x2590, debug_base: 0x2674, plain_len: 0x150, debug_len: 0x1FC };

/// `S4LZ_DecompressDict` .. `section:s4lz` — gate `SIGIL_EMP_S4LZ`. tests: s4lz_port
pub const S4LZ: Region = Region { plain_base: 0x26E0, debug_base: 0x2870, plain_len: 0xF8, debug_len: 0x200 };

/// `ZX0R_Decompress` .. `GetSineCosine`.
pub const ZX0_RESUME: Region = Region { plain_base: 0x27D8, debug_base: 0x2A70, plain_len: 0x78, debug_len: 0x80 };

/// `GetSineCosine` .. `Perform_DPLC` — gate `SIGIL_EMP_MATH`. tests: math_port
pub const MATH: Region = Region { plain_base: 0x2850, debug_base: 0x2AF0, plain_len: 0x3F8, debug_len: 0x3F8 };

/// `Perform_DPLC` .. `InitObjectRAM` — gate `SIGIL_EMP_DPLC`. tests: dplc_port
pub const DPLC: Region = Region { plain_base: 0x2C48, debug_base: 0x2EE8, plain_len: 0xA8, debug_len: 0xA8 };

/// `InitObjectRAM` .. `InitSpriteSystem` — gate `SIGIL_EMP_CORE`. tests: core_port
pub const CORE: Region = Region { plain_base: 0x2CF0, debug_base: 0x2F90, plain_len: 0x300, debug_len: 0x750 };

/// `InitSpriteSystem` .. `AnimateSprite` — gate `SIGIL_EMP_SPRITES`. tests: sprites_port
pub const SPRITES: Region = Region { plain_base: 0x2FF0, debug_base: 0x36E0, plain_len: 0x420, debug_len: 0x534 };

/// `AnimateSprite` .. `section:animate` — gate `SIGIL_EMP_ANIMATE`. tests: animate_port, test_objects_port
pub const ANIMATE: Region = Region { plain_base: 0x3410, debug_base: 0x3C14, plain_len: 0x194, debug_len: 0x2B8 };

/// `TouchResponse` .. `section:collision` — gate `SIGIL_EMP_COLLISION`. tests: collision_port
pub const COLLISION: Region = Region { plain_base: 0x35A4, debug_base: 0x3ECC, plain_len: 0x200, debug_len: 0x208 };

/// `RingBuffer_Add` .. `Collected_Init` — gate `SIGIL_EMP_RINGS`. tests: rings_port
pub const RINGS: Region = Region { plain_base: 0x37A4, debug_base: 0x40D4, plain_len: 0x1C0, debug_len: 0x224 };

/// `Collected_Init` .. `PopulateSpawnedPieceCount` — gate `SIGIL_EMP_ENTITY_WINDOW`. tests: entity_window_port
pub const ENTITY_WINDOW: Region = Region { plain_base: 0x3964, debug_base: 0x42F8, plain_len: 0x8FC, debug_len: 0xD68 };

/// `PopulateSpawnedPieceCount` .. `Load_Object` — gate `SIGIL_EMP_CHILDREN`. tests: children_port
pub const CHILDREN: Region = Region { plain_base: 0x4260, debug_base: 0x5060, plain_len: 0x2F0, debug_len: 0x3A0 };

/// `Load_Object` .. `section:load_object` — gate `SIGIL_EMP_LOAD_OBJECT`. tests: load_object_port, entity_window_port
pub const LOAD_OBJECT: Region = Region { plain_base: 0x4550, debug_base: 0x5400, plain_len: 0x88, debug_len: 0x88 };

/// `Plane_Buffer_Reset` .. `Tile_Cache_GetTile` — gate `SIGIL_EMP_PLANE_BUFFER`. tests: plane_buffer_port
pub const PLANE_BUFFER: Region = Region { plain_base: 0x45D8, debug_base: 0x5488, plain_len: 0x328, debug_len: 0x378 };

/// `Tile_Cache_GetTile` .. `Collision_GetType` — gate `SIGIL_EMP_TILE_CACHE`. tests: tile_cache_port
pub const TILE_CACHE: Region = Region { plain_base: 0x4900, debug_base: 0x5800, plain_len: 0xE90, debug_len: 0x1100 };

/// `Collision_GetType` .. `Collision_ProbeDown` — gate `SIGIL_EMP_COLLISION_LOOKUP`. tests: collision_lookup_port
pub const COLLISION_LOOKUP: Region = Region { plain_base: 0x5790, debug_base: 0x6900, plain_len: 0x70, debug_len: 0x70 };

/// `Section_Init` .. `Camera_Init` — gate `SIGIL_EMP_SECTION`. tests: section_port
pub const SECTION: Region = Region { plain_base: 0x5CF4, debug_base: 0x6E64, plain_len: 0x42C, debug_len: 0x48C };

/// `Camera_Init` .. `Parallax_Init` — gate `SIGIL_EMP_CAMERA`. tests: camera_port
pub const CAMERA: Region = Region { plain_base: 0x6120, debug_base: 0x72F0, plain_len: 0x1D0, debug_len: 0x1E0 };

/// `Parallax_Init` .. `Raster_Install` — gate `SIGIL_EMP_PARALLAX`. tests: parallax_port
pub const PARALLAX: Region = Region { plain_base: 0x62F0, debug_base: 0x74D0, plain_len: 0x904, debug_len: 0x998 };

/// `Raster_Install` .. `section:raster` — gate `SIGIL_EMP_RASTER`. tests: raster_port
pub const RASTER: Region = Region { plain_base: 0x6BF4, debug_base: 0x7E68, plain_len: 0x364, debug_len: 0x364 };

/// `Palette_LoadPal` .. `Effects_InstallPreset` — gate `SIGIL_EMP_PALETTE`. tests: palette_port
pub const PALETTE: Region = Region { plain_base: 0x6F58, debug_base: 0x81CC, plain_len: 0x4AE, debug_len: 0x4AE };

/// `Effects_InstallPreset` .. `Level_LoadArt`.
pub const PRESET: Region = Region { plain_base: 0x7406, debug_base: 0x867A, plain_len: 0x9A, debug_len: 0xA6 };

/// `Level_LoadArt` .. `PageIn_Process` — gate `SIGIL_EMP_LOAD_ART`. tests: load_art_port
pub const LOAD_ART: Region = Region { plain_base: 0x74A0, debug_base: 0x8720, plain_len: 0xB8, debug_len: 0xB8 };

/// `PageIn_Process` .. `PageCache_Init`.
pub const PAGE_IN: Region = Region { plain_base: 0x7558, debug_base: 0x87D8, plain_len: 0x2EC, debug_len: 0x45C };

/// `PageCache_Init` .. `BG_Init`.
pub const PAGE_CACHE: Region = Region { plain_base: 0x7844, debug_base: 0x8C34, plain_len: 0x4EC, debug_len: 0xE7C };

/// `BG_Init` .. `BgAnim_Init` — gate `SIGIL_EMP_BG`. tests: bg_port
pub const BG: Region = Region { plain_base: 0x7D30, debug_base: 0x9AB0, plain_len: 0xE0, debug_len: 0x140 };

/// `BgAnim_Init` .. start + 0x9E plain / 0x158 debug (literal — no end symbol) — gate `SIGIL_EMP_BG_ANIM`. tests: bg_anim_port
pub const BG_ANIM: Region = Region { plain_base: 0x7E10, debug_base: 0x9BF0, plain_len: 0x9E, debug_len: 0x158 };

/// `CompressionSelfTest` .. `Sound_PostByte` (debug-only region; plain empty at `Sound_PostByte`) — gate `SIGIL_EMP_COMPRESSION_SELFTEST`. tests: compression_selftest_port
pub const COMPRESSION_SELFTEST: Region = Region { plain_base: 0x7EAE, debug_base: 0x9D48, plain_len: 0x0, debug_len: 0xDE8 };

/// `Sound_PostByte` .. start + 0x2A8 plain / 0x452 debug (literal — no end symbol) — gate `SIGIL_EMP_SOUND_API`. tests: sound_api_port
pub const SOUND_API: Region = Region { plain_base: 0x7EAE, debug_base: 0xAB30, plain_len: 0x2A8, debug_len: 0x452 };

/// `TestSolid_Init` .. `ObjDef_PathSwap` plain / `TestParticle` debug — gate `SIGIL_EMP_TEST_OBJECTS`. tests: test_objects_port
pub const TEST_SOLID: Region = Region { plain_base: 0x12300, debug_base: 0x1280C, plain_len: 0x12, debug_len: 0x14 };

/// `TestParticle` .. `section:test_particle` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_OBJECTS`. tests: test_objects_port
pub const TEST_PARTICLE: Region = Region { plain_base: 0x12312, debug_base: 0x12820, plain_len: 0x0, debug_len: 0x58 };

/// `TestStatic_Main` .. `TestSolid_Init` plain / `TestAnimated` debug — gate `SIGIL_EMP_TEST_STATIC`. tests: test_g1_objects_port
pub const TEST_STATIC: Region = Region { plain_base: 0x122F0, debug_base: 0x124C0, plain_len: 0x10, debug_len: 0x10 };

/// `TestAnimated` .. `section:test_animated` (debug-only region; plain empty at `TestSolid_Init`) — gate `SIGIL_EMP_TEST_ANIMATED`. tests: test_g1_objects_port
pub const TEST_ANIMATED: Region = Region { plain_base: 0x12300, debug_base: 0x124D0, plain_len: 0x0, debug_len: 0x60 };

/// `TestEmitter` .. `section:test_emitter` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_EMITTER`. tests: test_g2_objects_port
pub const TEST_EMITTER: Region = Region { plain_base: 0x12312, debug_base: 0x12878, plain_len: 0x0, debug_len: 0x5E };

/// `TestStressEmitter` .. `TestChurnObj` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_STRESS_EMITTER`. tests: test_g2_objects_port
pub const TEST_STRESS_EMITTER: Region = Region { plain_base: 0x12312, debug_base: 0x12A10, plain_len: 0x0, debug_len: 0x60 };

/// `TestChurnObj` .. `section:test_churn` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_CHURN`. tests: test_g2_objects_port
pub const TEST_CHURN: Region = Region { plain_base: 0x12312, debug_base: 0x12A70, plain_len: 0x0, debug_len: 0x7C };

/// `TestChildPart` .. `TestStressEmitter` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_PARENT`. tests: test_g3_objects_port
pub const TEST_PARENT: Region = Region { plain_base: 0x12312, debug_base: 0x128D6, plain_len: 0x0, debug_len: 0x13A };

/// `TestPlayer` .. `section:test_player` (debug-only region; plain empty at `TestSolid_Init`) — gate `SIGIL_EMP_TEST_PLAYER`. tests: test_g4_final_objects_port
pub const TEST_PLAYER: Region = Region { plain_base: 0x12300, debug_base: 0x12530, plain_len: 0x0, debug_len: 0x294 };

/// `TestEnemy_Init` .. `section:test_enemy` (debug-only region; plain empty at `TestSolid_Init`) — gate `SIGIL_EMP_TEST_ENEMY`. tests: test_g4_final_objects_port
pub const TEST_ENEMY: Region = Region { plain_base: 0x12300, debug_base: 0x127C4, plain_len: 0x0, debug_len: 0x48 };

/// `ObjDef_PathSwap` .. `DeformTable_Zero` — gate `SIGIL_EMP_PATH_SWAP`. tests: test_g4_final_objects_port
pub const PATH_SWAP: Region = Region { plain_base: 0x12312, debug_base: 0x12AEC, plain_len: 0x92, debug_len: 0xFC };

/// `OJZ_TestRaster` .. `ObjDef_Static`.
pub const OJZ_EFFECTS: Region = Region { plain_base: 0x1328A, debug_base: 0x13ACE, plain_len: 0x536, debug_len: 0x5A2 };

/// `DeformTable_Zero` .. `section:scene_registry` — gate `SIGIL_EMP_SCENE_REGISTRY`. tests: scene_registry_port
pub const SCENE_REGISTRY: Region = Region { plain_base: 0x123A4, debug_base: 0x12BE8, plain_len: 0xD94, debug_len: 0xD94 };

/// `Map_TestObj` .. `section:test_mappings` — gate `SIGIL_EMP_TEST_MAPPINGS`. tests: test_mappings_port
pub const TEST_MAPPINGS: Region = Region { plain_base: 0x29680, debug_base: 0x29F3E, plain_len: 0x30, debug_len: 0x30 };

/// `Map_DustSpindash` .. `section:dust_data` — gate `SIGIL_EMP_DUST_DATA`.
pub const DUST_DATA: Region = Region { plain_base: 0x296B0, debug_base: 0x29F6E, plain_len: 0xBDA, debug_len: 0xBDA };

/// `Ani_Sonic` .. `section:sonic_anims` — gate `SIGIL_EMP_SONIC_ANIMS`. tests: sonic_anims_port
pub const SONIC_ANIMS: Region = Region { plain_base: 0x2A290, debug_base: 0x2AB48, plain_len: 0x10A, debug_len: 0x10A };

/// `Ani_Tails` .. `section:tails_anims` — gate `SIGIL_EMP_TAILS_ANIMS`. tests: sonic_anims_port
pub const TAILS_ANIMS: Region = Region { plain_base: 0x2A39A, debug_base: 0x2AC60, plain_len: 0x1BC, debug_len: 0x1BC };

/// `Ani_Knuckles` .. `section:knuckles_anims` — gate `SIGIL_EMP_KNUCKLES_ANIMS`. tests: sonic_anims_port
pub const KNUCKLES_ANIMS: Region = Region { plain_base: 0x2A556, debug_base: 0x2AE1C, plain_len: 0x16C, debug_len: 0x16C };

/// `Map_Tails` .. `section:tails_data` — gate `SIGIL_EMP_TAILS_DATA`. tests: collision_data_port
pub const TAILS_DATA: Region = Region { plain_base: 0x2A6E0, debug_base: 0x2AFB0, plain_len: 0x20F5E, debug_len: 0x20F5E };

/// `Map_Knuckles` .. `section:knuckles_data` — gate `SIGIL_EMP_KNUCKLES_DATA`. tests: collision_data_port
pub const KNUCKLES_DATA: Region = Region { plain_base: 0x4B63E, debug_base: 0x4BF0E, plain_len: 0x226C8, debug_len: 0x226C8 };

/// `Ani_Particle` .. `section:particle_anims` (debug-only region; plain empty at `Ani_DustSpindash`) — gate `SIGIL_EMP_PARTICLE_ANIMS`. tests: particle_anims_port, test_objects_port
pub const PARTICLE_ANIMS: Region = Region { plain_base: 0x2A6C2, debug_base: 0x2AF88, plain_len: 0x0, debug_len: 0x8 };

/// `Ani_DustSpindash` .. `section:dust_anims` — gate `SIGIL_EMP_DUST_ANIMS`.
pub const DUST_ANIMS: Region = Region { plain_base: 0x2A6C2, debug_base: 0x2AF90, plain_len: 0x14, debug_len: 0x14 };

/// `OJZ_Sec0_TypeTable` .. `section:entity_data`. tests: ojz_run_a_port
pub const ENTITY_DATA: Region = Region { plain_base: 0x137F8, debug_base: 0x140B0, plain_len: 0x170, debug_len: 0x170 };

/// `OJZ_Act_Pool_Page0` .. `OJZ_Act1_Descriptor`. tests: ojz_run_a_port
pub const OJZ_ACT_POOL: Region = Region { plain_base: 0x13968, debug_base: 0x14220, plain_len: 0x2F0C, debug_len: 0x2F10 };

/// `OJZ_Act1_Descriptor` .. `section:act_descriptor` — gate `SIGIL_EMP_ACT_DESCRIPTOR`. tests: act_descriptor_port
pub const ACT_DESCRIPTOR: Region = Region { plain_base: 0x16874, debug_base: 0x17130, plain_len: 0x27A, debug_len: 0x27A };

/// `OJZ_Sec0_Blocks` .. `OJZ_Sec0_LocalMap`. tests: ojz_run_b_port
pub const SEC_BLOCK_BLOBS: Region = Region { plain_base: 0x16AF0, debug_base: 0x173B0, plain_len: 0xB610, debug_len: 0xB60C };

/// `OJZ_Sec0_LocalMap` .. `OJZ_Palette`. tests: ojz_run_b_port
pub const SEC_LOCAL_MAPS: Region = Region { plain_base: 0x22100, debug_base: 0x229BC, plain_len: 0xCD0, debug_len: 0xCC4 };

/// `OJZ_Palette` .. `BgAnim_Table`. tests: ojz_run_b_port
pub const OJZ_ACT_ASSETS: Region = Region { plain_base: 0x22DD0, debug_base: 0x23680, plain_len: 0x4882, debug_len: 0x4890 };

/// `BgAnim_Table` .. `section:ojz_bg_anim`. tests: ojz_run_b_port
pub const OJZ_BG_ANIM: Region = Region { plain_base: 0x27652, debug_base: 0x27F10, plain_len: 0x202E, debug_len: 0x202E };

/// `ObjDef_Static` .. `OJZ_Sec0_TypeTable` — gate `SIGIL_EMP_OBJDEFS`. tests: objdef_port
pub const OBJDEFS: Region = Region { plain_base: 0x137C0, debug_base: 0x14070, plain_len: 0x38, debug_len: 0x40 };

/// `GameState_ObjectTest_Init` .. `section:object_test_state` (debug-only region; plain empty at `GameState_OJZScroll_Init`) — gate `SIGIL_EMP_OBJECT_TEST_STATE`. tests: test_t1_harness_states_port
pub const OBJECT_TEST_STATE: Region = Region { plain_base: 0xA4410, debug_base: 0xA5E60, plain_len: 0x0, debug_len: 0x384 };

/// `GameState_OJZScroll_Init` .. `Replay_OJZ_Fixture` — gate `SIGIL_EMP_OJZ_SCROLL_TEST`. tests: test_t1_harness_states_port
pub const OJZ_SCROLL_TEST: Region = Region { plain_base: 0xA4410, debug_base: 0xA61E4, plain_len: 0x570, debug_len: 0xA4C };

/// `Replay_OJZ_Fixture` .. `section:replay_fixture`.
pub const REPLAY_FIXTURE: Region = Region { plain_base: 0xA4980, debug_base: 0xA6C30, plain_len: 0x260, debug_len: 0x260 };

/// `BusError` .. `section:error_handler` — gate `SIGIL_EMP_ERROR_HANDLER`. tests: error_handler_port
pub const ERROR_HANDLER: Region = Region { plain_base: 0xA4BE0, debug_base: 0xA6E90, plain_len: 0x10B0, debug_len: 0x10B0 };

/// `Dac_Temp_Blip` .. start + 0xF8BC plain / 0xF8BC debug (literal — no end symbol) — gate `SIGIL_EMP_DAC`. tests: dac_bank_port
pub const DAC_BANKS: Region = Region { plain_base: 0x90000, debug_base: 0x90000, plain_len: 0xF8BC, debug_len: 0xF8BC };

/// `Song_MovingTrucks` .. start + 0x34E8 plain / 0x4F38 debug (literal — no end symbol) — gate `SIGIL_EMP_MT`. tests: mt_bank_port
pub const MT_BANK_BLOB: Region = Region { plain_base: 0xA0630, debug_base: 0xA0630, plain_len: 0x34E8, debug_len: 0x4F38 };

/// `Sfx_33` .. `section:sfx_bank_blob` — gate `SIGIL_EMP_SFX`. tests: sfx_bank_port
pub const SFX_BANK_BLOB: Region = Region { plain_base: 0xA3B20, debug_base: 0xA5570, plain_len: 0x8EC, debug_len: 0x8EC };

/// `SoundTablesZ80_Head` .. start + 0x630 plain / 0x630 debug (literal — no end symbol) — gate `SIGIL_EMP_SOUNDBANKHEAD`. tests: soundbankhead_port
pub const SOUNDBANKHEAD: Region = Region { plain_base: 0xA0000, debug_base: 0xA0000, plain_len: 0x630, debug_len: 0x630 };

/// `EndOfRom` .. start + 0x0 plain / 0x0 debug (literal — no end symbol) — gate `SIGIL_EMP_EPILOGUE`. tests: m1d_rom, m1d_debug_rom
pub const EPILOGUE: Region = Region { plain_base: 0xA5C90, debug_base: 0xA7F40, plain_len: 0x0, debug_len: 0x0 };

/// `ObjCodeBase` .. start + 0x2 plain / 0x2 debug (literal — no end symbol) — gate `SIGIL_EMP_OBJCODEBASE`. tests: m1d_rom, m1d_debug_rom
pub const OBJCODEBASE: Region = Region { plain_base: 0x10000, debug_base: 0x10000, plain_len: 0x2, debug_len: 0x2 };

/// `Player_Init` .. `PState_Ground` — gate `SIGIL_EMP_PLAYER_COMMON`. tests: test_p1_player_port
pub const PLAYER_COMMON: Region = Region { plain_base: 0x10002, debug_base: 0x10002, plain_len: 0x68E, debug_len: 0x79E };

/// `CharDef_Sonic` .. `CharDef_Tails` — gate `SIGIL_EMP_SONIC`. tests: test_p1_player_port
pub const SONIC: Region = Region { plain_base: 0x11E70, debug_base: 0x11F80, plain_len: 0x40, debug_len: 0x40 };

/// `CharDef_Tails` .. `section:tails` — gate `SIGIL_EMP_TAILS`. tests: test_p1_player_port
pub const TAILS: Region = Region { plain_base: 0x11EB0, debug_base: 0x11FC0, plain_len: 0x36, debug_len: 0x36 };

/// `CharDef_Knuckles` .. `CharacterDefs` — gate `SIGIL_EMP_KNUCKLES`. tests: test_p1_player_port
pub const KNUCKLES: Region = Region { plain_base: 0x11EE6, debug_base: 0x11FF6, plain_len: 0x3A, debug_len: 0x3A };

/// `CharacterDefs` .. `section:characters` — gate `SIGIL_EMP_CHARACTERS`. tests: test_p1_player_port
pub const CHARACTERS: Region = Region { plain_base: 0x11F20, debug_base: 0x12030, plain_len: 0x4A, debug_len: 0xB0 };

/// `TailsAppendage_Refresh` .. `section:tails_appendage` — gate `SIGIL_EMP_TAILS_APPENDAGE`. tests: test_p1_player_port
pub const TAILS_APPENDAGE: Region = Region { plain_base: 0x11F6A, debug_base: 0x120E0, plain_len: 0x11C, debug_len: 0x174 };

/// `DustPuff_Spawn` .. `section:dust_puff` — gate `SIGIL_EMP_DUST_PUFF`.
pub const DUST_PUFF: Region = Region { plain_base: 0x12086, debug_base: 0x12254, plain_len: 0x46, debug_len: 0x46 };

/// `Dust_Tick` .. `TestStatic_Main` — gate `SIGIL_EMP_DUST_SPINDASH`.
pub const DUST_SPINDASH: Region = Region { plain_base: 0x120CC, debug_base: 0x1229A, plain_len: 0x224, debug_len: 0x226 };

/// `PState_Ground` .. `PState_Air` — gate `SIGIL_EMP_PLAYER_GROUND`. tests: test_p2_player_states_port
pub const PLAYER_GROUND: Region = Region { plain_base: 0x10690, debug_base: 0x107A0, plain_len: 0x490, debug_len: 0x490 };

/// `PState_Air` .. `PState_Spindash` — gate `SIGIL_EMP_PLAYER_AIR`. tests: test_p2_player_states_port
pub const PLAYER_AIR: Region = Region { plain_base: 0x10B20, debug_base: 0x10C30, plain_len: 0x350, debug_len: 0x350 };

/// `PState_Spindash` .. `section:player_spindash` — gate `SIGIL_EMP_PLAYER_SPINDASH`. tests: test_p2_player_states_port
pub const PLAYER_SPINDASH: Region = Region { plain_base: 0x10E70, debug_base: 0x10F80, plain_len: 0x9C, debug_len: 0x9C };

/// `PState_Fly` .. `PState_Glide` — gate `SIGIL_EMP_PLAYER_FLY`. tests: test_p2_player_states_port
pub const PLAYER_FLY: Region = Region { plain_base: 0x10F0C, debug_base: 0x1101C, plain_len: 0x132, debug_len: 0x134 };

/// `PState_Glide` .. `Climb_WallDist` — gate `SIGIL_EMP_PLAYER_GLIDE`. tests: test_p2_player_states_port
pub const PLAYER_GLIDE: Region = Region { plain_base: 0x1103E, debug_base: 0x11150, plain_len: 0x2D2, debug_len: 0x2D6 };

/// `Climb_WallDist` .. `CharDef_Sonic` — gate `SIGIL_EMP_PLAYER_CLIMB`. tests: test_p2_player_states_port
pub const PLAYER_CLIMB: Region = Region { plain_base: 0x11310, debug_base: 0x11426, plain_len: 0xB60, debug_len: 0xB5A };

/// `Collision_ProbeDown` .. `section:player_sensors` — gate `SIGIL_EMP_PLAYER_SENSORS`. tests: test_p4_player_sensors_port
pub const PLAYER_SENSORS: Region = Region { plain_base: 0x5800, debug_base: 0x6970, plain_len: 0x4F4, debug_len: 0x4F4 };

// ── Symbols (manifest order) ──

/// `OJZ_Preset_Sec0`. tests: act_descriptor_port
pub const OJZ_PRESET_SEC0: Pin = Pin { plain: 0x1362A, debug: 0x13EDC };

/// `OJZ_Preset_Sec1`. tests: act_descriptor_port
pub const OJZ_PRESET_SEC1: Pin = Pin { plain: 0x13650, debug: 0x13F02 };

/// `OJZ_Preset_Sec2`. tests: act_descriptor_port
pub const OJZ_PRESET_SEC2: Pin = Pin { plain: 0x13676, debug: 0x13F28 };

/// `OJZ_Preset_Sec3`. tests: act_descriptor_port
pub const OJZ_PRESET_SEC3: Pin = Pin { plain: 0x1369C, debug: 0x13F4E };

/// `OJZ_Preset_Plain`. tests: act_descriptor_port
pub const OJZ_PRESET_PLAIN: Pin = Pin { plain: 0x136C2, debug: 0x13F74 };

/// `OJZ_Preset_Depth`. tests: act_descriptor_port
pub const OJZ_PRESET_DEPTH: Pin = Pin { plain: 0x136E8, debug: 0x13F9A };

/// `EditorSceneBinding_OJZ_Act1_Sec4`. tests: act_descriptor_port
pub const EDITOR_SCENE_BINDING_OJZ_ACT1_SEC4: Pin = Pin { plain: 0x131BA, debug: 0x139FE };

/// `EditorRaster_OJZ_Act1_authored_probe`. tests: act_descriptor_port
pub const EDITOR_RASTER_OJZ_ACT1_AUTHORED_PROBE: Pin = Pin { plain: 0x1323C, debug: 0x13A80 };

/// `Effects_InstallPreset`. tests: parallax_port
pub const EFFECTS_INSTALL_PRESET: Pin = Pin { plain: 0x7406, debug: 0x867A };

/// `Raster_GetChannelBand`. tests: parallax_port
pub const RASTER_GET_CHANNEL_BAND: Pin = Pin { plain: 0x6EFC, debug: 0x8170 };

/// `TestStatic_Main`. tests: objdef_port
pub const TEST_STATIC_MAIN: Pin = Pin { plain: 0x122F0, debug: 0x124C0 };

/// `TestSolid_Init`. tests: objdef_port
pub const TEST_SOLID_INIT: Pin = Pin { plain: 0x12300, debug: 0x1280C };

/// `TestEnemy_Init` — debug-shape consumer only (`debug_only`). tests: objdef_port
pub const TEST_ENEMY_INIT: u32 = 0x127C4;

/// `TestParent` — debug-shape consumer only (`debug_only`). tests: objdef_port
pub const TEST_PARENT_LABEL: u32 = 0x12960;

/// `Map_TestObj`. tests: objdef_port
pub const MAP_TEST_OBJ: Pin = Pin { plain: 0x29680, debug: 0x29F3E };

/// `Map_Sonic`. tests: test_g1_objects_port
pub const MAP_SONIC: Pin = Pin { plain: 0x6FF10, debug: 0x707E0 };

/// `DPLC_Sonic`. tests: test_g1_objects_port
pub const DPLC_SONIC: Pin = Pin { plain: 0x71B90, debug: 0x72460 };

/// `Art_Sonic`. tests: test_g1_objects_port
pub const ART_SONIC: Pin = Pin { plain: 0x724D0, debug: 0x72DA0 };

/// `CreateEffect_Normal`. tests: test_g2_objects_port
pub const CREATE_EFFECT_NORMAL: Pin = Pin { plain: 0x44B6, debug: 0x5366 };

/// `CreateChild_Normal`. tests: test_g3_objects_port
pub const CREATE_CHILD_NORMAL: Pin = Pin { plain: 0x428C, debug: 0x508C };

/// `DeleteChildren`. tests: test_g3_objects_port
pub const DELETE_CHILDREN: Pin = Pin { plain: 0x4498, debug: 0x5348 };

/// `GetSineCosine`. tests: test_g3_objects_port
pub const GET_SINE_COSINE: Pin = Pin { plain: 0x2850, debug: 0x2AF0 };

/// `EntryPoint`. tests: m1c_vector_table
pub const ENTRY_POINT: Pin = Pin { plain: 0x200, debug: 0x200 };

/// `BusError` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const BUS_ERROR: u32 = 0xA6E90;

/// `AddressError` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ADDRESS_ERROR: u32 = 0xA6EA8;

/// `IllegalInstr` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ILLEGAL_INSTR: u32 = 0xA6EC4;

/// `ZeroDivide` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ZERO_DIVIDE: u32 = 0xA6EE6;

/// `ChkInstr` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const CHK_INSTR: u32 = 0xA6F00;

/// `TrapvInstr` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const TRAPV_INSTR: u32 = 0xA6F1E;

/// `PrivilegeViol` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const PRIVILEGE_VIOL: u32 = 0xA6F3E;

/// `Trace` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const TRACE: u32 = 0xA6F60;

/// `Line1010Emu` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const LINE1010_EMU: u32 = 0xA6F74;

/// `Line1111Emu` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const LINE1111_EMU: u32 = 0xA6F94;

/// `ErrorExcept` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ERROR_EXCEPT: u32 = 0xA6FB4;

/// `ErrorTrap` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ERROR_TRAP: u32 = 0xA6FD2;

/// `VBlank_Handler`. tests: m1c_vector_table
pub const V_BLANK_HANDLER: Pin = Pin { plain: 0x2250, debug: 0x2320 };

/// `HBlank_Vector_Slot`. tests: hblank_port, m1c_vector_table
pub const H_BLANK_VECTOR_SLOT: Pin = Pin { plain: 0xFFFFB4EA, debug: 0xFFFFB578 };

/// `VDP_Shadow_Table`. tests: vdp_init_port
pub const VDP_SHADOW_TABLE: Pin = Pin { plain: 0xFFFF800E, debug: 0xFFFF800E };

/// `BootData_VDPRegs`. tests: vdp_init_port
pub const BOOT_DATA_VDP_REGS: Pin = Pin { plain: 0x3BA, debug: 0x3BA };

/// `Ctrl_1_Held`. tests: controllers_port
pub const CTRL_1_HELD: Pin = Pin { plain: 0xFFFF8028, debug: 0xFFFF8028 };

/// `Ctrl_1_Held_Raw`. tests: controllers_port
pub const CTRL_1_HELD_RAW: Pin = Pin { plain: 0xFFFFB8D8, debug: 0xFFFFB966 };

/// `Ctrl_2_Held`. tests: vblank_port
pub const CTRL_2_HELD: Pin = Pin { plain: 0xFFFF802A, debug: 0xFFFF802A };

/// `Ctrl_1_Ext_Held`. tests: vblank_port
pub const CTRL_1_EXT_HELD: Pin = Pin { plain: 0xFFFF802E, debug: 0xFFFF802E };

/// `Ctrl_2_Ext_Held`. tests: vblank_port
pub const CTRL_2_EXT_HELD: Pin = Pin { plain: 0xFFFF8030, debug: 0xFFFF8030 };

/// `Ctrl_2_Held_Raw`. tests: vblank_port
pub const CTRL_2_HELD_RAW: Pin = Pin { plain: 0xFFFFB8D9, debug: 0xFFFFB967 };

/// `Ctrl_1_Ext_Held_Raw`. tests: vblank_port
pub const CTRL_1_EXT_HELD_RAW: Pin = Pin { plain: 0xFFFFB8DA, debug: 0xFFFFB968 };

/// `Ctrl_2_Ext_Held_Raw`. tests: vblank_port
pub const CTRL_2_EXT_HELD_RAW: Pin = Pin { plain: 0xFFFFB8DB, debug: 0xFFFFB969 };

/// `VSync_Wait`. tests: game_loop_port, load_art_port
pub const V_SYNC_WAIT: Pin = Pin { plain: 0x2406, debug: 0x24DE };

/// `Sound_DrainSfxRing`. tests: game_loop_port, load_art_port
pub const SOUND_DRAIN_SFX_RING: Pin = Pin { plain: 0x801A, debug: 0xAE46 };

/// `Game_State`. tests: game_loop_port, load_art_port
pub const GAME_STATE: Pin = Pin { plain: 0xFFFF8008, debug: 0xFFFF8008 };

/// `Input_Tick`. tests: game_loop_port, game_debug_port
pub const INPUT_TICK: Pin = Pin { plain: 0x2590, debug: 0x2674 };

/// `Cache_Left_Col`. tests: collision_lookup_port, section_port
pub const CACHE_LEFT_COL: Pin = Pin { plain: 0xFFFFAC98, debug: 0xFFFFAD26 };

/// `Draw_TileColumn`. tests: section_port
pub const DRAW_TILE_COLUMN: Pin = Pin { plain: 0x45E0, debug: 0x5490 };

/// `Draw_TileRow_FromCache`. tests: section_port
pub const DRAW_TILE_ROW_FROM_CACHE: Pin = Pin { plain: 0x4734, debug: 0x55E4 };

/// `EntityWindow_Init`. tests: section_port
pub const ENTITY_WINDOW_INIT: Pin = Pin { plain: 0x3D22, debug: 0x4A34 };

/// `Section_Plane_Dirty`. tests: section_port
pub const SECTION_PLANE_DIRTY: Pin = Pin { plain: 0xFFFFAD0C, debug: 0xFFFFAD9A };

/// `Section_Right_Col_Written`. tests: section_port
pub const SECTION_RIGHT_COL_WRITTEN: Pin = Pin { plain: 0xFFFFAD0E, debug: 0xFFFFAD9C };

/// `Section_Left_Col_Written`. tests: section_port
pub const SECTION_LEFT_COL_WRITTEN: Pin = Pin { plain: 0xFFFFAD10, debug: 0xFFFFAD9E };

/// `Section_Top_Row_Written`. tests: section_port
pub const SECTION_TOP_ROW_WRITTEN: Pin = Pin { plain: 0xFFFFAD08, debug: 0xFFFFAD96 };

/// `Section_Bottom_Row_Written`. tests: section_port
pub const SECTION_BOTTOM_ROW_WRITTEN: Pin = Pin { plain: 0xFFFFAD0A, debug: 0xFFFFAD98 };

/// `Cache_Head_Col`. tests: section_port
pub const CACHE_HEAD_COL: Pin = Pin { plain: 0xFFFFAC9A, debug: 0xFFFFAD28 };

/// `Cache_Top_Row`. tests: section_port
pub const CACHE_TOP_ROW: Pin = Pin { plain: 0xFFFFAC9C, debug: 0xFFFFAD2A };

/// `Cache_Bottom_Row`. tests: section_port
pub const CACHE_BOTTOM_ROW: Pin = Pin { plain: 0xFFFFAC9E, debug: 0xFFFFAD2C };

/// `Cache_Origin_Col`. tests: section_port
pub const CACHE_ORIGIN_COL: Pin = Pin { plain: 0xFFFFACA0, debug: 0xFFFFAD2E };

/// `Cache_Origin_Row`. tests: section_port
pub const CACHE_ORIGIN_ROW: Pin = Pin { plain: 0xFFFFACA2, debug: 0xFFFFAD30 };

/// `Plane_Buffer_Ptr`. tests: section_port
pub const PLANE_BUFFER_PTR: Pin = Pin { plain: 0xFFFFAB84, debug: 0xFFFFAC12 };

/// `Plane_Buffer`. tests: plane_buffer_port
pub const PLANE_BUFFER_BASE: Pin = Pin { plain: 0xFFFFA584, debug: 0xFFFFA612 };

/// `Tile_Cache_Nametable`. tests: section_port
pub const TILE_CACHE_NAMETABLE: Pin = Pin { plain: 0xFFFF0000, debug: 0xFFFF0000 };

/// `Tile_Cache_Collision`. tests: tile_cache_port, collision_lookup_port
pub const TILE_CACHE_COLLISION: Pin = Pin { plain: 0xFFFF2580, debug: 0xFFFF2580 };

/// `Frame_Counter`. tests: tile_cache_port
pub const FRAME_COUNTER: Pin = Pin { plain: 0xFFFF8002, debug: 0xFFFF8002 };

/// `Logic_Tick`. tests: game_loop_port, bg_anim_port, tile_cache_port
pub const LOGIC_TICK: Pin = Pin { plain: 0xFFFF8004, debug: 0xFFFF8004 };

/// `Block_Stage_Keys`. tests: tile_cache_port
pub const BLOCK_STAGE_KEYS: Pin = Pin { plain: 0xFFFFACC6, debug: 0xFFFFAD54 };

/// `Block_Stage_Next`. tests: tile_cache_port
pub const BLOCK_STAGE_NEXT: Pin = Pin { plain: 0xFFFFAD06, debug: 0xFFFFAD94 };

/// `Block_Stage_Bucket`. tests: tile_cache_port
pub const BLOCK_STAGE_BUCKET: Pin = Pin { plain: 0xFFFF6842, debug: 0xFFFF6842 };

/// `Block_Stage_Chain`. tests: tile_cache_port
pub const BLOCK_STAGE_CHAIN: Pin = Pin { plain: 0xFFFF6942, debug: 0xFFFF6942 };

/// `Block_Stage_Buffers`. tests: tile_cache_port
pub const BLOCK_STAGE_BUFFERS: Pin = Pin { plain: 0xFFFF3842, debug: 0xFFFF3842 };

/// `Block_Stage_Ptrs`. tests: tile_cache_port
pub const BLOCK_STAGE_PTRS: Pin = Pin { plain: 0xFFFFB4F0, debug: 0xFFFFB57E };

/// `Block_Stage_ZeroPage`. tests: tile_cache_port
pub const BLOCK_STAGE_ZERO_PAGE: Pin = Pin { plain: 0xFFFFB574, debug: 0xFFFFB602 };

/// `Cache_Fill_Last_Frame`. tests: tile_cache_port
pub const CACHE_FILL_LAST_FRAME: Pin = Pin { plain: 0xFFFFACA4, debug: 0xFFFFAD32 };

/// `Cache_Fill_Budget`. tests: tile_cache_port
pub const CACHE_FILL_BUDGET: Pin = Pin { plain: 0xFFFFACAE, debug: 0xFFFFAD3C };

/// `Cache_Fill_Resume_Col`. tests: tile_cache_port
pub const CACHE_FILL_RESUME_COL: Pin = Pin { plain: 0xFFFFACA6, debug: 0xFFFFAD34 };

/// `Cache_Fill_Resume_Row`. tests: tile_cache_port
pub const CACHE_FILL_RESUME_ROW: Pin = Pin { plain: 0xFFFFACA8, debug: 0xFFFFAD36 };

/// `Cache_Fill_RowResume_Row`. tests: tile_cache_port
pub const CACHE_FILL_ROW_RESUME_ROW: Pin = Pin { plain: 0xFFFFACB0, debug: 0xFFFFAD3E };

/// `Cache_Fill_RowResume_Col`. tests: tile_cache_port
pub const CACHE_FILL_ROW_RESUME_COL: Pin = Pin { plain: 0xFFFFACB2, debug: 0xFFFFAD40 };

/// `Cache_Fill_Rows_Left`. tests: tile_cache_port
pub const CACHE_FILL_ROWS_LEFT: Pin = Pin { plain: 0xFFFFACB4, debug: 0xFFFFAD42 };

/// `Cache_Prev_Cam_Row`. tests: tile_cache_port
pub const CACHE_PREV_CAM_ROW: Pin = Pin { plain: 0xFFFFACB6, debug: 0xFFFFAD44 };

/// `Cache_Prev_Cam_X`. tests: tile_cache_port
pub const CACHE_PREV_CAM_X: Pin = Pin { plain: 0xFFFFACB8, debug: 0xFFFFAD46 };

/// `Cache_H_Pfx_Dir`. tests: tile_cache_port
pub const CACHE_H_PFX_DIR: Pin = Pin { plain: 0xFFFFACBA, debug: 0xFFFFAD48 };

/// `Cache_H_Pfx_Accum`. tests: tile_cache_port
pub const CACHE_H_PFX_ACCUM: Pin = Pin { plain: 0xFFFFACBC, debug: 0xFFFFAD4A };

/// `Cache_Pfx_Row_Target`. tests: tile_cache_port
pub const CACHE_PFX_ROW_TARGET: Pin = Pin { plain: 0xFFFFACBE, debug: 0xFFFFAD4C };

/// `Cache_Pfx_Col_Target`. tests: tile_cache_port
pub const CACHE_PFX_COL_TARGET: Pin = Pin { plain: 0xFFFFACC0, debug: 0xFFFFAD4E };

/// `Cache_Pfx_Skip_Armed`. tests: tile_cache_port
pub const CACHE_PFX_SKIP_ARMED: Pin = Pin { plain: 0xFFFFACC2, debug: 0xFFFFAD50 };

/// `Cache_Pfx_Lag_Flag`. tests: tile_cache_port
pub const CACHE_PFX_LAG_FLAG: Pin = Pin { plain: 0xFFFFACC4, debug: 0xFFFFAD52 };

/// `Block_Stage_Gen`. tests: tile_cache_port
pub const BLOCK_STAGE_GEN: Pin = Pin { plain: 0xFFFFB4D8, debug: 0xFFFFB566 };

/// `Pfx_Memo_Row`. tests: tile_cache_port
pub const PFX_MEMO_ROW: Pin = Pin { plain: 0xFFFFB4DA, debug: 0xFFFFB568 };

/// `Pfx_Memo_L16`. tests: tile_cache_port
pub const PFX_MEMO_L16: Pin = Pin { plain: 0xFFFFB4DC, debug: 0xFFFFB56A };

/// `Pfx_Memo_H16`. tests: tile_cache_port
pub const PFX_MEMO_H16: Pin = Pin { plain: 0xFFFFB4DE, debug: 0xFFFFB56C };

/// `Pfx_Memo_Gen`. tests: tile_cache_port
pub const PFX_MEMO_GEN: Pin = Pin { plain: 0xFFFFB4E0, debug: 0xFFFFB56E };

/// `Cs_Memo_Col`. tests: tile_cache_port
pub const CS_MEMO_COL: Pin = Pin { plain: 0xFFFFB4E2, debug: 0xFFFFB570 };

/// `Cs_Memo_T16`. tests: tile_cache_port
pub const CS_MEMO_T16: Pin = Pin { plain: 0xFFFFB4E4, debug: 0xFFFFB572 };

/// `Cs_Memo_B16`. tests: tile_cache_port
pub const CS_MEMO_B16: Pin = Pin { plain: 0xFFFFB4E6, debug: 0xFFFFB574 };

/// `Cs_Memo_Gen`. tests: tile_cache_port
pub const CS_MEMO_GEN: Pin = Pin { plain: 0xFFFFB4E8, debug: 0xFFFFB576 };

/// `Pfx_Memo_Mask`. tests: tile_cache_port
pub const PFX_MEMO_MASK: Pin = Pin { plain: 0xFFFF6998, debug: 0xFFFF6998 };

/// `Cs_Memo_Mask`. tests: tile_cache_port
pub const CS_MEMO_MASK: Pin = Pin { plain: 0xFFFF699A, debug: 0xFFFF699A };

/// `Cache_Spec_Gen_Ring`. tests: tile_cache_port
pub const CACHE_SPEC_GEN_RING: Pin = Pin { plain: 0xFFFF6982, debug: 0xFFFF6982 };

/// `Cache_Spec_Window`. tests: tile_cache_port
pub const CACHE_SPEC_WINDOW: Pin = Pin { plain: 0xFFFF6992, debug: 0xFFFF6992 };

/// `Cache_Spec_Blocked`. tests: tile_cache_port
pub const CACHE_SPEC_BLOCKED: Pin = Pin { plain: 0xFFFF6994, debug: 0xFFFF6994 };

/// `Cache_Spec_Skips`. tests: tile_cache_port
pub const CACHE_SPEC_SKIPS: Pin = Pin { plain: 0xFFFF6996, debug: 0xFFFF6996 };

/// `S4LZ_DecompressDict`. tests: tile_cache_port
pub const S4_LZ_DECOMPRESS_DICT: Pin = Pin { plain: 0x26E0, debug: 0x2870 };

/// `Player_1`. tests: collision_port, rings_port
pub const PLAYER_1: Pin = Pin { plain: 0xFFFF8E48, debug: 0xFFFF8ED6 };

/// `Cheat_Flags`. tests: test_g4_final_objects_port, test_p1_player_port
pub const CHEAT_FLAGS: Pin = Pin { plain: 0xFFFFB95C, debug: 0xFFFFE292 };

/// `Dynamic_Slots`. tests: collision_port
pub const DYNAMIC_SLOTS: Pin = Pin { plain: 0xFFFF8EE8, debug: 0xFFFF8F76 };

/// `Ring_Buffer`. tests: rings_port
pub const RING_BUFFER: Pin = Pin { plain: 0xFFFFAD7A, debug: 0xFFFFAE08 };

/// `Ring_Count`. tests: rings_port
pub const RING_COUNT: Pin = Pin { plain: 0xFFFFB07A, debug: 0xFFFFB108 };

/// `Ring_HighWater`. tests: rings_port
pub const RING_HIGH_WATER: Pin = Pin { plain: 0xFFFFB07B, debug: 0xFFFFB109 };

/// `Ring_Add_Dropped`. tests: rings_port
pub const RING_ADD_DROPPED: Pin = Pin { plain: 0xFFFFB07C, debug: 0xFFFFB10A };

/// `Ring_Counter`. tests: rings_port
pub const RING_COUNTER: Pin = Pin { plain: 0xFFFFB0E6, debug: 0xFFFFB174 };

/// `Ring_Anim_Frame`. tests: rings_port
pub const RING_ANIM_FRAME: Pin = Pin { plain: 0xFFFFB0E8, debug: 0xFFFFB176 };

/// `Ring_Anim_Timer`. tests: rings_port
pub const RING_ANIM_TIMER: Pin = Pin { plain: 0xFFFFB0E9, debug: 0xFFFFB177 };

/// `Camera_X`. tests: rings_port, section_port, camera_port, bg_anim_port
pub const CAMERA_X: Pin = Pin { plain: 0xFFFFA576, debug: 0xFFFFA604 };

/// `Camera_Y`. tests: rings_port, section_port, camera_port, bg_anim_port
pub const CAMERA_Y: Pin = Pin { plain: 0xFFFFA57A, debug: 0xFFFFA608 };

/// `Camera_Target`. tests: camera_port, test_g4_final_objects_port, test_t1_harness_states_port
pub const CAMERA_TARGET: Pin = Pin { plain: 0xFFFFAC92, debug: 0xFFFFAD20 };

/// `Camera_Curl_Offset`. tests: camera_port, test_p1_player_port
pub const CAMERA_CURL_OFFSET: Pin = Pin { plain: 0xFFFFAC94, debug: 0xFFFFAD22 };

/// `Camera_Deadzone_Base`. tests: camera_port
pub const CAMERA_DEADZONE_BASE: Pin = Pin { plain: 0xFFFFAC88, debug: 0xFFFFAD16 };

/// `Camera_Pan_Offset`. tests: camera_port
pub const CAMERA_PAN_OFFSET: Pin = Pin { plain: 0xFFFFAC8C, debug: 0xFFFFAD1A };

/// `Camera_Hold_Frames`. tests: camera_port
pub const CAMERA_HOLD_FRAMES: Pin = Pin { plain: 0xFFFFAC96, debug: 0xFFFFAD24 };

/// `Camera_Art_Hold`. tests: camera_port, tile_cache_port
pub const CAMERA_ART_HOLD: Pin = Pin { plain: 0xFFFFAC97, debug: 0xFFFFAD25 };

/// `Dbg_Cam_Clamp_Frames` — debug-shape consumer only (`debug_only`). tests: camera_port
pub const DBG_CAM_CLAMP_FRAMES: u32 = 0xFFFF8E7C;

/// `Camera_X_Max`. tests: camera_port
pub const CAMERA_X_MAX: Pin = Pin { plain: 0xFFFFAC8E, debug: 0xFFFFAD1C };

/// `Camera_Y_Max`. tests: camera_port
pub const CAMERA_Y_MAX: Pin = Pin { plain: 0xFFFFAC90, debug: 0xFFFFAD1E };

/// `BgAnim_LastStep`. tests: bg_anim_port
pub const BG_ANIM_LAST_STEP: Pin = Pin { plain: 0xFFFF8DDE, debug: 0xFFFF8DDE };

/// `BgAnim_Table`. tests: bg_anim_port
pub const BG_ANIM_TABLE: Pin = Pin { plain: 0x27652, debug: 0x27F10 };

/// `Camera_X_Biased`. tests: sprites_port
pub const CAMERA_X_BIASED: Pin = Pin { plain: 0xFFFFA57E, debug: 0xFFFFA60C };

/// `Camera_Y_Biased`. tests: sprites_port
pub const CAMERA_Y_BIASED: Pin = Pin { plain: 0xFFFFA580, debug: 0xFFFFA60E };

/// `Collected_MarkRing`. tests: rings_port
pub const COLLECTED_MARK_RING: Pin = Pin { plain: 0x39E6, debug: 0x43DC };

/// `EntityWindow_EntryForSection`. tests: rings_port
pub const ENTITY_WINDOW_ENTRY_FOR_SECTION: Pin = Pin { plain: 0x3C02, debug: 0x48BE };

/// `EntityLoaded_Clear`. tests: rings_port
pub const ENTITY_LOADED_CLEAR: Pin = Pin { plain: 0x3BEE, debug: 0x4848 };

/// `Sound_PlayRing`. tests: rings_port
pub const SOUND_PLAY_RING: Pin = Pin { plain: 0x806A, debug: 0xAE96 };

/// `MDDBG__ErrorHandler` — debug-shape consumer only (`debug_only`). tests: rings_port
pub const MDDBG_ERROR_HANDLER: u32 = 0xA6FEA;

/// `MDDBG__ErrorHandler_PagesController` — debug-shape consumer only (`debug_only`). tests: rings_port
pub const MDDBG_ERROR_HANDLER_PAGES_CONTROLLER: u32 = 0xA7DB0;

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
pub const ACT_ART_BUDGET: Pin = Pin { plain: 0xFFFFB8D4, debug: 0xFFFFB962 };

/// `Art_Budget_Remaining`. tests: load_art_port
pub const ART_BUDGET_REMAINING: Pin = Pin { plain: 0xFFFFB8D6, debug: 0xFFFFB964 };

/// `PageIn_Pool_Pages`. tests: load_art_port
pub const PAGE_IN_POOL_PAGES: Pin = Pin { plain: 0xFFFFB8C8, debug: 0xFFFFB956 };

/// `PageIn_Bulk_Drain`. tests: load_art_port
pub const PAGE_IN_BULK_DRAIN: Pin = Pin { plain: 0xFFFFB8C3, debug: 0xFFFFB951 };

/// `PageIn_Fully_Resident`. tests: load_art_port
pub const PAGE_IN_FULLY_RESIDENT: Pin = Pin { plain: 0xFFFFB8CA, debug: 0xFFFFB958 };

/// `Block_Stage_Maps`. tests: tile_cache_port
pub const BLOCK_STAGE_MAPS: Pin = Pin { plain: 0xFFFFB530, debug: 0xFFFFB5BE };

/// `Cache_Cur_LocalMap`. tests: tile_cache_port
pub const CACHE_CUR_LOCAL_MAP: Pin = Pin { plain: 0xFFFFB570, debug: 0xFFFFB5FE };

/// `PageCache_Direct_Map`. tests: load_art_port
pub const PAGE_CACHE_DIRECT_MAP: Pin = Pin { plain: 0xFFFFB8CB, debug: 0xFFFFB959 };

/// `Page_Table`. tests: load_art_port
pub const PAGE_TABLE: Pin = Pin { plain: 0xFFFF699C, debug: 0xFFFF699C };

/// `Dbg_DMA_Enq_Capped` — debug-shape consumer only (`debug_only`). tests: bg_anim_port, dma_queue_port, dplc_port
pub const DBG_DMA_ENQ_CAPPED: u32 = 0xFFFF8E52;

/// `DMA_Overflow_Count` — debug-shape consumer only (`debug_only`). tests: dma_queue_port
pub const DMA_OVERFLOW_COUNT: u32 = 0xFFFF8E50;

/// `Art_Staging_Buffer`. tests: load_art_port
pub const ART_STAGING_BUFFER: Pin = Pin { plain: 0xFFFF6B34, debug: 0xFFFF6B34 };

/// `S4LZ_Decompress`. tests: load_art_port
pub const S4_LZ_DECOMPRESS: Pin = Pin { plain: 0x26E4, debug: 0x28C8 };

/// `QueueDMA_Critical`. tests: load_art_port
pub const QUEUE_DMA_CRITICAL: Pin = Pin { plain: 0x1D58, debug: 0x1E2E };

/// `BG_Init`. tests: load_art_port
pub const BG_INIT: Pin = Pin { plain: 0x7D30, debug: 0x9AB0 };

/// `QueueDMA_Important`. tests: dplc_port
pub const QUEUE_DMA_IMPORTANT: Pin = Pin { plain: 0x1D62, debug: 0x1E38 };

/// `QueueDMA_Deferrable`. tests: dplc_port
pub const QUEUE_DMA_DEFERRABLE: Pin = Pin { plain: 0x1D6C, debug: 0x1E42 };

/// `Object_RAM`. tests: core_port
pub const OBJECT_RAM: Pin = Pin { plain: 0xFFFF8E48, debug: 0xFFFF8ED6 };

/// `System_Slots`. tests: core_port
pub const SYSTEM_SLOTS: Pin = Pin { plain: 0xFFFF9B68, debug: 0xFFFF9BF6 };

/// `Effect_Slots`. tests: core_port
pub const EFFECT_SLOTS: Pin = Pin { plain: 0xFFFF9DE8, debug: 0xFFFF9E76 };

/// `Game_Paused`. tests: core_port
pub const GAME_PAUSED: Pin = Pin { plain: 0xFFFFA582, debug: 0xFFFFA610 };

/// `Object_RAM_End`. tests: core_port
pub const OBJECT_RAM_END: Pin = Pin { plain: 0xFFFFA2E8, debug: 0xFFFFA376 };

/// `Dynamic_Free_Stack`. tests: core_port
pub const DYNAMIC_FREE_STACK: Pin = Pin { plain: 0xFFFFA2E8, debug: 0xFFFFA376 };

/// `Dynamic_Free_SP`. tests: core_port
pub const DYNAMIC_FREE_SP: Pin = Pin { plain: 0xFFFFA338, debug: 0xFFFFA3C6 };

/// `Effect_Free_Stack`. tests: core_port
pub const EFFECT_FREE_STACK: Pin = Pin { plain: 0xFFFFA33A, debug: 0xFFFFA3C8 };

/// `Effect_Free_SP`. tests: core_port
pub const EFFECT_FREE_SP: Pin = Pin { plain: 0xFFFFA35A, debug: 0xFFFFA3E8 };

/// `Dynamic_Live`. tests: core_port
pub const DYNAMIC_LIVE: Pin = Pin { plain: 0xFFFFB472, debug: 0xFFFFB500 };

/// `Dynamic_Live_Count`. tests: core_port
pub const DYNAMIC_LIVE_COUNT: Pin = Pin { plain: 0xFFFFB4C2, debug: 0xFFFFB550 };

/// `Dynamic_Live_Dirty`. tests: core_port
pub const DYNAMIC_LIVE_DIRTY: Pin = Pin { plain: 0xFFFFB4C4, debug: 0xFFFFB552 };

/// `Dynamic_Live_Walking` — debug-shape consumer only (`debug_only`). tests: core_port, collision_port, entity_window_port
pub const DYNAMIC_LIVE_WALKING: u32 = 0xFFFFB553;

/// `Dynamic_Live_Pending`. tests: core_port
pub const DYNAMIC_LIVE_PENDING: Pin = Pin { plain: 0xFFFFB4C6, debug: 0xFFFFB554 };

/// `Dynamic_Live_Pending_Count`. tests: core_port
pub const DYNAMIC_LIVE_PENDING_COUNT: Pin = Pin { plain: 0xFFFFB4D6, debug: 0xFFFFB564 };

/// `DeleteObject`. tests: animate_port, children_port
pub const DELETE_OBJECT: Pin = Pin { plain: 0x2DC0, debug: 0x3060 };

/// `DrawRings`. tests: sprites_port
pub const DRAW_RINGS: Pin = Pin { plain: 0x382A, debug: 0x41B6 };

/// `Sprite_Table_Buffer`. tests: sprites_port
pub const SPRITE_TABLE_BUFFER: Pin = Pin { plain: 0xFFFF8298, debug: 0xFFFF8298 };

/// `Sprite_Table_Dirty`. tests: sprites_port
pub const SPRITE_TABLE_DIRTY: Pin = Pin { plain: 0xFFFF8518, debug: 0xFFFF8518 };

/// `Sprite_Emit_Active`. tests: sprites_port, buffers_port
pub const SPRITE_EMIT_ACTIVE: Pin = Pin { plain: 0xFFFF8519, debug: 0xFFFF8519 };

/// `Sprite_Bands`. tests: sprites_port
pub const SPRITE_BANDS: Pin = Pin { plain: 0xFFFFA35C, debug: 0xFFFFA3EA };

/// `Sprite_Band_Counts`. tests: sprites_port
pub const SPRITE_BAND_COUNTS: Pin = Pin { plain: 0xFFFFA55C, debug: 0xFFFFA5EA };

/// `Sprites_Rendered`. tests: sprites_port
pub const SPRITES_RENDERED: Pin = Pin { plain: 0xFFFFA564, debug: 0xFFFFA5F2 };

/// `Sprite_Cycle_Counter`. tests: sprites_port
pub const SPRITE_CYCLE_COUNTER: Pin = Pin { plain: 0xFFFFA566, debug: 0xFFFFA5F4 };

/// `Sprite_Owner` — debug-shape consumer only (`debug_only`). tests: sprites_port
pub const SPRITE_OWNER: u32 = 0xFFFFE1EE;

/// `SpriteMask_Y`. tests: sprites_port
pub const SPRITE_MASK_Y: Pin = Pin { plain: 0xFFFFA568, debug: 0xFFFFA5F6 };

/// `SpriteMask_Height`. tests: sprites_port
pub const SPRITE_MASK_HEIGHT: Pin = Pin { plain: 0xFFFFA56A, debug: 0xFFFFA5F8 };

/// `SpriteMask_After_Band`. tests: sprites_port
pub const SPRITE_MASK_AFTER_BAND: Pin = Pin { plain: 0xFFFFA56C, debug: 0xFFFFA5FA };

/// `Scanline_Band_Sprites`. tests: sprites_port
pub const SCANLINE_BAND_SPRITES: Pin = Pin { plain: 0xFFFFA56E, debug: 0xFFFFA5FC };

/// `Sound_PlaySFX`. tests: animate_port
pub const SOUND_PLAY_SFX: Pin = Pin { plain: 0x7FD4, debug: 0xADBA };

/// `ObjectMoveX`. tests: test_g4_final_objects_port
pub const OBJECT_MOVE_X: Pin = Pin { plain: 0x2FCC, debug: 0x36B6 };

/// `ObjCodeBase`. tests: test_objects_port
pub const OBJ_CODE_BASE: Pin = Pin { plain: 0x10000, debug: 0x10000 };

/// `Draw_Sprite`. tests: test_objects_port
pub const DRAW_SPRITE: Pin = Pin { plain: 0x3004, debug: 0x36F4 };

/// `ObjectMove`. tests: test_objects_port
pub const OBJECT_MOVE: Pin = Pin { plain: 0x2FB2, debug: 0x369C };

/// `Ring_Sfx_Speaker`. tests: sound_api_port
pub const RING_SFX_SPEAKER: Pin = Pin { plain: 0xFFFFB3B6, debug: 0xFFFFB444 };

/// `Sfx_Ring_Buf`. tests: sound_api_port
pub const SFX_RING_BUF: Pin = Pin { plain: 0xFFFFB3B8, debug: 0xFFFFB446 };

/// `Sfx_Ring_Wr`. tests: sound_api_port
pub const SFX_RING_WR: Pin = Pin { plain: 0xFFFFB3C0, debug: 0xFFFFB44E };

/// `Sfx_Ring_Rd`. tests: sound_api_port
pub const SFX_RING_RD: Pin = Pin { plain: 0xFFFFB3C1, debug: 0xFFFFB44F };

/// `SongTable`. tests: sound_api_port
pub const SONG_TABLE: Pin = Pin { plain: 0xA3B10, debug: 0xA5550 };

/// `SongPatchTable`. tests: sound_api_port
pub const SONG_PATCH_TABLE: Pin = Pin { plain: 0xA3B14, debug: 0xA555C };

/// `OJZ_Palette`. tests: act_descriptor_port
pub const OJZ_PALETTE: Pin = Pin { plain: 0x22DD0, debug: 0x23680 };

/// `OJZ_Act1_BG_Layout`. tests: act_descriptor_port
pub const OJZ_ACT1_BG_LAYOUT: Pin = Pin { plain: 0x22E50, debug: 0x23700 };

/// `OJZ_Act1_BG_Tiles`. tests: act_descriptor_port
pub const OJZ_ACT1_BG_TILES: Pin = Pin { plain: 0x24E50, debug: 0x25700 };

/// `ParallaxConfig_OJZ_Default`. tests: act_descriptor_port
pub const PARALLAX_CONFIG_OJZ_DEFAULT: Pin = Pin { plain: 0x124A4, debug: 0x12CE8 };

/// `OJZ_Act_Pool_PageTable`. tests: act_descriptor_port
pub const OJZ_ACT_POOL_PAGE_TABLE: Pin = Pin { plain: 0x16824, debug: 0x170DC };

/// `OJZ_Sec_LocalMaps`. tests: act_descriptor_port
pub const OJZ_SEC_LOCAL_MAPS: Pin = Pin { plain: 0x22DA0, debug: 0x2365C };

/// `OJZ_Sec0_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC0_BLOCKS: Pin = Pin { plain: 0x16AF0, debug: 0x173B0 };

/// `OJZ_Sec1_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC1_BLOCKS: Pin = Pin { plain: 0x18C60, debug: 0x19520 };

/// `OJZ_Sec2_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC2_BLOCKS: Pin = Pin { plain: 0x19FDC, debug: 0x1A89C };

/// `OJZ_Sec3_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC3_BLOCKS: Pin = Pin { plain: 0x1B774, debug: 0x1C034 };

/// `OJZ_Sec4_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC4_BLOCKS: Pin = Pin { plain: 0x19FDC, debug: 0x1A89C };

/// `OJZ_Sec5_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC5_BLOCKS: Pin = Pin { plain: 0x1C8C0, debug: 0x1D180 };

/// `OJZ_Sec6_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC6_BLOCKS: Pin = Pin { plain: 0x1D6E6, debug: 0x1DFA6 };

/// `OJZ_Sec7_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC7_BLOCKS: Pin = Pin { plain: 0x1F2E6, debug: 0x1FBA6 };

/// `OJZ_Sec8_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC8_BLOCKS: Pin = Pin { plain: 0x2055A, debug: 0x20E1A };

/// `OJZ_Sec0_Objects`. tests: act_descriptor_port
pub const OJZ_SEC0_OBJECTS: Pin = Pin { plain: 0x137FE, debug: 0x140B6 };

/// `OJZ_Sec0_Rings`. tests: act_descriptor_port
pub const OJZ_SEC0_RINGS: Pin = Pin { plain: 0x13806, debug: 0x140BE };

/// `OJZ_Sec0_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC0_TYPE_TABLE: Pin = Pin { plain: 0x137F8, debug: 0x140B0 };

/// `OJZ_Sec1_Objects`. tests: act_descriptor_port
pub const OJZ_SEC1_OBJECTS: Pin = Pin { plain: 0x13830, debug: 0x140E8 };

/// `OJZ_Sec1_Rings`. tests: act_descriptor_port
pub const OJZ_SEC1_RINGS: Pin = Pin { plain: 0x13844, debug: 0x140FC };

/// `OJZ_Sec1_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC1_TYPE_TABLE: Pin = Pin { plain: 0x13826, debug: 0x140DE };

/// `OJZ_Sec2_Objects`. tests: act_descriptor_port
pub const OJZ_SEC2_OBJECTS: Pin = Pin { plain: 0x13876, debug: 0x1412E };

/// `OJZ_Sec2_Rings`. tests: act_descriptor_port
pub const OJZ_SEC2_RINGS: Pin = Pin { plain: 0x13884, debug: 0x1413C };

/// `OJZ_Sec2_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC2_TYPE_TABLE: Pin = Pin { plain: 0x1386C, debug: 0x14124 };

/// `OJZ_Sec3_Objects`. tests: act_descriptor_port
pub const OJZ_SEC3_OBJECTS: Pin = Pin { plain: 0x138BA, debug: 0x14172 };

/// `OJZ_Sec3_Rings`. tests: act_descriptor_port
pub const OJZ_SEC3_RINGS: Pin = Pin { plain: 0x138BC, debug: 0x14174 };

/// `OJZ_Sec3_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC3_TYPE_TABLE: Pin = Pin { plain: 0x138B8, debug: 0x14170 };

/// `OJZ_Sec4_Objects`. tests: act_descriptor_port
pub const OJZ_SEC4_OBJECTS: Pin = Pin { plain: 0x138C2, debug: 0x1417A };

/// `OJZ_Sec4_Rings`. tests: act_descriptor_port
pub const OJZ_SEC4_RINGS: Pin = Pin { plain: 0x138C4, debug: 0x1417C };

/// `OJZ_Sec4_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC4_TYPE_TABLE: Pin = Pin { plain: 0x138C0, debug: 0x14178 };

/// `OJZ_Sec5_Objects`. tests: act_descriptor_port
pub const OJZ_SEC5_OBJECTS: Pin = Pin { plain: 0x138FA, debug: 0x141B2 };

/// `OJZ_Sec5_Rings`. tests: act_descriptor_port
pub const OJZ_SEC5_RINGS: Pin = Pin { plain: 0x138FC, debug: 0x141B4 };

/// `OJZ_Sec5_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC5_TYPE_TABLE: Pin = Pin { plain: 0x138F8, debug: 0x141B0 };

/// `OJZ_Sec6_Objects`. tests: act_descriptor_port
pub const OJZ_SEC6_OBJECTS: Pin = Pin { plain: 0x13922, debug: 0x141DA };

/// `OJZ_Sec6_Rings`. tests: act_descriptor_port
pub const OJZ_SEC6_RINGS: Pin = Pin { plain: 0x13924, debug: 0x141DC };

/// `OJZ_Sec6_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC6_TYPE_TABLE: Pin = Pin { plain: 0x13920, debug: 0x141D8 };

/// `OJZ_Sec7_Objects`. tests: act_descriptor_port
pub const OJZ_SEC7_OBJECTS: Pin = Pin { plain: 0x1392A, debug: 0x141E2 };

/// `OJZ_Sec7_Rings`. tests: act_descriptor_port
pub const OJZ_SEC7_RINGS: Pin = Pin { plain: 0x1392C, debug: 0x141E4 };

/// `OJZ_Sec7_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC7_TYPE_TABLE: Pin = Pin { plain: 0x13928, debug: 0x141E0 };

/// `OJZ_Sec8_Objects`. tests: act_descriptor_port
pub const OJZ_SEC8_OBJECTS: Pin = Pin { plain: 0x13952, debug: 0x1420A };

/// `OJZ_Sec8_Rings`. tests: act_descriptor_port
pub const OJZ_SEC8_RINGS: Pin = Pin { plain: 0x13954, debug: 0x1420C };

/// `OJZ_Sec8_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC8_TYPE_TABLE: Pin = Pin { plain: 0x13950, debug: 0x14208 };

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
pub const CAMERA_Y_COARSE_PREV: Pin = Pin { plain: 0xFFFFB1F6, debug: 0xFFFFB284 };

/// `Current_Act_Ptr`. tests: entity_window_port, section_port
pub const CURRENT_ACT_PTR: Pin = Pin { plain: 0xFFFFB3B2, debug: 0xFFFFB440 };

/// `Entity_Window_Active`. tests: entity_window_port
pub const ENTITY_WINDOW_ACTIVE: Pin = Pin { plain: 0xFFFFB0EA, debug: 0xFFFFB178 };

/// `Entity_Window_Anchor`. tests: entity_window_port
pub const ENTITY_WINDOW_ANCHOR: Pin = Pin { plain: 0xFFFFB0EC, debug: 0xFFFFB17A };

/// `Entity_Window_OriginX`. tests: entity_window_port
pub const ENTITY_WINDOW_ORIGIN_X: Pin = Pin { plain: 0xFFFFB0EE, debug: 0xFFFFB17C };

/// `Entity_Window_OriginY`. tests: entity_window_port
pub const ENTITY_WINDOW_ORIGIN_Y: Pin = Pin { plain: 0xFFFFB0F0, debug: 0xFFFFB17E };

/// `Entity_Window_Center_ID`. tests: entity_window_port
pub const ENTITY_WINDOW_CENTER_ID: Pin = Pin { plain: 0xFFFFB0EB, debug: 0xFFFFB179 };

/// `Entity_Scan_State`. tests: entity_window_port
pub const ENTITY_SCAN_STATE: Pin = Pin { plain: 0xFFFFB07E, debug: 0xFFFFB10C };

/// `Entity_Loaded_Masks`. tests: entity_window_port
pub const ENTITY_LOADED_MASKS: Pin = Pin { plain: 0xFFFFB0F2, debug: 0xFFFFB180 };

/// `Entity_Mask_Scratch`. tests: entity_window_port
pub const ENTITY_MASK_SCRATCH: Pin = Pin { plain: 0xFFFFB172, debug: 0xFFFFB200 };

/// `Ring_Collected_Window`. tests: entity_window_port
pub const RING_COLLECTED_WINDOW: Pin = Pin { plain: 0xFFFFB1F8, debug: 0xFFFFB286 };

/// `Ring_Collected_Park`. tests: entity_window_port
pub const RING_COLLECTED_PARK: Pin = Pin { plain: 0xFFFFB32C, debug: 0xFFFFB3BA };

/// `Collected_Park_Next`. tests: entity_window_port
pub const COLLECTED_PARK_NEXT: Pin = Pin { plain: 0xFFFFB3B0, debug: 0xFFFFB43E };

/// `RingBuffer_Clear`. tests: entity_window_port
pub const RING_BUFFER_CLEAR: Pin = Pin { plain: 0x381C, debug: 0x41A8 };

/// `RingBuffer_Remove`. tests: entity_window_port
pub const RING_BUFFER_REMOVE: Pin = Pin { plain: 0x37E8, debug: 0x4174 };

/// `Section_GetSecPtrXY`. tests: entity_window_port
pub const SECTION_GET_SEC_PTR_XY: Pin = Pin { plain: 0x5D44, debug: 0x6EB4 };

/// `Section_FlatIDXY`. tests: entity_window_port
pub const SECTION_FLAT_IDXY: Pin = Pin { plain: 0x5D2A, debug: 0x6E9A };

/// `AllocDynamic`. tests: load_object_port, children_port
pub const ALLOC_DYNAMIC: Pin = Pin { plain: 0x2D42, debug: 0x2FE2 };

/// `AllocEffect`. tests: children_port
pub const ALLOC_EFFECT: Pin = Pin { plain: 0x2DA6, debug: 0x3046 };

/// `Palette_Buffer`. tests: buffers_port
pub const PALETTE_BUFFER: Pin = Pin { plain: 0xFFFF8216, debug: 0xFFFF8216 };

/// `Hscroll_Buffer`. tests: buffers_port
pub const HSCROLL_BUFFER: Pin = Pin { plain: 0xFFFF851A, debug: 0xFFFF851A };

/// `Static_Pal_Line0`. tests: buffers_port
pub const STATIC_PAL_LINE0: Pin = Pin { plain: 0xFFFF8DE6, debug: 0xFFFF8DE6 };

/// `Static_Pal_Line1`. tests: buffers_port
pub const STATIC_PAL_LINE1: Pin = Pin { plain: 0xFFFF8DF4, debug: 0xFFFF8DF4 };

/// `Static_Pal_Line2`. tests: buffers_port
pub const STATIC_PAL_LINE2: Pin = Pin { plain: 0xFFFF8E02, debug: 0xFFFF8E02 };

/// `Static_Pal_Line3`. tests: buffers_port
pub const STATIC_PAL_LINE3: Pin = Pin { plain: 0xFFFF8E1E, debug: 0xFFFF8E1E };

/// `Static_Sprite_DMA`. tests: buffers_port
pub const STATIC_SPRITE_DMA: Pin = Pin { plain: 0xFFFF8E2C, debug: 0xFFFF8E2C };

/// `Static_Hscroll_Line`. tests: buffers_port
pub const STATIC_HSCROLL_LINE: Pin = Pin { plain: 0xFFFF8E3A, debug: 0xFFFF8E3A };

/// `Palette_Dirty`. tests: buffers_port, palette_port
pub const PALETTE_DIRTY: Pin = Pin { plain: 0xFFFF8296, debug: 0xFFFF8296 };

/// `Parallax_Active_Config`. tests: buffers_port
pub const PARALLAX_ACTIVE_CONFIG: Pin = Pin { plain: 0x63C0, debug: 0x7634 };

/// `Palette_Ship_Snap`. tests: buffers_port
pub const PALETTE_SHIP_SNAP: Pin = Pin { plain: 0xFFFFB8DC, debug: 0xFFFFB96A };

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
pub const LAG_FRAME_COUNT: u32 = 0xFFFF8E54;

/// `DMA_Bytes_ThisFrame` — debug-shape consumer only (`debug_only`). tests: vblank_port
pub const DMA_BYTES_THIS_FRAME: u32 = 0xFFFF8E48;

/// `PageIn_InFlight`. tests: game_loop_port
pub const PAGE_IN_IN_FLIGHT: Pin = Pin { plain: 0xFFFFB896, debug: 0xFFFFB924 };

/// `PageIn_Saved_PC`. tests: game_loop_port
pub const PAGE_IN_SAVED_PC: Pin = Pin { plain: 0xFFFFB890, debug: 0xFFFFB91E };

/// `PageIn_BankRegs`. tests: game_loop_port
pub const PAGE_IN_BANK_REGS: Pin = Pin { plain: 0x772C, debug: 0x8B14 };

/// `Dbg_PageIn_Preempts` — debug-shape consumer only (`debug_only`). tests: game_loop_port
pub const DBG_PAGE_IN_PREEMPTS: u32 = 0xFFFF8E6E;

/// `ZX0R_Decompress.__end`. tests: game_loop_port
pub const ZX0R_DECOMPRESS_END: Pin = Pin { plain: 0x2850, debug: 0x2AE8 };

/// `PageIn_Staging_Busy`. tests: game_loop_port, load_art_port
pub const PAGE_IN_STAGING_BUSY: Pin = Pin { plain: 0xFFFFB898, debug: 0xFFFFB926 };

/// `PageIn_Flush`. tests: load_art_port
pub const PAGE_IN_FLUSH: Pin = Pin { plain: 0x77F4, debug: 0x8BE4 };

/// `PageIn_Enqueue`. tests: load_art_port
pub const PAGE_IN_ENQUEUE: Pin = Pin { plain: 0x77B6, debug: 0x8BA6 };

/// `PageIn_Pool_Table`. tests: load_art_port
pub const PAGE_IN_POOL_TABLE: Pin = Pin { plain: 0xFFFFB8C4, debug: 0xFFFFB952 };

/// `PageIn_Queue_Count`. tests: load_art_port
pub const PAGE_IN_QUEUE_COUNT: Pin = Pin { plain: 0xFFFFB89A, debug: 0xFFFFB928 };

/// `PageIn_Suspended`. tests: load_art_port
pub const PAGE_IN_SUSPENDED: Pin = Pin { plain: 0xFFFFB897, debug: 0xFFFFB925 };

/// `PageIn_Land_Pending`. tests: load_art_port
pub const PAGE_IN_LAND_PENDING: Pin = Pin { plain: 0xFFFFB899, debug: 0xFFFFB927 };

/// `PageCache_Init`. tests: load_art_port
pub const PAGE_CACHE_INIT: Pin = Pin { plain: 0x7844, debug: 0x8C34 };

/// `PageCache_AllocFrame`. tests: load_art_port
pub const PAGE_CACHE_ALLOC_FRAME: Pin = Pin { plain: 0x78F4, debug: 0x8D48 };

/// `PageCache_Publish`. tests: load_art_port
pub const PAGE_CACHE_PUBLISH: Pin = Pin { plain: 0x79B0, debug: 0x8F08 };

/// `PageCache_PatchRun_Seq`. tests: tile_cache_port
pub const PAGE_CACHE_PATCH_RUN_SEQ: Pin = Pin { plain: 0x7A1E, debug: 0x8FDC };

/// `PageCache_PatchRun_Col`. tests: tile_cache_port
pub const PAGE_CACHE_PATCH_RUN_COL: Pin = Pin { plain: 0x7B22, debug: 0x921C };

/// `PageCache_Audit`. tests: tile_cache_port
pub const PAGE_CACHE_AUDIT: Pin = Pin { plain: 0x7D26, debug: 0x959C };

/// `Cache_Art_Stall`. tests: tile_cache_port
pub const CACHE_ART_STALL: Pin = Pin { plain: 0xFFFFACAA, debug: 0xFFFFAD38 };

/// `Page_Audit_Ticks` — debug-shape consumer only (`debug_only`). tests: tile_cache_port
pub const PAGE_AUDIT_TICKS: u32 = 0xFFFF8E82;

/// `Cache_Stall_Watchdog` — debug-shape consumer only (`debug_only`). tests: tile_cache_port
pub const CACHE_STALL_WATCHDOG: u32 = 0xFFFF8E80;

/// `Flush_VDP_Shadow`. tests: vblank_port
pub const FLUSH_VDP_SHADOW: Pin = Pin { plain: 0x1C12, debug: 0x1C90 };

/// `VInt_DrawLevel`. tests: vblank_port
pub const V_INT_DRAW_LEVEL: Pin = Pin { plain: 0x4886, debug: 0x5736 };

/// `Vscroll_Write`. tests: vblank_port
pub const VSCROLL_WRITE: Pin = Pin { plain: 0x63D2, debug: 0x7646 };

/// `Read_Controllers`. tests: vblank_port
pub const READ_CONTROLLERS: Pin = Pin { plain: 0x2460, debug: 0x2540 };

/// `Process_DMA_Critical`. tests: vblank_port
pub const PROCESS_DMA_CRITICAL: Pin = Pin { plain: 0x1E32, debug: 0x1F14 };

/// `Process_DMA_Important`. tests: vblank_port
pub const PROCESS_DMA_IMPORTANT: Pin = Pin { plain: 0x1F00, debug: 0x1FE2 };

/// `Process_DMA_Deferrable`. tests: vblank_port
pub const PROCESS_DMA_DEFERRABLE: Pin = Pin { plain: 0x1F14, debug: 0x1FF6 };

/// `Enqueue_Dirty_Buffers`. tests: vblank_port
pub const ENQUEUE_DIRTY_BUFFERS: Pin = Pin { plain: 0x2056, debug: 0x212E };

/// `BootData`. tests: boot_port
pub const BOOT_DATA: Pin = Pin { plain: 0x3A0, debug: 0x3A0 };

/// `VInt_Level`. tests: boot_port
pub const V_INT_LEVEL: Pin = Pin { plain: 0x2298, debug: 0x236C };

/// `BuildStaticDMA`. tests: boot_port
pub const BUILD_STATIC_DMA: Pin = Pin { plain: 0x1F92, debug: 0x206A };

/// `Sound_Init`. tests: boot_port
pub const SOUND_INIT: Pin = Pin { plain: 0x7ED4, debug: 0xAB56 };

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
pub const P_STATE_GROUND: Pin = Pin { plain: 0x10690, debug: 0x107A0 };

/// `PState_Roll`. tests: test_p1_player_port
pub const P_STATE_ROLL: Pin = Pin { plain: 0x107F4, debug: 0x10904 };

/// `PState_Spindash`. tests: test_p1_player_port
pub const P_STATE_SPINDASH: Pin = Pin { plain: 0x10E70, debug: 0x10F80 };

/// `PState_Air`. tests: test_p1_player_port
pub const P_STATE_AIR: Pin = Pin { plain: 0x10B20, debug: 0x10C30 };

/// `PState_Jump`. tests: test_p1_player_port
pub const P_STATE_JUMP: Pin = Pin { plain: 0x10B28, debug: 0x10C38 };

/// `PState_RollJump`. tests: test_p1_player_port
pub const P_STATE_ROLL_JUMP: Pin = Pin { plain: 0x10B24, debug: 0x10C34 };

/// `PState_AirBall`. tests: test_p1_player_port
pub const P_STATE_AIR_BALL: Pin = Pin { plain: 0x10B20, debug: 0x10C30 };

/// `PState_Fly`. tests: test_p1_player_port
pub const P_STATE_FLY: Pin = Pin { plain: 0x10F0C, debug: 0x1101C };

/// `PState_Glide`. tests: test_p1_player_port
pub const P_STATE_GLIDE: Pin = Pin { plain: 0x1103E, debug: 0x11150 };

/// `PState_GlideFall`. tests: test_p1_player_port
pub const P_STATE_GLIDE_FALL: Pin = Pin { plain: 0x111D6, debug: 0x112E8 };

/// `PState_Slide`. tests: test_p1_player_port
pub const P_STATE_SLIDE: Pin = Pin { plain: 0x11222, debug: 0x11334 };

/// `PState_Climb`. tests: test_p1_player_port
pub const P_STATE_CLIMB: Pin = Pin { plain: 0x1136A, debug: 0x11480 };

/// `PState_Ledge`. tests: test_p1_player_port
pub const P_STATE_LEDGE: Pin = Pin { plain: 0x11516, debug: 0x1162C };

/// `Player_SensorFloor`. tests: test_p1_player_port
pub const PLAYER_SENSOR_FLOOR: Pin = Pin { plain: 0x5B6C, debug: 0x6CDC };

/// `Player_AtLedgeEdge`. tests: test_p1_player_port
pub const PLAYER_AT_LEDGE_EDGE: Pin = Pin { plain: 0x5C86, debug: 0x6DF6 };

/// `Player_SetState`. tests: test_p2_player_states_port
pub const PLAYER_SET_STATE: Pin = Pin { plain: 0x103E6, debug: 0x104AA };

/// `Player_SnapToSurface`. tests: test_p2_player_states_port
pub const PLAYER_SNAP_TO_SURFACE: Pin = Pin { plain: 0x10534, debug: 0x105F8 };

/// `Player_SensorCeiling`. tests: test_p2_player_states_port
pub const PLAYER_SENSOR_CEILING: Pin = Pin { plain: 0x5B82, debug: 0x6CF2 };

/// `Player_SensorWallDir`. tests: test_p2_player_states_port
pub const PLAYER_SENSOR_WALL_DIR: Pin = Pin { plain: 0x5C3C, debug: 0x6DAC };

/// `Player_SensorWallAt`. tests: test_p2_player_states_port
pub const PLAYER_SENSOR_WALL_AT: Pin = Pin { plain: 0x5C34, debug: 0x6DA4 };

/// `Collision_GetType`. tests: test_p4_player_sensors_port
pub const COLLISION_GET_TYPE: Pin = Pin { plain: 0x5790, debug: 0x6900 };

/// `SolidityTable`. tests: test_p4_player_sensors_port
pub const SOLIDITY_TABLE: Pin = Pin { plain: 0x6FE10, debug: 0x706E0 };

/// `AngleTable`. tests: test_p4_player_sensors_port
pub const ANGLE_TABLE: Pin = Pin { plain: 0x6FD10, debug: 0x705E0 };

/// `HeightMaps`. tests: test_p4_player_sensors_port
pub const HEIGHT_MAPS: Pin = Pin { plain: 0x6DD10, debug: 0x6E5E0 };

/// `HeightMapsRot`. tests: test_p4_player_sensors_port
pub const HEIGHT_MAPS_ROT: Pin = Pin { plain: 0x6ED10, debug: 0x6F5E0 };

/// `Character_ID`. tests: test_p1_player_port
pub const CHARACTER_ID: Pin = Pin { plain: 0xFFFFB95E, debug: 0xFFFFE294 };

/// `Player_Chardef`. tests: test_p1_player_port
pub const PLAYER_CHARDEF: Pin = Pin { plain: 0xFFFFB960, debug: 0xFFFFE296 };

/// `Ability_None`. tests: test_p1_player_port
pub const ABILITY_NONE: Pin = Pin { plain: 0x11F68, debug: 0x120DE };

/// `CharacterDefs`. tests: test_p1_player_port
pub const CHARACTER_DEFS: Pin = Pin { plain: 0x11F20, debug: 0x12030 };

/// `Player_InitAssets`. tests: test_p1_player_port
pub const PLAYER_INIT_ASSETS: Pin = Pin { plain: 0x11F2C, debug: 0x1203C };

/// `Player_LoadArt`. tests: test_p1_player_port
pub const PLAYER_LOAD_ART: Pin = Pin { plain: 0x11F44, debug: 0x12054 };

/// `Player_Ability`. tests: test_p2_player_states_port
pub const PLAYER_ABILITY: Pin = Pin { plain: 0x11F5E, debug: 0x1206E };

/// `PhysTable_Sonic`. tests: test_p1_player_port
pub const PHYS_TABLE_SONIC: Pin = Pin { plain: 0x11E96, debug: 0x11FA6 };

/// `Pal_SonicTails`. tests: test_p1_player_port
pub const PAL_SONIC_TAILS: Pin = Pin { plain: 0x6DCC6, debug: 0x6E596 };

/// `OJZ_TestRaster`. tests: act_descriptor_port
pub const OJZ_TEST_RASTER: Pin = Pin { plain: 0x1328A, debug: 0x13ACE };

/// `OJZ_TestPal`. tests: act_descriptor_port
pub const OJZ_TEST_PAL: Pin = Pin { plain: 0x132AC, debug: 0x13B5E };

/// `OJZ_TestGradient`. tests: act_descriptor_port
pub const OJZ_TEST_GRADIENT: Pin = Pin { plain: 0x13576, debug: 0x13E28 };

/// `OJZ_ShimmerCycle`. tests: act_descriptor_port
pub const OJZ_SHIMMER_CYCLE: Pin = Pin { plain: 0x1330C, debug: 0x13BBE };

/// `OJZ_TestVsram`. tests: act_descriptor_port
pub const OJZ_TEST_VSRAM: Pin = Pin { plain: 0x13594, debug: 0x13E46 };

/// `OJZ_TestRamp`. tests: act_descriptor_port
pub const OJZ_TEST_RAMP: Pin = Pin { plain: 0x135B2, debug: 0x13E64 };

/// `Raster_Program`. tests: raster_port
pub const RASTER_PROGRAM: Pin = Pin { plain: 0xFFFF8AC8, debug: 0xFFFF8AC8 };

/// `Raster_Cursor`. tests: raster_port
pub const RASTER_CURSOR: Pin = Pin { plain: 0xFFFF8ACC, debug: 0xFFFF8ACC };

/// `Raster_Pending`. tests: raster_port
pub const RASTER_PENDING: Pin = Pin { plain: 0xFFFF8AD0, debug: 0xFFFF8AD0 };

/// `Raster_Buf_A`. tests: raster_port
pub const RASTER_BUF_A: Pin = Pin { plain: 0xFFFF8AD6, debug: 0xFFFF8AD6 };

/// `Raster_Active_Buf`. tests: raster_port
pub const RASTER_ACTIVE_BUF: Pin = Pin { plain: 0xFFFF8BD6, debug: 0xFFFF8BD6 };

/// `Raster_Buf_B`. tests: raster_port
pub const RASTER_BUF_B: Pin = Pin { plain: 0xFFFF8B56, debug: 0xFFFF8B56 };

/// `Raster_Line`. tests: raster_port
pub const RASTER_LINE: Pin = Pin { plain: 0xFFFF8AD4, debug: 0xFFFF8AD4 };

/// `Raster_Dense_Lines`. tests: raster_port
pub const RASTER_DENSE_LINES: Pin = Pin { plain: 0xFFFF8BDA, debug: 0xFFFF8BDA };

/// `Raster_Dense_Cursor`. tests: raster_port
pub const RASTER_DENSE_CURSOR: Pin = Pin { plain: 0xFFFF8BDC, debug: 0xFFFF8BDC };

/// `Raster_Dense_Cmd`. tests: raster_port
pub const RASTER_DENSE_CMD: Pin = Pin { plain: 0xFFFF8BE0, debug: 0xFFFF8BE0 };

/// `Raster_Dense_Mode`. tests: raster_port
pub const RASTER_DENSE_MODE: Pin = Pin { plain: 0xFFFF8BE4, debug: 0xFFFF8BE4 };

/// `Raster_Ramp_Acc`. tests: raster_port
pub const RASTER_RAMP_ACC: Pin = Pin { plain: 0xFFFF8BE6, debug: 0xFFFF8BE6 };

/// `Raster_Ramp_Step`. tests: raster_port
pub const RASTER_RAMP_STEP: Pin = Pin { plain: 0xFFFF8BEA, debug: 0xFFFF8BEA };

/// `Effects_World_Y`. tests: raster_port
pub const EFFECTS_WORLD_Y: Pin = Pin { plain: 0xFFFF8BEE, debug: 0xFFFF8BEE };

/// `Effects_Screen_L`. tests: raster_port, parallax_port, buffers_port
pub const EFFECTS_SCREEN_L: Pin = Pin { plain: 0xFFFF8BF6, debug: 0xFFFF8BF6 };

/// `Effects_Offscreen_Entry`. tests: raster_port, buffers_port
pub const EFFECTS_OFFSCREEN_ENTRY: Pin = Pin { plain: 0xFFFF8BFE, debug: 0xFFFF8BFE };

/// `Static_Pal_Ship`. tests: raster_port
pub const STATIC_PAL_SHIP: Pin = Pin { plain: 0xFFFF8E10, debug: 0xFFFF8E10 };

/// `Build_DMA_Entry`. tests: raster_port
pub const BUILD_DMA_ENTRY: Pin = Pin { plain: 0x2020, debug: 0x20F8 };

/// `Raster_Patch_Tab`. tests: raster_port
pub const RASTER_PATCH_TAB: Pin = Pin { plain: 0xFFFF8C02, debug: 0xFFFF8C02 };

/// `Raster_State`. tests: raster_port
pub const RASTER_STATE: Pin = Pin { plain: 0xFFFF8AC8, debug: 0xFFFF8AC8 };

/// `Raster_State_End`. tests: raster_port
pub const RASTER_STATE_END: Pin = Pin { plain: 0xFFFF8C06, debug: 0xFFFF8C06 };

/// `Pal_Variant_Stage`. tests: raster_port
pub const PAL_VARIANT_STAGE: Pin = Pin { plain: 0xFFFF8CC6, debug: 0xFFFF8CC6 };

/// `Raster_VBlank`. tests: game_loop_port, vblank_port, load_art_port, boot_port
pub const RASTER_V_BLANK: Pin = Pin { plain: 0x6BFA, debug: 0x7E6E };

/// `Palette_Compose`. tests: game_loop_port
pub const PALETTE_COMPOSE: Pin = Pin { plain: 0x700C, debug: 0x8280 };

/// `Player_Blocks`. tests: test_p1_player_port
pub const PLAYER_BLOCKS: Pin = Pin { plain: 0xFFFFB964, debug: 0xFFFFE29A };

/// `Player_Ring_Index`. tests: test_p1_player_port
pub const PLAYER_RING_INDEX: Pin = Pin { plain: 0xFFFFBD00, debug: 0xFFFFE600 };

/// `Player_Pos_Ring`. tests: test_p1_player_port
pub const PLAYER_POS_RING: Pin = Pin { plain: 0xFFFFBB00, debug: 0xFFFFE400 };

/// `Player_Stat_Ring`. tests: test_p1_player_port
pub const PLAYER_STAT_RING: Pin = Pin { plain: 0xFFFFBC00, debug: 0xFFFFE500 };

/// `Player_Death_Pending`. tests: test_p1_player_port
pub const PLAYER_DEATH_PENDING: Pin = Pin { plain: 0xFFFFB98C, debug: 0xFFFFE2C2 };

/// `Player_Bound_Right`. tests: test_p1_player_port
pub const PLAYER_BOUND_RIGHT: Pin = Pin { plain: 0xFFFFB98E, debug: 0xFFFFE2C4 };

/// `Player_Bound_Bottom`. tests: test_p1_player_port
pub const PLAYER_BOUND_BOTTOM: Pin = Pin { plain: 0xFFFFB990, debug: 0xFFFFE2C6 };

/// `DustSpindash_Spawn`. tests: test_p1_player_port
pub const DUST_SPINDASH_SPAWN: Pin = Pin { plain: 0x12124, debug: 0x122F2 };

// ── Region-relative offsets (manifest order) ──

/// `AnimateSprite.cc_delete` − `animate` start (per-shape). tests: animate_port
pub const CC_DELETE_OFF: ShapeOffset = ShapeOffset { plain: 0x104, debug: 0x15E };

/// `RefreshSpritePieceCount` − `animate` start (per-shape). tests: animate_port
pub const REFRESH_OFF: ShapeOffset = ShapeOffset { plain: 0x16C, debug: 0x290 };

/// `RingCollision` − `rings` start (per-shape). tests: rings_port
pub const RINGCOL_OFF: ShapeOffset = ShapeOffset { plain: 0x116, debug: 0x17C };

/// `Sound_PlaySFX` − `sound_api` start (per-shape). tests: sound_api_port
pub const SOUND_PLAY_SFX_OFF: ShapeOffset = ShapeOffset { plain: 0x126, debug: 0x28A };

/// `Sine_Table` − `math` start (shape-invariant, asserted at generation). tests: math_port
pub const SINE_TABLE_OFF: usize = 0x18;

/// `Flush_VDP_Shadow` − `vdp_init` start (shape-invariant, asserted at generation). tests: vdp_init_port
pub const FLUSH_VDP_SHADOW_OFF: usize = 0x12;

/// `HBlank_Uninstall` − `hblank` start (shape-invariant, asserted at generation). tests: hblank_port, raster_port
pub const HBLANK_UNINSTALL_OFF: usize = 0x1C;
