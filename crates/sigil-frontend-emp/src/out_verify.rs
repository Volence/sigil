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
//!   `callee_uncond_out` map — the Load_Object←AllocDynamic shape.
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

use crate::flag_check::{Cfg, Edge};
use crate::lower::instr_written_regs;
use crate::value::{CodeItem, CodeOperand, Reg, Width};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// The proof outcome for one declared output register.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutStatus {
    /// Proven produced on every required return path.
    Produced,
    /// Some required return path does not produce it (a false `out()` claim).
    Unverified(String),
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
fn produced_regs(mnem: &str, ops: &[CodeOperand], size: Option<Width>) -> Vec<Reg> {
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
) -> BTreeMap<Reg, OutStatus> {
    let cfg = Cfg::build(items);

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
                    let changed = match in_state.get(&succ) {
                        None => {
                            in_state.insert(succ, st.clone());
                            true
                        }
                        Some(existing) => {
                            let mut merged = existing.clone();
                            join(&mut merged, &st);
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
fn cc_transparent(mnem: &str) -> bool {
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
