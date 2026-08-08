//! Contract-grammar v2 §G4.5 — the callee-side `out()` production residue over
//! the REAL aeon corpus, and D1c beside it. The residue is DUMPED for reading and
//! PINNED for enforcement: assert-empty is unavailable while most of the residue
//! is verifier-model gap rather than loose contract, so the gate is a ratchet.
//!
//! BOTH families now have TEETH, against the SHARED baseline constants in
//! `sigil_harness::contract_baseline` that the build-integrated closure gate also
//! reads — one copy, so a pin cannot fork into two halves that disagree.

use sigil_frontend_emp::corpus_contracts::{analyze_corpus, ContractReport};
use sigil_frontend_emp::out_verify::survives_message;
use sigil_frontend_emp::parse_str;
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
    eprintln!(
        "=== Z80 [proc.out-unverified] residue: {} firing(s), over {} out claim(s) ===",
        r.z80_out_firings.len(),
        r.z80_out_claims.len()
    );
    for f in &r.z80_out_firings {
        eprintln!("  {} :: out({}) — {}", f.proc, f.unit, f.reason);
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

/// No `out(rN: T)` in the corpus names a type the corpus cannot resolve to a
/// width. An unresolvable type is not unsound — it answers the bare 32-bit claim
/// on both sides, exactly as the declaration would with no type at all — but it
/// silently means something other than what its author wrote, and a width cannot
/// be guessed from a name that resolves to nothing.
///
/// Assert-EMPTY rather than baselined: there is no adjudication to make. Either
/// the name is a typo, or the module declaring it is missing from the walk, and
/// both are fixed rather than pinned.
#[test]
fn no_corpus_out_type_is_unresolvable() {
    let Some(r) = corpus_report() else { return };
    let rows: Vec<String> = r
        .unresolvable_out_types
        .iter()
        .map(|(p, reg, ty)| format!("{p} :: out({reg}: {ty})"))
        .collect();
    assert!(
        rows.is_empty(),
        "an `out(rN: T)` names a type the corpus cannot resolve to a width, so it \
         silently claims all 32 bits instead of what was written:\n  {}",
        rows.join("\n  ")
    );
    // NON-VACUITY, measured on the SCAN's own subject matter. `typed_out_slots` is
    // what the type walk saw; a map keyed by proc NAME cannot serve here, because
    // it carries a key whether or not that proc declares a type, so stripping every
    // type off the corpus would leave it unchanged.
    let slots: Vec<(&str, &str)> =
        r.typed_out_slots.iter().map(|(p, g)| (p.as_str(), g.as_str())).collect();
    for exemplar in [
        ("Collision_GetType", "d0"),
        ("GetSineCosine", "d0"),
        ("Tile_Cache_GetTile", "d2"),
    ] {
        assert!(
            slots.contains(&exemplar),
            "the walk did not see `{} :: out({}: T)`, so the assert above ranges \
             over less than it should. slots: {slots:?}",
            exemplar.0,
            exemplar.1
        );
    }
    // The corpus's typed out slots, pinned exactly. A count that only ever grows
    // would let a deletion pass; this notices in both directions and prints the
    // set, so an intended change is adjudicated rather than absorbed.
    assert_eq!(slots.len(), 20, "the corpus's typed out slots: {slots:?}");
}

/// Every proc that declares `out(rN if cc)` with rN ABSENT from its `clobbers` —
/// i.e. every proc whose survives claim the gate above actually proves.
/// `AllocDynamic` is deliberately not here: it names a1 in `clobbers` and makes
/// no claim.
const SURVIVES_CLAIM_SITES: &[&str] = &["AllocEffect"];

// The `[call.live-clobbered]` and `[proc.out-unverified]` baseline gates do NOT
// live here, and the reason is load-bearing: this file's `corpus_report()` walks
// DEFINE-FREE, and a define-free walk discards BOTH arms of every
// `if DEBUG == 1 { }` / `if SOUND_DRIVER_ENABLED == 1 { }`. It is a strict
// under-approximation of every shipped shape, not the view of any one of them —
// see `contract_closure_corpus.rs`'s header, which says so directly. Pinning a
// frozen baseline against it would make the pin unsatisfiable the moment a gated
// arm gained or lost a firing: the define-free walk would report one count and
// the per-shape builds another, with no baseline value satisfying both. Those
// gates walk every shipped shape, in `contract_closure_corpus.rs`.
