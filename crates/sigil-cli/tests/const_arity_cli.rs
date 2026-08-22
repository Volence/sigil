//! Const array-arity enforcement, observed through the `sigil` BINARY.
//!
//! `crates/sigil-frontend-emp/tests/const_array_arity.rs` proves the check at the
//! library level. Aeon does not call the library — it invokes the `sigil`
//! executable, so a property that is green in a frontend unit test can still be
//! absent from the interface aeon actually depends on (a stale binary is enough).
//! These two tests drive the built executable over committed fixtures, which is
//! the only place that end of the contract is observable.
//!
//! The pair is DISCRIMINATING, and the control arm is the point. A refusal on its
//! own proves only that the compiler says no; it says nothing about whether the
//! check distinguishes a wrong shape from a right one. The control — the same
//! module with the defect removed — is what proves the refusal tracks the defect
//! rather than the fixture. Both arms are load-bearing: poison must fail, control
//! must build.
//!
//! The asserted wording `array length mismatch: expected N element(s), got M` is a
//! cross-repo interface: aeon fixtures assert on that exact string, so rephrasing
//! the diagnostic breaks them.

use std::process::Command;

/// The committed fixture directory the neighbouring CLI vector tests use.
const VECTORS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/vectors");

/// Run `sigil emp <file>` from the fixture directory.
fn run_emp(file: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["emp", file])
        .current_dir(VECTORS)
        .output()
        .expect("run sigil emp")
}

/// POISON ARM: a typed `const` whose nested array tail carries one element against
/// a declared length of zero. The binary must refuse — non-zero exit, arity
/// diagnostic — and must not report a successful build.
#[test]
fn cli_refuses_wrong_const_array_arity() {
    let out = run_emp("const_arity_poison.emp");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!out.status.success(), "expected non-zero exit; stdout: {stdout}, stderr: {stderr}");
    assert!(
        stderr.contains("array length mismatch: expected 0 element(s), got 1"),
        "stderr: {stderr}"
    );
    assert!(!stdout.contains("built:"), "a refused module must not report a build: {stdout}");
}

/// CONTROL ARM: the same module with the arity defect removed. The binary must
/// build it clean — zero exit, no arity diagnostic. Without this arm the poison
/// arm is satisfied by a compiler that rejects everything.
#[test]
fn cli_accepts_correct_const_array_arity() {
    let out = run_emp("const_arity_control.emp");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "expected clean build; stderr: {stderr}");
    assert!(!stderr.contains("array length mismatch"), "control must not trip arity: {stderr}");
    assert!(stdout.contains("built:"), "stdout: {stdout}");
}
