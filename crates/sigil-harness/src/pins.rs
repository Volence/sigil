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
//! [provenance] 97 regions, 417 symbols, 7 offsets

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
pub const ASSEMBLED_LEN: usize = 0xA5C82;
/// Assembled (pre-convsym) ROM length, `__DEBUG__` shape. tests: m1d_rom, m1d_debug_rom, mixed_dac_rom
pub const DEBUG_ASSEMBLED_LEN: usize = 0xA81FC;

// ── Regions (manifest order) ──

/// `Vectors` .. start + 0x100 plain / 0x100 debug (literal — no end symbol) — gate `SIGIL_EMP_VECTORS`. tests: vectors_port
pub const VECTORS: Region = Region { plain_base: 0x0, debug_base: 0x0, plain_len: 0x100, debug_len: 0x100 };

/// `GameHeader` .. `section:header`. tests: header_port
pub const HEADER: Region = Region { plain_base: 0x100, debug_base: 0x100, plain_len: 0x100, debug_len: 0x100 };

/// `HeightMaps` .. `section:collision_data`. tests: collision_data_port
pub const COLLISION_DATA: Region = Region { plain_base: 0x6DEAC, debug_base: 0x6E76C, plain_len: 0x1D304, debug_len: 0x1D304 };

/// `EntryPoint` .. `section:boot` — gate `SIGIL_EMP_BOOT`. tests: boot_port
pub const BOOT: Region = Region { plain_base: 0x200, debug_base: 0x200, plain_len: 0x198, debug_len: 0x1A0 };

/// `BootData` .. `section:boot_head`. tests: boot_data_port
pub const BOOT_HEAD: Region = Region { plain_base: 0x398, debug_base: 0x3A0, plain_len: 0x184A, debug_len: 0x18CC };

/// `BootData_PostBlob` .. `section:boot_tail`. tests: boot_data_port
pub const BOOT_TAIL: Region = Region { plain_base: 0x1BE2, debug_base: 0x1C6C, plain_len: 0xE, debug_len: 0xE };

/// `VDP_Shadow_Init` .. `section:vdp_init` — gate `SIGIL_EMP_VDP_INIT`. tests: vdp_init_port
pub const VDP_INIT: Region = Region { plain_base: 0x1BF0, debug_base: 0x1C7A, plain_len: 0x3A, debug_len: 0x90 };

/// `Init_DMA_Queue` .. `section:dma_queue` — gate `SIGIL_EMP_DMA_QUEUE`. tests: dma_queue_port
pub const DMA_QUEUE: Region = Region { plain_base: 0x1C2A, debug_base: 0x1D0A, plain_len: 0x32C, debug_len: 0x338 };

/// `Init_SpriteTable` .. `section:buffers` — gate `SIGIL_EMP_BUFFERS`. tests: buffers_port
pub const BUFFERS: Region = Region { plain_base: 0x1F56, debug_base: 0x2042, plain_len: 0x2D8, debug_len: 0x2D8 };

/// `VBlank_Handler` .. `section:vblank` — gate `SIGIL_EMP_VBLANK`. tests: vblank_port
pub const VBLANK: Region = Region { plain_base: 0x222E, debug_base: 0x231A, plain_len: 0x1DC, debug_len: 0x1E8 };

/// `HBlank_Install` .. `section:hblank` — gate `SIGIL_EMP_HBLANK`. tests: hblank_port, m1c_vector_table
pub const HBLANK: Region = Region { plain_base: 0x240E, debug_base: 0x2506, plain_len: 0x30, debug_len: 0x30 };

/// `Read_Controllers` .. `section:controllers` — gate `SIGIL_EMP_CONTROLLERS`. tests: controllers_port
pub const CONTROLLERS: Region = Region { plain_base: 0x243E, debug_base: 0x2536, plain_len: 0x10E, debug_len: 0x10E };

/// `GameLoop` .. `section:game_loop` — gate `SIGIL_EMP_GAME_LOOP`. tests: game_loop_port, load_art_port
pub const GAME_LOOP: Region = Region { plain_base: 0x254C, debug_base: 0x2644, plain_len: 0x1C, debug_len: 0x1E };

/// `Input_Tick` .. `section:replay`. tests: game_loop_port
pub const REPLAY: Region = Region { plain_base: 0x256E, debug_base: 0x2668, plain_len: 0x146, debug_len: 0x1F6 };

/// `S4LZ_DecompressDict` .. `section:s4lz` — gate `SIGIL_EMP_S4LZ`. tests: s4lz_port
pub const S4LZ: Region = Region { plain_base: 0x26B6, debug_base: 0x2860, plain_len: 0xF8, debug_len: 0x200 };

/// `ZX0R_Decompress` .. `section:zx0_resume`.
pub const ZX0_RESUME: Region = Region { plain_base: 0x27AE, debug_base: 0x2A60, plain_len: 0x78, debug_len: 0x78 };

/// `GetSineCosine` .. `section:math` — gate `SIGIL_EMP_MATH`. tests: math_port, parallax_port
pub const MATH: Region = Region { plain_base: 0x2826, debug_base: 0x2AD8, plain_len: 0x3F6, debug_len: 0x3F6 };

/// `Perform_DPLC` .. `section:dplc` — gate `SIGIL_EMP_DPLC`. tests: dplc_port
pub const DPLC: Region = Region { plain_base: 0x2C1C, debug_base: 0x2ECE, plain_len: 0xA4, debug_len: 0xA4 };

/// `InitObjectRAM` .. `section:core` — gate `SIGIL_EMP_CORE`. tests: core_port
pub const CORE: Region = Region { plain_base: 0x2CC0, debug_base: 0x2F72, plain_len: 0x2F8, debug_len: 0x740 };

/// `InitSpriteSystem` .. `section:sprites` — gate `SIGIL_EMP_SPRITES`. tests: sprites_port
pub const SPRITES: Region = Region { plain_base: 0x2FB8, debug_base: 0x36B2, plain_len: 0x41A, debug_len: 0x534 };

/// `AnimateSprite` .. `section:animate` — gate `SIGIL_EMP_ANIMATE`. tests: animate_port, test_objects_port
pub const ANIMATE: Region = Region { plain_base: 0x33D2, debug_base: 0x3BE6, plain_len: 0x194, debug_len: 0x2B8 };

/// `TouchResponse` .. `section:collision` — gate `SIGIL_EMP_COLLISION`. tests: collision_port
pub const COLLISION: Region = Region { plain_base: 0x3566, debug_base: 0x3E9E, plain_len: 0x200, debug_len: 0x208 };

/// `RingBuffer_Add` .. `section:rings` — gate `SIGIL_EMP_RINGS`. tests: rings_port
pub const RINGS: Region = Region { plain_base: 0x3766, debug_base: 0x40A6, plain_len: 0x1BE, debug_len: 0x224 };

/// `Collected_Init` .. `section:entity_window` — gate `SIGIL_EMP_ENTITY_WINDOW`. tests: entity_window_port
pub const ENTITY_WINDOW: Region = Region { plain_base: 0x3924, debug_base: 0x42CA, plain_len: 0x8EE, debug_len: 0xD5C };

/// `PopulateSpawnedPieceCount` .. `section:children` — gate `SIGIL_EMP_CHILDREN`. tests: children_port
pub const CHILDREN: Region = Region { plain_base: 0x4212, debug_base: 0x5026, plain_len: 0x2EC, debug_len: 0x39C };

/// `Load_Object` .. `section:load_object` — gate `SIGIL_EMP_LOAD_OBJECT`. tests: load_object_port, entity_window_port
pub const LOAD_OBJECT: Region = Region { plain_base: 0x44FE, debug_base: 0x53C2, plain_len: 0x88, debug_len: 0x88 };

/// `Plane_Buffer_Reset` .. `section:plane_buffer` — gate `SIGIL_EMP_PLANE_BUFFER`. tests: plane_buffer_port
pub const PLANE_BUFFER: Region = Region { plain_base: 0x4586, debug_base: 0x544A, plain_len: 0x31A, debug_len: 0x424 };

/// `Tile_Cache_GetTile` .. `section:tile_cache` — gate `SIGIL_EMP_TILE_CACHE`. tests: tile_cache_port
pub const TILE_CACHE: Region = Region { plain_base: 0x48A0, debug_base: 0x586E, plain_len: 0xE86, debug_len: 0x10EC };

/// `Collision_GetType` .. `section:collision_lookup` — gate `SIGIL_EMP_COLLISION_LOOKUP`. tests: collision_lookup_port
pub const COLLISION_LOOKUP: Region = Region { plain_base: 0x572E, debug_base: 0x6964, plain_len: 0x68, debug_len: 0x68 };

/// `Section_Init` .. `section:section` — gate `SIGIL_EMP_SECTION`. tests: section_port
pub const SECTION: Region = Region { plain_base: 0x5C8A, debug_base: 0x6EC0, plain_len: 0x460, debug_len: 0x578 };

/// `Camera_Init` .. `section:camera` — gate `SIGIL_EMP_CAMERA`. tests: camera_port
pub const CAMERA: Region = Region { plain_base: 0x60EA, debug_base: 0x7438, plain_len: 0x1C8, debug_len: 0x1D2 };

/// `Parallax_Init` .. `section:parallax` — gate `SIGIL_EMP_PARALLAX`. tests: parallax_port
pub const PARALLAX: Region = Region { plain_base: 0x62B2, debug_base: 0x760A, plain_len: 0x95C, debug_len: 0x9F0 };

/// `Raster_Install` .. `section:raster` — gate `SIGIL_EMP_RASTER`. tests: raster_port
pub const RASTER: Region = Region { plain_base: 0x6C12, debug_base: 0x7FFE, plain_len: 0x364, debug_len: 0x364 };

/// `Palette_LoadPal` .. `Effects_InstallPreset` — gate `SIGIL_EMP_PALETTE`. tests: palette_port
pub const PALETTE: Region = Region { plain_base: 0x6F76, debug_base: 0x8362, plain_len: 0x4AE, debug_len: 0x4AE };

/// `Effects_InstallPreset` .. `section:preset`.
pub const PRESET: Region = Region { plain_base: 0x7424, debug_base: 0x8810, plain_len: 0x86, debug_len: 0x88 };

/// `Level_LoadArt` .. `section:load_art` — gate `SIGIL_EMP_LOAD_ART`. tests: load_art_port
pub const LOAD_ART: Region = Region { plain_base: 0x74BC, debug_base: 0x88A8, plain_len: 0xB6, debug_len: 0xB6 };

/// `PageIn_Process` .. `section:page_in`.
pub const PAGE_IN: Region = Region { plain_base: 0x7574, debug_base: 0x8960, plain_len: 0x2DE, debug_len: 0x452 };

/// `PageCache_Init` .. `section:page_cache`.
pub const PAGE_CACHE: Region = Region { plain_base: 0x7860, debug_base: 0x8DBC, plain_len: 0x4E4, debug_len: 0xE78 };

/// `BG_Init` .. `section:bg` — gate `SIGIL_EMP_BG`. tests: bg_port
pub const BG: Region = Region { plain_base: 0x7D46, debug_base: 0x9C34, plain_len: 0xD4, debug_len: 0x134 };

/// `BgAnim_Init` .. start + 0x9E plain / 0x158 debug (literal — no end symbol) — gate `SIGIL_EMP_BG_ANIM`. tests: bg_anim_port
pub const BG_ANIM: Region = Region { plain_base: 0x7E1A, debug_base: 0x9D68, plain_len: 0x9E, debug_len: 0x158 };

/// `CompressionSelfTest` .. `section:compression_selftest` (debug-only region; plain empty at `Sound_PostByte`) — gate `SIGIL_EMP_COMPRESSION_SELFTEST`. tests: compression_selftest_port
pub const COMPRESSION_SELFTEST: Region = Region { plain_base: 0x7EB8, debug_base: 0x9EC0, plain_len: 0x0, debug_len: 0xDE0 };

/// `Sound_PostByte` .. start + 0x2A8 plain / 0x452 debug (literal — no end symbol) — gate `SIGIL_EMP_SOUND_API`. tests: sound_api_port
pub const SOUND_API: Region = Region { plain_base: 0x7EB8, debug_base: 0xACA2, plain_len: 0x2A8, debug_len: 0x452 };

/// `TestSolid_Init` .. `section:test_solid` — gate `SIGIL_EMP_TEST_OBJECTS`. tests: test_objects_port
pub const TEST_SOLID: Region = Region { plain_base: 0x12316, debug_base: 0x12832, plain_len: 0x12, debug_len: 0x12 };

/// `TestParticle` .. `section:test_particle` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_OBJECTS`. tests: test_objects_port
pub const TEST_PARTICLE: Region = Region { plain_base: 0x12328, debug_base: 0x12844, plain_len: 0x0, debug_len: 0x58 };

/// `TestStatic_Main` .. `section:test_static` — gate `SIGIL_EMP_TEST_STATIC`. tests: test_g1_objects_port
pub const TEST_STATIC: Region = Region { plain_base: 0x12312, debug_base: 0x124F2, plain_len: 0x4, debug_len: 0x4 };

/// `TestAnimated` .. `section:test_animated` (debug-only region; plain empty at `TestSolid_Init`) — gate `SIGIL_EMP_TEST_ANIMATED`. tests: test_g1_objects_port
pub const TEST_ANIMATED: Region = Region { plain_base: 0x12316, debug_base: 0x124F6, plain_len: 0x0, debug_len: 0x60 };

/// `TestEmitter` .. `section:test_emitter` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_EMITTER`. tests: test_g2_objects_port
pub const TEST_EMITTER: Region = Region { plain_base: 0x12328, debug_base: 0x1289C, plain_len: 0x0, debug_len: 0x5E };

/// `TestStressEmitter` .. `section:test_stress_emitter` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_STRESS_EMITTER`. tests: test_g2_objects_port
pub const TEST_STRESS_EMITTER: Region = Region { plain_base: 0x12328, debug_base: 0x12A30, plain_len: 0x0, debug_len: 0x5E };

/// `TestChurnObj` .. `section:test_churn` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_CHURN`. tests: test_g2_objects_port
pub const TEST_CHURN: Region = Region { plain_base: 0x12328, debug_base: 0x12A8E, plain_len: 0x0, debug_len: 0x7C };

/// `TestChildPart` .. `section:test_parent` (debug-only region; plain empty at `ObjDef_PathSwap`) — gate `SIGIL_EMP_TEST_PARENT`. tests: test_g3_objects_port
pub const TEST_PARENT: Region = Region { plain_base: 0x12328, debug_base: 0x128FA, plain_len: 0x0, debug_len: 0x136 };

/// `TestPlayer` .. `section:test_player` (debug-only region; plain empty at `TestSolid_Init`) — gate `SIGIL_EMP_TEST_PLAYER`. tests: test_g4_final_objects_port
pub const TEST_PLAYER: Region = Region { plain_base: 0x12316, debug_base: 0x12556, plain_len: 0x0, debug_len: 0x294 };

/// `TestEnemy_Init` .. `section:test_enemy` (debug-only region; plain empty at `TestSolid_Init`) — gate `SIGIL_EMP_TEST_ENEMY`. tests: test_g4_final_objects_port
pub const TEST_ENEMY: Region = Region { plain_base: 0x12316, debug_base: 0x127EA, plain_len: 0x0, debug_len: 0x48 };

/// `ObjDef_PathSwap` .. `section:path_swap` — gate `SIGIL_EMP_PATH_SWAP`. tests: test_g4_final_objects_port
pub const PATH_SWAP: Region = Region { plain_base: 0x12328, debug_base: 0x12B0A, plain_len: 0x92, debug_len: 0xFA };

/// `OJZ_TestRaster` .. `section:ojz_effects`.
pub const OJZ_EFFECTS: Region = Region { plain_base: 0x13440, debug_base: 0x13C8A, plain_len: 0x54E, debug_len: 0x5BC };

/// `DeformTable_Zero` .. `section:scene_registry` — gate `SIGIL_EMP_SCENE_REGISTRY`. tests: scene_registry_port
pub const SCENE_REGISTRY: Region = Region { plain_base: 0x123BA, debug_base: 0x12C04, plain_len: 0xEA0, debug_len: 0xEA0 };

/// `Map_TestObj` .. `section:test_mappings` — gate `SIGIL_EMP_TEST_MAPPINGS`. tests: test_mappings_port
pub const TEST_MAPPINGS: Region = Region { plain_base: 0x29836, debug_base: 0x2A0EE, plain_len: 0x30, debug_len: 0x30 };

/// `Map_DustSpindash` .. `section:dust_data` — gate `SIGIL_EMP_DUST_DATA`.
pub const DUST_DATA: Region = Region { plain_base: 0x29866, debug_base: 0x2A11E, plain_len: 0xBDA, debug_len: 0xBDA };

/// `Ani_Sonic` .. `section:sonic_anims` — gate `SIGIL_EMP_SONIC_ANIMS`. tests: sonic_anims_port
pub const SONIC_ANIMS: Region = Region { plain_base: 0x2A440, debug_base: 0x2ACF8, plain_len: 0x10A, debug_len: 0x10A };

/// `Ani_Tails` .. `section:tails_anims` — gate `SIGIL_EMP_TAILS_ANIMS`. tests: sonic_anims_port
pub const TAILS_ANIMS: Region = Region { plain_base: 0x2A54A, debug_base: 0x2AE02, plain_len: 0x1BC, debug_len: 0x1BC };

/// `Ani_Knuckles` .. `section:knuckles_anims` — gate `SIGIL_EMP_KNUCKLES_ANIMS`. tests: sonic_anims_port
pub const KNUCKLES_ANIMS: Region = Region { plain_base: 0x2A706, debug_base: 0x2AFBE, plain_len: 0x16C, debug_len: 0x16C };

/// `Map_Tails` .. `section:tails_data` — gate `SIGIL_EMP_TAILS_DATA`. tests: collision_data_port
pub const TAILS_DATA: Region = Region { plain_base: 0x2A886, debug_base: 0x2B146, plain_len: 0x20F5E, debug_len: 0x20F5E };

/// `Map_Knuckles` .. `section:knuckles_data` — gate `SIGIL_EMP_KNUCKLES_DATA`. tests: collision_data_port
pub const KNUCKLES_DATA: Region = Region { plain_base: 0x4B7E4, debug_base: 0x4C0A4, plain_len: 0x226C8, debug_len: 0x226C8 };

/// `Ani_Particle` .. `section:particle_anims` (debug-only region; plain empty at `Ani_DustSpindash`) — gate `SIGIL_EMP_PARTICLE_ANIMS`. tests: particle_anims_port, test_objects_port
pub const PARTICLE_ANIMS: Region = Region { plain_base: 0x2A872, debug_base: 0x2B12A, plain_len: 0x0, debug_len: 0x8 };

/// `Ani_DustSpindash` .. `section:dust_anims` — gate `SIGIL_EMP_DUST_ANIMS`.
pub const DUST_ANIMS: Region = Region { plain_base: 0x2A872, debug_base: 0x2B132, plain_len: 0x14, debug_len: 0x14 };

/// `OJZ_Sec0_TypeTable` .. `section:entity_data`. tests: ojz_run_a_port
pub const ENTITY_DATA: Region = Region { plain_base: 0x139C2, debug_base: 0x1427A, plain_len: 0x170, debug_len: 0x170 };

/// `OJZ_Act_Pool_Page0` .. `section:ojz_act_pool`. tests: ojz_run_a_port
pub const OJZ_ACT_POOL: Region = Region { plain_base: 0x13B32, debug_base: 0x143EA, plain_len: 0x2F0C, debug_len: 0x2F0C };

/// `OJZ_Act1_Descriptor` .. `section:act_descriptor` — gate `SIGIL_EMP_ACT_DESCRIPTOR`. tests: act_descriptor_port
pub const ACT_DESCRIPTOR: Region = Region { plain_base: 0x16A3E, debug_base: 0x172F6, plain_len: 0x27A, debug_len: 0x27A };

/// `OJZ_Sec0_Blocks` .. `section:sec_block_blobs`. tests: ojz_run_b_port
pub const SEC_BLOCK_BLOBS: Region = Region { plain_base: 0x16CB8, debug_base: 0x17570, plain_len: 0xB60A, debug_len: 0xB60A };

/// `OJZ_Sec0_LocalMap` .. `section:sec_local_maps`. tests: ojz_run_b_port
pub const SEC_LOCAL_MAPS: Region = Region { plain_base: 0x222C2, debug_base: 0x22B7A, plain_len: 0xCC4, debug_len: 0xCC4 };

/// `OJZ_Palette` .. `section:ojz_act_assets`. tests: ojz_run_b_port
pub const OJZ_ACT_ASSETS: Region = Region { plain_base: 0x22F86, debug_base: 0x2383E, plain_len: 0x4882, debug_len: 0x4882 };

/// `BgAnim_Table` .. `section:ojz_bg_anim`. tests: ojz_run_b_port
pub const OJZ_BG_ANIM: Region = Region { plain_base: 0x27808, debug_base: 0x280C0, plain_len: 0x202E, debug_len: 0x202E };

/// `ObjDef_Static` .. start + 0x34 plain / 0x34 debug (literal — no end symbol) — gate `SIGIL_EMP_OBJDEFS`. tests: objdef_port
pub const OBJDEFS: Region = Region { plain_base: 0x1398E, debug_base: 0x14246, plain_len: 0x34, debug_len: 0x34 };

/// `GameState_ObjectTest_Init` .. `section:object_test_state` (debug-only region; plain empty at `GameState_OJZScroll_Init`) — gate `SIGIL_EMP_OBJECT_TEST_STATE`. tests: test_t1_harness_states_port
pub const OBJECT_TEST_STATE: Region = Region { plain_base: 0xA4404, debug_base: 0xA5E54, plain_len: 0x0, debug_len: 0x384 };

/// `GameState_OJZScroll_Init` .. `section:ojz_scroll_test` — gate `SIGIL_EMP_OJZ_SCROLL_TEST`. tests: test_t1_harness_states_port
pub const OJZ_SCROLL_TEST: Region = Region { plain_base: 0xA4404, debug_base: 0xA61D8, plain_len: 0x56A, debug_len: 0xD10 };

/// `Replay_OJZ_Fixture` .. `section:replay_fixture`.
pub const REPLAY_FIXTURE: Region = Region { plain_base: 0xA4972, debug_base: 0xA6EEC, plain_len: 0x260, debug_len: 0x260 };

/// `BusError` .. `section:error_handler` — gate `SIGIL_EMP_ERROR_HANDLER`. tests: error_handler_port
pub const ERROR_HANDLER: Region = Region { plain_base: 0xA4BD2, debug_base: 0xA714C, plain_len: 0x10B0, debug_len: 0x10B0 };

/// `Dac_Temp_Blip` .. start + 0xF8BC plain / 0xF8BC debug (literal — no end symbol) — gate `SIGIL_EMP_DAC`. tests: dac_bank_port
pub const DAC_BANKS: Region = Region { plain_base: 0x90000, debug_base: 0x90000, plain_len: 0xF8BC, debug_len: 0xF8BC };

/// `Song_MovingTrucks` .. start + 0x34E8 plain / 0x4F38 debug (literal — no end symbol) — gate `SIGIL_EMP_MT`. tests: mt_bank_port
pub const MT_BANK_BLOB: Region = Region { plain_base: 0xA0630, debug_base: 0xA0630, plain_len: 0x34E8, debug_len: 0x4F38 };

/// `Sfx_33` .. `section:sfx_bank_blob` — gate `SIGIL_EMP_SFX`. tests: sfx_bank_port
pub const SFX_BANK_BLOB: Region = Region { plain_base: 0xA3B18, debug_base: 0xA5568, plain_len: 0x8EC, debug_len: 0x8EC };

/// `SoundTablesZ80_Head` .. start + 0x630 plain / 0x630 debug (literal — no end symbol) — gate `SIGIL_EMP_SOUNDBANKHEAD`. tests: soundbankhead_port
pub const SOUNDBANKHEAD: Region = Region { plain_base: 0xA0000, debug_base: 0xA0000, plain_len: 0x630, debug_len: 0x630 };

/// `EndOfRom` .. start + 0x0 plain / 0x0 debug (literal — no end symbol) — gate `SIGIL_EMP_EPILOGUE`. tests: m1d_rom, m1d_debug_rom
pub const EPILOGUE: Region = Region { plain_base: 0xA5C82, debug_base: 0xA81FC, plain_len: 0x0, debug_len: 0x0 };

/// `ObjCodeBase` .. start + 0x2 plain / 0x2 debug (literal — no end symbol) — gate `SIGIL_EMP_OBJCODEBASE`. tests: m1d_rom, m1d_debug_rom
pub const OBJCODEBASE: Region = Region { plain_base: 0x10000, debug_base: 0x10000, plain_len: 0x2, debug_len: 0x2 };

/// `Player_Init` .. `section:player_common` — gate `SIGIL_EMP_PLAYER_COMMON`. tests: test_p1_player_port
pub const PLAYER_COMMON: Region = Region { plain_base: 0x10002, debug_base: 0x10002, plain_len: 0x6C4, debug_len: 0x7DC };

/// `CharDef_Sonic` .. `section:sonic` — gate `SIGIL_EMP_SONIC`. tests: test_p1_player_port
pub const SONIC: Region = Region { plain_base: 0x11EA4, debug_base: 0x11FC6, plain_len: 0x36, debug_len: 0x36 };

/// `CharDef_Tails` .. `section:tails` — gate `SIGIL_EMP_TAILS`. tests: test_p1_player_port
pub const TAILS: Region = Region { plain_base: 0x11EDA, debug_base: 0x11FFC, plain_len: 0x36, debug_len: 0x36 };

/// `CharDef_Knuckles` .. `section:knuckles` — gate `SIGIL_EMP_KNUCKLES`. tests: test_p1_player_port
pub const KNUCKLES: Region = Region { plain_base: 0x11F10, debug_base: 0x12032, plain_len: 0x36, debug_len: 0x36 };

/// `CharacterDefs` .. `section:characters` — gate `SIGIL_EMP_CHARACTERS`. tests: test_p1_player_port
pub const CHARACTERS: Region = Region { plain_base: 0x11F46, debug_base: 0x12068, plain_len: 0x4A, debug_len: 0xB0 };

/// `TailsAppendage_Refresh` .. `section:tails_appendage` — gate `SIGIL_EMP_TAILS_APPENDAGE`. tests: test_p1_player_port
pub const TAILS_APPENDAGE: Region = Region { plain_base: 0x11F90, debug_base: 0x12118, plain_len: 0x11C, debug_len: 0x174 };

/// `DustPuff_Spawn` .. `section:dust_puff` — gate `SIGIL_EMP_DUST_PUFF`.
pub const DUST_PUFF: Region = Region { plain_base: 0x120AC, debug_base: 0x1228C, plain_len: 0x46, debug_len: 0x46 };

/// `Dust_Tick` .. `section:dust_spindash` — gate `SIGIL_EMP_DUST_SPINDASH`.
pub const DUST_SPINDASH: Region = Region { plain_base: 0x120F2, debug_base: 0x122D2, plain_len: 0x102, debug_len: 0x102 };

/// `PState_Ground` .. `section:player_ground` — gate `SIGIL_EMP_PLAYER_GROUND`. tests: test_p2_player_states_port
pub const PLAYER_GROUND: Region = Region { plain_base: 0x106CA, debug_base: 0x107E2, plain_len: 0x48A, debug_len: 0x48A };

/// `PState_Air` .. `section:player_air` — gate `SIGIL_EMP_PLAYER_AIR`. tests: test_p2_player_states_port
pub const PLAYER_AIR: Region = Region { plain_base: 0x10B54, debug_base: 0x10C6C, plain_len: 0x34A, debug_len: 0x34A };

/// `PState_Spindash` .. `section:player_spindash` — gate `SIGIL_EMP_PLAYER_SPINDASH`. tests: test_p2_player_states_port
pub const PLAYER_SPINDASH: Region = Region { plain_base: 0x10E9E, debug_base: 0x10FB6, plain_len: 0x9C, debug_len: 0x9C };

/// `PState_Fly` .. `section:player_fly` — gate `SIGIL_EMP_PLAYER_FLY`. tests: test_p2_player_states_port
pub const PLAYER_FLY: Region = Region { plain_base: 0x10F3A, debug_base: 0x11052, plain_len: 0x138, debug_len: 0x138 };

/// `PState_Glide` .. `section:player_glide` — gate `SIGIL_EMP_PLAYER_GLIDE`. tests: test_p2_player_states_port
pub const PLAYER_GLIDE: Region = Region { plain_base: 0x1107C, debug_base: 0x11196, plain_len: 0x2B6, debug_len: 0x2B6 };

/// `Climb_WallDist` .. `CharDef_Sonic` — gate `SIGIL_EMP_PLAYER_CLIMB`. tests: test_p2_player_states_port
pub const PLAYER_CLIMB: Region = Region { plain_base: 0x1134E, debug_base: 0x1146C, plain_len: 0xB56, debug_len: 0xB5A };

/// `Collision_ProbeDown` .. `section:player_sensors` — gate `SIGIL_EMP_PLAYER_SENSORS`. tests: test_p4_player_sensors_port
pub const PLAYER_SENSORS: Region = Region { plain_base: 0x5796, debug_base: 0x69CC, plain_len: 0x4F4, debug_len: 0x4F4 };

// ── Symbols (manifest order) ──

/// `OJZ_Preset_Sec0`. tests: act_descriptor_port
pub const OJZ_PRESET_SEC0: Pin = Pin { plain: 0x137E0, debug: 0x14098 };

/// `OJZ_Preset_Sec1`. tests: act_descriptor_port
pub const OJZ_PRESET_SEC1: Pin = Pin { plain: 0x13806, debug: 0x140BE };

/// `OJZ_Preset_Sec2`. tests: act_descriptor_port
pub const OJZ_PRESET_SEC2: Pin = Pin { plain: 0x1382C, debug: 0x140E4 };

/// `OJZ_Preset_Sec3`. tests: act_descriptor_port
pub const OJZ_PRESET_SEC3: Pin = Pin { plain: 0x13852, debug: 0x1410A };

/// `OJZ_Preset_Plain`. tests: act_descriptor_port
pub const OJZ_PRESET_PLAIN: Pin = Pin { plain: 0x13878, debug: 0x14130 };

/// `OJZ_Preset_Depth`. tests: act_descriptor_port
pub const OJZ_PRESET_DEPTH: Pin = Pin { plain: 0x1389E, debug: 0x14156 };

/// `EditorSceneBinding_OJZ_Act1_Sec4`. tests: act_descriptor_port
pub const EDITOR_SCENE_BINDING_OJZ_ACT1_SEC4: Pin = Pin { plain: 0x132F0, debug: 0x13B3A };

/// `OJZ_Preset_Sec5`. tests: act_descriptor_port
pub const OJZ_PRESET_SEC5: Pin = Pin { plain: 0x138C4, debug: 0x1417C };

/// `EditorRaster_OJZ_Act1_authored_probe`. tests: act_descriptor_port
pub const EDITOR_RASTER_OJZ_ACT1_AUTHORED_PROBE: Pin = Pin { plain: 0x13386, debug: 0x13BD0 };

/// `EditorRaster_OJZ_Act1_ojz_sec5_showcase`. tests: act_descriptor_port
pub const EDITOR_RASTER_OJZ_ACT1_OJZ_SEC5_SHOWCASE: Pin = Pin { plain: 0x13402, debug: 0x13C4C };

/// `EditorRaster_OJZ_Act1_ojz_sec3_shimmer`. tests: act_descriptor_port
pub const EDITOR_RASTER_OJZ_ACT1_OJZ_SEC3_SHIMMER: Pin = Pin { plain: 0x133D4, debug: 0x13C1E };

/// `EditorCycle_OJZ_Act1_ojz_sec3_shimmer`.
pub const EDITOR_CYCLE_OJZ_ACT1_OJZ_SEC3_SHIMMER: Pin = Pin { plain: 0x13430, debug: 0x13C7A };

/// `CrossoverTable`. tests: act_descriptor_port
pub const CROSSOVER_TABLE: Pin = Pin { plain: 0x700AC, debug: 0x7096C };

/// `Effects_InstallPreset`. tests: parallax_port
pub const EFFECTS_INSTALL_PRESET: Pin = Pin { plain: 0x7424, debug: 0x8810 };

/// `Raster_GetChannelBand`. tests: parallax_port
pub const RASTER_GET_CHANNEL_BAND: Pin = Pin { plain: 0x6F1A, debug: 0x8306 };

/// `TestStatic_Main`. tests: objdef_port
pub const TEST_STATIC_MAIN: Pin = Pin { plain: 0x12312, debug: 0x124F2 };

/// `TestSolid_Init`. tests: objdef_port
pub const TEST_SOLID_INIT: Pin = Pin { plain: 0x12316, debug: 0x12832 };

/// `TestEnemy_Init` — debug-shape consumer only (`debug_only`). tests: objdef_port
pub const TEST_ENEMY_INIT: u32 = 0x127EA;

/// `TestParent` — debug-shape consumer only (`debug_only`). tests: objdef_port
pub const TEST_PARENT_LABEL: u32 = 0x12984;

/// `Map_TestObj`. tests: objdef_port
pub const MAP_TEST_OBJ: Pin = Pin { plain: 0x29836, debug: 0x2A0EE };

/// `Map_Sonic`. tests: test_g1_objects_port
pub const MAP_SONIC: Pin = Pin { plain: 0x701AC, debug: 0x70A6C };

/// `DPLC_Sonic`. tests: test_g1_objects_port
pub const DPLC_SONIC: Pin = Pin { plain: 0x71E2C, debug: 0x726EC };

/// `Art_Sonic`. tests: test_g1_objects_port
pub const ART_SONIC: Pin = Pin { plain: 0x726F0, debug: 0x72FB0 };

/// `CreateEffect_Normal`. tests: test_g2_objects_port
pub const CREATE_EFFECT_NORMAL: Pin = Pin { plain: 0x4468, debug: 0x532C };

/// `CreateChild_Normal`. tests: test_g3_objects_port
pub const CREATE_CHILD_NORMAL: Pin = Pin { plain: 0x423E, debug: 0x5052 };

/// `DeleteChildren`. tests: test_g3_objects_port
pub const DELETE_CHILDREN: Pin = Pin { plain: 0x444A, debug: 0x530E };

/// `GetSineCosine`. tests: test_g3_objects_port
pub const GET_SINE_COSINE: Pin = Pin { plain: 0x2826, debug: 0x2AD8 };

/// `EntryPoint`. tests: m1c_vector_table
pub const ENTRY_POINT: Pin = Pin { plain: 0x200, debug: 0x200 };

/// `BusError` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const BUS_ERROR: u32 = 0xA714C;

/// `AddressError` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ADDRESS_ERROR: u32 = 0xA7164;

/// `IllegalInstr` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ILLEGAL_INSTR: u32 = 0xA7180;

/// `ZeroDivide` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ZERO_DIVIDE: u32 = 0xA71A2;

/// `ChkInstr` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const CHK_INSTR: u32 = 0xA71BC;

/// `TrapvInstr` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const TRAPV_INSTR: u32 = 0xA71DA;

/// `PrivilegeViol` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const PRIVILEGE_VIOL: u32 = 0xA71FA;

/// `Trace` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const TRACE: u32 = 0xA721C;

/// `Line1010Emu` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const LINE1010_EMU: u32 = 0xA7230;

/// `Line1111Emu` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const LINE1111_EMU: u32 = 0xA7250;

/// `ErrorExcept` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ERROR_EXCEPT: u32 = 0xA7270;

/// `ErrorTrap` — debug-shape consumer only (`debug_only`). tests: vectors_port
pub const ERROR_TRAP: u32 = 0xA728E;

/// `VBlank_Handler`. tests: m1c_vector_table
pub const V_BLANK_HANDLER: Pin = Pin { plain: 0x222E, debug: 0x231A };

/// `HBlank_Vector_Slot`. tests: hblank_port, m1c_vector_table
pub const H_BLANK_VECTOR_SLOT: Pin = Pin { plain: 0xFFFFB56A, debug: 0xFFFFB5F8 };

/// `VDP_Shadow_Table`. tests: vdp_init_port
pub const VDP_SHADOW_TABLE: Pin = Pin { plain: 0xFFFF800E, debug: 0xFFFF800E };

/// `BootData_VDPRegs`. tests: vdp_init_port
pub const BOOT_DATA_VDP_REGS: Pin = Pin { plain: 0x3B2, debug: 0x3BA };

/// `Ctrl_1_Held`. tests: controllers_port
pub const CTRL_1_HELD: Pin = Pin { plain: 0xFFFF8028, debug: 0xFFFF8028 };

/// `Ctrl_1_Held_Raw`. tests: controllers_port
pub const CTRL_1_HELD_RAW: Pin = Pin { plain: 0xFFFFB958, debug: 0xFFFFB9E6 };

/// `Ctrl_2_Held`. tests: vblank_port
pub const CTRL_2_HELD: Pin = Pin { plain: 0xFFFF802A, debug: 0xFFFF802A };

/// `Ctrl_1_Ext_Held`. tests: vblank_port
pub const CTRL_1_EXT_HELD: Pin = Pin { plain: 0xFFFF802E, debug: 0xFFFF802E };

/// `Ctrl_2_Ext_Held`. tests: vblank_port
pub const CTRL_2_EXT_HELD: Pin = Pin { plain: 0xFFFF8030, debug: 0xFFFF8030 };

/// `Ctrl_2_Held_Raw`. tests: vblank_port
pub const CTRL_2_HELD_RAW: Pin = Pin { plain: 0xFFFFB959, debug: 0xFFFFB9E7 };

/// `Ctrl_1_Ext_Held_Raw`. tests: vblank_port
pub const CTRL_1_EXT_HELD_RAW: Pin = Pin { plain: 0xFFFFB95A, debug: 0xFFFFB9E8 };

/// `Ctrl_2_Ext_Held_Raw`. tests: vblank_port
pub const CTRL_2_EXT_HELD_RAW: Pin = Pin { plain: 0xFFFFB95B, debug: 0xFFFFB9E9 };

/// `VSync_Wait`. tests: game_loop_port, load_art_port
pub const V_SYNC_WAIT: Pin = Pin { plain: 0x23E4, debug: 0x24D8 };

/// `Sound_DrainSfxRing`. tests: game_loop_port, load_art_port
pub const SOUND_DRAIN_SFX_RING: Pin = Pin { plain: 0x8024, debug: 0xAFB8 };

/// `Game_State`. tests: game_loop_port, load_art_port
pub const GAME_STATE: Pin = Pin { plain: 0xFFFF8008, debug: 0xFFFF8008 };

/// `Input_Tick`. tests: game_loop_port, game_debug_port
pub const INPUT_TICK: Pin = Pin { plain: 0x256E, debug: 0x2668 };

/// `Cache_Left_Col`. tests: collision_lookup_port, section_port
pub const CACHE_LEFT_COL: Pin = Pin { plain: 0xFFFFAD18, debug: 0xFFFFADA6 };

/// `Draw_TileColumn`. tests: section_port
pub const DRAW_TILE_COLUMN: Pin = Pin { plain: 0x458E, debug: 0x5452 };

/// `Draw_TileRow_FromCache`. tests: section_port
pub const DRAW_TILE_ROW_FROM_CACHE: Pin = Pin { plain: 0x46E2, debug: 0x55FC };

/// `EntityWindow_Init`. tests: section_port
pub const ENTITY_WINDOW_INIT: Pin = Pin { plain: 0x3CE2, debug: 0x4A06 };

/// `Section_Plane_Dirty`. tests: section_port
pub const SECTION_PLANE_DIRTY: Pin = Pin { plain: 0xFFFFAD8C, debug: 0xFFFFAE1A };

/// `Section_Right_Col_Written`. tests: section_port
pub const SECTION_RIGHT_COL_WRITTEN: Pin = Pin { plain: 0xFFFFAD8E, debug: 0xFFFFAE1C };

/// `Section_Left_Col_Written`. tests: section_port
pub const SECTION_LEFT_COL_WRITTEN: Pin = Pin { plain: 0xFFFFAD90, debug: 0xFFFFAE1E };

/// `Section_Top_Row_Written`. tests: section_port
pub const SECTION_TOP_ROW_WRITTEN: Pin = Pin { plain: 0xFFFFAD88, debug: 0xFFFFAE16 };

/// `Section_Bottom_Row_Written`. tests: section_port
pub const SECTION_BOTTOM_ROW_WRITTEN: Pin = Pin { plain: 0xFFFFAD8A, debug: 0xFFFFAE18 };

/// `Cache_Head_Col`. tests: section_port
pub const CACHE_HEAD_COL: Pin = Pin { plain: 0xFFFFAD1A, debug: 0xFFFFADA8 };

/// `Cache_Top_Row`. tests: section_port
pub const CACHE_TOP_ROW: Pin = Pin { plain: 0xFFFFAD1C, debug: 0xFFFFADAA };

/// `Cache_Bottom_Row`. tests: section_port
pub const CACHE_BOTTOM_ROW: Pin = Pin { plain: 0xFFFFAD1E, debug: 0xFFFFADAC };

/// `Cache_Origin_Col`. tests: section_port
pub const CACHE_ORIGIN_COL: Pin = Pin { plain: 0xFFFFAD20, debug: 0xFFFFADAE };

/// `Cache_Origin_Row`. tests: section_port
pub const CACHE_ORIGIN_ROW: Pin = Pin { plain: 0xFFFFAD22, debug: 0xFFFFADB0 };

/// `Plane_Buffer_Ptr`. tests: section_port
pub const PLANE_BUFFER_PTR: Pin = Pin { plain: 0xFFFFAC04, debug: 0xFFFFAC92 };

/// `Plane_Buffer`. tests: plane_buffer_port
pub const PLANE_BUFFER_BASE: Pin = Pin { plain: 0xFFFFA604, debug: 0xFFFFA692 };

/// `Tile_Cache_Nametable`. tests: section_port
pub const TILE_CACHE_NAMETABLE: Pin = Pin { plain: 0xFFFF0000, debug: 0xFFFF0000 };

/// `Tile_Cache_Collision`. tests: tile_cache_port, collision_lookup_port
pub const TILE_CACHE_COLLISION: Pin = Pin { plain: 0xFFFF2580, debug: 0xFFFF2580 };

/// `Frame_Counter`. tests: tile_cache_port
pub const FRAME_COUNTER: Pin = Pin { plain: 0xFFFF8002, debug: 0xFFFF8002 };

/// `Logic_Tick`. tests: game_loop_port, bg_anim_port, tile_cache_port, parallax_port
pub const LOGIC_TICK: Pin = Pin { plain: 0xFFFF8004, debug: 0xFFFF8004 };

/// `Block_Stage_Keys`. tests: tile_cache_port
pub const BLOCK_STAGE_KEYS: Pin = Pin { plain: 0xFFFFAD46, debug: 0xFFFFADD4 };

/// `Block_Stage_Next`. tests: tile_cache_port
pub const BLOCK_STAGE_NEXT: Pin = Pin { plain: 0xFFFFAD86, debug: 0xFFFFAE14 };

/// `Block_Stage_Bucket`. tests: tile_cache_port
pub const BLOCK_STAGE_BUCKET: Pin = Pin { plain: 0xFFFF6842, debug: 0xFFFF6842 };

/// `Block_Stage_Chain`. tests: tile_cache_port
pub const BLOCK_STAGE_CHAIN: Pin = Pin { plain: 0xFFFF6942, debug: 0xFFFF6942 };

/// `Block_Stage_Buffers`. tests: tile_cache_port
pub const BLOCK_STAGE_BUFFERS: Pin = Pin { plain: 0xFFFF3842, debug: 0xFFFF3842 };

/// `Block_Stage_Ptrs`. tests: tile_cache_port
pub const BLOCK_STAGE_PTRS: Pin = Pin { plain: 0xFFFFB570, debug: 0xFFFFB5FE };

/// `Block_Stage_ZeroPage`. tests: tile_cache_port
pub const BLOCK_STAGE_ZERO_PAGE: Pin = Pin { plain: 0xFFFFB5F4, debug: 0xFFFFB682 };

/// `Cache_Fill_Last_Frame`. tests: tile_cache_port
pub const CACHE_FILL_LAST_FRAME: Pin = Pin { plain: 0xFFFFAD24, debug: 0xFFFFADB2 };

/// `Cache_Fill_Budget`. tests: tile_cache_port
pub const CACHE_FILL_BUDGET: Pin = Pin { plain: 0xFFFFAD2E, debug: 0xFFFFADBC };

/// `Cache_Fill_Resume_Col`. tests: tile_cache_port
pub const CACHE_FILL_RESUME_COL: Pin = Pin { plain: 0xFFFFAD26, debug: 0xFFFFADB4 };

/// `Cache_Fill_Resume_Row`. tests: tile_cache_port
pub const CACHE_FILL_RESUME_ROW: Pin = Pin { plain: 0xFFFFAD28, debug: 0xFFFFADB6 };

/// `Cache_Fill_RowResume_Row`. tests: tile_cache_port
pub const CACHE_FILL_ROW_RESUME_ROW: Pin = Pin { plain: 0xFFFFAD30, debug: 0xFFFFADBE };

/// `Cache_Fill_RowResume_Col`. tests: tile_cache_port
pub const CACHE_FILL_ROW_RESUME_COL: Pin = Pin { plain: 0xFFFFAD32, debug: 0xFFFFADC0 };

/// `Cache_Fill_Rows_Left`. tests: tile_cache_port
pub const CACHE_FILL_ROWS_LEFT: Pin = Pin { plain: 0xFFFFAD34, debug: 0xFFFFADC2 };

/// `Cache_Prev_Cam_Row`. tests: tile_cache_port
pub const CACHE_PREV_CAM_ROW: Pin = Pin { plain: 0xFFFFAD36, debug: 0xFFFFADC4 };

/// `Cache_Prev_Cam_X`. tests: tile_cache_port
pub const CACHE_PREV_CAM_X: Pin = Pin { plain: 0xFFFFAD38, debug: 0xFFFFADC6 };

/// `Cache_H_Pfx_Dir`. tests: tile_cache_port
pub const CACHE_H_PFX_DIR: Pin = Pin { plain: 0xFFFFAD3A, debug: 0xFFFFADC8 };

/// `Cache_H_Pfx_Accum`. tests: tile_cache_port
pub const CACHE_H_PFX_ACCUM: Pin = Pin { plain: 0xFFFFAD3C, debug: 0xFFFFADCA };

/// `Cache_Pfx_Row_Target`. tests: tile_cache_port
pub const CACHE_PFX_ROW_TARGET: Pin = Pin { plain: 0xFFFFAD3E, debug: 0xFFFFADCC };

/// `Cache_Pfx_Col_Target`. tests: tile_cache_port
pub const CACHE_PFX_COL_TARGET: Pin = Pin { plain: 0xFFFFAD40, debug: 0xFFFFADCE };

/// `Cache_Pfx_Skip_Armed`. tests: tile_cache_port
pub const CACHE_PFX_SKIP_ARMED: Pin = Pin { plain: 0xFFFFAD42, debug: 0xFFFFADD0 };

/// `Cache_Pfx_Lag_Flag`. tests: tile_cache_port
pub const CACHE_PFX_LAG_FLAG: Pin = Pin { plain: 0xFFFFAD44, debug: 0xFFFFADD2 };

/// `Block_Stage_Gen`. tests: tile_cache_port
pub const BLOCK_STAGE_GEN: Pin = Pin { plain: 0xFFFFB558, debug: 0xFFFFB5E6 };

/// `Pfx_Memo_Row`. tests: tile_cache_port
pub const PFX_MEMO_ROW: Pin = Pin { plain: 0xFFFFB55A, debug: 0xFFFFB5E8 };

/// `Pfx_Memo_L16`. tests: tile_cache_port
pub const PFX_MEMO_L16: Pin = Pin { plain: 0xFFFFB55C, debug: 0xFFFFB5EA };

/// `Pfx_Memo_H16`. tests: tile_cache_port
pub const PFX_MEMO_H16: Pin = Pin { plain: 0xFFFFB55E, debug: 0xFFFFB5EC };

/// `Pfx_Memo_Gen`. tests: tile_cache_port
pub const PFX_MEMO_GEN: Pin = Pin { plain: 0xFFFFB560, debug: 0xFFFFB5EE };

/// `Cs_Memo_Col`. tests: tile_cache_port
pub const CS_MEMO_COL: Pin = Pin { plain: 0xFFFFB562, debug: 0xFFFFB5F0 };

/// `Cs_Memo_T16`. tests: tile_cache_port
pub const CS_MEMO_T16: Pin = Pin { plain: 0xFFFFB564, debug: 0xFFFFB5F2 };

/// `Cs_Memo_B16`. tests: tile_cache_port
pub const CS_MEMO_B16: Pin = Pin { plain: 0xFFFFB566, debug: 0xFFFFB5F4 };

/// `Cs_Memo_Gen`. tests: tile_cache_port
pub const CS_MEMO_GEN: Pin = Pin { plain: 0xFFFFB568, debug: 0xFFFFB5F6 };

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
pub const S4_LZ_DECOMPRESS_DICT: Pin = Pin { plain: 0x26B6, debug: 0x2860 };

/// `Player_1`. tests: collision_port, rings_port
pub const PLAYER_1: Pin = Pin { plain: 0xFFFF8EC8, debug: 0xFFFF8F56 };

/// `Cheat_Flags`. tests: test_g4_final_objects_port, test_p1_player_port
pub const CHEAT_FLAGS: Pin = Pin { plain: 0xFFFFB9DC, debug: 0xFFFFE312 };

/// `Dynamic_Slots`. tests: collision_port
pub const DYNAMIC_SLOTS: Pin = Pin { plain: 0xFFFF8F68, debug: 0xFFFF8FF6 };

/// `Ring_Buffer`. tests: rings_port
pub const RING_BUFFER: Pin = Pin { plain: 0xFFFFADFA, debug: 0xFFFFAE88 };

/// `Ring_Count`. tests: rings_port
pub const RING_COUNT: Pin = Pin { plain: 0xFFFFB0FA, debug: 0xFFFFB188 };

/// `Ring_HighWater`. tests: rings_port
pub const RING_HIGH_WATER: Pin = Pin { plain: 0xFFFFB0FB, debug: 0xFFFFB189 };

/// `Ring_Add_Dropped`. tests: rings_port
pub const RING_ADD_DROPPED: Pin = Pin { plain: 0xFFFFB0FC, debug: 0xFFFFB18A };

/// `Ring_Counter`. tests: rings_port
pub const RING_COUNTER: Pin = Pin { plain: 0xFFFFB166, debug: 0xFFFFB1F4 };

/// `Ring_Anim_Frame`. tests: rings_port
pub const RING_ANIM_FRAME: Pin = Pin { plain: 0xFFFFB168, debug: 0xFFFFB1F6 };

/// `Ring_Anim_Timer`. tests: rings_port
pub const RING_ANIM_TIMER: Pin = Pin { plain: 0xFFFFB169, debug: 0xFFFFB1F7 };

/// `Camera_X`. tests: rings_port, section_port, camera_port, bg_anim_port
pub const CAMERA_X: Pin = Pin { plain: 0xFFFFA5F6, debug: 0xFFFFA684 };

/// `Camera_Y`. tests: rings_port, section_port, camera_port, bg_anim_port
pub const CAMERA_Y: Pin = Pin { plain: 0xFFFFA5FA, debug: 0xFFFFA688 };

/// `Camera_Target`. tests: camera_port, test_g4_final_objects_port, test_t1_harness_states_port
pub const CAMERA_TARGET: Pin = Pin { plain: 0xFFFFAD12, debug: 0xFFFFADA0 };

/// `Camera_Curl_Offset`. tests: camera_port, test_p1_player_port
pub const CAMERA_CURL_OFFSET: Pin = Pin { plain: 0xFFFFAD14, debug: 0xFFFFADA2 };

/// `Camera_Deadzone_Base`. tests: camera_port
pub const CAMERA_DEADZONE_BASE: Pin = Pin { plain: 0xFFFFAD08, debug: 0xFFFFAD96 };

/// `Camera_Pan_Offset`. tests: camera_port
pub const CAMERA_PAN_OFFSET: Pin = Pin { plain: 0xFFFFAD0C, debug: 0xFFFFAD9A };

/// `Camera_Hold_Frames`. tests: camera_port
pub const CAMERA_HOLD_FRAMES: Pin = Pin { plain: 0xFFFFAD16, debug: 0xFFFFADA4 };

/// `Camera_Art_Hold`. tests: camera_port, tile_cache_port
pub const CAMERA_ART_HOLD: Pin = Pin { plain: 0xFFFFAD17, debug: 0xFFFFADA5 };

/// `Dbg_Cam_Clamp_Frames` — debug-shape consumer only (`debug_only`). tests: camera_port
pub const DBG_CAM_CLAMP_FRAMES: u32 = 0xFFFF8EFC;

/// `Camera_X_Max`. tests: camera_port
pub const CAMERA_X_MAX: Pin = Pin { plain: 0xFFFFAD0E, debug: 0xFFFFAD9C };

/// `Camera_Y_Max`. tests: camera_port
pub const CAMERA_Y_MAX: Pin = Pin { plain: 0xFFFFAD10, debug: 0xFFFFAD9E };

/// `BgAnim_LastStep`. tests: bg_anim_port
pub const BG_ANIM_LAST_STEP: Pin = Pin { plain: 0xFFFF8E5E, debug: 0xFFFF8E5E };

/// `BgAnim_Table`. tests: bg_anim_port
pub const BG_ANIM_TABLE: Pin = Pin { plain: 0x27808, debug: 0x280C0 };

/// `Camera_X_Biased`. tests: sprites_port
pub const CAMERA_X_BIASED: Pin = Pin { plain: 0xFFFFA5FE, debug: 0xFFFFA68C };

/// `Camera_Y_Biased`. tests: sprites_port
pub const CAMERA_Y_BIASED: Pin = Pin { plain: 0xFFFFA600, debug: 0xFFFFA68E };

/// `Collected_MarkRing`. tests: rings_port
pub const COLLECTED_MARK_RING: Pin = Pin { plain: 0x39A6, debug: 0x43AE };

/// `EntityWindow_EntryForSection`. tests: rings_port
pub const ENTITY_WINDOW_ENTRY_FOR_SECTION: Pin = Pin { plain: 0x3BC2, debug: 0x4890 };

/// `EntityLoaded_Clear`. tests: rings_port
pub const ENTITY_LOADED_CLEAR: Pin = Pin { plain: 0x3BAE, debug: 0x481A };

/// `Sound_PlayRing`. tests: rings_port
pub const SOUND_PLAY_RING: Pin = Pin { plain: 0x8074, debug: 0xB008 };

/// `MDDBG__ErrorHandler` — debug-shape consumer only (`debug_only`). tests: rings_port
pub const MDDBG_ERROR_HANDLER: u32 = 0xA72A6;

/// `MDDBG__ErrorHandler_PagesController` — debug-shape consumer only (`debug_only`). tests: rings_port
pub const MDDBG_ERROR_HANDLER_PAGES_CONTROLLER: u32 = 0xA806C;

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
pub const ACT_ART_BUDGET: Pin = Pin { plain: 0xFFFFB954, debug: 0xFFFFB9E2 };

/// `Art_Budget_Remaining`. tests: load_art_port
pub const ART_BUDGET_REMAINING: Pin = Pin { plain: 0xFFFFB956, debug: 0xFFFFB9E4 };

/// `PageIn_Pool_Pages`. tests: load_art_port
pub const PAGE_IN_POOL_PAGES: Pin = Pin { plain: 0xFFFFB948, debug: 0xFFFFB9D6 };

/// `PageIn_Bulk_Drain`. tests: load_art_port
pub const PAGE_IN_BULK_DRAIN: Pin = Pin { plain: 0xFFFFB943, debug: 0xFFFFB9D1 };

/// `PageIn_Fully_Resident`. tests: load_art_port
pub const PAGE_IN_FULLY_RESIDENT: Pin = Pin { plain: 0xFFFFB94A, debug: 0xFFFFB9D8 };

/// `Block_Stage_Maps`. tests: tile_cache_port
pub const BLOCK_STAGE_MAPS: Pin = Pin { plain: 0xFFFFB5B0, debug: 0xFFFFB63E };

/// `Cache_Cur_LocalMap`. tests: tile_cache_port
pub const CACHE_CUR_LOCAL_MAP: Pin = Pin { plain: 0xFFFFB5F0, debug: 0xFFFFB67E };

/// `PageCache_Direct_Map`. tests: load_art_port
pub const PAGE_CACHE_DIRECT_MAP: Pin = Pin { plain: 0xFFFFB94B, debug: 0xFFFFB9D9 };

/// `Page_Table`. tests: load_art_port
pub const PAGE_TABLE: Pin = Pin { plain: 0xFFFF699C, debug: 0xFFFF699C };

/// `Dbg_DMA_Enq_Capped` — debug-shape consumer only (`debug_only`). tests: bg_anim_port, dma_queue_port, dplc_port
pub const DBG_DMA_ENQ_CAPPED: u32 = 0xFFFF8ED2;

/// `DMA_Overflow_Count` — debug-shape consumer only (`debug_only`). tests: dma_queue_port
pub const DMA_OVERFLOW_COUNT: u32 = 0xFFFF8ED0;

/// `Art_Staging_Buffer`. tests: load_art_port
pub const ART_STAGING_BUFFER: Pin = Pin { plain: 0xFFFF6B34, debug: 0xFFFF6B34 };

/// `S4LZ_Decompress`. tests: load_art_port
pub const S4_LZ_DECOMPRESS: Pin = Pin { plain: 0x26BA, debug: 0x28B8 };

/// `QueueDMA_Critical`. tests: load_art_port
pub const QUEUE_DMA_CRITICAL: Pin = Pin { plain: 0x1D48, debug: 0x1E28 };

/// `BG_Init`. tests: load_art_port
pub const BG_INIT: Pin = Pin { plain: 0x7D46, debug: 0x9C34 };

/// `QueueDMA_Important`. tests: dplc_port
pub const QUEUE_DMA_IMPORTANT: Pin = Pin { plain: 0x1D52, debug: 0x1E32 };

/// `QueueDMA_Deferrable`. tests: dplc_port
pub const QUEUE_DMA_DEFERRABLE: Pin = Pin { plain: 0x1D5C, debug: 0x1E3C };

/// `Object_RAM`. tests: core_port
pub const OBJECT_RAM: Pin = Pin { plain: 0xFFFF8EC8, debug: 0xFFFF8F56 };

/// `System_Slots`. tests: core_port
pub const SYSTEM_SLOTS: Pin = Pin { plain: 0xFFFF9BE8, debug: 0xFFFF9C76 };

/// `Effect_Slots`. tests: core_port
pub const EFFECT_SLOTS: Pin = Pin { plain: 0xFFFF9E68, debug: 0xFFFF9EF6 };

/// `Game_Paused`. tests: core_port
pub const GAME_PAUSED: Pin = Pin { plain: 0xFFFFA602, debug: 0xFFFFA690 };

/// `Object_RAM_End`. tests: core_port
pub const OBJECT_RAM_END: Pin = Pin { plain: 0xFFFFA368, debug: 0xFFFFA3F6 };

/// `Dynamic_Free_Stack`. tests: core_port
pub const DYNAMIC_FREE_STACK: Pin = Pin { plain: 0xFFFFA368, debug: 0xFFFFA3F6 };

/// `Dynamic_Free_SP`. tests: core_port
pub const DYNAMIC_FREE_SP: Pin = Pin { plain: 0xFFFFA3B8, debug: 0xFFFFA446 };

/// `Effect_Free_Stack`. tests: core_port
pub const EFFECT_FREE_STACK: Pin = Pin { plain: 0xFFFFA3BA, debug: 0xFFFFA448 };

/// `Effect_Free_SP`. tests: core_port
pub const EFFECT_FREE_SP: Pin = Pin { plain: 0xFFFFA3DA, debug: 0xFFFFA468 };

/// `Dynamic_Live`. tests: core_port
pub const DYNAMIC_LIVE: Pin = Pin { plain: 0xFFFFB4F2, debug: 0xFFFFB580 };

/// `Dynamic_Live_Count`. tests: core_port
pub const DYNAMIC_LIVE_COUNT: Pin = Pin { plain: 0xFFFFB542, debug: 0xFFFFB5D0 };

/// `Dynamic_Live_Dirty`. tests: core_port
pub const DYNAMIC_LIVE_DIRTY: Pin = Pin { plain: 0xFFFFB544, debug: 0xFFFFB5D2 };

/// `Dynamic_Live_Walking` — debug-shape consumer only (`debug_only`). tests: core_port, collision_port, entity_window_port
pub const DYNAMIC_LIVE_WALKING: u32 = 0xFFFFB5D3;

/// `Dynamic_Live_Pending`. tests: core_port
pub const DYNAMIC_LIVE_PENDING: Pin = Pin { plain: 0xFFFFB546, debug: 0xFFFFB5D4 };

/// `Dynamic_Live_Pending_Count`. tests: core_port
pub const DYNAMIC_LIVE_PENDING_COUNT: Pin = Pin { plain: 0xFFFFB556, debug: 0xFFFFB5E4 };

/// `DeleteObject`. tests: animate_port, children_port
pub const DELETE_OBJECT: Pin = Pin { plain: 0x2D90, debug: 0x3042 };

/// `DrawRings`. tests: sprites_port
pub const DRAW_RINGS: Pin = Pin { plain: 0x37EC, debug: 0x4188 };

/// `Sprite_Table_Buffer`. tests: sprites_port
pub const SPRITE_TABLE_BUFFER: Pin = Pin { plain: 0xFFFF8298, debug: 0xFFFF8298 };

/// `Sprite_Table_Dirty`. tests: sprites_port
pub const SPRITE_TABLE_DIRTY: Pin = Pin { plain: 0xFFFF8518, debug: 0xFFFF8518 };

/// `Sprite_Emit_Active`. tests: sprites_port, buffers_port
pub const SPRITE_EMIT_ACTIVE: Pin = Pin { plain: 0xFFFF8519, debug: 0xFFFF8519 };

/// `Sprite_Bands`. tests: sprites_port
pub const SPRITE_BANDS: Pin = Pin { plain: 0xFFFFA3DC, debug: 0xFFFFA46A };

/// `Sprite_Band_Counts`. tests: sprites_port
pub const SPRITE_BAND_COUNTS: Pin = Pin { plain: 0xFFFFA5DC, debug: 0xFFFFA66A };

/// `Sprites_Rendered`. tests: sprites_port
pub const SPRITES_RENDERED: Pin = Pin { plain: 0xFFFFA5E4, debug: 0xFFFFA672 };

/// `Sprite_Cycle_Counter`. tests: sprites_port
pub const SPRITE_CYCLE_COUNTER: Pin = Pin { plain: 0xFFFFA5E6, debug: 0xFFFFA674 };

/// `Sprite_Owner` — debug-shape consumer only (`debug_only`). tests: sprites_port
pub const SPRITE_OWNER: u32 = 0xFFFFE26E;

/// `SpriteMask_Y`. tests: sprites_port
pub const SPRITE_MASK_Y: Pin = Pin { plain: 0xFFFFA5E8, debug: 0xFFFFA676 };

/// `SpriteMask_Height`. tests: sprites_port
pub const SPRITE_MASK_HEIGHT: Pin = Pin { plain: 0xFFFFA5EA, debug: 0xFFFFA678 };

/// `SpriteMask_After_Band`. tests: sprites_port
pub const SPRITE_MASK_AFTER_BAND: Pin = Pin { plain: 0xFFFFA5EC, debug: 0xFFFFA67A };

/// `Scanline_Band_Sprites`. tests: sprites_port
pub const SCANLINE_BAND_SPRITES: Pin = Pin { plain: 0xFFFFA5EE, debug: 0xFFFFA67C };

/// `Sound_PlaySFX`. tests: animate_port
pub const SOUND_PLAY_SFX: Pin = Pin { plain: 0x7FDE, debug: 0xAF2C };

/// `ObjectMoveX`. tests: test_g4_final_objects_port
pub const OBJECT_MOVE_X: Pin = Pin { plain: 0x2F9C, debug: 0x3696 };

/// `ObjCodeBase`. tests: test_objects_port
pub const OBJ_CODE_BASE: Pin = Pin { plain: 0x10000, debug: 0x10000 };

/// `Draw_Sprite`. tests: test_objects_port
pub const DRAW_SPRITE: Pin = Pin { plain: 0x2FCC, debug: 0x36C6 };

/// `ObjectMove`. tests: test_objects_port
pub const OBJECT_MOVE: Pin = Pin { plain: 0x2F82, debug: 0x367C };

/// `Ring_Sfx_Speaker`. tests: sound_api_port
pub const RING_SFX_SPEAKER: Pin = Pin { plain: 0xFFFFB436, debug: 0xFFFFB4C4 };

/// `Sfx_Ring_Buf`. tests: sound_api_port
pub const SFX_RING_BUF: Pin = Pin { plain: 0xFFFFB438, debug: 0xFFFFB4C6 };

/// `Sfx_Ring_Wr`. tests: sound_api_port
pub const SFX_RING_WR: Pin = Pin { plain: 0xFFFFB440, debug: 0xFFFFB4CE };

/// `Sfx_Ring_Rd`. tests: sound_api_port
pub const SFX_RING_RD: Pin = Pin { plain: 0xFFFFB441, debug: 0xFFFFB4CF };

/// `SongTable`. tests: sound_api_port
pub const SONG_TABLE: Pin = Pin { plain: 0xA3B10, debug: 0xA5550 };

/// `SongPatchTable`. tests: sound_api_port
pub const SONG_PATCH_TABLE: Pin = Pin { plain: 0xA3B14, debug: 0xA555C };

/// `OJZ_Palette`. tests: act_descriptor_port
pub const OJZ_PALETTE: Pin = Pin { plain: 0x22F86, debug: 0x2383E };

/// `OJZ_Act1_BG_Layout`. tests: act_descriptor_port
pub const OJZ_ACT1_BG_LAYOUT: Pin = Pin { plain: 0x23006, debug: 0x238BE };

/// `OJZ_Act1_BG_Tiles`. tests: act_descriptor_port
pub const OJZ_ACT1_BG_TILES: Pin = Pin { plain: 0x25006, debug: 0x258BE };

/// `ParallaxConfig_OJZ_Default`. tests: act_descriptor_port
pub const PARALLAX_CONFIG_OJZ_DEFAULT: Pin = Pin { plain: 0x124BA, debug: 0x12D04 };

/// `OJZ_Act_Pool_PageTable`. tests: act_descriptor_port
pub const OJZ_ACT_POOL_PAGE_TABLE: Pin = Pin { plain: 0x169EE, debug: 0x172A6 };

/// `OJZ_Sec_LocalMaps`. tests: act_descriptor_port
pub const OJZ_SEC_LOCAL_MAPS: Pin = Pin { plain: 0x22F62, debug: 0x2381A };

/// `OJZ_Sec0_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC0_BLOCKS: Pin = Pin { plain: 0x16CB8, debug: 0x17570 };

/// `OJZ_Sec1_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC1_BLOCKS: Pin = Pin { plain: 0x18E28, debug: 0x196E0 };

/// `OJZ_Sec2_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC2_BLOCKS: Pin = Pin { plain: 0x1A1A4, debug: 0x1AA5C };

/// `OJZ_Sec3_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC3_BLOCKS: Pin = Pin { plain: 0x1B93C, debug: 0x1C1F4 };

/// `OJZ_Sec4_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC4_BLOCKS: Pin = Pin { plain: 0x1A1A4, debug: 0x1AA5C };

/// `OJZ_Sec5_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC5_BLOCKS: Pin = Pin { plain: 0x1CA88, debug: 0x1D340 };

/// `OJZ_Sec6_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC6_BLOCKS: Pin = Pin { plain: 0x1D8AE, debug: 0x1E166 };

/// `OJZ_Sec7_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC7_BLOCKS: Pin = Pin { plain: 0x1F4AE, debug: 0x1FD66 };

/// `OJZ_Sec8_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC8_BLOCKS: Pin = Pin { plain: 0x20722, debug: 0x20FDA };

/// `OJZ_Sec0_Objects`. tests: act_descriptor_port
pub const OJZ_SEC0_OBJECTS: Pin = Pin { plain: 0x139C8, debug: 0x14280 };

/// `OJZ_Sec0_Rings`. tests: act_descriptor_port
pub const OJZ_SEC0_RINGS: Pin = Pin { plain: 0x139D0, debug: 0x14288 };

/// `OJZ_Sec0_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC0_TYPE_TABLE: Pin = Pin { plain: 0x139C2, debug: 0x1427A };

/// `OJZ_Sec1_Objects`. tests: act_descriptor_port
pub const OJZ_SEC1_OBJECTS: Pin = Pin { plain: 0x139FA, debug: 0x142B2 };

/// `OJZ_Sec1_Rings`. tests: act_descriptor_port
pub const OJZ_SEC1_RINGS: Pin = Pin { plain: 0x13A0E, debug: 0x142C6 };

/// `OJZ_Sec1_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC1_TYPE_TABLE: Pin = Pin { plain: 0x139F0, debug: 0x142A8 };

/// `OJZ_Sec2_Objects`. tests: act_descriptor_port
pub const OJZ_SEC2_OBJECTS: Pin = Pin { plain: 0x13A40, debug: 0x142F8 };

/// `OJZ_Sec2_Rings`. tests: act_descriptor_port
pub const OJZ_SEC2_RINGS: Pin = Pin { plain: 0x13A4E, debug: 0x14306 };

/// `OJZ_Sec2_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC2_TYPE_TABLE: Pin = Pin { plain: 0x13A36, debug: 0x142EE };

/// `OJZ_Sec3_Objects`. tests: act_descriptor_port
pub const OJZ_SEC3_OBJECTS: Pin = Pin { plain: 0x13A84, debug: 0x1433C };

/// `OJZ_Sec3_Rings`. tests: act_descriptor_port
pub const OJZ_SEC3_RINGS: Pin = Pin { plain: 0x13A86, debug: 0x1433E };

/// `OJZ_Sec3_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC3_TYPE_TABLE: Pin = Pin { plain: 0x13A82, debug: 0x1433A };

/// `OJZ_Sec4_Objects`. tests: act_descriptor_port
pub const OJZ_SEC4_OBJECTS: Pin = Pin { plain: 0x13A8C, debug: 0x14344 };

/// `OJZ_Sec4_Rings`. tests: act_descriptor_port
pub const OJZ_SEC4_RINGS: Pin = Pin { plain: 0x13A8E, debug: 0x14346 };

/// `OJZ_Sec4_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC4_TYPE_TABLE: Pin = Pin { plain: 0x13A8A, debug: 0x14342 };

/// `OJZ_Sec5_Objects`. tests: act_descriptor_port
pub const OJZ_SEC5_OBJECTS: Pin = Pin { plain: 0x13AC4, debug: 0x1437C };

/// `OJZ_Sec5_Rings`. tests: act_descriptor_port
pub const OJZ_SEC5_RINGS: Pin = Pin { plain: 0x13AC6, debug: 0x1437E };

/// `OJZ_Sec5_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC5_TYPE_TABLE: Pin = Pin { plain: 0x13AC2, debug: 0x1437A };

/// `OJZ_Sec6_Objects`. tests: act_descriptor_port
pub const OJZ_SEC6_OBJECTS: Pin = Pin { plain: 0x13AEC, debug: 0x143A4 };

/// `OJZ_Sec6_Rings`. tests: act_descriptor_port
pub const OJZ_SEC6_RINGS: Pin = Pin { plain: 0x13AEE, debug: 0x143A6 };

/// `OJZ_Sec6_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC6_TYPE_TABLE: Pin = Pin { plain: 0x13AEA, debug: 0x143A2 };

/// `OJZ_Sec7_Objects`. tests: act_descriptor_port
pub const OJZ_SEC7_OBJECTS: Pin = Pin { plain: 0x13AF4, debug: 0x143AC };

/// `OJZ_Sec7_Rings`. tests: act_descriptor_port
pub const OJZ_SEC7_RINGS: Pin = Pin { plain: 0x13AF6, debug: 0x143AE };

/// `OJZ_Sec7_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC7_TYPE_TABLE: Pin = Pin { plain: 0x13AF2, debug: 0x143AA };

/// `OJZ_Sec8_Objects`. tests: act_descriptor_port
pub const OJZ_SEC8_OBJECTS: Pin = Pin { plain: 0x13B1C, debug: 0x143D4 };

/// `OJZ_Sec8_Rings`. tests: act_descriptor_port
pub const OJZ_SEC8_RINGS: Pin = Pin { plain: 0x13B1E, debug: 0x143D6 };

/// `OJZ_Sec8_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC8_TYPE_TABLE: Pin = Pin { plain: 0x13B1A, debug: 0x143D2 };

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
pub const CAMERA_Y_COARSE_PREV: Pin = Pin { plain: 0xFFFFB276, debug: 0xFFFFB304 };

/// `Current_Act_Ptr`. tests: entity_window_port, section_port
pub const CURRENT_ACT_PTR: Pin = Pin { plain: 0xFFFFB432, debug: 0xFFFFB4C0 };

/// `Entity_Window_Active`. tests: entity_window_port
pub const ENTITY_WINDOW_ACTIVE: Pin = Pin { plain: 0xFFFFB16A, debug: 0xFFFFB1F8 };

/// `Entity_Window_Anchor`. tests: entity_window_port
pub const ENTITY_WINDOW_ANCHOR: Pin = Pin { plain: 0xFFFFB16C, debug: 0xFFFFB1FA };

/// `Entity_Window_OriginX`. tests: entity_window_port
pub const ENTITY_WINDOW_ORIGIN_X: Pin = Pin { plain: 0xFFFFB16E, debug: 0xFFFFB1FC };

/// `Entity_Window_OriginY`. tests: entity_window_port
pub const ENTITY_WINDOW_ORIGIN_Y: Pin = Pin { plain: 0xFFFFB170, debug: 0xFFFFB1FE };

/// `Entity_Window_Center_ID`. tests: entity_window_port
pub const ENTITY_WINDOW_CENTER_ID: Pin = Pin { plain: 0xFFFFB16B, debug: 0xFFFFB1F9 };

/// `Entity_Scan_State`. tests: entity_window_port
pub const ENTITY_SCAN_STATE: Pin = Pin { plain: 0xFFFFB0FE, debug: 0xFFFFB18C };

/// `Entity_Loaded_Masks`. tests: entity_window_port
pub const ENTITY_LOADED_MASKS: Pin = Pin { plain: 0xFFFFB172, debug: 0xFFFFB200 };

/// `Entity_Mask_Scratch`. tests: entity_window_port
pub const ENTITY_MASK_SCRATCH: Pin = Pin { plain: 0xFFFFB1F2, debug: 0xFFFFB280 };

/// `Ring_Collected_Window`. tests: entity_window_port
pub const RING_COLLECTED_WINDOW: Pin = Pin { plain: 0xFFFFB278, debug: 0xFFFFB306 };

/// `Ring_Collected_Park`. tests: entity_window_port
pub const RING_COLLECTED_PARK: Pin = Pin { plain: 0xFFFFB3AC, debug: 0xFFFFB43A };

/// `Collected_Park_Next`. tests: entity_window_port
pub const COLLECTED_PARK_NEXT: Pin = Pin { plain: 0xFFFFB430, debug: 0xFFFFB4BE };

/// `RingBuffer_Clear`. tests: entity_window_port
pub const RING_BUFFER_CLEAR: Pin = Pin { plain: 0x37DE, debug: 0x417A };

/// `RingBuffer_Remove`. tests: entity_window_port
pub const RING_BUFFER_REMOVE: Pin = Pin { plain: 0x37AA, debug: 0x4146 };

/// `Section_GetSecPtrXY`. tests: entity_window_port
pub const SECTION_GET_SEC_PTR_XY: Pin = Pin { plain: 0x5CDA, debug: 0x6F10 };

/// `Section_FlatIDXY`. tests: entity_window_port
pub const SECTION_FLAT_IDXY: Pin = Pin { plain: 0x5CC0, debug: 0x6EF6 };

/// `AllocDynamic`. tests: load_object_port, children_port
pub const ALLOC_DYNAMIC: Pin = Pin { plain: 0x2D12, debug: 0x2FC4 };

/// `AllocEffect`. tests: children_port
pub const ALLOC_EFFECT: Pin = Pin { plain: 0x2D76, debug: 0x3028 };

/// `Palette_Buffer`. tests: buffers_port
pub const PALETTE_BUFFER: Pin = Pin { plain: 0xFFFF8216, debug: 0xFFFF8216 };

/// `Hscroll_Buffer`. tests: buffers_port
pub const HSCROLL_BUFFER: Pin = Pin { plain: 0xFFFF851A, debug: 0xFFFF851A };

/// `Static_Pal_Line0`. tests: buffers_port
pub const STATIC_PAL_LINE0: Pin = Pin { plain: 0xFFFF8E66, debug: 0xFFFF8E66 };

/// `Static_Pal_Line1`. tests: buffers_port
pub const STATIC_PAL_LINE1: Pin = Pin { plain: 0xFFFF8E74, debug: 0xFFFF8E74 };

/// `Static_Pal_Line2`. tests: buffers_port
pub const STATIC_PAL_LINE2: Pin = Pin { plain: 0xFFFF8E82, debug: 0xFFFF8E82 };

/// `Static_Pal_Line3`. tests: buffers_port
pub const STATIC_PAL_LINE3: Pin = Pin { plain: 0xFFFF8E9E, debug: 0xFFFF8E9E };

/// `Static_Sprite_DMA`. tests: buffers_port
pub const STATIC_SPRITE_DMA: Pin = Pin { plain: 0xFFFF8EAC, debug: 0xFFFF8EAC };

/// `Static_Hscroll_Line`. tests: buffers_port
pub const STATIC_HSCROLL_LINE: Pin = Pin { plain: 0xFFFF8EBA, debug: 0xFFFF8EBA };

/// `Palette_Dirty`. tests: buffers_port, palette_port
pub const PALETTE_DIRTY: Pin = Pin { plain: 0xFFFF8296, debug: 0xFFFF8296 };

/// `Parallax_Active_Config`. tests: buffers_port
pub const PARALLAX_ACTIVE_CONFIG: Pin = Pin { plain: 0x6382, debug: 0x776E };

/// `Palette_Ship_Snap`. tests: buffers_port
pub const PALETTE_SHIP_SNAP: Pin = Pin { plain: 0xFFFFB95C, debug: 0xFFFFB9EA };

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
pub const LAG_FRAME_COUNT: u32 = 0xFFFF8ED4;

/// `DMA_Bytes_ThisFrame` — debug-shape consumer only (`debug_only`). tests: vblank_port
pub const DMA_BYTES_THIS_FRAME: u32 = 0xFFFF8EC8;

/// `PageIn_InFlight`. tests: game_loop_port
pub const PAGE_IN_IN_FLIGHT: Pin = Pin { plain: 0xFFFFB916, debug: 0xFFFFB9A4 };

/// `PageIn_Saved_PC`. tests: game_loop_port
pub const PAGE_IN_SAVED_PC: Pin = Pin { plain: 0xFFFFB910, debug: 0xFFFFB99E };

/// `PageIn_BankRegs`. tests: game_loop_port
pub const PAGE_IN_BANK_REGS: Pin = Pin { plain: 0x7748, debug: 0x8C9C };

/// `Dbg_PageIn_Preempts` — debug-shape consumer only (`debug_only`). tests: game_loop_port
pub const DBG_PAGE_IN_PREEMPTS: u32 = 0xFFFF8EEE;

/// `ZX0R_Decompress.__end`. tests: game_loop_port
pub const ZX0R_DECOMPRESS_END: Pin = Pin { plain: 0x2826, debug: 0x2AD8 };

/// `PageIn_Staging_Busy`. tests: game_loop_port, load_art_port
pub const PAGE_IN_STAGING_BUSY: Pin = Pin { plain: 0xFFFFB918, debug: 0xFFFFB9A6 };

/// `PageIn_Flush`. tests: load_art_port
pub const PAGE_IN_FLUSH: Pin = Pin { plain: 0x7810, debug: 0x8D6C };

/// `PageIn_Enqueue`. tests: load_art_port
pub const PAGE_IN_ENQUEUE: Pin = Pin { plain: 0x77D2, debug: 0x8D2E };

/// `PageIn_Pool_Table`. tests: load_art_port
pub const PAGE_IN_POOL_TABLE: Pin = Pin { plain: 0xFFFFB944, debug: 0xFFFFB9D2 };

/// `PageIn_Queue_Count`. tests: load_art_port
pub const PAGE_IN_QUEUE_COUNT: Pin = Pin { plain: 0xFFFFB91A, debug: 0xFFFFB9A8 };

/// `PageIn_Suspended`. tests: load_art_port
pub const PAGE_IN_SUSPENDED: Pin = Pin { plain: 0xFFFFB917, debug: 0xFFFFB9A5 };

/// `PageIn_Land_Pending`. tests: load_art_port
pub const PAGE_IN_LAND_PENDING: Pin = Pin { plain: 0xFFFFB919, debug: 0xFFFFB9A7 };

/// `PageCache_Init`. tests: load_art_port
pub const PAGE_CACHE_INIT: Pin = Pin { plain: 0x7860, debug: 0x8DBC };

/// `PageCache_AllocFrame`. tests: load_art_port
pub const PAGE_CACHE_ALLOC_FRAME: Pin = Pin { plain: 0x7910, debug: 0x8ED0 };

/// `PageCache_Publish`. tests: load_art_port
pub const PAGE_CACHE_PUBLISH: Pin = Pin { plain: 0x79CC, debug: 0x9090 };

/// `PageCache_PatchRun_Seq`. tests: tile_cache_port
pub const PAGE_CACHE_PATCH_RUN_SEQ: Pin = Pin { plain: 0x7A3A, debug: 0x9164 };

/// `PageCache_PatchRun_Col`. tests: tile_cache_port
pub const PAGE_CACHE_PATCH_RUN_COL: Pin = Pin { plain: 0x7B3E, debug: 0x93A4 };

/// `PageCache_Audit`. tests: tile_cache_port
pub const PAGE_CACHE_AUDIT: Pin = Pin { plain: 0x7D42, debug: 0x9724 };

/// `Cache_Art_Stall`. tests: tile_cache_port
pub const CACHE_ART_STALL: Pin = Pin { plain: 0xFFFFAD2A, debug: 0xFFFFADB8 };

/// `Page_Audit_Ticks` — debug-shape consumer only (`debug_only`). tests: tile_cache_port
pub const PAGE_AUDIT_TICKS: u32 = 0xFFFF8F02;

/// `Cache_Stall_Watchdog` — debug-shape consumer only (`debug_only`). tests: tile_cache_port
pub const CACHE_STALL_WATCHDOG: u32 = 0xFFFF8F00;

/// `Flush_VDP_Shadow`. tests: vblank_port
pub const FLUSH_VDP_SHADOW: Pin = Pin { plain: 0x1C02, debug: 0x1C8C };

/// `VInt_DrawLevel`. tests: vblank_port
pub const V_INT_DRAW_LEVEL: Pin = Pin { plain: 0x4834, debug: 0x57A4 };

/// `Vscroll_Write`. tests: vblank_port
pub const VSCROLL_WRITE: Pin = Pin { plain: 0x6394, debug: 0x7780 };

/// `Read_Controllers`. tests: vblank_port
pub const READ_CONTROLLERS: Pin = Pin { plain: 0x243E, debug: 0x2536 };

/// `Process_DMA_Critical`. tests: vblank_port
pub const PROCESS_DMA_CRITICAL: Pin = Pin { plain: 0x1E22, debug: 0x1F0E };

/// `Process_DMA_Important`. tests: vblank_port
pub const PROCESS_DMA_IMPORTANT: Pin = Pin { plain: 0x1EF0, debug: 0x1FDC };

/// `Process_DMA_Deferrable`. tests: vblank_port
pub const PROCESS_DMA_DEFERRABLE: Pin = Pin { plain: 0x1F04, debug: 0x1FF0 };

/// `Enqueue_Dirty_Buffers`. tests: vblank_port
pub const ENQUEUE_DIRTY_BUFFERS: Pin = Pin { plain: 0x203C, debug: 0x2128 };

/// `BootData`. tests: boot_port
pub const BOOT_DATA: Pin = Pin { plain: 0x398, debug: 0x3A0 };

/// `VInt_Level`. tests: boot_port
pub const V_INT_LEVEL: Pin = Pin { plain: 0x2276, debug: 0x2366 };

/// `BuildStaticDMA`. tests: boot_port
pub const BUILD_STATIC_DMA: Pin = Pin { plain: 0x1F78, debug: 0x2064 };

/// `Sound_Init`. tests: boot_port
pub const SOUND_INIT: Pin = Pin { plain: 0x7EDE, debug: 0xACC8 };

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
pub const P_STATE_GROUND: Pin = Pin { plain: 0x106CA, debug: 0x107E2 };

/// `PState_Roll`. tests: test_p1_player_port
pub const P_STATE_ROLL: Pin = Pin { plain: 0x1082E, debug: 0x10946 };

/// `PState_Spindash`. tests: test_p1_player_port
pub const P_STATE_SPINDASH: Pin = Pin { plain: 0x10E9E, debug: 0x10FB6 };

/// `PState_Air`. tests: test_p1_player_port
pub const P_STATE_AIR: Pin = Pin { plain: 0x10B54, debug: 0x10C6C };

/// `PState_Jump`. tests: test_p1_player_port
pub const P_STATE_JUMP: Pin = Pin { plain: 0x10B5C, debug: 0x10C74 };

/// `PState_RollJump`. tests: test_p1_player_port
pub const P_STATE_ROLL_JUMP: Pin = Pin { plain: 0x10B58, debug: 0x10C70 };

/// `PState_AirBall`. tests: test_p1_player_port
pub const P_STATE_AIR_BALL: Pin = Pin { plain: 0x10B54, debug: 0x10C6C };

/// `PState_Fly`. tests: test_p1_player_port
pub const P_STATE_FLY: Pin = Pin { plain: 0x10F3A, debug: 0x11052 };

/// `PState_Glide`. tests: test_p1_player_port
pub const P_STATE_GLIDE: Pin = Pin { plain: 0x1107C, debug: 0x11196 };

/// `PState_GlideFall`. tests: test_p1_player_port
pub const P_STATE_GLIDE_FALL: Pin = Pin { plain: 0x11214, debug: 0x1132E };

/// `PState_Slide`. tests: test_p1_player_port
pub const P_STATE_SLIDE: Pin = Pin { plain: 0x11260, debug: 0x1137A };

/// `PState_Climb`. tests: test_p1_player_port
pub const P_STATE_CLIMB: Pin = Pin { plain: 0x113A8, debug: 0x114C6 };

/// `PState_Ledge`. tests: test_p1_player_port
pub const P_STATE_LEDGE: Pin = Pin { plain: 0x11554, debug: 0x11672 };

/// `Player_SensorFloor`. tests: test_p1_player_port
pub const PLAYER_SENSOR_FLOOR: Pin = Pin { plain: 0x5B02, debug: 0x6D38 };

/// `Player_AtLedgeEdge`. tests: test_p1_player_port
pub const PLAYER_AT_LEDGE_EDGE: Pin = Pin { plain: 0x5C1C, debug: 0x6E52 };

/// `Player_SetState`. tests: test_p2_player_states_port
pub const PLAYER_SET_STATE: Pin = Pin { plain: 0x1042A, debug: 0x104EE };

/// `Player_SnapToSurface`. tests: test_p2_player_states_port
pub const PLAYER_SNAP_TO_SURFACE: Pin = Pin { plain: 0x10578, debug: 0x1063C };

/// `Player_SensorCeiling`. tests: test_p2_player_states_port
pub const PLAYER_SENSOR_CEILING: Pin = Pin { plain: 0x5B18, debug: 0x6D4E };

/// `Player_SensorWallDir`. tests: test_p2_player_states_port
pub const PLAYER_SENSOR_WALL_DIR: Pin = Pin { plain: 0x5BD2, debug: 0x6E08 };

/// `Player_SensorWallAt`. tests: test_p2_player_states_port
pub const PLAYER_SENSOR_WALL_AT: Pin = Pin { plain: 0x5BCA, debug: 0x6E00 };

/// `Collision_GetType`. tests: test_p4_player_sensors_port
pub const COLLISION_GET_TYPE: Pin = Pin { plain: 0x572E, debug: 0x6964 };

/// `SolidityTable`. tests: test_p4_player_sensors_port
pub const SOLIDITY_TABLE: Pin = Pin { plain: 0x6FFAC, debug: 0x7086C };

/// `AngleTable`. tests: test_p4_player_sensors_port
pub const ANGLE_TABLE: Pin = Pin { plain: 0x6FEAC, debug: 0x7076C };

/// `HeightMaps`. tests: test_p4_player_sensors_port
pub const HEIGHT_MAPS: Pin = Pin { plain: 0x6DEAC, debug: 0x6E76C };

/// `HeightMapsRot`. tests: test_p4_player_sensors_port
pub const HEIGHT_MAPS_ROT: Pin = Pin { plain: 0x6EEAC, debug: 0x6F76C };

/// `Character_ID`. tests: test_p1_player_port
pub const CHARACTER_ID: Pin = Pin { plain: 0xFFFFB9DE, debug: 0xFFFFE314 };

/// `Player_Chardef`. tests: test_p1_player_port
pub const PLAYER_CHARDEF: Pin = Pin { plain: 0xFFFFB9E0, debug: 0xFFFFE316 };

/// `Ability_None`. tests: test_p1_player_port
pub const ABILITY_NONE: Pin = Pin { plain: 0x11F8E, debug: 0x12116 };

/// `CharacterDefs`. tests: test_p1_player_port
pub const CHARACTER_DEFS: Pin = Pin { plain: 0x11F46, debug: 0x12068 };

/// `Player_InitAssets`. tests: test_p1_player_port
pub const PLAYER_INIT_ASSETS: Pin = Pin { plain: 0x11F52, debug: 0x12074 };

/// `Player_LoadArt`. tests: test_p1_player_port
pub const PLAYER_LOAD_ART: Pin = Pin { plain: 0x11F6A, debug: 0x1208C };

/// `Player_Ability`. tests: test_p2_player_states_port
pub const PLAYER_ABILITY: Pin = Pin { plain: 0x11F84, debug: 0x120A6 };

/// `PhysTable_Sonic`. tests: test_p1_player_port
pub const PHYS_TABLE_SONIC: Pin = Pin { plain: 0x11ECA, debug: 0x11FEC };

/// `Pal_SonicTails`. tests: test_p1_player_port
pub const PAL_SONIC_TAILS: Pin = Pin { plain: 0x6DE6C, debug: 0x6E72C };

/// `OJZ_TestRaster`. tests: act_descriptor_port
pub const OJZ_TEST_RASTER: Pin = Pin { plain: 0x13440, debug: 0x13C8A };

/// `OJZ_TestPal`. tests: act_descriptor_port
pub const OJZ_TEST_PAL: Pin = Pin { plain: 0x13462, debug: 0x13D1A };

/// `OJZ_TestGradient`. tests: act_descriptor_port
pub const OJZ_TEST_GRADIENT: Pin = Pin { plain: 0x1372C, debug: 0x13FE4 };

/// `OJZ_ShimmerCycle`. tests: act_descriptor_port
pub const OJZ_SHIMMER_CYCLE: Pin = Pin { plain: 0x134C2, debug: 0x13D7A };

/// `OJZ_TestVsram`. tests: act_descriptor_port
pub const OJZ_TEST_VSRAM: Pin = Pin { plain: 0x1374A, debug: 0x14002 };

/// `OJZ_TestRamp`. tests: act_descriptor_port
pub const OJZ_TEST_RAMP: Pin = Pin { plain: 0x13768, debug: 0x14020 };

/// `Raster_Program`. tests: raster_port
pub const RASTER_PROGRAM: Pin = Pin { plain: 0xFFFF8B48, debug: 0xFFFF8B48 };

/// `Raster_Cursor`. tests: raster_port
pub const RASTER_CURSOR: Pin = Pin { plain: 0xFFFF8B4C, debug: 0xFFFF8B4C };

/// `Raster_Pending`. tests: raster_port
pub const RASTER_PENDING: Pin = Pin { plain: 0xFFFF8B50, debug: 0xFFFF8B50 };

/// `Raster_Buf_A`. tests: raster_port
pub const RASTER_BUF_A: Pin = Pin { plain: 0xFFFF8B56, debug: 0xFFFF8B56 };

/// `Raster_Active_Buf`. tests: raster_port
pub const RASTER_ACTIVE_BUF: Pin = Pin { plain: 0xFFFF8C56, debug: 0xFFFF8C56 };

/// `Raster_Buf_B`. tests: raster_port
pub const RASTER_BUF_B: Pin = Pin { plain: 0xFFFF8BD6, debug: 0xFFFF8BD6 };

/// `Raster_Line`. tests: raster_port
pub const RASTER_LINE: Pin = Pin { plain: 0xFFFF8B54, debug: 0xFFFF8B54 };

/// `Raster_Dense_Lines`. tests: raster_port
pub const RASTER_DENSE_LINES: Pin = Pin { plain: 0xFFFF8C5A, debug: 0xFFFF8C5A };

/// `Raster_Dense_Cursor`. tests: raster_port
pub const RASTER_DENSE_CURSOR: Pin = Pin { plain: 0xFFFF8C5C, debug: 0xFFFF8C5C };

/// `Raster_Dense_Cmd`. tests: raster_port
pub const RASTER_DENSE_CMD: Pin = Pin { plain: 0xFFFF8C60, debug: 0xFFFF8C60 };

/// `Raster_Dense_Mode`. tests: raster_port
pub const RASTER_DENSE_MODE: Pin = Pin { plain: 0xFFFF8C64, debug: 0xFFFF8C64 };

/// `Raster_Ramp_Acc`. tests: raster_port
pub const RASTER_RAMP_ACC: Pin = Pin { plain: 0xFFFF8C66, debug: 0xFFFF8C66 };

/// `Raster_Ramp_Step`. tests: raster_port
pub const RASTER_RAMP_STEP: Pin = Pin { plain: 0xFFFF8C6A, debug: 0xFFFF8C6A };

/// `Effects_World_Y`. tests: raster_port
pub const EFFECTS_WORLD_Y: Pin = Pin { plain: 0xFFFF8C6E, debug: 0xFFFF8C6E };

/// `Effects_Screen_L`. tests: raster_port, parallax_port, buffers_port
pub const EFFECTS_SCREEN_L: Pin = Pin { plain: 0xFFFF8C76, debug: 0xFFFF8C76 };

/// `Effects_Offscreen_Entry`. tests: raster_port, buffers_port
pub const EFFECTS_OFFSCREEN_ENTRY: Pin = Pin { plain: 0xFFFF8C7E, debug: 0xFFFF8C7E };

/// `Static_Pal_Ship`. tests: raster_port
pub const STATIC_PAL_SHIP: Pin = Pin { plain: 0xFFFF8E90, debug: 0xFFFF8E90 };

/// `Build_DMA_Entry`. tests: raster_port
pub const BUILD_DMA_ENTRY: Pin = Pin { plain: 0x2006, debug: 0x20F2 };

/// `Raster_Patch_Tab`. tests: raster_port
pub const RASTER_PATCH_TAB: Pin = Pin { plain: 0xFFFF8C82, debug: 0xFFFF8C82 };

/// `Raster_State`. tests: raster_port
pub const RASTER_STATE: Pin = Pin { plain: 0xFFFF8B48, debug: 0xFFFF8B48 };

/// `Raster_State_End`. tests: raster_port
pub const RASTER_STATE_END: Pin = Pin { plain: 0xFFFF8C86, debug: 0xFFFF8C86 };

/// `Pal_Variant_Stage`. tests: raster_port
pub const PAL_VARIANT_STAGE: Pin = Pin { plain: 0xFFFF8D46, debug: 0xFFFF8D46 };

/// `Raster_VBlank`. tests: game_loop_port, vblank_port, load_art_port, boot_port
pub const RASTER_V_BLANK: Pin = Pin { plain: 0x6C18, debug: 0x8004 };

/// `Palette_Compose`. tests: game_loop_port
pub const PALETTE_COMPOSE: Pin = Pin { plain: 0x702A, debug: 0x8416 };

/// `Player_Blocks`. tests: test_p1_player_port
pub const PLAYER_BLOCKS: Pin = Pin { plain: 0xFFFFB9E4, debug: 0xFFFFE31A };

/// `Player_Ring_Index`. tests: test_p1_player_port
pub const PLAYER_RING_INDEX: Pin = Pin { plain: 0xFFFFBE00, debug: 0xFFFFE700 };

/// `Player_Pos_Ring`. tests: test_p1_player_port
pub const PLAYER_POS_RING: Pin = Pin { plain: 0xFFFFBC00, debug: 0xFFFFE500 };

/// `Player_Stat_Ring`. tests: test_p1_player_port
pub const PLAYER_STAT_RING: Pin = Pin { plain: 0xFFFFBD00, debug: 0xFFFFE600 };

/// `Player_Death_Pending`. tests: test_p1_player_port
pub const PLAYER_DEATH_PENDING: Pin = Pin { plain: 0xFFFFBA14, debug: 0xFFFFE34A };

/// `Player_Bound_Right`. tests: test_p1_player_port
pub const PLAYER_BOUND_RIGHT: Pin = Pin { plain: 0xFFFFBA16, debug: 0xFFFFE34C };

/// `Player_Bound_Bottom`. tests: test_p1_player_port
pub const PLAYER_BOUND_BOTTOM: Pin = Pin { plain: 0xFFFFBA18, debug: 0xFFFFE34E };

/// `DustSpindash_Spawn`. tests: test_p1_player_port
pub const DUST_SPINDASH_SPAWN: Pin = Pin { plain: 0x1214A, debug: 0x1232A };

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
