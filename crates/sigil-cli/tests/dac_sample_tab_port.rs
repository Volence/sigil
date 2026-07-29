//! t27 phase-2 lane C — the REAL `dac_sample_tab.emp` port: the DAC sample
//! descriptor table (10 × 9-byte descriptor of build-time SND_* constants),
//! proven by the WINDOWED oracle (the z80_init_port template applied to a
//! banked comptime-data table). Closes the P1 (Z80 little-endian data) probe for
//! comptime-constant cells at file scale, and P5 (the `if/fatal` size guard has
//! a comptime `.emp` equivalent that FIRES on a doctored size).
//!
//! `dac_sample_tab.asm` is included inside main.asm's `cpu z80 / phase 08000h`
//! block — a CANONICAL banked head table. Its cells are the SND_* build-time
//! constants (from data/sound/dac_samples.asm), which the AS twin FOLDS to
//! literals at assemble time. The .emp folds the SAME comptime constants
//! (supplied as `-D` defines — a width-1 `db` cannot carry a link symbol in a
//! Z80 section, and these ARE constants). The size guard reads DacSample_len /
//! DAC_SAMPLE_COUNT via `extern(...)` (equ carriers): a link-time drift guard,
//! the doctored-extern P5 positive control.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test dac_sample_tab_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{SectionPlacement, SymbolTable};
use sigil_span::Level;
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The 10 descriptors' BANK/PTR/LEN build-time constants. Representative-but-
/// consistent values that fit the cell widths (BANK ≤ $FF, PTR/LEN ≤ $FFFF; PTR
/// in the $8000 window, LEN < $8000). `doctor` overrides ONE named value — the
/// t24 byte-gate positive control (it changes that descriptor's bytes on the
/// twin side only).
fn snd_seam(doctor: Option<(&str, i64)>) -> Vec<(String, i64)> {
    let descriptors = [
        "BLIP", "KICK", "SNARE", "HAT", "S3K_KICK", "S3K_SNARE", "S3K_HITOM", "S3K_MIDTOM",
        "S3K_LOWTOM", "S3K_FLOORTOM",
    ];
    let mut out = Vec::new();
    for (i, d) in descriptors.iter().enumerate() {
        let i = i as i64;
        out.push((format!("SND_{d}_BANK"), 0x0A + i));
        out.push((format!("SND_{d}_PTR"), 0x8000 + i * 0x0100));
        out.push((format!("SND_{d}_LEN"), 0x0100 + i * 0x0080));
    }
    if let Some((dn, dv)) = doctor {
        for (n, v) in out.iter_mut() {
            if n == dn {
                *v = dv;
            }
        }
    }
    out
}

/// The size-guard seam symbols (DacSample_len / DAC_SAMPLE_COUNT), fed as equ
/// carriers so the .emp's `extern(...)` size guard resolves them at link.
/// `doctor` overrides one — the P5 positive control (the guard fires).
fn guard_seam(doctor: Option<(&str, i64)>) -> Vec<(String, i64)> {
    let mut out = vec![("DAC_SAMPLE_COUNT".to_string(), 10i64), ("DacSample_len".to_string(), 9)];
    if let Some((dn, dv)) = doctor {
        for (n, v) in out.iter_mut() {
            if n == dn {
                *v = dv;
            }
        }
    }
    out
}

/// Compile dac_sample_tab.emp at `(cpu:z80, vma:$8000)` with the SND_* as
/// comptime `-D` defines and the size-guard symbols as equ carriers; resolve +
/// link; return the `dac_sample_tab` section bytes OR the link/lower diagnostics
/// (for the P5 positive control, where a doctored guard extern fails).
fn compile_emp(
    snd_doctor: Option<(&str, i64)>,
    guard_doctor: Option<(&str, i64)>,
) -> Result<Vec<u8>, Vec<sigil_span::Diagnostic>> {
    let aeon = aeon_dir();
    let dir = aeon.join("engine/sound");
    let src = std::fs::read_to_string(dir.join("dac_sample_tab.emp"))
        .unwrap_or_else(|e| panic!("read dac_sample_tab.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != Level::Error),
        "dac_sample_tab.emp parse errors: {pdiags:?}"
    );
    let defines: Vec<(String, i128)> =
        snd_seam(snd_doctor).into_iter().map(|(n, v)| (n, v as i128)).collect();
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines,
    };
    let (module, ldiags) = lower_module(&file, &opts);
    if ldiags.iter().any(|d| d.level == Level::Error) {
        return Err(ldiags);
    }
    // The extern-based size guard defers to a LinkAssert (its condition is a
    // link-time value); the linker evaluates it against the resolved symbol
    // table. Carry them past the section move so we can check them below.
    let link_asserts = module.link_asserts;
    // The module-level `ensure` lives in the implicit top-level `text` section
    // (it emits no bytes, but needs a region to place). The table is its own
    // named section at the banked LMA.
    let map = "fill = 0x00\n\n[[region]]\nname = \"text\"\nlma_base = 0x0\nsize = 0x10\nkind = \"rom\"\n\n[[region]]\nname = \"dac_sample_tab\"\nlma_base = 0x8000\nsize = 0x100\nkind = \"rom\"\n";
    let map = sigil_link::load_map(map).expect("map loads");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != Level::Error),
        "place_sections errors: {pdiags:?}"
    );
    // The size-guard symbols as equ carriers (the extern(...) reads at link).
    let pairs: Vec<(String, String)> =
        guard_seam(guard_doctor).into_iter().map(|(n, v)| (n, format!("${v:X}"))).collect();
    let refs: Vec<(&str, &str)> = pairs.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
    let mut carriers = sigil_harness::test_support::assemble_equ_pairs(&refs);
    for (i, sec) in carriers.iter_mut().enumerate() {
        sec.lma = 0x0100_0000 + (i as u32) * 0x1000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(carriers);
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)?;
    let linked = sigil_link::link(&resolved, &SymbolTable::new())?;
    // Evaluate the deferred size guard against the resolved symbol table (the
    // equ carriers provide DAC_SAMPLE_COUNT / DacSample_len). A firing guard is
    // the P5 positive control.
    let assert_diags =
        sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    if assert_diags.iter().any(|d| d.level == Level::Error) {
        return Err(assert_diags);
    }
    Ok(linked.section("dac_sample_tab").expect("linked dac_sample_tab").bytes.clone())
}

/// The AS twin oracle: assemble dac_sample_tab.asm through sigil's AS front-end
/// at `cpu z80`, with ALL SND_* + guard symbols as equs, return the bytes.
fn as_twin_bytes(snd_doctor: Option<(&str, i64)>, guard_doctor: Option<(&str, i64)>) -> Vec<u8> {
    let aeon = aeon_dir();
    let body = std::fs::read_to_string(aeon.join("engine/sound/dac_sample_tab.asm"))
        .unwrap_or_else(|e| panic!("read dac_sample_tab.asm: {e}"));
    // Equs in default (68000) mode, where `$` hex parses; the table body then
    // assembles under `cpu z80 / phase 08000h`.
    let mut seam = String::from("cpu 68000\n");
    for (name, val) in snd_seam(snd_doctor) {
        seam.push_str(&format!("{name} = ${val:X}\n"));
    }
    for (name, val) in guard_seam(guard_doctor) {
        seam.push_str(&format!("{name} = ${val:X}\n"));
    }
    seam.push_str("cpu z80\nphase 08000h\n");
    let src = format!("{seam}{body}\ndephase\n");
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
    if !aeon.join("engine/sound/dac_sample_tab.asm").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return true;
    }
    false
}

/// THE windowed byte gate: dac_sample_tab.emp == dac_sample_tab.asm. Proves P1
/// (Z80 little-endian data) for comptime-constant cells on the real 10-descriptor
/// table (90 bytes).
#[test]
fn dac_sample_tab_matches_as_twin() {
    if skip_if_missing() {
        return;
    }
    let emp = compile_emp(None, None).expect("undoctored .emp links");
    let twin = as_twin_bytes(None, None);
    assert!(!twin.is_empty(), "the AS twin must emit the descriptor table");
    assert_eq!(twin.len(), 10 * 9, "the table is 10 descriptors × 9 bytes (90 bytes)");
    assert_bytes_match(&emp, &twin, "dac_sample_tab.emp vs dac_sample_tab.asm");
}

/// Positive control (byte-gate non-triviality, t24): the UNDOCTORED .emp must
/// DIVERGE from a twin assembled with ONE doctored SND_* value — proving the
/// comparison genuinely detects a per-cell difference, not a vacuous match.
#[test]
fn emp_diverges_from_doctored_twin() {
    if skip_if_missing() {
        return;
    }
    let emp = compile_emp(None, None).expect("undoctored .emp links");
    let twin = as_twin_bytes(Some(("SND_BLIP_PTR", 0x9ABC)), None); // one ds_ptr moved
    assert_ne!(emp, twin, "the byte gate is vacuous if the .emp matches a doctored twin");
}

/// P5 positive control (the `if/fatal` size guard fires): compiling the .emp with
/// a DOCTORED DAC_SAMPLE_COUNT extern (11 instead of 10) makes the size guard's
/// `ensure` FAIL — 10×9 ≠ 11×9. The guard is falsifiable, not vacuous.
#[test]
fn doctored_dac_sample_count_fires_size_guard() {
    if skip_if_missing() {
        return;
    }
    let err = compile_emp(None, Some(("DAC_SAMPLE_COUNT", 11)))
        .expect_err("a doctored DAC_SAMPLE_COUNT must fire the size guard");
    assert!(
        err.iter().any(|d| d.level == Level::Error
            && d.message.contains("DacSampleTable size")),
        "expected the size-guard ensure to fire, got: {err:?}"
    );
}

/// P5 negative side (the guard is quiet when the count is right): the UNDOCTORED
/// count links cleanly (the undoctored compile of the same guard path — the
/// positive control's partner, so a passing gate is not vacuous silence).
#[test]
fn undoctored_dac_sample_count_passes_size_guard() {
    if skip_if_missing() {
        return;
    }
    compile_emp(None, None).expect("the undoctored size guard must pass (10×9 == 10×9)");
}
