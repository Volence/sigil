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
//! four ROM-table abs.l addresses + the Player_Quadrant abs.w) → compile twice.
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

/// The AS-side value seam: SST field equs (incl. SST_interact) + engine constants
/// (ST_*, PLAYER_X_RADIUS) + the file-local SOLID_* mirror the ensures resolve
/// against + Player_Quadrant (shape-dependent RAM addr, abs.w).
fn as_constant_equs(shape: &Shape) -> Vec<Section> {
    use sigil_harness::test_support::{engine_constant_equs, sst_field_equs};
    let mut pairs = sst_field_equs();
    pairs.extend(engine_constant_equs());
    // player_sensors' file-local engine-constant mirror (1st .emp consumer).
    pairs.push(("SOLID_TOP", "1"));
    pairs.push(("SOLID_LRB", "2"));
    // Player_Quadrant — RAM, shape-dependent, read as (Player_Quadrant).w.
    let player_quadrant = format!("${:X}", shape.player_quadrant);
    pairs.push(("Player_Quadrant", player_quadrant.as_str()));
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

struct Shape {
    collision_get_type: u32,
    solidity_table: u32,
    angle_table: u32,
    height_maps: u32,
    height_maps_rot: u32,
    player_quadrant: u32,
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
    player_quadrant: pins::PLAYER_QUADRANT.plain,
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
    player_quadrant: pins::PLAYER_QUADRANT.debug,
    base: pins::PLAYER_SENSORS.debug_base,
    len: pins::PLAYER_SENSORS.debug_len,
    rom: "s4.debug.bin",
};

/// Compile player_sensors.emp as its own region against the pinned seam.
fn sensors_bytes(shape: &Shape) -> Vec<u8> {
    let aeon = aeon_dir();
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
    let file = with_ambient(vec![types(), sst(), constants(), coords()], main);
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
    // collision lookup proc + its four ROM tables (Player_Quadrant is an equ).
    let labels: [(&str, u32); 5] = [
        ("Collision_GetType", shape.collision_get_type),
        ("SolidityTable", shape.solidity_table),
        ("AngleTable", shape.angle_table),
        ("HeightMaps", shape.height_maps),
        ("HeightMapsRot", shape.height_maps_rot),
    ];

    let mut groups: Vec<Vec<Section>> = vec![as_constant_equs(shape)];
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
    linked.section("player_sensors").expect("linked region").bytes.clone()
}

#[test]
fn p4_sensors_region_matches_reference() {
    let got = sensors_bytes(&PLAIN);
    if let Some(want) = ref_window(PLAIN.rom, PLAIN.base, PLAIN.len) {
        assert_region_matches(&got, &want, "player_sensors (plain)");
    }
}

#[test]
fn p4_sensors_debug_region_matches_reference() {
    let got = sensors_bytes(&DEBUG);
    if let Some(want) = ref_window(DEBUG.rom, DEBUG.base, DEBUG.len) {
        assert_region_matches(&got, &want, "player_sensors (debug)");
    }
}

#[test]
fn p4_sensors_undoctored_compile_equals_the_reference_window() {
    let got = sensors_bytes(&PLAIN);
    if let Some(want) = ref_window(PLAIN.rom, PLAIN.base, PLAIN.len) {
        assert_eq!(got, want, "undoctored player_sensors must match the reference window");
    }
}

#[test]
fn p4_sensors_doctored_reference_diverges() {
    let got = sensors_bytes(&PLAIN);
    let Some(mut want) = ref_window(PLAIN.rom, PLAIN.base, PLAIN.len) else {
        return;
    };
    want[0] ^= 0xFF;
    assert_ne!(got, want, "a doctored reference must diverge from the compiled bytes");
}
