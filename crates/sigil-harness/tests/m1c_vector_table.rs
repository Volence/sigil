//! M1.C T10 milestone: prove `sigil-frontend-as` assembles the REAL Aeon
//! `games/sonic4/main.asm` front-matter include tree + the 64-entry vector table
//! byte-exact vs the first 256 bytes of the reference ROM `aeon/s4.bin`.
//!
//! This is the first bounded integration of the front-end against real source:
//! it drives constants.asm, sound_constants.asm, structs.asm, macros.asm,
//! engine/parallax_macros.inc, ram.asm and engine/debug/debugger.asm through the
//! parser, then resolves a `dc.l` vector table whose ~16 external CODE labels are
//! seeded as stubs from the `s4.lst` symbol table (SYSTEM_STACK is a real equate
//! in constants.asm and is NOT stubbed).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree. Absent (e.g. GitHub CI),
//! it SKIPS green — unless SIGIL_STRICT_GATE=1, which turns a missing reference
//! into a hard failure. Mirrors `m1b_gate.rs`.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 cargo test -p sigil-harness --test m1c_vector_table
//! ```

use std::path::{Path, PathBuf};

use sigil_frontend_as::{assemble_root, Options};
use sigil_ir::{Cpu, SymbolTable, SymbolValue};

fn aeon_dir() -> PathBuf {
    PathBuf::from(std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()))
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// External CODE labels the vector table references but the front-matter includes
/// do NOT define. Values are the real ROM addresses from `aeon/s4.lst`. These are
/// seeded BOTH as front-end `-D` defines and as link-time symbols (mirroring the
/// M0 harness's dual stub seeding). SYSTEM_STACK is intentionally absent: it is a
/// genuine equate in constants.asm, so stubbing it would double-define it.
// Addresses re-derived from the current-source `s4.lst` (the tranche-3 step-5
// vdp_init `clr.l` re-baseline: the −4 shrink shifted everything between
// vdp_init and the `org $10000` boundary, so HBlank_Dispatch 0x227E→0x227A and
// VBlank_Handler 0x2156→0x2152; every $64xxx exception target in the
// object-code bank held, as at every prior re-baseline).
// Plain-shape stub VMAs sourced from `sigil_harness::pins` (regenerate via
// `repin`); the IRQ4 $70 entry now targets HBlank_Vector_Slot (the RAM
// trampoline slot at the RAM tail), not a ROM proc.
use sigil_harness::pins;
const STUBS: &[(&str, i64)] = &[
    ("EntryPoint", pins::ENTRY_POINT.plain as i64),
    ("BusError", pins::BUS_ERROR.plain as i64),
    ("AddressError", pins::ADDRESS_ERROR.plain as i64),
    ("IllegalInstr", pins::ILLEGAL_INSTR.plain as i64),
    ("ZeroDivide", pins::ZERO_DIVIDE.plain as i64),
    ("ChkInstr", pins::CHK_INSTR.plain as i64),
    ("TrapvInstr", pins::TRAPV_INSTR.plain as i64),
    ("PrivilegeViol", pins::PRIVILEGE_VIOL.plain as i64),
    ("Trace", pins::TRACE.plain as i64),
    ("Line1010Emu", pins::LINE1010_EMU.plain as i64),
    ("Line1111Emu", pins::LINE1111_EMU.plain as i64),
    ("ErrorExcept", pins::ERROR_EXCEPT.plain as i64),
    ("NullInterrupt", pins::NULL_INTERRUPT.plain as i64),
    ("HBlank_Vector_Slot", pins::H_BLANK_VECTOR_SLOT.plain as i64),
    ("VBlank_Handler", pins::V_BLANK_HANDLER.plain as i64),
    ("ErrorTrap", pins::ERROR_TRAP.plain as i64),
];

#[test]
fn vector_table_matches_reference_rom_first_256_bytes() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.bin");
    let Ok(rom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: aeon/s4.bin");
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };
    assert!(rom.len() >= 256, "reference ROM too small");
    let golden = &rom[0..256];

    // Front-end defines: mirror the real non-debug ASFLAGS from build.sh —
    // SOUND_DRIVER_ENABLED on, __DEBUG__ OFF — plus the external CODE-label stubs.
    let mut defines: Vec<(String, i64)> = vec![("SOUND_DRIVER_ENABLED".to_string(), 1)];
    defines.extend(STUBS.iter().map(|(n, v)| (n.to_string(), *v)));

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("m1c_root.asm");
    let opts = Options {
        initial_cpu: Cpu::M68000,
        defines,
        include_root: Some(aeon.clone()),
    };

    let module = match assemble_root(&root, &opts) {
        Ok(m) => m,
        Err(d) => panic!("assemble: {} diagnostics; first: {:?}", d.len(), d.first()),
    };

    // Seed the link symbol table with the same stubs (fallback for any surviving
    // fixup targets not resolved intra-module).
    let mut stub_table = SymbolTable::new();
    for (name, value) in STUBS {
        stub_table.define(name, SymbolValue::Int(*value));
    }

    let img = sigil_link::link(&module.sections, &stub_table)
        .unwrap_or_else(|d| panic!("link: {} diagnostics; first: {:?}", d.len(), d.first()));

    // The vector table is the single `org 0` M68000 section. Locate the section
    // whose bytes cover the first 256 ROM bytes and compare.
    let sec = img
        .sections
        .iter()
        .find(|s| s.lma == 0 && s.bytes.len() >= 256)
        .unwrap_or_else(|| {
            panic!(
                "no linked section at lma 0 with >=256 bytes; sections: {:?}",
                img.sections.iter().map(|s| (&s.name, s.lma, s.bytes.len())).collect::<Vec<_>>()
            )
        });

    if let Some(i) = (0..256).find(|&i| sec.bytes[i] != golden[i]) {
        panic!(
            "vector table first diff at offset {i:#x}: sigil {:#04x} != golden {:#04x}",
            sec.bytes[i], golden[i]
        );
    }
}
