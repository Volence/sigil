//! Contract-grammar v2 §G4.5 — the callee-side `out()` production residue over
//! the REAL aeon corpus, and D1c beside it. The residue is DUMPED for reading and
//! PINNED for enforcement: assert-empty is unavailable while most of the residue
//! is verifier-model gap rather than loose contract, so the gate is a ratchet.
//!
//! BOTH families now have TEETH, against the SHARED baseline constants in
//! `sigil_harness::contract_baseline` that the build-integrated closure gate also
//! reads — one copy, so a pin cannot fork into two halves that disagree.

use sigil_frontend_emp::corpus_contracts::{analyze_corpus, ContractReport};
use sigil_harness::contract_baseline;
use sigil_frontend_emp::out_verify::survives_message;
use sigil_frontend_emp::parse_str;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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

/// The whole-corpus contract report over the reference aeon tree, or `None` when
/// that tree is absent and the run is not strict.
///
/// House reference-gate pattern (repin_pins/mt_port, c5505f8): default the
/// sibling aeon tree; under `SIGIL_STRICT_GATE` a missing reference hard-fails so
/// these gates actually run under the standard strict invocation.
fn corpus_report() -> Option<ContractReport> {
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    );
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return None;
    }
    let mut paths = Vec::new();
    emp_files(&aeon.join("engine"), &mut paths);
    emp_files(&aeon.join("games"), &mut paths);
    paths.sort();
    assert!(!paths.is_empty(), "no .emp files under {}", aeon.display());
    let files: Vec<_> =
        paths.iter().map(|p| parse_str(&std::fs::read_to_string(p).unwrap()).0).collect();
    Some(analyze_corpus(&files))
}

#[test]
fn dump_out_unverified_residue() {
    let Some(r) = corpus_report() else { return };

    eprintln!("=== [proc.out-unverified] residue: {} firing(s) ===", r.out_firings.len());
    for f in &r.out_firings {
        eprintln!("  {} :: out({}) — {}", f.proc, f.reg, f.reason);
    }
    eprintln!("=== [call.live-clobbered] D1c: {} firing(s) ===", r.live_clobbered_firings.len());
    for f in &r.live_clobbered_firings {
        eprintln!("  {} @ {} :: {}", f.proc, f.callee, f.reg);
    }
    eprintln!(
        "=== [proc.out-cond-survives-unverifiable]: {} firing(s) ===",
        r.survives_firings.len()
    );
    for f in &r.survives_firings {
        eprintln!("  {} :: out({} if {}) — {}", f.proc, f.reg, f.cc, f.reason);
    }
}

/// The §7.1 SURVIVES claim over the real corpus, under the closure's
/// callee-preserves oracle — the FINAL authority behind the per-file gate's
/// call-blocked deferrals, so it is an assert-EMPTY gate, not a dump.
///
/// Non-vacuity is asserted, not asserted-about: [`SURVIVES_CLAIM_SITES`] names
/// the procs that must still be MAKING a claim, so the empty assert cannot go
/// quietly true by a contract edit that deletes the claim instead of proving it.
/// (The obvious mutation — reverting `TileCache_FindStagedBlock` to
/// `clobbers(d3-d4)` — lives in the aeon tree, so no sigil-side test can perform
/// it; it was run by hand and the gate failed as designed.)
#[test]
fn cond_out_survives_claims_all_prove() {
    let Some(r) = corpus_report() else { return };

    let rows: Vec<String> = r.survives_firings.iter().map(survives_message).collect();
    assert!(
        rows.is_empty(),
        "[proc.out-cond-survives-unverifiable] over the aeon corpus — a conditional out \
         claims its register survives the failure edges and the proof does not carry:\n  {}",
        rows.join("\n  ")
    );
    // The gate must have subjects. A cond-out register ABSENT from the proc's
    // declared clobbers is what makes the claim, and `survives_claim_sites` is
    // exactly that set — NOT `verified_cond_out`, which is the PRODUCTION half's
    // output and stays true even if every claim were downgraded away.
    let claim_sites: Vec<&str> = r.survives_claim_sites.iter().map(String::as_str).collect();
    assert_eq!(
        claim_sites, SURVIVES_CLAIM_SITES,
        "the set of procs MAKING a survives claim moved. The assert above proves nothing \
         about a proc that stopped claiming; adjudicate and update SURVIVES_CLAIM_SITES"
    );
}

/// Every proc that declares `out(rN if cc)` with rN ABSENT from its `clobbers` —
/// i.e. every proc whose survives claim the gate above actually proves.
/// `AllocDynamic` is deliberately not here: it names a1 in `clobbers` and makes
/// no claim.
const SURVIVES_CLAIM_SITES: &[&str] = &["AllocEffect"];

/// D1c HAS TEETH (the gate-weakness row): the corpus's `[call.live-clobbered]`
/// firing set must equal [`D1C_BASELINE`] exactly. A contract edit that narrows
/// a callee's effective set — the destructive direction, since the same closure
/// feeds `find_dead_saves` — drops a row here and fails.
#[test]
fn d1c_firings_match_the_frozen_baseline() {
    let Some(r) = corpus_report() else { return };

    let got: Vec<(String, String, String)> = r
        .live_clobbered_firings
        .iter()
        .map(|f| (f.proc.clone(), f.callee.clone(), f.reg.clone()))
        .collect();
    // The corpus walk here uses NO defines, which is the PLAIN family's view.
    let d = contract_baseline::diff_d1c(&got, false);
    assert!(d.is_clean(), "{}", contract_baseline::adjudication_message("[call.live-clobbered] (D1c)", &d));

    // ORDER is part of the pin: the multiset already matched, so a failure here
    // is the analysis sort alone.
    let want: Vec<(String, String, String)> = contract_baseline::d1c_baseline(false)
        .iter()
        .map(|(p, c, reg)| (p.to_string(), c.to_string(), reg.to_string()))
        .collect();
    assert_eq!(got, want, "D1c firing ORDER changed — the analysis sort is part of the pin");
}

/// `[proc.out-unverified]` now has TEETH too, on the same pattern and against the
/// same shared constants the build gate reads. Before this pin the residue was
/// DUMPED for adjudication and nothing failed when it moved — a new loose `out()`
/// contract could land and scroll past.
///
/// Assert-empty is not available: the corpus carries 30 firings today, most of
/// them verifier-model gaps rather than loose contracts, and the ruling bars
/// editing engine code to please a checker. So the gate is a RATCHET — the set may
/// not grow, and a row that stops firing must be adjudicated rather than silently
/// dropped.
#[test]
fn out_unverified_firings_match_the_frozen_baseline() {
    let Some(r) = corpus_report() else { return };

    let got: Vec<(String, String)> =
        r.out_firings.iter().map(|f| (f.proc.clone(), f.reg.clone())).collect();
    let d = contract_baseline::diff_out_unverified(&got);
    assert!(
        d.is_clean(),
        "{}",
        contract_baseline::adjudication_message("[proc.out-unverified]", &d)
    );

    // Non-vacuity: the baseline is non-empty and the corpus actually produced
    // firings, so a silently-emptied analysis fails here rather than passing.
    assert_eq!(
        got.len(),
        contract_baseline::OUT_UNVERIFIED_BASELINE.len(),
        "firing count moved against the baseline length"
    );
    assert!(!got.is_empty(), "no firings at all — the analysis produced nothing to pin");
}
