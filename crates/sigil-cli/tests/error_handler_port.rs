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
//! (MDDBG__ErrorHandler_PagesController).l`, and the blob's extension-button
//! pointers are `dc.l MDDBG__Debugger_AddressRegisters/Backtrace`. All four are
//! ErrorHandler-relative equs (ErrorHandler = base+0x15A) fed here as synthetic
//! pinned labels; in the full ROM they come from the AS-side mddbg_symbols.asm
//! table (mixed_dac_rom's mixed gate).
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

const STUB_TABLE_LEN: u32 = 0x15A; // BusError..ErrorHandler
const PAGES_OFFSET: u32 = 0xDC6; // ErrorHandler..PagesController

struct Shape {
    base: u32,
    len: usize,
    rom: &'static str,
    debug: i128,
}

const PLAIN: Shape =
    Shape { base: pins::ERROR_HANDLER.plain_base, len: pins::ERROR_HANDLER.plain_len, rom: "s4.bin", debug: 0 };
const DEBUG: Shape =
    Shape { base: pins::ERROR_HANDLER.debug_base, len: pins::ERROR_HANDLER.debug_len, rom: "s4.debug.bin", debug: 1 };

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

    let map = sigil_link::load_map(&map_toml(shape.base, shape.len)).expect("map loads");
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
        &refrom[base..base + shape.len],
        &format!("error_handler vs {}[{base:#x}..{:#x}]", shape.rom, base + shape.len),
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

/// The 12 exception-vector labels flip to `.emp` ownership. vectors.asm (stays
/// `.asm`, out of scope) references all 12 via `dc.l`; with error_handler.asm
/// gated out, those references must resolve to the `.emp`-owned stub labels. This
/// proves the ownership flip in isolation: assemble a synthetic vector table
/// (`dc.l BusError, AddressError, …`) on the AS side, link it against the placed
/// error_handler.emp, and confirm every entry resolves to the `.emp` label's VMA.
/// (Bare-symbol references resolve — unlike the derived-equ table above.)
#[test]
fn vector_labels_resolve_to_emp_ownership() {
    let aeon = aeon_dir();
    if !strict_gate() && !aeon.join("s4.bin").exists() {
        eprintln!("skip: aeon tree not present");
        return;
    }
    const STUBS: &[&str] = &[
        "BusError", "AddressError", "IllegalInstr", "ZeroDivide", "ChkInstr", "TrapvInstr",
        "PrivilegeViol", "Trace", "Line1010Emu", "Line1111Emu", "ErrorExcept", "ErrorTrap",
    ];
    // Lower + place error_handler.emp at the plain base (the flip is shape-
    // independent — vectors.asm spells the same symbols in both shapes).
    let src = std::fs::read_to_string(aeon.join("engine/debug/error_handler.emp"))
        .unwrap_or_else(|e| panic!("read error_handler.emp: {e}"));
    let (file, _) = parse_str(&src);
    let (module, _) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: Some(aeon.join("engine/debug")),
            embed_base: None,
            defines: vec![("DEBUG".to_string(), 0), ("SOUND_DRIVER_ENABLED".to_string(), 1)],
        },
    );
    let map = sigil_link::load_map(&map_toml(PLAIN.base, PLAIN.len)).expect("map loads");
    let mut sections = module.sections;
    place_sections(&mut sections, &map);

    // Synthetic vector table (the vectors.asm dc.l class) referencing the 12
    // stub labels as externs — the AS side that must resolve against the .emp.
    let vec_src = format!(
        "cpu 68000\nphase $1000000\nVecTable:\n\tdc.l {}\n",
        STUBS.join(", ")
    );
    let mut vec_secs = assemble(&vec_src, &AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() })
        .unwrap_or_else(|d| panic!("assemble vectors: {d:?}"))
        .sections;
    for s in vec_secs.iter_mut() {
        s.lma = 0x0200_0000;
        s.placement = SectionPlacement::Pinned;
        s.group = None;
    }
    let vec_name = vec_secs[0].name.clone();
    sections.append(&mut vec_secs);

    // The synthetic handler entry points the stubs' raise_exception jsr/jmp need.
    let eh = PLAIN.base + STUB_TABLE_LEN;
    let mut extra = vec![
        as_label_at("MDDBG__ErrorHandler", eh),
        as_label_at("MDDBG__ErrorHandler_PagesController", eh + PAGES_OFFSET),
        as_label_at("MDDBG__Debugger_AddressRegisters", eh + 0xE6C),
        as_label_at("MDDBG__Debugger_Backtrace", eh + 0xEB8),
    ];
    let mut lma = 0x0300_0000u32;
    for group in &mut extra {
        for sec in group.iter_mut() {
            sec.lma = lma;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        sections.append(group);
        lma += 0x10_0000;
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("flip resolve failed: {d:?}"));
    // The .emp stub label VMAs (the expected resolutions).
    let emp_vma = |want: &str| -> u32 {
        for sec in &resolved {
            if sec.name != "error_handler" {
                continue;
            }
            for l in &sec.labels {
                if l.name == want {
                    return sec.vma_origin().wrapping_add(l.offset);
                }
            }
        }
        panic!("error_handler.emp must export {want}");
    };
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("flip link failed: {d:?}"));
    let vt = linked.section(&vec_name).expect("linked image carries the vector table");
    for (i, name) in STUBS.iter().enumerate() {
        let got = u32::from_be_bytes(vt.bytes[i * 4..i * 4 + 4].try_into().unwrap());
        assert_eq!(
            got,
            emp_vma(name),
            "vector entry {i} (dc.l {name}) must resolve to the .emp-owned label"
        );
    }
}

/// t25 capability note (ledgered demand-1), made executable: sigil does NOT
/// resolve an AS-side `X: equ ExternalSym + const` at link — the derived symbol
/// `X` stays UNRESOLVED even when `ExternalSym` is provided by another module.
/// This is why the MDDBG__ equ table does NOT derive off the `.emp`-owned
/// ErrorHandler; instead engine.inc's gate-ON arm defines a NUMERIC per-shape
/// `ErrorHandler` equ so the table folds at assemble time (the numeric-org
/// derivation — `mixed_dac_rom::mixed_error_handler_rom_matches_assembled_reference`
/// ships the whole-ROM mixed gate on it). The external-base equ remains a
/// recorded demand; when sigil gains it this test FLIPS (the derived equ
/// resolves) and the numeric pin can retire.
#[test]
fn derived_equ_off_external_base_is_unresolved_today() {
    use sigil_ir::SymbolValue;
    let asm = "cpu 68000\nMDDBG__X: equ ErrorHandler+$128\n dc.l MDDBG__X\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let m = assemble(asm, &opts).expect("assemble tolerates the external-base equ (defers)");
    let mut st = SymbolTable::new();
    st.define("ErrorHandler", SymbolValue::Int(0x5CC0A));
    let resolved = sigil_link::resolve_layout(&m.sections, &st, true).expect("resolve_layout");
    let link = sigil_link::link(&resolved, &st);
    assert!(
        link.is_err(),
        "EXPECTED-GAP: a derived equ off an external base does not resolve today; \
         if this now links, the MDDBG__ flip capability arrived — build the flip"
    );
    let err = format!("{:?}", link.unwrap_err());
    assert!(err.contains("MDDBG__X") && err.contains("unresolved"), "the derived symbol is the unresolved one: {err}");
}
