//! seam-1's import check. The resident sound modules are lowered one file at a time
//! with no resolve pass, and `use_import_stubs` reads a `use` list only for the
//! `pub proc` stubs it can derive, so before EMP-UNUSED-IMPORT-UNCHECKED a resident
//! `use` naming nothing that exists lowered clean whenever nothing read it. Each gate
//! feeds the driver from memory ([`with_resident_source_override`]) with one planted
//! `use` line that nothing reads, so the reference tree is never edited.
//!
//! REFERENCE-DEPENDENT like `seam1_link_verdict`: absent a tree every gate skips,
//! unless `SIGIL_STRICT_GATE=1` makes the missing tree a failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-harness --test seam1_import_check
//! ```

use sigil_harness::seam1::{emit_sound_blob, native_blob_checked, with_resident_source_override};
use sigil_harness::test_support::reference_tree;
use std::path::{Path, PathBuf};

const DRIVER: &str = "engine/sound/z80_sound_driver.emp";
const FM: &str = "engine/sound/sound_fm.emp";

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

/// The driver's text with `line` inserted directly after its `module` header, and
/// the planted line's 1-based line number.
fn driver_with_use(aeon: &Path, line: &str) -> (String, usize) {
    let src = std::fs::read_to_string(aeon.join(DRIVER)).expect("read the driver");
    let header = src
        .lines()
        .position(|l| l.starts_with("module "))
        .unwrap_or_else(|| panic!("{DRIVER} has no `module` header line; re-point these gates"));
    let mut lines: Vec<&str> = src.lines().collect();
    lines.insert(header + 1, line);
    (lines.join("\n") + "\n", header + 2)
}

fn blob_with_driver(aeon: &Path, text: &str) -> Result<Vec<u8>, String> {
    with_resident_source_override(DRIVER, text, || native_blob_checked(aeon, false, None))
}

/// The first non-`pub` top-level `const` of `sound_fm.emp` (a column-0 `const NAME`
/// line), derived from the source.
fn a_private_fm_const(aeon: &Path) -> String {
    let src = std::fs::read_to_string(aeon.join(FM)).expect("read sound_fm");
    src.lines()
        .find_map(|l| l.strip_prefix("const ").map(|rest| rest.split([' ', '=']).next().unwrap().to_string()))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| panic!("{FM} declares no non-`pub` top-level const; re-point this gate"))
}

/// THE ROW, resident: an unread import of an item `sound_fm` does not have.
#[test]
fn an_unread_resident_import_of_a_missing_name_is_refused() {
    let Some(aeon) = sound_tree() else { return };
    let (text, line) = driver_with_use(&aeon, "use engine.sound_fm.{NO_SUCH_ITEM_UNUSED_IMPORT}");
    let err = blob_with_driver(&aeon, &text).expect_err("a missing name must not lower");
    let want = format!(
        "{DRIVER}:{line}:1: [Error] module `engine.sound_fm` has no `pub` name `NO_SUCH_ITEM_UNUSED_IMPORT`"
    );
    assert!(err.contains(&want), "want `{want}` in:\n{err}");
    assert_eq!(err.matches("[Error]").count(), 1, "{err}");
}

/// An unread import of a real but private `sound_fm` item is refused too.
#[test]
fn an_unread_resident_import_of_a_private_name_is_refused() {
    let Some(aeon) = sound_tree() else { return };
    let private = a_private_fm_const(&aeon);
    let (text, line) = driver_with_use(&aeon, &format!("use engine.sound_fm.{{{private}}}"));
    let err = blob_with_driver(&aeon, &text).expect_err("a private name must not import");
    let want = format!("{DRIVER}:{line}:1: [Error] module `engine.sound_fm` has no `pub` name `{private}`");
    assert!(err.contains(&want), "want `{want}` in:\n{err}");
}

/// An unread import of a module the tree does not contain.
#[test]
fn an_unread_resident_import_of_a_missing_module_is_refused() {
    let Some(aeon) = sound_tree() else { return };
    let (text, line) = driver_with_use(&aeon, "use engine.no_such_module_unused_import.{X}");
    let err = blob_with_driver(&aeon, &text).expect_err("a missing module must not import");
    let want = format!(
        "{DRIVER}:{line}:1: [Error] no module `engine.no_such_module_unused_import` found under the scan root"
    );
    assert!(err.contains(&want), "want `{want}` in:\n{err}");
}

/// The emitter (what `sigil build` and `emit_sound_blob` run) reports the refusal as
/// an `Err`, the error channel its callers render, never as a panic out of the
/// placement step, and writes nothing.
#[test]
fn an_unread_resident_import_reaches_the_emitter_as_an_error() {
    let Some(aeon) = sound_tree() else { return };
    let (text, line) = driver_with_use(&aeon, "use engine.sound_fm.{NO_SUCH_ITEM_UNUSED_IMPORT}");
    let out = tempfile::tempdir().unwrap();
    let got = std::panic::catch_unwind(|| {
        with_resident_source_override(DRIVER, &text, || emit_sound_blob(&aeon, out.path()))
    });
    let err = match got {
        Ok(Err(e)) => e,
        Ok(Ok(())) => panic!("a missing name must not emit"),
        Err(_) => panic!("the refusal escaped as a panic, not as the emitter's Err"),
    };
    let want = format!(
        "{DRIVER}:{line}:1: [Error] module `engine.sound_fm` has no `pub` name `NO_SUCH_ITEM_UNUSED_IMPORT`"
    );
    assert!(err.contains(&want), "want `{want}` in:\n{err}");
    let written = std::fs::read_dir(out.path()).unwrap().count();
    assert_eq!(written, 0, "a refused emit wrote {written} file(s)");
}

/// The accept arm: an unread import of a real `pub` item of another module in the
/// tree (not a resident one, so the tree scan is what answers) lowers and links.
#[test]
fn an_unread_resident_import_of_a_real_pub_name_is_accepted() {
    let Some(aeon) = sound_tree() else { return };
    let (text, _) = driver_with_use(&aeon, "use engine.sound_constants.{DAC_SAMPLE_COUNT}");
    blob_with_driver(&aeon, &text).unwrap_or_else(|e| panic!("a valid import must lower: {e}"));
}
