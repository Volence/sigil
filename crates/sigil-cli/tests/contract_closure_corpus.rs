//! Contract-grammar v2 G1 — the closure's WARN-tier surfacing as a strict-gated
//! regression pin. Runs the transitive clobber closure ([`analyze_corpus`]) over
//! the REAL aeon `.emp` corpus and pins its result: zero extern holes, zero
//! §11 Q4 collisions, and exactly the 6-row row-1030/G3-FP residue (the state
//! after G1's boundary + clobbers/out + verified-preserves work). When the debt
//! firing check flips WARN→ERROR at G3, this pin becomes the error gate.
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
fn corpus_closure_residue_is_the_g3_handoff() {
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
    // No unbounded indirect survives (all 6 dispatch sites are `as`-bounded).
    assert!(
        !r.firings.iter().any(|f| f.unbounded),
        "an unbounded indirect leaked: {:?}",
        r.firings.iter().filter(|f| f.unbounded).collect::<Vec<_>>()
    );

    // The residue is EXACTLY the 6-row G3 handoff — the genuinely-inexpressible-
    // today (row-1030) set: individual-push a0 saves + undeclared movem d1 saves.
    // Every real under-declaration is fixed; nothing here is a false clobbers.
    let mut got: Vec<(String, String)> = r
        .firings
        .iter()
        .map(|f| (f.proc.clone(), f.reg.clone().unwrap_or_default()))
        .collect();
    got.sort();
    let want: Vec<(String, String)> = [
        ("AllocDynamic", "a0"),
        ("Collected_CheckRing", "d1"),
        ("Collected_ParkSlot", "a0"),
        ("Collected_UnparkSlot", "a0"),
        ("Killed_CheckObject", "d1"),
        ("Load_Object", "a0"),
    ]
    .iter()
    .map(|(p, r)| (p.to_string(), r.to_string()))
    .collect();
    assert_eq!(got, want, "closure residue drifted from the G3 handoff set");
}
