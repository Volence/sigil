//! Parcel A1 — the seam-2 placement is a CONSUMER of the map authority.
//!
//! [`sound_layout`] derives every banked LMA from `games/sonic4/map.toml`'s two
//! declared anchors (`dac_banks`, `sound_bank` vma `$8000` — `$90000` / `$A0000` since
//! aeon's ROM re-layout of 2026-08-26, `$48000` / `$58000` before it) + the emit's own
//! measured artifact lengths. These tests are the INDEPENDENT
//! literal drift detector for that derivation (the same role `pins.rs` plays for
//! the pins): the frozen addresses are pinned as literals here, and a doctored map
//! must move the derivation (non-vacuity) or fail loud (order desync).

use sigil_harness::seam2::{sound_layout, SoundLayout};
use std::path::{Path, PathBuf};

fn aeon_dir() -> PathBuf {
    sigil_harness::test_support::aeon_dir()
}
#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}

/// THE TRANSITION ENSURE, made permanent: the map-derived layout equals the frozen
/// chain-22 addresses. Independent of the emit (literal comparands here), so it
/// countersigns the derivation exactly as `pins.rs` countersigns the pins — a
/// derivation that drifts from these addresses is a byte move and fails here.
#[test]
fn sound_layout_derives_the_frozen_addresses() {
    if !strict_gate() {
        eprintln!("skip: seam2_layout_derivation not measured (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let got = sound_layout(&aeon_dir()).expect("sound_layout derives from map.toml");
    // ROM re-layout (aeon parcel/rom-relayout, 2026-08-26): the banks moved +0x48000
    // as a block — dac_banks 0x48000 -> 0x90000, sound_bank 0x58000 -> 0xA0000 — by the
    // BANK PLACEMENT RULE in aeon's map.toml (dac_banks = align_up(packed_data_end +
    // 0x4000, 0x8000)). Every in-bank offset below is unchanged; only the bank moved.
    let want = SoundLayout {
        dac_blip_lma: 0x90000,
        dac_shared_lma: 0x98000,
        sound_tables_z80_lma: 0xA0000,
        pitchtable_lma: 0xA0357,
        sfx_win_tab_lma: 0xA045F,
        seq_opcode_tab_lma: 0xA0571,
        dac_sample_tab_lma: 0xA05B1,
        // The three below moved TWICE on 2026-08-11, and the split matters:
        //   * sound-pkg-3 (2026-08-10) grew the DAC descriptor 9 -> 12 bytes, taking
        //     DacSampleTable 90 -> 123 and pushing mt_bank +0x21 / both SFX bases
        //     +0x28. Those frozen values were never updated, which kept this target
        //     red under SIGIL_STRICT_GATE=1 for six chains (fixed in 2c49f538).
        //   * sfx-flight added the $BA/$BB SFX, widening SfxBlobWinTab by 4 bytes and
        //     re-rounding DacHeadPad 3 -> 7, so the head grew 8 and everything after
        //     it followed. seq_opcode_tab and dac_sample_tab are +4 from that alone.
        // The SFX bases are ALIGNED values, not sums: the packing walk rounds this
        // section's base up to 16 (see native::packed_align_of), and since 113f3006
        // sound_layout predicts that rather than assuming a contiguous pack. Nothing
        // upstream of seq_opcode_tab_lma moved in either step.
        mt_bank_lma: 0xA0630,
        sfx_bank_lma_plain: 0xA3B20,
        sfx_bank_lma_debug: 0xA5570,
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

/// NON-VACUITY: a doctored `dac_banks` anchor (moved one bank DOWN from wherever the
/// real map puts it) must move the derived DAC placement — proving the emit consumes
/// the map, not a hardcoded literal. The real anchor is READ off the real derivation
/// (never retyped here), so this probe stays valid across re-layouts: if the emit
/// ignored the map, `dac_blip_lma` would stay at the real value.
#[test]
fn moved_dac_anchor_moves_the_derivation() {
    if !strict_gate() {
        eprintln!("skip: seam2_layout_derivation not measured (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let real = sound_layout(&aeon_dir()).expect("the real derivation");
    let (from, to) = (real.dac_blip_lma, real.dac_blip_lma - 0x8000);
    let tmp = tempfile::tempdir().expect("tempdir");
    doctored_aeon(tmp.path(), |m| {
        let needle = format!("at = 0x{from:X}");
        assert!(
            m.contains(&needle),
            "map.toml no longer spells the dac_banks anchor as `{needle}` — the doctor would be a no-op"
        );
        m.replace(&needle, &format!("at = 0x{to:X}"))
    });
    let got = sound_layout(tmp.path()).expect("derivation succeeds with the moved anchor");
    assert_ne!(got, real, "the doctored map must move the derivation (non-vacuity)");
    assert_eq!(got.dac_blip_lma, to, "the DAC blip LMA must follow the moved anchor");
    assert_eq!(got.dac_shared_lma, from, "the shared bank follows blip + the intra-bank align");
    // The sound-bank chain (a separate anchor) is untouched by the DAC move.
    assert_eq!(got.mt_bank_lma, real.mt_bank_lma, "the head-bank chain is independent of the DAC anchor");
}

/// FAIL-LOUD: a reordered `order` (SFX block before its MT-bank predecessor) desyncs
/// the emit's lay-down order and must fail the whole derivation loudly — never
/// silently emit at the old addresses.
#[test]
fn reordered_map_order_fails_the_emit_loudly() {
    if !strict_gate() {
        eprintln!("skip: seam2_layout_derivation not measured (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
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
