//! THE LEDGER GATE, PROVEN BY RUNNING IT OVER BEDS WHOSE RIGHT ANSWER IS KNOWN.
//!
//! `tools/decisions_reader_audit.py` was committed, correct, and had NO CALLER anywhere
//! in the tree. It answers "which lines of `docs/decisions.jsonl` can the owner's console
//! actually render", and while nothing ran it that answer went from 3 of 16 (2026-08-30)
//! to 12 of 36 (2026-09-09): the file doubled and the unrenderable share went 19% to 33%
//! with nobody noticing. `scripts/ledger_gate.py` is the caller, `scripts/landing-run.sh`
//! runs it, and this file is what stops the caller from becoming decorative in turn.
//!
//! THE THIRD STATE, which is the one worth testing for. A check has three states, not
//! two: it can not exist, it can exist and be unwired, and it can RUN, REPORT, AND NOT
//! BLOCK. The third looks exactly like enforcement in a log and enforces nothing. So the
//! tests split along that seam:
//!
//!   * HERE: does the gate compute the right verdict, and can it be silently vacuous?
//!     Every "clean" answer below is measured against a bed that could have answered
//!     otherwise, and [`a_vacuous_audit_is_unmeasurable_not_clean`] is the direct
//!     instrument: an audit stubbed to return zero results must make this gate exit 2,
//!     never print "0 rejected".
//!   * `landing_verdict.rs`: does a red from this gate reach the landing wrapper's EXIT
//!     CODE. That is a different failure and one control cannot cover both.
//!
//! WHY BEDS RATHER THAN THE REAL LEDGER FOR THE RED CASES. `docs/decisions.jsonl` is
//! append-only by contract and carries a ratified non-repair (d-14, d-15, d-16), so a
//! test may not mutate it to go red. Each bed here is built from scratch, holds the
//! smallest ledger that exhibits one property, and is deleted after.
//!
//! Scratch is on disk, never under the system temp directory (tmpfs here), derived from
//! this binary's own location inside the target directory.
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the harness crate sits two levels under the repo root")
        .to_path_buf()
}

fn gate() -> PathBuf {
    let p = repo_root().join("scripts/ledger_gate.py");
    assert!(
        p.is_file(),
        "COULD NOT MEASURE: the ledger gate is not at {}, nothing below would be judging \
         the real script",
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
        .join("ledger-gate-beds")
        .join(format!("{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("mkdir {}: {e}", dir.display()));
    dir
}

/// One decision line the console renders: every field `parseEntry` requires is present.
fn renderable(id: &str) -> String {
    format!(
        r#"{{"id":"{id}","at":"2026-09-09T00:00:00Z","question":"q","options":[{{"key":"a","name":"A","what":"w","costs":"c"}},{{"key":"b","name":"B","what":"w","costs":"c"}}],"recommend":{{"key":"a","because":"r"}}}}"#
    )
}

/// One decision line the console DROPS: valid JSON, no `recommend`. This is the shape the
/// ratchet counts, and it is deliberately not also broken JSON, so the two assertions
/// cannot be confused for one another.
fn unrenderable(id: &str) -> String {
    format!(
        r#"{{"id":"{id}","at":"2026-09-09T00:00:00Z","question":"q","options":[{{"key":"a","name":"A","what":"w","costs":"c"}},{{"key":"b","name":"B","what":"w","costs":"c"}}]}}"#
    )
}

/// A bed: `<dir>/docs/decisions.jsonl` with the given lines, and a `tools/` holding the
/// REAL audit tool, so what the gate measures there is the real predicate.
fn bed(dir: &Path, lines: &[String]) -> PathBuf {
    bed_with_trailing(dir, lines, true)
}

fn bed_with_trailing(dir: &Path, lines: &[String], trailing_newline: bool) -> PathBuf {
    let docs = dir.join("docs");
    let tools = dir.join("tools");
    std::fs::create_dir_all(&docs).expect("mkdir docs");
    std::fs::create_dir_all(&tools).expect("mkdir tools");
    std::fs::copy(
        repo_root().join("tools/decisions_reader_audit.py"),
        tools.join("decisions_reader_audit.py"),
    )
    .expect("COULD NOT MEASURE: the real audit tool could not be copied into the bed");
    let mut text = lines.join("\n");
    if trailing_newline {
        text.push('\n');
    }
    std::fs::write(docs.join("decisions.jsonl"), text).expect("write ledger");
    docs
}

/// Run the gate over a bed. Returns (exit code, merged output).
fn run(repo: &Path, docs: &Path, extra: &[&str]) -> (i32, String) {
    let out = Command::new("python3")
        .arg(gate())
        .arg("--repo")
        .arg(repo)
        .arg("--docs")
        .arg(docs)
        .args(extra)
        .current_dir(repo_root())
        .output()
        .expect("COULD NOT MEASURE: python3 could not run the gate");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    let code = out
        .status
        .code()
        .unwrap_or_else(|| panic!("the gate was killed by a signal:\n{text}"));
    (code, text)
}

// ----------------------------------------------------------------------------------------
// The ratchet, in both directions, each with the control that proves the bed is the cause.
// ----------------------------------------------------------------------------------------

/// GROWTH PAST THE PIN IS RED, and the SAME bed under a pin that admits it is GREEN. The
/// pair is the point: one run alone cannot tell a working ratchet from a gate that reds on
/// any ledger at all.
#[test]
fn the_ratchet_fails_on_growth_and_only_on_growth() {
    let dir = scratch("ratchet");
    let docs = bed(
        &dir,
        &[renderable("d-1"), renderable("d-2"), unrenderable("d-3")],
    );

    let (code, text) = run(&dir, &docs, &["--pin", "0"]);
    assert_eq!(
        code, 1,
        "one unrenderable line against a pin of 0 must be RED, got exit {code}:\n{text}"
    );
    assert!(
        text.contains("RATCHET, FAILED"),
        "the ratchet must name itself as the failing assertion:\n{text}"
    );
    assert!(
        text.contains("THE RATCHET MOVED BACKWARDS"),
        "the failure must say which way it moved:\n{text}"
    );
    assert!(
        text.contains("d-3"),
        "the failure must name the rejecting id, that is the whole use of it:\n{text}"
    );
    assert!(
        text.contains("do not raise PIN"),
        "the failure must refuse the remedy it would otherwise invite:\n{text}"
    );

    // THE CONTROL. Same bed, same lines, pin raised by one: green. So the red above came
    // from the distance to the pin and not from anything else about this ledger.
    let (code, text) = run(&dir, &docs, &["--pin", "1"]);
    assert_eq!(
        code, 0,
        "the SAME bed at a pin that admits it must be GREEN, got exit {code}:\n{text}"
    );
    assert!(text.contains("RATCHET, ok"), "{text}");
    assert!(
        text.contains("3 line(s) / 2 render / 1 rejected"),
        "the green run must still state the measured figure:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A count BELOW the pin is green and says the pin can tighten. Ground won is only held if
/// somebody is told to hold it.
#[test]
fn a_count_below_the_pin_is_green_and_asks_for_a_tighter_pin() {
    let dir = scratch("tighten");
    let docs = bed(&dir, &[renderable("d-1"), renderable("d-2")]);
    let (code, text) = run(&dir, &docs, &["--pin", "3"]);
    assert_eq!(code, 0, "under the pin must be green, got {code}:\n{text}");
    assert!(
        text.contains("THE RATCHET CAN TIGHTEN"),
        "a green run below the pin must ask for the pin to come down:\n{text}"
    );
    assert!(text.contains("distance -3"), "the distance must be signed:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

// ----------------------------------------------------------------------------------------
// Assertion (1), HARD: JSON well-formedness and the trailing newline.
// ----------------------------------------------------------------------------------------

/// THE INCIDENT THIS GATE WAS ASKED FOR. An append onto a file with no trailing newline
/// concatenated two records into one unparseable line and destroyed the previous session's
/// record silently. Both halves of that are red here, and neither depends on the ratchet:
/// the pin is set wide open so the only thing that can fail is assertion (1).
#[test]
fn an_unparseable_line_is_hard_red_whatever_the_ratchet_says() {
    let dir = scratch("badjson");
    let docs = bed(
        &dir,
        &[
            renderable("d-1"),
            // The exact damage shape: two objects concatenated onto one line.
            format!("{}{}", renderable("d-2"), renderable("d-3")),
        ],
    );
    let (code, text) = run(&dir, &docs, &["--pin", "99"]);
    assert_eq!(
        code, 1,
        "an unparseable line must be RED even with the ratchet wide open, got {code}:\n{text}"
    );
    assert!(text.contains("HARD, FAILED"), "{text}");
    assert!(
        text.contains("unparseable  decisions.jsonl:2"),
        "the failure must name the file and the line number:\n{text}"
    );

    // THE CONTROL. The same two records on separate lines, same pin: green. The red came
    // from the concatenation and not from the records.
    let dir2 = scratch("badjson-control");
    let docs2 = bed(
        &dir2,
        &[renderable("d-1"), renderable("d-2"), renderable("d-3")],
    );
    let (code, text) = run(&dir2, &docs2, &["--pin", "99"]);
    assert_eq!(code, 0, "the split control must be green, got {code}:\n{text}");
    assert!(text.contains("0 unparseable"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}

/// The same defect one step EARLIER: a file that ends without a newline is a file whose
/// next append destroys its last record. Red before the damage, not after.
#[test]
fn a_missing_trailing_newline_is_hard_red() {
    let dir = scratch("nonewline");
    let docs = bed_with_trailing(&dir, &[renderable("d-1"), renderable("d-2")], false);
    let (code, text) = run(&dir, &docs, &["--pin", "99"]);
    assert_eq!(
        code, 1,
        "a ledger with no trailing newline must be RED, got {code}:\n{text}"
    );
    assert!(
        text.contains("no trailing newline  decisions.jsonl"),
        "the failure must name the file:\n{text}"
    );
    assert!(
        text.contains("printf"),
        "the failure must state the remedy, which adds a byte and not a record:\n{text}"
    );

    // THE CONTROL: byte-identical bed plus one newline, green.
    let dir2 = scratch("nonewline-control");
    let docs2 = bed_with_trailing(&dir2, &[renderable("d-1"), renderable("d-2")], true);
    let (code, _) = run(&dir2, &docs2, &["--pin", "99"]);
    assert_eq!(code, 0, "the same bed with a trailing newline must be green");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}

// ----------------------------------------------------------------------------------------
// Assertion (3), REPORTING ONLY. This one is a test that it does NOT fail.
// ----------------------------------------------------------------------------------------

/// ID UNIQUENESS IS REPORTED AND IS NOT A GATE, DELIBERATELY. `d-18` appears three times
/// and `d-25` twice in the real ledger today. Rule 8e makes re-iding the later line the
/// single sanctioned in-place edit to an append-only file, and that re-id is a separate
/// decision; oracle's `tools/land.sh` G2b refuses every landing on exactly this shape, so
/// wiring it here before the re-id is made would hand this repo an unlandable master.
///
/// A test that the check does NOT fail is unusual and is the point: this is the one line
/// in the gate that could quietly become a bar, and if someone wires it, this goes red and
/// says why.
#[test]
fn repeated_ids_are_reported_and_do_not_fail_the_gate() {
    let dir = scratch("dupids");
    let docs = bed(
        &dir,
        &[renderable("d-18"), renderable("d-18"), renderable("d-18")],
    );
    let (code, text) = run(&dir, &docs, &["--pin", "0"]);
    assert_eq!(
        code, 0,
        "repeated ids must NOT fail the gate; wiring that before the rule-8e re-id lands \
         makes master unlandable. Got exit {code}:\n{text}"
    );
    assert!(
        text.contains("d-18 x3"),
        "the repetition must be REPORTED even though it is not a gate:\n{text}"
    );
    assert!(
        text.contains("REPORTING ONLY, NOT A GATE"),
        "the line must say which it is; a bar reported beside a green verdict that does \
         not say so is how one gets landed over:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ----------------------------------------------------------------------------------------
// UNMEASURABLE IS NOT GREEN. These are the vacuity controls.
// ----------------------------------------------------------------------------------------

/// THE DIRECT ANSWER TO "how would this fail if the audit returned nothing".
///
/// The bed's `tools/decisions_reader_audit.py` is a STUB with the right module surface and
/// an `audit()` that returns an empty list. Against a ledger with three lines that is a
/// measurement of nothing, and the honest verdict is exit 2. Without the cross-check in
/// the gate it would print "0 rejected" over a three-line ledger and read as the cleanest
/// ledger this repo has ever had, under a pin of 0.
#[test]
fn a_vacuous_audit_is_unmeasurable_not_clean() {
    let dir = scratch("vacuous");
    let docs = bed(
        &dir,
        &[renderable("d-1"), renderable("d-2"), unrenderable("d-3")],
    );

    // THE CONTROL FIRST, with the REAL tool in place: the same bed at pin 0 is RED and
    // names d-3. So the instrument can return non-empty over this bed, which is what makes
    // the emptiness below a finding rather than a shrug.
    let (code, text) = run(&dir, &docs, &["--pin", "0"]);
    assert_eq!(code, 1, "the control must be RED with the real tool:\n{text}");
    assert!(text.contains("d-3"), "{text}");

    std::fs::write(
        dir.join("tools/decisions_reader_audit.py"),
        "class Unmeasurable(Exception):\n    pass\n\n\ndef audit(path):\n    return []\n",
    )
    .expect("write the stub audit");

    let (code, text) = run(&dir, &docs, &["--pin", "0"]);
    assert_eq!(
        code, 2,
        "an audit returning zero results over a three-line ledger must be UNMEASURABLE \
         (exit 2), not a clean 0-rejected. Got exit {code}:\n{text}"
    );
    assert!(
        text.contains("UNMEASURABLE"),
        "the verdict must say so in words, not only in an exit code:\n{text}"
    );
    assert!(
        text.contains("returned 0 result(s)") && text.contains("3 content line(s)"),
        "the refusal must state BOTH counts, so a reader can see the disagreement:\n{text}"
    );
    assert!(
        !text.contains("RATCHET, ok"),
        "a vacuous audit must never render as a passing ratchet:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A missing audit tool is unmeasurable, not clean. This is the state the repo was
/// effectively in for weeks: the question unasked reads identically to the question
/// answered well, unless something refuses.
#[test]
fn a_missing_audit_tool_is_unmeasurable_not_clean() {
    let dir = scratch("notool");
    let docs = bed(&dir, &[renderable("d-1")]);
    std::fs::remove_file(dir.join("tools/decisions_reader_audit.py")).expect("remove the tool");
    let (code, text) = run(&dir, &docs, &["--pin", "0"]);
    assert_eq!(code, 2, "a missing audit tool must be exit 2, got {code}:\n{text}");
    assert!(text.contains("UNMEASURABLE"), "{text}");
    assert!(
        text.contains("decisions_reader_audit.py"),
        "the refusal must name what is missing:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// AN EMPTINESS IS NOT A FINDING WITHOUT AN INSTRUMENT THAT COULD HAVE RETURNED NON-EMPTY.
/// A docs directory holding no `*.jsonl` means the path, the glob or the tree is wrong; it
/// does not mean every ledger is clean, and a gate that checked nothing would pass
/// trivially forever without anyone editing a line of it.
#[test]
fn a_docs_directory_with_no_ledgers_is_refused() {
    let dir = scratch("noledgers");
    let docs = dir.join("docs");
    std::fs::create_dir_all(&docs).expect("mkdir");
    let (code, text) = run(&dir, &docs, &[]);
    assert_eq!(
        code, 2,
        "zero ledgers must be REFUSED, not passed trivially. Got exit {code}:\n{text}"
    );
    assert!(text.contains("no *.jsonl files"), "{text}");

    // And a directory that does not exist at all, same answer.
    let (code, text) = run(&dir, &dir.join("nope"), &[]);
    assert_eq!(code, 2, "a missing docs directory must be refused:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A ledger with no content lines is refused for the same reason: nothing to state.
#[test]
fn an_empty_ledger_is_refused_not_reported_as_clean() {
    let dir = scratch("emptyledger");
    let docs = bed(&dir, &[]);
    let (code, text) = run(&dir, &docs, &["--pin", "0"]);
    assert_eq!(code, 2, "an empty ledger must be refused, got {code}:\n{text}");
    assert!(text.contains("no content lines"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A GATE MUST NOT DIRTY THE TREE IT MEASURES. Importing the audit tool by path writes
/// `tools/__pycache__/` beside it, that directory is not in `.gitignore`, and a landing
/// run stamps its log with whether the checkout is DIRTY. A gate whose own execution can
/// flip that stamp changes the answer by being run. Measured: the first hand run left it
/// behind, which is why `sys.dont_write_bytecode` is set before the import.
#[test]
fn running_the_gate_leaves_no_bytecode_behind() {
    let dir = scratch("nopycache");
    let docs = bed(&dir, &[renderable("d-1"), renderable("d-2")]);
    let (code, text) = run(&dir, &docs, &["--pin", "0"]);
    assert_eq!(code, 0, "the bed must be green so the only finding here is the litter:\n{text}");
    let cache = dir.join("tools/__pycache__");
    assert!(
        !cache.exists(),
        "the gate wrote {} into the tree it measured. That directory is untracked and \
         not gitignored, so running the gate would flip a landing log's DIRTY stamp.",
        cache.display()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ----------------------------------------------------------------------------------------
// The real tree. This is the assertion that actually holds the line day to day.
// ----------------------------------------------------------------------------------------

/// THE PIN HOLDS AGAINST THIS REPO'S OWN LEDGERS, with no `--pin` override, so what runs
/// here is the constant a landing enforces.
///
/// This goes red when a decision entry the owner cannot see on his console is added. THE
/// REMEDY IS THE ENTRY, NOT THE PIN: the ledger is append-only, so raising `PIN` accepts
/// an unreadable decision permanently.
#[test]
fn the_repo_ledgers_hold_the_pin_today() {
    let root = repo_root();
    let (code, text) = run(&root, &root.join("docs"), &[]);
    assert!(
        text.contains("built-in constant PIN"),
        "COULD NOT MEASURE: this run did not use the landing pin:\n{text}"
    );
    assert!(
        text.contains("RATCHET"),
        "COULD NOT MEASURE: the ratchet did not report at all:\n{text}"
    );
    assert_eq!(
        code, 0,
        "the ledger gate is RED against this repo's own docs/. If the ratchet moved, the \
         fix is the entry that moved it, NOT the PIN constant. Full report:\n{text}"
    );
}
