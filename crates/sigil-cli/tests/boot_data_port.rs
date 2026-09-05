//! Parcel K2 — the `engine/system/boot_data.emp` port + the $3FE MAP HOLE gate.
//!
//! Builds the whole ROM through the production frozen chainer and asserts the
//! BootData table region is byte-identical to the golden AND that the $3FE hole
//! relationship is REAL: in a no-sound shape the engine.z80_init idle program
//! occupies $3d8..$3fe (the hole) and the post-hole tail resumes at $3fe, purely
//! through contiguous declared-order packing (no absolute `org`). Covers both a
//! sound-ON shape (the resident driver blob rides boot_head) and a sound-OFF
//! shape (the hole is live).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test boot_data_port
//! ```
use sigil_harness::test_support::reference_tree_for_profile;
use sigil_harness::{native, pins};
use std::path::PathBuf;

/// The golden ROM, from sigil's OWN committed `golden/` directory — not the aeon
/// tree. Both gates build their shape from aeon source and compare against this, so
/// the reference tree they need is source-only (see `reference_tree_for_profile`).
fn golden(name: &str) -> Vec<u8> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sigil-harness/golden").join(name);
    std::fs::read(&p).unwrap_or_else(|e| panic!("read golden {}: {e}", p.display()))
}

// The frozen build touches the shared engine/sound/generated dir.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Assert `rom[lo..hi]` equals `golden[lo..hi]` — the BootData region window.
fn assert_window(shape: &str, rom: &[u8], gold: &[u8], lo: usize, hi: usize) {
    let bad: Vec<usize> = (lo..hi).filter(|&i| rom[i] != gold[i]).collect();
    assert!(
        bad.is_empty(),
        "{shape}: BootData region [{lo:#x},{hi:#x}) diverges from golden at {} offset(s); first {:#x} (rom {:#04x} != golden {:#04x})",
        bad.len(),
        bad.first().copied().unwrap_or(0),
        bad.first().map(|&i| rom[i]).unwrap_or(0),
        bad.first().map(|&i| gold[i]).unwrap_or(0),
    );
}

/// SOUND-OFF (config_b, sonic4 game): the hole is LIVE. BootData @ $392, the
/// 54-byte head ends at $3c8, the engine.z80_init idle (40 B, declared WORD so it
/// packs flush) fills $3c8..$3f0, the tail (BootData_PostBlob) resumes at $3f0,
/// BootData_End = $3fe. No pins exist for the off-canonical shape, so these stay
/// literal — the golden window compare above each assert is the loud control. The
/// idle window is non-empty (a real Z80 program), and the tail's first byte is the
/// $9F PSG silence — proving the idle landed IN the hole and the tail resumed AFTER it.
#[test]
fn config_b_boot_data_hole_filled() {
    let profile = native::config_b_profile();
    let Some(aeon) = reference_tree_for_profile(&profile) else {
        return;
    };
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let rom = native::build_rom_chained(&aeon, &profile)
        .unwrap_or_else(|e| panic!("build config_b: {e}"));
    let gold = golden("config_b.bin");
    // BootData ($392) .. BootData_End ($3fe): head + idle-in-hole + tail, byte-exact.
    assert_window("config_b", &rom, &gold, 0x392, 0x3fe);
    // The idle program fills the $3c8..$3f0 hole (not zero-fill — a real 40-byte
    // Z80 program; its first opcode is `xor a` = $AF).
    assert_eq!(rom[0x3c8], 0xAF, "config_b: z80 idle head opcode at the $3c8 hole start");
    let idle_nonzero = (0x3c8..0x3f0).any(|i| rom[i] != 0);
    assert!(idle_nonzero, "config_b: the $3c8..$3f0 hole is empty, z80_init did not fill it");
    // The tail resumes at $3f0 with the first PSG-silence byte ($9F).
    assert_eq!(rom[0x3f0], 0x9F, "config_b: boot_tail did not resume at $3f0 (post-hole PSG byte)");
}

/// SOUND-ON (s4): no hole — the resident driver blob rides boot_head. BootData =
/// pins::BOOT_HEAD.plain_base, the blob begins at BootData+54 (non-zero Z80 code),
/// the tail follows it (pins::BOOT_TAIL). The whole BootData region is
/// byte-identical to the golden. Pin-sourced so boot-size shifts don't rot this.
#[test]
fn s4_boot_data_blob_present() {
    let profile = native::sonic4_profile(false);
    let Some(aeon) = reference_tree_for_profile(&profile) else {
        return;
    };
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let rom = native::build_rom_chained(&aeon, &profile)
        .unwrap_or_else(|e| panic!("build s4: {e}"));
    let gold = golden("s4.bin");
    // BootData .. BootData_End: head + blob + tail, byte-exact (pin-sourced).
    let boot_data = pins::BOOT_HEAD.plain_base as usize;
    let tail = pins::BOOT_TAIL.plain_base as usize;
    let boot_data_end = tail + pins::BOOT_TAIL.plain_len;
    assert_window("s4", &rom, &gold, boot_data, boot_data_end);
    // The resident blob begins at BootData+54 and is non-empty Z80 code.
    let blob_nonzero = (boot_data + 54..tail).any(|i| rom[i] != 0);
    assert!(blob_nonzero, "s4: the resident sound blob region is empty");
    // The post-blob tail resumes at Z80_Sound_End (= BOOT_TAIL base) with the $9F PSG byte.
    assert_eq!(rom[tail], 0x9F, "s4: boot_tail did not resume after the blob");
}
