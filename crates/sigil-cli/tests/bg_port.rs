//! Tranche 19 — the REAL `bg.emp` port, region-level byte gate.
//!
//! Compiles the actual ported file — `engine/level/bg.emp` — through the
//! production parse -> lower -> place -> resolve -> link pipeline and asserts
//! the `bg` region's flattened bytes equal the reference ROM window at the
//! pinned base, in BOTH build shapes. BG_Init (§2 A.5): level-load blocking
//! copies of the act BG tile blob + Plane B nametable through the VDP data
//! port, under an SR interrupt mask + Z80 bus hold.
//!
//! ## Shape
//! Both the base and the LENGTH are shape-dependent, so each shape diffs
//! against its own ROM window. The four numbers live in
//! `pins::BG.{plain,debug}_{base,len}` — regenerate via `repin` — and are
//! deliberately not restated here: a bound copied into prose is executed by
//! nothing, so nothing can go red when it rots.
//!
//! ## Cross-seam symbols
//! - No RAM labels at all — BG_Init reads only the Act descriptor (via a3)
//!   and hardware ports. `Z80_BUS_REQUEST` is a bare link-resolved hardware
//!   address (truth: engine/constants.asm), supplied as an equ.
//! - `engine.structs`/`engine.vdp` twins ride the ambient prepend; their
//!   drift guards ride this gate.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test bg_port
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
    if debug { pins::BG.debug_base } else { pins::BG.plain_base }
}

fn region_len(debug: bool) -> usize {
    if debug { pins::BG.debug_len } else { pins::BG.plain_len }
}

fn aeon_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    PathBuf::from(aeon)
}

fn level_dir() -> PathBuf {
    aeon_dir().join("engine/level")
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}

/// The map: a `text` carrier for the zero-byte default section, and the
/// `bg` region pinned at the per-shape reference base + length.
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
         name = \"bg\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// bg.emp's OWN mirrored constants + the engine.vdp bit vocabulary + the
/// Z80 bus port. `doctor` overrides ONE pair (the negative probe). Act field
/// offsets ride the prepended structs twin via `act_sec_field_equs`.
fn bg_value_equs(doctor: Option<(&str, &str)>) -> Vec<Section> {
    let mut pairs: Vec<(&str, &str)> = vec![
        // bg.emp local mirrors (truth: engine/constants.asm).
        // VRAM_SPRITE_TABLE moved to engine_constant_equs at the t21 hoist.
        ("BG_TILE_BASE_VRAM", "$8000"),
        ("VDP_DATA", "$C00000"),
        ("VDP_CTRL", "$C00004"),
        // bare link-resolved hardware address (stop_z80/start_z80 templates)
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
        ("VDP_Shadow_vdp_mode3", "$0B"),
        ("VDP_Shadow_vdp_hint_rate", "$0A"),
    ];
    pairs.extend(sigil_harness::test_support::act_sec_field_equs());
    // The prepended constants.emp twin (t21: VRAM_SPRITE_TABLE hoisted there)
    // brings its full drift wall — supply the whole shared truth blob. The
    // doctor runs AFTER the extends so the negative probe can doctor shared
    // pairs too (VRAM_SPRITE_TABLE now lives in engine_constant_equs).
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    if let Some((name, val)) = doctor {
        for p in pairs.iter_mut() {
            if p.0 == name {
                p.1 = val;
            }
        }
    }
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// The cross-seam ADDRESS labels — the MD Debugger entry points only. bg.emp was a
/// pure-equ consumer until the blanket-restore parcel gave BG_Init a DEBUG-shape
/// `IPL >= 6` assert around its autoincrement excursion; `assert.w` expands to
/// `jsr (MDDBG__ErrorHandler).l` / `jmp (MDDBG__ErrorHandler_PagesController).l`, so
/// the two blob entry points become real cross-seam fixups (sprites_port precedent).
/// The pins are shape-invariant; the plain shape comptime-gates the assert away and
/// simply leaves the carriers unreferenced.
fn bg_addr_labels() -> Vec<Section> {
    let table: [(&str, u32); 2] = [
        ("MDDBG__ErrorHandler", pins::MDDBG_ERROR_HANDLER),
        ("MDDBG__ErrorHandler_PagesController", pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER),
    ];
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let mut out = Vec::new();
    for (i, (name, vma)) in table.iter().enumerate() {
        let asm = format!("cpu 68000\n\tphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        for mut sec in assemble(&asm, &opts)
            .unwrap_or_else(|d| panic!("AS assemble ({name}): {d:?}"))
            .sections
        {
            sec.lma = 0x0200_0000 + (i as u32) * 0x1_0000;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
            out.push(sec);
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

/// Lower the real `bg.emp` (prepend `engine.structs` + `engine.vdp`), place
/// into the per-shape map, append the value equs, one `resolve_layout` ->
/// `link`.
fn compile_real_file(
    debug: bool,
    doctor: Option<(&str, &str)>,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let dir = level_dir();
    let main = parse_file(&dir.join("bg.emp"));
    let structs_file = parse_file(&dir.parent().unwrap().join("structs.emp"));
    let vdp_file = parse_file(&dir.parent().unwrap().join("vdp.emp"));
    let z80_bus_file = parse_file(&dir.parent().unwrap().join("z80_bus.emp"));
    // engine.constants: VRAM_SPRITE_TABLE hoisted to the shared twin at the
    // t21 buffers port (bg was its file-local first consumer).
    let consts_file = parse_file(&dir.parent().unwrap().join("system/constants.emp"));
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items: structs_file
            .items
            .into_iter()
            .chain(vdp_file.items)
            .chain(z80_bus_file.items)
            .chain(consts_file.items)
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
        "bg.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut equs = bg_value_equs(doctor);
    for sec in &mut equs {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(equs);
    sections.extend(bg_addr_labels());

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

/// bg.emp's drift guards + the prepended twins' guards must PASS.
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "bg.emp drift guards must all PASS: {diags:?}"
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
    let section = linked.section("bg").expect("linked image must carry bg");
    let shape = if debug { "debug" } else { "plain" };
    assert_region_matches(&section.bytes, expected, &format!("bg ({shape})"));
}

#[test]
fn bg_region_matches_reference() {
    run(false);
}

#[test]
fn bg_debug_region_matches_reference() {
    run(true);
}

// The `doctored_vram_sprite_table_fires_its_guard` negative probe retired with
// the Stage-3 P5 ownership flip: `VRAM_SPRITE_TABLE` is now SOLE-authored by
// `constants.emp` (harvested into guarded AS defines), so its mirror drift guard
// was deleted — nothing for a doctored AS-side truth to fire. The undoctored
// reference gates above remain the proof the value is load-bearing.
