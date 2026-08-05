//! Tranche 22 (file 1) — the REAL `s4lz.emp` port, region-level
//! byte gate.
//!
//! Compiles the actual ported file — `engine/compression/s4lz.emp`
//! — through the production parse -> lower -> place -> resolve -> link
//! pipeline and asserts the `s4lz` region's flattened bytes equal the
//! reference ROM window at the pinned base, in BOTH build shapes. The S4LZ v3
//! blocking decompressor: the dict entry (`S4LZ_DecompressDict`, window-rebase
//! preamble) falling through into the shared-body plain entry
//! (`S4LZ_Decompress`) via `falls_into`, plus the internal `TileDelta_Undo`.
//!
//! ## Shape
//! Shape-DEPENDENT length ($FC plain / $200 debug — the debug surplus is the
//! three `ifdebug` assert blocks: dict-even, version-byte, dict-range).
//!
//! ## Cross-seam symbols
//! The module is SELF-CONTAINED (every branch/copy target is internal); the
//! only seams are the debug shape's MDDBG handlers (the assert blobs) and the
//! `TILE_SIZE` VALUE mirror (file-local const + `ensure(extern(...))` drift
//! lock — the negative probe doctors it).
//!
//! The ownership-flip link tests for rows 38/30 live with the CALLERS
//! (`load_art_port::two_module_ownership_flip_*`,
//! `tile_cache_port::two_module_tail_call_flip_*` — each gained
//! s4lz.emp as a compiled module and DROPPED its address carrier
//! the same commit the extern decl died).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test s4lz_port
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
    if debug { pins::S4LZ.debug_base } else { pins::S4LZ.plain_base }
}

fn region_len(debug: bool) -> usize {
    if debug { pins::S4LZ.debug_len } else { pins::S4LZ.plain_len }
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
         name = \"s4lz\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// The VALUE seam: the TILE_SIZE drift-lock truth (engine/constants.asm).
/// `doctor` overrides it (the negative probe).
fn value_equs(doctor: Option<&str>) -> Vec<Section> {
    let pairs: Vec<(&str, &str)> = vec![("TILE_SIZE", doctor.unwrap_or("32"))];
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// The cross-seam ADDRESS symbols — debug shape only: the assert blobs'
/// MDDBG error-handler entries (the rings_port precedent).
fn addr_labels(debug: bool) -> Vec<Section> {
    let mut table: Vec<(&str, u32)> = Vec::new();
    if debug {
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

/// Lower the real `s4lz.emp`, place into the per-shape map, append
/// the value equ + (debug) address labels, one `resolve_layout` -> `link`.
fn compile_real_file(
    debug: bool,
    doctor: Option<&str>,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let aeon = aeon_dir();
    let dir = aeon.join("engine/compression");
    let file = parse_file(&dir.join("s4lz.emp"));

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), i128::from(debug))],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "s4lz.emp lower errors: {ldiags:?}"
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
        "s4lz.emp drift guards must all PASS: {diags:?}"
    );

    let base = region_base(debug) as usize;
    let expected = &refrom[base..base + region_len(debug)];
    let section = linked.section("s4lz").expect("linked image must carry s4lz");
    let shape = if debug { "debug" } else { "plain" };
    assert_region_matches(&section.bytes, expected, &format!("s4lz ({shape})"));
}

#[test]
fn s4lz_region_matches_reference() {
    run(false);
}

#[test]
fn s4lz_debug_region_matches_reference() {
    run(true);
}

// `doctored_tile_size_fires_its_guard` RETIRED at the conv-b constants-tail
// flip: TILE_SIZE flipped from s4lz.emp's local mirror to `use engine.constants`,
// so no `ensure(extern("TILE_SIZE") == …)` wall survives for the doctored probe to
// fire. Its protection re-homes to the six-target byte-identity gate.
