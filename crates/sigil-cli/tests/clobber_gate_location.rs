//! `sigil build`'s contract closure gate refuses `[proc.clobber-undeclared]`
//! closure firings with a LOCATION (A7). Before, the gate printed a count and
//! nothing else, so the author of a 142-module corpus was left to rediscover
//! which proc, which register, and which call brought it. Each firing line now
//! names the proc, the register, the callee that carries it (or "in its own
//! body"), and the proc's `file:line:col`.
//!
//! The real binary over a self-contained described project (one 68k module in the
//! engine root, `test_support::write_project_fixture`, built with `--project` since G0b
//! Task 11), the same harness as `printed_families_gate.rs`. The fixture's contract
//! baselines are empty, so the gate's verdict is the fixture's alone: a clean control
//! passes it, and the `SIGIL_CONTRACTS=0` opt-out stands beside it.

use std::path::Path;
use std::process::Command;

/// `Q` declares what it writes. `P` declares `d0` but calls `Q`, which brings
/// `d1` (a transitive firing, carried by `Q`). `R` writes `d2` itself (a direct
/// firing). `U` makes an unbounded indirect call (the ⊤ firing). `S` writes
/// `d3` and calls ITSELF: its own body, never "through `S`" (A7 fix round 1).
const FIXTURE: &str = "module engine.clobfix
pub proc Q () clobbers(d0-d1) {
    moveq   #0, d0
    moveq   #0, d1
    rts
}
pub proc P () clobbers(d0) {
    jbsr    Q
    rts
}
pub proc R () clobbers(d0) {
    moveq   #0, d2
    rts
}
pub proc U () clobbers(d0) {
    jsr     (a0)
    rts
}
pub proc S () clobbers(d0) {
    moveq   #0, d3
    tst.w   d0
    beq     .out
    jbsr    S
.out:
    rts
}
";

/// The same procs with honest contracts: nothing for the closure to fire on.
const CLEAN: &str = "module engine.clobfix
pub proc Q () clobbers(d0-d1) {
    moveq   #0, d0
    moveq   #0, d1
    rts
}
pub proc P () clobbers(d0-d1) {
    jbsr    Q
    rts
}
pub proc R () clobbers(d0/d2) {
    moveq   #0, d2
    rts
}
";

const HEADER: &str = "error: [proc.clobber-undeclared] closure firings";

fn tree(root: &Path, src: &str) {
    sigil_harness::test_support::write_project_fixture(root, "g");
    std::fs::write(root.join("engine/clobfix.emp"), src).unwrap();
}

/// `sigil build --project <root>/games/g --shape release` over the fixture, returning stderr.
fn build_stderr(root: &Path, contracts_off: bool) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sigil"));
    cmd.args(["build", "--project"]).arg(root.join("games/g")).args(["--shape", "release", "-o"]).env_remove("AEON_DIR");
    cmd.arg(root.join("out.bin"));
    if contracts_off {
        cmd.env("SIGIL_CONTRACTS", "0");
    } else {
        cmd.env_remove("SIGIL_CONTRACTS");
    }
    let out = cmd.output().expect("run sigil build");
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// `file:line:col` of the first occurrence of `needle` in `src`: 1-based line,
/// 1-based column of the needle itself. A proc declaration's span starts at its
/// `proc` keyword (after any `pub`), so the needle is `"proc NAME"`.
fn loc(file: &Path, src: &str, needle: &str) -> String {
    let (i, col) = src
        .lines()
        .enumerate()
        .find_map(|(i, l)| l.find(needle).map(|c| (i, c + 1)))
        .unwrap_or_else(|| panic!("fixture has no line containing {needle:?}"));
    format!("{}:{}:{col}", file.display(), i + 1)
}

#[test]
fn the_clobber_refusal_names_proc_register_path_and_line() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), FIXTURE);
    let err = build_stderr(tmp.path(), false);
    let f = tmp.path().join("engine/clobfix.emp");

    assert!(
        err.contains(
            "error: [proc.clobber-undeclared] closure firings, 4 firing(s); this family is \
             zero-firing by contract:"
        ),
        "the gate must refuse the closure family with its count, got:\n{err}"
    );
    let p = format!(
        "  {}: [proc.clobber-undeclared] `P` destroys `d1` through `Q`, which its \
         clobbers(...) does not declare",
        loc(&f, FIXTURE, "proc P")
    );
    assert!(err.contains(&p), "missing refusal line\n{p}\ngot:\n{err}");
    let r = format!(
        "  {}: [proc.clobber-undeclared] `R` destroys `d2` in its own body, which its \
         clobbers(...) does not declare",
        loc(&f, FIXTURE, "proc R")
    );
    assert!(err.contains(&r), "missing refusal line\n{r}\ngot:\n{err}");
    let u = format!(
        "  {}: [proc.clobber-undeclared] `U` reaches an indirect call with no declared bound, \
         so it may destroy every register; bound the call with `as Type` or declare \
         `@allow(\"clobbers.unanalyzable\", \"<reason>\")`",
        loc(&f, FIXTURE, "proc U")
    );
    assert!(err.contains(&u), "missing refusal line\n{u}\ngot:\n{err}");
    let s = format!(
        "  {}: [proc.clobber-undeclared] `S` destroys `d3` in its own body, which its \
         clobbers(...) does not declare",
        loc(&f, FIXTURE, "proc S")
    );
    assert!(err.contains(&s), "missing refusal line\n{s}\ngot:\n{err}");
    assert!(!err.contains("through `S`"), "a proc is never its own carrier, got:\n{err}");
    // The old location-free form is gone.
    assert!(
        !err.contains("closure firings, 4 firing(s); this family is zero-firing by contract.\n"),
        "the old count-only refusal must not print, got:\n{err}"
    );
}

/// CONTROL: honest contracts reach the gate's verdict with no closure refusal.
#[test]
fn honest_contracts_draw_no_clobber_refusal() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), CLEAN);
    let err = build_stderr(tmp.path(), false);
    assert!(
        err.contains("contract closure gate: passed ("),
        "the control must pass the gate, or it shows nothing: {err}"
    );
    assert!(!err.contains(HEADER), "no closure refusal expected in:\n{err}");
}

/// CONTROL: `SIGIL_CONTRACTS=0` skips the gate before any refusal.
#[test]
fn sigil_contracts_0_still_skips_the_clobber_refusal() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), FIXTURE);
    let err = build_stderr(tmp.path(), true);
    assert!(
        err.contains("warning: contract closure gate SKIPPED (SIGIL_CONTRACTS=0)."),
        "the opt-out must say it skipped, got:\n{err}"
    );
    assert!(!err.contains(HEADER), "no closure refusal expected when skipped, in:\n{err}");
}

/// `--report contracts` names the same carrier the refusal does: `P`'s row reads
/// `transitive d1 via Q`, `R`'s direct row carries no `via`.
#[test]
fn the_contract_report_names_the_same_carrier() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), FIXTURE);
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["build", "--project"])
        .arg(tmp.path().join("games/g"))
        .args(["--shape", "release", "--report", "contracts"])
        .env_remove("AEON_DIR")
        .env_remove("SIGIL_CONTRACTS")
        .output()
        .expect("run sigil build --report contracts");
    let report = String::from_utf8_lossy(&out.stdout);
    let rows: Vec<&str> = report.lines().map(str::trim_end).collect();
    assert!(rows.contains(&format!("  {:<28} transitive d1 via Q", "P").as_str()), "got:\n{report}");
    assert!(rows.contains(&format!("  {:<28} direct     d2", "R").as_str()), "got:\n{report}");
}
