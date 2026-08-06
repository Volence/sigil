//! Contract-grammar v2 §G4.5 — the callee-side `out()` production residue over
//! the REAL aeon corpus. Checkpoint-B inspection: DUMP every `[proc.out-
//! unverified]` firing for adjudication (not yet an assert-empty gate).
//!
//! D1c (`[call.live-clobbered]`) rides here too, and unlike the residue it has
//! TEETH: [`d1c_firings_match_the_frozen_baseline`] pins the exact firing set, so
//! a contract edit that makes D1c miss a real held-value clobber — or invent one
//! — fails the suite rather than scrolling past in a dump.

use sigil_frontend_emp::corpus_contracts::{analyze_corpus, ContractReport};
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

/// The exact `[call.live-clobbered]` (D1c) firing set over the aeon corpus,
/// `(caller, callee, register)`, sorted as the analysis sorts it.
///
/// D1c is OBSERVE-ONLY — it is not an assert-empty gate, and these firings are
/// not all bugs. The baseline exists so the set cannot MOVE unnoticed: a contract
/// edit that suppresses a real held-value clobber (the destructive direction) or
/// invents one fails the suite instead of scrolling past in a dump.
///
/// Two entries are DOCUMENTED false positives, both of the same class — D1c's
/// close is edge-blind, so a register read that is really a conditional callee's
/// PRODUCED value on the cc-success edge looks like a destroyed held value. They
/// are `TileCache_FillRow @ TileCache_FindStagedBlock :: a1` and `Load_Object @
/// AllocDynamic :: a1`; `calls.rs`'s `destroys_value` header carries the
/// per-site reasoning. An edge-precise D1c that never degrades on a bail would
/// dissolve the class.
///
/// The remaining rows are recorded, not adjudicated: they are the corpus's
/// standing D1c surface as of this freeze. Changing one is a decision, and the
/// failure message says so.
const D1C_BASELINE: &[(&str, &str, &str)] = &[
    ("CreateChild_Complex", "AllocDynamic", "a1"),
    ("CreateChild_FlipAware", "AllocDynamic", "a1"),
    ("CreateChild_Linked", "AllocDynamic", "a1"),
    ("CreateChild_Normal", "AllocDynamic", "a1"),
    ("CreateEffect_Normal", "AllocEffect", "a1"),
    ("CreateEffect_Simple", "AllocEffect", "a1"),
    ("GameState_ObjectTestChurn_Init", "AllocDynamic", "a1"),
    ("GameState_ObjectTest_Init", "AllocDynamic", "a1"),
    ("Ground_Move_Cap", "Player_SensorWallDir", "d0"),
    // DOCUMENTED FP (edge-blind close) — see calls.rs::destroys_value.
    ("Load_Object", "AllocDynamic", "a1"),
    ("PState_AirShared", "Air_WallProbeLeft", "d4"),
    ("PState_AirShared", "Air_WallProbeRight", "d1"),
    ("PState_AirShared", "Air_WallProbeRight", "d4"),
    ("PState_Spindash", "Player_SensorFloor", "d0"),
    ("PState_Spindash", "Player_SensorFloor", "d1"),
    ("Parallax_Update", "Decode_Factor_A", "d2"),
    ("Parallax_Update", "Decode_Factor_B", "d2"),
    ("TestPlayer_Main", "Player_SensorFloor", "d0"),
    ("TestPlayer_Main", "Player_SensorFloor", "d2"),
    // DOCUMENTED FP (edge-blind close) — see calls.rs::destroys_value.
    ("TileCache_FillRow", "TileCache_FindStagedBlock", "a1"),
];

/// A firing list as a MULTISET: `(caller, callee, register)` → how many times it
/// fires. The same triple can fire at two call sites in one proc, so counting is
/// what makes a gained or lost duplicate visible; a set membership test would let
/// it through both the added and the removed list.
fn tally(rows: &[(String, String, String)]) -> BTreeMap<&(String, String, String), usize> {
    let mut m = BTreeMap::new();
    for r in rows {
        *m.entry(r).or_default() += 1;
    }
    m
}

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
    let want: Vec<(String, String, String)> = D1C_BASELINE
        .iter()
        .map(|(p, c, reg)| (p.to_string(), c.to_string(), reg.to_string()))
        .collect();

    // MULTISET diff (see `tally`), not a set diff.
    let (gt, wt) = (tally(&got), tally(&want));
    let added: Vec<_> =
        gt.iter().filter(|(k, n)| wt.get(*k).copied().unwrap_or(0) < **n).collect();
    let removed: Vec<_> =
        wt.iter().filter(|(k, n)| gt.get(*k).copied().unwrap_or(0) < **n).collect();
    assert!(
        added.is_empty() && removed.is_empty(),
        "[call.live-clobbered] (D1c) moved against the frozen baseline.\n  \
         NEW firings (a caller now holds a value across a call that destroys it): {added:?}\n  \
         GONE firings (a callee's effective set NARROWED — the same closure feeds \
         find_dead_saves, so a dropped row can mean a load-bearing save is now reported \
         dead): {removed:?}\n  \
         If the change is intended, adjudicate each row and update D1C_BASELINE in the \
         same commit."
    );
    assert_eq!(
        got, want,
        "D1c firing ORDER changed — the analysis sort is part of the pin (the multiset \
         matches, so this is ordering alone)"
    );
}
