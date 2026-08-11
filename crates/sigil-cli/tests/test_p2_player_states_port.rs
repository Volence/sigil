//! Tranche 35 — game-side P2+P3: the player STATE MACHINES
//! (player_ground / player_air / player_spindash). Region-level byte gates in
//! BOTH shapes, modelled on `test_p1_player_port.rs`.
//!
//! Per-file object-bank gates `SIGIL_EMP_PLAYER_{GROUND,AIR,SPINDASH}`. Bank
//! windows shape-invariant (base/len identical in s4.lst and s4.debug.lst); the
//! content bytes track per-shape cross-seam operands → compile twice. The three
//! files import the player_common keystone (PlayerV overlay + surviving-AS
//! mirrors) and dispatch back through the Player_States offset table.
//!
//! REFERENCE-DEPENDENT: the sources and the reference ROMs live in the sibling
//! `aeon` tree (`AEON_DIR`, default `/home/volence/sonic_hacks/aeon`). Absent,
//! every test here SKIPS green — unless `SIGIL_STRICT_GATE=1` makes a missing
//! reference a hard failure.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_harness::test_support::reference_tree;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

/// The aeon-relative sources EVERY state-file compile reads: the player_common
/// keystone plus the four ambient engine modules.
const COMMON_SOURCES: &[&str] = &[
    "games/sonic4/player/player_common.emp",
    "engine/system/types.emp",
    "engine/objects/sst.emp",
    "engine/system/constants.emp",
    "engine/coords.emp",
];

/// `Some(aeon_root)` when the tree carries the common sources AND the one state
/// file `state_rel` names; `None` — skip green — when it does not (a panic under
/// `SIGIL_STRICT_GATE=1`).
fn player_tree(state_rel: &str) -> Option<PathBuf> {
    let mut rels = COMMON_SOURCES.to_vec();
    rels.push(state_rel);
    reference_tree(&rels)
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

/// The `engine.structs` items the curl geometry stands on (aeon 512a5f9e): the
/// `CharacterDef` record plus `cd_stand_h_off` / `cd_roll_h_off`, the two blessed
/// height-half views `curl_head_rise` expands into. Filtered rather than prepended
/// whole — `structs.emp` carries the Act/Sec records and their drift-guard ensures,
/// which would demand externs this isolated compile has no seam for.
fn curl_geometry_items(aeon: &std::path::Path) -> Vec<sigil_frontend_emp::ast::Item> {
    use sigil_frontend_emp::ast::Item;
    parse_file(&aeon.join("engine/structs.emp"))
        .items
        .into_iter()
        .filter(|it| match it {
            Item::Struct(d) => d.name == "CharacterDef",
            Item::ComptimeFn(d) => matches!(d.name.as_str(), "cd_stand_h_off" | "cd_roll_h_off"),
            _ => false,
        })
        .collect()
}

/// The keystone items the state files import: the PlayerV overlay, the shared
/// comptime-fn macros (mask_opposing_lr / dist_to_fix / set_*_size), and the two
/// jump-button masks. Filtered from player_common.emp — the rest of the keystone
/// would drag in its own externs. The state files
/// `use ...player_common.{PlayerV, mask_opposing_lr, BUTTON_JUMP_MASK, ...}`.
///
/// `BUTTON_JUMP_MASK` / `BUTTON_JUMP_MASK_NO_B` are spliced from the REAL source
/// rather than hand-mirrored as AS equs (parcel `b-jumps`, 2026-08-05). The two
/// masks moved into player_common precisely so the press latch and the air
/// release-cap check cannot disagree; re-typing their values here would rebuild
/// the drift seam the move was made to close. Splicing the `Item::Const`s makes
/// them fold from the authority, so a change to either mask reaches this gate as
/// a byte diff, never as a silently-stale mirror.
fn player_common_imports(aeon: &std::path::Path) -> Vec<sigil_frontend_emp::ast::Item> {
    use sigil_frontend_emp::ast::Item;
    let file = parse_file(&aeon.join("games/sonic4/player/player_common.emp"));
    file.items
        .into_iter()
        .filter(|it| match it {
            Item::Vars(d) => d.name.as_deref() == Some("PlayerV"),
            // `curl_head_rise` / `curl_shift_px` joined the splice at aeon 512a5f9e:
            // the curl geometry stopped being an engine-wide CURL_Y_SHIFT constant and
            // became DERIVED per character from the two box heights in its record, so
            // player_ground and player_air call them where they used to fold a literal.
            // Same rule as the masks below — spliced from the authority rather than
            // mirrored, so a change to the derivation reaches this gate as a byte diff.
            // (`curl_shift_px` expands `curl_head_rise`, so both must be present.)
            Item::ComptimeFn(d) => matches!(
                d.name.as_str(),
                "mask_opposing_lr"
                    | "dist_to_fix"
                    | "set_standing_size"
                    | "set_ball_size"
                    | "abs_w"
                    | "curl_head_rise"
                    | "curl_shift_px"
            ),
            Item::Const(d) => {
                matches!(d.name.as_str(), "BUTTON_JUMP_MASK" | "BUTTON_JUMP_MASK_NO_B")
            }
            _ => false,
        })
        .collect()
}

/// The AS-side value seam: SST field equs + engine constants + ObjCodeBase +
/// the player game-config + engine physics/status mirrors the ensures resolve
/// against.
fn as_constant_equs(shape: &Shape) -> Vec<Section> {
    use sigil_harness::test_support::{engine_constant_equs, sst_field_equs};
    let mut pairs = sst_field_equs();
    pairs.extend(engine_constant_equs());
    let obj_code_base = format!("${:X}", OBJ_CODE_BASE);
    pairs.push(("ObjCodeBase", obj_code_base.as_str()));
    // RAM symbols (shape-dependent) — batched into this single equ blob so the
    // `Stub:` section it emits is unique (a second assemble_equ_pairs call would
    // collide on `Stub`).
    let ctrl_1_held = format!("${:X}", shape.ctrl_1_held);
    let ctrl_1_press = format!("${:X}", shape.ctrl_1_press);
    let camera_hold_frames = format!("${:X}", shape.camera_hold_frames);
    // Cheat_Flags: the runtime debug-fly gate. player_air's release-cap check reads
    // it to decide whether B still sustains a jump (parcel `b-jumps`), the same way
    // Player_Main's latch does. PIN-SOURCED, shape-dependent (game RAM) — the
    // test_p1 precedent.
    let cheat_flags = format!("${:X}", shape.cheat_flags);
    pairs.push(("Cheat_Flags", cheat_flags.as_str()));
    pairs.push(("Ctrl_1_Held", ctrl_1_held.as_str()));
    // C1: PState_AirShared gained the ability gate — a FRESH jump press while
    // already airborne calls Player_Ability — so player_air reads the press-edge
    // latch, not just the held state.
    pairs.push(("Ctrl_1_Press", ctrl_1_press.as_str()));
    pairs.push(("Camera_Hold_Frames", camera_hold_frames.as_str()));
    // C1: Player_Phys / Player_Quadrant / Player_JumpBuffer are GONE. The state
    // bodies read the same values as PBLK_* displacements off a4 (the calling
    // slot's PlayerBlock), so not one player-state RAM address survives as a
    // cross-seam operand of these three regions.
    // game-config (config/constants.asm + config/sound_ids.asm) mirrors
    pairs.push(("PSTATE_GROUND", "0"));
    pairs.push(("PSTATE_ROLL", "2"));
    pairs.push(("PSTATE_SPINDASH", "4"));
    pairs.push(("PSTATE_AIR", "6"));
    pairs.push(("PSTATE_JUMP", "8"));
    pairs.push(("PSTATE_ROLLJUMP", "10"));
    pairs.push(("PSTATE_AIRBALL", "12"));
    pairs.push(("CHEAT_DEBUG_FLY", "1"));
    pairs.push(("SPINDASH_BASE", "$800"));
    pairs.push(("SPINDASH_CHARGE_STEP", "$200"));
    pairs.push(("SPINDASH_CHARGE_MAX", "$800"));
    pairs.push(("SFXID_SPINDASH", "$AB"));
    pairs.push(("SFXID_DASH", "$B6"));
    pairs.push(("SFXID_ROLL", "$3C"));
    pairs.push(("SFXID_JUMP", "$62"));
    // The engine-truth ST_* / radii / PHYS_* / BUTTON_A/B/C block is supplied by the
    // `engine_constant_equs()` extend above. The state files' PPHYS_* / _pl_* drift
    // guards all retired at conv-d #49 (player_common.asm deleted — the state files
    // own their file-local PPHYS_* consts, verified by the region byte gate; no
    // extern mirror to supply). BUTTON_JUMP_MASK / BUTTON_JUMP_MASK_NO_B went the
    // other way at parcel `b-jumps` — back to ONE authority in player_common, and
    // spliced from it by `player_common_imports` rather than mirrored here.
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

/// The reference ROM window, or `None` when the aeon tree carries no such ROM
/// (skip green; a panic under `SIGIL_STRICT_GATE=1`).
fn ref_window(rom: &str, base: u32, len: usize) -> Option<Vec<u8>> {
    let path = reference_tree(&[rom])?.join(rom);
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let b = base as usize;
    Some(bytes[b..b + len].to_vec())
}

// ─── shared cross-seam callee + RAM pins ─────────────────────────────────────

struct Shape {
    debug: bool,
    // cross-seam callees
    player_set_state: u32,
    /// C1: the cd_ability dispatch (characters.emp). PState_AirShared calls it
    /// on a fresh jump press while already airborne, making the roster's third
    /// record reader a cross-seam branch target of player_air.
    player_ability: u32,
    /// aeon 512a5f9e: the curl geometry and the unroll ceiling probes read the
    /// ACTIVE character's record, so the state files now dereference the cached
    /// `Player_Chardef` pointer where they used to fold engine-wide constants.
    player_chardef: u32,
    player_snap_to_surface: u32,
    player_sensor_floor: u32,
    player_sensor_ceiling: u32,
    player_sensor_wall_dir: u32,
    player_sensor_wall_at: u32,
    get_sine_cosine: u32,
    object_move: u32,
    sound_play_sfx: u32,
    // sibling state-body entry points (cross-file, same object bank)
    pstate_ground: u32,
    pstate_roll: u32,
    pstate_spindash: u32,
    pstate_air: u32,
    pstate_jump: u32,
    pstate_rolljump: u32,
    pstate_airball: u32,
    player_jump: u32,
    // RAM
    ctrl_1_held: u32,
    ctrl_1_press: u32,
    camera_hold_frames: u32,
    cheat_flags: u32,
}

const PLAIN: Shape = Shape {
    debug: false,
    player_set_state: pins::PLAYER_SET_STATE.plain,
    player_ability: pins::PLAYER_ABILITY.plain,
    player_chardef: pins::PLAYER_CHARDEF.plain,
    player_snap_to_surface: pins::PLAYER_SNAP_TO_SURFACE.plain,
    player_sensor_floor: pins::PLAYER_SENSOR_FLOOR.plain,
    player_sensor_ceiling: pins::PLAYER_SENSOR_CEILING.plain,
    player_sensor_wall_dir: pins::PLAYER_SENSOR_WALL_DIR.plain,
    player_sensor_wall_at: pins::PLAYER_SENSOR_WALL_AT.plain,
    get_sine_cosine: pins::GET_SINE_COSINE.plain,
    object_move: pins::OBJECT_MOVE.plain,
    sound_play_sfx: pins::SOUND_PLAY_SFX.plain,
    pstate_ground: pins::P_STATE_GROUND.plain,
    pstate_roll: pins::P_STATE_ROLL.plain,
    pstate_spindash: pins::P_STATE_SPINDASH.plain,
    pstate_air: pins::P_STATE_AIR.plain,
    pstate_jump: pins::P_STATE_JUMP.plain,
    pstate_rolljump: pins::P_STATE_ROLL_JUMP.plain,
    pstate_airball: pins::P_STATE_AIR_BALL.plain,
    player_jump: 0x105F0, // filled at need; ground-local (see note)
    ctrl_1_held: pins::CTRL_1_HELD.plain,
    ctrl_1_press: pins::CTRL_1_PRESS.plain,
    camera_hold_frames: pins::CAMERA_HOLD_FRAMES.plain,
    cheat_flags: pins::CHEAT_FLAGS.plain,
};

const DEBUG: Shape = Shape {
    debug: true,
    player_set_state: pins::PLAYER_SET_STATE.debug,
    player_ability: pins::PLAYER_ABILITY.debug,
    player_chardef: pins::PLAYER_CHARDEF.debug,
    player_snap_to_surface: pins::PLAYER_SNAP_TO_SURFACE.debug,
    player_sensor_floor: pins::PLAYER_SENSOR_FLOOR.debug,
    player_sensor_ceiling: pins::PLAYER_SENSOR_CEILING.debug,
    player_sensor_wall_dir: pins::PLAYER_SENSOR_WALL_DIR.debug,
    player_sensor_wall_at: pins::PLAYER_SENSOR_WALL_AT.debug,
    get_sine_cosine: pins::GET_SINE_COSINE.debug,
    object_move: pins::OBJECT_MOVE.debug,
    sound_play_sfx: pins::SOUND_PLAY_SFX.debug,
    pstate_ground: pins::P_STATE_GROUND.debug,
    pstate_roll: pins::P_STATE_ROLL.debug,
    pstate_spindash: pins::P_STATE_SPINDASH.debug,
    pstate_air: pins::P_STATE_AIR.debug,
    pstate_jump: pins::P_STATE_JUMP.debug,
    pstate_rolljump: pins::P_STATE_ROLL_JUMP.debug,
    pstate_airball: pins::P_STATE_AIR_BALL.debug,
    player_jump: 0x105F0,
    ctrl_1_held: pins::CTRL_1_HELD.debug,
    ctrl_1_press: pins::CTRL_1_PRESS.debug,
    camera_hold_frames: pins::CAMERA_HOLD_FRAMES.debug,
    cheat_flags: pins::CHEAT_FLAGS.debug,
}
;

/// Compile one player state file as its own region against the pinned seam.
fn compile_region(
    aeon: &Path,
    shape: &Shape,
    emp_rel: &str,
    region: &str,
    base: u32,
    len: usize,
    locals: &[&str],
) -> Vec<u8> {
    let types = || parse_file(&aeon.join("engine/system/types.emp")).items;
    let sst = || parse_file(&aeon.join("engine/objects/sst.emp")).items;
    let constants = || parse_file(&aeon.join("engine/system/constants.emp")).items;
    let coords = || parse_file(&aeon.join("engine/coords.emp")).items;
    let pc = || player_common_imports(aeon);

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("games/sonic4/player")),
        embed_base: None,
        defines: vec![("SOUND_DRIVER_ENABLED".to_string(), 1)],
    };

    let main = parse_file(&aeon.join(emp_rel));
    let file =
        with_ambient(vec![types(), sst(), constants(), coords(), curl_geometry_items(aeon), pc()], main);
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{region} lower errors: {ldiags:?}"
    );
    let mut sections = module.sections;

    let map = sigil_link::load_map(&map_toml(region, base, len)).expect("map");
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{region} place_sections errors: {pdiags:?}"
    );

    // Cross-seam labels pinned at their reference addresses. A label the CURRENT
    // region defines locally (`locals`) is skipped — pinning it would collide.
    let labels: [(&str, u32); 19] = [
        ("Player_SetState", shape.player_set_state),
        ("Player_Ability", shape.player_ability),
        ("Player_Chardef", shape.player_chardef),
        ("Player_SnapToSurface", shape.player_snap_to_surface),
        ("Player_SensorFloor", shape.player_sensor_floor),
        ("Player_SensorCeiling", shape.player_sensor_ceiling),
        ("Player_SensorWallDir", shape.player_sensor_wall_dir),
        ("Player_SensorWallAt", shape.player_sensor_wall_at),
        ("GetSineCosine", shape.get_sine_cosine),
        ("ObjectMove", shape.object_move),
        ("Sound_PlaySFX", shape.sound_play_sfx),
        ("PState_Ground", shape.pstate_ground),
        ("PState_Roll", shape.pstate_roll),
        ("PState_Spindash", shape.pstate_spindash),
        ("PState_Air", shape.pstate_air),
        ("PState_Jump", shape.pstate_jump),
        ("PState_RollJump", shape.pstate_rolljump),
        ("PState_AirBall", shape.pstate_airball),
        ("Player_Jump", shape.player_jump),
    ];

    let mut groups: Vec<Vec<Section>> = vec![as_constant_equs(shape)];
    for (name, addr) in labels {
        if locals.contains(&name) {
            continue;
        }
        groups.push(as_label_at(name, addr));
    }

    let mut lma = 0x0100_0000u32;
    for group in groups.iter_mut() {
        for sec in group.iter_mut() {
            sec.lma = lma;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        sections.append(group);
        lma += 0x10_0000;
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("{region} resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("{region} link failed: {d:?}"));
    linked.section(region).expect("linked region").bytes.clone()
}

// ─── SPINDASH region ─────────────────────────────────────────────────────────

fn spindash_bytes(shape: &Shape) -> Option<Vec<u8>> {
    let aeon = player_tree("games/sonic4/player/player_spindash.emp")?;
    Some(compile_region(
        &aeon,
        shape,
        "games/sonic4/player/player_spindash.emp",
        "player_spindash",
        if shape.debug { pins::PLAYER_SPINDASH.debug_base } else { pins::PLAYER_SPINDASH.plain_base },
        if shape.debug { pins::PLAYER_SPINDASH.debug_len } else { pins::PLAYER_SPINDASH.plain_len },
        &["PState_Spindash"],
    ))
}

#[test]
fn p2_spindash_region_matches_reference() {
    let Some(got) = spindash_bytes(&PLAIN) else { return };
    if let Some(want) = ref_window("s4.bin", pins::PLAYER_SPINDASH.plain_base, pins::PLAYER_SPINDASH.plain_len) {
        assert_region_matches(&got, &want, "player_spindash (plain)");
    }
}

#[test]
fn p2_spindash_debug_region_matches_reference() {
    let Some(got) = spindash_bytes(&DEBUG) else { return };
    if let Some(want) = ref_window("s4.debug.bin", pins::PLAYER_SPINDASH.debug_base, pins::PLAYER_SPINDASH.debug_len) {
        assert_region_matches(&got, &want, "player_spindash (debug)");
    }
}

#[test]
fn p2_spindash_undoctored_compile_equals_the_reference_window() {
    let Some(got) = spindash_bytes(&PLAIN) else { return };
    if let Some(want) = ref_window("s4.bin", pins::PLAYER_SPINDASH.plain_base, pins::PLAYER_SPINDASH.plain_len) {
        assert_eq!(got, want, "undoctored player_spindash must match the reference window");
    }
}

#[test]
fn p2_spindash_doctored_reference_diverges() {
    let Some(got) = spindash_bytes(&PLAIN) else { return };
    let Some(mut want) = ref_window("s4.bin", pins::PLAYER_SPINDASH.plain_base, pins::PLAYER_SPINDASH.plain_len) else {
        return;
    };
    want[0] ^= 0xFF;
    assert_ne!(got, want, "a doctored reference must diverge from the compiled bytes");
}

// ─── AIR region ──────────────────────────────────────────────────────────────

fn air_bytes(shape: &Shape) -> Option<Vec<u8>> {
    let aeon = player_tree("games/sonic4/player/player_air.emp")?;
    Some(compile_region(
        &aeon,
        shape,
        "games/sonic4/player/player_air.emp",
        "player_air",
        if shape.debug { pins::PLAYER_AIR.debug_base } else { pins::PLAYER_AIR.plain_base },
        if shape.debug { pins::PLAYER_AIR.debug_len } else { pins::PLAYER_AIR.plain_len },
        &["PState_Air", "PState_AirBall", "PState_RollJump", "PState_Jump"],
    ))
}

#[test]
fn p2_air_region_matches_reference() {
    let Some(got) = air_bytes(&PLAIN) else { return };
    if let Some(want) = ref_window("s4.bin", pins::PLAYER_AIR.plain_base, pins::PLAYER_AIR.plain_len) {
        assert_region_matches(&got, &want, "player_air (plain)");
    }
}

#[test]
fn p2_air_debug_region_matches_reference() {
    let Some(got) = air_bytes(&DEBUG) else { return };
    if let Some(want) = ref_window("s4.debug.bin", pins::PLAYER_AIR.debug_base, pins::PLAYER_AIR.debug_len) {
        assert_region_matches(&got, &want, "player_air (debug)");
    }
}

#[test]
fn p2_air_undoctored_compile_equals_the_reference_window() {
    let Some(got) = air_bytes(&PLAIN) else { return };
    if let Some(want) = ref_window("s4.bin", pins::PLAYER_AIR.plain_base, pins::PLAYER_AIR.plain_len) {
        // The pin len spans to the next section base, which may include align
        // pad after the code (B-0 packed placement); the compile emits exact
        // code bytes. Strict equality on the code, zero-verified pad tail.
        assert!(want.len() >= got.len(), "reference window shorter than the compile");
        let (code, pad) = want.split_at(got.len());
        assert_eq!(got, code, "undoctored player_air must match the reference window");
        assert!(pad.iter().all(|&b| b == 0), "player_air reference tail must be zero align pad, got {pad:x?}");
    }
}

#[test]
fn p2_air_doctored_reference_diverges() {
    let Some(got) = air_bytes(&PLAIN) else { return };
    let Some(mut want) = ref_window("s4.bin", pins::PLAYER_AIR.plain_base, pins::PLAYER_AIR.plain_len) else {
        return;
    };
    want[0] ^= 0xFF;
    assert_ne!(got, want, "a doctored reference must diverge from the compiled bytes");
}

// ─── GROUND region ───────────────────────────────────────────────────────────

fn ground_bytes(shape: &Shape) -> Option<Vec<u8>> {
    let aeon = player_tree("games/sonic4/player/player_ground.emp")?;
    Some(compile_region(
        &aeon,
        shape,
        "games/sonic4/player/player_ground.emp",
        "player_ground",
        if shape.debug { pins::PLAYER_GROUND.debug_base } else { pins::PLAYER_GROUND.plain_base },
        if shape.debug { pins::PLAYER_GROUND.debug_len } else { pins::PLAYER_GROUND.plain_len },
        &["PState_Ground", "PState_Roll", "Player_Jump"],
    ))
}

#[test]
fn p2_ground_region_matches_reference() {
    let Some(got) = ground_bytes(&PLAIN) else { return };
    if let Some(want) = ref_window("s4.bin", pins::PLAYER_GROUND.plain_base, pins::PLAYER_GROUND.plain_len) {
        assert_region_matches(&got, &want, "player_ground (plain)");
    }
}

#[test]
fn p2_ground_debug_region_matches_reference() {
    let Some(got) = ground_bytes(&DEBUG) else { return };
    if let Some(want) = ref_window("s4.debug.bin", pins::PLAYER_GROUND.debug_base, pins::PLAYER_GROUND.debug_len) {
        assert_region_matches(&got, &want, "player_ground (debug)");
    }
}

#[test]
fn p2_ground_undoctored_compile_equals_the_reference_window() {
    let Some(got) = ground_bytes(&PLAIN) else { return };
    if let Some(want) = ref_window("s4.bin", pins::PLAYER_GROUND.plain_base, pins::PLAYER_GROUND.plain_len) {
        // Packed placement: the pin LEN spans to the next section's aligned
        // base, so the window may end in a short all-zero align pad beyond the
        // lowered image (same tolerance as assert_region_matches).
        let want_trimmed = if want.len() > got.len()
            && want.len() - got.len() < 16
            && want[got.len()..].iter().all(|&b| b == 0)
        {
            &want[..got.len()]
        } else {
            &want[..]
        };
        assert_eq!(got, want_trimmed, "undoctored player_ground must match the reference window");
    }
}

#[test]
fn p2_ground_doctored_reference_diverges() {
    let Some(got) = ground_bytes(&PLAIN) else { return };
    let Some(mut want) = ref_window("s4.bin", pins::PLAYER_GROUND.plain_base, pins::PLAYER_GROUND.plain_len) else {
        return;
    };
    want[0] ^= 0xFF;
    assert_ne!(got, want, "a doctored reference must diverge from the compiled bytes");
}
