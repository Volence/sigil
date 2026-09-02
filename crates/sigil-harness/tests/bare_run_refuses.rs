//! A BARE RUN STOPS. A DECLARED PARTIAL RUN SAYS HOW MUCH IT LEFT UNMEASURED.
//!
//! `d-18`, ruled `refuse` by the hub on 2026-09-02 under the owner's widened delegation
//! (`docs/OVERSEER.md`, R4; empyrean `4e8e865b`). The card's own recommendation was
//! say-only and the hub's reason for overruling it is the one this gate exists to keep
//! true: **a run that prints how much it skipped still exits 0**, and a silent green is the
//! class never dropped, because a green is trusted the moment it is in the run.
//!
//! The measured version of that, from the `absolute-path-classify` parcel (`8a377311`):
//! with the reference tree entirely absent, the ordinary run was 4168 passed / 0 failed /
//! exit 0 — a fully green suite that measured several hundred fewer rows than it appeared
//! to. Every one of those rows skipped through a guard printing one `skip:` line to stderr,
//! captured by the harness and counted by nobody.
//!
//! ## The three directions, and why all three are needed
//!
//! | Environment | Must |
//! |---|---|
//! | nothing named | STOP, naming both variables, the derived path it declined, and the opt-in |
//! | `SIGIL_ALLOW_PARTIAL=1`, nothing named | pass, skip the reference rows, print the derived not-measured size |
//! | `AEON_DIR` named | run normally, no refusal, no partial banner |
//!
//! The third is not a formality. Without it a refusal that fired unconditionally — from a
//! resolver broken in any way at all — would satisfy the first two, and this file would
//! report that `d-18` is implemented when what it had measured is that nothing works.
//!
//! ## Why a subprocess, and what the parent refuses to assume
//!
//! The property is about the ENVIRONMENT, and a landing run always sets `AEON_DIR`; libtest
//! runs a binary's tests in parallel threads of one process, so an in-process `set_var`
//! would race. Each direction is therefore a child of THIS binary — reference-dependent
//! itself, so it is a real subject rather than a stand-in — with an environment built from
//! scratch. A child that never started exits non-zero and prints nothing, which from
//! outside is indistinguishable from a child that ran and found nothing, so the parent
//! checks for libtest's own result line before believing any of it.

use std::process::Command;

use sigil_harness::test_support::{
    ALLOW_PARTIAL_VAR, AEON_DIR_VAR, NO_REFERENCE_TREE, SUITE_ROOT_VAR,
};

/// Selects the child body.
const CHILD_VAR: &str = "SIGIL_BARE_RUN_CHILD";

/// THE CHILD. It is reference-dependent on purpose: it opens with the same guard every
/// port gate opens with, so what this file measures is the real path and not a mock of it.
/// THE WITNESS TOKENS the parent matches on. Deliberately NOT spelled in skip vocabulary,
/// and `tests/skip_marker_lint.rs` is what made that explicit — it flagged an earlier
/// `"CHILD skipped"` here under its lexical detector.
///
/// The lint is right, and the fix is not a rewording to get past it. A `skip:`-shaped line
/// announces that a SUITE ROW measured nothing, and `scripts/landing-run.sh` and
/// `refreeze.rs` count those out of a run's log. These lines are neither: they are a child
/// process reporting to its parent, which asserts on them. Spelling them like skips would
/// put a skip in the log of a run that skipped nothing — the same wrong number the marker
/// discipline exists to prevent, pointing the other way.
///
/// Nothing is lost by not announcing a skip here: when the guard returns `None` it is
/// `test_support::reference_tree` that has already printed the canonical
/// `skip: reference ROM not at …` line, and the parent asserts on that path too.
const WITNESS_RAN: &str = "CHILD-WITNESS guard yielded a tree";
const WITNESS_NO_TREE: &str = "CHILD-WITNESS guard yielded no tree";

/// THE CHILD. It is reference-dependent on purpose: it opens with the same guard every
/// port gate opens with, so what this file measures is the real path and not a mock of it.
#[test]
fn the_child_opens_a_reference_dependent_gate() {
    if std::env::var_os(CHILD_VAR).is_none() {
        // Inert in the parent, and SILENT: an announcement here would be an early return
        // that says something, which is the shape the skip-marker lint holds to one
        // spelling. This row announces nothing because it did nothing.
        // `a_bare_run_refuses_and_a_declared_partial_run_says_its_size` is what drives it.
        return;
    }
    match sigil_harness::test_support::reference_tree(&["engine"]) {
        Some(aeon) => println!("{WITNESS_RAN} {}", aeon.display()),
        None => println!("{WITNESS_NO_TREE}"),
    }
}

/// A child run's combined output, with the parent refusing to believe a child it cannot
/// account for.
fn run_child(env: &[(&str, Option<&str>)]) -> (bool, String) {
    let exe = std::env::current_exe().expect("the test binary knows its own path");
    let mut cmd = Command::new(&exe);
    cmd.arg("--nocapture")
        .arg("--test-threads=1")
        .arg("the_child_opens_a_reference_dependent_gate")
        .env(CHILD_VAR, "1");
    // Every variable that could answer is removed first, so a direction's environment is
    // exactly what it declares rather than what the parent happened to inherit.
    cmd.env_remove(AEON_DIR_VAR)
        .env_remove(SUITE_ROOT_VAR)
        .env_remove(ALLOW_PARTIAL_VAR)
        .env_remove("SIGIL_STRICT_GATE");
    for (k, v) in env {
        match v {
            Some(val) => cmd.env(k, val),
            None => cmd.env_remove(k),
        };
    }
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("UNMEASURABLE: could not run {}: {e}", exe.display()));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        text.contains("test result:"),
        "UNMEASURABLE: the child produced no libtest result line, so it cannot be established \
         that it ran a test at all — and a child that never started looks exactly like one that \
         ran and stayed quiet.\n{text}"
    );
    (out.status.success(), text)
}

#[test]
fn a_bare_run_refuses_and_a_declared_partial_run_says_its_size() {
    if std::env::var_os(CHILD_VAR).is_some() {
        return; // the child runs only the gate row above
    }

    // ── DIRECTION 1: nothing named. The run STOPS.
    let (ok, bare) = run_child(&[]);
    assert!(
        !ok,
        "a bare run with no reference tree named PASSED. That is the whole defect d-18 \
         closed: the run measured nothing it could attribute and reported success.\n{bare}"
    );
    for needle in [AEON_DIR_VAR, SUITE_ROOT_VAR, ALLOW_PARTIAL_VAR] {
        assert!(
            bare.contains(needle),
            "the refusal must name `{needle}`. The variables that would have answered and the \
             opt-in that takes the partial run are the whole of what a reader can do about \
             it.\n{bare}"
        );
    }
    assert!(
        bare.contains("DECLINED to use"),
        "the refusal must say it DECLINED a tree it could have used, not merely that it found \
         none — otherwise a reader cannot tell this from a resolver that broke.\n{bare}"
    );
    // The refusal is a FAILURE and must not be countable as a skip: `landing-run.sh` and
    // `refreeze.rs` both count these two spellings out of a run's log, and a stop that
    // registered as a skip would be reported by the very run it stopped.
    let refusal_lines: Vec<&str> = bare
        .lines()
        .filter(|l| l.contains("NO REFERENCE TREE IS NAMED"))
        .collect();
    assert!(!refusal_lines.is_empty(), "no refusal line to check for skip spellings:\n{bare}");
    for l in &refusal_lines {
        assert!(
            !l.contains("skip:") && !l.contains("skipping"),
            "the refusal is countable as a skip: {l}"
        );
    }

    // ── DIRECTION 2: the declared partial run. It PASSES, the row skips, and the run says
    // how much it left alone.
    let (ok, partial) = run_child(&[(ALLOW_PARTIAL_VAR, Some("1"))]);
    assert!(ok, "a declared partial run must pass — that is what declaring it is for.\n{partial}");
    assert!(
        partial.contains(WITNESS_NO_TREE),
        "the partial run must leave the reference-dependent row UNMEASURED. It reported \
         otherwise, which means it found a tree — and a partial run that quietly measures \
         against the live checkout is the behaviour the refusal exists to prevent.\n{partial}"
    );
    assert!(
        partial.contains(NO_REFERENCE_TREE),
        "each skipped row must name `{NO_REFERENCE_TREE}` as the path it did not find, so the \
         reason for the skip travels with the skip instead of living in a banner hundreds of \
         lines earlier.\n{partial}"
    );
    assert!(
        partial.contains("PARTIAL RUN"),
        "the partial run must announce itself.\n{partial}"
    );
    // The SIZE, derived here the same way the banner derives it. Not a typed number on
    // either side, and an underivable population is UNMEASURABLE rather than zero.
    let gated = sigil_harness::reference_dependence::reference_dependent_binaries(
        &sigil_harness::reference_dependence::workspace_root(),
    );
    assert!(
        gated.len() > sigil_harness::reference_dependence::FLOOR,
        "UNMEASURABLE: the reference-dependent derivation found only {}, below its own floor, so \
         this gate cannot say what size the banner should have carried",
        gated.len()
    );
    assert!(
        partial.contains(&gated.len().to_string()),
        "the partial run must print the DERIVED size of what it did not measure ({} binaries). \
         A partial run that does not say its size is the say-nothing behaviour d-18 \
         replaced.\n{partial}",
        gated.len()
    );

    // ── DIRECTION 3: a NAMED tree runs normally. Without this arm, a refusal that fired
    // unconditionally would satisfy both arms above.
    let named = env!("CARGO_MANIFEST_DIR"); // a real directory, and not an aeon tree
    let (ok, run) = run_child(&[(AEON_DIR_VAR, Some(named))]);
    assert!(ok, "a run with AEON_DIR naming a directory must not be refused.\n{run}");
    assert!(
        !run.contains("NO REFERENCE TREE IS NAMED"),
        "the refusal fired even though AEON_DIR named a tree, so directions 1 and 2 above prove \
         nothing about the bare case specifically.\n{run}"
    );
    assert!(
        !run.contains("PARTIAL RUN"),
        "a named run is not a partial run.\n{run}"
    );
    assert!(
        run.contains(&format!("step 1 — named by {AEON_DIR_VAR}")),
        "a named run must announce which step answered.\n{run}"
    );
}

/// A STRICT run cannot also be a partial one, and the resolver says so rather than picking.
///
/// The two flags describe opposite runs: strict is the run that may not skip a gate.
/// Letting them coexist would produce a strict run whose reference rows all skipped — a
/// landing-grade green over an unmeasured half, which is the same defect wearing the
/// stricter flag.
#[test]
fn strict_and_partial_together_are_refused() {
    if std::env::var_os(CHILD_VAR).is_some() {
        return;
    }
    let (ok, out) = run_child(&[(ALLOW_PARTIAL_VAR, Some("1")), ("SIGIL_STRICT_GATE", Some("1"))]);
    assert!(
        !ok,
        "a run declaring itself both STRICT and PARTIAL was accepted. A strict run is the one \
         that may not skip a gate; accepting this produces a landing-grade green over rows that \
         were never measured.\n{out}"
    );
    assert!(
        out.contains(ALLOW_PARTIAL_VAR) && out.contains("SIGIL_STRICT_GATE"),
        "the refusal must name both flags, since the fix is to drop one of them.\n{out}"
    );
    // WHICH refusal fired, and this line is load-bearing. Measured, not assumed: with the
    // resolver's combined check removed, the child still failed — `reference_tree` reached
    // the absent partial-run path and the pre-existing strict assertion refused it by path.
    // The run does stop either way, but the message a reader gets is
    // "SIGIL_STRICT_GATE set but reference missing: /nonexistent/…", which names a symptom
    // and leaves the contradiction between the two flags to be inferred. The assertions
    // above could not tell the two apart, so they passed over the removed check.
    assert!(
        out.contains("describe opposite runs"),
        "the run stopped, but not with the resolver's own refusal — so what this gate measured \
         is that SOMETHING failed downstream of the contradiction rather than that the \
         contradiction itself was caught. A reader of that failure is told a path is missing, \
         not that the two flags they set cannot both hold.\n{out}"
    );
}
