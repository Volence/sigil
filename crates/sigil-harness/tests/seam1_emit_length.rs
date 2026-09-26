//! seam-1's resident blob LENGTH, split between the two places that care about it.
//!
//! The emitter (`emit_sound_blob`, what aeon's build runs) writes the blob at the
//! length the tree it is handed produces. The exact length of the pinned corpus
//! (`seam1::BLOB_LEN_{PLAIN,DEBUG}`) is a test assertion against the pinned tree.
//!
//! * `emitted_blob_lengths_are_the_pinned_corpus_lengths` reads the two files the
//!   emitter wrote and compares their lengths to the pins, so a change to the pinned
//!   tree's blob length goes red here.
//! * `emit_writes_a_grown_resident_blob` feeds a resident module grown by
//!   [`GROWTH`] `nop`s from memory ([`with_resident_source_override`]), so the tree
//!   is never edited, and asserts the emit succeeds, writes blobs exactly
//!   [`GROWTH`] bytes longer than the unmodified tree's, and that the corpus
//!   verdict refuses those grown blobs.
//!
//! REFERENCE-DEPENDENT: absent a tree every gate skips, unless `SIGIL_STRICT_GATE=1`
//! makes the missing tree a failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-harness --test seam1_emit_length
//! ```

use sigil_harness::seam1::{emit_sound_blob, with_resident_source_override, BLOB_LEN_DEBUG, BLOB_LEN_PLAIN};
use sigil_harness::test_support::reference_tree;
use std::path::{Path, PathBuf};

/// The resident module the growth is planted in. It is the last module in blob
/// order, so no other module re-bases and only the blob's length moves.
const PSG: &str = "engine/sound/sound_psg.emp";

/// Bytes added to the resident blob by the growth control: one `nop` each.
const GROWTH: usize = 17;

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

/// The lengths of the plain and debug blobs the emitter wrote into `out`.
fn emitted_lengths(out: &Path) -> (usize, usize) {
    let len = |name: &str| {
        let p = out.join(name);
        std::fs::metadata(&p).unwrap_or_else(|e| panic!("the emitter did not write {}: {e}", p.display())).len()
            as usize
    };
    (len("z80_sound_blob.bin"), len("z80_sound_blob_debug.bin"))
}

/// Whether an emitted `(plain, debug)` pair has the pinned corpus's lengths.
fn corpus_length_verdict(plain: usize, debug: usize) -> Result<(), String> {
    let mut wrong = Vec::new();
    if plain != BLOB_LEN_PLAIN {
        wrong.push(format!("plain blob is {plain} bytes, BLOB_LEN_PLAIN pins {BLOB_LEN_PLAIN}"));
    }
    if debug != BLOB_LEN_DEBUG {
        wrong.push(format!("debug blob is {debug} bytes, BLOB_LEN_DEBUG pins {BLOB_LEN_DEBUG}"));
    }
    if wrong.is_empty() {
        Ok(())
    } else {
        Err(wrong.join("; "))
    }
}

/// `sound_psg.emp` with [`GROWTH`] `nop`s inserted before its first bare `ret`.
fn psg_grown(aeon: &Path) -> String {
    let src = std::fs::read_to_string(aeon.join(PSG)).expect("read sound_psg");
    let mut lines: Vec<String> = src.lines().map(str::to_string).collect();
    let at = lines
        .iter()
        .position(|l| l.trim() == "ret")
        .unwrap_or_else(|| panic!("{PSG} has no bare `ret` line; re-point this control"));
    let indent: String = lines[at].chars().take_while(|c| c.is_whitespace()).collect();
    for _ in 0..GROWTH {
        lines.insert(at, format!("{indent}nop"));
    }
    lines.join("\n") + "\n"
}

/// THE PIN: the blob the emitter writes from the pinned tree has the corpus lengths.
#[test]
fn emitted_blob_lengths_are_the_pinned_corpus_lengths() {
    let Some(aeon) = sound_tree() else { return };
    let out = tempfile::tempdir().unwrap();
    emit_sound_blob(&aeon, out.path()).unwrap_or_else(|e| panic!("emit failed: {e}"));
    let (plain, debug) = emitted_lengths(out.path());
    corpus_length_verdict(plain, debug).unwrap_or_else(|e| panic!("{e}"));
}

/// THE EMIT PATH: a resident blob of a different length emits, at its own length,
/// and the corpus verdict refuses exactly that output.
#[test]
fn emit_writes_a_grown_resident_blob() {
    let Some(aeon) = sound_tree() else { return };
    let base = tempfile::tempdir().unwrap();
    emit_sound_blob(&aeon, base.path()).unwrap_or_else(|e| panic!("unmodified emit failed: {e}"));
    let (base_plain, base_debug) = emitted_lengths(base.path());

    let text = psg_grown(&aeon);
    let grown = tempfile::tempdir().unwrap();
    with_resident_source_override(PSG, &text, || emit_sound_blob(&aeon, grown.path()))
        .unwrap_or_else(|e| panic!("the emit refused a grown resident blob: {e}"));
    let (plain, debug) = emitted_lengths(grown.path());
    assert_eq!(plain, base_plain + GROWTH, "plain blob grew by {} not {GROWTH}", plain as isize - base_plain as isize);
    assert_eq!(debug, base_debug + GROWTH, "debug blob grew by {} not {GROWTH}", debug as isize - base_debug as isize);

    let err = corpus_length_verdict(plain, debug).expect_err("the corpus pin accepted a grown blob");
    assert!(err.contains("plain blob") && err.contains("debug blob"), "both shapes must be named: {err}");
}
