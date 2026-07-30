//! seam-2 stage-2b Option Y — the DAC descriptor HEAD, co-linked and proven
//! against the reference ROM (the twins-present dual proof, both shapes).
//!
//! `sigil_harness::seam2::emit_dac_body_and_head` CO-LINKS the REAL `dac_samples.emp`
//! (bank bodies + the `SND_*` equ carrier) with the REAL `dac_sample_tab.emp` (the
//! phased head) in one link, with NO `-D`: the head's `dc.b SND_KICK_BANK` / `dc.w
//! SND_KICK_PTR` / `dc.w SND_KICK_LEN` cells resolve as CROSS-MODULE link symbols
//! against `dac_samples.emp`'s `SND_*` equs (which fold same-module from
//! `bankid`/`winptr`/`.len`). The `SND_*` names live ONCE, at the producer.
//!
//! This gate proves the co-linked head is BYTE-IDENTICAL to the `DacSampleTable`
//! slice of the assembled reference ROM (`s4.bin` @ `$585AD`, 90 bytes) — the
//! "twins present, both paths byte-identical" dual proof that must be GREEN before
//! `dac_samples.asm` + `dac_sample_tab.asm` can be retired together (rows 5-dac + 57).
//!
//! t24 head-shape control: the head is SHAPE-INVARIANT (`DacSampleTable` sits at the
//! same VMA `$85AD` / LMA `$585AD` in `s4.lst` AND `s4.debug.lst`, and the reference
//! bytes there are byte-identical plain/debug — the `SND_*` fold does not move with
//! `__DEBUG__`). So one head serves both shapes; this test asserts against BOTH
//! reference ROMs to prove it.
//!
//! ADDITIVE: no build.sh / main.asm change, no `.asm` deletion — the assembled-ROM
//! provenance is UNCHANGED by this stage.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test seam2_dac_head_colink
//! ```

use sigil_harness::seam2::{emit_dac_body_and_head, DAC_SAMPLE_TAB_LEN, DAC_SAMPLE_TAB_LMA};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// THE HEAD BYTE GATE: the co-linked `DacSampleTable` == the reference ROM slice,
/// in BOTH shapes (the head is shape-invariant, so the same 90 bytes match both).
#[test]
fn colinked_dac_head_matches_the_reference_rom_slice_both_shapes() {
    if !strict_gate() {
        eprintln!("skipping seam2_dac_head_colink (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let out = emit_dac_body_and_head(&aeon).expect("emit_dac_body_and_head co-links");

    assert_eq!(out.head.len(), DAC_SAMPLE_TAB_LEN, "DacSampleTable is 10 × 9 = 90 bytes");

    // The head is shape-invariant: gate against BOTH reference ROMs at the same LMA.
    for (rom_name, shape) in [("s4.bin", "plain"), ("s4.debug.bin", "debug")] {
        let rom = std::fs::read(aeon.join(rom_name)).unwrap_or_else(|e| panic!("read {rom_name}: {e}"));
        let lo = DAC_SAMPLE_TAB_LMA as usize;
        let head_ref = &rom[lo..lo + DAC_SAMPLE_TAB_LEN];
        if let Some(i) = (0..out.head.len()).find(|&i| out.head[i] != head_ref[i]) {
            let d = i / 9; // descriptor index
            panic!(
                "co-linked head differs from {shape} reference @ descriptor {d} byte {}: \
                 emp {:#04x} vs rom {:#04x}\n  emp[{d}]:  {:02x?}\n  rom[{d}]:  {:02x?}",
                i % 9, out.head[i], head_ref[i],
                &out.head[d * 9..d * 9 + 9], &head_ref[d * 9..d * 9 + 9],
            );
        }
        assert_eq!(out.head, head_ref, "co-linked DacSampleTable must equal the {shape} reference slice @ $585AD");
    }
}

/// The banks the SAME co-link produces still match their reference slices — the
/// co-link does not perturb the bank bodies (they place at $48000/$50000 as before).
#[test]
fn colink_banks_still_match_reference() {
    if !strict_gate() {
        eprintln!("skipping seam2_dac_head_colink (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let out = emit_dac_body_and_head(&aeon).expect("emit_dac_body_and_head co-links");
    let rom = std::fs::read(aeon.join("s4.bin")).expect("read s4.bin");
    assert_eq!(out.blip, &rom[0x48000..0x48000 + out.blip.len()], "blip bank @ $48000");
    assert_eq!(out.shared, &rom[0x50000..0x50000 + out.shared.len()], "shared bank @ $50000");
}

/// Determinism: the co-link is byte-stable across runs (tracked `.emp` + `.pcm` +
/// toolchain), so the emitted head/banks are a provenance-trackable artifact.
#[test]
fn colink_is_deterministic() {
    if !strict_gate() {
        eprintln!("skipping seam2_dac_head_colink (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let a = emit_dac_body_and_head(&aeon).expect("emit 1");
    let b = emit_dac_body_and_head(&aeon).expect("emit 2");
    assert_eq!(a.head, b.head, "head emit must be deterministic");
    assert_eq!(a.blip, b.blip, "blip emit must be deterministic");
    assert_eq!(a.shared, b.shared, "shared emit must be deterministic");
}
