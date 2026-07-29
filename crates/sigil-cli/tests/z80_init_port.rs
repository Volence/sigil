//! t27 phase-2 lane A — the REAL `z80_init.emp` port: the FIRST Z80 CODE module
//! and the first live `module (cpu: z80)`. Proven by the OFF-CANONICAL
//! no-sound-shape oracle (the game_debug template, for a Z80 phase-0 section).
//!
//! z80_init.asm is included by boot_data.asm ONLY in the `else` of
//! `ifdef SOUND_DRIVER_ENABLED` — both shipped shapes have sound ENABLED, so it
//! emits ZERO bytes canonically. There is no canonical reference-ROM window to
//! diff. The oracle is the AS twin assembled through sigil's own AS front-end at
//! the no-sound shape: `z80_init.emp` compiled at `(cpu:z80, phase 0)` must
//! byte-match `z80_init.asm` assembled at phase 0, with `Z80_RAM`/`Z80_RAM_END`
//! (the only cross-seam symbols) shared as equs/externs by both sides.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test z80_init_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
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

/// The two cross-seam RAM constants (constants.asm). Shared by the .emp (as equ
/// carriers its `extern(...)` reads) and the AS twin (as equs). `doctor`
/// overrides Z80_RAM_END (the negative-probe: it feeds the ldir clear count).
fn ram_seam(doctor: Option<i64>) -> Vec<(&'static str, i64)> {
    vec![
        ("Z80_RAM", 0xA00000),
        ("Z80_RAM_END", doctor.unwrap_or(0xA02000)),
    ]
}

/// Compile z80_init.emp at phase 0, resolve + link, return the `z80_idle`
/// section bytes.
fn compile_emp(doctor: Option<i64>) -> Vec<u8> {
    let aeon = aeon_dir();
    let dir = aeon.join("engine/system");
    let src = std::fs::read_to_string(dir.join("z80_init.emp"))
        .unwrap_or_else(|e| panic!("read z80_init.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "z80_init.emp parse errors: {pdiags:?}"
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "z80_init.emp lower errors: {:?}",
        ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );
    // The single section is phased at vma $0; pin it at LMA 0.
    let map = "fill = 0x00\n\n[[region]]\nname = \"z80_idle\"\nlma_base = 0x0\nsize = 0x100\nkind = \"rom\"\n";
    let map = sigil_link::load_map(map).expect("map loads");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );
    // The two RAM consts as equ carriers (the extern(...) reads).
    let pairs: Vec<(&str, String)> =
        ram_seam(doctor).into_iter().map(|(n, v)| (n, format!("${v:X}"))).collect();
    let refs: Vec<(&str, &str)> = pairs.iter().map(|(n, v)| (*n, v.as_str())).collect();
    let mut carriers = sigil_harness::test_support::assemble_equ_pairs(&refs);
    for (i, sec) in carriers.iter_mut().enumerate() {
        sec.lma = 0x0100_0000 + (i as u32) * 0x1000; // clear of z80_idle's LMA 0
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(carriers);
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    linked.section("z80_idle").expect("linked z80_idle").bytes.clone()
}

/// The AS twin oracle: assemble z80_init.asm through sigil's AS front-end at
/// phase 0, with the two RAM equs, and return the emitted Z80 code bytes.
fn as_twin_bytes(doctor: Option<i64>) -> Vec<u8> {
    let aeon = aeon_dir();
    let body = std::fs::read_to_string(aeon.join("engine/system/z80_init.asm"))
        .unwrap_or_else(|e| panic!("read z80_init.asm: {e}"));
    let mut seam = String::from("cpu 68000\n");
    for (name, val) in ram_seam(doctor) {
        seam.push_str(&format!("{name} = ${val:X}\n"));
    }
    let src = format!("{seam}{body}\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let out = assemble(&src, &opts).unwrap_or_else(|d| panic!("AS twin assemble: {d:?}"));
    let mut sections = out.sections;
    for sec in &mut sections {
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("AS twin resolve failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("AS twin link failed: {d:?}"));
    // The idle program is the only emitted content (phase 0); collect the one
    // non-empty section's bytes.
    linked
        .sections
        .iter()
        .find(|s| !s.bytes.is_empty())
        .map(|s| s.bytes.clone())
        .unwrap_or_default()
}

fn assert_bytes_match(candidate: &[u8], expected: &[u8], what: &str) {
    assert_eq!(
        candidate.len(),
        expected.len(),
        "{what}: length mismatch ({} vs {})",
        candidate.len(),
        expected.len()
    );
    if let Some(i) = (0..candidate.len()).find(|&i| candidate[i] != expected[i]) {
        let lo = i.saturating_sub(6);
        let hi = (i + 10).min(candidate.len());
        panic!(
            "{what}: first diff at {i:#x}\n  emp[{lo:#x}..{hi:#x}]: {:02x?}\n  twin[{lo:#x}..{hi:#x}]: {:02x?}",
            &candidate[lo..hi],
            &expected[lo..hi]
        );
    }
}

fn skip_if_missing() -> bool {
    let aeon = aeon_dir();
    if !aeon.join("engine/system/z80_init.asm").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return true;
    }
    false
}

/// THE off-canonical byte gate: z80_init.emp == z80_init.asm (no-sound shape).
/// Proves the whole T1 operand model + the t27 expression-imm16 defer + the
/// trailing-local end label on a REAL Z80 code file.
#[test]
fn z80_init_matches_as_twin() {
    if skip_if_missing() {
        return;
    }
    let emp = compile_emp(None);
    let twin = as_twin_bytes(None);
    assert!(!twin.is_empty(), "the AS twin must emit the idle program");
    assert_eq!(twin.len(), 38, "the idle program is 38 bytes");
    assert_bytes_match(&emp, &twin, "z80_init.emp vs z80_init.asm (no-sound shape)");
}

/// Positive control (byte gate non-triviality, t24): the UNDOCTORED .emp must
/// DIVERGE from a twin assembled with a doctored Z80_RAM_END (which changes the
/// ldir clear-count immediate) — proving the comparison genuinely detects a
/// difference, not a vacuous equality.
#[test]
fn emp_diverges_from_doctored_twin() {
    if skip_if_missing() {
        return;
    }
    let emp = compile_emp(None);
    let twin = as_twin_bytes(Some(0xA04000)); // RAM_END moved → different clear count
    assert_ne!(emp, twin, "the byte gate is vacuous if the .emp matches a doctored twin");
}

/// The `extern(...)` clear-count tracks Z80_RAM_END: doctoring BOTH sides the
/// same way keeps them equal (the imm16 defer resolves the residual identically).
#[test]
fn doctored_both_sides_stay_equal() {
    if skip_if_missing() {
        return;
    }
    let emp = compile_emp(Some(0xA04000));
    let twin = as_twin_bytes(Some(0xA04000));
    assert_bytes_match(&emp, &twin, "z80_init.emp vs twin (both RAM_END=$A04000)");
}
