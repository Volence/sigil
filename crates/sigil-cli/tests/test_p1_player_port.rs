//! Tranche 34 — game-side P1: the PLAYER KEYSTONE (player_common + sonic).
//! Region-level byte gates in BOTH shapes, modelled on `test_g3_objects_port.rs`.
//!
//! ## What this port opens
//!
//! - **Two per-file object-bank gates** `SIGIL_EMP_PLAYER_COMMON` /
//!   `SIGIL_EMP_SONIC`. Bank windows shape-invariant (base/len identical in
//!   s4.lst and s4.debug.lst); content bytes track per-shape cross-seam operands
//!   → compile twice. player_common is the object-bank FIRST code region (base =
//!   Player_Init just past ObjCodeBase's rts); its INTERNAL gate keeps the
//!   zero-byte header (PlayerV struct / _pl_* equates / macros) always-emitted for
//!   the surviving state files. sonic is the last player file before test_static.
//! - **The PlayerV 13-field GUARDED overlay** (row-61 DplcV class): 5 drift guards
//!   on the fields the surviving player_ground/air/spindash read
//!   (gsp/state/move_lock/spindash/stick_convex); the region gate covers the rest.
//! - **The Player_States cross-seam offset table** — 7 `extern(PState_X) -
//!   extern(Player_States)` entries (surviving state files), the t31 extern-extern
//!   form; PState_EnterHooks/ExitHooks reach .emp-local PHook_*.
//! - **vram_bytes HOISTED to objdef.emp** (2nd consumer, row 63) — sonic + the
//!   retrofitted test_animated share it.
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, every test here SKIPS green —
//! unless `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test test_p1_player_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_harness::test_support::{reference_tree, strict_gate};
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

/// The reference tree, or `None` (skip green) when it lacks a source either
/// compile helper reads. Every test opens with it — both helpers run BEFORE any
/// ROM read, so guarding only the ROM window would leave the sources unguarded.
fn ref_sources() -> Option<PathBuf> {
    reference_tree(&[
        "engine/coords.emp",
        "engine/objects/objdef.emp",
        "engine/objects/sst.emp",
        "engine/structs.emp",
        "engine/system/constants.emp",
        "engine/system/types.emp",
        "games/sonic4/config/constants.emp",
        "games/sonic4/player/player_common.emp",
        "games/sonic4/player/sonic.emp",
    ])
}

const OBJ_CODE_BASE: u32 = pins::OBJ_CODE_BASE.plain;

fn parse_file(path: &std::path::Path) -> sigil_frontend_emp::ast::File {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (file, diags) = parse_str(&src);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} parse errors: {diags:?}",
        path.display()
    );
    file
}

fn with_ambient(
    deps: Vec<Vec<sigil_frontend_emp::ast::Item>>,
    main: sigil_frontend_emp::ast::File,
) -> sigil_frontend_emp::ast::File {
    let mut items = Vec::new();
    for d in deps {
        items.extend(d);
    }
    items.extend(main.items);
    sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items,
        docs: main.docs.clone(),
    }
}

/// The AS-side value seam: SST field equs + engine constants + ObjCodeBase +
/// the player game-config + engine physics mirrors the ensures resolve against.
fn as_constant_equs() -> Vec<Section> {
    use sigil_harness::test_support::{engine_constant_equs, sst_field_equs};
    let mut pairs = sst_field_equs();
    pairs.extend(engine_constant_equs());
    let obj_code_base = format!("${:X}", OBJ_CODE_BASE);
    pairs.push(("ObjCodeBase", obj_code_base.as_str()));
    // game-config (config/constants.asm + config/sound_ids.asm) mirrors
    pairs.push(("VRAM_TEST_SONIC", "$03C0"));
    pairs.push(("VRAM_TEST_MARKER", "1016"));
    pairs.push(("PSTATE_GROUND", "0"));
    pairs.push(("PSTATE_ROLL", "2"));
    pairs.push(("PSTATE_SPINDASH", "4"));
    pairs.push(("PSTATE_AIR", "6"));
    pairs.push(("PSTATE_JUMP", "8"));
    pairs.push(("PSTATE_ROLLJUMP", "10"));
    pairs.push(("PSTATE_AIRBALL", "12"));
    pairs.push(("PSTATE_COUNT", "7"));
    pairs.push(("ANIM_WALK", "0"));
    pairs.push(("ANIM_RUN", "1"));
    pairs.push(("ANIM_ROLL", "2"));
    pairs.push(("ANIM_SPINDASH", "3"));
    pairs.push(("ANIM_PUSH", "4"));
    pairs.push(("ANIM_IDLE", "5"));
    pairs.push(("ANIM_BALANCE", "6"));
    pairs.push(("ANIM_LOOKUP", "7"));
    pairs.push(("ANIM_DUCK", "8"));
    pairs.push(("ANIM_SKID", "9"));
    pairs.push(("ANIM_GETUP", "10"));
    pairs.push(("ANIM_RUN_THRESHOLD", "$600"));
    pairs.push(("SFXID_SKID", "$36"));
    // The engine-truth PHYS_* / radii / ST_* / EDGE_* / COLLISION_NONE /
    // BUTTON_A/C / BUTTON_*_BIT block moved into `engine_constant_equs()` at the
    // ambient-hoist parcel (the engine.constants twin now owns the drift guards),
    // so it is supplied by the `engine_constant_equs()` extend above — pushing it
    // again here would double-define the equs.
    // The _pl_* overlay drift guards retired at conv-d #49 (player_common.asm
    // deleted — player_common.emp owns PlayerV outright). It now DEFINES _pl_state
    // as its own `equ` (link-exported for camera), so supplying an _pl_state equ
    // here would double-define it.
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// One synthetic AS-side label phased at `vma`.
fn as_label_at(name: &str, vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts)
        .unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}"))
        .sections
}

fn map_toml(name: &str, base: u32, len: usize) -> String {
    format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"{name}\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
    // A gate over an EMPTY image proves nothing, and the tolerance below would
    // hide that: with no candidate bytes it shrinks `expected` to zero length, the
    // length assert compares 0 == 0, and the diff loop runs over an empty range —
    // so the test passes if the module emits nothing at all. Confirmed live on
    // OJZ_BG_ANIM, a 14-byte all-zero plain window (lens sweep, seat GATE, S15).
    assert!(
        !candidate.is_empty(),
        "{what}: the module emitted NO BYTES — a region gate over an empty window \
         proves nothing. Either the module stopped emitting, or this pin should not exist."
    );
    // Packed placement (Wave-B B-0) may end a region window in ALIGNMENT FILL: the
    // pins span runs to the next section's aligned base. Tolerate a short all-zero
    // tail beyond the lowered image; every real byte still compares.
    //
    // Threshold is < 32 (was < 16): once the character roster moved out of
    // player_common into its own `characters` section, player_common's debug
    // window ends in 18 bytes of zero fill before the next section's aligned
    // base. 32 is the largest section alignment in play, so it is the principled
    // bound — verified all-zero, not a masked content gap.
    let expected = if expected.len() > candidate.len()
        && expected.len() - candidate.len() < 32
        && expected[candidate.len()..].iter().all(|&b| b == 0)
    {
        &expected[..candidate.len()]
    } else {
        expected
    };
    assert_eq!(
        candidate.len(),
        expected.len(),
        "{what}: length mismatch — candidate {} bytes, expected {} bytes",
        candidate.len(),
        expected.len()
    );
    if let Some(i) = (0..candidate.len()).find(|&i| candidate[i] != expected[i]) {
        let lo = i.saturating_sub(8);
        let hi = (i + 16).min(candidate.len());
        panic!(
            "{what}: first diff at offset {i:#x} (region-relative)\n  candidate[{lo:#x}..{hi:#x}]: {:02x?}\n  expected[{lo:#x}..{hi:#x}]:  {:02x?}",
            &candidate[lo..hi],
            &expected[lo..hi]
        );
    }
}

fn ref_window(aeon: &Path, rom: &str, base: u32, len: usize) -> Option<Vec<u8>> {
    let path = aeon.join(rom);
    match std::fs::read(&path) {
        Ok(bytes) => {
            let b = base as usize;
            Some(bytes[b..b + len].to_vec())
        }
        Err(_) => {
            if strict_gate() {
                panic!("SIGIL_STRICT_GATE set but reference missing: {}", path.display());
            }
            eprintln!("skip: reference ROM not at {} (set AEON_DIR)", path.display());
            None
        }
    }
}

// ─── SONIC region ────────────────────────────────────────────────────────────

struct SonicShape {
    perform_dplc: u32,
    ability_none: u32,
    draw_sprite: u32,
    map_sonic: u32,
    ani_sonic: u32,
    dplc_sonic: u32,
    art_sonic: u32,
    /// knuckles-def (C4 task 9): `CharDef_Sonic.cd_palette` — the shared CRAM
    /// line 0 Sonic and Tails both name (a real pointer, not 0, so the roster
    /// cycle restores it on the way back from Knuckles). It lives in
    /// knuckles_data, so it is cross-seam from `sonic`.
    pal_sonic_tails: u32,
    base: u32,
    len: usize,
}

const SONIC_PLAIN: SonicShape = SonicShape {
    perform_dplc: pins::DPLC.plain_base,
    ability_none: pins::ABILITY_NONE.plain,
    draw_sprite: pins::DRAW_SPRITE.plain,
    map_sonic: pins::MAP_SONIC.plain,
    ani_sonic: pins::SONIC_ANIMS.plain_base,
    dplc_sonic: pins::DPLC_SONIC.plain,
    art_sonic: pins::ART_SONIC.plain,
    pal_sonic_tails: pins::PAL_SONIC_TAILS.plain,
    base: pins::SONIC.plain_base,
    len: pins::SONIC.plain_len,
};
const SONIC_DEBUG: SonicShape = SonicShape {
    perform_dplc: pins::DPLC.debug_base,
    ability_none: pins::ABILITY_NONE.debug,
    draw_sprite: pins::DRAW_SPRITE.debug,
    map_sonic: pins::MAP_SONIC.debug,
    ani_sonic: pins::SONIC_ANIMS.debug_base,
    dplc_sonic: pins::DPLC_SONIC.debug,
    art_sonic: pins::ART_SONIC.debug,
    pal_sonic_tails: pins::PAL_SONIC_TAILS.debug,
    base: pins::SONIC.debug_base,
    len: pins::SONIC.debug_len,
};

fn compile_sonic(aeon: &Path, shape: &SonicShape) -> (sigil_link::LinkedImage, usize) {
    let types = || parse_file(&aeon.join("engine/system/types.emp")).items;
    let sst = || parse_file(&aeon.join("engine/objects/sst.emp")).items;
    let constants = || parse_file(&aeon.join("engine/system/constants.emp")).items;
    let objdef = || parse_file(&aeon.join("engine/objects/objdef.emp")).items;
    // VRAM_TEST_SONIC's authority (Parcel F: config/constants.asm → `.emp`).
    let game_consts = || parse_file(&aeon.join("games/sonic4/config/constants.emp")).items;

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("games/sonic4/player")),
        embed_base: None,
        defines: vec![],
    };

    let main = parse_file(&aeon.join("games/sonic4/player/sonic.emp"));
    let file = with_ambient(
        vec![
            types(), sst(), constants(), objdef(), game_consts(),
            character_def_struct_items(&aeon), player_block_struct_items(&aeon),
        ],
        main,
    );
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sonic lower errors: {ldiags:?}"
    );
    let guards = sigil_harness::test_support::guard_assert_count(&module.link_asserts);
    let mut sections = module.sections;

    let map = sigil_link::load_map(&map_toml("sonic", shape.base, shape.len)).expect("map");
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sonic place_sections errors: {pdiags:?}"
    );

    let mut lma = 0x0100_0000u32;
    for group in [
        &mut as_constant_equs(),
        &mut as_label_at("Perform_DPLC", shape.perform_dplc),
        &mut as_label_at("Ability_None", shape.ability_none),
        &mut as_label_at("Draw_Sprite", shape.draw_sprite),
        &mut as_label_at("Map_Sonic", shape.map_sonic),
        &mut as_label_at("Ani_Sonic", shape.ani_sonic),
        &mut as_label_at("DPLC_Sonic", shape.dplc_sonic),
        &mut as_label_at("Art_Sonic", shape.art_sonic),
        &mut as_label_at("Pal_SonicTails", shape.pal_sonic_tails),
    ] {
        for sec in group.iter_mut() {
            sec.lma = lma;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        sections.append(group);
        lma += 0x10_0000;
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("sonic resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("sonic link failed: {d:?}"));
    (linked, guards)
}

fn sonic_region_bytes(linked: &sigil_link::LinkedImage) -> Vec<u8> {
    linked.section("sonic").expect("linked sonic").bytes.clone()
}

#[test]
fn p1_sonic_region_matches_reference() {
    let Some(aeon) = ref_sources() else { return };
    let (linked, _g) = compile_sonic(&aeon, &SONIC_PLAIN);
    let got = sonic_region_bytes(&linked);
    if let Some(want) = ref_window(&aeon, "s4.bin", SONIC_PLAIN.base, SONIC_PLAIN.len) {
        assert_region_matches(&got, &want, "sonic (plain)");
    }
}

#[test]
fn p1_sonic_debug_region_matches_reference() {
    let Some(aeon) = ref_sources() else { return };
    let (linked, _g) = compile_sonic(&aeon, &SONIC_DEBUG);
    let got = sonic_region_bytes(&linked);
    if let Some(want) = ref_window(&aeon, "s4.debug.bin", SONIC_DEBUG.base, SONIC_DEBUG.len) {
        assert_region_matches(&got, &want, "sonic (debug)");
    }
}

// ─── PLAYER_COMMON region ─────────────────────────────────────────────────────

/// structs.emp filtered to ONLY the `Act` struct (player_common reads
/// Act.grid_w/grid_h/edge_mode; the full module's Sec + drift-guard ensures would
/// demand extra externs).
fn act_struct_items(aeon: &std::path::Path) -> Vec<sigil_frontend_emp::ast::Item> {
    use sigil_frontend_emp::ast::Item;
    let file = parse_file(&aeon.join("engine/structs.emp"));
    file.items
        .into_iter()
        .filter(|it| matches!(it, Item::Struct(d) if d.name == "Act"))
        .collect()
}

/// The `PlayerBlock` record (games/sonic4/config/ram.emp). C1 made the game RAM
/// module the AUTHOR of the per-slot player layout, and both modules under test
/// bind to it at comptime — `sonic.emp` sizes PhysTable_Sonic against
/// `offsetof(PlayerBlock, quadrant)`, `player_common.emp` strides slots by
/// `sizeof(PlayerBlock)` and checks its PBLK_* displacements the same way — so the
/// isolated-lowering ambient set has to carry the struct or lowering fails
/// outright. Filtered to the struct alone: ram.emp's `region`/`vars` items would
/// drag the whole game RAM map into these single-module compiles.
fn player_block_struct_items(aeon: &std::path::Path) -> Vec<sigil_frontend_emp::ast::Item> {
    use sigil_frontend_emp::ast::Item;
    let file = parse_file(&aeon.join("games/sonic4/config/ram.emp"));
    file.items
        .into_iter()
        // PHYS_ROW_WORDS rides along (character-lens-sweep, 2026-08-13): it is the
        // shared authority for the physics row's length, used BOTH as each
        // PhysTable_*'s declared array size and by their drift guards, so a module
        // carrying a PhysTable cannot lower without it.
        .filter(|it| matches!(it, Item::Struct(d) if d.name == "PlayerBlock")
            || matches!(it, Item::Const(d) if d.name == "PHYS_ROW_WORDS"))
        .collect()
}

/// The `CharacterDef` record (engine/structs.emp). Both `sonic.emp` (which IS a
/// `CharacterDef` literal now) and `player_common.emp` (which dereferences one
/// through `Player_Chardef`) `use` it, so the isolated-lowering ambient set has
/// to carry it or lowering fails outright. Same shape as `act_struct_items`.
fn character_def_struct_items(aeon: &std::path::Path) -> Vec<sigil_frontend_emp::ast::Item> {
    use sigil_frontend_emp::ast::Item;
    let file = parse_file(&aeon.join("engine/structs.emp"));
    file.items
        .into_iter()
        .filter(|it| match it {
            Item::Struct(d) => d.name == "CharacterDef",
            // tails-flight (aeon 512a5f9e): the curl geometry became per-character,
            // derived from the record's two box heights, so player_common's
            // curl_head_rise expands these two blessed height-half views. They ride
            // the same splice as the record they offset into.
            Item::ComptimeFn(d) => matches!(d.name.as_str(), "cd_stand_h_off" | "cd_roll_h_off"),
            _ => false,
        })
        .collect()
}

struct PcShape {
    character_id: u32,
    player_chardef: u32,
    character_defs: u32,
    player_init_assets: u32,
    player_load_art: u32,
    phys_table_sonic: u32,
    animate_sprite: u32,
    draw_sprite: u32,
    sound_play_sfx: u32,
    at_ledge_edge: u32,
    map_test_obj: u32,
    pstate_ground: u32,
    pstate_roll: u32,
    pstate_spindash: u32,
    pstate_air: u32,
    pstate_jump: u32,
    pstate_rolljump: u32,
    pstate_airball: u32,
    /// tails-flight: PSTATE_FLY, the seventh entry in the state offset table.
    pstate_fly: u32,
    pstate_glide: u32,
    pstate_glidefall: u32,
    pstate_slide: u32,
    pstate_climb: u32,
    pstate_ledge: u32,
    /// C1: the per-slot player working blocks. ONE label now — the array base
    /// player_common's `player_block` splice `lea`s, then strides by
    /// `sizeof(PlayerBlock)` for slot 1. The three cells this replaced
    /// (Player_Phys / Player_Quadrant / Player_JumpBuffer) no longer exist.
    player_blocks: u32,
    /// C1: `player_block` and Player_Main's ring gate both `cmpa.w #Player_1` to
    /// tell the leader from the follower, so the slot-0 SST address is a
    /// cross-seam operand of this module's bytes for the first time.
    player_1: u32,
    /// tails-flight (aeon 89837e3e): Player_RefreshPhysics PUBLISHES the active
    /// character's curl compensation to this engine-RAM cell for the camera to
    /// read — the write side of the seam camera_port gates from the other end.
    camera_curl_offset: u32,
    /// knuckles-def (C4 task 9): the same proc now also publishes the active
    /// character's CRAM line 0 — `CharacterDef.cd_palette` copied into the engine's
    /// palette buffer, line 0 marked dirty for the VBlank per-line DMA. Two more
    /// cross-seam engine-RAM operands in player_common's bytes, so the isolated
    /// compile has to supply them the way it already supplies camera_curl_offset.
    palette_buffer: u32,
    palette_dirty: u32,
    player_ring_index: u32,
    player_pos_ring: u32,
    player_stat_ring: u32,
    player_death_pending: u32,
    ctrl_1_press: u32,
    ctrl_1_held: u32,
    current_act_ptr: u32,
    // bug005 M3: the act-invariant clamp-edge cache (Player_BoundsInit writes,
    // the per-frame clamps read back) — game ram.emp cells, abs.w EAs.
    player_bound_right: u32,
    cheat_flags: u32,
    player_bound_bottom: u32,
    /// bug005 H1 follow-ups: animate.emp's RefreshSpritePieceCount, now called
    /// cross-module by player_common (animate base + REFRESH_OFF per shape).
    refresh_spc: u32,
    /// dust Task 4/5: Player_Display calls Dust_Tick (the skid-dust cadence);
    /// PHook_SpindashEnter calls DustSpindash_Spawn (the charge-dust follower).
    /// Both live in the dust_spindash section — Dust_Tick IS its start label
    /// (DUST_SPINDASH region base per shape); the spawn proc sits at an interior
    /// offset and rides its own DUST_SPINDASH_SPAWN symbol pin.
    dust_tick: u32,
    dustspindash_spawn: u32,
    /// bug005: player_common gained `if DEBUG == 1 { assert.w … }` (the
    /// BoundsInit never-ran net), so the module now takes the DEBUG define and
    /// the debug shape carries the MDDBG__* error-handler seams.
    debug: bool,
    base: u32,
    len: usize,
}

const PC_PLAIN: PcShape = PcShape {
    character_id: pins::CHARACTER_ID.plain,
    player_chardef: pins::PLAYER_CHARDEF.plain,
    character_defs: pins::CHARACTER_DEFS.plain,
    player_init_assets: pins::PLAYER_INIT_ASSETS.plain,
    player_load_art: pins::PLAYER_LOAD_ART.plain,
    phys_table_sonic: pins::PHYS_TABLE_SONIC.plain,
    animate_sprite: pins::ANIMATE.plain_base,
    draw_sprite: pins::DRAW_SPRITE.plain,
    sound_play_sfx: pins::SOUND_PLAY_SFX.plain,
    at_ledge_edge: pins::PLAYER_AT_LEDGE_EDGE.plain,
    map_test_obj: pins::MAP_TEST_OBJ.plain,
    pstate_ground: pins::P_STATE_GROUND.plain,
    pstate_roll: pins::P_STATE_ROLL.plain,
    pstate_spindash: pins::P_STATE_SPINDASH.plain,
    pstate_air: pins::P_STATE_AIR.plain,
    pstate_jump: pins::P_STATE_JUMP.plain,
    pstate_rolljump: pins::P_STATE_ROLL_JUMP.plain,
    pstate_airball: pins::P_STATE_AIR_BALL.plain,
    pstate_fly: pins::P_STATE_FLY.plain,
    pstate_glide: pins::P_STATE_GLIDE.plain,
    pstate_glidefall: pins::P_STATE_GLIDE_FALL.plain,
    pstate_slide: pins::P_STATE_SLIDE.plain,
    pstate_climb: pins::P_STATE_CLIMB.plain,
    pstate_ledge: pins::P_STATE_LEDGE.plain,
    player_blocks: pins::PLAYER_BLOCKS.plain,
    player_1: pins::PLAYER_1.plain,
    camera_curl_offset: pins::CAMERA_CURL_OFFSET.plain,
    palette_buffer: pins::PALETTE_BUFFER.plain,
    palette_dirty: pins::PALETTE_DIRTY.plain,
    player_ring_index: pins::PLAYER_RING_INDEX.plain,
    player_pos_ring: pins::PLAYER_POS_RING.plain,
    player_stat_ring: pins::PLAYER_STAT_RING.plain,
    player_death_pending: pins::PLAYER_DEATH_PENDING.plain,
    ctrl_1_press: pins::CTRL_1_PRESS.plain,
    ctrl_1_held: pins::CTRL_1_HELD.plain,
    current_act_ptr: pins::CURRENT_ACT_PTR.plain,
    player_bound_right: pins::PLAYER_BOUND_RIGHT.plain,
    cheat_flags: pins::CHEAT_FLAGS.plain,
    player_bound_bottom: pins::PLAYER_BOUND_BOTTOM.plain,
    refresh_spc: pins::ANIMATE.plain_base + pins::REFRESH_OFF.plain as u32,
    dust_tick: pins::DUST_SPINDASH.plain_base,
    dustspindash_spawn: pins::DUST_SPINDASH_SPAWN.plain,
    debug: false,
    base: pins::PLAYER_COMMON.plain_base,
    len: pins::PLAYER_COMMON.plain_len,
};
const PC_DEBUG: PcShape = PcShape {
    character_id: pins::CHARACTER_ID.debug,
    player_chardef: pins::PLAYER_CHARDEF.debug,
    character_defs: pins::CHARACTER_DEFS.debug,
    player_init_assets: pins::PLAYER_INIT_ASSETS.debug,
    player_load_art: pins::PLAYER_LOAD_ART.debug,
    phys_table_sonic: pins::PHYS_TABLE_SONIC.debug,
    animate_sprite: pins::ANIMATE.debug_base,
    draw_sprite: pins::DRAW_SPRITE.debug,
    sound_play_sfx: pins::SOUND_PLAY_SFX.debug,
    at_ledge_edge: pins::PLAYER_AT_LEDGE_EDGE.debug,
    map_test_obj: pins::MAP_TEST_OBJ.debug,
    pstate_ground: pins::P_STATE_GROUND.debug,
    pstate_roll: pins::P_STATE_ROLL.debug,
    pstate_spindash: pins::P_STATE_SPINDASH.debug,
    pstate_air: pins::P_STATE_AIR.debug,
    pstate_jump: pins::P_STATE_JUMP.debug,
    pstate_rolljump: pins::P_STATE_ROLL_JUMP.debug,
    pstate_airball: pins::P_STATE_AIR_BALL.debug,
    pstate_fly: pins::P_STATE_FLY.debug,
    pstate_glide: pins::P_STATE_GLIDE.debug,
    pstate_glidefall: pins::P_STATE_GLIDE_FALL.debug,
    pstate_slide: pins::P_STATE_SLIDE.debug,
    pstate_climb: pins::P_STATE_CLIMB.debug,
    pstate_ledge: pins::P_STATE_LEDGE.debug,
    player_blocks: pins::PLAYER_BLOCKS.debug,
    player_1: pins::PLAYER_1.debug,
    camera_curl_offset: pins::CAMERA_CURL_OFFSET.debug,
    palette_buffer: pins::PALETTE_BUFFER.debug,
    palette_dirty: pins::PALETTE_DIRTY.debug,
    player_ring_index: pins::PLAYER_RING_INDEX.debug,
    player_pos_ring: pins::PLAYER_POS_RING.debug,
    player_stat_ring: pins::PLAYER_STAT_RING.debug,
    player_death_pending: pins::PLAYER_DEATH_PENDING.debug,
    ctrl_1_press: pins::CTRL_1_PRESS.debug,
    ctrl_1_held: pins::CTRL_1_HELD.debug,
    current_act_ptr: pins::CURRENT_ACT_PTR.debug,
    player_bound_right: pins::PLAYER_BOUND_RIGHT.debug,
    cheat_flags: pins::CHEAT_FLAGS.debug,
    player_bound_bottom: pins::PLAYER_BOUND_BOTTOM.debug,
    refresh_spc: pins::ANIMATE.debug_base + pins::REFRESH_OFF.debug as u32,
    dust_tick: pins::DUST_SPINDASH.debug_base,
    dustspindash_spawn: pins::DUST_SPINDASH_SPAWN.debug,
    debug: true,
    base: pins::PLAYER_COMMON.debug_base,
    len: pins::PLAYER_COMMON.debug_len,
};

fn compile_player_common(
    aeon: &Path,
    shape: &PcShape,
) -> (sigil_link::LinkedImage, Vec<Section>, Vec<sigil_ir::LinkAssert>, usize) {
    let types = || parse_file(&aeon.join("engine/system/types.emp")).items;
    let sst = || parse_file(&aeon.join("engine/objects/sst.emp")).items;
    let constants = || parse_file(&aeon.join("engine/system/constants.emp")).items;
    let objdef = || parse_file(&aeon.join("engine/objects/objdef.emp")).items;
    let coords = || parse_file(&aeon.join("engine/coords.emp")).items;
    let act = || act_struct_items(aeon);
    // PSTATE_*/ANIM_*/VRAM_TEST_MARKER authority (Parcel F: config/constants.asm → `.emp`).
    let game_consts = || parse_file(&aeon.join("games/sonic4/config/constants.emp")).items;

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("games/sonic4/player")),
        embed_base: None,
        defines: vec![
            ("SOUND_DRIVER_ENABLED".to_string(), 1),
            // bug005: the BoundsInit never-ran assert rides `if DEBUG == 1 {}`.
            ("DEBUG".to_string(), i128::from(shape.debug)),
        ],
    };

    let main = parse_file(&aeon.join("games/sonic4/player/player_common.emp"));
    let file = with_ambient(
        vec![
            types(), sst(), constants(), objdef(), coords(), act(), game_consts(),
            character_def_struct_items(&aeon), player_block_struct_items(&aeon),
        ],
        main,
    );
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "player_common lower errors: {ldiags:?}"
    );
    let guards = sigil_harness::test_support::guard_assert_count(&module.link_asserts);
    let asserts = module.link_asserts.clone();
    let mut sections = module.sections;

    let map = sigil_link::load_map(&map_toml("player_common", shape.base, shape.len)).expect("map");
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "player_common place_sections errors: {pdiags:?}"
    );

    let mut lma = 0x0100_0000u32;
    let mut groups: Vec<Vec<Section>> = vec![
        as_constant_equs(),
        as_label_at("Character_ID", shape.character_id),
        as_label_at("Player_Chardef", shape.player_chardef),
        as_label_at("CharacterDefs", shape.character_defs),
        as_label_at("Player_InitAssets", shape.player_init_assets),
        as_label_at("Player_LoadArt", shape.player_load_art),
        as_label_at("PhysTable_Sonic", shape.phys_table_sonic),
        as_label_at("AnimateSprite", shape.animate_sprite),
        as_label_at("Draw_Sprite", shape.draw_sprite),
        as_label_at("Sound_PlaySFX", shape.sound_play_sfx),
        as_label_at("Player_AtLedgeEdge", shape.at_ledge_edge),
        as_label_at("Map_TestObj", shape.map_test_obj),
        as_label_at("PState_Ground", shape.pstate_ground),
        as_label_at("PState_Roll", shape.pstate_roll),
        as_label_at("PState_Spindash", shape.pstate_spindash),
        as_label_at("PState_Air", shape.pstate_air),
        as_label_at("PState_Jump", shape.pstate_jump),
        as_label_at("PState_RollJump", shape.pstate_rolljump),
        as_label_at("PState_AirBall", shape.pstate_airball),
        as_label_at("PState_Fly", shape.pstate_fly),
        // knuckles-c4: the five glide-family rows player_common's offset
        // tables name — cross-seam branch targets, like PState_Fly above.
        as_label_at("PState_Glide", shape.pstate_glide),
        as_label_at("PState_GlideFall", shape.pstate_glidefall),
        as_label_at("PState_Slide", shape.pstate_slide),
        as_label_at("PState_Climb", shape.pstate_climb),
        as_label_at("PState_Ledge", shape.pstate_ledge),
        as_label_at("Player_Blocks", shape.player_blocks),
        as_label_at("Player_1", shape.player_1),
        as_label_at("Camera_Curl_Offset", shape.camera_curl_offset),
        as_label_at("Palette_Buffer", shape.palette_buffer),
        as_label_at("Palette_Dirty", shape.palette_dirty),
        as_label_at("Player_Ring_Index", shape.player_ring_index),
        as_label_at("Player_Pos_Ring", shape.player_pos_ring),
        as_label_at("Player_Stat_Ring", shape.player_stat_ring),
        as_label_at("Player_Death_Pending", shape.player_death_pending),
        as_label_at("Ctrl_1_Press", shape.ctrl_1_press),
        as_label_at("Ctrl_1_Held", shape.ctrl_1_held),
        as_label_at("Current_Act_Ptr", shape.current_act_ptr),
        // bug005 M3: the clamp-edge cache cells (abs.w reads + BoundsInit writes).
        as_label_at("Player_Bound_Right", shape.player_bound_right),
        // Cheat_Flags: the runtime debug-fly gate read by Player_Main and
        // Player_Init. PIN-SOURCED, shape-dependent (game RAM).
        as_label_at("Cheat_Flags", shape.cheat_flags),
        as_label_at("Player_Bound_Bottom", shape.player_bound_bottom),
        // bug005 H1 follow-ups: the animator-owned refresh idiom made
        // player_common call animate.emp's RefreshSpritePieceCount cross-module
        // (was intra-module in animate only). VMA = animate base + REFRESH_OFF.
        as_label_at("RefreshSpritePieceCount", shape.refresh_spc),
        // dust Task 4/5: Player_Display calls Dust_Tick; PHook_SpindashEnter
        // calls DustSpindash_Spawn. Dust_Tick heads the dust_spindash section
        // (= DUST_SPINDASH region base); the spawn proc is interior, pinned as
        // DUST_SPINDASH_SPAWN.
        as_label_at("Dust_Tick", shape.dust_tick),
        as_label_at("DustSpindash_Spawn", shape.dustspindash_spawn),
    ];
    if shape.debug {
        // DEBUG-only: the BoundsInit never-ran assert.w expansion jsr/jmps these
        // (core_port/sprites_port precedent).
        groups.push(as_label_at("MDDBG__ErrorHandler", pins::MDDBG_ERROR_HANDLER));
        groups.push(as_label_at(
            "MDDBG__ErrorHandler_PagesController",
            pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER,
        ));
    }
    for group in &mut groups {
        for sec in group.iter_mut() {
            sec.lma = lma;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        sections.append(group);
        lma += 0x10_0000;
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("player_common resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("player_common link failed: {d:?}"));
    (linked, resolved, asserts, guards)
}

fn pc_region_bytes(linked: &sigil_link::LinkedImage) -> Vec<u8> {
    linked.section("player_common").expect("linked player_common").bytes.clone()
}

#[test]
fn p1_player_common_region_matches_reference() {
    let Some(aeon) = ref_sources() else { return };
    let (linked, _r, _a, _g) = compile_player_common(&aeon, &PC_PLAIN);
    let got = pc_region_bytes(&linked);
    if let Some(want) = ref_window(&aeon, "s4.bin", PC_PLAIN.base, PC_PLAIN.len) {
        assert_region_matches(&got, &want, "player_common (plain)");
    }
}

#[test]
fn p1_player_common_debug_region_matches_reference() {
    let Some(aeon) = ref_sources() else { return };
    let (linked, _r, _a, _g) = compile_player_common(&aeon, &PC_DEBUG);
    let got = pc_region_bytes(&linked);
    if let Some(want) = ref_window(&aeon, "s4.debug.bin", PC_DEBUG.base, PC_DEBUG.len) {
        assert_region_matches(&got, &want, "player_common (debug)");
    }
}

// ── guard gate + t24 positive control / negative probe ───────────────────────

/// player_common's MIRROR drift guards are fully retired. sst.emp's 30 SST_*
/// ambient guards retired at the conv-a structs flip; the PSTATE_*/ANIM_*/
/// VRAM_TEST_MARKER mirror guards retired at conv-f (config constants flipped to
/// `.emp`, `use`d now); and the last one — the SFXID_SKID sound mirror — retired at
/// conv-f #24/F2 (SFXID_SKID `use`d from games.sonic4.sound_ids, its authority).
///
/// ONE link assert survives, and it is NOT a mirror guard: C1's
/// `extern("Player_1") & $FFFF8000 == $FFFF8000`. `player_block` and Player_Main's
/// ring gate both tell the leader from the follower with `cmpa.w #Player_1` — a
/// SIGN-EXTENDED word compared against a full address register — which is only
/// correct while Player_1 lives in the w-addressable window. It reads a link-time
/// address, so it cannot fold at comptime the way the PBLK_* `offsetof` ensures in
/// the same module do; it has to ride the link. Counting it here is the point:
/// this test is where a silently-dropped build-time check would show up.
#[test]
fn p1_drift_guards_all_pass() {
    let Some(aeon) = ref_sources() else { return };
    let (_linked, resolved, asserts, guards) = compile_player_common(&aeon, &PC_PLAIN);
    assert_eq!(
        guards, 1,
        "player_common's mirror guards are retired; the one survivor is C1's Player_1 \
         w-addressable check (cmpa.w sign-extension); got {guards}"
    );
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the drift guards must all PASS: {diags:?}"
    );
}

/// POSITIVE CONTROL (t24 rule): the UNDOCTORED compile equals the reference
/// window exactly. If this fails, the negative probe below proves nothing.
#[test]
fn p1_undoctored_compile_equals_the_reference_window() {
    let Some(aeon) = ref_sources() else { return };
    let (linked, _r, _a, _g) = compile_player_common(&aeon, &PC_PLAIN);
    let got = pc_region_bytes(&linked);
    if let Some(want) = ref_window(&aeon, "s4.bin", PC_PLAIN.base, PC_PLAIN.len) {
        // Packed placement: the pin LEN spans to the next section's aligned
        // base, so the window may end in a short all-zero align pad beyond the
        // lowered image (same tolerance as assert_region_matches — 32, the
        // largest section alignment in play; C1's plain window ends in exactly
        // 16 pad bytes, which the old `< 16` bound excluded by one).
        let want_trimmed = if want.len() > got.len()
            && want.len() - got.len() < 32
            && want[got.len()..].iter().all(|&b| b == 0)
        {
            &want[..got.len()]
        } else {
            &want[..]
        };
        assert_eq!(got, want_trimmed, "undoctored player_common must match the reference window");
    }
}

/// NEGATIVE PROBE (t24 rule): a doctored reference window must NOT match the
/// compiled bytes — the gate can actually fail. Doctors a pins-derived offset.
#[test]
fn p1_doctored_reference_diverges() {
    let Some(aeon) = ref_sources() else { return };
    let (linked, _r, _a, _g) = compile_player_common(&aeon, &PC_PLAIN);
    let got = pc_region_bytes(&linked);
    let Some(mut want) = ref_window(&aeon, "s4.bin", PC_PLAIN.base, PC_PLAIN.len) else {
        return;
    };
    want[0] ^= 0xFF; // flip the first opcode byte
    assert_ne!(got, want, "a doctored reference must diverge from the compiled bytes");
}

/// NEGATIVE PROBE (sonic region): a doctored reference window must NOT match.
#[test]
fn p1_sonic_doctored_reference_diverges() {
    let Some(aeon) = ref_sources() else { return };
    let (linked, _g) = compile_sonic(&aeon, &SONIC_PLAIN);
    let got = sonic_region_bytes(&linked);
    let Some(mut want) = ref_window(&aeon, "s4.bin", SONIC_PLAIN.base, SONIC_PLAIN.len) else {
        return;
    };
    want[0] ^= 0xFF;
    assert_ne!(got, want, "a doctored sonic reference must diverge from the compiled bytes");
}
