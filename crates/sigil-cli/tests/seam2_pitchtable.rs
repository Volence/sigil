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

use sigil_harness::seam2::{
    emit_pitchtable, emit_pitchtable_artifacts, emit_pitchtable_doctored, sound_layout,
    PITCHTABLE_LEN,
};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    sigil_harness::test_support::aeon_dir()
}
#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}
/// The FROZEN golden slice comparand (the asl-witnessed reference), NOT the live
/// tree ROM — post-flip `aeon/s4.bin` is itself sigil-built (row-91 bar b).
fn golden(name: &str) -> Vec<u8> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../sigil-harness/golden/{name}"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("read golden {}: {e}", path.display()))
}

/// THE HEAD BYTE GATE: the emitted `movingtrucks_pitchtable` == the reference ROM
/// slice at `$58357`, in BOTH shapes (shape-invariant, so the same 264 bytes match
/// both).
#[test]
fn pitchtable_matches_the_reference_rom_slice_both_shapes() {
    if !strict_gate() {
        eprintln!("skip: seam2_pitchtable not measured (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let out = emit_pitchtable(&aeon).expect("emit_pitchtable");
    assert_eq!(out.len(), PITCHTABLE_LEN, "movingtrucks_pitchtable is 264 bytes (2*132)");
    let lma = sound_layout(&aeon).expect("sound_layout derives pitchtable_lma").pitchtable_lma;
    for rom_name in ["s4.bin", "s4.debug.bin"] {
        let rom = golden(rom_name);
        let lo = lma as usize;
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

/// t24 NON-VACUITY control (row-91 bar c): a doctored composition — the first
/// page-0 data cell edited `$00 → $01` at the `.emp` source, recomposed — must
/// make the table DIVERGE from the golden slice. The table is pure `dc.b` with a
/// 1:1 source→output map (no fold, no placement sensitivity), so the AS-side size
/// guard only catches LENGTH drift; this proves the byte gate catches CONTENT
/// drift. The gate is vacuous if a changed source cell still matches.
#[test]
fn pitchtable_diverges_when_source_cell_doctored() {
    if !strict_gate() {
        eprintln!("skip: seam2_pitchtable not measured (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let doctored = emit_pitchtable_doctored(&aeon, true).expect("doctored recompose");
    let rom = golden("s4.bin");
    let lo = sound_layout(&aeon).expect("sound_layout").pitchtable_lma as usize;
    let refslice = &rom[lo..lo + PITCHTABLE_LEN];
    assert_ne!(
        doctored, refslice,
        "the pitchtable gate is vacuous if a doctored source cell still matches the golden slice"
    );
}

/// Determinism + the emitted `.bin` matches the in-memory emit.
#[test]
fn emit_pitchtable_artifacts_writes_reference_bin() {
    if !strict_gate() {
        eprintln!("skip: seam2_pitchtable not measured (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let dir = tempfile::tempdir().expect("tempdir");
    emit_pitchtable_artifacts(&aeon, dir.path()).expect("emit_pitchtable_artifacts");
    let mem = emit_pitchtable(&aeon).expect("emit");
    let got = std::fs::read(dir.path().join("movingtrucks_pitchtable.bin")).expect("read bin");
    assert_eq!(got, mem, "emitted movingtrucks_pitchtable.bin must equal the in-memory emit (== reference)");
}
