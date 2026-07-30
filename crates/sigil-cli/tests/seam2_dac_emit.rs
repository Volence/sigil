//! seam-2 step-2a — the DAC bank BODY emit, proven against the reference ROM.
//!
//! `sigil_harness::seam2::emit_dac_banks` lowers the REAL `dac_samples.emp` and
//! places its two `bank:` sections at the CURRENT-baseline pins ($48000 blip /
//! $50000 shared). This gate proves the emitted bank payloads are BYTE-IDENTICAL
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
//! the STALE aeon-f828406 layout ($50000/$58000, SND_BLIP_BANK=$A). This gate
//! uses the CURRENT baseline ($48000/$50000, bank $9/$A — s4.lst / mixed_dac_rom).
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
        eprintln!("skipping seam2_dac_emit (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let banks = sigil_harness::seam2::emit_dac_banks(&aeon).expect("emit_dac_banks");

    // The banks are 2880 B (blip) + 30908 B (shared) at the current baseline.
    assert_eq!(banks.blip.len(), 0xB40, "blip bank length (temp_blip.bin)");
    assert_eq!(banks.shared.len(), 0x78BC, "shared drum bank length (9 .pcm)");

    let rom = std::fs::read(aeon.join("s4.bin")).expect("read reference s4.bin");
    let blip_ref = &rom[0x48000..0x48000 + banks.blip.len()];
    let shared_ref = &rom[0x50000..0x50000 + banks.shared.len()];

    assert_eq!(
        banks.blip, blip_ref,
        "emitted dac_blip_bank must be byte-identical to the reference ROM slice @ $48000"
    );
    assert_eq!(
        banks.shared, shared_ref,
        "emitted dac_shared_bank must be byte-identical to the reference ROM slice @ $50000"
    );

    // The align-$8000 gaps around the banks are zero pad in the reference — the
    // emit carries no trailing pad (asl's `align $8000` produces it), so the
    // bank payloads end exactly where the reference bytes stop being sample data.
    assert!(
        rom[0x48000 + banks.blip.len()..0x50000].iter().all(|&b| b == 0),
        "the blip→shared gap must be zero pad in the reference"
    );
    assert!(
        rom[0x50000 + banks.shared.len()..0x58000].iter().all(|&b| b == 0),
        "the shared→MT gap must be zero pad in the reference"
    );
}

#[test]
fn emit_dac_banks_is_deterministic() {
    if !strict_gate() {
        eprintln!("skipping seam2_dac_emit (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let a = sigil_harness::seam2::emit_dac_banks(&aeon).expect("emit 1");
    let b = sigil_harness::seam2::emit_dac_banks(&aeon).expect("emit 2");
    assert_eq!(a.blip, b.blip, "blip emit must be deterministic");
    assert_eq!(a.shared, b.shared, "shared emit must be deterministic");
}
