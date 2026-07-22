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
//! Reference tree: defaults to the sibling aeon checkout (override with `AEON_DIR`).
//! Under `SIGIL_STRICT_GATE` a missing tree HARD-FAILS — these are shipping ERROR
//! gates and must run in the standard strict invocation, not silently skip.

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

/// The substrate gate — DROPS ARE LOUD. The contract analysis evaluates each
/// `.emp` against the whole-corpus TYPE ENVIRONMENT (every struct/const/type
/// declaration in scope), so no field operand on an imported struct silently
/// vanishes from an analysis buffer. This pins the count of dropped instructions
/// to ZERO across the corpus: a silent under-approximation of any downstream
/// analysis (write set, clobber closure, dead-save, liveness) can no longer
/// return. It is the load-bearing precondition of every other contract gate —
/// before the corpus type environment, ~150 instructions across 24 files were
/// dropping, hiding real register effects beneath the closure/dead-save gates.
#[test]
fn corpus_has_zero_dropped_instructions() {
    // House reference-gate pattern (repin_pins/mt_port, c5505f8): default the
    // sibling aeon tree, and under SIGIL_STRICT_GATE a missing reference is a HARD
    // failure. A shipping ERROR gate that silently skips whenever AEON_DIR is unset
    // — as the standard strict invocation (`SIGIL_STRICT_GATE=1 cargo test
    // --workspace`, no AEON_DIR) leaves it — never actually runs in the gate it
    // exists for.
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    );
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return;
    }
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
    assert_eq!(
        r.dropped_instrs, 0,
        "instructions dropped from analysis buffers (missing import/type in scope?): {:?}",
        r.dropped_by_proc
    );
}

#[test]
fn corpus_closure_residue_is_empty_the_error_gate() {
    // House reference-gate pattern (repin_pins/mt_port, c5505f8): default the
    // sibling aeon tree, and under SIGIL_STRICT_GATE a missing reference is a HARD
    // failure. A shipping ERROR gate that silently skips whenever AEON_DIR is unset
    // — as the standard strict invocation (`SIGIL_STRICT_GATE=1 cargo test
    // --workspace`, no AEON_DIR) leaves it — never actually runs in the gate it
    // exists for.
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    );
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return;
    }
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
    // House reference-gate pattern (repin_pins/mt_port, c5505f8): default the
    // sibling aeon tree, and under SIGIL_STRICT_GATE a missing reference is a HARD
    // failure. A shipping ERROR gate that silently skips whenever AEON_DIR is unset
    // — as the standard strict invocation (`SIGIL_STRICT_GATE=1 cargo test
    // --workspace`, no AEON_DIR) leaves it — never actually runs in the gate it
    // exists for.
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    );
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return;
    }
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

/// Load the aeon corpus + run the contract analysis, or `None` (skip) when the
/// reference tree is absent — hard-failing under `SIGIL_STRICT_GATE` (the house
/// reference-gate pattern). Shared by the D1b-flip gates below.
fn corpus_report() -> Option<sigil_frontend_emp::corpus_contracts::ContractReport> {
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

/// THE D1b ERROR GATE (Phase-1 item #4 flip). Every register param of every
/// callee has a reaching definition on EVERY path at each `.emp` call site — the
/// corpus has ZERO `[call.input-undefined]` firings. This shipped WARN through G4;
/// now, with the credit source switched to the VERIFIED-out fixpoint (an out is a
/// definition only once PROVEN honest — the FindStagedBlock existence-lie can no
/// longer silently satisfy an input), it is the permanent ERROR gate: under the
/// strict invocation, ANY call passing an undefined register input is a build error
/// — the exact mistake a pass-3 contract-trusting register hoist could make.
#[test]
fn corpus_input_undefined_is_empty_the_error_gate() {
    let Some(r) = corpus_report() else { return };
    assert!(
        r.input_firings.is_empty(),
        "[call.input-undefined] (D1b): a callee register-param input has no reaching \
         definition on some path — it must be defined before the call: {:?}",
        r.input_firings
            .iter()
            .map(|f| (f.proc.as_str(), f.callee.as_str(), f.reg.as_str()))
            .collect::<Vec<_>>()
    );
}

/// §6 divergence TRIPWIRE (the honest-residual guard for keeping §6 on DECLARED
/// credit). §6 result-invalid-path uses the declared out maps (redefine-kill
/// semantics — a width-unverified out still redefines its register). This asserts
/// the §6 firings computed with DECLARED credit EQUAL those under VERIFIED credit
/// TODAY. The day a corpus change makes them diverge, declared credit is
/// suppressing a real firing that verified credit would surface on this ERROR gate
/// — the test fails and forces adjudication (move §6 to verified, or a per-lie-class
/// credit) at the moment it matters, instead of a silent miss. See the gap-ledger
/// row + the dividing-line table in the residue note.
#[test]
fn corpus_flag_results_declared_vs_verified_credit_agree() {
    let Some(r) = corpus_report() else { return };
    assert_eq!(
        r.flag_firings, r.flag_firings_verified_credit,
        "§6 invalid-path DIVERGES between declared and verified out-credit — declared \
         credit is suppressing a firing verified credit would show on the ERROR gate. \
         Adjudicate (the define-vs-redefine boundary may need §6 moved to verified). \
         declared={:?} verified={:?}",
        r.flag_firings, r.flag_firings_verified_credit
    );
}

/// CONSISTENCY (brief §2.6): the out-verify residue surface and D1b must-def read
/// ONE fixpoint source, so they cannot disagree on whether an out is honest.
/// (1) every residue firing names an out ABSENT from the verified map (the residue
/// IS the verified complement); (2) a corpus witness that the residue-reporting
/// switch actually landed — `Collision_GetType::out(d0)`, which grounds ONLY in the
/// narrow-width (unverified) `Tile_Cache_GetCollision::out(d0)`, appears here (it
/// would NOT under the pre-switch declared credit). If someone re-points the residue
/// surface back at the declared map, the witness fails.
#[test]
fn corpus_out_residue_is_the_verified_complement() {
    let Some(r) = corpus_report() else { return };
    for f in &r.out_firings {
        let marked_verified =
            r.verified_uncond_out.get(&f.proc).is_some_and(|s| s.contains(&f.reg));
        assert!(
            !marked_verified,
            "{}::out({}) is in the out-verify residue yet marked VERIFIED — the residue \
             surface and must-def credit have drifted apart",
            f.proc, f.reg
        );
    }
    assert!(
        r.out_firings.iter().any(|f| f.proc == "Collision_GetType" && f.reg == "d0"),
        "expected Collision_GetType::out(d0) in the fixpoint residue (chain-grounding \
         through the unverified Tile_Cache_GetCollision) — the residue-reporting switch \
         to verified credit did not land. got: {:?}",
        r.out_firings.iter().map(|f| (f.proc.as_str(), f.reg.as_str())).collect::<Vec<_>>()
    );
}
