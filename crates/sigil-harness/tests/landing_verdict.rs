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

/// A BINARY THAT DIES WITH OTHERS STILL TO RUN, which is the shape the OOM killer actually
/// produces: under `--no-fail-fast` cargo carries on to the next target, so the victim is
/// followed by more launches rather than by the end of the log. That is a DIFFERENT branch
/// of the pairing from the one above, and it had no test: a mutation blanking the
/// mid-span emit left the suite green, because both earlier fixtures put the victim last
/// and the end-of-span emit caught them. This test is what makes the mid-span branch
/// load bearing.
#[test]
fn a_binary_that_dies_with_others_still_to_run_is_still_named() {
    let dir = scratch("mid-span-victim");
    let p = dir.join("mid-span.log");
    std::fs::write(
        &p,
        format!(
            "{STAMP}\
test b_gate ... ok
test c_gate ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/oom_victim.rs (/fixture/.target-land/release/deps/oom_victim-fedcba9876543210)

running 40 tests
test victim_one ... ok

     Running tests/after_victim.rs (/fixture/.target-land/release/deps/after_victim-00112233445566)

running 1 test
test after_one ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
CARGO_EXIT=0
# finished (UTC) 2026-09-07T00:01:00Z
"
        ),
    )
    .unwrap();

    let (code, text) = judge(&p, &[]);
    assert_eq!(
        code, 1,
        "a binary that died with another still to run must fail the run, got {code}:\n{text}"
    );
    assert!(
        text.contains("binaries        3 launched, 2 reported"),
        "three launches and two reports must both be shown:\n{text}"
    );
    assert!(
        text.contains("oom_victim"),
        "the MID-SPAN victim must be named. If this is silent the pairing only reports a \
         victim that happens to be the last target in the log:\n{text}"
    );
    assert!(
        !text.contains("after_victim"),
        "the target that ran AFTER the victim reported and must not be named:\n{text}"
    );
    assert!(
        !text.contains("gates.rs"),
        "the target that ran BEFORE the victim reported and must not be named:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ----------------------------------------------------------------------------------------
// THE TEST-TARGET CENSUS, the one population in the verdict that does not come out of the
// log.
//
// The launch/report pairing above catches a binary that STARTED and went quiet. It is
// structurally blind to a target that STOPPED BEING BUILT: such a target neither launches
// nor reports, so it is absent from both of the log's own populations, and a check that
// compared them would be asserting a set against itself. `scripts/test_target_census.py`
// reads the workspace manifests through `cargo metadata --no-deps`, which builds nothing
// and cannot be affected by what the run did, and the wrapper stamps the figure into the
// log so `--verdict-only` judges the tree the run was made from.
// ----------------------------------------------------------------------------------------

/// A census stamp, in the shape `run_landing` writes it.
fn with_census(census: &str) -> String {
    format!("{STAMP}{TAIL}").replace(
        "# baseline       3\n",
        &format!("# baseline       3\n# test targets   {census}\n"),
    )
}

/// A CENSUS SHORTFALL FAILS THE RUN, with the log internally consistent. Every binary that
/// launched also reported, every test passed, cargo exited 0: nothing inside the log is
/// wrong. It describes fewer targets than the manifests do, and only a figure from outside
/// the log can say so.
#[test]
fn a_census_shortfall_fails_a_run_whose_log_agrees_with_itself() {
    let dir = scratch("census-short");

    // The control: the same log with a census that matches its single launch.
    let ok = dir.join("census-ok.log");
    std::fs::write(&ok, with_census("1 expected launches (1 runnable + 0 doctest, 0 excluded for required-features; cargo metadata --no-deps)")).unwrap();
    let (code, text) = judge(&ok, &[]);
    assert_eq!(code, 0, "a matching census must be GREEN, got {code}:\n{text}");
    assert!(text.contains("all launched"), "a matching census must say so:\n{text}");

    // The same log, one target the manifests describe that never launched.
    let short = dir.join("census-short.log");
    std::fs::write(&short, with_census("2 expected launches (2 runnable + 0 doctest, 0 excluded for required-features; cargo metadata --no-deps)")).unwrap();
    let (code, text) = judge(&short, &[]);
    assert_eq!(
        code, 1,
        "a target that stopped being built must fail the run. The pairing cannot see it, \
         so if this is 0 nothing can. Got {code}:\n{text}"
    );
    assert!(
        text.contains("1 target(s) the manifests"),
        "the verdict must say how many targets are missing:\n{text}"
    );
    assert!(
        text.contains("TEST-TARGET CENSUS does not match"),
        "the RESULT line must name the census as the reason, with every other bar clean:\n{text}"
    );
    assert!(!text.contains("RESULT          GREEN"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// MORE LAUNCHES THAN THE MANIFESTS DESCRIBE also fails, and is named as its own thing. It
/// is not a run that lost a target, it is a derivation that disagrees with cargo, and
/// until that is settled neither number is a population.
#[test]
fn a_census_that_undercounts_cargo_is_a_finding_of_its_own() {
    let dir = scratch("census-over");
    let p = dir.join("census-over.log");
    std::fs::write(&p, with_census("0 expected launches (0 runnable + 0 doctest, 0 excluded for required-features; cargo metadata --no-deps)")).unwrap();
    let (code, text) = judge(&p, &[]);
    assert_eq!(code, 1, "a census below the launch count must fail, got {code}:\n{text}");
    assert!(
        text.contains("MORE launch(es) than the"),
        "the two directions must be told apart; they have different causes:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A CENSUS THAT COULD NOT BE TAKEN IS NOT A SATISFIED CROSS-CHECK. The stamp records the
/// failure by name rather than a number, and the verdict refuses rather than reading the
/// absence of a figure as agreement.
#[test]
fn a_census_that_could_not_be_taken_fails_rather_than_passing_quietly() {
    let dir = scratch("census-unmeasured");
    let p = dir.join("census-unmeasured.log");
    std::fs::write(
        &p,
        with_census("COULD NOT MEASURE (scripts/test_target_census.py exited 2: cargo metadata did not return JSON)"),
    )
    .unwrap();
    let (code, text) = judge(&p, &[]);
    assert_eq!(code, 1, "an untaken census must fail the run, got {code}:\n{text}");
    assert!(text.contains("COULD NOT MEASURE"), "{text}");
    assert!(
        text.contains("absent cross-check, not a satisfied one"),
        "the verdict must say which of the two it is:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A LOG PREDATING THE STAMP IS NOT FAILED FOR IT. Every landing log written before this
/// parcel carries no census line, and a rule that reddened all of them would be one people
/// learn to route around. The verdict says NOT STATED in words instead, so a reader can
/// tell a log that was checked from one that could not be.
#[test]
fn a_log_predating_the_census_stamp_says_so_rather_than_failing() {
    let dir = scratch("census-absent");
    let clean = fixture(&dir, "clean.log", "");
    let (code, text) = judge(&clean, &[]);
    assert_eq!(code, 0, "an old log must not be failed for a stamp it could not carry, got {code}:\n{text}");
    assert!(
        text.contains("NOT STATED, this log predates the census stamp"),
        "the verdict must distinguish a missing stamp from a satisfied one:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A --scoped RUN REPORTS THE CENSUS WITHOUT CHECKING IT. A scoped run launches a subset
/// on purpose, so a shortfall is the flag working rather than a finding, and failing on it
/// would make the census a reason to stop using `--scoped`.
#[test]
fn a_scoped_run_reports_the_census_rather_than_gating_on_it() {
    let dir = scratch("census-scoped");
    let p = dir.join("census-scoped.log");
    let body = with_census("99 expected launches (99 runnable + 0 doctest, 0 excluded for required-features; cargo metadata --no-deps)")
        .replace(
            "# scoped         no (full workspace)",
            "# scoped         YES, this is a PARTIAL run, not a landing",
        );
    std::fs::write(&p, body).unwrap();
    let (code, text) = judge(&p, &[]);
    assert_eq!(code, 0, "a scoped run must not fail on a census shortfall, got {code}:\n{text}");
    assert!(
        text.contains("REPORTED, NOT CHECKED"),
        "a scoped run must still SHOW the census, or the reader cannot tell it was skipped \
         from it being satisfied:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// THE DERIVATION ITSELF, against a workspace whose answer is known by construction rather
/// than copied from a nearby pin. One lib, two integration tests, one bin, one example and
/// one bench: cargo launches a binary for the lib's unit tests, one per integration test,
/// one for the bin's unit tests, one for the bench, and a doctest binary for the lib. The
/// example is NOT launched, because `cargo test` runs an example only when its `test` flag
/// is true and it is false by default.
///
/// THE ANSWER IS NOT ASSERTED FROM THE CARGO BOOK. Run against real cargo on this exact
/// fixture, `cargo test` prints five `Running` lines (lib, bin, tests/one, tests/two,
/// benches/cf-bench) and one `Doc-tests`, which is what the numbers below are.
#[test]
fn the_census_counts_what_cargo_would_launch() {
    let dir = scratch("census-derivation");
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::create_dir_all(dir.join("examples")).unwrap();
    std::fs::create_dir_all(dir.join("benches")).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"census-fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\
         [workspace]\n\
         [lib]\npath = \"src/lib.rs\"\n\
         [[bin]]\nname = \"cf-bin\"\npath = \"src/main.rs\"\n\
         [[example]]\nname = \"cf-example\"\npath = \"examples/cf-example.rs\"\n\
         [[bench]]\nname = \"cf-bench\"\npath = \"benches/cf-bench.rs\"\ntest = true\nharness = true\n",
    )
    .unwrap();
    std::fs::write(dir.join("src/lib.rs"), "").unwrap();
    std::fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(dir.join("tests/one.rs"), "").unwrap();
    std::fs::write(dir.join("tests/two.rs"), "").unwrap();
    std::fs::write(dir.join("examples/cf-example.rs"), "fn main() {}\n").unwrap();
    std::fs::write(dir.join("benches/cf-bench.rs"), "fn main() {}\n").unwrap();

    let out = Command::new("python3")
        .arg(repo_root().join("scripts/test_target_census.py"))
        .arg(dir.join("Cargo.toml"))
        .output()
        .expect("COULD NOT MEASURE: python3 could not run the census");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "the census must succeed on a valid workspace:\n{text}");
    // lib + bin + tests/one + tests/two + the bench that opted in = 5.
    assert!(
        text.contains("runnable 5"),
        "expected 5 runnable targets (lib, bin, two integration tests, one opted-in bench) \
         and the example NOT counted, because `cargo test` does not run examples unless \
         they set `test = true`:\n{text}"
    );
    assert!(text.contains("doctest 1"), "one lib target, one doctest binary:\n{text}");
    assert!(text.contains("excluded-required-features 0"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A TARGET WITH `required-features` IS EXCLUDED AND SAID SO, never silently counted.
/// Whether cargo builds one depends on the feature set the run selected, which the
/// manifests alone do not settle; counting it would make the census red on correct work
/// the first time somebody adds one, and dropping it silently would make the figure a
/// quiet undercount. It is reported as its own number so a reader knows the census is a
/// lower bound by that much.
#[test]
fn a_required_features_target_is_excluded_by_name_not_silently() {
    let dir = scratch("census-required-features");
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"rf-fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\
         [workspace]\n\
         [features]\nextra = []\n\
         [lib]\npath = \"src/lib.rs\"\ndoctest = false\n\
         [[test]]\nname = \"plain\"\npath = \"tests/plain.rs\"\n\
         [[test]]\nname = \"gated\"\npath = \"tests/gated.rs\"\nrequired-features = [\"extra\"]\n",
    )
    .unwrap();
    std::fs::write(dir.join("src/lib.rs"), "").unwrap();
    std::fs::write(dir.join("tests/plain.rs"), "").unwrap();
    std::fs::write(dir.join("tests/gated.rs"), "").unwrap();

    let out = Command::new("python3")
        .arg(repo_root().join("scripts/test_target_census.py"))
        .arg(dir.join("Cargo.toml"))
        .output()
        .expect("COULD NOT MEASURE: python3 could not run the census");
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        text.contains("runnable 2"),
        "the lib and the plain test are counted, the gated one is not:\n{text}"
    );
    assert!(
        text.contains("doctest 0"),
        "`doctest = false` must be honoured, or every such crate reads as a lost launch:\n{text}"
    );
    assert!(
        text.contains("excluded-required-features 1"),
        "the excluded target must be REPORTED, so the census is known to be a lower bound \
         rather than looking exact:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
