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
    /// Contexts every call site of this proc must have active (§3.3,
    /// `requires(...)`). Part of the ONE contract record, alongside the register
    /// facets — but NOT a fixpoint input: unlike a clobber set, a context
    /// requirement is DECLARED at every level (§2's signatures-on-exports
    /// discipline), so the propagation rule "a caller inherits its callees'
    /// requirements minus what its own brackets discharge, and the residue must
    /// appear in its own `requires`" is discharged by the per-call-site check in
    /// [`crate::corpus_contracts`], with nothing left to converge.
    pub requires: BTreeSet<String>,
    /// Contexts this proc asserts are active for its whole body (`grants(...)`)
    /// — a TRUST ROOT, never inferred and never verified, only recorded.
    pub grants: BTreeSet<String>,
    /// The S2-D6 U4 escape hatch: the proc declares
    /// `@allow("clobbers.unanalyzable", "<reason>")`. A genuinely-unanalyzable
    /// site — a raw computed dispatch outside the typed-dispatch idiom whose ⊤
    /// effect cannot be narrowed — opts OUT of the `unbounded` firing (the reason
    /// is mandatory; the analysis lists every annotation so the surface stays
    /// audited). Suppresses ONLY the ⊤/unbounded firing, never a concrete
    /// register under-declaration (a named register in the effective set still
    /// fires).
    pub unanalyzable_allowed: bool,
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
    /// Result registers produced on EVERY return path — the UNCONDITIONAL outs
    /// only (see [`crate::ast::ProcDecl::unconditional_outs`]). A conditional
    /// `out(rN if cc)` lands in [`Self::out_cond`], never here: a caller of the
    /// bound may read an unconditional out with no cc test, so a conditional
    /// producer does not satisfy that promise.
    pub out: BTreeSet<String>,
    /// Result registers produced only on their `if cc` edge, each mapped to the
    /// condition codes it is guarded by (canonical — `hs`/`lo` fold onto `cc`/`cs`).
    /// Written by the callee (so they license a clobber like [`Self::out`] does)
    /// but not promised on every return path.
    ///
    /// The condition is part of the promise, not decoration: a register alone
    /// cannot distinguish `out(a1 if eq)` from `out(a1 if ne)`, and a caller that
    /// tests `eq` and reads a1 must not be served by a target that fills a1 only
    /// on `ne`. Multiple codes for one register mean it is produced on any of
    /// those edges.
    pub out_cond: BTreeMap<String, BTreeSet<String>>,
}

/// The §4 subcontract relation `target ⊑ bound` — what makes a dispatch target
/// installable. Returns a human list of violations (empty ⇒ conforming), for
/// `[dispatch.target-exceeds-bound]`:
///
/// - `target.clobbers ⊆ bound.clobbers ∪ bound.out` — a target may destroy no
///   MORE than the dispatch site's callers already tolerate, and a register the
///   bound declares an UNCONDITIONAL result is one those callers already know is
///   written. Without the `out` term the relation is asymmetric with the
///   production test below and rejects the honest declaration: a target spelling
///   `clobbers(d0/a1) out(a1 if eq)` against a bound spelling the same
///   `clobbers(d0/a1)` fires a false `[contract.hook-signature]`.
///
///   `out_cond` is deliberately NOT a term here. Under the normative reading of
///   `out(rN if cc)`, a cond-out register ABSENT from `clobbers` is a claim that
///   rN is PRESERVED on every ¬cc return path — and `Contract` encodes that claim
///   purely by absence from `clobbers`. Licensing a clobber off `out_cond` would
///   erase it: a bound spelling `clobbers(d0) out(a1 if eq)` entitles its callers
///   to hold a1 across the call and re-read it on the ne edge, and a target
///   spelling `clobbers(d0/a1) out(a1 if eq)` leaves a1 indeterminate there. That
///   pair must violate;
/// - `target.preserves ⊇ bound.preserves` — it must preserve everything the
///   bound promises callers;
/// - `target.params ⊆ bound.params` — it may READ fewer inputs, never more;
/// - `bound.out ⊆ target.out` — it must produce UNCONDITIONALLY everything the
///   bound promises unconditionally. A conditional producer does NOT satisfy an
///   unconditional promise: the bound's callers may read the register with no cc
///   test at all;
/// - each `(rN, ccs)` the bound promises conditionally is satisfied by an
///   UNCONDITIONAL `rN` in the target (producing on every path is strictly
///   stronger, whatever the codes) or by a conditional `rN` whose own code set
///   COVERS `ccs` — every edge the bound promises must be an edge the target
///   produces on. A target guarding a different code produces a register the
///   bound's callers will read on the wrong edge, so it violates.
///
///   Codes compare by canonical EQUALITY, not by implication: `eq` implies `le`
///   on 68k, so a target guarding `le` does in fact satisfy a bound promising
///   `eq`, and this relation rejects that pair. The rejection is the SAFE
///   polarity — the author's remedy is to spell the bound's own code, the same
///   escape the survives verifier relies on — and an implication lattice is a
///   per-CPU flag-semantics table with no corpus demand behind it.
pub fn subcontract_violations(target: &Contract, bound: &Contract) -> Vec<String> {
    let mut v = Vec::new();
    let writable: BTreeSet<&String> = bound.clobbers.iter().chain(&bound.out).collect();
    for r in target.clobbers.iter().filter(|r| !writable.contains(r)) {
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
    for (r, want) in &bound.out_cond {
        // An unconditional producer satisfies any conditional promise — it fills
        // the register on every return edge, so it covers the bound's edge too.
        if target.out.contains(r) {
            continue;
        }
        let Some(have) = target.out_cond.get(r) else {
            v.push(format!(
                "does not produce conditional output `{r}`, which the bound promises callers"
            ));
            continue;
        };
        let missing: Vec<&str> = want.difference(have).map(String::as_str).collect();
        if !missing.is_empty() {
            v.push(format!(
                "produces conditional output `{r}` on `{}`, not on `{}`, which the bound \
                 promises callers",
                have.iter().map(String::as_str).collect::<Vec<_>>().join("/"),
                missing.join("/"),
            ));
        }
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
            // register names the violation) — UNLESS the proc opted out via the
            // U4 escape hatch `@allow("clobbers.unanalyzable", "<reason>")`. The
            // hatch suppresses ONLY this ⊤/unbounded case (a genuinely
            // unanalyzable computed dispatch); a concrete register leak is never
            // ⊤, so it is never silenced here.
            if !node.unanalyzable_allowed {
                firings.push(Firing {
                    proc: name.clone(),
                    reg: None,
                    transitive: true,
                    unbounded: true,
                });
            }
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
