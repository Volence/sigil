//! Port #1 T3 — the REAL `hblank.emp` port, region-level byte gate.
//!
//! `sfx_port.rs`'s sibling for the campaign's first CODE port: compiles the
//! ACTUAL ported file from aeon's tree — `engine/system/hblank.emp` — through
//! the production parse -> lower -> place -> resolve -> link pipeline, and
//! asserts the `hblank` section's flattened bytes equal the reference ROM
//! window at the pinned addresses, in BOTH build shapes.
//!
//! ## No shape define
//!
//! `hblank.emp` carries no `DEBUG` member and needs none: the block's CONTENT
//! is byte-identical plain and debug (18 bytes in both) — only its BASE
//! address shifts (plain `$227E`, debug `$230C`), so the shape lives entirely
//! in the MAP (per-shape `map_toml(debug)` region base), exactly like
//! `sfx_port.rs`. `lower_module` runs with an EMPTY `defines` vec for both
//! shapes.
//!
//! ## The cross-seam symbols
//!
//! Two directions:
//!
//! - INBOUND: `HBlank_Dispatch`'s body reads `HBlank_Handler_Ptr`, an AS-side
//!   RAM label (`engine/ram.asm:72`, `$FFFF8022`, same value both shapes).
//!   Supplied here by a synthetic AS unit that `phase`s a label to that exact
//!   VMA — the `sfx_port.rs::as_bank_start_label` technique verbatim.
//! - OUTBOUND (THE BARE-NAME PROOF): `vectors.asm:36` (`dc.l HBlank_Dispatch`)
//!   and `boot.asm:185` (`move.l #HBlank_Null, (HBlank_Handler_Ptr).w`) are
//!   real AS-side consumers of the two `pub proc` names. This test builds a
//!   SYNTHETIC stand-in for both consumer shapes (through the AS front-end,
//!   like the SFX template's synthetic consumers) and asserts the linked
//!   bytes carry the correct per-shape address — proving `pub proc` names
//!   surface as BARE link symbols cross-seam (mirroring `pub data`, proven by
//!   the sound ports' `dc.l`/`dc.w` consumers). This was the one open risk
//!   flagged in the port's fact base (T3 risk note) — settled here.
//!
//! ## Reference windows
//!
//! Plain (map base `$227E`): `s4.bin[0x227E..0x2290]` (18 bytes).
//! Debug (map base `$230C`): `s4.debug.bin[0x230C..0x231E]` (18 bytes).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, both tests SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure (mirrors the
//! `sfx_port.rs` gate idiom).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test hblank_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

/// The module's own directory in aeon's tree — `hblank.emp` has no `embed`s,
/// but `include_root` is still set for parity with every other port template.
fn hblank_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    Path::new(&aeon).join("engine/system")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The map: a `text` region for the module's zero-byte default-section carrier
/// (opened by the module's top-level items, if any — `place_sections` requires
/// a home for it regardless), and the real `hblank` region pinned at the
/// per-shape reference base, sized to the 18-byte block. Only the region base
/// differs from `sfx_port.rs`'s map shape: plain `$227E`, debug `$230C`, both
/// size `$12`.
fn map_toml(debug: bool) -> String {
    let base = if debug { "0x230C" } else { "0x227E" };
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
         name = \"hblank\"\n\
         lma_base = {base}\n\
         size = 0x12\n\
         kind = \"rom\"\n"
    )
}

/// The synthetic AS-side cross-seam unit: a label `phase`d to the exact VMA
/// the real `ram.asm` pins `HBlank_Handler_Ptr` at (`$FFFF8022`, same value
/// both shapes) — the `sfx_port.rs::as_bank_start_label` idiom verbatim.
fn as_handler_ptr_label() -> Vec<Section> {
    let asm = "cpu 68000\nphase $FFFF8022\nHBlank_Handler_Ptr:\n\tdc.l 0\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (cross-seam label): {d:?}")).sections
}

/// The synthetic AS-side OUTBOUND consumer — THE BARE-NAME PROOF. Mirrors the
/// real `vectors.asm:36` (`dc.l HBlank_Dispatch`, an Abs32 fixup in a vector
/// table) and `boot.asm:185` (`move.l #HBlank_Null, (HBlank_Handler_Ptr).w`,
/// an imm32 fixup) shapes, assembled through the AS front-end exactly like a
/// real consumer would be. If `HBlank_Dispatch`/`HBlank_Null` do not surface
/// as BARE link symbols from the `.emp` module, these two fixups fail to
/// resolve at `link` time (or resolve to the wrong address), so this section's
/// bytes are the proof surface.
fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tdc.l HBlank_Dispatch\n\
               \tmove.l #HBlank_Null, ($FFF0).w\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}")).sections
}

/// Parse -> lower (with the module-dir include_root, NO defines) -> place the
/// `.emp` sections into the per-shape map -> append the synthetic cross-seam
/// sections (inbound RAM label + outbound consumer) at harness-private LMAs
/// (clear of both map regions) -> ONE `resolve_layout` -> `link`. Returns the
/// placed+resolved `.emp` sections and the linked image.
fn compile_real_file(debug: bool) -> (Vec<Section>, sigil_link::LinkedImage) {
    let dir = hblank_dir();
    let emp_path = dir.join("hblank.emp");
    let src = std::fs::read_to_string(&emp_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", emp_path.display()));

    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "parse errors: {pdiags:?}"
    );

    // NO defines: the hblank block is shape-invariant; the shape lives in the map.
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "lower errors: {ldiags:?}"
    );

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors (region-per-section): {pdiags:?}"
    );

    // Append BOTH synthetic cross-seam sections at harness-private LMAs — well
    // clear of `text` ($0..$10) and `hblank` — so neither collides with either
    // map region. The inbound label's VMA ($FFFF8022, from `phase`) is what
    // `movea.l HBlank_Handler_Ptr, a0` reads; the outbound consumer's LMA here
    // is inert (its own fixups target `.emp`-defined VMAs, not its own address).
    let mut cross_seam = as_handler_ptr_label();
    for sec in &mut cross_seam {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(cross_seam);

    let mut consumer = as_outbound_consumer();
    for sec in &mut consumer {
        sec.lma = 0x0200_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(consumer);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked)
}

/// On mismatch, report the first differing offset plus 8 bytes of context on
/// each side (`sfx_port.rs` style byte-diff reporting).
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

/// (plain) The `hblank` section's linked bytes equal `s4.bin[0x227E..0x2290]`,
/// AND the outbound consumer's fixups resolve to the correct per-shape
/// addresses (`$0000227E`/`$0000228E`) — the bare-name proof.
#[test]
fn hblank_region_matches_reference() {
    let aeon = std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    let rom_path = Path::new(&aeon).join("s4.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (_resolved, linked) = compile_real_file(false);

    let expected = &refrom[0x227E..0x2290];
    let section = linked.section("hblank").expect("linked image must carry hblank");
    assert_region_matches(&section.bytes, expected, "hblank (plain) vs s4.bin[0x227E..0x2290]");

    // The bare-name proof: `dc.l HBlank_Dispatch` = $0000227E at bytes [0..4);
    // `move.l #HBlank_Null, ($FFF0).w` follows at [4..12) — opcode word
    // [4..6), the imm32 fixup hole at [6..10) resolving to $0000228E
    // (HBlank_Null is the SECOND proc, right after the 16-byte
    // HBlank_Dispatch body), then the abs.w dest ext word at [10..12).
    let consumer = linked.section("sec0").expect("linked image must carry the outbound consumer");
    assert_eq!(
        &consumer.bytes[0..4],
        &[0x00, 0x00, 0x22, 0x7E],
        "bare-name proof: `dc.l HBlank_Dispatch` must resolve to $0000227E (plain)"
    );
    assert_eq!(
        &consumer.bytes[6..10],
        &[0x00, 0x00, 0x22, 0x8E],
        "bare-name proof: `move.l #HBlank_Null` imm32 must resolve to $0000228E (plain)"
    );
}

/// (debug) The `hblank` section's linked bytes equal
/// `s4.debug.bin[0x230C..0x231E]`, AND the outbound consumer's fixups resolve
/// to the correct per-shape addresses (`$0000230C`/`$0000231C`).
#[test]
fn hblank_debug_region_matches_reference() {
    let aeon = std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    let rom_path = Path::new(&aeon).join("s4.debug.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but debug reference missing: {}", rom_path.display());
        }
        eprintln!("skip: debug reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (_resolved, linked) = compile_real_file(true);

    let expected = &refrom[0x230C..0x231E];
    let section = linked.section("hblank").expect("linked image must carry hblank");
    assert_region_matches(&section.bytes, expected, "hblank (debug) vs s4.debug.bin[0x230C..0x231E]");

    let consumer = linked.section("sec0").expect("linked image must carry the outbound consumer");
    assert_eq!(
        &consumer.bytes[0..4],
        &[0x00, 0x00, 0x23, 0x0C],
        "bare-name proof: `dc.l HBlank_Dispatch` must resolve to $0000230C (debug)"
    );
    assert_eq!(
        &consumer.bytes[6..10],
        &[0x00, 0x00, 0x23, 0x1C],
        "bare-name proof: `move.l #HBlank_Null` imm32 must resolve to $0000231C (debug)"
    );
}

