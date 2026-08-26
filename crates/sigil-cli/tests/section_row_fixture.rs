//! Parcel SECTION-ROW — the `"section:<name>"` `order` row, proven on the live corpus.
//!
//! `games/sonic4/map.toml` declares `ojz_effects_editor_act1` either by the CONTENT-
//! DERIVED head label `tools/effects_gen.py` mints (it moves whenever the generated
//! block's first emitted symbol moves) or by the durable section row. These gates are
//! DIRECTION-AGNOSTIC and SELF-DERIVING: the head label comes from the build's own
//! section table, the current spelling from the live map, and the doctored copy of
//! the aeon tree carries the OTHER spelling. The invariant — the two spellings are the
//! same ROM — holds forever, so aeon's migration of the row changes nothing here.
//! The aeon tree itself is never edited; aeon lands the real row.
//!
//!   * INTENDED USE — the other spelling yields the same ROM as the live spelling AND
//!     the provenance tip, both canonical shapes.
//!   * NON-VACUITY — the same substrate with a misspelled section row, or with both
//!     spellings, fails the build on the named diagnostic (so the identity above is
//!     not a doctoring that never reached the build).
//!
//! The only real unmeasurable — NEITHER spelling in the live map — fails loud naming
//! the section.

use sigil_harness::native;
use sigil_harness::test_support::{aeon_dir, strict_gate};
use std::path::{Path, PathBuf};

/// The section under test: the `module … in <name>` target of the generated block.
const SECTION: &str = "ojz_effects_editor_act1";

/// The two spellings of one section's `order` row, both derived: the head label from
/// the resolved section table of the live build, the section row from the name.
struct Spellings {
    /// `"<head-label>"` — quoted, as it appears in the map.
    label: String,
    /// `"section:<name>"` — quoted.
    section: String,
}

impl Spellings {
    fn derive(aeon: &Path) -> Spellings {
        native::ensure_generated(aeon);
        let prog = native::build_emp(aeon, &native::sonic4_profile(false)).unwrap_or_else(|e| panic!("build_emp: {e}"));
        let sec = prog
            .sections
            .iter()
            .find(|s| s.name == SECTION)
            .unwrap_or_else(|| panic!("the live build has no section named `{SECTION}`"));
        let head = sec
            .labels
            .iter()
            .min_by_key(|l| l.offset)
            .unwrap_or_else(|| panic!("section `{SECTION}` has no labels — its head label is unmeasurable"));
        assert!(
            !sec.image_bytes().is_empty(),
            "section `{SECTION}` emits zero bytes in the live build — the identity below would be vacuous"
        );
        Spellings { label: format!("\"{}\"", head.name), section: format!("\"section:{SECTION}\"") }
    }

    /// (current spelling in the live map, the other one). LOUD when neither is present,
    /// or when both are (the live map would itself be double-declared).
    fn current_and_other(&self, live_map: &str) -> (&str, &str) {
        let (nl, ns) = (live_map.matches(self.label.as_str()).count(), live_map.matches(self.section.as_str()).count());
        match (nl, ns) {
            (1, 0) => (&self.label, &self.section),
            (0, 1) => (&self.section, &self.label),
            _ => panic!(
                "games/sonic4/map.toml must declare `{SECTION}` by exactly one spelling — found {nl} × {} and {ns} × {}",
                self.label, self.section
            ),
        }
    }
}

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

/// The mirrored aeon whose `games/sonic4/map.toml` has the section's CURRENT row
/// replaced by `replacement(current, other)`. Returns the derived pair.
fn doctored_aeon(root: &Path, replacement: impl FnOnce(&str, &str) -> String) -> Spellings {
    let real = aeon_dir();
    let spellings = Spellings::derive(&real);
    mirror(&real, root);
    let map = root.join("games/sonic4/map.toml");
    let live = std::fs::read_to_string(&map).unwrap();
    let (current, other) = spellings.current_and_other(&live);
    std::fs::write(&map, live.replace(current, &replacement(current, other))).unwrap();
    spellings
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

/// INTENDED USE: the OTHER spelling of the section's row builds the same ROM as the live
/// map's spelling, and both equal the provenance tip, in both canonical shapes.
#[test]
fn both_spellings_of_the_section_row_build_the_same_rom() {
    if !gate_on() {
        return;
    }
    let tmp = tempfile::tempdir().expect("tempdir");
    doctored_aeon(tmp.path(), |_current, other| other.to_string());
    for (debug, key) in [(false, "s4"), (true, "s4_debug")] {
        let (want_crc, want_len) = expected_full(key);
        let live = native::build_native_full_file(&aeon_dir(), debug).unwrap_or_else(|e| panic!("{key} live: {e}"));
        let other = native::build_native_full_file(tmp.path(), debug).unwrap_or_else(|e| panic!("{key} other spelling: {e}"));
        assert!(live == other, "{key}: the two spellings of the `{SECTION}` row must build the same ROM");
        let got_crc = native::crc32(&other);
        assert_eq!(
            (got_crc, other.len()),
            (want_crc, want_len),
            "{key}: must reproduce the provenance tip ({want_crc:08x}/{want_len}); got {got_crc:08x}/{}",
            other.len()
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
    let bad = format!("\"section:{SECTION}_nope\"");
    doctored_aeon(tmp.path(), |_, _| bad.clone());
    let e = native::build_native_full_file(tmp.path(), false).expect_err("an unknown section row must stop the build");
    let want = format!("`section:{SECTION}_nope`");
    assert!(e.contains("[map.order-unknown-section]") && e.contains(&want), "got: {e}");
}

/// NON-VACUITY (b): the label row AND the section row for the same section is two rows
/// for one section — `[map.order-double-declared]`, naming both spellings.
#[test]
fn section_row_and_label_for_one_section_fail_the_build_loudly() {
    if !gate_on() {
        return;
    }
    let tmp = tempfile::tempdir().expect("tempdir");
    let sp = doctored_aeon(tmp.path(), |current, other| format!("{current}, {other}"));
    let e = native::build_native_full_file(tmp.path(), false).expect_err("a double-declared section must stop the build");
    let (label, section) = (sp.label.replace('"', "`"), sp.section.replace('"', "`"));
    assert!(
        e.contains("[map.order-double-declared]") && e.contains(&label) && e.contains(&section),
        "got: {e}"
    );
}
