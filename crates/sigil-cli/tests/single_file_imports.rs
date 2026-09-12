//! A single-file `sigil emp` build is a one-module program. Before
//! EMP-UNUSED-IMPORT-UNCHECKED it never read a `use` line at all, so
//! `use nowhere.{X}` built clean whenever nothing read `X`. The import rule now
//! applies: a `use` of any other module is refused at the `use`, and a `use` of the
//! file's own module must list its `pub` names.

use std::process::Command;

/// Build `src` as `solo.emp` in a scratch directory with no `--root`, returning
/// whether it succeeded and everything it printed.
fn build_single(src: &str) -> (bool, String) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("solo.emp");
    std::fs::write(&path, src).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["emp", path.to_str().unwrap(), "-o", dir.path().join("solo.bin").to_str().unwrap()])
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), text)
}

const BODY: &str = "pub proc init (a0: *u8) {\n    rts\n}\n";

/// THE ROW, single-file: an unread import of a module the build does not contain.
#[test]
fn an_unread_import_of_another_module_is_refused_at_the_use() {
    let (ok, text) = build_single(&format!("module solo\n\nuse nowhere.{{NOPE}}\n\n{BODY}"));
    assert!(!ok, "the build must fail: {text}");
    assert!(
        text.contains(
            "solo.emp:3:1: no module `nowhere` in this build: a single-file `sigil emp` \
             compiles only module `solo`"
        ),
        "{text}"
    );
}

/// The blank form binds nothing, and its module still has to exist.
#[test]
fn a_blank_import_of_another_module_is_refused() {
    let (ok, text) = build_single(&format!("module solo\n\nuse nowhere._\n\n{BODY}"));
    assert!(!ok, "the build must fail: {text}");
    assert!(text.contains("no module `nowhere` in this build"), "{text}");
}

/// A `use` of the file's own module is held to the rule against that module.
#[test]
fn an_own_module_import_of_a_missing_name_is_refused() {
    let (ok, text) = build_single(&format!("module solo\n\nuse solo.{{NOPE}}\n\n{BODY}"));
    assert!(!ok, "the build must fail: {text}");
    assert!(text.contains("solo.emp:3:1: module `solo` has no `pub` name `NOPE`"), "{text}");
}

/// The accept arms: no import, and an own-module import of a real `pub` item.
#[test]
fn an_import_free_file_and_a_valid_own_import_both_build() {
    let (ok, text) = build_single(&format!("module solo\n\n{BODY}"));
    assert!(ok, "an import-free file must build: {text}");
    let (ok, text) = build_single(&format!("module solo\n\nuse solo.{{init}}\n\n{BODY}"));
    assert!(ok, "an own import of a `pub` item must build: {text}");
}
