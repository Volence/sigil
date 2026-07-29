//! Tranche 25 — the REAL `error_handler.emp` port, region-level byte gate.
//!
//! Compiles `engine/debug/error_handler.emp` (the 12 exception-vector stubs via
//! the `raise_exception` construct + the vendored MD Debugger blob) through the
//! production parse → lower → place → resolve → link pipeline and asserts the
//! `error_handler` region's flattened bytes equal the reference ROM window at
//! `[BusError, EndOfRom)` in BOTH shapes. Length 0x10B0 both shapes (stub table
//! 0x15A + blob 0xF56) — same size, different base.
//!
//! The `raise_exception` construct emits `jsr (MDDBG__ErrorHandler).l` / `jmp
//! (MDDBG__ErrorHandler_PagesController).l`; those two handler entry points are
//! fed as synthetic pinned labels at their real ErrorHandler-relative addresses
//! (ErrorHandler = base+0x15A; PagesController = ErrorHandler+0xDC6). The blob's
//! own `ErrorHandler+$E6C/$EB8` extension-button pointers resolve against the
//! `.emp`'s own `ErrorHandler` label.
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`). Absent, the
//! gates SKIP green unless `SIGIL_STRICT_GATE=1`.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
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

const REGION_LEN: usize = 0x10B0;
const STUB_TABLE_LEN: u32 = 0x15A; // BusError..ErrorHandler
const PAGES_OFFSET: u32 = 0xDC6; // ErrorHandler..PagesController

struct Shape {
    base: u32,
    rom: &'static str,
    debug: i128,
}

const PLAIN: Shape = Shape { base: 0x5CAB0, rom: "s4.bin", debug: 0 };
const DEBUG: Shape = Shape { base: 0x5E5AA, rom: "s4.debug.bin", debug: 1 };

fn as_label_at(name: &str, vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts)
        .unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}"))
        .sections
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
         name = \"error_handler\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

fn compile(shape: &Shape) -> sigil_link::LinkedImage {
    let aeon = aeon_dir();
    let src = std::fs::read_to_string(aeon.join("engine/debug/error_handler.emp"))
        .unwrap_or_else(|e| panic!("read error_handler.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "error_handler.emp parse errors: {pdiags:?}"
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("engine/debug")),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), shape.debug), ("SOUND_DRIVER_ENABLED".to_string(), 1)],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "error_handler.emp lower errors: {:?}",
        ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );

    let map = sigil_link::load_map(&map_toml(shape.base, REGION_LEN)).expect("map loads");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    // Synthetic handler entry points (the raise_exception jsr/jmp targets).
    let mut lma = 0x0100_0000u32;
    // The raise_exception jsr/jmp targets + the two blob extension-button
    // pointers, all ErrorHandler-relative equs (ErrorHandler = base+0x15A).
    let eh = shape.base + STUB_TABLE_LEN;
    let mut groups = vec![
        as_label_at("MDDBG__ErrorHandler", eh),
        as_label_at("MDDBG__ErrorHandler_PagesController", eh + PAGES_OFFSET),
        as_label_at("MDDBG__Debugger_AddressRegisters", eh + 0xE6C),
        as_label_at("MDDBG__Debugger_Backtrace", eh + 0xEB8),
    ];
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
    sigil_link::link(&resolved, &SymbolTable::new()).unwrap_or_else(|d| panic!("link failed: {d:?}"))
}

fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
    assert_eq!(candidate.len(), expected.len(), "{what}: length mismatch");
    if let Some(i) = (0..candidate.len()).find(|&i| candidate[i] != expected[i]) {
        let lo = i.saturating_sub(8);
        let hi = (i + 16).min(candidate.len());
        panic!(
            "{what}: first diff at region offset {i:#x}\n  candidate[{lo:#x}..{hi:#x}]: {:02x?}\n  expected[{lo:#x}..{hi:#x}]:  {:02x?}",
            &candidate[lo..hi],
            &expected[lo..hi]
        );
    }
}

fn reference_gate(shape: &Shape) {
    let rom_path = aeon_dir().join(shape.rom);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference not at {} (set AEON_DIR)", rom_path.display());
        return;
    };
    let linked = compile(shape);
    let section = linked.section("error_handler").expect("linked image carries error_handler");
    let base = shape.base as usize;
    assert_region_matches(
        &section.bytes,
        &refrom[base..base + REGION_LEN],
        &format!("error_handler vs {}[{base:#x}..{:#x}]", shape.rom, base + REGION_LEN),
    );
}

#[test]
fn error_handler_region_matches_reference() {
    reference_gate(&PLAIN);
}

#[test]
fn error_handler_debug_region_matches_reference() {
    reference_gate(&DEBUG);
}
