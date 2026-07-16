//! Tranche 17 — the REAL `plane_buffer.emp` port, region-level byte gate.
//!
//! Compiles the actual ported file — `engine/level/plane_buffer.emp` — through
//! the production parse -> lower -> place -> resolve -> link pipeline and asserts
//! the `plane_buffer` region's flattened bytes equal the reference ROM window at
//! the pinned base, in BOTH build shapes. The VDP-side DRAW half of the streaming
//! engine (section.emp's paired *draw* sibling); producers append column/row
//! entries to `Plane_Buffer`, `VInt_DrawLevel` drains them to the VDP in VBlank.
//!
//! ## Shape
//! SHAPE-INVARIANT length ($29C both shapes — plane_buffer.asm has NO `__DEBUG__`
//! code, no asserts), like section/load_object; only the base shifts (plain
//! `$405E`, debug `$4C28`), driven entirely by the upstream base slide.
//!
//! ## Cross-seam symbols (synthetic AS-side sections, per-shape VMAs)
//! - RAM labels (abs operands): the `Plane_Buffer`(_Ptr), `Cache_*`,
//!   `Section_Right_Col_Written`, `Current_Act_Ptr`, `Tile_Cache_Nametable`
//!   addresses. plane_buffer is a LEAF — it calls nothing cross-seam, so there
//!   are NO ROM transfer targets.
//! - `engine.constants` twin (`use …TILE_CACHE_*`) + `engine.structs` (`use Act,
//!   Sec`) ride via the ambient prepend; their drift guards ride this gate.
//!   plane_buffer.emp's OWN mirrors (VDP/plane consts) are supplied by
//!   `plane_buffer_value_equs`.
//!
//! ## Two-module ownership flip (the campaign's 3rd flip)
//! `two_module_ownership_flip_{plain,debug}` compiles plane_buffer.emp +
//! section.emp together, links ONE image over the union, and DROPS section's
//! synthetic `Draw_TileColumn`/`Draw_TileRow_FromCache` labels (now owned by
//! plane_buffer.emp). section's 4 `jbsr` bytes match the reference ONLY when each
//! `jbsr->bsr.w` disp lands on plane_buffer.emp's pinned symbol VMA — the flip,
//! proven per shape. (entity_window/tile_cache are the jbsr/tail-call templates.)
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test plane_buffer_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn region_base(debug: bool) -> u32 {
    if debug { pins::PLANE_BUFFER.debug_base } else { pins::PLANE_BUFFER.plain_base }
}

fn region_len(debug: bool) -> usize {
    if debug { pins::PLANE_BUFFER.debug_len } else { pins::PLANE_BUFFER.plain_len }
}

fn aeon_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    PathBuf::from(aeon)
}

fn level_dir() -> PathBuf {
    aeon_dir().join("engine/level")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The map: a `text` carrier for the zero-byte default section, and the
/// `plane_buffer` region pinned at the per-shape reference base + length
/// (SHAPE-INVARIANT: $29C both shapes).
fn map_toml(debug: bool) -> String {
    let base = region_base(debug);
    let len = region_len(debug);
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
         name = \"plane_buffer\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// plane_buffer.emp's OWN mirrored constants (truth: engine/constants.asm, except
/// VRAM_PLANE_B_BYTES from engine/level/bg.asm) — the values its drift-guard
/// `ensure`s read back through `extern()`. `doctor` overrides ONE pair's value
/// (the negative probe). TILE_CACHE_*/Act_*/Sec_* ride the prepended twins.
fn plane_buffer_value_equs(doctor: Option<(&str, &str)>) -> Vec<Section> {
    let mut pairs: Vec<(&str, &str)> = vec![
        ("VRAM_PLANE_A", "$C000"),
        ("VRAM_PLANE_B_BYTES", "$E000"),
        ("VDP_DATA", "$C00000"),
        ("VDP_CTRL", "$C00004"),
        ("PLANE_H_CELLS", "64"),
        ("PLANE_V_CELLS", "64"),
        ("PLANE_BUFFER_SIZE", "1536"),
    ];
    if let Some((name, val)) = doctor {
        for p in pairs.iter_mut() {
            if p.0 == name {
                p.1 = val;
            }
        }
    }
    // engine.constants twin values (incl. TILE_CACHE_*) feed the prepended
    // constants.emp drift wall; Act_*/Sec_* feed the prepended structs.emp wall.
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    pairs.extend(sigil_harness::test_support::act_sec_field_equs());
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// The cross-seam ADDRESS symbols — RAM labels only (plane_buffer is a LEAF, no
/// ROM transfer targets) — each a `phase`d one-byte carrier at its true per-shape
/// VMA (label position is load-bearing: abs.w/abs.l width selection reads it).
fn plane_buffer_addr_labels(debug: bool) -> Vec<Section> {
    let pick = |p: pins::Pin| -> u32 { if debug { p.debug } else { p.plain } };
    let table: [(&str, u32); 11] = [
        ("Plane_Buffer", pick(pins::PLANE_BUFFER_BASE)),
        ("Plane_Buffer_Ptr", pick(pins::PLANE_BUFFER_PTR)),
        ("Cache_Left_Col", pick(pins::CACHE_LEFT_COL)),
        ("Cache_Head_Col", pick(pins::CACHE_HEAD_COL)),
        ("Cache_Top_Row", pick(pins::CACHE_TOP_ROW)),
        ("Cache_Bottom_Row", pick(pins::CACHE_BOTTOM_ROW)),
        ("Cache_Origin_Col", pick(pins::CACHE_ORIGIN_COL)),
        ("Cache_Origin_Row", pick(pins::CACHE_ORIGIN_ROW)),
        ("Tile_Cache_Nametable", pick(pins::TILE_CACHE_NAMETABLE)),
        ("Section_Right_Col_Written", pick(pins::SECTION_RIGHT_COL_WRITTEN)),
        ("Current_Act_Ptr", pick(pins::CURRENT_ACT_PTR)),
    ];
    let mut out = Vec::new();
    for (i, (name, vma)) in table.iter().enumerate() {
        let asm = format!("cpu 68000\n\tphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
        let mut secs = assemble(&asm, &opts)
            .unwrap_or_else(|d| panic!("AS assemble ({name}): {d:?}"))
            .sections;
        for mut s in secs.drain(..) {
            s.lma = 0x0200_0000 + (i as u32) * 0x1_0000;
            s.placement = SectionPlacement::Pinned;
            s.group = None;
            out.push(s);
        }
    }
    out
}

/// Parse a .emp file, panicking on parse errors.
fn parse_file(path: &Path) -> sigil_frontend_emp::ast::File {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} parse errors: {pdiags:?}",
        path.display()
    );
    file
}

/// Lower the real `plane_buffer.emp` (prepend the `engine.constants` twin +
/// `engine.structs`), place into the per-shape map, append the value equs +
/// cross-seam address labels, one `resolve_layout` -> `link`.
fn compile_real_file(
    debug: bool,
    doctor: Option<(&str, &str)>,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let dir = level_dir();
    let main = parse_file(&dir.join("plane_buffer.emp"));
    // plane_buffer.emp `use`s engine.constants.{TILE_CACHE_*} + engine.structs.{Act, Sec}
    // — prepend the engine.constants twin and the shared struct module.
    let constants_file = parse_file(&dir.parent().unwrap().join("system/constants.emp"));
    let structs_file = parse_file(&dir.parent().unwrap().join("structs.emp"));
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items: constants_file
            .items
            .into_iter()
            .chain(structs_file.items)
            .chain(main.items)
            .collect(),
        docs: main.docs.clone(),
    };

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), i128::from(debug))],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "plane_buffer.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut equs = plane_buffer_value_equs(doctor);
    for sec in &mut equs {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(equs);

    sections.extend(plane_buffer_addr_labels(debug));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

/// plane_buffer.emp's drift guards + the prepended twins' guards must PASS.
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "plane_buffer.emp drift guards must all PASS: {diags:?}"
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

fn run(debug: bool) {
    let aeon = aeon_dir();
    let rom_name = if debug { "s4.debug.bin" } else { "s4.bin" };
    let rom_path = aeon.join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (resolved, linked, link_asserts) = compile_real_file(debug, None);
    assert_drift_guards(&resolved, &link_asserts);

    let base = region_base(debug) as usize;
    let expected = &refrom[base..base + region_len(debug)];
    let section = linked.section("plane_buffer").expect("linked image must carry plane_buffer");
    let shape = if debug { "debug" } else { "plain" };
    assert_region_matches(&section.bytes, expected, &format!("plane_buffer ({shape})"));
}

#[test]
fn plane_buffer_region_matches_reference() {
    run(false);
}

#[test]
fn plane_buffer_debug_region_matches_reference() {
    run(true);
}

/// Negative probe: a DOCTORED `PLANE_BUFFER_SIZE` truth (1024 AS-side while
/// plane_buffer.emp says 1536) must fire plane_buffer.emp's own `ensure(extern(…))`
/// guard NAMING the constant — the undoctored control passes through the same
/// plumbing (the reference gates above).
#[test]
fn doctored_plane_buffer_size_fires_its_guard() {
    let aeon = aeon_dir();
    if !aeon.join("engine/level/plane_buffer.emp").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    let (resolved, _, link_asserts) = compile_real_file(false, Some(("PLANE_BUFFER_SIZE", "1024")));
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    let fired: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(!fired.is_empty(), "the doctored PLANE_BUFFER_SIZE truth must fire a drift guard");
    assert!(
        fired.iter().any(|d| d.message.contains("PLANE_BUFFER_SIZE")),
        "the fired guard must NAME the drifted constant: {fired:?}"
    );
}

// ============================================================================
// Two-module ownership flip (tranche 17) — the campaign's 3rd flip. plane_buffer
// PORTS out from under section.emp, whose 4 `jbsr` sites into Draw_TileColumn /
// Draw_TileRow_FromCache now re-resolve to plane_buffer.emp's OWNED symbols.
// UNIDIRECTIONAL (section -> plane_buffer; plane_buffer is a leaf). Both regions
// are compiled, placed at their per-shape regions, and linked in ONE
// resolve_layout + link over the union; section's label list DROPS the two
// Draw_* labels (no synthetic stand-in), so section's `jbsr->bsr.w` bytes match
// the reference ONLY when the disp lands on plane_buffer.emp's real symbol VMA
// ($4066/$4188 plain, $4C30/$4D52 debug) — the flip, proven per shape.
// ============================================================================

/// Lower one .emp (ambient deps prepended), place into a single-region map.
fn lower_and_place(
    emp_path: &Path,
    ambient: Vec<sigil_frontend_emp::ast::File>,
    include_root: PathBuf,
    region: &str,
    base: u32,
    len: usize,
    debug: bool,
) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>) {
    let main = parse_file(emp_path);
    let mut items = Vec::new();
    for d in ambient {
        items.extend(d.items);
    }
    items.extend(main.items.clone());
    let file = sigil_frontend_emp::ast::File {
        module: main.module,
        attrs: main.attrs,
        items,
        docs: main.docs,
    };
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(include_root),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), i128::from(debug))],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} lower errors: {ldiags:?}",
        emp_path.display()
    );
    let mt = format!(
        "fill = 0x00\n\n[[region]]\nname = \"text\"\nlma_base = 0x0000\nsize = 0x10\nkind = \"rom\"\n\n[[region]]\nname = \"{region}\"\nlma_base = {base:#x}\nsize = {len:#x}\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&mt).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} place errors: {pdiags:?}",
        emp_path.display()
    );
    (sections, module.link_asserts)
}

/// section.emp's OWN mirrored value pairs (mirror of section_port::section_value_equs
/// minus the shared twins). Unioned with plane_buffer's + the shared twins.
fn section_value_pairs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("VDP_DATA", "$C00000"),
        ("VDP_CTRL", "$C00004"),
        ("VRAM_PLANE_A", "$C000"),
        ("VRAM_PLANE_B_BYTES", "$E000"),
        ("PLANE_H_CELLS", "64"),
        ("PLANE_V_CELLS", "64"),
        ("PLANE_BUFFER_SIZE", "1536"),
        ("VRAM", "%100001"),
        ("CRAM", "%101011"),
        ("VSRAM", "%100101"),
        ("READ", "%001100"),
        ("WRITE", "%000111"),
        ("DMA", "%100111"),
    ]
}

/// plane_buffer.emp's own value pairs (the non-shared mirrors).
fn plane_buffer_value_pairs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("VRAM_PLANE_A", "$C000"),
        ("VRAM_PLANE_B_BYTES", "$E000"),
        ("VDP_DATA", "$C00000"),
        ("VDP_CTRL", "$C00004"),
        ("PLANE_H_CELLS", "64"),
        ("PLANE_V_CELLS", "64"),
        ("PLANE_BUFFER_SIZE", "1536"),
    ]
}

/// section.emp's cross-seam ADDRESS labels MINUS the two Draw_* labels (now owned
/// by plane_buffer.emp), PLUS plane_buffer's `Plane_Buffer` base. Unioned by name.
fn flip_labels(debug: bool) -> Vec<(&'static str, u32)> {
    let pick = |p: pins::Pin| -> u32 { if debug { p.debug } else { p.plain } };
    // section's list, DROPPING Draw_TileColumn / Draw_TileRow_FromCache.
    let mut v: Vec<(&'static str, u32)> = vec![
        ("EntityWindow_Init", pick(pins::ENTITY_WINDOW_INIT)),
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
    ];
    // plane_buffer's additional base label (its RAM buffer base).
    v.push(("Plane_Buffer", pick(pins::PLANE_BUFFER_BASE)));
    v
}

fn two_module_flip(debug: bool, rom_name: &str) {
    let aeon = aeon_dir();
    let Ok(refrom) = std::fs::read(aeon.join(rom_name)) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {rom_name}");
        }
        eprintln!("skip: reference ROM {rom_name} missing");
        return;
    };

    let pb_base = region_base(debug);
    let (mut pb_sections, pb_asserts) = lower_and_place(
        &aeon.join("engine/level/plane_buffer.emp"),
        vec![
            parse_file(&aeon.join("engine/structs.emp")),
            parse_file(&aeon.join("engine/system/constants.emp")),
        ],
        aeon.join("engine/level"),
        "plane_buffer",
        pb_base,
        region_len(debug),
        debug,
    );

    let sec_base = if debug { pins::SECTION.debug_base } else { pins::SECTION.plain_base };
    let (mut sec_sections, sec_asserts) = lower_and_place(
        &aeon.join("engine/level/section.emp"),
        vec![
            parse_file(&aeon.join("engine/structs.emp")),
            parse_file(&aeon.join("engine/system/constants.emp")),
        ],
        aeon.join("engine/level"),
        "section",
        sec_base,
        pins::SECTION.plain_len,
        debug,
    );

    // Union the value seam (dedup, assert consistent).
    let mut vmap: BTreeMap<&str, &str> = BTreeMap::new();
    let cl_twin: Vec<(&str, &str)> = sigil_harness::test_support::engine_constant_equs();
    let act_sec: Vec<(&str, &str)> = sigil_harness::test_support::act_sec_field_equs();
    for (n, v) in plane_buffer_value_pairs()
        .into_iter()
        .chain(section_value_pairs())
        .chain(cl_twin)
        .chain(act_sec)
    {
        if let Some(prev) = vmap.insert(n, v) {
            assert_eq!(prev, v, "seam value conflict for `{n}`");
        }
    }
    let vpairs: Vec<(&str, &str)> = vmap.into_iter().collect();

    // Union the address labels (section's set minus Draw_*, plus Plane_Buffer).
    let mut lmap: BTreeMap<&str, u32> = BTreeMap::new();
    for (n, v) in flip_labels(debug) {
        if let Some(prev) = lmap.insert(n, v) {
            assert_eq!(prev, v, "label VMA conflict for `{n}`: {prev:#x} vs {v:#x}");
        }
    }

    let mut sections = Vec::new();
    sections.append(&mut pb_sections);
    sections.append(&mut sec_sections);
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

    let mut all = pb_asserts;
    all.extend(sec_asserts);
    let adiags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &all);
    assert!(adiags.iter().all(|d| d.level != sigil_span::Level::Error), "drift guards: {adiags:?}");

    let pr = &refrom[pb_base as usize..pb_base as usize + region_len(debug)];
    assert_region_matches(
        &linked.section("plane_buffer").expect("plane_buffer region").bytes,
        pr,
        "plane_buffer (two-module flip)",
    );
    let sr = &refrom[sec_base as usize..sec_base as usize + pins::SECTION.plain_len];
    assert_region_matches(
        &linked.section("section").expect("section region").bytes,
        sr,
        "section (two-module flip)",
    );
}

#[test]
fn two_module_ownership_flip_plain() {
    two_module_flip(false, "s4.bin");
}

#[test]
fn two_module_ownership_flip_debug() {
    two_module_flip(true, "s4.debug.bin");
}
