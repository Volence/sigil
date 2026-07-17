//! Contract-grammar v2 §1 — the transitive register-effect closure (the pure
//! fixpoint algorithm, tested on synthetic `ProcNode` maps with no grammar
//! dependency).

use sigil_frontend_emp::closure::{check_firings, compute_closure, Firing, ProcNode, RegEffect};
use std::collections::{BTreeMap, BTreeSet};

/// A concrete (non-⊤) effect over the given register spellings.
fn eff(rs: &[&str]) -> RegEffect {
    RegEffect { top: false, regs: regs(rs) }
}

fn regs(rs: &[&str]) -> BTreeSet<String> {
    rs.iter().map(|s| s.to_string()).collect()
}

/// A leaf proc (no callees, no indirect sites) has `effective` equal to its own
/// local writes — the base case of the fixpoint.
#[test]
fn leaf_effective_is_local_writes() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "Leaf".to_string(),
        ProcNode { local_writes: regs(&["d0", "d1"]), ..Default::default() },
    );
    let c = compute_closure(&procs, &BTreeMap::new());
    assert_eq!(c.effective["Leaf"], eff(&["d0", "d1"]));
}

/// A caller's effective set unions in its direct callee's effect (§1) — the
/// whole point of transitivity: a proc that itself writes nothing but calls a
/// scribbler is charged the scribbler's writes.
#[test]
fn direct_callee_effect_unions_in() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "Caller".to_string(),
        ProcNode {
            local_writes: regs(&["a0"]),
            direct_callees: vec!["Callee".to_string()],
            ..Default::default()
        },
    );
    procs.insert(
        "Callee".to_string(),
        ProcNode { local_writes: regs(&["d3", "d4"]), ..Default::default() },
    );
    let c = compute_closure(&procs, &BTreeMap::new());
    assert_eq!(c.effective["Caller"], eff(&["a0", "d3", "d4"]));
    assert_eq!(c.effective["Callee"], eff(&["d3", "d4"]));
}

/// Transitivity chains through multiple levels (A→B→C): A is charged C's writes.
#[test]
fn transitive_chain_propagates() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "A".to_string(),
        ProcNode { direct_callees: vec!["B".to_string()], ..Default::default() },
    );
    procs.insert(
        "B".to_string(),
        ProcNode {
            local_writes: regs(&["d1"]),
            direct_callees: vec!["C".to_string()],
            ..Default::default()
        },
    );
    procs.insert(
        "C".to_string(),
        ProcNode { local_writes: regs(&["d2"]), ..Default::default() },
    );
    let c = compute_closure(&procs, &BTreeMap::new());
    assert_eq!(c.effective["A"], eff(&["d1", "d2"]));
}

/// Mutual recursion (A↔B) — the fixpoint must TERMINATE (this test hanging is
/// the failure) and both procs get the union of the whole SCC's writes (§1's
/// "Recursion/SCCs: fixpoint from ∅ … terminates").
#[test]
fn mutual_recursion_scc_terminates_and_unions() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "A".to_string(),
        ProcNode {
            local_writes: regs(&["d0"]),
            direct_callees: vec!["B".to_string()],
            ..Default::default()
        },
    );
    procs.insert(
        "B".to_string(),
        ProcNode {
            local_writes: regs(&["d1"]),
            direct_callees: vec!["A".to_string()],
            ..Default::default()
        },
    );
    let c = compute_closure(&procs, &BTreeMap::new());
    assert_eq!(c.effective["A"], eff(&["d0", "d1"]));
    assert_eq!(c.effective["B"], eff(&["d0", "d1"]));
}

/// An UNBOUNDED indirect call site (`None`) makes the proc's effect ⊤ — §1's
/// load-bearing fact (`RunObjects`'s bare `jsr (a1)` poisons the graph).
#[test]
fn unbounded_indirect_is_top() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "Dispatch".to_string(),
        ProcNode { indirect_sites: vec![None], ..Default::default() },
    );
    let c = compute_closure(&procs, &BTreeMap::new());
    assert!(c.effective["Dispatch"].top);
}

/// ⊤ is absorbing and propagates transitively: a caller of a ⊤ proc is ⊤.
#[test]
fn top_propagates_to_callers() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "Caller".to_string(),
        ProcNode {
            local_writes: regs(&["d0"]),
            direct_callees: vec!["Unbounded".to_string()],
            ..Default::default()
        },
    );
    procs.insert(
        "Unbounded".to_string(),
        ProcNode { indirect_sites: vec![None], ..Default::default() },
    );
    let c = compute_closure(&procs, &BTreeMap::new());
    assert!(c.effective["Caller"].top);
}

/// A BOUNDED indirect site (`Some(type)`) contributes only the contract type's
/// clobber bound, not ⊤ (§4) — this is why the boundary decls stop the poison.
#[test]
fn bounded_indirect_uses_type_clobbers() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "Dispatch".to_string(),
        ProcNode {
            indirect_sites: vec![Some("HBlankHandler".to_string())],
            ..Default::default()
        },
    );
    let mut types = BTreeMap::new();
    types.insert("HBlankHandler".to_string(), eff(&["d0", "d1", "a0"]));
    let c = compute_closure(&procs, &types);
    assert_eq!(c.effective["Dispatch"], eff(&["d0", "d1", "a0"]));
}

/// An `extern proc` is a closure LEAF: its effect is exactly its declared
/// clobbers, callees/indirect ignored (§3 — opaque `.asm` body, trusted).
#[test]
fn extern_leaf_effect_is_declared_clobbers() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "VSync_Wait".to_string(),
        ProcNode {
            is_extern: true,
            declared_clobbers: regs(&["d0"]),
            // even if some junk callee/indirect were present, extern ignores it:
            indirect_sites: vec![None],
            ..Default::default()
        },
    );
    let c = compute_closure(&procs, &BTreeMap::new());
    assert_eq!(c.effective["VSync_Wait"], eff(&["d0"]));
    assert!(!c.effective["VSync_Wait"].top);
}

/// A callee named by some proc but absent from the map (and not an extern) is a
/// HOLE — surfaced in `unresolved_callees` (§1: an undeclared extern call is a
/// hole, error under strict).
#[test]
fn absent_callee_is_unresolved() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "Caller".to_string(),
        ProcNode {
            direct_callees: vec!["MysteryAsmRoutine".to_string()],
            ..Default::default()
        },
    );
    let c = compute_closure(&procs, &BTreeMap::new());
    assert!(c.unresolved_callees.contains("MysteryAsmRoutine"));
}

// ---------------------------------------------------------------------------
// The firing check (§9) — effective vs declared clobbers∪params∪out.
// ---------------------------------------------------------------------------

/// A proc that declares `clobbers(d0)` but writes `d7` itself fires a
/// DIRECT (non-transitive) firing — the transitive analog of the local
/// `[proc.clobber-undeclared]` (the RunObjects d7 census case).
#[test]
fn direct_under_declaration_fires() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "RunObjects".to_string(),
        ProcNode {
            local_writes: regs(&["d0", "d7"]),
            declared_clobbers: regs(&["d0"]),
            has_clobber_contract: true,
            ..Default::default()
        },
    );
    let c = compute_closure(&procs, &BTreeMap::new());
    let f = check_firings(&procs, &c);
    assert_eq!(
        f,
        vec![Firing {
            proc: "RunObjects".to_string(),
            reg: Some("d7".to_string()),
            transitive: false,
            unbounded: false,
        }]
    );
}

/// A proc that declares a tight `clobbers(d0)` and writes only d0 itself, but
/// CALLS a callee clobbering d1, fires a TRANSITIVE firing on d1 — the new
/// class the local lint cannot see (this is what the checkpoint watches for).
#[test]
fn transitive_leak_fires_as_transitive() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "Caller".to_string(),
        ProcNode {
            local_writes: regs(&["d0"]),
            declared_clobbers: regs(&["d0"]),
            direct_callees: vec!["Scribbler".to_string()],
            has_clobber_contract: true,
            ..Default::default()
        },
    );
    procs.insert(
        "Scribbler".to_string(),
        ProcNode { local_writes: regs(&["d1"]), ..Default::default() },
    );
    let c = compute_closure(&procs, &BTreeMap::new());
    let f = check_firings(&procs, &c);
    assert_eq!(
        f,
        vec![Firing {
            proc: "Caller".to_string(),
            reg: Some("d1".to_string()),
            transitive: true,
            unbounded: false,
        }]
    );
}

/// A register that is a param or an `out` result is ALLOWED — not a firing
/// (the 3 SAT a4s land as `out(a4)`, and params are declarative bindings).
#[test]
fn params_and_out_are_allowed() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "DrawRings".to_string(),
        ProcNode {
            local_writes: regs(&["a0", "a4", "d5"]),
            params: regs(&["a0"]),
            out: regs(&["a4", "d5"]),
            declared_clobbers: BTreeSet::new(),
            has_clobber_contract: true,
            ..Default::default()
        },
    );
    let c = compute_closure(&procs, &BTreeMap::new());
    assert_eq!(check_firings(&procs, &c), vec![]);
}

/// A NO-CONTRACT proc fires nothing even if it scribbles — invisible to the
/// lint until it declares a contract (census A2; the 12 stubs are a retrofit
/// worklist, not firings, until they gain `clobbers()`).
#[test]
fn no_contract_proc_never_fires() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "Touch_None".to_string(),
        ProcNode {
            local_writes: regs(&["d0", "d1", "d2"]),
            has_clobber_contract: false,
            ..Default::default()
        },
    );
    let c = compute_closure(&procs, &BTreeMap::new());
    assert_eq!(check_firings(&procs, &c), vec![]);
}

/// A proc whose effective set is ⊤ (an unbounded indirect leaked through) but
/// which declares a bounded `clobbers` contract fires ONE unbounded firing.
#[test]
fn unbounded_effective_against_bounded_contract_fires() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "Leaky".to_string(),
        ProcNode {
            declared_clobbers: regs(&["d0"]),
            indirect_sites: vec![None],
            has_clobber_contract: true,
            ..Default::default()
        },
    );
    let c = compute_closure(&procs, &BTreeMap::new());
    let f = check_firings(&procs, &c);
    assert_eq!(f.len(), 1);
    assert!(f[0].unbounded);
    assert_eq!(f[0].reg, None);
}

// ---------------------------------------------------------------------------
// verifiedPreserved subtraction (§1/§5 D2.32 fast path): a register a proc
// writes but PROVABLY preserves (declared + movem-verified) is subtracted from
// its effective set — so it neither fires nor propagates to callers.
// ---------------------------------------------------------------------------

/// A proc that writes d0,d1 but has verified_preserves {d1} has effective {d0}
/// — the preserved register is subtracted after the union.
#[test]
fn verified_preserves_subtracts_from_effective() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "P".to_string(),
        ProcNode {
            local_writes: regs(&["d0", "d1"]),
            verified_preserves: regs(&["d1"]),
            ..Default::default()
        },
    );
    let c = compute_closure(&procs, &BTreeMap::new());
    assert_eq!(c.effective["P"], eff(&["d0"]));
}

/// A verified-preserved register does NOT propagate to callers: a caller of a
/// proc that preserves d1 (even though it writes it) is not charged d1 — this
/// is what clears Sound_PlayRing (Sound_PlaySFX declares+verifies preserves).
#[test]
fn verified_preserves_not_inherited_by_callers() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "Caller".to_string(),
        ProcNode {
            local_writes: regs(&["d0"]),
            direct_callees: vec!["Preserver".to_string()],
            declared_clobbers: regs(&["d0"]),
            has_clobber_contract: true,
            ..Default::default()
        },
    );
    procs.insert(
        "Preserver".to_string(),
        ProcNode {
            local_writes: regs(&["d0", "d1", "a0"]),
            verified_preserves: regs(&["d1", "a0"]),
            ..Default::default()
        },
    );
    let c = compute_closure(&procs, &BTreeMap::new());
    // Preserver's effective is d0 only (d1/a0 preserved); Caller inherits only d0.
    assert_eq!(c.effective["Preserver"], eff(&["d0"]));
    assert_eq!(c.effective["Caller"], eff(&["d0"]));
    assert!(check_firings(&procs, &c).is_empty(), "no firing — d1/a0 preserved");
}
