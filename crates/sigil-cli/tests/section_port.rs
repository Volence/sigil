//! Tranche 15 — the REAL `section.emp` port, region-level byte gate.
//!
//! Compiles the actual ported file — `engine/level/section.emp` — through the
//! production parse -> lower -> place -> resolve -> link pipeline and asserts
//! the `section` region's flattened bytes equal the reference ROM window at the
//! pinned base, in BOTH build shapes. The biggest region ported to date and the
//! first exercising the two-pinned-abs-operand (mem-to-mem) lowering feature +
//! the typed VDP command interface.
//!
//! ## Shape
//! SHAPE-INVARIANT length ($3EA both shapes — no `__DEBUG__` code in
//! section.asm), like sprites/load_object; only the base shifts (plain `$513E`,
//! debug `$5DC0`), so the shape lives entirely in the MAP + the per-shape
//! synthetic label VMAs.
//!
//! ## Cross-seam symbols (synthetic AS-side sections, per-shape VMAs)
//! - ROM transfer targets: `Draw_TileColumn`/`Draw_TileRow_FromCache`
//!   (PC-rel `bsr.w`, plane_buffer.asm), `EntityWindow_Init` (`jsr`).
//!   `Section_GetSecPtrXY`/`FlatIDXY` are now INTERNAL (.emp-owned) — the R5
//!   flip from their inbound-extern status in entity_window_port.
//! - RAM labels (abs operands + the two mem-to-mem fixups): the `Cache_*`,
//!   `Section_*_Written`, `Camera_*`, `Current_Act_Ptr`, `Section_Plane_Dirty`,
//!   `Plane_Buffer_Ptr`, `Tile_Cache_Nametable` addresses.
//! - `engine.constants` twin (`use …SECTION_SIZE_SHIFT`) rides via the ambient
//!   prepend; its drift guards ride this gate. section.emp's OWN mirrors
//!   (VDP/plane/tile-cache consts, Sec/Act offsets, the six VDP type equs the
//!   mapper drift-locks read) are supplied by `section_value_equs`.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test section_port
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
    if debug { pins::SECTION.debug_base } else { pins::SECTION.plain_base }
}

fn section_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    Path::new(&aeon).join("engine/level")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The map: a `text` carrier for the zero-byte default section, and the `section`
/// region pinned at the per-shape reference base, sized to $3EA (both shapes).
fn map_toml(debug: bool) -> String {
    let base = region_base(debug);
    let len = pins::SECTION.plain_len;
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
         name = \"section\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// section.emp's OWN mirrored constants (truth: engine/constants.asm, bg.asm,
/// structs.asm) PLUS the engine.constants twin's values (its drift guards ride
/// via the ambient prepend) — one blob, so a single `Stub` carrier. The six VDP
/// type equs feed the `target_bits`/`op_bits` mapper drift-locks.
fn section_value_equs() -> Vec<Section> {
    let mut pairs: Vec<(&str, &str)> = vec![
        ("VDP_DATA", "$C00000"),
        ("VDP_CTRL", "$C00004"),
        ("VRAM_PLANE_A", "$C000"),
        ("VRAM_PLANE_B_BYTES", "$E000"),
        ("PLANE_H_CELLS", "64"),
        ("PLANE_V_CELLS", "64"),
        ("PLANE_BUFFER_SIZE", "1536"),
        // TILE_CACHE_COLS/ROWS/STRIDE/NT_SIZE now come from engine_constant_equs()
        // (hoisted into the shared engine.constants twin).
        ("VRAM", "%100001"),
        ("CRAM", "%101011"),
        ("VSRAM", "%100101"),
        ("READ", "%001100"),
        ("WRITE", "%000111"),
        ("DMA", "%100111"),
    ];
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    // The Act_*/Sec_* field equs + Act_len/Sec_len feed the prepended
    // engine.structs drift wall (section no longer mirrors Sec/Act offsets).
    pairs.extend(sigil_harness::test_support::act_sec_field_equs());
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// The 19 cross-seam ADDRESS symbols — RAM labels + ROM transfer targets — each
/// a `phase`d one-byte carrier at its true per-shape VMA (label position is
/// load-bearing: abs.w/abs.l width selection and PC-rel disp both read it).
fn section_addr_labels(debug: bool) -> Vec<Section> {
    let pick = |p: pins::Pin| -> u32 { if debug { p.debug } else { p.plain } };
    let table: [(&str, u32); 19] = [
        ("Draw_TileColumn", pick(pins::DRAW_TILE_COLUMN)),
        ("Draw_TileRow_FromCache", pick(pins::DRAW_TILE_ROW_FROM_CACHE)),
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

/// Lower the real `section.emp` (prepend the `engine.constants` twin), place
/// into the per-shape map, append the value equs + twin equs + cross-seam
/// address labels, one `resolve_layout` -> `link`.
fn compile_real_file(
    debug: bool,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let dir = section_dir();
    let src = std::fs::read_to_string(dir.join("section.emp"))
        .unwrap_or_else(|e| panic!("cannot read section.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "section.emp parse errors: {pdiags:?}"
    );

    let constants_path = dir.parent().unwrap().join("system/constants.emp");
    let constants_src = std::fs::read_to_string(&constants_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", constants_path.display()));
    let (constants_file, cdiags) = parse_str(&constants_src);
    assert!(
        cdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "constants.emp parse errors: {cdiags:?}"
    );
    // section.emp also `use`s engine.structs.{Act, Sec, Act_grid_w_lo, Act_grid_h_lo}
    // — prepend the shared struct module (the Sec/Act layout + drift wall + the
    // grid-dim low-byte consts).
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
        items: constants_file
            .items
            .into_iter()
            .chain(structs_file.items)
            .chain(file.items)
            .collect(),
        docs: file.docs.clone(),
    };

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "section.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut equs = section_value_equs();
    for sec in &mut equs {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(equs);

    sections.extend(section_addr_labels(debug));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

/// The engine.constants twin's drift guards must be captured and PASS.
fn assert_twin_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "section.emp + engine.constants drift guards must all PASS: {diags:?}"
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
    assert_twin_guards(&resolved, &link_asserts);

    let base = region_base(debug) as usize;
    let expected = &refrom[base..base + pins::SECTION.plain_len];
    let section = linked.section("section").expect("linked image must carry section");
    let shape = if debug { "debug" } else { "plain" };
    assert_region_matches(&section.bytes, expected, &format!("section ({shape})"));
}

#[test]
fn section_region_matches_reference() {
    run(false);
}

#[test]
fn section_debug_region_matches_reference() {
    run(true);
}
