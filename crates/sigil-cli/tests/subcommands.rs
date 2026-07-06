//! Integration smoke test for the `sigil build` / `sigil diff` subcommands.
//!
//! `sigil diff --aeon <aeon>` assembles the M0 integration harness's Region
//! A + Region B reference source and compares it byte-for-byte against the
//! committed golden blobs (see `sigil-harness`). This test reads the aeon
//! source tree, so it's `#[ignore]`d by default — run with `--ignored`.

use std::process::Command;

#[test]
#[ignore = "reads the aeon source tree; run with --ignored"]
fn sigil_diff_reports_byte_identity() {
    let aeon = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../aeon");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["diff", "--aeon", aeon])
        .output()
        .expect("run sigil diff");
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
}

/// `sigil emp <relative.emp>` run from the source directory must derive the
/// include-root from the file's own parent (here the empty parent → `.` →
/// `canonicalize(cwd)`), so a relative `embed("blob.bin")` resolves against the
/// cwd. This is the ONLY regression guard for `run_emp`'s parent/canonicalize
/// derivation (the unit test passes the root in explicitly).
#[test]
fn sigil_emp_derives_include_root_from_relative_source_path() {
    let vectors = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/vectors");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["emp", "prog.emp", "--hex"])
        .current_dir(vectors)
        .output()
        .expect("run sigil emp");
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("DE AD BE EF"), "stdout: {stdout}");
}

/// A comptime error (an `embed` of a missing file) must exit non-zero and print
/// the diagnostic — no partial/empty binary silently succeeding.
#[test]
fn sigil_emp_error_exits_nonzero() {
    let vectors = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/vectors");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["emp", "missing_embed.emp"])
        .current_dir(vectors)
        .output()
        .expect("run sigil emp");
    assert!(!out.status.success(), "expected non-zero exit");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("[embed.read]"),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
