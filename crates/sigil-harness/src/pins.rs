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
//! [provenance] 56 regions, 313 symbols, 7 offsets

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
pub const ASSEMBLED_LEN: usize = 0x5DB60;
/// Assembled (pre-convsym) ROM length, `__DEBUG__` shape. tests: m1d_rom, m1d_debug_rom, mixed_dac_rom
pub const DEBUG_ASSEMBLED_LEN: usize = 0x5F65A;

// ── Regions (manifest order) ──

/// `Vectors` .. start + 0x100 plain / 0x100 debug (literal — no end symbol) — gate `SIGIL_EMP_VECTORS`. tests: vectors_port
pub const VECTORS: Region = Region { plain_base: 0x0, debug_base: 0x0, plain_len: 0x100, debug_len: 0x100 };

/// `EntryPoint` .. `BootData` — gate `SIGIL_EMP_BOOT`. tests: boot_port
pub const BOOT: Region = Region { plain_base: 0x200, debug_base: 0x200, plain_len: 0x1A8, debug_len: 0x1AC };

/// `VDP_Shadow_Init` .. `Init_DMA_Queue` — gate `SIGIL_EMP_VDP_INIT`. tests: vdp_init_port
pub const VDP_INIT: Region = Region { plain_base: 0x1C08, debug_base: 0x1C8A, plain_len: 0x4A, debug_len: 0x4A };

/// `Init_DMA_Queue` .. `Init_SpriteTable` — gate `SIGIL_EMP_DMA_QUEUE`. tests: dma_queue_port
pub const DMA_QUEUE: Region = Region { plain_base: 0x1C52, debug_base: 0x1CD4, plain_len: 0x302, debug_len: 0x306 };

/// `Init_SpriteTable` .. `VBlank_Handler` — gate `SIGIL_EMP_BUFFERS`. tests: buffers_port
pub const BUFFERS: Region = Region { plain_base: 0x1F54, debug_base: 0x1FDA, plain_len: 0x258, debug_len: 0x258 };

/// `VBlank_Handler` .. `HBlank_Install` — gate `SIGIL_EMP_VBLANK`. tests: vblank_port
pub const VBLANK: Region = Region { plain_base: 0x21AC, debug_base: 0x2232, plain_len: 0x130, debug_len: 0x138 };

/// `HBlank_Install` .. `Read_Controllers` — gate `SIGIL_EMP_HBLANK`. tests: hblank_port, m1c_vector_table
pub const HBLANK: Region = Region { plain_base: 0x22DC, debug_base: 0x236A, plain_len: 0x48, debug_len: 0x48 };

/// `Read_Controllers` .. `GameLoop` — gate `SIGIL_EMP_CONTROLLERS`. tests: controllers_port
pub const CONTROLLERS: Region = Region { plain_base: 0x2324, debug_base: 0x23B2, plain_len: 0x76, debug_len: 0x76 };

/// `GameLoop` .. `S4LZ_DecompressDict` — gate `SIGIL_EMP_GAME_LOOP`. tests: game_loop_port, load_art_port
pub const GAME_LOOP: Region = Region { plain_base: 0x239A, debug_base: 0x2428, plain_len: 0x12, debug_len: 0x12 };

/// `S4LZ_DecompressDict` .. `ZX0_Decompress` — gate `SIGIL_EMP_S4LZ`. tests: s4lz_port
pub const S4LZ: Region = Region { plain_base: 0x23AC, debug_base: 0x243A, plain_len: 0xF8, debug_len: 0x200 };

/// `ZX0_Decompress` .. `GetSineCosine` — gate `SIGIL_EMP_ZX0`. tests: zx0_port
pub const ZX0: Region = Region { plain_base: 0x24A4, debug_base: 0x263A, plain_len: 0x58, debug_len: 0x58 };

/// `GetSineCosine` .. `Perform_DPLC` — gate `SIGIL_EMP_MATH`. tests: math_port
pub const MATH: Region = Region { plain_base: 0x24FC, debug_base: 0x2692, plain_len: 0x298, debug_len: 0x298 };

/// `Perform_DPLC` .. `InitObjectRAM` — gate `SIGIL_EMP_DPLC`. tests: dplc_port
pub const DPLC: Region = Region { plain_base: 0x2794, debug_base: 0x292A, plain_len: 0xA4, debug_len: 0xA4 };

/// `InitObjectRAM` .. `InitSpriteSystem` — gate `SIGIL_EMP_CORE`. tests: core_port
pub const CORE: Region = Region { plain_base: 0x2838, debug_base: 0x29CE, plain_len: 0x2E4, debug_len: 0x72C };

/// `InitSpriteSystem` .. `AnimateSprite` — gate `SIGIL_EMP_SPRITES`. tests: sprites_port
pub const SPRITES: Region = Region { plain_base: 0x2B1C, debug_base: 0x30FA, plain_len: 0x420, debug_len: 0x420 };

/// `AnimateSprite` .. `TouchResponse` — gate `SIGIL_EMP_ANIMATE`. tests: animate_port, test_objects_port
pub const ANIMATE: Region = Region { plain_base: 0x2F3C, debug_base: 0x351A, plain_len: 0x18A, debug_len: 0x2A8 };

/// `TouchResponse` .. `RingBuffer_Add` — gate `SIGIL_EMP_COLLISION`. tests: collision_port
pub const COLLISION: Region = Region { plain_base: 0x30C6, debug_base: 0x37C2, plain_len: 0x200, debug_len: 0x208 };

/// `RingBuffer_Add` .. `Collected_Init` — gate `SIGIL_EMP_RINGS`. tests: rings_port
pub const RINGS: Region = Region { plain_base: 0x32C6, debug_base: 0x39CA, plain_len: 0x1B8, debug_len: 0x214 };

/// `Collected_Init` .. `PopulateSpawnedPieceCount` — gate `SIGIL_EMP_ENTITY_WINDOW`. tests: entity_window_port
pub const ENTITY_WINDOW: Region = Region { plain_base: 0x347E, debug_base: 0x3BDE, plain_len: 0x8BA, debug_len: 0xD28 };

/// `PopulateSpawnedPieceCount` .. `Load_Object` — gate `SIGIL_EMP_CHILDREN`. tests: children_port
pub const CHILDREN: Region = Region { plain_base: 0x3D38, debug_base: 0x4906, plain_len: 0x2D0, debug_len: 0x3DA };

/// `Load_Object` .. `Plane_Buffer_Reset` — gate `SIGIL_EMP_LOAD_OBJECT`. tests: load_object_port, entity_window_port
pub const LOAD_OBJECT: Region = Region { plain_base: 0x4008, debug_base: 0x4CE0, plain_len: 0x82, debug_len: 0x82 };

/// `Plane_Buffer_Reset` .. `Tile_Cache_GetTile` — gate `SIGIL_EMP_PLANE_BUFFER`. tests: plane_buffer_port
pub const PLANE_BUFFER: Region = Region { plain_base: 0x408A, debug_base: 0x4D62, plain_len: 0x2DA, debug_len: 0x2DA };

/// `Tile_Cache_GetTile` .. `Collision_GetType` — gate `SIGIL_EMP_TILE_CACHE`. tests: tile_cache_port
pub const TILE_CACHE: Region = Region { plain_base: 0x4364, debug_base: 0x503C, plain_len: 0xD20, debug_len: 0xDE0 };

/// `Collision_GetType` .. `Collision_ProbeDown` — gate `SIGIL_EMP_COLLISION_LOOKUP`. tests: collision_lookup_port
pub const COLLISION_LOOKUP: Region = Region { plain_base: 0x5084, debug_base: 0x5E1C, plain_len: 0x24, debug_len: 0x24 };

/// `Section_Init` .. `Camera_Init` — gate `SIGIL_EMP_SECTION`. tests: section_port
pub const SECTION: Region = Region { plain_base: 0x55A4, debug_base: 0x633C, plain_len: 0x3DC, debug_len: 0x3DC };

/// `Camera_Init` .. `Parallax_Init` — gate `SIGIL_EMP_CAMERA`. tests: camera_port
pub const CAMERA: Region = Region { plain_base: 0x5980, debug_base: 0x6718, plain_len: 0x164, debug_len: 0x164 };

/// `Parallax_Init` .. `Art_Decompress` — gate `SIGIL_EMP_PARALLAX`. tests: parallax_port
pub const PARALLAX: Region = Region { plain_base: 0x5AE4, debug_base: 0x687C, plain_len: 0x576, debug_len: 0x576 };

/// `Art_Decompress` .. `BG_Init` — gate `SIGIL_EMP_LOAD_ART`. tests: load_art_port
pub const LOAD_ART: Region = Region { plain_base: 0x605A, debug_base: 0x6DF2, plain_len: 0x64, debug_len: 0xAE };

/// `BG_Init` .. `BgAnim_Init` — gate `SIGIL_EMP_BG`. tests: bg_port
pub const BG: Region = Region { plain_base: 0x60BE, debug_base: 0x6EA0, plain_len: 0xAE, debug_len: 0xAE };

/// `BgAnim_Init` .. start + 0x9E plain / 0x158 debug (literal — no end symbol) — gate `SIGIL_EMP_BG_ANIM`. tests: bg_anim_port
pub const BG_ANIM: Region = Region { plain_base: 0x616C, debug_base: 0x6F4E, plain_len: 0x9E, debug_len: 0x158 };

/// `CompressionSelfTest` .. `CSelf_S4LZ_Plain` (debug-only region; plain empty at `Sound_PostByte`) — gate `SIGIL_EMP_COMPRESSION_SELFTEST`. tests: compression_selftest_port
pub const COMPRESSION_SELFTEST: Region = Region { plain_base: 0x620A, debug_base: 0x70A6, plain_len: 0x0, debug_len: 0x218 };

/// `Sound_PostByte` .. start + 0x20A plain / 0x3B4 debug (literal — no end symbol) — gate `SIGIL_EMP_SOUND_API`. tests: sound_api_port
pub const SOUND_API: Region = Region { plain_base: 0x620A, debug_base: 0x7D30, plain_len: 0x20A, debug_len: 0x3B4 };

/// `TestSolid_Init` .. `TestParticle` — gate `SIGIL_EMP_TEST_OBJECTS`. tests: test_objects_port
pub const TEST_SOLID: Region = Region { plain_base: 0x10F4A, debug_base: 0x10F4A, plain_len: 0xE, debug_len: 0xE };

/// `TestParticle` .. `TestEmitter` — gate `SIGIL_EMP_TEST_OBJECTS`. tests: test_objects_port
pub const TEST_PARTICLE: Region = Region { plain_base: 0x10F58, debug_base: 0x10F58, plain_len: 0x52, debug_len: 0x52 };

/// `TestStatic_Main` .. `TestAnimated` — gate `SIGIL_EMP_TEST_STATIC`. tests: test_g1_objects_port
pub const TEST_STATIC: Region = Region { plain_base: 0x10C34, debug_base: 0x10C34, plain_len: 0x4, debug_len: 0x4 };

/// `TestAnimated` .. `TestPlayer` — gate `SIGIL_EMP_TEST_ANIMATED`. tests: test_g1_objects_port
pub const TEST_ANIMATED: Region = Region { plain_base: 0x10C38, debug_base: 0x10C38, plain_len: 0x5A, debug_len: 0x5A };

/// `TestEmitter` .. `TestChildPart` — gate `SIGIL_EMP_TEST_EMITTER`. tests: test_g2_objects_port
pub const TEST_EMITTER: Region = Region { plain_base: 0x10FAA, debug_base: 0x10FAA, plain_len: 0x54, debug_len: 0x54 };

/// `TestStressEmitter` .. `TestChurnObj` — gate `SIGIL_EMP_TEST_STRESS_EMITTER`. tests: test_g2_objects_port
pub const TEST_STRESS_EMITTER: Region = Region { plain_base: 0x11128, debug_base: 0x11128, plain_len: 0x5A, debug_len: 0x5A };

/// `TestChurnObj` .. `ObjDef_PathSwap` — gate `SIGIL_EMP_TEST_CHURN`. tests: test_g2_objects_port
pub const TEST_CHURN: Region = Region { plain_base: 0x11182, debug_base: 0x11182, plain_len: 0x78, debug_len: 0x78 };

/// `TestChildPart` .. `TestStressEmitter` — gate `SIGIL_EMP_TEST_PARENT`. tests: test_g3_objects_port
pub const TEST_PARENT: Region = Region { plain_base: 0x10FFE, debug_base: 0x10FFE, plain_len: 0x12A, debug_len: 0x12A };

/// `TestPlayer` .. `TestEnemy_Init` — gate `SIGIL_EMP_TEST_PLAYER`. tests: test_g4_final_objects_port
pub const TEST_PLAYER: Region = Region { plain_base: 0x10C92, debug_base: 0x10C92, plain_len: 0x270, debug_len: 0x270 };

/// `TestEnemy_Init` .. `TestSolid_Init` — gate `SIGIL_EMP_TEST_ENEMY`. tests: test_g4_final_objects_port
pub const TEST_ENEMY: Region = Region { plain_base: 0x10F02, debug_base: 0x10F02, plain_len: 0x48, debug_len: 0x48 };

/// `ObjDef_PathSwap` .. `DeformTable_Zero` — gate `SIGIL_EMP_PATH_SWAP`. tests: test_g4_final_objects_port
pub const PATH_SWAP: Region = Region { plain_base: 0x111FA, debug_base: 0x111FA, plain_len: 0x92, debug_len: 0xFA };

/// `Ani_Sonic` .. `Ani_Sonic_End` — gate `SIGIL_EMP_SONIC_ANIMS`. tests: sonic_anims_port
pub const SONIC_ANIMS: Region = Region { plain_base: 0x256DC, debug_base: 0x25744, plain_len: 0x6E, debug_len: 0x6E };

/// `Ani_Particle` .. `Ani_Particle_End` — gate `SIGIL_EMP_PARTICLE_ANIMS`. tests: particle_anims_port, test_objects_port
pub const PARTICLE_ANIMS: Region = Region { plain_base: 0x2574A, debug_base: 0x257B2, plain_len: 0x8, debug_len: 0x8 };

/// `OJZ_Act1_Descriptor` .. `OJZ_Sec0_Blocks` — gate `SIGIL_EMP_ACT_DESCRIPTOR`. tests: act_descriptor_port
pub const ACT_DESCRIPTOR: Region = Region { plain_base: 0x14B2A, debug_base: 0x14B92, plain_len: 0x274, debug_len: 0x274 };

/// `ObjDef_Static` .. `OJZ_Sec0_TypeTable` — gate `SIGIL_EMP_OBJDEFS`. tests: objdef_port
pub const OBJDEFS: Region = Region { plain_base: 0x11D16, debug_base: 0x11D7E, plain_len: 0x68, debug_len: 0x68 };

/// `GameState_ObjectTest_Init` .. `GameState_OJZScroll_Init` — gate `SIGIL_EMP_OBJECT_TEST_STATE`. tests: test_t1_harness_states_port
pub const OBJECT_TEST_STATE: Region = Region { plain_base: 0x5C230, debug_base: 0x5DC82, plain_len: 0x5BC, debug_len: 0x658 };

/// `GameState_OJZScroll_Init` .. `NullInterrupt` — gate `SIGIL_EMP_OJZ_SCROLL_TEST`. tests: test_t1_harness_states_port
pub const OJZ_SCROLL_TEST: Region = Region { plain_base: 0x5C7EC, debug_base: 0x5E2DA, plain_len: 0x2C2, debug_len: 0x2CE };

/// `BusError` .. `EndOfRom` — gate `SIGIL_EMP_ERROR_HANDLER`. tests: error_handler_port
pub const ERROR_HANDLER: Region = Region { plain_base: 0x5CAB0, debug_base: 0x5E5AA, plain_len: 0x10B0, debug_len: 0x10B0 };

/// `Player_Init` .. `PState_Ground` — gate `SIGIL_EMP_PLAYER_COMMON`. tests: test_p1_player_port
pub const PLAYER_COMMON: Region = Region { plain_base: 0x10002, debug_base: 0x10002, plain_len: 0x446, debug_len: 0x446 };

/// `Sonic_InitAssets` .. `TestStatic_Main` — gate `SIGIL_EMP_SONIC`. tests: test_p1_player_port
pub const SONIC: Region = Region { plain_base: 0x10BF4, debug_base: 0x10BF4, plain_len: 0x40, debug_len: 0x40 };

/// `PState_Ground` .. `PState_Air` — gate `SIGIL_EMP_PLAYER_GROUND`. tests: test_p2_player_states_port
pub const PLAYER_GROUND: Region = Region { plain_base: 0x10448, debug_base: 0x10448, plain_len: 0x44E, debug_len: 0x44E };

/// `PState_Air` .. `PState_Spindash` — gate `SIGIL_EMP_PLAYER_AIR`. tests: test_p2_player_states_port
pub const PLAYER_AIR: Region = Region { plain_base: 0x10896, debug_base: 0x10896, plain_len: 0x2C2, debug_len: 0x2C2 };

/// `PState_Spindash` .. `Sonic_InitAssets` — gate `SIGIL_EMP_PLAYER_SPINDASH`. tests: test_p2_player_states_port
pub const PLAYER_SPINDASH: Region = Region { plain_base: 0x10B58, debug_base: 0x10B58, plain_len: 0x9C, debug_len: 0x9C };

/// `Collision_ProbeDown` .. `Section_Init` — gate `SIGIL_EMP_PLAYER_SENSORS`. tests: test_p4_player_sensors_port
pub const PLAYER_SENSORS: Region = Region { plain_base: 0x50A8, debug_base: 0x5E40, plain_len: 0x4FC, debug_len: 0x4FC };

// ── Symbols (manifest order) ──

/// `TestStatic_Main`. tests: objdef_port
pub const TEST_STATIC_MAIN: Pin = Pin { plain: 0x10C34, debug: 0x10C34 };

/// `TestSolid_Init`. tests: objdef_port
pub const TEST_SOLID_INIT: Pin = Pin { plain: 0x10F4A, debug: 0x10F4A };

/// `TestEnemy_Init`. tests: objdef_port
pub const TEST_ENEMY_INIT: Pin = Pin { plain: 0x10F02, debug: 0x10F02 };

/// `TestParent`. tests: objdef_port
pub const TEST_PARENT_LABEL: Pin = Pin { plain: 0x11082, debug: 0x11082 };

/// `Map_TestObj`. tests: objdef_port
pub const MAP_TEST_OBJ: Pin = Pin { plain: 0x256AC, debug: 0x25714 };

/// `Map_Sonic`. tests: test_g1_objects_port
pub const MAP_SONIC: Pin = Pin { plain: 0x27952, debug: 0x279BA };

/// `DPLC_Sonic`. tests: test_g1_objects_port
pub const DPLC_SONIC: Pin = Pin { plain: 0x295D2, debug: 0x2963A };

/// `Art_Sonic`. tests: test_g1_objects_port
pub const ART_SONIC: Pin = Pin { plain: 0x29F12, debug: 0x29F7A };

/// `CreateEffect_Normal`. tests: test_g2_objects_port
pub const CREATE_EFFECT_NORMAL: Pin = Pin { plain: 0x3F72, debug: 0x4C4A };

/// `CreateChild_Normal`. tests: test_g3_objects_port
pub const CREATE_CHILD_NORMAL: Pin = Pin { plain: 0x3D5A, debug: 0x4928 };

/// `DeleteChildren`. tests: test_g3_objects_port
pub const DELETE_CHILDREN: Pin = Pin { plain: 0x3F54, debug: 0x4C2C };

/// `GetSineCosine`. tests: test_g3_objects_port
pub const GET_SINE_COSINE: Pin = Pin { plain: 0x24FC, debug: 0x2692 };

/// `EntryPoint`. tests: m1c_vector_table
pub const ENTRY_POINT: Pin = Pin { plain: 0x200, debug: 0x200 };

/// `NullInterrupt`. tests: m1c_vector_table
pub const NULL_INTERRUPT: Pin = Pin { plain: 0x5CAAE, debug: 0x5E5A8 };

/// `BusError`. tests: m1c_vector_table
pub const BUS_ERROR: Pin = Pin { plain: 0x5CAB0, debug: 0x5E5AA };

/// `AddressError`. tests: m1c_vector_table
pub const ADDRESS_ERROR: Pin = Pin { plain: 0x5CAC8, debug: 0x5E5C2 };

/// `IllegalInstr`. tests: m1c_vector_table
pub const ILLEGAL_INSTR: Pin = Pin { plain: 0x5CAE4, debug: 0x5E5DE };

/// `ZeroDivide`. tests: m1c_vector_table
pub const ZERO_DIVIDE: Pin = Pin { plain: 0x5CB06, debug: 0x5E600 };

/// `ChkInstr`. tests: m1c_vector_table
pub const CHK_INSTR: Pin = Pin { plain: 0x5CB20, debug: 0x5E61A };

/// `TrapvInstr`. tests: m1c_vector_table
pub const TRAPV_INSTR: Pin = Pin { plain: 0x5CB3E, debug: 0x5E638 };

/// `PrivilegeViol`. tests: m1c_vector_table
pub const PRIVILEGE_VIOL: Pin = Pin { plain: 0x5CB5E, debug: 0x5E658 };

/// `Trace`. tests: m1c_vector_table
pub const TRACE: Pin = Pin { plain: 0x5CB80, debug: 0x5E67A };

/// `Line1010Emu`. tests: m1c_vector_table
pub const LINE1010_EMU: Pin = Pin { plain: 0x5CB94, debug: 0x5E68E };

/// `Line1111Emu`. tests: m1c_vector_table
pub const LINE1111_EMU: Pin = Pin { plain: 0x5CBB4, debug: 0x5E6AE };

/// `ErrorExcept`. tests: m1c_vector_table
pub const ERROR_EXCEPT: Pin = Pin { plain: 0x5CBD4, debug: 0x5E6CE };

/// `ErrorTrap`. tests: m1c_vector_table
pub const ERROR_TRAP: Pin = Pin { plain: 0x5CBF2, debug: 0x5E6EC };

/// `VBlank_Handler`. tests: m1c_vector_table
pub const V_BLANK_HANDLER: Pin = Pin { plain: 0x21AC, debug: 0x2232 };

/// `HBlank_Vector_Slot`. tests: hblank_port, m1c_vector_table
pub const H_BLANK_VECTOR_SLOT: Pin = Pin { plain: 0xFFFFB074, debug: 0xFFFFB098 };

/// `VDP_Shadow_Table`. tests: vdp_init_port
pub const VDP_SHADOW_TABLE: Pin = Pin { plain: 0xFFFF800A, debug: 0xFFFF800A };

/// `VDP_Dirty_Mask`. tests: vdp_init_port
pub const VDP_DIRTY_MASK: Pin = Pin { plain: 0xFFFF801E, debug: 0xFFFF801E };

/// `BootData_VDPRegs`. tests: vdp_init_port
pub const BOOT_DATA_VDP_REGS: Pin = Pin { plain: 0x3C2, debug: 0x3C6 };

/// `Ctrl_1_Held`. tests: controllers_port
pub const CTRL_1_HELD: Pin = Pin { plain: 0xFFFF802C, debug: 0xFFFF802C };

/// `VSync_Wait`. tests: game_loop_port, load_art_port
pub const V_SYNC_WAIT: Pin = Pin { plain: 0x22BC, debug: 0x2346 };

/// `Sound_DrainSfxRing`. tests: game_loop_port, load_art_port
pub const SOUND_DRAIN_SFX_RING: Pin = Pin { plain: 0x6376, debug: 0x8046 };

/// `Game_State`. tests: game_loop_port, load_art_port
pub const GAME_STATE: Pin = Pin { plain: 0xFFFF8004, debug: 0xFFFF8004 };

/// `Cache_Left_Col`. tests: collision_lookup_port, section_port
pub const CACHE_LEFT_COL: Pin = Pin { plain: 0xFFFFA836, debug: 0xFFFFA85A };

/// `Tile_Cache_GetCollision`. tests: collision_lookup_port
pub const TILE_CACHE_GET_COLLISION: Pin = Pin { plain: 0x43A0, debug: 0x5078 };

/// `Draw_TileColumn`. tests: section_port
pub const DRAW_TILE_COLUMN: Pin = Pin { plain: 0x4092, debug: 0x4D6A };

/// `Draw_TileRow_FromCache`. tests: section_port
pub const DRAW_TILE_ROW_FROM_CACHE: Pin = Pin { plain: 0x41B4, debug: 0x4E8C };

/// `EntityWindow_Init`. tests: section_port
pub const ENTITY_WINDOW_INIT: Pin = Pin { plain: 0x3834, debug: 0x4312 };

/// `Section_Plane_Dirty`. tests: section_port
pub const SECTION_PLANE_DIRTY: Pin = Pin { plain: 0xFFFFA8A6, debug: 0xFFFFA8CA };

/// `Section_Right_Col_Written`. tests: section_port
pub const SECTION_RIGHT_COL_WRITTEN: Pin = Pin { plain: 0xFFFFA8A8, debug: 0xFFFFA8CC };

/// `Section_Left_Col_Written`. tests: section_port
pub const SECTION_LEFT_COL_WRITTEN: Pin = Pin { plain: 0xFFFFA8AA, debug: 0xFFFFA8CE };

/// `Section_Top_Row_Written`. tests: section_port
pub const SECTION_TOP_ROW_WRITTEN: Pin = Pin { plain: 0xFFFFA8A2, debug: 0xFFFFA8C6 };

/// `Section_Bottom_Row_Written`. tests: section_port
pub const SECTION_BOTTOM_ROW_WRITTEN: Pin = Pin { plain: 0xFFFFA8A4, debug: 0xFFFFA8C8 };

/// `Cache_Head_Col`. tests: section_port
pub const CACHE_HEAD_COL: Pin = Pin { plain: 0xFFFFA838, debug: 0xFFFFA85C };

/// `Cache_Top_Row`. tests: section_port
pub const CACHE_TOP_ROW: Pin = Pin { plain: 0xFFFFA83A, debug: 0xFFFFA85E };

/// `Cache_Bottom_Row`. tests: section_port
pub const CACHE_BOTTOM_ROW: Pin = Pin { plain: 0xFFFFA83C, debug: 0xFFFFA860 };

/// `Cache_Origin_Col`. tests: section_port
pub const CACHE_ORIGIN_COL: Pin = Pin { plain: 0xFFFFA83E, debug: 0xFFFFA862 };

/// `Cache_Origin_Row`. tests: section_port
pub const CACHE_ORIGIN_ROW: Pin = Pin { plain: 0xFFFFA840, debug: 0xFFFFA864 };

/// `Plane_Buffer_Ptr`. tests: section_port
pub const PLANE_BUFFER_PTR: Pin = Pin { plain: 0xFFFFA72A, debug: 0xFFFFA74E };

/// `Plane_Buffer`. tests: plane_buffer_port
pub const PLANE_BUFFER_BASE: Pin = Pin { plain: 0xFFFFA12A, debug: 0xFFFFA14E };

/// `Tile_Cache_Nametable`. tests: section_port
pub const TILE_CACHE_NAMETABLE: Pin = Pin { plain: 0xFFFF0000, debug: 0xFFFF0000 };

/// `Tile_Cache_Collision`. tests: tile_cache_port
pub const TILE_CACHE_COLLISION: Pin = Pin { plain: 0xFFFF2580, debug: 0xFFFF2580 };

/// `Frame_Counter`. tests: tile_cache_port
pub const FRAME_COUNTER: Pin = Pin { plain: 0xFFFF8002, debug: 0xFFFF8002 };

/// `Block_Stage_Keys`. tests: tile_cache_port
pub const BLOCK_STAGE_KEYS: Pin = Pin { plain: 0xFFFFA860, debug: 0xFFFFA884 };

/// `Block_Stage_Next`. tests: tile_cache_port
pub const BLOCK_STAGE_NEXT: Pin = Pin { plain: 0xFFFFA8A0, debug: 0xFFFFA8C4 };

/// `Block_Stage_Buffers`. tests: tile_cache_port
pub const BLOCK_STAGE_BUFFERS: Pin = Pin { plain: 0xFFFF3842, debug: 0xFFFF3842 };

/// `Cache_Fill_Last_Frame`. tests: tile_cache_port
pub const CACHE_FILL_LAST_FRAME: Pin = Pin { plain: 0xFFFFA842, debug: 0xFFFFA866 };

/// `Cache_Fill_Budget`. tests: tile_cache_port
pub const CACHE_FILL_BUDGET: Pin = Pin { plain: 0xFFFFA848, debug: 0xFFFFA86C };

/// `Cache_Fill_Resume_Col`. tests: tile_cache_port
pub const CACHE_FILL_RESUME_COL: Pin = Pin { plain: 0xFFFFA844, debug: 0xFFFFA868 };

/// `Cache_Fill_Resume_Row`. tests: tile_cache_port
pub const CACHE_FILL_RESUME_ROW: Pin = Pin { plain: 0xFFFFA846, debug: 0xFFFFA86A };

/// `Cache_Fill_RowResume_Row`. tests: tile_cache_port
pub const CACHE_FILL_ROW_RESUME_ROW: Pin = Pin { plain: 0xFFFFA84A, debug: 0xFFFFA86E };

/// `Cache_Fill_RowResume_Col`. tests: tile_cache_port
pub const CACHE_FILL_ROW_RESUME_COL: Pin = Pin { plain: 0xFFFFA84C, debug: 0xFFFFA870 };

/// `Cache_Fill_Rows_Left`. tests: tile_cache_port
pub const CACHE_FILL_ROWS_LEFT: Pin = Pin { plain: 0xFFFFA84E, debug: 0xFFFFA872 };

/// `Cache_Prev_Cam_Row`. tests: tile_cache_port
pub const CACHE_PREV_CAM_ROW: Pin = Pin { plain: 0xFFFFA850, debug: 0xFFFFA874 };

/// `Cache_Prev_Cam_X`. tests: tile_cache_port
pub const CACHE_PREV_CAM_X: Pin = Pin { plain: 0xFFFFA852, debug: 0xFFFFA876 };

/// `Cache_H_Pfx_Dir`. tests: tile_cache_port
pub const CACHE_H_PFX_DIR: Pin = Pin { plain: 0xFFFFA854, debug: 0xFFFFA878 };

/// `Cache_H_Pfx_Accum`. tests: tile_cache_port
pub const CACHE_H_PFX_ACCUM: Pin = Pin { plain: 0xFFFFA856, debug: 0xFFFFA87A };

/// `Cache_Pfx_Row_Target`. tests: tile_cache_port
pub const CACHE_PFX_ROW_TARGET: Pin = Pin { plain: 0xFFFFA858, debug: 0xFFFFA87C };

/// `Cache_Pfx_Col_Target`. tests: tile_cache_port
pub const CACHE_PFX_COL_TARGET: Pin = Pin { plain: 0xFFFFA85A, debug: 0xFFFFA87E };

/// `Cache_Pfx_Skip_Armed`. tests: tile_cache_port
pub const CACHE_PFX_SKIP_ARMED: Pin = Pin { plain: 0xFFFFA85C, debug: 0xFFFFA880 };

/// `Cache_Pfx_Lag_Flag`. tests: tile_cache_port
pub const CACHE_PFX_LAG_FLAG: Pin = Pin { plain: 0xFFFFA85E, debug: 0xFFFFA882 };

/// `Block_Stage_Gen`. tests: tile_cache_port
pub const BLOCK_STAGE_GEN: Pin = Pin { plain: 0xFFFFB062, debug: 0xFFFFB086 };

/// `Pfx_Memo_Row`. tests: tile_cache_port
pub const PFX_MEMO_ROW: Pin = Pin { plain: 0xFFFFB064, debug: 0xFFFFB088 };

/// `Pfx_Memo_L`. tests: tile_cache_port
pub const PFX_MEMO_L: Pin = Pin { plain: 0xFFFFB066, debug: 0xFFFFB08A };

/// `Pfx_Memo_H`. tests: tile_cache_port
pub const PFX_MEMO_H: Pin = Pin { plain: 0xFFFFB068, debug: 0xFFFFB08C };

/// `Pfx_Memo_Gen`. tests: tile_cache_port
pub const PFX_MEMO_GEN: Pin = Pin { plain: 0xFFFFB06A, debug: 0xFFFFB08E };

/// `Cs_Memo_Col`. tests: tile_cache_port
pub const CS_MEMO_COL: Pin = Pin { plain: 0xFFFFB06C, debug: 0xFFFFB090 };

/// `Cs_Memo_T`. tests: tile_cache_port
pub const CS_MEMO_T: Pin = Pin { plain: 0xFFFFB06E, debug: 0xFFFFB092 };

/// `Cs_Memo_B`. tests: tile_cache_port
pub const CS_MEMO_B: Pin = Pin { plain: 0xFFFFB070, debug: 0xFFFFB094 };

/// `Cs_Memo_Gen`. tests: tile_cache_port
pub const CS_MEMO_GEN: Pin = Pin { plain: 0xFFFFB072, debug: 0xFFFFB096 };

/// `S4LZ_DecompressDict`. tests: tile_cache_port
pub const S4_LZ_DECOMPRESS_DICT: Pin = Pin { plain: 0x23AC, debug: 0x243A };

/// `Player_1`. tests: collision_port, rings_port
pub const PLAYER_1: Pin = Pin { plain: 0xFFFF89EE, debug: 0xFFFF8A12 };

/// `Dynamic_Slots`. tests: collision_port
pub const DYNAMIC_SLOTS: Pin = Pin { plain: 0xFFFF8A8E, debug: 0xFFFF8AB2 };

/// `Ring_Buffer`. tests: rings_port
pub const RING_BUFFER: Pin = Pin { plain: 0xFFFFA914, debug: 0xFFFFA938 };

/// `Ring_Count`. tests: rings_port
pub const RING_COUNT: Pin = Pin { plain: 0xFFFFAC14, debug: 0xFFFFAC38 };

/// `Ring_HighWater`. tests: rings_port
pub const RING_HIGH_WATER: Pin = Pin { plain: 0xFFFFAC15, debug: 0xFFFFAC39 };

/// `Ring_Add_Dropped`. tests: rings_port
pub const RING_ADD_DROPPED: Pin = Pin { plain: 0xFFFFAC16, debug: 0xFFFFAC3A };

/// `Ring_Counter`. tests: rings_port
pub const RING_COUNTER: Pin = Pin { plain: 0xFFFFAC70, debug: 0xFFFFAC94 };

/// `Ring_Anim_Frame`. tests: rings_port
pub const RING_ANIM_FRAME: Pin = Pin { plain: 0xFFFFAC72, debug: 0xFFFFAC96 };

/// `Ring_Anim_Timer`. tests: rings_port
pub const RING_ANIM_TIMER: Pin = Pin { plain: 0xFFFFAC73, debug: 0xFFFFAC97 };

/// `Camera_X`. tests: rings_port, section_port, camera_port, bg_anim_port
pub const CAMERA_X: Pin = Pin { plain: 0xFFFFA11C, debug: 0xFFFFA140 };

/// `Camera_Y`. tests: rings_port, section_port, camera_port, bg_anim_port
pub const CAMERA_Y: Pin = Pin { plain: 0xFFFFA120, debug: 0xFFFFA144 };

/// `Camera_Deadzone_Base`. tests: camera_port
pub const CAMERA_DEADZONE_BASE: Pin = Pin { plain: 0xFFFFA82E, debug: 0xFFFFA852 };

/// `Camera_Pan_Offset`. tests: camera_port
pub const CAMERA_PAN_OFFSET: Pin = Pin { plain: 0xFFFFA832, debug: 0xFFFFA856 };

/// `Camera_Hold_Frames`. tests: camera_port
pub const CAMERA_HOLD_FRAMES: Pin = Pin { plain: 0xFFFFA834, debug: 0xFFFFA858 };

/// `BgAnim_LastStep`. tests: bg_anim_port
pub const BG_ANIM_LAST_STEP: Pin = Pin { plain: 0xFFFF8984, debug: 0xFFFF8984 };

/// `BgAnim_Table`. tests: bg_anim_port
pub const BG_ANIM_TABLE: Pin = Pin { plain: 0x256AA, debug: 0x25712 };

/// `Camera_X_Biased`. tests: sprites_port
pub const CAMERA_X_BIASED: Pin = Pin { plain: 0xFFFFA124, debug: 0xFFFFA148 };

/// `Camera_Y_Biased`. tests: sprites_port
pub const CAMERA_Y_BIASED: Pin = Pin { plain: 0xFFFFA126, debug: 0xFFFFA14A };

/// `Collected_MarkRing`. tests: rings_port
pub const COLLECTED_MARK_RING: Pin = Pin { plain: 0x3500, debug: 0x3CC2 };

/// `EntityWindow_EntryForSection`. tests: rings_port
pub const ENTITY_WINDOW_ENTRY_FOR_SECTION: Pin = Pin { plain: 0x371C, debug: 0x41A4 };

/// `EntityLoaded_Clear`. tests: rings_port
pub const ENTITY_LOADED_CLEAR: Pin = Pin { plain: 0x3708, debug: 0x412E };

/// `Sound_PlayRing`. tests: rings_port
pub const SOUND_PLAY_RING: Pin = Pin { plain: 0x63C6, debug: 0x8096 };

/// `CSelf_S4LZ_Plain` — debug-shape consumer only (`debug_only`). tests: compression_selftest_port
pub const C_SELF_S4_LZ_PLAIN: u32 = 0x72BE;

/// `CSelf_S4LZ_Dict` — debug-shape consumer only (`debug_only`). tests: compression_selftest_port
pub const C_SELF_S4_LZ_DICT: u32 = 0x7540;

/// `CSelf_Dict_Blob` — debug-shape consumer only (`debug_only`). tests: compression_selftest_port
pub const C_SELF_DICT_BLOB: u32 = 0x7726;

/// `CSelf_ZX0` — debug-shape consumer only (`debug_only`). tests: compression_selftest_port
pub const C_SELF_ZX0: u32 = 0x7826;

/// `CSelf_Expected` — debug-shape consumer only (`debug_only`). tests: compression_selftest_port
pub const C_SELF_EXPECTED: u32 = 0x7A48;

/// `MDDBG__ErrorHandler` — debug-shape consumer only (`debug_only`). tests: rings_port
pub const MDDBG_ERROR_HANDLER: u32 = 0x5E704;

/// `MDDBG__ErrorHandler_PagesController` — debug-shape consumer only (`debug_only`). tests: rings_port
pub const MDDBG_ERROR_HANDLER_PAGES_CONTROLLER: u32 = 0x5F4CA;

/// `DMA_Critical`. tests: dma_queue_port
pub const DMA_CRITICAL: Pin = Pin { plain: 0xFFFF803C, debug: 0xFFFF803C };

/// `DMA_Critical_End`. tests: dma_queue_port
pub const DMA_CRITICAL_END: Pin = Pin { plain: 0xFFFF80AC, debug: 0xFFFF80AC };

/// `DMA_Important`. tests: dma_queue_port
pub const DMA_IMPORTANT: Pin = Pin { plain: 0xFFFF80AC, debug: 0xFFFF80AC };

/// `DMA_Important_End`. tests: dma_queue_port
pub const DMA_IMPORTANT_END: Pin = Pin { plain: 0xFFFF8154, debug: 0xFFFF8154 };

/// `DMA_Deferrable`. tests: dma_queue_port
pub const DMA_DEFERRABLE: Pin = Pin { plain: 0xFFFF8154, debug: 0xFFFF8154 };

/// `DMA_Deferrable_End`. tests: dma_queue_port
pub const DMA_DEFERRABLE_END: Pin = Pin { plain: 0xFFFF81FC, debug: 0xFFFF81FC };

/// `DMA_Critical_Slot`. tests: dma_queue_port
pub const DMA_CRITICAL_SLOT: Pin = Pin { plain: 0xFFFF81FC, debug: 0xFFFF81FC };

/// `DMA_Important_Slot`. tests: dma_queue_port
pub const DMA_IMPORTANT_SLOT: Pin = Pin { plain: 0xFFFF81FE, debug: 0xFFFF81FE };

/// `DMA_Deferrable_Slot`. tests: dma_queue_port
pub const DMA_DEFERRABLE_SLOT: Pin = Pin { plain: 0xFFFF8200, debug: 0xFFFF8200 };

/// `DMA_Budget_Remaining`. tests: dma_queue_port
pub const DMA_BUDGET_REMAINING: Pin = Pin { plain: 0xFFFF8204, debug: 0xFFFF8204 };

/// `DMA_Overflow_Count` — debug-shape consumer only (`debug_only`). tests: dma_queue_port
pub const DMA_OVERFLOW_COUNT: u32 = 0xFFFF89F6;

/// `Art_Staging_Buffer`. tests: load_art_port
pub const ART_STAGING_BUFFER: Pin = Pin { plain: 0xFFFF0000, debug: 0xFFFF0000 };

/// `S4LZ_Decompress`. tests: load_art_port
pub const S4_LZ_DECOMPRESS: Pin = Pin { plain: 0x23B0, debug: 0x2492 };

/// `ZX0_Decompress`. tests: load_art_port
pub const ZX0_DECOMPRESS: Pin = Pin { plain: 0x24A4, debug: 0x263A };

/// `QueueDMA_Critical`. tests: load_art_port
pub const QUEUE_DMA_CRITICAL: Pin = Pin { plain: 0x1D70, debug: 0x1DF2 };

/// `BG_Init`. tests: load_art_port
pub const BG_INIT: Pin = Pin { plain: 0x60BE, debug: 0x6EA0 };

/// `QueueDMA_Important`. tests: dplc_port
pub const QUEUE_DMA_IMPORTANT: Pin = Pin { plain: 0x1D7A, debug: 0x1DFC };

/// `QueueDMA_Deferrable`. tests: dplc_port
pub const QUEUE_DMA_DEFERRABLE: Pin = Pin { plain: 0x1D84, debug: 0x1E06 };

/// `Object_RAM`. tests: core_port
pub const OBJECT_RAM: Pin = Pin { plain: 0xFFFF89EE, debug: 0xFFFF8A12 };

/// `System_Slots`. tests: core_port
pub const SYSTEM_SLOTS: Pin = Pin { plain: 0xFFFF970E, debug: 0xFFFF9732 };

/// `Effect_Slots`. tests: core_port
pub const EFFECT_SLOTS: Pin = Pin { plain: 0xFFFF998E, debug: 0xFFFF99B2 };

/// `Game_Paused`. tests: core_port
pub const GAME_PAUSED: Pin = Pin { plain: 0xFFFFA128, debug: 0xFFFFA14C };

/// `Object_RAM_End`. tests: core_port
pub const OBJECT_RAM_END: Pin = Pin { plain: 0xFFFF9E8E, debug: 0xFFFF9EB2 };

/// `Dynamic_Free_Stack`. tests: core_port
pub const DYNAMIC_FREE_STACK: Pin = Pin { plain: 0xFFFF9E8E, debug: 0xFFFF9EB2 };

/// `Dynamic_Free_SP`. tests: core_port
pub const DYNAMIC_FREE_SP: Pin = Pin { plain: 0xFFFF9EDE, debug: 0xFFFF9F02 };

/// `Effect_Free_Stack`. tests: core_port
pub const EFFECT_FREE_STACK: Pin = Pin { plain: 0xFFFF9EE0, debug: 0xFFFF9F04 };

/// `Effect_Free_SP`. tests: core_port
pub const EFFECT_FREE_SP: Pin = Pin { plain: 0xFFFF9F00, debug: 0xFFFF9F24 };

/// `Dynamic_Live`. tests: core_port
pub const DYNAMIC_LIVE: Pin = Pin { plain: 0xFFFFAFFC, debug: 0xFFFFB020 };

/// `Dynamic_Live_Count`. tests: core_port
pub const DYNAMIC_LIVE_COUNT: Pin = Pin { plain: 0xFFFFB04C, debug: 0xFFFFB070 };

/// `Dynamic_Live_Dirty`. tests: core_port
pub const DYNAMIC_LIVE_DIRTY: Pin = Pin { plain: 0xFFFFB04E, debug: 0xFFFFB072 };

/// `Dynamic_Live_Walking` — debug-shape consumer only (`debug_only`). tests: core_port, collision_port, entity_window_port
pub const DYNAMIC_LIVE_WALKING: u32 = 0xFFFFB073;

/// `Dynamic_Live_Pending`. tests: core_port
pub const DYNAMIC_LIVE_PENDING: Pin = Pin { plain: 0xFFFFB050, debug: 0xFFFFB074 };

/// `Dynamic_Live_Pending_Count`. tests: core_port
pub const DYNAMIC_LIVE_PENDING_COUNT: Pin = Pin { plain: 0xFFFFB060, debug: 0xFFFFB084 };

/// `DeleteObject`. tests: animate_port, children_port
pub const DELETE_OBJECT: Pin = Pin { plain: 0x2908, debug: 0x2A9E };

/// `DrawRings`. tests: sprites_port
pub const DRAW_RINGS: Pin = Pin { plain: 0x334C, debug: 0x3AAC };

/// `Sprite_Table_Buffer`. tests: sprites_port
pub const SPRITE_TABLE_BUFFER: Pin = Pin { plain: 0xFFFF8288, debug: 0xFFFF8288 };

/// `Sprite_Table_Dirty`. tests: sprites_port
pub const SPRITE_TABLE_DIRTY: Pin = Pin { plain: 0xFFFF8508, debug: 0xFFFF8508 };

/// `Sprite_Bands`. tests: sprites_port
pub const SPRITE_BANDS: Pin = Pin { plain: 0xFFFF9F02, debug: 0xFFFF9F26 };

/// `Sprite_Band_Counts`. tests: sprites_port
pub const SPRITE_BAND_COUNTS: Pin = Pin { plain: 0xFFFFA102, debug: 0xFFFFA126 };

/// `Sprites_Rendered`. tests: sprites_port
pub const SPRITES_RENDERED: Pin = Pin { plain: 0xFFFFA10A, debug: 0xFFFFA12E };

/// `Sprite_Cycle_Counter`. tests: sprites_port
pub const SPRITE_CYCLE_COUNTER: Pin = Pin { plain: 0xFFFFA10C, debug: 0xFFFFA130 };

/// `SpriteMask_Y`. tests: sprites_port
pub const SPRITE_MASK_Y: Pin = Pin { plain: 0xFFFFA10E, debug: 0xFFFFA132 };

/// `SpriteMask_Height`. tests: sprites_port
pub const SPRITE_MASK_HEIGHT: Pin = Pin { plain: 0xFFFFA110, debug: 0xFFFFA134 };

/// `SpriteMask_After_Band`. tests: sprites_port
pub const SPRITE_MASK_AFTER_BAND: Pin = Pin { plain: 0xFFFFA112, debug: 0xFFFFA136 };

/// `Scanline_Band_Sprites`. tests: sprites_port
pub const SCANLINE_BAND_SPRITES: Pin = Pin { plain: 0xFFFFA114, debug: 0xFFFFA138 };

/// `Sound_PlaySFX`. tests: animate_port
pub const SOUND_PLAY_SFX: Pin = Pin { plain: 0x6330, debug: 0x7FBA };

/// `ObjectMoveX`. tests: test_g4_final_objects_port
pub const OBJECT_MOVE_X: Pin = Pin { plain: 0x2B00, debug: 0x30DE };

/// `ObjCodeBase`. tests: test_objects_port
pub const OBJ_CODE_BASE: Pin = Pin { plain: 0x10000, debug: 0x10000 };

/// `Draw_Sprite`. tests: test_objects_port
pub const DRAW_SPRITE: Pin = Pin { plain: 0x2B30, debug: 0x310E };

/// `ObjectMove`. tests: test_objects_port
pub const OBJECT_MOVE: Pin = Pin { plain: 0x2AE6, debug: 0x30C4 };

/// `Ring_Sfx_Speaker`. tests: sound_api_port
pub const RING_SFX_SPEAKER: Pin = Pin { plain: 0xFFFFAF40, debug: 0xFFFFAF64 };

/// `Sfx_Ring_Buf`. tests: sound_api_port
pub const SFX_RING_BUF: Pin = Pin { plain: 0xFFFFAF42, debug: 0xFFFFAF66 };

/// `Sfx_Ring_Wr`. tests: sound_api_port
pub const SFX_RING_WR: Pin = Pin { plain: 0xFFFFAF4A, debug: 0xFFFFAF6E };

/// `Sfx_Ring_Rd`. tests: sound_api_port
pub const SFX_RING_RD: Pin = Pin { plain: 0xFFFFAF4B, debug: 0xFFFFAF6F };

/// `SongTable`. tests: sound_api_port
pub const SONG_TABLE: Pin = Pin { plain: 0x5BAE0, debug: 0x5D522 };

/// `SongPatchTable`. tests: sound_api_port
pub const SONG_PATCH_TABLE: Pin = Pin { plain: 0x5BAE4, debug: 0x5D52E };

/// `OJZ_Palette`. tests: act_descriptor_port
pub const OJZ_PALETTE: Pin = Pin { plain: 0x1FE28, debug: 0x1FE90 };

/// `OJZ_Act1_BG_Layout`. tests: act_descriptor_port
pub const OJZ_ACT1_BG_LAYOUT: Pin = Pin { plain: 0x1FEA8, debug: 0x1FF10 };

/// `OJZ_Act1_BG_Tiles`. tests: act_descriptor_port
pub const OJZ_ACT1_BG_TILES: Pin = Pin { plain: 0x21EA8, debug: 0x21F10 };

/// `ParallaxConfig_OJZ_Default`. tests: act_descriptor_port
pub const PARALLAX_CONFIG_OJZ_DEFAULT: Pin = Pin { plain: 0x1138C, debug: 0x113F4 };

/// `OJZ_Act_Pool_PageTable`. tests: act_descriptor_port
pub const OJZ_ACT_POOL_PAGE_TABLE: Pin = Pin { plain: 0x14B1E, debug: 0x14B86 };

/// `OJZ_Sec0_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC0_BLOCKS: Pin = Pin { plain: 0x14D9E, debug: 0x14E06 };

/// `OJZ_Sec1_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC1_BLOCKS: Pin = Pin { plain: 0x1698E, debug: 0x169F6 };

/// `OJZ_Sec2_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC2_BLOCKS: Pin = Pin { plain: 0x17D0A, debug: 0x17D72 };

/// `OJZ_Sec3_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC3_BLOCKS: Pin = Pin { plain: 0x194A2, debug: 0x1950A };

/// `OJZ_Sec4_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC4_BLOCKS: Pin = Pin { plain: 0x17D0A, debug: 0x17D72 };

/// `OJZ_Sec5_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC5_BLOCKS: Pin = Pin { plain: 0x1A5EE, debug: 0x1A656 };

/// `OJZ_Sec6_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC6_BLOCKS: Pin = Pin { plain: 0x1B414, debug: 0x1B47C };

/// `OJZ_Sec7_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC7_BLOCKS: Pin = Pin { plain: 0x1D014, debug: 0x1D07C };

/// `OJZ_Sec8_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC8_BLOCKS: Pin = Pin { plain: 0x1E288, debug: 0x1E2F0 };

/// `OJZ_Sec0_Objects`. tests: act_descriptor_port
pub const OJZ_SEC0_OBJECTS: Pin = Pin { plain: 0x11D84, debug: 0x11DEC };

/// `OJZ_Sec0_Rings`. tests: act_descriptor_port
pub const OJZ_SEC0_RINGS: Pin = Pin { plain: 0x11D8C, debug: 0x11DF4 };

/// `OJZ_Sec0_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC0_TYPE_TABLE: Pin = Pin { plain: 0x11D7E, debug: 0x11DE6 };

/// `OJZ_Sec1_Objects`. tests: act_descriptor_port
pub const OJZ_SEC1_OBJECTS: Pin = Pin { plain: 0x11DB6, debug: 0x11E1E };

/// `OJZ_Sec1_Rings`. tests: act_descriptor_port
pub const OJZ_SEC1_RINGS: Pin = Pin { plain: 0x11DCA, debug: 0x11E32 };

/// `OJZ_Sec1_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC1_TYPE_TABLE: Pin = Pin { plain: 0x11DAC, debug: 0x11E14 };

/// `OJZ_Sec2_Objects`. tests: act_descriptor_port
pub const OJZ_SEC2_OBJECTS: Pin = Pin { plain: 0x11DFC, debug: 0x11E64 };

/// `OJZ_Sec2_Rings`. tests: act_descriptor_port
pub const OJZ_SEC2_RINGS: Pin = Pin { plain: 0x11E0A, debug: 0x11E72 };

/// `OJZ_Sec2_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC2_TYPE_TABLE: Pin = Pin { plain: 0x11DF2, debug: 0x11E5A };

/// `OJZ_Sec3_Objects`. tests: act_descriptor_port
pub const OJZ_SEC3_OBJECTS: Pin = Pin { plain: 0x11E40, debug: 0x11EA8 };

/// `OJZ_Sec3_Rings`. tests: act_descriptor_port
pub const OJZ_SEC3_RINGS: Pin = Pin { plain: 0x11E42, debug: 0x11EAA };

/// `OJZ_Sec3_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC3_TYPE_TABLE: Pin = Pin { plain: 0x11E3E, debug: 0x11EA6 };

/// `OJZ_Sec4_Objects`. tests: act_descriptor_port
pub const OJZ_SEC4_OBJECTS: Pin = Pin { plain: 0x11E48, debug: 0x11EB0 };

/// `OJZ_Sec4_Rings`. tests: act_descriptor_port
pub const OJZ_SEC4_RINGS: Pin = Pin { plain: 0x11E4A, debug: 0x11EB2 };

/// `OJZ_Sec4_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC4_TYPE_TABLE: Pin = Pin { plain: 0x11E46, debug: 0x11EAE };

/// `OJZ_Sec5_Objects`. tests: act_descriptor_port
pub const OJZ_SEC5_OBJECTS: Pin = Pin { plain: 0x11E80, debug: 0x11EE8 };

/// `OJZ_Sec5_Rings`. tests: act_descriptor_port
pub const OJZ_SEC5_RINGS: Pin = Pin { plain: 0x11E82, debug: 0x11EEA };

/// `OJZ_Sec5_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC5_TYPE_TABLE: Pin = Pin { plain: 0x11E7E, debug: 0x11EE6 };

/// `OJZ_Sec6_Objects`. tests: act_descriptor_port
pub const OJZ_SEC6_OBJECTS: Pin = Pin { plain: 0x11EA8, debug: 0x11F10 };

/// `OJZ_Sec6_Rings`. tests: act_descriptor_port
pub const OJZ_SEC6_RINGS: Pin = Pin { plain: 0x11EAA, debug: 0x11F12 };

/// `OJZ_Sec6_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC6_TYPE_TABLE: Pin = Pin { plain: 0x11EA6, debug: 0x11F0E };

/// `OJZ_Sec7_Objects`. tests: act_descriptor_port
pub const OJZ_SEC7_OBJECTS: Pin = Pin { plain: 0x11EB0, debug: 0x11F18 };

/// `OJZ_Sec7_Rings`. tests: act_descriptor_port
pub const OJZ_SEC7_RINGS: Pin = Pin { plain: 0x11EB2, debug: 0x11F1A };

/// `OJZ_Sec7_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC7_TYPE_TABLE: Pin = Pin { plain: 0x11EAE, debug: 0x11F16 };

/// `OJZ_Sec8_Objects`. tests: act_descriptor_port
pub const OJZ_SEC8_OBJECTS: Pin = Pin { plain: 0x11ED8, debug: 0x11F40 };

/// `OJZ_Sec8_Rings`. tests: act_descriptor_port
pub const OJZ_SEC8_RINGS: Pin = Pin { plain: 0x11EDA, debug: 0x11F42 };

/// `OJZ_Sec8_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC8_TYPE_TABLE: Pin = Pin { plain: 0x11ED6, debug: 0x11F3E };

/// `OJZ_ACT_POOL_PAGES`. tests: act_descriptor_port
pub const OJZ_ACT_POOL_PAGES: Pin = Pin { plain: 0x3, debug: 0x3 };

/// `BLOCK_INDEX_SIZE`. tests: act_descriptor_port
pub const BLOCK_INDEX_SIZE: Pin = Pin { plain: 0x400, debug: 0x400 };

/// `EDGE_CLAMP`. tests: act_descriptor_port
pub const EDGE_CLAMP: Pin = Pin { plain: 0x0, debug: 0x0 };

/// `MAX_ACT_SECTIONS`. tests: act_descriptor_port
pub const MAX_ACT_SECTIONS: Pin = Pin { plain: 0x30, debug: 0x30 };

/// `SECTION_SIZE_SHIFT`. tests: act_descriptor_port
pub const SECTION_SIZE_SHIFT: Pin = Pin { plain: 0xB, debug: 0xB };

/// `Act_len`. tests: act_descriptor_port
pub const ACT_LEN: Pin = Pin { plain: 0x22, debug: 0x22 };

/// `Sec_len`. tests: act_descriptor_port
pub const SEC_LEN: Pin = Pin { plain: 0x42, debug: 0x42 };

/// `OJZ_SEC0_BLOCK_DICT_LEN`. tests: act_descriptor_port
pub const OJZ_SEC0_BLOCK_DICT_LEN: Pin = Pin { plain: 0x0, debug: 0x0 };

/// `OJZ_SEC1_BLOCK_DICT_LEN`. tests: act_descriptor_port
pub const OJZ_SEC1_BLOCK_DICT_LEN: Pin = Pin { plain: 0x300, debug: 0x300 };

/// `OJZ_SEC2_BLOCK_DICT_LEN`. tests: act_descriptor_port
pub const OJZ_SEC2_BLOCK_DICT_LEN: Pin = Pin { plain: 0x300, debug: 0x300 };

/// `OJZ_SEC3_BLOCK_DICT_LEN`. tests: act_descriptor_port
pub const OJZ_SEC3_BLOCK_DICT_LEN: Pin = Pin { plain: 0x300, debug: 0x300 };

/// `OJZ_SEC4_BLOCK_DICT_LEN`. tests: act_descriptor_port
pub const OJZ_SEC4_BLOCK_DICT_LEN: Pin = Pin { plain: 0x300, debug: 0x300 };

/// `OJZ_SEC5_BLOCK_DICT_LEN`. tests: act_descriptor_port
pub const OJZ_SEC5_BLOCK_DICT_LEN: Pin = Pin { plain: 0x300, debug: 0x300 };

/// `OJZ_SEC6_BLOCK_DICT_LEN`. tests: act_descriptor_port
pub const OJZ_SEC6_BLOCK_DICT_LEN: Pin = Pin { plain: 0x300, debug: 0x300 };

/// `OJZ_SEC7_BLOCK_DICT_LEN`. tests: act_descriptor_port
pub const OJZ_SEC7_BLOCK_DICT_LEN: Pin = Pin { plain: 0x300, debug: 0x300 };

/// `OJZ_SEC8_BLOCK_DICT_LEN`. tests: act_descriptor_port
pub const OJZ_SEC8_BLOCK_DICT_LEN: Pin = Pin { plain: 0x300, debug: 0x300 };

/// `Camera_Y_Coarse_Prev`. tests: entity_window_port
pub const CAMERA_Y_COARSE_PREV: Pin = Pin { plain: 0xFFFFAD80, debug: 0xFFFFADA4 };

/// `Current_Act_Ptr`. tests: entity_window_port, section_port
pub const CURRENT_ACT_PTR: Pin = Pin { plain: 0xFFFFAF3C, debug: 0xFFFFAF60 };

/// `Entity_Window_Active`. tests: entity_window_port
pub const ENTITY_WINDOW_ACTIVE: Pin = Pin { plain: 0xFFFFAC74, debug: 0xFFFFAC98 };

/// `Entity_Window_Anchor`. tests: entity_window_port
pub const ENTITY_WINDOW_ANCHOR: Pin = Pin { plain: 0xFFFFAC76, debug: 0xFFFFAC9A };

/// `Entity_Window_OriginX`. tests: entity_window_port
pub const ENTITY_WINDOW_ORIGIN_X: Pin = Pin { plain: 0xFFFFAC78, debug: 0xFFFFAC9C };

/// `Entity_Window_OriginY`. tests: entity_window_port
pub const ENTITY_WINDOW_ORIGIN_Y: Pin = Pin { plain: 0xFFFFAC7A, debug: 0xFFFFAC9E };

/// `Entity_Window_Center_ID`. tests: entity_window_port
pub const ENTITY_WINDOW_CENTER_ID: Pin = Pin { plain: 0xFFFFAC75, debug: 0xFFFFAC99 };

/// `Entity_Scan_State`. tests: entity_window_port
pub const ENTITY_SCAN_STATE: Pin = Pin { plain: 0xFFFFAC18, debug: 0xFFFFAC3C };

/// `Entity_Loaded_Masks`. tests: entity_window_port
pub const ENTITY_LOADED_MASKS: Pin = Pin { plain: 0xFFFFAC7C, debug: 0xFFFFACA0 };

/// `Entity_Mask_Scratch`. tests: entity_window_port
pub const ENTITY_MASK_SCRATCH: Pin = Pin { plain: 0xFFFFACFC, debug: 0xFFFFAD20 };

/// `Ring_Collected_Window`. tests: entity_window_port
pub const RING_COLLECTED_WINDOW: Pin = Pin { plain: 0xFFFFAD82, debug: 0xFFFFADA6 };

/// `Ring_Collected_Park`. tests: entity_window_port
pub const RING_COLLECTED_PARK: Pin = Pin { plain: 0xFFFFAEB6, debug: 0xFFFFAEDA };

/// `Collected_Park_Next`. tests: entity_window_port
pub const COLLECTED_PARK_NEXT: Pin = Pin { plain: 0xFFFFAF3A, debug: 0xFFFFAF5E };

/// `RingBuffer_Clear`. tests: entity_window_port
pub const RING_BUFFER_CLEAR: Pin = Pin { plain: 0x333E, debug: 0x3A9E };

/// `RingBuffer_Remove`. tests: entity_window_port
pub const RING_BUFFER_REMOVE: Pin = Pin { plain: 0x330A, debug: 0x3A6A };

/// `Section_GetSecPtrXY`. tests: entity_window_port
pub const SECTION_GET_SEC_PTR_XY: Pin = Pin { plain: 0x55F4, debug: 0x638C };

/// `Section_FlatIDXY`. tests: entity_window_port
pub const SECTION_FLAT_IDXY: Pin = Pin { plain: 0x55DA, debug: 0x6372 };

/// `AllocDynamic`. tests: load_object_port, children_port
pub const ALLOC_DYNAMIC: Pin = Pin { plain: 0x288A, debug: 0x2A20 };

/// `AllocEffect`. tests: children_port
pub const ALLOC_EFFECT: Pin = Pin { plain: 0x28EE, debug: 0x2A84 };

/// `Palette_Buffer`. tests: buffers_port
pub const PALETTE_BUFFER: Pin = Pin { plain: 0xFFFF8206, debug: 0xFFFF8206 };

/// `Hscroll_Buffer`. tests: buffers_port
pub const HSCROLL_BUFFER: Pin = Pin { plain: 0xFFFF850A, debug: 0xFFFF850A };

/// `Static_Pal_Line0`. tests: buffers_port
pub const STATIC_PAL_LINE0: Pin = Pin { plain: 0xFFFF898C, debug: 0xFFFF898C };

/// `Static_Pal_Line1`. tests: buffers_port
pub const STATIC_PAL_LINE1: Pin = Pin { plain: 0xFFFF899A, debug: 0xFFFF899A };

/// `Static_Pal_Line2`. tests: buffers_port
pub const STATIC_PAL_LINE2: Pin = Pin { plain: 0xFFFF89A8, debug: 0xFFFF89A8 };

/// `Static_Pal_Line3`. tests: buffers_port
pub const STATIC_PAL_LINE3: Pin = Pin { plain: 0xFFFF89B6, debug: 0xFFFF89B6 };

/// `Static_Sprite_DMA`. tests: buffers_port
pub const STATIC_SPRITE_DMA: Pin = Pin { plain: 0xFFFF89C4, debug: 0xFFFF89C4 };

/// `Static_Hscroll_Cell`. tests: buffers_port
pub const STATIC_HSCROLL_CELL: Pin = Pin { plain: 0xFFFF89D2, debug: 0xFFFF89D2 };

/// `Static_Hscroll_Line`. tests: buffers_port
pub const STATIC_HSCROLL_LINE: Pin = Pin { plain: 0xFFFF89E0, debug: 0xFFFF89E0 };

/// `Palette_Dirty`. tests: buffers_port
pub const PALETTE_DIRTY: Pin = Pin { plain: 0xFFFF8286, debug: 0xFFFF8286 };

/// `Parallax_Active_Config`. tests: buffers_port
pub const PARALLAX_ACTIVE_CONFIG: Pin = Pin { plain: 0x5BD4, debug: 0x696C };

/// `VBlank_Ready`. tests: vblank_port
pub const V_BLANK_READY: Pin = Pin { plain: 0xFFFF803A, debug: 0xFFFF803A };

/// `VBlank_Flag`. tests: vblank_port
pub const V_BLANK_FLAG: Pin = Pin { plain: 0xFFFF8000, debug: 0xFFFF8000 };

/// `VInt_Ptr`. tests: vblank_port
pub const V_INT_PTR: Pin = Pin { plain: 0xFFFF8036, debug: 0xFFFF8036 };

/// `Ctrl_1_Press`. tests: vblank_port
pub const CTRL_1_PRESS: Pin = Pin { plain: 0xFFFF802D, debug: 0xFFFF802D };

/// `Ctrl_1_Press_Accum`. tests: vblank_port
pub const CTRL_1_PRESS_ACCUM: Pin = Pin { plain: 0xFFFF8030, debug: 0xFFFF8030 };

/// `Ctrl_2_Press`. tests: vblank_port
pub const CTRL_2_PRESS: Pin = Pin { plain: 0xFFFF802F, debug: 0xFFFF802F };

/// `Ctrl_2_Press_Accum`. tests: vblank_port
pub const CTRL_2_PRESS_ACCUM: Pin = Pin { plain: 0xFFFF8031, debug: 0xFFFF8031 };

/// `DMA_Budget_Default`. tests: vblank_port
pub const DMA_BUDGET_DEFAULT: Pin = Pin { plain: 0xFFFF8202, debug: 0xFFFF8202 };

/// `Lag_Frame_Count` — debug-shape consumer only (`debug_only`). tests: vblank_port
pub const LAG_FRAME_COUNT: u32 = 0xFFFF89F8;

/// `DMA_Bytes_ThisFrame` — debug-shape consumer only (`debug_only`). tests: vblank_port
pub const DMA_BYTES_THIS_FRAME: u32 = 0xFFFF89EE;

/// `Flush_VDP_Shadow`. tests: vblank_port
pub const FLUSH_VDP_SHADOW: Pin = Pin { plain: 0x1C1E, debug: 0x1CA0 };

/// `VInt_DrawLevel`. tests: vblank_port
pub const V_INT_DRAW_LEVEL: Pin = Pin { plain: 0x430A, debug: 0x4FE2 };

/// `Vscroll_Write`. tests: vblank_port
pub const VSCROLL_WRITE: Pin = Pin { plain: 0x5BE6, debug: 0x697E };

/// `Read_Controllers`. tests: vblank_port
pub const READ_CONTROLLERS: Pin = Pin { plain: 0x2324, debug: 0x23B2 };

/// `Process_DMA_Critical`. tests: vblank_port
pub const PROCESS_DMA_CRITICAL: Pin = Pin { plain: 0x1E22, debug: 0x1EA8 };

/// `Process_DMA_Important`. tests: vblank_port
pub const PROCESS_DMA_IMPORTANT: Pin = Pin { plain: 0x1EF0, debug: 0x1F76 };

/// `Process_DMA_Deferrable`. tests: vblank_port
pub const PROCESS_DMA_DEFERRABLE: Pin = Pin { plain: 0x1F04, debug: 0x1F8A };

/// `Enqueue_Dirty_Buffers`. tests: vblank_port
pub const ENQUEUE_DIRTY_BUFFERS: Pin = Pin { plain: 0x204E, debug: 0x20D4 };

/// `BootData`. tests: boot_port
pub const BOOT_DATA: Pin = Pin { plain: 0x3A8, debug: 0x3AC };

/// `VInt_Level`. tests: boot_port
pub const V_INT_LEVEL: Pin = Pin { plain: 0x21CE, debug: 0x2254 };

/// `BuildStaticDMA`. tests: boot_port
pub const BUILD_STATIC_DMA: Pin = Pin { plain: 0x1F76, debug: 0x1FFC };

/// `Sound_Init`. tests: boot_port
pub const SOUND_INIT: Pin = Pin { plain: 0x6230, debug: 0x7D56 };

/// `Hardware_Region`. tests: boot_port
pub const HARDWARE_REGION: Pin = Pin { plain: 0xFFFF8026, debug: 0xFFFF8026 };

/// `Region_Flags`. tests: boot_port
pub const REGION_FLAGS: Pin = Pin { plain: 0xFFFF8027, debug: 0xFFFF8027 };

/// `Timing_Step`. tests: boot_port
pub const TIMING_STEP: Pin = Pin { plain: 0xFFFF8028, debug: 0xFFFF8028 };

/// `Frame_Accumulator`. tests: boot_port
pub const FRAME_ACCUMULATOR: Pin = Pin { plain: 0xFFFF802A, debug: 0xFFFF802A };

/// `Game_State_ID`. tests: boot_port
pub const GAME_STATE_ID: Pin = Pin { plain: 0xFFFF8008, debug: 0xFFFF8008 };

/// `Game_State_Init`. tests: boot_port
pub const GAME_STATE_INIT: Pin = Pin { plain: 0xFFFF8009, debug: 0xFFFF8009 };

/// `RAM_Start`. tests: boot_port
pub const RAM_START: Pin = Pin { plain: 0xFFFF8000, debug: 0xFFFF8000 };

/// `PState_Ground`. tests: test_p1_player_port
pub const P_STATE_GROUND: Pin = Pin { plain: 0x10448, debug: 0x10448 };

/// `PState_Roll`. tests: test_p1_player_port
pub const P_STATE_ROLL: Pin = Pin { plain: 0x105A0, debug: 0x105A0 };

/// `PState_Spindash`. tests: test_p1_player_port
pub const P_STATE_SPINDASH: Pin = Pin { plain: 0x10B58, debug: 0x10B58 };

/// `PState_Air`. tests: test_p1_player_port
pub const P_STATE_AIR: Pin = Pin { plain: 0x10896, debug: 0x10896 };

/// `PState_Jump`. tests: test_p1_player_port
pub const P_STATE_JUMP: Pin = Pin { plain: 0x1089E, debug: 0x1089E };

/// `PState_RollJump`. tests: test_p1_player_port
pub const P_STATE_ROLL_JUMP: Pin = Pin { plain: 0x1089A, debug: 0x1089A };

/// `PState_AirBall`. tests: test_p1_player_port
pub const P_STATE_AIR_BALL: Pin = Pin { plain: 0x10896, debug: 0x10896 };

/// `Player_SensorFloor`. tests: test_p1_player_port
pub const PLAYER_SENSOR_FLOOR: Pin = Pin { plain: 0x5414, debug: 0x61AC };

/// `Player_AtLedgeEdge`. tests: test_p1_player_port
pub const PLAYER_AT_LEDGE_EDGE: Pin = Pin { plain: 0x552E, debug: 0x62C6 };

/// `Player_SetState`. tests: test_p2_player_states_port
pub const PLAYER_SET_STATE: Pin = Pin { plain: 0x1023C, debug: 0x1023C };

/// `Player_SnapToSurface`. tests: test_p2_player_states_port
pub const PLAYER_SNAP_TO_SURFACE: Pin = Pin { plain: 0x1030C, debug: 0x1030C };

/// `Player_SensorCeiling`. tests: test_p2_player_states_port
pub const PLAYER_SENSOR_CEILING: Pin = Pin { plain: 0x542A, debug: 0x61C2 };

/// `Player_SensorWallDir`. tests: test_p2_player_states_port
pub const PLAYER_SENSOR_WALL_DIR: Pin = Pin { plain: 0x54E4, debug: 0x627C };

/// `Player_SensorWallAt`. tests: test_p2_player_states_port
pub const PLAYER_SENSOR_WALL_AT: Pin = Pin { plain: 0x54DC, debug: 0x6274 };

/// `Collision_GetType`. tests: test_p4_player_sensors_port
pub const COLLISION_GET_TYPE: Pin = Pin { plain: 0x5084, debug: 0x5E1C };

/// `SolidityTable`. tests: test_p4_player_sensors_port
pub const SOLIDITY_TABLE: Pin = Pin { plain: 0x27852, debug: 0x278BA };

/// `AngleTable`. tests: test_p4_player_sensors_port
pub const ANGLE_TABLE: Pin = Pin { plain: 0x27752, debug: 0x277BA };

/// `HeightMaps`. tests: test_p4_player_sensors_port
pub const HEIGHT_MAPS: Pin = Pin { plain: 0x25752, debug: 0x257BA };

/// `HeightMapsRot`. tests: test_p4_player_sensors_port
pub const HEIGHT_MAPS_ROT: Pin = Pin { plain: 0x26752, debug: 0x267BA };

/// `Sonic_LoadArt`. tests: test_p1_player_port
pub const SONIC_LOAD_ART: Pin = Pin { plain: 0x10C0C, debug: 0x10C0C };

/// `PhysTable_Sonic`. tests: test_p1_player_port
pub const PHYS_TABLE_SONIC: Pin = Pin { plain: 0x10C24, debug: 0x10C24 };

/// `Player_Phys`. tests: test_p1_player_port
pub const PLAYER_PHYS: Pin = Pin { plain: 0xFFFFB07A, debug: 0xFFFFB0A2 };

/// `Player_Phys_End`. tests: test_p1_player_port
pub const PLAYER_PHYS_END: Pin = Pin { plain: 0xFFFFB08A, debug: 0xFFFFB0B2 };

/// `Player_Quadrant`. tests: test_p1_player_port
pub const PLAYER_QUADRANT: Pin = Pin { plain: 0xFFFFB08A, debug: 0xFFFFB0B2 };

/// `Player_JumpBuffer`. tests: test_p1_player_port
pub const PLAYER_JUMP_BUFFER: Pin = Pin { plain: 0xFFFFB08B, debug: 0xFFFFB0B3 };

/// `Player_Ring_Index`. tests: test_p1_player_port
pub const PLAYER_RING_INDEX: Pin = Pin { plain: 0xFFFFB400, debug: 0xFFFFB400 };

/// `Player_Pos_Ring`. tests: test_p1_player_port
pub const PLAYER_POS_RING: Pin = Pin { plain: 0xFFFFB200, debug: 0xFFFFB200 };

/// `Player_Stat_Ring`. tests: test_p1_player_port
pub const PLAYER_STAT_RING: Pin = Pin { plain: 0xFFFFB300, debug: 0xFFFFB300 };

/// `Player_Death_Pending`. tests: test_p1_player_port
pub const PLAYER_DEATH_PENDING: Pin = Pin { plain: 0xFFFFB08C, debug: 0xFFFFB0B4 };

// ── Region-relative offsets (manifest order) ──

/// `AnimateSprite.cc_delete` − `animate` start (per-shape). tests: animate_port
pub const CC_DELETE_OFF: ShapeOffset = ShapeOffset { plain: 0x104, debug: 0x15E };

/// `RefreshSpritePieceCount` − `animate` start (per-shape). tests: animate_port
pub const REFRESH_OFF: ShapeOffset = ShapeOffset { plain: 0x16C, debug: 0x28A };

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
