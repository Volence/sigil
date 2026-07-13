//! Tranche 13 — the REAL `load_object.emp` port, region-level byte gate.
//!
//! Compiles the actual ported file — `engine/objects/load_object.emp` — through
//! the production parse → lower → place → resolve → link pipeline and asserts the
//! `load_object` region's flattened bytes equal the reference ROM window at the
//! pinned addresses, in BOTH build shapes.
//!
//! The cleanest region since dplc: NO `assert`, NO `if DEBUG == 1 {}`, so the
//! length is shape-INVARIANT ($9E both shapes) and the two shapes are byte-
//! identical (only relative branches, register-indirect EAs, and one
//! `jsr AllocDynamic` — an absolute-word link ref).
//!
//! Cross-seam INBOUND: sst.emp's SST_* struct equs + the engine constants twin
//! (RF_XFLIP/RF_YFLIP/FRAME_PIECE_COUNT), plus the sole external code target,
//! `AllocDynamic`. `Load_ObjectList`'s `jsr Load_Object` resolves within the
//! placed section, so it needs no pin.
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
    base: pins::LOAD_OBJECT.plain_base,
    len: pins::LOAD_OBJECT.plain_len,
    labels: &[("AllocDynamic", pins::ALLOC_DYNAMIC.plain)],
};

const DEBUG: Shape = Shape {
    base: pins::LOAD_OBJECT.debug_base,
    len: pins::LOAD_OBJECT.debug_len,
    labels: &[("AllocDynamic", pins::ALLOC_DYNAMIC.debug)],
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

/// The AS-side value seam: SST struct equs + engine constants twin. load_object
/// carries NO local const mirrors (unlike entity_window) — it `use`s the shared
/// twins directly, whose drift guards fire against these equs.
fn as_constant_equs() -> Vec<Section> {
    let mut pairs = sigil_harness::test_support::sst_field_equs();
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
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
         name = \"load_object\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

fn compile_real_file(
    shape: &Shape,
    defines: &[(&str, i128)],
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let aeon = aeon_dir();
    let types = parse_file(&aeon.join("engine/system/types.emp"));
    let sst = parse_file(&aeon.join("engine/objects/sst.emp"));
    let constants = parse_file(&aeon.join("engine/system/constants.emp"));
    let lo = parse_file(&aeon.join("engine/objects/load_object.emp"));

    let file = with_ambient(vec![types, sst, constants], lo);

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("engine/objects")),
        embed_base: None,
        defines: defines.iter().map(|(n, v)| (n.to_string(), *v)).collect(),
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "load_object.emp lower errors: {ldiags:?}"
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
    let mut groups: Vec<Vec<Section>> = vec![as_constant_equs()];
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

    let defines: Vec<(&str, i128)> =
        vec![("DEBUG", debug_define), ("SOUND_DRIVER_ENABLED", 1)];
    let (resolved, linked, link_asserts) = compile_real_file(shape, &defines);
    assert_drift_guards(&resolved, &link_asserts);

    let base = shape.base as usize;
    let section = linked.section("load_object").expect("linked image must carry load_object");
    assert_region_matches(
        &section.bytes,
        &refrom[base..base + shape.len],
        &format!("load_object vs {rom_name}[{base:#x}..{:#x}]", base + shape.len),
    );
}

/// (plain) the `load_object` region == `s4.bin[0x3FDC..0x407A]` — DEBUG=0.
#[test]
fn load_object_region_matches_reference() {
    reference_gate(&PLAIN, "s4.bin", 0);
}

/// (debug) the `load_object` region == `s4.debug.bin[0x4BA6..0x4C44]` — DEBUG=1
/// (byte-identical to plain: no DEBUG-conditional code in the file).
#[test]
fn load_object_debug_region_matches_reference() {
    reference_gate(&DEBUG, "s4.debug.bin", 1);
}
