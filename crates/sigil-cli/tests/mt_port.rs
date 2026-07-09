//! Sound-migration T2 — the REAL `mt_bank.emp` port, region-level byte gate.
//!
//! `dac_port.rs`'s sibling (Task 6): compiles the ACTUAL ported file from
//! aeon's tree — `games/sonic4/data/sound/mt_bank.emp` — through the
//! production parse -> lower -> place -> resolve -> link pipeline, with
//! `include_root` set to the module's OWN directory (so the six `embed(...)`
//! blobs resolve), and asserts the `mt_bank` section's flattened bytes equal
//! the reference ROM window at the pinned addresses, in BOTH build shapes
//! (`-D DEBUG=0` / `-D DEBUG=1`).
//!
//! ## The cross-seam symbol
//!
//! `mt_bank.emp` carries five link-time `ensure`s, each of the shape
//! `ensure(bankid("X") == bankid("MovingTrucks_Bank_Start"), "...")` — see the
//! module's own header comment for why the ensures read the LABEL rather than
//! the `SND_ENGINE_TABLE_BANK` equ directly (a bare unquoted equ name is not a
//! legal `bankid()` operand outside a call-argument position; the label folds
//! to the identical value since the equ IS that label's address >> 15, and the
//! label is bank-aligned). So the ONLY external symbol this test must supply
//! is `MovingTrucks_Bank_Start` — proven via the T0 `ports.rs::probe_b`
//! technique: a synthetic AS unit that `phase`s a label to the exact VMA the
//! real `.asm` head pins it at ($60000, main.asm:129's `align $8000`), placed
//! at a harness-private LMA that cannot collide with the `mt_bank`/`text`
//! map regions, then concatenated with the `.emp` sections before ONE
//! `resolve_layout` + `link` + `check_link_asserts` pass — mirroring exactly
//! what the real mixed build's cross-seam resolution does (Task 7).
//!
//! ## Reference windows
//!
//! Plain (`DEBUG=0`): `s4.bin[0x60607..0x63AE8]` (13,537 bytes).
//! Debug (`DEBUG=1`): `s4.debug.bin[0x60607..0x6553A]` (20,275 bytes).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, both tests SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure (mirrors the
//! `sigil-harness` `m1d_rom.rs`/`mixed_dac_rom.rs` gate idiom).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test mt_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{LinkAssert, Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

/// The module's own directory in aeon's tree — the `include_root` under which
/// the six `embed("*.bin")` fixtures resolve. Honors `AEON_DIR` (mirroring
/// `dac_port.rs`/the `sigil-harness` gates) with the workspace default.
fn sound_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    Path::new(&aeon).join("games/sonic4/data/sound")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The map: a `text` region for the module's zero-byte default-section
/// carrier (opened by its top-level `ensure`s — R-T0.3's carrier contract;
/// TWO such carriers land here, one before and one after `mt_bank` in
/// declaration order, both zero bytes, sharing the region cumulatively per
/// P5/R7) and the real `mt_bank` region pinned at the R7 LMA, sized to the
/// bank top ($68000).
fn map_toml() -> &'static str {
    "fill = 0x00\n\
     \n\
     [[region]]\n\
     name = \"text\"\n\
     lma_base = 0x0000\n\
     size = 0x10\n\
     kind = \"rom\"\n\
     \n\
     [[region]]\n\
     name = \"mt_bank\"\n\
     lma_base = 0x60607\n\
     size = 0x79F9\n\
     kind = \"rom\"\n"
}

/// The synthetic AS-side cross-seam unit: a label `phase`d to the exact VMA
/// the real `.asm` pins `MovingTrucks_Bank_Start` at ($60000) — the T0
/// `probe_b` idiom (`ports.rs`), which proved a `bankid("Name")` ensure
/// resolves against a label defined this way exactly as it would against the
/// real cross-source symbol.
fn as_bank_start_label() -> Vec<Section> {
    let asm = "cpu 68000\nphase $60000\nMovingTrucks_Bank_Start:\n\tdc.w 0\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (cross-seam label): {d:?}")).sections
}

/// Parse -> lower (with the sound-dir include-root + shape defines) -> place
/// the `.emp` sections into the map -> append the synthetic cross-seam label
/// section at a harness-private LMA (clear of both map regions) -> ONE
/// `resolve_layout` -> `link` -> `check_link_asserts`. Returns the placed+
/// resolved `.emp` sections (for locating `mt_bank`'s bytes), the linked
/// image, and the link-assert diagnostics (expected empty — all five ensures
/// pass).
fn compile_real_file(
    debug: i128,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_span::Diagnostic>, Vec<LinkAssert>) {
    let dir = sound_dir();
    let emp_path = dir.join("mt_bank.emp");
    let src = std::fs::read_to_string(&emp_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", emp_path.display()));

    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "parse errors: {pdiags:?}"
    );

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        defines: vec![("DEBUG".to_string(), debug)],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "lower errors (embed/ensure): {ldiags:?}"
    );

    let map = sigil_link::load_map(map_toml()).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors (region-per-section): {pdiags:?}"
    );

    // Append the cross-seam label section at a harness-private LMA — well
    // clear of both `text` ($0..$10) and `mt_bank` ($60607..$68000) — so it
    // cannot collide with either map region. Its VMA ($60000, from `phase`)
    // is what the `bankid()` ensures actually read; its LMA placement here is
    // an inert harness bookkeeping detail, exactly as in `probe_b`.
    let mut cross_seam = as_bank_start_label();
    for sec in &mut cross_seam {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(cross_seam);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed (bank straddle / ensure?): {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    let assert_diags =
        sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &module.link_asserts);
    (resolved, linked, assert_diags, module.link_asserts)
}

/// On mismatch, report the first differing offset plus 8 bytes of context on
/// each side (`dac_port.rs`/`ports.rs` style byte-diff reporting). M3: the
/// window starts 8 bytes BEFORE the first-diff offset (not at it) so the
/// panic message shows bytes on both sides of the diff, not just after it.
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

/// (DEBUG=0) The `mt_bank` section's linked bytes equal `s4.bin[0x60607..0x63AE8]`.
#[test]
fn mt_bank_region_matches_reference() {
    let aeon = std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    let rom_path = Path::new(&aeon).join("s4.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (_resolved, linked, assert_diags, link_asserts) = compile_real_file(0);
    assert_eq!(
        link_asserts.len(),
        5,
        "mt_bank.emp's five co-residency ensures must be captured"
    );
    assert!(
        assert_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the five cross-seam co-residency ensures must all PASS (link succeeded): {assert_diags:?}"
    );

    let expected = &refrom[0x60607..0x63AE8];
    let section = linked.section("mt_bank").expect("linked image must carry mt_bank");
    assert_region_matches(&section.bytes, expected, "mt_bank (DEBUG=0) vs s4.bin[0x60607..0x63AE8]");
}

/// (DEBUG=1) The `mt_bank` section's linked bytes equal
/// `s4.debug.bin[0x60607..0x6553A]`.
#[test]
fn mt_bank_debug_region_matches_reference() {
    let aeon = std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    let rom_path = Path::new(&aeon).join("s4.debug.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but debug reference missing: {}", rom_path.display());
        }
        eprintln!("skip: debug reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (_resolved, linked, assert_diags, link_asserts) = compile_real_file(1);
    assert_eq!(
        link_asserts.len(),
        5,
        "mt_bank.emp's five co-residency ensures must be captured"
    );
    assert!(
        assert_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the five cross-seam co-residency ensures must all PASS (link succeeded): {assert_diags:?}"
    );

    let expected = &refrom[0x60607..0x6553A];
    let section = linked.section("mt_bank").expect("linked image must carry mt_bank");
    assert_region_matches(&section.bytes, expected, "mt_bank (DEBUG=1) vs s4.debug.bin[0x60607..0x6553A]");
}
