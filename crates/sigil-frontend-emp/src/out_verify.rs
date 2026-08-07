//! Contract-grammar v2 §G4.5 — verified `out()` by symbolic production tracking.
//!
//! The callee-side dual of `preserves` (§5): a proc declaring `out(rN)` must
//! PRODUCE rN on every required return path, where "produce" means rN holds a
//! full-width value written on THIS pass. This is a forward MUST-produce dataflow
//! over the SAME lightweight CFG G2 ([`crate::flag_check::Cfg`]) `preserves` and
//! `calls` already share — modeled on `preserves::verify_preserved` (the true
//! structural dual), NOT on `must_defined_in` (which is width-blind and
//! param-seeded, both unsound for out-honesty):
//!
//! - **Width-aware** (Finding 1): only a FULL-WIDTH write produces. For a DATA
//!   register that is a `.l` write or a `moveq` (all 32 bits); a `.w`/`.b` write
//!   leaves the high word stale and does NOT verify (exactly `preserves`'s
//!   `is_long` rule). For an ADDRESS register every write/advance is full-width
//!   (68k address writes touch all 32 bits — `movea.w` sign-extends, `(aN)+`
//!   advances the pointer), so any address-register write/advance produces it.
//! - **No param seed** (Finding 2): entry state credits NOTHING; a production
//!   must come from a write / callee-out / tail-out on the path. An `out(rN)`
//!   where rN is P's own param but is never re-written FIRES (a mislabeled
//!   `preserves`); a cursor `out(a4)` un-advanced on an early-exit path FIRES.
//! - **Callee-out credit** at a `jsr`/`jbsr`/`bsr` via the SHARED
//!   `callee_uncond_out` map — the Load_Object←AllocDynamic shape. A callee's
//!   CONDITIONAL `out(rN if cc)` is credited edge-sensitively via the shared
//!   `conditional_out_edge_credits` primitive (item #2) — only on the caller's
//!   cc-success edge, so a proc whose out is sourced from a relabeled conditional
//!   callee (Load_Object←`AllocDynamic out(a1 if eq)`) still verifies.
//! - **Tail-out credit** at an `Edge::TailOut` (Finding 3): a tail transfer is a
//!   return of P from the caller's view, so it is a required return path; if the
//!   target is a known proc, credit its unconditional out, else the out cannot be
//!   verified ⇒ FIRE. An `Edge::BranchOut` is not a required return path and is
//!   skipped — the edge builder decides which of the two an exit is, so this
//!   module holds no mnemonic table of its own.
//!
//! Soundness polarity: a dishonest out ⇒ must-def falsely credits rN defined ⇒
//! D1b false NEGATIVE (the dangerous direction). So the verifier only blesses a
//! PROVEN full-width production; when in doubt it FIRES. This is a MUST analysis
//! (produced-on-all-required-paths = intersection join).
//!
//! **Property boundary** (Finding 5): this proves rN holds a full-width value
//! produced on this pass, NOT that the value is semantically correct — a proc that
//! produces rN then stomps it before `rts` still verifies (the stomp is itself a
//! production). Value-provenance is out of scope.
//!
//! # The other half of `out(rN if cc)` — the SURVIVES claim
//!
//! A conditional out is a TWO-part contract (delta spec §7.1). Production on the
//! cc edges is the half above. The ¬cc edges are governed by `clobbers`
//! membership:
//!
//! - rN **∈ `clobbers`** — no claim; rN is indeterminate on the ¬cc edges and
//!   callers save on every path (the `AllocDynamic` shape).
//! - rN **∉ `clobbers`** — rN is PRESERVED (entry value intact) on every ¬cc
//!   return path (the `AllocEffect` shape). [`check_cond_out_survives`] proves
//!   that claim, and `[proc.out-cond-survives-unverifiable]` is the failure.
//!
//! Both halves read ONE cc classification ([`flags_after`]), so they can never
//! disagree about which edge a return sits on. They are NOT complements at ⊤, and
//! deliberately so: production is obligated on `Some(true)` **and ⊤**, survival
//! only on `Some(false)`. An unclassifiable return is charged the obligation whose
//! escape is honest (produce the register) and spared the one whose escape would
//! force a false declaration — see [`not_cc_exit_sites`] for the measurement
//! behind that asymmetry.
//!
//! The proof itself is
//! [`preserves::verify_preserved_on`](crate::preserves::verify_preserved_on)
//! SCOPED to the ¬cc exits — the same save/restore round-trip, never-written and
//! callee-preserves-oracle proofs `preserves(rN)` accepts, with no second
//! dataflow.

use crate::flag_check::{conditional_out_edge_credits, Cfg, Edge};
use crate::lower::instr_written_regs;
use crate::preserves::{verify_preserved_on, CallPolicy, Facet, PreserveStatus, ReturnScope};
use crate::value::{CodeItem, CodeOperand, Reg, Width};
use sigil_span::Span;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Proc name → its set of UNCONDITIONAL out registers (canonical `d0`..`a7`
/// spellings). Used for both the DECLARED map and the fixpoint's VERIFIED map.
pub type UncondOutMap = BTreeMap<String, BTreeSet<String>>;
/// Proc name → its `(reg, cc)` CONDITIONAL outs. Used for both the DECLARED map
/// and the fixpoint's VERIFIED map.
pub type CondOutMap = BTreeMap<String, Vec<(String, String)>>;

/// The proof outcome for one declared output register.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutStatus {
    /// Proven produced on every required return path.
    Produced,
    /// Some required return path does not produce it (a false `out()` claim).
    Unverified(String),
}

/// One `[proc.out-unverified]` firing: proc `proc` declares `out(reg)` but the
/// body does not PRODUCE it on every required return path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutFiring {
    pub proc: String,
    pub reg: String,
    pub reason: String,
    pub span: Span,
}

/// Run the `out()` production check for a proc and collect its `[proc.out-
/// unverified]` firings. `uncond` are the unconditional outs, `cond` the
/// `(reg, cc)` conditional outs; `callee_uncond_out` is the SHARED map (callee /
/// tail-target credit).
pub fn check_out(
    proc_name: &str,
    items: &[CodeItem],
    uncond: &[Reg],
    cond: &[(Reg, String)],
    callee_uncond_out: &BTreeMap<String, BTreeSet<String>>,
    cond_callees: &BTreeMap<String, Vec<(String, String)>>,
    span: Span,
    charge_fall_off_end: bool,
    noreturn: &BTreeSet<String>,
) -> Vec<OutFiring> {
    verify_out(items, uncond, cond, callee_uncond_out, cond_callees, charge_fall_off_end, noreturn)
        .into_iter()
        .filter_map(|(r, status)| match status {
            OutStatus::Produced => None,
            OutStatus::Unverified(reason) => Some(OutFiring {
                proc: proc_name.to_string(),
                reg: r.to_string(),
                reason,
                span,
            }),
        })
        .collect()
}

fn reg_idx(r: Reg) -> usize {
    r as usize
}

/// A data register is `d0`..`d7` (index 0..7); an address register is `a0`..`a7`
/// (index 8..15). Only address writes are inherently full-width.
fn is_addr_reg(r: Reg) -> bool {
    reg_idx(r) >= 8
}

fn is_call(mnem: &str) -> bool {
    matches!(mnem, "jsr" | "jbsr" | "bsr")
}

/// The registers instruction `(mnem, ops, size)` PRODUCES at full width. Built on
/// the shared [`instr_written_regs`] detector (dest register + auto-inc/dec
/// bases), then width-filtered: an address register is always full-width; a data
/// register only via a `.l` write or a `moveq`. A `.w`/`.b` data write is dropped
/// (Finding 1).
///
/// **movem LOAD (item #2 cascade growth).** A `movem` whose register list is the
/// DESTINATION — the LAST operand, e.g. `movem.l (sp)+, d0-d2/a1` — writes every
/// listed register at FULL WIDTH and so PRODUCES each of them, for BOTH sizes:
/// `movem.l` writes 32 bits and `movem.w` SIGN-EXTENDS each word to 32 bits (all
/// registers, data and address alike — unlike a plain `move.w`). So the reglist
/// credit is exempt from the data-reg `.l`/`moveq` width filter. This is the same
/// growth [`crate::calls`]'s `written_names` already applies for must-def, and it
/// is what makes a caller's `movem (sp)+, …aN` restore of a saved out-register
/// verify (the Load_Object alloc-fail path — a1 restored on failure). A movem
/// STORE (`movem.l d0-d2/a1, -(sp)`, reglist FIRST = source) is NOT a production:
/// its `ops.last()` is the `-(sp)` predec, not a `RegList`, so it falls through to
/// the base-advance path exactly as it must (it READS the reglist).
fn produced_regs(mnem: &str, ops: &[CodeOperand], size: Option<Width>) -> Vec<Reg> {
    if mnem == "movem" {
        if let Some(CodeOperand::RegList(mask)) = ops.last() {
            // Full-width for every listed register (both sizes) — plus the
            // auto-inc/dec base advance the detector reports for `(aN)+`/`-(aN)`.
            let mut regs = crate::preserves::expand_mask(*mask);
            for r in instr_written_regs(mnem, ops) {
                if !regs.contains(&r) {
                    regs.push(r);
                }
            }
            return regs;
        }
    }
    let data_full_width = size == Some(Width::L) || mnem == "moveq";
    instr_written_regs(mnem, ops)
        .into_iter()
        .filter(|r| is_addr_reg(*r) || data_full_width)
        .collect()
}

/// The abstract state at a program point: which registers are MUST-produced on
/// every path here. The cc classification lives in its own dataflow
/// ([`flags_after`]), which both halves of `out(rN if cc)` read.
type State = [bool; 16];

/// The bare `Sym` target of a direct call/tail (`jbsr Foo` / `jbra Foo`), or
/// `None` for an indirect / local-label (`$`-mangled) target.
fn direct_target(ops: &[CodeOperand]) -> Option<&str> {
    match ops {
        [CodeOperand::Sym(name)] if !name.contains('$') => Some(name.as_str()),
        _ => None,
    }
}

/// Apply instruction `idx`'s effect to `st`: gen full-width productions and
/// credit a call's callee unconditional outs.
fn transfer(
    cfg: &Cfg,
    idx: usize,
    st: &mut State,
    items: &[CodeItem],
    callee_uncond_out: &BTreeMap<String, BTreeSet<String>>,
) {
    let Some((mnem, ops)) = cfg.instr(idx) else { return };
    let size = match items.get(idx) {
        Some(CodeItem::Instr { size, .. }) => *size,
        _ => None,
    };

    if is_call(mnem) {
        // A returning call: credit its UNCONDITIONAL out registers as produced
        // (the shared map).
        if let Some(target) = direct_target(ops) {
            if let Some(outs) = callee_uncond_out.get(target) {
                for name in outs {
                    if let Some(r) = Reg::from_name(name) {
                        st[reg_idx(r)] = true;
                    }
                }
            }
        }
        return;
    }

    // Production is gen-only (a value produced upstream stays produced —
    // Finding 5; a later partial write does not un-produce).
    for r in produced_regs(mnem, ops, size) {
        st[reg_idx(r)] = true;
    }
}

/// Verify each declared output register of a proc over its evaluated CodeBuf.
/// `uncond` are the unconditional outs; `cond` are `(reg, cc)` conditional outs
/// (obligated only on the cc-success return paths — an UNKNOWN cc keeps the
/// obligation).
pub fn verify_out(
    items: &[CodeItem],
    uncond: &[Reg],
    cond: &[(Reg, String)],
    callee_uncond_out: &BTreeMap<String, BTreeSet<String>>,
    cond_callees: &BTreeMap<String, Vec<(String, String)>>,
    charge_fall_off_end: bool,
    noreturn: &BTreeSet<String>,
) -> BTreeMap<Reg, OutStatus> {
    let cfg = Cfg::build(items);

    // Item #2: a callee's CONDITIONAL `out(rN if cc)` is credited as PRODUCED
    // only on the caller's provably-cc-success edge (the shared edge-ID primitive,
    // §4). The edge-blind `callee_uncond_out` credit in `transfer` covers plain
    // `out(rN)`; this covers the Load_Object←AllocDynamic-relabeled cascade, where
    // AllocDynamic's `out(a1 if eq)` produces a1 on Load_Object's `bne .alloc_fail`
    // fall-through (the eq-success edge). Applied as a per-edge transfer that
    // re-joins by intersection at merges — NOT a global post-call fact (§3).
    let edge_credit = conditional_out_edge_credits(&cfg, items, cond_callees);
    // The cc classification at every return — the SHARED map the ¬cc survives
    // obligation reads too, so the two halves of `out(rN if cc)` partition the
    // return paths by one predicate.
    let flags = flags_after(&cfg, items);

    // The condition guarding each checked register: `None` = unconditional
    // (obligated on every return), `Some(cc)` = conditional (obligated only where
    // cc is not provably false).
    let mut guard: BTreeMap<Reg, Option<String>> = BTreeMap::new();
    for r in uncond {
        guard.insert(*r, None);
    }
    for (r, cc) in cond {
        guard.insert(*r, Some(cc.clone()));
    }

    let Some(entry_idx) = items.iter().position(|it| matches!(it, CodeItem::Instr { .. })) else {
        // No body → every out is vacuously produced (nothing to disprove).
        return guard.keys().map(|r| (*r, OutStatus::Produced)).collect();
    };

    let mut in_state: BTreeMap<usize, State> = BTreeMap::new();
    in_state.insert(entry_idx, [false; 16]);
    let mut work: VecDeque<usize> = VecDeque::from([entry_idx]);

    // Per checked register: produced on EVERY required return path so far, and
    // the reason of the first failing return (for the diagnostic).
    let mut ok: BTreeMap<Reg, bool> = guard.keys().map(|r| (*r, true)).collect();
    let mut fail_reason: BTreeMap<Reg, String> = BTreeMap::new();

    while let Some(idx) = work.pop_front() {
        let mut st = in_state[&idx];
        transfer(&cfg, idx, &mut st, items, callee_uncond_out);
        let here = flags.get(&idx).copied().unwrap_or(Flags::TOP);

        for edge in cfg.edges(idx) {
            match edge {
                Edge::Follow(succ) => {
                    let mut edge_st = st;
                    // Item #2: credit a callee's conditional out on THIS success
                    // edge only (per-edge transfer; the join below re-intersects).
                    if let Some(regs) = edge_credit.get(&(idx, succ)) {
                        for name in regs {
                            if let Some(r) = Reg::from_name(name) {
                                edge_st[reg_idx(r)] = true;
                            }
                        }
                    }
                    let changed = match in_state.get(&succ) {
                        None => {
                            in_state.insert(succ, edge_st);
                            true
                        }
                        Some(existing) => {
                            let merged = join(existing, &edge_st);
                            if merged != *existing {
                                in_state.insert(succ, merged);
                                true
                            } else {
                                false
                            }
                        }
                    };
                    if changed {
                        work.push_back(succ);
                    }
                }
                // A return: the caller reads the register file from here, so the
                // claim is due, with no extra credit on the way out.
                Edge::Return => {
                    check_return(&st, &here, &guard, &mut ok, &mut fail_reason);
                }
                // Control running off the end is a required return path ONLY when
                // the proc actually returns there. A declared `falls_into
                // Successor` continues into its successor INSIDE THE SAME CALL,
                // and the successor is what produces the value — so charging the
                // claim here would demand production from a proc that never
                // returns. The stack checker draws the same line, and its caller
                // computes the same flag: `charge_fall_off_end =
                // proc.falls_into.is_none()` into `check_stack_balance`.
                Edge::FallOff => {
                    if charge_fall_off_end {
                        check_return(&st, &here, &guard, &mut ok, &mut fail_reason);
                    }
                }
                // A conditional branch out (a divergent handler or a transitive
                // tail) is not a local counterexample — ignore it, mirroring
                // `preserves`. The edge states which flavor this is; the mnemonic
                // is not consulted.
                Edge::BranchOut => {}
                Edge::TailOut => {
                    // ...UNLESS it DIVERGES. An `@noreturn` target or an
                    // `AssertDesugar`-authored raise rail never returns to P's
                    // caller, so it is not a return path and carries no out
                    // obligation. Same predicate `preserves` uses — one place, so a
                    // checker cannot know half the successor story.
                    if crate::preserves::exit_diverges(&items[idx], noreturn) {
                        continue;
                    }
                    // An UNCONDITIONAL tail transfer is a required return path
                    // (Finding 3): control leaves P for the target, which returns
                    // to P's caller.
                    let Some((_, ops)) = cfg.instr(idx) else { continue };
                    // Credit the tail target's UNCONDITIONAL out (a known proc
                    // producing rN); an external / unresolved target credits
                    // nothing ⇒ any un-produced out fails here.
                    let mut credit = st;
                    if let Some(target) = direct_target(ops) {
                        if let Some(outs) = callee_uncond_out.get(target) {
                            for name in outs {
                                if let Some(r) = Reg::from_name(name) {
                                    credit[reg_idx(r)] = true;
                                }
                            }
                        }
                    }
                    check_return(&credit, &here, &guard, &mut ok, &mut fail_reason);
                }
            }
        }
    }

    guard
        .keys()
        .map(|r| {
            let status = if ok[r] {
                OutStatus::Produced
            } else {
                OutStatus::Unverified(
                    fail_reason.get(r).cloned().unwrap_or_else(|| "not produced".to_string()),
                )
            };
            (*r, status)
        })
        .collect()
}

/// The VERIFIED-out fixpoint (contract-grammar v2 §G4.5, the D1b-flip foundation).
///
/// A proc's `out(rN)` is VERIFIED iff [`verify_out`] proves rN PRODUCED on every
/// required return path when callee/tail credit is drawn ONLY from
/// already-VERIFIED outs. Computed as a least-fixpoint: extern procs seed VERIFIED
/// (their declared outs are §3 boundary AXIOMS — there is no body to check, and
/// the verifier deliberately does not reach across the `.asm` link seam; an
/// extern's out-honesty is its twin's contract, gated elsewhere); body procs start
/// ⊥ (nothing verified) and grow until stable.
///
/// **Monotone ⇒ terminates.** More verified credit can only turn an out
/// Unverified→Produced (the credit only adds `produced` bits in
/// [`verify_out`]'s `transfer`), never the reverse — so each proc's verified set
/// grows monotonically, bounded by its declared outs (finite). A round that adds
/// nothing ends the loop; a hard round-cap `assert!` turns any monotonicity
/// regression into a loud panic rather than a hang.
///
/// A mutual/circular out-dependency that never grounds in a LOCAL production stays
/// UNVERIFIED — the correct conservative answer (Finding 2), not an error.
///
/// Returns `(verified_uncond, verified_cond)` — the maps the caller-side
/// DEFINITION gates must credit INSTEAD of the raw DECLARED maps: D1b must-def (an
/// out is a reaching definition of a callee param) and the out-verify residue
/// surface. An out is trusted as a definition only once PROVEN honest, so the
/// FindStagedBlock existence-lie can no longer silently satisfy a must-def input.
/// REDEFINE-excuse consumers (§6 taint-kill, D1c held-value) keep the DECLARED
/// maps — a width-unverified out genuinely redefines its register.
pub fn compute_verified_outs(
    proc_items: &BTreeMap<String, &[CodeItem]>,
    declared_uncond: &UncondOutMap,
    declared_cond: &CondOutMap,
    extern_names: &BTreeSet<String>,
    falls_into: &BTreeSet<String>,
    noreturn: &BTreeSet<String>,
) -> (UncondOutMap, CondOutMap) {
    // SEED: externs verified by axiom; every other proc empty.
    let mut v_uncond: BTreeMap<String, BTreeSet<String>> = declared_uncond
        .iter()
        .map(|(name, outs)| {
            let seed = if extern_names.contains(name) { outs.clone() } else { BTreeSet::new() };
            (name.clone(), seed)
        })
        .collect();
    let mut v_cond: BTreeMap<String, Vec<(String, String)>> = declared_cond
        .iter()
        .filter(|(name, _)| extern_names.contains(*name))
        .map(|(name, conds)| (name.clone(), conds.clone()))
        .collect();

    // Round-cap (R1): each round that CHANGES the maps adds ≥1 verified fact
    // (monotone growth); a stable round ends the loop. So the total declared-fact
    // count + 2 is a strict upper bound on rounds — exceeding it is a
    // monotonicity regression, not a tuning knob.
    let total_facts: usize = declared_uncond.values().map(|s| s.len()).sum::<usize>()
        + declared_cond.values().map(|c| c.len()).sum::<usize>();
    let cap = total_facts + 2;

    let mut round = 0usize;
    loop {
        let mut changed = false;
        for (name, items) in proc_items {
            let uncond: Vec<Reg> = declared_uncond
                .get(name)
                .into_iter()
                .flatten()
                .filter_map(|r| Reg::from_name(r))
                .collect();
            let cond: Vec<(Reg, String)> = declared_cond
                .get(name)
                .into_iter()
                .flatten()
                .filter_map(|(r, cc)| Reg::from_name(r).map(|x| (x, cc.clone())))
                .collect();
            if uncond.is_empty() && cond.is_empty() {
                continue;
            }
            let statuses =
                verify_out(items, &uncond, &cond, &v_uncond, &v_cond, !falls_into.contains(name), noreturn);
            let unver: BTreeSet<String> = statuses
                .iter()
                .filter(|(_, s)| matches!(s, OutStatus::Unverified(_)))
                .map(|(r, _)| r.to_string())
                .collect();
            let new_u: BTreeSet<String> =
                uncond.iter().map(|r| r.to_string()).filter(|r| !unver.contains(r)).collect();
            if v_uncond.get(name) != Some(&new_u) {
                v_uncond.insert(name.clone(), new_u);
                changed = true;
            }
            let new_c: Vec<(String, String)> = cond
                .iter()
                .filter(|(r, _)| !unver.contains(&r.to_string()))
                .map(|(r, cc)| (r.to_string(), cc.clone()))
                .collect();
            // Compare against absent≡empty (the scratch-experiment bug: `None !=
            // Some(&empty)` flips `changed` every round for a proc with no cond
            // outs → non-termination).
            let prev_c = v_cond.get(name).cloned().unwrap_or_default();
            if prev_c != new_c {
                if new_c.is_empty() {
                    v_cond.remove(name);
                } else {
                    v_cond.insert(name.clone(), new_c);
                }
                changed = true;
            }
        }
        if !changed {
            break;
        }
        round += 1;
        assert!(
            round <= cap,
            "verified-out fixpoint did not stabilize in {cap} rounds — monotonicity regression"
        );
    }
    (v_uncond, v_cond)
}

// ===========================================================================
// The ¬cc half of `out(rN if cc)` — the SURVIVES claim (delta spec §7.1/§7.2).
// ===========================================================================

/// One `[proc.out-cond-survives-unverifiable]` firing: `proc` declares
/// `out(reg if cc)` with `reg` ABSENT from its `clobbers` set — which claims
/// `reg` still holds its ENTRY value on every ¬cc return path — but the proof
/// does not carry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CondOutSurvivesFiring {
    pub proc: String,
    pub reg: String,
    pub cc: String,
    pub reason: String,
    pub span: Span,
}

/// The instruction indices that END a ¬cc return path of this body: an `rts` /
/// fall-off-end, or an UNCONDITIONAL tail transfer (a return of P from the
/// caller's view — [`verify_out`]'s Finding 3, applied to the dual), at which the
/// shared cc classification does NOT prove `cc`.
///
/// **⊤ DOES NOT OBLIGATE, and that asymmetry against [`verify_out`] is
/// deliberate.** A return the cc lattice cannot classify is EXCLUDED.
///
/// The two halves look like mirror images, so the mirror ruling — fire at ⊤, as
/// `[proc.preserves-unverifiable]` fires on a proof bailout — is the tempting
/// one. It is wrong here, because the two ⊤s are not the same thing. A
/// `preserves` bailout means "your claim was checked and the machinery gave up";
/// a ⊤ cc means "I cannot tell whether you made a claim at this return at all".
/// Charging the second is charging an obligation the author never incurred — and
/// specifically, a ⊤ return may BE the cc-success return, where writing `reg` is
/// the contract. Measured on honest bodies: an ordinary `clr.w d0` or
/// `move.w #0, d0` result convention, a store, or any call on the success path
/// sends the flags to ⊤ and would reject a contract that is true. The escape (add
/// `reg` to `clobbers`) is no escape there — it forces the author to publish a
/// WEAKER, FALSE claim, the exact polarity delta spec §7.2 rejects in the other
/// direction ("the house never forces a weaker claim to be sharpened").
///
/// [`verify_out`] keeps ITS obligation at ⊤ because its escape genuinely is free
/// and honest: produce the register. Same lattice, opposite correct answer.
///
/// **The cost is a false NEGATIVE**, bounded and named: a survives claim whose
/// ¬cc returns are all unclassifiable goes unchecked rather than misjudged. Every
/// return that ends in the corpus's `moveq #0`/`moveq #1` Z-result convention IS
/// classified, which is why both claim sites are fully checked today. Widening
/// [`Flags::after`] (`clr`, `move #imm` — `branch_const::const_flag_writer`
/// already folds them) shrinks this set monotonically and can only add checking.
///
/// An EXIT is any edge that ends the proc — a `Return`, a `FallOff`, or a
/// transfer out in either flavor (`TailOut`/`BranchOut`). The survives claim is a
/// promise to the CALLER, so it must hold wherever control leaves, not only where
/// it returns; a `bne ErrorPath` out of the ¬cc edge would otherwise be a hole.
/// What the transfer's TARGET does is not charged here — see the transfer-out
/// arm in [`crate::preserves`].
///
/// Absence from `flags` (an unreachable index) contributes no site, which agrees
/// with the ⊤ rule above: unclassified means unobligated.
fn not_cc_exit_sites(cfg: &Cfg, flags: &BTreeMap<usize, Flags>, cc: &str) -> BTreeSet<usize> {
    let mut sites = BTreeSet::new();
    for (&idx, f) in flags {
        if f.eval(cc) != Some(false) {
            continue; // not a PROVABLY-¬cc exit — no survives obligation here
        }
        let leaves = cfg
            .edges(idx)
            .iter()
            .any(|e| !matches!(e, Edge::Follow(_)));
        if leaves {
            sites.insert(idx);
        }
    }
    sites
}

/// Verify the SURVIVES half of every conditional out this proc declares without
/// also declaring the register clobbered (delta spec §7.1). `cond` are the
/// `(reg, cc)` pairs; `clobbers` is the proc's CANONICAL declared clobber set —
/// a register in it makes no claim and is skipped entirely (the `AllocDynamic`
/// shape). `policy` is the call model: the per-file gate has no callee knowledge
/// (`ClobberAll` + a `PreserveAll` deferral probe), the corpus walk supplies the
/// closure's verified `Oracle` so a preserving call does not kill the proof.
///
/// The proof is [`verify_preserved_on`] SCOPED to [`not_cc_exit_sites`] — the
/// same save/restore round-trip, never-written and callee-preserves proofs
/// `preserves(rN)` accepts, obligated on the ¬cc exits only. Scoping is what
/// separates this from a plain `preserves(rN)`: `reg` IS deliberately written on
/// the cc edge, so an all-paths proof would reject every honest survives-claim.
pub fn check_cond_out_survives(
    proc_name: &str,
    items: &[CodeItem],
    cond: &[(Reg, String)],
    clobbers: &BTreeSet<String>,
    policy: CallPolicy,
    span: Span,
) -> Vec<CondOutSurvivesFiring> {
    let cfg = Cfg::build(items);
    // One classification for every checked register — the cc a claim names varies,
    // the abstract flag state at each return does not.
    let flags = flags_after(&cfg, items);
    let mut firings = Vec::new();
    for (reg, cc) in cond {
        if clobbers.contains(&reg.to_string()) {
            continue; // declared destroyed on every edge — no claim to check
        }
        let sites = not_cc_exit_sites(&cfg, &flags, cc);
        if sites.is_empty() {
            continue; // no PROVABLY-¬cc exit — nothing this analysis can judge
        }
        // The survives-claim charges its named `Sites` at raw entry bits — the tail
        // credit (a `preserves`-contract concept) does not apply, so the
        // `falls_into`/`@noreturn` context is empty here.
        let status = verify_preserved_on(
            items,
            &[*reg],
            policy,
            ReturnScope::Sites(&sites),
            Facet::Full,
            None,
            &BTreeSet::new(),
        );
        // Only a PROVEN entry-value round-trip clears the claim; every other
        // outcome fires. The polarity is `check_preserves`': absence of a positive
        // proof is the failure, never a pass.
        let reason = match status.get(reg) {
            Some(PreserveStatus::Verified) => continue,
            Some(PreserveStatus::NotPreserved) => format!(
                "`{reg}` does not provably hold its entry value on a `!{cc}` exit \
                 (written and not restored, or destroyed by a call / tail transfer there)"
            ),
            Some(PreserveStatus::Unverifiable(why)) => {
                format!("the entry-value proof bailed: {why}")
            }
            None => format!("no entry-value proof was produced for `{reg}`"),
        };
        firings.push(CondOutSurvivesFiring {
            proc: proc_name.to_string(),
            reg: reg.to_string(),
            cc: cc.clone(),
            reason,
            span,
        });
    }
    firings
}

/// The `[proc.out-cond-survives-unverifiable]` message for one firing — the ONE
/// text the per-file gate and the whole-corpus gate both report, so an author
/// meets the same diagnosis and the same remedy from either.
pub fn survives_message(f: &CondOutSurvivesFiring) -> String {
    format!(
        "[proc.out-cond-survives-unverifiable] `{}` declares `out({} if {})` with `{}` absent \
         from `clobbers(...)`, which claims `{}` still holds its entry value on every `!{}` \
         return path — but {}. Add `{}` to `clobbers(...)` if it is destroyed on the failure \
         edges (the conditional result stands either way), or save/restore it across them",
        f.proc, f.reg, f.cc, f.reg, f.reg, f.cc, f.reason, f.reg,
    )
}

/// At one return, charge every checked register whose obligation applies here but
/// whose value is not in `produced`.
fn check_return(
    produced: &[bool; 16],
    flags: &Flags,
    guard: &BTreeMap<Reg, Option<String>>,
    ok: &mut BTreeMap<Reg, bool>,
    fail_reason: &mut BTreeMap<Reg, String>,
) {
    for (r, cc) in guard {
        // A conditional out is obligated only where cc is not PROVABLY false; an
        // unknown (⊤) cc keeps the obligation (false-positive-leaning = sound).
        if let Some(cc) = cc {
            if flags.eval(cc) == Some(false) {
                continue; // this return is on the !cc edge — no obligation
            }
        }
        if !produced[reg_idx(*r)] {
            *ok.get_mut(r).unwrap() = false;
            fail_reason.entry(*r).or_insert_with(|| {
                format!("`{r}` not produced on a required return path")
            });
        }
    }
}

/// The abstract flags along the `idx → succ` edge, branch-split refined: if
/// `idx` is a SIMPLE conditional branch (`bXX`), the tested cc is provably TRUE
/// on the taken edge and FALSE on the fall-through. A `dbcc`, an unconditional
/// tail, or a composite/unknown cc refines nothing (sound). The degenerate
/// branch-to-fall-through case cannot be split and stays ⊤-safe.
fn split_cc(cfg: &Cfg, idx: usize, succ: usize, flags: Flags) -> Flags {
    let Some((mnem, ops)) = cfg.instr(idx) else { return flags };
    let Some(cc) = simple_branch_cond(mnem) else { return flags };
    let fallthrough = cfg.next_instr(idx);
    let taken = ops
        .iter()
        .rev()
        .find_map(|o| match o {
            CodeOperand::Sym(name) => Some(name.as_str()),
            _ => None,
        })
        .and_then(|t| cfg.label_index(t));
    // Branch to the fall-through instruction — the two edges coincide, can't
    // split.
    if Some(succ) == fallthrough && taken == fallthrough {
        return flags;
    }
    let holds = Some(succ) != fallthrough; // the taken edge iff not the fall-through
    flags.refine(cc, holds)
}

/// The abstract condition-code state AFTER each reachable instruction — the ONE
/// cc classification both halves of `out(rN if cc)` read (delta spec §7.1).
/// [`verify_out`] consults it to skip the PRODUCTION obligation on a provably-¬cc
/// return; [`not_cc_exit_sites`] consults it to select the exits the SURVIVES
/// obligation applies to. Sharing the map is what stops the two obligations from
/// becoming drifting approximations of "which edge is this" — they disagree only
/// where they are MEANT to, at ⊤ (see [`not_cc_exit_sites`]).
///
/// A forward fixpoint over the shared CFG, joining by [`Flags::meet`]: a call
/// clobbers to ⊤, [`Flags::after`] handles straight-line instructions, and
/// [`split_cc`] refines a simple conditional branch's two edges. It stands apart
/// from [`verify_out`]'s production fixpoint because the two never read each
/// other — no flag transfer consults a production bit and no production
/// transfer consults a flag — so the pair is a product of independent
/// lattices and solving them separately gives the identical answer.
fn flags_after(cfg: &Cfg, items: &[CodeItem]) -> BTreeMap<usize, Flags> {
    let mut after: BTreeMap<usize, Flags> = BTreeMap::new();
    let Some(entry_idx) = items.iter().position(|it| matches!(it, CodeItem::Instr { .. })) else {
        return after;
    };
    let mut in_flags: BTreeMap<usize, Flags> = BTreeMap::from([(entry_idx, Flags::TOP)]);
    let mut work: VecDeque<usize> = VecDeque::from([entry_idx]);
    while let Some(idx) = work.pop_front() {
        let incoming = in_flags[&idx];
        let here = match cfg.instr(idx) {
            // A call clobbers the condition codes (its callee's last instruction
            // is unknown); everything else takes the straight-line transfer.
            Some((mnem, _)) if is_call(mnem) => Flags::TOP,
            Some((mnem, ops)) => incoming.after(mnem, ops),
            None => incoming,
        };
        after.insert(idx, here);
        for edge in cfg.edges(idx) {
            let Edge::Follow(succ) = edge else { continue };
            let edge_flags = split_cc(cfg, idx, succ, here);
            let changed = match in_flags.get(&succ) {
                None => {
                    in_flags.insert(succ, edge_flags);
                    true
                }
                Some(existing) => {
                    let merged = existing.meet(&edge_flags);
                    if merged != *existing {
                        in_flags.insert(succ, merged);
                        true
                    } else {
                        false
                    }
                }
            };
            if changed {
                work.push_back(succ);
            }
        }
    }
    after
}

/// The condition tested by a SIMPLE conditional branch (`bcc`→`cc`, `bhs`→`cc`,
/// `blo`→`cs`, `bne`→`ne`, …). `None` for a non-branch, an unconditional `bra`,
/// a `dbcc` counting form, or `Scc` — only a plain `bXX` (3-char, `b`-prefix)
/// splits.
fn simple_branch_cond(mnem: &str) -> Option<&'static str> {
    if !(mnem.starts_with('b') && mnem.len() == 3) {
        return None;
    }
    let bare = mnem.strip_prefix('b')?;
    Some(match bare {
        "cc" | "hs" => "cc",
        "cs" | "lo" => "cs",
        "eq" => "eq",
        "ne" => "ne",
        "hi" => "hi",
        "ls" => "ls",
        "pl" => "pl",
        "mi" => "mi",
        "vc" => "vc",
        "vs" => "vs",
        "ge" => "ge",
        "lt" => "lt",
        "gt" => "gt",
        "le" => "le",
        _ => return None,
    })
}

/// Join two produced-sets on entry to the same node: INTERSECTION (MUST —
/// produced only if BOTH paths produce it).
fn join(a: &State, b: &State) -> State {
    let mut out = *a;
    for i in 0..16 {
        out[i] = a[i] && b[i];
    }
    out
}

// ===========================================================================
// The cc-abstract lattice (moveq-fold / branch-split transfer). Inert for the
// unconditional-out check; consumed by BOTH conditional-out obligations —
// production on the cc edges and survival on the ¬cc ones. Guardrail
// 1: only a KNOWN-cc source (a moveq-class immediate fold, or a branch-split)
// establishes a classified flag; EVERY other cc-writing instruction → ⊤; joins
// meet to ⊤ on disagreement. Never infer a known cc not proven.
// ===========================================================================

/// The abstract condition-code state: each of N/Z/V/C is `Some(bit)` when proven,
/// `None` (⊤) when unknown.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Flags {
    n: Option<bool>,
    z: Option<bool>,
    v: Option<bool>,
    c: Option<bool>,
}

impl Flags {
    const TOP: Flags = Flags { n: None, z: None, v: None, c: None };

    /// Meet two abstract flag states: a flag is known only where both agree.
    fn meet(&self, other: &Flags) -> Flags {
        fn m(a: Option<bool>, b: Option<bool>) -> Option<bool> {
            if a == b {
                a
            } else {
                None
            }
        }
        Flags {
            n: m(self.n, other.n),
            z: m(self.z, other.z),
            v: m(self.v, other.v),
            c: m(self.c, other.c),
        }
    }

    /// The abstract flags AFTER a straight-line instruction. Only `moveq` folds
    /// (its immediate fully determines N/Z/V/C); a cc-transparent instruction
    /// leaves the flags unchanged; every other instruction clobbers them to ⊤.
    fn after(&self, mnem: &str, ops: &[CodeOperand]) -> Flags {
        if mnem == "moveq" {
            if let Some(CodeOperand::Imm(v)) = ops.first() {
                let byte = (*v as i64) & 0xFF;
                return Flags {
                    n: Some(byte & 0x80 != 0),
                    z: Some(byte == 0),
                    v: Some(false),
                    c: Some(false),
                };
            }
            return Flags::TOP;
        }
        if cc_transparent(mnem) {
            *self
        } else {
            Flags::TOP
        }
    }

    /// Refine the abstract flags to reflect that condition `cc` evaluates to
    /// `holds` (the branch-split fact). Only the SINGLE-FLAG conditions pin a
    /// flag; a composite condition (`hi`/`ls`/`ge`/…) constrains a combination,
    /// so it refines nothing (sound — less precision, never a wrong known flag).
    fn refine(&self, cc: &str, holds: bool) -> Flags {
        let mut f = *self;
        match cc {
            "eq" => f.z = Some(holds),
            "ne" => f.z = Some(!holds),
            "cs" | "lo" => f.c = Some(holds),
            "cc" | "hs" => f.c = Some(!holds),
            "mi" => f.n = Some(holds),
            "pl" => f.n = Some(!holds),
            "vs" => f.v = Some(holds),
            "vc" => f.v = Some(!holds),
            _ => {} // composite — no single-flag refinement
        }
        f
    }

    /// Evaluate condition code `cc` against the abstract flags: `Some(true)` /
    /// `Some(false)` when proven, `None` when any needed flag is ⊤.
    fn eval(&self, cc: &str) -> Option<bool> {
        let not = |o: Option<bool>| o.map(|b| !b);
        match cc {
            "eq" => self.z,
            "ne" => not(self.z),
            "cs" | "lo" => self.c,
            "cc" | "hs" => not(self.c),
            "mi" => self.n,
            "pl" => not(self.n),
            "vs" => self.v,
            "vc" => not(self.v),
            "hi" => match (self.c, self.z) {
                (Some(c), Some(z)) => Some(!c && !z),
                _ => None,
            },
            "ls" => match (self.c, self.z) {
                (Some(c), Some(z)) => Some(c || z),
                _ => None,
            },
            "ge" => match (self.n, self.v) {
                (Some(n), Some(v)) => Some(n == v),
                _ => None,
            },
            "lt" => match (self.n, self.v) {
                (Some(n), Some(v)) => Some(n != v),
                _ => None,
            },
            "gt" => match (self.n, self.v, self.z) {
                (Some(n), Some(v), Some(z)) => Some(!z && (n == v)),
                _ => None,
            },
            "le" => match (self.n, self.v, self.z) {
                (Some(n), Some(v), Some(z)) => Some(z || (n != v)),
                _ => None,
            },
            _ => None,
        }
    }
}

/// Instructions that provably do NOT write the condition codes (68k): address
/// arithmetic and moves (`movea`/`lea`/`pea`/`adda`/`suba`), `movem`/`exg`/`nop`,
/// and all control transfers (branches READ the CC, returns/jumps do not write
/// it). Everything not listed — including a returning `jsr` (handled separately)
/// — is treated as a CC clobber (→ ⊤). Deliberately conservative: an unmodeled
/// mnemonic clobbers rather than silently preserving a stale flag (guardrail 1).
///
/// **Sound-complete for #2's edge-identification** (`flag_check::Cfg::valid_edge`,
/// the corrected banner): the Z-only writers `btst`/`bset`/`bclr`/`bchg` are NOT
/// listed, so they BAIL — unlike `flag_check::writes_carry` (§6's carry-polarity
/// allowlist), which lets them through as transparent. #2's cc is `eq`(Z), so a
/// Z-clobber between a conditional-out call and its `beq` guard MUST bail; reusing
/// `writes_carry` there would credit on a stale-Z edge = a must-def false negative.
pub(crate) fn cc_transparent(mnem: &str) -> bool {
    cc_inert_data_op(mnem) || is_branch_or_return(mnem)
}

/// The NON-CONTROL mnemonics that provably do not write the condition codes —
/// [`cc_transparent`]'s data-op half, exported on its own so the
/// `preserves(sr.ccr)` slice (`lower/proc.rs`) shares one allowlist with the
/// flag machinery instead of growing a drifting copy. Control transfers are
/// deliberately NOT here: the two consumers disagree about them (a returning
/// call preserves a COMPUTED flag's transparency question differently from an
/// ENTRY-CCR question), so each classifies control flow itself.
pub(crate) fn cc_inert_data_op(mnem: &str) -> bool {
    // `assume_some` is the niche-option extraction marker: it emits no bytes and
    // executes nothing, so it writes no condition code. Listing it keeps a marker
    // placed between a conditional-out call and its guard from bailing the
    // edge-credit walk on a mnemonic that is not even in the ROM.
    matches!(
        mnem,
        "movea" | "lea" | "pea" | "movem" | "exg" | "nop" | "adda" | "suba" | "assume_some"
    )
}

/// A control-transfer mnemonic — a conditional/unconditional branch, a `dbcc`
/// counting form, a jump/tail, or a return. None WRITE the data condition codes.
///
/// This asks a FLAG-WRITE question, not a control-flow one: its callers hold a
/// bare mnemonic and no CFG, so there is no edge to read the answer off. The
/// `b`-prefixed three-letter forms cover `bra` and every `bXX`; `jbra` is the one
/// tail spelling that needs naming.
fn is_branch_or_return(mnem: &str) -> bool {
    matches!(mnem, "rts" | "rte" | "rtr" | "rtd" | "jmp" | "jra" | "jbra")
        || (mnem.starts_with('b') && mnem.len() == 3)
        || mnem.starts_with("db")
}
