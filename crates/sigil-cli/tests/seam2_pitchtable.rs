//! flip Stage-0 — the `movingtrucks_pitchtable` banked head, proven against the
//! reference ROM. The `SndDefaultPitchTable` / `MovingTrucks_PitchTable` fallback
//! pitch table (the exact Zyrinx "Moving Trucks" 132-entry two-page chromatic fnum
//! table) — the LAST AS sound head to go native.
//!
//! `sigil_harness::seam2::emit_pitchtable` lowers the REAL
//! `games/sonic4/data/sound/movingtrucks_pitchtable.emp` placed at VMA `$8357`.
//! SELF-CONTAINED — pure `dc.b` data, no external symbols and no intra-module
//! references (the labels are provided by `sound_bank.inc`'s AS side ahead of the
//! BINCLUDE). Proven BYTE-IDENTICAL to the reference ROM slice (`$58357`, 264
//! bytes).
//!
//! SHAPE-INVARIANT (fixed data; 264 bytes both shapes), so one emission serves
//! both — gated against BOTH reference ROMs.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test seam2_pitchtable
//! ```

use sigil_harness::seam2::{emit_pitchtable, emit_pitchtable_artifacts, PITCHTABLE_LEN, PITCHTABLE_LMA};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// THE HEAD BYTE GATE: the emitted `movingtrucks_pitchtable` == the reference ROM
/// slice at `$58357`, in BOTH shapes (shape-invariant, so the same 264 bytes match
/// both).
#[test]
fn pitchtable_matches_the_reference_rom_slice_both_shapes() {
    if !strict_gate() {
        eprintln!("skipping seam2_pitchtable (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let out = emit_pitchtable(&aeon).expect("emit_pitchtable");
    assert_eq!(out.len(), PITCHTABLE_LEN, "movingtrucks_pitchtable is 264 bytes (2*132)");
    for rom_name in ["s4.bin", "s4.debug.bin"] {
        let rom = std::fs::read(aeon.join(rom_name)).unwrap_or_else(|e| panic!("read {rom_name}: {e}"));
        let lo = PITCHTABLE_LMA as usize;
        let refslice = &rom[lo..lo + PITCHTABLE_LEN];
        if let Some(i) = (0..out.len()).find(|&i| out[i] != refslice[i]) {
            panic!(
                "movingtrucks_pitchtable differs from {rom_name} @ byte {i:#x}: emp {:#04x} vs rom {:#04x}\n  emp: {:02x?}\n  rom: {:02x?}",
                out[i], refslice[i],
                &out[i.saturating_sub(4)..(i + 4).min(out.len())],
                &refslice[i.saturating_sub(4)..(i + 4).min(refslice.len())],
            );
        }
        assert_eq!(out, refslice, "movingtrucks_pitchtable must equal the {rom_name} reference @ $58357");
    }
}

/// Determinism + the emitted `.bin` matches the in-memory emit.
#[test]
fn emit_pitchtable_artifacts_writes_reference_bin() {
    if !strict_gate() {
        eprintln!("skipping seam2_pitchtable (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let dir = tempfile::tempdir().expect("tempdir");
    emit_pitchtable_artifacts(&aeon, dir.path()).expect("emit_pitchtable_artifacts");
    let mem = emit_pitchtable(&aeon).expect("emit");
    let got = std::fs::read(dir.path().join("movingtrucks_pitchtable.bin")).expect("read bin");
    assert_eq!(got, mem, "emitted movingtrucks_pitchtable.bin must equal the in-memory emit (== reference)");
}
