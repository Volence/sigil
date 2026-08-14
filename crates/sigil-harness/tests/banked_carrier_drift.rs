//! seam-1's banked `$8000`-window carrier VMAs must agree with the seam-2
//! derivation (lens sweep, seat LINK, finding S9).
//!
//! `seam1::banked_carriers` is a hand-maintained `(symbol, VMA)` literal table
//! that is injected as equ carriers into the seam-1 link producing the SHIPPED
//! resident Z80 blob — so the literals end up in the driver's operand bytes.
//! `seam2::sound_layout` derives the same addresses correctly from the map
//! authority the placement flows from, and nothing compared the two.
//!
//! The failure mode is specific: the next SFX id-range growth moves the derived
//! head, leaves the literal stale, breaks the whole-ROM golden byte gate — and the
//! natural remediation, a refreeze, blesses the WRONG blob. seam-1's own comment
//! records the `0x856D -> 0x8571` bump when `$BA`/`$BB` widened the win table,
//! i.e. that move having already happened once and been caught by hand.
//!
//! `native.rs` states the rule: "a second copy of this arithmetic is the bug, not
//! the fix — that is the lesson of the three unmaintained copies of the sound-bank
//! addresses." The copy stays (the carriers must be literals at the link), but it
//! is now checked on every blob emission, and here.
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()))
}
fn strict() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

#[test]
fn banked_carriers_agree_with_the_seam2_derivation() {
    let aeon = aeon_dir();
    if !aeon.join("engine/sound/z80_sound_driver.emp").exists() {
        assert!(!strict(), "SIGIL_STRICT_GATE set but aeon tree not at {}", aeon.display());
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    sigil_harness::seam1::check_banked_carrier_drift(&aeon)
        .expect("the pinned banked-head VMAs must equal the derived ones");
}

/// The derivation is not vacuous: it must actually produce the three head members,
/// at plausible `$8000`-window addresses in head order. A derivation that returned
/// an empty list would make the drift check above pass for free.
#[test]
fn the_derivation_produces_the_three_head_members_in_order() {
    let aeon = aeon_dir();
    if !aeon.join("engine/sound/z80_sound_driver.emp").exists() {
        assert!(!strict(), "SIGIL_STRICT_GATE set but aeon tree not at {}", aeon.display());
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    let vmas = sigil_harness::seam2::banked_head_vmas(&aeon).expect("derivation must succeed");
    let names: Vec<&str> = vmas.iter().map(|(n, _)| *n).collect();
    assert_eq!(names, ["SndDefaultPitchTable", "SfxBlobWinTab", "SeqOpcodeTable"]);
    // Head order is the layout's own order, and every member sits in the window.
    let mut prev = 0x8000u32;
    for (name, vma) in &vmas {
        assert!(
            (0x8000..0xA000).contains(vma),
            "{name} at {vma:#06x} is outside the $8000 window — the derivation is not producing a VMA"
        );
        assert!(*vma > prev, "{name} at {vma:#06x} must follow the previous head member ({prev:#06x})");
        prev = *vma;
    }
}
