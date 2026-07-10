//! Sound-migration T3 — the REAL `sfx_bank.emp` port, region-level byte gate.
//!
//! `mt_port.rs`'s sibling (Task 4): compiles the ACTUAL ported file from aeon's
//! tree — `games/sonic4/data/sound/sfx/sfx_bank.emp` — through the production
//! parse -> lower -> place -> resolve -> link pipeline, with `include_root` set
//! to the module's OWN directory (so the eighteen `embed(...)` blobs resolve),
//! and asserts the `sfx_bank` section's flattened bytes equal the reference ROM
//! window at the pinned addresses, in BOTH build shapes.
//!
//! ## No shape define
//!
//! Unlike `mt_bank.emp`, this module carries NO `DEBUG` member: the SFX block's
//! CONTENT is byte-identical plain and debug (1864 bytes = `$748` in both) — only
//! its BASE address shifts, because it sits after the shape-dependent song tables
//! (plain `$63AE8` / debug `$6553A`). So the SHAPE lives entirely in the MAP
//! (per-shape `map_toml(debug)` region base, R7), not in the module: `lower` runs
//! with an EMPTY `defines` vec for both shapes.
//!
//! ## The cross-seam symbol
//!
//! `sfx_bank.emp` carries ONE link-time `ensure` of the shape
//! `ensure(bankid("Sfx_33") == bankid("MovingTrucks_Bank_Start"), "...")` (R5 —
//! the :260 co-residency fatal's successor). It reads the LABEL rather than the
//! `SND_ENGINE_TABLE_BANK` equ directly for the same reason `mt_bank.emp` does
//! (the bankid-label idiom, T2 Deviation 2). So the ONLY external symbol this
//! test must supply is `MovingTrucks_Bank_Start` — via the `mt_port` `phase`-label
//! technique verbatim: a synthetic AS unit that `phase`s a label to the exact VMA
//! the real `.asm` head pins it at ($60000, main.asm's `align $8000`), placed at a
//! harness-private LMA that cannot collide with the `sfx_bank`/`text` map regions,
//! then concatenated with the `.emp` sections before ONE `resolve_layout` + `link`
//! + `check_link_asserts` pass.
//!
//! ## Reference windows
//!
//! Plain (map base `$63AE8`): `s4.bin[0x63AE8..0x64230]` (1864 bytes).
//! Debug (map base `$6553A`): `s4.debug.bin[0x6553A..0x65C82]` (1864 bytes).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, both tests SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure (mirrors the
//! `mt_port.rs` gate idiom).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test sfx_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{LinkAssert, Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

/// The module's own directory in aeon's tree — the `include_root` under which
/// the eighteen `embed("sfx_*.bin")` fixtures resolve. Honors `AEON_DIR`
/// (mirroring `mt_port.rs`/the `sigil-harness` gates) with the workspace default.
fn sound_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    Path::new(&aeon).join("games/sonic4/data/sound/sfx")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The map: a `text` region for the module's zero-byte default-section carrier
/// (opened by its top-level `ensure` — R-T0.3's carrier contract) and the real
/// `sfx_bank` region pinned at the R7 PER-SHAPE LMA, sized to the bank top
/// ($68000). The ONLY structural difference from `mt_port.rs`'s map: the region
/// base is shape-dependent, so this is a `fn of debug` — plain `$63AE8`/`$4518`,
/// debug `$6553A`/`$2AC6` (both run to the `$68000` bank top).
fn map_toml(debug: bool) -> String {
    let (base, size) = if debug {
        ("0x6553A", "0x2AC6")
    } else {
        ("0x63AE8", "0x4518")
    };
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
         name = \"sfx_bank\"\n\
         lma_base = {base}\n\
         size = {size}\n\
         kind = \"rom\"\n"
    )
}

/// The synthetic AS-side cross-seam unit: a label `phase`d to the exact VMA the
/// real `.asm` pins `MovingTrucks_Bank_Start` at ($60000) — the `mt_port`/T0
/// `probe_b` idiom, which proved a `bankid("Name")` ensure resolves against a
/// label defined this way exactly as it would against the real cross-source
/// symbol.
fn as_bank_start_label() -> Vec<Section> {
    let asm = "cpu 68000\nphase $60000\nMovingTrucks_Bank_Start:\n\tdc.w 0\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (cross-seam label): {d:?}")).sections
}

/// Parse -> lower (with the sfx-dir include-root, NO defines) -> place the `.emp`
/// sections into the per-shape map -> append the synthetic cross-seam label
/// section at a harness-private LMA (clear of both map regions) -> ONE
/// `resolve_layout` -> `link` -> `check_link_asserts`. Returns the placed+resolved
/// `.emp` sections, the linked image, the link-assert diagnostics (expected empty
/// — the ONE ensure passes), and the module's captured link asserts.
fn compile_real_file(
    debug: bool,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_span::Diagnostic>, Vec<LinkAssert>) {
    let dir = sound_dir();
    let emp_path = dir.join("sfx_bank.emp");
    let src = std::fs::read_to_string(&emp_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", emp_path.display()));

    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "parse errors: {pdiags:?}"
    );

    // NO defines: the SFX block is shape-invariant; the shape lives in the map.
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "lower errors (embed/ensure): {ldiags:?}"
    );

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors (region-per-section): {pdiags:?}"
    );

    // Append the cross-seam label section at a harness-private LMA — well clear
    // of both `text` ($0..$10) and `sfx_bank` — so it cannot collide with either
    // map region. Its VMA ($60000, from `phase`) is what the `bankid()` ensure
    // actually reads; its LMA placement here is inert harness bookkeeping.
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

/// On mismatch, report the first differing offset plus 8 bytes of context on each
/// side (`mt_port.rs` style byte-diff reporting): the window starts 8 bytes BEFORE
/// the first-diff offset (not at it) so the panic message shows bytes on both
/// sides of the diff, not just after it.
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

/// (plain) The `sfx_bank` section's linked bytes equal `s4.bin[0x63AE8..0x64230]`.
#[test]
fn sfx_bank_region_matches_reference() {
    let aeon = std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    let rom_path = Path::new(&aeon).join("s4.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (_resolved, linked, assert_diags, link_asserts) = compile_real_file(false);
    assert_eq!(
        guard_assert_count(&link_asserts),
        1,
        "sfx_bank.emp's single co-residency ensure must be captured"
    );
    assert!(
        assert_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the cross-seam co-residency ensure must PASS (link succeeded): {assert_diags:?}"
    );

    let expected = &refrom[0x63AE8..0x64230];
    let section = linked.section("sfx_bank").expect("linked image must carry sfx_bank");
    assert_region_matches(&section.bytes, expected, "sfx_bank (plain) vs s4.bin[0x63AE8..0x64230]");
}

/// (debug) The `sfx_bank` section's linked bytes equal
/// `s4.debug.bin[0x6553A..0x65C82]`.
#[test]
fn sfx_bank_debug_region_matches_reference() {
    let aeon = std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    let rom_path = Path::new(&aeon).join("s4.debug.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but debug reference missing: {}", rom_path.display());
        }
        eprintln!("skip: debug reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (_resolved, linked, assert_diags, link_asserts) = compile_real_file(true);
    assert_eq!(
        guard_assert_count(&link_asserts),
        1,
        "sfx_bank.emp's single co-residency ensure must be captured"
    );
    assert!(
        assert_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the cross-seam co-residency ensure must PASS (link succeeded): {assert_diags:?}"
    );

    let expected = &refrom[0x6553A..0x65C82];
    let section = linked.section("sfx_bank").expect("linked image must carry sfx_bank");
    assert_region_matches(&section.bytes, expected, "sfx_bank (debug) vs s4.debug.bin[0x6553A..0x65C82]");
}

/// Count the deferred GUARD asserts, excluding the D2.29 [layout.odd-item]
/// parity asserts that now also ride module.link_asserts. Shared idiom in
/// `sigil_harness::test_support`.
use sigil_harness::test_support::guard_assert_count;
