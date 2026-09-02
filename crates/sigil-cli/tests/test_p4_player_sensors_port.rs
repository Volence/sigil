//! Tranche 38 — game-side P4: player_sensors (the collision sensor primitives,
//! the player-cluster CLOSE). Region-level byte gate in BOTH shapes, modelled on
//! `test_p2_player_states_port.rs`.
//!
//! player_sensors lives in the ENGINE BLOCK (main.asm gameEngineBlockIncludes,
//! the first include) — a called primitive with NO code_addr entry points. Gate
//! `SIGIL_EMP_PLAYER_SENSORS`. Unlike the object-bank state files, its region
//! BASE shifts with upstream __DEBUG__ growth ($50A8 plain / $5E40 debug), but
//! its OWN layout is shape-invariant ($4FC both) — so the content bytes track
//! per-shape cross-seam operands (the Collision_GetType bsr.w displacement + the
//! four ROM-table abs.l addresses) → compile twice. C1 retired the fifth such
//! operand: the quadrant the surface body rotates by is per-slot player-block
//! state now, read `PBLK_QUADRANT(a4)`, so no RAM address enters these bytes.
//!
//! REFERENCE-DEPENDENT: the sources and the reference ROMs live in the sibling
//! `aeon` tree (`AEON_DIR`, or `EMPYREAN_SUITE_ROOT`). Absent,
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
use std::path::PathBuf;

/// `Some(aeon_root)` when the tree carries every source this gate compiles —
/// `player_sensors.emp` plus its four ambient engine modules; `None` — skip
/// green — when it does not (a panic under `SIGIL_STRICT_GATE=1`).
fn sensors_tree() -> Option<PathBuf> {
    reference_tree(&[
        "engine/system/types.emp",
        "engine/objects/sst.emp",
        "engine/system/constants.emp",
        "engine/coords.emp",
        "games/sonic4/player/player_sensors.emp",
    ])
}

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


/// The `PlayerBlock` record (games/sonic4/config/ram.emp) — needed since the
/// character-lens-sweep parcel bound each module's PBLK_* displacements to the type
/// that owns the layout (`ensure(PBLK_X == offsetof(PlayerBlock, x))`). 16 of the 21
/// displacements across 8 files previously carried no guard at all; the guards are
/// comptime-only in the ROM but they ARE a cross-seam reference, so the
/// isolated-lowering ambient set has to carry the struct. Filtered to the struct
/// alone: ram.emp's `region`/`vars` items would drag the whole game RAM map in.
fn player_block_struct_items(aeon: &std::path::Path) -> Vec<sigil_frontend_emp::ast::Item> {
    use sigil_frontend_emp::ast::Item;
    let file = parse_file(&aeon.join("games/sonic4/config/ram.emp"));
    file.items
        .into_iter()
        .filter(|it| matches!(it, Item::Struct(d) if d.name == "PlayerBlock"))
        .collect()
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

/// The AS-side value seam: SST field equs (incl. SST_interact) + engine constants
/// (ST_*, PLAYER_X_RADIUS) + the file-local SOLID_* mirror the ensures resolve
/// against. Shape-independent since C1: the module's last shape-dependent extern
/// was `Player_Quadrant`, and that cell no longer exists.
fn as_constant_equs() -> Vec<Section> {
    use sigil_harness::test_support::{engine_constant_equs, sst_field_equs};
    let mut pairs = sst_field_equs();
    pairs.extend(engine_constant_equs());
    // player_sensors' file-local engine-constant mirror (1st .emp consumer).
    pairs.push(("SOLID_TOP", "1"));
    pairs.push(("SOLID_LRB", "2"));
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

struct Shape {
    collision_get_type: u32,
    solidity_table: u32,
    angle_table: u32,
    height_maps: u32,
    height_maps_rot: u32,
    base: u32,
    len: usize,
    rom: &'static str,
}

const PLAIN: Shape = Shape {
    collision_get_type: pins::COLLISION_GET_TYPE.plain,
    solidity_table: pins::SOLIDITY_TABLE.plain,
    angle_table: pins::ANGLE_TABLE.plain,
    height_maps: pins::HEIGHT_MAPS.plain,
    height_maps_rot: pins::HEIGHT_MAPS_ROT.plain,
    base: pins::PLAYER_SENSORS.plain_base,
    len: pins::PLAYER_SENSORS.plain_len,
    rom: "s4.bin",
};

const DEBUG: Shape = Shape {
    collision_get_type: pins::COLLISION_GET_TYPE.debug,
    solidity_table: pins::SOLIDITY_TABLE.debug,
    angle_table: pins::ANGLE_TABLE.debug,
    height_maps: pins::HEIGHT_MAPS.debug,
    height_maps_rot: pins::HEIGHT_MAPS_ROT.debug,
    base: pins::PLAYER_SENSORS.debug_base,
    len: pins::PLAYER_SENSORS.debug_len,
    rom: "s4.debug.bin",
};

/// Compile player_sensors.emp as its own region against the pinned seam.
fn sensors_bytes(shape: &Shape) -> Option<Vec<u8>> {
    let aeon = sensors_tree()?;
    let types = || parse_file(&aeon.join("engine/system/types.emp")).items;
    let sst = || parse_file(&aeon.join("engine/objects/sst.emp")).items;
    let constants = || parse_file(&aeon.join("engine/system/constants.emp")).items;
    // coords ambient: player_sensors adopts abs_w (engine.coords) at Player_AtLedgeEdge
    // (the |player-center − object-center| balance probe); the fn must be in scope.
    let coords = || parse_file(&aeon.join("engine/coords.emp")).items;

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("games/sonic4/player")),
        embed_base: None,
        defines: vec![("SOUND_DRIVER_ENABLED".to_string(), 1)],
    };

    let main = parse_file(&aeon.join("games/sonic4/player/player_sensors.emp"));
    let file = with_ambient(vec![types(), sst(), constants(), coords(), player_block_struct_items(&aeon)], main);
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "player_sensors lower errors: {ldiags:?}"
    );
    let mut sections = module.sections;

    let map = sigil_link::load_map(&map_toml("player_sensors", shape.base, shape.len)).expect("map");
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "player_sensors place_sections errors: {pdiags:?}"
    );

    // Cross-seam labels pinned at their reference addresses: the surviving-AS
    // collision lookup proc + its four ROM tables.
    let labels: [(&str, u32); 5] = [
        ("Collision_GetType", shape.collision_get_type),
        ("SolidityTable", shape.solidity_table),
        ("AngleTable", shape.angle_table),
        ("HeightMaps", shape.height_maps),
        ("HeightMapsRot", shape.height_maps_rot),
    ];

    let mut groups: Vec<Vec<Section>> = vec![as_constant_equs()];
    for (name, addr) in labels {
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
        .unwrap_or_else(|d| panic!("player_sensors resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("player_sensors link failed: {d:?}"));
    Some(linked.section("player_sensors").expect("linked region").bytes.clone())
}

#[test]
fn p4_sensors_region_matches_reference() {
    let Some(got) = sensors_bytes(&PLAIN) else { return };
    if let Some(want) = ref_window(PLAIN.rom, PLAIN.base, PLAIN.len) {
        assert_region_matches(&got, &want, "player_sensors (plain)");
    }
}

#[test]
fn p4_sensors_debug_region_matches_reference() {
    let Some(got) = sensors_bytes(&DEBUG) else { return };
    if let Some(want) = ref_window(DEBUG.rom, DEBUG.base, DEBUG.len) {
        assert_region_matches(&got, &want, "player_sensors (debug)");
    }
}

#[test]
fn p4_sensors_undoctored_compile_equals_the_reference_window() {
    let Some(got) = sensors_bytes(&PLAIN) else { return };
    if let Some(want) = ref_window(PLAIN.rom, PLAIN.base, PLAIN.len) {
        assert_eq!(got, want, "undoctored player_sensors must match the reference window");
    }
}

#[test]
fn p4_sensors_doctored_reference_diverges() {
    let Some(got) = sensors_bytes(&PLAIN) else { return };
    let Some(mut want) = ref_window(PLAIN.rom, PLAIN.base, PLAIN.len) else {
        return;
    };
    want[0] ^= 0xFF;
    assert_ne!(got, want, "a doctored reference must diverge from the compiled bytes");
}
