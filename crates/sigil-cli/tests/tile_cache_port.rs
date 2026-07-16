//! Tranche 16 — the REAL `tile_cache.emp` port, region-level byte gate.
//!
//! Compiles the actual ported file — `engine/level/tile_cache.emp` — through the
//! production parse -> lower -> place -> resolve -> link pipeline and asserts the
//! `tile_cache` region's flattened bytes equal the reference ROM window at the
//! pinned base, in BOTH build shapes. The BIGGEST region ported to date
//! ($916 plain / $9D6 debug), section.emp's paired streaming sibling, and the
//! measured vertical-streaming lag driver.
//!
//! ## Shape
//! SHAPE-VARYING: the debug shape carries +$B8 over plain — the `.raw_direct`
//! `assert.l`/`assert.w` block (`if DEBUG == 1 {}`, mirroring tile_cache.asm's
//! `ifdebug`), so the DEBUG define drives the divergence and the two
//! `MDDBG__ErrorHandler*` targets the assert expansions jump to are supplied
//! ONLY in the debug shape.
//!
//! ## Cross-seam symbols (synthetic AS-side sections, per-shape VMAs)
//! - ROM transfer target: `S4LZ_DecompressDict` (PC-rel `bsr.w`,
//!   engine/compression/s4lz_decompress.asm — NOT ported; extern).
//! - RAM labels (abs operands + the `Block_Stage_Buffers` data-table fixup):
//!   the `Cache_*`, `Cache_Fill_*`, `Block_Stage_*`, `Tile_Cache_Nametable`/
//!   `Collision`, `Camera_*`, `Current_Act_Ptr`, `Frame_Counter`,
//!   `Section_Plane_Dirty` addresses.
//! - DEBUG-only: `MDDBG__ErrorHandler` + `_PagesController` (assert expansions).
//! - `tile_cache.emp` has NO `use` — every const is a file-local drift-locked
//!   mirror, so `tile_cache_value_equs` supplies all 27 AS-side equ values its
//!   `ensure`s read back; no engine.constants twin prepend.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test tile_cache_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

fn region_base(debug: bool) -> u32 {
    if debug { pins::TILE_CACHE.debug_base } else { pins::TILE_CACHE.plain_base }
}

fn region_len(debug: bool) -> usize {
    if debug { pins::TILE_CACHE.debug_len } else { pins::TILE_CACHE.plain_len }
}

fn tile_cache_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    Path::new(&aeon).join("engine/level")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The map: a `text` carrier for the zero-byte default section, and the
/// `tile_cache` region pinned at the per-shape reference base + per-shape length
/// (SHAPE-VARYING: plain $916, debug $9D6).
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
         name = \"tile_cache\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// tile_cache.emp's OWN mirrored constants (truth: engine/constants.asm,
/// structs.asm) — the values its 27 drift-guard `ensure`s read back through
/// `extern()`. No engine.constants twin (tile_cache.emp has no `use`).
fn tile_cache_value_equs() -> Vec<Section> {
    let mut pairs: Vec<(&str, &str)> = vec![
        ("TILE_CACHE_COLS", "80"),
        ("TILE_CACHE_ROWS", "60"),
        ("TILE_CACHE_STRIDE", "80"),
        ("TILE_CACHE_NT_SIZE", "9600"),
        ("TILE_CACHE_COLL_ROWS", "30"),
        ("TILE_CACHE_COLL_SIZE", "2400"),
        ("TILE_CACHE_COLL_PLANES", "2"),
        ("TILE_CACHE_MARGIN_H", "20"),
        ("TILE_CACHE_MARGIN_V", "16"),
        ("BLOCK_TILE_SIZE", "16"),
        ("BLOCK_TILE_SHIFT", "4"),
        ("BLOCK_NT_SIZE", "512"),
        ("BLOCK_COLL_ROWS", "8"),
        ("BLOCK_COLL_COLS", "16"),
        ("BLOCK_COLL_PLANE_SIZE", "128"),
        ("BLOCK_COLL_SIZE", "256"),
        ("BLOCK_RAW_SIZE", "768"),
        ("BLOCK_STAGE_SLOTS", "16"),
        ("BLOCK_DECOMP_BUDGET", "6"),
        ("BLOCK_INDEX_SIZE", "1024"),
        ("VFILL_ROWS_PER_FRAME", "2"),
        ("H_PFX_HYST", "16"),
    ];
    // Act_*/Sec_* field equs + Act_len/Sec_len feed the prepended engine.structs
    // drift wall (tile_cache no longer mirrors Sec/Act offsets).
    pairs.extend(sigil_harness::test_support::act_sec_field_equs());
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// The cross-seam ADDRESS symbols — RAM labels + the `S4LZ_DecompressDict` ROM
/// transfer target — each a `phase`d one-byte carrier at its true per-shape VMA
/// (label position is load-bearing: abs.w/abs.l width selection, the PC-rel
/// `bsr.w` disp, and the `Block_Stage_Buffers` data-table fixup all read it).
/// The two `MDDBG__ErrorHandler*` targets ride ONLY in the debug shape (the
/// assert expansions that reference them are elided in plain).
fn tile_cache_addr_labels(debug: bool) -> Vec<Section> {
    let pick = |p: pins::Pin| -> u32 { if debug { p.debug } else { p.plain } };
    let mut table: Vec<(&str, u32)> = vec![
        ("Cache_Left_Col", pick(pins::CACHE_LEFT_COL)),
        ("Cache_Head_Col", pick(pins::CACHE_HEAD_COL)),
        ("Cache_Top_Row", pick(pins::CACHE_TOP_ROW)),
        ("Cache_Bottom_Row", pick(pins::CACHE_BOTTOM_ROW)),
        ("Cache_Origin_Col", pick(pins::CACHE_ORIGIN_COL)),
        ("Cache_Origin_Row", pick(pins::CACHE_ORIGIN_ROW)),
        ("Cache_Fill_Last_Frame", pick(pins::CACHE_FILL_LAST_FRAME)),
        ("Cache_Fill_Resume_Col", pick(pins::CACHE_FILL_RESUME_COL)),
        ("Cache_Fill_Resume_Row", pick(pins::CACHE_FILL_RESUME_ROW)),
        ("Cache_Fill_RowResume_Row", pick(pins::CACHE_FILL_ROW_RESUME_ROW)),
        ("Cache_Fill_RowResume_Col", pick(pins::CACHE_FILL_ROW_RESUME_COL)),
        ("Cache_Fill_Budget", pick(pins::CACHE_FILL_BUDGET)),
        ("Cache_Fill_Rows_Left", pick(pins::CACHE_FILL_ROWS_LEFT)),
        ("Cache_Prev_Cam_Row", pick(pins::CACHE_PREV_CAM_ROW)),
        ("Cache_Prev_Cam_X", pick(pins::CACHE_PREV_CAM_X)),
        ("Cache_H_Pfx_Dir", pick(pins::CACHE_H_PFX_DIR)),
        ("Cache_H_Pfx_Accum", pick(pins::CACHE_H_PFX_ACCUM)),
        ("Cache_Pfx_Row_Target", pick(pins::CACHE_PFX_ROW_TARGET)),
        ("Cache_Pfx_Col_Target", pick(pins::CACHE_PFX_COL_TARGET)),
        ("Cache_Pfx_Skip_Armed", pick(pins::CACHE_PFX_SKIP_ARMED)),
        ("Cache_Pfx_Lag_Flag", pick(pins::CACHE_PFX_LAG_FLAG)),
        ("Block_Stage_Keys", pick(pins::BLOCK_STAGE_KEYS)),
        ("Block_Stage_Next", pick(pins::BLOCK_STAGE_NEXT)),
        ("Block_Stage_Buffers", pick(pins::BLOCK_STAGE_BUFFERS)),
        ("Tile_Cache_Nametable", pick(pins::TILE_CACHE_NAMETABLE)),
        ("Tile_Cache_Collision", pick(pins::TILE_CACHE_COLLISION)),
        ("Camera_X", pick(pins::CAMERA_X)),
        ("Camera_Y", pick(pins::CAMERA_Y)),
        ("Current_Act_Ptr", pick(pins::CURRENT_ACT_PTR)),
        ("Frame_Counter", pick(pins::FRAME_COUNTER)),
        ("Section_Plane_Dirty", pick(pins::SECTION_PLANE_DIRTY)),
        ("S4LZ_DecompressDict", pick(pins::S4_LZ_DECOMPRESS_DICT)),
    ];
    if debug {
        table.push(("MDDBG__ErrorHandler", pins::MDDBG_ERROR_HANDLER));
        table.push(("MDDBG__ErrorHandler_PagesController", pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER));
    }
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

/// Lower the real `tile_cache.emp` (NO twin prepend), place into the per-shape
/// map, append the value equs + cross-seam address labels, one `resolve_layout`
/// -> `link`. The DEBUG define drives the shape-varying assert block.
fn compile_real_file(
    debug: bool,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let dir = tile_cache_dir();
    let src = std::fs::read_to_string(dir.join("tile_cache.emp"))
        .unwrap_or_else(|e| panic!("cannot read tile_cache.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "tile_cache.emp parse errors: {pdiags:?}"
    );
    // tile_cache.emp now `use`s engine.structs.{Act, Sec, Act_grid_w_lo, Act_grid_h_lo}
    // — prepend the shared struct module (Sec/Act layout + drift wall + grid-lo consts).
    let structs_path = dir.parent().unwrap().join("structs.emp");
    let structs_src = std::fs::read_to_string(&structs_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", structs_path.display()));
    let (structs_file, sdiags) = parse_str(&structs_src);
    assert!(
        sdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "structs.emp parse errors: {sdiags:?}"
    );
    let file = sigil_frontend_emp::ast::File {
        module: file.module.clone(),
        attrs: file.attrs.clone(),
        items: structs_file.items.into_iter().chain(file.items).collect(),
        docs: file.docs.clone(),
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
        "tile_cache.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut equs = tile_cache_value_equs();
    for sec in &mut equs {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(equs);

    sections.extend(tile_cache_addr_labels(debug));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

/// tile_cache.emp's 27 drift guards + the `Sec_len == 66` stride guard must be
/// captured and PASS against `tile_cache_value_equs`' values.
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "tile_cache.emp drift guards must all PASS: {diags:?}"
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
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    let rom_name = if debug { "s4.debug.bin" } else { "s4.bin" };
    let rom_path = Path::new(&aeon).join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (resolved, linked, link_asserts) = compile_real_file(debug);
    assert_drift_guards(&resolved, &link_asserts);

    let base = region_base(debug) as usize;
    let expected = &refrom[base..base + region_len(debug)];
    let section = linked.section("tile_cache").expect("linked image must carry tile_cache");
    let shape = if debug { "debug" } else { "plain" };
    assert_region_matches(&section.bytes, expected, &format!("tile_cache ({shape})"));
}

#[test]
fn tile_cache_region_matches_reference() {
    run(false);
}

#[test]
fn tile_cache_debug_region_matches_reference() {
    run(true);
}

// ============================================================================
// Two-module link test (tranche 16) — the campaign's FIRST TAIL-CALL flip,
// proven (not argued by construction). collision_lookup.emp and tile_cache.emp
// are compiled, placed at their per-shape regions, and linked in ONE
// resolve_layout + link over the union. UNIDIRECTIONAL flip:
//   collision_lookup.emp's `jbra Tile_Cache_GetCollision` (a tail-call BRANCH,
//   twin `bra.w`) → tile_cache.emp's owned `Tile_Cache_GetCollision` — no
//   synthetic label. Both regions byte-compare against the reference ROM; the
//   collision_lookup bytes match ONLY when the PC-rel disp lands on tile_cache's
//   real symbol VMA ($4336 plain / $4F00 debug) — the flip, proven per shape.
// (entity_window_port::two_module_ownership_flip_* is the bidirectional template.)
// ============================================================================
use std::collections::BTreeMap;

/// Parse a .emp file, panicking on parse errors.
fn parse_file(path: &Path) -> sigil_frontend_emp::ast::File {
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} parse errors: {pdiags:?}", path.display()
    );
    file
}

/// Prepend ambient dependency modules' items to `main` (the `use`-target seam).
fn with_ambient(
    ambient: Vec<sigil_frontend_emp::ast::File>,
    main: sigil_frontend_emp::ast::File,
) -> sigil_frontend_emp::ast::File {
    let mut items = Vec::new();
    for d in ambient {
        items.extend(d.items);
    }
    items.extend(main.items.clone());
    sigil_frontend_emp::ast::File { module: main.module, attrs: main.attrs, items, docs: main.docs }
}

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
    let file = with_ambient(ambient, parse_file(emp_path));
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(include_root),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), i128::from(debug))],
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

/// tile_cache.emp's value seam as name/value pairs (for the union dedup).
fn tile_cache_value_pairs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("TILE_CACHE_COLS", "80"), ("TILE_CACHE_ROWS", "60"), ("TILE_CACHE_STRIDE", "80"),
        ("TILE_CACHE_NT_SIZE", "9600"), ("TILE_CACHE_COLL_ROWS", "30"),
        ("TILE_CACHE_COLL_SIZE", "2400"), ("TILE_CACHE_COLL_PLANES", "2"),
        ("TILE_CACHE_MARGIN_H", "20"), ("TILE_CACHE_MARGIN_V", "16"),
        ("BLOCK_TILE_SIZE", "16"), ("BLOCK_TILE_SHIFT", "4"), ("BLOCK_NT_SIZE", "512"),
        ("BLOCK_COLL_ROWS", "8"), ("BLOCK_COLL_COLS", "16"), ("BLOCK_COLL_PLANE_SIZE", "128"),
        ("BLOCK_COLL_SIZE", "256"), ("BLOCK_RAW_SIZE", "768"), ("BLOCK_STAGE_SLOTS", "16"),
        ("BLOCK_DECOMP_BUDGET", "6"), ("BLOCK_INDEX_SIZE", "1024"), ("VFILL_ROWS_PER_FRAME", "2"),
        ("H_PFX_HYST", "16"),
        // Act_*/Sec_* now come from act_sec_field_equs() in the flip union.
    ]
}

/// tile_cache.emp's cross-seam ADDRESS labels for the two-module link — the byte
/// gate's list MINUS `Tile_Cache_GetCollision` (owned by tile_cache.emp here).
/// (tile_cache's list never synthesized it — it is internal — so this is just
/// the same 24 base labels + the 2 debug MDDBG handlers.)
fn tile_cache_labels_for_link(debug: bool) -> Vec<(&'static str, u32)> {
    let pick = |p: pins::Pin| -> u32 { if debug { p.debug } else { p.plain } };
    let mut v: Vec<(&'static str, u32)> = vec![
        ("Cache_Left_Col", pick(pins::CACHE_LEFT_COL)),
        ("Cache_Head_Col", pick(pins::CACHE_HEAD_COL)),
        ("Cache_Top_Row", pick(pins::CACHE_TOP_ROW)),
        ("Cache_Bottom_Row", pick(pins::CACHE_BOTTOM_ROW)),
        ("Cache_Origin_Col", pick(pins::CACHE_ORIGIN_COL)),
        ("Cache_Origin_Row", pick(pins::CACHE_ORIGIN_ROW)),
        ("Cache_Fill_Last_Frame", pick(pins::CACHE_FILL_LAST_FRAME)),
        ("Cache_Fill_Resume_Col", pick(pins::CACHE_FILL_RESUME_COL)),
        ("Cache_Fill_Resume_Row", pick(pins::CACHE_FILL_RESUME_ROW)),
        ("Cache_Fill_RowResume_Row", pick(pins::CACHE_FILL_ROW_RESUME_ROW)),
        ("Cache_Fill_RowResume_Col", pick(pins::CACHE_FILL_ROW_RESUME_COL)),
        ("Cache_Fill_Budget", pick(pins::CACHE_FILL_BUDGET)),
        ("Cache_Fill_Rows_Left", pick(pins::CACHE_FILL_ROWS_LEFT)),
        ("Cache_Prev_Cam_Row", pick(pins::CACHE_PREV_CAM_ROW)),
        ("Cache_Prev_Cam_X", pick(pins::CACHE_PREV_CAM_X)),
        ("Cache_H_Pfx_Dir", pick(pins::CACHE_H_PFX_DIR)),
        ("Cache_H_Pfx_Accum", pick(pins::CACHE_H_PFX_ACCUM)),
        ("Cache_Pfx_Row_Target", pick(pins::CACHE_PFX_ROW_TARGET)),
        ("Cache_Pfx_Col_Target", pick(pins::CACHE_PFX_COL_TARGET)),
        ("Cache_Pfx_Skip_Armed", pick(pins::CACHE_PFX_SKIP_ARMED)),
        ("Cache_Pfx_Lag_Flag", pick(pins::CACHE_PFX_LAG_FLAG)),
        ("Block_Stage_Keys", pick(pins::BLOCK_STAGE_KEYS)),
        ("Block_Stage_Next", pick(pins::BLOCK_STAGE_NEXT)),
        ("Block_Stage_Buffers", pick(pins::BLOCK_STAGE_BUFFERS)),
        ("Tile_Cache_Nametable", pick(pins::TILE_CACHE_NAMETABLE)),
        ("Tile_Cache_Collision", pick(pins::TILE_CACHE_COLLISION)),
        ("Camera_X", pick(pins::CAMERA_X)),
        ("Camera_Y", pick(pins::CAMERA_Y)),
        ("Current_Act_Ptr", pick(pins::CURRENT_ACT_PTR)),
        ("Frame_Counter", pick(pins::FRAME_COUNTER)),
        ("Section_Plane_Dirty", pick(pins::SECTION_PLANE_DIRTY)),
        ("S4LZ_DecompressDict", pick(pins::S4_LZ_DECOMPRESS_DICT)),
    ];
    if debug {
        v.push(("MDDBG__ErrorHandler", pins::MDDBG_ERROR_HANDLER));
        v.push(("MDDBG__ErrorHandler_PagesController", pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER));
    }
    v
}

fn two_module_flip(debug: bool, rom_name: &str) {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    let aeon = PathBuf::from(aeon);
    let Ok(refrom) = std::fs::read(aeon.join(rom_name)) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {rom_name}");
        }
        eprintln!("skip: reference ROM {rom_name} missing");
        return;
    };

    let tc_base = region_base(debug);
    let (mut tc_sections, tc_asserts) = lower_and_place(
        &aeon.join("engine/level/tile_cache.emp"),
        vec![parse_file(&aeon.join("engine/structs.emp"))],
        aeon.join("engine/level"),
        "tile_cache",
        tc_base,
        region_len(debug),
        debug,
    );

    let cl_base = if debug { pins::COLLISION_LOOKUP.debug_base } else { pins::COLLISION_LOOKUP.plain_base };
    let (mut cl_sections, cl_asserts) = lower_and_place(
        &aeon.join("engine/level/collision_lookup.emp"),
        vec![parse_file(&aeon.join("engine/system/constants.emp"))],
        aeon.join("engine/level"),
        "collision_lookup",
        cl_base,
        pins::COLLISION_LOOKUP.plain_len,
        debug,
    );

    // Union the value seam (dedup, assert consistent).
    let mut vmap: BTreeMap<&str, &str> = BTreeMap::new();
    let cl_twin: Vec<(&str, &str)> = sigil_harness::test_support::engine_constant_equs();
    let act_sec: Vec<(&str, &str)> = sigil_harness::test_support::act_sec_field_equs();
    for (n, v) in tile_cache_value_pairs().into_iter().chain(cl_twin).chain(act_sec) {
        if let Some(prev) = vmap.insert(n, v) {
            assert_eq!(prev, v, "seam value conflict for `{n}`");
        }
    }
    let vpairs: Vec<(&str, &str)> = vmap.into_iter().collect();

    // Union the address labels. collision_lookup's Cache_* are a subset of
    // tile_cache's; Tile_Cache_GetCollision is DROPPED (now owned by tile_cache).
    let mut lmap: BTreeMap<&str, u32> = BTreeMap::new();
    for (n, v) in tile_cache_labels_for_link(debug) {
        if let Some(prev) = lmap.insert(n, v) {
            assert_eq!(prev, v, "label VMA conflict for `{n}`: {prev:#x} vs {v:#x}");
        }
    }

    let mut sections = Vec::new();
    sections.append(&mut tc_sections);
    sections.append(&mut cl_sections);
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

    let mut all = tc_asserts;
    all.extend(cl_asserts);
    let adiags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &all);
    assert!(adiags.iter().all(|d| d.level != sigil_span::Level::Error), "drift guards: {adiags:?}");

    let tr = &refrom[tc_base as usize..tc_base as usize + region_len(debug)];
    assert_region_matches(&linked.section("tile_cache").expect("tile_cache region").bytes, tr, "tile_cache (two-module flip)");
    let cr = &refrom[cl_base as usize..cl_base as usize + pins::COLLISION_LOOKUP.plain_len];
    assert_region_matches(&linked.section("collision_lookup").expect("collision_lookup region").bytes, cr, "collision_lookup (two-module flip)");
}

#[test]
fn two_module_tail_call_flip_plain() {
    two_module_flip(false, "s4.bin");
}

#[test]
fn two_module_tail_call_flip_debug() {
    two_module_flip(true, "s4.debug.bin");
}
