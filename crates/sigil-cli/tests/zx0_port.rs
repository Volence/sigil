//! Tranche 22 (file 2) — the REAL `zx0_decompress.emp` port, region-level
//! byte gate.
//!
//! Compiles the actual ported file — `engine/compression/zx0_decompress.emp`
//! — through the production parse -> lower -> place -> resolve -> link
//! pipeline and asserts the `zx0` region's flattened bytes equal the
//! reference ROM window at the pinned base, in BOTH build shapes. The ZX0
//! (modern/V2) blocking decompressor — Emmanuel Marty's 88-byte depacker,
//! algorithm untouched.
//!
//! ## Shape
//! Shape-INVARIANT length ($58 — the file has no comptime arms), so the two
//! gates differ only in the pinned base.
//!
//! ## Cross-seam symbols
//! NONE — the module is fully self-contained (every branch target is a proc
//! local; no consts, no RAM, no asserts). There is consequently no value
//! seam to doctor: the tranche's negative-probe obligation is carried by
//! `s4lz_port::doctored_tile_size_fires_its_guard` and the
//! compression_selftest gate's doctored-seam probe.
//!
//! The ownership-flip link test for row 39 lives with the caller
//! (`load_art_port::two_module_ownership_flip_*` — gained
//! zx0_decompress.emp as a compiled module and DROPPED the ZX0_Decompress
//! carrier the same commit the extern decl died).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test zx0_port
//! ```

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use std::path::{Path, PathBuf};

fn region_base(debug: bool) -> u32 {
    if debug { pins::ZX0.debug_base } else { pins::ZX0.plain_base }
}

fn region_len(debug: bool) -> usize {
    if debug { pins::ZX0.debug_len } else { pins::ZX0.plain_len }
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
         name = \"zx0\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
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

/// Lower the real `zx0_decompress.emp`, place into the per-shape map, one
/// `resolve_layout` -> `link`.
fn compile_real_file(debug: bool) -> sigil_link::LinkedImage {
    let aeon = aeon_dir();
    let dir = aeon.join("engine/compression");
    let file = parse_file(&dir.join("zx0_decompress.emp"));

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), i128::from(debug))],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "zx0_decompress.emp lower errors: {ldiags:?}"
    );

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"))
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

    let linked = compile_real_file(debug);
    let base = region_base(debug) as usize;
    let expected = &refrom[base..base + region_len(debug)];
    let section = linked.section("zx0").expect("linked image must carry zx0");
    let shape = if debug { "debug" } else { "plain" };
    assert_region_matches(&section.bytes, expected, &format!("zx0 ({shape})"));
}

#[test]
fn zx0_region_matches_reference() {
    run(false);
}

#[test]
fn zx0_debug_region_matches_reference() {
    run(true);
}
