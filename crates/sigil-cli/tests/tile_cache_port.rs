//! Tranche 16 — the REAL `tile_cache.emp` port, region-level byte gate.
//!
//! Compiles the actual ported file — `engine/level/tile_cache.emp` — through the
//! production parse -> lower -> place -> resolve -> link pipeline and asserts the
//! `tile_cache` region's flattened bytes equal the reference ROM window at the
//! pinned base, in BOTH build shapes. The BIGGEST region ported to date
//! ($924 plain / $9DC debug), section.emp's paired streaming sibling, and the
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
/// (SHAPE-VARYING: plain $924, debug $9DC).
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
    let pairs: Vec<(&str, &str)> = vec![
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
        ("BLOCK_STAGE_SLOTS", "12"),
        ("BLOCK_DECOMP_BUDGET", "6"),
        ("BLOCK_INDEX_SIZE", "1024"),
        ("VFILL_ROWS_PER_FRAME", "2"),
        ("Act_sec_grid_ptr", "$00"),
        ("Act_grid_w", "$04"),
        ("Act_grid_h", "$06"),
        ("Sec_sec_block_index", "$00"),
        ("Sec_sec_block_dict", "$2C"),
        ("Sec_sec_block_dict_len", "$40"),
        ("Sec_len", "$42"),
    ];
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
