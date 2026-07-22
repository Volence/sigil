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
//! - **Tail-out credit** at an `Edge::Defer` from an UNCONDITIONAL tail
//!   (`bra`/`jbra`/`jmp`/`jra`, Finding 3): a tail transfer is a return of P from
//!   the caller's view, so it is a required return path; if the target is a known
//!   proc, credit its unconditional out, else the out cannot be verified ⇒ FIRE.
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

use crate::flag_check::{conditional_out_edge_credits, Cfg, Edge};
use crate::lower::instr_written_regs;
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
) -> Vec<OutFiring> {
    verify_out(items, uncond, cond, callee_uncond_out, cond_callees)
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

/// UNCONDITIONAL tail transfers — a `Defer` from one of these is a required
/// return path (control leaves P for the target, which returns to P's caller).
fn is_uncond_tail(mnem: &str) -> bool {
    matches!(mnem, "bra" | "jbra" | "jmp" | "jra")
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
/// every path here, and the abstract condition-code state (for conditional-out
/// success-edge classification — inert until the cc layer).
#[derive(Clone, PartialEq, Eq)]
struct State {
    produced: [bool; 16],
    flags: Flags,
}

/// The bare `Sym` target of a direct call/tail (`jbsr Foo` / `jbra Foo`), or
/// `None` for an indirect / local-label (`$`-mangled) target.
fn direct_target(ops: &[CodeOperand]) -> Option<&str> {
    match ops {
        [CodeOperand::Sym(name)] if !name.contains('$') => Some(name.as_str()),
        _ => None,
    }
}

/// Apply instruction `idx`'s effect to `st`: gen full-width productions, credit a
/// call's callee unconditional outs, and update the abstract flags.
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
        // (the shared map). It also clobbers the condition codes.
        if let Some(target) = direct_target(ops) {
            if let Some(outs) = callee_uncond_out.get(target) {
                for name in outs {
                    if let Some(r) = Reg::from_name(name) {
                        st.produced[reg_idx(r)] = true;
                    }
                }
            }
        }
        st.flags = Flags::TOP;
        return;
    }

    // Production is gen-only (a value produced upstream stays produced —
    // Finding 5; a later partial write does not un-produce).
    for r in produced_regs(mnem, ops, size) {
        st.produced[reg_idx(r)] = true;
    }

    st.flags = st.flags.after(mnem, ops);
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
    in_state.insert(entry_idx, State { produced: [false; 16], flags: Flags::TOP });
    let mut work: VecDeque<usize> = VecDeque::from([entry_idx]);

    // Per checked register: produced on EVERY required return path so far, and
    // the reason of the first failing return (for the diagnostic).
    let mut ok: BTreeMap<Reg, bool> = guard.keys().map(|r| (*r, true)).collect();
    let mut fail_reason: BTreeMap<Reg, String> = BTreeMap::new();

    while let Some(idx) = work.pop_front() {
        let mut st = in_state[&idx].clone();
        transfer(&cfg, idx, &mut st, items, callee_uncond_out);

        for edge in cfg.edges(idx) {
            match edge {
                Edge::Follow(succ) => {
                    // Branch-split (guardrail 1): along a conditional branch's
                    // taken / fall-through edges the tested cc is provably TRUE /
                    // FALSE respectively — refine the propagated flags so a `!cc`
                    // return reached directly off the branch is classified.
                    let mut edge_st = split_flags(&cfg, idx, succ, &st);
                    // Item #2: credit a callee's conditional out on THIS success
                    // edge only (per-edge transfer; the join below re-intersects).
                    if let Some(regs) = edge_credit.get(&(idx, succ)) {
                        for name in regs {
                            if let Some(r) = Reg::from_name(name) {
                                edge_st.produced[reg_idx(r)] = true;
                            }
                        }
                    }
                    let changed = match in_state.get(&succ) {
                        None => {
                            in_state.insert(succ, edge_st);
                            true
                        }
                        Some(existing) => {
                            let mut merged = existing.clone();
                            join(&mut merged, &edge_st);
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
                Edge::Abandon => {
                    // A return / fall-off-end: no extra credit.
                    check_return(&st.produced, &st.flags, &guard, &mut ok, &mut fail_reason);
                }
                Edge::Defer => {
                    // An UNCONDITIONAL tail transfer is a required return path
                    // (Finding 3); a conditional-branch Defer (a divergent handler
                    // or a transitive tail) is not a local counterexample — ignore
                    // it, mirroring `preserves`.
                    let Some((mnem, ops)) = cfg.instr(idx) else { continue };
                    if !is_uncond_tail(mnem) {
                        continue;
                    }
                    // Credit the tail target's UNCONDITIONAL out (a known proc
                    // producing rN); an external / unresolved target credits
                    // nothing ⇒ any un-produced out fails here.
                    let mut credit = st.produced;
                    if let Some(target) = direct_target(ops) {
                        if let Some(outs) = callee_uncond_out.get(target) {
                            for name in outs {
                                if let Some(r) = Reg::from_name(name) {
                                    credit[reg_idx(r)] = true;
                                }
                            }
                        }
                    }
                    check_return(&credit, &st.flags, &guard, &mut ok, &mut fail_reason);
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
            let statuses = verify_out(items, &uncond, &cond, &v_uncond, &v_cond);
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

/// The propagated state along the `idx → succ` edge, branch-split refined: if
/// `idx` is a SIMPLE conditional branch (`bXX`), the tested cc is provably TRUE
/// on the taken edge and FALSE on the fall-through. A `dbcc`, an unconditional
/// tail, or a composite/unknown cc refines nothing (sound). The degenerate
/// branch-to-fall-through case cannot be split and stays ⊤-safe.
fn split_flags(cfg: &Cfg, idx: usize, succ: usize, st: &State) -> State {
    let Some((mnem, ops)) = cfg.instr(idx) else { return st.clone() };
    let Some(cc) = simple_branch_cond(mnem) else { return st.clone() };
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
        return st.clone();
    }
    let holds = Some(succ) != fallthrough; // the taken edge iff not the fall-through
    let mut out = st.clone();
    out.flags = out.flags.refine(cc, holds);
    out
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

/// Join `other` into `acc` (both on entry to the same node). `produced` meets by
/// INTERSECTION (MUST — produced only if BOTH paths produce it); `flags` meet
/// pointwise.
fn join(acc: &mut State, other: &State) {
    for i in 0..16 {
        acc.produced[i] = acc.produced[i] && other.produced[i];
    }
    acc.flags = acc.flags.meet(&other.flags);
}

// ===========================================================================
// The cc-abstract lattice (moveq-fold / branch-split transfer). Inert for the
// unconditional-out check; consumed by the conditional-out obligation. Guardrail
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
    matches!(
        mnem,
        "movea" | "lea" | "pea" | "movem" | "exg" | "nop" | "adda" | "suba"
    ) || is_branch_or_return(mnem)
}

/// A control-transfer mnemonic — a conditional/unconditional branch, a `dbcc`
/// counting form, a jump/tail, or a return. None WRITE the data condition codes.
fn is_branch_or_return(mnem: &str) -> bool {
    matches!(mnem, "rts" | "rte" | "rtr" | "rtd" | "jmp" | "jra")
        || is_uncond_tail(mnem)
        || (mnem.starts_with('b') && mnem.len() == 3)
        || mnem.starts_with("db")
}
