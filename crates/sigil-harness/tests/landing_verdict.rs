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

##### LEDGER SPAN, python3 /fixture/sigil/scripts/ledger_gate.py
LEDGER: json      3 file(s), 388 line(s), 0 unparseable, 0 missing a trailing newline  -- HARD, ok
LEDGER: pin       12 (built-in constant PIN in scripts/ledger_gate.py)
LEDGER: render    decisions.jsonl  36 line(s) / 24 render / 12 rejected   pin 12, distance +0  -- RATCHET, ok
LEDGER: result    ok
LEDGER_EXIT=0
##### LEDGER SPAN ENDS
##### CLIPPY SPAN, cargo clippy --release --workspace --all-targets -- -D warnings
    Finished `release` profile [optimized] target(s) in 1.00s
CLIPPY_EXIT=0
##### CLIPPY SPAN ENDS
##### TEST SPAN, cargo test --release --no-fail-fast --workspace -- --nocapture
     Running tests/gates.rs (/fixture/.target-land/release/deps/gates-0123456789abcdef)

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

// ----------------------------------------------------------------------------------------
// THE LEDGER GATE'S VERDICT REACHES THE EXIT CODE.
//
// THE THIRD STATE. A check has three states, not two: absent, present-but-unwired, and
// RUNS-REPORTS-AND-DOES-NOT-BLOCK. The third is the worst of them, because its output
// looks like enforcement in a log while enforcing nothing, and it is invisible to every
// test that only asks whether the check ran.
//
// `landing-run.sh` is a collect-then-decide gate: the `(9) THE LEDGER GATE` block runs the
// gate and writes `LEDGER_EXIT=` WITHOUT ABORTING, and `LEDGER_RC != 0` in the `RESULT
// FAILED` condition three hundred lines later is what turns that into a refusal. That is
// the same split as oracle's tools/land.sh, whose `fail()` at :260 merely appends to an
// array while `finish_red` at :663 is what exits 1 before the push -- a correct gate that
// reads as decorative to anyone who stops at the first half. READING EITHER HALF IS NOT
// PROOF. These tests make the gate red on purpose and observe the wrapper REFUSE.
// ----------------------------------------------------------------------------------------

/// A RED LEDGER GATE MAKES THE LANDING REFUSE, with every test green and the lint bar
/// clean. The clean twin exits 0, which is the control proving the red comes from
/// `LEDGER_EXIT` and not from the fixture's shape.
#[test]
fn a_red_ledger_gate_fails_the_landing_verdict() {
    let dir = scratch("ledger-red");

    // The control first: the stock fixture carries LEDGER_EXIT=0 and is GREEN.
    let clean = fixture(&dir, "clean.log", "");
    let (code, text) = judge(&clean, &[]);
    assert_eq!(code, 0, "the CONTROL log must be GREEN, got exit {code}:\n{text}");
    assert!(text.contains("RESULT          GREEN"), "{text}");
    assert!(text.contains("LEDGER_EXIT     0"), "the verdict must carry the ledger exit:\n{text}");

    // The same log with ONE character changed: the gate's exit code.
    let red = dir.join("ledger-red.log");
    let body = std::fs::read_to_string(&clean)
        .unwrap()
        .replace("LEDGER_EXIT=0", "LEDGER_EXIT=1")
        .replace("RATCHET, ok", "RATCHET, FAILED")
        .replace("LEDGER: result    ok", "LEDGER: result    FAILED");
    std::fs::write(&red, body).unwrap();

    let (code, text) = judge(&red, &[]);
    assert_eq!(
        code, 1,
        "a red ledger gate must make the landing exit 1. If this is 0, the gate RUNS AND \
         REPORTS AND DOES NOT BLOCK, which is worse than not running. Got {code}:\n{text}"
    );
    assert!(text.contains("RESULT          FAILED"), "the verdict line must say FAILED:\n{text}");
    assert!(
        text.contains("the LEDGER GATE is red"),
        "the verdict must name the ledger as the reason, not leave it to be inferred:\n{text}"
    );
    assert!(!text.contains("RESULT          GREEN"), "a red ledger printed GREEN:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// UNMEASURABLE (exit 2 from the gate) IS ALSO A RED RUN, and it is named as its own
/// thing. "The ledger question is unanswered" and "the ledger is clean" are different
/// facts and only one of them is a landing.
#[test]
fn an_unmeasurable_ledger_gate_fails_the_landing_verdict() {
    let dir = scratch("ledger-unmeasurable");
    let clean = fixture(&dir, "clean.log", "");
    let p = dir.join("unmeasurable.log");
    let body = std::fs::read_to_string(&clean)
        .unwrap()
        .replace("LEDGER_EXIT=0", "LEDGER_EXIT=2");
    std::fs::write(&p, body).unwrap();

    let (code, text) = judge(&p, &[]);
    assert_eq!(code, 1, "an unmeasurable ledger gate must exit 1, got {code}:\n{text}");
    assert!(
        text.contains("could not measure"),
        "the verdict must distinguish unmeasurable from red:\n{text}"
    );
    assert!(text.contains("UNMEASURABLE"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A GATE THAT PRINTED NOTHING IS NOT A GATE THAT FOUND NOTHING. `LEDGER_EXIT=0` with no
/// report above it is a run whose measurement is missing, and it fails rather than
/// warning: an emptiness is not a finding without an instrument that could have returned
/// non-empty, and this log demonstrably has none.
#[test]
fn a_silent_ledger_gate_is_not_a_clean_one() {
    let dir = scratch("ledger-silent");
    let clean = fixture(&dir, "clean.log", "");
    let p = dir.join("silent.log");
    let body: String = std::fs::read_to_string(&clean)
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with("LEDGER: "))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&p, format!("{body}\n")).unwrap();

    let (code, text) = judge(&p, &[]);
    assert_eq!(
        code, 1,
        "an exit code with no measurement behind it must fail, got {code}:\n{text}"
    );
    assert!(text.contains("PRODUCED NO REPORT LINES"), "{text}");
    assert!(!text.contains("RESULT          GREEN"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The measured figure reaches the VERDICT BLOCK, not only the log body. The renderability
/// count is a TREND -- it went 3-of-16 to 12-of-36 unnoticed -- and a number visible only
/// to someone who opens the log is a number nobody reads.
#[test]
fn the_ledger_report_is_reprinted_in_the_verdict() {
    let dir = scratch("ledger-echo");
    let clean = fixture(&dir, "clean.log", "");
    let (code, text) = judge(&clean, &[]);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("LEDGER GATE (docs/*.jsonl), all of it:"), "{text}");
    assert!(
        text.contains("36 line(s) / 24 render / 12 rejected"),
        "the verdict must carry the measured figure:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A ledger line QUOTED by a test is not a measurement. The span-scoped parser must not
/// lift `LEDGER:` out of the test span, or a verdict could report a figure nothing
/// produced.
#[test]
fn a_ledger_prefix_quoted_in_the_test_span_is_not_a_measurement() {
    let dir = scratch("ledger-quote");
    let log = fixture(
        &dir,
        "quoted.log",
        "LEDGER: render    decisions.jsonl  99 line(s) / 0 render / 99 rejected\n",
    );
    let (code, text) = judge(&log, &[]);
    assert_eq!(code, 0, "a quoted line must not change the verdict:\n{text}");
    assert!(
        !text.contains("99 rejected"),
        "a line from the TEST span was lifted into the ledger report:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// UNMEASURABLE IS NOT GREEN. A log with no `LEDGER_EXIT=` line is a run in which the
/// ledger gate never executed; the wrapper refuses it (exit 2) rather than defaulting the
/// code to 0. This is the state every landing log in this repo was in before this parcel,
/// and defaulting it would have made the whole wiring optional in practice.
#[test]
fn a_log_without_a_ledger_exit_line_is_refused_not_judged() {
    let dir = scratch("no-ledger-exit");
    let p = dir.join("unwired.log");
    let text = format!("{STAMP}{TAIL}").replace("LEDGER_EXIT=0\n", "");
    std::fs::write(&p, text).unwrap();
    let (code, out) = judge(&p, &[]);
    assert_eq!(code, 2, "a log with no ledger exit must be REFUSED, got {code}:\n{out}");
    assert!(out.contains("LEDGER_EXIT"), "the refusal must name the missing line:\n{out}");
    assert!(!out.contains("RESULT          GREEN"), "{out}");
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

// ----------------------------------------------------------------------------------------
// COMPLETENESS: A BINARY THAT LAUNCHED AND NEVER REPORTED.
//
// The totals are sums over `test result:` lines and nothing said what that count should
// be. A test binary that dies mid-run prints no `test result:` line, so its tests leave
// `suites` and `passed` silently while every line in the verdict still reads complete.
// Measured on a real green landing log with one binary's report removed: 449 suites became
// 448 and 5077 passed became 5075, and the wrapper printed `RESULT GREEN` with
// `5063 baseline + 12 new = 5075 observed`. The baseline does not cover it, because the
// reconciliation fails only on a shortfall and every test added since the caller last
// moved that number is slack the loss hides inside.
//
// THE POPULATION AND THE REPORT COME FROM DIFFERENT PRODUCERS, which is what keeps this
// from being a set checked against itself: cargo (the parent) writes `Running` before each
// child starts, the child writes `test result:` when it finishes, and a child that dies
// cannot retract a line its parent already flushed.
// ----------------------------------------------------------------------------------------

/// A second test binary that launches AFTER the first one has reported, runs one test and
/// goes quiet. Spliced in by replacing the first binary's `finished in 0.01s` line ending,
/// so the text it replaces is put back and the victim follows it: the ORDER matters,
/// because the check pairs each launch with the next `test result:` and a victim spliced
/// before the first binary's report would make the FIRST binary the silent one.
const VICTIM: &str = "\
finished in 0.01s

     Running tests/oom_victim.rs (/fixture/.target-land/release/deps/oom_victim-fedcba9876543210)

running 40 tests
test victim_one ... ok
";

/// A launch line with no `test result:` after it FAILS THE RUN and the verdict NAMES the
/// binary. The control is the stock fixture, whose single launch does report, and which
/// stays GREEN: that is what proves the red comes from the missing report and not from the
/// fixture's shape.
#[test]
fn a_binary_that_launched_and_never_reported_fails_the_landing_verdict() {
    let dir = scratch("silent-binary");

    let clean = fixture(&dir, "clean.log", "");
    let (code, text) = judge(&clean, &[]);
    assert_eq!(code, 0, "the CONTROL log must be GREEN, got exit {code}:\n{text}");
    assert!(
        text.contains("binaries        1 launched, 1 reported"),
        "the control must report a matched launch/report count:\n{text}"
    );

    // A SECOND binary that starts and goes quiet, spliced in AFTER the first binary's
    // `test result:` line so the first one is complete and only the second is short.
    // Everything else about the log is untouched: cargo still exits 0, every test that
    // reported passed, the lint bar and the ledger gate are clean.
    let log = dir.join("silent.log");
    std::fs::write(&log, format!("{STAMP}{TAIL}").replace("finished in 0.01s\n", VICTIM)).unwrap();
    let (code, text) = judge(&log, &[]);
    assert_eq!(
        code, 1,
        "a binary that launched and never reported must exit 1. If this is 0, a run that \
         lost a whole test binary reads as a complete one. Got {code}:\n{text}"
    );
    assert!(text.contains("RESULT          FAILED"), "the verdict line must say FAILED:\n{text}");
    assert!(!text.contains("RESULT          GREEN"), "a run missing a binary printed GREEN:\n{text}");
    assert!(
        text.contains("BINARIES THAT LAUNCHED AND NEVER REPORTED (1)"),
        "the verdict must name the silent binaries, not just count them:\n{text}"
    );
    assert!(
        text.contains("oom_victim"),
        "the verdict must name WHICH binary went quiet, or the operator has to find it:\n{text}"
    );
    assert!(
        !text.contains("gates.rs"),
        "the completed binary must NOT be named: pairing each launch with the next report \
         is what makes the name right, and naming the wrong target sends an operator to \
         read a binary that finished:\n{text}"
    );
    assert!(
        text.contains("binaries        2 launched, 1 reported"),
        "both counts must be shown, so the shortfall is checkable by hand:\n{text}"
    );
    assert!(
        text.contains("launched and never"),
        "the RESULT line must name completeness as the reason, with every other bar clean:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The death shape cargo DOES flag, kept separate because the two are different facts.
/// Measured against real cargo (`--no-fail-fast`, a test binary that SIGKILLs itself):
/// cargo exits 101, prints `error: test failed` and a `signal: 9, SIGKILL` cause, and the
/// dead binary emits no `test result:` line and no `test ... FAILED` line. So the failing
/// list is EMPTY and the completeness check is the only thing in the verdict that can say
/// which target died.
#[test]
fn a_signal_killed_binary_is_named_even_though_no_test_line_says_failed() {
    let dir = scratch("sigkill");
    let clean = fixture(&dir, "clean.log", "");
    let body = std::fs::read_to_string(&clean)
        .unwrap()
        .replace("CARGO_EXIT=0", "CARGO_EXIT=101")
        .replace("finished in 0.01s\n", VICTIM)
        .replace(
            "test victim_one ... ok\n",
            "test victim_one ... ok\n\
             error: test failed, to rerun pass `--test oom_victim`\n\
             \n\
             Caused by:\n\
               process didn't exit successfully: `oom_victim-fedcba9876543210 --nocapture` (signal: 9, SIGKILL: kill)\n",
        );
    let p = dir.join("sigkill.log");
    std::fs::write(&p, body).unwrap();

    let (code, text) = judge(&p, &[]);
    assert_eq!(code, 1, "a signal-killed binary must fail the run, got {code}:\n{text}");
    assert!(
        !text.contains("FAILING TESTS"),
        "a SIGKILL leaves no `... FAILED` line, so the failing list must be empty here; if \
         this fired the fixture no longer models the measured shape:\n{text}"
    );
    assert!(
        text.contains("oom_victim"),
        "with no failing test name to print, the completeness check is the only thing that \
         can say which target died, and it must:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A DOCTEST TARGET COUNTS AS A LAUNCH. `Doc-tests <crate>` is cargo's launch line for the
/// doctest binary and it reports a `test result:` like any other; a check that matched only
/// `Running` would read all thirteen doctest targets in this workspace as silent.
#[test]
fn a_doctest_launch_that_reports_is_not_counted_as_silent() {
    let dir = scratch("doctest-launch");
    let log = fixture(
        &dir,
        "doctest.log",
        "test b_gate ... ok\n\
         test c_gate ... ok\n\
         test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n\
         \n\
            Doc-tests sigil-span\n\
         \n\
         running 0 tests\n",
    );
    // The fixture's TAIL supplies the doctest target's own `test result:` line.
    let (code, text) = judge(&log, &[]);
    assert_eq!(code, 0, "a doctest launch that reports must stay GREEN, got {code}:\n{text}");
    assert!(
        text.contains("binaries        2 launched, 2 reported"),
        "the doctest launch must be counted on both sides:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A `Running` line QUOTED by clippy is not a launch. The counter is scoped to the test
/// span for the same reason the skip counter is: clippy prints source lines verbatim, and
/// a launch counted out of the lint bar would make a complete run read as short by one.
#[test]
fn a_running_line_quoted_by_clippy_is_not_a_launch() {
    let dir = scratch("running-quote");
    let p = dir.join("running-quote.log");
    let text = format!("{STAMP}{TAIL}").replace(
        "    Finished `release`",
        "warning: unused\n  --> crates/x/tests/y.rs:1:1\n   |\n 1 |     eprintln!(\"     Running tests/z.rs\");\n    Finished `release`",
    );
    std::fs::write(&p, text).unwrap();
    let (code, out) = judge(&p, &[]);
    assert_eq!(code, 0, "a quoted launch line must not fail the run:\n{out}");
    assert!(
        out.contains("binaries        1 launched, 1 reported"),
        "a line from the CLIPPY span was counted as a launch:\n{out}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// UNMEASURABLE IS NOT GREEN, for the completeness check too. A log whose test span
/// reports suites but records no launch line at all is a log this check cannot measure,
/// and rendering that as a satisfied population would be the exact defect the check
/// exists to close. Every landing log written before this parcel has that shape.
#[test]
fn a_log_with_no_launch_lines_is_not_read_as_complete() {
    let dir = scratch("no-launches");
    let p = dir.join("no-launches.log");
    let text = format!("{STAMP}{TAIL}")
        .lines()
        .filter(|l| !l.trim_start().starts_with("Running tests/gates.rs"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&p, format!("{text}\n")).unwrap();

    let (code, out) = judge(&p, &[]);
    assert_eq!(
        code, 1,
        "a log with suites but no launch record must not be read as a complete run, got \
         {code}:\n{out}"
    );
    assert!(out.contains("COULD NOT MEASURE"), "the verdict must say so in those words:\n{out}");
    assert!(!out.contains("RESULT          GREEN"), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}
