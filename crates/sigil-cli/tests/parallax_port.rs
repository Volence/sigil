//! Tranche 18 — the REAL `parallax.emp` port, region-level byte gate.
//!
//! Compiles the actual ported file — `engine/level/parallax.emp` — through the
//! production parse -> lower -> place -> resolve -> link pipeline and asserts the
//! `parallax` region's flattened bytes equal the reference ROM window at the
//! pinned base, in BOTH build shapes. The §4.6 parallax pipeline: per-frame band
//! lerp + HScroll buffer fill (per-cell / per-line deform) + whole-plane /
//! per-column Vscroll.
//!
//! ## Shape
//! SHAPE-INVARIANT length ($556 both shapes — parallax.asm has NO `__DEBUG__`
//! code, no asserts), like section/plane_buffer; only the base shifts (plain
//! `$5B02`, debug `$678C`).
//!
//! ## Cross-seam symbols
//! - RAM labels (abs.w operands): the `Parallax_*` state block, `Camera_X/Y`,
//!   `Current_Act_Ptr`, `Vscroll_Factor`, `Hscroll_*`, `VDP_Shadow_Table`,
//!   `VDP_Dirty_Mask` — each a `phase`d one-byte carrier at its true per-shape VMA.
//! - ROM transfer target: `Section_GetSecPtrXY` (the one cross-module `jsr`; a NEW
//!   caller of section.emp's already-owned symbol — a standard cross-module link
//!   resolution, NOT an ownership flip).
//! - `engine.constants`/`engine.structs`/`engine.vdp` twins ride the ambient
//!   prepend; their drift guards ride this gate.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test parallax_port
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
    if debug { pins::PARALLAX.debug_base } else { pins::PARALLAX.plain_base }
}

fn region_len(debug: bool) -> usize {
    if debug { pins::PARALLAX.debug_len } else { pins::PARALLAX.plain_len }
}

fn aeon_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    PathBuf::from(aeon)
}

fn level_dir() -> PathBuf {
    aeon_dir().join("engine/level")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The map: a `text` carrier for the zero-byte default section, and the
/// `parallax` region pinned at the per-shape reference base + length.
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
         name = \"parallax\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// parallax.emp's OWN mirrored constants + the struct-field/size externs its drift
/// wall reads back through `extern()`. `doctor` overrides ONE pair (the negative
/// probe). SCREEN_*/SECTION_SIZE_SHIFT + Act_*/Sec_* + VDP target/op bits ride the
/// prepended twins via `engine_constant_equs` / `act_sec_field_equs`.
fn parallax_value_equs(doctor: Option<(&str, &str)>) -> Vec<Section> {
    let mut pairs: Vec<(&str, &str)> = vec![
        // parallax.emp local mirrors
        ("PARALLAX_TRANS_DEFAULT", "16"),
        ("PARALLAX_LERP_SHIFT", "4"),
        ("VDP_DATA", "$C00000"),
        ("VDP_CTRL", "$C00004"),
        ("VDP_Shadow_vdp_mode3", "$0B"),
        // engine.z80_bus — the per-frame Mode Set 3 direct-write wraps the reg $0B
        // write in stop_z80/start_z80; the templates resolve this bare hw address.
        ("Z80_BUS_REQUEST", "$A11100"),
        // engine.vdp target_bits/op_bits drift-lock ensures read these six
        ("VRAM", "%100001"),
        ("CRAM", "%101011"),
        ("VSRAM", "%100101"),
        ("READ", "%001100"),
        ("WRITE", "%000111"),
        ("DMA", "%100111"),
        // VDP_Shadow offset twins (engine.vdp shadow-offset block)
        ("VDP_Shadow_vdp_mode1", "$00"),
        ("VDP_Shadow_vdp_mode2", "$01"),
        ("VDP_Shadow_vdp_hint_rate", "$0A"),
        // band_entry struct (10 bytes) — the .emp's per-field drift wall
        ("band_entry_len", "10"),
        ("band_entry_band_top_cell", "0"),
        ("band_entry_band_factor_a_s1", "1"),
        ("band_entry_band_factor_a_s2", "2"),
        ("band_entry_band_factor_a_op", "3"),
        ("band_entry_band_factor_b_s1", "4"),
        ("band_entry_band_factor_b_s2", "5"),
        ("band_entry_band_factor_b_op", "6"),
        ("band_entry_band_deform_shift_a", "7"),
        ("band_entry_band_deform_shift_b", "8"),
        ("band_entry_band_phase_offset", "9"),
        // parallax_config pairs come from act_sec_field_equs (the struct moved
        // to engine.structs at the tranche-21 buffers port — 2nd consumer).
    ];
    if let Some((name, val)) = doctor {
        for p in pairs.iter_mut() {
            if p.0 == name {
                p.1 = val;
            }
        }
    }
    // shared-twin values: engine.constants (incl. SCREEN_*/SECTION_SIZE_SHIFT) feed
    // the prepended constants.emp drift wall; Act_*/Sec_* feed structs.emp's wall.
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    pairs.extend(sigil_harness::test_support::act_sec_field_equs());
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// The cross-seam ADDRESS symbols — the `Parallax_*` RAM state block + shared RAM
/// (`Camera_*`, `Current_Act_Ptr`, `Vscroll_Factor`, `Hscroll_*`, `VDP_*`) plus the
/// one ROM transfer target `Section_GetSecPtrXY` — each a `phase`d one-byte carrier
/// at its true per-shape VMA (label position selects abs.w/abs.l width and the
/// low-word bytes; `Section_GetSecPtrXY`'s VMA fixes the `jsr` disp).
fn parallax_addr_labels(debug: bool) -> Vec<Section> {
    // (name, plain VMA, debug VMA) — RAM mostly shape-invariant; Camera_*/
    // Current_Act_Ptr live in a debug-shifted RAM region; Section_GetSecPtrXY is ROM.
    // PAL NTSC-only (ruling B, 2026-08-02): the Timing_Step/Frame_Accumulator u16
    // pair at $FFFF8028 was deleted, so every RAM symbol at/after $FFFF802C slid
    // −4 (the Parallax_* block, Camera_*, Vscroll_Factor, Hscroll_Buffer). The two
    // VDP_* cells below live at $800A/$801E — BEFORE the deleted pair — so they hold.
    // Camera_X/Y now pin-sourced (they carry the −4 too; matches Current_Act_Ptr style).
    let table: [(&str, u32, u32); 29] = [
        // Effects P1: Parallax_CheckBoundary is the one place the engine notices a
        // section crossing, so the palette and raster consumers hang off it — two more
        // cross-module transfer targets (buffers / hblank).
        ("Palette_LoadSection", pins::PALETTE_LOAD_SECTION.plain, pins::PALETTE_LOAD_SECTION.debug),
        ("Raster_InstallSection", pins::RASTER_INSTALL_SECTION.plain, pins::RASTER_INSTALL_SECTION.debug),
        // Effects P2 Task 8: the same crossing now also consumes Sec.sec_pal_cycle, so
        // per-section palette cycling is a third consumer hanging off the boundary.
        (
            "Palette_InstallCycleSection",
            pins::PALETTE_INSTALL_CYCLE_SECTION.plain,
            pins::PALETTE_INSTALL_CYCLE_SECTION.debug,
        ),
        // Pin-sourced base + ram.emp-mirror intra-block offsets (input-6button:
        // the third hand-shift of this table killed the literal class — the t24
        // rot rule; the offsets are the block layout this test pins down anyway).
        ("Parallax_State", pins::PARALLAX_STATE.plain, pins::PARALLAX_STATE.debug),
        ("Parallax_State_End", pins::PARALLAX_STATE.plain + 0xF4, pins::PARALLAX_STATE.debug + 0xF4),
        ("Parallax_Current_Config", pins::PARALLAX_STATE.plain + 0x28, pins::PARALLAX_STATE.debug + 0x28),
        ("Parallax_Target_Config", pins::PARALLAX_STATE.plain + 0x2C, pins::PARALLAX_STATE.debug + 0x2C),
        ("Parallax_Transition_Frames", pins::PARALLAX_STATE.plain + 0x30, pins::PARALLAX_STATE.debug + 0x30),
        ("Parallax_Snap_Pending", pins::PARALLAX_STATE.plain + 0x31, pins::PARALLAX_STATE.debug + 0x31),
        ("Parallax_Prev_Sec_X", pins::PARALLAX_STATE.plain + 0x32, pins::PARALLAX_STATE.debug + 0x32),
        ("Parallax_Prev_Sec_Y", pins::PARALLAX_STATE.plain + 0x33, pins::PARALLAX_STATE.debug + 0x33),
        ("Parallax_Current_Scroll_A", pins::PARALLAX_STATE.plain + 0x06, pins::PARALLAX_STATE.debug + 0x06),
        ("Parallax_Current_Scroll_B", pins::PARALLAX_STATE.plain + 0x16, pins::PARALLAX_STATE.debug + 0x16),
        ("Parallax_Current_Vscroll_BG", pins::PARALLAX_STATE.plain + 0x26, pins::PARALLAX_STATE.debug + 0x26),
        ("Parallax_Deform_Phase_FG", pins::PARALLAX_STATE.plain, pins::PARALLAX_STATE.debug),
        ("Parallax_Deform_Phase_BG", pins::PARALLAX_STATE.plain + 0x02, pins::PARALLAX_STATE.debug + 0x02),
        ("Parallax_V_Deform_Phase_BG", pins::PARALLAX_STATE.plain + 0x04, pins::PARALLAX_STATE.debug + 0x04),
        ("Parallax_Vscroll_Column_Buf", pins::PARALLAX_STATE.plain + 0x34, pins::PARALLAX_STATE.debug + 0x34),
        ("Parallax_Shadow_Bands", pins::PARALLAX_STATE.plain + 0x84, pins::PARALLAX_STATE.debug + 0x84),
        ("Parallax_Shadow_Scroll_A", pins::PARALLAX_STATE.plain + 0xD4, pins::PARALLAX_STATE.debug + 0xD4),
        ("Parallax_Shadow_Scroll_B", pins::PARALLAX_STATE.plain + 0xE4, pins::PARALLAX_STATE.debug + 0xE4),
        ("Camera_X", pins::CAMERA_X.plain, pins::CAMERA_X.debug),
        ("Camera_Y", pins::CAMERA_Y.plain, pins::CAMERA_Y.debug),
        // Sourced from pins — this RAM cell rides the tail of the shifting RAM map
        // (the F-2 camera-ceiling insert pushed it +4), so a hand-typed VMA goes stale.
        ("Current_Act_Ptr", pins::CURRENT_ACT_PTR.plain, pins::CURRENT_ACT_PTR.debug),
        ("Vscroll_Factor", pins::VSCROLL_FACTOR.plain, pins::VSCROLL_FACTOR.debug),
        ("Hscroll_Buffer", pins::HSCROLL_BUFFER.plain, pins::HSCROLL_BUFFER.debug),
        // Pin-sourced (t24 rule — never hand-shift a RAM VMA): both slid +4 in I2
        // (input/replay, 2026-08-02) via the Logic_Tick u32 after Frame_Counter
        // ($800A/$801E → $800E/$8022). Shape-invariant engine RAM.
        ("VDP_Shadow_Table", pins::VDP_SHADOW_TABLE.plain, pins::VDP_SHADOW_TABLE.debug),
        ("VDP_Dirty_Mask", pins::VDP_DIRTY_MASK.plain, pins::VDP_DIRTY_MASK.debug),
        // ROM transfer target — sourced from pins (shifts with the engine bank;
        // t18 trampoline moved it +0x36). RAM symbols above are tail/pad-stable.
        ("Section_GetSecPtrXY", pins::SECTION_GET_SEC_PTR_XY.plain, pins::SECTION_GET_SEC_PTR_XY.debug),
    ];
    let mut out = Vec::new();
    for (i, (name, plain, dbg)) in table.iter().enumerate() {
        let vma = if debug { *dbg } else { *plain };
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

/// Lower the real `parallax.emp` (prepend the `engine.constants` twin +
/// `engine.structs` + `engine.vdp`), place into the per-shape map, append the value
/// equs + cross-seam address labels, one `resolve_layout` -> `link`.
fn compile_real_file(
    debug: bool,
    doctor: Option<(&str, &str)>,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let dir = level_dir();
    let main = parse_file(&dir.join("parallax.emp"));
    let constants_file = parse_file(&dir.parent().unwrap().join("system/constants.emp"));
    let structs_file = parse_file(&dir.parent().unwrap().join("structs.emp"));
    let vdp_file = parse_file(&dir.parent().unwrap().join("vdp.emp"));
    let z80_bus_file = parse_file(&dir.parent().unwrap().join("z80_bus.emp"));
    let irq_file = parse_file(&dir.parent().unwrap().join("irq.emp"));
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items: constants_file
            .items
            .into_iter()
            .chain(structs_file.items)
            .chain(vdp_file.items)
            .chain(z80_bus_file.items)
            .chain(irq_file.items)
            .chain(main.items)
            .collect(),
        docs: main.docs.clone(),
    };

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), i128::from(debug))],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "parallax.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut equs = parallax_value_equs(doctor);
    for sec in &mut equs {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(equs);

    sections.extend(parallax_addr_labels(debug));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

/// parallax.emp's drift guards + the prepended twins' guards must PASS.
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "parallax.emp drift guards must all PASS: {diags:?}"
    );
}

fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
    // A gate over an EMPTY image proves nothing, and the tolerance below would
    // hide that: with no candidate bytes it shrinks `expected` to zero length, the
    // length assert compares 0 == 0, and the diff loop runs over an empty range —
    // so the test passes if the module emits nothing at all. Confirmed live on
    // OJZ_BG_ANIM, a 14-byte all-zero plain window (lens sweep, seat GATE, S15).
    assert!(
        !candidate.is_empty(),
        "{what}: the module emitted NO BYTES — a region gate over an empty window \
         proves nothing. Either the module stopped emitting, or this pin should not exist."
    );
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
    assert_drift_guards(&resolved, &link_asserts);

    let base = region_base(debug) as usize;
    let expected = &refrom[base..base + region_len(debug)];
    let section = linked.section("parallax").expect("linked image must carry parallax");
    let shape = if debug { "debug" } else { "plain" };
    assert_region_matches(&section.bytes, expected, &format!("parallax ({shape})"));
}

#[test]
fn parallax_region_matches_reference() {
    run(false);
}

#[test]
fn parallax_debug_region_matches_reference() {
    run(true);
}

// `doctored_parallax_lerp_shift_fires_its_guard` RETIRED at the conv-b constants-tail
// flip: PARALLAX_LERP_SHIFT flipped from parallax.emp's local mirror to `use engine.constants`,
// so no `ensure(extern("PARALLAX_LERP_SHIFT") == …)` wall survives for the doctored probe to
// fire. Its protection re-homes to the six-target byte-identity gate.
