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
//! [provenance] 96 regions, 416 symbols, 7 offsets

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
pub const ASSEMBLED_LEN: usize = 0xC2BA2;
/// Assembled (pre-convsym) ROM length, `__DEBUG__` shape.
pub const DEBUG_ASSEMBLED_LEN: usize = 0xC49B0;

// ── Regions (manifest order) ──

/// `Vectors` .. start + 0x100 plain / 0x100 debug (literal, no end symbol), gate `SIGIL_EMP_VECTORS`.
pub const VECTORS: Region = Region { plain_base: 0x0, debug_base: 0x0, plain_len: 0x100, debug_len: 0x100 };

/// `GameHeader` .. `section:header`.
pub const HEADER: Region = Region { plain_base: 0x100, debug_base: 0x100, plain_len: 0x100, debug_len: 0x100 };

/// `HeightMaps` .. `section:collision_data`.
pub const COLLISION_DATA: Region = Region { plain_base: 0x6C9F8, debug_base: 0x733F6, plain_len: 0x1D3E2, debug_len: 0x1D3E2 };

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
pub const HBLANK: Region = Region { plain_base: 0x2510, debug_base: 0x269E, plain_len: 0x30, debug_len: 0x30 };

/// `Read_Controllers` .. `section:controllers`, gate `SIGIL_EMP_CONTROLLERS`.
pub const CONTROLLERS: Region = Region { plain_base: 0x2540, debug_base: 0x26CE, plain_len: 0x10E, debug_len: 0x10E };

/// `GameLoop` .. `section:game_loop`, gate `SIGIL_EMP_GAME_LOOP`.
pub const GAME_LOOP: Region = Region { plain_base: 0x264E, debug_base: 0x27DC, plain_len: 0x20, debug_len: 0x24 };

/// `Input_Tick` .. `section:replay`.
pub const REPLAY: Region = Region { plain_base: 0x2674, debug_base: 0x2806, plain_len: 0x146, debug_len: 0x1F6 };

/// `S4LZ_DecompressDict` .. `section:s4lz`, gate `SIGIL_EMP_S4LZ`.
pub const S4LZ: Region = Region { plain_base: 0x27BC, debug_base: 0x29FE, plain_len: 0x146, debug_len: 0x19A };

/// `ZX0R_Decompress` .. `section:zx0_resume`.
pub const ZX0_RESUME: Region = Region { plain_base: 0x2902, debug_base: 0x2B98, plain_len: 0x78, debug_len: 0x78 };

/// `GetSineCosine` .. `section:math`, gate `SIGIL_EMP_MATH`.
pub const MATH: Region = Region { plain_base: 0x297A, debug_base: 0x2C10, plain_len: 0x3F6, debug_len: 0x3F6 };

/// `Perform_DPLC` .. `section:dplc`, gate `SIGIL_EMP_DPLC`.
pub const DPLC: Region = Region { plain_base: 0x2D70, debug_base: 0x3006, plain_len: 0xA4, debug_len: 0xA4 };

/// `InitObjectRAM` .. `section:core`, gate `SIGIL_EMP_CORE`.
pub const CORE: Region = Region { plain_base: 0x2E14, debug_base: 0x30AA, plain_len: 0x2F0, debug_len: 0x742 };

/// `InitSpriteSystem` .. `section:sprites`, gate `SIGIL_EMP_SPRITES`.
pub const SPRITES: Region = Region { plain_base: 0x3104, debug_base: 0x37EC, plain_len: 0x410, debug_len: 0x532 };

/// `AnimateSprite` .. `section:animate`, gate `SIGIL_EMP_ANIMATE`.
pub const ANIMATE: Region = Region { plain_base: 0x3514, debug_base: 0x3D1E, plain_len: 0x194, debug_len: 0x2BA };

/// `TouchResponse` .. `section:collision`, gate `SIGIL_EMP_COLLISION`.
pub const COLLISION: Region = Region { plain_base: 0x36A8, debug_base: 0x3FD8, plain_len: 0x32C, debug_len: 0x334 };

/// `RingBuffer_Add` .. `section:rings`, gate `SIGIL_EMP_RINGS`.
pub const RINGS: Region = Region { plain_base: 0x39D4, debug_base: 0x430C, plain_len: 0x1BC, debug_len: 0x226 };

/// `Collected_Init` .. `section:entity_window`, gate `SIGIL_EMP_ENTITY_WINDOW`.
pub const ENTITY_WINDOW: Region = Region { plain_base: 0x3B90, debug_base: 0x4532, plain_len: 0x8F0, debug_len: 0xDC4 };

/// `PopulateSpawnedPieceCount` .. `section:children`, gate `SIGIL_EMP_CHILDREN`.
pub const CHILDREN: Region = Region { plain_base: 0x4480, debug_base: 0x52F6, plain_len: 0x2EC, debug_len: 0x44C };

/// `Load_Object` .. `section:load_object`, gate `SIGIL_EMP_LOAD_OBJECT`.
pub const LOAD_OBJECT: Region = Region { plain_base: 0x476C, debug_base: 0x5742, plain_len: 0x88, debug_len: 0x88 };

/// `Plane_Buffer_Reset` .. `section:plane_buffer`, gate `SIGIL_EMP_PLANE_BUFFER`.
pub const PLANE_BUFFER: Region = Region { plain_base: 0x47F4, debug_base: 0x57CA, plain_len: 0x31C, debug_len: 0x53A };

/// `Tile_Cache_GetTile` .. `section:tile_cache`, gate `SIGIL_EMP_TILE_CACHE`.
pub const TILE_CACHE: Region = Region { plain_base: 0x4B10, debug_base: 0x5D04, plain_len: 0xF92, debug_len: 0x1274 };

/// `Collision_GetType` .. `section:collision_lookup`, gate `SIGIL_EMP_COLLISION_LOOKUP`.
pub const COLLISION_LOOKUP: Region = Region { plain_base: 0x5AB0, debug_base: 0x6F88, plain_len: 0x64, debug_len: 0x64 };

/// `Section_Init` .. `section:section`, gate `SIGIL_EMP_SECTION`.
pub const SECTION: Region = Region { plain_base: 0x6050, debug_base: 0x7528, plain_len: 0x528, debug_len: 0x958 };

/// `Camera_Init` .. `section:camera`, gate `SIGIL_EMP_CAMERA`.
pub const CAMERA: Region = Region { plain_base: 0x6578, debug_base: 0x7E80, plain_len: 0x1C8, debug_len: 0x1D2 };

/// `Parallax_Init` .. `section:parallax`, gate `SIGIL_EMP_PARALLAX`.
pub const PARALLAX: Region = Region { plain_base: 0x6740, debug_base: 0x8052, plain_len: 0xD54, debug_len: 0xE46 };

/// `Raster_Install` .. `section:raster`, gate `SIGIL_EMP_RASTER`.
pub const RASTER: Region = Region { plain_base: 0x7498, debug_base: 0x8E9C, plain_len: 0x3EC, debug_len: 0x3EC };

/// `Palette_LoadPal` .. `Effects_InstallPreset`, gate `SIGIL_EMP_PALETTE`.
pub const PALETTE: Region = Region { plain_base: 0x7884, debug_base: 0x9288, plain_len: 0x508, debug_len: 0x508 };

/// `Effects_InstallPreset` .. `section:preset`.
pub const PRESET: Region = Region { plain_base: 0x7D8C, debug_base: 0x9790, plain_len: 0xA8, debug_len: 0xAA };

/// `Level_LoadArt` .. `section:load_art`, gate `SIGIL_EMP_LOAD_ART`.
pub const LOAD_ART: Region = Region { plain_base: 0x7E46, debug_base: 0x984A, plain_len: 0xC4, debug_len: 0xC4 };

/// `PageIn_Process` .. `section:page_in`.
pub const PAGE_IN: Region = Region { plain_base: 0x7F10, debug_base: 0x9914, plain_len: 0x3DE, debug_len: 0x4EC };

/// `PageCache_Init` .. `section:page_cache`.
pub const PAGE_CACHE: Region = Region { plain_base: 0x82FA, debug_base: 0x9E0C, plain_len: 0x862, debug_len: 0x1410 };

/// `BG_Init` .. `section:bg`, gate `SIGIL_EMP_BG`.
pub const BG: Region = Region { plain_base: 0x8B60, debug_base: 0xB220, plain_len: 0x2C8, debug_len: 0x3E8 };

/// `BgAnim_Init` .. `section:bg_anim`, gate `SIGIL_EMP_BG_ANIM`.
pub const BG_ANIM: Region = Region { plain_base: 0x8E28, debug_base: 0xB608, plain_len: 0xFC, debug_len: 0x1D0 };

/// `CompressionSelfTest` .. `section:compression_selftest` (debug-only region; plain empty at `Sound_PostByte`), gate `SIGIL_EMP_COMPRESSION_SELFTEST`.
pub const COMPRESSION_SELFTEST: Region = Region { plain_base: 0x8F24, debug_base: 0xB7D8, plain_len: 0x0, debug_len: 0xDE0 };

/// `Sound_PostByte` .. `section:sound_api`, gate `SIGIL_EMP_SOUND_API`.
pub const SOUND_API: Region = Region { plain_base: 0x8F24, debug_base: 0xC5BA, plain_len: 0x2D6, debug_len: 0x480 };

/// `TestSolid_Init` .. `section:test_solid`, gate `SIGIL_EMP_TEST_OBJECTS`.
pub const TEST_SOLID: Region = Region { plain_base: 0x126E6, debug_base: 0x12BF2, plain_len: 0x4E6, debug_len: 0x540 };

/// `TestParticle` .. `section:test_particle` (debug-only region; plain empty at `DeformTable_Zero`), gate `SIGIL_EMP_TEST_OBJECTS`.
pub const TEST_PARTICLE: Region = Region { plain_base: 0x12BCC, debug_base: 0x13132, plain_len: 0x0, debug_len: 0x58 };

/// `TestStatic_Main` .. `section:test_static`, gate `SIGIL_EMP_TEST_STATIC`.
pub const TEST_STATIC: Region = Region { plain_base: 0x126E2, debug_base: 0x128B2, plain_len: 0x4, debug_len: 0x4 };

/// `TestAnimated` .. `section:test_animated` (debug-only region; plain empty at `TestSolid_Init`), gate `SIGIL_EMP_TEST_ANIMATED`.
pub const TEST_ANIMATED: Region = Region { plain_base: 0x126E6, debug_base: 0x128B6, plain_len: 0x0, debug_len: 0x60 };

/// `TestEmitter` .. `section:test_emitter` (debug-only region; plain empty at `DeformTable_Zero`), gate `SIGIL_EMP_TEST_EMITTER`.
pub const TEST_EMITTER: Region = Region { plain_base: 0x12BCC, debug_base: 0x1318A, plain_len: 0x0, debug_len: 0x5E };

/// `TestStressEmitter` .. `section:test_stress_emitter` (debug-only region; plain empty at `DeformTable_Zero`), gate `SIGIL_EMP_TEST_STRESS_EMITTER`.
pub const TEST_STRESS_EMITTER: Region = Region { plain_base: 0x12BCC, debug_base: 0x1331E, plain_len: 0x0, debug_len: 0x5E };

/// `TestChurnObj` .. `section:test_churn` (debug-only region; plain empty at `DeformTable_Zero`), gate `SIGIL_EMP_TEST_CHURN`.
pub const TEST_CHURN: Region = Region { plain_base: 0x12BCC, debug_base: 0x1337C, plain_len: 0x0, debug_len: 0x7C };

/// `TestChildPart` .. `section:test_parent` (debug-only region; plain empty at `DeformTable_Zero`), gate `SIGIL_EMP_TEST_PARENT`.
pub const TEST_PARENT: Region = Region { plain_base: 0x12BCC, debug_base: 0x131E8, plain_len: 0x0, debug_len: 0x136 };

/// `TestPlayer` .. `section:test_player` (debug-only region; plain empty at `TestSolid_Init`), gate `SIGIL_EMP_TEST_PLAYER`.
pub const TEST_PLAYER: Region = Region { plain_base: 0x126E6, debug_base: 0x12916, plain_len: 0x0, debug_len: 0x294 };

/// `TestEnemy_Init` .. `section:test_enemy` (debug-only region; plain empty at `TestSolid_Init`), gate `SIGIL_EMP_TEST_ENEMY`.
pub const TEST_ENEMY: Region = Region { plain_base: 0x126E6, debug_base: 0x12BAA, plain_len: 0x0, debug_len: 0x48 };

/// `OJZ_TestRaster` .. `section:ojz_effects`.
pub const OJZ_EFFECTS: Region = Region { plain_base: 0x145DA, debug_base: 0x14E18, plain_len: 0x94A, debug_len: 0xB62 };

/// `DeformTable_Zero` .. `section:scene_registry`, gate `SIGIL_EMP_SCENE_REGISTRY`.
pub const SCENE_REGISTRY: Region = Region { plain_base: 0x12BCC, debug_base: 0x133F8, plain_len: 0x1486, debug_len: 0x1486 };

/// `Map_TestObj` .. `section:test_mappings`, gate `SIGIL_EMP_TEST_MAPPINGS`.
pub const TEST_MAPPINGS: Region = Region { plain_base: 0x28382, debug_base: 0x2ED78, plain_len: 0x30, debug_len: 0x30 };

/// `Map_DustSpindash` .. `section:dust_data`, gate `SIGIL_EMP_DUST_DATA`.
pub const DUST_DATA: Region = Region { plain_base: 0x283B2, debug_base: 0x2EDA8, plain_len: 0xBDA, debug_len: 0xBDA };

/// `Ani_Sonic` .. `section:sonic_anims`, gate `SIGIL_EMP_SONIC_ANIMS`.
pub const SONIC_ANIMS: Region = Region { plain_base: 0x28F8C, debug_base: 0x2F982, plain_len: 0x10A, debug_len: 0x10A };

/// `Ani_Tails` .. `section:tails_anims`, gate `SIGIL_EMP_TAILS_ANIMS`.
pub const TAILS_ANIMS: Region = Region { plain_base: 0x29096, debug_base: 0x2FA8C, plain_len: 0x1BC, debug_len: 0x1BC };

/// `Ani_Knuckles` .. `section:knuckles_anims`, gate `SIGIL_EMP_KNUCKLES_ANIMS`.
pub const KNUCKLES_ANIMS: Region = Region { plain_base: 0x29252, debug_base: 0x2FC48, plain_len: 0x16B, debug_len: 0x16B };

/// `Map_Tails` .. `section:tails_data`, gate `SIGIL_EMP_TAILS_DATA`.
pub const TAILS_DATA: Region = Region { plain_base: 0x293D2, debug_base: 0x2FDD0, plain_len: 0x20F5E, debug_len: 0x20F5E };

/// `Map_Knuckles` .. `section:knuckles_data`, gate `SIGIL_EMP_KNUCKLES_DATA`.
pub const KNUCKLES_DATA: Region = Region { plain_base: 0x4A330, debug_base: 0x50D2E, plain_len: 0x226C8, debug_len: 0x226C8 };

/// `Ani_Particle` .. `section:particle_anims` (debug-only region; plain empty at `Ani_DustSpindash`), gate `SIGIL_EMP_PARTICLE_ANIMS`.
pub const PARTICLE_ANIMS: Region = Region { plain_base: 0x293BE, debug_base: 0x2FDB4, plain_len: 0x0, debug_len: 0x7 };

/// `Ani_DustSpindash` .. `section:dust_anims`, gate `SIGIL_EMP_DUST_ANIMS`.
pub const DUST_ANIMS: Region = Region { plain_base: 0x293BE, debug_base: 0x2FDBC, plain_len: 0x14, debug_len: 0x14 };

/// `OJZ_Sec0_TypeTable` .. `section:entity_data`.
pub const ENTITY_DATA: Region = Region { plain_base: 0x14F58, debug_base: 0x159AE, plain_len: 0x18E, debug_len: 0x18E };

/// `OJZ_Act_Pool_Page0` .. `section:ojz_act_pool`.
pub const OJZ_ACT_POOL: Region = Region { plain_base: 0x150E6, debug_base: 0x15B3C, plain_len: 0x2F0C, debug_len: 0x2F0C };

/// `OJZ_Act1_Descriptor` .. `section:act_descriptor`, gate `SIGIL_EMP_ACT_DESCRIPTOR`.
pub const ACT_DESCRIPTOR: Region = Region { plain_base: 0x17FF2, debug_base: 0x18A48, plain_len: 0x240, debug_len: 0x294 };

/// `OJZ_Sec0_Blocks` .. `section:sec_block_blobs`.
pub const SEC_BLOCK_BLOBS: Region = Region { plain_base: 0x18232, debug_base: 0x18CDC, plain_len: 0x93B4, debug_len: 0x93B4 };

/// `OJZ_Sec0_LocalMap` .. `section:sec_local_maps`.
pub const SEC_LOCAL_MAPS: Region = Region { plain_base: 0x215E6, debug_base: 0x22090, plain_len: 0x4EC, debug_len: 0x4EC };

/// `OJZ_Palette` .. `section:ojz_act_assets`.
pub const OJZ_ACT_ASSETS: Region = Region { plain_base: 0x21AD2, debug_base: 0x2257C, plain_len: 0x4882, debug_len: 0xA744 };

/// `BgAnim_Table` .. `section:ojz_bg_anim`.
pub const OJZ_BG_ANIM: Region = Region { plain_base: 0x26354, debug_base: 0x2CCC0, plain_len: 0x202E, debug_len: 0x20B8 };

/// `ObjDef_Static` .. start + 0x34 plain / 0x34 debug (literal, no end symbol), gate `SIGIL_EMP_OBJDEFS`.
pub const OBJDEFS: Region = Region { plain_base: 0x14F24, debug_base: 0x1597A, plain_len: 0x34, debug_len: 0x34 };

/// `GameState_ObjectTest_Init` .. `section:object_test_state` (debug-only region; plain empty at `GameState_OJZScroll_Init`), gate `SIGIL_EMP_OBJECT_TEST_STATE`.
pub const OBJECT_TEST_STATE: Region = Region { plain_base: 0xC12AC, debug_base: 0xC12AC, plain_len: 0x0, debug_len: 0x384 };

/// `GameState_OJZScroll_Init` .. `section:ojz_scroll_test`, gate `SIGIL_EMP_OJZ_SCROLL_TEST`.
pub const OJZ_SCROLL_TEST: Region = Region { plain_base: 0xC12AC, debug_base: 0xC1630, plain_len: 0x5E2, debug_len: 0x2070 };

/// `Replay_OJZ_Fixture` .. `section:replay_fixture`.
pub const REPLAY_FIXTURE: Region = Region { plain_base: 0xC1892, debug_base: 0xC36A0, plain_len: 0x260, debug_len: 0x260 };

/// `BusError` .. `section:error_handler`, gate `SIGIL_EMP_ERROR_HANDLER`.
pub const ERROR_HANDLER: Region = Region { plain_base: 0xC1AF2, debug_base: 0xC3900, plain_len: 0x10B0, debug_len: 0x10B0 };

/// `Dac_Temp_Blip` .. `section:dac_banks`, gate `SIGIL_EMP_DAC`.
pub const DAC_BANKS: Region = Region { plain_base: 0xA8000, debug_base: 0xA8000, plain_len: 0xE49A, debug_len: 0xE49A };

/// `Song_MovingTrucks` .. `section:mt_bank`, gate `SIGIL_EMP_MT`.
pub const MT_BANK_BLOB: Region = Region { plain_base: 0xB86B0, debug_base: 0xB86B0, plain_len: 0x4EB8, debug_len: 0x6900 };

/// `Sfx_33` .. `section:sfx_bank_blob`, gate `SIGIL_EMP_SFX`.
pub const SFX_BANK_BLOB: Region = Region { plain_base: 0xBD568, debug_base: 0xBEFB0, plain_len: 0x664, debug_len: 0x674 };

/// `SoundTablesZ80_Head` .. `section:soundbankhead`, gate `SIGIL_EMP_SOUNDBANKHEAD`.
pub const SOUNDBANKHEAD: Region = Region { plain_base: 0xB8000, debug_base: 0xB8000, plain_len: 0x6B0, debug_len: 0x6B0 };

/// `EndOfRom` .. start + 0x0 plain / 0x0 debug (literal, no end symbol), gate `SIGIL_EMP_EPILOGUE`.
pub const EPILOGUE: Region = Region { plain_base: 0xC2BA2, debug_base: 0xC49B0, plain_len: 0x0, debug_len: 0x0 };

/// `ObjCodeBase` .. start + 0x2 plain / 0x2 debug (literal, no end symbol), gate `SIGIL_EMP_OBJCODEBASE`.
pub const OBJCODEBASE: Region = Region { plain_base: 0x10000, debug_base: 0x10000, plain_len: 0x2, debug_len: 0x2 };

/// `Player_Init` .. `section:player_common`, gate `SIGIL_EMP_PLAYER_COMMON`.
pub const PLAYER_COMMON: Region = Region { plain_base: 0x10002, debug_base: 0x10002, plain_len: 0x9C6, debug_len: 0xADC };

/// `CharDef_Sonic` .. `section:sonic`, gate `SIGIL_EMP_SONIC`.
pub const SONIC: Region = Region { plain_base: 0x12274, debug_base: 0x12386, plain_len: 0x36, debug_len: 0x36 };

/// `CharDef_Tails` .. `section:tails`, gate `SIGIL_EMP_TAILS`.
pub const TAILS: Region = Region { plain_base: 0x122AA, debug_base: 0x123BC, plain_len: 0x36, debug_len: 0x36 };

/// `CharDef_Knuckles` .. `section:knuckles`, gate `SIGIL_EMP_KNUCKLES`.
pub const KNUCKLES: Region = Region { plain_base: 0x122E0, debug_base: 0x123F2, plain_len: 0x36, debug_len: 0x36 };

/// `CharacterDefs` .. `section:characters`, gate `SIGIL_EMP_CHARACTERS`.
pub const CHARACTERS: Region = Region { plain_base: 0x12316, debug_base: 0x12428, plain_len: 0x4A, debug_len: 0xB0 };

/// `TailsAppendage_Refresh` .. `section:tails_appendage`, gate `SIGIL_EMP_TAILS_APPENDAGE`.
pub const TAILS_APPENDAGE: Region = Region { plain_base: 0x12360, debug_base: 0x124D8, plain_len: 0x11C, debug_len: 0x174 };

/// `DustPuff_Spawn` .. `section:dust_puff`, gate `SIGIL_EMP_DUST_PUFF`.
pub const DUST_PUFF: Region = Region { plain_base: 0x1247C, debug_base: 0x1264C, plain_len: 0x46, debug_len: 0x46 };

/// `Dust_Tick` .. `section:dust_spindash`, gate `SIGIL_EMP_DUST_SPINDASH`.
pub const DUST_SPINDASH: Region = Region { plain_base: 0x124C2, debug_base: 0x12692, plain_len: 0x102, debug_len: 0x102 };

/// `PState_Ground` .. `section:player_ground`, gate `SIGIL_EMP_PLAYER_GROUND`.
pub const PLAYER_GROUND: Region = Region { plain_base: 0x109CC, debug_base: 0x10AE2, plain_len: 0x4C6, debug_len: 0x4C6 };

/// `PState_Air` .. `section:player_air`, gate `SIGIL_EMP_PLAYER_AIR`.
pub const PLAYER_AIR: Region = Region { plain_base: 0x10E92, debug_base: 0x10FA8, plain_len: 0x34A, debug_len: 0x34A };

/// `PState_Spindash` .. `section:player_spindash`, gate `SIGIL_EMP_PLAYER_SPINDASH`.
pub const PLAYER_SPINDASH: Region = Region { plain_base: 0x111DC, debug_base: 0x112F2, plain_len: 0xDE, debug_len: 0xDA };

/// `PState_Fly` .. `section:player_fly`, gate `SIGIL_EMP_PLAYER_FLY`.
pub const PLAYER_FLY: Region = Region { plain_base: 0x112BA, debug_base: 0x113CC, plain_len: 0x13A, debug_len: 0x138 };

/// `PState_Glide` .. `section:player_glide`, gate `SIGIL_EMP_PLAYER_GLIDE`.
pub const PLAYER_GLIDE: Region = Region { plain_base: 0x113FE, debug_base: 0x11510, plain_len: 0x2DC, debug_len: 0x2D8 };

/// `Climb_WallDist` .. `CharDef_Sonic`, gate `SIGIL_EMP_PLAYER_CLIMB`.
pub const PLAYER_CLIMB: Region = Region { plain_base: 0x116F8, debug_base: 0x1180A, plain_len: 0xB7C, debug_len: 0xB7C };

/// `Collision_ProbeDown` .. `section:player_sensors`, gate `SIGIL_EMP_PLAYER_SENSORS`.
pub const PLAYER_SENSORS: Region = Region { plain_base: 0x5B14, debug_base: 0x6FEC, plain_len: 0x53C, debug_len: 0x53C };

// ── Symbols (manifest order) ──

/// `OJZ_Preset_Sec0`.
pub const OJZ_PRESET_SEC0: Pin = Pin { plain: 0x14BB2, debug: 0x154F0 };

/// `OJZ_Preset_Sec1`.
pub const OJZ_PRESET_SEC1: Pin = Pin { plain: 0x14BE0, debug: 0x1551E };

/// `OJZ_Preset_Sec2`.
pub const OJZ_PRESET_SEC2: Pin = Pin { plain: 0x14C0E, debug: 0x1554C };

/// `OJZ_Preset_Sec3`.
pub const OJZ_PRESET_SEC3: Pin = Pin { plain: 0x14C3C, debug: 0x1557A };

/// `OJZ_Preset_Plain`.
pub const OJZ_PRESET_PLAIN: Pin = Pin { plain: 0x14C6A, debug: 0x155A8 };

/// `OJZ_Preset_Depth`.
pub const OJZ_PRESET_DEPTH: Pin = Pin { plain: 0x14C98, debug: 0x155D6 };

/// `EditorSceneBinding_OJZ_Act1_Sec4`.
pub const EDITOR_SCENE_BINDING_OJZ_ACT1_SEC4: Pin = Pin { plain: 0x14110, debug: 0x1493C };

/// `OJZ_Preset_Sec5`.
pub const OJZ_PRESET_SEC5: Pin = Pin { plain: 0x14CC6, debug: 0x15604 };

/// `EditorRaster_OJZ_Act1_authored_probe`.
pub const EDITOR_RASTER_OJZ_ACT1_AUTHORED_PROBE: Pin = Pin { plain: 0x1434A, debug: 0x14B88 };

/// `EditorRaster_OJZ_Act1_ojz_sec5_showcase`.
pub const EDITOR_RASTER_OJZ_ACT1_OJZ_SEC5_SHOWCASE: Pin = Pin { plain: 0x1444A, debug: 0x14C88 };

/// `EditorRaster_OJZ_Act1_ojz_sec3_shimmer`.
pub const EDITOR_RASTER_OJZ_ACT1_OJZ_SEC3_SHIMMER: Pin = Pin { plain: 0x143CA, debug: 0x14C08 };

/// `EditorCycle_OJZ_Act1_ojz_sec3_shimmer`.
pub const EDITOR_CYCLE_OJZ_ACT1_OJZ_SEC3_SHIMMER: Pin = Pin { plain: 0x145CA, debug: 0x14E08 };

/// `Effects_InstallPreset`.
pub const EFFECTS_INSTALL_PRESET: Pin = Pin { plain: 0x7D8C, debug: 0x9790 };

/// `Raster_GetChannelBand`.
pub const RASTER_GET_CHANNEL_BAND: Pin = Pin { plain: 0x77A0, debug: 0x91A4 };

/// `TestStatic_Main`.
pub const TEST_STATIC_MAIN: Pin = Pin { plain: 0x126E2, debug: 0x128B2 };

/// `TestSolid_Init`.
pub const TEST_SOLID_INIT: Pin = Pin { plain: 0x126E6, debug: 0x12BF2 };

/// `TestEnemy_Init`, debug-shape consumer only (`debug_only`).
pub const TEST_ENEMY_INIT: u32 = 0x12BAA;

/// `TestParent`, debug-shape consumer only (`debug_only`).
pub const TEST_PARENT_LABEL: u32 = 0x13272;

/// `Map_TestObj`.
pub const MAP_TEST_OBJ: Pin = Pin { plain: 0x28382, debug: 0x2ED78 };

/// `Map_Sonic`.
pub const MAP_SONIC: Pin = Pin { plain: 0x6EBF8, debug: 0x755F6 };

/// `DPLC_Sonic`.
pub const DPLC_SONIC: Pin = Pin { plain: 0x70878, debug: 0x77276 };

/// `Art_Sonic`.
pub const ART_SONIC: Pin = Pin { plain: 0x7113A, debug: 0x77B38 };

/// `CreateEffect_Normal`.
pub const CREATE_EFFECT_NORMAL: Pin = Pin { plain: 0x46D6, debug: 0x56AC };

/// `CreateChild_Normal`.
pub const CREATE_CHILD_NORMAL: Pin = Pin { plain: 0x44AC, debug: 0x5322 };

/// `DeleteChildren`.
pub const DELETE_CHILDREN: Pin = Pin { plain: 0x46B8, debug: 0x568E };

/// `GetSineCosine`.
pub const GET_SINE_COSINE: Pin = Pin { plain: 0x297A, debug: 0x2C10 };

/// `EntryPoint`.
pub const ENTRY_POINT: Pin = Pin { plain: 0x200, debug: 0x200 };

/// `BusError`, debug-shape consumer only (`debug_only`).
pub const BUS_ERROR: u32 = 0xC3900;

/// `AddressError`, debug-shape consumer only (`debug_only`).
pub const ADDRESS_ERROR: u32 = 0xC3918;

/// `IllegalInstr`, debug-shape consumer only (`debug_only`).
pub const ILLEGAL_INSTR: u32 = 0xC3934;

/// `ZeroDivide`, debug-shape consumer only (`debug_only`).
pub const ZERO_DIVIDE: u32 = 0xC3956;

/// `ChkInstr`, debug-shape consumer only (`debug_only`).
pub const CHK_INSTR: u32 = 0xC3970;

/// `TrapvInstr`, debug-shape consumer only (`debug_only`).
pub const TRAPV_INSTR: u32 = 0xC398E;

/// `PrivilegeViol`, debug-shape consumer only (`debug_only`).
pub const PRIVILEGE_VIOL: u32 = 0xC39AE;

/// `Trace`, debug-shape consumer only (`debug_only`).
pub const TRACE: u32 = 0xC39D0;

/// `Line1010Emu`, debug-shape consumer only (`debug_only`).
pub const LINE1010_EMU: u32 = 0xC39E4;

/// `Line1111Emu`, debug-shape consumer only (`debug_only`).
pub const LINE1111_EMU: u32 = 0xC3A04;

/// `ErrorExcept`, debug-shape consumer only (`debug_only`).
pub const ERROR_EXCEPT: u32 = 0xC3A24;

/// `ErrorTrap`, debug-shape consumer only (`debug_only`).
pub const ERROR_TRAP: u32 = 0xC3A42;

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
pub const SOUND_DRAIN_SFX_RING: Pin = Pin { plain: 0x9098, debug: 0xC8D8 };

/// `Game_State`.
pub const GAME_STATE: Pin = Pin { plain: 0xFFFF8008, debug: 0xFFFF8008 };

/// `Input_Tick`.
pub const INPUT_TICK: Pin = Pin { plain: 0x2674, debug: 0x2806 };

/// `Cache_Left_Col`.
pub const CACHE_LEFT_COL: Pin = Pin { plain: 0xFFFFAE04, debug: 0xFFFFAE78 };

/// `Draw_TileColumn`.
pub const DRAW_TILE_COLUMN: Pin = Pin { plain: 0x47FC, debug: 0x57D2 };

/// `Draw_TileRow_FromCache`.
pub const DRAW_TILE_ROW_FROM_CACHE: Pin = Pin { plain: 0x4950, debug: 0x5A18 };

/// `EntityWindow_Init`.
pub const ENTITY_WINDOW_INIT: Pin = Pin { plain: 0x3F14, debug: 0x4B6E };

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
pub const S4_LZ_DECOMPRESS_DICT: Pin = Pin { plain: 0x27BC, debug: 0x29FE };

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
pub const BG_ANIM_TABLE: Pin = Pin { plain: 0x26354, debug: 0x2CCC0 };

/// `Camera_X_Biased`.
pub const CAMERA_X_BIASED: Pin = Pin { plain: 0xFFFFA6EA, debug: 0xFFFFA75E };

/// `Camera_Y_Biased`.
pub const CAMERA_Y_BIASED: Pin = Pin { plain: 0xFFFFA6EC, debug: 0xFFFFA760 };

/// `Collected_MarkRing`.
pub const COLLECTED_MARK_RING: Pin = Pin { plain: 0x3BF8, debug: 0x459A };

/// `EntityWindow_EntryForSection`.
pub const ENTITY_WINDOW_ENTRY_FOR_SECTION: Pin = Pin { plain: 0x3DF8, debug: 0x49FC };

/// `EntityLoaded_Clear`.
pub const ENTITY_LOADED_CLEAR: Pin = Pin { plain: 0x3DE4, debug: 0x4986 };

/// `Sound_PlayRing`.
pub const SOUND_PLAY_RING: Pin = Pin { plain: 0x910E, debug: 0xC94E };

/// `MDDBG__ErrorHandler`, debug-shape consumer only (`debug_only`).
pub const MDDBG_ERROR_HANDLER: u32 = 0xC3A5A;

/// `MDDBG__ErrorHandler_PagesController`, debug-shape consumer only (`debug_only`).
pub const MDDBG_ERROR_HANDLER_PAGES_CONTROLLER: u32 = 0xC4820;

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
pub const S4_LZ_DECOMPRESS: Pin = Pin { plain: 0x27C0, debug: 0x2A56 };

/// `QueueDMA_Critical`.
pub const QUEUE_DMA_CRITICAL: Pin = Pin { plain: 0x1D98, debug: 0x1E76 };

/// `BG_Init`.
pub const BG_INIT: Pin = Pin { plain: 0x8B60, debug: 0xB220 };

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
pub const DELETE_OBJECT: Pin = Pin { plain: 0x2EE4, debug: 0x317A };

/// `DrawRings`.
pub const DRAW_RINGS: Pin = Pin { plain: 0x3A80, debug: 0x4414 };

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
pub const SOUND_PLAY_SFX: Pin = Pin { plain: 0x9052, debug: 0xC84C };

/// `ObjectMoveX`.
pub const OBJECT_MOVE_X: Pin = Pin { plain: 0x30E8, debug: 0x37D0 };

/// `ObjCodeBase`.
pub const OBJ_CODE_BASE: Pin = Pin { plain: 0x10000, debug: 0x10000 };

/// `Draw_Sprite`.
pub const DRAW_SPRITE: Pin = Pin { plain: 0x3118, debug: 0x3800 };

/// `ObjectMove`.
pub const OBJECT_MOVE: Pin = Pin { plain: 0x30CE, debug: 0x37B6 };

/// `Ring_Sfx_Speaker`.
pub const RING_SFX_SPEAKER: Pin = Pin { plain: 0xFFFFB524, debug: 0xFFFFB598 };

/// `Sfx_Ring_Buf`.
pub const SFX_RING_BUF: Pin = Pin { plain: 0xFFFFB526, debug: 0xFFFFB59A };

/// `Sfx_Ring_Wr`.
pub const SFX_RING_WR: Pin = Pin { plain: 0xFFFFB52E, debug: 0xFFFFB5A2 };

/// `Sfx_Ring_Rd`.
pub const SFX_RING_RD: Pin = Pin { plain: 0xFFFFB52F, debug: 0xFFFFB5A3 };

/// `SongTable`.
pub const SONG_TABLE: Pin = Pin { plain: 0xBDBAC, debug: 0xBF5F4 };

/// `SongPatchTable`.
pub const SONG_PATCH_TABLE: Pin = Pin { plain: 0xBDBBC, debug: 0xBF60C };

/// `OJZ_Palette`.
pub const OJZ_PALETTE: Pin = Pin { plain: 0x21AD2, debug: 0x2257C };

/// `OJZ_Act1_BG_Layout`.
pub const OJZ_ACT1_BG_LAYOUT: Pin = Pin { plain: 0x21B52, debug: 0x225FC };

/// `OJZ_Act1_BG_Tiles`.
pub const OJZ_ACT1_BG_TILES: Pin = Pin { plain: 0x23B52, debug: 0x245FC };

/// `ParallaxConfig_OJZ_Default`.
pub const PARALLAX_CONFIG_OJZ_DEFAULT: Pin = Pin { plain: 0x12CCC, debug: 0x134F8 };

/// `OJZ_Act_Pool_PageTable`.
pub const OJZ_ACT_POOL_PAGE_TABLE: Pin = Pin { plain: 0x17FA2, debug: 0x189F8 };

/// `OJZ_Sec_LocalMaps`.
pub const OJZ_SEC_LOCAL_MAPS: Pin = Pin { plain: 0x21AAE, debug: 0x22558 };

/// `OJZ_Sec0_Blocks`.
pub const OJZ_SEC0_BLOCKS: Pin = Pin { plain: 0x18232, debug: 0x18CDC };

/// `OJZ_Sec1_Blocks`.
pub const OJZ_SEC1_BLOCKS: Pin = Pin { plain: 0x19EE6, debug: 0x1A990 };

/// `OJZ_Sec2_Blocks`.
pub const OJZ_SEC2_BLOCKS: Pin = Pin { plain: 0x1AF04, debug: 0x1B9AE };

/// `OJZ_Sec3_Blocks`.
pub const OJZ_SEC3_BLOCKS: Pin = Pin { plain: 0x1C198, debug: 0x1CC42 };

/// `OJZ_Sec4_Blocks`.
pub const OJZ_SEC4_BLOCKS: Pin = Pin { plain: 0x1AF04, debug: 0x1B9AE };

/// `OJZ_Sec5_Blocks`.
pub const OJZ_SEC5_BLOCKS: Pin = Pin { plain: 0x1CFA6, debug: 0x1DA50 };

/// `OJZ_Sec6_Blocks`.
pub const OJZ_SEC6_BLOCKS: Pin = Pin { plain: 0x1D9A2, debug: 0x1E44C };

/// `OJZ_Sec7_Blocks`.
pub const OJZ_SEC7_BLOCKS: Pin = Pin { plain: 0x1F068, debug: 0x1FB12 };

/// `OJZ_Sec8_Blocks`.
pub const OJZ_SEC8_BLOCKS: Pin = Pin { plain: 0x1FF86, debug: 0x20A30 };

/// `OJZ_Sec0_Objects`.
pub const OJZ_SEC0_OBJECTS: Pin = Pin { plain: 0x14F62, debug: 0x159B8 };

/// `OJZ_Sec0_Rings`.
pub const OJZ_SEC0_RINGS: Pin = Pin { plain: 0x14F94, debug: 0x159EA };

/// `OJZ_Sec0_TypeTable`.
pub const OJZ_SEC0_TYPE_TABLE: Pin = Pin { plain: 0x14F58, debug: 0x159AE };

/// `OJZ_Sec1_Objects`.
pub const OJZ_SEC1_OBJECTS: Pin = Pin { plain: 0x14FBA, debug: 0x15A10 };

/// `OJZ_Sec1_Rings`.
pub const OJZ_SEC1_RINGS: Pin = Pin { plain: 0x14FC2, debug: 0x15A18 };

/// `OJZ_Sec1_TypeTable`.
pub const OJZ_SEC1_TYPE_TABLE: Pin = Pin { plain: 0x14FB4, debug: 0x15A0A };

/// `OJZ_Sec2_Objects`.
pub const OJZ_SEC2_OBJECTS: Pin = Pin { plain: 0x14FF4, debug: 0x15A4A };

/// `OJZ_Sec2_Rings`.
pub const OJZ_SEC2_RINGS: Pin = Pin { plain: 0x15002, debug: 0x15A58 };

/// `OJZ_Sec2_TypeTable`.
pub const OJZ_SEC2_TYPE_TABLE: Pin = Pin { plain: 0x14FEA, debug: 0x15A40 };

/// `OJZ_Sec3_Objects`.
pub const OJZ_SEC3_OBJECTS: Pin = Pin { plain: 0x15038, debug: 0x15A8E };

/// `OJZ_Sec3_Rings`.
pub const OJZ_SEC3_RINGS: Pin = Pin { plain: 0x1503A, debug: 0x15A90 };

/// `OJZ_Sec3_TypeTable`.
pub const OJZ_SEC3_TYPE_TABLE: Pin = Pin { plain: 0x15036, debug: 0x15A8C };

/// `OJZ_Sec4_Objects`.
pub const OJZ_SEC4_OBJECTS: Pin = Pin { plain: 0x15040, debug: 0x15A96 };

/// `OJZ_Sec4_Rings`.
pub const OJZ_SEC4_RINGS: Pin = Pin { plain: 0x15042, debug: 0x15A98 };

/// `OJZ_Sec4_TypeTable`.
pub const OJZ_SEC4_TYPE_TABLE: Pin = Pin { plain: 0x1503E, debug: 0x15A94 };

/// `OJZ_Sec5_Objects`.
pub const OJZ_SEC5_OBJECTS: Pin = Pin { plain: 0x15078, debug: 0x15ACE };

/// `OJZ_Sec5_Rings`.
pub const OJZ_SEC5_RINGS: Pin = Pin { plain: 0x1507A, debug: 0x15AD0 };

/// `OJZ_Sec5_TypeTable`.
pub const OJZ_SEC5_TYPE_TABLE: Pin = Pin { plain: 0x15076, debug: 0x15ACC };

/// `OJZ_Sec6_Objects`.
pub const OJZ_SEC6_OBJECTS: Pin = Pin { plain: 0x150A0, debug: 0x15AF6 };

/// `OJZ_Sec6_Rings`.
pub const OJZ_SEC6_RINGS: Pin = Pin { plain: 0x150A2, debug: 0x15AF8 };

/// `OJZ_Sec6_TypeTable`.
pub const OJZ_SEC6_TYPE_TABLE: Pin = Pin { plain: 0x1509E, debug: 0x15AF4 };

/// `OJZ_Sec7_Objects`.
pub const OJZ_SEC7_OBJECTS: Pin = Pin { plain: 0x150A8, debug: 0x15AFE };

/// `OJZ_Sec7_Rings`.
pub const OJZ_SEC7_RINGS: Pin = Pin { plain: 0x150AA, debug: 0x15B00 };

/// `OJZ_Sec7_TypeTable`.
pub const OJZ_SEC7_TYPE_TABLE: Pin = Pin { plain: 0x150A6, debug: 0x15AFC };

/// `OJZ_Sec8_Objects`.
pub const OJZ_SEC8_OBJECTS: Pin = Pin { plain: 0x150D0, debug: 0x15B26 };

/// `OJZ_Sec8_Rings`.
pub const OJZ_SEC8_RINGS: Pin = Pin { plain: 0x150D2, debug: 0x15B28 };

/// `OJZ_Sec8_TypeTable`.
pub const OJZ_SEC8_TYPE_TABLE: Pin = Pin { plain: 0x150CE, debug: 0x15B24 };

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
pub const RING_BUFFER_CLEAR: Pin = Pin { plain: 0x3A4C, debug: 0x43E0 };

/// `RingBuffer_Remove`.
pub const RING_BUFFER_REMOVE: Pin = Pin { plain: 0x3A18, debug: 0x43AC };

/// `Section_GetSecPtrXY`.
pub const SECTION_GET_SEC_PTR_XY: Pin = Pin { plain: 0x60A0, debug: 0x7578 };

/// `Section_FlatIDXY`.
pub const SECTION_FLAT_IDXY: Pin = Pin { plain: 0x6086, debug: 0x755E };

/// `AllocDynamic`.
pub const ALLOC_DYNAMIC: Pin = Pin { plain: 0x2E66, debug: 0x30FC };

/// `AllocEffect`.
pub const ALLOC_EFFECT: Pin = Pin { plain: 0x2ECA, debug: 0x3160 };

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
pub const PARALLAX_ACTIVE_CONFIG: Pin = Pin { plain: 0x6896, debug: 0x824E };

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
pub const PAGE_IN_BANK_REGS: Pin = Pin { plain: 0x816C, debug: 0x9C72 };

/// `Dbg_PageIn_Preempts`, debug-shape consumer only (`debug_only`).
pub const DBG_PAGE_IN_PREEMPTS: u32 = 0xFFFF8FDA;

/// `ZX0R_Decompress.__end`.
pub const ZX0R_DECOMPRESS_END: Pin = Pin { plain: 0x297A, debug: 0x2C10 };

/// `PageIn_Staging_Busy`.
pub const PAGE_IN_STAGING_BUSY: Pin = Pin { plain: 0xFFFFBA08, debug: 0xFFFFBA7C };

/// `PageIn_Flush`.
pub const PAGE_IN_FLUSH: Pin = Pin { plain: 0x8288, debug: 0x9D96 };

/// `PageIn_Enqueue`.
pub const PAGE_IN_ENQUEUE: Pin = Pin { plain: 0x824A, debug: 0x9D58 };

/// `PageIn_Pool_Table`.
pub const PAGE_IN_POOL_TABLE: Pin = Pin { plain: 0xFFFFBA34, debug: 0xFFFFBAA8 };

/// `PageIn_Queue_Count`.
pub const PAGE_IN_QUEUE_COUNT: Pin = Pin { plain: 0xFFFFBA0A, debug: 0xFFFFBA7E };

/// `PageIn_Suspended`.
pub const PAGE_IN_SUSPENDED: Pin = Pin { plain: 0xFFFFBA07, debug: 0xFFFFBA7B };

/// `PageIn_Land_Pending`.
pub const PAGE_IN_LAND_PENDING: Pin = Pin { plain: 0xFFFFBA09, debug: 0xFFFFBA7D };

/// `PageCache_Init`.
pub const PAGE_CACHE_INIT: Pin = Pin { plain: 0x82FA, debug: 0x9E0C };

/// `PageCache_AllocFrame`.
pub const PAGE_CACHE_ALLOC_FRAME: Pin = Pin { plain: 0x83B0, debug: 0x9F28 };

/// `PageCache_Publish`.
pub const PAGE_CACHE_PUBLISH: Pin = Pin { plain: 0x84EA, debug: 0xA10A };

/// `PageCache_PatchRun_Seq`.
pub const PAGE_CACHE_PATCH_RUN_SEQ: Pin = Pin { plain: 0x85A0, debug: 0xA226 };

/// `PageCache_PatchRun_Col`.
pub const PAGE_CACHE_PATCH_RUN_COL: Pin = Pin { plain: 0x86D4, debug: 0xA4AA };

/// `PageCache_Audit`.
pub const PAGE_CACHE_AUDIT: Pin = Pin { plain: 0x8B5A, debug: 0xAAA0 };

/// `Cache_Art_Stall`.
pub const CACHE_ART_STALL: Pin = Pin { plain: 0xFFFFAE16, debug: 0xFFFFAE8A };

/// `Page_Audit_Ticks`, debug-shape consumer only (`debug_only`).
pub const PAGE_AUDIT_TICKS: u32 = 0xFFFF8FEE;

/// `Cache_Stall_Watchdog`, debug-shape consumer only (`debug_only`).
pub const CACHE_STALL_WATCHDOG: u32 = 0xFFFF8FEC;

/// `Flush_VDP_Shadow`.
pub const FLUSH_VDP_SHADOW: Pin = Pin { plain: 0x1C52, debug: 0x1CDA };

/// `VInt_DrawLevel`.
pub const V_INT_DRAW_LEVEL: Pin = Pin { plain: 0x4AA4, debug: 0x5C2C };

/// `Vscroll_Write`.
pub const VSCROLL_WRITE: Pin = Pin { plain: 0x68A8, debug: 0x8260 };

/// `Read_Controllers`.
pub const READ_CONTROLLERS: Pin = Pin { plain: 0x2540, debug: 0x26CE };

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
pub const SOUND_INIT: Pin = Pin { plain: 0x8F4A, debug: 0xC5E0 };

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
pub const P_STATE_GROUND: Pin = Pin { plain: 0x109CC, debug: 0x10AE2 };

/// `PState_Roll`.
pub const P_STATE_ROLL: Pin = Pin { plain: 0x10B6C, debug: 0x10C82 };

/// `PState_Spindash`.
pub const P_STATE_SPINDASH: Pin = Pin { plain: 0x111DC, debug: 0x112F2 };

/// `PState_Air`.
pub const P_STATE_AIR: Pin = Pin { plain: 0x10E92, debug: 0x10FA8 };

/// `PState_Jump`.
pub const P_STATE_JUMP: Pin = Pin { plain: 0x10E9A, debug: 0x10FB0 };

/// `PState_RollJump`.
pub const P_STATE_ROLL_JUMP: Pin = Pin { plain: 0x10E96, debug: 0x10FAC };

/// `PState_AirBall`.
pub const P_STATE_AIR_BALL: Pin = Pin { plain: 0x10E92, debug: 0x10FA8 };

/// `PState_Fly`.
pub const P_STATE_FLY: Pin = Pin { plain: 0x112BA, debug: 0x113CC };

/// `PState_Glide`.
pub const P_STATE_GLIDE: Pin = Pin { plain: 0x113FE, debug: 0x11510 };

/// `PState_GlideFall`.
pub const P_STATE_GLIDE_FALL: Pin = Pin { plain: 0x115B8, debug: 0x116CA };

/// `PState_Slide`.
pub const P_STATE_SLIDE: Pin = Pin { plain: 0x11606, debug: 0x11716 };

/// `PState_Climb`.
pub const P_STATE_CLIMB: Pin = Pin { plain: 0x11752, debug: 0x11864 };

/// `PState_Ledge`.
pub const P_STATE_LEDGE: Pin = Pin { plain: 0x118FE, debug: 0x11A10 };

/// `Player_SensorFloor`.
pub const PLAYER_SENSOR_FLOOR: Pin = Pin { plain: 0x5E84, debug: 0x735C };

/// `Player_AtLedgeEdge`.
pub const PLAYER_AT_LEDGE_EDGE: Pin = Pin { plain: 0x5F9C, debug: 0x7474 };

/// `Player_SetState`.
pub const PLAYER_SET_STATE: Pin = Pin { plain: 0x106F6, debug: 0x107B8 };

/// `Player_SnapToSurface`.
pub const PLAYER_SNAP_TO_SURFACE: Pin = Pin { plain: 0x10860, debug: 0x10922 };

/// `Player_SensorCeiling`.
pub const PLAYER_SENSOR_CEILING: Pin = Pin { plain: 0x5E9A, debug: 0x7372 };

/// `Player_SensorWallDir`.
pub const PLAYER_SENSOR_WALL_DIR: Pin = Pin { plain: 0x5F52, debug: 0x742A };

/// `Player_SensorWallAt`.
pub const PLAYER_SENSOR_WALL_AT: Pin = Pin { plain: 0x5F4A, debug: 0x7422 };

/// `Collision_GetType`.
pub const COLLISION_GET_TYPE: Pin = Pin { plain: 0x5AB0, debug: 0x6F88 };

/// `SolidityTable`.
pub const SOLIDITY_TABLE: Pin = Pin { plain: 0x6EAF8, debug: 0x754F6 };

/// `AngleTable`.
pub const ANGLE_TABLE: Pin = Pin { plain: 0x6E9F8, debug: 0x753F6 };

/// `HeightMaps`.
pub const HEIGHT_MAPS: Pin = Pin { plain: 0x6C9F8, debug: 0x733F6 };

/// `HeightMapsRot`.
pub const HEIGHT_MAPS_ROT: Pin = Pin { plain: 0x6D9F8, debug: 0x743F6 };

/// `Character_ID`.
pub const CHARACTER_ID: Pin = Pin { plain: 0xFFFFBD0C, debug: 0xFFFFEDB6 };

/// `Player_Chardef`.
pub const PLAYER_CHARDEF: Pin = Pin { plain: 0xFFFFBD0E, debug: 0xFFFFEDB8 };

/// `Ability_None`.
pub const ABILITY_NONE: Pin = Pin { plain: 0x1235E, debug: 0x124D6 };

/// `CharacterDefs`.
pub const CHARACTER_DEFS: Pin = Pin { plain: 0x12316, debug: 0x12428 };

/// `Player_InitAssets`.
pub const PLAYER_INIT_ASSETS: Pin = Pin { plain: 0x12322, debug: 0x12434 };

/// `Player_LoadArt`.
pub const PLAYER_LOAD_ART: Pin = Pin { plain: 0x1233A, debug: 0x1244C };

/// `Player_Ability`.
pub const PLAYER_ABILITY: Pin = Pin { plain: 0x12354, debug: 0x12466 };

/// `PhysTable_Sonic`.
pub const PHYS_TABLE_SONIC: Pin = Pin { plain: 0x1229A, debug: 0x123AC };

/// `Pal_SonicTails`.
pub const PAL_SONIC_TAILS: Pin = Pin { plain: 0x6C9B8, debug: 0x733B6 };

/// `OJZ_TestRaster`.
pub const OJZ_TEST_RASTER: Pin = Pin { plain: 0x145DA, debug: 0x14E18 };

/// `OJZ_TestPal`.
pub const OJZ_TEST_PAL: Pin = Pin { plain: 0x1465A, debug: 0x14F98 };

/// `OJZ_TestGradient`.
pub const OJZ_TEST_GRADIENT: Pin = Pin { plain: 0x14982, debug: 0x152C0 };

/// `OJZ_ShimmerCycle`.
pub const OJZ_SHIMMER_CYCLE: Pin = Pin { plain: 0x146BA, debug: 0x14FF8 };

/// `OJZ_TestVsram`.
pub const OJZ_TEST_VSRAM: Pin = Pin { plain: 0x14A02, debug: 0x15340 };

/// `OJZ_TestRamp`.
pub const OJZ_TEST_RAMP: Pin = Pin { plain: 0x14A82, debug: 0x153C0 };

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
pub const RASTER_V_BLANK: Pin = Pin { plain: 0x749E, debug: 0x8EA2 };

/// `Palette_Compose`.
pub const PALETTE_COMPOSE: Pin = Pin { plain: 0x7942, debug: 0x9346 };

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
pub const DUST_SPINDASH_SPAWN: Pin = Pin { plain: 0x1251A, debug: 0x126EA };

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
