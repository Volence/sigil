//! seam-2 stage-3 — the `sound_tables_z80` head, proven against the reference
//! ROM (the row-1619 identity bar). The generated FM/PSG data tables — 4
//! pure-math LUTs (FmPitchTableZ / PsgDivisorTableZ / LogVolumeLutZ /
//! CarrierMaskTableZ) + the PSG/FM vol-env id-lists, pointer tables, and bodies.
//!
//! `sigil_harness::seam2::emit_sound_tables_z80` lowers the REAL
//! `engine/sound/sound_tables_z80.emp` placed at VMA `$8000`, so its intra-module
//! `dc.w PsgVolEnv_XX`/`FmVolEnv_XX` pointer cells resolve to their `$8000`-window
//! addresses. SELF-CONTAINED (no external symbols — the pointer cells reference
//! this module's own body labels). The table is proven BYTE-IDENTICAL to the
//! reference ROM slice (`$58000`, 855 bytes).
//!
//! SHAPE-INVARIANT (pure-math LUTs + fixed vol-env data; 855 bytes both shapes),
//! so one emission serves both — gated against BOTH reference ROMs.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test seam2_soundtables_colink
//! ```

use sigil_harness::seam2::{
    emit_sound_tables_artifacts, emit_sound_tables_z80, emit_sound_tables_z80_doctored,
    SOUND_TABLES_Z80_LEN, SOUND_TABLES_Z80_LMA,
};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}
/// The FROZEN golden slice comparand (the asl-witnessed reference), NOT the live
/// tree ROM — post-flip `aeon/s4.bin` is itself sigil-built (row-91 bar b).
fn golden(name: &str) -> Vec<u8> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../sigil-harness/golden/{name}"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("read golden {}: {e}", path.display()))
}

/// THE HEAD BYTE GATE: the emitted `sound_tables_z80` == the reference ROM slice
/// at `$58000`, in BOTH shapes (shape-invariant, so the same 855 bytes match both).
#[test]
fn sound_tables_z80_matches_the_reference_rom_slice_both_shapes() {
    if !strict_gate() {
        eprintln!("skipping seam2_soundtables_colink (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let out = emit_sound_tables_z80(&aeon).expect("emit_sound_tables_z80");
    assert_eq!(out.len(), SOUND_TABLES_Z80_LEN, "sound_tables_z80 is 855 bytes ($357)");
    for rom_name in ["s4.bin", "s4.debug.bin"] {
        let rom = golden(rom_name);
        let lo = SOUND_TABLES_Z80_LMA as usize;
        let refslice = &rom[lo..lo + SOUND_TABLES_Z80_LEN];
        if let Some(i) = (0..out.len()).find(|&i| out[i] != refslice[i]) {
            panic!(
                "sound_tables_z80 differs from {rom_name} @ byte {i:#x}: emp {:#04x} vs rom {:#04x}\n  emp: {:02x?}\n  rom: {:02x?}",
                out[i], refslice[i],
                &out[i.saturating_sub(4)..(i + 4).min(out.len())],
                &refslice[i.saturating_sub(4)..(i + 4).min(refslice.len())],
            );
        }
        assert_eq!(out, refslice, "sound_tables_z80 must equal the {rom_name} reference @ $58000");
    }
}

/// t24 NON-VACUITY control (row-91 bar c): a doctored composition — the section's
/// `$8000` window base moved to `$9000` — must make the table DIVERGE from the
/// golden slice, because every intra-module `dc.w PsgVolEnv_XX`/`FmVolEnv_XX`
/// pointer cell re-folds to the moved window. The byte-exact `$8000`-based layout
/// is load-bearing (the resident writers read these labels through the window);
/// the gate is vacuous if a moved window still matches.
#[test]
fn sound_tables_diverge_when_window_moved() {
    if !strict_gate() {
        eprintln!("skipping seam2_soundtables_colink (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let doctored = emit_sound_tables_z80_doctored(&aeon, Some(0x9000)).expect("doctored emit");
    let rom = golden("s4.bin");
    let lo = SOUND_TABLES_Z80_LMA as usize;
    let refslice = &rom[lo..lo + SOUND_TABLES_Z80_LEN];
    assert_ne!(
        doctored, refslice,
        "the sound_tables_z80 gate is vacuous if a moved $8000 window still matches the golden slice"
    );
}

/// Determinism + the emitted `.bin` matches the in-memory emit.
#[test]
fn emit_sound_tables_artifacts_writes_reference_bin() {
    if !strict_gate() {
        eprintln!("skipping seam2_soundtables_colink (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let dir = tempfile::tempdir().expect("tempdir");
    emit_sound_tables_artifacts(&aeon, dir.path()).expect("emit_sound_tables_artifacts");
    let mem = emit_sound_tables_z80(&aeon).expect("emit");
    let got = std::fs::read(dir.path().join("sound_tables_z80.bin")).expect("read bin");
    assert_eq!(got, mem, "emitted sound_tables_z80.bin must equal the in-memory emit (== reference)");
}
