//! Parcel SECTION-ROW — the `"section:<name>"` `order` row, proven on the live corpus.
//!
//! `games/sonic4/map.toml` declares `ojz_effects_editor_act1` by the CONTENT-DERIVED head
//! label `tools/effects_gen.py` mints (`EditorSceneBinding_OJZ_Act1_Sec0` today; it moves
//! whenever the generated block's first emitted symbol moves). The durable spelling is
//! the section row. These gates build sonic4 from a MIRROR of the aeon tree whose map is
//! doctored — the aeon tree itself is never edited; aeon lands the real row — and prove:
//!
//!   * INTENDED USE — the section row in the literal's position yields the provenance-tip
//!     ROM byte-for-byte, both canonical shapes (the section lands where it did).
//!   * NON-VACUITY — the same substrate with a misspelled section row, or with both rows,
//!     fails the build on the named diagnostic (so the identity above is not a doctoring
//!     that never reached the build).
//!
//! When aeon migrates map.toml:124 to the section row, the literal disappears and
//! `doctored_map` panics by design: retire the doctoring arm then (the live CRC gates
//! carry the identity from that day on).

use sigil_harness::native;
use sigil_harness::test_support::{aeon_dir, strict_gate};
use std::path::{Path, PathBuf};

const LITERAL_ROW: &str = "\"EditorSceneBinding_OJZ_Act1_Sec0\"";
const SECTION_ROW: &str = "\"section:ojz_effects_editor_act1\"";

/// Mirror `real` into `root` as a real COPY (directories and files). Not symlinks: the
/// `.emp` manifest scan does not follow symlinked directories, and `embed` paths
/// canonicalize, so a symlinked file escapes the source sandbox
/// (`[sandbox.path-escape]`). `.git` / nested checkouts / build targets are not mirrored.
fn mirror(real: &Path, root: &Path) {
    std::fs::create_dir_all(root).unwrap();
    for entry in std::fs::read_dir(real).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name == ".git" || name == ".worktrees" || name == "target" {
            continue;
        }
        let src = entry.path();
        let dst = root.join(&name);
        if entry.file_type().unwrap().is_dir() {
            mirror(&src, &dst);
        } else {
            std::fs::copy(&src, &dst).unwrap();
        }
    }
}

/// The mirrored aeon with `games/sonic4/map.toml` replaced by `doctor(live map)`.
fn doctored_aeon(root: &Path, doctor: impl FnOnce(String) -> String) {
    let real = aeon_dir();
    mirror(&real, root);
    let map = root.join("games/sonic4/map.toml");
    let live = std::fs::read_to_string(&map).unwrap();
    std::fs::write(&map, doctor(live)).unwrap();
}

/// The live map with the literal row rewritten — LOUD if the literal is not exactly once
/// in the map (the corpus moved; see the module doc).
fn with_literal_replaced(live: String, replacement: &str) -> String {
    let n = live.matches(LITERAL_ROW).count();
    assert_eq!(
        n, 1,
        "expected exactly one {LITERAL_ROW} row in the live games/sonic4/map.toml, found {n} — \
         if aeon migrated the row to {SECTION_ROW}, retire this fixture's doctoring arm"
    );
    live.replace(LITERAL_ROW, replacement)
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sigil-harness/golden")
}

fn expected_full(key: &str) -> (u32, usize) {
    let t = sigil_harness::provenance::tip_target(&golden_dir(), key)
        .unwrap_or_else(|e| panic!("provenance tip: {e}"));
    let crc = sigil_harness::provenance::hex_u32(&t.full_crc).unwrap_or_else(|e| panic!("{e}"));
    (crc, t.full_size)
}

fn gate_on() -> bool {
    if !strict_gate() {
        eprintln!("skipping section_row_fixture (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return false;
    }
    true
}

/// INTENDED USE: `"section:ojz_effects_editor_act1"` in the literal's position builds the
/// provenance-tip ROM byte-for-byte in both canonical shapes.
#[test]
fn section_row_in_the_literal_position_is_byte_identical() {
    if !gate_on() {
        return;
    }
    let tmp = tempfile::tempdir().expect("tempdir");
    doctored_aeon(tmp.path(), |m| with_literal_replaced(m, SECTION_ROW));
    for (debug, key) in [(false, "s4"), (true, "s4_debug")] {
        let (want_crc, want_len) = expected_full(key);
        let full = native::build_native_full_file(tmp.path(), debug).unwrap_or_else(|e| panic!("{key}: {e}"));
        let got_crc = native::crc32(&full);
        assert_eq!(
            (got_crc, full.len()),
            (want_crc, want_len),
            "{key}: the section-row map must reproduce the provenance tip ({want_crc:08x}/{want_len}); got {got_crc:08x}/{}",
            full.len()
        );
    }
}

/// NON-VACUITY (a): a section row naming a section the build does not have stops the
/// build on `[map.order-unknown-section]`, carrying the row as written.
#[test]
fn misspelled_section_row_fails_the_build_loudly() {
    if !gate_on() {
        return;
    }
    let tmp = tempfile::tempdir().expect("tempdir");
    doctored_aeon(tmp.path(), |m| with_literal_replaced(m, "\"section:ojz_effects_editor_act1_nope\""));
    let e = native::build_native_full_file(tmp.path(), false).expect_err("an unknown section row must stop the build");
    assert!(
        e.contains("[map.order-unknown-section]") && e.contains("`section:ojz_effects_editor_act1_nope`"),
        "got: {e}"
    );
}

/// NON-VACUITY (b): the literal row AND the section row for the same section is two rows
/// for one section — `[map.order-double-declared]`, naming both spellings.
#[test]
fn section_row_and_label_for_one_section_fail_the_build_loudly() {
    if !gate_on() {
        return;
    }
    let tmp = tempfile::tempdir().expect("tempdir");
    doctored_aeon(tmp.path(), |m| with_literal_replaced(m, &format!("{LITERAL_ROW}, {SECTION_ROW}")));
    let e = native::build_native_full_file(tmp.path(), false).expect_err("a double-declared section must stop the build");
    assert!(
        e.contains("[map.order-double-declared]")
            && e.contains("`EditorSceneBinding_OJZ_Act1_Sec0`")
            && e.contains("`section:ojz_effects_editor_act1`"),
        "got: {e}"
    );
}
