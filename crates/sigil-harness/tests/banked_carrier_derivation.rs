//! seam-1's banked `$8000`-window carriers are DERIVED from the aeon tree, all
//! eleven of them, and every one reaches the resident driver's bytes.
//!
//! The carriers are equ symbols injected into the seam-1 link that produces the
//! SHIPPED resident Z80 blob, so their values end up in the driver's operand bytes.
//! `seam2::banked_carrier_vmas` derives them: the three head-bank members
//! (`SndDefaultPitchTable`, `SfxBlobWinTab`, `SeqOpcodeTable`) from the map-derived
//! head layout, and the eight labels inside the `sound_tables_z80` head from the
//! lowered `sound_tables_z80.emp`'s own labels. A table or head change in aeon moves
//! them with no edit in sigil, so there is no hand copy left to drift; the gates
//! here hold the derivation to its shape and prove each value is consumed.
//!
//! How each derived value is proven to be the right one, rather than merely present:
//! at the pinned reference tree the resident-blob byte gate (`seam1_native_link`)
//! compares the blob linked with these values against the frozen reference, and the
//! seam-2 unit test `banked_table_carriers_follow_the_table_labels` moves the table's
//! labels in memory and requires each carrier to move by exactly what precedes it.
use std::path::PathBuf;

use sigil_harness::seam2::{
    banked_carrier_vmas, emit_sound_tables_z80, BANKED_HEAD_MEMBERS, SOUND_TABLES_BANKED_LABELS,
    SOUND_TABLES_Z80_REL,
};

fn sound_tree() -> Option<PathBuf> {
    sigil_harness::test_support::reference_tree(&["engine/sound/z80_sound_driver.emp", SOUND_TABLES_Z80_REL])
}

/// The derivation produces all eleven carriers, head members first in head order
/// and then the table labels, every one inside the `$8000` window. The table labels
/// sit inside the table, which the head bank opens with: every table label lies in
/// the `table_len` bytes that end at `SndDefaultPitchTable`, the first head member.
/// That ties the head layout, the table's labels and its emitted length to one
/// another, each derived by a different path.
#[test]
fn every_banked_carrier_is_derived() {
    let Some(aeon) = sound_tree() else { return };
    let vmas = banked_carrier_vmas(&aeon).expect("the derivation must succeed");
    let names: Vec<&str> = vmas.iter().map(|(n, _)| *n).collect();
    let want: Vec<&str> = BANKED_HEAD_MEMBERS.into_iter().chain(SOUND_TABLES_BANKED_LABELS).collect();
    assert_eq!(names, want, "the carrier list must be the three head members then the eight table labels");

    let by = |name: &str| vmas.iter().find(|(n, _)| *n == name).map(|(_, v)| *v).unwrap();
    let table_len = emit_sound_tables_z80(&aeon).expect("sound_tables_z80 lowers").len() as u32;
    let table_end = by("SndDefaultPitchTable");
    let window = table_end.checked_sub(table_len).expect("the first head member must follow the table");

    let mut prev = 0x8000u32;
    for name in BANKED_HEAD_MEMBERS {
        let vma = by(name);
        assert!((0x8000..0x10000).contains(&vma), "{name} at {vma:#06x} is outside the $8000 window");
        assert!(vma > prev, "{name} at {vma:#06x} must follow the previous head member ({prev:#06x})");
        prev = vma;
    }
    for name in SOUND_TABLES_BANKED_LABELS {
        let vma = by(name);
        assert!(
            (window..window + table_len).contains(&vma),
            "{name} at {vma:#06x} must lie inside the table [{window:#06x}, {:#06x})",
            window + table_len
        );
    }
}

/// Each of the eleven derived carriers reaches the resident driver's bytes: moving
/// any one of them by `$10` changes the linked blob in at least one shape. A carrier
/// the driver never read would make its derivation unverifiable by the byte gate,
/// and that is reported by name.
#[test]
fn every_banked_carrier_reaches_the_resident_blob() {
    let Some(aeon) = sound_tree() else { return };
    let vmas = banked_carrier_vmas(&aeon).expect("the derivation must succeed");
    let base: Vec<Vec<u8>> =
        [false, true].iter().map(|&d| sigil_harness::seam1::native_blob_doctored(&aeon, d, None)).collect();
    let mut unread = Vec::new();
    for (name, vma) in &vmas {
        let moved = [false, true].iter().zip(&base).any(|(&debug, b)| {
            sigil_harness::seam1::native_blob_doctored(&aeon, debug, Some((name, i64::from(*vma) + 0x10))) != *b
        });
        if !moved {
            unread.push(*name);
        }
    }
    assert!(unread.is_empty(), "banked carriers the resident blob never reads: {unread:?}");
}
