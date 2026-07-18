//! Contract-grammar v2 — THE ERROR GATE (§9 tier-timing flip, G3's closing act).
//! Runs the transitive clobber closure ([`analyze_corpus`]) over the REAL aeon
//! `.emp` corpus and pins zero extern holes, zero §11 Q4 collisions, and — now
//! that the §5 verified-preserves retrofit has landed and the row-1030/G3-FP
//! residue reached ZERO — an EMPTY firing set. This pin shipped WARN-tier through
//! G1/G2 (surfacing the 6-row G3 handoff as documented debt); at G3, with the
//! residue provably 0, it flips to the ERROR gate: under `SIGIL_STRICT_GATE`, ANY
//! transitive under-declaration is a BUILD ERROR. An undeclared register effect
//! in `.emp` can no longer ship.
//!
//! Gated on `AEON_DIR` (skips green when the tree is absent, like the port gates).

use sigil_frontend_emp::corpus_contracts::analyze_corpus;
use sigil_frontend_emp::parse_str;
use std::path::{Path, PathBuf};

/// Recursively collect `*.emp` files under `dir`, skipping `.worktrees`.
fn emp_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == ".worktrees") {
                continue;
            }
            emp_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "emp") {
            out.push(p);
        }
    }
}

#[test]
fn corpus_closure_residue_is_empty_the_error_gate() {
    let Ok(aeon) = std::env::var("AEON_DIR") else {
        eprintln!("skip: AEON_DIR not set");
        return;
    };
    let aeon = PathBuf::from(aeon);
    let mut paths = Vec::new();
    emp_files(&aeon.join("engine"), &mut paths);
    emp_files(&aeon.join("games"), &mut paths);
    paths.sort();
    assert!(!paths.is_empty(), "no .emp files under {}", aeon.display());

    let files: Vec<_> = paths
        .iter()
        .map(|p| parse_str(&std::fs::read_to_string(p).unwrap()).0)
        .collect();
    let r = analyze_corpus(&files);

    // Boundary decls resolve every extern call — no holes.
    assert!(
        r.closure.unresolved_callees.is_empty(),
        "unexpected extern holes (missing extern proc?): {:?}",
        r.closure.unresolved_callees
    );
    // No name declared both extern proc and proc (§11 Q4).
    assert!(r.extern_collisions.is_empty(), "extern/proc collisions: {:?}", r.extern_collisions);

    // THE ERROR GATE (WARN→ERROR flip, §9): the residue is now ZERO. Every real
    // under-declaration is fixed; the 6-row G3-FP handoff cleared via §5 verified
    // preserves (5 declared, Load_Object transitively). ANY firing here — an
    // undeclared transitive register effect, or an unbounded indirect — is a build
    // error under the strict gate. This is the permanent gate: an undeclared
    // register effect in `.emp` can no longer ship.
    let residue: Vec<(String, String)> = r
        .firings
        .iter()
        .map(|f| (f.proc.clone(), f.reg.clone().unwrap_or_else(|| "<unbounded>".into())))
        .collect();
    assert!(
        r.firings.is_empty(),
        "closure firing(s) — an undeclared register effect must be declared or \
         verified-preserved before it can ship: {residue:?}"
    );
}

/// Contract-grammar v2 G2 — the §6 flag-result must-use pin: every `.emp` caller
/// of a flag-result callee (`out(carry:)`) CONSUMES the carry, so the corpus has
/// ZERO `[call.flag-result-unused]` / `[call.result-invalid-path]` firings. The
/// three retrofitted callees (QueueDMA_Important/_Deferrable `dropped`,
/// RingBuffer_Add `full`) are all consumed via a `bcs` — no `@discards` anywhere.
/// This pin is the G2 regression guard (mirrors the G1 residue pin); a future
/// caller that drops a flag result breaks it.
#[test]
fn corpus_flag_results_are_all_consumed() {
    let Ok(aeon) = std::env::var("AEON_DIR") else {
        eprintln!("skip: AEON_DIR not set");
        return;
    };
    let aeon = PathBuf::from(aeon);
    let mut paths = Vec::new();
    emp_files(&aeon.join("engine"), &mut paths);
    emp_files(&aeon.join("games"), &mut paths);
    paths.sort();
    assert!(!paths.is_empty(), "no .emp files under {}", aeon.display());

    let files: Vec<_> = paths
        .iter()
        .map(|p| parse_str(&std::fs::read_to_string(p).unwrap()).0)
        .collect();
    let r = analyze_corpus(&files);

    assert!(
        r.flag_firings.is_empty(),
        "unexpected flag-result firings (a dropped carry?): {:?}",
        r.flag_firings
    );
}
