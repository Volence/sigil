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
//! REFERENCE-DEPENDENT (`AEON_DIR`, default sibling). Absent, tests SKIP green
//! unless `SIGIL_STRICT_GATE=1`.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
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
    // the surviving-AS PlayerV struct equates (guard targets)
    pairs.push(("_pl_gsp", "$2E"));
    pairs.push(("_pl_state", "$30"));
    pairs.push(("_pl_move_lock", "$32"));
    pairs.push(("_pl_spindash", "$34"));
    pairs.push(("_pl_stick_convex", "$39"));
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
    // Packed placement (Wave-B B-0) may end a region window in ALIGNMENT FILL: the
    // pins span runs to the next section's aligned base. Tolerate a short (< 16 B)
    // all-zero tail beyond the lowered image; every real byte still compares.
    let expected = if expected.len() > candidate.len()
        && expected.len() - candidate.len() < 16
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

fn ref_window(rom: &str, base: u32, len: usize) -> Option<Vec<u8>> {
    let path = aeon_dir().join(rom);
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
    draw_sprite: u32,
    map_sonic: u32,
    ani_sonic: u32,
    dplc_sonic: u32,
    art_sonic: u32,
    player_phys: u32,
    player_phys_end: u32,
    base: u32,
    len: usize,
}

const SONIC_PLAIN: SonicShape = SonicShape {
    perform_dplc: pins::DPLC.plain_base,
    draw_sprite: pins::DRAW_SPRITE.plain,
    map_sonic: pins::MAP_SONIC.plain,
    ani_sonic: pins::SONIC_ANIMS.plain_base,
    dplc_sonic: pins::DPLC_SONIC.plain,
    art_sonic: pins::ART_SONIC.plain,
    player_phys: pins::PLAYER_PHYS.plain,
    player_phys_end: pins::PLAYER_PHYS_END.plain,
    base: pins::SONIC.plain_base,
    len: pins::SONIC.plain_len,
};
const SONIC_DEBUG: SonicShape = SonicShape {
    perform_dplc: pins::DPLC.debug_base,
    draw_sprite: pins::DRAW_SPRITE.debug,
    map_sonic: pins::MAP_SONIC.debug,
    ani_sonic: pins::SONIC_ANIMS.debug_base,
    dplc_sonic: pins::DPLC_SONIC.debug,
    art_sonic: pins::ART_SONIC.debug,
    player_phys: pins::PLAYER_PHYS.debug,
    player_phys_end: pins::PLAYER_PHYS_END.debug,
    base: pins::SONIC.debug_base,
    len: pins::SONIC.debug_len,
};

fn compile_sonic(shape: &SonicShape) -> (sigil_link::LinkedImage, usize) {
    let aeon = aeon_dir();
    let types = || parse_file(&aeon.join("engine/system/types.emp")).items;
    let sst = || parse_file(&aeon.join("engine/objects/sst.emp")).items;
    let constants = || parse_file(&aeon.join("engine/system/constants.emp")).items;
    let objdef = || parse_file(&aeon.join("engine/objects/objdef.emp")).items;

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("games/sonic4/player")),
        embed_base: None,
        defines: vec![],
    };

    let main = parse_file(&aeon.join("games/sonic4/player/sonic.emp"));
    let file = with_ambient(vec![types(), sst(), constants(), objdef()], main);
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
        &mut as_label_at("Draw_Sprite", shape.draw_sprite),
        &mut as_label_at("Map_Sonic", shape.map_sonic),
        &mut as_label_at("Ani_Sonic", shape.ani_sonic),
        &mut as_label_at("DPLC_Sonic", shape.dplc_sonic),
        &mut as_label_at("Art_Sonic", shape.art_sonic),
        &mut as_label_at("Player_Phys", shape.player_phys),
        &mut as_label_at("Player_Phys_End", shape.player_phys_end),
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
    let (linked, _g) = compile_sonic(&SONIC_PLAIN);
    let got = sonic_region_bytes(&linked);
    if let Some(want) = ref_window("s4.bin", SONIC_PLAIN.base, SONIC_PLAIN.len) {
        assert_region_matches(&got, &want, "sonic (plain)");
    }
}

#[test]
fn p1_sonic_debug_region_matches_reference() {
    let (linked, _g) = compile_sonic(&SONIC_DEBUG);
    let got = sonic_region_bytes(&linked);
    if let Some(want) = ref_window("s4.debug.bin", SONIC_DEBUG.base, SONIC_DEBUG.len) {
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

struct PcShape {
    sonic_init_assets: u32,
    sonic_load_art: u32,
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
    player_phys: u32,
    player_quadrant: u32,
    player_jump_buffer: u32,
    player_ring_index: u32,
    player_pos_ring: u32,
    player_stat_ring: u32,
    player_death_pending: u32,
    ctrl_1_press: u32,
    ctrl_1_held: u32,
    current_act_ptr: u32,
    base: u32,
    len: usize,
}

const PC_PLAIN: PcShape = PcShape {
    sonic_init_assets: pins::SONIC.plain_base,
    sonic_load_art: pins::SONIC_LOAD_ART.plain,
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
    player_phys: pins::PLAYER_PHYS.plain,
    player_quadrant: pins::PLAYER_QUADRANT.plain,
    player_jump_buffer: pins::PLAYER_JUMP_BUFFER.plain,
    player_ring_index: pins::PLAYER_RING_INDEX.plain,
    player_pos_ring: pins::PLAYER_POS_RING.plain,
    player_stat_ring: pins::PLAYER_STAT_RING.plain,
    player_death_pending: pins::PLAYER_DEATH_PENDING.plain,
    ctrl_1_press: pins::CTRL_1_PRESS.plain,
    ctrl_1_held: pins::CTRL_1_HELD.plain,
    current_act_ptr: pins::CURRENT_ACT_PTR.plain,
    base: pins::PLAYER_COMMON.plain_base,
    len: pins::PLAYER_COMMON.plain_len,
};
const PC_DEBUG: PcShape = PcShape {
    sonic_init_assets: pins::SONIC.debug_base,
    sonic_load_art: pins::SONIC_LOAD_ART.debug,
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
    player_phys: pins::PLAYER_PHYS.debug,
    player_quadrant: pins::PLAYER_QUADRANT.debug,
    player_jump_buffer: pins::PLAYER_JUMP_BUFFER.debug,
    player_ring_index: pins::PLAYER_RING_INDEX.debug,
    player_pos_ring: pins::PLAYER_POS_RING.debug,
    player_stat_ring: pins::PLAYER_STAT_RING.debug,
    player_death_pending: pins::PLAYER_DEATH_PENDING.debug,
    ctrl_1_press: pins::CTRL_1_PRESS.debug,
    ctrl_1_held: pins::CTRL_1_HELD.debug,
    current_act_ptr: pins::CURRENT_ACT_PTR.debug,
    base: pins::PLAYER_COMMON.debug_base,
    len: pins::PLAYER_COMMON.debug_len,
};

fn compile_player_common(shape: &PcShape) -> (sigil_link::LinkedImage, Vec<Section>, Vec<sigil_ir::LinkAssert>, usize) {
    let aeon = aeon_dir();
    let types = || parse_file(&aeon.join("engine/system/types.emp")).items;
    let sst = || parse_file(&aeon.join("engine/objects/sst.emp")).items;
    let constants = || parse_file(&aeon.join("engine/system/constants.emp")).items;
    let objdef = || parse_file(&aeon.join("engine/objects/objdef.emp")).items;
    let coords = || parse_file(&aeon.join("engine/coords.emp")).items;
    let act = || act_struct_items(&aeon);

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("games/sonic4/player")),
        embed_base: None,
        defines: vec![("SOUND_DRIVER_ENABLED".to_string(), 1)],
    };

    let main = parse_file(&aeon.join("games/sonic4/player/player_common.emp"));
    let file = with_ambient(
        vec![types(), sst(), constants(), objdef(), coords(), act()],
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
    for group in [
        &mut as_constant_equs(),
        &mut as_label_at("Sonic_InitAssets", shape.sonic_init_assets),
        &mut as_label_at("Sonic_LoadArt", shape.sonic_load_art),
        &mut as_label_at("PhysTable_Sonic", shape.phys_table_sonic),
        &mut as_label_at("AnimateSprite", shape.animate_sprite),
        &mut as_label_at("Draw_Sprite", shape.draw_sprite),
        &mut as_label_at("Sound_PlaySFX", shape.sound_play_sfx),
        &mut as_label_at("Player_AtLedgeEdge", shape.at_ledge_edge),
        &mut as_label_at("Map_TestObj", shape.map_test_obj),
        &mut as_label_at("PState_Ground", shape.pstate_ground),
        &mut as_label_at("PState_Roll", shape.pstate_roll),
        &mut as_label_at("PState_Spindash", shape.pstate_spindash),
        &mut as_label_at("PState_Air", shape.pstate_air),
        &mut as_label_at("PState_Jump", shape.pstate_jump),
        &mut as_label_at("PState_RollJump", shape.pstate_rolljump),
        &mut as_label_at("PState_AirBall", shape.pstate_airball),
        &mut as_label_at("Player_Phys", shape.player_phys),
        &mut as_label_at("Player_Quadrant", shape.player_quadrant),
        &mut as_label_at("Player_JumpBuffer", shape.player_jump_buffer),
        &mut as_label_at("Player_Ring_Index", shape.player_ring_index),
        &mut as_label_at("Player_Pos_Ring", shape.player_pos_ring),
        &mut as_label_at("Player_Stat_Ring", shape.player_stat_ring),
        &mut as_label_at("Player_Death_Pending", shape.player_death_pending),
        &mut as_label_at("Ctrl_1_Press", shape.ctrl_1_press),
        &mut as_label_at("Ctrl_1_Held", shape.ctrl_1_held),
        &mut as_label_at("Current_Act_Ptr", shape.current_act_ptr),
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
    let (linked, _r, _a, _g) = compile_player_common(&PC_PLAIN);
    let got = pc_region_bytes(&linked);
    if let Some(want) = ref_window("s4.bin", PC_PLAIN.base, PC_PLAIN.len) {
        assert_region_matches(&got, &want, "player_common (plain)");
    }
}

#[test]
fn p1_player_common_debug_region_matches_reference() {
    let (linked, _r, _a, _g) = compile_player_common(&PC_DEBUG);
    let got = pc_region_bytes(&linked);
    if let Some(want) = ref_window("s4.debug.bin", PC_DEBUG.base, PC_DEBUG.len) {
        assert_region_matches(&got, &want, "player_common (debug)");
    }
}

// ── guard gate + t24 positive control / negative probe ───────────────────────

/// The drift guards (PlayerV overlay + constant mirrors) all resolve and PASS
/// against the AS-side equ seam. sst.emp's 30 SST_* ambient guards retired at the
/// conv-a structs flip, so the captured count dropped (27 now).
#[test]
fn p1_drift_guards_all_pass() {
    let (_linked, resolved, asserts, guards) = compile_player_common(&PC_PLAIN);
    assert!(guards > 20, "player_common must capture its overlay + mirror drift guards (got {guards})");
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
    let (linked, _r, _a, _g) = compile_player_common(&PC_PLAIN);
    let got = pc_region_bytes(&linked);
    if let Some(want) = ref_window("s4.bin", PC_PLAIN.base, PC_PLAIN.len) {
        assert_eq!(got, want, "undoctored player_common must match the reference window");
    }
}

/// NEGATIVE PROBE (t24 rule): a doctored reference window must NOT match the
/// compiled bytes — the gate can actually fail. Doctors a pins-derived offset.
#[test]
fn p1_doctored_reference_diverges() {
    let (linked, _r, _a, _g) = compile_player_common(&PC_PLAIN);
    let got = pc_region_bytes(&linked);
    let Some(mut want) = ref_window("s4.bin", PC_PLAIN.base, PC_PLAIN.len) else {
        return;
    };
    want[0] ^= 0xFF; // flip the first opcode byte
    assert_ne!(got, want, "a doctored reference must diverge from the compiled bytes");
}

/// NEGATIVE PROBE (sonic region): a doctored reference window must NOT match.
#[test]
fn p1_sonic_doctored_reference_diverges() {
    let (linked, _g) = compile_sonic(&SONIC_PLAIN);
    let got = sonic_region_bytes(&linked);
    let Some(mut want) = ref_window("s4.bin", SONIC_PLAIN.base, SONIC_PLAIN.len) else {
        return;
    };
    want[0] ^= 0xFF;
    assert_ne!(got, want, "a doctored sonic reference must diverge from the compiled bytes");
}
