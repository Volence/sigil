//! THE NIGHTLY SOURCE-GATE LANE'S CLASSIFICATION, ASKED BY THE WORKSPACE SUITE.
//!
//! `scripts/nightly_source_gates.sh` refuses to run — the entire nightly backstop dark,
//! reporting nothing — when a test file that reads the engine reference tree lands in
//! none of its three buckets. That refusal is correct and this gate does not soften it.
//! What it changes is WHO FINDS OUT. Nothing in `cargo test` saw that lane, so the only
//! thing that ever asked the question was the 05:17 timer, and the answer arrived as a
//! critical desktop popup on the owner's machine — four files sat unclassified across
//! several nights, the lane produced no coverage the whole time, and no landing run was
//! any the wiser.
//!
//! This gate asks the same question at landing time, of the checkout being landed, by
//! invoking the script's own `--audit` flag. ONE definition of the rule, two callers: a
//! second implementation here would be a second thing to keep in step, which is the
//! defect class this lane has already been bitten by twice (a retyped skip marker, a
//! retyped gate count).
//!
//! `--audit` is read-only. It creates no worktree, builds nothing, and never reaches the
//! script's notification path, so running it cannot page anybody.
//!
//! NAMING, AND WHY THIS FILE IS CAREFUL ABOUT IT. The lane selects the files it
//! classifies by matching identifier spellings — the environment variable that names the
//! tree, the harness guards that open it, the command-line flag. A file that merely
//! DISCUSSES those identifiers is selected just the same, which is exactly the
//! false-positive shape the third bucket exists to absorb. This file describes them
//! without writing them, so it stays out of a population it is meant to be judging.

use std::process::Command;

/// The repository root, from the crate root baked in at compile time.
fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the harness crate sits two levels under the repo root")
        .to_path_buf()
}

fn audit() -> (bool, String) {
    let root = repo_root();
    let script = root.join("scripts/nightly_source_gates.sh");
    assert!(
        script.is_file(),
        "{} is missing — the lane this gate speaks for cannot be audited",
        script.display()
    );
    let out = Command::new("bash")
        .arg(&script)
        .arg("--audit")
        .arg(&root)
        .output()
        .unwrap_or_else(|e| panic!("{}: {e}", script.display()));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

/// The counts the audit printed, keyed by bucket.
fn counts(text: &str) -> std::collections::BTreeMap<String, usize> {
    text.lines()
        .flat_map(|l| l.split_whitespace())
        .filter_map(|f| f.split_once('='))
        .filter_map(|(k, v)| v.parse().ok().map(|n| (k.to_string(), n)))
        .collect()
}

/// THE GATE: every test file the lane selects is classified, so the lane can run.
///
/// A failure here is the same defect that darkens the nightly lane, found one landing
/// earlier and against this tree rather than against master at 05:17.
#[test]
fn every_selected_test_file_is_classified() {
    let (ok, text) = audit();
    assert!(
        ok,
        "the source-gate lane cannot classify this tree, so it will refuse to run and \
         produce no coverage until this is fixed. Put each named file in the run list in \
         scripts/nightly_source_gates.sh if it reads the tree, or leave it alone if it \
         only names one — the lane derives that half itself. Audit output:\n{text}"
    );
}

/// THE PARTITION IS TOTAL — every selected file lands in exactly one bucket.
///
/// A green audit is only worth what its population is worth: a classifier that silently
/// dropped files would report `unclassified=0` and be believed. The four bucket sizes
/// must reconcile against the number of files the selector actually scanned, and the
/// scan must have found something — zero files is a vacuous pass and is refused here
/// rather than counted as agreement.
#[test]
fn the_bucket_counts_reconcile_against_the_scanned_population() {
    let (_, text) = audit();
    let c = counts(&text);
    let get = |k: &str| {
        *c.get(k)
            .unwrap_or_else(|| panic!("the audit printed no `{k}=` count:\n{text}"))
    };
    let scanned = get("scanned");
    assert!(scanned > 0, "the selector matched no test file — a classification over an empty population is not one:\n{text}");
    let source = get("source");
    assert!(source > 0, "no file is classified as a source gate, so the lane would run nothing:\n{text}");
    let sum = source + get("artifact") + get("no-reference") + get("unclassified");
    assert_eq!(sum, scanned, "the buckets hold {sum} of {scanned} scanned files — the classification lost some:\n{text}");
}
