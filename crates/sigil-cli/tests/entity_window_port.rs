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
        // SECTION_SIZE/_SHIFT/SEC_VOID moved to engine_constant_equs() (the
        // shared twin) at the tranche-15 consolidation — supplied via the
        // engine_constant_equs() extend below, no longer entity_window-local.
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
        // EntityScanState struct (engine/structs.asm), $16 bytes.
        ("EntityScanState_ess_ring_right_idx", "$00"),
        ("EntityScanState_ess_obj_right_idx", "$02"),
        ("EntityScanState_ess_rom_ring_ptr", "$04"),
        ("EntityScanState_ess_rom_obj_ptr", "$08"),
        ("EntityScanState_ess_rom_type_tbl_ptr", "$0C"),
        ("EntityScanState_ess_origin_x", "$10"),
        ("EntityScanState_ess_section_id", "$12"),
        ("EntityScanState_ess_entry_idx", "$13"),
        ("EntityScanState_ess_origin_y", "$14"),
        ("EntityScanState_len", "$16"),
        // Sec (ROM section descriptor) + Act field equs now come from
        // act_sec_field_equs() (the prepended engine.structs drift wall).
    ]
}

/// The full AS-side value seam: SST struct equs + engine constants twin +
/// entity_window's own mirrors/offsets.
fn as_constant_equs() -> Vec<Section> {
    let mut pairs = sigil_harness::test_support::sst_field_equs();
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    // The Act_*/Sec_* field equs feed the prepended engine.structs drift wall
    // (entity_window's Sec.field access + Act_grid_w_lo).
    pairs.extend(sigil_harness::test_support::act_sec_field_equs());
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
    let structs = parse_file(&aeon.join("engine/structs.emp"));
    let ew = parse_file(&aeon.join("engine/objects/entity_window.emp"));

    // entity_window also `use`s engine.structs.{Sec, Act_grid_w_lo}.
    let file = with_ambient(vec![types, sst, constants, structs], ew);

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

// ============================================================================
// Two-module link test (tranche 15) — the campaign's FIRST symbol-ownership
// FLIP, proven (not argued by construction). entity_window.emp and section.emp
// are compiled, placed at their per-shape regions, and linked in ONE
// resolve_layout + link over the union. BIDIRECTIONAL flip:
//   entity_window.emp's `jbsr Section_GetSecPtrXY`/`FlatIDXY` → section.emp,
//   section.emp's `jsr EntityWindow_Init` → entity_window.emp,
// each resolving to the OTHER module's owned symbol (no synthetic label).
// Both regions byte-compare against the reference ROM.
// ============================================================================
use std::collections::BTreeMap;

/// section.emp's own mirrored values (engine.constants comes from
/// entity_window's seam, not re-added). Overlaps (Act_grid_w, Sec_*) dedup in
/// the union, asserting consistent values.
fn section_value_pairs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("VDP_DATA", "$C00000"), ("VDP_CTRL", "$C00004"),
        ("VRAM_PLANE_A", "$C000"), ("VRAM_PLANE_B_BYTES", "$E000"),
        ("PLANE_H_CELLS", "64"), ("PLANE_V_CELLS", "64"), ("PLANE_BUFFER_SIZE", "1536"),
        ("TILE_CACHE_COLS", "80"), ("TILE_CACHE_ROWS", "60"), ("TILE_CACHE_STRIDE", "80"),
        ("TILE_CACHE_NT_SIZE", "9600"),
        ("VRAM", "%100001"), ("CRAM", "%101011"), ("VSRAM", "%100101"),
        ("READ", "%001100"), ("WRITE", "%000111"), ("DMA", "%100111"),
        // Act_*/Sec_* now come from act_sec_field_equs() in the union (section
        // reads them through the prepended engine.structs drift wall).
    ]
}

/// section.emp's cross-seam ADDRESS labels, MINUS EntityWindow_Init (owned by
/// entity_window.emp in the two-module link — the section→entity_window flip).
fn section_labels_for_link(debug: bool) -> Vec<(&'static str, u32)> {
    let pick = |p: pins::Pin| -> u32 { if debug { p.debug } else { p.plain } };
    vec![
        ("Draw_TileColumn", pick(pins::DRAW_TILE_COLUMN)),
        ("Draw_TileRow_FromCache", pick(pins::DRAW_TILE_ROW_FROM_CACHE)),
        ("Camera_X", pick(pins::CAMERA_X)),
        ("Camera_Y", pick(pins::CAMERA_Y)),
        ("Current_Act_Ptr", pick(pins::CURRENT_ACT_PTR)),
        ("Section_Plane_Dirty", pick(pins::SECTION_PLANE_DIRTY)),
        ("Section_Right_Col_Written", pick(pins::SECTION_RIGHT_COL_WRITTEN)),
        ("Section_Left_Col_Written", pick(pins::SECTION_LEFT_COL_WRITTEN)),
        ("Section_Top_Row_Written", pick(pins::SECTION_TOP_ROW_WRITTEN)),
        ("Section_Bottom_Row_Written", pick(pins::SECTION_BOTTOM_ROW_WRITTEN)),
        ("Cache_Left_Col", pick(pins::CACHE_LEFT_COL)),
        ("Cache_Head_Col", pick(pins::CACHE_HEAD_COL)),
        ("Cache_Top_Row", pick(pins::CACHE_TOP_ROW)),
        ("Cache_Bottom_Row", pick(pins::CACHE_BOTTOM_ROW)),
        ("Cache_Origin_Col", pick(pins::CACHE_ORIGIN_COL)),
        ("Cache_Origin_Row", pick(pins::CACHE_ORIGIN_ROW)),
        ("Plane_Buffer_Ptr", pick(pins::PLANE_BUFFER_PTR)),
        ("Tile_Cache_Nametable", pick(pins::TILE_CACHE_NAMETABLE)),
    ]
}

/// Lower one .emp (ambient deps prepended), place into a single-region map.
fn lower_and_place(
    emp_path: &std::path::Path,
    ambient: Vec<sigil_frontend_emp::ast::File>,
    include_root: PathBuf,
    region: &str,
    base: u32,
    len: usize,
    debug: bool,
) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>) {
    let file = with_ambient(ambient, parse_file(emp_path));
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(include_root),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), if debug { 1 } else { 0 })],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} lower errors: {ldiags:?}", emp_path.display()
    );
    let mt = format!(
        "fill = 0x00\n\n[[region]]\nname = \"text\"\nlma_base = 0x0000\nsize = 0x10\nkind = \"rom\"\n\n[[region]]\nname = \"{region}\"\nlma_base = {base:#x}\nsize = {len:#x}\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&mt).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} place errors: {pdiags:?}", emp_path.display()
    );
    (sections, module.link_asserts)
}

fn two_module_flip(shape: &Shape, debug: bool, rom_name: &str) {
    let aeon = aeon_dir();
    let Ok(refrom) = std::fs::read(aeon.join(rom_name)) else {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {rom_name}");
        }
        eprintln!("skip: reference ROM {rom_name} missing");
        return;
    };

    let sec_base = if debug { pins::SECTION.debug_base } else { pins::SECTION.plain_base };
    let (mut sec_sections, sec_asserts) = lower_and_place(
        &aeon.join("engine/level/section.emp"),
        vec![
            parse_file(&aeon.join("engine/system/constants.emp")),
            parse_file(&aeon.join("engine/structs.emp")),
            parse_file(&aeon.join("engine/vdp.emp")),
        ],
        aeon.join("engine/level"),
        "section",
        sec_base,
        pins::SECTION.plain_len,
        debug,
    );
    let (mut ew_sections, ew_asserts) = lower_and_place(
        &aeon.join("engine/objects/entity_window.emp"),
        vec![
            parse_file(&aeon.join("engine/system/types.emp")),
            parse_file(&aeon.join("engine/objects/sst.emp")),
            parse_file(&aeon.join("engine/system/constants.emp")),
            parse_file(&aeon.join("engine/structs.emp")),
        ],
        aeon.join("engine/objects"),
        "entity_window",
        shape.base,
        shape.len,
        debug,
    );

    // Union the value seam (dedup, assert consistent).
    let mut vmap: BTreeMap<&str, &str> = BTreeMap::new();
    for pairs in [
        sigil_harness::test_support::sst_field_equs(),
        sigil_harness::test_support::engine_constant_equs(),
        sigil_harness::test_support::act_sec_field_equs(),
        entity_window_equs(),
        section_value_pairs(),
    ] {
        for (n, v) in pairs {
            if let Some(prev) = vmap.insert(n, v) {
                assert_eq!(prev, v, "seam value conflict for `{n}`");
            }
        }
    }
    let vpairs: Vec<(&str, &str)> = vmap.into_iter().collect();

    // Union the address labels: entity_window's MINUS the two flipped to
    // section.emp; section's MINUS EntityWindow_Init (flipped to entity_window).
    let mut lmap: BTreeMap<&str, u32> = BTreeMap::new();
    for (n, v) in shape.labels {
        if *n == "Section_GetSecPtrXY" || *n == "Section_FlatIDXY" { continue; }
        lmap.insert(n, *v);
    }
    for (n, v) in section_labels_for_link(debug) {
        if let Some(prev) = lmap.insert(n, v) {
            assert_eq!(prev, v, "label VMA conflict for `{n}`: {prev:#x} vs {v:#x}");
        }
    }

    let mut sections = Vec::new();
    sections.append(&mut sec_sections);
    sections.append(&mut ew_sections);
    let mut lma = 0x0100_0000u32;
    let mut equs = sigil_harness::test_support::assemble_equ_pairs(&vpairs);
    for sec in &mut equs {
        sec.lma = lma;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.append(&mut equs);
    lma += 0x10_0000;
    for (name, vma) in lmap {
        let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
        for mut s in assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS ({name}): {d:?}")).sections.drain(..) {
            s.lma = lma;
            s.placement = SectionPlacement::Pinned;
            s.group = None;
            sections.push(s);
        }
        lma += 0x10_0000;
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));

    let mut all = sec_asserts;
    all.extend(ew_asserts);
    let adiags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &all);
    assert!(adiags.iter().all(|d| d.level != sigil_span::Level::Error), "drift guards: {adiags:?}");

    let sr = &refrom[sec_base as usize..sec_base as usize + pins::SECTION.plain_len];
    assert_region_matches(&linked.section("section").expect("section region").bytes, sr, "section (two-module flip)");
    let er = &refrom[shape.base as usize..shape.base as usize + shape.len];
    assert_region_matches(&linked.section("entity_window").expect("entity_window region").bytes, er, "entity_window (two-module flip)");
}

#[test]
fn two_module_ownership_flip_plain() {
    two_module_flip(&PLAIN, false, "s4.bin");
}

#[test]
fn two_module_ownership_flip_debug() {
    two_module_flip(&DEBUG, true, "s4.debug.bin");
}
