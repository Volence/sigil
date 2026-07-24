//! Contract-grammar v2 §1 — the transitive register-effect closure.
//!
//! The per-proc `check_clobbers` lint ([`crate::lower::proc`]) is LOCAL: it sees
//! only a proc's own direct register writes, never its callees' (census caveat
//! 1). v2's upgrade is **transitivity** — each proc's *effective* clobber set
//! over the whole-corpus call graph:
//!
//! ```text
//! effective(P) = localWrites(P)
//!              ∪ ⋃ { effective(C) | C ∈ directCallees(P) }
//!              ∪ ⋃ { bound(S).clobbers | S ∈ indirectSites(P) }
//!              − verifiedPreserved(P)
//! ```
//!
//! (spec `2026-07-17-contract-grammar-v2-design.md` §1). This module is the
//! pure algorithm: a monotone set-union fixpoint from ∅ over a finite lattice,
//! so it terminates even with recursion / SCCs. It is deliberately decoupled
//! from the grammar — it consumes a name-keyed [`ProcNode`] map plus a
//! contract-type bound map, both of which the corpus walk builds from the
//! frontend AST + the shared [`crate::lower::proc_written_registers`] detector
//! (no second write analysis at the link-IR level — the §11 Q2 decision).
//!
//! **`sr` is out of scope here** (§1): interrupt-mask clobbers stay the LOCAL
//! `[proc.sr-undeclared]` check. This closure tracks only the register file
//! `d0`..`a7` — `a7` filtered as stack discipline by the caller, exactly as the
//! census and `check_clobbers` do.

use std::collections::{BTreeMap, BTreeSet};

/// A register-effect lattice element — the set of registers a proc's execution
/// may clobber, seen by the caller. `top` is ⊤ ("all registers"): an unbounded
/// indirect call contributes ⊤ (§1's load-bearing fact — `RunObjects`'s
/// `jsr (a1)` without a bound would poison the whole graph, which is why the
/// §4 indirect bounds ship in G1 with the closure).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct RegEffect {
    /// ⊤ — every register. Once set, unions short-circuit (⊤ ∪ x = ⊤).
    pub top: bool,
    /// The concrete clobbered registers, canonical `d0`..`a7` spellings
    /// (meaningless when `top`) — the same spelling `proc_written_registers`
    /// and `reglist_expand` produce, so no conversion and no drift.
    pub regs: BTreeSet<String>,
}

/// One node in the corpus call graph: a proc's local facts + declared contract,
/// enough for both the closure fixpoint and the firing check. Built by the
/// corpus walk from the frontend AST.
#[derive(Clone, Debug, Default)]
pub struct ProcNode {
    /// Registers this proc writes DIRECTLY (its own body), per the shared
    /// write detector, `a7` stack-discipline already filtered.
    pub local_writes: BTreeSet<String>,
    /// Symbols this proc calls via `jsr`/`jbsr`/`bsr` (resolved by name against
    /// the proc map; a name that is neither a proc nor an extern is a hole).
    pub direct_callees: Vec<String>,
    /// Each indirect call site's declared bound: `Some(type_name)` names a §4
    /// contract type; `None` is an UNBOUNDED indirect call (⊤).
    pub indirect_sites: Vec<Option<String>>,
    /// An `extern proc` leaf (§3): `effective == declared_clobbers`, callees and
    /// indirect sites ignored (the `.asm` body is opaque, its contract trusted).
    pub is_extern: bool,
    /// The declared `clobbers(...)` set (for an extern leaf, its whole effect;
    /// for the firing check, part of `allowed`).
    pub declared_clobbers: BTreeSet<String>,
    /// `params` register bindings — allowed writes (not clobbers).
    pub params: BTreeSet<String>,
    /// `out(...)` results — allowed writes (not clobbers).
    pub out: BTreeSet<String>,
    /// Whether the proc declares any clobber contract at all — the firing check
    /// only runs on procs that opted in (mirrors `check_clobbers`' gate).
    pub has_clobber_contract: bool,
    /// Registers this proc provably PRESERVES — its DECLARED `preserves` set when
    /// that set passes the D2.32 syntactic (movem-pair) verification. Subtracted
    /// from `effective` (§1's `− verifiedPreserved(P)`): a register the proc
    /// writes but saves/restores does not escape it, so it neither fires nor
    /// propagates. The D2.32 slice is EXISTING proof machinery (§5: "the movem
    /// pair remains the trivial fast path") — G3 extends the SAME subtraction to
    /// the individual-push class. A declared-but-UNVERIFIABLE preserves
    /// contributes NOTHING here (it stays a D2.32 error); `sr` is out of the
    /// register-file closure's scope.
    pub verified_preserves: BTreeSet<String>,
}

/// The result of the closure fixpoint.
#[derive(Clone, Debug, Default)]
pub struct Closure {
    /// Each proc's `effective` register-effect.
    pub effective: BTreeMap<String, RegEffect>,
    /// Callee names referenced by some proc that are neither in the proc map nor
    /// an extern declaration — holes in the closure (§1: an undeclared extern
    /// call is a hole, error under strict once G1 lands).
    pub unresolved_callees: BTreeSet<String>,
}

impl RegEffect {
    /// The ⊥ element — no clobbers (the fixpoint seed).
    fn bottom() -> Self {
        RegEffect::default()
    }

    /// Fold another effect in: ⊤ is absorbing (⊤ ∪ x = ⊤).
    fn union_with(&mut self, other: &RegEffect) {
        if self.top {
            return;
        }
        if other.top {
            self.set_top();
            return;
        }
        self.regs.extend(other.regs.iter().cloned());
    }

    /// Fold a raw register set in (a leaf's local writes).
    fn union_regs(&mut self, regs: &BTreeSet<String>) {
        if self.top {
            return;
        }
        self.regs.extend(regs.iter().cloned());
    }

    /// Raise to ⊤.
    fn set_top(&mut self) {
        self.top = true;
        self.regs.clear();
    }
}

/// Resolve a callee NAME to its proc-map key. A plain proc name maps to
/// itself; an `Owner.label` exported-label target (§5.2 — a tail branch INTO
/// a proc's body, e.g. `bra QueueDMA_Deferrable.transfer`, the shared-core
/// idiom) maps to `Owner` when `Owner` is a known proc. Attributing the WHOLE
/// owner's effect to a mid-body entry is a sound over-approximation: the
/// label's tail is a subset of the body whose writes the closure already
/// unions. An unknown owner falls through unchanged and surfaces as a hole.
fn resolve_callee_key<'a>(procs: &BTreeMap<String, ProcNode>, callee: &'a str) -> &'a str {
    if procs.contains_key(callee) {
        return callee;
    }
    if let Some((owner, _label)) = callee.split_once('.') {
        if procs.contains_key(owner) {
            return owner;
        }
    }
    callee
}

/// Compute the transitive `effective` clobber set for every proc (§1). A
/// monotone union fixpoint from ∅; terminates on the finite register lattice.
pub fn compute_closure(
    procs: &BTreeMap<String, ProcNode>,
    contract_types: &BTreeMap<String, RegEffect>,
) -> Closure {
    let mut effective: BTreeMap<String, RegEffect> =
        procs.keys().map(|k| (k.clone(), RegEffect::bottom())).collect();

    // Fixpoint: recompute every proc's effect from its callees' CURRENT effects
    // until nothing grows. Monotone union on a finite lattice → terminates.
    loop {
        let mut changed = false;
        for (name, node) in procs {
            // An extern leaf's effect is fixed at its declared clobbers — the
            // `.asm` body is opaque and its contract is trusted (§3).
            if node.is_extern {
                let mut e = RegEffect::bottom();
                e.union_regs(&node.declared_clobbers);
                if effective[name] != e {
                    effective.insert(name.clone(), e);
                    changed = true;
                }
                continue;
            }

            let mut acc = RegEffect::bottom();
            acc.union_regs(&node.local_writes);
            for callee in &node.direct_callees {
                if let Some(ce) = effective.get(resolve_callee_key(procs, callee)) {
                    acc.union_with(ce);
                }
                // A callee absent from the proc map is a hole — collected
                // once after the fixpoint (it contributes nothing to the
                // union, i.e. treated as ⊥, and is surfaced as unresolved).
            }
            for site in &node.indirect_sites {
                match site {
                    // Unbounded indirect = ⊤ (§1's load-bearing fact).
                    None => {
                        acc.set_top();
                    }
                    Some(ty) => match contract_types.get(ty) {
                        Some(bound) => {
                            acc.union_with(bound);
                        }
                        // A named-but-undefined contract type is conservatively
                        // ⊤ (never silently narrower than the truth).
                        None => {
                            acc.set_top();
                        }
                    },
                }
            }
            // − verifiedPreserved(P) (§1): a register the proc writes but
            // provably preserves (declared + D2.32 movem-verified) does not
            // escape it. ⊤ stays ⊤ (an unbounded indirect can clobber anything,
            // including a "preserved" register, so we cannot subtract from ⊤).
            if !acc.top {
                for r in &node.verified_preserves {
                    acc.regs.remove(r);
                }
            }
            if effective[name] != acc {
                effective.insert(name.clone(), acc);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // Collect holes: direct callees named by some proc that are neither a proc
    // nor an extern in the map (nor an exported label of one — see
    // `resolve_callee_key`).
    let mut unresolved_callees = BTreeSet::new();
    for node in procs.values() {
        for callee in &node.direct_callees {
            if !procs.contains_key(resolve_callee_key(procs, callee)) {
                unresolved_callees.insert(callee.clone());
            }
        }
    }

    Closure { effective, unresolved_callees }
}

/// One transitive-clobber firing: a register in a proc's `effective` set that is
/// not in its declared `clobbers ∪ params ∪ out` (§9, the transitive analog of
/// `[proc.clobber-undeclared]`). `transitive` distinguishes a register the proc
/// writes ITSELF (also caught by the local `check_clobbers`) from one that
/// leaks in only through a callee/indirect site — the interesting new class.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Firing {
    /// The offending proc.
    pub proc: String,
    /// The offending register (canonical spelling), or `None` when the effect is
    /// ⊤ (unbounded) and no single register names the violation.
    pub reg: Option<String>,
    /// `true` when `reg` is NOT among the proc's own `local_writes` — it came
    /// transitively from a callee or an indirect bound (the class the local
    /// lint cannot see).
    pub transitive: bool,
    /// `true` when the proc's effective set is ⊤ (an unbounded indirect leaked
    /// through) yet it declares a bounded `clobbers` contract.
    pub unbounded: bool,
}

/// A proc/contract-type's register partition, for the §4 subcontract relation.
/// Register-name sets (canonical `d0`..`a7`); `sr` out of scope like the closure.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Contract {
    /// Registers destroyed (may clobber).
    pub clobbers: BTreeSet<String>,
    /// Registers left untouched.
    pub preserves: BTreeSet<String>,
    /// Input registers read.
    pub params: BTreeSet<String>,
    /// Result registers written for the caller.
    pub out: BTreeSet<String>,
}

/// The §4 subcontract relation `target ⊑ bound` — what makes a dispatch target
/// installable. Returns a human list of violations (empty ⇒ conforming), for
/// `[dispatch.target-exceeds-bound]`:
///
/// - `target.clobbers ⊆ bound.clobbers` — a target may clobber no MORE than the
///   dispatch site's callers already tolerate;
/// - `target.preserves ⊇ bound.preserves` — it must preserve everything the
///   bound promises callers;
/// - `target.params ⊆ bound.params` — it may READ fewer inputs, never more;
/// - `target.out ⊇ bound.out` — it must PRODUCE everything the caller may read.
pub fn subcontract_violations(target: &Contract, bound: &Contract) -> Vec<String> {
    let mut v = Vec::new();
    for r in target.clobbers.difference(&bound.clobbers) {
        v.push(format!("clobbers `{r}`, which the bound does not permit"));
    }
    for r in bound.preserves.difference(&target.preserves) {
        v.push(format!("does not preserve `{r}`, which the bound requires"));
    }
    for r in target.params.difference(&bound.params) {
        v.push(format!("reads input `{r}`, which the bound does not provide"));
    }
    for r in bound.out.difference(&target.out) {
        v.push(format!("does not produce output `{r}`, which the bound promises callers"));
    }
    v.sort();
    v
}

/// Produce the transitive firing list: for every proc that OPTED IN to a clobber
/// contract (`has_clobber_contract`, mirroring `check_clobbers`' gate), every
/// register in its `effective` set outside `declared_clobbers ∪ params ∪ out`
/// fires (§9). A no-contract proc fires nothing (invisible to the lint until it
/// declares one — the census A2 class). Results are sorted (proc, reg) for a
/// deterministic report.
pub fn check_firings(procs: &BTreeMap<String, ProcNode>, closure: &Closure) -> Vec<Firing> {
    let mut firings = Vec::new();
    for (name, node) in procs {
        // Only procs that opted in to a clobber contract are checked (an extern
        // leaf's contract IS its declared clobbers — nothing to verify).
        if !node.has_clobber_contract || node.is_extern {
            continue;
        }
        let effective = &closure.effective[name];
        // Allowed = declared clobbers ∪ out ONLY. Params are NOT allowed writes:
        // a param declares an INPUT (a register the proc reads), not a licence to
        // destroy it. A proc that genuinely trashes its input register must
        // declare the EFFECT (`clobbers`/`out`, or a verified `preserves` if it
        // round-trips) — otherwise the closure would be blind to exactly the
        // clobbered-input class the caller-side D1c exists to catch. (A param that
        // is only READ never enters `effective`, so it never fires; only a
        // WRITTEN param does, which is correct.)
        let allowed: BTreeSet<&String> =
            node.declared_clobbers.iter().chain(node.out.iter()).collect();
        if effective.top {
            // ⊤ against a bounded contract: one unbounded firing (no single
            // register names the violation).
            firings.push(Firing { proc: name.clone(), reg: None, transitive: true, unbounded: true });
            continue;
        }
        for r in &effective.regs {
            if !allowed.contains(r) {
                firings.push(Firing {
                    proc: name.clone(),
                    reg: Some(r.clone()),
                    transitive: !node.local_writes.contains(r),
                    unbounded: false,
                });
            }
        }
    }
    firings
}
