//! Tranche 31 — game-side G3: the struct-overlay object. The REAL
//! `test_parent.emp` port, region-level byte gates in BOTH shapes, modelled on
//! `test_g2_objects_port.rs` (single region).
//!
//! ## What this port opens
//!
//! - **The per-file object-bank gate** `SIGIL_EMP_TEST_PARENT`. The bank window is
//!   shape-invariant (bank not slid — base/len identical in s4.lst and
//!   s4.debug.lst); content bytes move per shape (the cross-seam callee operands).
//!   test_parent.asm's FIRST label is `TestChildPart` (test_emitter's end anchor);
//!   its end is `TestStressEmitter` (test_stress_emitter's first label).
//! - **Two `vars` SST overlays over the SAME sst_custom region** (TParentV /
//!   TOrbitChildV) whose window-overflow checks replace the two AS `objvarsCheck`
//!   calls. Single-consumer (no surviving AS truth) → ZERO extern drift guards
//!   (the offset $2E is guaranteed by sst.emp's SST_sst_custom).
//! - **The child-dispatch + orbit seam**: CreateChild_Normal + DeleteChildren
//!   (children.emp), GetSineCosine (math.emp), DeleteObject (core.emp), Draw_Sprite
//!   (sprites.emp) — all bare link symbols, ZERO externs. `TestChildPart` is the
//!   INTERNAL child code the descriptor points at (resolved within the region).
//! - **The shared `SpawnDesc` record**: `[SpawnDesc; 3]` + a `dc.w 0` terminator
//!   is the multi-entry consumer of children.emp's hoisted descriptor struct.
//! - **`vram_art` adoption** + the `VRAM_TEST_OBJ: VramTile` game-config mirror
//!   (drift guard resolves against config/constants.asm, which survives).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, or
//! `EMPYREAN_SUITE_ROOT`). Absent, every test here SKIPS green —
//! unless `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test test_g3_objects_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_harness::test_support::{reference_tree, strict_gate};
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

/// The reference tree, or `None` (skip green) when it lacks a source this gate
/// compiles. Every test opens with it — the compile side runs BEFORE any ROM
/// read, so guarding only the ROM window would leave the sources unguarded.
fn ref_sources() -> Option<PathBuf> {
    reference_tree(&[
        "engine/objects/children.emp",
        "engine/objects/objdef.emp",
        "engine/objects/sst.emp",
        "engine/system/constants.emp",
        "engine/system/types.emp",
        "games/sonic4/config/constants.emp",
        "games/sonic4/objects/test_helpers.emp",
        "games/sonic4/objects/test_parent.emp",
    ])
}

/// The engine-constants twin's guard count (shared with the sibling object ports).
fn twin_guards() -> usize {
    sigil_harness::test_support::engine_constant_equs().len()
}

const OBJ_CODE_BASE: u32 = pins::OBJ_CODE_BASE.plain;

/// Per-shape cross-seam VMAs + region geometry. Base shape-invariant; operand
/// bytes track the per-shape engine targets → compile twice.
struct Shape {
    draw_sprite: u32,
    create_child_normal: u32,
    delete_children: u32,
    delete_object: u32,
    get_sine_cosine: u32,
    map_test_obj: u32,
    /// bug005: the spawn-descriptor helper (ambient test_helpers) calls the
    /// animator-owned RefreshSpritePieceCount (animate base + REFRESH_OFF).
    refresh_spc: u32,
    parent_base: u32,
    parent_len: usize,
}

const PLAIN: Shape = Shape {
    draw_sprite: pins::DRAW_SPRITE.plain,
    create_child_normal: pins::CREATE_CHILD_NORMAL.plain,
    delete_children: pins::DELETE_CHILDREN.plain,
    delete_object: pins::DELETE_OBJECT.plain,
    get_sine_cosine: pins::GET_SINE_COSINE.plain,
    map_test_obj: pins::MAP_TEST_OBJ.plain,
    refresh_spc: pins::ANIMATE.plain_base + pins::REFRESH_OFF.plain as u32,
    parent_base: pins::TEST_PARENT.plain_base,
    parent_len: pins::TEST_PARENT.plain_len,
};
const DEBUG: Shape = Shape {
    draw_sprite: pins::DRAW_SPRITE.debug,
    create_child_normal: pins::CREATE_CHILD_NORMAL.debug,
    delete_children: pins::DELETE_CHILDREN.debug,
    delete_object: pins::DELETE_OBJECT.debug,
    get_sine_cosine: pins::GET_SINE_COSINE.debug,
    map_test_obj: pins::MAP_TEST_OBJ.debug,
    refresh_spc: pins::ANIMATE.debug_base + pins::REFRESH_OFF.debug as u32,
    parent_base: pins::TEST_PARENT.debug_base,
    parent_len: pins::TEST_PARENT.debug_len,
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
/// descriptor record). children.emp is code-bearing, so prepending it whole would
/// emit its procs into the region — this keeps just the type-only struct decl.
fn spawn_desc_items(aeon: &std::path::Path) -> Vec<sigil_frontend_emp::ast::Item> {
    use sigil_frontend_emp::ast::Item;
    let file = parse_file(&aeon.join("engine/objects/children.emp"));
    file.items
        .into_iter()
        .filter(|it| matches!(it, Item::Struct(d) if d.name == "SpawnDesc"))
        .collect()
}

/// test_helpers.emp with its `use` imports stripped (the ambient set already
/// supplies sst/objdef/constants/game_consts) — provides the `test_obj_prolog`
/// builder that TestParent now `use`s (the R2-panel frame fold). The module
/// emits no bytes of its own, so this adds nothing to the gated region.
fn test_helpers_items(aeon: &std::path::Path) -> Vec<sigil_frontend_emp::ast::Item> {
    use sigil_frontend_emp::ast::Item;
    let file = parse_file(&aeon.join("games/sonic4/objects/test_helpers.emp"));
    file.items
        .into_iter()
        .filter(|it| !matches!(it, Item::Use(_)))
        .collect()
}

fn with_ambient(
    deps: Vec<Vec<sigil_frontend_emp::ast::Item>>,
    main: sigil_frontend_emp::ast::File,
) -> sigil_frontend_emp::ast::File {
    let mut items = Vec::new();
    for d in deps {
        items.extend(d);
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
/// the game-config VRAM_TEST_OBJ.
fn as_constant_equs() -> Vec<Section> {
    use sigil_harness::test_support::{engine_constant_equs, sst_field_equs};
    let mut pairs = sst_field_equs();
    pairs.extend(engine_constant_equs());
    let obj_code_base = format!("${:X}", OBJ_CODE_BASE);
    pairs.push(("ObjCodeBase", obj_code_base.as_str()));
    pairs.push(("VRAM_TEST_OBJ", "$03E0"));
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// One synthetic AS-side label phased at `vma`.
fn as_label_at(name: &str, vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Some(Cpu::M68000), ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}")).sections
}

fn map_toml(shape: &Shape) -> String {
    let (pb, pl) = (shape.parent_base, shape.parent_len);
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
         name = \"test_parent\"\n\
         lma_base = {pb:#x}\n\
         size = {pl:#x}\n\
         kind = \"rom\"\n"
    )
}

struct Compiled {
    resolved: Vec<Section>,
    linked: sigil_link::LinkedImage,
    guards: usize,
    link_asserts: Vec<sigil_ir::LinkAssert>,
}

/// Compile the real test_parent module with ambient deps (incl. the hoisted
/// SpawnDesc struct), place at the bank address, append the synthetic cross-seam
/// sections, and link.
fn compile_real_file(aeon: &Path, shape: &Shape) -> Compiled {
    let types = || parse_file(&aeon.join("engine/system/types.emp")).items;
    let sst = || parse_file(&aeon.join("engine/objects/sst.emp")).items;
    let constants = || parse_file(&aeon.join("engine/system/constants.emp")).items;
    let objdef = || parse_file(&aeon.join("engine/objects/objdef.emp")).items;
    let spawn_desc = || spawn_desc_items(aeon);
    let test_helpers = || test_helpers_items(aeon);
    // VRAM_TEST_OBJ's authority (Parcel F: config/constants.asm → `.emp`).
    let game_consts = || parse_file(&aeon.join("games/sonic4/config/constants.emp")).items;

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("games/sonic4/objects")),
        embed_base: None,
        defines: vec![],
    };

    let main = parse_file(&aeon.join("games/sonic4/objects/test_parent.emp"));
    let file = with_ambient(
        vec![types(), sst(), constants(), objdef(), spawn_desc(), test_helpers(), game_consts()],
        main,
    );
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "test_parent lower errors: {ldiags:?}"
    );
    let guards = sigil_harness::test_support::guard_assert_count(&module.link_asserts);
    let mut sections = module.sections;
    let link_asserts = module.link_asserts;

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
        &mut as_label_at("CreateChild_Normal", shape.create_child_normal),
        &mut as_label_at("DeleteChildren", shape.delete_children),
        &mut as_label_at("DeleteObject", shape.delete_object),
        &mut as_label_at("GetSineCosine", shape.get_sine_cosine),
        &mut as_label_at("Map_TestObj", shape.map_test_obj),
        &mut as_label_at("RefreshSpritePieceCount", shape.refresh_spc),
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

/// Load the reference ROM window `[base..base+len]`, or None (skip) when the
/// tree carries no built ROM and strict gating is off.
fn ref_window(aeon: &Path, rom_name: &str, base: usize, len: usize) -> Option<Vec<u8>> {
    let rom_path = aeon.join(rom_name);
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

/// The region's reference gate + the drift guards.
fn reference_gate(shape: &Shape, rom_name: &str) {
    let Some(aeon) = ref_sources() else { return };
    // objtest-gate (2026-08-05): test_parent is DEBUG-only — the plain arm proves
    // the region carries ZERO plain bytes and that the module still compiles (at
    // the DEBUG base; the plain pin is the collapsed plain_anchor).
    if shape.parent_len == 0 {
        let _ = compile_real_file(&aeon, &DEBUG);
        return;
    }
    let Some(parent_ref) =
        ref_window(&aeon, rom_name, shape.parent_base as usize, shape.parent_len)
    else {
        return;
    };

    let c = compile_real_file(&aeon, shape);

    // sst.emp's SST_* wall retired at the conv-a structs flip; test_parent's own
    // VRAM_TEST_OBJ mirror guard retired at conv-f (config constants flipped to
    // `.emp`, `use`d now), leaving the constants twin's guards.
    let want = twin_guards();
    assert_eq!(c.guards, want, "test_parent's ambient sst + constants twin guards captured (own VRAM_TEST_OBJ guard retired at conv-f)");

    let diags = sigil_link::check_link_asserts(&c.resolved, &SymbolTable::new(), &c.link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the drift guards must all PASS: {diags:?}"
    );

    let sec = c.linked.section("test_parent").expect("linked test_parent");
    assert_region_matches(&sec.bytes, &parent_ref, &format!("test_parent vs {rom_name}"));
}

/// (plain) the region == `s4.bin` window.
#[test]
fn g3_objects_regions_match_reference() {
    reference_gate(&PLAIN, "s4.bin");
}

/// (debug) the region == `s4.debug.bin` window.
#[test]
fn g3_objects_debug_regions_match_reference() {
    reference_gate(&DEBUG, "s4.debug.bin");
}

// ---- negative probe + positive control (the t24 rule) -------------------

/// The UNDOCTORED compile equals the reference window (positive control): if
/// this ever fails, the negative probe below proves nothing.
#[test]
fn g3_undoctored_compile_equals_the_reference_window() {
    // objtest-gate: DEBUG-only module — the probe pair runs on the debug shape.
    let shape = &DEBUG;
    let Some(aeon) = ref_sources() else { return };
    let Some(parent_ref) =
        ref_window(&aeon, "s4.debug.bin", shape.parent_base as usize, shape.parent_len)
    else {
        return;
    };
    let c = compile_real_file(&aeon, shape);
    let sec = c.linked.section("test_parent").expect("linked test_parent");
    // Packed placement: the pin LEN spans to the next section's aligned base,
    // so the window may end in a short all-zero align pad beyond the lowered
    // image (same tolerance as assert_region_matches).
    let want = if parent_ref.len() > sec.bytes.len()
        && parent_ref.len() - sec.bytes.len() < 16
        && parent_ref[sec.bytes.len()..].iter().all(|&b| b == 0)
    {
        &parent_ref[..sec.bytes.len()]
    } else {
        &parent_ref[..]
    };
    assert_eq!(sec.bytes, want, "undoctored test_parent must match");
}

/// A doctored reference window must NOT match the compiled bytes (the gate can
/// actually fail). Pins-derived (a re-pin cannot re-stale it).
#[test]
fn g3_doctored_reference_diverges() {
    let shape = &DEBUG;
    let Some(aeon) = ref_sources() else { return };
    let Some(mut parent_ref) =
        ref_window(&aeon, "s4.debug.bin", shape.parent_base as usize, shape.parent_len)
    else {
        return;
    };
    let c = compile_real_file(&aeon, shape);
    let sec = c.linked.section("test_parent").expect("linked test_parent");
    parent_ref[0] ^= 0xFF;
    assert_ne!(
        sec.bytes, parent_ref,
        "a doctored reference must diverge from the compiled bytes"
    );
}
