//! Tranche 24 — the REAL `children.emp` port, region-level byte gate.
//!
//! Compiles the actual ported file — `engine/objects/children.emp` — through
//! the production parse → lower → place → resolve → link pipeline and asserts
//! the `children` region's flattened bytes equal the reference ROM window at
//! the pinned addresses, in BOTH build shapes.
//!
//! Shape-DEPENDENT since the step-5 chain-contract asserts: the debug shape
//! carries TWO `assert.w` sites + their message blobs (and the twin's one
//! `ifdef __DEBUG__` branch width they push out of `.s` reach); the plain
//! shape self-gates them to ZERO bytes. Cross-region link refs:
//! `AllocDynamic`/`AllocEffect`/`DeleteObject` (all `core.emp`-owned) plus,
//! in the debug shape, the assert construct's error-handler tail.
//!
//! Cross-seam INBOUND: sst.emp's SST_* struct equs + the engine constants twin
//! (`FRAME_PIECE_COUNT`, `RF_XFLIP`). The six internal
//! `PopulateSpawnedPieceCount` calls resolve within the placed section.
//!
//! SHARED ANCHORS (the tranche's novel pin machinery): the region's start
//! symbol is simultaneously `entity_window`'s END anchor, and its end symbol is
//! simultaneously `load_object`'s START. `children_region_pins_share_both_anchors`
//! proves the three regions still tile without gap or overlap after the new
//! gate exists.
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`). Absent, the
//! gates SKIP green unless `SIGIL_STRICT_GATE=1`.

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

struct Shape {
    base: u32,
    len: usize,
    labels: &'static [(&'static str, u32)],
}

const PLAIN: Shape = Shape {
    base: pins::CHILDREN.plain_base,
    len: pins::CHILDREN.plain_len,
    labels: &[
        ("AllocDynamic", pins::ALLOC_DYNAMIC.plain),
        ("AllocEffect", pins::ALLOC_EFFECT.plain),
        ("DeleteObject", pins::DELETE_OBJECT.plain),
    ],
};

const DEBUG: Shape = Shape {
    base: pins::CHILDREN.debug_base,
    len: pins::CHILDREN.debug_len,
    labels: &[
        ("AllocDynamic", pins::ALLOC_DYNAMIC.debug),
        ("AllocEffect", pins::ALLOC_EFFECT.debug),
        ("DeleteObject", pins::DELETE_OBJECT.debug),
        // The chain-contract asserts' error-handler tail (debug shape only).
        ("MDDBG__ErrorHandler", pins::MDDBG_ERROR_HANDLER),
        ("MDDBG__ErrorHandler_PagesController", pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER),
    ],
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

/// The AS-side value seam: SST struct equs + engine constants twin. children
/// carries NO local const mirrors — it `use`s the shared twins directly
/// (FRAME_PIECE_COUNT / RF_XFLIP), whose drift guards fire against these equs.
/// The seam, with ONE truth value optionally replaced — the doctored-truth
/// negative probe (`boot_port::doctored_psg_port_fires_its_guard` class).
fn as_constant_equs_with(doctor: Option<(&str, &str)>) -> Vec<Section> {
    let mut pairs = sigil_harness::test_support::sst_field_equs();
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    if let Some((name, value)) = doctor {
        let before = pairs.len();
        pairs.retain(|(n, _)| *n != name);
        assert_eq!(pairs.len() + 1, before, "doctored name {name} must exist in the seam");
        pairs.push((name, value));
    }
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

fn as_label_at(name: &str, vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}")).sections
}

fn map_toml(base: u32, len: usize) -> String {
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
         name = \"children\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

fn compile_real_file(
    shape: &Shape,
    defines: &[(&str, i128)],
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    compile_with_seam(shape, defines, None)
}

fn compile_with_seam(
    shape: &Shape,
    defines: &[(&str, i128)],
    doctor: Option<(&str, &str)>,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let aeon = aeon_dir();
    let types = parse_file(&aeon.join("engine/system/types.emp"));
    let sst = parse_file(&aeon.join("engine/objects/sst.emp"));
    let constants = parse_file(&aeon.join("engine/system/constants.emp"));
    let coords = parse_file(&aeon.join("engine/coords.emp"));
    let frames = parse_file(&aeon.join("engine/objects/frames.emp"));
    let ch = parse_file(&aeon.join("engine/objects/children.emp"));

    let file = with_ambient(vec![types, sst, constants, coords, frames], ch);

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("engine/objects")),
        embed_base: None,
        defines: defines.iter().map(|(n, v)| (n.to_string(), *v)).collect(),
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "children.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(shape.base, shape.len)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut lma = 0x0100_0000u32;
    let mut groups: Vec<Vec<Section>> = vec![as_constant_equs_with(doctor)];
    for (name, vma) in shape.labels {
        groups.push(as_label_at(name, *vma));
    }
    for group in &mut groups {
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
    (resolved, linked, link_asserts)
}

/// The drift guards (sst.emp's 30 + constants.emp's) must all be captured and PASS.
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the drift guards must all PASS: {diags:?}"
    );
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

fn reference_gate(shape: &Shape, rom_name: &str, debug_define: i128) {
    let rom_path = aeon_dir().join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let defines: Vec<(&str, i128)> = vec![("DEBUG", debug_define), ("SOUND_DRIVER_ENABLED", 1)];
    let (resolved, linked, link_asserts) = compile_real_file(shape, &defines);
    assert_drift_guards(&resolved, &link_asserts);

    let base = shape.base as usize;
    let section = linked.section("children").expect("linked image must carry children");
    assert_region_matches(
        &section.bytes,
        &refrom[base..base + shape.len],
        &format!("children vs {rom_name}[{base:#x}..{:#x}]", base + shape.len),
    );
}

/// (plain) the `children` region == the `s4.bin` window at `pins::CHILDREN`'s
/// plain base/len — DEBUG=0.
#[test]
fn children_region_matches_reference() {
    reference_gate(&PLAIN, "s4.bin", 0);
}

/// (debug) the `children` region == the `s4.debug.bin` window at
/// `pins::CHILDREN`'s debug base/len — DEBUG=1
/// (byte-identical to plain: no DEBUG-conditional code in the file).
#[test]
fn children_debug_region_matches_reference() {
    reference_gate(&DEBUG, "s4.debug.bin", 1);
}

// The `doctored_sibling_ptr_fires_its_guard` negative probe retired with the
// conv-a structs flip: `sst.emp` is the sole author of the object-record layout
// now (its `SST_*` drift wall is deleted — it became a tautology), so there is no
// guard to fire. A wrong SST offset instead moves ROM bytes, caught by the
// six-target byte-identity. (The `doctored_rf_xflip_fires_its_guard` probe
// likewise retired at the Stage-3 P5 flip when `RF_XFLIP` became `.emp`-owned.)

/// SHARED-ANCHOR proof. `PopulateSpawnedPieceCount` is entity_window's END
/// anchor AND children's START anchor; `Load_Object` is children's END anchor
/// AND load_object's START. The three regions must tile exactly — no gap (a
/// dropped byte range no gate covers) and no overlap (two gates claiming the
/// same bytes) — in BOTH shapes. A repin that resolved either shared symbol
/// differently for the two regions would fail here.
#[test]
// The asserted operands are repin-generated pin constants — live contracts on
// the generated table, not tautologies.
#[allow(clippy::assertions_on_constants)]
fn children_region_pins_share_both_anchors() {
    assert_eq!(
        pins::ENTITY_WINDOW.plain_base + pins::ENTITY_WINDOW.plain_len as u32,
        pins::CHILDREN.plain_base,
        "plain: entity_window must end exactly where children begins (shared PopulateSpawnedPieceCount anchor)"
    );
    assert_eq!(
        pins::ENTITY_WINDOW.debug_base + pins::ENTITY_WINDOW.debug_len as u32,
        pins::CHILDREN.debug_base,
        "debug: entity_window must end exactly where children begins (shared PopulateSpawnedPieceCount anchor)"
    );
    assert_eq!(
        pins::CHILDREN.plain_base + pins::CHILDREN.plain_len as u32,
        pins::LOAD_OBJECT.plain_base,
        "plain: children must end exactly where load_object begins (shared Load_Object anchor)"
    );
    assert_eq!(
        pins::CHILDREN.debug_base + pins::CHILDREN.debug_len as u32,
        pins::LOAD_OBJECT.debug_base,
        "debug: children must end exactly where load_object begins (shared Load_Object anchor)"
    );
    // Shape-DEPENDENT length since the chain-contract asserts landed: the
    // debug shape carries two `assert.w` sites plus their message blobs, and
    // the plain shape self-gates them to ZERO bytes (rings/core precedent).
    assert!(
        pins::CHILDREN.debug_len > pins::CHILDREN.plain_len,
        "the debug shape must carry the chain-contract assert blobs (debug {:#x} vs plain {:#x})",
        pins::CHILDREN.debug_len,
        pins::CHILDREN.plain_len
    );
}
