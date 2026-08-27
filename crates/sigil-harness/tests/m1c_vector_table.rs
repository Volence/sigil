//! M1.C T10 milestone: prove `sigil-frontend-as` assembles the REAL Aeon
//! `games/sonic4/main.asm` front-matter include tree + the 64-entry vector table
//! byte-exact vs the first 256 bytes of the reference ROM `aeon/s4.bin`.
//!
//! This is the first bounded integration of the front-end against real source:
//! it drives sound_bank.inc, header.inc and engine/debug/debugger.asm through the
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
#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
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
// The RELEASE (plain) vector table routes every fault at the 12 error_handler
// per-class stubs since the crash-report ruling (owner-ruled 2026-08-04): the island
// SHIPS in release, so the plain table names the same targets the debug one does and
// `ReleaseFault` is gone from every canonical listing (it is the LEAN shape's handler,
// and has no pin). The plain replica (m1c_root.asm) therefore references
// EntryPoint, the 12 stubs, HBlank_Vector_Slot and VBlank_Handler. SYSTEM_STACK stays
// unstubbed (a genuine constants.emp equate).
//
// PLAIN STUB ADDRESSES: the 12 stub pins are `debug_only` in repin.toml (bare `u32`,
// the debug value) — the per-shape island BASE is carried once by the `error_handler`
// REGION pin instead of being duplicated across 24 numbers. error_handler.emp has no
// `if DEBUG` in it at all, so the island's internal layout is shape-invariant and a
// stub's plain address is `ERROR_HANDLER.plain_base + (pin - BUS_ERROR)`; `BUS_ERROR`
// IS the island head, so the expression is the identity in the debug shape.
fn stubs() -> Vec<(&'static str, i64)> {
    let rebase =
        |pin: u32| -> i64 { (pins::ERROR_HANDLER.plain_base + (pin - pins::BUS_ERROR)) as i64 };
    vec![
        ("EntryPoint", pins::ENTRY_POINT.plain as i64),
        ("HBlank_Vector_Slot", pins::H_BLANK_VECTOR_SLOT.plain as i64),
        ("VBlank_Handler", pins::V_BLANK_HANDLER.plain as i64),
        ("BusError", rebase(pins::BUS_ERROR)),
        ("AddressError", rebase(pins::ADDRESS_ERROR)),
        ("IllegalInstr", rebase(pins::ILLEGAL_INSTR)),
        ("ZeroDivide", rebase(pins::ZERO_DIVIDE)),
        ("ChkInstr", rebase(pins::CHK_INSTR)),
        ("TrapvInstr", rebase(pins::TRAPV_INSTR)),
        ("PrivilegeViol", rebase(pins::PRIVILEGE_VIOL)),
        ("Trace", rebase(pins::TRACE)),
        ("Line1010Emu", rebase(pins::LINE1010_EMU)),
        ("Line1111Emu", rebase(pins::LINE1111_EMU)),
        ("ErrorExcept", rebase(pins::ERROR_EXCEPT)),
        ("ErrorTrap", rebase(pins::ERROR_TRAP)),
    ]
}

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
    let stubs = stubs();
    let mut defines: Vec<(String, i64)> = vec![("SOUND_DRIVER_ENABLED".to_string(), 1)];
    defines.extend(stubs.iter().map(|(n, v)| (n.to_string(), *v)));

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("m1c_root.asm");
    // Post the Stage-3 P5 ownership flip, `constants.asm` no longer defines the
    // 114 engine constants — `constants.emp` is their sole author and the build
    // injects them as GUARDED AS `-D` defines so residual AS reads them at
    // comptime. Seed the same harvested guarded defines here, exactly as the real
    // harness does, so this standalone front-matter assembly resolves them.
    let mut guarded_defines = sigil_harness::native::harvest_engine_constants(&aeon)
        .expect("harvest engine constants");
    // Post the conv-a structs flip, structs.asm is gone too — ram.asm's `ds`
    // slot sizes (SST_len/DMAEntry_len/…) come from the struct-offset harvest.
    guarded_defines.extend(
        sigil_harness::native::harvest_engine_struct_offsets(&aeon)
            .expect("harvest struct offsets"),
    );
    // Post conv-f, games/sonic4/config/constants.asm is gone too — the game
    // constants (GS_OJZ_SCROLL_TEST feeding game.asm's GAME_ENTRY_ID, etc.) are
    // authored in games/sonic4/config/constants.emp and harvested the same way.
    guarded_defines.extend(
        sigil_harness::native::harvest_game_constants(
            &aeon,
            "games/sonic4/config/constants.emp",
            false,
        )
        .expect("harvest game constants"),
    );
    // Parcel F2: the game's sound ids (song/SFX ids + priority ladder) are authored
    // in games/sonic4/config/sound_ids.emp and harvested the same way (plain shape).
    guarded_defines.extend(
        sigil_harness::native::harvest_game_constants(
            &aeon,
            "games/sonic4/config/sound_ids.emp",
            false,
        )
        .expect("harvest game sound ids"),
    );
    // Parcel F2: the SFX-bank id counts (SFX_TABLE_LEN, read by sound_bank.inc's
    // span guard) are DERIVED in sfx_bank.emp and harvested the same way.
    guarded_defines.extend(
        sigil_harness::native::harvest_game_constants(
            &aeon,
            "games/sonic4/data/sound/sfx/sfx_bank.emp",
            false,
        )
        .expect("harvest sfx bank counts"),
    );
    // Item #7b + #7c: engine/ram.asm AND games/sonic4/config/ram.asm are retired —
    // the RAM labels the front-matter needs (HBlank_Vector_Slot and any game RAM
    // label) come from the engine+game ram.emp address harvest seeded as PLAIN -D
    // defines, exactly as the real build's assemble_as_side does. `sonic4_profile`
    // carries `game_ram_module = "games.sonic4.ram"`, so the harvest reaches the
    // game RAM region too (non-debug shape here).
    defines.extend(
        sigil_harness::native::harvest_engine_ram_addresses(
            &aeon,
            &sigil_harness::native::sonic4_profile(false),
        )
        .expect("harvest ram addresses"),
    );
    let opts = Options {
        initial_cpu: Cpu::M68000,
        defines,
        include_root: Some(aeon.clone()),
        guarded_defines,
    };

    let module = match assemble_root(&root, &opts) {
        Ok(m) => m,
        Err(d) => panic!("assemble: {} diagnostics; first: {:?}", d.len(), d.first()),
    };

    // Seed the link symbol table with the same stubs (fallback for any surviving
    // fixup targets not resolved intra-module).
    let mut stub_table = SymbolTable::new();
    for (name, value) in &stubs {
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
