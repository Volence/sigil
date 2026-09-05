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

/// An `out` result register is ALLOWED — a written result the caller reads is
/// not a firing (the 3 SAT a4s land as `out(a4)`). A param that is only READ (not
/// written) never enters `effective`, so it never fires either.
#[test]
fn out_results_and_read_only_params_are_allowed() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "DrawRings".to_string(),
        ProcNode {
            // a4/d5 are written AND declared out; a0 is a READ-ONLY input (not in
            // local_writes) — so nothing is an undeclared write.
            local_writes: regs(&["a4", "d5"]),
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

/// A param is NOT an allowed write: a proc that WRITES its input register (and
/// does not declare the effect via `clobbers`/`out`/verified `preserves`) FIRES.
/// This is the soundness the strip fixes — otherwise a proc that trashes its own
/// input would be invisible to the closure and to D1c.
#[test]
fn a_written_param_is_not_excused() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "TrashInput".to_string(),
        ProcNode {
            local_writes: regs(&["a0"]),
            params: regs(&["a0"]),
            declared_clobbers: BTreeSet::new(),
            has_clobber_contract: true,
            ..Default::default()
        },
    );
    let c = compute_closure(&procs, &BTreeMap::new());
    assert_eq!(
        check_firings(&procs, &c),
        vec![Firing { proc: "TrashInput".to_string(), reg: Some("a0".to_string()), transitive: false, unbounded: false }]
    );
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
    assert!(check_firings(&procs, &c).is_empty(), "no firing, d1/a0 preserved");
}

// ---------------------------------------------------------------------------
// §4 subcontract relation (target ⊑ bound) — [dispatch.target-exceeds-bound].
// ---------------------------------------------------------------------------

use sigil_frontend_emp::closure::{subcontract_violations, Contract};

fn contract(clob: &[&str], pres: &[&str], params: &[&str], out: &[&str]) -> Contract {
    Contract {
        clobbers: regs(clob),
        preserves: regs(pres),
        params: regs(params),
        out: regs(out),
        out_cond: Default::default(),
    }
}

/// [`contract`] plus CONDITIONAL outs (`out(rN if cc)`), spelled as `(reg, cc)`
/// pairs. Repeating a register names it under several guards, exactly as two
/// `out(rN if cc)` clauses would.
fn contract_cond(
    clob: &[&str],
    pres: &[&str],
    params: &[&str],
    out: &[&str],
    out_cond: &[(&str, &str)],
) -> Contract {
    let mut map: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
        Default::default();
    for (r, cc) in out_cond {
        map.entry((*r).to_string()).or_default().insert((*cc).to_string());
    }
    Contract { out_cond: map, ..contract(clob, pres, params, out) }
}

/// A target that clobbers MORE than the bound allows violates it (the register
/// is named). target.clobbers ⊆ bound.clobbers.
#[test]
fn target_clobbers_exceeding_bound_violates() {
    let bound = contract(&["d0", "d1"], &[], &[], &[]);
    let target = contract(&["d0", "d1", "d2"], &[], &[], &[]);
    let v = subcontract_violations(&target, &bound);
    assert!(v.iter().any(|s| s.contains("d2")), "must name the offending clobber: {v:?}");
}

/// A conforming target (clobbers ⊆, preserves ⊇, params ⊆, out ⊇) has no
/// violations — the Touch stubs (clobbers()) under TouchHandler(d6-d7/a2-a4).
#[test]
fn conforming_target_is_clean() {
    let bound = contract(&["d6", "d7", "a2", "a3", "a4"], &["a0"], &["a0"], &[]);
    let target = contract(&[], &["a0", "d7"], &[], &["d0"]); // clobbers⊆, preserves⊇, out⊇
    assert!(subcontract_violations(&target, &bound).is_empty());
}

/// A target that fails to preserve what the bound requires violates it
/// (preserves ⊇). The bound demands a0 preserved; the target doesn't.
#[test]
fn target_missing_required_preserve_violates() {
    let bound = contract(&[], &["a0", "d7"], &[], &[]);
    let target = contract(&["d0"], &["a0"], &[], &[]); // preserves a0 but not d7
    let v = subcontract_violations(&target, &bound);
    assert!(v.iter().any(|s| s.contains("d7")), "must name the un-preserved reg: {v:?}");
}

/// A BOUND'S OWN UNCONDITIONAL `out` licenses the target to clobber that
/// register. The clobber test is `target.clobbers ⊆ bound.clobbers ∪ bound.out`:
/// every caller of the bound already knows an unconditional result register is
/// written, so charging the target for writing it is asymmetric with the
/// production test.
#[test]
fn a_bounds_out_licenses_the_target_to_clobber_it() {
    let bound = contract(&["d0"], &[], &[], &["a1"]);
    let target = contract(&["d0", "a1"], &[], &[], &["a1"]);
    assert!(
        subcontract_violations(&target, &bound).is_empty(),
        "a1 is the bound's own declared result: {:?}",
        subcontract_violations(&target, &bound)
    );
}

/// The honest AllocDynamic shape — `clobbers(d0/a1) out(a1 if eq)` bound to a
/// hook spelling the same `clobbers(d0/a1) out(a1 if eq)` — conforms. The license
/// comes from the bound's own `clobbers`, which is where "a1 is indeterminate on
/// the ne edge" is written down.
///
/// Doubles as the MATCHING-CONDITION control for
/// `a_conditional_bound_rejects_a_target_guarding_a_different_cc` below: the same
/// pair with the same guard on both sides, so the rejection there is attributable
/// to the code and to nothing else.
#[test]
fn the_honest_alloc_dynamic_shape_conforms_to_a_matching_bound() {
    let bound = contract_cond(&["d0", "a1"], &[], &[], &[], &[("a1", "eq")]);
    let target = contract_cond(&["d0", "a1"], &[], &[], &[], &[("a1", "eq")]);
    assert!(
        subcontract_violations(&target, &bound).is_empty(),
        "the AllocDynamic-shaped target must conform: {:?}",
        subcontract_violations(&target, &bound)
    );
}

/// THE WALL: a CONDITIONAL out does NOT license a clobber. A cond-out register
/// absent from the bound's `clobbers` is a survives-claim — the bound's callers
/// are entitled to hold it across the call and re-read it on the ¬cc edge. The
/// AllocEffect-shaped bound (`clobbers(d0) out(a1 if eq)`) must therefore REJECT
/// an AllocDynamic-shaped target (`clobbers(d0/a1) out(a1 if eq)`), which leaves
/// a1 indeterminate there.
#[test]
fn a_conditional_out_does_not_license_the_target_to_clobber_it() {
    let bound = contract_cond(&["d0"], &[], &[], &[], &[("a1", "eq")]);
    let target = contract_cond(&["d0", "a1"], &[], &[], &[], &[("a1", "eq")]);
    let v = subcontract_violations(&target, &bound);
    assert!(
        v.iter().any(|s| s.contains("clobbers `a1`")),
        "the bound's survives-claim on a1 must reject a target that destroys it: {v:?}"
    );
}

/// The license is exactly the bound's OWN result registers — a target clobbering
/// something the bound neither permits nor returns still violates.
#[test]
fn the_out_license_does_not_widen_to_arbitrary_registers() {
    let bound = contract(&["d0"], &[], &[], &["a1"]);
    let target = contract(&["d0", "a1", "d5"], &[], &[], &["a1"]);
    let v = subcontract_violations(&target, &bound);
    assert!(v.iter().any(|s| s.contains("d5")), "d5 is unlicensed: {v:?}");
    assert!(!v.iter().any(|s| s.contains("a1")), "a1 is the bound's own out: {v:?}");
}

/// A CONDITIONAL producer does NOT satisfy an UNCONDITIONAL `out` promise: the
/// bound's callers may read the register with no cc test at all, while the target
/// may legally declare `clobbers(rN)` alongside its conditional result — stating
/// outright that it destroys the register the bound promises.
#[test]
fn conditional_out_does_not_satisfy_an_unconditional_bound_out() {
    let bound = contract(&["d0"], &[], &[], &["a1"]);
    let target = contract_cond(&["d0", "a1"], &[], &[], &[], &[("a1", "eq")]);
    let v = subcontract_violations(&target, &bound);
    assert!(
        v.iter().any(|s| s.contains("does not produce output `a1`")),
        "a conditional producer must not satisfy out(a1): {v:?}"
    );
}

/// THE CONDITION IS PART OF THE PROMISE. A bound promising `out(a1 if eq)` is NOT
/// satisfied by a target producing a1 only on `ne`: the bound's callers test `eq`
/// and read a1 there, which is exactly the edge the target leaves indeterminate.
/// Comparing register NAMES alone silently accepts this pair.
#[test]
fn a_conditional_bound_rejects_a_target_guarding_a_different_cc() {
    let bound = contract_cond(&["d0", "a1"], &[], &[], &[], &[("a1", "eq")]);
    let target = contract_cond(&["d0", "a1"], &[], &[], &[], &[("a1", "ne")]);
    let v = subcontract_violations(&target, &bound);
    assert!(
        v.iter().any(|s| s.contains("conditional output `a1`") && s.contains("`eq`")),
        "the target produces a1 on the wrong edge and must be named: {v:?}"
    );
}

/// A target guarding on MORE conditions than the bound demands satisfies it: the
/// bound's edge is covered. This is why the relation is code-set COVERAGE rather
/// than equality.
///
/// a1 is in BOTH contracts' `clobbers` here, which is what makes the extra `ne`
/// edge a write the bound's callers already tolerate. With a1 absent from the
/// bound's `clobbers` that extra edge would break a §7.1 survives-claim, and this
/// relation would still accept it — the unlicensed-`out` term recorded in the gap
/// ledger.
#[test]
fn a_target_guarding_more_ccs_than_the_bound_demands_satisfies_it() {
    let bound = contract_cond(&["d0", "a1"], &[], &[], &[], &[("a1", "eq")]);
    let target = contract_cond(&["d0", "a1"], &[], &[], &[], &[("a1", "eq"), ("a1", "ne")]);
    assert!(
        subcontract_violations(&target, &bound).is_empty(),
        "covering the demanded edge conforms: {:?}",
        subcontract_violations(&target, &bound)
    );
}

/// THE STRENGTHENING ESCAPE: an UNCONDITIONAL producer satisfies a CONDITIONAL
/// promise whatever condition the bound names — producing on every return path
/// covers every edge, so the code comparison must not reach it.
#[test]
fn unconditional_out_satisfies_a_conditional_bound_out() {
    let bound = contract_cond(&["d0"], &[], &[], &[], &[("a1", "eq")]);
    let target = contract(&["d0"], &[], &[], &["a1"]);
    assert!(subcontract_violations(&target, &bound).is_empty());
}

/// A bound's conditional promise with NO producer at all still violates.
#[test]
fn missing_conditional_out_violates() {
    let bound = contract_cond(&["d0"], &[], &[], &[], &[("a1", "eq")]);
    let target = contract(&["d0"], &[], &[], &[]);
    let v = subcontract_violations(&target, &bound);
    assert!(
        v.iter().any(|s| s.contains("conditional output `a1`")),
        "must name the unproduced conditional result: {v:?}"
    );
}

// ---- IndirectPolicy: the `as Type` bound is TRUSTED, not proven (S12) -------
//
// `jsr (aN) as Type` replaces the ⊤ an unbounded indirect would contribute with
// the contract type's clobber set. The relation that would JUSTIFY that —
// `subcontract_violations`, documented as "what makes a dispatch target
// installable" — has exactly one call site, interface `implement` binding, and is
// never called at a dispatch site. So nothing checks that the procs actually
// installed in the table satisfy the bound (lens sweep, seat Va, finding S12);
// aeon ships a DEBUG-only RUNTIME assert as the workaround.
//
// The narrowing is load-bearing and cannot just be dropped: forcing ⊤ at every
// site produces 53 `[proc.clobber-undeclared]` firings on the corpus. So the
// policy is chosen per consumer, and these tests pin that both readings exist and
// genuinely differ.

use sigil_frontend_emp::closure::{compute_closure_with, IndirectPolicy};

/// The two policies disagree on a BOUNDED site — the narrowing is real, so
/// choosing the wrong policy for a consumer is a real hazard, not a formality.
#[test]
fn bounded_indirect_is_narrowed_only_under_trust() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "Dispatch".to_string(),
        ProcNode { indirect_sites: vec![Some("HBlankHandler".to_string())], ..Default::default() },
    );
    let mut types = BTreeMap::new();
    types.insert("HBlankHandler".to_string(), eff(&["d0", "d1", "a0"]));

    let trusting = compute_closure_with(&procs, &types, IndirectPolicy::TrustTypeBound);
    assert_eq!(trusting.effective["Dispatch"], eff(&["d0", "d1", "a0"]));
    assert!(!trusting.effective["Dispatch"].top, "the bound narrows away from ⊤");

    let sound = compute_closure_with(&procs, &types, IndirectPolicy::Unbounded);
    assert!(
        sound.effective["Dispatch"].top,
        "an unverified bound must read as ⊤ under the sound policy, got {:?}",
        sound.effective["Dispatch"]
    );
}

/// `compute_closure` keeps its historical meaning — the trusting policy — so every
/// existing caller and frozen baseline is unchanged by the split.
#[test]
fn default_closure_still_trusts_the_bound() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "Dispatch".to_string(),
        ProcNode { indirect_sites: vec![Some("HBlankHandler".to_string())], ..Default::default() },
    );
    let mut types = BTreeMap::new();
    types.insert("HBlankHandler".to_string(), eff(&["d0"]));
    assert_eq!(
        compute_closure(&procs, &types).effective["Dispatch"],
        compute_closure_with(&procs, &types, IndirectPolicy::TrustTypeBound).effective["Dispatch"]
    );
}

/// An UNBOUNDED indirect is ⊤ under both policies — the policy only ever concerns
/// the bounded case.
#[test]
fn unbounded_indirect_is_top_under_both_policies() {
    let mut procs = BTreeMap::new();
    procs.insert(
        "Dispatch".to_string(),
        ProcNode { indirect_sites: vec![None], ..Default::default() },
    );
    let types = BTreeMap::new();
    assert!(compute_closure_with(&procs, &types, IndirectPolicy::TrustTypeBound).effective["Dispatch"].top);
    assert!(compute_closure_with(&procs, &types, IndirectPolicy::Unbounded).effective["Dispatch"].top);
}
