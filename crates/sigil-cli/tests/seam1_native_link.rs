//! seam-1 — THE RESIDENT-BLOB NATIVE LINK gates. The five sound `.emp` files link
//! as ONE native Z80 module set at VMA `$0000` / LMA `$3DE` (plain) / `$3E2`
//! (debug), with the intra-blob cross-file `extern proc` references resolving
//! INTERNALLY against the sibling sections' `pub proc` exports. The machinery lives
//! in `sigil_harness::seam1` (shared with the `emit_sound_blob` bin); this file is
//! the byte-gate + whole-ROM dual-build proofs.
//!
//! Comparands (the reference-ROM blob slice at LMA): for each shape, the
//! reference ROM window starting at `blob_lma(debug)` and running
//! `BLOB_LEN_{PLAIN,DEBUG}` bytes. Those two lengths and their delta are pinned
//! by `blob_lengths_are_canonical`, which is where to read them — restating a
//! bound here would be executed by nothing, so nothing could go red when it
//! rots. The two shapes do NOT share a base: the debug base sits +4 upstream of
//! BootData, which the design §2.1/OQ-2 comparand originally missed.
//!
//! REFERENCE-DEPENDENT: the five sound `.emp` sources live in the sibling `aeon`
//! tree (`AEON_DIR`, or `EMPYREAN_SUITE_ROOT`). Absent, every
//! test that links the blob SKIPS green — unless `SIGIL_STRICT_GATE=1` makes a
//! missing reference a hard failure, so the pre-merge run cannot skip the byte
//! gate. `blob_lengths_are_canonical` is reference-free and always runs.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test seam1_native_link
//! ```

use sigil_harness::seam1::{
    self, blob_lma, native_blob_doctored, native_sound_blob, resident_module_bases,
    BLOB_LEN_DEBUG, BLOB_LEN_PLAIN,
};
use sigil_harness::test_support::{reference_tree, strict_gate};
use std::path::PathBuf;

/// The reference tree the native link reads: the five resident modules plus the
/// constant authorities their `-D` defines resolve against. `None` skips.
fn sound_tree() -> Option<PathBuf> {
    reference_tree(&[
        "engine/sound/z80_sound_driver.emp",
        "engine/sound/sound_sequencer.emp",
        "engine/sound/sound_sfx.emp",
        "engine/sound/sound_fm.emp",
        "engine/sound/sound_psg.emp",
        "engine/sound/sound_constants.emp",
        "engine/system/constants.emp",
    ])
}

/// The FROZEN golden slice comparand (the asl-witnessed reference), NOT the live
/// tree ROM — post-flip `aeon/s4.bin` is itself sigil-built, so composing `.emp`
/// and comparing to it would be circular; the committed golden is the independent
/// row-91 witness of the resident blob (bar b). Mirrors `native_offcanonical_rom`.
fn read_ref(name: &str) -> Option<Vec<u8>> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../sigil-harness/golden/{name}"));
    match std::fs::read(&path) {
        Ok(b) => Some(b),
        Err(_) => {
            if strict_gate() {
                panic!("SIGIL_STRICT_GATE set but golden missing: {}", path.display());
            }
            eprintln!("skip: golden ROM not at {} (set SIGIL_STRICT_GATE)", path.display());
            None
        }
    }
}

fn assert_blob_matches(blob: &[u8], expected: &[u8], debug: bool, what: &str) {
    assert_eq!(
        blob.len(),
        expected.len(),
        "{what}: length mismatch (blob {} vs reference {})",
        blob.len(),
        expected.len()
    );
    if let Some(i) = (0..blob.len()).find(|&i| blob[i] != expected[i]) {
        let lo = i.saturating_sub(8);
        let hi = (i + 16).min(blob.len());
        panic!(
            "{what}: first diff at blob offset {i:#x} (LMA {:#x})\n  blob[{lo:#x}..{hi:#x}]: {:02x?}\n   ref[{lo:#x}..{hi:#x}]: {:02x?}",
            blob_lma(debug) as usize + i,
            &blob[lo..hi],
            &expected[lo..hi]
        );
    }
}

// ===========================================================================
// §2.1/2.2/2.3 — the whole-blob byte gate (both shapes) + t24 non-vacuity controls.
// ===========================================================================

/// (PLAIN) the natively-linked blob == the `s4.bin` window at `blob_lma(false)`
/// for `BLOB_LEN_PLAIN` bytes.
#[test]
fn native_blob_matches_reference_plain() {
    let Some(aeon) = sound_tree() else { return };
    let Some(refrom) = read_ref("s4.bin") else { return };
    let blob = native_sound_blob(&aeon, false).bytes;
    let base = blob_lma(false) as usize;
    let expected = &refrom[base..base + BLOB_LEN_PLAIN];
    assert_blob_matches(&blob, expected, false, "native blob (plain) vs the s4.bin window at blob_lma(false)");
}

/// (DEBUG) the natively-linked blob == the `s4.debug.bin` window at
/// `blob_lma(true)` for `BLOB_LEN_DEBUG` bytes — the sequencer growth EMITTED
/// (its size is the delta asserted by `blob_lengths_are_canonical`), sfx/fm/psg
/// re-based, the driver's 9 cross-seam operand bytes derived from the real
/// imports per shape.
#[test]
fn native_blob_matches_reference_debug() {
    let Some(aeon) = sound_tree() else { return };
    let Some(refrom) = read_ref("s4.debug.bin") else { return };
    let blob = native_sound_blob(&aeon, true).bytes;
    let base = blob_lma(true) as usize;
    let expected = &refrom[base..base + BLOB_LEN_DEBUG];
    assert_blob_matches(&blob, expected, true, "native blob (debug) vs the s4.debug.bin window at blob_lma(true)");
}

/// The debug blob is longer than plain by the delta this test asserts below —
/// read it there rather than from prose. These constants are a size
/// TRIPWIRE, not a placement input — the module bases are derived (see
/// `module_bases_are_a_gapless_cursor`).
///
/// NOTE these are the BLOB lengths, which since aeon `5526113` are no longer
/// necessarily equal to `Z80_SOUND_SIZE`: aeon aligns the blob to an even length
/// inside the `Z80_Sound_Start`/`_End` brackets, so `Z80_SOUND_SIZE` is the blob
/// length rounded UP to even (it is `boot_port.rs` that pins that value). The pad
/// is load-bearing — boot walks `a5` through the blob into word-wide VDP reads in
/// `boot_tail`, so an odd blob address-errors the 68k before the first frame.
/// Both current lengths are odd, hence a 1-byte pad in each shape.
#[test]
fn blob_lengths_are_canonical() {
    assert_eq!(BLOB_LEN_DEBUG - BLOB_LEN_PLAIN, 0x82, "debug grows +$82 over plain (pkg 4 D7 added a 4 B debug-only operand-0 trap)");
    assert_eq!(BLOB_LEN_PLAIN, 0x1813, "plain resident blob length = 6163 B; the debug shape is this + $82");
}

/// The PLACEMENT contract, stated structurally instead of by re-pinned addresses:
/// the five resident modules' bases are a strictly-increasing, GAPLESS cursor that
/// starts at `$0000` and whose final module ends exactly at the blob length. This
/// is what makes a module's size change safe — its successors re-base with it —
/// and it fails loudly if a base is ever computed from anything but the running
/// cursor. (The old hand-pinned VMA table could under-state a base, whereupon the
/// gap was silently dropped from the concatenated blob and every cross-module
/// `call` missed.)
#[test]
fn module_bases_are_a_gapless_cursor() {
    let Some(aeon) = sound_tree() else { return };
    let Some(_) = read_ref("s4.bin") else { return };
    for (debug, len) in [(false, BLOB_LEN_PLAIN), (true, BLOB_LEN_DEBUG)] {
        let bases = resident_module_bases(&aeon, debug);
        assert_eq!(bases.len(), 5, "five resident modules");
        assert_eq!(bases[0].1, 0, "the driver anchors the blob at VMA $0000 (debug={debug})");
        for w in bases.windows(2) {
            assert!(
                w[0].1 < w[1].1,
                "module bases must strictly increase: {} at {:#06x} then {} at {:#06x}",
                w[0].0,
                w[0].1,
                w[1].0,
                w[1].1
            );
        }
        // The blob is a CONCATENATION, so the last base + its span == the total.
        let blob = native_sound_blob(&aeon, debug).bytes;
        assert_eq!(blob.len(), len, "blob length tripwire (debug={debug})");
        assert!(
            (bases[4].1 as usize) < len,
            "the last module must start inside the blob (debug={debug})"
        );
    }
}

/// t24 positive control (BANKED-CARRIER axis): a doctored `SeqOpcodeTable` must make
/// the blob DIVERGE from the reference — the byte gate is not vacuous.
#[test]
fn blob_diverges_when_banked_carrier_doctored() {
    let Some(aeon) = sound_tree() else { return };
    let Some(refrom) = read_ref("s4.bin") else { return };
    let base = blob_lma(false) as usize;
    let expected = &refrom[base..base + BLOB_LEN_PLAIN];
    let doctored = native_blob_doctored(&aeon, false, Some(("SeqOpcodeTable", 0x8ABC)));
    assert_ne!(doctored, expected, "the byte gate is vacuous if a doctored carrier still matches");
}

/// t24 positive control (CONST `-D` axis): a doctored request-slot const must make
/// the blob's abs-mem operands DIVERGE from the reference.
///
/// RE-AIMED at `SND_REQ_MUSIC` (sound-pkg1, 2026-08-09): the original carrier
/// `SND_STAT_TICK` is now the anchor the package's game-feel mirrors derive
/// from, and the module's own drift-lock `ensure` catches a doctored value at
/// LOWER time — strictly earlier than this test's blob compare, so the axis it
/// proved is now enforced upstream. The probe moves to a mailbox slot that is
/// abs-mem-consumed but not mirror-anchored.
#[test]
fn blob_diverges_when_const_doctored() {
    let Some(aeon) = sound_tree() else { return };
    let Some(refrom) = read_ref("s4.bin") else { return };
    let base = blob_lma(false) as usize;
    let expected = &refrom[base..base + BLOB_LEN_PLAIN];
    let doctored = native_blob_doctored(&aeon, false, Some(("SND_REQ_MUSIC", 0x1DED)));
    assert_ne!(doctored, expected, "a moved SND_REQ_MUSIC must change the blob's abs-mem operands");
}

/// The exported-symbol contract is present: `sound_sequencer.emp` exports all 26
/// handler labels the banked `seq_opcode_tab.asm` references (a missing one panics
/// in `native_sound_blob`, so reaching here means the contract is complete), and
/// each lies in the sequencer window. The window is DERIVED from the module bases
/// (`[sequencer, sfx)`), not re-pinned — a legitimate size change re-bases it
/// instead of false-failing here.
#[test]
fn handler_symbol_contract_complete() {
    let Some(aeon) = sound_tree() else { return };
    let Some(_) = read_ref("s4.bin") else { return };
    let bases = resident_module_bases(&aeon, false);
    let lo = bases.iter().find(|(n, _)| n == "sound_sequencer").expect("sequencer base").1;
    let hi = bases.iter().find(|(n, _)| n == "sound_sfx").expect("sfx base").1;
    let syms = native_sound_blob(&aeon, false).symbols;
    assert_eq!(
        syms.len(),
        seam1::HANDLER_SYMBOLS.len(),
        "every seq_opcode_tab handler must be exported by the blob"
    );
    for (name, addr) in &syms {
        assert!(
            (lo..hi).contains(addr),
            "handler {name} at {addr:#x} must lie in the sequencer window {lo:#06x}..{hi:#06x}"
        );
    }
}
