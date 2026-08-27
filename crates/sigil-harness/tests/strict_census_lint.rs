//! A FLOOR IS SATISFIABLE BY THE FAILURE IT EXISTS TO CATCH.
//!
//! `refreeze --attest` records `strict_bodies` — the strict-gated consultations a run
//! reached with `SIGIL_STRICT_GATE` observed set — and used to refuse only when that
//! number was ZERO. Zero is a floor, and every partial loss clears it: delete a
//! strict-gated gate, `#[ignore]` it, drop its guard, and the witness falls from 29 to
//! 28 and the tool records a PASS. A gate going dark showed up as a smaller green.
//!
//! [`sigil_harness::strict_census`] replaces that floor with a POPULATION derived from
//! the tree, and `--attest` set-diffs the run's witness against it. This file is the
//! other half of the same discipline: `--attest` runs once per freeze, and a census that
//! has quietly stopped working would not be discovered until then. Here it is exercised
//! against the real tree in every `cargo test --workspace`.
//!
//! WHAT IT ASSERTS
//!
//!   1. The census can be TAKEN on this tree at all — including that every
//!      `strict_gate()` occurrence in the corpus is one the classifier recognises. An
//!      idiom nobody taught it would otherwise shrink the expectation silently, which is
//!      the floor defect wearing a scanner's clothes.
//!   2. It is NON-VACUOUS: files scanned, sites found, tests found. A walker that finds
//!      nothing exits green and is indistinguishable from a clean tree by result alone.
//!   3. It is LOUD when it cannot measure — a missing tree is an `Err`, never an empty
//!      census. This is the poison the census's own design must survive, and it is
//!      exactly the shape of the defect being fixed.
//!   4. Its two detectors are CONSISTENT with the source they claim to describe: every
//!      declared site's line really holds a consultation, and every gated file really
//!      holds the tests the census attributes to it.
//!   5. libtest really does name a test's thread after the test — the measured fact the
//!      witness's second field depends on. Pinned here so a toolchain change that
//!      withdrew it would be a named red rather than a census that quietly reports every
//!      test as unguarded.

use sigil_harness::strict_census::{self, Census};
use std::path::{Path, PathBuf};

fn crates_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is <workspace>/crates/sigil-harness.
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    here.parent()
        .unwrap_or_else(|| panic!("no parent of {}", here.display()))
        .to_path_buf()
}

fn live_census() -> Census {
    strict_census::census(&crates_dir()).unwrap_or_else(|e| panic!("{e}"))
}

/// THE GATE. The census can be taken, and every `strict_gate()` occurrence in the
/// corpus is classified — so the expectation describes the whole population, not the
/// part the scanner happened to recognise.
#[test]
fn the_strict_gate_census_can_be_taken_and_classifies_every_occurrence() {
    let c = live_census();

    // NON-VACUITY, all three axes. Each of these zeroes is a working scanner's
    // impossible result and a broken scanner's ordinary one.
    assert!(
        c.files_scanned > 0,
        "COULD NOT MEASURE: the census walked no files"
    );
    assert!(
        !c.sites.is_empty(),
        "COULD NOT MEASURE: {} file(s) scanned and zero declared strict-gate sites. The \
         corpus has them; a zero means the census broke, not that the tree is clean",
        c.files_scanned
    );
    assert!(
        !c.tests.is_empty(),
        "COULD NOT MEASURE: {} declared site(s) but zero tests attributed to their files — \
         detector B is inert",
        c.sites.len()
    );
    // The excluded population is large and must STAY excluded: these are the
    // missing-reference panics, consulted only when something is absent and never
    // reached in a healthy strict run. Counting them would overstate the expectation
    // several-fold and produce a permanently red gate.
    assert!(
        c.missing_reference_paths > c.sites.len(),
        "the missing-reference population ({}) is no longer larger than the unconditional \
         one ({}) — one of the two classifications has drifted",
        c.missing_reference_paths,
        c.sites.len()
    );

    println!(
        "strict-gate census: {} declared site(s), {} declared test(s), {} scanned file(s), \
         {} missing-reference path(s) excluded, {} plumbing occurrence(s)",
        c.sites.len(),
        c.tests.len(),
        c.files_scanned,
        c.missing_reference_paths,
        c.plumbing,
    );
}

/// LOUD ON UNMEASURABLE. A census that cannot walk the tree must REFUSE, never return an
/// empty population — an empty population compares equal to an empty witness, which is
/// the "couldn't measure rendered as green" shape this whole parcel is closing.
#[test]
fn a_census_that_cannot_walk_the_tree_refuses_rather_than_finding_nothing() {
    let e = strict_census::census(Path::new("/nonexistent/crates"))
        .expect_err("a census of a tree that is not there must be an error, not an empty set");
    assert!(
        e.contains("COULD NOT MEASURE"),
        "the refusal must say it could not measure, not merely fail: {e}"
    );

    // And an empty-but-present tree: scanning succeeds, finds no FILES, and must still
    // refuse rather than hand back a vacuous expectation.
    let tmp = tempfile::tempdir().expect("tempdir");
    let crates = tmp.path().join("crates");
    std::fs::create_dir_all(crates.join("empty-crate/tests")).expect("mkdir");
    let e = strict_census::census(&crates)
        .expect_err("a tree with no files in it must refuse, not report zero");
    assert!(e.contains("COULD NOT MEASURE"), "{e}");

    // EACH FLOOR SEPARATELY REACHED. A guard that no reachable input can reach is a
    // guard that is not there — a sibling lane landed exactly that shape tonight, an
    // anti-vacuity check sitting behind a condition that was false on every path. The
    // case above stops at the no-FILES floor, so the no-SITES floor below it is
    // untested by it; this reaches that one specifically, with a real file present.
    std::fs::write(
        crates.join("empty-crate/tests/no_gates.rs"),
        "#[test]\nfn t() {\n    assert!(true);\n}\n",
    )
    .expect("write");
    let e = strict_census::census(&crates)
        .expect_err("files present but no strict gates must refuse, not report zero sites");
    assert!(
        e.contains("COULD NOT MEASURE") && e.contains("ZERO unconditional"),
        "the no-sites floor must be the one that fired, not the no-files floor: {e}"
    );

    // And the classification floor, likewise reached on its own: an idiom the scanner
    // does not know must REFUSE, not be silently dropped from the expectation.
    std::fs::write(
        crates.join("empty-crate/tests/no_gates.rs"),
        "#[test]\nfn t() {\n    let on = strict_gate();\n    assert!(on);\n}\n",
    )
    .expect("write");
    let e = strict_census::census(&crates)
        .expect_err("an unclassifiable occurrence must refuse, not be dropped");
    assert!(
        e.contains("cannot CLASSIFY"),
        "the classification floor must be the one that fired: {e}"
    );
}

/// The census's two detectors must describe the source they claim to. A scanner can be
/// non-vacuous and still be reading the wrong lines.
#[test]
fn every_declared_site_and_test_is_where_the_census_says_it_is() {
    let c = live_census();
    let root = crates_dir().parent().expect("workspace root").to_path_buf();

    for s in &c.sites {
        let path = root.join(&s.file);
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("COULD NOT MEASURE: {} unreadable ({e})", path.display()));
        let line = src
            .lines()
            .nth(s.line - 1)
            .unwrap_or_else(|| panic!("{} has no line {}", s.file, s.line));
        assert!(
            line.contains("strict_gate()") && line.trim().starts_with("if !"),
            "census says {} declares an unconditional consultation, but that line reads {:?}",
            s.key(),
            line.trim()
        );
    }

    for t in &c.tests {
        let path = root.join(&t.file);
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("COULD NOT MEASURE: {} unreadable ({e})", path.display()));
        let line = src
            .lines()
            .nth(t.line - 1)
            .unwrap_or_else(|| panic!("{} has no line {}", t.file, t.line));
        assert!(
            line.starts_with("fn ") && line.contains(&t.name),
            "census says {}:{} declares test `{}`, but that line reads {:?}",
            t.file,
            t.line,
            t.name,
            line.trim()
        );
        // Detector B's whole premise: a test in a gated file is expected to reach a gate.
        assert!(
            c.sites.iter().any(|s| s.file == t.file),
            "`{}` is attributed to {}, which the census records no site in",
            t.name,
            t.file
        );
    }
}

/// THE MEASURED FACT the witness's test-name field rests on. `std::thread::current()`
/// inside a libtest test is named after the test — measured, not assumed, and including
/// under `--test-threads=1`. If a toolchain ever withdrew it every test would read as
/// unguarded, so the dependency is pinned by name here rather than discovered as a
/// mass red in `--attest`.
#[test]
fn libtest_names_the_test_thread_after_the_test() {
    let current = std::thread::current();
    let name = current.name().unwrap_or(strict_census::UNNAMED_THREAD);
    assert_eq!(
        name, "libtest_names_the_test_thread_after_the_test",
        "libtest no longer names test threads after the test; the witness's second field \
         and detector B both depend on it"
    );
}

/// The population comparison must catch a gate going dark, and must do it BY NAME. A
/// count could only ever have said "28". Driven on synthetic populations here; the
/// end-to-end proof is a real sabotaged run through `refreeze --attest`.
#[test]
fn the_comparison_names_the_gate_that_went_dark() {
    let c = live_census();

    // A witness that satisfies BOTH detectors: every declared site reached, and every
    // declared test reaching one. Built from the census itself, so the only thing under
    // test below is the difference the sabotage introduces.
    let site_in = |file: &str| -> String {
        c.sites
            .iter()
            .find(|s| s.file == file)
            .unwrap_or_else(|| panic!("no site in {file}"))
            .key()
    };
    let any_test_in = |file: &str| -> String {
        c.tests
            .iter()
            .find(|t| t.file == file)
            .map(|t| t.name.clone())
            .unwrap_or_else(|| panic!("no test in {file}"))
    };
    let mut full = String::new();
    for s in &c.sites {
        full.push_str(&format!("{}\t{}\n", s.key(), any_test_in(&s.file)));
    }
    for t in &c.tests {
        full.push_str(&format!("{}\t{}\n", site_in(&t.file), t.name));
    }
    assert_eq!(
        strict_census::defects(&c, &strict_census::parse_witness(&full)),
        Vec::<String>::new(),
        "a witness carrying the whole declared population must be clean, or the sabotage \
         below would be red for the wrong reason"
    );

    // SABOTAGE A — a declared site that the run never reached.
    let dark = c.sites[0].key();
    let without_site: String = full.lines().filter(|l| !l.starts_with(&dark)).fold(
        String::new(),
        |mut acc, l| {
            acc.push_str(l);
            acc.push('\n');
            acc
        },
    );
    let d = strict_census::defects(&c, &strict_census::parse_witness(&without_site));
    assert!(
        d.iter().any(|m| m.contains("DARK GATE") && m.contains(&dark)),
        "the missing gate must be NAMED, not counted: {d:?}"
    );

    // SABOTAGE B — a declared test that reached no gate, with its site still reached by
    // its neighbours. This is the guard-deleted case, invisible to detector A.
    let victim = &c.tests[0];
    let without_test: String = full
        .lines()
        .filter(|l| !l.ends_with(&format!("\t{}", victim.name)))
        .fold(String::new(), |mut acc, l| {
            acc.push_str(l);
            acc.push('\n');
            acc
        });
    let d = strict_census::defects(&c, &strict_census::parse_witness(&without_test));
    assert!(
        d.iter().any(|m| m.contains("UNGUARDED TEST") && m.contains(&victim.name)),
        "the test that reached no gate must be NAMED: {d:?}"
    );
}
