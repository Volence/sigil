//! Tranche 30 — game-side G2: the effect/child-lifecycle trio. The REAL
//! `test_emitter.emp` + `test_stress_emitter.emp` + `test_churn.emp` ports,
//! region-level byte gates in BOTH shapes, modelled on `test_g1_objects_port.rs`.
//!
//! ## What this port opens
//!
//! - **Three per-file object-bank gates** (`SIGIL_EMP_TEST_EMITTER` /
//!   `_TEST_STRESS_EMITTER` / `_TEST_CHURN`). Bases are shape-invariant (bank not
//!   slid — verified identical in s4.lst and s4.debug.lst); content bytes move
//!   per shape (the effect-seam callee operands). test_parent.asm sits BETWEEN
//!   test_emitter and test_stress_emitter (not ported), so test_emitter's end
//!   anchor is `TestParent`.
//! - **The game→children.emp/core.emp EFFECT SEAM at scale**: CreateEffect_Normal
//!   + PopulateSpawnedPieceCount (children.emp), AllocDynamic + DeleteObject
//!   (core.emp), Draw_Sprite (sprites.emp) — all bare link symbols, ZERO externs.
//! - **Three self-contained `vars` SST overlays** (TEmitterV/TStressEmitterV/
//!   TChurnV) whose window-overflow check replaces the AS `objvarsCheck`. Unlike
//!   t29's DplcV, their field equates live ONLY in the gated-out .asm → NO extern
//!   drift guard (the offset $2E is guaranteed by sst.emp's SST_sst_custom).
//! - **`vram_art` adoption** + the `VRAM_TEST_OBJ: VramTile` game-config mirror
//!   (drift guard resolves against config/constants.asm, which survives).
//! - **The `.particle_desc` descriptor blob**: `dc.w TestParticle - ObjCodeBase`
//!   resolves the effect child's bank offset across the seam.
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
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The engine-constants twin's guard count (shared with test_particle/animated).
fn twin_guards() -> usize {
    sigil_harness::test_support::engine_constant_equs().len()
}

const OBJ_CODE_BASE: u32 = pins::OBJ_CODE_BASE.plain;

/// Per-shape cross-seam VMAs + region geometry. Bases shape-invariant; operand
/// bytes track the per-shape engine targets → compile twice.
struct Shape {
    draw_sprite: u32,
    create_effect_normal: u32,
    populate_spawned: u32,
    alloc_dynamic: u32,
    delete_object: u32,
    test_particle: u32,
    map_test_obj: u32,
    /// bug005: the spawn-descriptor helper (ambient test_helpers) calls the
    /// animator-owned RefreshSpritePieceCount (animate base + REFRESH_OFF).
    refresh_spc: u32,
    emitter_base: u32,
    emitter_len: usize,
    stress_base: u32,
    stress_len: usize,
    churn_base: u32,
    churn_len: usize,
}

const PLAIN: Shape = Shape {
    draw_sprite: pins::DRAW_SPRITE.plain,
    create_effect_normal: pins::CREATE_EFFECT_NORMAL.plain,
    populate_spawned: pins::CHILDREN.plain_base,
    alloc_dynamic: pins::ALLOC_DYNAMIC.plain,
    delete_object: pins::DELETE_OBJECT.plain,
    test_particle: pins::TEST_PARTICLE.plain_base,
    map_test_obj: pins::MAP_TEST_OBJ.plain,
    refresh_spc: pins::ANIMATE.plain_base + pins::REFRESH_OFF.plain as u32,
    emitter_base: pins::TEST_EMITTER.plain_base,
    emitter_len: pins::TEST_EMITTER.plain_len,
    stress_base: pins::TEST_STRESS_EMITTER.plain_base,
    stress_len: pins::TEST_STRESS_EMITTER.plain_len,
    churn_base: pins::TEST_CHURN.plain_base,
    churn_len: pins::TEST_CHURN.plain_len,
};
const DEBUG: Shape = Shape {
    draw_sprite: pins::DRAW_SPRITE.debug,
    create_effect_normal: pins::CREATE_EFFECT_NORMAL.debug,
    populate_spawned: pins::CHILDREN.debug_base,
    alloc_dynamic: pins::ALLOC_DYNAMIC.debug,
    delete_object: pins::DELETE_OBJECT.debug,
    test_particle: pins::TEST_PARTICLE.debug_base,
    map_test_obj: pins::MAP_TEST_OBJ.debug,
    refresh_spc: pins::ANIMATE.debug_base + pins::REFRESH_OFF.debug as u32,
    emitter_base: pins::TEST_EMITTER.debug_base,
    emitter_len: pins::TEST_EMITTER.debug_len,
    stress_base: pins::TEST_STRESS_EMITTER.debug_base,
    stress_len: pins::TEST_STRESS_EMITTER.debug_len,
    churn_base: pins::TEST_CHURN.debug_base,
    churn_len: pins::TEST_CHURN.debug_len,
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

/// children.emp filtered to ONLY the `SpawnDesc` struct (the hoisted 4-byte
/// descriptor record the two emitters now `use`). children.emp is code-bearing,
/// so prepending it whole would emit its procs — this keeps just the type decl.
fn spawn_desc_file(aeon: &std::path::Path) -> sigil_frontend_emp::ast::File {
    use sigil_frontend_emp::ast::Item;
    let mut file = parse_file(&aeon.join("engine/objects/children.emp"));
    file.items
        .retain(|it| matches!(it, Item::Struct(d) if d.name == "SpawnDesc"));
    file
}

/// test_helpers.emp with its `use` imports stripped (the ambient set already
/// supplies sst/objdef/constants/game_consts) — keeps the shared EmitterV overlay
/// and the test_obj_prolog builder the three objects now `use`.
fn test_helpers_file(aeon: &std::path::Path) -> sigil_frontend_emp::ast::File {
    use sigil_frontend_emp::ast::Item;
    let mut file = parse_file(&aeon.join("games/sonic4/objects/test_helpers.emp"));
    file.items.retain(|it| !matches!(it, Item::Use(_)));
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
/// the game-config VRAM_TEST_OBJ + the RAM Frame_Counter equate.
fn as_constant_equs() -> Vec<Section> {
    use sigil_harness::test_support::{engine_constant_equs, sst_field_equs};
    let mut pairs = sst_field_equs();
    pairs.extend(engine_constant_equs());
    let obj_code_base = format!("${:X}", OBJ_CODE_BASE);
    pairs.push(("ObjCodeBase", obj_code_base.as_str()));
    pairs.push(("VRAM_TEST_OBJ", "$03E0"));
    pairs.push(("Frame_Counter", "$FFFF8002"));
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// One synthetic AS-side label phased at `vma`.
fn as_label_at(name: &str, vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}")).sections
}

/// The AS-side OUTBOUND consumers — object_test_state.asm's spawn words
/// `dc.w objroutine(TestStressEmitter)` / `dc.w objroutine(TestChurnObj)`,
/// `sym - ObjCodeBase` differences with sym UNDEFINED in-unit (the `.emp` owns it).
fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tdc.w TestStressEmitter-ObjCodeBase\n\
               \tdc.w TestChurnObj-ObjCodeBase\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}")).sections
}

fn map_toml(shape: &Shape) -> String {
    let (eb, el) = (shape.emitter_base, shape.emitter_len);
    let (sb, sl) = (shape.stress_base, shape.stress_len);
    let (cb, cl) = (shape.churn_base, shape.churn_len);
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
         name = \"test_emitter\"\n\
         lma_base = {eb:#x}\n\
         size = {el:#x}\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"test_stress_emitter\"\n\
         lma_base = {sb:#x}\n\
         size = {sl:#x}\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"test_churn\"\n\
         lma_base = {cb:#x}\n\
         size = {cl:#x}\n\
         kind = \"rom\"\n"
    )
}

struct Compiled {
    resolved: Vec<Section>,
    linked: sigil_link::LinkedImage,
    guards: Vec<(String, usize)>,
    link_asserts: Vec<sigil_ir::LinkAssert>,
}

/// Compile the three real object modules with ambient deps, place at the bank
/// addresses, append the synthetic cross-seam sections, and link.
fn compile_real_files(shape: &Shape) -> Compiled {
    let aeon = aeon_dir();
    let types = || parse_file(&aeon.join("engine/system/types.emp"));
    let sst = || parse_file(&aeon.join("engine/objects/sst.emp"));
    let constants = || parse_file(&aeon.join("engine/system/constants.emp"));
    let objdef = || parse_file(&aeon.join("engine/objects/objdef.emp"));
    // VRAM_TEST_OBJ's authority (Parcel F: config/constants.asm → `.emp`).
    let game_consts = || parse_file(&aeon.join("games/sonic4/config/constants.emp"));

    let files = [
        ("test_emitter", "games/sonic4/objects/test_emitter.emp"),
        ("test_stress_emitter", "games/sonic4/objects/test_stress_emitter.emp"),
        ("test_churn", "games/sonic4/objects/test_churn.emp"),
    ];

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("games/sonic4/objects")),
        embed_base: None,
        defines: vec![],
    };
    let mut sections = Vec::new();
    let mut link_asserts = Vec::new();
    let mut guards = Vec::new();
    for (region, rel) in files {
        let main = parse_file(&aeon.join(rel));
        let file = with_ambient(
            vec![types(), sst(), constants(), objdef(), spawn_desc_file(&aeon), game_consts(), test_helpers_file(&aeon)],
            main,
        );
        let (module, ldiags) = lower_module(&file, &opts);
        assert!(
            ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
            "{region} lower errors: {ldiags:?}"
        );
        let g = sigil_harness::test_support::guard_assert_count(&module.link_asserts);
        guards.push((region.to_string(), g));
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
        &mut as_label_at("CreateEffect_Normal", shape.create_effect_normal),
        &mut as_label_at("PopulateSpawnedPieceCount", shape.populate_spawned),
        &mut as_label_at("AllocDynamic", shape.alloc_dynamic),
        &mut as_label_at("DeleteObject", shape.delete_object),
        &mut as_label_at("TestParticle", shape.test_particle),
        &mut as_label_at("Map_TestObj", shape.map_test_obj),
        &mut as_label_at("RefreshSpritePieceCount", shape.refresh_spc),
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
    Compiled { resolved, linked, guards, link_asserts }
}

fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
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

/// All three regions' reference gate + the drift guards + the outbound proof.
fn reference_gate(shape: &Shape, rom_name: &str) {
    // objtest-gate (2026-08-05): all three G2 modules are DEBUG-only — the plain
    // arm's job flips to proving their regions carry ZERO plain bytes (the
    // modules still compile; the byte gates run on the debug arm).
    if shape.emitter_len == 0 {
        assert_eq!(shape.stress_len, 0, "gated trio must be all-empty in plain");
        assert_eq!(shape.churn_len, 0, "gated trio must be all-empty in plain");
        // The trio must still COMPILE standalone — at the DEBUG bases (the plain
        // pins all collapse onto the plain_anchor and would overlap).
        let _ = compile_real_files(&DEBUG);
        return;
    }
    let Some(emitter_ref) = ref_window(rom_name, shape.emitter_base as usize, shape.emitter_len)
    else {
        return;
    };
    let stress_ref =
        ref_window(rom_name, shape.stress_base as usize, shape.stress_len).unwrap();
    let churn_ref = ref_window(rom_name, shape.churn_base as usize, shape.churn_len).unwrap();

    let c = compile_real_files(shape);

    // sst.emp's SST_* wall retired at the conv-a structs flip; each file's own
    // VRAM_TEST_OBJ mirror guard retired at conv-f (config constants flipped to
    // `.emp`, `use`d now), leaving the constants twin's guards.
    let want = twin_guards();
    for (region, g) in &c.guards {
        assert_eq!(*g, want, "{region}'s ambient sst + constants twin guards captured (own VRAM_TEST_OBJ guard retired at conv-f)");
    }
    let diags = sigil_link::check_link_asserts(&c.resolved, &SymbolTable::new(), &c.link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the drift guards must all PASS: {diags:?}"
    );

    for (region, refwin) in [
        ("test_emitter", &emitter_ref),
        ("test_stress_emitter", &stress_ref),
        ("test_churn", &churn_ref),
    ] {
        let sec = c.linked.section(region).unwrap_or_else(|| panic!("linked {region}"));
        assert_region_matches(&sec.bytes, refwin, &format!("{region} vs {rom_name}"));
    }

    // Outbound proof: the AS-side objroutine words resolve to the bank offsets
    // of the `.emp`-owned pub labels (10th synthetic group — the bug005
    // RefreshSpritePieceCount seam slotted in ahead of the consumer).
    let consumer = c
        .linked
        .sections
        .iter()
        .find(|s| s.lma == 0x0100_0000 + 9 * 0x10_0000)
        .expect("outbound consumer at its harness-private LMA");
    let stress_word = u16::from_be_bytes([consumer.bytes[0], consumer.bytes[1]]);
    let churn_word = u16::from_be_bytes([consumer.bytes[2], consumer.bytes[3]]);
    assert_eq!(
        stress_word,
        (shape.stress_base - OBJ_CODE_BASE) as u16,
        "object_test_state's `dc.w objroutine(TestStressEmitter)` must resolve to the bank offset"
    );
    assert_eq!(
        churn_word,
        (shape.churn_base - OBJ_CODE_BASE) as u16,
        "object_test_state's `dc.w objroutine(TestChurnObj)` must resolve to the bank offset"
    );
}

/// (plain) all three regions == `s4.bin` windows.
#[test]
fn g2_objects_regions_match_reference() {
    reference_gate(&PLAIN, "s4.bin");
}

/// (debug) all three regions == `s4.debug.bin` windows.
#[test]
fn g2_objects_debug_regions_match_reference() {
    reference_gate(&DEBUG, "s4.debug.bin");
}

// ---- negative probe + positive control (the t24 rule) -------------------

/// The UNDOCTORED compile equals the reference window (positive control): if
/// this ever fails, the negative probe below proves nothing.
#[test]
fn g2_undoctored_compile_equals_the_reference_window() {
    // objtest-gate: the trio is DEBUG-only — the probe pair runs on the debug shape.
    let shape = &DEBUG;
    let Some(churn_ref) = ref_window("s4.debug.bin", shape.churn_base as usize, shape.churn_len) else {
        return;
    };
    let c = compile_real_files(shape);
    let churn_sec = c.linked.section("test_churn").expect("linked test_churn");
    assert_eq!(churn_sec.bytes, churn_ref, "undoctored test_churn must match");
}

/// A doctored reference window must NOT match the compiled bytes (the gate can
/// actually fail). Pins-derived (a re-pin cannot re-stale it).
#[test]
fn g2_doctored_reference_diverges() {
    let shape = &DEBUG;
    let Some(mut churn_ref) = ref_window("s4.debug.bin", shape.churn_base as usize, shape.churn_len)
    else {
        return;
    };
    let c = compile_real_files(shape);
    let churn_sec = c.linked.section("test_churn").expect("linked test_churn");
    churn_ref[0] ^= 0xFF;
    assert_ne!(
        churn_sec.bytes, churn_ref,
        "a doctored reference must diverge from the compiled bytes"
    );
}
