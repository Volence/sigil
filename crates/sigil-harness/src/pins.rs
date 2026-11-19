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
//! [provenance] 96 regions, 417 symbols, 7 offsets

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

/// Assembled (pre-convsym) ROM length, plain shape.
pub const ASSEMBLED_LEN: usize = 0xC3E78;
/// Assembled (pre-convsym) ROM length, `__DEBUG__` shape.
pub const DEBUG_ASSEMBLED_LEN: usize = 0xC5C8A;

// ── Regions (manifest order) ──

/// `Vectors` .. start + 0x100 plain / 0x100 debug (literal, no end symbol), gate `SIGIL_EMP_VECTORS`.
pub const VECTORS: Region = Region { plain_base: 0x0, debug_base: 0x0, plain_len: 0x100, debug_len: 0x100 };

/// `GameHeader` .. `section:header`.
pub const HEADER: Region = Region { plain_base: 0x100, debug_base: 0x100, plain_len: 0x100, debug_len: 0x100 };

/// `HeightMaps` .. `section:collision_data`.
pub const COLLISION_DATA: Region = Region { plain_base: 0x6C9BA, debug_base: 0x733AE, plain_len: 0x1D3E2, debug_len: 0x1D3E2 };

/// `EntryPoint` .. `section:boot`, gate `SIGIL_EMP_BOOT`.
pub const BOOT: Region = Region { plain_base: 0x200, debug_base: 0x200, plain_len: 0x1AC, debug_len: 0x1B2 };

/// `BootData` .. `section:boot_head`.
pub const BOOT_HEAD: Region = Region { plain_base: 0x3AC, debug_base: 0x3B2, plain_len: 0x1886, debug_len: 0x1908 };

/// `BootData_PostBlob` .. `section:boot_tail`.
pub const BOOT_TAIL: Region = Region { plain_base: 0x1C32, debug_base: 0x1CBA, plain_len: 0xE, debug_len: 0xE };

/// `VDP_Shadow_Init` .. `section:vdp_init`, gate `SIGIL_EMP_VDP_INIT`.
pub const VDP_INIT: Region = Region { plain_base: 0x1C40, debug_base: 0x1CC8, plain_len: 0x3A, debug_len: 0x90 };

/// `Init_DMA_Queue` .. `section:dma_queue`, gate `SIGIL_EMP_DMA_QUEUE`.
pub const DMA_QUEUE: Region = Region { plain_base: 0x1C7A, debug_base: 0x1D58, plain_len: 0x3D6, debug_len: 0x44E };

/// `Init_SpriteTable` .. `section:buffers`, gate `SIGIL_EMP_BUFFERS`.
pub const BUFFERS: Region = Region { plain_base: 0x2050, debug_base: 0x21A6, plain_len: 0x2D8, debug_len: 0x2D8 };

/// `VBlank_Handler` .. `section:vblank`, gate `SIGIL_EMP_VBLANK`.
pub const VBLANK: Region = Region { plain_base: 0x2328, debug_base: 0x247E, plain_len: 0x1E0, debug_len: 0x21A };

/// `HBlank_Install` .. `section:hblank`, gate `SIGIL_EMP_HBLANK`.
pub const HBLANK: Region = Region { plain_base: 0x2508, debug_base: 0x2698, plain_len: 0x30, debug_len: 0x30 };

/// `Read_Controllers` .. `section:controllers`, gate `SIGIL_EMP_CONTROLLERS`.
pub const CONTROLLERS: Region = Region { plain_base: 0x2538, debug_base: 0x26C8, plain_len: 0x10E, debug_len: 0x10E };

/// `GameLoop` .. `section:game_loop`, gate `SIGIL_EMP_GAME_LOOP`.
pub const GAME_LOOP: Region = Region { plain_base: 0x2646, debug_base: 0x27D6, plain_len: 0x20, debug_len: 0x24 };

/// `Input_Tick` .. `section:replay`.
pub const REPLAY: Region = Region { plain_base: 0x2666, debug_base: 0x27FA, plain_len: 0x146, debug_len: 0x1F6 };

/// `S4LZ_DecompressDict` .. `section:s4lz`, gate `SIGIL_EMP_S4LZ`.
pub const S4LZ: Region = Region { plain_base: 0x27AC, debug_base: 0x29F0, plain_len: 0x146, debug_len: 0x19A };

/// `ZX0R_Decompress` .. `section:zx0_resume`.
pub const ZX0_RESUME: Region = Region { plain_base: 0x28F2, debug_base: 0x2B8A, plain_len: 0x78, debug_len: 0x78 };

/// `GetSineCosine` .. `section:math`, gate `SIGIL_EMP_MATH`.
pub const MATH: Region = Region { plain_base: 0x296A, debug_base: 0x2C02, plain_len: 0x3F6, debug_len: 0x3F6 };

/// `Perform_DPLC` .. `section:dplc`, gate `SIGIL_EMP_DPLC`.
pub const DPLC: Region = Region { plain_base: 0x2D60, debug_base: 0x2FF8, plain_len: 0xA4, debug_len: 0xA4 };

/// `InitObjectRAM` .. `section:core`, gate `SIGIL_EMP_CORE`.
pub const CORE: Region = Region { plain_base: 0x2E04, debug_base: 0x309C, plain_len: 0x2F0, debug_len: 0x742 };

/// `InitSpriteSystem` .. `section:sprites`, gate `SIGIL_EMP_SPRITES`.
pub const SPRITES: Region = Region { plain_base: 0x30F4, debug_base: 0x37DE, plain_len: 0x410, debug_len: 0x532 };

/// `AnimateSprite` .. `section:animate`, gate `SIGIL_EMP_ANIMATE`.
pub const ANIMATE: Region = Region { plain_base: 0x3504, debug_base: 0x3D10, plain_len: 0x194, debug_len: 0x2BA };

/// `TouchResponse` .. `section:collision`, gate `SIGIL_EMP_COLLISION`.
pub const COLLISION: Region = Region { plain_base: 0x3698, debug_base: 0x3FCA, plain_len: 0x32C, debug_len: 0x334 };

/// `RingBuffer_Add` .. `section:rings`, gate `SIGIL_EMP_RINGS`.
pub const RINGS: Region = Region { plain_base: 0x39C4, debug_base: 0x42FE, plain_len: 0x1BC, debug_len: 0x226 };

/// `Collected_Init` .. `section:entity_window`, gate `SIGIL_EMP_ENTITY_WINDOW`.
pub const ENTITY_WINDOW: Region = Region { plain_base: 0x3B80, debug_base: 0x4524, plain_len: 0x8F0, debug_len: 0xDC4 };

/// `PopulateSpawnedPieceCount` .. `section:children`, gate `SIGIL_EMP_CHILDREN`.
pub const CHILDREN: Region = Region { plain_base: 0x4470, debug_base: 0x52E8, plain_len: 0x2EC, debug_len: 0x44C };

/// `Load_Object` .. `section:load_object`, gate `SIGIL_EMP_LOAD_OBJECT`.
pub const LOAD_OBJECT: Region = Region { plain_base: 0x475C, debug_base: 0x5734, plain_len: 0x88, debug_len: 0x88 };

/// `Plane_Buffer_Reset` .. `section:plane_buffer`, gate `SIGIL_EMP_PLANE_BUFFER`.
pub const PLANE_BUFFER: Region = Region { plain_base: 0x47E4, debug_base: 0x57BC, plain_len: 0x31C, debug_len: 0x53A };

/// `Tile_Cache_GetTile` .. `section:tile_cache`, gate `SIGIL_EMP_TILE_CACHE`.
pub const TILE_CACHE: Region = Region { plain_base: 0x4B00, debug_base: 0x5CF6, plain_len: 0xF92, debug_len: 0x1274 };

/// `Collision_GetType` .. `section:collision_lookup`, gate `SIGIL_EMP_COLLISION_LOOKUP`.
pub const COLLISION_LOOKUP: Region = Region { plain_base: 0x5A92, debug_base: 0x6F6A, plain_len: 0x64, debug_len: 0x64 };

/// `Section_Init` .. `section:section`, gate `SIGIL_EMP_SECTION`.
pub const SECTION: Region = Region { plain_base: 0x6032, debug_base: 0x750A, plain_len: 0x528, debug_len: 0x958 };

/// `Camera_Init` .. `section:camera`, gate `SIGIL_EMP_CAMERA`.
pub const CAMERA: Region = Region { plain_base: 0x655A, debug_base: 0x7E62, plain_len: 0x1C8, debug_len: 0x1D2 };

/// `Parallax_Init` .. `section:parallax`, gate `SIGIL_EMP_PARALLAX`.
pub const PARALLAX: Region = Region { plain_base: 0x6722, debug_base: 0x8034, plain_len: 0xD54, debug_len: 0xE46 };

/// `Raster_Install` .. `section:raster`, gate `SIGIL_EMP_RASTER`.
pub const RASTER: Region = Region { plain_base: 0x7476, debug_base: 0x8E7A, plain_len: 0x3EC, debug_len: 0x3EC };

/// `Palette_LoadPal` .. `Effects_InstallPreset`, gate `SIGIL_EMP_PALETTE`.
pub const PALETTE: Region = Region { plain_base: 0x7862, debug_base: 0x9266, plain_len: 0x508, debug_len: 0x508 };

/// `Effects_InstallPreset` .. `section:preset`.
pub const PRESET: Region = Region { plain_base: 0x7D6A, debug_base: 0x976E, plain_len: 0xA8, debug_len: 0xAA };

/// `Level_LoadArt` .. `section:load_art`, gate `SIGIL_EMP_LOAD_ART`.
pub const LOAD_ART: Region = Region { plain_base: 0x7E12, debug_base: 0x9818, plain_len: 0xC4, debug_len: 0xC4 };

/// `PageIn_Process` .. `section:page_in`.
pub const PAGE_IN: Region = Region { plain_base: 0x7ED6, debug_base: 0x98DC, plain_len: 0x3DE, debug_len: 0x4EC };

/// `PageCache_Init` .. `section:page_cache`.
pub const PAGE_CACHE: Region = Region { plain_base: 0x82B4, debug_base: 0x9DC8, plain_len: 0x862, debug_len: 0x1410 };

/// `BG_Init` .. `section:bg`, gate `SIGIL_EMP_BG`.
pub const BG: Region = Region { plain_base: 0x8B16, debug_base: 0xB1D8, plain_len: 0x2C8, debug_len: 0x3E8 };

/// `BgAnim_Init` .. `section:bg_anim`, gate `SIGIL_EMP_BG_ANIM`.
pub const BG_ANIM: Region = Region { plain_base: 0x8DDE, debug_base: 0xB5C0, plain_len: 0xFC, debug_len: 0x1D0 };

/// `CompressionSelfTest` .. `section:compression_selftest` (debug-only region; plain empty at `Sound_PostByte`), gate `SIGIL_EMP_COMPRESSION_SELFTEST`.
pub const COMPRESSION_SELFTEST: Region = Region { plain_base: 0x8EDA, debug_base: 0xB790, plain_len: 0x0, debug_len: 0xDE0 };

/// `Sound_PostByte` .. `section:sound_api`, gate `SIGIL_EMP_SOUND_API`.
pub const SOUND_API: Region = Region { plain_base: 0x8EDA, debug_base: 0xC570, plain_len: 0x2D6, debug_len: 0x480 };

/// `TestSolid_Init` .. `section:test_solid`, gate `SIGIL_EMP_TEST_OBJECTS`.
pub const TEST_SOLID: Region = Region { plain_base: 0x126A8, debug_base: 0x12BAA, plain_len: 0x4E6, debug_len: 0x540 };

/// `TestParticle` .. `section:test_particle` (debug-only region; plain empty at `DeformTable_Zero`), gate `SIGIL_EMP_TEST_OBJECTS`.
pub const TEST_PARTICLE: Region = Region { plain_base: 0x12B8E, debug_base: 0x130EA, plain_len: 0x0, debug_len: 0x58 };

/// `TestStatic_Main` .. `section:test_static`, gate `SIGIL_EMP_TEST_STATIC`.
pub const TEST_STATIC: Region = Region { plain_base: 0x126A4, debug_base: 0x1286A, plain_len: 0x4, debug_len: 0x4 };

/// `TestAnimated` .. `section:test_animated` (debug-only region; plain empty at `TestSolid_Init`), gate `SIGIL_EMP_TEST_ANIMATED`.
pub const TEST_ANIMATED: Region = Region { plain_base: 0x126A8, debug_base: 0x1286E, plain_len: 0x0, debug_len: 0x60 };

/// `TestEmitter` .. `section:test_emitter` (debug-only region; plain empty at `DeformTable_Zero`), gate `SIGIL_EMP_TEST_EMITTER`.
pub const TEST_EMITTER: Region = Region { plain_base: 0x12B8E, debug_base: 0x13142, plain_len: 0x0, debug_len: 0x5E };

/// `TestStressEmitter` .. `section:test_stress_emitter` (debug-only region; plain empty at `DeformTable_Zero`), gate `SIGIL_EMP_TEST_STRESS_EMITTER`.
pub const TEST_STRESS_EMITTER: Region = Region { plain_base: 0x12B8E, debug_base: 0x132D6, plain_len: 0x0, debug_len: 0x5E };

/// `TestChurnObj` .. `section:test_churn` (debug-only region; plain empty at `DeformTable_Zero`), gate `SIGIL_EMP_TEST_CHURN`.
pub const TEST_CHURN: Region = Region { plain_base: 0x12B8E, debug_base: 0x13334, plain_len: 0x0, debug_len: 0x7C };

/// `TestChildPart` .. `section:test_parent` (debug-only region; plain empty at `DeformTable_Zero`), gate `SIGIL_EMP_TEST_PARENT`.
pub const TEST_PARENT: Region = Region { plain_base: 0x12B8E, debug_base: 0x131A0, plain_len: 0x0, debug_len: 0x136 };

/// `TestPlayer` .. `section:test_player` (debug-only region; plain empty at `TestSolid_Init`), gate `SIGIL_EMP_TEST_PLAYER`.
pub const TEST_PLAYER: Region = Region { plain_base: 0x126A8, debug_base: 0x128CE, plain_len: 0x0, debug_len: 0x294 };

/// `TestEnemy_Init` .. `section:test_enemy` (debug-only region; plain empty at `TestSolid_Init`), gate `SIGIL_EMP_TEST_ENEMY`.
pub const TEST_ENEMY: Region = Region { plain_base: 0x126A8, debug_base: 0x12B62, plain_len: 0x0, debug_len: 0x48 };

/// `OJZ_TestRaster` .. `section:ojz_effects`.
pub const OJZ_EFFECTS: Region = Region { plain_base: 0x1459C, debug_base: 0x14DD0, plain_len: 0x94A, debug_len: 0xB62 };

/// `DeformTable_Zero` .. `section:scene_registry`, gate `SIGIL_EMP_SCENE_REGISTRY`.
pub const SCENE_REGISTRY: Region = Region { plain_base: 0x12B8E, debug_base: 0x133B0, plain_len: 0x1486, debug_len: 0x1486 };

/// `Map_TestObj` .. `section:test_mappings`, gate `SIGIL_EMP_TEST_MAPPINGS`.
pub const TEST_MAPPINGS: Region = Region { plain_base: 0x28344, debug_base: 0x2ED30, plain_len: 0x30, debug_len: 0x30 };

/// `Map_DustSpindash` .. `section:dust_data`, gate `SIGIL_EMP_DUST_DATA`.
pub const DUST_DATA: Region = Region { plain_base: 0x28374, debug_base: 0x2ED60, plain_len: 0xBDA, debug_len: 0xBDA };

/// `Ani_Sonic` .. `section:sonic_anims`, gate `SIGIL_EMP_SONIC_ANIMS`.
pub const SONIC_ANIMS: Region = Region { plain_base: 0x28F4E, debug_base: 0x2F93A, plain_len: 0x10A, debug_len: 0x10A };

/// `Ani_Tails` .. `section:tails_anims`, gate `SIGIL_EMP_TAILS_ANIMS`.
pub const TAILS_ANIMS: Region = Region { plain_base: 0x29058, debug_base: 0x2FA44, plain_len: 0x1BC, debug_len: 0x1BC };

/// `Ani_Knuckles` .. `section:knuckles_anims`, gate `SIGIL_EMP_KNUCKLES_ANIMS`.
pub const KNUCKLES_ANIMS: Region = Region { plain_base: 0x29214, debug_base: 0x2FC00, plain_len: 0x16B, debug_len: 0x16B };

/// `Map_Tails` .. `section:tails_data`, gate `SIGIL_EMP_TAILS_DATA`.
pub const TAILS_DATA: Region = Region { plain_base: 0x29394, debug_base: 0x2FD88, plain_len: 0x20F5E, debug_len: 0x20F5E };

/// `Map_Knuckles` .. `section:knuckles_data`, gate `SIGIL_EMP_KNUCKLES_DATA`.
pub const KNUCKLES_DATA: Region = Region { plain_base: 0x4A2F2, debug_base: 0x50CE6, plain_len: 0x226C8, debug_len: 0x226C8 };

/// `Ani_Particle` .. `section:particle_anims` (debug-only region; plain empty at `Ani_DustSpindash`), gate `SIGIL_EMP_PARTICLE_ANIMS`.
pub const PARTICLE_ANIMS: Region = Region { plain_base: 0x29380, debug_base: 0x2FD6C, plain_len: 0x0, debug_len: 0x7 };

/// `Ani_DustSpindash` .. `section:dust_anims`, gate `SIGIL_EMP_DUST_ANIMS`.
pub const DUST_ANIMS: Region = Region { plain_base: 0x29380, debug_base: 0x2FD74, plain_len: 0x14, debug_len: 0x14 };

/// `OJZ_Sec0_TypeTable` .. `section:entity_data`.
pub const ENTITY_DATA: Region = Region { plain_base: 0x14F1A, debug_base: 0x15966, plain_len: 0x18E, debug_len: 0x18E };

/// `OJZ_Act_Pool_Page0` .. `section:ojz_act_pool`.
pub const OJZ_ACT_POOL: Region = Region { plain_base: 0x150A8, debug_base: 0x15AF4, plain_len: 0x2F0C, debug_len: 0x2F0C };

/// `OJZ_Act1_Descriptor` .. `section:act_descriptor`, gate `SIGIL_EMP_ACT_DESCRIPTOR`.
pub const ACT_DESCRIPTOR: Region = Region { plain_base: 0x17FB4, debug_base: 0x18A00, plain_len: 0x240, debug_len: 0x294 };

/// `OJZ_Sec0_Blocks` .. `section:sec_block_blobs`.
pub const SEC_BLOCK_BLOBS: Region = Region { plain_base: 0x181F4, debug_base: 0x18C94, plain_len: 0x93B4, debug_len: 0x93B4 };

/// `OJZ_Sec0_LocalMap` .. `section:sec_local_maps`.
pub const SEC_LOCAL_MAPS: Region = Region { plain_base: 0x215A8, debug_base: 0x22048, plain_len: 0x4EC, debug_len: 0x4EC };

/// `OJZ_Palette` .. `section:ojz_act_assets`.
pub const OJZ_ACT_ASSETS: Region = Region { plain_base: 0x21A94, debug_base: 0x22534, plain_len: 0x4882, debug_len: 0xA744 };

/// `BgAnim_Table` .. `section:ojz_bg_anim`.
pub const OJZ_BG_ANIM: Region = Region { plain_base: 0x26316, debug_base: 0x2CC78, plain_len: 0x202E, debug_len: 0x20B8 };

/// `ObjDef_Static` .. start + 0x34 plain / 0x34 debug (literal, no end symbol), gate `SIGIL_EMP_OBJDEFS`.
pub const OBJDEFS: Region = Region { plain_base: 0x14EE6, debug_base: 0x15932, plain_len: 0x34, debug_len: 0x34 };

/// `GameState_ObjectTest_Init` .. `section:object_test_state` (debug-only region; plain empty at `GameState_OJZScroll_Init`), gate `SIGIL_EMP_OBJECT_TEST_STATE`.
pub const OBJECT_TEST_STATE: Region = Region { plain_base: 0xC2586, debug_base: 0xC2586, plain_len: 0x0, debug_len: 0x384 };

/// `GameState_OJZScroll_Init` .. `section:ojz_scroll_test`, gate `SIGIL_EMP_OJZ_SCROLL_TEST`.
pub const OJZ_SCROLL_TEST: Region = Region { plain_base: 0xC2586, debug_base: 0xC290A, plain_len: 0x5E2, debug_len: 0x2070 };

/// `Replay_OJZ_Fixture` .. `section:replay_fixture`.
pub const REPLAY_FIXTURE: Region = Region { plain_base: 0xC2B68, debug_base: 0xC497A, plain_len: 0x260, debug_len: 0x260 };

/// `BusError` .. `section:error_handler`, gate `SIGIL_EMP_ERROR_HANDLER`.
pub const ERROR_HANDLER: Region = Region { plain_base: 0xC2DC8, debug_base: 0xC4BDA, plain_len: 0x10B0, debug_len: 0x10B0 };

/// `Dac_Temp_Blip` .. `section:dac_banks`, gate `SIGIL_EMP_DAC`.
pub const DAC_BANKS: Region = Region { plain_base: 0xA8000, debug_base: 0xA8000, plain_len: 0xE49A, debug_len: 0xE49A };

/// `Song_MovingTrucks` .. `section:mt_bank`, gate `SIGIL_EMP_MT`.
pub const MT_BANK_BLOB: Region = Region { plain_base: 0xB8730, debug_base: 0xB8730, plain_len: 0x4EB8, debug_len: 0x6900 };

/// `Sfx_33` .. `section:sfx_bank_blob`, gate `SIGIL_EMP_SFX`.
pub const SFX_BANK_BLOB: Region = Region { plain_base: 0xBD5E8, debug_base: 0xBF030, plain_len: 0x674, debug_len: 0x684 };

/// `SoundTablesZ80_Head` .. `section:soundbankhead`, gate `SIGIL_EMP_SOUNDBANKHEAD`.
pub const SOUNDBANKHEAD: Region = Region { plain_base: 0xB8000, debug_base: 0xB8000, plain_len: 0x730, debug_len: 0x730 };

/// `EndOfRom` .. start + 0x0 plain / 0x0 debug (literal, no end symbol), gate `SIGIL_EMP_EPILOGUE`.
pub const EPILOGUE: Region = Region { plain_base: 0xC3E78, debug_base: 0xC5C8A, plain_len: 0x0, debug_len: 0x0 };

/// `ObjCodeBase` .. start + 0x2 plain / 0x2 debug (literal, no end symbol), gate `SIGIL_EMP_OBJCODEBASE`.
pub const OBJCODEBASE: Region = Region { plain_base: 0x10000, debug_base: 0x10000, plain_len: 0x2, debug_len: 0x2 };

/// `Player_Init` .. `section:player_common`, gate `SIGIL_EMP_PLAYER_COMMON`.
pub const PLAYER_COMMON: Region = Region { plain_base: 0x10002, debug_base: 0x10002, plain_len: 0x9C6, debug_len: 0xADC };

/// `CharDef_Sonic` .. `section:sonic`, gate `SIGIL_EMP_SONIC`.
pub const SONIC: Region = Region { plain_base: 0x12238, debug_base: 0x12340, plain_len: 0x36, debug_len: 0x36 };

/// `CharDef_Tails` .. `section:tails`, gate `SIGIL_EMP_TAILS`.
pub const TAILS: Region = Region { plain_base: 0x1226E, debug_base: 0x12376, plain_len: 0x36, debug_len: 0x36 };

/// `CharDef_Knuckles` .. `section:knuckles`, gate `SIGIL_EMP_KNUCKLES`.
pub const KNUCKLES: Region = Region { plain_base: 0x122A4, debug_base: 0x123AC, plain_len: 0x36, debug_len: 0x36 };

/// `CharacterDefs` .. `section:characters`, gate `SIGIL_EMP_CHARACTERS`.
pub const CHARACTERS: Region = Region { plain_base: 0x122DA, debug_base: 0x123E2, plain_len: 0x4A, debug_len: 0xB0 };

/// `TailsAppendage_Refresh` .. `section:tails_appendage`, gate `SIGIL_EMP_TAILS_APPENDAGE`.
pub const TAILS_APPENDAGE: Region = Region { plain_base: 0x12324, debug_base: 0x12492, plain_len: 0x11C, debug_len: 0x174 };

/// `DustPuff_Spawn` .. `section:dust_puff`, gate `SIGIL_EMP_DUST_PUFF`.
pub const DUST_PUFF: Region = Region { plain_base: 0x12440, debug_base: 0x12606, plain_len: 0x46, debug_len: 0x46 };

/// `Dust_Tick` .. `section:dust_spindash`, gate `SIGIL_EMP_DUST_SPINDASH`.
pub const DUST_SPINDASH: Region = Region { plain_base: 0x12486, debug_base: 0x1264C, plain_len: 0x102, debug_len: 0x102 };

/// `PState_Ground` .. `section:player_ground`, gate `SIGIL_EMP_PLAYER_GROUND`.
pub const PLAYER_GROUND: Region = Region { plain_base: 0x109C8, debug_base: 0x10ADE, plain_len: 0x4C6, debug_len: 0x4C6 };

/// `PState_Air` .. `section:player_air`, gate `SIGIL_EMP_PLAYER_AIR`.
pub const PLAYER_AIR: Region = Region { plain_base: 0x10E8E, debug_base: 0x10FA4, plain_len: 0x34A, debug_len: 0x34A };

/// `PState_Spindash` .. `section:player_spindash`, gate `SIGIL_EMP_PLAYER_SPINDASH`.
pub const PLAYER_SPINDASH: Region = Region { plain_base: 0x111D8, debug_base: 0x112EE, plain_len: 0xDE, debug_len: 0xDA };

/// `PState_Fly` .. `section:player_fly`, gate `SIGIL_EMP_PLAYER_FLY`.
pub const PLAYER_FLY: Region = Region { plain_base: 0x112B6, debug_base: 0x113C8, plain_len: 0x13A, debug_len: 0x138 };

/// `PState_Glide` .. `section:player_glide`, gate `SIGIL_EMP_PLAYER_GLIDE`.
pub const PLAYER_GLIDE: Region = Region { plain_base: 0x113F0, debug_base: 0x11500, plain_len: 0x2DC, debug_len: 0x2D8 };

/// `Climb_WallDist` .. `CharDef_Sonic`, gate `SIGIL_EMP_PLAYER_CLIMB`.
pub const PLAYER_CLIMB: Region = Region { plain_base: 0x116CC, debug_base: 0x117D8, plain_len: 0xB6C, debug_len: 0xB68 };

/// `Collision_ProbeDown` .. `section:player_sensors`, gate `SIGIL_EMP_PLAYER_SENSORS`.
pub const PLAYER_SENSORS: Region = Region { plain_base: 0x5AF6, debug_base: 0x6FCE, plain_len: 0x53C, debug_len: 0x53C };

// ── Symbols (manifest order) ──

/// `OJZ_Preset_Sec0`.
pub const OJZ_PRESET_SEC0: Pin = Pin { plain: 0x14B74, debug: 0x154A8 };

/// `OJZ_Preset_Sec1`.
pub const OJZ_PRESET_SEC1: Pin = Pin { plain: 0x14BA2, debug: 0x154D6 };

/// `OJZ_Preset_Sec2`.
pub const OJZ_PRESET_SEC2: Pin = Pin { plain: 0x14BD0, debug: 0x15504 };

/// `OJZ_Preset_Sec3`.
pub const OJZ_PRESET_SEC3: Pin = Pin { plain: 0x14BFE, debug: 0x15532 };

/// `OJZ_Preset_Plain`.
pub const OJZ_PRESET_PLAIN: Pin = Pin { plain: 0x14C2C, debug: 0x15560 };

/// `OJZ_Preset_Depth`.
pub const OJZ_PRESET_DEPTH: Pin = Pin { plain: 0x14C5A, debug: 0x1558E };

/// `EditorSceneBinding_OJZ_Act1_Sec4`.
pub const EDITOR_SCENE_BINDING_OJZ_ACT1_SEC4: Pin = Pin { plain: 0x140D2, debug: 0x148F4 };

/// `OJZ_Preset_Sec5`.
pub const OJZ_PRESET_SEC5: Pin = Pin { plain: 0x14C88, debug: 0x155BC };

/// `EditorRaster_OJZ_Act1_authored_probe`.
pub const EDITOR_RASTER_OJZ_ACT1_AUTHORED_PROBE: Pin = Pin { plain: 0x1430C, debug: 0x14B40 };

/// `EditorRaster_OJZ_Act1_ojz_sec5_showcase`.
pub const EDITOR_RASTER_OJZ_ACT1_OJZ_SEC5_SHOWCASE: Pin = Pin { plain: 0x1440C, debug: 0x14C40 };

/// `EditorRaster_OJZ_Act1_ojz_sec3_shimmer`.
pub const EDITOR_RASTER_OJZ_ACT1_OJZ_SEC3_SHIMMER: Pin = Pin { plain: 0x1438C, debug: 0x14BC0 };

/// `EditorCycle_OJZ_Act1_ojz_sec3_shimmer`.
pub const EDITOR_CYCLE_OJZ_ACT1_OJZ_SEC3_SHIMMER: Pin = Pin { plain: 0x1458C, debug: 0x14DC0 };

/// `Effects_InstallPreset`.
pub const EFFECTS_INSTALL_PRESET: Pin = Pin { plain: 0x7D6A, debug: 0x976E };

/// `Raster_GetChannelBand`.
pub const RASTER_GET_CHANNEL_BAND: Pin = Pin { plain: 0x777E, debug: 0x9182 };

/// `TestStatic_Main`.
pub const TEST_STATIC_MAIN: Pin = Pin { plain: 0x126A4, debug: 0x1286A };

/// `TestSolid_Init`.
pub const TEST_SOLID_INIT: Pin = Pin { plain: 0x126A8, debug: 0x12BAA };

/// `TestEnemy_Init`, debug-shape consumer only (`debug_only`).
pub const TEST_ENEMY_INIT: u32 = 0x12B62;

/// `TestParent`, debug-shape consumer only (`debug_only`).
pub const TEST_PARENT_LABEL: u32 = 0x1322A;

/// `Map_TestObj`.
pub const MAP_TEST_OBJ: Pin = Pin { plain: 0x28344, debug: 0x2ED30 };

/// `Map_Sonic`.
pub const MAP_SONIC: Pin = Pin { plain: 0x6EBBA, debug: 0x755AE };

/// `DPLC_Sonic`.
pub const DPLC_SONIC: Pin = Pin { plain: 0x7083A, debug: 0x7722E };

/// `Art_Sonic`.
pub const ART_SONIC: Pin = Pin { plain: 0x710FC, debug: 0x77AF0 };

/// `CreateEffect_Normal`.
pub const CREATE_EFFECT_NORMAL: Pin = Pin { plain: 0x46C6, debug: 0x569E };

/// `CreateChild_Normal`.
pub const CREATE_CHILD_NORMAL: Pin = Pin { plain: 0x449C, debug: 0x5314 };

/// `DeleteChildren`.
pub const DELETE_CHILDREN: Pin = Pin { plain: 0x46A8, debug: 0x5680 };

/// `GetSineCosine`.
pub const GET_SINE_COSINE: Pin = Pin { plain: 0x296A, debug: 0x2C02 };

/// `EntryPoint`.
pub const ENTRY_POINT: Pin = Pin { plain: 0x200, debug: 0x200 };

/// `BusError`, debug-shape consumer only (`debug_only`).
pub const BUS_ERROR: u32 = 0xC4BDA;

/// `AddressError`, debug-shape consumer only (`debug_only`).
pub const ADDRESS_ERROR: u32 = 0xC4BF2;

/// `IllegalInstr`, debug-shape consumer only (`debug_only`).
pub const ILLEGAL_INSTR: u32 = 0xC4C0E;

/// `ZeroDivide`, debug-shape consumer only (`debug_only`).
pub const ZERO_DIVIDE: u32 = 0xC4C30;

/// `ChkInstr`, debug-shape consumer only (`debug_only`).
pub const CHK_INSTR: u32 = 0xC4C4A;

/// `TrapvInstr`, debug-shape consumer only (`debug_only`).
pub const TRAPV_INSTR: u32 = 0xC4C68;

/// `PrivilegeViol`, debug-shape consumer only (`debug_only`).
pub const PRIVILEGE_VIOL: u32 = 0xC4C88;

/// `Trace`, debug-shape consumer only (`debug_only`).
pub const TRACE: u32 = 0xC4CAA;

/// `Line1010Emu`, debug-shape consumer only (`debug_only`).
pub const LINE1010_EMU: u32 = 0xC4CBE;

/// `Line1111Emu`, debug-shape consumer only (`debug_only`).
pub const LINE1111_EMU: u32 = 0xC4CDE;

/// `ErrorExcept`, debug-shape consumer only (`debug_only`).
pub const ERROR_EXCEPT: u32 = 0xC4CFE;

/// `ErrorTrap`, debug-shape consumer only (`debug_only`).
pub const ERROR_TRAP: u32 = 0xC4D1C;

/// `VBlank_Handler`.
pub const V_BLANK_HANDLER: Pin = Pin { plain: 0x2328, debug: 0x247E };

/// `HBlank_Vector_Slot`.
pub const H_BLANK_VECTOR_SLOT: Pin = Pin { plain: 0xFFFFB65A, debug: 0xFFFFB6CE };

/// `VDP_Shadow_Table`.
pub const VDP_SHADOW_TABLE: Pin = Pin { plain: 0xFFFF800E, debug: 0xFFFF800E };

/// `BootData_VDPRegs`.
pub const BOOT_DATA_VDP_REGS: Pin = Pin { plain: 0x3C2, debug: 0x3C8 };

/// `Ctrl_1_Held`.
pub const CTRL_1_HELD: Pin = Pin { plain: 0xFFFF8028, debug: 0xFFFF8028 };

/// `Ctrl_1_Held_Raw`.
pub const CTRL_1_HELD_RAW: Pin = Pin { plain: 0xFFFFBA48, debug: 0xFFFFBABC };

/// `Ctrl_2_Held`.
pub const CTRL_2_HELD: Pin = Pin { plain: 0xFFFF802A, debug: 0xFFFF802A };

/// `Ctrl_1_Ext_Held`.
pub const CTRL_1_EXT_HELD: Pin = Pin { plain: 0xFFFF802E, debug: 0xFFFF802E };

/// `Ctrl_2_Ext_Held`.
pub const CTRL_2_EXT_HELD: Pin = Pin { plain: 0xFFFF8030, debug: 0xFFFF8030 };

/// `Ctrl_2_Held_Raw`.
pub const CTRL_2_HELD_RAW: Pin = Pin { plain: 0xFFFFBA49, debug: 0xFFFFBABD };

/// `Ctrl_1_Ext_Held_Raw`.
pub const CTRL_1_EXT_HELD_RAW: Pin = Pin { plain: 0xFFFFBA4A, debug: 0xFFFFBABE };

/// `Ctrl_2_Ext_Held_Raw`.
pub const CTRL_2_EXT_HELD_RAW: Pin = Pin { plain: 0xFFFFBA4B, debug: 0xFFFFBABF };

/// `VSync_Wait`.
pub const V_SYNC_WAIT: Pin = Pin { plain: 0x24DE, debug: 0x2660 };

/// `Sound_DrainSfxRing`.
pub const SOUND_DRAIN_SFX_RING: Pin = Pin { plain: 0x904E, debug: 0xC88E };

/// `Game_State`.
pub const GAME_STATE: Pin = Pin { plain: 0xFFFF8008, debug: 0xFFFF8008 };

/// `Input_Tick`.
pub const INPUT_TICK: Pin = Pin { plain: 0x2666, debug: 0x27FA };

/// `Cache_Left_Col`.
pub const CACHE_LEFT_COL: Pin = Pin { plain: 0xFFFFAE04, debug: 0xFFFFAE78 };

/// `Draw_TileColumn`.
pub const DRAW_TILE_COLUMN: Pin = Pin { plain: 0x47EC, debug: 0x57C4 };

/// `Draw_TileRow_FromCache`.
pub const DRAW_TILE_ROW_FROM_CACHE: Pin = Pin { plain: 0x4940, debug: 0x5A0A };

/// `EntityWindow_Init`.
pub const ENTITY_WINDOW_INIT: Pin = Pin { plain: 0x3F04, debug: 0x4B60 };

/// `Section_Plane_Dirty`.
pub const SECTION_PLANE_DIRTY: Pin = Pin { plain: 0xFFFFAE7A, debug: 0xFFFFAEEE };

/// `Section_Right_Col_Written`.
pub const SECTION_RIGHT_COL_WRITTEN: Pin = Pin { plain: 0xFFFFAE7C, debug: 0xFFFFAEF0 };

/// `Section_Left_Col_Written`.
pub const SECTION_LEFT_COL_WRITTEN: Pin = Pin { plain: 0xFFFFAE7E, debug: 0xFFFFAEF2 };

/// `Section_Top_Row_Written`.
pub const SECTION_TOP_ROW_WRITTEN: Pin = Pin { plain: 0xFFFFAE76, debug: 0xFFFFAEEA };

/// `Section_Bottom_Row_Written`.
pub const SECTION_BOTTOM_ROW_WRITTEN: Pin = Pin { plain: 0xFFFFAE78, debug: 0xFFFFAEEC };

/// `Cache_Head_Col`.
pub const CACHE_HEAD_COL: Pin = Pin { plain: 0xFFFFAE06, debug: 0xFFFFAE7A };

/// `Cache_Top_Row`.
pub const CACHE_TOP_ROW: Pin = Pin { plain: 0xFFFFAE08, debug: 0xFFFFAE7C };

/// `Cache_Bottom_Row`.
pub const CACHE_BOTTOM_ROW: Pin = Pin { plain: 0xFFFFAE0A, debug: 0xFFFFAE7E };

/// `Cache_Origin_Col`.
pub const CACHE_ORIGIN_COL: Pin = Pin { plain: 0xFFFFAE0C, debug: 0xFFFFAE80 };

/// `Cache_Origin_Row`.
pub const CACHE_ORIGIN_ROW: Pin = Pin { plain: 0xFFFFAE0E, debug: 0xFFFFAE82 };

/// `Plane_Buffer_Ptr`.
pub const PLANE_BUFFER_PTR: Pin = Pin { plain: 0xFFFFACF0, debug: 0xFFFFAD64 };

/// `Plane_Buffer`.
pub const PLANE_BUFFER_BASE: Pin = Pin { plain: 0xFFFFA6F0, debug: 0xFFFFA764 };

/// `Tile_Cache_Nametable`.
pub const TILE_CACHE_NAMETABLE: Pin = Pin { plain: 0xFFFF0000, debug: 0xFFFF0000 };

/// `Tile_Cache_Collision`.
pub const TILE_CACHE_COLLISION: Pin = Pin { plain: 0xFFFF2580, debug: 0xFFFF2580 };

/// `Frame_Counter`.
pub const FRAME_COUNTER: Pin = Pin { plain: 0xFFFF8002, debug: 0xFFFF8002 };

/// `Logic_Tick`.
pub const LOGIC_TICK: Pin = Pin { plain: 0xFFFF8004, debug: 0xFFFF8004 };

/// `Block_Stage_Keys`.
pub const BLOCK_STAGE_KEYS: Pin = Pin { plain: 0xFFFFAE34, debug: 0xFFFFAEA8 };

/// `Block_Stage_Next`.
pub const BLOCK_STAGE_NEXT: Pin = Pin { plain: 0xFFFFAE74, debug: 0xFFFFAEE8 };

/// `Block_Stage_Bucket`.
pub const BLOCK_STAGE_BUCKET: Pin = Pin { plain: 0xFFFF6842, debug: 0xFFFF6842 };

/// `Block_Stage_Chain`.
pub const BLOCK_STAGE_CHAIN: Pin = Pin { plain: 0xFFFF6942, debug: 0xFFFF6942 };

/// `Block_Stage_Buffers`.
pub const BLOCK_STAGE_BUFFERS: Pin = Pin { plain: 0xFFFF3842, debug: 0xFFFF3842 };

/// `Block_Stage_Ptrs`.
pub const BLOCK_STAGE_PTRS: Pin = Pin { plain: 0xFFFFB660, debug: 0xFFFFB6D4 };

/// `Block_Stage_ZeroPage`.
pub const BLOCK_STAGE_ZERO_PAGE: Pin = Pin { plain: 0xFFFFB6E4, debug: 0xFFFFB758 };

/// `Cache_Fill_Last_Frame`.
pub const CACHE_FILL_LAST_FRAME: Pin = Pin { plain: 0xFFFFAE10, debug: 0xFFFFAE84 };

/// `Cache_Fill_Budget`.
pub const CACHE_FILL_BUDGET: Pin = Pin { plain: 0xFFFFAE1A, debug: 0xFFFFAE8E };

/// `Cache_Fill_Resume_Col`.
pub const CACHE_FILL_RESUME_COL: Pin = Pin { plain: 0xFFFFAE12, debug: 0xFFFFAE86 };

/// `Cache_Fill_Resume_Row`.
pub const CACHE_FILL_RESUME_ROW: Pin = Pin { plain: 0xFFFFAE14, debug: 0xFFFFAE88 };

/// `Cache_Fill_RowResume_Row`.
pub const CACHE_FILL_ROW_RESUME_ROW: Pin = Pin { plain: 0xFFFFAE1C, debug: 0xFFFFAE90 };

/// `Cache_Fill_RowResume_Col`.
pub const CACHE_FILL_ROW_RESUME_COL: Pin = Pin { plain: 0xFFFFAE1E, debug: 0xFFFFAE92 };

/// `Cache_Fill_Rows_Left`.
pub const CACHE_FILL_ROWS_LEFT: Pin = Pin { plain: 0xFFFFAE20, debug: 0xFFFFAE94 };

/// `Cache_Prev_Cam_Row`.
pub const CACHE_PREV_CAM_ROW: Pin = Pin { plain: 0xFFFFAE22, debug: 0xFFFFAE96 };

/// `Cache_Prev_Cam_X`.
pub const CACHE_PREV_CAM_X: Pin = Pin { plain: 0xFFFFAE24, debug: 0xFFFFAE98 };

/// `Cache_H_Pfx_Dir`.
pub const CACHE_H_PFX_DIR: Pin = Pin { plain: 0xFFFFAE26, debug: 0xFFFFAE9A };

/// `Cache_H_Pfx_Accum`.
pub const CACHE_H_PFX_ACCUM: Pin = Pin { plain: 0xFFFFAE28, debug: 0xFFFFAE9C };

/// `Cache_Pfx_Row_Target`.
pub const CACHE_PFX_ROW_TARGET: Pin = Pin { plain: 0xFFFFAE2C, debug: 0xFFFFAEA0 };

/// `Cache_Pfx_Col_Target`.
pub const CACHE_PFX_COL_TARGET: Pin = Pin { plain: 0xFFFFAE2E, debug: 0xFFFFAEA2 };

/// `Cache_Pfx_Skip_Armed`.
pub const CACHE_PFX_SKIP_ARMED: Pin = Pin { plain: 0xFFFFAE30, debug: 0xFFFFAEA4 };

/// `Cache_Pfx_Lag_Flag`.
pub const CACHE_PFX_LAG_FLAG: Pin = Pin { plain: 0xFFFFAE32, debug: 0xFFFFAEA6 };

/// `Block_Stage_Gen`.
pub const BLOCK_STAGE_GEN: Pin = Pin { plain: 0xFFFFB648, debug: 0xFFFFB6BC };

/// `Pfx_Memo_Row`.
pub const PFX_MEMO_ROW: Pin = Pin { plain: 0xFFFFB64A, debug: 0xFFFFB6BE };

/// `Pfx_Memo_L16`.
pub const PFX_MEMO_L16: Pin = Pin { plain: 0xFFFFB64C, debug: 0xFFFFB6C0 };

/// `Pfx_Memo_H16`.
pub const PFX_MEMO_H16: Pin = Pin { plain: 0xFFFFB64E, debug: 0xFFFFB6C2 };

/// `Pfx_Memo_Gen`.
pub const PFX_MEMO_GEN: Pin = Pin { plain: 0xFFFFB650, debug: 0xFFFFB6C4 };

/// `Cs_Memo_Col`.
pub const CS_MEMO_COL: Pin = Pin { plain: 0xFFFFB652, debug: 0xFFFFB6C6 };

/// `Cs_Memo_T16`.
pub const CS_MEMO_T16: Pin = Pin { plain: 0xFFFFB654, debug: 0xFFFFB6C8 };

/// `Cs_Memo_B16`.
pub const CS_MEMO_B16: Pin = Pin { plain: 0xFFFFB656, debug: 0xFFFFB6CA };

/// `Cs_Memo_Gen`.
pub const CS_MEMO_GEN: Pin = Pin { plain: 0xFFFFB658, debug: 0xFFFFB6CC };

/// `Pfx_Memo_Mask`.
pub const PFX_MEMO_MASK: Pin = Pin { plain: 0xFFFF6998, debug: 0xFFFF6998 };

/// `Cs_Memo_Mask`.
pub const CS_MEMO_MASK: Pin = Pin { plain: 0xFFFF699A, debug: 0xFFFF699A };

/// `Cache_Spec_Gen_Ring`.
pub const CACHE_SPEC_GEN_RING: Pin = Pin { plain: 0xFFFF6982, debug: 0xFFFF6982 };

/// `Cache_Spec_Window`.
pub const CACHE_SPEC_WINDOW: Pin = Pin { plain: 0xFFFF6992, debug: 0xFFFF6992 };

/// `Cache_Spec_Blocked`.
pub const CACHE_SPEC_BLOCKED: Pin = Pin { plain: 0xFFFF6994, debug: 0xFFFF6994 };

/// `Cache_Spec_Skips`.
pub const CACHE_SPEC_SKIPS: Pin = Pin { plain: 0xFFFF6996, debug: 0xFFFF6996 };

/// `S4LZ_DecompressDict`.
pub const S4_LZ_DECOMPRESS_DICT: Pin = Pin { plain: 0x27AC, debug: 0x29F0 };

/// `Player_1`.
pub const PLAYER_1: Pin = Pin { plain: 0xFFFF8FB4, debug: 0xFFFF9028 };

/// `Cheat_Flags`.
pub const CHEAT_FLAGS: Pin = Pin { plain: 0xFFFFBD0A, debug: 0xFFFFEDB4 };

/// `Dynamic_Slots`.
pub const DYNAMIC_SLOTS: Pin = Pin { plain: 0xFFFF9054, debug: 0xFFFF90C8 };

/// `Ring_Buffer`.
pub const RING_BUFFER: Pin = Pin { plain: 0xFFFFAEE8, debug: 0xFFFFAF5C };

/// `Ring_Count`.
pub const RING_COUNT: Pin = Pin { plain: 0xFFFFB1E8, debug: 0xFFFFB25C };

/// `Ring_HighWater`.
pub const RING_HIGH_WATER: Pin = Pin { plain: 0xFFFFB1E9, debug: 0xFFFFB25D };

/// `Ring_Add_Dropped`.
pub const RING_ADD_DROPPED: Pin = Pin { plain: 0xFFFFB1EA, debug: 0xFFFFB25E };

/// `Ring_Counter`.
pub const RING_COUNTER: Pin = Pin { plain: 0xFFFFB254, debug: 0xFFFFB2C8 };

/// `Ring_Anim_Frame`.
pub const RING_ANIM_FRAME: Pin = Pin { plain: 0xFFFFB256, debug: 0xFFFFB2CA };

/// `Ring_Anim_Timer`.
pub const RING_ANIM_TIMER: Pin = Pin { plain: 0xFFFFB257, debug: 0xFFFFB2CB };

/// `Camera_X`.
pub const CAMERA_X: Pin = Pin { plain: 0xFFFFA6E2, debug: 0xFFFFA756 };

/// `Camera_Y`.
pub const CAMERA_Y: Pin = Pin { plain: 0xFFFFA6E6, debug: 0xFFFFA75A };

/// `Camera_Target`.
pub const CAMERA_TARGET: Pin = Pin { plain: 0xFFFFADFE, debug: 0xFFFFAE72 };

/// `Camera_Curl_Offset`.
pub const CAMERA_CURL_OFFSET: Pin = Pin { plain: 0xFFFFAE00, debug: 0xFFFFAE74 };

/// `Camera_Deadzone_Base`.
pub const CAMERA_DEADZONE_BASE: Pin = Pin { plain: 0xFFFFADF4, debug: 0xFFFFAE68 };

/// `Camera_Pan_Offset`.
pub const CAMERA_PAN_OFFSET: Pin = Pin { plain: 0xFFFFADF8, debug: 0xFFFFAE6C };

/// `Camera_Hold_Frames`.
pub const CAMERA_HOLD_FRAMES: Pin = Pin { plain: 0xFFFFAE02, debug: 0xFFFFAE76 };

/// `Camera_Art_Hold`.
pub const CAMERA_ART_HOLD: Pin = Pin { plain: 0xFFFFAE03, debug: 0xFFFFAE77 };

/// `Dbg_Cam_Clamp_Frames`, debug-shape consumer only (`debug_only`).
pub const DBG_CAM_CLAMP_FRAMES: u32 = 0xFFFF8FE8;

/// `Camera_X_Max`.
pub const CAMERA_X_MAX: Pin = Pin { plain: 0xFFFFADFA, debug: 0xFFFFAE6E };

/// `Camera_Y_Max`.
pub const CAMERA_Y_MAX: Pin = Pin { plain: 0xFFFFADFC, debug: 0xFFFFAE70 };

/// `BgAnim_LastStep`.
pub const BG_ANIM_LAST_STEP: Pin = Pin { plain: 0xFFFF8F4A, debug: 0xFFFF8F4A };

/// `BgAnim_Table`.
pub const BG_ANIM_TABLE: Pin = Pin { plain: 0x26316, debug: 0x2CC78 };

/// `Camera_X_Biased`.
pub const CAMERA_X_BIASED: Pin = Pin { plain: 0xFFFFA6EA, debug: 0xFFFFA75E };

/// `Camera_Y_Biased`.
pub const CAMERA_Y_BIASED: Pin = Pin { plain: 0xFFFFA6EC, debug: 0xFFFFA760 };

/// `Collected_MarkRing`.
pub const COLLECTED_MARK_RING: Pin = Pin { plain: 0x3BE8, debug: 0x458C };

/// `EntityWindow_EntryForSection`.
pub const ENTITY_WINDOW_ENTRY_FOR_SECTION: Pin = Pin { plain: 0x3DE8, debug: 0x49EE };

/// `EntityLoaded_Clear`.
pub const ENTITY_LOADED_CLEAR: Pin = Pin { plain: 0x3DD4, debug: 0x4978 };

/// `Sound_PlayRing`.
pub const SOUND_PLAY_RING: Pin = Pin { plain: 0x90C4, debug: 0xC904 };

/// `MDDBG__ErrorHandler`, debug-shape consumer only (`debug_only`).
pub const MDDBG_ERROR_HANDLER: u32 = 0xC4D34;

/// `MDDBG__ErrorHandler_PagesController`, debug-shape consumer only (`debug_only`).
pub const MDDBG_ERROR_HANDLER_PAGES_CONTROLLER: u32 = 0xC5AFA;

/// `DMA_Critical`.
pub const DMA_CRITICAL: Pin = Pin { plain: 0xFFFF804A, debug: 0xFFFF804A };

/// `DMA_Critical_End`.
pub const DMA_CRITICAL_END: Pin = Pin { plain: 0xFFFF80BA, debug: 0xFFFF80BA };

/// `DMA_Important`.
pub const DMA_IMPORTANT: Pin = Pin { plain: 0xFFFF80BA, debug: 0xFFFF80BA };

/// `DMA_Important_End`.
pub const DMA_IMPORTANT_END: Pin = Pin { plain: 0xFFFF8162, debug: 0xFFFF8162 };

/// `DMA_Deferrable`.
pub const DMA_DEFERRABLE: Pin = Pin { plain: 0xFFFF8162, debug: 0xFFFF8162 };

/// `DMA_Deferrable_End`.
pub const DMA_DEFERRABLE_END: Pin = Pin { plain: 0xFFFF820A, debug: 0xFFFF820A };

/// `DMA_Critical_Slot`.
pub const DMA_CRITICAL_SLOT: Pin = Pin { plain: 0xFFFF820A, debug: 0xFFFF820A };

/// `DMA_Important_Slot`.
pub const DMA_IMPORTANT_SLOT: Pin = Pin { plain: 0xFFFF820C, debug: 0xFFFF820C };

/// `DMA_Deferrable_Slot`.
pub const DMA_DEFERRABLE_SLOT: Pin = Pin { plain: 0xFFFF820E, debug: 0xFFFF820E };

/// `DMA_Budget_Remaining`.
pub const DMA_BUDGET_REMAINING: Pin = Pin { plain: 0xFFFF8212, debug: 0xFFFF8212 };

/// `DMA_Enq_Bytes_Frame`.
pub const DMA_ENQ_BYTES_FRAME: Pin = Pin { plain: 0xFFFF8214, debug: 0xFFFF8214 };

/// `Act_Art_Budget`.
pub const ACT_ART_BUDGET: Pin = Pin { plain: 0xFFFFBA44, debug: 0xFFFFBAB8 };

/// `Art_Budget_Remaining`.
pub const ART_BUDGET_REMAINING: Pin = Pin { plain: 0xFFFFBA46, debug: 0xFFFFBABA };

/// `PageIn_Pool_Pages`.
pub const PAGE_IN_POOL_PAGES: Pin = Pin { plain: 0xFFFFBA38, debug: 0xFFFFBAAC };

/// `PageIn_Bulk_Drain`.
pub const PAGE_IN_BULK_DRAIN: Pin = Pin { plain: 0xFFFFBA33, debug: 0xFFFFBAA7 };

/// `PageIn_Fully_Resident`.
pub const PAGE_IN_FULLY_RESIDENT: Pin = Pin { plain: 0xFFFFBA3A, debug: 0xFFFFBAAE };

/// `Block_Stage_Maps`.
pub const BLOCK_STAGE_MAPS: Pin = Pin { plain: 0xFFFFB6A0, debug: 0xFFFFB714 };

/// `Cache_Cur_LocalMap`.
pub const CACHE_CUR_LOCAL_MAP: Pin = Pin { plain: 0xFFFFB6E0, debug: 0xFFFFB754 };

/// `PageCache_Direct_Map`.
pub const PAGE_CACHE_DIRECT_MAP: Pin = Pin { plain: 0xFFFFBA3B, debug: 0xFFFFBAAF };

/// `Page_Table`.
pub const PAGE_TABLE: Pin = Pin { plain: 0xFFFF699C, debug: 0xFFFF699C };

/// `Dbg_DMA_Enq_Capped`, debug-shape consumer only (`debug_only`).
pub const DBG_DMA_ENQ_CAPPED: u32 = 0xFFFF8FBE;

/// `DMA_Overflow_Count`, debug-shape consumer only (`debug_only`).
pub const DMA_OVERFLOW_COUNT: u32 = 0xFFFF8FBC;

/// `Art_Staging_Buffer`.
pub const ART_STAGING_BUFFER: Pin = Pin { plain: 0xFFFF6B34, debug: 0xFFFF6B34 };

/// `S4LZ_Decompress`.
pub const S4_LZ_DECOMPRESS: Pin = Pin { plain: 0x27B0, debug: 0x2A48 };

/// `QueueDMA_Critical`.
pub const QUEUE_DMA_CRITICAL: Pin = Pin { plain: 0x1D98, debug: 0x1E76 };

/// `BG_Init`.
pub const BG_INIT: Pin = Pin { plain: 0x8B16, debug: 0xB1D8 };

/// `QueueDMA_Important`.
pub const QUEUE_DMA_IMPORTANT: Pin = Pin { plain: 0x1DA2, debug: 0x1E80 };

/// `QueueDMA_Deferrable`.
pub const QUEUE_DMA_DEFERRABLE: Pin = Pin { plain: 0x1DAC, debug: 0x1E8A };

/// `Object_RAM`.
pub const OBJECT_RAM: Pin = Pin { plain: 0xFFFF8FB4, debug: 0xFFFF9028 };

/// `System_Slots`.
pub const SYSTEM_SLOTS: Pin = Pin { plain: 0xFFFF9CD4, debug: 0xFFFF9D48 };

/// `Effect_Slots`.
pub const EFFECT_SLOTS: Pin = Pin { plain: 0xFFFF9F54, debug: 0xFFFF9FC8 };

/// `Game_Paused`.
pub const GAME_PAUSED: Pin = Pin { plain: 0xFFFFA6EE, debug: 0xFFFFA762 };

/// `Object_RAM_End`.
pub const OBJECT_RAM_END: Pin = Pin { plain: 0xFFFFA454, debug: 0xFFFFA4C8 };

/// `Dynamic_Free_Stack`.
pub const DYNAMIC_FREE_STACK: Pin = Pin { plain: 0xFFFFA454, debug: 0xFFFFA4C8 };

/// `Dynamic_Free_SP`.
pub const DYNAMIC_FREE_SP: Pin = Pin { plain: 0xFFFFA4A4, debug: 0xFFFFA518 };

/// `Effect_Free_Stack`.
pub const EFFECT_FREE_STACK: Pin = Pin { plain: 0xFFFFA4A6, debug: 0xFFFFA51A };

/// `Effect_Free_SP`.
pub const EFFECT_FREE_SP: Pin = Pin { plain: 0xFFFFA4C6, debug: 0xFFFFA53A };

/// `Dynamic_Live`.
pub const DYNAMIC_LIVE: Pin = Pin { plain: 0xFFFFB5E2, debug: 0xFFFFB656 };

/// `Dynamic_Live_Count`.
pub const DYNAMIC_LIVE_COUNT: Pin = Pin { plain: 0xFFFFB632, debug: 0xFFFFB6A6 };

/// `Dynamic_Live_Dirty`.
pub const DYNAMIC_LIVE_DIRTY: Pin = Pin { plain: 0xFFFFB634, debug: 0xFFFFB6A8 };

/// `Dynamic_Live_Walking`, debug-shape consumer only (`debug_only`).
pub const DYNAMIC_LIVE_WALKING: u32 = 0xFFFFB6A9;

/// `Dynamic_Live_Pending`.
pub const DYNAMIC_LIVE_PENDING: Pin = Pin { plain: 0xFFFFB636, debug: 0xFFFFB6AA };

/// `Dynamic_Live_Pending_Count`.
pub const DYNAMIC_LIVE_PENDING_COUNT: Pin = Pin { plain: 0xFFFFB646, debug: 0xFFFFB6BA };

/// `DeleteObject`.
pub const DELETE_OBJECT: Pin = Pin { plain: 0x2ED4, debug: 0x316C };

/// `DrawRings`.
pub const DRAW_RINGS: Pin = Pin { plain: 0x3A70, debug: 0x4406 };

/// `Sprite_Table_Buffer`.
pub const SPRITE_TABLE_BUFFER: Pin = Pin { plain: 0xFFFF8298, debug: 0xFFFF8298 };

/// `Sprite_Table_Dirty`.
pub const SPRITE_TABLE_DIRTY: Pin = Pin { plain: 0xFFFF8518, debug: 0xFFFF8518 };

/// `Sprite_Emit_Active`.
pub const SPRITE_EMIT_ACTIVE: Pin = Pin { plain: 0xFFFF8519, debug: 0xFFFF8519 };

/// `Sprite_Bands`.
pub const SPRITE_BANDS: Pin = Pin { plain: 0xFFFFA4C8, debug: 0xFFFFA53C };

/// `Sprite_Band_Counts`.
pub const SPRITE_BAND_COUNTS: Pin = Pin { plain: 0xFFFFA6C8, debug: 0xFFFFA73C };

/// `Sprites_Rendered`.
pub const SPRITES_RENDERED: Pin = Pin { plain: 0xFFFFA6D0, debug: 0xFFFFA744 };

/// `Sprite_Cycle_Counter`.
pub const SPRITE_CYCLE_COUNTER: Pin = Pin { plain: 0xFFFFA6D2, debug: 0xFFFFA746 };

/// `Sprite_Owner`, debug-shape consumer only (`debug_only`).
pub const SPRITE_OWNER: u32 = 0xFFFFE47A;

/// `SpriteMask_Y`.
pub const SPRITE_MASK_Y: Pin = Pin { plain: 0xFFFFA6D4, debug: 0xFFFFA748 };

/// `SpriteMask_Height`.
pub const SPRITE_MASK_HEIGHT: Pin = Pin { plain: 0xFFFFA6D6, debug: 0xFFFFA74A };

/// `SpriteMask_After_Band`.
pub const SPRITE_MASK_AFTER_BAND: Pin = Pin { plain: 0xFFFFA6D8, debug: 0xFFFFA74C };

/// `Scanline_Band_Sprites`.
pub const SCANLINE_BAND_SPRITES: Pin = Pin { plain: 0xFFFFA6DA, debug: 0xFFFFA74E };

/// `Sound_PlaySFX`.
pub const SOUND_PLAY_SFX: Pin = Pin { plain: 0x9008, debug: 0xC802 };

/// `ObjectMoveX`.
pub const OBJECT_MOVE_X: Pin = Pin { plain: 0x30D8, debug: 0x37C2 };

/// `ObjCodeBase`.
pub const OBJ_CODE_BASE: Pin = Pin { plain: 0x10000, debug: 0x10000 };

/// `Draw_Sprite`.
pub const DRAW_SPRITE: Pin = Pin { plain: 0x3108, debug: 0x37F2 };

/// `ObjectMove`.
pub const OBJECT_MOVE: Pin = Pin { plain: 0x30BE, debug: 0x37A8 };

/// `Ring_Sfx_Speaker`.
pub const RING_SFX_SPEAKER: Pin = Pin { plain: 0xFFFFB524, debug: 0xFFFFB598 };

/// `Sfx_Ring_Buf`.
pub const SFX_RING_BUF: Pin = Pin { plain: 0xFFFFB526, debug: 0xFFFFB59A };

/// `Sfx_Ring_Wr`.
pub const SFX_RING_WR: Pin = Pin { plain: 0xFFFFB52E, debug: 0xFFFFB5A2 };

/// `Sfx_Ring_Rd`.
pub const SFX_RING_RD: Pin = Pin { plain: 0xFFFFB52F, debug: 0xFFFFB5A3 };

/// `SongTable`.
pub const SONG_TABLE: Pin = Pin { plain: 0xBDC2C, debug: 0xBF674 };

/// `SongPatchTable`.
pub const SONG_PATCH_TABLE: Pin = Pin { plain: 0xBDC44, debug: 0xBF694 };

/// `OJZ_Palette`.
pub const OJZ_PALETTE: Pin = Pin { plain: 0x21A94, debug: 0x22534 };

/// `OJZ_Act1_BG_Layout`.
pub const OJZ_ACT1_BG_LAYOUT: Pin = Pin { plain: 0x21B14, debug: 0x225B4 };

/// `OJZ_Act1_BG_Tiles`.
pub const OJZ_ACT1_BG_TILES: Pin = Pin { plain: 0x23B14, debug: 0x245B4 };

/// `ParallaxConfig_OJZ_Default`.
pub const PARALLAX_CONFIG_OJZ_DEFAULT: Pin = Pin { plain: 0x12C8E, debug: 0x134B0 };

/// `OJZ_Act_Pool_PageTable`.
pub const OJZ_ACT_POOL_PAGE_TABLE: Pin = Pin { plain: 0x17F64, debug: 0x189B0 };

/// `OJZ_Sec_LocalMaps`.
pub const OJZ_SEC_LOCAL_MAPS: Pin = Pin { plain: 0x21A70, debug: 0x22510 };

/// `OJZ_Sec0_Blocks`.
pub const OJZ_SEC0_BLOCKS: Pin = Pin { plain: 0x181F4, debug: 0x18C94 };

/// `OJZ_Sec1_Blocks`.
pub const OJZ_SEC1_BLOCKS: Pin = Pin { plain: 0x19EA8, debug: 0x1A948 };

/// `OJZ_Sec2_Blocks`.
pub const OJZ_SEC2_BLOCKS: Pin = Pin { plain: 0x1AEC6, debug: 0x1B966 };

/// `OJZ_Sec3_Blocks`.
pub const OJZ_SEC3_BLOCKS: Pin = Pin { plain: 0x1C15A, debug: 0x1CBFA };

/// `OJZ_Sec4_Blocks`.
pub const OJZ_SEC4_BLOCKS: Pin = Pin { plain: 0x1AEC6, debug: 0x1B966 };

/// `OJZ_Sec5_Blocks`.
pub const OJZ_SEC5_BLOCKS: Pin = Pin { plain: 0x1CF68, debug: 0x1DA08 };

/// `OJZ_Sec6_Blocks`.
pub const OJZ_SEC6_BLOCKS: Pin = Pin { plain: 0x1D964, debug: 0x1E404 };

/// `OJZ_Sec7_Blocks`.
pub const OJZ_SEC7_BLOCKS: Pin = Pin { plain: 0x1F02A, debug: 0x1FACA };

/// `OJZ_Sec8_Blocks`.
pub const OJZ_SEC8_BLOCKS: Pin = Pin { plain: 0x1FF48, debug: 0x209E8 };

/// `OJZ_Sec0_Objects`.
pub const OJZ_SEC0_OBJECTS: Pin = Pin { plain: 0x14F24, debug: 0x15970 };

/// `OJZ_Sec0_Rings`.
pub const OJZ_SEC0_RINGS: Pin = Pin { plain: 0x14F56, debug: 0x159A2 };

/// `OJZ_Sec0_TypeTable`.
pub const OJZ_SEC0_TYPE_TABLE: Pin = Pin { plain: 0x14F1A, debug: 0x15966 };

/// `OJZ_Sec1_Objects`.
pub const OJZ_SEC1_OBJECTS: Pin = Pin { plain: 0x14F7C, debug: 0x159C8 };

/// `OJZ_Sec1_Rings`.
pub const OJZ_SEC1_RINGS: Pin = Pin { plain: 0x14F84, debug: 0x159D0 };

/// `OJZ_Sec1_TypeTable`.
pub const OJZ_SEC1_TYPE_TABLE: Pin = Pin { plain: 0x14F76, debug: 0x159C2 };

/// `OJZ_Sec2_Objects`.
pub const OJZ_SEC2_OBJECTS: Pin = Pin { plain: 0x14FB6, debug: 0x15A02 };

/// `OJZ_Sec2_Rings`.
pub const OJZ_SEC2_RINGS: Pin = Pin { plain: 0x14FC4, debug: 0x15A10 };

/// `OJZ_Sec2_TypeTable`.
pub const OJZ_SEC2_TYPE_TABLE: Pin = Pin { plain: 0x14FAC, debug: 0x159F8 };

/// `OJZ_Sec3_Objects`.
pub const OJZ_SEC3_OBJECTS: Pin = Pin { plain: 0x14FFA, debug: 0x15A46 };

/// `OJZ_Sec3_Rings`.
pub const OJZ_SEC3_RINGS: Pin = Pin { plain: 0x14FFC, debug: 0x15A48 };

/// `OJZ_Sec3_TypeTable`.
pub const OJZ_SEC3_TYPE_TABLE: Pin = Pin { plain: 0x14FF8, debug: 0x15A44 };

/// `OJZ_Sec4_Objects`.
pub const OJZ_SEC4_OBJECTS: Pin = Pin { plain: 0x15002, debug: 0x15A4E };

/// `OJZ_Sec4_Rings`.
pub const OJZ_SEC4_RINGS: Pin = Pin { plain: 0x15004, debug: 0x15A50 };

/// `OJZ_Sec4_TypeTable`.
pub const OJZ_SEC4_TYPE_TABLE: Pin = Pin { plain: 0x15000, debug: 0x15A4C };

/// `OJZ_Sec5_Objects`.
pub const OJZ_SEC5_OBJECTS: Pin = Pin { plain: 0x1503A, debug: 0x15A86 };

/// `OJZ_Sec5_Rings`.
pub const OJZ_SEC5_RINGS: Pin = Pin { plain: 0x1503C, debug: 0x15A88 };

/// `OJZ_Sec5_TypeTable`.
pub const OJZ_SEC5_TYPE_TABLE: Pin = Pin { plain: 0x15038, debug: 0x15A84 };

/// `OJZ_Sec6_Objects`.
pub const OJZ_SEC6_OBJECTS: Pin = Pin { plain: 0x15062, debug: 0x15AAE };

/// `OJZ_Sec6_Rings`.
pub const OJZ_SEC6_RINGS: Pin = Pin { plain: 0x15064, debug: 0x15AB0 };

/// `OJZ_Sec6_TypeTable`.
pub const OJZ_SEC6_TYPE_TABLE: Pin = Pin { plain: 0x15060, debug: 0x15AAC };

/// `OJZ_Sec7_Objects`.
pub const OJZ_SEC7_OBJECTS: Pin = Pin { plain: 0x1506A, debug: 0x15AB6 };

/// `OJZ_Sec7_Rings`.
pub const OJZ_SEC7_RINGS: Pin = Pin { plain: 0x1506C, debug: 0x15AB8 };

/// `OJZ_Sec7_TypeTable`.
pub const OJZ_SEC7_TYPE_TABLE: Pin = Pin { plain: 0x15068, debug: 0x15AB4 };

/// `OJZ_Sec8_Objects`.
pub const OJZ_SEC8_OBJECTS: Pin = Pin { plain: 0x15092, debug: 0x15ADE };

/// `OJZ_Sec8_Rings`.
pub const OJZ_SEC8_RINGS: Pin = Pin { plain: 0x15094, debug: 0x15AE0 };

/// `OJZ_Sec8_TypeTable`.
pub const OJZ_SEC8_TYPE_TABLE: Pin = Pin { plain: 0x15090, debug: 0x15ADC };

/// `BLOCK_INDEX_SIZE`.
pub const BLOCK_INDEX_SIZE: Pin = Pin { plain: 0x400, debug: 0x400 };

/// `EDGE_CLAMP`.
pub const EDGE_CLAMP: Pin = Pin { plain: 0x0, debug: 0x0 };

/// `MAX_ACT_SECTIONS`.
pub const MAX_ACT_SECTIONS: Pin = Pin { plain: 0x30, debug: 0x30 };

/// `SECTION_SIZE_SHIFT`.
pub const SECTION_SIZE_SHIFT: Pin = Pin { plain: 0xB, debug: 0xB };

/// `Act_len`.
pub const ACT_LEN: Pin = Pin { plain: 0x32, debug: 0x32 };

/// `Sec_len`.
pub const SEC_LEN: Pin = Pin { plain: 0x16, debug: 0x16 };

/// `Camera_Y_Coarse_Prev`.
pub const CAMERA_Y_COARSE_PREV: Pin = Pin { plain: 0xFFFFB364, debug: 0xFFFFB3D8 };

/// `Current_Act_Ptr`.
pub const CURRENT_ACT_PTR: Pin = Pin { plain: 0xFFFFB520, debug: 0xFFFFB594 };

/// `Entity_Window_Active`.
pub const ENTITY_WINDOW_ACTIVE: Pin = Pin { plain: 0xFFFFB258, debug: 0xFFFFB2CC };

/// `Entity_Window_Anchor`.
pub const ENTITY_WINDOW_ANCHOR: Pin = Pin { plain: 0xFFFFB25A, debug: 0xFFFFB2CE };

/// `Entity_Window_OriginX`.
pub const ENTITY_WINDOW_ORIGIN_X: Pin = Pin { plain: 0xFFFFB25C, debug: 0xFFFFB2D0 };

/// `Entity_Window_OriginY`.
pub const ENTITY_WINDOW_ORIGIN_Y: Pin = Pin { plain: 0xFFFFB25E, debug: 0xFFFFB2D2 };

/// `Entity_Window_Center_ID`.
pub const ENTITY_WINDOW_CENTER_ID: Pin = Pin { plain: 0xFFFFB259, debug: 0xFFFFB2CD };

/// `Entity_Scan_State`.
pub const ENTITY_SCAN_STATE: Pin = Pin { plain: 0xFFFFB1EC, debug: 0xFFFFB260 };

/// `Entity_Loaded_Masks`.
pub const ENTITY_LOADED_MASKS: Pin = Pin { plain: 0xFFFFB260, debug: 0xFFFFB2D4 };

/// `Entity_Mask_Scratch`.
pub const ENTITY_MASK_SCRATCH: Pin = Pin { plain: 0xFFFFB2E0, debug: 0xFFFFB354 };

/// `Ring_Collected_Window`.
pub const RING_COLLECTED_WINDOW: Pin = Pin { plain: 0xFFFFB366, debug: 0xFFFFB3DA };

/// `Ring_Collected_Park`.
pub const RING_COLLECTED_PARK: Pin = Pin { plain: 0xFFFFB49A, debug: 0xFFFFB50E };

/// `Collected_Park_Next`.
pub const COLLECTED_PARK_NEXT: Pin = Pin { plain: 0xFFFFB51E, debug: 0xFFFFB592 };

/// `RingBuffer_Clear`.
pub const RING_BUFFER_CLEAR: Pin = Pin { plain: 0x3A3C, debug: 0x43D2 };

/// `RingBuffer_Remove`.
pub const RING_BUFFER_REMOVE: Pin = Pin { plain: 0x3A08, debug: 0x439E };

/// `Section_GetSecPtrXY`.
pub const SECTION_GET_SEC_PTR_XY: Pin = Pin { plain: 0x6082, debug: 0x755A };

/// `Section_FlatIDXY`.
pub const SECTION_FLAT_IDXY: Pin = Pin { plain: 0x6068, debug: 0x7540 };

/// `AllocDynamic`.
pub const ALLOC_DYNAMIC: Pin = Pin { plain: 0x2E56, debug: 0x30EE };

/// `AllocEffect`.
pub const ALLOC_EFFECT: Pin = Pin { plain: 0x2EBA, debug: 0x3152 };

/// `Palette_Buffer`.
pub const PALETTE_BUFFER: Pin = Pin { plain: 0xFFFF8216, debug: 0xFFFF8216 };

/// `Hscroll_Buffer`.
pub const HSCROLL_BUFFER: Pin = Pin { plain: 0xFFFF851A, debug: 0xFFFF851A };

/// `Static_Pal_Line0`.
pub const STATIC_PAL_LINE0: Pin = Pin { plain: 0xFFFF8F52, debug: 0xFFFF8F52 };

/// `Static_Pal_Line1`.
pub const STATIC_PAL_LINE1: Pin = Pin { plain: 0xFFFF8F60, debug: 0xFFFF8F60 };

/// `Static_Pal_Line2`.
pub const STATIC_PAL_LINE2: Pin = Pin { plain: 0xFFFF8F6E, debug: 0xFFFF8F6E };

/// `Static_Pal_Line3`.
pub const STATIC_PAL_LINE3: Pin = Pin { plain: 0xFFFF8F8A, debug: 0xFFFF8F8A };

/// `Static_Sprite_DMA`.
pub const STATIC_SPRITE_DMA: Pin = Pin { plain: 0xFFFF8F98, debug: 0xFFFF8F98 };

/// `Static_Hscroll_Line`.
pub const STATIC_HSCROLL_LINE: Pin = Pin { plain: 0xFFFF8FA6, debug: 0xFFFF8FA6 };

/// `Palette_Dirty`.
pub const PALETTE_DIRTY: Pin = Pin { plain: 0xFFFF8296, debug: 0xFFFF8296 };

/// `Parallax_Active_Config`.
pub const PARALLAX_ACTIVE_CONFIG: Pin = Pin { plain: 0x6878, debug: 0x8230 };

/// `Palette_Ship_Snap`.
pub const PALETTE_SHIP_SNAP: Pin = Pin { plain: 0xFFFFBA4C, debug: 0xFFFFBAC0 };

/// `VBlank_Ready`.
pub const V_BLANK_READY: Pin = Pin { plain: 0xFFFF8048, debug: 0xFFFF8048 };

/// `VBlank_Flag`.
pub const V_BLANK_FLAG: Pin = Pin { plain: 0xFFFF8000, debug: 0xFFFF8000 };

/// `VInt_Ptr`.
pub const V_INT_PTR: Pin = Pin { plain: 0xFFFF8044, debug: 0xFFFF8044 };

/// `Ctrl_1_Press`.
pub const CTRL_1_PRESS: Pin = Pin { plain: 0xFFFF8029, debug: 0xFFFF8029 };

/// `Ctrl_1_Press_Accum`.
pub const CTRL_1_PRESS_ACCUM: Pin = Pin { plain: 0xFFFF802C, debug: 0xFFFF802C };

/// `Ctrl_2_Press`.
pub const CTRL_2_PRESS: Pin = Pin { plain: 0xFFFF802B, debug: 0xFFFF802B };

/// `Ctrl_2_Press_Accum`.
pub const CTRL_2_PRESS_ACCUM: Pin = Pin { plain: 0xFFFF802D, debug: 0xFFFF802D };

/// `Ctrl_1_Ext_Press`.
pub const CTRL_1_EXT_PRESS: Pin = Pin { plain: 0xFFFF802F, debug: 0xFFFF802F };

/// `Ctrl_1_Ext_Press_Accum`.
pub const CTRL_1_EXT_PRESS_ACCUM: Pin = Pin { plain: 0xFFFF8032, debug: 0xFFFF8032 };

/// `Ctrl_2_Ext_Press`.
pub const CTRL_2_EXT_PRESS: Pin = Pin { plain: 0xFFFF8031, debug: 0xFFFF8031 };

/// `Ctrl_2_Ext_Press_Accum`.
pub const CTRL_2_EXT_PRESS_ACCUM: Pin = Pin { plain: 0xFFFF8033, debug: 0xFFFF8033 };

/// `Parallax_State`.
pub const PARALLAX_STATE: Pin = Pin { plain: 0xFFFF88A0, debug: 0xFFFF88A0 };

/// `Vscroll_Factor`.
pub const VSCROLL_FACTOR: Pin = Pin { plain: 0xFFFF889C, debug: 0xFFFF889C };

/// `DMA_Budget_Default`.
pub const DMA_BUDGET_DEFAULT: Pin = Pin { plain: 0xFFFF8210, debug: 0xFFFF8210 };

/// `Lag_Frame_Count`, debug-shape consumer only (`debug_only`).
pub const LAG_FRAME_COUNT: u32 = 0xFFFF8FC0;

/// `DMA_Bytes_ThisFrame`, debug-shape consumer only (`debug_only`).
pub const DMA_BYTES_THIS_FRAME: u32 = 0xFFFF8FB4;

/// `PageIn_InFlight`.
pub const PAGE_IN_IN_FLIGHT: Pin = Pin { plain: 0xFFFFBA06, debug: 0xFFFFBA7A };

/// `PageIn_Saved_PC`.
pub const PAGE_IN_SAVED_PC: Pin = Pin { plain: 0xFFFFBA00, debug: 0xFFFFBA74 };

/// `PageIn_BankRegs`.
pub const PAGE_IN_BANK_REGS: Pin = Pin { plain: 0x8132, debug: 0x9C3A };

/// `Dbg_PageIn_Preempts`, debug-shape consumer only (`debug_only`).
pub const DBG_PAGE_IN_PREEMPTS: u32 = 0xFFFF8FDA;

/// `ZX0R_Decompress.__end`.
pub const ZX0R_DECOMPRESS_END: Pin = Pin { plain: 0x296A, debug: 0x2C02 };

/// `PageIn_Staging_Busy`.
pub const PAGE_IN_STAGING_BUSY: Pin = Pin { plain: 0xFFFFBA08, debug: 0xFFFFBA7C };

/// `PageIn_Flush`.
pub const PAGE_IN_FLUSH: Pin = Pin { plain: 0x824E, debug: 0x9D5E };

/// `PageIn_Enqueue`.
pub const PAGE_IN_ENQUEUE: Pin = Pin { plain: 0x8210, debug: 0x9D20 };

/// `PageIn_Pool_Table`.
pub const PAGE_IN_POOL_TABLE: Pin = Pin { plain: 0xFFFFBA34, debug: 0xFFFFBAA8 };

/// `PageIn_Queue_Count`.
pub const PAGE_IN_QUEUE_COUNT: Pin = Pin { plain: 0xFFFFBA0A, debug: 0xFFFFBA7E };

/// `PageIn_Suspended`.
pub const PAGE_IN_SUSPENDED: Pin = Pin { plain: 0xFFFFBA07, debug: 0xFFFFBA7B };

/// `PageIn_Land_Pending`.
pub const PAGE_IN_LAND_PENDING: Pin = Pin { plain: 0xFFFFBA09, debug: 0xFFFFBA7D };

/// `PageCache_Init`.
pub const PAGE_CACHE_INIT: Pin = Pin { plain: 0x82B4, debug: 0x9DC8 };

/// `PageCache_AllocFrame`.
pub const PAGE_CACHE_ALLOC_FRAME: Pin = Pin { plain: 0x836A, debug: 0x9EE4 };

/// `PageCache_Publish`.
pub const PAGE_CACHE_PUBLISH: Pin = Pin { plain: 0x84A4, debug: 0xA0C6 };

/// `PageCache_PatchRun_Seq`.
pub const PAGE_CACHE_PATCH_RUN_SEQ: Pin = Pin { plain: 0x855A, debug: 0xA1E2 };

/// `PageCache_PatchRun_Col`.
pub const PAGE_CACHE_PATCH_RUN_COL: Pin = Pin { plain: 0x868E, debug: 0xA466 };

/// `PageCache_Audit`.
pub const PAGE_CACHE_AUDIT: Pin = Pin { plain: 0x8B14, debug: 0xAA5C };

/// `Cache_Art_Stall`.
pub const CACHE_ART_STALL: Pin = Pin { plain: 0xFFFFAE16, debug: 0xFFFFAE8A };

/// `Page_Audit_Ticks`, debug-shape consumer only (`debug_only`).
pub const PAGE_AUDIT_TICKS: u32 = 0xFFFF8FEE;

/// `Cache_Stall_Watchdog`, debug-shape consumer only (`debug_only`).
pub const CACHE_STALL_WATCHDOG: u32 = 0xFFFF8FEC;

/// `Flush_VDP_Shadow`.
pub const FLUSH_VDP_SHADOW: Pin = Pin { plain: 0x1C52, debug: 0x1CDA };

/// `VInt_DrawLevel`.
pub const V_INT_DRAW_LEVEL: Pin = Pin { plain: 0x4A94, debug: 0x5C1E };

/// `Vscroll_Write`.
pub const VSCROLL_WRITE: Pin = Pin { plain: 0x688A, debug: 0x8242 };

/// `Read_Controllers`.
pub const READ_CONTROLLERS: Pin = Pin { plain: 0x2538, debug: 0x26C8 };

/// `Process_DMA_Critical`.
pub const PROCESS_DMA_CRITICAL: Pin = Pin { plain: 0x1E72, debug: 0x1F6A };

/// `Process_DMA_Important`.
pub const PROCESS_DMA_IMPORTANT: Pin = Pin { plain: 0x1F40, debug: 0x2038 };

/// `Process_DMA_Deferrable`.
pub const PROCESS_DMA_DEFERRABLE: Pin = Pin { plain: 0x1F54, debug: 0x204C };

/// `Enqueue_Dirty_Buffers`.
pub const ENQUEUE_DIRTY_BUFFERS: Pin = Pin { plain: 0x2136, debug: 0x228C };

/// `BootData`.
pub const BOOT_DATA: Pin = Pin { plain: 0x3AC, debug: 0x3B2 };

/// `VInt_Level`.
pub const V_INT_LEVEL: Pin = Pin { plain: 0x2370, debug: 0x24CA };

/// `BuildStaticDMA`.
pub const BUILD_STATIC_DMA: Pin = Pin { plain: 0x2072, debug: 0x21C8 };

/// `Sound_Init`.
pub const SOUND_INIT: Pin = Pin { plain: 0x8F00, debug: 0xC596 };

/// `Hardware_Region`.
pub const HARDWARE_REGION: Pin = Pin { plain: 0xFFFF8026, debug: 0xFFFF8026 };

/// `Region_Flags`.
pub const REGION_FLAGS: Pin = Pin { plain: 0xFFFF8027, debug: 0xFFFF8027 };

/// `Game_State_ID`.
pub const GAME_STATE_ID: Pin = Pin { plain: 0xFFFF800C, debug: 0xFFFF800C };

/// `Game_State_Init`.
pub const GAME_STATE_INIT: Pin = Pin { plain: 0xFFFF800D, debug: 0xFFFF800D };

/// `RAM_Start`.
pub const RAM_START: Pin = Pin { plain: 0xFFFF8000, debug: 0xFFFF8000 };

/// `PState_Ground`.
pub const P_STATE_GROUND: Pin = Pin { plain: 0x109C8, debug: 0x10ADE };

/// `PState_Roll`.
pub const P_STATE_ROLL: Pin = Pin { plain: 0x10B68, debug: 0x10C7E };

/// `PState_Spindash`.
pub const P_STATE_SPINDASH: Pin = Pin { plain: 0x111D8, debug: 0x112EE };

/// `PState_Air`.
pub const P_STATE_AIR: Pin = Pin { plain: 0x10E8E, debug: 0x10FA4 };

/// `PState_Jump`.
pub const P_STATE_JUMP: Pin = Pin { plain: 0x10E96, debug: 0x10FAC };

/// `PState_RollJump`.
pub const P_STATE_ROLL_JUMP: Pin = Pin { plain: 0x10E92, debug: 0x10FA8 };

/// `PState_AirBall`.
pub const P_STATE_AIR_BALL: Pin = Pin { plain: 0x10E8E, debug: 0x10FA4 };

/// `PState_Fly`.
pub const P_STATE_FLY: Pin = Pin { plain: 0x112B6, debug: 0x113C8 };

/// `PState_Glide`.
pub const P_STATE_GLIDE: Pin = Pin { plain: 0x113F0, debug: 0x11500 };

/// `PState_GlideFall`.
pub const P_STATE_GLIDE_FALL: Pin = Pin { plain: 0x115AA, debug: 0x116BA };

/// `PState_Slide`.
pub const P_STATE_SLIDE: Pin = Pin { plain: 0x115F8, debug: 0x11706 };

/// `PState_Climb`.
pub const P_STATE_CLIMB: Pin = Pin { plain: 0x11726, debug: 0x11832 };

/// `PState_Ledge`.
pub const P_STATE_LEDGE: Pin = Pin { plain: 0x118D2, debug: 0x119DE };

/// `Player_SensorFloor`.
pub const PLAYER_SENSOR_FLOOR: Pin = Pin { plain: 0x5E66, debug: 0x733E };

/// `Player_AtLedgeEdge`.
pub const PLAYER_AT_LEDGE_EDGE: Pin = Pin { plain: 0x5F7E, debug: 0x7456 };

/// `Player_SetState`.
pub const PLAYER_SET_STATE: Pin = Pin { plain: 0x106F6, debug: 0x107B8 };

/// `Player_SnapToSurface`.
pub const PLAYER_SNAP_TO_SURFACE: Pin = Pin { plain: 0x10860, debug: 0x10922 };

/// `Player_SensorCeiling`.
pub const PLAYER_SENSOR_CEILING: Pin = Pin { plain: 0x5E7C, debug: 0x7354 };

/// `Player_SensorWallDir`.
pub const PLAYER_SENSOR_WALL_DIR: Pin = Pin { plain: 0x5F34, debug: 0x740C };

/// `Player_SensorWallAt`.
pub const PLAYER_SENSOR_WALL_AT: Pin = Pin { plain: 0x5F2C, debug: 0x7404 };

/// `Collision_GetType`.
pub const COLLISION_GET_TYPE: Pin = Pin { plain: 0x5A92, debug: 0x6F6A };

/// `SolidityTable`.
pub const SOLIDITY_TABLE: Pin = Pin { plain: 0x6EABA, debug: 0x754AE };

/// `AngleTable`.
pub const ANGLE_TABLE: Pin = Pin { plain: 0x6E9BA, debug: 0x753AE };

/// `HeightMaps`.
pub const HEIGHT_MAPS: Pin = Pin { plain: 0x6C9BA, debug: 0x733AE };

/// `HeightMapsRot`.
pub const HEIGHT_MAPS_ROT: Pin = Pin { plain: 0x6D9BA, debug: 0x743AE };

/// `Character_ID`.
pub const CHARACTER_ID: Pin = Pin { plain: 0xFFFFBD0C, debug: 0xFFFFEDB6 };

/// `Player_Chardef`.
pub const PLAYER_CHARDEF: Pin = Pin { plain: 0xFFFFBD0E, debug: 0xFFFFEDB8 };

/// `Ability_None`.
pub const ABILITY_NONE: Pin = Pin { plain: 0x12322, debug: 0x12490 };

/// `CharacterDefs`.
pub const CHARACTER_DEFS: Pin = Pin { plain: 0x122DA, debug: 0x123E2 };

/// `Player_InitAssets`.
pub const PLAYER_INIT_ASSETS: Pin = Pin { plain: 0x122E6, debug: 0x123EE };

/// `Player_LoadArt`.
pub const PLAYER_LOAD_ART: Pin = Pin { plain: 0x122FE, debug: 0x12406 };

/// `Player_Ability`.
pub const PLAYER_ABILITY: Pin = Pin { plain: 0x12318, debug: 0x12420 };

/// `PhysTable_Sonic`.
pub const PHYS_TABLE_SONIC: Pin = Pin { plain: 0x1225E, debug: 0x12366 };

/// `Pal_SonicTails`.
pub const PAL_SONIC_TAILS: Pin = Pin { plain: 0x6C97A, debug: 0x7336E };

/// `OJZ_TestRaster`.
pub const OJZ_TEST_RASTER: Pin = Pin { plain: 0x1459C, debug: 0x14DD0 };

/// `OJZ_TestPal`.
pub const OJZ_TEST_PAL: Pin = Pin { plain: 0x1461C, debug: 0x14F50 };

/// `OJZ_TestGradient`.
pub const OJZ_TEST_GRADIENT: Pin = Pin { plain: 0x14944, debug: 0x15278 };

/// `OJZ_ShimmerCycle`.
pub const OJZ_SHIMMER_CYCLE: Pin = Pin { plain: 0x1467C, debug: 0x14FB0 };

/// `OJZ_TestVsram`.
pub const OJZ_TEST_VSRAM: Pin = Pin { plain: 0x149C4, debug: 0x152F8 };

/// `OJZ_TestRamp`.
pub const OJZ_TEST_RAMP: Pin = Pin { plain: 0x14A44, debug: 0x15378 };

/// `Raster_Program`.
pub const RASTER_PROGRAM: Pin = Pin { plain: 0xFFFF8C1A, debug: 0xFFFF8C1A };

/// `Raster_Cursor`.
pub const RASTER_CURSOR: Pin = Pin { plain: 0xFFFF8C1E, debug: 0xFFFF8C1E };

/// `Raster_Pending`.
pub const RASTER_PENDING: Pin = Pin { plain: 0xFFFF8C22, debug: 0xFFFF8C22 };

/// `Raster_Buf_A`.
pub const RASTER_BUF_A: Pin = Pin { plain: 0xFFFF8C28, debug: 0xFFFF8C28 };

/// `Raster_Active_Buf`.
pub const RASTER_ACTIVE_BUF: Pin = Pin { plain: 0xFFFF8D28, debug: 0xFFFF8D28 };

/// `Raster_Buf_B`.
pub const RASTER_BUF_B: Pin = Pin { plain: 0xFFFF8CA8, debug: 0xFFFF8CA8 };

/// `Raster_Line`.
pub const RASTER_LINE: Pin = Pin { plain: 0xFFFF8C26, debug: 0xFFFF8C26 };

/// `Raster_Dense_Lines`.
pub const RASTER_DENSE_LINES: Pin = Pin { plain: 0xFFFF8D2C, debug: 0xFFFF8D2C };

/// `Raster_Dense_Cursor`.
pub const RASTER_DENSE_CURSOR: Pin = Pin { plain: 0xFFFF8D2E, debug: 0xFFFF8D2E };

/// `Raster_Dense_Cmd`.
pub const RASTER_DENSE_CMD: Pin = Pin { plain: 0xFFFF8D32, debug: 0xFFFF8D32 };

/// `Raster_Dense_Mode`.
pub const RASTER_DENSE_MODE: Pin = Pin { plain: 0xFFFF8D36, debug: 0xFFFF8D36 };

/// `Raster_Ramp_Acc`.
pub const RASTER_RAMP_ACC: Pin = Pin { plain: 0xFFFF8D38, debug: 0xFFFF8D38 };

/// `Raster_Ramp_Step`.
pub const RASTER_RAMP_STEP: Pin = Pin { plain: 0xFFFF8D3C, debug: 0xFFFF8D3C };

/// `Effects_World_Y`.
pub const EFFECTS_WORLD_Y: Pin = Pin { plain: 0xFFFF8D40, debug: 0xFFFF8D40 };

/// `Effects_Screen_L`.
pub const EFFECTS_SCREEN_L: Pin = Pin { plain: 0xFFFF8D48, debug: 0xFFFF8D48 };

/// `Effects_Offscreen_Entry`.
pub const EFFECTS_OFFSCREEN_ENTRY: Pin = Pin { plain: 0xFFFF8D6A, debug: 0xFFFF8D6A };

/// `Static_Pal_Ship`.
pub const STATIC_PAL_SHIP: Pin = Pin { plain: 0xFFFF8F7C, debug: 0xFFFF8F7C };

/// `Build_DMA_Entry`.
pub const BUILD_DMA_ENTRY: Pin = Pin { plain: 0x2100, debug: 0x2256 };

/// `Raster_Patch_Tab`.
pub const RASTER_PATCH_TAB: Pin = Pin { plain: 0xFFFF8D6E, debug: 0xFFFF8D6E };

/// `Raster_State`.
pub const RASTER_STATE: Pin = Pin { plain: 0xFFFF8C1A, debug: 0xFFFF8C1A };

/// `Raster_State_End`.
pub const RASTER_STATE_END: Pin = Pin { plain: 0xFFFF8D72, debug: 0xFFFF8D72 };

/// `Pal_Variant_Stage`.
pub const PAL_VARIANT_STAGE: Pin = Pin { plain: 0xFFFF8E32, debug: 0xFFFF8E32 };

/// `Raster_VBlank`.
pub const RASTER_V_BLANK: Pin = Pin { plain: 0x747C, debug: 0x8E80 };

/// `Palette_Compose`.
pub const PALETTE_COMPOSE: Pin = Pin { plain: 0x7920, debug: 0x9324 };

/// `Player_Blocks`.
pub const PLAYER_BLOCKS: Pin = Pin { plain: 0xFFFFBD12, debug: 0xFFFFEDBC };

/// `Player_Ring_Index`.
pub const PLAYER_RING_INDEX: Pin = Pin { plain: 0xFFFFC100, debug: 0xFFFFF100 };

/// `Player_Pos_Ring`.
pub const PLAYER_POS_RING: Pin = Pin { plain: 0xFFFFBF00, debug: 0xFFFFEF00 };

/// `Player_Stat_Ring`.
pub const PLAYER_STAT_RING: Pin = Pin { plain: 0xFFFFC000, debug: 0xFFFFF000 };

/// `Player_Death_Pending`.
pub const PLAYER_DEATH_PENDING: Pin = Pin { plain: 0xFFFFBD4A, debug: 0xFFFFEDF4 };

/// `Player_Bound_Right`.
pub const PLAYER_BOUND_RIGHT: Pin = Pin { plain: 0xFFFFBD4C, debug: 0xFFFFEDF6 };

/// `Player_Bound_Bottom`.
pub const PLAYER_BOUND_BOTTOM: Pin = Pin { plain: 0xFFFFBD4E, debug: 0xFFFFEDF8 };

/// `DustSpindash_Spawn`.
pub const DUST_SPINDASH_SPAWN: Pin = Pin { plain: 0x124DE, debug: 0x126A4 };

/// `Z80_Sound_Start`.
pub const Z80_SOUND_START: Pin = Pin { plain: 0x3DE, debug: 0x3E4 };

// ── Region-relative offsets (manifest order) ──

/// `AnimateSprite.cc_delete` − `animate` start (per-shape).
pub const CC_DELETE_OFF: ShapeOffset = ShapeOffset { plain: 0x104, debug: 0x15E };

/// `RefreshSpritePieceCount` − `animate` start (per-shape).
pub const REFRESH_OFF: ShapeOffset = ShapeOffset { plain: 0x16C, debug: 0x292 };

/// `RingCollision` − `rings` start (per-shape).
pub const RINGCOL_OFF: ShapeOffset = ShapeOffset { plain: 0x114, debug: 0x17A };

/// `Sound_PlaySFX` − `sound_api` start (per-shape).
pub const SOUND_PLAY_SFX_OFF: ShapeOffset = ShapeOffset { plain: 0x12E, debug: 0x292 };

/// `Sine_Table` − `math` start (shape-invariant, asserted at generation).
pub const SINE_TABLE_OFF: usize = 0x18;

/// `Flush_VDP_Shadow` − `vdp_init` start (shape-invariant, asserted at generation).
pub const FLUSH_VDP_SHADOW_OFF: usize = 0x12;

/// `HBlank_Uninstall` − `hblank` start (shape-invariant, asserted at generation).
pub const HBLANK_UNINSTALL_OFF: usize = 0x1C;
