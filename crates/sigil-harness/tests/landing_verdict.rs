//! THE LANDING WRAPPER'S VERDICT, PROVEN BY RUNNING IT OVER A LOG.
//!
//! `scripts/landing-run.sh` reads its own log back to decide `RESULT` and its exit
//! code. Four documents state that the landing bar fails a strict run carrying a
//! `skip:` line; until `--verdict-only` existed nothing exercised that rule short of a
//! full suite run, and the rule was in fact absent: the count printed as a WARNING
//! beside `RESULT GREEN`, exit 0. The tests here put fixture logs through the SAME
//! verdict code path the wrapper uses after a real run (`--verdict-only` takes the
//! other branch of one dispatch; there is no second verdict), so the rule is now a
//! test that goes red the moment someone drops `SKIPS` out of the condition again.
//!
//! What is measured: the exit code AND the `RESULT` line, because the two have
//! disagreed in this script's history and the exit code is the half a merge reads.
//!
//! Scratch is on disk, never under the system temp directory (tmpfs here), derived
//! from this binary's own location inside the target directory.
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the harness crate sits two levels under the repo root")
        .to_path_buf()
}

fn script() -> PathBuf {
    let p = repo_root().join("scripts/landing-run.sh");
    assert!(
        p.is_file(),
        "COULD NOT MEASURE: the landing wrapper is not at {}, nothing below would be \
         judging the real script",
        p.display()
    );
    p
}

fn scratch(tag: &str) -> PathBuf {
    let exe = std::env::current_exe().expect("COULD NOT MEASURE: this test has no path");
    let target = exe
        .ancestors()
        .nth(3)
        .expect("COULD NOT MEASURE: the test binary is not nested under a target directory")
        .to_path_buf();
    let dir = target
        .join("landing-verdict-logs")
        .join(format!("{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("mkdir {}: {e}", dir.display()));
    dir
}

/// The stamp the wrapper writes before a run, in the shape `load_verdict_inputs` reads.
const STAMP: &str = "\
# sigil landing run
# started (UTC)  2026-09-07T00:00:00Z
# pwd            /fixture/sigil
# sigil HEAD     0123456789abcdef0123456789abcdef01234567
# sigil branch   fixture (clean)
# AEON_DIR       /fixture/aeon (step 1: explicit --aeon)
# aeon HEAD      fedcba9876543210fedcba9876543210fedcba98
# aeon branch    master (clean)
# aeon ROMs      all four present
# TARGET_DIR     /fixture/.target-land
# scoped         no (full workspace)
# baseline       3

##### CLIPPY SPAN, cargo clippy --release --workspace --all-targets -- -D warnings
    Finished `release` profile [optimized] target(s) in 1.00s
CLIPPY_EXIT=0
##### CLIPPY SPAN ENDS
##### TEST SPAN, cargo test --release --no-fail-fast --workspace -- --nocapture
running 3 tests
test a_gate ... ok
";

const TAIL: &str = "\
test b_gate ... ok
test c_gate ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
CARGO_EXIT=0
# finished (UTC) 2026-09-07T00:01:00Z
";

/// A three-test green log; `middle` is spliced between the first and second test lines.
fn fixture(dir: &Path, name: &str, middle: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, format!("{STAMP}{middle}{TAIL}"))
        .unwrap_or_else(|e| panic!("write {}: {e}", p.display()));
    p
}

/// Run `landing-run.sh --verdict-only <log>` and return (exit code, merged output).
fn judge(log: &Path, extra: &[&str]) -> (i32, String) {
    let out = Command::new("bash")
        .arg(script())
        .arg("--verdict-only")
        .arg(log)
        .args(extra)
        .current_dir(repo_root())
        .output()
        .expect("COULD NOT MEASURE: bash could not run the wrapper");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    let code = out.status.code().unwrap_or_else(|| panic!("the wrapper was killed by a signal:\n{text}"));
    (code, text)
}

/// THE GATE. A `skip:` line inside the test span makes the verdict FAILED and the exit
/// code 1, with every test green and the lint bar clean. The clean twin of the same log
/// is GREEN and exits 0, which is the control that proves the red comes from the skip
/// line and not from the fixture's shape.
#[test]
fn a_skip_line_under_strict_fails_the_landing_verdict() {
    let dir = scratch("skip");
    let with_skip = fixture(&dir, "with-skip.log", "skip: reference ROM not at /fixture/aeon/s4.bin (set AEON_DIR)\n");
    let clean = fixture(&dir, "clean.log", "");

    let (code, text) = judge(&clean, &[]);
    assert_eq!(code, 0, "the CONTROL log (no skip line) must be GREEN, got exit {code}:\n{text}");
    assert!(text.contains("RESULT          GREEN"), "control verdict line missing:\n{text}");
    assert!(text.contains("skip lines      0"), "control must count zero skip lines:\n{text}");

    let (code, text) = judge(&with_skip, &[]);
    assert_eq!(
        code, 1,
        "a strict-run log carrying a `skip:` line must exit 1 (the suite-FAILED code), got {code}:\n{text}"
    );
    assert!(text.contains("RESULT          FAILED"), "the verdict line must say FAILED:\n{text}");
    assert!(
        text.contains("1 skip line(s) survived SIGIL_STRICT_GATE=1"),
        "the verdict must name the skip count as the reason:\n{text}"
    );
    assert!(!text.contains("RESULT          GREEN"), "a skip-carrying log printed GREEN:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The other spelling the counter matches. 27 sites once said `skipping` and were
/// invisible to a `skip:`-only grep; the verdict must fail on that spelling too.
///
/// THE TEST'S OWN NAME MUST NOT SPELL IT. `cargo test` prints every test name into
/// the log the wrapper counts, so a name carrying that word is itself a skip line in
/// a strict landing run. Measured: the first name of this test did exactly that.
#[test]
fn the_second_spelling_fails_the_landing_verdict_too() {
    let dir = scratch("skipping");
    let log = fixture(&dir, "skipping.log", "skipping gate_x: no reference tree\n");
    let (code, text) = judge(&log, &[]);
    assert_eq!(code, 1, "`skipping` inside the test span must exit 1, got {code}:\n{text}");
    assert!(text.contains("RESULT          FAILED"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A `skip:` line in the CLIPPY span is a quoted source line, not a gate; the counter is
/// scoped to the test span and the verdict stays GREEN.
#[test]
fn a_skip_word_quoted_by_clippy_is_not_a_skipped_gate() {
    let dir = scratch("clippy-quote");
    let p = dir.join("clippy-quote.log");
    let text = format!("{STAMP}{TAIL}").replace(
        "    Finished `release`",
        "warning: unused\n  --> crates/x/tests/y.rs:1:1\n   |\n 1 |     eprintln!(\"skip: x\");\n    Finished `release`",
    );
    std::fs::write(&p, text).unwrap();
    let (code, out) = judge(&p, &[]);
    assert_eq!(code, 0, "a skip word inside the clippy span must not fail the run:\n{out}");
    assert!(out.contains("skip lines      0"), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// UNMEASURABLE IS NOT GREEN. A log with no `CARGO_EXIT=` line is a run that never
/// finished; `--verdict-only` refuses it (exit 2) rather than defaulting the exit code.
#[test]
fn a_log_without_an_exit_line_is_refused_not_judged() {
    let dir = scratch("no-exit");
    let p = dir.join("unfinished.log");
    let text = format!("{STAMP}{TAIL}").replace("CARGO_EXIT=0\n", "");
    std::fs::write(&p, text).unwrap();
    let (code, out) = judge(&p, &[]);
    assert_eq!(code, 2, "an unfinished log must be REFUSED (exit 2), got {code}:\n{out}");
    assert!(out.contains("CARGO_EXIT"), "the refusal must name the missing line:\n{out}");
    assert!(!out.contains("RESULT          GREEN"), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}
