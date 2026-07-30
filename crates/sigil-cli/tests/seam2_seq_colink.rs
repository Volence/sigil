//! seam-2 stage-3 — the SeqOpcodeTable head, co-linked and proven against the
//! reference ROM (the row-56 identity bar, both shapes). Replaces the retired
//! `seq_opcode_tab_port.rs` windowed oracle (its `.asm` twin is deleted).
//!
//! `sigil_harness::seam2::emit_seq_opcode_tab` lowers the REAL
//! `engine/sound/seq_opcode_tab.emp` and resolves its 32 `dc.w Seq_Op_*` cells
//! against the REAL resident handler VMAs — read off the SAME seam-1 blob link
//! (`native_sound_blob`) the resident driver ships from, so the Seq_Op_* imports
//! resolve to the exact VMAs the `z80_sound_syms.asm` contract exported (design
//! §2c). This gate proves the emitted head is BYTE-IDENTICAL to the
//! `SeqOpcodeTable` slice of the assembled reference ROM (`$5856D`, 64 bytes).
//!
//! SHAPE-DEPENDENT: the handlers re-base after `sound_sequencer.emp`'s
//! `if DEBUG==1` growth, so the table differs per shape — each is gated against
//! ITS OWN reference ROM, and `plain != debug` proves the emit tracks the shape.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test seam2_seq_colink
//! ```

use sigil_harness::seam2::{
    emit_seq_opcode_artifacts, emit_seq_opcode_tab, emit_seq_opcode_tab_doctored,
    SEQ_OPCODE_TAB_LEN, SEQ_OPCODE_TAB_LMA,
};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}
/// The FROZEN golden slice comparand (the asl-witnessed reference), NOT the live
/// tree ROM — post-flip `aeon/s4.bin` is itself sigil-built (row-91 bar b).
fn golden(name: &str) -> Vec<u8> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../sigil-harness/golden/{name}"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("read golden {}: {e}", path.display()))
}

/// THE HEAD BYTE GATE: the co-linked `SeqOpcodeTable` == the reference ROM slice
/// at `$5856D`, in BOTH shapes (each vs its own ROM — the head is shape-dependent).
#[test]
fn colinked_seq_opcode_tab_matches_the_reference_rom_slice_both_shapes() {
    if !strict_gate() {
        eprintln!("skipping seam2_seq_colink (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    for (debug, shape) in [(false, "plain"), (true, "debug")] {
        let head = emit_seq_opcode_tab(&aeon, debug).expect("emit_seq_opcode_tab co-links");
        assert_eq!(head.len(), SEQ_OPCODE_TAB_LEN, "SeqOpcodeTable is 32 × 2 = 64 bytes");
        let rom = golden(if debug { "s4.debug.bin" } else { "s4.bin" });
        let lo = SEQ_OPCODE_TAB_LMA as usize;
        let head_ref = &rom[lo..lo + SEQ_OPCODE_TAB_LEN];
        if let Some(i) = (0..head.len()).find(|&i| head[i] != head_ref[i]) {
            let slot = i / 2;
            panic!(
                "co-linked SeqOpcodeTable differs from {shape} reference @ slot {slot} byte {}: \
                 emp {:#04x} vs rom {:#04x}",
                i % 2, head[i], head_ref[i]
            );
        }
        assert_eq!(head, head_ref, "co-linked SeqOpcodeTable must equal the {shape} reference @ $5856D");
    }
}

/// Shape-variance: the plain and debug tables DIFFER (the handlers re-base), so
/// the emit genuinely tracks the shape (not a frozen literal both times).
#[test]
fn plain_and_debug_tables_differ() {
    if !strict_gate() {
        eprintln!("skipping seam2_seq_colink (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let p = emit_seq_opcode_tab(&aeon, false).expect("plain");
    let d = emit_seq_opcode_tab(&aeon, true).expect("debug");
    assert_ne!(p, d, "the shape-dependent handler re-base must move the table bytes");
}

/// t24 NON-VACUITY control (row-91 bar c): a doctored composition — the resident
/// `Seq_Op_Dac` handler VMA overridden to `$0ABC` — must make the table DIVERGE
/// from the golden slice, because that opcode's `dc.w` cell re-folds to the moved
/// handler. The seq table gate is vacuous if a moved handler still matches.
#[test]
fn seq_tab_diverges_when_handler_moved() {
    if !strict_gate() {
        eprintln!("skipping seam2_seq_colink (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let doctored = emit_seq_opcode_tab_doctored(&aeon, false, Some(("Seq_Op_Dac", 0x0ABC)))
        .expect("doctored co-link");
    let rom = golden("s4.bin");
    let lo = SEQ_OPCODE_TAB_LMA as usize;
    let head_ref = &rom[lo..lo + SEQ_OPCODE_TAB_LEN];
    assert_ne!(
        doctored, head_ref,
        "the seq table gate is vacuous if a moved Seq_Op_Dac handler still matches the golden slice"
    );
}

/// Determinism + the emitted `.bin`s match the in-memory co-link.
#[test]
fn emit_seq_artifacts_writes_reference_bins() {
    if !strict_gate() {
        eprintln!("skipping seam2_seq_colink (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let aeon = aeon_dir();
    let dir = tempfile::tempdir().expect("tempdir");
    emit_seq_opcode_artifacts(&aeon, dir.path()).expect("emit_seq_opcode_artifacts writes the 2 .bins");
    for (debug, name) in [(false, "seq_opcode_tab.bin"), (true, "seq_opcode_tab_debug.bin")] {
        let mem = emit_seq_opcode_tab(&aeon, debug).expect("co-link");
        let got = std::fs::read(dir.path().join(name)).unwrap_or_else(|e| panic!("read {name}: {e}"));
        assert_eq!(got, mem, "emitted {name} must equal the in-memory co-link (== reference)");
    }
}
