//! GENERATED FILE — DO NOT EDIT BY HAND.
//!
//! Emitted by `cargo run -p sigil-harness --bin repin` from `repin.toml`
//! + the aeon listings (D-T10.3, tranche-10 step 0). Edit the MANIFEST,
//! then regenerate; `tests/repin_pins.rs::pins_rs_is_current` guards
//! staleness. All values are LISTING truth — per-shape VMAs/lengths from
//! `s4.lst` (plain) and `s4.debug.lst` (`__DEBUG__`).
//!
//! [provenance] plain: /home/volence/sonic_hacks/aeon/s4.lst (07/12/2026 02:32:46 PM)
//! [provenance] debug: /home/volence/sonic_hacks/aeon/s4.debug.lst (07/12/2026 02:32:40 PM)
//! [provenance] 18 regions, 138 symbols, 7 offsets

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
pub const ASSEMBLED_LEN: usize = 0x65A94;
/// Assembled (pre-convsym) ROM length, `__DEBUG__` shape. tests: m1d_rom, m1d_debug_rom, mixed_dac_rom
pub const DEBUG_ASSEMBLED_LEN: usize = 0x67582;

// ── Regions (manifest order) ──

/// `VDP_Shadow_Init` .. `Init_DMA_Queue` — gate `SIGIL_EMP_VDP_INIT`. tests: vdp_init_port
pub const VDP_INIT: Region = Region { plain_base: 0x1C14, debug_base: 0x1C96, plain_len: 0x48, debug_len: 0x48 };

/// `HBlank_Dispatch` .. `Read_Controllers` — gate `SIGIL_EMP_HBLANK`. tests: hblank_port, m1c_vector_table
pub const HBLANK: Region = Region { plain_base: 0x227A, debug_base: 0x2308, plain_len: 0x12, debug_len: 0x12 };

/// `Read_Controllers` .. `GameLoop` — gate `SIGIL_EMP_CONTROLLERS`. tests: controllers_port
pub const CONTROLLERS: Region = Region { plain_base: 0x228C, debug_base: 0x231A, plain_len: 0x72, debug_len: 0x72 };

/// `GameLoop` .. `S4LZ_DecompressDict` — gate `SIGIL_EMP_GAME_LOOP`. tests: game_loop_port
pub const GAME_LOOP: Region = Region { plain_base: 0x22FE, debug_base: 0x238C, plain_len: 0x12, debug_len: 0x12 };

/// `GetSineCosine` .. `Perform_DPLC` — gate `SIGIL_EMP_MATH`. tests: math_port
pub const MATH: Region = Region { plain_base: 0x2464, debug_base: 0x25F6, plain_len: 0x298, debug_len: 0x298 };

/// `Perform_DPLC` .. `InitObjectRAM` — gate `SIGIL_EMP_DPLC`. tests: dplc_port
pub const DPLC: Region = Region { plain_base: 0x26FC, debug_base: 0x288E, plain_len: 0x98, debug_len: 0x98 };

/// `InitObjectRAM` .. `InitSpriteSystem` — gate `SIGIL_EMP_CORE`. tests: core_port
pub const CORE: Region = Region { plain_base: 0x2794, debug_base: 0x2926, plain_len: 0x27C, debug_len: 0x3A0 };

/// `InitSpriteSystem` .. `AnimateSprite` — gate `SIGIL_EMP_SPRITES`. tests: sprites_port
pub const SPRITES: Region = Region { plain_base: 0x2A10, debug_base: 0x2CC6, plain_len: 0x420, debug_len: 0x420 };

/// `AnimateSprite` .. `TouchResponse` — gate `SIGIL_EMP_ANIMATE`. tests: animate_port, test_objects_port
pub const ANIMATE: Region = Region { plain_base: 0x2E30, debug_base: 0x30E6, plain_len: 0x192, debug_len: 0x192 };

/// `TouchResponse` .. `RingBuffer_Add` — gate `SIGIL_EMP_COLLISION`. tests: collision_port
pub const COLLISION: Region = Region { plain_base: 0x2FC2, debug_base: 0x3278, plain_len: 0x200, debug_len: 0x200 };

/// `RingBuffer_Add` .. `Collected_Init` — gate `SIGIL_EMP_RINGS`. tests: rings_port
pub const RINGS: Region = Region { plain_base: 0x31C2, debug_base: 0x3478, plain_len: 0x1BE, debug_len: 0x21A };

/// `Collision_GetType` .. `Collision_ProbeDown` — gate `SIGIL_EMP_COLLISION_LOOKUP`. tests: collision_lookup_port
pub const COLLISION_LOOKUP: Region = Region { plain_base: 0x4BDA, debug_base: 0x53FA, plain_len: 0x24, debug_len: 0x24 };

/// `Sound_PostByte` .. start + 0x1E4 (no end symbol in the listing) — gate `SIGIL_EMP_SOUND_API`. tests: sound_api_port
pub const SOUND_API: Region = Region { plain_base: 0x5D50, debug_base: 0x720A, plain_len: 0x1E4, debug_len: 0x1E4 };

/// `TestSolid_Init` .. `TestParticle` — gate `SIGIL_EMP_TEST_OBJECTS`. tests: test_objects_port
pub const TEST_SOLID: Region = Region { plain_base: 0x10F7C, debug_base: 0x10F7C, plain_len: 0xE, debug_len: 0xE };

/// `TestParticle` .. `TestEmitter` — gate `SIGIL_EMP_TEST_OBJECTS`. tests: test_objects_port
pub const TEST_PARTICLE: Region = Region { plain_base: 0x10F8A, debug_base: 0x10F8A, plain_len: 0x52, debug_len: 0x52 };

/// `Ani_Sonic` .. `Ani_Sonic_End` — gate `SIGIL_EMP_SONIC_ANIMS`. tests: sonic_anims_port
pub const SONIC_ANIMS: Region = Region { plain_base: 0x30970, debug_base: 0x309D8, plain_len: 0x6E, debug_len: 0x6E };

/// `Ani_Particle` .. `Ani_Particle_End` — gate `SIGIL_EMP_PARTICLE_ANIMS`. tests: particle_anims_port, test_objects_port
pub const PARTICLE_ANIMS: Region = Region { plain_base: 0x309DE, debug_base: 0x30A46, plain_len: 0x8, debug_len: 0x8 };

/// `OJZ_Act1_Descriptor` .. `OJZ_Sec0_Blocks` — gate `SIGIL_EMP_ACT_DESCRIPTOR`. tests: act_descriptor_port
pub const ACT_DESCRIPTOR: Region = Region { plain_base: 0x14AE6, debug_base: 0x14B4E, plain_len: 0x274, debug_len: 0x274 };

// ── Symbols (manifest order) ──

/// `EntryPoint`. tests: m1c_vector_table
pub const ENTRY_POINT: Pin = Pin { plain: 0x200, debug: 0x200 };

/// `NullInterrupt`. tests: m1c_vector_table
pub const NULL_INTERRUPT: Pin = Pin { plain: 0x649E2, debug: 0x664D0 };

/// `BusError`. tests: m1c_vector_table
pub const BUS_ERROR: Pin = Pin { plain: 0x649E4, debug: 0x664D2 };

/// `AddressError`. tests: m1c_vector_table
pub const ADDRESS_ERROR: Pin = Pin { plain: 0x649FC, debug: 0x664EA };

/// `IllegalInstr`. tests: m1c_vector_table
pub const ILLEGAL_INSTR: Pin = Pin { plain: 0x64A18, debug: 0x66506 };

/// `ZeroDivide`. tests: m1c_vector_table
pub const ZERO_DIVIDE: Pin = Pin { plain: 0x64A3A, debug: 0x66528 };

/// `ChkInstr`. tests: m1c_vector_table
pub const CHK_INSTR: Pin = Pin { plain: 0x64A54, debug: 0x66542 };

/// `TrapvInstr`. tests: m1c_vector_table
pub const TRAPV_INSTR: Pin = Pin { plain: 0x64A72, debug: 0x66560 };

/// `PrivilegeViol`. tests: m1c_vector_table
pub const PRIVILEGE_VIOL: Pin = Pin { plain: 0x64A92, debug: 0x66580 };

/// `Trace`. tests: m1c_vector_table
pub const TRACE: Pin = Pin { plain: 0x64AB4, debug: 0x665A2 };

/// `Line1010Emu`. tests: m1c_vector_table
pub const LINE1010_EMU: Pin = Pin { plain: 0x64AC8, debug: 0x665B6 };

/// `Line1111Emu`. tests: m1c_vector_table
pub const LINE1111_EMU: Pin = Pin { plain: 0x64AE8, debug: 0x665D6 };

/// `ErrorExcept`. tests: m1c_vector_table
pub const ERROR_EXCEPT: Pin = Pin { plain: 0x64B08, debug: 0x665F6 };

/// `ErrorTrap`. tests: m1c_vector_table
pub const ERROR_TRAP: Pin = Pin { plain: 0x64B26, debug: 0x66614 };

/// `VBlank_Handler`. tests: m1c_vector_table
pub const V_BLANK_HANDLER: Pin = Pin { plain: 0x2152, debug: 0x21D8 };

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
pub const V_SYNC_WAIT: Pin = Pin { plain: 0x2262, debug: 0x22EC };

/// `Sound_DrainSfxRing`. tests: game_loop_port
pub const SOUND_DRAIN_SFX_RING: Pin = Pin { plain: 0x5E96, debug: 0x7350 };

/// `Game_State`. tests: game_loop_port
pub const GAME_STATE: Pin = Pin { plain: 0xFFFF8004, debug: 0xFFFF8004 };

/// `Cache_Left_Col`. tests: collision_lookup_port
pub const CACHE_LEFT_COL: Pin = Pin { plain: 0xFFFFA838, debug: 0xFFFFA85A };

/// `Tile_Cache_GetCollision`. tests: collision_lookup_port
pub const TILE_CACHE_GET_COLLISION: Pin = Pin { plain: 0x42F2, debug: 0x4A5A };

/// `Player_1`. tests: collision_port, rings_port
pub const PLAYER_1: Pin = Pin { plain: 0xFFFF89EE, debug: 0xFFFF8A10 };

/// `Dynamic_Slots`. tests: collision_port
pub const DYNAMIC_SLOTS: Pin = Pin { plain: 0xFFFF8A8E, debug: 0xFFFF8AB0 };

/// `Ring_Buffer`. tests: rings_port
pub const RING_BUFFER: Pin = Pin { plain: 0xFFFFA8F8, debug: 0xFFFFA91A };

/// `Ring_Count`. tests: rings_port
pub const RING_COUNT: Pin = Pin { plain: 0xFFFFABF8, debug: 0xFFFFAC1A };

/// `Ring_HighWater`. tests: rings_port
pub const RING_HIGH_WATER: Pin = Pin { plain: 0xFFFFABF9, debug: 0xFFFFAC1B };

/// `Ring_Add_Dropped`. tests: rings_port
pub const RING_ADD_DROPPED: Pin = Pin { plain: 0xFFFFABFA, debug: 0xFFFFAC1C };

/// `Ring_Counter`. tests: rings_port
pub const RING_COUNTER: Pin = Pin { plain: 0xFFFFAC64, debug: 0xFFFFAC86 };

/// `Ring_Anim_Frame`. tests: rings_port
pub const RING_ANIM_FRAME: Pin = Pin { plain: 0xFFFFAC66, debug: 0xFFFFAC88 };

/// `Ring_Anim_Timer`. tests: rings_port
pub const RING_ANIM_TIMER: Pin = Pin { plain: 0xFFFFAC67, debug: 0xFFFFAC89 };

/// `Camera_X`. tests: rings_port
pub const CAMERA_X: Pin = Pin { plain: 0xFFFFA11E, debug: 0xFFFFA140 };

/// `Camera_Y`. tests: rings_port
pub const CAMERA_Y: Pin = Pin { plain: 0xFFFFA122, debug: 0xFFFFA144 };

/// `Camera_X_Biased`. tests: sprites_port
pub const CAMERA_X_BIASED: Pin = Pin { plain: 0xFFFFA126, debug: 0xFFFFA148 };

/// `Camera_Y_Biased`. tests: sprites_port
pub const CAMERA_Y_BIASED: Pin = Pin { plain: 0xFFFFA128, debug: 0xFFFFA14A };

/// `Collected_MarkRing`. tests: rings_port
pub const COLLECTED_MARK_RING: Pin = Pin { plain: 0x3404, debug: 0x3778 };

/// `EntityWindow_EntryForSection`. tests: rings_port
pub const ENTITY_WINDOW_ENTRY_FOR_SECTION: Pin = Pin { plain: 0x3628, debug: 0x3C5A };

/// `EntityLoaded_Clear`. tests: rings_port
pub const ENTITY_LOADED_CLEAR: Pin = Pin { plain: 0x3614, debug: 0x3BE4 };

/// `Sound_PlayRing`. tests: rings_port
pub const SOUND_PLAY_RING: Pin = Pin { plain: 0x5EE6, debug: 0x73A0 };

/// `MDDBG__ErrorHandler` — debug-shape consumer only (`debug_only`). tests: rings_port
pub const MDDBG_ERROR_HANDLER: u32 = 0x6662C;

/// `MDDBG__ErrorHandler_PagesController` — debug-shape consumer only (`debug_only`). tests: rings_port
pub const MDDBG_ERROR_HANDLER_PAGES_CONTROLLER: u32 = 0x673F2;

/// `QueueDMA_Important`. tests: dplc_port
pub const QUEUE_DMA_IMPORTANT: Pin = Pin { plain: 0x1D84, debug: 0x1E06 };

/// `QueueDMA_Deferrable`. tests: dplc_port
pub const QUEUE_DMA_DEFERRABLE: Pin = Pin { plain: 0x1D8E, debug: 0x1E10 };

/// `Object_RAM`. tests: core_port
pub const OBJECT_RAM: Pin = Pin { plain: 0xFFFF89EE, debug: 0xFFFF8A10 };

/// `System_Slots`. tests: core_port
pub const SYSTEM_SLOTS: Pin = Pin { plain: 0xFFFF970E, debug: 0xFFFF9730 };

/// `Effect_Slots`. tests: core_port
pub const EFFECT_SLOTS: Pin = Pin { plain: 0xFFFF998E, debug: 0xFFFF99B0 };

/// `Spawn_Count`. tests: core_port
pub const SPAWN_COUNT: Pin = Pin { plain: 0xFFFF9F02, debug: 0xFFFF9F24 };

/// `Game_Paused`. tests: core_port
pub const GAME_PAUSED: Pin = Pin { plain: 0xFFFFA12A, debug: 0xFFFFA14C };

/// `Object_RAM_End`. tests: core_port
pub const OBJECT_RAM_END: Pin = Pin { plain: 0xFFFF9E8E, debug: 0xFFFF9EB0 };

/// `Dynamic_Free_Stack`. tests: core_port
pub const DYNAMIC_FREE_STACK: Pin = Pin { plain: 0xFFFF9E8E, debug: 0xFFFF9EB0 };

/// `Dynamic_Free_SP`. tests: core_port
pub const DYNAMIC_FREE_SP: Pin = Pin { plain: 0xFFFF9EDE, debug: 0xFFFF9F00 };

/// `Effect_Free_Stack`. tests: core_port
pub const EFFECT_FREE_STACK: Pin = Pin { plain: 0xFFFF9EE0, debug: 0xFFFF9F02 };

/// `Effect_Free_SP`. tests: core_port
pub const EFFECT_FREE_SP: Pin = Pin { plain: 0xFFFF9F00, debug: 0xFFFF9F22 };

/// `Dynamic_Live`. tests: core_port
pub const DYNAMIC_LIVE: Pin = Pin { plain: 0xFFFFAFF0, debug: 0xFFFFB012 };

/// `Dynamic_Live_Count`. tests: core_port
pub const DYNAMIC_LIVE_COUNT: Pin = Pin { plain: 0xFFFFB040, debug: 0xFFFFB062 };

/// `Dynamic_Live_Dirty`. tests: core_port
pub const DYNAMIC_LIVE_DIRTY: Pin = Pin { plain: 0xFFFFB042, debug: 0xFFFFB064 };

/// `DeleteObject`. tests: animate_port
pub const DELETE_OBJECT: Pin = Pin { plain: 0x284E, debug: 0x29E0 };

/// `DrawRings`. tests: sprites_port
pub const DRAW_RINGS: Pin = Pin { plain: 0x3248, debug: 0x355A };

/// `Sprite_Table_Buffer`. tests: sprites_port
pub const SPRITE_TABLE_BUFFER: Pin = Pin { plain: 0xFFFF8288, debug: 0xFFFF8288 };

/// `Sprite_Table_Dirty`. tests: sprites_port
pub const SPRITE_TABLE_DIRTY: Pin = Pin { plain: 0xFFFF8508, debug: 0xFFFF8508 };

/// `Sprite_Bands`. tests: sprites_port
pub const SPRITE_BANDS: Pin = Pin { plain: 0xFFFF9F04, debug: 0xFFFF9F26 };

/// `Sprite_Band_Counts`. tests: sprites_port
pub const SPRITE_BAND_COUNTS: Pin = Pin { plain: 0xFFFFA104, debug: 0xFFFFA126 };

/// `Sprites_Rendered`. tests: sprites_port
pub const SPRITES_RENDERED: Pin = Pin { plain: 0xFFFFA10C, debug: 0xFFFFA12E };

/// `Sprite_Cycle_Counter`. tests: sprites_port
pub const SPRITE_CYCLE_COUNTER: Pin = Pin { plain: 0xFFFFA10E, debug: 0xFFFFA130 };

/// `SpriteMask_Y`. tests: sprites_port
pub const SPRITE_MASK_Y: Pin = Pin { plain: 0xFFFFA110, debug: 0xFFFFA132 };

/// `SpriteMask_Height`. tests: sprites_port
pub const SPRITE_MASK_HEIGHT: Pin = Pin { plain: 0xFFFFA112, debug: 0xFFFFA134 };

/// `SpriteMask_After_Band`. tests: sprites_port
pub const SPRITE_MASK_AFTER_BAND: Pin = Pin { plain: 0xFFFFA114, debug: 0xFFFFA136 };

/// `Scanline_Band_Sprites`. tests: sprites_port
pub const SCANLINE_BAND_SPRITES: Pin = Pin { plain: 0xFFFFA116, debug: 0xFFFFA138 };

/// `Sound_PlaySFX`. tests: animate_port
pub const SOUND_PLAY_SFX: Pin = Pin { plain: 0x5E50, debug: 0x730A };

/// `ObjCodeBase`. tests: test_objects_port
pub const OBJ_CODE_BASE: Pin = Pin { plain: 0x10000, debug: 0x10000 };

/// `Draw_Sprite`. tests: test_objects_port
pub const DRAW_SPRITE: Pin = Pin { plain: 0x2A28, debug: 0x2CDE };

/// `ObjectMove`. tests: test_objects_port
pub const OBJECT_MOVE: Pin = Pin { plain: 0x29DA, debug: 0x2C90 };

/// `Ring_Sfx_Speaker`. tests: sound_api_port
pub const RING_SFX_SPEAKER: Pin = Pin { plain: 0xFFFFAF34, debug: 0xFFFFAF56 };

/// `Sfx_Ring_Buf`. tests: sound_api_port
pub const SFX_RING_BUF: Pin = Pin { plain: 0xFFFFAF36, debug: 0xFFFFAF58 };

/// `Sfx_Ring_Wr`. tests: sound_api_port
pub const SFX_RING_WR: Pin = Pin { plain: 0xFFFFAF3E, debug: 0xFFFFAF60 };

/// `Sfx_Ring_Rd`. tests: sound_api_port
pub const SFX_RING_RD: Pin = Pin { plain: 0xFFFFAF3F, debug: 0xFFFFAF61 };

/// `SongTable`. tests: sound_api_port
pub const SONG_TABLE: Pin = Pin { plain: 0x63AE0, debug: 0x65522 };

/// `SongPatchTable`. tests: sound_api_port
pub const SONG_PATCH_TABLE: Pin = Pin { plain: 0x63AE4, debug: 0x6552E };

/// `OJZ_Palette`. tests: act_descriptor_port
pub const OJZ_PALETTE: Pin = Pin { plain: 0x1FDE4, debug: 0x1FE4C };

/// `OJZ_Act1_BG_Layout`. tests: act_descriptor_port
pub const OJZ_ACT1_BG_LAYOUT: Pin = Pin { plain: 0x1FE64, debug: 0x1FECC };

/// `OJZ_Act1_BG_Tiles`. tests: act_descriptor_port
pub const OJZ_ACT1_BG_TILES: Pin = Pin { plain: 0x21E64, debug: 0x21ECC };

/// `ParallaxConfig_OJZ_Default`. tests: act_descriptor_port
pub const PARALLAX_CONFIG_OJZ_DEFAULT: Pin = Pin { plain: 0x11348, debug: 0x113B0 };

/// `OJZ_Act_Pool_PageTable`. tests: act_descriptor_port
pub const OJZ_ACT_POOL_PAGE_TABLE: Pin = Pin { plain: 0x14ADA, debug: 0x14B42 };

/// `OJZ_Sec0_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC0_BLOCKS: Pin = Pin { plain: 0x14D5A, debug: 0x14DC2 };

/// `OJZ_Sec1_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC1_BLOCKS: Pin = Pin { plain: 0x1694A, debug: 0x169B2 };

/// `OJZ_Sec2_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC2_BLOCKS: Pin = Pin { plain: 0x17CC6, debug: 0x17D2E };

/// `OJZ_Sec3_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC3_BLOCKS: Pin = Pin { plain: 0x1945E, debug: 0x194C6 };

/// `OJZ_Sec4_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC4_BLOCKS: Pin = Pin { plain: 0x17CC6, debug: 0x17D2E };

/// `OJZ_Sec5_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC5_BLOCKS: Pin = Pin { plain: 0x1A5AA, debug: 0x1A612 };

/// `OJZ_Sec6_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC6_BLOCKS: Pin = Pin { plain: 0x1B3D0, debug: 0x1B438 };

/// `OJZ_Sec7_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC7_BLOCKS: Pin = Pin { plain: 0x1CFD0, debug: 0x1D038 };

/// `OJZ_Sec8_Blocks`. tests: act_descriptor_port
pub const OJZ_SEC8_BLOCKS: Pin = Pin { plain: 0x1E244, debug: 0x1E2AC };

/// `OJZ_Sec0_Objects`. tests: act_descriptor_port
pub const OJZ_SEC0_OBJECTS: Pin = Pin { plain: 0x11D40, debug: 0x11DA8 };

/// `OJZ_Sec0_Rings`. tests: act_descriptor_port
pub const OJZ_SEC0_RINGS: Pin = Pin { plain: 0x11D48, debug: 0x11DB0 };

/// `OJZ_Sec0_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC0_TYPE_TABLE: Pin = Pin { plain: 0x11D3A, debug: 0x11DA2 };

/// `OJZ_Sec1_Objects`. tests: act_descriptor_port
pub const OJZ_SEC1_OBJECTS: Pin = Pin { plain: 0x11D72, debug: 0x11DDA };

/// `OJZ_Sec1_Rings`. tests: act_descriptor_port
pub const OJZ_SEC1_RINGS: Pin = Pin { plain: 0x11D86, debug: 0x11DEE };

/// `OJZ_Sec1_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC1_TYPE_TABLE: Pin = Pin { plain: 0x11D68, debug: 0x11DD0 };

/// `OJZ_Sec2_Objects`. tests: act_descriptor_port
pub const OJZ_SEC2_OBJECTS: Pin = Pin { plain: 0x11DB8, debug: 0x11E20 };

/// `OJZ_Sec2_Rings`. tests: act_descriptor_port
pub const OJZ_SEC2_RINGS: Pin = Pin { plain: 0x11DC6, debug: 0x11E2E };

/// `OJZ_Sec2_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC2_TYPE_TABLE: Pin = Pin { plain: 0x11DAE, debug: 0x11E16 };

/// `OJZ_Sec3_Objects`. tests: act_descriptor_port
pub const OJZ_SEC3_OBJECTS: Pin = Pin { plain: 0x11DFC, debug: 0x11E64 };

/// `OJZ_Sec3_Rings`. tests: act_descriptor_port
pub const OJZ_SEC3_RINGS: Pin = Pin { plain: 0x11DFE, debug: 0x11E66 };

/// `OJZ_Sec3_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC3_TYPE_TABLE: Pin = Pin { plain: 0x11DFA, debug: 0x11E62 };

/// `OJZ_Sec4_Objects`. tests: act_descriptor_port
pub const OJZ_SEC4_OBJECTS: Pin = Pin { plain: 0x11E04, debug: 0x11E6C };

/// `OJZ_Sec4_Rings`. tests: act_descriptor_port
pub const OJZ_SEC4_RINGS: Pin = Pin { plain: 0x11E06, debug: 0x11E6E };

/// `OJZ_Sec4_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC4_TYPE_TABLE: Pin = Pin { plain: 0x11E02, debug: 0x11E6A };

/// `OJZ_Sec5_Objects`. tests: act_descriptor_port
pub const OJZ_SEC5_OBJECTS: Pin = Pin { plain: 0x11E3C, debug: 0x11EA4 };

/// `OJZ_Sec5_Rings`. tests: act_descriptor_port
pub const OJZ_SEC5_RINGS: Pin = Pin { plain: 0x11E3E, debug: 0x11EA6 };

/// `OJZ_Sec5_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC5_TYPE_TABLE: Pin = Pin { plain: 0x11E3A, debug: 0x11EA2 };

/// `OJZ_Sec6_Objects`. tests: act_descriptor_port
pub const OJZ_SEC6_OBJECTS: Pin = Pin { plain: 0x11E64, debug: 0x11ECC };

/// `OJZ_Sec6_Rings`. tests: act_descriptor_port
pub const OJZ_SEC6_RINGS: Pin = Pin { plain: 0x11E66, debug: 0x11ECE };

/// `OJZ_Sec6_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC6_TYPE_TABLE: Pin = Pin { plain: 0x11E62, debug: 0x11ECA };

/// `OJZ_Sec7_Objects`. tests: act_descriptor_port
pub const OJZ_SEC7_OBJECTS: Pin = Pin { plain: 0x11E6C, debug: 0x11ED4 };

/// `OJZ_Sec7_Rings`. tests: act_descriptor_port
pub const OJZ_SEC7_RINGS: Pin = Pin { plain: 0x11E6E, debug: 0x11ED6 };

/// `OJZ_Sec7_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC7_TYPE_TABLE: Pin = Pin { plain: 0x11E6A, debug: 0x11ED2 };

/// `OJZ_Sec8_Objects`. tests: act_descriptor_port
pub const OJZ_SEC8_OBJECTS: Pin = Pin { plain: 0x11E94, debug: 0x11EFC };

/// `OJZ_Sec8_Rings`. tests: act_descriptor_port
pub const OJZ_SEC8_RINGS: Pin = Pin { plain: 0x11E96, debug: 0x11EFE };

/// `OJZ_Sec8_TypeTable`. tests: act_descriptor_port
pub const OJZ_SEC8_TYPE_TABLE: Pin = Pin { plain: 0x11E92, debug: 0x11EFA };

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

// ── Region-relative offsets (manifest order) ──

/// `AnimateSprite.cc_delete` − `animate` start (shape-invariant, asserted at generation). tests: animate_port
pub const CC_DELETE_OFF: usize = 0x104;

/// `RefreshSpritePieceCount` − `animate` start (shape-invariant, asserted at generation). tests: animate_port
pub const REFRESH_OFF: usize = 0x174;

/// `RingCollision` − `rings` start (per-shape). tests: rings_port
pub const RINGCOL_OFF: ShapeOffset = ShapeOffset { plain: 0x11C, debug: 0x178 };

/// `Sound_PlaySFX` − `sound_api` start (shape-invariant, asserted at generation). tests: sound_api_port
pub const SOUND_PLAY_SFX_OFF: usize = 0x100;

/// `Sine_Table` − `math` start (shape-invariant, asserted at generation). tests: math_port
pub const SINE_TABLE_OFF: usize = 0x18;

/// `Flush_VDP_Shadow` − `vdp_init` start (shape-invariant, asserted at generation). tests: vdp_init_port
pub const FLUSH_VDP_SHADOW_OFF: usize = 0x16;

/// `HBlank_Null` − `hblank` start (shape-invariant, asserted at generation). tests: hblank_port
pub const HBLANK_NULL_OFF: usize = 0x10;
