//! A `jsr`/`jmp` to a symbol nothing defines is a DIAGNOSTIC, not a panic.
//!
//! ## What reached the user
//!
//! ```text
//! thread 'main' panicked at crates/sigil-ir/src/lib.rs:424:21:
//! internal error: entered unreachable code: JmpJsrSym must be lowered by
//! resolve_layout before layout/link
//! ```
//!
//! exit 101, no source location, for the one-line program `jsr Nowhere`. The
//! front end's convergence path defers a `jsr`/`jmp` whose bare-symbol target
//! still folds to Poison as a `Fragment::JmpJsrSym`, because that is what a
//! genuine cross-seam reference to a sibling `.emp` `pub proc` looks like and it
//! is joined at LINK time. Nothing downstream of that decision asked what
//! happens when the symbol is not cross-seam but simply absent: `link()`
//! asserted the fragment had already been lowered, and asserted it with
//! `unreachable!`.
//!
//! The control says this was never about `jsr`: `bsr.w Nowhere` in the same
//! position reported `unresolved symbol` and exited 1 throughout. Only the
//! width-deferred path had no answer.
//!
//! ## Why it was visible only in a CLEAN file
//!
//! An unrelated error anywhere else in the unit makes the front end return
//! `Err`, so the CLI exits before `link()` ever runs and the panic never fires.
//! The internal error therefore struck exactly the person whose file was
//! otherwise correct, which is the person closest to a working build. Both
//! directions are gated below.

use std::process::Command;

/// Assemble `src` through the shipped CLI and return (exit code, stderr).
fn run(src: &str) -> (Option<i32>, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("root.asm");
    std::fs::write(&root, src).expect("write root.asm");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([root.to_str().unwrap(), "--hex"])
        .output()
        .expect("spawn sigil");
    (out.status.code(), String::from_utf8_lossy(&out.stderr).into_owned())
}

/// Nothing the assembler prints to a user may be an internal-error message, and
/// no exit status may be a panic's. 101 is checked by number as well as by text
/// because a panic whose message changed is still a panic.
fn assert_not_a_panic(code: Option<i32>, stderr: &str, what: &str) {
    assert_ne!(code, Some(101), "{what}: exit 101 is a panic. stderr:\n{stderr}");
    assert!(
        !stderr.contains("internal error: entered unreachable code"),
        "{what}: an internal-error message reached the user. stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("panicked at"),
        "{what}: a panic reached the user. stderr:\n{stderr}"
    );
}

#[test]
fn jsr_to_an_undefined_symbol_is_a_located_diagnostic() {
    let (code, stderr) = run("\tcpu\t68000\n\tjsr\tNowhere\n");
    assert_not_a_panic(code, &stderr, "jsr Nowhere");
    assert_eq!(code, Some(1), "a refused assembly exits 1. stderr:\n{stderr}");
    assert!(
        stderr.contains("`Nowhere`"),
        "the refusal must name the symbol. stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("root.asm(2): error: "),
        "the refusal must name the file and line it is about, in the same shape \
         as every other diagnostic. stderr:\n{stderr}"
    );
}

#[test]
fn jmp_to_an_undefined_symbol_is_a_located_diagnostic() {
    let (code, stderr) = run("\tcpu\t68000\n\tjmp\tNowhere\n");
    assert_not_a_panic(code, &stderr, "jmp Nowhere");
    assert_eq!(code, Some(1), "a refused assembly exits 1. stderr:\n{stderr}");
    assert!(
        stderr.contains("`Nowhere`"),
        "the refusal must name the symbol. stderr:\n{stderr}"
    );
}

/// The other direction of the visibility asymmetry: with an unrelated error in
/// the same file the panic never fired, so a gate that only ever probed a dirty
/// file would have been green before the fix. It must stay non-panicking here
/// too, and the unrelated error must still be reported.
#[test]
fn an_undefined_jsr_beside_an_unrelated_error_still_never_panics() {
    let (code, stderr) = run("\tcpu\t68000\n\tjsr\tNowhere\n\tdc.l\t$11111111,\n");
    assert_not_a_panic(code, &stderr, "jsr Nowhere + an unrelated error");
    assert_eq!(code, Some(1), "a refused assembly exits 1. stderr:\n{stderr}");
}

/// The control the brief supplies: the fault was never register- or
/// `jsr`-specific in its CAUSE, but `bsr.w` took a different path and was
/// already loud. It must stay loud, and it gains the location every other
/// diagnostic has.
#[test]
fn bsr_to_an_undefined_symbol_stays_loud_and_gains_a_location() {
    let (code, stderr) = run("\tcpu\t68000\n\tbsr.w\tNowhere\n");
    assert_not_a_panic(code, &stderr, "bsr.w Nowhere");
    assert_eq!(code, Some(1), "a refused assembly exits 1. stderr:\n{stderr}");
    assert!(
        stderr.contains("`Nowhere`"),
        "the refusal must name the symbol. stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("root.asm(2): error: "),
        "a LINK diagnostic must be located like a front-end one. stderr:\n{stderr}"
    );
}

/// Over-firing gate. A `jsr` to a label defined LATER in the same file is the
/// ordinary shape, it assembles, and the refusal must not touch it. The bytes
/// are pinned so a guard that turned the whole path off would be caught.
#[test]
fn a_forward_jsr_to_a_defined_label_still_assembles() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("root.asm");
    std::fs::write(&root, "\tcpu\t68000\n\tjsr\tLater\nLater:\n\trts\n").expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([root.to_str().unwrap(), "--hex"])
        .output()
        .expect("spawn sigil");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "a forward jsr must assemble. stderr:\n{stderr}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "4E B8 00 04 4E 75",
        "jsr Later + rts, abs.w, unchanged by the refusal"
    );
}
