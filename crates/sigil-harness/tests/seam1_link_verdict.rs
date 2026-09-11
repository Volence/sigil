//! seam-1's link verdict. The resident sound blob is linked by the harness, not by a
//! map build, so seam-1 is the only stage that can decide the resident modules'
//! link asserts: their deferred guards and the name check every `extern()` records.
//! Each gate feeds one resident file from memory
//! ([`with_resident_source_override`]), so the reference tree is never edited.
//!
//! The first gate is the reported case (aeon lens-pins finding 1): a driver const
//! bound to `extern("NO_SUCH_SYMBOL_LENS_PIN")`, read by the driver's eight
//! `ensure(cycles(..) >= YM_ADDR_TO_DATA_MIN_T)` guards, used to emit a byte-identical
//! blob.
//!
//! REFERENCE-DEPENDENT like `seam1_native_link`: absent a tree every gate skips,
//! unless `SIGIL_STRICT_GATE=1` makes the missing tree a failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-harness --test seam1_link_verdict
//! ```

use sigil_harness::seam1::{native_blob_checked, with_resident_source_override};
use sigil_harness::test_support::reference_tree;
use std::path::{Path, PathBuf};

const DRIVER: &str = "engine/sound/z80_sound_driver.emp";
/// The driver's YM floor declaration, the line every gate rewrites.
const YM_LINE: &str = "const YM_ADDR_TO_DATA_MIN_T = 8";
const UNKNOWN: &str = "NO_SUCH_SYMBOL_LENS_PIN";

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

/// The driver's text with [`YM_LINE`] replaced by `replacement`.
fn driver_with(aeon: &Path, replacement: &str) -> String {
    let src = std::fs::read_to_string(aeon.join(DRIVER)).expect("read the driver");
    assert_eq!(
        src.matches(YM_LINE).count(),
        1,
        "{DRIVER} no longer declares `{YM_LINE}` exactly once; re-point these gates"
    );
    let cycle_guards = src.matches(">= YM_ADDR_TO_DATA_MIN_T").count();
    assert!(cycle_guards > 0, "{DRIVER} has no guard reading the YM floor; re-point these gates");
    src.replacen(YM_LINE, replacement, 1)
}

/// `path:line:col` of the first `needle` in `text`, in the located-diagnostic dialect.
fn located(text: &str, needle: &str) -> String {
    let at = text.find(needle).unwrap_or_else(|| panic!("`{needle}` not in the edited driver"));
    let before = &text[..at];
    let line = before.matches('\n').count() + 1;
    let col = at - before.rfind('\n').map_or(0, |i| i + 1) + 1;
    format!("{DRIVER}:{line}:{col}")
}

fn blob_with_driver(aeon: &Path, text: &str, debug: bool) -> Result<Vec<u8>, String> {
    with_resident_source_override(DRIVER, text, || native_blob_checked(aeon, debug, None))
}

/// THE REPORTED CASE: the const's `extern()` is refused once, at its own line and
/// column, in both shapes; the eight guards that read it add nothing.
#[test]
fn an_unknown_extern_read_by_the_driver_cycle_guards_is_refused() {
    let Some(aeon) = sound_tree() else { return };
    let text = driver_with(&aeon, &format!("const YM_ADDR_TO_DATA_MIN_T = extern(\"{UNKNOWN}\")"));
    let want = format!(
        "{}: [Error] [extern.unknown] `extern(\"{UNKNOWN}\")`",
        located(&text, "extern(\"NO_SUCH")
    );
    for debug in [false, true] {
        let err = blob_with_driver(&aeon, &text, debug).expect_err("an unknown name must not link");
        assert!(err.contains(&want), "debug={debug}: want `{want}` in:\n{err}");
        assert_eq!(err.matches("[extern.unknown]").count(), 1, "one refusal, not one per guard:\n{err}");
    }
}

/// An item-position guard in a resident module was dropped with the module's link
/// asserts; its unknown name is refused at the reference.
#[test]
fn an_unknown_extern_in_a_resident_item_guard_is_refused() {
    let Some(aeon) = sound_tree() else { return };
    let text = driver_with(
        &aeon,
        &format!("{YM_LINE}\nensure(extern(\"{UNKNOWN}\") == 1, \"resident item-position probe\")"),
    );
    let want = format!("{}: [Error] [extern.unknown]", located(&text, "extern(\"NO_SUCH"));
    let err = blob_with_driver(&aeon, &text, false).expect_err("an unknown name must not link");
    assert!(err.contains(&want), "want `{want}` in:\n{err}");
}

/// A defined blob label resolves in a resident module and its guard is DECIDED: a
/// true guard links to the pristine bytes (an `ensure` emits none), and a false
/// one fails with its own message at its own line.
#[test]
fn a_defined_extern_in_a_resident_module_resolves_and_its_guard_is_decided() {
    let Some(aeon) = sound_tree() else { return };
    for debug in [false, true] {
        let pristine = native_blob_checked(&aeon, debug, None)
            .unwrap_or_else(|e| panic!("the pristine resident blob must link: {e}"));

        let holds = driver_with(
            &aeon,
            &format!("{YM_LINE}\nensure(extern(\"Sequencer_Frame\") >= 0, \"resident known-name probe\")"),
        );
        let bytes = blob_with_driver(&aeon, &holds, debug)
            .unwrap_or_else(|e| panic!("debug={debug}: a defined name must resolve: {e}"));
        assert_eq!(bytes, pristine, "debug={debug}: a passing guard must not move a byte");

        // $FFFF is past the Z80's 8 KB RAM, so no resident label can sit there.
        let fails = driver_with(
            &aeon,
            &format!("{YM_LINE}\nensure(extern(\"Sequencer_Frame\") == $FFFF, \"resident guard decided\")"),
        );
        let err = blob_with_driver(&aeon, &fails, debug)
            .expect_err("a false guard over a defined name must fail the link");
        let want = format!("{}: [Error] resident guard decided", located(&fails, "ensure(extern(\"Sequencer_Frame\")"));
        assert!(err.contains(&want), "debug={debug}: want `{want}` in:\n{err}");
    }
}
