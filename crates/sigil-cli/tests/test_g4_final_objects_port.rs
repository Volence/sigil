//! Tranche 39 — the FINAL three game objects: the REAL `test_player.emp` +
//! `test_enemy.emp` + `path_swap.emp` ports, region-level byte gates in BOTH
//! shapes, modelled on `test_g1_objects_port.rs`. After t39 the object bank is
//! ALL-.emp.
//!
//! ## What this port opens
//!
//! - **Three per-file object-bank gates** (`SIGIL_EMP_TEST_PLAYER` /
//!   `SIGIL_EMP_TEST_ENEMY` / `SIGIL_EMP_PATH_SWAP`). test_player and test_enemy
//!   are shape-INVARIANT ($270 / $48 both shapes); **path_swap is SHAPE-DEPENDENT**
//!   ($92 plain / $FA debug — 2 `__DEBUG__` blocks: the reserved-bit `raise_error`
//!   guard + the debug `jmp Draw_Sprite` vs release `rts` tail).
//! - **Two GUARDED overlays** — test_player's `TPlayerV` (dplc_ptr/art_base/
//!   debug_flag; the AS twin's internal-gated header survives) and one UNGUARDED
//!   overlay each for test_enemy (`TEnemyV`) and path_swap (`PathSwapV`).
//! - **The DEBUG-shape `raise_error` seam** (path_swap): the error-handler entry
//!   points MDDBG__ErrorHandler(_PagesController), the load_art_port precedent.
//! - **path_swap's inline `ObjDef_PathSwap`** objdef descriptor (26 B) + the two
//!   procs, and its `dc.l ObjDef_PathSwap` outbound consumers (act_descriptor).
//! - **Zero externs / zero extern-proc**: every callee (ObjectMove/ObjectMoveX/
//!   AnimateSprite/Perform_DPLC/Player_SensorFloor/Draw_Sprite) is a bare link
//!   symbol resolved by the shared link.
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

fn twin_guards() -> usize {
    sigil_harness::test_support::engine_constant_equs().len()
}

const OBJ_CODE_BASE: u32 = pins::OBJ_CODE_BASE.plain;

/// Per-shape cross-seam VMAs + region geometry.
struct Shape {
    debug: bool,
    draw_sprite: u32,
    object_move: u32,
    object_move_x: u32,
    animate_sprite: u32,
    /// bug005: the spawn-descriptor helper (ambient test_helpers) calls the
    /// animator-owned RefreshSpritePieceCount (animate base + REFRESH_OFF).
    refresh_spc: u32,
    perform_dplc: u32,
    player_sensor_floor: u32,
    map_sonic: u32,
    ani_sonic: u32,
    dplc_sonic: u32,
    art_sonic: u32,
    map_test_obj: u32,
    player_1: u32,
    /// C1: the per-slot player-block array base. The `player_block` splice
    /// test_player expands `lea`s it, so it is a cross-seam RAM operand of
    /// this region now.
    player_blocks: u32,
    /// C1: path_swap follows `Camera_Target` (the leader SST pointer in engine
    /// RAM) instead of a hardwired `Player_1`, so the follow works for whichever
    /// character is leading.
    camera_target: u32,
    cheat_flags: u32,
    player_base: u32,
    player_len: usize,
    enemy_base: u32,
    enemy_len: usize,
    swap_base: u32,
    swap_len: usize,
}

const PLAIN: Shape = Shape {
    debug: false,
    draw_sprite: pins::DRAW_SPRITE.plain,
    object_move: pins::OBJECT_MOVE.plain,
    object_move_x: pins::OBJECT_MOVE_X.plain,
    animate_sprite: pins::ANIMATE.plain_base,
    refresh_spc: pins::ANIMATE.plain_base + pins::REFRESH_OFF.plain as u32,
    perform_dplc: pins::DPLC.plain_base,
    player_sensor_floor: pins::PLAYER_SENSOR_FLOOR.plain,
    map_sonic: pins::MAP_SONIC.plain,
    ani_sonic: pins::SONIC_ANIMS.plain_base,
    dplc_sonic: pins::DPLC_SONIC.plain,
    art_sonic: pins::ART_SONIC.plain,
    map_test_obj: pins::MAP_TEST_OBJ.plain,
    player_1: pins::PLAYER_1.plain,
    player_blocks: pins::PLAYER_BLOCKS.plain,
    camera_target: pins::CAMERA_TARGET.plain,
    cheat_flags: pins::CHEAT_FLAGS.plain,
    player_base: pins::TEST_PLAYER.plain_base,
    player_len: pins::TEST_PLAYER.plain_len,
    enemy_base: pins::TEST_ENEMY.plain_base,
    enemy_len: pins::TEST_ENEMY.plain_len,
    swap_base: pins::PATH_SWAP.plain_base,
    swap_len: pins::PATH_SWAP.plain_len,
};
const DEBUG: Shape = Shape {
    debug: true,
    draw_sprite: pins::DRAW_SPRITE.debug,
    object_move: pins::OBJECT_MOVE.debug,
    object_move_x: pins::OBJECT_MOVE_X.debug,
    animate_sprite: pins::ANIMATE.debug_base,
    refresh_spc: pins::ANIMATE.debug_base + pins::REFRESH_OFF.debug as u32,
    perform_dplc: pins::DPLC.debug_base,
    player_sensor_floor: pins::PLAYER_SENSOR_FLOOR.debug,
    map_sonic: pins::MAP_SONIC.debug,
    ani_sonic: pins::SONIC_ANIMS.debug_base,
    dplc_sonic: pins::DPLC_SONIC.debug,
    art_sonic: pins::ART_SONIC.debug,
    map_test_obj: pins::MAP_TEST_OBJ.debug,
    player_1: pins::PLAYER_1.debug,
    player_blocks: pins::PLAYER_BLOCKS.debug,
    camera_target: pins::CAMERA_TARGET.debug,
    cheat_flags: pins::CHEAT_FLAGS.debug,
    player_base: pins::TEST_PLAYER.debug_base,
    player_len: pins::TEST_PLAYER.debug_len,
    enemy_base: pins::TEST_ENEMY.debug_base,
    enemy_len: pins::TEST_ENEMY.debug_len,
    swap_base: pins::PATH_SWAP.debug_base,
    swap_len: pins::PATH_SWAP.debug_len,
};

// objtest-gate (2026-08-05): the PLAIN compile shape. test_player/test_enemy are
// DEBUG-only — their plain pins collapse onto the plain_anchor (overlapping) — so
// the plain-arm COMPILE pins them at harness-private scratch bases while every
// carrier + path_swap keeps its true plain address. Only path_swap's bytes are
// compared against s4.bin; the gated pair just has to keep compiling.
const PLAIN_COMPILE: Shape = Shape {
    // Scratch bases INSIDE the 64KB object bank window (objroutine link words are
    // `X - ObjCodeBase` and must fit 16 bits) but past all real plain content.
    player_base: 0x1E000,
    player_len: pins::TEST_PLAYER.debug_len,
    enemy_base: 0x1F800,
    enemy_len: pins::TEST_ENEMY.debug_len,
    ..PLAIN
};

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

/// C1 ambient items test_player borrows from the player cluster: the `PlayerBlock`
/// record (games/sonic4/config/ram.emp) and the `player_block` splice
/// (games/sonic4/player/player_common.emp). test_player calls the sensor wrappers,
/// which since C1 take a4 = the calling slot's block, so it derives a4 through the
/// SAME splice the real player frame uses — and a comptime fn expands in the
/// CALLER's scope, so both the fn and the type it strides by have to resolve here.
///
/// Filtered to those two items on purpose: prepending either module wholesale would
/// drag player_common's entire code span (and ram.emp's whole RAM map) into a
/// single-module compile whose bytes are being compared against a ROM window.
fn player_block_ambient(aeon: &std::path::Path) -> sigil_frontend_emp::ast::File {
    use sigil_frontend_emp::ast::Item;
    let mut ram = parse_file(&aeon.join("games/sonic4/config/ram.emp"));
    let mut items: Vec<Item> = ram
        .items
        .drain(..)
        .filter(|it| matches!(it, Item::Struct(d) if d.name == "PlayerBlock"))
        .collect();
    items.extend(
        parse_file(&aeon.join("games/sonic4/player/player_common.emp"))
            .items
            .into_iter()
            .filter(|it| matches!(it, Item::ComptimeFn(d) if d.name == "player_block")),
    );
    assert_eq!(items.len(), 2, "expected PlayerBlock + player_block in the C1 ambient set");
    ram.items = items;
    ram
}

fn with_ambient(
    deps: Vec<sigil_frontend_emp::ast::File>,
    main: sigil_frontend_emp::ast::File,
) -> sigil_frontend_emp::ast::File {
    let mut items = Vec::new();
    for d in deps {
        items.extend(d.items);
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
/// the game-config VRAM_TEST_* + the overlay equs the drift guards read + the RAM
/// symbols (Ctrl_1_Held/Press, Player_1 — the width rule picks abs.w from the
/// resolved RAM value; Player_1 is shape-dependent). ONE assemble_equ_pairs call
/// (each emits a `Stub` sentinel, so a second call would collide).
fn as_constant_equs(shape: &Shape) -> Vec<Section> {
    use sigil_harness::test_support::{engine_constant_equs, sst_field_equs};
    let mut pairs = sst_field_equs();
    pairs.extend(engine_constant_equs());
    let obj_code_base = format!("${:X}", OBJ_CODE_BASE);
    pairs.push(("ObjCodeBase", obj_code_base.as_str()));
    pairs.push(("VRAM_TEST_SONIC", "$03C0"));
    pairs.push(("VRAM_TEST_OBJ", "$03E0"));
    // (test_player's _dplc_ptr/_art_base/_debug_flag overlay guards retired at
    // conv-d #48 — test_player.emp owns the TPlayerV overlay, no extern mirror.)
    // Ctrl_1_Held/Press are PIN-SOURCED (t24 rule — never hand-shift a RAM VMA):
    // they ride every RAM-layout parcel automatically. Shape-invariant engine RAM,
    // so `.plain` serves both shapes. I2 (input/replay, 2026-08-02) slid them +4
    // ($8028/$8029 → $802C/$802D) via the Logic_Tick u32 inserted after Frame_Counter.
    let ctrl_1_held = format!("${:X}", pins::CTRL_1_HELD.plain);
    let ctrl_1_press = format!("${:X}", pins::CTRL_1_PRESS.plain);
    pairs.push(("Ctrl_1_Held", ctrl_1_held.as_str()));
    pairs.push(("Ctrl_1_Press", ctrl_1_press.as_str()));
    let player_1 = format!("${:X}", shape.player_1);
    pairs.push(("Player_1", player_1.as_str()));
    // Cheat_Flags: game-side cheat bitfield read by test_player's debug-fly gate.
    // PIN-SOURCED and SHAPE-DEPENDENT, same rule as Player_1 above.
    let cheat_flags = format!("${:X}", shape.cheat_flags);
    pairs.push(("Cheat_Flags", cheat_flags.as_str()));
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// One synthetic AS-side label phased at `vma`.
fn as_label_at(name: &str, vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}")).sections
}

/// The AS-side OUTBOUND consumers: `dc.w objroutine(TestEnemy_Init)` (objdef) and
/// `dc.l ObjDef_PathSwap` (act_descriptor / objdef type table) — both symbols
/// UNDEFINED in-unit (the `.emp` owns them).
fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tdc.w TestEnemy_Init-ObjCodeBase\n\
               \tdc.l ObjDef_PathSwap\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}")).sections
}

fn map_toml(shape: &Shape) -> String {
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
         name = \"test_player\"\n\
         lma_base = {:#x}\n\
         size = {:#x}\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"test_enemy\"\n\
         lma_base = {:#x}\n\
         size = {:#x}\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"path_swap\"\n\
         lma_base = {:#x}\n\
         size = {:#x}\n\
         kind = \"rom\"\n",
        shape.player_base, shape.player_len,
        shape.enemy_base, shape.enemy_len,
        shape.swap_base, shape.swap_len,
    )
}

struct Compiled {
    resolved: Vec<Section>,
    linked: sigil_link::LinkedImage,
    player_guards: usize,
    enemy_guards: usize,
    swap_guards: usize,
    link_asserts: Vec<sigil_ir::LinkAssert>,
}

fn compile_real_files(shape: &Shape) -> Compiled {
    let aeon = aeon_dir();
    let types = || parse_file(&aeon.join("engine/system/types.emp"));
    let sst = || parse_file(&aeon.join("engine/objects/sst.emp"));
    let constants = || parse_file(&aeon.join("engine/system/constants.emp"));
    let objdef = || parse_file(&aeon.join("engine/objects/objdef.emp"));
    // VRAM_TEST_SONIC / VRAM_TEST_OBJ authority (Parcel F: config/constants.asm → `.emp`).
    let game_consts = || parse_file(&aeon.join("games/sonic4/config/constants.emp"));

    let player = parse_file(&aeon.join("games/sonic4/objects/test_player.emp"));
    let enemy = parse_file(&aeon.join("games/sonic4/objects/test_enemy.emp"));
    let swap = parse_file(&aeon.join("games/sonic4/objects/path_swap.emp"));

    let player_file = with_ambient(
        vec![types(), sst(), constants(), objdef(), game_consts(), player_block_ambient(&aeon)],
        player,
    );
    let enemy_file = with_ambient(vec![types(), sst(), constants()], enemy);
    let swap_file = with_ambient(vec![types(), sst(), constants(), objdef(), game_consts()], swap);

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("games/sonic4/objects")),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), i128::from(shape.debug))],
    };
    let mut sections = Vec::new();
    let mut link_asserts = Vec::new();
    let mut player_guards = 0;
    let mut enemy_guards = 0;
    let mut swap_guards = 0;
    for (file, what) in
        [(player_file, "test_player"), (enemy_file, "test_enemy"), (swap_file, "path_swap")]
    {
        let (module, ldiags) = lower_module(&file, &opts);
        assert!(
            ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
            "{what} lower errors: {ldiags:?}"
        );
        let g = sigil_harness::test_support::guard_assert_count(&module.link_asserts);
        match what {
            "test_player" => player_guards = g,
            "test_enemy" => enemy_guards = g,
            _ => swap_guards = g,
        }
        sections.extend(module.sections);
        link_asserts.extend(module.link_asserts);
    }

    let map = sigil_link::load_map(&map_toml(shape)).expect("map must load");
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut groups = vec![
        as_constant_equs(shape),
        as_label_at("Draw_Sprite", shape.draw_sprite),
        as_label_at("ObjectMove", shape.object_move),
        as_label_at("ObjectMoveX", shape.object_move_x),
        as_label_at("AnimateSprite", shape.animate_sprite),
        as_label_at("RefreshSpritePieceCount", shape.refresh_spc),
        as_label_at("Perform_DPLC", shape.perform_dplc),
        as_label_at("Player_SensorFloor", shape.player_sensor_floor),
        as_label_at("Map_Sonic", shape.map_sonic),
        as_label_at("Ani_Sonic", shape.ani_sonic),
        as_label_at("DPLC_Sonic", shape.dplc_sonic),
        as_label_at("Art_Sonic", shape.art_sonic),
        as_label_at("Map_TestObj", shape.map_test_obj),
        as_label_at("Player_Blocks", shape.player_blocks),
        as_label_at("Camera_Target", shape.camera_target),
        as_outbound_consumer(),
    ];
    if shape.debug {
        // path_swap's DEBUG-only raise_error error-handler entry points.
        groups.push(as_label_at("MDDBG__ErrorHandler", pins::MDDBG_ERROR_HANDLER));
        groups.push(as_label_at(
            "MDDBG__ErrorHandler_PagesController",
            pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER,
        ));
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
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    Compiled { resolved, linked, player_guards, enemy_guards, swap_guards, link_asserts }
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

fn ref_window(rom_name: &str, base: usize, len: usize) -> Option<Vec<u8>> {
    let rom_path = aeon_dir().join(rom_name);
    match std::fs::read(&rom_path) {
        Ok(rom) => Some(rom[base..base + len].to_vec()),
        Err(_) => {
            if strict_gate() {
                panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
            }
            eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
            None
        }
    }
}

fn reference_gate(shape: &Shape, rom_name: &str) {
    // objtest-gate: in PLAIN, test_player/test_enemy carry zero bytes — compile
    // via the hybrid shape and byte-gate ONLY path_swap (which still ships).
    let plain_gated = !shape.debug;
    let compile_shape = if plain_gated { &PLAIN_COMPILE } else { shape };
    let (player_ref, enemy_ref) = if plain_gated {
        assert_eq!(pins::TEST_PLAYER.plain_len, 0, "test_player must be plain-empty");
        assert_eq!(pins::TEST_ENEMY.plain_len, 0, "test_enemy must be plain-empty");
        (Vec::new(), Vec::new())
    } else {
        let Some(p) = ref_window(rom_name, shape.player_base as usize, shape.player_len) else {
            return;
        };
        (p, ref_window(rom_name, shape.enemy_base as usize, shape.enemy_len).unwrap())
    };
    let Some(swap_ref) = ref_window(rom_name, shape.swap_base as usize, shape.swap_len) else {
        return;
    };

    let c = compile_real_files(compile_shape);

    // sst.emp's SST_* wall retired at the conv-a structs flip; the constants
    // twin's guards remain. test_player's own VRAM_TEST_SONIC and path_swap's own
    // VRAM_TEST_OBJ mirror guards retired at conv-f (config constants flipped to
    // `.emp`, `use`d now); test_player's _dplc_ptr/_art_base overlay guards had
    // already retired at conv-d #48. test_enemy carries an unguarded overlay.
    assert_eq!(c.player_guards, twin_guards(), "test_player constants twin guards (own VRAM_TEST_SONIC guard retired at conv-f)");
    assert_eq!(c.enemy_guards, twin_guards(), "test_enemy guards (unguarded overlay)");
    assert_eq!(c.swap_guards, twin_guards(), "path_swap constants twin guards (own VRAM_TEST_OBJ guard retired at conv-f)");
    let diags = sigil_link::check_link_asserts(&c.resolved, &SymbolTable::new(), &c.link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the drift guards must all PASS: {diags:?}"
    );

    if !plain_gated {
        let player_sec = c.linked.section("test_player").expect("linked test_player");
        assert_region_matches(&player_sec.bytes, &player_ref, &format!("test_player vs {rom_name}"));
        let enemy_sec = c.linked.section("test_enemy").expect("linked test_enemy");
        assert_region_matches(&enemy_sec.bytes, &enemy_ref, &format!("test_enemy vs {rom_name}"));
    }
    let swap_sec = c.linked.section("path_swap").expect("linked path_swap");
    assert_region_matches(&swap_sec.bytes, &swap_ref, &format!("path_swap vs {rom_name}"));

    // Outbound proof: the AS objroutine(TestEnemy_Init) word + the dc.l
    // ObjDef_PathSwap resolve to the .emp-owned addresses.
    // Located by its `Consumer:` label, not by an ordinal into the harness-private
    // LMA ladder: that index silently pointed at the wrong section the moment C1
    // added two cross-seam label groups ahead of it.
    let consumer_lma = c
        .resolved
        .iter()
        .find(|s| s.labels.iter().any(|l| l.name == "Consumer"))
        .expect("outbound consumer section (the `Consumer:` label)")
        .lma;
    let consumer = c
        .linked
        .sections
        .iter()
        .find(|s| s.lma == consumer_lma)
        .expect("linked outbound consumer");
    let enemy_word = u16::from_be_bytes([consumer.bytes[0], consumer.bytes[1]]);
    assert_eq!(
        enemy_word,
        (compile_shape.enemy_base - OBJ_CODE_BASE) as u16,
        "objdef objroutine(TestEnemy_Init) must resolve to the bank offset"
    );
    let path_swap_addr = u32::from_be_bytes([
        consumer.bytes[2],
        consumer.bytes[3],
        consumer.bytes[4],
        consumer.bytes[5],
    ]);
    assert_eq!(
        path_swap_addr, shape.swap_base,
        "dc.l ObjDef_PathSwap must resolve to the .emp-owned region base"
    );
}

/// (plain) all three regions == `s4.bin` windows.
#[test]
fn g4_objects_regions_match_reference() {
    reference_gate(&PLAIN, "s4.bin");
}

/// (debug) all three regions == `s4.debug.bin` windows (path_swap's per-shape
/// $FA length + the raise_error block).
#[test]
fn g4_objects_debug_regions_match_reference() {
    reference_gate(&DEBUG, "s4.debug.bin");
}

// ---- negative probe + positive control (the t24 rule) -------------------

/// The UNDOCTORED compile equals the reference window (positive control):
/// path_swap's SHAPE-DEPENDENT plain window.
#[test]
fn g4_undoctored_compile_equals_the_reference_window() {
    let shape = &PLAIN;
    let Some(swap_ref) = ref_window("s4.bin", shape.swap_base as usize, shape.swap_len) else {
        return;
    };
    let c = compile_real_files(&PLAIN_COMPILE);
    let swap_sec = c.linked.section("path_swap").expect("linked path_swap");
    // Same align-fill tolerance as the reference gate (defect-batch-8: the parcel's
    // shift left 2 pad bytes inside the pin span, which a raw assert_eq! read as a
    // mismatch while g4_objects_regions_match_reference — using the tolerant
    // comparator — stayed green).
    assert_region_matches(&swap_sec.bytes, &swap_ref, "undoctored path_swap");
}

/// A doctored reference window must NOT match the compiled bytes (the gate can
/// actually fail). Pins-derived.
#[test]
fn g4_doctored_reference_diverges() {
    let shape = &PLAIN;
    let Some(mut swap_ref) = ref_window("s4.bin", shape.swap_base as usize, shape.swap_len) else {
        return;
    };
    let c = compile_real_files(&PLAIN_COMPILE);
    let swap_sec = c.linked.section("path_swap").expect("linked path_swap");
    swap_ref[0] ^= 0xFF;
    assert_ne!(swap_sec.bytes, swap_ref, "a doctored reference must diverge from the compiled bytes");
}
