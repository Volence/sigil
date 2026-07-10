//! Port #2 Task 3 — the REAL `controllers.emp` port, region-level byte gate.
//!
//! `hblank_port.rs`'s sibling for the second CODE port: compiles the ACTUAL
//! ported file from aeon's tree — `engine/system/controllers.emp` — through
//! the production parse -> lower -> place -> resolve -> link pipeline, and
//! asserts the `controllers` section's flattened bytes equal the reference
//! ROM window at the pinned addresses, in BOTH build shapes.
//!
//! ## No shape define
//!
//! Like `hblank.emp`, `controllers.emp` carries no `DEBUG` member: the
//! block's CONTENT is byte-identical plain and debug (0x72 bytes in both) —
//! only its BASE address shifts (plain `$228C`, debug `$231A`), so the shape
//! lives entirely in the MAP. `lower_module` runs with an EMPTY `defines`
//! vec for both shapes.
//!
//! ## `controllers.emp` now composes `engine.constants`
//!
//! `controllers.emp` no longer declares its own `BUTTON_*` consts — it `use`s
//! them from the `engine.constants` twin (`engine/system/constants.emp`,
//! tranche 2's step-2 modernize pass), alongside the two `HW_PORT_*_DATA`
//! consts the twin also carries (unused as consts here — the `lea.l
//! HW_PORT_1_DATA, a0` operands stay bare symbolic references, per
//! `map_plain`'s design: a bare non-`#` operand is ALWAYS a symbol lookup,
//! never a const read, so importing the same-named const is inert for that
//! operand shape and byte-neutral).
//!
//! Compiling `controllers.emp` standalone therefore needs `engine.constants`'
//! six `pub const`s in scope. The full multi-module resolver
//! (`resolve::build_program`) is NOT used here: its whole-program
//! `report_unresolved` check demands every bare symbol reference resolve
//! within the SCANNED `.emp` manifest, which wrongly rejects this file's
//! genuinely AS-side-only symbols (`Ctrl_1_Held` & co — never declared in any
//! `.emp` file, supplied only via the synthetic `as_ctrl_ram_labels` section
//! below). Every prior cross-seam port (dac/mt/sfx/hblank/math) sidesteps
//! this the same way: lower each `.emp` file SEPARATELY via plain
//! `lower_module` and concatenate. `prepend_ambient_consts` extends that
//! precedent one step — mirroring `build_program`'s own internal technique
//! (`resolve/mod.rs`'s `ambient_items` + synthetic-`File` prepend, `Item`
//! being `Clone` and carrying no lowering side effects until `lower_module`
//! walks it) MINUS the whole-program closure check: `engine.constants`'s six
//! items (`pub const` ×8) are parsed once and prepended to `controllers.emp`'s
//! own items before ONE `lower_module` call over the merged synthetic file —
//! so the consts resolve locally, `engine.constants`'s own eight
//! `ensure(extern(...) == ...)` drift guards ride along and their link asserts
//! attach to the merged module, and every genuinely-external bare symbol
//! (`HW_PORT_*_DATA`, `Ctrl_*`) stays exactly the link-time-deferred `Sym`
//! fixup it always was.
//!
//! `engine.constants`'s eight drift guards need the REAL AS-side equs to check
//! against. `as_hw_port_equs` below now defines the four `BUTTON_*` equs
//! (`engine/constants.asm:89-92`) alongside the two `HW_PORT_*_DATA` equs it
//! already carried, so this test's own compile is what the twin's drift
//! guards check against; the guards must all PASS (checked via
//! `check_link_asserts`).
//!
//! ## The cross-seam symbols
//!
//! Two kinds of INBOUND references, both supplied as synthetic AS-side
//! sections (`hblank_port.rs`'s technique, extended to equs per
//! `symbolic_operands.rs::as_side_int_equ_resolves_an_emp_relax_abs_sym_operand_in_a_mixed_link`):
//!
//! - Six AS-side `equ`s: `HW_PORT_1_DATA = $A10003` / `HW_PORT_2_DATA =
//!   $A10005` (`engine/constants.asm:17-18`), read via `lea.l
//!   HW_PORT_1_DATA, a0` / `lea.l HW_PORT_2_DATA, a0` — bare symbolic
//!   absolute operands, exported through the genuine AS-equ-export path
//!   (`directive_equate`, unconditional int-equ export to `equ_syms`) — PLUS
//!   `BUTTON_UP`/`BUTTON_DOWN`/`BUTTON_LEFT`/`BUTTON_RIGHT`
//!   (`engine/constants.asm:89-92`), read ONLY by `engine.constants`'s own
//!   drift-guard `ensure`s (never as bare operands).
//! - Four AS-side RAM labels: `Ctrl_1_Held`/`Ctrl_2_Held`/
//!   `Ctrl_1_Press_Accum`/`Ctrl_2_Press_Accum` (`engine/ram.asm`, phased at
//!   `$FFFF802C`/`$FFFF802E`/`$FFFF8030`/`$FFFF8031` — verified by counting
//!   forward from `HBlank_Handler_Ptr` = `$FFFF8022`, port #1's pinned base,
//!   through the intervening `ds.b`/`ds.w` fields), read via bare `move.b`
//!   operands — `hblank_port.rs::as_handler_ptr_label`'s technique, one
//!   `phase`d section carrying all four labels.
//!
//! OUTBOUND: `Read_Controllers` is called from `engine/system/vblank.asm`
//! (`bsr.w Read_Controllers`, both the lag and non-lag paths) — this test
//! builds a synthetic AS-side `bsr.w Read_Controllers` consumer (through the
//! AS front-end) and asserts the linked fixup resolves to the correct
//! per-shape address, proving the `pub proc` name surfaces as a bare link
//! symbol cross-seam (the same proof shape as `hblank_port.rs`'s
//! `dc.l HBlank_Dispatch`, but for a PC-relative `bsr.w` fixup instead of an
//! absolute one).
//!
//! ## Reference windows
//!
//! Plain (map base `$228C`): `s4.bin[0x228C..0x22FE]` (0x72 bytes).
//! Debug (map base `$231A`): `s4.debug.bin[0x231A..0x238C]` (0x72 bytes).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, both tests SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test controllers_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

/// The module's own directory in aeon's tree — `controllers.emp` has no
/// `embed`s, but `include_root` is still set for parity with every other
/// port template.
fn controllers_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    Path::new(&aeon).join("engine/system")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The map: a `text` region for BOTH `controllers.emp`'s own zero-byte
/// default-section carrier AND `engine.constants`'s zero-byte carrier (same
/// name, chained — the P5/R7 same-named-`text`-chains-cleanly precedent), and
/// the real `controllers` region pinned at the per-shape reference base,
/// sized to the 0x72-byte block. Only the region base differs from
/// `hblank_port.rs`'s map shape: plain `$228C`, debug `$231A`, both size
/// `$72`.
fn map_toml(debug: bool) -> String {
    let base = if debug { "0x231A" } else { "0x228C" };
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
         name = \"controllers\"\n\
         lma_base = {base}\n\
         size = 0x72\n\
         kind = \"rom\"\n"
    )
}

/// The synthetic AS-side cross-seam unit supplying the two hardware-port
/// `equ`s — `engine/constants.asm:17-18` verbatim — through the genuine
/// AS-equ-export path (`directive_equate`'s unconditional int-equ export to
/// `equ_syms`, per `symbolic_operands.rs`'s
/// `as_side_int_equ_resolves_an_emp_relax_abs_sym_operand_in_a_mixed_link`
/// precedent), PLUS the four `BUTTON_*` equs — `engine/constants.asm:89-92`
/// verbatim — that `engine.constants`'s own drift-guard `ensure`s read back
/// via `extern(...)` (tranche 2's constants twin). A trailing label+`dc.w`
/// opens a section so the equs (defined before any section) flush via
/// `pending_equ_syms` into it.
fn as_hw_port_equs() -> Vec<Section> {
    // The 19-value AS-truth blob for the `engine.constants` twin (SOURCE OF
    // TRUTH: `constants.asm`), consolidated in `sigil_harness::test_support`.
    sigil_harness::test_support::as_engine_constants_equs()
}

/// The synthetic AS-side cross-seam unit supplying the four `Ctrl_*` RAM
/// labels — `engine/ram.asm`, phased at their exact VMAs (verified by
/// counting forward from port #1's pinned `HBlank_Handler_Ptr` = `$FFFF8022`
/// through the intervening `ds.b`/`ds.w` fields: `Hardware_Region` +4,
/// `Region_Flags` +1, `Timing_Step` +1 (`ds.w`), `Frame_Accumulator` +2
/// (`ds.w`), `Ctrl_1_Held` +2, `Ctrl_1_Press` +1, `Ctrl_2_Held` +1,
/// `Ctrl_2_Press` +1, `Ctrl_1_Press_Accum` +1, `Ctrl_2_Press_Accum` +1) —
/// `hblank_port.rs::as_handler_ptr_label`'s technique, one `phase`d section
/// carrying all four labels at their real relative offsets.
fn as_ctrl_ram_labels() -> Vec<Section> {
    let asm = "cpu 68000\n\
               phase $FFFF802C\n\
               Ctrl_1_Held:\n\
               \tdc.b 0\n\
               \tds.b 1\n\
               Ctrl_2_Held:\n\
               \tdc.b 0\n\
               \tds.b 1\n\
               Ctrl_1_Press_Accum:\n\
               \tdc.b 0\n\
               Ctrl_2_Press_Accum:\n\
               \tdc.b 0\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (ctrl ram labels): {d:?}")).sections
}

/// The synthetic AS-side OUTBOUND consumer — THE BARE-NAME PROOF. Mirrors
/// the real `vblank.asm:89`/`:152` shape (`bsr.w Read_Controllers`),
/// assembled through the AS front-end exactly like a real consumer would be.
/// If `Read_Controllers` does not surface as a BARE link symbol from the
/// `.emp` module, this fixup fails to resolve at `link` time (or resolves to
/// the wrong address), so this section's bytes are the proof surface.
fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tbsr.w   Read_Controllers\n\
               \trts\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}")).sections
}

/// Parse `constants.emp`, parse `controllers.emp`, and return a synthetic
/// [`ast::File`] carrying `constants.emp`'s items (its eight `pub const`s AND
/// its six `ensure(extern(...) == ...)` drift guards — both ride along, same
/// as `build_program`'s own ambient-injection technique) PREPENDED to
/// `controllers.emp`'s own items, under `controllers.emp`'s own module
/// header. See this file's top doc comment for why a full multi-module
/// resolve isn't used instead.
fn controllers_with_ambient_constants() -> sigil_frontend_emp::ast::File {
    let dir = controllers_dir();
    let constants_src = std::fs::read_to_string(dir.join("constants.emp"))
        .unwrap_or_else(|e| panic!("cannot read constants.emp: {e}"));
    let (constants_file, cdiags) = parse_str(&constants_src);
    assert!(
        cdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "constants.emp parse errors: {cdiags:?}"
    );

    let controllers_src = std::fs::read_to_string(dir.join("controllers.emp"))
        .unwrap_or_else(|e| panic!("cannot read controllers.emp: {e}"));
    let (controllers_file, pdiags) = parse_str(&controllers_src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "controllers.emp parse errors: {pdiags:?}"
    );

    sigil_frontend_emp::ast::File {
        module: controllers_file.module.clone(),
        attrs: controllers_file.attrs.clone(),
        items: constants_file
            .items
            .into_iter()
            .chain(controllers_file.items)
            .collect(),
        docs: controllers_file.docs.clone(),
    }
}

/// Lower the merged `constants.emp` + `controllers.emp` synthetic file (NO
/// defines: the controllers block is shape-invariant; the shape lives in the
/// map) -> place the `.emp` sections into the per-shape map (both modules'
/// zero-byte `text` carriers chain into the same region; `controllers.emp`'s
/// real section lands in `controllers`) -> append the THREE synthetic
/// cross-seam sections (hw-port + BUTTON_* equs + ctrl RAM labels + outbound
/// consumer) at harness-private LMAs (clear of both map regions) -> ONE
/// `resolve_layout` -> `link`. Returns the placed+resolved `.emp` sections,
/// the linked image, and `engine.constants`' six drift-guard link asserts
/// (for `check_link_asserts`).
fn compile_real_file(debug: bool) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let dir = controllers_dir();
    let file = controllers_with_ambient_constants();

    // NO defines: the controllers block is shape-invariant; the shape lives
    // in the map.
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors (region-per-section): {pdiags:?}"
    );

    // Append all THREE synthetic cross-seam sections at harness-private LMAs
    // — well clear of `text` ($0..$10) and `controllers` — so none collides
    // with either map region.
    let mut hw_equs = as_hw_port_equs();
    for sec in &mut hw_equs {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(hw_equs);

    let mut ram_labels = as_ctrl_ram_labels();
    for sec in &mut ram_labels {
        sec.lma = 0x0200_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(ram_labels);

    let mut consumer = as_outbound_consumer();
    for sec in &mut consumer {
        sec.lma = 0x0300_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(consumer);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

/// On mismatch, report the first differing offset plus 8 bytes of context on
/// each side.
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

/// (plain) The `controllers` section's linked bytes equal
/// `s4.bin[0x228C..0x22FE]`, AND the outbound `bsr.w` consumer's fixup
/// resolves to the correct per-shape address ($228C) — the bare-name proof.
#[test]
fn controllers_region_matches_reference() {
    let aeon = std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    let rom_path = Path::new(&aeon).join("s4.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (resolved, linked, link_asserts) = compile_real_file(false);

    // `engine.constants`'s eight drift-guard `ensure`s must genuinely run and
    // pass — the twin's BUTTON_*/HW_PORT_*_DATA consts must equal the
    // synthetic AS-side equs `as_hw_port_equs` defines. `guard_assert_count`
    // (mirrors `mixed_dac_rom.rs`'s helper) excludes the D2.29
    // `[layout.odd-item]` parity assert that also rides `module.link_asserts`
    // (the merged file's odd 68k proc labels), so the eight drift guards pin
    // exactly.
    let assert_diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    assert!(
        assert_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "engine.constants's drift-guard ensures must all PASS: {assert_diags:?}"
    );
    assert_eq!(guard_assert_count(&link_asserts), 19, "engine.constants's nineteen drift guards must be captured");

    let expected = &refrom[0x228C..0x22FE];
    let section = linked.section("controllers").expect("linked image must carry controllers");
    assert_region_matches(&section.bytes, expected, "controllers (plain) vs s4.bin[0x228C..0x22FE]");

    // The bare-name proof: `bsr.w Read_Controllers` = opcode `6100` + a
    // 16-bit PC-relative displacement, computed as `target - (consumer.lma +
    // 2)` truncated to 16 bits (the harness-private LMA $0300_0000 is far
    // from the ROM-range target $228C, so the raw disp16 wraps — expected
    // and harmless, since the linker and this assertion both do modular
    // 16-bit arithmetic; a REAL consumer, placed near its target in ROM,
    // would see the same disp16 bytes without any wraparound).
    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == 0x0300_0000)
        .expect("linked image must carry the outbound consumer at its harness-private LMA");
    let disp = i16::from_be_bytes([consumer.bytes[2], consumer.bytes[3]]);
    let expected_disp = (0x228Ci64 - (consumer.lma as i64 + 2)) as i16;
    assert_eq!(
        disp, expected_disp,
        "bare-name proof: `bsr.w Read_Controllers` must resolve to $228C (plain)"
    );
}

/// (debug) The `controllers` section's linked bytes equal
/// `s4.debug.bin[0x231A..0x238C]`, AND the outbound consumer's fixup
/// resolves to the correct per-shape address ($231A).
#[test]
fn controllers_debug_region_matches_reference() {
    let aeon = std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    let rom_path = Path::new(&aeon).join("s4.debug.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but debug reference missing: {}", rom_path.display());
        }
        eprintln!("skip: debug reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (resolved, linked, link_asserts) = compile_real_file(true);

    let assert_diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    assert!(
        assert_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "engine.constants's drift-guard ensures must all PASS: {assert_diags:?}"
    );
    assert_eq!(guard_assert_count(&link_asserts), 19, "engine.constants's nineteen drift guards must be captured");

    let expected = &refrom[0x231A..0x238C];
    let section = linked.section("controllers").expect("linked image must carry controllers");
    assert_region_matches(&section.bytes, expected, "controllers (debug) vs s4.debug.bin[0x231A..0x238C]");

    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == 0x0300_0000)
        .expect("linked image must carry the outbound consumer at its harness-private LMA");
    let disp = i16::from_be_bytes([consumer.bytes[2], consumer.bytes[3]]);
    let expected_disp = (0x231Ai64 - (consumer.lma as i64 + 2)) as i16;
    assert_eq!(
        disp, expected_disp,
        "bare-name proof: `bsr.w Read_Controllers` must resolve to $231A (debug)"
    );
}

/// Count the deferred GUARD asserts, excluding the D2.29 `[layout.odd-item]`
/// parity asserts that also ride `module.link_asserts`. Shared idiom in
/// `sigil_harness::test_support`.
use sigil_harness::test_support::guard_assert_count;
