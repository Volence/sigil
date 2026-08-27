//! seam-2 step-2a — the DAC bank BODY emit, proven against the reference ROM.
//!
//! `sigil_harness::seam2::emit_dac_banks` lowers the REAL `dac_samples.emp` and
//! places its two `bank:` sections at the map-derived bank LMAs (`seam2::sound_layout`
//! — the `dac_banks` anchor and the shared bank one window above it; `$90000`/`$98000`
//! since aeon's 2026-08-26 re-layout, `$48000`/`$50000` before it). The slices below
//! are DERIVED from that layout, never retyped. This gate proves the emitted bank payloads are BYTE-IDENTICAL
//! to the corresponding slices of the assembled reference ROM (`aeon/s4.bin`) —
//! the "twins present, both paths byte-identical" dual proof that must be GREEN
//! before `dac_samples.asm` can be retired (the Option-A canonicalization: emit
//! the bank .bin, asl BINCLUDEs it, delete the .asm).
//!
//! This is ADDITIVE: no build.sh / main.asm change, no `.asm` deletion, so the
//! assembled-ROM provenance is UNCHANGED by this stage. The emit is the artifact
//! a later stage wires into the build (behind the mixed_dac_rom gate).
//!
//! Distinct from `dac_port.rs`, which is a self-consistency WINDOWED oracle at
//! a self-consistent synthetic layout. This gate uses the live map's layout.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test seam2_dac_emit
//! ```

use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

#[test]
fn emitted_dac_banks_match_the_reference_rom_slices() {
    if !strict_gate() {
        eprintln!("skip: seam2_dac_emit not measured (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let banks = sigil_harness::seam2::emit_dac_banks(&aeon).expect("emit_dac_banks");

    // The banks are 2880 B (blip) + 30908 B (shared) at the current baseline.
    assert_eq!(banks.blip.len(), 0xB40, "blip bank length (temp_blip.bin)");
    assert_eq!(banks.shared.len(), 0x78BC, "shared drum bank length (9 .pcm)");

    let l = sigil_harness::seam2::sound_layout(&aeon).expect("sound_layout derives the bank LMAs");
    let (blip, shared, head) =
        (l.dac_blip_lma as usize, l.dac_shared_lma as usize, l.sound_tables_z80_lma as usize);
    let rom = std::fs::read(aeon.join("s4.bin")).expect("read reference s4.bin");
    let blip_ref = &rom[blip..blip + banks.blip.len()];
    let shared_ref = &rom[shared..shared + banks.shared.len()];

    assert_eq!(
        banks.blip, blip_ref,
        "emitted dac_blip_bank must be byte-identical to the reference ROM slice @ {blip:#X}"
    );
    assert_eq!(
        banks.shared, shared_ref,
        "emitted dac_shared_bank must be byte-identical to the reference ROM slice @ {shared:#X}"
    );

    // The align-$8000 gaps around the banks are zero pad in the reference — the
    // emit carries no trailing pad (asl's `align $8000` produces it), so the
    // bank payloads end exactly where the reference bytes stop being sample data.
    assert!(
        rom[blip + banks.blip.len()..shared].iter().all(|&b| b == 0),
        "the blip→shared gap must be zero pad in the reference"
    );
    assert!(
        rom[shared + banks.shared.len()..head].iter().all(|&b| b == 0),
        "the shared→head-bank gap must be zero pad in the reference"
    );
}

#[test]
fn emit_dac_banks_is_deterministic() {
    if !strict_gate() {
        eprintln!("skip: seam2_dac_emit not measured (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let a = sigil_harness::seam2::emit_dac_banks(&aeon).expect("emit 1");
    let b = sigil_harness::seam2::emit_dac_banks(&aeon).expect("emit 2");
    assert_eq!(a.blip, b.blip, "blip emit must be deterministic");
    assert_eq!(a.shared, b.shared, "shared emit must be deterministic");
}
