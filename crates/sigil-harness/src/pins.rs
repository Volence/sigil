//! GENERATED FILE, DO NOT EDIT BY HAND.
//!
//! Emitted by `cargo run -p sigil-harness --bin repin` from `repin.toml`
//! + SIGIL'S OWN resolved layout (Stage-3 P4c; the asl-`.lst` parse retired).
//! Edit the MANIFEST, then regenerate; `tests/repin_pins.rs::
//! pins_rs_is_current` guards staleness. All values are per-shape VMAs/lengths
//! from sigil's native canonical resolve (plain + `__DEBUG__`).
//!
//! [provenance] plain: sigil-native canonical resolve (plain)
//! [provenance] debug: sigil-native canonical resolve (debug)
//! [provenance] 97 regions, 417 symbols, 7 offsets

/// A per-shape address pin: one cross-seam symbol's VMA in each shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pin {
    pub plain: u32,
    pub debug: u32,
}

/// A gated region's geometry. Slice as `base..base + len`, the lens are
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
pub const ASSEMBLED_LEN: usize = 0xBDC82;
/// Assembled (pre-convsym) ROM length, `__DEBUG__` shape. tests: m1d_rom, m1d_debug_rom, mixed_dac_rom
pub const DEBUG_ASSEMBLED_LEN: usize = 0xC055E;

// ── Regions (manifest order) ──

/// `Vectors` .. start + 0x100 plain / 0x100 debug (literal, no end symbol), gate `SIGIL_EMP_VECTORS`. tests: vectors_port
pub const VECTORS: Region = Region { plain_base: 0x0, debug_base: 0x0, plain_len: 0x100, debug_len: 0x100 };

/// `GameHeader` .. `section:header`. tests: header_port
pub const HEADER: Region = Region { plain_base: 0x100, debug_base: 0x100, plain_len: 0x100, debug_len: 0x100 };

/// `HeightMaps` .. `section:collision_data`. tests: collision_data_port
pub const COLLISION_DATA: Region = Region { plain_base: 0x6E1D6, debug_base: 0x6EB60, plain_len: 0x1D304, debug_len: 0x1D304 };

/// `EntryPoint` .. `section:boot`, gate `SIGIL_EMP_BOOT`. tests: boot_port
pub const BOOT: Region = Region { plain_base: 0x200, debug_base: 0x200, plain_len: 0x198, debug_len: 0x1A0 };

/// `BootData` .. `section:boot_head`. tests: boot_data_port
pub const BOOT_HEAD: Region = Region { plain_base: 0x398, debug_base: 0x3A0, plain_len: 0x184A, debug_len: 0x18CC };

/// `BootData_PostBlob` .. `section:boot_tail`. tests: boot_data_port
pub const BOOT_TAIL: Region = Region { plain_base: 0x1BE2, debug_base: 0x1C6C, plain_len: 0xE, debug_len: 0xE };

/// `VDP_Shadow_Init` .. `section:vdp_init`, gate `SIGIL_EMP_VDP_INIT`. tests: vdp_init_port
pub const VDP_INIT: Region = Region { plain_base: 0x1BF0, debug_base: 0x1C7A, plain_len: 0x3A, debug_len: 0x90 };

/// `Init_DMA_Queue` .. `section:dma_queue`, gate `SIGIL_EMP_DMA_QUEUE`. tests: dma_queue_port
pub const DMA_QUEUE: Region = Region { plain_base: 0x1C2A, debug_base: 0x1D0A, plain_len: 0x32C, debug_len: 0x346 };

/// `Init_SpriteTable` .. `section:buffers`, gate `SIGIL_EMP_BUFFERS`. tests: buffers_port
pub const BUFFERS: Region = Region { plain_base: 0x1F56, debug_base: 0x2050, plain_len: 0x2D8, debug_len: 0x2D8 };

/// `VBlank_Handler` .. `section:vblank`, gate `SIGIL_EMP_VBLANK`. tests: vblank_port
pub const VBLANK: Region = Region { plain_base: 0x222E, debug_base: 0x2328, plain_len: 0x1DC, debug_len: 0x20C };

/// `HBlank_Install` .. `section:hblank`, gate `SIGIL_EMP_HBLANK`. tests: hblank_port, m1c_vector_table
pub const HBLANK: Region = Region { plain_base: 0x240E, debug_base: 0x2538, plain_len: 0x30, debug_len: 0x30 };

/// `Read_Controllers` .. `section:controllers`, gate `SIGIL_EMP_CONTROLLERS`. tests: controllers_port
pub const CONTROLLERS: Region = Region { plain_base: 0x243E, debug_base: 0x2568, plain_len: 0x10E, debug_len: 0x10E };

/// `GameLoop` .. `section:game_loop`, gate `SIGIL_EMP_GAME_LOOP`. tests: game_loop_port, load_art_port
pub const GAME_LOOP: Region = Region { plain_base: 0x254C, debug_base: 0x2676, plain_len: 0x1C, debug_len: 0x1E };

/// `Input_Tick` .. `section:replay`. tests: game_loop_port
pub const REPLAY: Region = Region { plain_base: 0x256E, debug_base: 0x269A, plain_len: 0x146, debug_len: 0x1F6 };

/// `S4LZ_DecompressDict` .. `section:s4lz`, gate `SIGIL_EMP_S4LZ`. tests: s4lz_port
pub const S4LZ: Region = Region { plain_base: 0x26B6, debug_base: 0x2892, plain_len: 0xF8, debug_len: 0x200 };

/// `ZX0R_Decompress` .. `section:zx0_resume`.
pub const ZX0_RESUME: Region = Region { plain_base: 0x27AE, debug_base: 0x2A92, plain_len: 0x78, debug_len: 0x78 };

/// `GetSineCosine` .. `section:math`, gate `SIGIL_EMP_MATH`. tests: math_port, parallax_port
pub const MATH: Region = Region { plain_base: 0x2826, debug_base: 0x2B0A, plain_len: 0x3F6, debug_len: 0x3F6 };

/// `Perform_DPLC` .. `section:dplc`, gate `SIGIL_EMP_DPLC`. tests: dplc_port
pub const DPLC: Region = Region { plain_base: 0x2C1C, debug_base: 0x2F00, plain_len: 0xA4, debug_len: 0xA4 };

/// `InitObjectRAM` .. `section:core`, gate `SIGIL_EMP_CORE`. tests: core_port
pub const CORE: Region = Region { plain_base: 0x2CC0, debug_base: 0x2FA4, plain_len: 0x2F8, debug_len: 0x740 };

/// `InitSpriteSystem` .. `section:sprites`, gate `SIGIL_EMP_SPRITES`. tests: sprites_port
pub const SPRITES: Region = Region { plain_base: 0x2FB8, debug_base: 0x36E4, plain_len: 0x41A, debug_len: 0x534 };

/// `AnimateSprite` .. `section:animate`, gate `SIGIL_EMP_ANIMATE`. tests: animate_port, test_objects_port
pub const ANIMATE: Region = Region { plain_base: 0x33D2, debug_base: 0x3C18, plain_len: 0x194, debug_len: 0x2B8 };

/// `TouchResponse` .. `section:collision`, gate `SIGIL_EMP_COLLISION`. tests: collision_port
pub const COLLISION: Region = Region { plain_base: 0x3566, debug_base: 0x3ED0, plain_len: 0x200, debug_len: 0x208 };

/// `RingBuffer_Add` .. `section:rings`, gate `SIGIL_EMP_RINGS`. tests: rings_port
pub const RINGS: Region = Region { plain_base: 0x3766, debug_base: 0x40D8, plain_len: 0x1BE, debug_len: 0x224 };

/// `Collected_Init` .. `section:entity_window`, gate `SIGIL_EMP_ENTITY_WINDOW`. tests: entity_window_port
pub const ENTITY_WINDOW: Region = Region { plain_base: 0x3924, debug_base: 0x42FC, plain_len: 0x8EE, debug_len: 0xD5C };

/// `PopulateSpawnedPieceCount` .. `section:children`, gate `SIGIL_EMP_CHILDREN`. tests: children_port
pub const CHILDREN: Region = Region { plain_base: 0x4212, debug_base: 0x5058, plain_len: 0x2EC, debug_len: 0x39C };

/// `Load_Object` .. `section:load_object`, gate `SIGIL_EMP_LOAD_OBJECT`. tests: load_object_port, entity_window_port
pub const LOAD_OBJECT: Region = Region { plain_base: 0x44FE, debug_base: 0x53F4, plain_len: 0x88, debug_len: 0x88 };

/// `Plane_Buffer_Reset` .. `section:plane_buffer`, gate `SIGIL_EMP_PLANE_BUFFER`. tests: plane_buffer_port
pub const PLANE_BUFFER: Region = Region { plain_base: 0x4586, debug_base: 0x547C, plain_len: 0x31A, debug_len: 0x4B6 };

/// `Tile_Cache_GetTile` .. `section:tile_cache`, gate `SIGIL_EMP_TILE_CACHE`. tests: tile_cache_port
pub const TILE_CACHE: Region = Region { plain_base: 0x48A0, debug_base: 0x5932, plain_len: 0xE86, debug_len: 0x10EC };

/// `Collision_GetType` .. `section:collision_lookup`, gate `SIGIL_EMP_COLLISION_LOOKUP`. tests: collision_lookup_port
pub const COLLISION_LOOKUP: Region = Region { plain_base: 0x572E, debug_base: 0x6A28, plain_len: 0x68, debug_len: 0x68 };

/// `Section_Init` .. `section:section`, gate `SIGIL_EMP_SECTION`. tests: section_port
pub const SECTION: Region = Region { plain_base: 0x5C8A, debug_base: 0x6F84, plain_len: 0x460, debug_len: 0x890 };

/// `Camera_Init` .. `section:camera`, gate `SIGIL_EMP_CAMERA`. tests: camera_port
pub const CAMERA: Region = Region { plain_base: 0x60EA, debug_base: 0x7814, plain_len: 0x1C8, debug_len: 0x1D2 };

/// `Parallax_Init` .. `section:parallax`, gate `SIGIL_EMP_PARALLAX`. tests: parallax_port
pub const PARALLAX: Region = Region { plain_base: 0x62B2, debug_base: 0x79E6, plain_len: 0xA7C, debug_len: 0xB10 };

/// `Raster_Install` .. `section:raster`, gate `SIGIL_EMP_RASTER`. tests: raster_port
pub const RASTER: Region = Region { plain_base: 0x6D32, debug_base: 0x84FA, plain_len: 0x3EC, debug_len: 0x3EC };

/// `Palette_LoadPal` .. `Effects_InstallPreset`, gate `SIGIL_EMP_PALETTE`. tests: palette_port
pub const PALETTE: Region = Region { plain_base: 0x711E, debug_base: 0x88E6, plain_len: 0x4AE, debug_len: 0x4AE };

/// `Effects_InstallPreset` .. `section:preset`.
pub const PRESET: Region = Region { plain_base: 0x75CC, debug_base: 0x8D94, plain_len: 0xA8, debug_len: 0xAA };

/// `Level_LoadArt` .. `section:load_art`, gate `SIGIL_EMP_LOAD_ART`. tests: load_art_port
pub const LOAD_ART: Region = Region { plain_base: 0x7686, debug_base: 0x8E4E, plain_len: 0xB6, debug_len: 0xB6 };

/// `PageIn_Process` .. `section:page_in`.
pub const PAGE_IN: Region = Region { plain_base: 0x773E, debug_base: 0x8F06, plain_len: 0x2DE, debug_len: 0x452 };

/// `PageCache_Init` .. `section:page_cache`.
pub const PAGE_CACHE: Region = Region { plain_base: 0x7A2A, debug_base: 0x9362, plain_len: 0x4E4, debug_len: 0xE78 };

/// `BG_Init` .. `section:bg`, gate `SIGIL_EMP_BG`. tests: bg_port
pub const BG: Region = Region { plain_base: 0x7F10, debug_base: 0xA1DA, plain_len: 0xD4, debug_len: 0x134 };

/// `BgAnim_Init` .. start + 0x9E plain / 0x170 debug (literal, no end symbol), gate `SIGIL_EMP_BG_ANIM`. tests: bg_anim_port
pub const BG_ANIM: Region = Region { plain_base: 0x7FE4, debug_base: 0xA30E, plain_len: 0x9E, debug_len: 0x170 };

/// `CompressionSelfTest` .. `section:compression_selftest` (debug-only region; plain empty at `Sound_PostByte`), gate `SIGIL_EMP_COMPRESSION_SELFTEST`. tests: compression_selftest_port
pub const COMPRESSION_SELFTEST: Region = Region { plain_base: 0x8082, debug_base: 0xA47E, plain_len: 0x0, debug_len: 0xDE0 };

/// `Sound_PostByte` .. start + 0x2A8 plain / 0x452 debug (literal, no end symbol), gate `SIGIL_EMP_SOUND_API`. tests: sound_api_port
pub const SOUND_API: Region = Region { plain_base: 0x8082, debug_base: 0xB260, plain_len: 0x2A8, debug_len: 0x452 };

/// `TestSolid_Init` .. `section:test_solid`, gate `SIGIL_EMP_TEST_OBJECTS`. tests: test_objects_port
pub const TEST_SOLID: Region = Region { plain_base: 0x12334, debug_base: 0x1283A, plain_len: 0x12, debug_len: 0x12 };

/// `TestParticle` .. `section:test_particle` (debug-only region; plain empty at `ObjDef_PathSwap`), gate `SIGIL_EMP_TEST_OBJECTS`. tests: test_objects_port
pub const TEST_PARTICLE: Region = Region { plain_base: 0x12346, debug_base: 0x1284C, plain_len: 0x0, debug_len: 0x58 };

/// `TestStatic_Main` .. `section:test_static`, gate `SIGIL_EMP_TEST_STATIC`. tests: test_g1_objects_port
pub const TEST_STATIC: Region = Region { plain_base: 0x12330, debug_base: 0x124FA, plain_len: 0x4, debug_len: 0x4 };

/// `TestAnimated` .. `section:test_animated` (debug-only region; plain empty at `TestSolid_Init`), gate `SIGIL_EMP_TEST_ANIMATED`. tests: test_g1_objects_port
pub const TEST_ANIMATED: Region = Region { plain_base: 0x12334, debug_base: 0x124FE, plain_len: 0x0, debug_len: 0x60 };

/// `TestEmitter` .. `section:test_emitter` (debug-only region; plain empty at `ObjDef_PathSwap`), gate `SIGIL_EMP_TEST_EMITTER`. tests: test_g2_objects_port
pub const TEST_EMITTER: Region = Region { plain_base: 0x12346, debug_base: 0x128A4, plain_len: 0x0, debug_len: 0x5E };

/// `TestStressEmitter` .. `section:test_stress_emitter` (debug-only region; plain empty at `ObjDef_PathSwap`), gate `SIGIL_EMP_TEST_STRESS_EMITTER`. tests: test_g2_objects_port
pub const TEST_STRESS_EMITTER: Region = Region { plain_base: 0x12346, debug_base: 0x12A38, plain_len: 0x0, debug_len: 0x5E };

/// `TestChurnObj` .. `section:test_churn` (debug-only region; plain empty at `ObjDef_PathSwap`), gate `SIGIL_EMP_TEST_CHURN`. tests: test_g2_objects_port
pub const TEST_CHURN: Region = Region { plain_base: 0x12346, debug_base: 0x12A96, plain_len: 0x0, debug_len: 0x7C };

/// `TestChildPart` .. `section:test_parent` (debug-only region; plain empty at `ObjDef_PathSwap`), gate `SIGIL_EMP_TEST_PARENT`. tests: test_g3_objects_port
pub const TEST_PARENT: Region = Region { plain_base: 0x12346, debug_base: 0x12902, plain_len: 0x0, debug_len: 0x136 };

/// `TestPlayer` .. `section:test_player` (debug-only region; plain empty at `TestSolid_Init`), gate `SIGIL_EMP_TEST_PLAYER`. tests: test_g4_final_objects_port
pub const TEST_PLAYER: Region = Region { plain_base: 0x12334, debug_base: 0x1255E, plain_len: 0x0, debug_len: 0x294 };

/// `TestEnemy_Init` .. `section:test_enemy` (debug-only region; plain empty at `TestSolid_Init`), gate `SIGIL_EMP_TEST_ENEMY`. tests: test_g4_final_objects_port
pub const TEST_ENEMY: Region = Region { plain_base: 0x12334, debug_base: 0x127F2, plain_len: 0x0, debug_len: 0x48 };

/// `ObjDef_PathSwap` .. `section:path_swap`, gate `SIGIL_EMP_PATH_SWAP`. tests: test_g4_final_objects_port
pub const PATH_SWAP: Region = Region { plain_base: 0x12346, debug_base: 0x12B12, plain_len: 0x92, debug_len: 0xFA };

/// `OJZ_TestRaster` .. `section:ojz_effects`.
pub const OJZ_EFFECTS: Region = Region { plain_base: 0x138EE, debug_base: 0x14134, plain_len: 0x5B4, debug_len: 0x694 };

/// `DeformTable_Zero` .. `section:scene_registry`, gate `SIGIL_EMP_SCENE_REGISTRY`. tests: scene_registry_port
pub const SCENE_REGISTRY: Region = Region { plain_base: 0x123D8, debug_base: 0x12C0C, plain_len: 0x11C8, debug_len: 0x11C8 };

/// `Map_TestObj` .. `section:test_mappings`, gate `SIGIL_EMP_TEST_MAPPINGS`. tests: test_mappings_port
pub const TEST_MAPPINGS: Region = Region { plain_base: 0x29B60, debug_base: 0x2A4E2, plain_len: 0x30, debug_len: 0x30 };

/// `Map_DustSpindash` .. `section:dust_data`, gate `SIGIL_EMP_DUST_DATA`.
pub const DUST_DATA: Region = Region { plain_base: 0x29B90, debug_base: 0x2A512, plain_len: 0xBDA, debug_len: 0xBDA };

/// `Ani_Sonic` .. `section:sonic_anims`, gate `SIGIL_EMP_SONIC_ANIMS`. tests: sonic_anims_port
pub const SONIC_ANIMS: Region = Region { plain_base: 0x2A76A, debug_base: 0x2B0EC, plain_len: 0x10A, debug_len: 0x10A };

/// `Ani_Tails` .. `section:tails_anims`, gate `SIGIL_EMP_TAILS_ANIMS`. tests: sonic_anims_port
pub const TAILS_ANIMS: Region = Region { plain_base: 0x2A874, debug_base: 0x2B1F6, plain_len: 0x1BC, debug_len: 0x1BC };

/// `Ani_Knuckles` .. `section:knuckles_anims`, gate `SIGIL_EMP_KNUCKLES_ANIMS`. tests: sonic_anims_port
pub const KNUCKLES_ANIMS: Region = Region { plain_base: 0x2AA30, debug_base: 0x2B3B2, plain_len: 0x16C, debug_len: 0x16C };

/// `Map_Tails` .. `section:tails_data`, gate `SIGIL_EMP_TAILS_DATA`. tests: collision_data_port
pub const TAILS_DATA: Region = Region { plain_base: 0x2ABB0, debug_base: 0x2B53A, plain_len: 0x20F5E, debug_len: 0x20F5E };

/// `Map_Knuckles` .. `section:knuckles_data`, gate `SIGIL_EMP_KNUCKLES_DATA`. tests: collision_data_port
pub const KNUCKLES_DATA: Region = Region { plain_base: 0x4BB0E, debug_base: 0x4C498, plain_len: 0x226C8, debug_len: 0x226C8 };

/// `Ani_Particle` .. `section:particle_anims` (debug-only region; plain empty at `Ani_DustSpindash`), gate `SIGIL_EMP_PARTICLE_ANIMS`. tests: particle_anims_port, test_objects_port
pub const PARTICLE_ANIMS: Region = Region { plain_base: 0x2AB9C, debug_base: 0x2B51E, plain_len: 0x0, debug_len: 0x8 };

/// `Ani_DustSpindash` .. `section:dust_anims`, gate `SIGIL_EMP_DUST_ANIMS`.
pub const DUST_ANIMS: Region = Region { plain_base: 0x2AB9C, debug_base: 0x2B526, plain_len: 0x14, debug_len: 0x14 };

/// `OJZ_Sec0_TypeTable` .. `section:entity_data`. tests: ojz_run_a_port
pub const ENTITY_DATA: Region = Region { plain_base: 0x13ED6, debug_base: 0x147FC, plain_len: 0x170, debug_len: 0x170 };

/// `OJZ_Act_Pool_Page0` .. `section:ojz_act_pool`. tests: ojz_run_a_port
pub const OJZ_ACT_POOL: Region = Region { plain_base: 0x14046, debug_base: 0x1496C, plain_len: 0x2F0C, debug_len: 0x2F0C };

/// `OJZ_Act1_Descriptor` .. `section:act_descriptor`, gate `SIGIL_EMP_ACT_DESCRIPTOR`. tests: act_descriptor_port
pub const ACT_DESCRIPTOR: Region = Region { plain_base: 0x16F52, debug_base: 0x17878, plain_len: 0x15A, debug_len: 0x15A };

/// `OJZ_Sec0_Blocks` .. `section:sec_block_blobs`. tests: ojz_run_b_port
pub const SEC_BLOCK_BLOBS: Region = Region { plain_base: 0x170AC, debug_base: 0x179D2, plain_len: 0xB60A, debug_len: 0xB60A };

/// `OJZ_Sec0_LocalMap` .. `section:sec_local_maps`. tests: ojz_run_b_port
pub const SEC_LOCAL_MAPS: Region = Region { plain_base: 0x226B6, debug_base: 0x22FDC, plain_len: 0xBFA, debug_len: 0xBFA };

/// `OJZ_Palette` .. `section:ojz_act_assets`. tests: ojz_run_b_port
pub const OJZ_ACT_ASSETS: Region = Region { plain_base: 0x232B0, debug_base: 0x23BD6, plain_len: 0x4882, debug_len: 0x4882 };

/// `BgAnim_Table` .. `section:ojz_bg_anim`. tests: ojz_run_b_port
pub const OJZ_BG_ANIM: Region = Region { plain_base: 0x27B32, debug_base: 0x28458, plain_len: 0x202E, debug_len: 0x208A };

/// `ObjDef_Static` .. start + 0x34 plain / 0x34 debug (literal, no end symbol), gate `SIGIL_EMP_OBJDEFS`. tests: objdef_port
pub const OBJDEFS: Region = Region { plain_base: 0x13EA2, debug_base: 0x147C8, plain_len: 0x34, debug_len: 0x34 };

/// `GameState_ObjectTest_Init` .. `section:object_test_state` (debug-only region; plain empty at `GameState_OJZScroll_Init`), gate `SIGIL_EMP_OBJECT_TEST_STATE`. tests: test_t1_harness_states_port
pub const OBJECT_TEST_STATE: Region = Region { plain_base: 0xBC404, debug_base: 0xBDE54, plain_len: 0x0, debug_len: 0x384 };

/// `GameState_OJZScroll_Init` .. `section:ojz_scroll_test`, gate `SIGIL_EMP_OJZ_SCROLL_TEST`. tests: test_t1_harness_states_port
pub const OJZ_SCROLL_TEST: Region = Region { plain_base: 0xBC404, debug_base: 0xBE1D8, plain_len: 0x56A, debug_len: 0x1076 };

/// `Replay_OJZ_Fixture` .. `section:replay_fixture`.
pub const REPLAY_FIXTURE: Region = Region { plain_base: 0xBC972, debug_base: 0xBF24E, plain_len: 0x260, debug_len: 0x260 };

/// `BusError` .. `section:error_handler`, gate `SIGIL_EMP_ERROR_HANDLER`. tests: error_handler_port
pub const ERROR_HANDLER: Region = Region { plain_base: 0xBCBD2, debug_base: 0xBF4AE, plain_len: 0x10B0, debug_len: 0x10B0 };

/// `Dac_Temp_Blip` .. start + 0xF8BC plain / 0xF8BC debug (literal, no end symbol), gate `SIGIL_EMP_DAC`. tests: dac_bank_port
pub const DAC_BANKS: Region = Region { plain_base: 0xA8000, debug_base: 0xA8000, plain_len: 0xF8BC, debug_len: 0xF8BC };

/// `Song_MovingTrucks` .. start + 0x34E8 plain / 0x4F38 debug (literal, no end symbol), gate `SIGIL_EMP_MT`. tests: mt_bank_port
pub const MT_BANK_BLOB: Region = Region { plain_base: 0xB8630, debug_base: 0xB8630, plain_len: 0x34E8, debug_len: 0x4F38 };

/// `Sfx_33` .. `section:sfx_bank_blob`, gate `SIGIL_EMP_SFX`. tests: sfx_bank_port
pub const SFX_BANK_BLOB: Region = Region { plain_base: 0xBBB18, debug_base: 0xBD568, plain_len: 0x8EC, debug_len: 0x8EC };

/// `SoundTablesZ80_Head` .. start + 0x630 plain / 0x630 debug (literal, no end symbol), gate `SIGIL_EMP_SOUNDBANKHEAD`. tests: soundbankhead_port
pub const SOUNDBANKHEAD: Region = Region { plain_base: 0xB8000, debug_base: 0xB8000, plain_len: 0x630, debug_len: 0x630 };

/// `EndOfRom` .. start + 0x0 plain / 0x0 debug (literal, no end symbol), gate `SIGIL_EMP_EPILOGUE`. tests: m1d_rom, m1d_debug_rom
pub const EPILOGUE: Region = Region { plain_base: 0xBDC82, debug_base: 0xC055E, plain_len: 0x0, debug_len: 0x0 };

/// `ObjCodeBase` .. start + 0x2 plain / 0x2 debug (literal, no end symbol), gate `SIGIL_EMP_OBJCODEBASE`. tests: m1d_rom, m1d_debug_rom
pub const OBJCODEBASE: Region = Region { plain_base: 0x10000, debug_base: 0x10000, plain_len: 0x2, debug_len: 0x2 };

/// `Player_Init` .. `section:player_common`, gate `SIGIL_EMP_PLAYER_COMMON`. tests: test_p1_player_port
pub const PLAYER_COMMON: Region = Region { plain_base: 0x10002, debug_base: 0x10002, plain_len: 0x6CE, debug_len: 0x7E4 };

/// `CharDef_Sonic` .. `section:sonic`, gate `SIGIL_EMP_SONIC`. tests: test_p1_player_port
pub const SONIC: Region = Region { plain_base: 0x11EC2, debug_base: 0x11FCE, plain_len: 0x36, debug_len: 0x36 };

/// `CharDef_Tails` .. `section:tails`, gate `SIGIL_EMP_TAILS`. tests: test_p1_player_port
pub const TAILS: Region = Region { plain_base: 0x11EF8, debug_base: 0x12004, plain_len: 0x36, debug_len: 0x36 };

/// `CharDef_Knuckles` .. `section:knuckles`, gate `SIGIL_EMP_KNUCKLES`. tests: test_p1_player_port
pub const KNUCKLES: Region = Region { plain_base: 0x11F2E, debug_base: 0x1203A, plain_len: 0x36, debug_len: 0x36 };

/// `CharacterDefs` .. `section:characters`, gate `SIGIL_EMP_CHARACTERS`. tests: test_p1_player_port
pub const CHARACTERS: Region = Region { plain_base: 0x11F64, debug_base: 0x12070, plain_len: 0x4A, debug_len: 0xB0 };

/// `TailsAppendage_Refresh` .. `section:tails_appendage`, gate `SIGIL_EMP_TAILS_APPENDAGE`. tests: test_p1_player_port
pub const TAILS_APPENDAGE: Region = Region { plain_base: 0x11FAE, debug_base: 0x12120, plain_len: 0x11C, debug_len: 0x174 };

/// `DustPuff_Spawn` .. `section:dust_puff`, gate `SIGIL_EMP_DUST_PUFF`.
pub const DUST_PUFF: Region = Region { plain_base: 0x120CA, debug_base: 0x12294, plain_len: 0x46, debug_len: 0x46 };

/// `Dust_Tick` .. `section:dust_spindash`, gate `SIGIL_EMP_DUST_SPINDASH`.
pub const DUST_SPINDASH: Region = Region { plain_base: 0x12110, debug_base: 0x122DA, plain_len: 0x102, debug_len: 0x102 };

/// `PState_Ground` .. `section:player_ground`, gate `SIGIL_EMP_PLAYER_GROUND`. tests: test_p2_player_states_port
pub const PLAYER_GROUND: Region = Region { plain_base: 0x106D4, debug_base: 0x107EA, plain_len: 0x490, debug_len: 0x48A };

/// `PState_Air` .. `section:player_air`, gate `SIGIL_EMP_PLAYER_AIR`. tests: test_p2_player_states_port
pub const PLAYER_AIR: Region = Region { plain_base: 0x10B64, debug_base: 0x10C74, plain_len: 0x34A, debug_len: 0x34A };

/// `PState_Spindash` .. `section:player_spindash`, gate `SIGIL_EMP_PLAYER_SPINDASH`. tests: test_p2_player_states_port
pub const PLAYER_SPINDASH: Region = Region { plain_base: 0x10EAE, debug_base: 0x10FBE, plain_len: 0xA0, debug_len: 0x9C };

/// `PState_Fly` .. `section:player_fly`, gate `SIGIL_EMP_PLAYER_FLY`. tests: test_p2_player_states_port
pub const PLAYER_FLY: Region = Region { plain_base: 0x10F4E, debug_base: 0x1105A, plain_len: 0x13A, debug_len: 0x138 };

/// `PState_Glide` .. `section:player_glide`, gate `SIGIL_EMP_PLAYER_GLIDE`. tests: test_p2_player_states_port
pub const PLAYER_GLIDE: Region = Region { plain_base: 0x11092, debug_base: 0x1119E, plain_len: 0x2BA, debug_len: 0x2B6 };

/// `Climb_WallDist` .. `CharDef_Sonic`, gate `SIGIL_EMP_PLAYER_CLIMB`. tests: test_p2_player_states_port
pub const PLAYER_CLIMB: Region = Region { plain_base: 0x11368, debug_base: 0x11474, plain_len: 0xB5A, debug_len: 0xB5A };

/// `Collision_ProbeDown` .. `section:player_sensors`, gate `SIGIL_EMP_PLAYER_SENSORS`. tests: test_p4_player_sensors_port
pub const PLAYER_SENSORS: Region = Region { plain_base: 0x5796, debug_base: 0x6A90, plain_len: 0x4F4, debug_len: 0x4F4 };

// ── Symbols (manifest order) ──

/// `OJZ_Preset_Sec0`. tests: act_descriptor_port
pub const OJZ_PRESET_SEC0: Pin = Pin { plain: 0x13C8E, debug: 0x14558 };

/// `OJZ_Preset_Sec1`. tests: act_descriptor_port
pub const OJZ_PRESET_SEC1: Pin = Pin { plain: 0x13CBC, debug: 0x14586 };

/// `OJZ_Preset_Sec2`. tests: act_descriptor_port
pub const OJZ_PRESET_SEC2: Pin = Pin { plain: 0x13CEA, debug: 0x145B4 };

/// `OJZ_Preset_Sec3`. tests: act_descriptor_port
pub const OJZ_PRESET_SEC3: Pin = Pin { plain: 0x13D18, debug: 0x145E2 };

/// `OJZ_Preset_Plain`. tests: act_descriptor_port
pub const OJZ_PRESET_PLAIN: Pin = Pin { plain: 0x13D46, debug: 0x14610 };

/// `OJZ_Preset_Depth`. tests: act_descriptor_port
pub const OJZ_PRESET_DEPTH: Pin = Pin { plain: 0x13D74, debug: 0x1463E };

/// `EditorSceneBinding_OJZ_Act1_Sec4`. tests: act_descriptor_port
pub const EDITOR_SCENE_BINDING_OJZ_ACT1_SEC4: Pin = Pin { plain: 0x1365E, debug: 0x13E92 };

/// `OJZ_Preset_Sec5`. tests: act_descriptor_port
pub const OJZ_PRESET_SEC5: Pin = Pin { plain: 0x13DA2, debug: 0x1466C };

/// `EditorRaster_OJZ_Act1_authored_probe`. tests: act_descriptor_port
pub const EDITOR_RASTER_OJZ_ACT1_AUTHORED_PROBE: Pin = Pin { plain: 0x137BC, debug: 0x14002 };

/// `EditorRaster_OJZ_Act1_ojz_sec5_showcase`. tests: act_descriptor_port
pub const EDITOR_RASTER_OJZ_ACT1_OJZ_SEC5_SHOWCASE: Pin = Pin { plain: 0x13838, debug: 0x1407E };

/// `EditorRaster_OJZ_Act1_ojz_sec3_shimmer`. tests: act_descriptor_port
pub const EDITOR_RASTER_OJZ_ACT1_OJZ_SEC3_SHIMMER: Pin = Pin { plain: 0x1380A, debug: 0x14050 };

/// `EditorCycle_OJZ_Act1_ojz_sec3_shimmer`.
pub const EDITOR_CYCLE_OJZ_ACT1_OJZ_SEC3_SHIMMER: Pin = Pin { plain: 0x138DE, debug: 0x14124 };

/// `CrossoverTable`. tests: act_descriptor_port
pub const CROSSOVER_TABLE: Pin = Pin { plain: 0x703D6, debug: 0x70D60 };

/// `Effects_InstallPreset`. tests: parallax_port
pub const EFFECTS_INSTALL_PRESET: Pin = Pin { plain: 0x75CC, debug: 0x8D94 };

/// `Raster_GetChannelBand`. tests: parallax_port
pub const RASTER_GET_CHANNEL_BAND: Pin = Pin { plain: 0x703A, debug: 0x8802 };

/// `TestStatic_Main`. tests: objdef_port
pub const TEST_STATIC_MAIN: Pin = Pin { plain: 0x12330, debug: 0x124FA };

/// `TestSolid_Init`. tests: objdef_port
pub const TEST_SOLID_INIT: Pin = Pin { plain: 0x12334, debug: 0x1283A };

/// `TestEnemy_Init`, debug-shape consumer only (`debug_only`). tests: objdef_port
pub const TEST_ENEMY_INIT: u32 = 0x127F2;

/// `TestParent`, debug-shape consumer only (`debug_only`). tests: objdef_port
pub const TEST_PARENT_LABEL: u32 = 0x1298C;

/// `Map_TestObj`. tests: objdef_port
pub const MAP_TEST_OBJ: Pin = Pin { plain: 0x29B60, debug: 0x2A4E2 };

/// `Map_Sonic`. tests: test_g1_objects_port
pub const MAP_SONIC: Pin = Pin { plain: 0x704D6, debug: 0x70E60 };

/// `DPLC_Sonic`. tests: test_g1_objects_port
pub const DPLC_SONIC: Pin = Pin { plain: 0x72156, debug: 0x72AE0 };

/// `Art_Sonic`. tests: test_g1_objects_port
pub const ART_SONIC: Pin = Pin { plain: 0x72A1A, debug: 0x733A4 };

/// `CreateEffect_Normal`. tests: test_g2_objects_port
pub const CREATE_EFFECT_NORMAL: Pin = Pin { plain: 0x4468, debug: 0x535E };

/// `CreateChild_Normal`. tests: test_g3_objects_port
pub const CREATE_CHILD_NORMAL: Pin = Pin { plain: 0x423E, debug: 0x5084 };

/// `DeleteChildren`. tests: test_g3_objects_port
pub const DELETE_CHILDREN: Pin = Pin { plain: 0x444A, debug: 0x5340 };

/// `GetSineCosine`. tests: test_g3_objects_port
pub const GET_SINE_COSINE: Pin = Pin { plain: 0x2826, debug: 0x2B0A };

/// `EntryPoint`. tests: m1c_vector_table
pub const ENTRY_POINT: Pin = Pin { plain: 0x200, debug: 0x200 };

/// `BusError`, debug-shape consumer only (`debug_only`). tests: vectors_port
pub const BUS_ERROR: u32 = 0xBF4AE;

/// `AddressError`, debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ADDRESS_ERROR: u32 = 0xBF4C6;

/// `IllegalInstr`, debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ILLEGAL_INSTR: u32 = 0xBF4E2;

/// `ZeroDivide`, debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ZERO_DIVIDE: u32 = 0xBF504;

/// `ChkInstr`, debug-shape consumer only (`debug_only`). tests: vectors_port
pub const CHK_INSTR: u32 = 0xBF51E;

/// `TrapvInstr`, debug-shape consumer only (`debug_only`). tests: vectors_port
pub const TRAPV_INSTR: u32 = 0xBF53C;

/// `PrivilegeViol`, debug-shape consumer only (`debug_only`). tests: vectors_port
pub const PRIVILEGE_VIOL: u32 = 0xBF55C;

/// `Trace`, debug-shape consumer only (`debug_only`). tests: vectors_port
pub const TRACE: u32 = 0xBF57E;

/// `Line1010Emu`, debug-shape consumer only (`debug_only`). tests: vectors_port
pub const LINE1010_EMU: u32 = 0xBF592;

/// `Line1111Emu`, debug-shape consumer only (`debug_only`). tests: vectors_port
pub const LINE1111_EMU: u32 = 0xBF5B2;

/// `ErrorExcept`, debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ERROR_EXCEPT: u32 = 0xBF5D2;

/// `ErrorTrap`, debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ERROR_TRAP: u32 = 0xBF5F0;

/// `VBlank_Handler`. tests: m1c_vector_table
pub const V_BLANK_HANDLER: Pin = Pin { plain: 0x222E, debug: 0x2328 };

/// `HBlank_Vector_Slot`. tests: hblank_port, m1c_vector_table
pub const H_BLANK_VECTOR_SLOT: Pin = Pin { plain: 0xFFFFB60E, debug: 0xFFFFB69C };

/// `VDP_Shadow_Table`. tests: vdp_init_port
pub const VDP_SHADOW_TABLE: Pin = Pin { plain: 0xFFFF800E, debug: 0xFFFF800E };

/// `BootData_VDPRegs`. tests: vdp_init_port
pub const BOOT_DATA_VDP_REGS: Pin = Pin { plain: 0x3B2, debug: 0x3BA };

/// `Ctrl_1_Held`. tests: controllers_port
pub const CTRL_1_HELD: Pin = Pin { plain: 0xFFFF8028, debug: 0xFFFF8028 };

/// `Ctrl_1_Held_Raw`. tests: controllers_port
pub const CTRL_1_HELD_RAW: Pin = Pin { plain: 0xFFFFB9FC, debug: 0xFFFFBA8A };

/// `Ctrl_2_Held`. tests: vblank_port
pub const CTRL_2_HELD: Pin = Pin { plain: 0xFFFF802A, debug: 0xFFFF802A };

/// `Ctrl_1_Ext_Held`. tests: vblank_port
pub const CTRL_1_EXT_HELD: Pin = Pin { plain: 0xFFFF802E, debug: 0xFFFF802E };

/// `Ctrl_2_Ext_Held`. tests: vblank_port
pub const CTRL_2_EXT_HELD: Pin = Pin { plain: 0xFFFF8030, debug: 0xFFFF8030 };

/// `Ctrl_2_Held_Raw`. tests: vblank_port
pub const CTRL_2_HELD_RAW: Pin = Pin { plain: 0xFFFFB9FD, debug: 0xFFFFBA8B };

/// `Ctrl_1_Ext_Held_Raw`. tests: vblank_port
pub const CTRL_1_EXT_HELD_RAW: Pin = Pin { plain: 0xFFFFB9FE, debug: 0xFFFFBA8C };

/// `Ctrl_2_Ext_Held_Raw`. tests: vblank_port
pub const CTRL_2_EXT_HELD_RAW: Pin = Pin { plain: 0xFFFFB9FF, debug: 0xFFFFBA8D };

/// `VSync_Wait`. tests: game_loop_port, load_art_port
pub const V_SYNC_WAIT: Pin = Pin { plain: 0x23E4, debug: 0x250A };

/// `Sound_DrainSfxRing`. tests: game_loop_port, load_art_port
pub const SOUND_DRAIN_SFX_RING: Pin = Pin { plain: 0x81EE, debug: 0xB576 };

/// `Game_State`. tests: game_loop_port, load_art_port
pub const GAME_STATE: Pin = Pin { plain: 0xFFFF8008, debug: 0xFFFF8008 };

/// `Input_Tick`. tests: game_loop_port, game_debug_port
pub const INPUT_TICK: Pin = Pin { plain: 0x256E, debug: 0x269A };

/// `Cache_Left_Col`. tests: collision_lookup_port, section_port
pub const CACHE_LEFT_COL: Pin = Pin { plain: 0xFFFFADBC, debug: 0xFFFFAE4A };

/// `Draw_TileColumn`. tests: section_port
pub const DRAW_TILE_COLUMN: Pin = Pin { plain: 0x458E, debug: 0x5484 };

/// `Draw_TileRow_FromCache`. tests: section_port
pub const DRAW_TILE_ROW_FROM_CACHE: Pin = Pin { plain: 0x46E2, debug: 0x5656 };

/// `EntityWindow_Init`. tests: section_port
pub const ENTITY_WINDOW_INIT: Pin = Pin { plain: 0x3CE2, debug: 0x4A38 };

/// `Section_Plane_Dirty`. tests: section_port
pub const SECTION_PLANE_DIRTY: Pin = Pin { plain: 0xFFFFAE30, debug: 0xFFFFAEBE };

/// `Section_Right_Col_Written`. tests: section_port
pub const SECTION_RIGHT_COL_WRITTEN: Pin = Pin { plain: 0xFFFFAE32, debug: 0xFFFFAEC0 };

/// `Section_Left_Col_Written`. tests: section_port
pub const SECTION_LEFT_COL_WRITTEN: Pin = Pin { plain: 0xFFFFAE34, debug: 0xFFFFAEC2 };

/// `Section_Top_Row_Written`. tests: section_port
pub const SECTION_TOP_ROW_WRITTEN: Pin = Pin { plain: 0xFFFFAE2C, debug: 0xFFFFAEBA };

/// `Section_Bottom_Row_Written`. tests: section_port
pub const SECTION_BOTTOM_ROW_WRITTEN: Pin = Pin { plain: 0xFFFFAE2E, debug: 0xFFFFAEBC };

/// `Cache_Head_Col`. tests: section_port
pub const CACHE_HEAD_COL: Pin = Pin { plain: 0xFFFFADBE, debug: 0xFFFFAE4C };

/// `Cache_Top_Row`. tests: section_port
pub const CACHE_TOP_ROW: Pin = Pin { plain: 0xFFFFADC0, debug: 0xFFFFAE4E };

/// `Cache_Bottom_Row`. tests: section_port
pub const CACHE_BOTTOM_ROW: Pin = Pin { plain: 0xFFFFADC2, debug: 0xFFFFAE50 };

/// `Cache_Origin_Col`. tests: section_port
pub const CACHE_ORIGIN_COL: Pin = Pin { plain: 0xFFFFADC4, debug: 0xFFFFAE52 };

/// `Cache_Origin_Row`. tests: section_port
pub const CACHE_ORIGIN_ROW: Pin = Pin { plain: 0xFFFFADC6, debug: 0xFFFFAE54 };

/// `Plane_Buffer_Ptr`. tests: section_port
pub const PLANE_BUFFER_PTR: Pin = Pin { plain: 0xFFFFACA8, debug: 0xFFFFAD36 };

/// `Plane_Buffer`. tests: plane_buffer_port
pub const PLANE_BUFFER_BASE: Pin = Pin { plain: 0xFFFFA6A8, debug: 0xFFFFA736 };

/// `Tile_Cache_Nametable`. tests: section_port
pub const TILE_CACHE_NAMETABLE: Pin = Pin { plain: 0xFFFF0000, debug: 0xFFFF0000 };

/// `Tile_Cache_Collision`. tests: tile_cache_port, collision_lookup_port
pub const TILE_CACHE_COLLISION: Pin = Pin { plain: 0xFFFF2580, debug: 0xFFFF2580 };

/// `Frame_Counter`. tests: tile_cache_port
pub const FRAME_COUNTER: Pin = Pin { plain: 0xFFFF8002, debug: 0xFFFF8002 };

/// `Logic_Tick`. tests: game_loop_port, bg_anim_port, tile_cache_port, parallax_port
pub const LOGIC_TICK: Pin = Pin { plain: 0xFFFF8004, debug: 0xFFFF8004 };

/// `Block_Stage_Keys`. tests: tile_cache_port
pub const BLOCK_STAGE_KEYS: Pin = Pin { plain: 0xFFFFADEA, debug: 0xFFFFAE78 };

/// `Block_Stage_Next`. tests: tile_cache_port
pub const BLOCK_STAGE_NEXT: Pin = Pin { plain: 0xFFFFAE2A, debug: 0xFFFFAEB8 };

/// `Block_Stage_Bucket`. tests: tile_cache_port
pub const BLOCK_STAGE_BUCKET: Pin = Pin { plain: 0xFFFF6842, debug: 0xFFFF6842 };

/// `Block_Stage_Chain`. tests: tile_cache_port
pub const BLOCK_STAGE_CHAIN: Pin = Pin { plain: 0xFFFF6942, debug: 0xFFFF6942 };

/// `Block_Stage_Buffers`. tests: tile_cache_port
pub const BLOCK_STAGE_BUFFERS: Pin = Pin { plain: 0xFFFF3842, debug: 0xFFFF3842 };

/// `Block_Stage_Ptrs`. tests: tile_cache_port
pub const BLOCK_STAGE_PTRS: Pin = Pin { plain: 0xFFFFB614, debug: 0xFFFFB6A2 };

/// `Block_Stage_ZeroPage`. tests: tile_cache_port
pub const BLOCK_STAGE_ZERO_PAGE: Pin = Pin { plain: 0xFFFFB698, debug: 0xFFFFB726 };

/// `Cache_Fill_Last_Frame`. tests: tile_cache_port
pub const CACHE_FILL_LAST_FRAME: Pin = Pin { plain: 0xFFFFADC8, debug: 0xFFFFAE56 };

/// `Cache_Fill_Budget`. tests: tile_cache_port
pub const CACHE_FILL_BUDGET: Pin = Pin { plain: 0xFFFFADD2, debug: 0xFFFFAE60 };

/// `Cache_Fill_Resume_Col`. tests: tile_cache_port
pub const CACHE_FILL_RESUME_COL: Pin = Pin { plain: 0xFFFFADCA, debug: 0xFFFFAE58 };

/// `Cache_Fill_Resume_Row`. tests: tile_cache_port
pub const CACHE_FILL_RESUME_ROW: Pin = Pin { plain: 0xFFFFADCC, debug: 0xFFFFAE5A };

/// `Cache_Fill_RowResume_Row`. tests: tile_cache_port
pub const CACHE_FILL_ROW_RESUME_ROW: Pin = Pin { plain: 0xFFFFADD4, debug: 0xFFFFAE62 };

/// `Cache_Fill_RowResume_Col`. tests: tile_cache_port
pub const CACHE_FILL_ROW_RESUME_COL: Pin = Pin { plain: 0xFFFFADD6, debug: 0xFFFFAE64 };

/// `Cache_Fill_Rows_Left`. tests: tile_cache_port
pub const CACHE_FILL_ROWS_LEFT: Pin = Pin { plain: 0xFFFFADD8, debug: 0xFFFFAE66 };

/// `Cache_Prev_Cam_Row`. tests: tile_cache_port
pub const CACHE_PREV_CAM_ROW: Pin = Pin { plain: 0xFFFFADDA, debug: 0xFFFFAE68 };

/// `Cache_Prev_Cam_X`. tests: tile_cache_port
pub const CACHE_PREV_CAM_X: Pin = Pin { plain: 0xFFFFADDC, debug: 0xFFFFAE6A };

/// `Cache_H_Pfx_Dir`. tests: tile_cache_port
pub const CACHE_H_PFX_DIR: Pin = Pin { plain: 0xFFFFADDE, debug: 0xFFFFAE6C };

/// `Cache_H_Pfx_Accum`. tests: tile_cache_port
pub const CACHE_H_PFX_ACCUM: Pin = Pin { plain: 0xFFFFADE0, debug: 0xFFFFAE6E };

/// `Cache_Pfx_Row_Target`. tests: tile_cache_port
pub const CACHE_PFX_ROW_TARGET: Pin = Pin { plain: 0xFFFFADE2, debug: 0xFFFFAE70 };

/// `Cache_Pfx_Col_Target`. tests: tile_cache_port
pub const CACHE_PFX_COL_TARGET: Pin = Pin { plain: 0xFFFFADE4, debug: 0xFFFFAE72 };

/// `Cache_Pfx_Skip_Armed`. tests: tile_cache_port
pub const CACHE_PFX_SKIP_ARMED: Pin = Pin { plain: 0xFFFFADE6, debug: 0xFFFFAE74 };

/// `Cache_Pfx_Lag_Flag`. tests: tile_cache_port
pub const CACHE_PFX_LAG_FLAG: Pin = Pin { plain: 0xFFFFADE8, debug: 0xFFFFAE76 };

/// `Block_Stage_Gen`. tests: tile_cache_port
pub const BLOCK_STAGE_GEN: Pin = Pin { plain: 0xFFFFB5FC, debug: 0xFFFFB68A };

/// `Pfx_Memo_Row`. tests: tile_cache_port
pub const PFX_MEMO_ROW: Pin = Pin { plain: 0xFFFFB5FE, debug: 0xFFFFB68C };

/// `Pfx_Memo_L16`. tests: tile_cache_port
pub const PFX_MEMO_L16: Pin = Pin { plain: 0xFFFFB600, debug: 0xFFFFB68E };

/// `Pfx_Memo_H16`. tests: tile_cache_port
pub const PFX_MEMO_H16: Pin = Pin { plain: 0xFFFFB602, debug: 0xFFFFB690 };

/// `Pfx_Memo_Gen`. tests: tile_cache_port
pub const PFX_MEMO_GEN: Pin = Pin { plain: 0xFFFFB604, debug: 0xFFFFB692 };

/// `Cs_Memo_Col`. tests: tile_cache_port
pub const CS_MEMO_COL: Pin = Pin { plain: 0xFFFFB606, debug: 0xFFFFB694 };

/// `Cs_Memo_T16`. tests: tile_cache_port
pub const CS_MEMO_T16: Pin = Pin { plain: 0xFFFFB608, debug: 0xFFFFB696 };

/// `Cs_Memo_B16`. tests: tile_cache_port
pub const CS_MEMO_B16: Pin = Pin { plain: 0xFFFFB60A, debug: 0xFFFFB698 };

/// `Cs_Memo_Gen`. tests: tile_cache_port
pub const CS_MEMO_GEN: Pin = Pin { plain: 0xFFFFB60C, debug: 0xFFFFB69A };

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
pub const S4_LZ_DECOMPRESS_DICT: Pin = Pin { plain: 0x26B6, debug: 0x2892 };

/// `Player_1`. tests: collision_port, rings_port
pub const PLAYER_1: Pin = Pin { plain: 0xFFFF8F6C, debug: 0xFFFF8FFA };

/// `Cheat_Flags`. tests: test_g4_final_objects_port, test_p1_player_port
pub const CHEAT_FLAGS: Pin = Pin { plain: 0xFFFFBA80, debug: 0xFFFFE91E };

/// `Dynamic_Slots`. tests: collision_port
pub const DYNAMIC_SLOTS: Pin = Pin { plain: 0xFFFF900C, debug: 0xFFFF909A };

/// `Ring_Buffer`. tests: rings_port
pub const RING_BUFFER: Pin = Pin { plain: 0xFFFFAE9E, debug: 0xFFFFAF2C };

/// `Ring_Count`. tests: rings_port
pub const RING_COUNT: Pin = Pin { plain: 0xFFFFB19E, debug: 0xFFFFB22C };

/// `Ring_HighWater`. tests: rings_port
pub const RING_HIGH_WATER: Pin = Pin { plain: 0xFFFFB19F, debug: 0xFFFFB22D };

/// `Ring_Add_Dropped`. tests: rings_port
pub const RING_ADD_DROPPED: Pin = Pin { plain: 0xFFFFB1A0, debug: 0xFFFFB22E };

/// `Ring_Counter`. tests: rings_port
pub const RING_COUNTER: Pin = Pin { plain: 0xFFFFB20A, debug: 0xFFFFB298 };

/// `Ring_Anim_Frame`. tests: rings_port
pub const RING_ANIM_FRAME: Pin = Pin { plain: 0xFFFFB20C, debug: 0xFFFFB29A };

/// `Ring_Anim_Timer`. tests: rings_port
pub const RING_ANIM_TIMER: Pin = Pin { plain: 0xFFFFB20D, debug: 0xFFFFB29B };

/// `Camera_X`. tests: rings_port, section_port, camera_port, bg_anim_port
pub const CAMERA_X: Pin = Pin { plain: 0xFFFFA69A, debug: 0xFFFFA728 };

/// `Camera_Y`. tests: rings_port, section_port, camera_port, bg_anim_port
pub const CAMERA_Y: Pin = Pin { plain: 0xFFFFA69E, debug: 0xFFFFA72C };

/// `Camera_Target`. tests: camera_port, test_g4_final_objects_port, test_t1_harness_states_port
pub const CAMERA_TARGET: Pin = Pin { plain: 0xFFFFADB6, debug: 0xFFFFAE44 };

/// `Camera_Curl_Offset`. tests: camera_port, test_p1_player_port
pub const CAMERA_CURL_OFFSET: Pin = Pin { plain: 0xFFFFADB8, debug: 0xFFFFAE46 };

/// `Camera_Deadzone_Base`. tests: camera_port
pub const CAMERA_DEADZONE_BASE: Pin = Pin { plain: 0xFFFFADAC, debug: 0xFFFFAE3A };

/// `Camera_Pan_Offset`. tests: camera_port
pub const CAMERA_PAN_OFFSET: Pin = Pin { plain: 0xFFFFADB0, debug: 0xFFFFAE3E };

/// `Camera_Hold_Frames`. tests: camera_port
pub const CAMERA_HOLD_FRAMES: Pin = Pin { plain: 0xFFFFADBA, debug: 0xFFFFAE48 };

/// `Camera_Art_Hold`. tests: camera_port, tile_cache_port
pub const CAMERA_ART_HOLD: Pin = Pin { plain: 0xFFFFADBB, debug: 0xFFFFAE49 };

/// `Dbg_Cam_Clamp_Frames`, debug-shape consumer only (`debug_only`). tests: camera_port
pub const DBG_CAM_CLAMP_FRAMES: u32 = 0xFFFF8FA0;

/// `Camera_X_Max`. tests: camera_port
pub const CAMERA_X_MAX: Pin = Pin { plain: 0xFFFFADB2, debug: 0xFFFFAE40 };

/// `Camera_Y_Max`. tests: camera_port
pub const CAMERA_Y_MAX: Pin = Pin { plain: 0xFFFFADB4, debug: 0xFFFFAE42 };

/// `BgAnim_LastStep`. tests: bg_anim_port
pub const BG_ANIM_LAST_STEP: Pin = Pin { plain: 0xFFFF8F02, debug: 0xFFFF8F02 };

/// `BgAnim_Table`. tests: bg_anim_port
pub const BG_ANIM_TABLE: Pin = Pin { plain: 0x27B32, debug: 0x28458 };

/// `Camera_X_Biased`. tests: sprites_port
pub const CAMERA_X_BIASED: Pin = Pin { plain: 0xFFFFA6A2, debug: 0xFFFFA730 };

/// `Camera_Y_Biased`. tests: sprites_port
pub const CAMERA_Y_BIASED: Pin = Pin { plain: 0xFFFFA6A4, debug: 0xFFFFA732 };

/// `Collected_MarkRing`. tests: rings_port
pub const COLLECTED_MARK_RING: Pin = Pin { plain: 0x39A6, debug: 0x43E0 };

/// `EntityWindow_EntryForSection`. tests: rings_port
pub const ENTITY_WINDOW_ENTRY_FOR_SECTION: Pin = Pin { plain: 0x3BC2, debug: 0x48C2 };

/// `EntityLoaded_Clear`. tests: rings_port
pub const ENTITY_LOADED_CLEAR: Pin = Pin { plain: 0x3BAE, debug: 0x484C };

/// `Sound_PlayRing`. tests: rings_port
pub const SOUND_PLAY_RING: Pin = Pin { plain: 0x823E, debug: 0xB5C6 };

/// `MDDBG__ErrorHandler`, debug-shape consumer only (`debug_only`). tests: rings_port
pub const MDDBG_ERROR_HANDLER: u32 = 0xBF608;

/// `MDDBG__ErrorHandler_PagesController`, debug-shape consumer only (`debug_only`). tests: rings_port
pub const MDDBG_ERROR_HANDLER_PAGES_CONTROLLER: u32 = 0xC03CE;

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
pub const ACT_ART_BUDGET: Pin = Pin { plain: 0xFFFFB9F8, debug: 0xFFFFBA86 };

/// `Art_Budget_Remaining`. tests: load_art_port
pub const ART_BUDGET_REMAINING: Pin = Pin { plain: 0xFFFFB9FA, debug: 0xFFFFBA88 };

/// `PageIn_Pool_Pages`. tests: load_art_port
pub const PAGE_IN_POOL_PAGES: Pin = Pin { plain: 0xFFFFB9EC, debug: 0xFFFFBA7A };

/// `PageIn_Bulk_Drain`. tests: load_art_port
pub const PAGE_IN_BULK_DRAIN: Pin = Pin { plain: 0xFFFFB9E7, debug: 0xFFFFBA75 };

/// `PageIn_Fully_Resident`. tests: load_art_port
pub const PAGE_IN_FULLY_RESIDENT: Pin = Pin { plain: 0xFFFFB9EE, debug: 0xFFFFBA7C };

/// `Block_Stage_Maps`. tests: tile_cache_port
pub const BLOCK_STAGE_MAPS: Pin = Pin { plain: 0xFFFFB654, debug: 0xFFFFB6E2 };

/// `Cache_Cur_LocalMap`. tests: tile_cache_port
pub const CACHE_CUR_LOCAL_MAP: Pin = Pin { plain: 0xFFFFB694, debug: 0xFFFFB722 };

/// `PageCache_Direct_Map`. tests: load_art_port
pub const PAGE_CACHE_DIRECT_MAP: Pin = Pin { plain: 0xFFFFB9EF, debug: 0xFFFFBA7D };

/// `Page_Table`. tests: load_art_port
pub const PAGE_TABLE: Pin = Pin { plain: 0xFFFF699C, debug: 0xFFFF699C };

/// `Dbg_DMA_Enq_Capped`, debug-shape consumer only (`debug_only`). tests: bg_anim_port, dma_queue_port, dplc_port
pub const DBG_DMA_ENQ_CAPPED: u32 = 0xFFFF8F76;

/// `DMA_Overflow_Count`, debug-shape consumer only (`debug_only`). tests: dma_queue_port
pub const DMA_OVERFLOW_COUNT: u32 = 0xFFFF8F74;

/// `Art_Staging_Buffer`. tests: load_art_port
pub const ART_STAGING_BUFFER: Pin = Pin { plain: 0xFFFF6B34, debug: 0xFFFF6B34 };

/// `S4LZ_Decompress`. tests: load_art_port
pub const S4_LZ_DECOMPRESS: Pin = Pin { plain: 0x26BA, debug: 0x28EA };

/// `QueueDMA_Critical`. tests: load_art_port
pub const QUEUE_DMA_CRITICAL: Pin = Pin { plain: 0x1D48, debug: 0x1E28 };

/// `BG_Init`. tests: load_art_port
pub const BG_INIT: Pin = Pin { plain: 0x7F10, debug: 0xA1DA };

/// `QueueDMA_Important`. tests: dplc_port
pub const QUEUE_DMA_IMPORTANT: Pin = Pin { plain: 0x1D52, debug: 0x1E32 };

/// `QueueDMA_Deferrable`. tests: dplc_port
pub const QUEUE_DMA_DEFERRABLE: Pin = Pin { plain: 0x1D5C, debug: 0x1E3C };

/// `Object_RAM`. tests: core_port
pub const OBJECT_RAM: Pin = Pin { plain: 0xFFFF8F6C, debug: 0xFFFF8FFA };

/// `System_Slots`. tests: core_port
pub const SYSTEM_SLOTS: Pin = Pin { plain: 0xFFFF9C8C, debug: 0xFFFF9D1A };

/// `Effect_Slots`. tests: core_port
pub const EFFECT_SLOTS: Pin = Pin { plain: 0xFFFF9F0C, debug: 0xFFFF9F9A };

/// `Game_Paused`. tests: core_port
pub const GAME_PAUSED: Pin = Pin { plain: 0xFFFFA6A6, debug: 0xFFFFA734 };

/// `Object_RAM_End`. tests: core_port
pub const OBJECT_RAM_END: Pin = Pin { plain: 0xFFFFA40C, debug: 0xFFFFA49A };

/// `Dynamic_Free_Stack`. tests: core_port
pub const DYNAMIC_FREE_STACK: Pin = Pin { plain: 0xFFFFA40C, debug: 0xFFFFA49A };

/// `Dynamic_Free_SP`. tests: core_port
pub const DYNAMIC_FREE_SP: Pin = Pin { plain: 0xFFFFA45C, debug: 0xFFFFA4EA };

/// `Effect_Free_Stack`. tests: core_port
pub const EFFECT_FREE_STACK: Pin = Pin { plain: 0xFFFFA45E, debug: 0xFFFFA4EC };

/// `Effect_Free_SP`. tests: core_port
pub const EFFECT_FREE_SP: Pin = Pin { plain: 0xFFFFA47E, debug: 0xFFFFA50C };

/// `Dynamic_Live`. tests: core_port
pub const DYNAMIC_LIVE: Pin = Pin { plain: 0xFFFFB596, debug: 0xFFFFB624 };

/// `Dynamic_Live_Count`. tests: core_port
pub const DYNAMIC_LIVE_COUNT: Pin = Pin { plain: 0xFFFFB5E6, debug: 0xFFFFB674 };

/// `Dynamic_Live_Dirty`. tests: core_port
pub const DYNAMIC_LIVE_DIRTY: Pin = Pin { plain: 0xFFFFB5E8, debug: 0xFFFFB676 };

/// `Dynamic_Live_Walking`, debug-shape consumer only (`debug_only`). tests: core_port, collision_port, entity_window_port
pub const DYNAMIC_LIVE_WALKING: u32 = 0xFFFFB677;

/// `Dynamic_Live_Pending`. tests: core_port
pub const DYNAMIC_LIVE_PENDING: Pin = Pin { plain: 0xFFFFB5EA, debug: 0xFFFFB678 };

/// `Dynamic_Live_Pending_Count`. tests: core_port
pub const DYNAMIC_LIVE_PENDING_COUNT: Pin = Pin { plain: 0xFFFFB5FA, debug: 0xFFFFB688 };

/// `DeleteObject`. tests: animate_port, children_port
pub const DELETE_OBJECT: Pin = Pin { plain: 0x2D90, debug: 0x3074 };

/// `DrawRings`. tests: sprites_port
pub const DRAW_RINGS: Pin = Pin { plain: 0x37EC, debug: 0x41BA };

/// `Sprite_Table_Buffer`. tests: sprites_port
pub const SPRITE_TABLE_BUFFER: Pin = Pin { plain: 0xFFFF8298, debug: 0xFFFF8298 };

/// `Sprite_Table_Dirty`. tests: sprites_port
pub const SPRITE_TABLE_DIRTY: Pin = Pin { plain: 0xFFFF8518, debug: 0xFFFF8518 };

/// `Sprite_Emit_Active`. tests: sprites_port, buffers_port
pub const SPRITE_EMIT_ACTIVE: Pin = Pin { plain: 0xFFFF8519, debug: 0xFFFF8519 };

/// `Sprite_Bands`. tests: sprites_port
pub const SPRITE_BANDS: Pin = Pin { plain: 0xFFFFA480, debug: 0xFFFFA50E };

/// `Sprite_Band_Counts`. tests: sprites_port
pub const SPRITE_BAND_COUNTS: Pin = Pin { plain: 0xFFFFA680, debug: 0xFFFFA70E };

/// `Sprites_Rendered`. tests: sprites_port
pub const SPRITES_RENDERED: Pin = Pin { plain: 0xFFFFA688, debug: 0xFFFFA716 };

/// `Sprite_Cycle_Counter`. tests: sprites_port
pub const SPRITE_CYCLE_COUNTER: Pin = Pin { plain: 0xFFFFA68A, debug: 0xFFFFA718 };

/// `Sprite_Owner`, debug-shape consumer only (`debug_only`). tests: sprites_port
pub const SPRITE_OWNER: u32 = 0xFFFFE312;

/// `SpriteMask_Y`. tests: sprites_port
pub const SPRITE_MASK_Y: Pin = Pin { plain: 0xFFFFA68C, debug: 0xFFFFA71A };

/// `SpriteMask_Height`. tests: sprites_port
pub const SPRITE_MASK_HEIGHT: Pin = Pin { plain: 0xFFFFA68E, debug: 0xFFFFA71C };

/// `SpriteMask_After_Band`. tests: sprites_port
pub const SPRITE_MASK_AFTER_BAND: Pin = Pin { plain: 0xFFFFA690, debug: 0xFFFFA71E };

/// `Scanline_Band_Sprites`. tests: sprites_port
pub const SCANLINE_BAND_SPRITES: Pin = Pin { plain: 0xFFFFA692, debug: 0xFFFFA720 };

/// `Sound_PlaySFX`. tests: animate_port
pub const SOUND_PLAY_SFX: Pin = Pin { plain: 0x81A8, debug: 0xB4EA };

/// `ObjectMoveX`. tests: test_g4_final_objects_port
pub const OBJECT_MOVE_X: Pin = Pin { plain: 0x2F9C, debug: 0x36C8 };

/// `ObjCodeBase`. tests: test_objects_port
pub const OBJ_CODE_BASE: Pin = Pin { plain: 0x10000, debug: 0x10000 };

/// `Draw_Sprite`. tests: test_objects_port
pub const DRAW_SPRITE: Pin = Pin { plain: 0x2FCC, debug: 0x36F8 };

/// `ObjectMove`. tests: test_objects_port
pub const OBJECT_MOVE: Pin = Pin { plain: 0x2F82, debug: 0x36AE };

/// `Ring_Sfx_Speaker`. tests: sound_api_port
pub const RING_SFX_SPEAKER: Pin = Pin { plain: 0xFFFFB4DA, debug: 0xFFFFB568 };

/// `Sfx_Ring_Buf`. tests: sound_api_port
pub const SFX_RING_BUF: Pin = Pin { plain: 0xFFFFB4DC, debug: 0xFFFFB56A };

/// `Sfx_Ring_Wr`. tests: sound_api_port
pub const SFX_RING_WR: Pin = Pin { plain: 0xFFFFB4E4, debug: 0xFFFFB572 };

/// `Sfx_Ring_Rd`. tests: sound_api_port
pub const SFX_RING_RD: Pin = Pin { plain: 0xFFFFB4E5, debug: 0xFFFFB573 };

/// `SongTable`. tests: sound_api_port
pub const SONG_TABLE: Pin = Pin { plain: 0xBBB10, debug: 0xBD550 };

/// `SongPatchTable`. tests: sound_api_port
pub const SONG_PATCH_TABLE: Pin = Pin { plain: 0xBBB14, debug: 0xBD55C };

/// `OJZ_Palette`. tests: act_descriptor_port
pub const OJZ_PALETTE: Pin = Pin { plain: 0x232B0, debug: 0x23BD6 };

/// `OJZ_Act1_BG_Layout`. tests: act_descriptor_port
pub const OJZ_ACT1_BG_LAYOUT: Pin = Pin { plain: 0x23330, debug: 0x23C56 };

/// `OJZ_Act1_BG_Tiles`. tests: act_descriptor_port
pub const OJZ_ACT1_BG_TILES: Pin = Pin { plain: 0x25330, debug: 0x25C56 };

/// `ParallaxConfig_OJZ_Default`. tests: act_descriptor_port
pub const PARALLAX_CONFIG_OJZ_DEFAULT: Pin = Pin { plain: 0x124D8, debug: 0x12D0C };

/// `OJZ_Act_Pool_PageTable`. tests: act_descriptor_port
pub const OJZ_ACT_POOL_PAGE_TABLE: Pin = Pin { plain: 0x16F02, debug: 0x17828 };

/// `OJZ_Sec_LocalMaps`. tests: act_descriptor_port
pub const OJZ_SEC_LOCAL_MAPS: Pin = Pin { plain: 0x2328C, debug: 0x23BB2 };

/// `OJZ_Sec0_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC0_BLOCKS: Pin = Pin { plain: 0x170AC, debug: 0x179D2 };

/// `OJZ_Sec1_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC1_BLOCKS: Pin = Pin { plain: 0x1921C, debug: 0x19B42 };

/// `OJZ_Sec2_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC2_BLOCKS: Pin = Pin { plain: 0x1A598, debug: 0x1AEBE };

/// `OJZ_Sec3_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC3_BLOCKS: Pin = Pin { plain: 0x1BD30, debug: 0x1C656 };

/// `OJZ_Sec4_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC4_BLOCKS: Pin = Pin { plain: 0x1A598, debug: 0x1AEBE };

/// `OJZ_Sec5_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC5_BLOCKS: Pin = Pin { plain: 0x1CE7C, debug: 0x1D7A2 };

/// `OJZ_Sec6_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC6_BLOCKS: Pin = Pin { plain: 0x1DCA2, debug: 0x1E5C8 };

/// `OJZ_Sec7_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC7_BLOCKS: Pin = Pin { plain: 0x1F8A2, debug: 0x201C8 };

/// `OJZ_Sec8_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC8_BLOCKS: Pin = Pin { plain: 0x20B16, debug: 0x2143C };

/// `OJZ_Sec0_Objects`. tests: act_descriptor_port
pub const OJZ_SEC0_OBJECTS: Pin = Pin { plain: 0x13EDC, debug: 0x14802 };

/// `OJZ_Sec0_Rings`. tests: act_descriptor_port
pub const OJZ_SEC0_RINGS: Pin = Pin { plain: 0x13EE4, debug: 0x1480A };

/// `OJZ_Sec0_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC0_TYPE_TABLE: Pin = Pin { plain: 0x13ED6, debug: 0x147FC };

/// `OJZ_Sec1_Objects`. tests: act_descriptor_port
pub const OJZ_SEC1_OBJECTS: Pin = Pin { plain: 0x13F0E, debug: 0x14834 };

/// `OJZ_Sec1_Rings`. tests: act_descriptor_port
pub const OJZ_SEC1_RINGS: Pin = Pin { plain: 0x13F22, debug: 0x14848 };

/// `OJZ_Sec1_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC1_TYPE_TABLE: Pin = Pin { plain: 0x13F04, debug: 0x1482A };

/// `OJZ_Sec2_Objects`. tests: act_descriptor_port
pub const OJZ_SEC2_OBJECTS: Pin = Pin { plain: 0x13F54, debug: 0x1487A };

/// `OJZ_Sec2_Rings`. tests: act_descriptor_port
pub const OJZ_SEC2_RINGS: Pin = Pin { plain: 0x13F62, debug: 0x14888 };

/// `OJZ_Sec2_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC2_TYPE_TABLE: Pin = Pin { plain: 0x13F4A, debug: 0x14870 };

/// `OJZ_Sec3_Objects`. tests: act_descriptor_port
pub const OJZ_SEC3_OBJECTS: Pin = Pin { plain: 0x13F98, debug: 0x148BE };

/// `OJZ_Sec3_Rings`. tests: act_descriptor_port
pub const OJZ_SEC3_RINGS: Pin = Pin { plain: 0x13F9A, debug: 0x148C0 };

/// `OJZ_Sec3_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC3_TYPE_TABLE: Pin = Pin { plain: 0x13F96, debug: 0x148BC };

/// `OJZ_Sec4_Objects`. tests: act_descriptor_port
pub const OJZ_SEC4_OBJECTS: Pin = Pin { plain: 0x13FA0, debug: 0x148C6 };

/// `OJZ_Sec4_Rings`. tests: act_descriptor_port
pub const OJZ_SEC4_RINGS: Pin = Pin { plain: 0x13FA2, debug: 0x148C8 };

/// `OJZ_Sec4_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC4_TYPE_TABLE: Pin = Pin { plain: 0x13F9E, debug: 0x148C4 };

/// `OJZ_Sec5_Objects`. tests: act_descriptor_port
pub const OJZ_SEC5_OBJECTS: Pin = Pin { plain: 0x13FD8, debug: 0x148FE };

/// `OJZ_Sec5_Rings`. tests: act_descriptor_port
pub const OJZ_SEC5_RINGS: Pin = Pin { plain: 0x13FDA, debug: 0x14900 };

/// `OJZ_Sec5_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC5_TYPE_TABLE: Pin = Pin { plain: 0x13FD6, debug: 0x148FC };

/// `OJZ_Sec6_Objects`. tests: act_descriptor_port
pub const OJZ_SEC6_OBJECTS: Pin = Pin { plain: 0x14000, debug: 0x14926 };

/// `OJZ_Sec6_Rings`. tests: act_descriptor_port
pub const OJZ_SEC6_RINGS: Pin = Pin { plain: 0x14002, debug: 0x14928 };

/// `OJZ_Sec6_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC6_TYPE_TABLE: Pin = Pin { plain: 0x13FFE, debug: 0x14924 };

/// `OJZ_Sec7_Objects`. tests: act_descriptor_port
pub const OJZ_SEC7_OBJECTS: Pin = Pin { plain: 0x14008, debug: 0x1492E };

/// `OJZ_Sec7_Rings`. tests: act_descriptor_port
pub const OJZ_SEC7_RINGS: Pin = Pin { plain: 0x1400A, debug: 0x14930 };

/// `OJZ_Sec7_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC7_TYPE_TABLE: Pin = Pin { plain: 0x14006, debug: 0x1492C };

/// `OJZ_Sec8_Objects`. tests: act_descriptor_port
pub const OJZ_SEC8_OBJECTS: Pin = Pin { plain: 0x14030, debug: 0x14956 };

/// `OJZ_Sec8_Rings`. tests: act_descriptor_port
pub const OJZ_SEC8_RINGS: Pin = Pin { plain: 0x14032, debug: 0x14958 };

/// `OJZ_Sec8_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC8_TYPE_TABLE: Pin = Pin { plain: 0x1402E, debug: 0x14954 };

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
pub const SEC_LEN: Pin = Pin { plain: 0x22, debug: 0x22 };

/// `Camera_Y_Coarse_Prev`. tests: entity_window_port
pub const CAMERA_Y_COARSE_PREV: Pin = Pin { plain: 0xFFFFB31A, debug: 0xFFFFB3A8 };

/// `Current_Act_Ptr`. tests: entity_window_port, section_port
pub const CURRENT_ACT_PTR: Pin = Pin { plain: 0xFFFFB4D6, debug: 0xFFFFB564 };

/// `Entity_Window_Active`. tests: entity_window_port
pub const ENTITY_WINDOW_ACTIVE: Pin = Pin { plain: 0xFFFFB20E, debug: 0xFFFFB29C };

/// `Entity_Window_Anchor`. tests: entity_window_port
pub const ENTITY_WINDOW_ANCHOR: Pin = Pin { plain: 0xFFFFB210, debug: 0xFFFFB29E };

/// `Entity_Window_OriginX`. tests: entity_window_port
pub const ENTITY_WINDOW_ORIGIN_X: Pin = Pin { plain: 0xFFFFB212, debug: 0xFFFFB2A0 };

/// `Entity_Window_OriginY`. tests: entity_window_port
pub const ENTITY_WINDOW_ORIGIN_Y: Pin = Pin { plain: 0xFFFFB214, debug: 0xFFFFB2A2 };

/// `Entity_Window_Center_ID`. tests: entity_window_port
pub const ENTITY_WINDOW_CENTER_ID: Pin = Pin { plain: 0xFFFFB20F, debug: 0xFFFFB29D };

/// `Entity_Scan_State`. tests: entity_window_port
pub const ENTITY_SCAN_STATE: Pin = Pin { plain: 0xFFFFB1A2, debug: 0xFFFFB230 };

/// `Entity_Loaded_Masks`. tests: entity_window_port
pub const ENTITY_LOADED_MASKS: Pin = Pin { plain: 0xFFFFB216, debug: 0xFFFFB2A4 };

/// `Entity_Mask_Scratch`. tests: entity_window_port
pub const ENTITY_MASK_SCRATCH: Pin = Pin { plain: 0xFFFFB296, debug: 0xFFFFB324 };

/// `Ring_Collected_Window`. tests: entity_window_port
pub const RING_COLLECTED_WINDOW: Pin = Pin { plain: 0xFFFFB31C, debug: 0xFFFFB3AA };

/// `Ring_Collected_Park`. tests: entity_window_port
pub const RING_COLLECTED_PARK: Pin = Pin { plain: 0xFFFFB450, debug: 0xFFFFB4DE };

/// `Collected_Park_Next`. tests: entity_window_port
pub const COLLECTED_PARK_NEXT: Pin = Pin { plain: 0xFFFFB4D4, debug: 0xFFFFB562 };

/// `RingBuffer_Clear`. tests: entity_window_port
pub const RING_BUFFER_CLEAR: Pin = Pin { plain: 0x37DE, debug: 0x41AC };

/// `RingBuffer_Remove`. tests: entity_window_port
pub const RING_BUFFER_REMOVE: Pin = Pin { plain: 0x37AA, debug: 0x4178 };

/// `Section_GetSecPtrXY`. tests: entity_window_port
pub const SECTION_GET_SEC_PTR_XY: Pin = Pin { plain: 0x5CDA, debug: 0x6FD4 };

/// `Section_FlatIDXY`. tests: entity_window_port
pub const SECTION_FLAT_IDXY: Pin = Pin { plain: 0x5CC0, debug: 0x6FBA };

/// `AllocDynamic`. tests: load_object_port, children_port
pub const ALLOC_DYNAMIC: Pin = Pin { plain: 0x2D12, debug: 0x2FF6 };

/// `AllocEffect`. tests: children_port
pub const ALLOC_EFFECT: Pin = Pin { plain: 0x2D76, debug: 0x305A };

/// `Palette_Buffer`. tests: buffers_port
pub const PALETTE_BUFFER: Pin = Pin { plain: 0xFFFF8216, debug: 0xFFFF8216 };

/// `Hscroll_Buffer`. tests: buffers_port
pub const HSCROLL_BUFFER: Pin = Pin { plain: 0xFFFF851A, debug: 0xFFFF851A };

/// `Static_Pal_Line0`. tests: buffers_port
pub const STATIC_PAL_LINE0: Pin = Pin { plain: 0xFFFF8F0A, debug: 0xFFFF8F0A };

/// `Static_Pal_Line1`. tests: buffers_port
pub const STATIC_PAL_LINE1: Pin = Pin { plain: 0xFFFF8F18, debug: 0xFFFF8F18 };

/// `Static_Pal_Line2`. tests: buffers_port
pub const STATIC_PAL_LINE2: Pin = Pin { plain: 0xFFFF8F26, debug: 0xFFFF8F26 };

/// `Static_Pal_Line3`. tests: buffers_port
pub const STATIC_PAL_LINE3: Pin = Pin { plain: 0xFFFF8F42, debug: 0xFFFF8F42 };

/// `Static_Sprite_DMA`. tests: buffers_port
pub const STATIC_SPRITE_DMA: Pin = Pin { plain: 0xFFFF8F50, debug: 0xFFFF8F50 };

/// `Static_Hscroll_Line`. tests: buffers_port
pub const STATIC_HSCROLL_LINE: Pin = Pin { plain: 0xFFFF8F5E, debug: 0xFFFF8F5E };

/// `Palette_Dirty`. tests: buffers_port, palette_port
pub const PALETTE_DIRTY: Pin = Pin { plain: 0xFFFF8296, debug: 0xFFFF8296 };

/// `Parallax_Active_Config`. tests: buffers_port
pub const PARALLAX_ACTIVE_CONFIG: Pin = Pin { plain: 0x6386, debug: 0x7B4E };

/// `Palette_Ship_Snap`. tests: buffers_port
pub const PALETTE_SHIP_SNAP: Pin = Pin { plain: 0xFFFFBA00, debug: 0xFFFFBA8E };

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

/// `Lag_Frame_Count`, debug-shape consumer only (`debug_only`). tests: vblank_port
pub const LAG_FRAME_COUNT: u32 = 0xFFFF8F78;

/// `DMA_Bytes_ThisFrame`, debug-shape consumer only (`debug_only`). tests: vblank_port
pub const DMA_BYTES_THIS_FRAME: u32 = 0xFFFF8F6C;

/// `PageIn_InFlight`. tests: game_loop_port
pub const PAGE_IN_IN_FLIGHT: Pin = Pin { plain: 0xFFFFB9BA, debug: 0xFFFFBA48 };

/// `PageIn_Saved_PC`. tests: game_loop_port
pub const PAGE_IN_SAVED_PC: Pin = Pin { plain: 0xFFFFB9B4, debug: 0xFFFFBA42 };

/// `PageIn_BankRegs`. tests: game_loop_port
pub const PAGE_IN_BANK_REGS: Pin = Pin { plain: 0x7912, debug: 0x9242 };

/// `Dbg_PageIn_Preempts`, debug-shape consumer only (`debug_only`). tests: game_loop_port
pub const DBG_PAGE_IN_PREEMPTS: u32 = 0xFFFF8F92;

/// `ZX0R_Decompress.__end`. tests: game_loop_port
pub const ZX0R_DECOMPRESS_END: Pin = Pin { plain: 0x2826, debug: 0x2B0A };

/// `PageIn_Staging_Busy`. tests: game_loop_port, load_art_port
pub const PAGE_IN_STAGING_BUSY: Pin = Pin { plain: 0xFFFFB9BC, debug: 0xFFFFBA4A };

/// `PageIn_Flush`. tests: load_art_port
pub const PAGE_IN_FLUSH: Pin = Pin { plain: 0x79DA, debug: 0x9312 };

/// `PageIn_Enqueue`. tests: load_art_port
pub const PAGE_IN_ENQUEUE: Pin = Pin { plain: 0x799C, debug: 0x92D4 };

/// `PageIn_Pool_Table`. tests: load_art_port
pub const PAGE_IN_POOL_TABLE: Pin = Pin { plain: 0xFFFFB9E8, debug: 0xFFFFBA76 };

/// `PageIn_Queue_Count`. tests: load_art_port
pub const PAGE_IN_QUEUE_COUNT: Pin = Pin { plain: 0xFFFFB9BE, debug: 0xFFFFBA4C };

/// `PageIn_Suspended`. tests: load_art_port
pub const PAGE_IN_SUSPENDED: Pin = Pin { plain: 0xFFFFB9BB, debug: 0xFFFFBA49 };

/// `PageIn_Land_Pending`. tests: load_art_port
pub const PAGE_IN_LAND_PENDING: Pin = Pin { plain: 0xFFFFB9BD, debug: 0xFFFFBA4B };

/// `PageCache_Init`. tests: load_art_port
pub const PAGE_CACHE_INIT: Pin = Pin { plain: 0x7A2A, debug: 0x9362 };

/// `PageCache_AllocFrame`. tests: load_art_port
pub const PAGE_CACHE_ALLOC_FRAME: Pin = Pin { plain: 0x7ADA, debug: 0x9476 };

/// `PageCache_Publish`. tests: load_art_port
pub const PAGE_CACHE_PUBLISH: Pin = Pin { plain: 0x7B96, debug: 0x9636 };

/// `PageCache_PatchRun_Seq`. tests: tile_cache_port
pub const PAGE_CACHE_PATCH_RUN_SEQ: Pin = Pin { plain: 0x7C04, debug: 0x970A };

/// `PageCache_PatchRun_Col`. tests: tile_cache_port
pub const PAGE_CACHE_PATCH_RUN_COL: Pin = Pin { plain: 0x7D08, debug: 0x994A };

/// `PageCache_Audit`. tests: tile_cache_port
pub const PAGE_CACHE_AUDIT: Pin = Pin { plain: 0x7F0C, debug: 0x9CCA };

/// `Cache_Art_Stall`. tests: tile_cache_port
pub const CACHE_ART_STALL: Pin = Pin { plain: 0xFFFFADCE, debug: 0xFFFFAE5C };

/// `Page_Audit_Ticks`, debug-shape consumer only (`debug_only`). tests: tile_cache_port
pub const PAGE_AUDIT_TICKS: u32 = 0xFFFF8FA6;

/// `Cache_Stall_Watchdog`, debug-shape consumer only (`debug_only`). tests: tile_cache_port
pub const CACHE_STALL_WATCHDOG: u32 = 0xFFFF8FA4;

/// `Flush_VDP_Shadow`. tests: vblank_port
pub const FLUSH_VDP_SHADOW: Pin = Pin { plain: 0x1C02, debug: 0x1C8C };

/// `VInt_DrawLevel`. tests: vblank_port
pub const V_INT_DRAW_LEVEL: Pin = Pin { plain: 0x4834, debug: 0x5868 };

/// `Vscroll_Write`. tests: vblank_port
pub const VSCROLL_WRITE: Pin = Pin { plain: 0x6398, debug: 0x7B60 };

/// `Read_Controllers`. tests: vblank_port
pub const READ_CONTROLLERS: Pin = Pin { plain: 0x243E, debug: 0x2568 };

/// `Process_DMA_Critical`. tests: vblank_port
pub const PROCESS_DMA_CRITICAL: Pin = Pin { plain: 0x1E22, debug: 0x1F1C };

/// `Process_DMA_Important`. tests: vblank_port
pub const PROCESS_DMA_IMPORTANT: Pin = Pin { plain: 0x1EF0, debug: 0x1FEA };

/// `Process_DMA_Deferrable`. tests: vblank_port
pub const PROCESS_DMA_DEFERRABLE: Pin = Pin { plain: 0x1F04, debug: 0x1FFE };

/// `Enqueue_Dirty_Buffers`. tests: vblank_port
pub const ENQUEUE_DIRTY_BUFFERS: Pin = Pin { plain: 0x203C, debug: 0x2136 };

/// `BootData`. tests: boot_port
pub const BOOT_DATA: Pin = Pin { plain: 0x398, debug: 0x3A0 };

/// `VInt_Level`. tests: boot_port
pub const V_INT_LEVEL: Pin = Pin { plain: 0x2276, debug: 0x2374 };

/// `BuildStaticDMA`. tests: boot_port
pub const BUILD_STATIC_DMA: Pin = Pin { plain: 0x1F78, debug: 0x2072 };

/// `Sound_Init`. tests: boot_port
pub const SOUND_INIT: Pin = Pin { plain: 0x80A8, debug: 0xB286 };

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
pub const P_STATE_GROUND: Pin = Pin { plain: 0x106D4, debug: 0x107EA };

/// `PState_Roll`. tests: test_p1_player_port
pub const P_STATE_ROLL: Pin = Pin { plain: 0x1083C, debug: 0x1094E };

/// `PState_Spindash`. tests: test_p1_player_port
pub const P_STATE_SPINDASH: Pin = Pin { plain: 0x10EAE, debug: 0x10FBE };

/// `PState_Air`. tests: test_p1_player_port
pub const P_STATE_AIR: Pin = Pin { plain: 0x10B64, debug: 0x10C74 };

/// `PState_Jump`. tests: test_p1_player_port
pub const P_STATE_JUMP: Pin = Pin { plain: 0x10B6C, debug: 0x10C7C };

/// `PState_RollJump`. tests: test_p1_player_port
pub const P_STATE_ROLL_JUMP: Pin = Pin { plain: 0x10B68, debug: 0x10C78 };

/// `PState_AirBall`. tests: test_p1_player_port
pub const P_STATE_AIR_BALL: Pin = Pin { plain: 0x10B64, debug: 0x10C74 };

/// `PState_Fly`. tests: test_p1_player_port
pub const P_STATE_FLY: Pin = Pin { plain: 0x10F4E, debug: 0x1105A };

/// `PState_Glide`. tests: test_p1_player_port
pub const P_STATE_GLIDE: Pin = Pin { plain: 0x11092, debug: 0x1119E };

/// `PState_GlideFall`. tests: test_p1_player_port
pub const P_STATE_GLIDE_FALL: Pin = Pin { plain: 0x1122A, debug: 0x11336 };

/// `PState_Slide`. tests: test_p1_player_port
pub const P_STATE_SLIDE: Pin = Pin { plain: 0x11278, debug: 0x11382 };

/// `PState_Climb`. tests: test_p1_player_port
pub const P_STATE_CLIMB: Pin = Pin { plain: 0x113C2, debug: 0x114CE };

/// `PState_Ledge`. tests: test_p1_player_port
pub const P_STATE_LEDGE: Pin = Pin { plain: 0x1156E, debug: 0x1167A };

/// `Player_SensorFloor`. tests: test_p1_player_port
pub const PLAYER_SENSOR_FLOOR: Pin = Pin { plain: 0x5B02, debug: 0x6DFC };

/// `Player_AtLedgeEdge`. tests: test_p1_player_port
pub const PLAYER_AT_LEDGE_EDGE: Pin = Pin { plain: 0x5C1C, debug: 0x6F16 };

/// `Player_SetState`. tests: test_p2_player_states_port
pub const PLAYER_SET_STATE: Pin = Pin { plain: 0x1042C, debug: 0x104EE };

/// `Player_SnapToSurface`. tests: test_p2_player_states_port
pub const PLAYER_SNAP_TO_SURFACE: Pin = Pin { plain: 0x1057A, debug: 0x1063C };

/// `Player_SensorCeiling`. tests: test_p2_player_states_port
pub const PLAYER_SENSOR_CEILING: Pin = Pin { plain: 0x5B18, debug: 0x6E12 };

/// `Player_SensorWallDir`. tests: test_p2_player_states_port
pub const PLAYER_SENSOR_WALL_DIR: Pin = Pin { plain: 0x5BD2, debug: 0x6ECC };

/// `Player_SensorWallAt`. tests: test_p2_player_states_port
pub const PLAYER_SENSOR_WALL_AT: Pin = Pin { plain: 0x5BCA, debug: 0x6EC4 };

/// `Collision_GetType`. tests: test_p4_player_sensors_port
pub const COLLISION_GET_TYPE: Pin = Pin { plain: 0x572E, debug: 0x6A28 };

/// `SolidityTable`. tests: test_p4_player_sensors_port
pub const SOLIDITY_TABLE: Pin = Pin { plain: 0x702D6, debug: 0x70C60 };

/// `AngleTable`. tests: test_p4_player_sensors_port
pub const ANGLE_TABLE: Pin = Pin { plain: 0x701D6, debug: 0x70B60 };

/// `HeightMaps`. tests: test_p4_player_sensors_port
pub const HEIGHT_MAPS: Pin = Pin { plain: 0x6E1D6, debug: 0x6EB60 };

/// `HeightMapsRot`. tests: test_p4_player_sensors_port
pub const HEIGHT_MAPS_ROT: Pin = Pin { plain: 0x6F1D6, debug: 0x6FB60 };

/// `Character_ID`. tests: test_p1_player_port
pub const CHARACTER_ID: Pin = Pin { plain: 0xFFFFBA82, debug: 0xFFFFE920 };

/// `Player_Chardef`. tests: test_p1_player_port
pub const PLAYER_CHARDEF: Pin = Pin { plain: 0xFFFFBA84, debug: 0xFFFFE922 };

/// `Ability_None`. tests: test_p1_player_port
pub const ABILITY_NONE: Pin = Pin { plain: 0x11FAC, debug: 0x1211E };

/// `CharacterDefs`. tests: test_p1_player_port
pub const CHARACTER_DEFS: Pin = Pin { plain: 0x11F64, debug: 0x12070 };

/// `Player_InitAssets`. tests: test_p1_player_port
pub const PLAYER_INIT_ASSETS: Pin = Pin { plain: 0x11F70, debug: 0x1207C };

/// `Player_LoadArt`. tests: test_p1_player_port
pub const PLAYER_LOAD_ART: Pin = Pin { plain: 0x11F88, debug: 0x12094 };

/// `Player_Ability`. tests: test_p2_player_states_port
pub const PLAYER_ABILITY: Pin = Pin { plain: 0x11FA2, debug: 0x120AE };

/// `PhysTable_Sonic`. tests: test_p1_player_port
pub const PHYS_TABLE_SONIC: Pin = Pin { plain: 0x11EE8, debug: 0x11FF4 };

/// `Pal_SonicTails`. tests: test_p1_player_port
pub const PAL_SONIC_TAILS: Pin = Pin { plain: 0x6E196, debug: 0x6EB20 };

/// `OJZ_TestRaster`. tests: act_descriptor_port
pub const OJZ_TEST_RASTER: Pin = Pin { plain: 0x138EE, debug: 0x14134 };

/// `OJZ_TestPal`. tests: act_descriptor_port
pub const OJZ_TEST_PAL: Pin = Pin { plain: 0x13910, debug: 0x141DA };

/// `OJZ_TestGradient`. tests: act_descriptor_port
pub const OJZ_TEST_GRADIENT: Pin = Pin { plain: 0x13BDA, debug: 0x144A4 };

/// `OJZ_ShimmerCycle`. tests: act_descriptor_port
pub const OJZ_SHIMMER_CYCLE: Pin = Pin { plain: 0x13970, debug: 0x1423A };

/// `OJZ_TestVsram`. tests: act_descriptor_port
pub const OJZ_TEST_VSRAM: Pin = Pin { plain: 0x13BF8, debug: 0x144C2 };

/// `OJZ_TestRamp`. tests: act_descriptor_port
pub const OJZ_TEST_RAMP: Pin = Pin { plain: 0x13C16, debug: 0x144E0 };

/// `Raster_Program`. tests: raster_port
pub const RASTER_PROGRAM: Pin = Pin { plain: 0xFFFF8BD2, debug: 0xFFFF8BD2 };

/// `Raster_Cursor`. tests: raster_port
pub const RASTER_CURSOR: Pin = Pin { plain: 0xFFFF8BD6, debug: 0xFFFF8BD6 };

/// `Raster_Pending`. tests: raster_port
pub const RASTER_PENDING: Pin = Pin { plain: 0xFFFF8BDA, debug: 0xFFFF8BDA };

/// `Raster_Buf_A`. tests: raster_port
pub const RASTER_BUF_A: Pin = Pin { plain: 0xFFFF8BE0, debug: 0xFFFF8BE0 };

/// `Raster_Active_Buf`. tests: raster_port
pub const RASTER_ACTIVE_BUF: Pin = Pin { plain: 0xFFFF8CE0, debug: 0xFFFF8CE0 };

/// `Raster_Buf_B`. tests: raster_port
pub const RASTER_BUF_B: Pin = Pin { plain: 0xFFFF8C60, debug: 0xFFFF8C60 };

/// `Raster_Line`. tests: raster_port
pub const RASTER_LINE: Pin = Pin { plain: 0xFFFF8BDE, debug: 0xFFFF8BDE };

/// `Raster_Dense_Lines`. tests: raster_port
pub const RASTER_DENSE_LINES: Pin = Pin { plain: 0xFFFF8CE4, debug: 0xFFFF8CE4 };

/// `Raster_Dense_Cursor`. tests: raster_port
pub const RASTER_DENSE_CURSOR: Pin = Pin { plain: 0xFFFF8CE6, debug: 0xFFFF8CE6 };

/// `Raster_Dense_Cmd`. tests: raster_port
pub const RASTER_DENSE_CMD: Pin = Pin { plain: 0xFFFF8CEA, debug: 0xFFFF8CEA };

/// `Raster_Dense_Mode`. tests: raster_port
pub const RASTER_DENSE_MODE: Pin = Pin { plain: 0xFFFF8CEE, debug: 0xFFFF8CEE };

/// `Raster_Ramp_Acc`. tests: raster_port
pub const RASTER_RAMP_ACC: Pin = Pin { plain: 0xFFFF8CF0, debug: 0xFFFF8CF0 };

/// `Raster_Ramp_Step`. tests: raster_port
pub const RASTER_RAMP_STEP: Pin = Pin { plain: 0xFFFF8CF4, debug: 0xFFFF8CF4 };

/// `Effects_World_Y`. tests: raster_port
pub const EFFECTS_WORLD_Y: Pin = Pin { plain: 0xFFFF8CF8, debug: 0xFFFF8CF8 };

/// `Effects_Screen_L`. tests: raster_port, parallax_port, buffers_port
pub const EFFECTS_SCREEN_L: Pin = Pin { plain: 0xFFFF8D00, debug: 0xFFFF8D00 };

/// `Effects_Offscreen_Entry`. tests: raster_port, buffers_port
pub const EFFECTS_OFFSCREEN_ENTRY: Pin = Pin { plain: 0xFFFF8D22, debug: 0xFFFF8D22 };

/// `Static_Pal_Ship`. tests: raster_port
pub const STATIC_PAL_SHIP: Pin = Pin { plain: 0xFFFF8F34, debug: 0xFFFF8F34 };

/// `Build_DMA_Entry`. tests: raster_port
pub const BUILD_DMA_ENTRY: Pin = Pin { plain: 0x2006, debug: 0x2100 };

/// `Raster_Patch_Tab`. tests: raster_port
pub const RASTER_PATCH_TAB: Pin = Pin { plain: 0xFFFF8D26, debug: 0xFFFF8D26 };

/// `Raster_State`. tests: raster_port
pub const RASTER_STATE: Pin = Pin { plain: 0xFFFF8BD2, debug: 0xFFFF8BD2 };

/// `Raster_State_End`. tests: raster_port
pub const RASTER_STATE_END: Pin = Pin { plain: 0xFFFF8D2A, debug: 0xFFFF8D2A };

/// `Pal_Variant_Stage`. tests: raster_port
pub const PAL_VARIANT_STAGE: Pin = Pin { plain: 0xFFFF8DEA, debug: 0xFFFF8DEA };

/// `Raster_VBlank`. tests: game_loop_port, vblank_port, load_art_port, boot_port
pub const RASTER_V_BLANK: Pin = Pin { plain: 0x6D38, debug: 0x8500 };

/// `Palette_Compose`. tests: game_loop_port
pub const PALETTE_COMPOSE: Pin = Pin { plain: 0x71D2, debug: 0x899A };

/// `Player_Blocks`. tests: test_p1_player_port
pub const PLAYER_BLOCKS: Pin = Pin { plain: 0xFFFFBA88, debug: 0xFFFFE926 };

/// `Player_Ring_Index`. tests: test_p1_player_port
pub const PLAYER_RING_INDEX: Pin = Pin { plain: 0xFFFFBE00, debug: 0xFFFFED00 };

/// `Player_Pos_Ring`. tests: test_p1_player_port
pub const PLAYER_POS_RING: Pin = Pin { plain: 0xFFFFBC00, debug: 0xFFFFEB00 };

/// `Player_Stat_Ring`. tests: test_p1_player_port
pub const PLAYER_STAT_RING: Pin = Pin { plain: 0xFFFFBD00, debug: 0xFFFFEC00 };

/// `Player_Death_Pending`. tests: test_p1_player_port
pub const PLAYER_DEATH_PENDING: Pin = Pin { plain: 0xFFFFBAB8, debug: 0xFFFFE956 };

/// `Player_Bound_Right`. tests: test_p1_player_port
pub const PLAYER_BOUND_RIGHT: Pin = Pin { plain: 0xFFFFBABA, debug: 0xFFFFE958 };

/// `Player_Bound_Bottom`. tests: test_p1_player_port
pub const PLAYER_BOUND_BOTTOM: Pin = Pin { plain: 0xFFFFBABC, debug: 0xFFFFE95A };

/// `DustSpindash_Spawn`. tests: test_p1_player_port
pub const DUST_SPINDASH_SPAWN: Pin = Pin { plain: 0x12168, debug: 0x12332 };

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
