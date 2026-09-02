//! Tranche 29 — game-side G1: the trivial display/animate objects. The REAL
//! `test_static.emp` + `test_animated.emp` ports, region-level byte gates in
//! BOTH shapes, modelled on `test_objects_port.rs` (the shipped
//! test_solid/test_particle template).
//!
//! ## What this port opens
//!
//! - **Two per-file object-bank gates** (`SIGIL_EMP_TEST_STATIC` /
//!   `SIGIL_EMP_TEST_ANIMATED`) with SHARED anchor `TestAnimated`
//!   (test_static's end = test_animated's start). Bases are shape-invariant
//!   (bank not slid); content bytes move per shape (cross-seam operands).
//! - **The first aeon-corpus `vars` SST-overlay consumer** — test_animated's
//!   `DplcV: Sst.sst_custom` overlay (`DplcV.dplc_ptr(a0)` = `$2E(a0)`), whose
//!   always-on window-overflow check replaces the AS `objvarsCheck`.
//! - **`vram_art` adoption** (objdef.emp's comptime fn, ambient-prepended, the
//!   rings_port technique) + the new file-local `vram_bytes` comptime fn.
//! - **Zero externs**: Draw_Sprite / AnimateSprite / Perform_DPLC and the four
//!   Sonic data pointers are bare link symbols, resolved by the shared link.
//!
//! ## Compile technique — the `test_objects_port.rs` ambient injection: each
//! module lowers as ONE synthetic `ast::File` with its `use`-deps' items
//! prepended (types + sst for test_static; + constants + objdef for
//! test_animated, objdef being vram_art's home). Cross-seam symbols are
//! synthetic AS-side sections at the pinned per-shape VMAs.
//!
//! REFERENCE-DEPENDENT (`AEON_DIR`, default sibling). Absent, tests SKIP green
//! unless `SIGIL_STRICT_GATE=1`.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    sigil_harness::test_support::aeon_dir()
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}

/// The engine-constants twin's guard count (test_particle's shared derivation).
fn twin_guards() -> usize {
    sigil_harness::test_support::engine_constant_equs().len()
}

const OBJ_CODE_BASE: u32 = pins::OBJ_CODE_BASE.plain;

/// Per-shape cross-seam VMAs + region geometry (shape-invariant bases, but the
/// operand bytes track the per-shape engine/data targets, so we compile twice).
struct Shape {
    draw_sprite: u32,
    animate_sprite: u32,
    perform_dplc: u32,
    map_sonic: u32,
    ani_sonic: u32,
    dplc_sonic: u32,
    art_sonic: u32,
    static_base: u32,
    static_len: usize,
    animated_base: u32,
    animated_len: usize,
}

const PLAIN: Shape = Shape {
    draw_sprite: pins::DRAW_SPRITE.plain,
    animate_sprite: pins::ANIMATE.plain_base,
    perform_dplc: pins::DPLC.plain_base,
    map_sonic: pins::MAP_SONIC.plain,
    ani_sonic: pins::SONIC_ANIMS.plain_base,
    dplc_sonic: pins::DPLC_SONIC.plain,
    art_sonic: pins::ART_SONIC.plain,
    static_base: pins::TEST_STATIC.plain_base,
    static_len: pins::TEST_STATIC.plain_len,
    animated_base: pins::TEST_ANIMATED.plain_base,
    animated_len: pins::TEST_ANIMATED.plain_len,
};
const DEBUG: Shape = Shape {
    draw_sprite: pins::DRAW_SPRITE.debug,
    animate_sprite: pins::ANIMATE.debug_base,
    perform_dplc: pins::DPLC.debug_base,
    map_sonic: pins::MAP_SONIC.debug,
    ani_sonic: pins::SONIC_ANIMS.debug_base,
    dplc_sonic: pins::DPLC_SONIC.debug,
    art_sonic: pins::ART_SONIC.debug,
    static_base: pins::TEST_STATIC.debug_base,
    static_len: pins::TEST_STATIC.debug_len,
    animated_base: pins::TEST_ANIMATED.debug_base,
    animated_len: pins::TEST_ANIMATED.debug_len,
};

fn parse_file(path: &std::path::Path) -> sigil_frontend_emp::ast::File {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (file, diags) = parse_str(&src);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} parse errors: {diags:?}",
        path.display()
    );
    file
}

fn with_ambient(
    deps: Vec<sigil_frontend_emp::ast::File>,
    main: sigil_frontend_emp::ast::File,
) -> sigil_frontend_emp::ast::File {
    let mut items = Vec::new();
    for d in deps {
        items.extend(d.items);
    }
    items.extend(main.items);
    sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items,
        docs: main.docs.clone(),
    }
}

/// The AS-side value seam: SST field equs + engine constants + ObjCodeBase +
/// the game-config VRAM_TEST_SONIC + the DplcV overlay equs test_animated's
/// drift guards read.
fn as_constant_equs() -> Vec<Section> {
    use sigil_harness::test_support::{engine_constant_equs, sst_field_equs};
    let mut pairs = sst_field_equs();
    pairs.extend(engine_constant_equs());
    let obj_code_base = format!("${:X}", OBJ_CODE_BASE);
    pairs.push(("ObjCodeBase", obj_code_base.as_str()));
    pairs.push(("VRAM_TEST_SONIC", "$03C0"));
    // (test_animated's _dplc_ptr/_art_base overlay guards retired at conv-d #48 —
    // test_animated.emp owns the DplcV overlay, no extern mirror.)
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// One synthetic AS-side label phased at `vma`.
fn as_label_at(name: &str, vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}")).sections
}

/// The AS-side OUTBOUND consumer — the objdef word `dc.w objroutine(TestStatic_Main)`
/// and the harness spawn word `dc.w objroutine(TestAnimated)`, both
/// `sym - ObjCodeBase` differences with sym UNDEFINED in-unit (the `.emp` owns it).
fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tdc.w TestStatic_Main-ObjCodeBase\n\
               \tdc.w TestAnimated-ObjCodeBase\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}")).sections
}

fn map_toml(shape: &Shape) -> String {
    let (sb, sl) = (shape.static_base, shape.static_len);
    let (ab, al) = (shape.animated_base, shape.animated_len);
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
         name = \"test_static\"\n\
         lma_base = {sb:#x}\n\
         size = {sl:#x}\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"test_animated\"\n\
         lma_base = {ab:#x}\n\
         size = {al:#x}\n\
         kind = \"rom\"\n"
    )
}

struct Compiled {
    resolved: Vec<Section>,
    linked: sigil_link::LinkedImage,
    static_guards: usize,
    animated_guards: usize,
    link_asserts: Vec<sigil_ir::LinkAssert>,
}

/// Compile both real object modules with ambient deps, place at the bank
/// addresses, append the synthetic cross-seam sections, and link.
fn compile_real_files(shape: &Shape) -> Compiled {
    let aeon = aeon_dir();
    let types = || parse_file(&aeon.join("engine/system/types.emp"));
    let sst = || parse_file(&aeon.join("engine/objects/sst.emp"));
    let constants = parse_file(&aeon.join("engine/system/constants.emp"));
    let objdef = parse_file(&aeon.join("engine/objects/objdef.emp"));
    // VRAM_TEST_SONIC's authority (Parcel F: config/constants.asm → `.emp`).
    let game_consts = parse_file(&aeon.join("games/sonic4/config/constants.emp"));
    let stat = parse_file(&aeon.join("games/sonic4/objects/test_static.emp"));
    let anim = parse_file(&aeon.join("games/sonic4/objects/test_animated.emp"));

    let static_file = with_ambient(vec![types(), sst()], stat);
    let animated_file = with_ambient(vec![types(), sst(), constants, objdef, game_consts], anim);

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("games/sonic4/objects")),
        embed_base: None,
        defines: vec![],
    };
    let mut sections = Vec::new();
    let mut link_asserts = Vec::new();
    let mut static_guards = 0;
    let mut animated_guards = 0;
    for (file, what) in [(static_file, "test_static"), (animated_file, "test_animated")] {
        let (module, ldiags) = lower_module(&file, &opts);
        assert!(
            ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
            "{what} lower errors: {ldiags:?}"
        );
        let g = sigil_harness::test_support::guard_assert_count(&module.link_asserts);
        if what == "test_static" {
            static_guards = g;
        } else {
            animated_guards = g;
        }
        sections.extend(module.sections);
        link_asserts.extend(module.link_asserts);
    }

    let map = sigil_link::load_map(&map_toml(shape)).expect("map must load");
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut lma = 0x0100_0000u32;
    for group in [
        &mut as_constant_equs(),
        &mut as_label_at("Draw_Sprite", shape.draw_sprite),
        &mut as_label_at("AnimateSprite", shape.animate_sprite),
        &mut as_label_at("Perform_DPLC", shape.perform_dplc),
        &mut as_label_at("Map_Sonic", shape.map_sonic),
        &mut as_label_at("Ani_Sonic", shape.ani_sonic),
        &mut as_label_at("DPLC_Sonic", shape.dplc_sonic),
        &mut as_label_at("Art_Sonic", shape.art_sonic),
        &mut as_outbound_consumer(),
    ] {
        for sec in group.iter_mut() {
            sec.lma = lma;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        sections.append(group);
        lma += 0x10_0000;
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    Compiled { resolved, linked, static_guards, animated_guards, link_asserts }
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
        "{what}: length mismatch — candidate {} bytes, expected {} bytes\n  candidate: {candidate:02x?}\n  expected:  {expected:02x?}",
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

/// Load the reference ROM window `[base..base+len]`, or None (skip) when
/// AEON_DIR has no built ROM and strict gating is off.
fn ref_window(rom_name: &str, base: usize, len: usize) -> Option<Vec<u8>> {
    let rom_path = aeon_dir().join(rom_name);
    match std::fs::read(&rom_path) {
        Ok(rom) => Some(rom[base..base + len].to_vec()),
        Err(_) => {
            if strict_gate() {
                panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
            }
            eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
            None
        }
    }
}

/// Both regions' reference gate + the drift guards + the outbound proof.
fn reference_gate(shape: &Shape, rom_name: &str) {
    let Some(static_ref) = ref_window(rom_name, shape.static_base as usize, shape.static_len)
    else {
        return;
    };
    let animated_ref =
        ref_window(rom_name, shape.animated_base as usize, shape.animated_len).unwrap();

    let c = compile_real_files(shape);

    // Drift guards: sst.emp's SST_* wall retired at the conv-a structs flip, so
    // test_static now rides zero; test_animated keeps the constants twin's guards.
    // Its own VRAM_TEST_SONIC mirror guard retired at conv-f (config constants
    // flipped to `.emp`, `use`d now) — as did its _dplc_ptr/_art_base overlay guards
    // at conv-d #48. The remaining guards must be captured AND pass.
    assert_eq!(c.static_guards, 0, "test_static's sst ambient wall retired");
    assert_eq!(
        c.animated_guards,
        twin_guards(),
        "test_animated's constants twin guards captured (own VRAM_TEST_SONIC guard retired at conv-f)"
    );
    let diags = sigil_link::check_link_asserts(&c.resolved, &SymbolTable::new(), &c.link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the drift guards must all PASS: {diags:?}"
    );

    let static_sec = c.linked.section("test_static").expect("linked test_static");
    assert_region_matches(
        &static_sec.bytes,
        &static_ref,
        &format!("test_static vs {rom_name}"),
    );
    // objtest-gate (2026-08-05): test_animated is DEBUG-only — the plain arm
    // proves the region is EMPTY instead of comparing bytes.
    let animated_sec = c.linked.section("test_animated").expect("linked test_animated");
    if shape.animated_len == 0 {
        assert!(animated_ref.is_empty(), "plain animated window must be empty");
    } else {
        assert_region_matches(
            &animated_sec.bytes,
            &animated_ref,
            &format!("test_animated vs {rom_name}"),
        );
    }


    // Outbound proof: the AS-side objroutine words resolve to the bank offsets
    // of the `.emp`-owned pub labels (9th synthetic group).
    let consumer = c
        .linked
        .sections
        .iter()
        .find(|s| s.lma == 0x0100_0000 + 8 * 0x10_0000)
        .expect("outbound consumer at its harness-private LMA");
    let static_word = u16::from_be_bytes([consumer.bytes[0], consumer.bytes[1]]);
    let animated_word = u16::from_be_bytes([consumer.bytes[2], consumer.bytes[3]]);
    assert_eq!(
        static_word,
        (shape.static_base - OBJ_CODE_BASE) as u16,
        "objdef's `dc.w objroutine(TestStatic_Main)` must resolve to the bank offset"
    );
    assert_eq!(
        animated_word,
        (shape.animated_base - OBJ_CODE_BASE) as u16,
        "the harness `dc.w objroutine(TestAnimated)` must resolve to the bank offset"
    );
}

/// (plain) both regions == `s4.bin` windows.
#[test]
fn g1_objects_regions_match_reference() {
    reference_gate(&PLAIN, "s4.bin");
}

/// (debug) both regions == `s4.debug.bin` windows.
#[test]
fn g1_objects_debug_regions_match_reference() {
    reference_gate(&DEBUG, "s4.debug.bin");
}

// ---- negative probe + positive control (the t24 rule) -------------------

/// The UNDOCTORED compile equals the reference window (positive control): if
/// this ever fails, the negative probe below proves nothing.
#[test]
fn g1_undoctored_compile_equals_the_reference_window() {
    // objtest-gate: the plain animated window is empty now — the probe pair runs
    // against the DEBUG shape, where the region still carries bytes.
    let shape = &DEBUG;
    let Some(animated_ref) =
        ref_window("s4.debug.bin", shape.animated_base as usize, shape.animated_len)
    else {
        return;
    };
    let c = compile_real_files(shape);
    let animated_sec = c.linked.section("test_animated").expect("linked test_animated");
    assert_eq!(animated_sec.bytes, animated_ref, "undoctored test_animated must match");
}

/// A doctored reference window must NOT match the compiled bytes (the gate can
/// actually fail). Pins-derived (a re-pin cannot re-stale it).
#[test]
fn g1_doctored_reference_diverges() {
    let shape = &DEBUG;
    let Some(mut animated_ref) =
        ref_window("s4.debug.bin", shape.animated_base as usize, shape.animated_len)
    else {
        return;
    };
    let c = compile_real_files(shape);
    let animated_sec = c.linked.section("test_animated").expect("linked test_animated");
    // Flip one byte of the reference: the compiled bytes must now differ.
    animated_ref[0] ^= 0xFF;
    assert_ne!(
        animated_sec.bytes, animated_ref,
        "a doctored reference must diverge from the compiled bytes"
    );
}
