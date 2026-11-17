//! A6 (step 0 soundness): the `.emp` and AS front ends must agree on every
//! generated instruction, either the same bytes or both refusing. The
//! generator, the comparison and the named exclusions are
//! `support/frontend_diff.rs` (shared with the harness's coverage test, which
//! proves the generator reaches every family x size x EA the aeon build emits).
//!
//! Red-first: with Task 3's `check_imm_ranges` disabled, the first test lists
//! `move.b #256, d1` and its peers as `Disagree` (emp bytes, AS
//! `operand 256 out of range -128..=255`): Sigil finding 2.

use sigil_backend_m68k::m68k;

#[path = "support/frontend_diff.rs"]
mod frontend_diff;

use frontend_diff::{
    cases_m68k, cases_z80, compare, compare_z80, expected_case_count, expected_z80_case_count,
    is_out_of_range, label, report, report_z80, run_all, Outcome, RunResult, EXCLUSIONS,
};
use std::sync::OnceLock;

/// The whole matrix, run once per test binary: the agreement test and the
/// exclusion-use test read the same run (the libtest threads share it).
fn matrix() -> &'static RunResult {
    static M: OnceLock<RunResult> = OnceLock::new();
    M.get_or_init(|| run_all(cases_m68k()))
}

#[test]
fn emp_and_as_front_ends_agree_on_every_generated_m68k_case() {
    let r = matrix();
    let mut tot = (0, 0, 0, 0);
    eprintln!("family            same  refuse  excluded  disagree");
    for (f, t) in &r.per_family {
        eprintln!("{f:<16} {:>5} {:>7} {:>9} {:>9}", t.same, t.refuse, t.excluded, t.disagree);
        tot = (tot.0 + t.same, tot.1 + t.refuse, tot.2 + t.excluded, tot.3 + t.disagree);
    }
    eprintln!("TOTAL n={} same={} refuse={} excluded={} disagree={}", r.n, tot.0, tot.1, tot.2, tot.3);
    assert!(r.bad.is_empty(), "{}", report(&r.bad));
    assert_eq!(r.n, expected_case_count());
}

#[test]
fn every_out_of_range_immediate_is_refused_by_both() {
    let oor: Vec<_> = cases_m68k().into_iter().filter(is_out_of_range).collect();
    assert!(!oor.is_empty());
    let not_refused: Vec<String> = oor
        .iter()
        .filter_map(|c| match compare(c) {
            Outcome::BothRefuse => None,
            o => Some(format!("`{}`: {o:?}", label(c))),
        })
        .collect();
    assert!(
        not_refused.is_empty(),
        "{} of {} out-of-range immediates not refused by both front ends:\n{}",
        not_refused.len(),
        oor.len(),
        not_refused.iter().take(100).cloned().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn emp_and_as_front_ends_agree_on_every_generated_z80_case() {
    let cases = cases_z80();
    assert_eq!(cases.len(), expected_z80_case_count());
    let bad: Vec<_> = cases
        .into_iter()
        .filter_map(|c| match compare_z80(&c) {
            o @ Outcome::Disagree { .. } => Some((c, o)),
            _ => None,
        })
        .collect();
    assert!(bad.is_empty(), "{}", report_z80(&bad));
}

/// An EXCLUSIONS row that matches nothing is a stale hole: fail.
#[test]
fn every_exclusion_is_used() {
    let r = matrix();
    let unused: Vec<&str> = EXCLUSIONS
        .iter()
        .zip(&r.exclusion_hits)
        .filter(|(_, &h)| h == 0)
        .map(|((p, _), _)| *p)
        .collect();
    for ((p, _), h) in EXCLUSIONS.iter().zip(&r.exclusion_hits) {
        eprintln!("exclusion `{p}`: {h} hit(s)");
    }
    assert!(unused.is_empty(), "EXCLUSIONS rows that match no disagreement: {unused:?}");
}

/// The case-count formula is the tables', and the matrix is one case per
/// table row combination (no duplicates, none dropped).
#[test]
fn generator_is_deterministic_and_matches_its_tables() {
    let a = cases_m68k();
    assert_eq!(a, cases_m68k());
    assert_eq!(a.len(), expected_case_count());
}
