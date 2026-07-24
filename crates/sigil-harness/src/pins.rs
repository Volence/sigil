//! GENERATED FILE — DO NOT EDIT BY HAND.
//!
//! Emitted by `cargo run -p sigil-harness --bin repin` from `repin.toml`
//! + the aeon listings (D-T10.3, tranche-10 step 0). Edit the MANIFEST,
//! then regenerate; `tests/repin_pins.rs::pins_rs_is_current` guards
//! staleness. All values are LISTING truth — per-shape VMAs/lengths from
//! `s4.lst` (plain) and `s4.debug.lst` (`__DEBUG__`).
//!
//! [provenance] plain: /home/volence/sonic_hacks/aeon/.worktrees/parallax-transition-parcel/s4.lst (07/23/2026 11:03:36 PM)
//! [provenance] debug: /home/volence/sonic_hacks/aeon/.worktrees/parallax-transition-parcel/s4.debug.lst (07/23/2026 11:04:23 PM)
//! [provenance] 25 regions, 209 symbols, 7 offsets

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

/// `VDP_Shadow_Init` .. `Init_DMA_Queue` — gate `SIGIL_EMP_VDP_INIT`. tests: vdp_init_port
pub const VDP_INIT: Region = Region { plain_base: 0x1C0A, debug_base: 0x1C8C, plain_len: 0x4A, debug_len: 0x4A };

/// `HBlank_Install` .. `Read_Controllers` — gate `SIGIL_EMP_HBLANK`. tests: hblank_port, m1c_vector_table
pub const HBLANK: Region = Region { plain_base: 0x22E8, debug_base: 0x2376, plain_len: 0x48, debug_len: 0x48 };

/// `Read_Controllers` .. `GameLoop` — gate `SIGIL_EMP_CONTROLLERS`. tests: controllers_port
pub const CONTROLLERS: Region = Region { plain_base: 0x2330, debug_base: 0x23BE, plain_len: 0x76, debug_len: 0x76 };

/// `GameLoop` .. `S4LZ_DecompressDict` — gate `SIGIL_EMP_GAME_LOOP`. tests: game_loop_port
pub const GAME_LOOP: Region = Region { plain_base: 0x23A6, debug_base: 0x2434, plain_len: 0x12, debug_len: 0x12 };

/// `GetSineCosine` .. `Perform_DPLC` — gate `SIGIL_EMP_MATH`. tests: math_port
pub const MATH: Region = Region { plain_base: 0x250C, debug_base: 0x269E, plain_len: 0x298, debug_len: 0x298 };

/// `Perform_DPLC` .. `InitObjectRAM` — gate `SIGIL_EMP_DPLC`. tests: dplc_port
pub const DPLC: Region = Region { plain_base: 0x27A4, debug_base: 0x2936, plain_len: 0xA4, debug_len: 0xA4 };

/// `InitObjectRAM` .. `InitSpriteSystem` — gate `SIGIL_EMP_CORE`. tests: core_port
pub const CORE: Region = Region { plain_base: 0x2848, debug_base: 0x29DA, plain_len: 0x2E4, debug_len: 0x72C };

/// `InitSpriteSystem` .. `AnimateSprite` — gate `SIGIL_EMP_SPRITES`. tests: sprites_port
pub const SPRITES: Region = Region { plain_base: 0x2B2C, debug_base: 0x3106, plain_len: 0x420, debug_len: 0x420 };

/// `AnimateSprite` .. `TouchResponse` — gate `SIGIL_EMP_ANIMATE`. tests: animate_port, test_objects_port
pub const ANIMATE: Region = Region { plain_base: 0x2F4C, debug_base: 0x3526, plain_len: 0x18A, debug_len: 0x2A8 };

/// `TouchResponse` .. `RingBuffer_Add` — gate `SIGIL_EMP_COLLISION`. tests: collision_port
pub const COLLISION: Region = Region { plain_base: 0x30D6, debug_base: 0x37CE, plain_len: 0x200, debug_len: 0x208 };

/// `RingBuffer_Add` .. `Collected_Init` — gate `SIGIL_EMP_RINGS`. tests: rings_port
pub const RINGS: Region = Region { plain_base: 0x32D6, debug_base: 0x39D6, plain_len: 0x1B8, debug_len: 0x214 };

/// `Collected_Init` .. `PopulateSpawnedPieceCount` — gate `SIGIL_EMP_ENTITY_WINDOW`. tests: entity_window_port
pub const ENTITY_WINDOW: Region = Region { plain_base: 0x348E, debug_base: 0x3BEA, plain_len: 0x8BA, debug_len: 0xD28 };

/// `Load_Object` .. `Plane_Buffer_Reset` — gate `SIGIL_EMP_LOAD_OBJECT`. tests: load_object_port, entity_window_port
pub const LOAD_OBJECT: Region = Region { plain_base: 0x4056, debug_base: 0x4C20, plain_len: 0x82, debug_len: 0x82 };

/// `Plane_Buffer_Reset` .. `Tile_Cache_GetTile` — gate `SIGIL_EMP_PLANE_BUFFER`. tests: plane_buffer_port
pub const PLANE_BUFFER: Region = Region { plain_base: 0x40D8, debug_base: 0x4CA2, plain_len: 0x2DA, debug_len: 0x2DA };

/// `Tile_Cache_GetTile` .. `Collision_GetType` — gate `SIGIL_EMP_TILE_CACHE`. tests: tile_cache_port
pub const TILE_CACHE: Region = Region { plain_base: 0x43B2, debug_base: 0x4F7C, plain_len: 0xD20, debug_len: 0xDE0 };

/// `Collision_GetType` .. `Collision_ProbeDown` — gate `SIGIL_EMP_COLLISION_LOOKUP`. tests: collision_lookup_port
pub const COLLISION_LOOKUP: Region = Region { plain_base: 0x50D2, debug_base: 0x5D5C, plain_len: 0x24, debug_len: 0x24 };

/// `Section_Init` .. `Camera_Init` — gate `SIGIL_EMP_SECTION`. tests: section_port
pub const SECTION: Region = Region { plain_base: 0x55F2, debug_base: 0x627C, plain_len: 0x3DC, debug_len: 0x3DC };

/// `Parallax_Init` .. `Art_Decompress` — gate `SIGIL_EMP_PARALLAX`. tests: parallax_port
pub const PARALLAX: Region = Region { plain_base: 0x5B38, debug_base: 0x67C2, plain_len: 0x552, debug_len: 0x552 };

/// `Sound_PostByte` .. start + 0x20A plain / 0x3B4 debug (literal — no end symbol) — gate `SIGIL_EMP_SOUND_API`. tests: sound_api_port
pub const SOUND_API: Region = Region { plain_base: 0x6240, debug_base: 0x7BAE, plain_len: 0x20A, debug_len: 0x3B4 };

/// `TestSolid_Init` .. `TestParticle` — gate `SIGIL_EMP_TEST_OBJECTS`. tests: test_objects_port
pub const TEST_SOLID: Region = Region { plain_base: 0x10F7C, debug_base: 0x10F7C, plain_len: 0xE, debug_len: 0xE };

/// `TestParticle` .. `TestEmitter` — gate `SIGIL_EMP_TEST_OBJECTS`. tests: test_objects_port
pub const TEST_PARTICLE: Region = Region { plain_base: 0x10F8A, debug_base: 0x10F8A, plain_len: 0x52, debug_len: 0x52 };

/// `Ani_Sonic` .. `Ani_Sonic_End` — gate `SIGIL_EMP_SONIC_ANIMS`. tests: sonic_anims_port
pub const SONIC_ANIMS: Region = Region { plain_base: 0x25710, debug_base: 0x25778, plain_len: 0x6E, debug_len: 0x6E };

/// `Ani_Particle` .. `Ani_Particle_End` — gate `SIGIL_EMP_PARTICLE_ANIMS`. tests: particle_anims_port, test_objects_port
pub const PARTICLE_ANIMS: Region = Region { plain_base: 0x2577E, debug_base: 0x257E6, plain_len: 0x8, debug_len: 0x8 };

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
pub const MAP_TEST_OBJ: Pin = Pin { plain: 0x256E0, debug: 0x25748 };

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
pub const V_BLANK_HANDLER: Pin = Pin { plain: 0x21B8, debug: 0x223E };

/// `HBlank_Vector_Slot`. tests: hblank_port, m1c_vector_table
pub const H_BLANK_VECTOR_SLOT: Pin = Pin { plain: 0xFFFFB074, debug: 0xFFFFB098 };

/// `VDP_Shadow_Table`. tests: vdp_init_port
pub const VDP_SHADOW_TABLE: Pin = Pin { plain: 0xFFFF800A, debug: 0xFFFF800A };

/// `VDP_Dirty_Mask`. tests: vdp_init_port
pub const VDP_DIRTY_MASK: Pin = Pin { plain: 0xFFFF801E, debug: 0xFFFF801E };

/// `BootData_VDPRegs`. tests: vdp_init_port
pub const BOOT_DATA_VDP_REGS: Pin = Pin { plain: 0x3C4, debug: 0x3C8 };

/// `Ctrl_1_Held`. tests: controllers_port
pub const CTRL_1_HELD: Pin = Pin { plain: 0xFFFF802C, debug: 0xFFFF802C };

/// `VSync_Wait`. tests: game_loop_port
pub const V_SYNC_WAIT: Pin = Pin { plain: 0x22C8, debug: 0x2352 };

/// `Sound_DrainSfxRing`. tests: game_loop_port
pub const SOUND_DRAIN_SFX_RING: Pin = Pin { plain: 0x63AC, debug: 0x7EC4 };

/// `Game_State`. tests: game_loop_port
pub const GAME_STATE: Pin = Pin { plain: 0xFFFF8004, debug: 0xFFFF8004 };

/// `Cache_Left_Col`. tests: collision_lookup_port, section_port
pub const CACHE_LEFT_COL: Pin = Pin { plain: 0xFFFFA836, debug: 0xFFFFA85A };

/// `Tile_Cache_GetCollision`. tests: collision_lookup_port
pub const TILE_CACHE_GET_COLLISION: Pin = Pin { plain: 0x43EE, debug: 0x4FB8 };

/// `Draw_TileColumn`. tests: section_port
pub const DRAW_TILE_COLUMN: Pin = Pin { plain: 0x40E0, debug: 0x4CAA };

/// `Draw_TileRow_FromCache`. tests: section_port
pub const DRAW_TILE_ROW_FROM_CACHE: Pin = Pin { plain: 0x4202, debug: 0x4DCC };

/// `EntityWindow_Init`. tests: section_port
pub const ENTITY_WINDOW_INIT: Pin = Pin { plain: 0x3844, debug: 0x431E };

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
pub const S4_LZ_DECOMPRESS_DICT: Pin = Pin { plain: 0x23B8, debug: 0x2446 };

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

/// `Camera_X`. tests: rings_port, section_port
pub const CAMERA_X: Pin = Pin { plain: 0xFFFFA11C, debug: 0xFFFFA140 };

/// `Camera_Y`. tests: rings_port, section_port
pub const CAMERA_Y: Pin = Pin { plain: 0xFFFFA120, debug: 0xFFFFA144 };

/// `Camera_X_Biased`. tests: sprites_port
pub const CAMERA_X_BIASED: Pin = Pin { plain: 0xFFFFA124, debug: 0xFFFFA148 };

/// `Camera_Y_Biased`. tests: sprites_port
pub const CAMERA_Y_BIASED: Pin = Pin { plain: 0xFFFFA126, debug: 0xFFFFA14A };

/// `Collected_MarkRing`. tests: rings_port
pub const COLLECTED_MARK_RING: Pin = Pin { plain: 0x3510, debug: 0x3CCE };

/// `EntityWindow_EntryForSection`. tests: rings_port
pub const ENTITY_WINDOW_ENTRY_FOR_SECTION: Pin = Pin { plain: 0x372C, debug: 0x41B0 };

/// `EntityLoaded_Clear`. tests: rings_port
pub const ENTITY_LOADED_CLEAR: Pin = Pin { plain: 0x3718, debug: 0x413A };

/// `Sound_PlayRing`. tests: rings_port
pub const SOUND_PLAY_RING: Pin = Pin { plain: 0x63FC, debug: 0x7F14 };

/// `MDDBG__ErrorHandler` — debug-shape consumer only (`debug_only`). tests: rings_port
pub const MDDBG_ERROR_HANDLER: u32 = 0x5E704;

/// `MDDBG__ErrorHandler_PagesController` — debug-shape consumer only (`debug_only`). tests: rings_port
pub const MDDBG_ERROR_HANDLER_PAGES_CONTROLLER: u32 = 0x5F4CA;

/// `QueueDMA_Important`. tests: dplc_port
pub const QUEUE_DMA_IMPORTANT: Pin = Pin { plain: 0x1D7C, debug: 0x1DFE };

/// `QueueDMA_Deferrable`. tests: dplc_port
pub const QUEUE_DMA_DEFERRABLE: Pin = Pin { plain: 0x1D86, debug: 0x1E08 };

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

/// `DeleteObject`. tests: animate_port
pub const DELETE_OBJECT: Pin = Pin { plain: 0x2918, debug: 0x2AAA };

/// `DrawRings`. tests: sprites_port
pub const DRAW_RINGS: Pin = Pin { plain: 0x335C, debug: 0x3AB8 };

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
pub const SOUND_PLAY_SFX: Pin = Pin { plain: 0x6366, debug: 0x7E38 };

/// `ObjCodeBase`. tests: test_objects_port
pub const OBJ_CODE_BASE: Pin = Pin { plain: 0x10000, debug: 0x10000 };

/// `Draw_Sprite`. tests: test_objects_port
pub const DRAW_SPRITE: Pin = Pin { plain: 0x2B40, debug: 0x311A };

/// `ObjectMove`. tests: test_objects_port
pub const OBJECT_MOVE: Pin = Pin { plain: 0x2AF6, debug: 0x30D0 };

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
pub const RING_BUFFER_CLEAR: Pin = Pin { plain: 0x334E, debug: 0x3AAA };

/// `RingBuffer_Remove`. tests: entity_window_port
pub const RING_BUFFER_REMOVE: Pin = Pin { plain: 0x331A, debug: 0x3A76 };

/// `Section_GetSecPtrXY`. tests: entity_window_port
pub const SECTION_GET_SEC_PTR_XY: Pin = Pin { plain: 0x5642, debug: 0x62CC };

/// `Section_FlatIDXY`. tests: entity_window_port
pub const SECTION_FLAT_IDXY: Pin = Pin { plain: 0x5628, debug: 0x62B2 };

/// `AllocDynamic`. tests: load_object_port
pub const ALLOC_DYNAMIC: Pin = Pin { plain: 0x289A, debug: 0x2A2C };

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
