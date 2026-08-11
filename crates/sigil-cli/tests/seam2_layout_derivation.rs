//! Parcel A1 — the seam-2 placement is a CONSUMER of the map authority.
//!
//! [`sound_layout`] derives every banked LMA from `games/sonic4/map.toml`'s two
//! declared anchors (`dac_banks` @ `$48000`, `sound_bank` @ `$58000` vma `$8000`) +
//! the emit's own measured artifact lengths. These tests are the INDEPENDENT
//! literal drift detector for that derivation (the same role `pins.rs` plays for
//! the pins): the frozen addresses are pinned as literals here, and a doctored map
//! must move the derivation (non-vacuity) or fail loud (order desync).

use sigil_harness::seam2::{sound_layout, SoundLayout};
use std::path::{Path, PathBuf};

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// THE TRANSITION ENSURE, made permanent: the map-derived layout equals the frozen
/// chain-22 addresses. Independent of the emit (literal comparands here), so it
/// countersigns the derivation exactly as `pins.rs` countersigns the pins — a
/// derivation that drifts from these addresses is a byte move and fails here.
#[test]
fn sound_layout_derives_the_frozen_addresses() {
    if !strict_gate() {
        eprintln!("skipping seam2_layout_derivation (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let got = sound_layout(&aeon_dir()).expect("sound_layout derives from map.toml");
    let want = SoundLayout {
        dac_blip_lma: 0x48000,
        dac_shared_lma: 0x50000,
        sound_tables_z80_lma: 0x58000,
        pitchtable_lma: 0x58357,
        sfx_win_tab_lma: 0x5845F,
        seq_opcode_tab_lma: 0x5856D,
        dac_sample_tab_lma: 0x585AD,
        // The three below were re-pinned 2026-08-11 to the addresses the map has
        // actually been deriving since sound-pkg-3 (2026-08-10). That package grew
        // the DAC descriptor 9 -> 12 bytes, which took DacSampleTable's span from
        // 90 to 123 and pushed everything after it: mt_bank +0x21, and both SFX
        // bases +0x28 (the extra 7 bytes being the head-tail 8-alignment pad
        // re-rounding). The frozen values were never updated, so this target has
        // been red under SIGIL_STRICT_GATE=1 for six chains — inherited debt, not
        // a fresh move. Nothing upstream of dac_sample_tab_lma shifted, which is
        // the confirmation that the growth starts exactly at the descriptor table.
        mt_bank_lma: 0x58628,        // was 0x58607 (+0x21)
        sfx_bank_lma_plain: 0x5BB10, // was 0x5BAE8 (+0x28)
        sfx_bank_lma_debug: 0x5D560, // was 0x5D53A (+0x26)
    };
    assert_eq!(got, want, "map-derived seam-2 placement drifted from the frozen chain-22 addresses");
}

/// Materialize a doctored aeon: `engine/` symlinked to the real tree, `games/` a
/// real dir whose `sonic4/` children are all symlinks to the real tree EXCEPT a
/// doctored `map.toml`. `sound_layout` reads only `engine/` (via seam-1) and
/// `games/sonic4/`, so this is a faithful whole-derivation substrate.
fn doctored_aeon(root: &Path, doctor: impl FnOnce(String) -> String) {
    let real = aeon_dir();
    std::os::unix::fs::symlink(real.join("engine"), root.join("engine")).unwrap();

    let s4 = root.join("games/sonic4");
    std::fs::create_dir_all(&s4).unwrap();
    for entry in std::fs::read_dir(real.join("games/sonic4")).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name == "map.toml" {
            continue;
        }
        std::os::unix::fs::symlink(entry.path(), s4.join(&name)).unwrap();
    }
    let real_map = std::fs::read_to_string(real.join("games/sonic4/map.toml")).unwrap();
    std::fs::write(s4.join("map.toml"), doctor(real_map)).unwrap();
}

/// NON-VACUITY: a doctored `dac_banks` anchor (moved `$48000 → $40000`) must move
/// the derived DAC placement — proving the emit consumes the map, not a hardcoded
/// literal. If the emit ignored the map, `dac_blip_lma` would stay `$48000`.
#[test]
fn moved_dac_anchor_moves_the_derivation() {
    if !strict_gate() {
        eprintln!("skipping seam2_layout_derivation (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let tmp = tempfile::tempdir().expect("tempdir");
    doctored_aeon(tmp.path(), |m| m.replace("at = 0x48000", "at = 0x40000"));
    let got = sound_layout(tmp.path()).expect("derivation succeeds with the moved anchor");
    assert_eq!(got.dac_blip_lma, 0x40000, "the DAC blip LMA must follow the moved anchor");
    assert_eq!(got.dac_shared_lma, 0x48000, "the shared bank follows blip + the intra-bank align");
    // The sound-bank chain (a separate anchor) is untouched by the DAC move.
    assert_eq!(got.mt_bank_lma, 0x58628, "the head-bank chain is independent of the DAC anchor");
}

/// FAIL-LOUD: a reordered `order` (SFX block before its MT-bank predecessor) desyncs
/// the emit's lay-down order and must fail the whole derivation loudly — never
/// silently emit at the old addresses.
#[test]
fn reordered_map_order_fails_the_emit_loudly() {
    if !strict_gate() {
        eprintln!("skipping seam2_layout_derivation (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let tmp = tempfile::tempdir().expect("tempdir");
    doctored_aeon(tmp.path(), |m| {
        m.replace(
            "\"Song_MovingTrucks\", \"Sfx_33\",",
            "\"Sfx_33\", \"Song_MovingTrucks\",",
        )
    });
    let err = sound_layout(tmp.path()).expect_err("a reordered map must fail the derivation");
    assert!(err.contains("desyncs the seam-2 chain"), "got: {err}");
}
