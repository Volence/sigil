//! GENERATED FILE — DO NOT EDIT BY HAND.
//!
//! Emitted by `cargo run -p sigil-harness --bin repin` from `repin.toml`
//! + the aeon listings (D-T10.3, tranche-10 step 0). Edit the MANIFEST,
//! then regenerate; `tests/repin_pins.rs::pins_rs_is_current` guards
//! staleness. All values are LISTING truth — per-shape VMAs/lengths from
//! `s4.lst` (plain) and `s4.debug.lst` (`__DEBUG__`).
//!
//! [provenance] plain: /home/volence/sonic_hacks/aeon/.worktrees/port-tranche16/s4.lst (07/15/2026 04:10:15 PM)
//! [provenance] debug: /home/volence/sonic_hacks/aeon/.worktrees/port-tranche16/s4.debug.lst (07/15/2026 04:10:14 PM)
//! [provenance] 23 regions, 193 symbols, 7 offsets

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
pub const ASSEMBLED_LEN: usize = 0x65B60;
/// Assembled (pre-convsym) ROM length, `__DEBUG__` shape. tests: m1d_rom, m1d_debug_rom, mixed_dac_rom
pub const DEBUG_ASSEMBLED_LEN: usize = 0x6765A;

// ── Regions (manifest order) ──

/// `VDP_Shadow_Init` .. `Init_DMA_Queue` — gate `SIGIL_EMP_VDP_INIT`. tests: vdp_init_port
pub const VDP_INIT: Region = Region { plain_base: 0x1C14, debug_base: 0x1C96, plain_len: 0x48, debug_len: 0x48 };

/// `HBlank_Dispatch` .. `Read_Controllers` — gate `SIGIL_EMP_HBLANK`. tests: hblank_port, m1c_vector_table
pub const HBLANK: Region = Region { plain_base: 0x2286, debug_base: 0x2314, plain_len: 0x12, debug_len: 0x12 };

/// `Read_Controllers` .. `GameLoop` — gate `SIGIL_EMP_CONTROLLERS`. tests: controllers_port
pub const CONTROLLERS: Region = Region { plain_base: 0x2298, debug_base: 0x2326, plain_len: 0x72, debug_len: 0x72 };

/// `GameLoop` .. `S4LZ_DecompressDict` — gate `SIGIL_EMP_GAME_LOOP`. tests: game_loop_port
pub const GAME_LOOP: Region = Region { plain_base: 0x230A, debug_base: 0x2398, plain_len: 0x12, debug_len: 0x12 };

/// `GetSineCosine` .. `Perform_DPLC` — gate `SIGIL_EMP_MATH`. tests: math_port
pub const MATH: Region = Region { plain_base: 0x2470, debug_base: 0x2602, plain_len: 0x298, debug_len: 0x298 };

/// `Perform_DPLC` .. `InitObjectRAM` — gate `SIGIL_EMP_DPLC`. tests: dplc_port
pub const DPLC: Region = Region { plain_base: 0x2708, debug_base: 0x289A, plain_len: 0xA4, debug_len: 0xA4 };

/// `InitObjectRAM` .. `InitSpriteSystem` — gate `SIGIL_EMP_CORE`. tests: core_port
pub const CORE: Region = Region { plain_base: 0x27AC, debug_base: 0x293E, plain_len: 0x2EE, debug_len: 0x736 };

/// `InitSpriteSystem` .. `AnimateSprite` — gate `SIGIL_EMP_SPRITES`. tests: sprites_port
pub const SPRITES: Region = Region { plain_base: 0x2A9A, debug_base: 0x3074, plain_len: 0x420, debug_len: 0x420 };

/// `AnimateSprite` .. `TouchResponse` — gate `SIGIL_EMP_ANIMATE`. tests: animate_port, test_objects_port
pub const ANIMATE: Region = Region { plain_base: 0x2EBA, debug_base: 0x3494, plain_len: 0x18A, debug_len: 0x2A8 };

/// `TouchResponse` .. `RingBuffer_Add` — gate `SIGIL_EMP_COLLISION`. tests: collision_port
pub const COLLISION: Region = Region { plain_base: 0x3044, debug_base: 0x373C, plain_len: 0x200, debug_len: 0x208 };

/// `RingBuffer_Add` .. `Collected_Init` — gate `SIGIL_EMP_RINGS`. tests: rings_port
pub const RINGS: Region = Region { plain_base: 0x3244, debug_base: 0x3944, plain_len: 0x1B8, debug_len: 0x214 };

/// `Collected_Init` .. `PopulateSpawnedPieceCount` — gate `SIGIL_EMP_ENTITY_WINDOW`. tests: entity_window_port
pub const ENTITY_WINDOW: Region = Region { plain_base: 0x33FC, debug_base: 0x3B58, plain_len: 0x8D2, debug_len: 0xD40 };

/// `Load_Object` .. `Plane_Buffer_Reset` — gate `SIGIL_EMP_LOAD_OBJECT`. tests: load_object_port, entity_window_port
pub const LOAD_OBJECT: Region = Region { plain_base: 0x3FDC, debug_base: 0x4BA6, plain_len: 0x82, debug_len: 0x82 };

/// `Tile_Cache_GetTile` .. `Collision_GetType` — gate `SIGIL_EMP_TILE_CACHE`. tests: tile_cache_port
pub const TILE_CACHE: Region = Region { plain_base: 0x42FA, debug_base: 0x4EC4, plain_len: 0x924, debug_len: 0x9DC };

/// `Collision_GetType` .. `Collision_ProbeDown` — gate `SIGIL_EMP_COLLISION_LOOKUP`. tests: collision_lookup_port
pub const COLLISION_LOOKUP: Region = Region { plain_base: 0x4C1E, debug_base: 0x58A0, plain_len: 0x24, debug_len: 0x24 };

/// `Section_Init` .. `Camera_Init` — gate `SIGIL_EMP_SECTION`. tests: section_port
pub const SECTION: Region = Region { plain_base: 0x513E, debug_base: 0x5DC0, plain_len: 0x3DC, debug_len: 0x3DC };

/// `Sound_PostByte` .. start + 0x1E4 plain / 0x2DA debug (literal — no end symbol) — gate `SIGIL_EMP_SOUND_API`. tests: sound_api_port
pub const SOUND_API: Region = Region { plain_base: 0x5D86, debug_base: 0x76A2, plain_len: 0x1E4, debug_len: 0x2DA };

/// `TestSolid_Init` .. `TestParticle` — gate `SIGIL_EMP_TEST_OBJECTS`. tests: test_objects_port
pub const TEST_SOLID: Region = Region { plain_base: 0x10F7C, debug_base: 0x10F7C, plain_len: 0xE, debug_len: 0xE };

/// `TestParticle` .. `TestEmitter` — gate `SIGIL_EMP_TEST_OBJECTS`. tests: test_objects_port
pub const TEST_PARTICLE: Region = Region { plain_base: 0x10F8A, debug_base: 0x10F8A, plain_len: 0x52, debug_len: 0x52 };

/// `Ani_Sonic` .. `Ani_Sonic_End` — gate `SIGIL_EMP_SONIC_ANIMS`. tests: sonic_anims_port
pub const SONIC_ANIMS: Region = Region { plain_base: 0x309E8, debug_base: 0x30A50, plain_len: 0x6E, debug_len: 0x6E };

/// `Ani_Particle` .. `Ani_Particle_End` — gate `SIGIL_EMP_PARTICLE_ANIMS`. tests: particle_anims_port, test_objects_port
pub const PARTICLE_ANIMS: Region = Region { plain_base: 0x30A56, debug_base: 0x30ABE, plain_len: 0x8, debug_len: 0x8 };

/// `OJZ_Act1_Descriptor` .. `OJZ_Sec0_Blocks` — gate `SIGIL_EMP_ACT_DESCRIPTOR`. tests: act_descriptor_port
pub const ACT_DESCRIPTOR: Region = Region { plain_base: 0x14B5E, debug_base: 0x14BC6, plain_len: 0x274, debug_len: 0x274 };

/// `ObjDef_Static` .. `OJZ_Sec0_TypeTable` — gate `SIGIL_EMP_OBJDEFS`. tests: objdef_port
pub const OBJDEFS: Region = Region { plain_base: 0x11D4A, debug_base: 0x11DB2, plain_len: 0x68, debug_len: 0x68 };

// ── Symbols (manifest order) ──

/// `TestStatic_Main`. tests: objdef_port
pub const TEST_STATIC_MAIN: Pin = Pin { plain: 0x10C66, debug: 0x10C66 };

/// `TestSolid_Init`. tests: objdef_port
pub const TEST_SOLID_INIT: Pin = Pin { plain: 0x10F7C, debug: 0x10F7C };

/// `TestEnemy_Init`. tests: objdef_port
pub const TEST_ENEMY_INIT: Pin = Pin { plain: 0x10F34, debug: 0x10F34 };

/// `TestParent`. tests: objdef_port
pub const TEST_PARENT: Pin = Pin { plain: 0x110C0, debug: 0x110C0 };

/// `Map_TestObj`. tests: objdef_port
pub const MAP_TEST_OBJ: Pin = Pin { plain: 0x309B8, debug: 0x30A20 };

/// `EntryPoint`. tests: m1c_vector_table
pub const ENTRY_POINT: Pin = Pin { plain: 0x200, debug: 0x200 };

/// `NullInterrupt`. tests: m1c_vector_table
pub const NULL_INTERRUPT: Pin = Pin { plain: 0x64AAE, debug: 0x665A8 };

/// `BusError`. tests: m1c_vector_table
pub const BUS_ERROR: Pin = Pin { plain: 0x64AB0, debug: 0x665AA };

/// `AddressError`. tests: m1c_vector_table
pub const ADDRESS_ERROR: Pin = Pin { plain: 0x64AC8, debug: 0x665C2 };

/// `IllegalInstr`. tests: m1c_vector_table
pub const ILLEGAL_INSTR: Pin = Pin { plain: 0x64AE4, debug: 0x665DE };

/// `ZeroDivide`. tests: m1c_vector_table
pub const ZERO_DIVIDE: Pin = Pin { plain: 0x64B06, debug: 0x66600 };

/// `ChkInstr`. tests: m1c_vector_table
pub const CHK_INSTR: Pin = Pin { plain: 0x64B20, debug: 0x6661A };

/// `TrapvInstr`. tests: m1c_vector_table
pub const TRAPV_INSTR: Pin = Pin { plain: 0x64B3E, debug: 0x66638 };

/// `PrivilegeViol`. tests: m1c_vector_table
pub const PRIVILEGE_VIOL: Pin = Pin { plain: 0x64B5E, debug: 0x66658 };

/// `Trace`. tests: m1c_vector_table
pub const TRACE: Pin = Pin { plain: 0x64B80, debug: 0x6667A };

/// `Line1010Emu`. tests: m1c_vector_table
pub const LINE1010_EMU: Pin = Pin { plain: 0x64B94, debug: 0x6668E };

/// `Line1111Emu`. tests: m1c_vector_table
pub const LINE1111_EMU: Pin = Pin { plain: 0x64BB4, debug: 0x666AE };

/// `ErrorExcept`. tests: m1c_vector_table
pub const ERROR_EXCEPT: Pin = Pin { plain: 0x64BD4, debug: 0x666CE };

/// `ErrorTrap`. tests: m1c_vector_table
pub const ERROR_TRAP: Pin = Pin { plain: 0x64BF2, debug: 0x666EC };

/// `VBlank_Handler`. tests: m1c_vector_table
pub const V_BLANK_HANDLER: Pin = Pin { plain: 0x215E, debug: 0x21E4 };

/// `HBlank_Handler_Ptr`. tests: hblank_port
pub const H_BLANK_HANDLER_PTR: Pin = Pin { plain: 0xFFFF8022, debug: 0xFFFF8022 };

/// `VDP_Shadow_Table`. tests: vdp_init_port
pub const VDP_SHADOW_TABLE: Pin = Pin { plain: 0xFFFF800A, debug: 0xFFFF800A };

/// `VDP_Dirty_Mask`. tests: vdp_init_port
pub const VDP_DIRTY_MASK: Pin = Pin { plain: 0xFFFF801E, debug: 0xFFFF801E };

/// `BootData_VDPRegs`. tests: vdp_init_port
pub const BOOT_DATA_VDP_REGS: Pin = Pin { plain: 0x3CE, debug: 0x3D2 };

/// `Ctrl_1_Held`. tests: controllers_port
pub const CTRL_1_HELD: Pin = Pin { plain: 0xFFFF802C, debug: 0xFFFF802C };

/// `VSync_Wait`. tests: game_loop_port
pub const V_SYNC_WAIT: Pin = Pin { plain: 0x226E, debug: 0x22F8 };

/// `Sound_DrainSfxRing`. tests: game_loop_port
pub const SOUND_DRAIN_SFX_RING: Pin = Pin { plain: 0x5ECC, debug: 0x78DE };

/// `Game_State`. tests: game_loop_port
pub const GAME_STATE: Pin = Pin { plain: 0xFFFF8004, debug: 0xFFFF8004 };

/// `Cache_Left_Col`. tests: collision_lookup_port, section_port
pub const CACHE_LEFT_COL: Pin = Pin { plain: 0xFFFFA838, debug: 0xFFFFA85C };

/// `Tile_Cache_GetCollision`. tests: collision_lookup_port
pub const TILE_CACHE_GET_COLLISION: Pin = Pin { plain: 0x4336, debug: 0x4F00 };

/// `Draw_TileColumn`. tests: section_port
pub const DRAW_TILE_COLUMN: Pin = Pin { plain: 0x4066, debug: 0x4C30 };

/// `Draw_TileRow_FromCache`. tests: section_port
pub const DRAW_TILE_ROW_FROM_CACHE: Pin = Pin { plain: 0x4188, debug: 0x4D52 };

/// `EntityWindow_Init`. tests: section_port
pub const ENTITY_WINDOW_INIT: Pin = Pin { plain: 0x37C2, debug: 0x429C };

/// `Section_Plane_Dirty`. tests: section_port
pub const SECTION_PLANE_DIRTY: Pin = Pin { plain: 0xFFFFA88A, debug: 0xFFFFA8AE };

/// `Section_Right_Col_Written`. tests: section_port
pub const SECTION_RIGHT_COL_WRITTEN: Pin = Pin { plain: 0xFFFFA88C, debug: 0xFFFFA8B0 };

/// `Section_Left_Col_Written`. tests: section_port
pub const SECTION_LEFT_COL_WRITTEN: Pin = Pin { plain: 0xFFFFA88E, debug: 0xFFFFA8B2 };

/// `Section_Top_Row_Written`. tests: section_port
pub const SECTION_TOP_ROW_WRITTEN: Pin = Pin { plain: 0xFFFFA886, debug: 0xFFFFA8AA };

/// `Section_Bottom_Row_Written`. tests: section_port
pub const SECTION_BOTTOM_ROW_WRITTEN: Pin = Pin { plain: 0xFFFFA888, debug: 0xFFFFA8AC };

/// `Cache_Head_Col`. tests: section_port
pub const CACHE_HEAD_COL: Pin = Pin { plain: 0xFFFFA83A, debug: 0xFFFFA85E };

/// `Cache_Top_Row`. tests: section_port
pub const CACHE_TOP_ROW: Pin = Pin { plain: 0xFFFFA83C, debug: 0xFFFFA860 };

/// `Cache_Bottom_Row`. tests: section_port
pub const CACHE_BOTTOM_ROW: Pin = Pin { plain: 0xFFFFA83E, debug: 0xFFFFA862 };

/// `Cache_Origin_Col`. tests: section_port
pub const CACHE_ORIGIN_COL: Pin = Pin { plain: 0xFFFFA840, debug: 0xFFFFA864 };

/// `Cache_Origin_Row`. tests: section_port
pub const CACHE_ORIGIN_ROW: Pin = Pin { plain: 0xFFFFA842, debug: 0xFFFFA866 };

/// `Plane_Buffer_Ptr`. tests: section_port
pub const PLANE_BUFFER_PTR: Pin = Pin { plain: 0xFFFFA72C, debug: 0xFFFFA750 };

/// `Tile_Cache_Nametable`. tests: section_port
pub const TILE_CACHE_NAMETABLE: Pin = Pin { plain: 0xFFFF0000, debug: 0xFFFF0000 };

/// `Tile_Cache_Collision`. tests: tile_cache_port
pub const TILE_CACHE_COLLISION: Pin = Pin { plain: 0xFFFF2580, debug: 0xFFFF2580 };

/// `Frame_Counter`. tests: tile_cache_port
pub const FRAME_COUNTER: Pin = Pin { plain: 0xFFFF8002, debug: 0xFFFF8002 };

/// `Block_Stage_Keys`. tests: tile_cache_port
pub const BLOCK_STAGE_KEYS: Pin = Pin { plain: 0xFFFFA854, debug: 0xFFFFA878 };

/// `Block_Stage_Next`. tests: tile_cache_port
pub const BLOCK_STAGE_NEXT: Pin = Pin { plain: 0xFFFFA884, debug: 0xFFFFA8A8 };

/// `Block_Stage_Buffers`. tests: tile_cache_port
pub const BLOCK_STAGE_BUFFERS: Pin = Pin { plain: 0xFFFF3842, debug: 0xFFFF3842 };

/// `Cache_Fill_Last_Frame`. tests: tile_cache_port
pub const CACHE_FILL_LAST_FRAME: Pin = Pin { plain: 0xFFFFA844, debug: 0xFFFFA868 };

/// `Cache_Fill_Budget`. tests: tile_cache_port
pub const CACHE_FILL_BUDGET: Pin = Pin { plain: 0xFFFFA84A, debug: 0xFFFFA86E };

/// `Cache_Fill_Resume_Col`. tests: tile_cache_port
pub const CACHE_FILL_RESUME_COL: Pin = Pin { plain: 0xFFFFA846, debug: 0xFFFFA86A };

/// `Cache_Fill_Resume_Row`. tests: tile_cache_port
pub const CACHE_FILL_RESUME_ROW: Pin = Pin { plain: 0xFFFFA848, debug: 0xFFFFA86C };

/// `Cache_Fill_RowResume_Row`. tests: tile_cache_port
pub const CACHE_FILL_ROW_RESUME_ROW: Pin = Pin { plain: 0xFFFFA84C, debug: 0xFFFFA870 };

/// `Cache_Fill_RowResume_Col`. tests: tile_cache_port
pub const CACHE_FILL_ROW_RESUME_COL: Pin = Pin { plain: 0xFFFFA84E, debug: 0xFFFFA872 };

/// `Cache_Fill_Rows_Left`. tests: tile_cache_port
pub const CACHE_FILL_ROWS_LEFT: Pin = Pin { plain: 0xFFFFA850, debug: 0xFFFFA874 };

/// `Cache_Prev_Cam_Row`. tests: tile_cache_port
pub const CACHE_PREV_CAM_ROW: Pin = Pin { plain: 0xFFFFA852, debug: 0xFFFFA876 };

/// `S4LZ_DecompressDict`. tests: tile_cache_port
pub const S4_LZ_DECOMPRESS_DICT: Pin = Pin { plain: 0x231C, debug: 0x23AA };

/// `Player_1`. tests: collision_port, rings_port
pub const PLAYER_1: Pin = Pin { plain: 0xFFFF89EE, debug: 0xFFFF8A12 };

/// `Dynamic_Slots`. tests: collision_port
pub const DYNAMIC_SLOTS: Pin = Pin { plain: 0xFFFF8A8E, debug: 0xFFFF8AB2 };

/// `Ring_Buffer`. tests: rings_port
pub const RING_BUFFER: Pin = Pin { plain: 0xFFFFA8F8, debug: 0xFFFFA91C };

/// `Ring_Count`. tests: rings_port
pub const RING_COUNT: Pin = Pin { plain: 0xFFFFABF8, debug: 0xFFFFAC1C };

/// `Ring_HighWater`. tests: rings_port
pub const RING_HIGH_WATER: Pin = Pin { plain: 0xFFFFABF9, debug: 0xFFFFAC1D };

/// `Ring_Add_Dropped`. tests: rings_port
pub const RING_ADD_DROPPED: Pin = Pin { plain: 0xFFFFABFA, debug: 0xFFFFAC1E };

/// `Ring_Counter`. tests: rings_port
pub const RING_COUNTER: Pin = Pin { plain: 0xFFFFAC64, debug: 0xFFFFAC88 };

/// `Ring_Anim_Frame`. tests: rings_port
pub const RING_ANIM_FRAME: Pin = Pin { plain: 0xFFFFAC66, debug: 0xFFFFAC8A };

/// `Ring_Anim_Timer`. tests: rings_port
pub const RING_ANIM_TIMER: Pin = Pin { plain: 0xFFFFAC67, debug: 0xFFFFAC8B };

/// `Camera_X`. tests: rings_port, section_port
pub const CAMERA_X: Pin = Pin { plain: 0xFFFFA11E, debug: 0xFFFFA142 };

/// `Camera_Y`. tests: rings_port, section_port
pub const CAMERA_Y: Pin = Pin { plain: 0xFFFFA122, debug: 0xFFFFA146 };

/// `Camera_X_Biased`. tests: sprites_port
pub const CAMERA_X_BIASED: Pin = Pin { plain: 0xFFFFA126, debug: 0xFFFFA14A };

/// `Camera_Y_Biased`. tests: sprites_port
pub const CAMERA_Y_BIASED: Pin = Pin { plain: 0xFFFFA128, debug: 0xFFFFA14C };

/// `Collected_MarkRing`. tests: rings_port
pub const COLLECTED_MARK_RING: Pin = Pin { plain: 0x347E, debug: 0x3C3C };

/// `EntityWindow_EntryForSection`. tests: rings_port
pub const ENTITY_WINDOW_ENTRY_FOR_SECTION: Pin = Pin { plain: 0x369A, debug: 0x411E };

/// `EntityLoaded_Clear`. tests: rings_port
pub const ENTITY_LOADED_CLEAR: Pin = Pin { plain: 0x3686, debug: 0x40A8 };

/// `Sound_PlayRing`. tests: rings_port
pub const SOUND_PLAY_RING: Pin = Pin { plain: 0x5F1C, debug: 0x792E };

/// `MDDBG__ErrorHandler` — debug-shape consumer only (`debug_only`). tests: rings_port
pub const MDDBG_ERROR_HANDLER: u32 = 0x66704;

/// `MDDBG__ErrorHandler_PagesController` — debug-shape consumer only (`debug_only`). tests: rings_port
pub const MDDBG_ERROR_HANDLER_PAGES_CONTROLLER: u32 = 0x674CA;

/// `QueueDMA_Important`. tests: dplc_port
pub const QUEUE_DMA_IMPORTANT: Pin = Pin { plain: 0x1D84, debug: 0x1E06 };

/// `QueueDMA_Deferrable`. tests: dplc_port
pub const QUEUE_DMA_DEFERRABLE: Pin = Pin { plain: 0x1D8E, debug: 0x1E10 };

/// `Object_RAM`. tests: core_port
pub const OBJECT_RAM: Pin = Pin { plain: 0xFFFF89EE, debug: 0xFFFF8A12 };

/// `System_Slots`. tests: core_port
pub const SYSTEM_SLOTS: Pin = Pin { plain: 0xFFFF970E, debug: 0xFFFF9732 };

/// `Effect_Slots`. tests: core_port
pub const EFFECT_SLOTS: Pin = Pin { plain: 0xFFFF998E, debug: 0xFFFF99B2 };

/// `Spawn_Count`. tests: core_port
pub const SPAWN_COUNT: Pin = Pin { plain: 0xFFFF9F02, debug: 0xFFFF9F26 };

/// `Game_Paused`. tests: core_port
pub const GAME_PAUSED: Pin = Pin { plain: 0xFFFFA12A, debug: 0xFFFFA14E };

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
pub const DYNAMIC_LIVE: Pin = Pin { plain: 0xFFFFAFF0, debug: 0xFFFFB014 };

/// `Dynamic_Live_Count`. tests: core_port
pub const DYNAMIC_LIVE_COUNT: Pin = Pin { plain: 0xFFFFB040, debug: 0xFFFFB064 };

/// `Dynamic_Live_Dirty`. tests: core_port
pub const DYNAMIC_LIVE_DIRTY: Pin = Pin { plain: 0xFFFFB042, debug: 0xFFFFB066 };

/// `Dynamic_Live_Walking` — debug-shape consumer only (`debug_only`). tests: core_port, collision_port, entity_window_port
pub const DYNAMIC_LIVE_WALKING: u32 = 0xFFFFB067;

/// `Dynamic_Live_Pending`. tests: core_port
pub const DYNAMIC_LIVE_PENDING: Pin = Pin { plain: 0xFFFFB044, debug: 0xFFFFB068 };

/// `Dynamic_Live_Pending_Count`. tests: core_port
pub const DYNAMIC_LIVE_PENDING_COUNT: Pin = Pin { plain: 0xFFFFB054, debug: 0xFFFFB078 };

/// `DeleteObject`. tests: animate_port
pub const DELETE_OBJECT: Pin = Pin { plain: 0x2880, debug: 0x2A12 };

/// `DrawRings`. tests: sprites_port
pub const DRAW_RINGS: Pin = Pin { plain: 0x32CA, debug: 0x3A26 };

/// `Sprite_Table_Buffer`. tests: sprites_port
pub const SPRITE_TABLE_BUFFER: Pin = Pin { plain: 0xFFFF8288, debug: 0xFFFF8288 };

/// `Sprite_Table_Dirty`. tests: sprites_port
pub const SPRITE_TABLE_DIRTY: Pin = Pin { plain: 0xFFFF8508, debug: 0xFFFF8508 };

/// `Sprite_Bands`. tests: sprites_port
pub const SPRITE_BANDS: Pin = Pin { plain: 0xFFFF9F04, debug: 0xFFFF9F28 };

/// `Sprite_Band_Counts`. tests: sprites_port
pub const SPRITE_BAND_COUNTS: Pin = Pin { plain: 0xFFFFA104, debug: 0xFFFFA128 };

/// `Sprites_Rendered`. tests: sprites_port
pub const SPRITES_RENDERED: Pin = Pin { plain: 0xFFFFA10C, debug: 0xFFFFA130 };

/// `Sprite_Cycle_Counter`. tests: sprites_port
pub const SPRITE_CYCLE_COUNTER: Pin = Pin { plain: 0xFFFFA10E, debug: 0xFFFFA132 };

/// `SpriteMask_Y`. tests: sprites_port
pub const SPRITE_MASK_Y: Pin = Pin { plain: 0xFFFFA110, debug: 0xFFFFA134 };

/// `SpriteMask_Height`. tests: sprites_port
pub const SPRITE_MASK_HEIGHT: Pin = Pin { plain: 0xFFFFA112, debug: 0xFFFFA136 };

/// `SpriteMask_After_Band`. tests: sprites_port
pub const SPRITE_MASK_AFTER_BAND: Pin = Pin { plain: 0xFFFFA114, debug: 0xFFFFA138 };

/// `Scanline_Band_Sprites`. tests: sprites_port
pub const SCANLINE_BAND_SPRITES: Pin = Pin { plain: 0xFFFFA116, debug: 0xFFFFA13A };

/// `Sound_PlaySFX`. tests: animate_port
pub const SOUND_PLAY_SFX: Pin = Pin { plain: 0x5E86, debug: 0x7852 };

/// `ObjCodeBase`. tests: test_objects_port
pub const OBJ_CODE_BASE: Pin = Pin { plain: 0x10000, debug: 0x10000 };

/// `Draw_Sprite`. tests: test_objects_port
pub const DRAW_SPRITE: Pin = Pin { plain: 0x2AB2, debug: 0x308C };

/// `ObjectMove`. tests: test_objects_port
pub const OBJECT_MOVE: Pin = Pin { plain: 0x2A64, debug: 0x303E };

/// `Ring_Sfx_Speaker`. tests: sound_api_port
pub const RING_SFX_SPEAKER: Pin = Pin { plain: 0xFFFFAF34, debug: 0xFFFFAF58 };

/// `Sfx_Ring_Buf`. tests: sound_api_port
pub const SFX_RING_BUF: Pin = Pin { plain: 0xFFFFAF36, debug: 0xFFFFAF5A };

/// `Sfx_Ring_Wr`. tests: sound_api_port
pub const SFX_RING_WR: Pin = Pin { plain: 0xFFFFAF3E, debug: 0xFFFFAF62 };

/// `Sfx_Ring_Rd`. tests: sound_api_port
pub const SFX_RING_RD: Pin = Pin { plain: 0xFFFFAF3F, debug: 0xFFFFAF63 };

/// `SongTable`. tests: sound_api_port
pub const SONG_TABLE: Pin = Pin { plain: 0x63AE0, debug: 0x65522 };

/// `SongPatchTable`. tests: sound_api_port
pub const SONG_PATCH_TABLE: Pin = Pin { plain: 0x63AE4, debug: 0x6552E };

/// `OJZ_Palette`. tests: act_descriptor_port
pub const OJZ_PALETTE: Pin = Pin { plain: 0x1FE5C, debug: 0x1FEC4 };

/// `OJZ_Act1_BG_Layout`. tests: act_descriptor_port
pub const OJZ_ACT1_BG_LAYOUT: Pin = Pin { plain: 0x1FEDC, debug: 0x1FF44 };

/// `OJZ_Act1_BG_Tiles`. tests: act_descriptor_port
pub const OJZ_ACT1_BG_TILES: Pin = Pin { plain: 0x21EDC, debug: 0x21F44 };

/// `ParallaxConfig_OJZ_Default`. tests: act_descriptor_port
pub const PARALLAX_CONFIG_OJZ_DEFAULT: Pin = Pin { plain: 0x113C0, debug: 0x11428 };

/// `OJZ_Act_Pool_PageTable`. tests: act_descriptor_port
pub const OJZ_ACT_POOL_PAGE_TABLE: Pin = Pin { plain: 0x14B52, debug: 0x14BBA };

/// `OJZ_Sec0_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC0_BLOCKS: Pin = Pin { plain: 0x14DD2, debug: 0x14E3A };

/// `OJZ_Sec1_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC1_BLOCKS: Pin = Pin { plain: 0x169C2, debug: 0x16A2A };

/// `OJZ_Sec2_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC2_BLOCKS: Pin = Pin { plain: 0x17D3E, debug: 0x17DA6 };

/// `OJZ_Sec3_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC3_BLOCKS: Pin = Pin { plain: 0x194D6, debug: 0x1953E };

/// `OJZ_Sec4_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC4_BLOCKS: Pin = Pin { plain: 0x17D3E, debug: 0x17DA6 };

/// `OJZ_Sec5_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC5_BLOCKS: Pin = Pin { plain: 0x1A622, debug: 0x1A68A };

/// `OJZ_Sec6_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC6_BLOCKS: Pin = Pin { plain: 0x1B448, debug: 0x1B4B0 };

/// `OJZ_Sec7_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC7_BLOCKS: Pin = Pin { plain: 0x1D048, debug: 0x1D0B0 };

/// `OJZ_Sec8_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC8_BLOCKS: Pin = Pin { plain: 0x1E2BC, debug: 0x1E324 };

/// `OJZ_Sec0_Objects`. tests: act_descriptor_port
pub const OJZ_SEC0_OBJECTS: Pin = Pin { plain: 0x11DB8, debug: 0x11E20 };

/// `OJZ_Sec0_Rings`. tests: act_descriptor_port
pub const OJZ_SEC0_RINGS: Pin = Pin { plain: 0x11DC0, debug: 0x11E28 };

/// `OJZ_Sec0_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC0_TYPE_TABLE: Pin = Pin { plain: 0x11DB2, debug: 0x11E1A };

/// `OJZ_Sec1_Objects`. tests: act_descriptor_port
pub const OJZ_SEC1_OBJECTS: Pin = Pin { plain: 0x11DEA, debug: 0x11E52 };

/// `OJZ_Sec1_Rings`. tests: act_descriptor_port
pub const OJZ_SEC1_RINGS: Pin = Pin { plain: 0x11DFE, debug: 0x11E66 };

/// `OJZ_Sec1_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC1_TYPE_TABLE: Pin = Pin { plain: 0x11DE0, debug: 0x11E48 };

/// `OJZ_Sec2_Objects`. tests: act_descriptor_port
pub const OJZ_SEC2_OBJECTS: Pin = Pin { plain: 0x11E30, debug: 0x11E98 };

/// `OJZ_Sec2_Rings`. tests: act_descriptor_port
pub const OJZ_SEC2_RINGS: Pin = Pin { plain: 0x11E3E, debug: 0x11EA6 };

/// `OJZ_Sec2_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC2_TYPE_TABLE: Pin = Pin { plain: 0x11E26, debug: 0x11E8E };

/// `OJZ_Sec3_Objects`. tests: act_descriptor_port
pub const OJZ_SEC3_OBJECTS: Pin = Pin { plain: 0x11E74, debug: 0x11EDC };

/// `OJZ_Sec3_Rings`. tests: act_descriptor_port
pub const OJZ_SEC3_RINGS: Pin = Pin { plain: 0x11E76, debug: 0x11EDE };

/// `OJZ_Sec3_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC3_TYPE_TABLE: Pin = Pin { plain: 0x11E72, debug: 0x11EDA };

/// `OJZ_Sec4_Objects`. tests: act_descriptor_port
pub const OJZ_SEC4_OBJECTS: Pin = Pin { plain: 0x11E7C, debug: 0x11EE4 };

/// `OJZ_Sec4_Rings`. tests: act_descriptor_port
pub const OJZ_SEC4_RINGS: Pin = Pin { plain: 0x11E7E, debug: 0x11EE6 };

/// `OJZ_Sec4_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC4_TYPE_TABLE: Pin = Pin { plain: 0x11E7A, debug: 0x11EE2 };

/// `OJZ_Sec5_Objects`. tests: act_descriptor_port
pub const OJZ_SEC5_OBJECTS: Pin = Pin { plain: 0x11EB4, debug: 0x11F1C };

/// `OJZ_Sec5_Rings`. tests: act_descriptor_port
pub const OJZ_SEC5_RINGS: Pin = Pin { plain: 0x11EB6, debug: 0x11F1E };

/// `OJZ_Sec5_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC5_TYPE_TABLE: Pin = Pin { plain: 0x11EB2, debug: 0x11F1A };

/// `OJZ_Sec6_Objects`. tests: act_descriptor_port
pub const OJZ_SEC6_OBJECTS: Pin = Pin { plain: 0x11EDC, debug: 0x11F44 };

/// `OJZ_Sec6_Rings`. tests: act_descriptor_port
pub const OJZ_SEC6_RINGS: Pin = Pin { plain: 0x11EDE, debug: 0x11F46 };

/// `OJZ_Sec6_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC6_TYPE_TABLE: Pin = Pin { plain: 0x11EDA, debug: 0x11F42 };

/// `OJZ_Sec7_Objects`. tests: act_descriptor_port
pub const OJZ_SEC7_OBJECTS: Pin = Pin { plain: 0x11EE4, debug: 0x11F4C };

/// `OJZ_Sec7_Rings`. tests: act_descriptor_port
pub const OJZ_SEC7_RINGS: Pin = Pin { plain: 0x11EE6, debug: 0x11F4E };

/// `OJZ_Sec7_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC7_TYPE_TABLE: Pin = Pin { plain: 0x11EE2, debug: 0x11F4A };

/// `OJZ_Sec8_Objects`. tests: act_descriptor_port
pub const OJZ_SEC8_OBJECTS: Pin = Pin { plain: 0x11F0C, debug: 0x11F74 };

/// `OJZ_Sec8_Rings`. tests: act_descriptor_port
pub const OJZ_SEC8_RINGS: Pin = Pin { plain: 0x11F0E, debug: 0x11F76 };

/// `OJZ_Sec8_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC8_TYPE_TABLE: Pin = Pin { plain: 0x11F0A, debug: 0x11F72 };

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
pub const CAMERA_Y_COARSE_PREV: Pin = Pin { plain: 0xFFFFAD74, debug: 0xFFFFAD98 };

/// `Current_Act_Ptr`. tests: entity_window_port, section_port
pub const CURRENT_ACT_PTR: Pin = Pin { plain: 0xFFFFAF30, debug: 0xFFFFAF54 };

/// `Entity_Window_Active`. tests: entity_window_port
pub const ENTITY_WINDOW_ACTIVE: Pin = Pin { plain: 0xFFFFAC68, debug: 0xFFFFAC8C };

/// `Entity_Window_Anchor`. tests: entity_window_port
pub const ENTITY_WINDOW_ANCHOR: Pin = Pin { plain: 0xFFFFAC6A, debug: 0xFFFFAC8E };

/// `Entity_Window_OriginX`. tests: entity_window_port
pub const ENTITY_WINDOW_ORIGIN_X: Pin = Pin { plain: 0xFFFFAC6C, debug: 0xFFFFAC90 };

/// `Entity_Window_OriginY`. tests: entity_window_port
pub const ENTITY_WINDOW_ORIGIN_Y: Pin = Pin { plain: 0xFFFFAC6E, debug: 0xFFFFAC92 };

/// `Entity_Window_Center_ID`. tests: entity_window_port
pub const ENTITY_WINDOW_CENTER_ID: Pin = Pin { plain: 0xFFFFAC69, debug: 0xFFFFAC8D };

/// `Entity_Scan_State`. tests: entity_window_port
pub const ENTITY_SCAN_STATE: Pin = Pin { plain: 0xFFFFABFC, debug: 0xFFFFAC20 };

/// `Entity_Loaded_Masks`. tests: entity_window_port
pub const ENTITY_LOADED_MASKS: Pin = Pin { plain: 0xFFFFAC70, debug: 0xFFFFAC94 };

/// `Entity_Mask_Scratch`. tests: entity_window_port
pub const ENTITY_MASK_SCRATCH: Pin = Pin { plain: 0xFFFFACF0, debug: 0xFFFFAD14 };

/// `Ring_Collected_Window`. tests: entity_window_port
pub const RING_COLLECTED_WINDOW: Pin = Pin { plain: 0xFFFFAD76, debug: 0xFFFFAD9A };

/// `Ring_Collected_Park`. tests: entity_window_port
pub const RING_COLLECTED_PARK: Pin = Pin { plain: 0xFFFFAEAA, debug: 0xFFFFAECE };

/// `Collected_Park_Next`. tests: entity_window_port
pub const COLLECTED_PARK_NEXT: Pin = Pin { plain: 0xFFFFAF2E, debug: 0xFFFFAF52 };

/// `RingBuffer_Clear`. tests: entity_window_port
pub const RING_BUFFER_CLEAR: Pin = Pin { plain: 0x32BC, debug: 0x3A18 };

/// `RingBuffer_Remove`. tests: entity_window_port
pub const RING_BUFFER_REMOVE: Pin = Pin { plain: 0x3288, debug: 0x39E4 };

/// `Section_GetSecPtrXY`. tests: entity_window_port
pub const SECTION_GET_SEC_PTR_XY: Pin = Pin { plain: 0x518E, debug: 0x5E10 };

/// `Section_FlatIDXY`. tests: entity_window_port
pub const SECTION_FLAT_IDXY: Pin = Pin { plain: 0x5174, debug: 0x5DF6 };

/// `AllocDynamic`. tests: load_object_port
pub const ALLOC_DYNAMIC: Pin = Pin { plain: 0x2802, debug: 0x2994 };

// ── Region-relative offsets (manifest order) ──

/// `AnimateSprite.cc_delete` − `animate` start (per-shape). tests: animate_port
pub const CC_DELETE_OFF: ShapeOffset = ShapeOffset { plain: 0x104, debug: 0x15E };

/// `RefreshSpritePieceCount` − `animate` start (per-shape). tests: animate_port
pub const REFRESH_OFF: ShapeOffset = ShapeOffset { plain: 0x16C, debug: 0x28A };

/// `RingCollision` − `rings` start (per-shape). tests: rings_port
pub const RINGCOL_OFF: ShapeOffset = ShapeOffset { plain: 0x116, debug: 0x172 };

/// `Sound_PlaySFX` − `sound_api` start (per-shape). tests: sound_api_port
pub const SOUND_PLAY_SFX_OFF: ShapeOffset = ShapeOffset { plain: 0x100, debug: 0x1B0 };

/// `Sine_Table` − `math` start (shape-invariant, asserted at generation). tests: math_port
pub const SINE_TABLE_OFF: usize = 0x18;

/// `Flush_VDP_Shadow` − `vdp_init` start (shape-invariant, asserted at generation). tests: vdp_init_port
pub const FLUSH_VDP_SHADOW_OFF: usize = 0x16;

/// `HBlank_Null` − `hblank` start (shape-invariant, asserted at generation). tests: hblank_port
pub const HBLANK_NULL_OFF: usize = 0x10;
