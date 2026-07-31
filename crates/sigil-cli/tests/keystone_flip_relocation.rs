//! Stage-3 P1b — the keystone-flip relocation regression + fold-vs-placement validator.
//!
//! The row-94 bug: with the three player keystones flipped into the config_a chained
//! set, the residual-AS parallax `dc.l DeformTable_Zero` (`P_DBG := deformBg` in
//! engine/parallax_macros.inc) BAKED a stale this-pass VMA (0x112F4) instead of
//! relocating to DeformTable_Zero's placed base (0x11300) — because config_a's residual
//! converges poison-free, so the (poison-gated) deferral pass that keeps section labels
//! symbolic was SKIPPED. The fix runs that deferral pass unconditionally.
//!
//! `frozen_placement_mismatches` is LABELED-only, so it returned EMPTY through this bug
//! (DeformTable_Zero WAS placed at 0x11300 — the stale value was in a REFERENCE, not the
//! label). This file closes that blind spot with a fold-vs-placement check + t24.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test keystone_flip_relocation
//! ```
use sigil_harness::native;
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()))
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}
fn have_aeon(aeon: &PathBuf) -> bool {
    if aeon.join("s4.bin").exists() {
        return true;
    }
    if strict_gate() {
        panic!("SIGIL_STRICT_GATE set but aeon tree missing at {}", aeon.display());
    }
    eprintln!("skip: aeon tree not present (set AEON_DIR)");
    false
}
fn golden() -> Vec<u8> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sigil-harness/golden/config_a.bin");
    std::fs::read(&p).unwrap_or_else(|e| panic!("golden config_a.bin: {e}"))
}
fn is_header_field(i: usize) -> bool {
    sigil_harness::CHECKSUM_FIELD_RANGE.contains(&i) || sigil_harness::ROM_END_FIELD_RANGE.contains(&i)
}

// The chained build touches the shared engine/sound/generated dir — serialize.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

const EOR: usize = 0x5f5f2; // config_a assembled_end (== config_a_profile; −0x68 wave-c)
const DEFORM_PTR_OFF: usize = 0x11410; // ParallaxConfig_OJZ_Default header +0x10: dc.l DeformTable_Zero
const DEFORM_DIVERGENT_BYTE: usize = 0x11412; // the byte that went stale (0x12 vs 0x13) — the row-94 site

/// POSITIVE: the keystones-flipped config_a assembled anchor is byte-identical to the
/// golden — the residual parallax pointer relocates to DeformTable_Zero's placed base.
/// Determinism: a second build is identical.
#[test]
fn flipped_config_a_anchor_matches_golden() {
    let aeon = aeon_dir();
    if !have_aeon(&aeon) {
        return;
    }
    let _lk = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let p = native::config_a_keystones_flipped_profile();
    let rom = native::build_rom_chained(&aeon, &p).unwrap_or_else(|e| panic!("{e}"));
    let rom2 = native::build_rom_chained(&aeon, &p).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(rom, rom2, "flipped config_a: build non-deterministic");
    assert_eq!(rom.len(), EOR, "flipped config_a: assembled length");
    let g = golden();
    let bad: Vec<usize> = (0..EOR).filter(|&i| !is_header_field(i) && rom[i] != g[i]).collect();
    assert!(
        bad.is_empty(),
        "flipped config_a anchor diverges from golden at {} offset(s); first {:#x} (sig {:#04x} != gold {:#04x})",
        bad.len(),
        bad.first().copied().unwrap_or(0),
        bad.first().map(|&i| rom[i]).unwrap_or(0),
        bad.first().map(|&i| g[i]).unwrap_or(0),
    );
}

/// t24 NEGATIVE (non-vacuity): the byte compare is LIVE at exactly the previously-blind
/// site — a golden doctored at 0x11412 (the deform pointer) is caught. Proves the
/// positive gate is not passing vacuously over the fold-vs-placement site.
#[test]
fn doctored_golden_at_deform_pointer_is_caught() {
    let aeon = aeon_dir();
    if !have_aeon(&aeon) {
        return;
    }
    let _lk = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let p = native::config_a_keystones_flipped_profile();
    let rom = native::build_rom_chained(&aeon, &p).unwrap_or_else(|e| panic!("{e}"));
    let mut g = golden();
    g[DEFORM_DIVERGENT_BYTE] ^= 0xFF; // doctor the exact byte that went stale in the bug
    let bad: Vec<usize> = (0..EOR).filter(|&i| !is_header_field(i) && rom[i] != g[i]).collect();
    assert!(
        bad.contains(&DEFORM_DIVERGENT_BYTE),
        "t24: doctoring the golden at {:#x} was NOT caught — the gate is vacuous there",
        DEFORM_DIVERGENT_BYTE
    );
}

/// FOLD-vs-PLACEMENT VALIDATOR (closes the frozen_placement_mismatches blind spot): the
/// emitted parallax `dc.l DeformTable_Zero` at 0x11412 EQUALS DeformTable_Zero's placed
/// VMA read from the same build's listing. A baked-stale reference (the row-94 bug)
/// makes these disagree even while `frozen_placement_mismatches` stays empty.
#[test]
fn deform_pointer_equals_placed_label_vma() {
    let aeon = aeon_dir();
    if !have_aeon(&aeon) {
        return;
    }
    let _lk = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let p = native::config_a_keystones_flipped_profile();
    let (rom, listing) =
        native::build_rom_chained_with_listing(&aeon, &p).unwrap_or_else(|e| panic!("{e}"));
    let placed = listing
        .iter()
        .find(|s| s.name == "DeformTable_Zero")
        .map(|s| s.value)
        .expect("DeformTable_Zero in listing");
    let emitted = u32::from_be_bytes([
        rom[DEFORM_PTR_OFF],
        rom[DEFORM_PTR_OFF + 1],
        rom[DEFORM_PTR_OFF + 2],
        rom[DEFORM_PTR_OFF + 3],
    ]);
    // Sanity: labeled placement itself is correct (the blind spot — this was already
    // true through the bug), so this check is strictly stronger.
    assert!(
        native::frozen_placement_mismatches(&aeon, &p).unwrap().is_empty(),
        "labeled placement regressed (independent of the fold check)"
    );
    assert_eq!(
        emitted, placed,
        "fold-vs-placement divergence: dc.l @{:#x} emitted {:#x} but DeformTable_Zero is placed at {:#x} (a baked-stale reference)",
        DEFORM_PTR_OFF, emitted, placed
    );
}
