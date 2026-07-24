//! Tranche 22 (file 3) — the REAL `compression_selftest.emp` port,
//! region-level byte gate. THE CAMPAIGN'S FIRST DEBUG-ONLY REGION.
//!
//! Compiles the actual ported file — `engine/debug/compression_selftest.emp`
//! — through the production pipeline and asserts the `compression_selftest`
//! region's bytes equal the DEBUG reference ROM window. The twin is
//! whole-file `ifdef __DEBUG__`: the region exists ONLY in the debug shape
//! (`pins::COMPRESSION_SELFTEST.plain_len == 0`), so the byte gate has one
//! shape arm; the plain-shape proof (the gate emits NOTHING into the plain
//! ROM) rides the `mixed_tranche22_rom` full-ROM arm.
//!
//! ## Cross-seam symbols
//! - VALUE symbols: `CSELF_PAYLOAD_SIZE` / `CSELF_PAYLOAD_SUM` /
//!   `CSELF_DICT_LEN` are GENERATED per build (tools/gen_compression_vectors
//!   .py) — this test PARSES them from the real
//!   `engine/debug/generated/vectors.asm`, never hardcodes (the generator
//!   owns the values). The bare link-immediate arithmetic spelling
//!   (`#CSELF_PAYLOAD_SIZE/2-1`) is probe-pinned in
//!   `tranche22_spelling_probes`.
//! - ADDRESS carriers: `Art_Staging_Buffer`, the five `CSelf_*` data labels
//!   (debug-only pins), the two .emp-owned callees `S4LZ_DecompressDict` /
//!   `Art_Decompress` (standalone-gate carriers — the real module-to-module
//!   link rides the mixed arm), and the MDDBG assert handlers.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test compression_selftest_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

fn aeon_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    PathBuf::from(aeon)
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// Parse the three generated `CSELF_*` constants from the REAL vectors.asm —
/// the generator owns the values; hardcoding them here would drift on the
/// next payload change.
fn generated_cself_values(aeon: &Path) -> Vec<(String, String)> {
    let path = aeon.join("engine/debug/generated/vectors.asm");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let mut out = Vec::new();
    for name in ["CSELF_PAYLOAD_SIZE", "CSELF_PAYLOAD_SUM", "CSELF_DICT_LEN"] {
        let line = text
            .lines()
            .find(|l| l.trim_start().starts_with(name))
            .unwrap_or_else(|| panic!("{name} not in {}", path.display()));
        let rhs = line.split('=').nth(1).expect("CSELF line has `=`");
        let value = rhs.split(';').next().unwrap().trim().to_string();
        out.push((name.to_string(), value));
    }
    out
}

/// The VALUE seam: the generated CSELF_* truths. `doctor` overrides ONE pair
/// (the negative probe).
fn value_equs(aeon: &Path, doctor: Option<(&str, &str)>) -> Vec<Section> {
    let mut owned = generated_cself_values(aeon);
    if let Some((name, val)) = doctor {
        let mut hit = false;
        for p in owned.iter_mut() {
            if p.0 == name {
                p.1 = val.to_string();
                hit = true;
            }
        }
        assert!(hit, "doctor target `{name}` not in the value seam");
    }
    let pairs: Vec<(&str, &str)> = owned.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// The cross-seam ADDRESS symbols, each a `phase`d one-byte carrier.
fn addr_labels() -> Vec<Section> {
    let table: Vec<(&str, u32)> = vec![
        ("Art_Staging_Buffer", pins::ART_STAGING_BUFFER.debug),
        ("CSelf_S4LZ_Plain", pins::C_SELF_S4_LZ_PLAIN),
        ("CSelf_S4LZ_Dict", pins::C_SELF_S4_LZ_DICT),
        ("CSelf_Dict_Blob", pins::C_SELF_DICT_BLOB),
        ("CSelf_ZX0", pins::C_SELF_ZX0),
        ("CSelf_Expected", pins::C_SELF_EXPECTED),
        ("S4LZ_DecompressDict", pins::S4_LZ_DECOMPRESS_DICT.debug),
        // Art_Decompress = the load_art region's start symbol.
        ("Art_Decompress", pins::LOAD_ART.debug_base),
        ("MDDBG__ErrorHandler", pins::MDDBG_ERROR_HANDLER),
        ("MDDBG__ErrorHandler_PagesController", pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER),
    ];
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

/// Lower the real `compression_selftest.emp` at DEBUG=1 (the only shape the
/// module exists in), place at the debug window, link with the seams.
fn compile_real_file(doctor: Option<(&str, &str)>) -> sigil_link::LinkedImage {
    let aeon = aeon_dir();
    let dir = aeon.join("engine/debug");
    let file = parse_file(&dir.join("compression_selftest.emp"));

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), 1)],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "compression_selftest.emp lower errors: {ldiags:?}"
    );

    let base = pins::COMPRESSION_SELFTEST.debug_base;
    let len = pins::COMPRESSION_SELFTEST.debug_len;
    let map_toml = format!(
        "fill = 0x00\n\n[[region]]\nname = \"text\"\nlma_base = 0x0000\nsize = 0x10\nkind = \"rom\"\n\n[[region]]\nname = \"compression_selftest\"\nlma_base = {base:#x}\nsize = {len:#x}\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map_toml).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    sections.extend(value_equs(&aeon, doctor));
    sections.extend(addr_labels());

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"))
}

/// The debug-shape byte gate: the .emp code region vs the shipped
/// s4.debug.bin window.
#[test]
fn compression_selftest_debug_region_matches_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.debug.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let linked = compile_real_file(None);
    let base = pins::COMPRESSION_SELFTEST.debug_base as usize;
    let len = pins::COMPRESSION_SELFTEST.debug_len;
    let expected = &refrom[base..base + len];
    let section =
        linked.section("compression_selftest").expect("linked image must carry the region");
    assert_eq!(section.bytes.len(), expected.len(), "compression_selftest (debug): length");
    if let Some(i) = (0..expected.len()).find(|&i| section.bytes[i] != expected[i]) {
        let lo = i.saturating_sub(8);
        let hi = (i + 16).min(expected.len());
        panic!(
            "compression_selftest (debug): first diff at {i:#x}\n  candidate[{lo:#x}..{hi:#x}]: {:02x?}\n  expected[{lo:#x}..{hi:#x}]:  {:02x?}",
            &section.bytes[lo..hi],
            &expected[lo..hi]
        );
    }
}

/// The debug-only-region shape fact, pinned: the plain shape carries ZERO
/// bytes (the full-ROM proof that the gated plain build emits nothing is the
/// `mixed_tranche22_rom` arm).
#[test]
fn compression_selftest_plain_region_is_empty() {
    assert_eq!(pins::COMPRESSION_SELFTEST.plain_len, 0, "debug-only region: plain must be empty");
}

/// Negative probe: a DOCTORED `CSELF_PAYLOAD_SUM` truth must CHANGE the
/// emitted bytes (the assert immediates genuinely ride the link-time value
/// seam — a hardcoded mirror would keep matching and hide generator drift).
#[test]
fn doctored_cself_sum_diverges_from_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.debug.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM missing");
        return;
    };
    let linked = compile_real_file(Some(("CSELF_PAYLOAD_SUM", "$1234")));
    let base = pins::COMPRESSION_SELFTEST.debug_base as usize;
    let len = pins::COMPRESSION_SELFTEST.debug_len;
    let expected = &refrom[base..base + len];
    let section =
        linked.section("compression_selftest").expect("linked image must carry the region");
    assert_ne!(
        section.bytes, expected,
        "a doctored CSELF_PAYLOAD_SUM must change the emitted bytes — the value seam is dead if this matches"
    );
}
