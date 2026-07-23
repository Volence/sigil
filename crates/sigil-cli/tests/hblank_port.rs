//! Port #1 T3 — the REAL `hblank.emp` port, region-level byte gate.
//!
//! `sfx_port.rs`'s sibling for the campaign's first CODE port: compiles the
//! ACTUAL ported file from aeon's tree — `engine/system/hblank.emp` — through
//! the production parse -> lower -> place -> resolve -> link pipeline, and
//! asserts the `hblank` section's flattened bytes equal the reference ROM
//! window at the pinned addresses, in BOTH build shapes.
//!
//! ## The trampoline (t18)
//!
//! The region is now the HBlank RAM jmp-slot trampoline: `HBlank_Install`
//! (arm the RAM slot + enable HInt) and `HBlank_Uninstall` (disarm + disable),
//! replacing the old `HBlank_Dispatch`/`HBlank_Null` ROM dispatch pair. The
//! block's CONTENT is byte-identical plain and debug (`$48` bytes in both);
//! only its BASE address shifts (plain `pins::HBLANK.plain_base`, debug
//! `.debug_base`), so the shape lives entirely in the MAP. `lower_module` runs
//! with an EMPTY `defines` vec for both shapes.
//!
//! ## The cross-seam symbols
//!
//! Two directions:
//!
//! - INBOUND (AS-side RAM the `.emp` reads/writes): `HBlank_Vector_Slot` (the
//!   patched slot at the RAM tail — per-shape VMA, `pins::H_BLANK_VECTOR_SLOT`),
//!   `VDP_Shadow_Table` and `VDP_Dirty_Mask` (the shadow write-through). Each is
//!   supplied by a `phase`d one-byte carrier at its true VMA — the
//!   `parallax_port.rs`/`sfx_port.rs` idiom. The two `VDP_Shadow_*` struct-field
//!   offsets the `.emp` reads back through `extern()` (its drift-lock ensures)
//!   are supplied as `equ` pairs.
//! - OUTBOUND (THE BARE-NAME PROOF): both `pub proc`s must surface as BARE link
//!   symbols. No shipped code calls them yet (the trampoline is preemptive
//!   infra + oracle-driven live-verify), so this synthetic `jsr HBlank_Install`
//!   / `jsr HBlank_Uninstall` consumer — assembled through the AS front-end — is
//!   the proof surface: if the names do not export, the fixups fail to resolve
//!   (or resolve wrong). Install must land at the region base, Uninstall at
//!   `base + HBLANK_UNINSTALL_OFF`.
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
use sigil_harness::pins;
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

/// The map: a `text` region for the module's zero-byte default-section carrier,
/// and the real `hblank` region pinned at the per-shape reference base, sized to
/// the block. Base/size sourced from `pins` (regenerate via `repin`).
fn map_toml(debug: bool) -> String {
    let base = if debug { pins::HBLANK.debug_base } else { pins::HBLANK.plain_base };
    let size = pins::HBLANK.plain_len;
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
         lma_base = {base:#x}\n\
         size = {size:#x}\n\
         kind = \"rom\"\n"
    )
}

/// The two `VDP_Shadow` struct-field offsets `hblank.emp` reads back through
/// `extern()` in its drift-lock ensures (`VDP_MODE1_OFF`/`VDP_HINT_OFF`),
/// supplied as `equ` pairs (parallax_port.rs's `VDP_Shadow_vdp_mode3` idiom).
fn hblank_value_equs() -> Vec<Section> {
    let pairs = [
        ("VDP_Shadow_vdp_mode1", "$00"),
        ("VDP_Shadow_vdp_hint_rate", "$0A"),
    ];
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// The INBOUND cross-seam RAM labels (abs.w operands the `.emp` writes/reads),
/// each a `phase`d one-byte carrier at its true per-shape VMA. Only the slot
/// differs by shape (it lives at the RAM tail, past the `__DEBUG__` block).
fn cross_seam_labels(debug: bool) -> Vec<Section> {
    let slot = if debug { pins::H_BLANK_VECTOR_SLOT.debug } else { pins::H_BLANK_VECTOR_SLOT.plain };
    let labels: [(&str, u32); 3] = [
        ("HBlank_Vector_Slot", slot),
        ("VDP_Shadow_Table", pins::VDP_SHADOW_TABLE.plain),
        ("VDP_Dirty_Mask", pins::VDP_DIRTY_MASK.plain),
    ];
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let mut out = Vec::new();
    for (i, (name, vma)) in labels.iter().enumerate() {
        let asm = format!("cpu 68000\n\tphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        let secs = assemble(&asm, &opts)
            .unwrap_or_else(|d| panic!("AS assemble ({name}): {d:?}"))
            .sections;
        for mut s in secs {
            // Distinct harness-private LMA per carrier (the phased VMA is only
            // for symbol resolution). Clear of the consumer (0x0200_0000) and
            // equs (0x0300_0000).
            s.lma = 0x0400_0000 + (i as u32) * 0x1_0000;
            s.placement = SectionPlacement::Pinned;
            s.group = None;
            out.push(s);
        }
    }
    out
}

/// The synthetic AS-side OUTBOUND consumer — THE BARE-NAME PROOF. A `dc.l` to
/// each `pub proc` (Abs32, width-independent): `dc.l HBlank_Install` at [0..4),
/// `dc.l HBlank_Uninstall` at [4..8). If the names do not surface as BARE link
/// symbols from the `.emp` module, these fixups fail to resolve (or resolve
/// wrong).
fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tdc.l HBlank_Install\n\
               \tdc.l HBlank_Uninstall\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}")).sections
}

/// Parse -> lower (module-dir include_root, NO defines) -> place the `.emp`
/// sections into the per-shape map -> append the field equs + inbound RAM
/// carriers + outbound consumer at harness-private LMAs -> ONE `resolve_layout`
/// -> `link`. Returns the linked image.
fn compile_real_file(debug: bool) -> sigil_link::LinkedImage {
    let dir = hblank_dir();
    let emp_path = dir.join("hblank.emp");
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
        embed_base: None,
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

    // Field-offset equs (comptime, no placement needed) + inbound RAM carriers
    // (their own `phase`d VMAs) + outbound consumer, all at harness-private LMAs
    // clear of both map regions.
    let mut equs = hblank_value_equs();
    for sec in &mut equs {
        sec.lma = 0x0300_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(equs);

    sections.extend(cross_seam_labels(debug));

    let mut consumer = as_outbound_consumer();
    for sec in &mut consumer {
        sec.lma = 0x0200_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
        // Distinct name — the equ/carrier units also emit an unphased `sec0`.
        sec.name = "outbound".to_string();
    }
    sections.extend(consumer);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"))
}

/// On mismatch, report the first differing offset plus context (`sfx_port.rs`
/// byte-diff style).
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

/// Assert the outbound consumer's two `jsr` Abs32 holes resolve to the region
/// base (Install) and `base + HBLANK_UNINSTALL_OFF` (Uninstall) — the bare-name
/// export proof.
fn assert_outbound(linked: &sigil_link::LinkedImage, base: u32) {
    let install = base;
    let uninstall = base + pins::HBLANK_UNINSTALL_OFF as u32;
    let consumer = linked.section("outbound").expect("linked image must carry the outbound consumer");
    assert_eq!(
        &consumer.bytes[0..4],
        &install.to_be_bytes(),
        "bare-name proof: `dc.l HBlank_Install` must resolve to the region base {install:#010x}"
    );
    assert_eq!(
        &consumer.bytes[4..8],
        &uninstall.to_be_bytes(),
        "bare-name proof: `dc.l HBlank_Uninstall` must resolve to base+{:#x} = {uninstall:#010x}",
        pins::HBLANK_UNINSTALL_OFF
    );
}

/// (plain) The `hblank` section's linked bytes equal the plain reference window,
/// AND both `pub proc` names export at the correct per-shape addresses.
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

    let linked = compile_real_file(false);

    let base = pins::HBLANK.plain_base as usize;
    let expected = &refrom[base..base + pins::HBLANK.plain_len];
    let section = linked.section("hblank").expect("linked image must carry hblank");
    assert_region_matches(&section.bytes, expected, "hblank (plain) vs reference window");
    assert_outbound(&linked, pins::HBLANK.plain_base);
}

/// (debug) The `hblank` section's linked bytes equal the debug reference window,
/// AND both `pub proc` names export at the correct per-shape addresses.
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

    let linked = compile_real_file(true);

    let base = pins::HBLANK.debug_base as usize;
    let expected = &refrom[base..base + pins::HBLANK.debug_len];
    let section = linked.section("hblank").expect("linked image must carry hblank");
    assert_region_matches(&section.bytes, expected, "hblank (debug) vs reference window");
    assert_outbound(&linked, pins::HBLANK.debug_base);
}
