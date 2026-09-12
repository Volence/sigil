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

/// Assembled (pre-convsym) ROM length, plain shape.
pub const ASSEMBLED_LEN: usize = 0xBDC92;
/// Assembled (pre-convsym) ROM length, `__DEBUG__` shape.
pub const DEBUG_ASSEMBLED_LEN: usize = 0xC14F4;

// ── Regions (manifest order) ──

/// `Vectors` .. start + 0x100 plain / 0x100 debug (literal, no end symbol), gate `SIGIL_EMP_VECTORS`.
pub const VECTORS: Region = Region { plain_base: 0x0, debug_base: 0x0, plain_len: 0x100, debug_len: 0x100 };

/// `GameHeader` .. `section:header`.
pub const HEADER: Region = Region { plain_base: 0x100, debug_base: 0x100, plain_len: 0x100, debug_len: 0x100 };

/// `HeightMaps` .. `section:collision_data`.
pub const COLLISION_DATA: Region = Region { plain_base: 0x6E8E0, debug_base: 0x6F30A, plain_len: 0x1D304, debug_len: 0x1D304 };

/// `EntryPoint` .. `section:boot`, gate `SIGIL_EMP_BOOT`.
pub const BOOT: Region = Region { plain_base: 0x200, debug_base: 0x200, plain_len: 0x198, debug_len: 0x1A0 };

/// `BootData` .. `section:boot_head`.
pub const BOOT_HEAD: Region = Region { plain_base: 0x398, debug_base: 0x3A0, plain_len: 0x1856, debug_len: 0x18D8 };

/// `BootData_PostBlob` .. `section:boot_tail`.
pub const BOOT_TAIL: Region = Region { plain_base: 0x1BEE, debug_base: 0x1C78, plain_len: 0xE, debug_len: 0xE };

/// `VDP_Shadow_Init` .. `section:vdp_init`, gate `SIGIL_EMP_VDP_INIT`.
pub const VDP_INIT: Region = Region { plain_base: 0x1BFC, debug_base: 0x1C86, plain_len: 0x3A, debug_len: 0x90 };

/// `Init_DMA_Queue` .. `section:dma_queue`, gate `SIGIL_EMP_DMA_QUEUE`.
pub const DMA_QUEUE: Region = Region { plain_base: 0x1C36, debug_base: 0x1D16, plain_len: 0x32C, debug_len: 0x346 };

/// `Init_SpriteTable` .. `section:buffers`, gate `SIGIL_EMP_BUFFERS`.
pub const BUFFERS: Region = Region { plain_base: 0x1F62, debug_base: 0x205C, plain_len: 0x2D8, debug_len: 0x2D8 };

/// `VBlank_Handler` .. `section:vblank`, gate `SIGIL_EMP_VBLANK`.
pub const VBLANK: Region = Region { plain_base: 0x223A, debug_base: 0x2334, plain_len: 0x1DC, debug_len: 0x20C };

/// `HBlank_Install` .. `section:hblank`, gate `SIGIL_EMP_HBLANK`.
pub const HBLANK: Region = Region { plain_base: 0x241C, debug_base: 0x2546, plain_len: 0x30, debug_len: 0x30 };

/// `Read_Controllers` .. `section:controllers`, gate `SIGIL_EMP_CONTROLLERS`.
pub const CONTROLLERS: Region = Region { plain_base: 0x244C, debug_base: 0x2576, plain_len: 0x10E, debug_len: 0x10E };

/// `GameLoop` .. `section:game_loop`, gate `SIGIL_EMP_GAME_LOOP`.
pub const GAME_LOOP: Region = Region { plain_base: 0x255A, debug_base: 0x2684, plain_len: 0x1C, debug_len: 0x1E };

/// `Input_Tick` .. `section:replay`.
pub const REPLAY: Region = Region { plain_base: 0x257C, debug_base: 0x26A8, plain_len: 0x146, debug_len: 0x1F6 };

/// `S4LZ_DecompressDict` .. `section:s4lz`, gate `SIGIL_EMP_S4LZ`.
pub const S4LZ: Region = Region { plain_base: 0x26C4, debug_base: 0x28A0, plain_len: 0xF8, debug_len: 0x200 };

/// `ZX0R_Decompress` .. `section:zx0_resume`.
pub const ZX0_RESUME: Region = Region { plain_base: 0x27BC, debug_base: 0x2AA0, plain_len: 0x78, debug_len: 0x78 };

/// `GetSineCosine` .. `section:math`, gate `SIGIL_EMP_MATH`.
pub const MATH: Region = Region { plain_base: 0x2834, debug_base: 0x2B18, plain_len: 0x3F6, debug_len: 0x3F6 };

/// `Perform_DPLC` .. `section:dplc`, gate `SIGIL_EMP_DPLC`.
pub const DPLC: Region = Region { plain_base: 0x2C2A, debug_base: 0x2F0E, plain_len: 0xA4, debug_len: 0xA4 };

/// `InitObjectRAM` .. `section:core`, gate `SIGIL_EMP_CORE`.
pub const CORE: Region = Region { plain_base: 0x2CCE, debug_base: 0x2FB2, plain_len: 0x2EE, debug_len: 0x740 };

/// `InitSpriteSystem` .. `section:sprites`, gate `SIGIL_EMP_SPRITES`.
pub const SPRITES: Region = Region { plain_base: 0x2FBC, debug_base: 0x36F2, plain_len: 0x41A, debug_len: 0x534 };

/// `AnimateSprite` .. `section:animate`, gate `SIGIL_EMP_ANIMATE`.
pub const ANIMATE: Region = Region { plain_base: 0x33D6, debug_base: 0x3C26, plain_len: 0x194, debug_len: 0x2B8 };

/// `TouchResponse` .. `section:collision`, gate `SIGIL_EMP_COLLISION`.
pub const COLLISION: Region = Region { plain_base: 0x356A, debug_base: 0x3EDE, plain_len: 0x306, debug_len: 0x30E };

/// `RingBuffer_Add` .. `section:rings`, gate `SIGIL_EMP_RINGS`.
pub const RINGS: Region = Region { plain_base: 0x3870, debug_base: 0x41EC, plain_len: 0x1BE, debug_len: 0x224 };

/// `Collected_Init` .. `section:entity_window`, gate `SIGIL_EMP_ENTITY_WINDOW`.
pub const ENTITY_WINDOW: Region = Region { plain_base: 0x3A2E, debug_base: 0x4410, plain_len: 0x8EE, debug_len: 0xD5C };

/// `PopulateSpawnedPieceCount` .. `section:children`, gate `SIGIL_EMP_CHILDREN`.
pub const CHILDREN: Region = Region { plain_base: 0x431C, debug_base: 0x516C, plain_len: 0x2EC, debug_len: 0x39C };

/// `Load_Object` .. `section:load_object`, gate `SIGIL_EMP_LOAD_OBJECT`.
pub const LOAD_OBJECT: Region = Region { plain_base: 0x4608, debug_base: 0x5508, plain_len: 0x88, debug_len: 0x88 };

/// `Plane_Buffer_Reset` .. `section:plane_buffer`, gate `SIGIL_EMP_PLANE_BUFFER`.
pub const PLANE_BUFFER: Region = Region { plain_base: 0x4690, debug_base: 0x5590, plain_len: 0x31A, debug_len: 0x4B6 };

/// `Tile_Cache_GetTile` .. `section:tile_cache`, gate `SIGIL_EMP_TILE_CACHE`.
pub const TILE_CACHE: Region = Region { plain_base: 0x49AA, debug_base: 0x5A46, plain_len: 0xE86, debug_len: 0x10EC };

/// `Collision_GetType` .. `section:collision_lookup`, gate `SIGIL_EMP_COLLISION_LOOKUP`.
pub const COLLISION_LOOKUP: Region = Region { plain_base: 0x5838, debug_base: 0x6B3C, plain_len: 0x68, debug_len: 0x68 };

/// `Section_Init` .. `section:section`, gate `SIGIL_EMP_SECTION`.
pub const SECTION: Region = Region { plain_base: 0x5D94, debug_base: 0x7098, plain_len: 0x460, debug_len: 0x890 };

/// `Camera_Init` .. `section:camera`, gate `SIGIL_EMP_CAMERA`.
pub const CAMERA: Region = Region { plain_base: 0x61F4, debug_base: 0x7928, plain_len: 0x1C8, debug_len: 0x1D2 };

/// `Parallax_Init` .. `section:parallax`, gate `SIGIL_EMP_PARALLAX`.
pub const PARALLAX: Region = Region { plain_base: 0x63BC, debug_base: 0x7AFA, plain_len: 0xAD8, debug_len: 0xBB8 };

/// `Raster_Install` .. `section:raster`, gate `SIGIL_EMP_RASTER`.
pub const RASTER: Region = Region { plain_base: 0x6E98, debug_base: 0x86B6, plain_len: 0x3EC, debug_len: 0x3EC };

/// `Palette_LoadPal` .. `Effects_InstallPreset`, gate `SIGIL_EMP_PALETTE`.
pub const PALETTE: Region = Region { plain_base: 0x7284, debug_base: 0x8AA2, plain_len: 0x4AE, debug_len: 0x4AE };

/// `Effects_InstallPreset` .. `section:preset`.
pub const PRESET: Region = Region { plain_base: 0x7732, debug_base: 0x8F50, plain_len: 0xA8, debug_len: 0xAA };

/// `Level_LoadArt` .. `section:load_art`, gate `SIGIL_EMP_LOAD_ART`.
pub const LOAD_ART: Region = Region { plain_base: 0x77EC, debug_base: 0x900A, plain_len: 0xB6, debug_len: 0xB6 };

/// `PageIn_Process` .. `section:page_in`.
pub const PAGE_IN: Region = Region { plain_base: 0x78A6, debug_base: 0x90C4, plain_len: 0x2DE, debug_len: 0x452 };

/// `PageCache_Init` .. `section:page_cache`.
pub const PAGE_CACHE: Region = Region { plain_base: 0x7B94, debug_base: 0x9524, plain_len: 0x4E6, debug_len: 0xE78 };

/// `BG_Init` .. `section:bg`, gate `SIGIL_EMP_BG`.
pub const BG: Region = Region { plain_base: 0x807E, debug_base: 0xA3A0, plain_len: 0xD4, debug_len: 0x134 };

/// `BgAnim_Init` .. `section:bg_anim`, gate `SIGIL_EMP_BG_ANIM`.
pub const BG_ANIM: Region = Region { plain_base: 0x8152, debug_base: 0xA4D4, plain_len: 0xF4, debug_len: 0x1C8 };

/// `CompressionSelfTest` .. `section:compression_selftest` (debug-only region; plain empty at `Sound_PostByte`), gate `SIGIL_EMP_COMPRESSION_SELFTEST`.
pub const COMPRESSION_SELFTEST: Region = Region { plain_base: 0x8246, debug_base: 0xA69C, plain_len: 0x0, debug_len: 0xDE0 };

/// `Sound_PostByte` .. `section:sound_api`, gate `SIGIL_EMP_SOUND_API`.
pub const SOUND_API: Region = Region { plain_base: 0x8246, debug_base: 0xB47E, plain_len: 0x2B0, debug_len: 0x45A };

/// `TestSolid_Init` .. `section:test_solid`, gate `SIGIL_EMP_TEST_OBJECTS`.
pub const TEST_SOLID: Region = Region { plain_base: 0x123C6, debug_base: 0x128CC, plain_len: 0x122, debug_len: 0x17C };

/// `TestParticle` .. `section:test_particle` (debug-only region; plain empty at `ObjDef_PathSwap`), gate `SIGIL_EMP_TEST_OBJECTS`.
pub const TEST_PARTICLE: Region = Region { plain_base: 0x124E8, debug_base: 0x12A48, plain_len: 0x0, debug_len: 0x58 };

/// `TestStatic_Main` .. `section:test_static`, gate `SIGIL_EMP_TEST_STATIC`.
pub const TEST_STATIC: Region = Region { plain_base: 0x123C2, debug_base: 0x1258C, plain_len: 0x4, debug_len: 0x4 };

/// `TestAnimated` .. `section:test_animated` (debug-only region; plain empty at `TestSolid_Init`), gate `SIGIL_EMP_TEST_ANIMATED`.
pub const TEST_ANIMATED: Region = Region { plain_base: 0x123C6, debug_base: 0x12590, plain_len: 0x0, debug_len: 0x60 };

/// `TestEmitter` .. `section:test_emitter` (debug-only region; plain empty at `ObjDef_PathSwap`), gate `SIGIL_EMP_TEST_EMITTER`.
pub const TEST_EMITTER: Region = Region { plain_base: 0x124E8, debug_base: 0x12AA0, plain_len: 0x0, debug_len: 0x5E };

/// `TestStressEmitter` .. `section:test_stress_emitter` (debug-only region; plain empty at `ObjDef_PathSwap`), gate `SIGIL_EMP_TEST_STRESS_EMITTER`.
pub const TEST_STRESS_EMITTER: Region = Region { plain_base: 0x124E8, debug_base: 0x12C34, plain_len: 0x0, debug_len: 0x5E };

/// `TestChurnObj` .. `section:test_churn` (debug-only region; plain empty at `ObjDef_PathSwap`), gate `SIGIL_EMP_TEST_CHURN`.
pub const TEST_CHURN: Region = Region { plain_base: 0x124E8, debug_base: 0x12C92, plain_len: 0x0, debug_len: 0x7C };

/// `TestChildPart` .. `section:test_parent` (debug-only region; plain empty at `ObjDef_PathSwap`), gate `SIGIL_EMP_TEST_PARENT`.
pub const TEST_PARENT: Region = Region { plain_base: 0x124E8, debug_base: 0x12AFE, plain_len: 0x0, debug_len: 0x136 };

/// `TestPlayer` .. `section:test_player` (debug-only region; plain empty at `TestSolid_Init`), gate `SIGIL_EMP_TEST_PLAYER`.
pub const TEST_PLAYER: Region = Region { plain_base: 0x123C6, debug_base: 0x125F0, plain_len: 0x0, debug_len: 0x294 };

/// `TestEnemy_Init` .. `section:test_enemy` (debug-only region; plain empty at `TestSolid_Init`), gate `SIGIL_EMP_TEST_ENEMY`.
pub const TEST_ENEMY: Region = Region { plain_base: 0x123C6, debug_base: 0x12884, plain_len: 0x0, debug_len: 0x48 };

/// `ObjDef_PathSwap` .. `section:path_swap`, gate `SIGIL_EMP_PATH_SWAP`.
pub const PATH_SWAP: Region = Region { plain_base: 0x124E8, debug_base: 0x12D0E, plain_len: 0x92, debug_len: 0xFA };

/// `OJZ_TestRaster` .. `section:ojz_effects`.
pub const OJZ_EFFECTS: Region = Region { plain_base: 0x13DE4, debug_base: 0x14684, plain_len: 0x68C, debug_len: 0x784 };

/// `DeformTable_Zero` .. `section:scene_registry`, gate `SIGIL_EMP_SCENE_REGISTRY`.
pub const SCENE_REGISTRY: Region = Region { plain_base: 0x1257A, debug_base: 0x12E08, plain_len: 0x1486, debug_len: 0x1486 };

/// `Map_TestObj` .. `section:test_mappings`, gate `SIGIL_EMP_TEST_MAPPINGS`.
pub const TEST_MAPPINGS: Region = Region { plain_base: 0x2A26A, debug_base: 0x2AC8C, plain_len: 0x30, debug_len: 0x30 };

/// `Map_DustSpindash` .. `section:dust_data`, gate `SIGIL_EMP_DUST_DATA`.
pub const DUST_DATA: Region = Region { plain_base: 0x2A29A, debug_base: 0x2ACBC, plain_len: 0xBDA, debug_len: 0xBDA };

/// `Ani_Sonic` .. `section:sonic_anims`, gate `SIGIL_EMP_SONIC_ANIMS`.
pub const SONIC_ANIMS: Region = Region { plain_base: 0x2AE74, debug_base: 0x2B896, plain_len: 0x10A, debug_len: 0x10A };

/// `Ani_Tails` .. `section:tails_anims`, gate `SIGIL_EMP_TAILS_ANIMS`.
pub const TAILS_ANIMS: Region = Region { plain_base: 0x2AF7E, debug_base: 0x2B9A0, plain_len: 0x1BC, debug_len: 0x1BC };

/// `Ani_Knuckles` .. `section:knuckles_anims`, gate `SIGIL_EMP_KNUCKLES_ANIMS`.
pub const KNUCKLES_ANIMS: Region = Region { plain_base: 0x2B13A, debug_base: 0x2BB5C, plain_len: 0x16B, debug_len: 0x16B };

/// `Map_Tails` .. `section:tails_data`, gate `SIGIL_EMP_TAILS_DATA`.
pub const TAILS_DATA: Region = Region { plain_base: 0x2B2BA, debug_base: 0x2BCE4, plain_len: 0x20F5E, debug_len: 0x20F5E };

/// `Map_Knuckles` .. `section:knuckles_data`, gate `SIGIL_EMP_KNUCKLES_DATA`.
pub const KNUCKLES_DATA: Region = Region { plain_base: 0x4C218, debug_base: 0x4CC42, plain_len: 0x226C8, debug_len: 0x226C8 };

/// `Ani_Particle` .. `section:particle_anims` (debug-only region; plain empty at `Ani_DustSpindash`), gate `SIGIL_EMP_PARTICLE_ANIMS`.
pub const PARTICLE_ANIMS: Region = Region { plain_base: 0x2B2A6, debug_base: 0x2BCC8, plain_len: 0x0, debug_len: 0x7 };

/// `Ani_DustSpindash` .. `section:dust_anims`, gate `SIGIL_EMP_DUST_ANIMS`.
pub const DUST_ANIMS: Region = Region { plain_base: 0x2B2A6, debug_base: 0x2BCD0, plain_len: 0x14, debug_len: 0x14 };

/// `OJZ_Sec0_TypeTable` .. `section:entity_data`.
pub const ENTITY_DATA: Region = Region { plain_base: 0x144A4, debug_base: 0x14E3C, plain_len: 0x188, debug_len: 0x188 };

/// `OJZ_Act_Pool_Page0` .. `section:ojz_act_pool`.
pub const OJZ_ACT_POOL: Region = Region { plain_base: 0x1462C, debug_base: 0x14FC4, plain_len: 0x2F0C, debug_len: 0x2F0C };

/// `OJZ_Act1_Descriptor` .. `section:act_descriptor`, gate `SIGIL_EMP_ACT_DESCRIPTOR`.
pub const ACT_DESCRIPTOR: Region = Region { plain_base: 0x17538, debug_base: 0x17ED0, plain_len: 0x15A, debug_len: 0x15A };

/// `OJZ_Sec0_Blocks` .. `section:sec_block_blobs`.
pub const SEC_BLOCK_BLOBS: Region = Region { plain_base: 0x17692, debug_base: 0x1802A, plain_len: 0xB72E, debug_len: 0xB72E };

/// `OJZ_Sec0_LocalMap` .. `section:sec_local_maps`.
pub const SEC_LOCAL_MAPS: Region = Region { plain_base: 0x22DC0, debug_base: 0x23758, plain_len: 0xBFA, debug_len: 0xBFA };

/// `OJZ_Palette` .. `section:ojz_act_assets`.
pub const OJZ_ACT_ASSETS: Region = Region { plain_base: 0x239BA, debug_base: 0x24352, plain_len: 0x4882, debug_len: 0x4882 };

/// `BgAnim_Table` .. `section:ojz_bg_anim`.
pub const OJZ_BG_ANIM: Region = Region { plain_base: 0x2823C, debug_base: 0x28BD4, plain_len: 0x202E, debug_len: 0x20B8 };

/// `ObjDef_Static` .. start + 0x34 plain / 0x34 debug (literal, no end symbol), gate `SIGIL_EMP_OBJDEFS`.
pub const OBJDEFS: Region = Region { plain_base: 0x14470, debug_base: 0x14E08, plain_len: 0x34, debug_len: 0x34 };

/// `GameState_ObjectTest_Init` .. `section:object_test_state` (debug-only region; plain empty at `GameState_OJZScroll_Init`), gate `SIGIL_EMP_OBJECT_TEST_STATE`.
pub const OBJECT_TEST_STATE: Region = Region { plain_base: 0xBC404, debug_base: 0xBDE54, plain_len: 0x0, debug_len: 0x384 };

/// `GameState_OJZScroll_Init` .. `section:ojz_scroll_test`, gate `SIGIL_EMP_OJZ_SCROLL_TEST`.
pub const OJZ_SCROLL_TEST: Region = Region { plain_base: 0xBC404, debug_base: 0xBE1D8, plain_len: 0x57A, debug_len: 0x200C };

/// `Replay_OJZ_Fixture` .. `section:replay_fixture`.
pub const REPLAY_FIXTURE: Region = Region { plain_base: 0xBC982, debug_base: 0xC01E4, plain_len: 0x260, debug_len: 0x260 };

/// `BusError` .. `section:error_handler`, gate `SIGIL_EMP_ERROR_HANDLER`.
pub const ERROR_HANDLER: Region = Region { plain_base: 0xBCBE2, debug_base: 0xC0444, plain_len: 0x10B0, debug_len: 0x10B0 };

/// `Dac_Temp_Blip` .. `section:dac_banks`, gate `SIGIL_EMP_DAC`.
pub const DAC_BANKS: Region = Region { plain_base: 0xA8000, debug_base: 0xA8000, plain_len: 0xE49A, debug_len: 0xE49A };

/// `Song_MovingTrucks` .. start + 0x34E8 plain / 0x4F38 debug (literal, no end symbol), gate `SIGIL_EMP_MT`.
pub const MT_BANK_BLOB: Region = Region { plain_base: 0xB8630, debug_base: 0xB8630, plain_len: 0x34E8, debug_len: 0x4F38 };

/// `Sfx_33` .. `section:sfx_bank_blob`, gate `SIGIL_EMP_SFX`.
pub const SFX_BANK_BLOB: Region = Region { plain_base: 0xBBB18, debug_base: 0xBD568, plain_len: 0x8EC, debug_len: 0x8EC };

/// `SoundTablesZ80_Head` .. start + 0x630 plain / 0x630 debug (literal, no end symbol), gate `SIGIL_EMP_SOUNDBANKHEAD`.
pub const SOUNDBANKHEAD: Region = Region { plain_base: 0xB8000, debug_base: 0xB8000, plain_len: 0x630, debug_len: 0x630 };

/// `EndOfRom` .. start + 0x0 plain / 0x0 debug (literal, no end symbol), gate `SIGIL_EMP_EPILOGUE`.
pub const EPILOGUE: Region = Region { plain_base: 0xBDC92, debug_base: 0xC14F4, plain_len: 0x0, debug_len: 0x0 };

/// `ObjCodeBase` .. start + 0x2 plain / 0x2 debug (literal, no end symbol), gate `SIGIL_EMP_OBJCODEBASE`.
pub const OBJCODEBASE: Region = Region { plain_base: 0x10000, debug_base: 0x10000, plain_len: 0x2, debug_len: 0x2 };

/// `Player_Init` .. `section:player_common`, gate `SIGIL_EMP_PLAYER_COMMON`.
pub const PLAYER_COMMON: Region = Region { plain_base: 0x10002, debug_base: 0x10002, plain_len: 0x760, debug_len: 0x876 };

/// `CharDef_Sonic` .. `section:sonic`, gate `SIGIL_EMP_SONIC`.
pub const SONIC: Region = Region { plain_base: 0x11F54, debug_base: 0x12060, plain_len: 0x36, debug_len: 0x36 };

/// `CharDef_Tails` .. `section:tails`, gate `SIGIL_EMP_TAILS`.
pub const TAILS: Region = Region { plain_base: 0x11F8A, debug_base: 0x12096, plain_len: 0x36, debug_len: 0x36 };

/// `CharDef_Knuckles` .. `section:knuckles`, gate `SIGIL_EMP_KNUCKLES`.
pub const KNUCKLES: Region = Region { plain_base: 0x11FC0, debug_base: 0x120CC, plain_len: 0x36, debug_len: 0x36 };

/// `CharacterDefs` .. `section:characters`, gate `SIGIL_EMP_CHARACTERS`.
pub const CHARACTERS: Region = Region { plain_base: 0x11FF6, debug_base: 0x12102, plain_len: 0x4A, debug_len: 0xB0 };

/// `TailsAppendage_Refresh` .. `section:tails_appendage`, gate `SIGIL_EMP_TAILS_APPENDAGE`.
pub const TAILS_APPENDAGE: Region = Region { plain_base: 0x12040, debug_base: 0x121B2, plain_len: 0x11C, debug_len: 0x174 };

/// `DustPuff_Spawn` .. `section:dust_puff`, gate `SIGIL_EMP_DUST_PUFF`.
pub const DUST_PUFF: Region = Region { plain_base: 0x1215C, debug_base: 0x12326, plain_len: 0x46, debug_len: 0x46 };

/// `Dust_Tick` .. `section:dust_spindash`, gate `SIGIL_EMP_DUST_SPINDASH`.
pub const DUST_SPINDASH: Region = Region { plain_base: 0x121A2, debug_base: 0x1236C, plain_len: 0x102, debug_len: 0x102 };

/// `PState_Ground` .. `section:player_ground`, gate `SIGIL_EMP_PLAYER_GROUND`.
pub const PLAYER_GROUND: Region = Region { plain_base: 0x10766, debug_base: 0x1087C, plain_len: 0x490, debug_len: 0x48A };

/// `PState_Air` .. `section:player_air`, gate `SIGIL_EMP_PLAYER_AIR`.
pub const PLAYER_AIR: Region = Region { plain_base: 0x10BF6, debug_base: 0x10D06, plain_len: 0x34A, debug_len: 0x34A };

/// `PState_Spindash` .. `section:player_spindash`, gate `SIGIL_EMP_PLAYER_SPINDASH`.
pub const PLAYER_SPINDASH: Region = Region { plain_base: 0x10F40, debug_base: 0x11050, plain_len: 0xA0, debug_len: 0x9C };

/// `PState_Fly` .. `section:player_fly`, gate `SIGIL_EMP_PLAYER_FLY`.
pub const PLAYER_FLY: Region = Region { plain_base: 0x10FE0, debug_base: 0x110EC, plain_len: 0x13A, debug_len: 0x138 };

/// `PState_Glide` .. `section:player_glide`, gate `SIGIL_EMP_PLAYER_GLIDE`.
pub const PLAYER_GLIDE: Region = Region { plain_base: 0x11124, debug_base: 0x11230, plain_len: 0x2BA, debug_len: 0x2B6 };

/// `Climb_WallDist` .. `CharDef_Sonic`, gate `SIGIL_EMP_PLAYER_CLIMB`.
pub const PLAYER_CLIMB: Region = Region { plain_base: 0x113FA, debug_base: 0x11506, plain_len: 0xB5A, debug_len: 0xB5A };

/// `Collision_ProbeDown` .. `section:player_sensors`, gate `SIGIL_EMP_PLAYER_SENSORS`.
pub const PLAYER_SENSORS: Region = Region { plain_base: 0x58A0, debug_base: 0x6BA4, plain_len: 0x4F4, debug_len: 0x4F4 };

// ── Symbols (manifest order) ──

/// `OJZ_Preset_Sec0`.
pub const OJZ_PRESET_SEC0: Pin = Pin { plain: 0x1418C, debug: 0x14AC8 };

/// `OJZ_Preset_Sec1`.
pub const OJZ_PRESET_SEC1: Pin = Pin { plain: 0x141BA, debug: 0x14AF6 };

/// `OJZ_Preset_Sec2`.
pub const OJZ_PRESET_SEC2: Pin = Pin { plain: 0x141E8, debug: 0x14B24 };

/// `OJZ_Preset_Sec3`.
pub const OJZ_PRESET_SEC3: Pin = Pin { plain: 0x14216, debug: 0x14B52 };

/// `OJZ_Preset_Plain`.
pub const OJZ_PRESET_PLAIN: Pin = Pin { plain: 0x14244, debug: 0x14B80 };

/// `OJZ_Preset_Depth`.
pub const OJZ_PRESET_DEPTH: Pin = Pin { plain: 0x14272, debug: 0x14BAE };

/// `EditorSceneBinding_OJZ_Act1_Sec4`.
pub const EDITOR_SCENE_BINDING_OJZ_ACT1_SEC4: Pin = Pin { plain: 0x13ABE, debug: 0x1434C };

/// `OJZ_Preset_Sec5`.
pub const OJZ_PRESET_SEC5: Pin = Pin { plain: 0x142A0, debug: 0x14BDC };

/// `EditorRaster_OJZ_Act1_authored_probe`.
pub const EDITOR_RASTER_OJZ_ACT1_AUTHORED_PROBE: Pin = Pin { plain: 0x13C9A, debug: 0x1453A };

/// `EditorRaster_OJZ_Act1_ojz_sec5_showcase`.
pub const EDITOR_RASTER_OJZ_ACT1_OJZ_SEC5_SHOWCASE: Pin = Pin { plain: 0x13D16, debug: 0x145B6 };

/// `EditorRaster_OJZ_Act1_ojz_sec3_shimmer`.
pub const EDITOR_RASTER_OJZ_ACT1_OJZ_SEC3_SHIMMER: Pin = Pin { plain: 0x13CE8, debug: 0x14588 };

/// `EditorCycle_OJZ_Act1_ojz_sec3_shimmer`.
pub const EDITOR_CYCLE_OJZ_ACT1_OJZ_SEC3_SHIMMER: Pin = Pin { plain: 0x13DD4, debug: 0x14674 };

/// `CrossoverTable`.
pub const CROSSOVER_TABLE: Pin = Pin { plain: 0x70AE0, debug: 0x7150A };

/// `Effects_InstallPreset`.
pub const EFFECTS_INSTALL_PRESET: Pin = Pin { plain: 0x7732, debug: 0x8F50 };

/// `Raster_GetChannelBand`.
pub const RASTER_GET_CHANNEL_BAND: Pin = Pin { plain: 0x71A0, debug: 0x89BE };

/// `TestStatic_Main`.
pub const TEST_STATIC_MAIN: Pin = Pin { plain: 0x123C2, debug: 0x1258C };

/// `TestSolid_Init`.
pub const TEST_SOLID_INIT: Pin = Pin { plain: 0x123C6, debug: 0x128CC };

/// `TestEnemy_Init`, debug-shape consumer only (`debug_only`).
pub const TEST_ENEMY_INIT: u32 = 0x12884;

/// `TestParent`, debug-shape consumer only (`debug_only`).
pub const TEST_PARENT_LABEL: u32 = 0x12B88;

/// `Map_TestObj`.
pub const MAP_TEST_OBJ: Pin = Pin { plain: 0x2A26A, debug: 0x2AC8C };

/// `Map_Sonic`.
pub const MAP_SONIC: Pin = Pin { plain: 0x70BE0, debug: 0x7160A };

/// `DPLC_Sonic`.
pub const DPLC_SONIC: Pin = Pin { plain: 0x72860, debug: 0x7328A };

/// `Art_Sonic`.
pub const ART_SONIC: Pin = Pin { plain: 0x73124, debug: 0x73B4E };

/// `CreateEffect_Normal`.
pub const CREATE_EFFECT_NORMAL: Pin = Pin { plain: 0x4572, debug: 0x5472 };

/// `CreateChild_Normal`.
pub const CREATE_CHILD_NORMAL: Pin = Pin { plain: 0x4348, debug: 0x5198 };

/// `DeleteChildren`.
pub const DELETE_CHILDREN: Pin = Pin { plain: 0x4554, debug: 0x5454 };

/// `GetSineCosine`.
pub const GET_SINE_COSINE: Pin = Pin { plain: 0x2834, debug: 0x2B18 };

/// `EntryPoint`.
pub const ENTRY_POINT: Pin = Pin { plain: 0x200, debug: 0x200 };

/// `BusError`, debug-shape consumer only (`debug_only`).
pub const BUS_ERROR: u32 = 0xC0444;

/// `AddressError`, debug-shape consumer only (`debug_only`).
pub const ADDRESS_ERROR: u32 = 0xC045C;

/// `IllegalInstr`, debug-shape consumer only (`debug_only`).
pub const ILLEGAL_INSTR: u32 = 0xC0478;

/// `ZeroDivide`, debug-shape consumer only (`debug_only`).
pub const ZERO_DIVIDE: u32 = 0xC049A;

/// `ChkInstr`, debug-shape consumer only (`debug_only`).
pub const CHK_INSTR: u32 = 0xC04B4;

/// `TrapvInstr`, debug-shape consumer only (`debug_only`).
pub const TRAPV_INSTR: u32 = 0xC04D2;

/// `PrivilegeViol`, debug-shape consumer only (`debug_only`).
pub const PRIVILEGE_VIOL: u32 = 0xC04F2;

/// `Trace`, debug-shape consumer only (`debug_only`).
pub const TRACE: u32 = 0xC0514;

/// `Line1010Emu`, debug-shape consumer only (`debug_only`).
pub const LINE1010_EMU: u32 = 0xC0528;

/// `Line1111Emu`, debug-shape consumer only (`debug_only`).
pub const LINE1111_EMU: u32 = 0xC0548;

/// `ErrorExcept`, debug-shape consumer only (`debug_only`).
pub const ERROR_EXCEPT: u32 = 0xC0568;

/// `ErrorTrap`, debug-shape consumer only (`debug_only`).
pub const ERROR_TRAP: u32 = 0xC0586;

/// `VBlank_Handler`.
pub const V_BLANK_HANDLER: Pin = Pin { plain: 0x223A, debug: 0x2334 };

/// `HBlank_Vector_Slot`.
pub const H_BLANK_VECTOR_SLOT: Pin = Pin { plain: 0xFFFFB612, debug: 0xFFFFB6A0 };

/// `VDP_Shadow_Table`.
pub const VDP_SHADOW_TABLE: Pin = Pin { plain: 0xFFFF800E, debug: 0xFFFF800E };

/// `BootData_VDPRegs`.
pub const BOOT_DATA_VDP_REGS: Pin = Pin { plain: 0x3B2, debug: 0x3BA };

/// `Ctrl_1_Held`.
pub const CTRL_1_HELD: Pin = Pin { plain: 0xFFFF8028, debug: 0xFFFF8028 };

/// `Ctrl_1_Held_Raw`.
pub const CTRL_1_HELD_RAW: Pin = Pin { plain: 0xFFFFBA00, debug: 0xFFFFBA8E };

/// `Ctrl_2_Held`.
pub const CTRL_2_HELD: Pin = Pin { plain: 0xFFFF802A, debug: 0xFFFF802A };

/// `Ctrl_1_Ext_Held`.
pub const CTRL_1_EXT_HELD: Pin = Pin { plain: 0xFFFF802E, debug: 0xFFFF802E };

/// `Ctrl_2_Ext_Held`.
pub const CTRL_2_EXT_HELD: Pin = Pin { plain: 0xFFFF8030, debug: 0xFFFF8030 };

/// `Ctrl_2_Held_Raw`.
pub const CTRL_2_HELD_RAW: Pin = Pin { plain: 0xFFFFBA01, debug: 0xFFFFBA8F };

/// `Ctrl_1_Ext_Held_Raw`.
pub const CTRL_1_EXT_HELD_RAW: Pin = Pin { plain: 0xFFFFBA02, debug: 0xFFFFBA90 };

/// `Ctrl_2_Ext_Held_Raw`.
pub const CTRL_2_EXT_HELD_RAW: Pin = Pin { plain: 0xFFFFBA03, debug: 0xFFFFBA91 };

/// `VSync_Wait`.
pub const V_SYNC_WAIT: Pin = Pin { plain: 0x23F0, debug: 0x2516 };

/// `Sound_DrainSfxRing`.
pub const SOUND_DRAIN_SFX_RING: Pin = Pin { plain: 0x83BA, debug: 0xB79C };

/// `Game_State`.
pub const GAME_STATE: Pin = Pin { plain: 0xFFFF8008, debug: 0xFFFF8008 };

/// `Input_Tick`.
pub const INPUT_TICK: Pin = Pin { plain: 0x257C, debug: 0x26A8 };

/// `Cache_Left_Col`.
pub const CACHE_LEFT_COL: Pin = Pin { plain: 0xFFFFADC0, debug: 0xFFFFAE4E };

/// `Draw_TileColumn`.
pub const DRAW_TILE_COLUMN: Pin = Pin { plain: 0x4698, debug: 0x5598 };

/// `Draw_TileRow_FromCache`.
pub const DRAW_TILE_ROW_FROM_CACHE: Pin = Pin { plain: 0x47EC, debug: 0x576A };

/// `EntityWindow_Init`.
pub const ENTITY_WINDOW_INIT: Pin = Pin { plain: 0x3DEC, debug: 0x4B4C };

/// `Section_Plane_Dirty`.
pub const SECTION_PLANE_DIRTY: Pin = Pin { plain: 0xFFFFAE34, debug: 0xFFFFAEC2 };

/// `Section_Right_Col_Written`.
pub const SECTION_RIGHT_COL_WRITTEN: Pin = Pin { plain: 0xFFFFAE36, debug: 0xFFFFAEC4 };

/// `Section_Left_Col_Written`.
pub const SECTION_LEFT_COL_WRITTEN: Pin = Pin { plain: 0xFFFFAE38, debug: 0xFFFFAEC6 };

/// `Section_Top_Row_Written`.
pub const SECTION_TOP_ROW_WRITTEN: Pin = Pin { plain: 0xFFFFAE30, debug: 0xFFFFAEBE };

/// `Section_Bottom_Row_Written`.
pub const SECTION_BOTTOM_ROW_WRITTEN: Pin = Pin { plain: 0xFFFFAE32, debug: 0xFFFFAEC0 };

/// `Cache_Head_Col`.
pub const CACHE_HEAD_COL: Pin = Pin { plain: 0xFFFFADC2, debug: 0xFFFFAE50 };

/// `Cache_Top_Row`.
pub const CACHE_TOP_ROW: Pin = Pin { plain: 0xFFFFADC4, debug: 0xFFFFAE52 };

/// `Cache_Bottom_Row`.
pub const CACHE_BOTTOM_ROW: Pin = Pin { plain: 0xFFFFADC6, debug: 0xFFFFAE54 };

/// `Cache_Origin_Col`.
pub const CACHE_ORIGIN_COL: Pin = Pin { plain: 0xFFFFADC8, debug: 0xFFFFAE56 };

/// `Cache_Origin_Row`.
pub const CACHE_ORIGIN_ROW: Pin = Pin { plain: 0xFFFFADCA, debug: 0xFFFFAE58 };

/// `Plane_Buffer_Ptr`.
pub const PLANE_BUFFER_PTR: Pin = Pin { plain: 0xFFFFACAC, debug: 0xFFFFAD3A };

/// `Plane_Buffer`.
pub const PLANE_BUFFER_BASE: Pin = Pin { plain: 0xFFFFA6AC, debug: 0xFFFFA73A };

/// `Tile_Cache_Nametable`.
pub const TILE_CACHE_NAMETABLE: Pin = Pin { plain: 0xFFFF0000, debug: 0xFFFF0000 };

/// `Tile_Cache_Collision`.
pub const TILE_CACHE_COLLISION: Pin = Pin { plain: 0xFFFF2580, debug: 0xFFFF2580 };

/// `Frame_Counter`.
pub const FRAME_COUNTER: Pin = Pin { plain: 0xFFFF8002, debug: 0xFFFF8002 };

/// `Logic_Tick`.
pub const LOGIC_TICK: Pin = Pin { plain: 0xFFFF8004, debug: 0xFFFF8004 };

/// `Block_Stage_Keys`.
pub const BLOCK_STAGE_KEYS: Pin = Pin { plain: 0xFFFFADEE, debug: 0xFFFFAE7C };

/// `Block_Stage_Next`.
pub const BLOCK_STAGE_NEXT: Pin = Pin { plain: 0xFFFFAE2E, debug: 0xFFFFAEBC };

/// `Block_Stage_Bucket`.
pub const BLOCK_STAGE_BUCKET: Pin = Pin { plain: 0xFFFF6842, debug: 0xFFFF6842 };

/// `Block_Stage_Chain`.
pub const BLOCK_STAGE_CHAIN: Pin = Pin { plain: 0xFFFF6942, debug: 0xFFFF6942 };

/// `Block_Stage_Buffers`.
pub const BLOCK_STAGE_BUFFERS: Pin = Pin { plain: 0xFFFF3842, debug: 0xFFFF3842 };

/// `Block_Stage_Ptrs`.
pub const BLOCK_STAGE_PTRS: Pin = Pin { plain: 0xFFFFB618, debug: 0xFFFFB6A6 };

/// `Block_Stage_ZeroPage`.
pub const BLOCK_STAGE_ZERO_PAGE: Pin = Pin { plain: 0xFFFFB69C, debug: 0xFFFFB72A };

/// `Cache_Fill_Last_Frame`.
pub const CACHE_FILL_LAST_FRAME: Pin = Pin { plain: 0xFFFFADCC, debug: 0xFFFFAE5A };

/// `Cache_Fill_Budget`.
pub const CACHE_FILL_BUDGET: Pin = Pin { plain: 0xFFFFADD6, debug: 0xFFFFAE64 };

/// `Cache_Fill_Resume_Col`.
pub const CACHE_FILL_RESUME_COL: Pin = Pin { plain: 0xFFFFADCE, debug: 0xFFFFAE5C };

/// `Cache_Fill_Resume_Row`.
pub const CACHE_FILL_RESUME_ROW: Pin = Pin { plain: 0xFFFFADD0, debug: 0xFFFFAE5E };

/// `Cache_Fill_RowResume_Row`.
pub const CACHE_FILL_ROW_RESUME_ROW: Pin = Pin { plain: 0xFFFFADD8, debug: 0xFFFFAE66 };

/// `Cache_Fill_RowResume_Col`.
pub const CACHE_FILL_ROW_RESUME_COL: Pin = Pin { plain: 0xFFFFADDA, debug: 0xFFFFAE68 };

/// `Cache_Fill_Rows_Left`.
pub const CACHE_FILL_ROWS_LEFT: Pin = Pin { plain: 0xFFFFADDC, debug: 0xFFFFAE6A };

/// `Cache_Prev_Cam_Row`.
pub const CACHE_PREV_CAM_ROW: Pin = Pin { plain: 0xFFFFADDE, debug: 0xFFFFAE6C };

/// `Cache_Prev_Cam_X`.
pub const CACHE_PREV_CAM_X: Pin = Pin { plain: 0xFFFFADE0, debug: 0xFFFFAE6E };

/// `Cache_H_Pfx_Dir`.
pub const CACHE_H_PFX_DIR: Pin = Pin { plain: 0xFFFFADE2, debug: 0xFFFFAE70 };

/// `Cache_H_Pfx_Accum`.
pub const CACHE_H_PFX_ACCUM: Pin = Pin { plain: 0xFFFFADE4, debug: 0xFFFFAE72 };

/// `Cache_Pfx_Row_Target`.
pub const CACHE_PFX_ROW_TARGET: Pin = Pin { plain: 0xFFFFADE6, debug: 0xFFFFAE74 };

/// `Cache_Pfx_Col_Target`.
pub const CACHE_PFX_COL_TARGET: Pin = Pin { plain: 0xFFFFADE8, debug: 0xFFFFAE76 };

/// `Cache_Pfx_Skip_Armed`.
pub const CACHE_PFX_SKIP_ARMED: Pin = Pin { plain: 0xFFFFADEA, debug: 0xFFFFAE78 };

/// `Cache_Pfx_Lag_Flag`.
pub const CACHE_PFX_LAG_FLAG: Pin = Pin { plain: 0xFFFFADEC, debug: 0xFFFFAE7A };

/// `Block_Stage_Gen`.
pub const BLOCK_STAGE_GEN: Pin = Pin { plain: 0xFFFFB600, debug: 0xFFFFB68E };

/// `Pfx_Memo_Row`.
pub const PFX_MEMO_ROW: Pin = Pin { plain: 0xFFFFB602, debug: 0xFFFFB690 };

/// `Pfx_Memo_L16`.
pub const PFX_MEMO_L16: Pin = Pin { plain: 0xFFFFB604, debug: 0xFFFFB692 };

/// `Pfx_Memo_H16`.
pub const PFX_MEMO_H16: Pin = Pin { plain: 0xFFFFB606, debug: 0xFFFFB694 };

/// `Pfx_Memo_Gen`.
pub const PFX_MEMO_GEN: Pin = Pin { plain: 0xFFFFB608, debug: 0xFFFFB696 };

/// `Cs_Memo_Col`.
pub const CS_MEMO_COL: Pin = Pin { plain: 0xFFFFB60A, debug: 0xFFFFB698 };

/// `Cs_Memo_T16`.
pub const CS_MEMO_T16: Pin = Pin { plain: 0xFFFFB60C, debug: 0xFFFFB69A };

/// `Cs_Memo_B16`.
pub const CS_MEMO_B16: Pin = Pin { plain: 0xFFFFB60E, debug: 0xFFFFB69C };

/// `Cs_Memo_Gen`.
pub const CS_MEMO_GEN: Pin = Pin { plain: 0xFFFFB610, debug: 0xFFFFB69E };

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
pub const S4_LZ_DECOMPRESS_DICT: Pin = Pin { plain: 0x26C4, debug: 0x28A0 };

/// `Player_1`.
pub const PLAYER_1: Pin = Pin { plain: 0xFFFF8F70, debug: 0xFFFF8FFE };

/// `Cheat_Flags`.
pub const CHEAT_FLAGS: Pin = Pin { plain: 0xFFFFBB8C, debug: 0xFFFFEC4A };

/// `Dynamic_Slots`.
pub const DYNAMIC_SLOTS: Pin = Pin { plain: 0xFFFF9010, debug: 0xFFFF909E };

/// `Ring_Buffer`.
pub const RING_BUFFER: Pin = Pin { plain: 0xFFFFAEA2, debug: 0xFFFFAF30 };

/// `Ring_Count`.
pub const RING_COUNT: Pin = Pin { plain: 0xFFFFB1A2, debug: 0xFFFFB230 };

/// `Ring_HighWater`.
pub const RING_HIGH_WATER: Pin = Pin { plain: 0xFFFFB1A3, debug: 0xFFFFB231 };

/// `Ring_Add_Dropped`.
pub const RING_ADD_DROPPED: Pin = Pin { plain: 0xFFFFB1A4, debug: 0xFFFFB232 };

/// `Ring_Counter`.
pub const RING_COUNTER: Pin = Pin { plain: 0xFFFFB20E, debug: 0xFFFFB29C };

/// `Ring_Anim_Frame`.
pub const RING_ANIM_FRAME: Pin = Pin { plain: 0xFFFFB210, debug: 0xFFFFB29E };

/// `Ring_Anim_Timer`.
pub const RING_ANIM_TIMER: Pin = Pin { plain: 0xFFFFB211, debug: 0xFFFFB29F };

/// `Camera_X`.
pub const CAMERA_X: Pin = Pin { plain: 0xFFFFA69E, debug: 0xFFFFA72C };

/// `Camera_Y`.
pub const CAMERA_Y: Pin = Pin { plain: 0xFFFFA6A2, debug: 0xFFFFA730 };

/// `Camera_Target`.
pub const CAMERA_TARGET: Pin = Pin { plain: 0xFFFFADBA, debug: 0xFFFFAE48 };

/// `Camera_Curl_Offset`.
pub const CAMERA_CURL_OFFSET: Pin = Pin { plain: 0xFFFFADBC, debug: 0xFFFFAE4A };

/// `Camera_Deadzone_Base`.
pub const CAMERA_DEADZONE_BASE: Pin = Pin { plain: 0xFFFFADB0, debug: 0xFFFFAE3E };

/// `Camera_Pan_Offset`.
pub const CAMERA_PAN_OFFSET: Pin = Pin { plain: 0xFFFFADB4, debug: 0xFFFFAE42 };

/// `Camera_Hold_Frames`.
pub const CAMERA_HOLD_FRAMES: Pin = Pin { plain: 0xFFFFADBE, debug: 0xFFFFAE4C };

/// `Camera_Art_Hold`.
pub const CAMERA_ART_HOLD: Pin = Pin { plain: 0xFFFFADBF, debug: 0xFFFFAE4D };

/// `Dbg_Cam_Clamp_Frames`, debug-shape consumer only (`debug_only`).
pub const DBG_CAM_CLAMP_FRAMES: u32 = 0xFFFF8FA4;

/// `Camera_X_Max`.
pub const CAMERA_X_MAX: Pin = Pin { plain: 0xFFFFADB6, debug: 0xFFFFAE44 };

/// `Camera_Y_Max`.
pub const CAMERA_Y_MAX: Pin = Pin { plain: 0xFFFFADB8, debug: 0xFFFFAE46 };

/// `BgAnim_LastStep`.
pub const BG_ANIM_LAST_STEP: Pin = Pin { plain: 0xFFFF8F06, debug: 0xFFFF8F06 };

/// `BgAnim_Table`.
pub const BG_ANIM_TABLE: Pin = Pin { plain: 0x2823C, debug: 0x28BD4 };

/// `Camera_X_Biased`.
pub const CAMERA_X_BIASED: Pin = Pin { plain: 0xFFFFA6A6, debug: 0xFFFFA734 };

/// `Camera_Y_Biased`.
pub const CAMERA_Y_BIASED: Pin = Pin { plain: 0xFFFFA6A8, debug: 0xFFFFA736 };

/// `Collected_MarkRing`.
pub const COLLECTED_MARK_RING: Pin = Pin { plain: 0x3AB0, debug: 0x44F4 };

/// `EntityWindow_EntryForSection`.
pub const ENTITY_WINDOW_ENTRY_FOR_SECTION: Pin = Pin { plain: 0x3CCC, debug: 0x49D6 };

/// `EntityLoaded_Clear`.
pub const ENTITY_LOADED_CLEAR: Pin = Pin { plain: 0x3CB8, debug: 0x4960 };

/// `Sound_PlayRing`.
pub const SOUND_PLAY_RING: Pin = Pin { plain: 0x840A, debug: 0xB7EC };

/// `MDDBG__ErrorHandler`, debug-shape consumer only (`debug_only`).
pub const MDDBG_ERROR_HANDLER: u32 = 0xC059E;

/// `MDDBG__ErrorHandler_PagesController`, debug-shape consumer only (`debug_only`).
pub const MDDBG_ERROR_HANDLER_PAGES_CONTROLLER: u32 = 0xC1364;

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
pub const ACT_ART_BUDGET: Pin = Pin { plain: 0xFFFFB9FC, debug: 0xFFFFBA8A };

/// `Art_Budget_Remaining`.
pub const ART_BUDGET_REMAINING: Pin = Pin { plain: 0xFFFFB9FE, debug: 0xFFFFBA8C };

/// `PageIn_Pool_Pages`.
pub const PAGE_IN_POOL_PAGES: Pin = Pin { plain: 0xFFFFB9F0, debug: 0xFFFFBA7E };

/// `PageIn_Bulk_Drain`.
pub const PAGE_IN_BULK_DRAIN: Pin = Pin { plain: 0xFFFFB9EB, debug: 0xFFFFBA79 };

/// `PageIn_Fully_Resident`.
pub const PAGE_IN_FULLY_RESIDENT: Pin = Pin { plain: 0xFFFFB9F2, debug: 0xFFFFBA80 };

/// `Block_Stage_Maps`.
pub const BLOCK_STAGE_MAPS: Pin = Pin { plain: 0xFFFFB658, debug: 0xFFFFB6E6 };

/// `Cache_Cur_LocalMap`.
pub const CACHE_CUR_LOCAL_MAP: Pin = Pin { plain: 0xFFFFB698, debug: 0xFFFFB726 };

/// `PageCache_Direct_Map`.
pub const PAGE_CACHE_DIRECT_MAP: Pin = Pin { plain: 0xFFFFB9F3, debug: 0xFFFFBA81 };

/// `Page_Table`.
pub const PAGE_TABLE: Pin = Pin { plain: 0xFFFF699C, debug: 0xFFFF699C };

/// `Dbg_DMA_Enq_Capped`, debug-shape consumer only (`debug_only`).
pub const DBG_DMA_ENQ_CAPPED: u32 = 0xFFFF8F7A;

/// `DMA_Overflow_Count`, debug-shape consumer only (`debug_only`).
pub const DMA_OVERFLOW_COUNT: u32 = 0xFFFF8F78;

/// `Art_Staging_Buffer`.
pub const ART_STAGING_BUFFER: Pin = Pin { plain: 0xFFFF6B34, debug: 0xFFFF6B34 };

/// `S4LZ_Decompress`.
pub const S4_LZ_DECOMPRESS: Pin = Pin { plain: 0x26C8, debug: 0x28F8 };

/// `QueueDMA_Critical`.
pub const QUEUE_DMA_CRITICAL: Pin = Pin { plain: 0x1D54, debug: 0x1E34 };

/// `BG_Init`.
pub const BG_INIT: Pin = Pin { plain: 0x807E, debug: 0xA3A0 };

/// `QueueDMA_Important`.
pub const QUEUE_DMA_IMPORTANT: Pin = Pin { plain: 0x1D5E, debug: 0x1E3E };

/// `QueueDMA_Deferrable`.
pub const QUEUE_DMA_DEFERRABLE: Pin = Pin { plain: 0x1D68, debug: 0x1E48 };

/// `Object_RAM`.
pub const OBJECT_RAM: Pin = Pin { plain: 0xFFFF8F70, debug: 0xFFFF8FFE };

/// `System_Slots`.
pub const SYSTEM_SLOTS: Pin = Pin { plain: 0xFFFF9C90, debug: 0xFFFF9D1E };

/// `Effect_Slots`.
pub const EFFECT_SLOTS: Pin = Pin { plain: 0xFFFF9F10, debug: 0xFFFF9F9E };

/// `Game_Paused`.
pub const GAME_PAUSED: Pin = Pin { plain: 0xFFFFA6AA, debug: 0xFFFFA738 };

/// `Object_RAM_End`.
pub const OBJECT_RAM_END: Pin = Pin { plain: 0xFFFFA410, debug: 0xFFFFA49E };

/// `Dynamic_Free_Stack`.
pub const DYNAMIC_FREE_STACK: Pin = Pin { plain: 0xFFFFA410, debug: 0xFFFFA49E };

/// `Dynamic_Free_SP`.
pub const DYNAMIC_FREE_SP: Pin = Pin { plain: 0xFFFFA460, debug: 0xFFFFA4EE };

/// `Effect_Free_Stack`.
pub const EFFECT_FREE_STACK: Pin = Pin { plain: 0xFFFFA462, debug: 0xFFFFA4F0 };

/// `Effect_Free_SP`.
pub const EFFECT_FREE_SP: Pin = Pin { plain: 0xFFFFA482, debug: 0xFFFFA510 };

/// `Dynamic_Live`.
pub const DYNAMIC_LIVE: Pin = Pin { plain: 0xFFFFB59A, debug: 0xFFFFB628 };

/// `Dynamic_Live_Count`.
pub const DYNAMIC_LIVE_COUNT: Pin = Pin { plain: 0xFFFFB5EA, debug: 0xFFFFB678 };

/// `Dynamic_Live_Dirty`.
pub const DYNAMIC_LIVE_DIRTY: Pin = Pin { plain: 0xFFFFB5EC, debug: 0xFFFFB67A };

/// `Dynamic_Live_Walking`, debug-shape consumer only (`debug_only`).
pub const DYNAMIC_LIVE_WALKING: u32 = 0xFFFFB67B;

/// `Dynamic_Live_Pending`.
pub const DYNAMIC_LIVE_PENDING: Pin = Pin { plain: 0xFFFFB5EE, debug: 0xFFFFB67C };

/// `Dynamic_Live_Pending_Count`.
pub const DYNAMIC_LIVE_PENDING_COUNT: Pin = Pin { plain: 0xFFFFB5FE, debug: 0xFFFFB68C };

/// `DeleteObject`.
pub const DELETE_OBJECT: Pin = Pin { plain: 0x2D9E, debug: 0x3082 };

/// `DrawRings`.
pub const DRAW_RINGS: Pin = Pin { plain: 0x38F6, debug: 0x42CE };

/// `Sprite_Table_Buffer`.
pub const SPRITE_TABLE_BUFFER: Pin = Pin { plain: 0xFFFF8298, debug: 0xFFFF8298 };

/// `Sprite_Table_Dirty`.
pub const SPRITE_TABLE_DIRTY: Pin = Pin { plain: 0xFFFF8518, debug: 0xFFFF8518 };

/// `Sprite_Emit_Active`.
pub const SPRITE_EMIT_ACTIVE: Pin = Pin { plain: 0xFFFF8519, debug: 0xFFFF8519 };

/// `Sprite_Bands`.
pub const SPRITE_BANDS: Pin = Pin { plain: 0xFFFFA484, debug: 0xFFFFA512 };

/// `Sprite_Band_Counts`.
pub const SPRITE_BAND_COUNTS: Pin = Pin { plain: 0xFFFFA684, debug: 0xFFFFA712 };

/// `Sprites_Rendered`.
pub const SPRITES_RENDERED: Pin = Pin { plain: 0xFFFFA68C, debug: 0xFFFFA71A };

/// `Sprite_Cycle_Counter`.
pub const SPRITE_CYCLE_COUNTER: Pin = Pin { plain: 0xFFFFA68E, debug: 0xFFFFA71C };

/// `Sprite_Owner`, debug-shape consumer only (`debug_only`).
pub const SPRITE_OWNER: u32 = 0xFFFFE316;

/// `SpriteMask_Y`.
pub const SPRITE_MASK_Y: Pin = Pin { plain: 0xFFFFA690, debug: 0xFFFFA71E };

/// `SpriteMask_Height`.
pub const SPRITE_MASK_HEIGHT: Pin = Pin { plain: 0xFFFFA692, debug: 0xFFFFA720 };

/// `SpriteMask_After_Band`.
pub const SPRITE_MASK_AFTER_BAND: Pin = Pin { plain: 0xFFFFA694, debug: 0xFFFFA722 };

/// `Scanline_Band_Sprites`.
pub const SCANLINE_BAND_SPRITES: Pin = Pin { plain: 0xFFFFA696, debug: 0xFFFFA724 };

/// `Sound_PlaySFX`.
pub const SOUND_PLAY_SFX: Pin = Pin { plain: 0x8374, debug: 0xB710 };

/// `ObjectMoveX`.
pub const OBJECT_MOVE_X: Pin = Pin { plain: 0x2FA0, debug: 0x36D6 };

/// `ObjCodeBase`.
pub const OBJ_CODE_BASE: Pin = Pin { plain: 0x10000, debug: 0x10000 };

/// `Draw_Sprite`.
pub const DRAW_SPRITE: Pin = Pin { plain: 0x2FD0, debug: 0x3706 };

/// `ObjectMove`.
pub const OBJECT_MOVE: Pin = Pin { plain: 0x2F86, debug: 0x36BC };

/// `Ring_Sfx_Speaker`.
pub const RING_SFX_SPEAKER: Pin = Pin { plain: 0xFFFFB4DE, debug: 0xFFFFB56C };

/// `Sfx_Ring_Buf`.
pub const SFX_RING_BUF: Pin = Pin { plain: 0xFFFFB4E0, debug: 0xFFFFB56E };

/// `Sfx_Ring_Wr`.
pub const SFX_RING_WR: Pin = Pin { plain: 0xFFFFB4E8, debug: 0xFFFFB576 };

/// `Sfx_Ring_Rd`.
pub const SFX_RING_RD: Pin = Pin { plain: 0xFFFFB4E9, debug: 0xFFFFB577 };

/// `SongTable`.
pub const SONG_TABLE: Pin = Pin { plain: 0xBBB10, debug: 0xBD550 };

/// `SongPatchTable`.
pub const SONG_PATCH_TABLE: Pin = Pin { plain: 0xBBB14, debug: 0xBD55C };

/// `OJZ_Palette`.
pub const OJZ_PALETTE: Pin = Pin { plain: 0x239BA, debug: 0x24352 };

/// `OJZ_Act1_BG_Layout`.
pub const OJZ_ACT1_BG_LAYOUT: Pin = Pin { plain: 0x23A3A, debug: 0x243D2 };

/// `OJZ_Act1_BG_Tiles`.
pub const OJZ_ACT1_BG_TILES: Pin = Pin { plain: 0x25A3A, debug: 0x263D2 };

/// `ParallaxConfig_OJZ_Default`.
pub const PARALLAX_CONFIG_OJZ_DEFAULT: Pin = Pin { plain: 0x1267A, debug: 0x12F08 };

/// `OJZ_Act_Pool_PageTable`.
pub const OJZ_ACT_POOL_PAGE_TABLE: Pin = Pin { plain: 0x174E8, debug: 0x17E80 };

/// `OJZ_Sec_LocalMaps`.
pub const OJZ_SEC_LOCAL_MAPS: Pin = Pin { plain: 0x23996, debug: 0x2432E };

/// `OJZ_Sec0_Blocks`.
pub const OJZ_SEC0_BLOCKS: Pin = Pin { plain: 0x17692, debug: 0x1802A };

/// `OJZ_Sec1_Blocks`.
pub const OJZ_SEC1_BLOCKS: Pin = Pin { plain: 0x19926, debug: 0x1A2BE };

/// `OJZ_Sec2_Blocks`.
pub const OJZ_SEC2_BLOCKS: Pin = Pin { plain: 0x1ACA2, debug: 0x1B63A };

/// `OJZ_Sec3_Blocks`.
pub const OJZ_SEC3_BLOCKS: Pin = Pin { plain: 0x1C43A, debug: 0x1CDD2 };

/// `OJZ_Sec4_Blocks`.
pub const OJZ_SEC4_BLOCKS: Pin = Pin { plain: 0x1ACA2, debug: 0x1B63A };

/// `OJZ_Sec5_Blocks`.
pub const OJZ_SEC5_BLOCKS: Pin = Pin { plain: 0x1D586, debug: 0x1DF1E };

/// `OJZ_Sec6_Blocks`.
pub const OJZ_SEC6_BLOCKS: Pin = Pin { plain: 0x1E3AC, debug: 0x1ED44 };

/// `OJZ_Sec7_Blocks`.
pub const OJZ_SEC7_BLOCKS: Pin = Pin { plain: 0x1FFAC, debug: 0x20944 };

/// `OJZ_Sec8_Blocks`.
pub const OJZ_SEC8_BLOCKS: Pin = Pin { plain: 0x21220, debug: 0x21BB8 };

/// `OJZ_Sec0_Objects`.
pub const OJZ_SEC0_OBJECTS: Pin = Pin { plain: 0x144AE, debug: 0x14E46 };

/// `OJZ_Sec0_Rings`.
pub const OJZ_SEC0_RINGS: Pin = Pin { plain: 0x144DA, debug: 0x14E72 };

/// `OJZ_Sec0_TypeTable`.
pub const OJZ_SEC0_TYPE_TABLE: Pin = Pin { plain: 0x144A4, debug: 0x14E3C };

/// `OJZ_Sec1_Objects`.
pub const OJZ_SEC1_OBJECTS: Pin = Pin { plain: 0x14500, debug: 0x14E98 };

/// `OJZ_Sec1_Rings`.
pub const OJZ_SEC1_RINGS: Pin = Pin { plain: 0x14508, debug: 0x14EA0 };

/// `OJZ_Sec1_TypeTable`.
pub const OJZ_SEC1_TYPE_TABLE: Pin = Pin { plain: 0x144FA, debug: 0x14E92 };

/// `OJZ_Sec2_Objects`.
pub const OJZ_SEC2_OBJECTS: Pin = Pin { plain: 0x1453A, debug: 0x14ED2 };

/// `OJZ_Sec2_Rings`.
pub const OJZ_SEC2_RINGS: Pin = Pin { plain: 0x14548, debug: 0x14EE0 };

/// `OJZ_Sec2_TypeTable`.
pub const OJZ_SEC2_TYPE_TABLE: Pin = Pin { plain: 0x14530, debug: 0x14EC8 };

/// `OJZ_Sec3_Objects`.
pub const OJZ_SEC3_OBJECTS: Pin = Pin { plain: 0x1457E, debug: 0x14F16 };

/// `OJZ_Sec3_Rings`.
pub const OJZ_SEC3_RINGS: Pin = Pin { plain: 0x14580, debug: 0x14F18 };

/// `OJZ_Sec3_TypeTable`.
pub const OJZ_SEC3_TYPE_TABLE: Pin = Pin { plain: 0x1457C, debug: 0x14F14 };

/// `OJZ_Sec4_Objects`.
pub const OJZ_SEC4_OBJECTS: Pin = Pin { plain: 0x14586, debug: 0x14F1E };

/// `OJZ_Sec4_Rings`.
pub const OJZ_SEC4_RINGS: Pin = Pin { plain: 0x14588, debug: 0x14F20 };

/// `OJZ_Sec4_TypeTable`.
pub const OJZ_SEC4_TYPE_TABLE: Pin = Pin { plain: 0x14584, debug: 0x14F1C };

/// `OJZ_Sec5_Objects`.
pub const OJZ_SEC5_OBJECTS: Pin = Pin { plain: 0x145BE, debug: 0x14F56 };

/// `OJZ_Sec5_Rings`.
pub const OJZ_SEC5_RINGS: Pin = Pin { plain: 0x145C0, debug: 0x14F58 };

/// `OJZ_Sec5_TypeTable`.
pub const OJZ_SEC5_TYPE_TABLE: Pin = Pin { plain: 0x145BC, debug: 0x14F54 };

/// `OJZ_Sec6_Objects`.
pub const OJZ_SEC6_OBJECTS: Pin = Pin { plain: 0x145E6, debug: 0x14F7E };

/// `OJZ_Sec6_Rings`.
pub const OJZ_SEC6_RINGS: Pin = Pin { plain: 0x145E8, debug: 0x14F80 };

/// `OJZ_Sec6_TypeTable`.
pub const OJZ_SEC6_TYPE_TABLE: Pin = Pin { plain: 0x145E4, debug: 0x14F7C };

/// `OJZ_Sec7_Objects`.
pub const OJZ_SEC7_OBJECTS: Pin = Pin { plain: 0x145EE, debug: 0x14F86 };

/// `OJZ_Sec7_Rings`.
pub const OJZ_SEC7_RINGS: Pin = Pin { plain: 0x145F0, debug: 0x14F88 };

/// `OJZ_Sec7_TypeTable`.
pub const OJZ_SEC7_TYPE_TABLE: Pin = Pin { plain: 0x145EC, debug: 0x14F84 };

/// `OJZ_Sec8_Objects`.
pub const OJZ_SEC8_OBJECTS: Pin = Pin { plain: 0x14616, debug: 0x14FAE };

/// `OJZ_Sec8_Rings`.
pub const OJZ_SEC8_RINGS: Pin = Pin { plain: 0x14618, debug: 0x14FB0 };

/// `OJZ_Sec8_TypeTable`.
pub const OJZ_SEC8_TYPE_TABLE: Pin = Pin { plain: 0x14614, debug: 0x14FAC };

/// `BLOCK_INDEX_SIZE`.
pub const BLOCK_INDEX_SIZE: Pin = Pin { plain: 0x400, debug: 0x400 };

/// `EDGE_CLAMP`.
pub const EDGE_CLAMP: Pin = Pin { plain: 0x0, debug: 0x0 };

/// `MAX_ACT_SECTIONS`.
pub const MAX_ACT_SECTIONS: Pin = Pin { plain: 0x30, debug: 0x30 };

/// `SECTION_SIZE_SHIFT`.
pub const SECTION_SIZE_SHIFT: Pin = Pin { plain: 0xB, debug: 0xB };

/// `Act_len`.
pub const ACT_LEN: Pin = Pin { plain: 0x28, debug: 0x28 };

/// `Sec_len`.
pub const SEC_LEN: Pin = Pin { plain: 0x22, debug: 0x22 };

/// `Camera_Y_Coarse_Prev`.
pub const CAMERA_Y_COARSE_PREV: Pin = Pin { plain: 0xFFFFB31E, debug: 0xFFFFB3AC };

/// `Current_Act_Ptr`.
pub const CURRENT_ACT_PTR: Pin = Pin { plain: 0xFFFFB4DA, debug: 0xFFFFB568 };

/// `Entity_Window_Active`.
pub const ENTITY_WINDOW_ACTIVE: Pin = Pin { plain: 0xFFFFB212, debug: 0xFFFFB2A0 };

/// `Entity_Window_Anchor`.
pub const ENTITY_WINDOW_ANCHOR: Pin = Pin { plain: 0xFFFFB214, debug: 0xFFFFB2A2 };

/// `Entity_Window_OriginX`.
pub const ENTITY_WINDOW_ORIGIN_X: Pin = Pin { plain: 0xFFFFB216, debug: 0xFFFFB2A4 };

/// `Entity_Window_OriginY`.
pub const ENTITY_WINDOW_ORIGIN_Y: Pin = Pin { plain: 0xFFFFB218, debug: 0xFFFFB2A6 };

/// `Entity_Window_Center_ID`.
pub const ENTITY_WINDOW_CENTER_ID: Pin = Pin { plain: 0xFFFFB213, debug: 0xFFFFB2A1 };

/// `Entity_Scan_State`.
pub const ENTITY_SCAN_STATE: Pin = Pin { plain: 0xFFFFB1A6, debug: 0xFFFFB234 };

/// `Entity_Loaded_Masks`.
pub const ENTITY_LOADED_MASKS: Pin = Pin { plain: 0xFFFFB21A, debug: 0xFFFFB2A8 };

/// `Entity_Mask_Scratch`.
pub const ENTITY_MASK_SCRATCH: Pin = Pin { plain: 0xFFFFB29A, debug: 0xFFFFB328 };

/// `Ring_Collected_Window`.
pub const RING_COLLECTED_WINDOW: Pin = Pin { plain: 0xFFFFB320, debug: 0xFFFFB3AE };

/// `Ring_Collected_Park`.
pub const RING_COLLECTED_PARK: Pin = Pin { plain: 0xFFFFB454, debug: 0xFFFFB4E2 };

/// `Collected_Park_Next`.
pub const COLLECTED_PARK_NEXT: Pin = Pin { plain: 0xFFFFB4D8, debug: 0xFFFFB566 };

/// `RingBuffer_Clear`.
pub const RING_BUFFER_CLEAR: Pin = Pin { plain: 0x38E8, debug: 0x42C0 };

/// `RingBuffer_Remove`.
pub const RING_BUFFER_REMOVE: Pin = Pin { plain: 0x38B4, debug: 0x428C };

/// `Section_GetSecPtrXY`.
pub const SECTION_GET_SEC_PTR_XY: Pin = Pin { plain: 0x5DE4, debug: 0x70E8 };

/// `Section_FlatIDXY`.
pub const SECTION_FLAT_IDXY: Pin = Pin { plain: 0x5DCA, debug: 0x70CE };

/// `AllocDynamic`.
pub const ALLOC_DYNAMIC: Pin = Pin { plain: 0x2D20, debug: 0x3004 };

/// `AllocEffect`.
pub const ALLOC_EFFECT: Pin = Pin { plain: 0x2D84, debug: 0x3068 };

/// `Palette_Buffer`.
pub const PALETTE_BUFFER: Pin = Pin { plain: 0xFFFF8216, debug: 0xFFFF8216 };

/// `Hscroll_Buffer`.
pub const HSCROLL_BUFFER: Pin = Pin { plain: 0xFFFF851A, debug: 0xFFFF851A };

/// `Static_Pal_Line0`.
pub const STATIC_PAL_LINE0: Pin = Pin { plain: 0xFFFF8F0E, debug: 0xFFFF8F0E };

/// `Static_Pal_Line1`.
pub const STATIC_PAL_LINE1: Pin = Pin { plain: 0xFFFF8F1C, debug: 0xFFFF8F1C };

/// `Static_Pal_Line2`.
pub const STATIC_PAL_LINE2: Pin = Pin { plain: 0xFFFF8F2A, debug: 0xFFFF8F2A };

/// `Static_Pal_Line3`.
pub const STATIC_PAL_LINE3: Pin = Pin { plain: 0xFFFF8F46, debug: 0xFFFF8F46 };

/// `Static_Sprite_DMA`.
pub const STATIC_SPRITE_DMA: Pin = Pin { plain: 0xFFFF8F54, debug: 0xFFFF8F54 };

/// `Static_Hscroll_Line`.
pub const STATIC_HSCROLL_LINE: Pin = Pin { plain: 0xFFFF8F62, debug: 0xFFFF8F62 };

/// `Palette_Dirty`.
pub const PALETTE_DIRTY: Pin = Pin { plain: 0xFFFF8296, debug: 0xFFFF8296 };

/// `Parallax_Active_Config`.
pub const PARALLAX_ACTIVE_CONFIG: Pin = Pin { plain: 0x6490, debug: 0x7C62 };

/// `Palette_Ship_Snap`.
pub const PALETTE_SHIP_SNAP: Pin = Pin { plain: 0xFFFFBA04, debug: 0xFFFFBA92 };

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
pub const LAG_FRAME_COUNT: u32 = 0xFFFF8F7C;

/// `DMA_Bytes_ThisFrame`, debug-shape consumer only (`debug_only`).
pub const DMA_BYTES_THIS_FRAME: u32 = 0xFFFF8F70;

/// `PageIn_InFlight`.
pub const PAGE_IN_IN_FLIGHT: Pin = Pin { plain: 0xFFFFB9BE, debug: 0xFFFFBA4C };

/// `PageIn_Saved_PC`.
pub const PAGE_IN_SAVED_PC: Pin = Pin { plain: 0xFFFFB9B8, debug: 0xFFFFBA46 };

/// `PageIn_BankRegs`.
pub const PAGE_IN_BANK_REGS: Pin = Pin { plain: 0x7A7A, debug: 0x9400 };

/// `Dbg_PageIn_Preempts`, debug-shape consumer only (`debug_only`).
pub const DBG_PAGE_IN_PREEMPTS: u32 = 0xFFFF8F96;

/// `ZX0R_Decompress.__end`.
pub const ZX0R_DECOMPRESS_END: Pin = Pin { plain: 0x2834, debug: 0x2B18 };

/// `PageIn_Staging_Busy`.
pub const PAGE_IN_STAGING_BUSY: Pin = Pin { plain: 0xFFFFB9C0, debug: 0xFFFFBA4E };

/// `PageIn_Flush`.
pub const PAGE_IN_FLUSH: Pin = Pin { plain: 0x7B42, debug: 0x94D0 };

/// `PageIn_Enqueue`.
pub const PAGE_IN_ENQUEUE: Pin = Pin { plain: 0x7B04, debug: 0x9492 };

/// `PageIn_Pool_Table`.
pub const PAGE_IN_POOL_TABLE: Pin = Pin { plain: 0xFFFFB9EC, debug: 0xFFFFBA7A };

/// `PageIn_Queue_Count`.
pub const PAGE_IN_QUEUE_COUNT: Pin = Pin { plain: 0xFFFFB9C2, debug: 0xFFFFBA50 };

/// `PageIn_Suspended`.
pub const PAGE_IN_SUSPENDED: Pin = Pin { plain: 0xFFFFB9BF, debug: 0xFFFFBA4D };

/// `PageIn_Land_Pending`.
pub const PAGE_IN_LAND_PENDING: Pin = Pin { plain: 0xFFFFB9C1, debug: 0xFFFFBA4F };

/// `PageCache_Init`.
pub const PAGE_CACHE_INIT: Pin = Pin { plain: 0x7B94, debug: 0x9524 };

/// `PageCache_AllocFrame`.
pub const PAGE_CACHE_ALLOC_FRAME: Pin = Pin { plain: 0x7C46, debug: 0x9638 };

/// `PageCache_Publish`.
pub const PAGE_CACHE_PUBLISH: Pin = Pin { plain: 0x7D02, debug: 0x97F8 };

/// `PageCache_PatchRun_Seq`.
pub const PAGE_CACHE_PATCH_RUN_SEQ: Pin = Pin { plain: 0x7D70, debug: 0x98CC };

/// `PageCache_PatchRun_Col`.
pub const PAGE_CACHE_PATCH_RUN_COL: Pin = Pin { plain: 0x7E74, debug: 0x9B0C };

/// `PageCache_Audit`.
pub const PAGE_CACHE_AUDIT: Pin = Pin { plain: 0x8078, debug: 0x9E8C };

/// `Cache_Art_Stall`.
pub const CACHE_ART_STALL: Pin = Pin { plain: 0xFFFFADD2, debug: 0xFFFFAE60 };

/// `Page_Audit_Ticks`, debug-shape consumer only (`debug_only`).
pub const PAGE_AUDIT_TICKS: u32 = 0xFFFF8FAA;

/// `Cache_Stall_Watchdog`, debug-shape consumer only (`debug_only`).
pub const CACHE_STALL_WATCHDOG: u32 = 0xFFFF8FA8;

/// `Flush_VDP_Shadow`.
pub const FLUSH_VDP_SHADOW: Pin = Pin { plain: 0x1C0E, debug: 0x1C98 };

/// `VInt_DrawLevel`.
pub const V_INT_DRAW_LEVEL: Pin = Pin { plain: 0x493E, debug: 0x597C };

/// `Vscroll_Write`.
pub const VSCROLL_WRITE: Pin = Pin { plain: 0x64A2, debug: 0x7C74 };

/// `Read_Controllers`.
pub const READ_CONTROLLERS: Pin = Pin { plain: 0x244C, debug: 0x2576 };

/// `Process_DMA_Critical`.
pub const PROCESS_DMA_CRITICAL: Pin = Pin { plain: 0x1E2E, debug: 0x1F28 };

/// `Process_DMA_Important`.
pub const PROCESS_DMA_IMPORTANT: Pin = Pin { plain: 0x1EFC, debug: 0x1FF6 };

/// `Process_DMA_Deferrable`.
pub const PROCESS_DMA_DEFERRABLE: Pin = Pin { plain: 0x1F10, debug: 0x200A };

/// `Enqueue_Dirty_Buffers`.
pub const ENQUEUE_DIRTY_BUFFERS: Pin = Pin { plain: 0x2048, debug: 0x2142 };

/// `BootData`.
pub const BOOT_DATA: Pin = Pin { plain: 0x398, debug: 0x3A0 };

/// `VInt_Level`.
pub const V_INT_LEVEL: Pin = Pin { plain: 0x2282, debug: 0x2380 };

/// `BuildStaticDMA`.
pub const BUILD_STATIC_DMA: Pin = Pin { plain: 0x1F84, debug: 0x207E };

/// `Sound_Init`.
pub const SOUND_INIT: Pin = Pin { plain: 0x826C, debug: 0xB4A4 };

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
pub const P_STATE_GROUND: Pin = Pin { plain: 0x10766, debug: 0x1087C };

/// `PState_Roll`.
pub const P_STATE_ROLL: Pin = Pin { plain: 0x108CE, debug: 0x109E0 };

/// `PState_Spindash`.
pub const P_STATE_SPINDASH: Pin = Pin { plain: 0x10F40, debug: 0x11050 };

/// `PState_Air`.
pub const P_STATE_AIR: Pin = Pin { plain: 0x10BF6, debug: 0x10D06 };

/// `PState_Jump`.
pub const P_STATE_JUMP: Pin = Pin { plain: 0x10BFE, debug: 0x10D0E };

/// `PState_RollJump`.
pub const P_STATE_ROLL_JUMP: Pin = Pin { plain: 0x10BFA, debug: 0x10D0A };

/// `PState_AirBall`.
pub const P_STATE_AIR_BALL: Pin = Pin { plain: 0x10BF6, debug: 0x10D06 };

/// `PState_Fly`.
pub const P_STATE_FLY: Pin = Pin { plain: 0x10FE0, debug: 0x110EC };

/// `PState_Glide`.
pub const P_STATE_GLIDE: Pin = Pin { plain: 0x11124, debug: 0x11230 };

/// `PState_GlideFall`.
pub const P_STATE_GLIDE_FALL: Pin = Pin { plain: 0x112BC, debug: 0x113C8 };

/// `PState_Slide`.
pub const P_STATE_SLIDE: Pin = Pin { plain: 0x1130A, debug: 0x11414 };

/// `PState_Climb`.
pub const P_STATE_CLIMB: Pin = Pin { plain: 0x11454, debug: 0x11560 };

/// `PState_Ledge`.
pub const P_STATE_LEDGE: Pin = Pin { plain: 0x11600, debug: 0x1170C };

/// `Player_SensorFloor`.
pub const PLAYER_SENSOR_FLOOR: Pin = Pin { plain: 0x5C0C, debug: 0x6F10 };

/// `Player_AtLedgeEdge`.
pub const PLAYER_AT_LEDGE_EDGE: Pin = Pin { plain: 0x5D26, debug: 0x702A };

/// `Player_SetState`.
pub const PLAYER_SET_STATE: Pin = Pin { plain: 0x104BE, debug: 0x10580 };

/// `Player_SnapToSurface`.
pub const PLAYER_SNAP_TO_SURFACE: Pin = Pin { plain: 0x1060C, debug: 0x106CE };

/// `Player_SensorCeiling`.
pub const PLAYER_SENSOR_CEILING: Pin = Pin { plain: 0x5C22, debug: 0x6F26 };

/// `Player_SensorWallDir`.
pub const PLAYER_SENSOR_WALL_DIR: Pin = Pin { plain: 0x5CDC, debug: 0x6FE0 };

/// `Player_SensorWallAt`.
pub const PLAYER_SENSOR_WALL_AT: Pin = Pin { plain: 0x5CD4, debug: 0x6FD8 };

/// `Collision_GetType`.
pub const COLLISION_GET_TYPE: Pin = Pin { plain: 0x5838, debug: 0x6B3C };

/// `SolidityTable`.
pub const SOLIDITY_TABLE: Pin = Pin { plain: 0x709E0, debug: 0x7140A };

/// `AngleTable`.
pub const ANGLE_TABLE: Pin = Pin { plain: 0x708E0, debug: 0x7130A };

/// `HeightMaps`.
pub const HEIGHT_MAPS: Pin = Pin { plain: 0x6E8E0, debug: 0x6F30A };

/// `HeightMapsRot`.
pub const HEIGHT_MAPS_ROT: Pin = Pin { plain: 0x6F8E0, debug: 0x7030A };

/// `Character_ID`.
pub const CHARACTER_ID: Pin = Pin { plain: 0xFFFFBB8E, debug: 0xFFFFEC4C };

/// `Player_Chardef`.
pub const PLAYER_CHARDEF: Pin = Pin { plain: 0xFFFFBB90, debug: 0xFFFFEC4E };

/// `Ability_None`.
pub const ABILITY_NONE: Pin = Pin { plain: 0x1203E, debug: 0x121B0 };

/// `CharacterDefs`.
pub const CHARACTER_DEFS: Pin = Pin { plain: 0x11FF6, debug: 0x12102 };

/// `Player_InitAssets`.
pub const PLAYER_INIT_ASSETS: Pin = Pin { plain: 0x12002, debug: 0x1210E };

/// `Player_LoadArt`.
pub const PLAYER_LOAD_ART: Pin = Pin { plain: 0x1201A, debug: 0x12126 };

/// `Player_Ability`.
pub const PLAYER_ABILITY: Pin = Pin { plain: 0x12034, debug: 0x12140 };

/// `PhysTable_Sonic`.
pub const PHYS_TABLE_SONIC: Pin = Pin { plain: 0x11F7A, debug: 0x12086 };

/// `Pal_SonicTails`.
pub const PAL_SONIC_TAILS: Pin = Pin { plain: 0x6E8A0, debug: 0x6F2CA };

/// `OJZ_TestRaster`.
pub const OJZ_TEST_RASTER: Pin = Pin { plain: 0x13DE4, debug: 0x14684 };

/// `OJZ_TestPal`.
pub const OJZ_TEST_PAL: Pin = Pin { plain: 0x13E06, debug: 0x14742 };

/// `OJZ_TestGradient`.
pub const OJZ_TEST_GRADIENT: Pin = Pin { plain: 0x140D0, debug: 0x14A0C };

/// `OJZ_ShimmerCycle`.
pub const OJZ_SHIMMER_CYCLE: Pin = Pin { plain: 0x13E66, debug: 0x147A2 };

/// `OJZ_TestVsram`.
pub const OJZ_TEST_VSRAM: Pin = Pin { plain: 0x140EE, debug: 0x14A2A };

/// `OJZ_TestRamp`.
pub const OJZ_TEST_RAMP: Pin = Pin { plain: 0x1410C, debug: 0x14A48 };

/// `Raster_Program`.
pub const RASTER_PROGRAM: Pin = Pin { plain: 0xFFFF8BD6, debug: 0xFFFF8BD6 };

/// `Raster_Cursor`.
pub const RASTER_CURSOR: Pin = Pin { plain: 0xFFFF8BDA, debug: 0xFFFF8BDA };

/// `Raster_Pending`.
pub const RASTER_PENDING: Pin = Pin { plain: 0xFFFF8BDE, debug: 0xFFFF8BDE };

/// `Raster_Buf_A`.
pub const RASTER_BUF_A: Pin = Pin { plain: 0xFFFF8BE4, debug: 0xFFFF8BE4 };

/// `Raster_Active_Buf`.
pub const RASTER_ACTIVE_BUF: Pin = Pin { plain: 0xFFFF8CE4, debug: 0xFFFF8CE4 };

/// `Raster_Buf_B`.
pub const RASTER_BUF_B: Pin = Pin { plain: 0xFFFF8C64, debug: 0xFFFF8C64 };

/// `Raster_Line`.
pub const RASTER_LINE: Pin = Pin { plain: 0xFFFF8BE2, debug: 0xFFFF8BE2 };

/// `Raster_Dense_Lines`.
pub const RASTER_DENSE_LINES: Pin = Pin { plain: 0xFFFF8CE8, debug: 0xFFFF8CE8 };

/// `Raster_Dense_Cursor`.
pub const RASTER_DENSE_CURSOR: Pin = Pin { plain: 0xFFFF8CEA, debug: 0xFFFF8CEA };

/// `Raster_Dense_Cmd`.
pub const RASTER_DENSE_CMD: Pin = Pin { plain: 0xFFFF8CEE, debug: 0xFFFF8CEE };

/// `Raster_Dense_Mode`.
pub const RASTER_DENSE_MODE: Pin = Pin { plain: 0xFFFF8CF2, debug: 0xFFFF8CF2 };

/// `Raster_Ramp_Acc`.
pub const RASTER_RAMP_ACC: Pin = Pin { plain: 0xFFFF8CF4, debug: 0xFFFF8CF4 };

/// `Raster_Ramp_Step`.
pub const RASTER_RAMP_STEP: Pin = Pin { plain: 0xFFFF8CF8, debug: 0xFFFF8CF8 };

/// `Effects_World_Y`.
pub const EFFECTS_WORLD_Y: Pin = Pin { plain: 0xFFFF8CFC, debug: 0xFFFF8CFC };

/// `Effects_Screen_L`.
pub const EFFECTS_SCREEN_L: Pin = Pin { plain: 0xFFFF8D04, debug: 0xFFFF8D04 };

/// `Effects_Offscreen_Entry`.
pub const EFFECTS_OFFSCREEN_ENTRY: Pin = Pin { plain: 0xFFFF8D26, debug: 0xFFFF8D26 };

/// `Static_Pal_Ship`.
pub const STATIC_PAL_SHIP: Pin = Pin { plain: 0xFFFF8F38, debug: 0xFFFF8F38 };

/// `Build_DMA_Entry`.
pub const BUILD_DMA_ENTRY: Pin = Pin { plain: 0x2012, debug: 0x210C };

/// `Raster_Patch_Tab`.
pub const RASTER_PATCH_TAB: Pin = Pin { plain: 0xFFFF8D2A, debug: 0xFFFF8D2A };

/// `Raster_State`.
pub const RASTER_STATE: Pin = Pin { plain: 0xFFFF8BD6, debug: 0xFFFF8BD6 };

/// `Raster_State_End`.
pub const RASTER_STATE_END: Pin = Pin { plain: 0xFFFF8D2E, debug: 0xFFFF8D2E };

/// `Pal_Variant_Stage`.
pub const PAL_VARIANT_STAGE: Pin = Pin { plain: 0xFFFF8DEE, debug: 0xFFFF8DEE };

/// `Raster_VBlank`.
pub const RASTER_V_BLANK: Pin = Pin { plain: 0x6E9E, debug: 0x86BC };

/// `Palette_Compose`.
pub const PALETTE_COMPOSE: Pin = Pin { plain: 0x7338, debug: 0x8B56 };

/// `Player_Blocks`.
pub const PLAYER_BLOCKS: Pin = Pin { plain: 0xFFFFBB94, debug: 0xFFFFEC52 };

/// `Player_Ring_Index`.
pub const PLAYER_RING_INDEX: Pin = Pin { plain: 0xFFFFBF00, debug: 0xFFFFF000 };

/// `Player_Pos_Ring`.
pub const PLAYER_POS_RING: Pin = Pin { plain: 0xFFFFBD00, debug: 0xFFFFEE00 };

/// `Player_Stat_Ring`.
pub const PLAYER_STAT_RING: Pin = Pin { plain: 0xFFFFBE00, debug: 0xFFFFEF00 };

/// `Player_Death_Pending`.
pub const PLAYER_DEATH_PENDING: Pin = Pin { plain: 0xFFFFBBCC, debug: 0xFFFFEC8A };

/// `Player_Bound_Right`.
pub const PLAYER_BOUND_RIGHT: Pin = Pin { plain: 0xFFFFBBCE, debug: 0xFFFFEC8C };

/// `Player_Bound_Bottom`.
pub const PLAYER_BOUND_BOTTOM: Pin = Pin { plain: 0xFFFFBBD0, debug: 0xFFFFEC8E };

/// `DustSpindash_Spawn`.
pub const DUST_SPINDASH_SPAWN: Pin = Pin { plain: 0x121FA, debug: 0x123C4 };

// ── Region-relative offsets (manifest order) ──

/// `AnimateSprite.cc_delete` − `animate` start (per-shape).
pub const CC_DELETE_OFF: ShapeOffset = ShapeOffset { plain: 0x104, debug: 0x15E };

/// `RefreshSpritePieceCount` − `animate` start (per-shape).
pub const REFRESH_OFF: ShapeOffset = ShapeOffset { plain: 0x16C, debug: 0x290 };

/// `RingCollision` − `rings` start (per-shape).
pub const RINGCOL_OFF: ShapeOffset = ShapeOffset { plain: 0x116, debug: 0x17C };

/// `Sound_PlaySFX` − `sound_api` start (per-shape).
pub const SOUND_PLAY_SFX_OFF: ShapeOffset = ShapeOffset { plain: 0x12E, debug: 0x292 };

/// `Sine_Table` − `math` start (shape-invariant, asserted at generation).
pub const SINE_TABLE_OFF: usize = 0x18;

/// `Flush_VDP_Shadow` − `vdp_init` start (shape-invariant, asserted at generation).
pub const FLUSH_VDP_SHADOW_OFF: usize = 0x12;

/// `HBlank_Uninstall` − `hblank` start (shape-invariant, asserted at generation).
pub const HBLANK_UNINSTALL_OFF: usize = 0x1C;
