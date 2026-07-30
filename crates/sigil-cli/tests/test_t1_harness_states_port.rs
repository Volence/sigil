//! Tranche 41 — the T1 harness states: the REAL `object_test_state.emp` +
//! `ojz_scroll_test.emp` ports, region-level byte gates in BOTH shapes, plus the
//! t24 positive-control / negative-probe pair and the drift-guard counts. These
//! are the LAST game-side code files; after t41 the 68k game side is
//! code-complete (only main/config remain — the Spec-5 flip).
//!
//! ## What this port opens
//!
//! - **Two per-file, SHAPE-DEPENDENT game-STATE gates**
//!   (`SIGIL_EMP_OBJECT_TEST_STATE` $5BC/$658, `SIGIL_EMP_OJZ_SCROLL_TEST`
//!   $2C2/$2CE) — the first game-state banks (no objdef header), the
//!   GameState_* procs dispatched through Game_State.
//! - **A cross-module DATA reference**: ojz's marker DMA reads
//!   object_test_state's `TestArt` blob (the length mirrored as a pure-int
//!   `TEST_ART_LEN`; the ring blob's size drift-guarded via an `ensure`).
//! - **The FIRST `embed()` game-state port**: object_test_state's `TestArt`
//!   embeds `games/sonic4/test/ring_art.bin` + `art/palettes/sonic.bin` from the
//!   AEON REPO ROOT (the BINCLUDE path model — include_root = repo root).
//! - **The row-35 mode-set-3 force-write** ported verbatim (kill row 35 stays
//!   open against the parallax-hardening parcel).
//!
//! The cross-reference seam comes from the REAL AS-side game
//! (`assemble_mixed_tranche41_as_side`, both gates on), so every callee / RAM /
//! data address is the reference ROM's own — no synthetic address table to drift.
//!
//! REFERENCE-DEPENDENT (`AEON_DIR`, default sibling). Absent, tests SKIP green
//! unless `SIGIL_STRICT_GATE=1`.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::{assemble_mixed_tranche41_as_side, pins};
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use std::path::{Path, PathBuf};

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

struct Shape {
    debug: bool,
    rom: &'static str,
    ots_base: u32,
    ots_len: usize,
    ojz_base: u32,
    ojz_len: usize,
}

const PLAIN: Shape = Shape {
    debug: false,
    rom: "s4.bin",
    ots_base: pins::OBJECT_TEST_STATE.plain_base,
    ots_len: pins::OBJECT_TEST_STATE.plain_len,
    ojz_base: pins::OJZ_SCROLL_TEST.plain_base,
    ojz_len: pins::OJZ_SCROLL_TEST.plain_len,
};
const DEBUG: Shape = Shape {
    debug: true,
    rom: "s4.debug.bin",
    ots_base: pins::OBJECT_TEST_STATE.debug_base,
    ots_len: pins::OBJECT_TEST_STATE.debug_len,
    ojz_base: pins::OJZ_SCROLL_TEST.debug_base,
    ojz_len: pins::OJZ_SCROLL_TEST.debug_len,
};

fn parse_items(path: &Path) -> Vec<sigil_frontend_emp::ast::Item> {
    let src =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (file, diags) = parse_str(&src);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} parse errors: {diags:?}",
        path.display()
    );
    file.items
}

/// Lower one .emp with its `use`-twin items prepended, placed into `region`.
/// Returns (sections, guard_count). include_root = embed_base = the aeon repo
/// root so object_test_state's `embed(...)` blobs resolve repo-root-relative.
#[allow(clippy::too_many_arguments)]
fn lower_one(
    aeon: &Path,
    emp_rel: &str,
    ambient: Vec<Vec<sigil_frontend_emp::ast::Item>>,
    region: &str,
    base: u32,
    len: usize,
    debug: bool,
) -> (Vec<Section>, usize, Vec<sigil_ir::LinkAssert>) {
    let src = std::fs::read_to_string(aeon.join(emp_rel))
        .unwrap_or_else(|e| panic!("cannot read {emp_rel}: {e}"));
    let (main, mdiags) = parse_str(&src);
    assert!(
        mdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{emp_rel} parse errors: {mdiags:?}"
    );
    let mut items = Vec::new();
    for a in ambient {
        items.extend(a);
    }
    items.extend(main.items);
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items,
        docs: main.docs.clone(),
    };
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.to_path_buf()),
        embed_base: Some(aeon.to_path_buf()),
        defines: vec![("DEBUG".to_string(), i128::from(debug))],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{emp_rel} lower errors: {ldiags:?}"
    );
    let guards = sigil_harness::test_support::guard_assert_count(&module.link_asserts);
    let mt = format!(
        "fill = 0x00\n\n[[region]]\nname = \"text\"\nlma_base = 0x0000\nsize = 0x10\nkind = \"rom\"\n\n[[region]]\nname = \"{region}\"\nlma_base = {base:#x}\nsize = {len:#x}\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&mt).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{emp_rel} place_sections errors: {pdiags:?}"
    );
    (sections, guards, module.link_asserts)
}

struct Compiled {
    ots: Vec<u8>,
    ojz: Vec<u8>,
    ots_guards: usize,
    ojz_guards: usize,
    resolved: Vec<Section>,
    link_asserts: Vec<sigil_ir::LinkAssert>,
}

fn compile(shape: &Shape) -> Compiled {
    let aeon = aeon_dir();
    let engine = aeon.join("engine");
    let system = engine.join("system");
    let objects = engine.join("objects");

    let types = || parse_items(&system.join("types.emp"));
    let sst = || parse_items(&objects.join("sst.emp"));
    let constants = || parse_items(&system.join("constants.emp"));
    let objdef = || parse_items(&objects.join("objdef.emp"));
    let vdp = || parse_items(&engine.join("vdp.emp"));
    let structs = || parse_items(&engine.join("structs.emp"));
    let z80_bus = || parse_items(&engine.join("z80_bus.emp"));
    let coords = || parse_items(&engine.join("coords.emp"));

    let (ots_secs, ots_guards, ots_asserts) = lower_one(
        &aeon,
        "games/sonic4/test/object_test_state.emp",
        vec![types(), sst(), constants(), objdef(), vdp(), coords()],
        "object_test_state",
        shape.ots_base,
        shape.ots_len,
        shape.debug,
    );
    let (ojz_secs, ojz_guards, ojz_asserts) = lower_one(
        &aeon,
        "games/sonic4/test/ojz_scroll_test.emp",
        vec![types(), sst(), constants(), objdef(), structs(), vdp(), z80_bus(), coords()],
        "ojz_scroll_test",
        shape.ojz_base,
        shape.ojz_len,
        shape.debug,
    );

    // The cross-reference seam: the REAL AS-side game (both gates on).
    let as_module = assemble_mixed_tranche41_as_side(&aeon, shape.debug)
        .unwrap_or_else(|e| panic!("AS side: {e}"));

    let mut sections = as_module.sections;
    sections.extend(ots_secs);
    sections.extend(ojz_secs);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));

    let ots = linked.section("object_test_state").expect("linked object_test_state").bytes.clone();
    let ojz = linked.section("ojz_scroll_test").expect("linked ojz_scroll_test").bytes.clone();

    let mut link_asserts = ots_asserts;
    link_asserts.extend(ojz_asserts);
    Compiled { ots, ojz, ots_guards, ojz_guards, resolved, link_asserts }
}

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

fn assert_window(candidate: &[u8], expected: &[u8], what: &str) {
    assert_eq!(candidate.len(), expected.len(), "{what}: length mismatch");
    if let Some(i) = (0..candidate.len()).find(|&i| candidate[i] != expected[i]) {
        let lo = i.saturating_sub(8);
        let hi = (i + 16).min(candidate.len());
        panic!(
            "{what}: first diff at region-offset {i:#x}\n  candidate[{lo:#x}..{hi:#x}]: {:02x?}\n  expected:  {:02x?}",
            &candidate[lo..hi],
            &expected[lo..hi]
        );
    }
}

fn reference_gate(shape: &Shape) {
    let Some(ots_ref) = ref_window(shape.rom, shape.ots_base as usize, shape.ots_len) else {
        return;
    };
    let ojz_ref = ref_window(shape.rom, shape.ojz_base as usize, shape.ojz_len).unwrap();

    let c = compile(shape);

    // Drift-guard counts: object_test_state rides sst.emp's 30 + constants.emp's
    // engine guards + objdef.emp's, plus its own 3 (VRAM_TEST_OBJ + STUB_FLOOR_Y +
    // VDP_HV_COUNTER) and the ring-blob-size ensure; ojz adds VRAM_TEST_OBJ +
    // VRAM_TEST_MARKER + CAM_SCREEN_HALF_W/H. Both nonzero (the ensures fire).
    assert!(c.ots_guards > 0, "object_test_state drift guards must fire");
    assert!(c.ojz_guards > 0, "ojz_scroll_test drift guards must fire");
    let diags = sigil_link::check_link_asserts(&c.resolved, &SymbolTable::new(), &c.link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the drift guards must all PASS against the real AS tree: {diags:?}"
    );

    assert_window(&c.ots, &ots_ref, &format!("object_test_state vs {}", shape.rom));
    assert_window(&c.ojz, &ojz_ref, &format!("ojz_scroll_test vs {}", shape.rom));
}

/// (plain) both regions == `s4.bin` windows.
#[test]
fn t1_regions_match_reference() {
    reference_gate(&PLAIN);
}

/// (debug) both regions == `s4.debug.bin` windows (the +$9C profiling block /
/// the +$C Debug_Scene_Freeze skips).
#[test]
fn t1_debug_regions_match_reference() {
    reference_gate(&DEBUG);
}

// ---- positive control + negative probe (the t24 rule) -------------------

/// The UNDOCTORED compile equals the reference window (positive control), plain
/// ojz_scroll_test — the shape-dependent, VDP-heavy region.
#[test]
fn t1_undoctored_compile_equals_the_reference_window() {
    let shape = &PLAIN;
    let Some(ojz_ref) = ref_window(shape.rom, shape.ojz_base as usize, shape.ojz_len) else {
        return;
    };
    let c = compile(shape);
    assert_eq!(c.ojz, ojz_ref, "undoctored ojz_scroll_test must match the reference window");
}

/// A doctored reference window must NOT match the compiled bytes (the gate can
/// actually fail).
#[test]
fn t1_doctored_reference_diverges() {
    let shape = &PLAIN;
    let Some(mut ots_ref) = ref_window(shape.rom, shape.ots_base as usize, shape.ots_len) else {
        return;
    };
    let c = compile(shape);
    ots_ref[0] ^= 0xFF;
    assert_ne!(c.ots, ots_ref, "a doctored reference must diverge from the compiled bytes");
}
