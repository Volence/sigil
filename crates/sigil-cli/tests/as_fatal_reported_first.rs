//! When a source's own `fatal` fires, the `fatal` is the first error a user
//! reads, and the errors that exist only because assembly stopped at it are
//! counted in one note instead of being reported as real.
//!
//! ## What was wrong
//!
//! A `fatal` stops the pass it fires on, on every pass, so a label defined
//! after it never gets a value. Every reference to such a label written BEFORE
//! the `fatal` was then reported as `unresolved`, and those follow-ons came
//! ahead of the cause. On the Sonic 3&K disassembly with
//! `FixMusicAndSFXDataBugs=1` asl prints one line, the source's `fatal`; sigil
//! printed 563 error lines, and the one naming the cause was the 468th.
//!
//! ## Which errors are consequences of the stop
//!
//! An error is a consequence of the stop when it goes away once assembly is
//! allowed to continue past the `fatal`. That is decided by assembling again
//! with `fatal` not stopping the pass, and keeping every error from the real
//! run that the continued run raises too, at the same place with the same
//! text. So a bad operand before the `fatal` stays, and so does a symbol that
//! is defined nowhere at all: neither goes away when the rest of the file is
//! assembled. Nothing the continued run raises past the `fatal` is reported,
//! because the author's `fatal` said assembly must not get there.
//!
//! The build fails exactly as before; only the order and the follow-ons change.

use std::process::{Command, Output};

/// The binary under test, built by cargo for this integration target.
const SIGIL: &str = env!("CARGO_BIN_EXE_sigil");

/// Assemble `src` as `probe.asm` from a fresh directory, named relatively so
/// every located diagnostic reads `probe.asm(LINE):COL:`.
fn run(src: &str) -> Output {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("probe.asm"), src).expect("write");
    Command::new(SIGIL)
        .current_dir(dir.path())
        .arg("probe.asm")
        .output()
        .expect("spawn sigil")
}

fn stderr_lines(out: &Output) -> Vec<String> {
    String::from_utf8_lossy(&out.stderr).lines().map(str::to_string).collect()
}

fn last_stdout_line(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).lines().next_back().unwrap_or("").to_string()
}

/// The small form: a reference to a label that is defined after the `fatal`.
/// The `fatal` is the only error, it comes first, and the one follow-on is
/// counted in the note rather than reported.
#[test]
fn a_fatal_is_the_first_error_and_its_forward_reference_is_not_reported() {
    let out = run("\tcpu 68000\n\tmove.w #Later,d0\n\tfatal \"stop here\"\n\tnop\nLater:\n");
    let lines = stderr_lines(&out);
    assert_eq!(out.status.code(), Some(1), "the build must still fail: {lines:?}");
    assert_eq!(
        lines,
        vec![
            "probe.asm(3):2: error: stop here".to_string(),
            "note: 1 further error not reported: it disappears when assembly continues \
             past the fatal, so it exists only because assembly stopped there"
                .to_string(),
        ],
        "the fatal first, the follow-on counted and not reported"
    );
    assert_eq!(
        last_stdout_line(&out),
        "assembly failed: 1 error, 1 note (reported on stderr)"
    );
}

/// Errors that do NOT go away when assembly continues are still reported: a
/// bad operand before the `fatal`, and a symbol defined nowhere. The forward
/// reference beside them is still counted, not reported.
#[test]
fn an_unrelated_error_before_the_fatal_is_still_reported() {
    let out = run(
        "\tcpu 68000\n\tmoveq #1000,d0\n\tjsr Nowhere\n\tmove.w #Later,d0\n\tfatal \"stop here\"\nLater:\n",
    );
    let lines = stderr_lines(&out);
    assert_eq!(out.status.code(), Some(1), "the build must still fail: {lines:?}");
    assert_eq!(
        lines.first().map(String::as_str),
        Some("probe.asm(5):2: error: stop here"),
        "the fatal is the first line: {lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("probe.asm(2):2: error: ") && l.contains("moveq")),
        "the out-of-range moveq before the fatal is still reported: {lines:?}"
    );
    assert!(
        lines.contains(&"probe.asm(3):2: error: unresolved symbol `Nowhere` in operand".to_string()),
        "a symbol defined nowhere is still reported: {lines:?}"
    );
    assert!(
        !lines.iter().any(|l| l.contains("Later") || l.starts_with("probe.asm(4)")),
        "the forward reference to a label past the fatal is not reported: {lines:?}"
    );
    assert_eq!(
        lines.last().map(String::as_str),
        Some(
            "note: 1 further error not reported: it disappears when assembly continues \
             past the fatal, so it exists only because assembly stopped there"
        ),
        "the suppressed follow-on is counted: {lines:?}"
    );
    assert_eq!(lines.len(), 4, "fatal, moveq, Nowhere, note: {lines:?}");
    assert_eq!(
        last_stdout_line(&out),
        "assembly failed: 3 errors, 1 note (reported on stderr)"
    );
}

/// With no follow-on to suppress there is no note, and an error raised before
/// the `fatal` is moved behind it rather than dropped.
#[test]
fn a_fatal_with_nothing_suppressed_prints_no_note() {
    let out = run("\tcpu 68000\n\tmoveq #1000,d0\n\tfatal \"stop here\"\n");
    let lines = stderr_lines(&out);
    assert_eq!(out.status.code(), Some(1), "the build must still fail: {lines:?}");
    assert_eq!(lines.len(), 2, "the fatal and the moveq error, no note: {lines:?}");
    assert_eq!(lines[0], "probe.asm(3):2: error: stop here", "the fatal first: {lines:?}");
    assert!(
        lines[1].starts_with("probe.asm(2):2: error: ") && lines[1].contains("moveq"),
        "the moveq error follows it: {lines:?}"
    );
    assert_eq!(last_stdout_line(&out), "assembly failed: 2 errors (reported on stderr)");
}
