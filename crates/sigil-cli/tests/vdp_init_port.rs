//! Tranche 3 — the REAL `vdp_init.emp` port, region-level byte gate.
//!
//! `controllers_port.rs`'s sibling for the FIFTH code port: compiles the
//! ACTUAL ported file from aeon's tree — `engine/system/vdp_init.emp` —
//! through the production parse -> lower -> place -> resolve -> link
//! pipeline, and asserts the `vdp_init` section's flattened bytes equal the
//! reference ROM window at the pinned addresses, in BOTH build shapes.
//!
//! ## No shape define
//!
//! Like every code port so far, `vdp_init.emp` carries no `DEBUG` member:
//! the block's CONTENT is byte-identical plain and debug (0x48 bytes in
//! both) — only its BASE address shifts (plain `$1C14`, debug `$1C96`), so
//! the shape lives entirely in the MAP. `lower_module` runs with an EMPTY
//! `defines` vec for both shapes.
//!
//! ## What this port exercises that the prior four did not
//!
//! - **`(An,Xn)` indexed EA** — `move.b (a0,d2.w), d0` (Flush's shadow-value
//!   load) is the first real consumer of the tranche-3 operand-grammar
//!   addition (68k `(d8,An,Xn)`, brief extension word, d=0).
//! - **Cross-seam pc-relative EA** — `lea.l BootData_VDPRegs(pc), a0`
//!   targets an AS-side ROM label (`engine/system/boot.asm`), so the
//!   `PcRelDisp16` fixup resolves against a link-supplied symbol at its true
//!   per-shape VMA (plain `$3CE`, debug `$3D2`) — tranche 2 proved the
//!   pc-rel machinery cross-SECTION; this is the first cross-SEAM consumer.
//! - **`dbf` + `btst Dn,Dn` + dual-postinc `move.b (a0)+, (a1)+`** in a real
//!   ported body.
//!
//! ## The cross-seam symbols
//!
//! INBOUND references, supplied as synthetic AS-side sections (the
//! `controllers_port.rs` technique):
//!
//! - One AS-side `equ`: `VDP_CTRL = $C00004` (`engine/constants.asm:12`),
//!   read via `lea.l VDP_CTRL, a1` — a bare symbolic absolute operand the
//!   linker widths to abs.l ($C00004 has no abs.w spelling).
//! - Two AS-side RAM labels: `VDP_Shadow_Table`/`VDP_Dirty_Mask`
//!   (`engine/ram.asm`, phased at `$FFFF800A`/`$FFFF801E` — engine RAM, so
//!   SHAPE-INVARIANT; the 20-byte gap is the 19-byte shadow table plus its
//!   odd-length pad byte), widthed to abs.w by the asl rule.
//! - One AS-side ROM label: `BootData_VDPRegs` (`engine/system/boot.asm`),
//!   phased at its true per-shape VMA — the pc-rel target described above.
//!
//! `VDP_Shadow_len` comes from the `engine.constants` twin (step 2's
//! migration — `use engine.constants.{VDP_Shadow_len}`), so the twin's EIGHT
//! drift-guard `ensure`s ride this gate: `as_twin_equs` supplies all eight
//! AS-side values (incl. `VDP_Shadow_len`, which the REAL tree derives from
//! the `VDP_Shadow` struct — struct-generated `_len` symbols export over the
//! same equ seam), and both tests `check_link_asserts` them.
//!
//! OUTBOUND: `VDP_Shadow_Init` is called from `engine/system/boot.asm`
//! (`bsr.w VDP_Shadow_Init`) and `Flush_VDP_Shadow` from the VBlank path —
//! this test builds a synthetic AS-side consumer with BOTH `bsr.w` forms and
//! asserts each fixup resolves to the correct per-shape address (plain
//! `$1C14`/`$1C2A`, debug `$1C96`/`$1CAC`), proving both `pub proc` names
//! surface as bare link symbols cross-seam.
//!
//! ## Reference windows
//!
//! Plain (map base `$1C14`): `s4.bin[0x1C14..0x1C5C]` (0x48 bytes).
//! Debug (map base `$1C96`): `s4.debug.bin[0x1C96..0x1CDE]` (0x48 bytes).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, both tests SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test vdp_init_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

/// The module's own directory in aeon's tree — `vdp_init.emp` has no
/// `embed`s, but `include_root` is still set for parity with every other
/// port template.
fn vdp_init_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    Path::new(&aeon).join("engine/system")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The map: a `text` region for `vdp_init.emp`'s zero-byte default-section
/// carrier, and the real `vdp_init` region pinned at the per-shape reference
/// base, sized to the 0x48-byte block (plain `$1C14`, debug `$1C96`).
fn map_toml(debug: bool) -> String {
    let base = if debug { "0x1C96" } else { "0x1C14" };
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
         name = \"vdp_init\"\n\
         lma_base = {base}\n\
         size = 0x48\n\
         kind = \"rom\"\n"
    )
}

/// The synthetic AS-side cross-seam unit supplying the `VDP_CTRL` equ —
/// `engine/constants.asm:12` verbatim — PLUS the eight values
/// `engine.constants`'s drift guards read back through `extern()` (the twin
/// rides along via the ambient prepend, `controllers_port.rs`'s technique).
/// A trailing label+`dc.w` opens a section so the equs (defined before any
/// section) flush via `pending_equ_syms` into it.
fn as_twin_equs() -> Vec<Section> {
    // `VDP_CTRL` is this gate's own extra (a real operand it consumes); the
    // 19-value constants-twin blob (SOURCE OF TRUTH: `constants.asm`) is shared
    // via `sigil_harness::test_support`.
    let mut pairs = vec![("VDP_CTRL", "$C00004")];
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// The synthetic AS-side cross-seam unit supplying the two VDP RAM labels —
/// `engine/ram.asm`, phased at their exact VMAs (`VDP_Shadow_Table` =
/// `$FFFF800A`, then 19 shadow bytes + the odd-length pad byte =
/// `VDP_Dirty_Mask` at `$FFFF801E`; engine RAM, shape-invariant — verified
/// against both shapes' symbol tables).
fn as_vdp_ram_labels() -> Vec<Section> {
    let asm = "cpu 68000\n\
               phase $FFFF800A\n\
               VDP_Shadow_Table:\n\
               \tdc.b 0\n\
               \tds.b 19\n\
               VDP_Dirty_Mask:\n\
               \tdc.l 0\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (vdp ram labels): {d:?}")).sections
}

/// The synthetic AS-side cross-seam unit supplying `BootData_VDPRegs` at its
/// TRUE per-shape VMA (plain `$3CE`, debug `$3D2` — `engine/system/boot.asm`,
/// read from the shape's symbol table). Unlike the abs-widthed RAM labels,
/// this one is a PC-RELATIVE target (`lea.l BootData_VDPRegs(pc), a0`), so
/// the label's absolute position is load-bearing: the `PcRelDisp16` fixup
/// resolves to `target_vma - (site_vma + 2)` and the reference bytes only
/// match when the label sits where the real boot.asm put it.
fn as_bootdata_label(debug: bool) -> Vec<Section> {
    let base = if debug { "$3D2" } else { "$3CE" };
    let asm = format!(
        "cpu 68000\n\
         phase {base}\n\
         BootData_VDPRegs:\n\
         \tdc.b 0\n"
    );
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (bootdata label): {d:?}")).sections
}

/// The synthetic AS-side OUTBOUND consumer — THE BARE-NAME PROOF for both
/// entry points. Mirrors the real callers' shape (`boot.asm`'s
/// `bsr.w VDP_Shadow_Init`; the VBlank path's `Flush_VDP_Shadow` call),
/// assembled through the AS front-end exactly like a real consumer would be.
fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tbsr.w   VDP_Shadow_Init\n\
               \tbsr.w   Flush_VDP_Shadow\n\
               \trts\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}")).sections
}

/// Lower the real `vdp_init.emp` (NO defines: the block is shape-invariant;
/// the shape lives in the map) -> place the `.emp` sections into the
/// per-shape map -> append the FOUR synthetic cross-seam sections (VDP_CTRL
/// equ + VDP RAM labels + per-shape BootData_VDPRegs + outbound consumer) at
/// harness-private LMAs (clear of both map regions) -> ONE `resolve_layout`
/// -> `link`.
fn compile_real_file(
    debug: bool,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let dir = vdp_init_dir();
    let src = std::fs::read_to_string(dir.join("vdp_init.emp"))
        .unwrap_or_else(|e| panic!("cannot read vdp_init.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "vdp_init.emp parse errors: {pdiags:?}"
    );
    // `vdp_init.emp` `use`s the `engine.constants` twin (step 2) — prepend
    // its items (eight `pub const`s + eight drift guards), the ambient
    // technique `controllers_port.rs` documents.
    let constants_src = std::fs::read_to_string(dir.join("constants.emp"))
        .unwrap_or_else(|e| panic!("cannot read constants.emp: {e}"));
    let (constants_file, cdiags) = parse_str(&constants_src);
    assert!(
        cdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "constants.emp parse errors: {cdiags:?}"
    );
    let file = sigil_frontend_emp::ast::File {
        module: file.module.clone(),
        attrs: file.attrs.clone(),
        items: constants_file.items.into_iter().chain(file.items).collect(),
        docs: file.docs.clone(),
    };

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

    let mut equs = as_twin_equs();
    for sec in &mut equs {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(equs);

    let mut ram_labels = as_vdp_ram_labels();
    for sec in &mut ram_labels {
        sec.lma = 0x0200_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(ram_labels);

    // BootData_VDPRegs is a PC-RELATIVE target: its section is `phase`d at
    // the true per-shape VMA, so its LABEL address is already correct — the
    // LMA of the carrier bytes is harness-private like the others.
    let mut bootdata = as_bootdata_label(debug);
    for sec in &mut bootdata {
        sec.lma = 0x0280_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(bootdata);

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

/// The twin's eight drift guards must be captured and PASS against
/// `as_twin_equs`' values (excluding the D2.29 `[layout.odd-item]` parity
/// asserts that also ride `module.link_asserts` — same filter as
/// `controllers_port.rs`'s `guard_assert_count`).
fn assert_twin_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let guards = sigil_harness::test_support::guard_assert_count(link_asserts);
    assert_eq!(guards, 18, "engine.constants's eighteen drift guards must be captured");
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "engine.constants's drift guards must all PASS: {diags:?}"
    );
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

/// Assert the outbound consumer's two `bsr.w` fixups resolve to the
/// per-shape `VDP_Shadow_Init` / `Flush_VDP_Shadow` addresses. The consumer
/// sits at a harness-private LMA far from the ROM-range targets, so the raw
/// disp16 wraps — expected and harmless (modular 16-bit arithmetic on both
/// sides), same as `controllers_port.rs`'s bare-name proof.
fn assert_outbound_consumer(linked: &sigil_link::LinkedImage, init: i64, flush: i64, shape: &str) {
    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == 0x0300_0000)
        .expect("linked image must carry the outbound consumer at its harness-private LMA");
    let disp_init = i16::from_be_bytes([consumer.bytes[2], consumer.bytes[3]]);
    let expected_init = (init - (consumer.lma as i64 + 2)) as i16;
    assert_eq!(
        disp_init, expected_init,
        "bare-name proof: `bsr.w VDP_Shadow_Init` must resolve to {init:#x} ({shape})"
    );
    let disp_flush = i16::from_be_bytes([consumer.bytes[6], consumer.bytes[7]]);
    let expected_flush = (flush - (consumer.lma as i64 + 6)) as i16;
    assert_eq!(
        disp_flush, expected_flush,
        "bare-name proof: `bsr.w Flush_VDP_Shadow` must resolve to {flush:#x} ({shape})"
    );
}

/// (plain) The `vdp_init` section's linked bytes equal
/// `s4.bin[0x1C14..0x1C5C]`, AND both outbound `bsr.w` fixups resolve to the
/// per-shape proc addresses.
#[test]
fn vdp_init_region_matches_reference() {
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
    assert_twin_guards(&resolved, &link_asserts);

    let expected = &refrom[0x1C14..0x1C5C];
    let section = linked.section("vdp_init").expect("linked image must carry vdp_init");
    assert_region_matches(&section.bytes, expected, "vdp_init (plain) vs s4.bin[0x1C14..0x1C5C]");

    assert_outbound_consumer(&linked, 0x1C14, 0x1C2A, "plain");
}

/// (debug) The `vdp_init` section's linked bytes equal
/// `s4.debug.bin[0x1C96..0x1CDE]`, AND both outbound fixups resolve to the
/// per-shape proc addresses.
#[test]
fn vdp_init_debug_region_matches_reference() {
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
    assert_twin_guards(&resolved, &link_asserts);

    let expected = &refrom[0x1C96..0x1CDE];
    let section = linked.section("vdp_init").expect("linked image must carry vdp_init");
    assert_region_matches(&section.bytes, expected, "vdp_init (debug) vs s4.debug.bin[0x1C96..0x1CDE]");

    assert_outbound_consumer(&linked, 0x1C96, 0x1CAC, "debug");
}
