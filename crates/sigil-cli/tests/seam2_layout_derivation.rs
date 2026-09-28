//! Parcel A1 — the seam-2 placement is a CONSUMER of the map authority.
//!
//! [`sound_layout`] derives every banked LMA from `games/sonic4/map.toml`'s two
//! declared anchors (`dac_banks` `$A8000`, `sound_bank` `$B8000` vma `$8000`) + the
//! emit's own measured artifact lengths. Those two addresses are DERIVED by the BANK
//! PLACEMENT RULE in aeon's map.toml, not chosen, so they move whenever the packed data
//! region grows past the reserve. These tests are the INDEPENDENT
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
    // THE BANK BASES ARE DERIVED, and they move as a BLOCK. aeon's map.toml computes
    // dac_banks = align_up(packed_data_end + RESERVE + 0x8000, 0x8000) and sound_bank one
    // 64 KB bank above it, so growth in the packed data region relocates both. Every
    // in-bank OFFSET below is a property of the sound artifacts and is independent of
    // that: when the banks move, all ten literals move by the same amount and nothing
    // else changes. Two such moves so far, +0x48000 then +0x18000.
    let want = SoundLayout {
        dac_blip_lma: 0xA8000,
        dac_shared_lma: 0xB0000,
        sound_tables_z80_lma: 0xB8000,
        // The five heads lie contiguously from the sound_bank anchor, each at the running
        // sum of the heads before it (sound_tables_z80, pitchtable, SfxBlobWinTab,
        // SeqOpcodeTable, DacSampleTable).
        pitchtable_lma: 0xB83D9,
        sfx_win_tab_lma: 0xB84E1,
        seq_opcode_tab_lma: 0xB85F3,
        dac_sample_tab_lma: 0xB8633,
        // The MT and SFX bases are ALIGNED values, not sums: the packing walk rounds
        // each section's base up to its DECLARED alignment (8 for both, aeon's mod-8
        // fold wall, section_align::DECLARED), and sound_layout predicts that through
        // the walk's own native::packed_chained_base rather than assuming a contiguous
        // pack. The head ends 8-aligned (soundbankhead.emp's tail pad), so mt_bank
        // starts where it ends.
        mt_bank_lma: 0xB86B0,
        sfx_bank_lma_plain: 0xBD568,
        sfx_bank_lma_debug: 0xBEFB0,
    };
    assert_eq!(got, want, "map-derived seam-2 placement drifted from the frozen chain-22 addresses");
}

/// Materialize a doctored aeon: `engine/` COPIED from the real tree, `games/` a real
/// dir whose `sonic4/` children are all symlinks to the real tree EXCEPT a doctored
/// `map.toml` and `data/sound/`, copied. `sound_layout` reads only `engine/` (via seam-1) and `games/sonic4/`,
/// so this is a faithful whole-derivation substrate.
///
/// `engine/` is a real copy, not a symlink: the seam-2 import check resolves a `use`
/// of another module (`dac_sample_tab.emp`'s `use engine.sound_constants.{..}`) by
/// scanning the tree, and the module scan does not follow a symlinked directory, so
/// a symlinked `engine/` holds no module the check can find.
fn doctored_aeon(root: &Path, doctor: impl FnOnce(String) -> String) {
    let real = aeon_dir();
    copy_tree(&real.join("engine"), &root.join("engine"));

    let s4 = root.join("games/sonic4");
    std::fs::create_dir_all(&s4).unwrap();
    for entry in std::fs::read_dir(real.join("games/sonic4")).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name == "map.toml" || name == "data" {
            continue;
        }
        std::os::unix::fs::symlink(entry.path(), s4.join(&name)).unwrap();
    }
    // `data/` is a real directory whose children are symlinks EXCEPT `sound/`, which is
    // copied: mt_bank.emp's embeds resolve against the aeon root and the lowerer
    // refuses a path that leaves that root (`[sandbox.path-escape]`), which a symlink
    // into the real tree does once canonicalized.
    let data = s4.join("data");
    std::fs::create_dir_all(&data).unwrap();
    for entry in std::fs::read_dir(real.join("games/sonic4/data")).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name == "sound" {
            copy_tree(&entry.path(), &data.join(&name));
        } else {
            std::os::unix::fs::symlink(entry.path(), data.join(&name)).unwrap();
        }
    }
    let real_map = std::fs::read_to_string(real.join("games/sonic4/map.toml")).unwrap();
    std::fs::write(s4.join("map.toml"), doctor(real_map)).unwrap();
}

/// Copy the directory tree at `from` to `to`, files and subdirectories, following
/// nothing but real entries.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), &dest).unwrap();
        }
    }
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
            "map.toml no longer spells the dac_banks anchor as `{needle}`, the doctor would be a no-op"
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
