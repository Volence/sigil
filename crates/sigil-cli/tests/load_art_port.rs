//! Tranche 20 — the REAL `load_art.emp` port, region-level byte gate.
//!
//! Compiles the actual ported file — `engine/level/load_art.emp` — through the
//! production parse -> lower -> place -> resolve -> link pipeline and asserts
//! the `load_art` region's flattened bytes equal the reference ROM window at
//! the pinned base, in BOTH build shapes. The level art loader: the
//! version-dispatched blocking decompressor (Art_Decompress) and the paged
//! act-pool load loop (Level_LoadArt) with its out-of-line drop handler.
//!
//! ## Shape
//! Shape-DEPENDENT length ($68 plain / $B2 debug — the debug surplus is the
//! `.drop_page` raise_error expansion; the release arm is the 6-byte
//! drain-and-retry).
//!
//! ## Cross-seam symbols
//! - Address carriers: `Art_Staging_Buffer` (abs.l RAM — $FFFF0000 sits
//!   OUTSIDE the abs.w window, so the bare spelling width-selects .l),
//!   `S4LZ_Decompress` / `ZX0_Decompress` / `VSync_Wait` (extern .asm
//!   callees), `QueueDMA_Critical` + `BG_Init` (.emp-owned in their own
//!   modules — supplied as address carriers HERE exactly like the dplc
//!   standalone gate supplies its .emp-owned callees; the module-to-module
//!   proof lives in the flip tests), and (debug) the MDDBG handlers the
//!   raise_error blob targets.
//! - VALUE mirrors: the engine.constants ART_*/DMA consts + the
//!   engine.structs walls (prepended twins' ensures).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test load_art_port
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
    if debug { pins::LOAD_ART.debug_base } else { pins::LOAD_ART.plain_base }
}

fn region_len(debug: bool) -> usize {
    if debug { pins::LOAD_ART.debug_len } else { pins::LOAD_ART.plain_len }
}

fn aeon_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    PathBuf::from(aeon)
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

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
         name = \"load_art\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// The VALUE seam: the prepended twins' drift-lock truths. `doctor` overrides
/// ONE pair (the negative probe).
fn value_equs(doctor: Option<(&str, &str)>) -> Vec<Section> {
    let mut pairs: Vec<(&str, &str)> = Vec::new();
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    pairs.extend(sigil_harness::test_support::act_sec_field_equs());
    if let Some((name, val)) = doctor {
        let mut hit = false;
        for p in pairs.iter_mut() {
            if p.0 == name {
                p.1 = val;
                hit = true;
            }
        }
        assert!(hit, "doctor target `{name}` not in the value seam");
    }
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// The cross-seam ADDRESS symbols, each a `phase`d one-byte carrier at its
/// pinned per-shape VMA.
fn addr_labels(debug: bool) -> Vec<Section> {
    let pick = |p: pins::Pin| -> u32 { if debug { p.debug } else { p.plain } };
    let mut table: Vec<(&str, u32)> = vec![
        ("Art_Staging_Buffer", pick(pins::ART_STAGING_BUFFER)),
        ("S4LZ_Decompress", pick(pins::S4_LZ_DECOMPRESS)),
        ("ZX0_Decompress", pick(pins::ZX0_DECOMPRESS)),
        ("VSync_Wait", pick(pins::V_SYNC_WAIT)),
        ("QueueDMA_Critical", pick(pins::QUEUE_DMA_CRITICAL)),
        ("BG_Init", pick(pins::BG_INIT)),
    ];
    if debug {
        // Debug shape only: the raise_error construct's error-handler entry
        // points (the rings_port precedent).
        table.push(("MDDBG__ErrorHandler", pins::MDDBG_ERROR_HANDLER));
        table.push((
            "MDDBG__ErrorHandler_PagesController",
            pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER,
        ));
    }
    let mut out = Vec::new();
    for (i, (name, vma)) in table.iter().enumerate() {
        let vma = *vma;
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

/// Lower the real `load_art.emp` (prepend the engine.structs +
/// engine.constants twins its `use` lines read), place into the per-shape
/// map, append the value equs + address labels, one `resolve_layout` -> `link`.
fn compile_real_file(
    debug: bool,
    doctor: Option<(&str, &str)>,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let aeon = aeon_dir();
    let dir = aeon.join("engine/level");
    let main = parse_file(&dir.join("load_art.emp"));
    let structs_file = parse_file(&aeon.join("engine/structs.emp"));
    let consts_file = parse_file(&aeon.join("engine/system/constants.emp"));
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items: structs_file
            .items
            .into_iter()
            .chain(consts_file.items)
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
        "load_art.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    sections.extend(value_equs(doctor));
    sections.extend(addr_labels(debug));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
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
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "load_art.emp drift guards must all PASS: {diags:?}"
    );

    let base = region_base(debug) as usize;
    let expected = &refrom[base..base + region_len(debug)];
    let section = linked.section("load_art").expect("linked image must carry load_art");
    let shape = if debug { "debug" } else { "plain" };
    assert_region_matches(&section.bytes, expected, &format!("load_art ({shape})"));
}

#[test]
fn load_art_region_matches_reference() {
    run(false);
}

#[test]
fn load_art_debug_region_matches_reference() {
    run(true);
}

/// Negative probe: a DOCTORED `ART_VER_ZX0` truth (3 AS-side while the twin
/// says 2) must FIRE the engine.constants drift guard, naming the const —
/// the version-dispatch compare rides exactly this value.
#[test]
fn doctored_art_ver_zx0_fires_its_guard() {
    if !strict_gate() && !aeon_dir().join("s4.bin").exists() {
        eprintln!("skip: reference ROM missing");
        return;
    }
    let (resolved, _, link_asserts) = compile_real_file(false, Some(("ART_VER_ZX0", "3")));
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    let fired: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(!fired.is_empty(), "the doctored ART_VER_ZX0 truth must fire a drift guard");
    assert!(
        fired.iter().any(|d| d.message.contains("ART_VER_ZX0")),
        "the guard must NAME the constant: {fired:?}"
    );
}
