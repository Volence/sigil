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
//! Shape-DEPENDENT in BOTH base and length. `pins::PARALLAX` carries all four
//! values (`plain_base`/`debug_base`, `plain_len`/`debug_len`) and `region_base`
//! /`region_len` below pick the pair for the shape under test, so no address or
//! size is written down here — read the pin for today's numbers. The debug shape
//! is the longer of the two: `Parallax_CheckBoundary` guards the sec_effects
//! total-binding contract with a `DEBUG == 1` `raise_error`, whose expansion
//! reaches the `MDDBG__ErrorHandler*` carriers this scope pins in both shapes.
//!
//! ## Cross-seam symbols
//! - RAM labels (abs.w operands): the `Parallax_*` state block, `Camera_X/Y`,
//!   `Current_Act_Ptr`, `Vscroll_Factor`, `Hscroll_*`, `VDP_Shadow_Table`
//!   — each a `phase`d one-byte carrier at its true per-shape VMA. (`VDP_Dirty_Mask`
//!   left this list with the blanket-restore parcel: the mode3 write-throughs are
//!   now shadow-only, so the symbol no longer exists in aeon's RAM map.)
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
use sigil_frontend_emp::lower::{lower_module_with_contracts, LowerOptions};
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
    sigil_harness::test_support::aeon_dir()
}

fn level_dir() -> PathBuf {
    aeon_dir().join("engine/level")
}

/// The reference build's listing for a shape — the `.bin`'s own sibling from the
/// same build, so a symbol's address read here and the operand encoded in the
/// reference ROM window cannot disagree.
fn listing_path(debug: bool) -> PathBuf {
    aeon_dir().join(if debug { "s4.debug.lst" } else { "s4.lst" })
}

/// One `Parallax_*` RAM symbol's VMA in both shapes, read out of the reference
/// listings.
///
/// **These addresses are not written down in this file, and the block's layout is
/// why.** `engine/ram.emp` sizes the parallax state block from capability
/// constants — `MAX_PARALLAX_BANDS` and the four `BAND_*` tails — so a capability
/// flip resizes three arrays at once and moves every symbol below them. A
/// transcribed offset is wrong from that moment, and nothing in a transcription
/// says so: it links, it resolves, and it encodes a wrong `abs.w` operand into a
/// byte gate. The listing is regenerated with the reference tree, so it cannot
/// carry yesterday's layout.
///
/// This does **not** make [`assert_drift_guards`] vacuous by the back door, and
/// the distinction matters. Those guards are `parallax.emp`'s own `ensure`s; with
/// the addresses derived from the same build the guards describe, they pass by
/// construction and are a PRECONDITION here, not evidence — aeon's build is where
/// they do their work. This scope's oracle is the region byte diff against the
/// reference ROM, and that is what checks the derivation: an address derived
/// wrongly changes the operand bytes and fails the diff.
///
/// A missing listing REFUSES by name rather than falling back to a guess.
fn ram_block_vma(name: &'static str) -> (&'static str, u32, u32) {
    let read = |debug: bool| {
        let path = listing_path(debug);
        sigil_harness::test_support::listing_symbol_addr(&path, name).unwrap_or_else(|| {
            panic!(
                "no listing at {} — this gate derives every `Parallax_*` RAM address from \
                 the listing beside the reference ROM, so a source-only checkout cannot \
                 serve it. Point AEON_DIR at a tree with all four shapes built.",
                path.display()
            )
        })
    };
    (name, read(false), read(true))
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
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
    // VDP_Shadow_Table cell below lives BEFORE the deleted pair, so it holds. (It had
    // a VDP_Dirty_Mask sibling until the blanket-restore parcel deleted that symbol.)
    // Camera_X/Y now pin-sourced (they carry the −4 too; matches Current_Act_Ptr style).
    let table: [(&str, u32, u32); 35] = [
        // The MD Debugger carriers the DEBUG-shape asserts jsr/jmp (the section_port /
        // sprites_port precedent). Shape-invariant pins carried in BOTH shapes: in plain
        // the assert is comptime-gated out and these simply go unreferenced.
        //
        // Needed here as of Parcel C2: deleting the three per-field installers moved this
        // region's extent, and the code now inside it carries a DEBUG-only assert.
        (
            "MDDBG__ErrorHandler",
            pins::MDDBG_ERROR_HANDLER,
            pins::MDDBG_ERROR_HANDLER,
        ),
        (
            "MDDBG__ErrorHandler_PagesController",
            pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER,
            pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER,
        ),
        // Effects P3 Parcel C2: the crossing's three per-field consumers
        // (Palette_LoadSection / Raster_InstallSection / Palette_InstallCycleSection)
        // are GONE, and their pins with them. A section now names ONE EffectsPreset and
        // Effects_InstallPreset writes every channel — total binding — so this scope
        // supplies that single target instead of three.
        //
        // Their three `pins::*` constants were removed by `repin` the moment the procs
        // stopped existing, which is what broke this file: a deleted symbol takes its
        // pin with it, and any test scope still naming it fails to COMPILE. That is the
        // mirror of the usual port-flip trap (a NEW cross-seam ref failing to link).
        (
            "Effects_InstallPreset",
            pins::EFFECTS_INSTALL_PRESET.plain,
            pins::EFFECTS_INSTALL_PRESET.debug,
        ),
        // The `Parallax_*` RAM state block, every address read from the reference
        // build's own listing — see `ram_block_vma`, which carries the reason. The
        // block is capability-sized in `engine/ram.emp`
        // (`104 + (28 + 4 * BAND_DRIFT_N) * MAX_PARALLAX_BANDS` at the shipped set),
        // so its interior is not a constant this file can hold; three of its arrays
        // resize together and the symbols below them all move.
        //
        // `parallax.emp` pins the last three by ADJACENT-SYMBOL DIFFERENCE
        // (`Curve_Carry - Drift_Acc`, `Shadow_Bands - Curve_Carry`,
        // `Shadow_Scroll_A - Shadow_Bands`) so a short reservation names itself, and
        // `ram.emp` records that inserting a field between any two of those three
        // breaks the pin it splits. Deriving the addresses keeps this scope agreeing
        // with that layout by construction instead of by transcription.
        ram_block_vma("Parallax_State"),
        ram_block_vma("Parallax_State_End"),
        ram_block_vma("Parallax_Current_Config"),
        ram_block_vma("Parallax_Target_Config"),
        ram_block_vma("Parallax_Transition_Frames"),
        ram_block_vma("Parallax_Snap_Pending"),
        ram_block_vma("Parallax_Prev_Sec_X"),
        ram_block_vma("Parallax_Prev_Sec_Y"),
        ram_block_vma("Parallax_Current_Scroll_A"),
        ram_block_vma("Parallax_Current_Scroll_B"),
        ram_block_vma("Parallax_Current_Vscroll_BG"),
        ram_block_vma("Parallax_Deform_Phase_FG"),
        ram_block_vma("Parallax_Deform_Phase_BG"),
        ram_block_vma("Parallax_V_Deform_Phase_BG"),
        ram_block_vma("Parallax_Vscroll_Column_Buf"),
        ram_block_vma("Parallax_Drift_Acc"),
        ram_block_vma("Parallax_Curve_Carry"),
        ram_block_vma("Parallax_Shadow_Bands"),
        ram_block_vma("Parallax_Shadow_Scroll_A"),
        ram_block_vma("Parallax_Shadow_Scroll_B"),
        ("Camera_X", pins::CAMERA_X.plain, pins::CAMERA_X.debug),
        ("Camera_Y", pins::CAMERA_Y.plain, pins::CAMERA_Y.debug),
        // Sourced from pins — this RAM cell rides the tail of the shifting RAM map
        // (the F-2 camera-ceiling insert pushed it +4), so a hand-typed VMA goes stale.
        ("Current_Act_Ptr", pins::CURRENT_ACT_PTR.plain, pins::CURRENT_ACT_PTR.debug),
        ("Vscroll_Factor", pins::VSCROLL_FACTOR.plain, pins::VSCROLL_FACTOR.debug),
        ("Hscroll_Buffer", pins::HSCROLL_BUFFER.plain, pins::HSCROLL_BUFFER.debug),
        // Pin-sourced (t24 rule — never hand-shift a RAM VMA): slid +4 in I2
        // (input/replay, 2026-08-02) via the Logic_Tick u32 after Frame_Counter
        // ($800A → $800E). Shape-invariant engine RAM.
        ("VDP_Shadow_Table", pins::VDP_SHADOW_TABLE.plain, pins::VDP_SHADOW_TABLE.debug),
        // ROM transfer target — sourced from pins (shifts with the engine bank;
        // t18 trampoline moved it +0x36). RAM symbols above are tail/pad-stable.
        ("Section_GetSecPtrXY", pins::SECTION_GET_SEC_PTR_XY.plain, pins::SECTION_GET_SEC_PTR_XY.debug),
        // Parcel W — the world-anchored deform overlay (Step 4b). TWO new cross-seam
        // references, and both had to be declared here or this gate stops resolving:
        //   Effects_World_Y        the anchor bank the overlay READS, owned by engine.ram
        //                          and shared with the raster patcher — the whole point of
        //                          the parcel is that it has two readers.
        //   Raster_GetChannelBand  the outbound call that fetches the channel's authored
        //                          clamp band, so this side pins where Raster_PatchAll pins.
        ("Effects_World_Y", pins::EFFECTS_WORLD_Y.plain, pins::EFFECTS_WORLD_Y.debug),
        // The off-screen ship parcel: the overlay stopped deriving `anchor - Camera_Y` and now
        // READS the latch, so this is the cross-seam reference that replaces that arithmetic.
        // Effects_World_Y stays declared above — parallax no longer reads it, but the pin
        // documents the bank the latch is derived from and costs nothing.
        ("Effects_Screen_L", pins::EFFECTS_SCREEN_L.plain, pins::EFFECTS_SCREEN_L.debug),
        ("Raster_GetChannelBand", pins::RASTER_GET_CHANNEL_BAND.plain, pins::RASTER_GET_CHANNEL_BAND.debug),
        // Item 7, the vertical-bob overlay: TWO new cross-seam references, and they are
        // declared DIFFERENTLY on purpose. The test is not "does a pin exist" but "is this
        // address invariant" — pin what is absolute, derive what is derived.
        //
        //   Logic_Tick   RAM. 0xFFFF8004 in BOTH shapes, which is the tell: RAM does not
        //                move when ROM grows, so an absolute pin is the honest form and
        //                stays true across every byte-mover. Sourced from pins per the t24
        //                rule (never hand-shift a RAM VMA) — it already rides the shifting
        //                RAM map, having slid +4 in I2 behind Frame_Counter.
        ("Logic_Tick", pins::LOGIC_TICK.plain, pins::LOGIC_TICK.debug),
        //   Sine_Table   ROM data, and therefore NOT given an absolute pin. It is DERIVED
        //                as the math section's base plus a module-internal distance:
        //                SINE_TABLE_OFF (0x18) is the 24-byte code body of GetSineCosine,
        //                which is the first proc in the section (math_port.rs:328 proves
        //                that offset from both built listings). The offset is a fact about
        //                math.emp's own layout and survives every parcel that moves where
        //                the section lands; an absolute pin would be a derived quantity
        //                minted as an independent one, needing a repin each time anything
        //                upstream grew — chain 190 moved 204/207 symbols at +32 and would
        //                have invalidated it silently.
        //                Base is the REGION base, not GET_SINE_COSINE: numerically equal
        //                today, but that equality holds only because GetSineCosine happens
        //                to be first, and SINE_TABLE_OFF is defined from the section base.
        //                Derivation checked against the item-7 build: 0x2850 + 0x18 = 0x2868,
        //                which is where the aeon lane measured Sine_Table.
        (
            "Sine_Table",
            pins::MATH.plain_base + pins::SINE_TABLE_OFF as u32,
            pins::MATH.debug_base + pins::SINE_TABLE_OFF as u32,
        ),
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
    // The SYNTHESIZED `CAP_*` block standing in for
    // `use engine.level.scene_dsl.{CAP_ANCHORS, …}`: a single-module lower has no module
    // to follow, and the bit values are read out of scene_dsl.emp at test runtime
    // (test_support §4) so this gate can never bind a stale mask. (CAP_PER_LINE was
    // retired with the per-cell HScroll path, aeon 55ea2557 / d-29-corrected; bit 0 is
    // a hole and the enumerator simply does not find it.)
    let caps_src = sigil_harness::test_support::scene_dsl_cap_consts_src(&aeon_dir());
    let (caps_file, caps_diags) = parse_str(&caps_src);
    assert!(
        caps_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "synthesized CAP_* block parse errors: {caps_diags:?}"
    );
    // (Until aeon 55ea2557 this shim also spliced scene_dsl.emp's `parallax_mode_key`
    // comptime fn, the runtime per-line/per-cell mode key that chose Parallax_Fill_PerCell.
    // The per-cell fill and the fn went with owner ruling d-29-corrected — one fill, one
    // DMA length, one hardware mode — so parallax.emp no longer imports it and nothing is
    // spliced.)
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
            .chain(caps_file.items)
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
    // The game-contract env: parallax.emp gates several blocks on
    // `Game.SCANLINE_CAPS & CAP_*`, which the whole-program bind pass resolves and a
    // single-module lower does not. Bound to SONIC4's declared mask, read from its
    // game.emp — the reference windows are sonic4-shaped, so any other binding would
    // compare a specialisation the reference never took.
    let contracts = sigil_harness::test_support::scanline_caps_contract_env(&aeon_dir());
    let (module, ldiags) = lower_module_with_contracts(&file, &opts, &contracts);
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
