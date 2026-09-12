//! A failed `sigil <root.asm>` run says so on STDOUT.
//!
//! The `message` directive writes to stdout and diagnostics write to stderr,
//! so `sigil root.asm > build.log` on a failing run left a log holding the
//! author's reassuring line and not one word about the failure. Against a C
//! compiler the asymmetry is the whole defect: `cc foo.c > log` on a failing
//! build leaves the log EMPTY, which misleads nobody, where this left
//! `Uncompressed driver size: 1BC6h bytes.` and exit 1.
//!
//! Moving the stream was the remedy the finding proposed and it was refused on
//! the consumers, see `as_message_stdout.rs`. The remedy here instead makes
//! stdout unable to END a failing run on a reassuring note.
//!
//! Two kinds of gate live in this file, and they are not redundant:
//!
//! - the BEHAVIOURAL ones run the binary and read its two streams;
//! - the STRUCTURAL one reads `run_asm`'s own body out of `main.rs` and fails
//!   if a bare `process::exit(1)` reappears in it. The behavioural gates can
//!   only speak for the failure paths they happen to reach, and `run_asm` has
//!   six. A seventh added later would be invisible to every gate that works by
//!   provoking a failure, so the property is stated over the source instead,
//!   where it has no population to enumerate.

use std::process::{Command, Output};

/// The binary under test, built by cargo for this integration target.
const SIGIL: &str = env!("CARGO_BIN_EXE_sigil");

/// The binary's own source, read at compile time. Same device as
/// `cli_help.rs`, and for the same reason.
const SOURCE: &str = include_str!("../src/main.rs");

/// Assemble `body` (after a `cpu 68000` line) from a fresh directory.
fn run(body: &str) -> Output {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, format!("\tcpu 68000\n\tpadding off\n\torg 0\n{body}")).expect("write");
    Command::new(SIGIL).arg(&path).output().expect("spawn sigil")
}

/// The last line of `out`'s stdout, or `""` when there is none.
fn last_stdout_line(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .next_back()
        .unwrap_or("")
        .to_string()
}

/// The finding's own transcript: a `message` above a line that fails. The log
/// a person keeps must not end on the reassuring line.
#[test]
fn a_captured_stdout_log_of_a_failing_run_does_not_end_on_the_message() {
    let out = run("\tmessage \"driver size is 42 bytes\"\n\tdc.b nothing_defines_this\n\tend\n");
    assert_eq!(out.status.code(), Some(1), "the program must fail");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        stdout.contains("driver size is 42 bytes"),
        "the message still prints, on stdout, as asl prints it: {stdout:?}"
    );
    assert_eq!(
        last_stdout_line(&out),
        "assembly failed: 1 error (reported on stderr)",
        "stdout: {stdout:?}"
    );
}

/// A SUCCEEDING run says nothing about failure: its stdout is the message and
/// then the `built:` line every succeeding run ends on (pinned in
/// `asm_output_disposition.rs`, not duplicated here). This is the control that
/// stops the failure line from being bought by printing it always, which would
/// satisfy every other gate here and destroy the meaning of the line.
#[test]
fn a_succeeding_run_says_nothing_about_failure() {
    let out = run("\tmessage \"driver size is 42 bytes\"\n\tdc.b 1\n\tend\n");
    assert_eq!(out.status.code(), Some(0), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "stdout must hold the message and the success line, nothing else: {stdout:?}"
    );
    assert_eq!(lines[0], "driver size is 42 bytes");
    assert!(lines[1].starts_with("built: "), "stdout: {stdout:?}");
}

/// A failing run with NO `message` still says so. The line is a property of
/// failing, not of having printed something first.
///
/// The assertion was an exact string equality until the UXa F5 closure put an
/// incompleteness caveat above the failure line (this probe stops at LINK, so
/// the image checks do not run). It is stated over stdout's SHAPE instead of
/// being deleted or loosened to a `contains`: two lines and no more, the last
/// being the failure line, which is what stops a reassuring line from returning
/// here. The caveat's own wording is pinned in
/// `partial_error_list_stage_note.rs` and deliberately not duplicated.
#[test]
fn a_failing_run_with_no_message_still_says_it_failed() {
    let out = run("\tdc.b nothing_defines_this\n\tend\n");
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "stdout must hold the incompleteness caveat and the failure line, nothing else: {stdout:?}"
    );
    assert!(
        lines[0].starts_with("this error list may be incomplete:"),
        "stdout: {stdout:?}"
    );
    assert_eq!(lines[1], "assembly failed: 1 error (reported on stderr)");
}

/// The count is the errors actually rendered, and it pluralises. Two
/// errors rather than one, so a hard-coded `1` cannot pass.
#[test]
fn the_count_is_the_number_of_diagnostics_and_it_pluralises() {
    let out = run("\tdc.b nothing_defines_this\n\tdc.b nor_this_one\n\tend\n");
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    let rendered = err.lines().filter(|l| l.contains("error:")).count();
    assert_eq!(rendered, 2, "the probe must render two diagnostics: {err}");
    assert_eq!(
        last_stdout_line(&out),
        format!("assembly failed: {rendered} errors (reported on stderr)"),
        "the count is derived from what was rendered, not from a literal"
    );
}

/// Count the rendered stderr lines at `level` (`error` or `warning`), so each
/// test below can confirm its probe produced the population its expected line
/// names before asserting on that line.
fn rendered_at(out: &Output, level: &str) -> usize {
    let needle = format!(": {level}: ");
    String::from_utf8_lossy(&out.stderr).lines().filter(|l| l.contains(&needle)).count()
}

/// A warning is not counted as an error. One error and TWO warnings, so a line
/// that folds warnings into errors (`3 errors`) and one that swaps the two counts
/// (`2 errors, 1 warning`) both fail. The front end refuses this probe, so the
/// three diagnostics arrive in one list.
#[test]
fn a_front_end_failure_counts_its_warnings_apart_from_its_errors() {
    let out = run("\twarning \"first\"\n\twarning \"second\"\n\tzzbogus d0\n\tend\n");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(rendered_at(&out, "error"), 1, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(rendered_at(&out, "warning"), 2, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(stdout.contains("sigil stopped at the front end"), "stdout: {stdout:?}");
    assert_eq!(
        last_stdout_line(&out),
        "assembly failed: 1 error, 2 warnings (reported on stderr)",
        "stdout: {stdout:?}"
    );
}

/// The warnings a run printed are counted when a LATER stage fails. The front
/// end accepts this probe and prints its two warnings, then link refuses the
/// undefined symbol, so the error and the warnings come from two different lists.
/// Counting only the failing stage's list reads `1 error` here.
#[test]
fn a_later_stage_failure_counts_the_front_ends_warnings() {
    let out = run("\twarning \"first\"\n\twarning \"second\"\n\tdc.b nothing_defines_this\n\tend\n");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(rendered_at(&out, "error"), 1, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(rendered_at(&out, "warning"), 2, "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(stdout.contains("sigil stopped at link"), "stdout: {stdout:?}");
    assert_eq!(
        last_stdout_line(&out),
        "assembly failed: 1 error, 2 warnings (reported on stderr)",
        "stdout: {stdout:?}"
    );
}

/// The failure line is on stdout, NOT on stderr. It exists for the stream a
/// habitual `> build.log` captures; putting it on stderr would be a second
/// diagnostic and would close nothing.
#[test]
fn the_failure_line_is_on_stdout_and_not_on_stderr() {
    let out = run("\tdc.b nothing_defines_this\n\tend\n");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!err.contains("assembly failed"), "stderr: {err}");
}

/// The body of `run_asm` as `main.rs` declares it, from its `fn` line to the
/// closing brace at column zero.
///
/// A slice this returned empty or short would make the gate below green for
/// the wrong reason, so it asserts its own shape before returning.
fn run_asm_body() -> &'static str {
    let start = SOURCE.find("\nfn run_asm(").expect("main.rs declares fn run_asm");
    let rest = &SOURCE[start + 1..];
    let end = rest.find("\n}\n").expect("fn run_asm closes at column zero") + 3;
    let body = &rest[..end];
    assert!(
        body.len() > 500,
        "run_asm's body parsed to {} bytes, which is too short to be the function",
        body.len()
    );
    assert!(
        body.contains("assemble_root_located_warned"),
        "the slice is not run_asm's body"
    );
    body
}

/// **The structural gate.** Every failure exit in `run_asm` goes through
/// `fail_asm`, so the guarantee is a property of one function rather than of a
/// list of call sites a later edit can quietly leave out.
///
/// `process::exit(2)` is untouched by this: those are USAGE refusals, which
/// print their own usage line and never claim to have assembled anything.
#[test]
fn asm_failure_exits_all_go_through_fail_asm() {
    let body = run_asm_body();
    assert!(
        !body.contains("process::exit(1)"),
        "run_asm exits 1 directly somewhere. Every failure exit on this route must call \
         fail_asm, or a failing run can leave a stdout log that reads as a successful \
         one. Body:\n{body}"
    );
    let calls = body.matches("fail_asm(").count();
    assert!(
        calls >= 6,
        "run_asm has {calls} fail_asm call sites, fewer than the six failure paths this \
         gate was written over. If a path was removed, lower this number deliberately; \
         if the gate stopped seeing the body, fix the slice."
    );
}
