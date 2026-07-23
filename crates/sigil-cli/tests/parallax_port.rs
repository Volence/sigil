//! Tranche 18 — the REAL `parallax.emp` port, region-level byte gate.
//!
//! Compiles the actual ported file — `engine/level/parallax.emp` — through the
//! production parse -> lower -> place -> resolve -> link pipeline and asserts the
//! `parallax` region's flattened bytes equal the reference ROM window at the
//! pinned base, in BOTH build shapes. The §4.6 parallax pipeline: per-frame band
//! lerp + HScroll buffer fill (per-cell / per-line deform) + whole-plane /
//! per-column Vscroll.
//!
//! ## Shape
//! SHAPE-INVARIANT length ($556 both shapes — parallax.asm has NO `__DEBUG__`
//! code, no asserts), like section/plane_buffer; only the base shifts (plain
//! `$5B02`, debug `$678C`).
//!
//! ## Cross-seam symbols
//! - RAM labels (abs.w operands): the `Parallax_*` state block, `Camera_X/Y`,
//!   `Current_Act_Ptr`, `Vscroll_Factor`, `Hscroll_*`, `VDP_Shadow_Table`,
//!   `VDP_Dirty_Mask` — each a `phase`d one-byte carrier at its true per-shape VMA.
//! - ROM transfer target: `Section_GetSecPtrXY` (the one cross-module `jsr`; a NEW
//!   caller of section.emp's already-owned symbol — a standard cross-module link
//!   resolution, NOT an ownership flip).
//! - `engine.constants`/`engine.structs`/`engine.vdp` twins ride the ambient
//!   prepend; their drift guards ride this gate.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test parallax_port
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
    if debug { pins::PARALLAX.debug_base } else { pins::PARALLAX.plain_base }
}

fn region_len(debug: bool) -> usize {
    if debug { pins::PARALLAX.debug_len } else { pins::PARALLAX.plain_len }
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
/// `parallax` region pinned at the per-shape reference base + length.
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
         name = \"parallax\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// parallax.emp's OWN mirrored constants + the struct-field/size externs its drift
/// wall reads back through `extern()`. `doctor` overrides ONE pair (the negative
/// probe). SCREEN_*/SECTION_SIZE_SHIFT + Act_*/Sec_* + VDP target/op bits ride the
/// prepended twins via `engine_constant_equs` / `act_sec_field_equs`.
fn parallax_value_equs(doctor: Option<(&str, &str)>) -> Vec<Section> {
    let mut pairs: Vec<(&str, &str)> = vec![
        // parallax.emp local mirrors
        ("PARALLAX_TRANS_DEFAULT", "16"),
        ("PARALLAX_LERP_SHIFT", "4"),
        ("VDP_DATA", "$C00000"),
        ("VDP_CTRL", "$C00004"),
        ("VDP_Shadow_vdp_mode3", "$0B"),
        // engine.vdp target_bits/op_bits drift-lock ensures read these six
        ("VRAM", "%100001"),
        ("CRAM", "%101011"),
        ("VSRAM", "%100101"),
        ("READ", "%001100"),
        ("WRITE", "%000111"),
        ("DMA", "%100111"),
        // band_entry struct (10 bytes) — the .emp's per-field drift wall
        ("band_entry_len", "10"),
        ("band_entry_band_top_cell", "0"),
        ("band_entry_band_factor_a_s1", "1"),
        ("band_entry_band_factor_a_s2", "2"),
        ("band_entry_band_factor_a_op", "3"),
        ("band_entry_band_factor_b_s1", "4"),
        ("band_entry_band_factor_b_s2", "5"),
        ("band_entry_band_factor_b_op", "6"),
        ("band_entry_band_deform_shift_a", "7"),
        ("band_entry_band_deform_shift_b", "8"),
        ("band_entry_band_phase_offset", "9"),
        // parallax_config struct (28 bytes / $1C header)
        ("parallax_config_len", "$1C"),
        ("parallax_config_pcfg_band_count", "$00"),
        ("parallax_config_pcfg_v_factor_bg", "$01"),
        ("parallax_config_pcfg_layer_mask", "$03"),
        ("parallax_config_pcfg_v_center_y", "$04"),
        ("parallax_config_pcfg_v_offset", "$06"),
        ("parallax_config_pcfg_transition", "$08"),
        ("parallax_config_pcfg_deform_speed_fg", "$09"),
        ("parallax_config_pcfg_deform_speed_bg", "$0A"),
        ("parallax_config_pcfg_deform_table_fg", "$0C"),
        ("parallax_config_pcfg_deform_table_bg", "$10"),
        ("parallax_config_pcfg_v_deform_table_bg", "$14"),
        ("parallax_config_pcfg_v_deform_speed_bg", "$18"),
        ("parallax_config_pcfg_v_deform_shift_bg", "$19"),
    ];
    if let Some((name, val)) = doctor {
        for p in pairs.iter_mut() {
            if p.0 == name {
                p.1 = val;
            }
        }
    }
    // shared-twin values: engine.constants (incl. SCREEN_*/SECTION_SIZE_SHIFT) feed
    // the prepended constants.emp drift wall; Act_*/Sec_* feed structs.emp's wall.
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    pairs.extend(sigil_harness::test_support::act_sec_field_equs());
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// The cross-seam ADDRESS symbols — the `Parallax_*` RAM state block + shared RAM
/// (`Camera_*`, `Current_Act_Ptr`, `Vscroll_Factor`, `Hscroll_*`, `VDP_*`) plus the
/// one ROM transfer target `Section_GetSecPtrXY` — each a `phase`d one-byte carrier
/// at its true per-shape VMA (label position selects abs.w/abs.l width and the
/// low-word bytes; `Section_GetSecPtrXY`'s VMA fixes the `jsr` disp).
fn parallax_addr_labels(debug: bool) -> Vec<Section> {
    // (name, plain VMA, debug VMA) — RAM mostly shape-invariant; Camera_*/
    // Current_Act_Ptr live in a debug-shifted RAM region; Section_GetSecPtrXY is ROM.
    let table: [(&str, u32, u32); 28] = [
        ("Parallax_State", 0xFFFF_8890, 0xFFFF_8890),
        ("Parallax_State_End", 0xFFFF_8984, 0xFFFF_8984),
        ("Parallax_Current_Config", 0xFFFF_88B8, 0xFFFF_88B8),
        ("Parallax_Target_Config", 0xFFFF_88BC, 0xFFFF_88BC),
        ("Parallax_Transition_Frames", 0xFFFF_88C0, 0xFFFF_88C0),
        ("Parallax_Snap_Pending", 0xFFFF_88C1, 0xFFFF_88C1),
        ("Parallax_Prev_Sec_X", 0xFFFF_88C2, 0xFFFF_88C2),
        ("Parallax_Prev_Sec_Y", 0xFFFF_88C3, 0xFFFF_88C3),
        ("Parallax_Current_Scroll_A", 0xFFFF_8896, 0xFFFF_8896),
        ("Parallax_Current_Scroll_B", 0xFFFF_88A6, 0xFFFF_88A6),
        ("Parallax_Current_Vscroll_BG", 0xFFFF_88B6, 0xFFFF_88B6),
        ("Parallax_Deform_Phase_FG", 0xFFFF_8890, 0xFFFF_8890),
        ("Parallax_Deform_Phase_BG", 0xFFFF_8892, 0xFFFF_8892),
        ("Parallax_V_Deform_Phase_BG", 0xFFFF_8894, 0xFFFF_8894),
        ("Parallax_Vscroll_Column_Buf", 0xFFFF_88C4, 0xFFFF_88C4),
        ("Parallax_Shadow_Bands", 0xFFFF_8914, 0xFFFF_8914),
        ("Parallax_Shadow_Scroll_A", 0xFFFF_8964, 0xFFFF_8964),
        ("Parallax_Shadow_Scroll_B", 0xFFFF_8974, 0xFFFF_8974),
        ("Camera_X", 0xFFFF_A11C, 0xFFFF_A140),
        ("Camera_Y", 0xFFFF_A120, 0xFFFF_A144),
        ("Current_Act_Ptr", 0xFFFF_AF3C, 0xFFFF_AF60),
        ("Vscroll_Factor", 0xFFFF_888C, 0xFFFF_888C),
        ("Hscroll_Buffer", 0xFFFF_850A, 0xFFFF_850A),
        ("Hscroll_Dirty_Start", 0xFFFF_888A, 0xFFFF_888A),
        ("Hscroll_Dirty_End", 0xFFFF_888B, 0xFFFF_888B),
        ("VDP_Shadow_Table", 0xFFFF_800A, 0xFFFF_800A),
        ("VDP_Dirty_Mask", 0xFFFF_801E, 0xFFFF_801E),
        ("Section_GetSecPtrXY", 0x0000_560C, 0x0000_6296),
    ];
    let mut out = Vec::new();
    for (i, (name, plain, dbg)) in table.iter().enumerate() {
        let vma = if debug { *dbg } else { *plain };
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

/// Lower the real `parallax.emp` (prepend the `engine.constants` twin +
/// `engine.structs` + `engine.vdp`), place into the per-shape map, append the value
/// equs + cross-seam address labels, one `resolve_layout` -> `link`.
fn compile_real_file(
    debug: bool,
    doctor: Option<(&str, &str)>,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let dir = level_dir();
    let main = parse_file(&dir.join("parallax.emp"));
    let constants_file = parse_file(&dir.parent().unwrap().join("system/constants.emp"));
    let structs_file = parse_file(&dir.parent().unwrap().join("structs.emp"));
    let vdp_file = parse_file(&dir.parent().unwrap().join("vdp.emp"));
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items: constants_file
            .items
            .into_iter()
            .chain(structs_file.items)
            .chain(vdp_file.items)
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
        "parallax.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut equs = parallax_value_equs(doctor);
    for sec in &mut equs {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(equs);

    sections.extend(parallax_addr_labels(debug));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

/// parallax.emp's drift guards + the prepended twins' guards must PASS.
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "parallax.emp drift guards must all PASS: {diags:?}"
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
    let section = linked.section("parallax").expect("linked image must carry parallax");
    let shape = if debug { "debug" } else { "plain" };
    assert_region_matches(&section.bytes, expected, &format!("parallax ({shape})"));
}

#[test]
fn parallax_region_matches_reference() {
    run(false);
}

#[test]
fn parallax_debug_region_matches_reference() {
    run(true);
}

/// Negative probe: a DOCTORED `PARALLAX_LERP_SHIFT` truth (5 AS-side while
/// parallax.emp says 4) must fire parallax.emp's own `ensure(extern(…))` guard
/// NAMING the constant — the undoctored control passes (the reference gates above).
#[test]
fn doctored_parallax_lerp_shift_fires_its_guard() {
    let aeon = aeon_dir();
    if !aeon.join("engine/level/parallax.emp").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    let (resolved, _, link_asserts) = compile_real_file(false, Some(("PARALLAX_LERP_SHIFT", "5")));
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    let fired: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(!fired.is_empty(), "the doctored PARALLAX_LERP_SHIFT truth must fire a drift guard");
    assert!(
        fired.iter().any(|d| d.message.contains("PARALLAX_LERP_SHIFT")),
        "the fired guard must NAME the drifted constant: {fired:?}"
    );
}
