//! Tranche 12 — the REAL `entity_window.emp` port, region-level byte gate.
//!
//! Compiles the actual ported file — `engine/objects/entity_window.emp` — through
//! the production parse → lower → place → resolve → link pipeline and asserts the
//! `entity_window` region's flattened bytes equal the reference ROM window at the
//! pinned addresses, in BOTH build shapes.
//!
//! The largest port yet (30 procs) and the ratifying demand for the diagnostics
//! construct: 11 `assert` sites + 3 `if DEBUG == 1 {}` blocks make the DEBUG shape
//! ~$456 bytes longer than plain (shape-dependent length, like rings/core).
//!
//! Cross-seam INBOUND: sst.emp's SST_* struct equs + the engine constants twin +
//! entity_window's own 24 mirrored consts + 17 struct-field offsets (local
//! `ensure(extern(…))` guards). INBOUND labels at per-shape VMAs: the Entity_* /
//! Ring_Collected_* / collected-park RAM cells, Camera_*, Current_Act_Ptr, the
//! dynamic live-list cells, and the external code targets (RingBuffer_*,
//! Section_*, Load_Object, DeleteObject) + (debug only) the assert construct's
//! MDDBG__ErrorHandler* entries.
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`). Absent, the
//! gates SKIP green unless `SIGIL_STRICT_GATE=1`.

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

struct Shape {
    base: u32,
    len: usize,
    labels: &'static [(&'static str, u32)],
}

const PLAIN: Shape = Shape {
    base: pins::ENTITY_WINDOW.plain_base,
    len: pins::ENTITY_WINDOW.plain_len,
    labels: &[
        ("Camera_X", pins::CAMERA_X.plain),
        ("Camera_Y", pins::CAMERA_Y.plain),
        ("Camera_Y_Coarse_Prev", pins::CAMERA_Y_COARSE_PREV.plain),
        ("Current_Act_Ptr", pins::CURRENT_ACT_PTR.plain),
        ("Ring_Count", pins::RING_COUNT.plain),
        ("Ring_Buffer", pins::RING_BUFFER.plain),
        ("Entity_Window_Active", pins::ENTITY_WINDOW_ACTIVE.plain),
        ("Entity_Window_Anchor", pins::ENTITY_WINDOW_ANCHOR.plain),
        ("Entity_Window_OriginX", pins::ENTITY_WINDOW_ORIGIN_X.plain),
        ("Entity_Window_OriginY", pins::ENTITY_WINDOW_ORIGIN_Y.plain),
        ("Entity_Window_Center_ID", pins::ENTITY_WINDOW_CENTER_ID.plain),
        ("Entity_Scan_State", pins::ENTITY_SCAN_STATE.plain),
        ("Entity_Loaded_Masks", pins::ENTITY_LOADED_MASKS.plain),
        ("Entity_Mask_Scratch", pins::ENTITY_MASK_SCRATCH.plain),
        ("Ring_Collected_Window", pins::RING_COLLECTED_WINDOW.plain),
        ("Ring_Collected_Park", pins::RING_COLLECTED_PARK.plain),
        ("Collected_Park_Next", pins::COLLECTED_PARK_NEXT.plain),
        ("Dynamic_Live", pins::DYNAMIC_LIVE.plain),
        ("Dynamic_Live_Count", pins::DYNAMIC_LIVE_COUNT.plain),
        ("RingBuffer_Clear", pins::RING_BUFFER_CLEAR.plain),
        ("RingBuffer_Add", pins::RINGS.plain_base),
        ("RingBuffer_Remove", pins::RING_BUFFER_REMOVE.plain),
        ("Section_GetSecPtrXY", pins::SECTION_GET_SEC_PTR_XY.plain),
        ("Section_FlatIDXY", pins::SECTION_FLAT_IDXY.plain),
        ("Load_Object", pins::LOAD_OBJECT.plain_base),
        ("DeleteObject", pins::DELETE_OBJECT.plain),
    ],
};

const DEBUG: Shape = Shape {
    base: pins::ENTITY_WINDOW.debug_base,
    len: pins::ENTITY_WINDOW.debug_len,
    labels: &[
        ("Camera_X", pins::CAMERA_X.debug),
        ("Camera_Y", pins::CAMERA_Y.debug),
        ("Camera_Y_Coarse_Prev", pins::CAMERA_Y_COARSE_PREV.debug),
        ("Current_Act_Ptr", pins::CURRENT_ACT_PTR.debug),
        ("Ring_Count", pins::RING_COUNT.debug),
        ("Ring_Buffer", pins::RING_BUFFER.debug),
        ("Entity_Window_Active", pins::ENTITY_WINDOW_ACTIVE.debug),
        ("Entity_Window_Anchor", pins::ENTITY_WINDOW_ANCHOR.debug),
        ("Entity_Window_OriginX", pins::ENTITY_WINDOW_ORIGIN_X.debug),
        ("Entity_Window_OriginY", pins::ENTITY_WINDOW_ORIGIN_Y.debug),
        ("Entity_Window_Center_ID", pins::ENTITY_WINDOW_CENTER_ID.debug),
        ("Entity_Scan_State", pins::ENTITY_SCAN_STATE.debug),
        ("Entity_Loaded_Masks", pins::ENTITY_LOADED_MASKS.debug),
        ("Entity_Mask_Scratch", pins::ENTITY_MASK_SCRATCH.debug),
        ("Ring_Collected_Window", pins::RING_COLLECTED_WINDOW.debug),
        ("Ring_Collected_Park", pins::RING_COLLECTED_PARK.debug),
        ("Collected_Park_Next", pins::COLLECTED_PARK_NEXT.debug),
        ("Dynamic_Live", pins::DYNAMIC_LIVE.debug),
        ("Dynamic_Live_Count", pins::DYNAMIC_LIVE_COUNT.debug),
        // A2 walk-live rail rider (item 12): DespawnObjects sets/clears this
        // DEBUG-only flag around its live-list walk.
        ("Dynamic_Live_Walking", pins::DYNAMIC_LIVE_WALKING),
        ("RingBuffer_Clear", pins::RING_BUFFER_CLEAR.debug),
        ("RingBuffer_Add", pins::RINGS.debug_base),
        ("RingBuffer_Remove", pins::RING_BUFFER_REMOVE.debug),
        ("Section_GetSecPtrXY", pins::SECTION_GET_SEC_PTR_XY.debug),
        ("Section_FlatIDXY", pins::SECTION_FLAT_IDXY.debug),
        ("Load_Object", pins::LOAD_OBJECT.debug_base),
        ("DeleteObject", pins::DELETE_OBJECT.debug),
        // Debug shape only: the assert construct's error-handler entries.
        ("MDDBG__ErrorHandler", pins::MDDBG_ERROR_HANDLER),
        ("MDDBG__ErrorHandler_PagesController", pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER),
    ],
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

/// entity_window.emp's own mirrored constants (games/sonic4/config + engine
/// constants not in the shared twin) and the EntityScanState/Sec/Act field
/// offsets its `ensure(extern(…))` guards read back. SOURCE OF TRUTH:
/// games/sonic4/config/constants.asm + engine/constants.asm + engine/structs.asm.
fn entity_window_equs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("COLLECTED_WINDOW_SLOTS", "9"),
        ("COLLECTED_SLOT_SIZE", "34"),
        ("COLLECTED_PARK_SLOTS", "4"),
        ("COLLECTED_PARK_ENTRY_SIZE", "33"),
        ("SECTION_SIZE", "$800"),
        ("SECTION_SIZE_SHIFT", "11"),
        ("SEC_VOID", "$FF"),
        ("MAX_TRACKED_SECTIONS", "4"),
        ("MAX_LIST_ENTRIES", "128"),
        ("ENTITY_LOAD_BUFFER", "$180"),
        ("ENTITY_DESPAWN_BUFFER", "$200"),
        ("ENTITY_LOAD_BUFFER_Y", "$100"),
        ("ENTITY_DESPAWN_BUFFER_Y", "$180"),
        ("ENTITY_LOADED_SLOT_SIZE", "32"),
        ("ENTITY_LOADED_OBJ_OFFSET", "16"),
        ("ENTITY_RESCAN_COARSE_MASK", "$FF80"),
        ("COLLECTED_BITMASK_OFFSET", "2"),
        ("KILLED_BITMASK_OFFSET", "18"),
        ("COLLECTED_EMPTY_TAG", "$FF"),
        ("COLLECTED_MASK_BYTES", "16"),
        ("OEF_TYPE_SHIFT", "8"),
        ("OEF_TYPE_MASK", "$1F"),
        ("OBJ_ENTRY_SIZE", "6"),
        ("RING_BUFFER_ENTRY_SIZE", "6"),
        // EntityScanState struct (engine/structs.asm), $1A bytes.
        ("EntityScanState_ess_ring_right_idx", "$00"),
        ("EntityScanState_ess_ring_left_idx", "$02"),
        ("EntityScanState_ess_obj_right_idx", "$04"),
        ("EntityScanState_ess_obj_left_idx", "$06"),
        ("EntityScanState_ess_rom_ring_ptr", "$08"),
        ("EntityScanState_ess_rom_obj_ptr", "$0C"),
        ("EntityScanState_ess_rom_type_tbl_ptr", "$10"),
        ("EntityScanState_ess_origin_x", "$14"),
        ("EntityScanState_ess_section_id", "$16"),
        ("EntityScanState_ess_entry_idx", "$17"),
        ("EntityScanState_ess_origin_y", "$18"),
        ("EntityScanState_len", "$1A"),
        // Sec (ROM section descriptor) + Act fields used here.
        ("Sec_sec_objects", "$04"),
        ("Sec_sec_rings", "$08"),
        ("Sec_sec_type_table", "$20"),
        ("Act_grid_w", "$04"),
    ]
}

/// The full AS-side value seam: SST struct equs + engine constants twin +
/// entity_window's own mirrors/offsets.
fn as_constant_equs() -> Vec<Section> {
    let mut pairs = sigil_harness::test_support::sst_field_equs();
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    pairs.extend(entity_window_equs());
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

fn as_label_at(name: &str, vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}")).sections
}

fn map_toml(base: u32, len: usize) -> String {
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
         name = \"entity_window\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

fn compile_real_file(
    shape: &Shape,
    defines: &[(&str, i128)],
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let aeon = aeon_dir();
    let types = parse_file(&aeon.join("engine/system/types.emp"));
    let sst = parse_file(&aeon.join("engine/objects/sst.emp"));
    let constants = parse_file(&aeon.join("engine/system/constants.emp"));
    let ew = parse_file(&aeon.join("engine/objects/entity_window.emp"));

    let file = with_ambient(vec![types, sst, constants], ew);

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("engine/objects")),
        embed_base: None,
        defines: defines.iter().map(|(n, v)| (n.to_string(), *v)).collect(),
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "entity_window.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(shape.base, shape.len)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut lma = 0x0100_0000u32;
    let mut groups: Vec<Vec<Section>> = vec![as_constant_equs()];
    for (name, vma) in shape.labels {
        groups.push(as_label_at(name, *vma));
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
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

/// All drift guards (sst.emp's 30 + constants.emp's + entity_window.emp's) must
/// be captured and PASS.
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the drift guards must all PASS: {diags:?}"
    );
}

fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
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

fn reference_gate(shape: &Shape, rom_name: &str, debug_define: i128) {
    let rom_path = aeon_dir().join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let defines: Vec<(&str, i128)> =
        vec![("DEBUG", debug_define), ("SOUND_DRIVER_ENABLED", 1)];
    let (resolved, linked, link_asserts) = compile_real_file(shape, &defines);
    assert_drift_guards(&resolved, &link_asserts);

    let base = shape.base as usize;
    let section = linked.section("entity_window").expect("linked image must carry entity_window");
    assert_region_matches(
        &section.bytes,
        &refrom[base..base + shape.len],
        &format!("entity_window vs {rom_name}[{base:#x}..{:#x}]", base + shape.len),
    );
}

/// (plain) the `entity_window` region == `s4.bin[0x3388..0x3C76]` — DEBUG=0.
#[test]
fn entity_window_region_matches_reference() {
    reference_gate(&PLAIN, "s4.bin", 0);
}

/// (debug) the `entity_window` region == `s4.debug.bin[0x3838..0x457C]` — DEBUG=1,
/// including the 11 asserts + 3 __DEBUG__ blocks and their FSTRING dc.b data.
#[test]
fn entity_window_debug_region_matches_reference() {
    reference_gate(&DEBUG, "s4.debug.bin", 1);
}
