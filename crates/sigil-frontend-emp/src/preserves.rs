//! Contract-grammar v2 §5 — verified `preserves` by symbolic stack tracking.
//!
//! The dataflow upgrade over the D2.32 syntactic movem-pair slice (§5): a proc
//! `preserves(rN)` iff on EVERY return path rN holds its ENTRY value — restored
//! from the matching stack slot, or never written. This is a forward dataflow
//! over the SAME lightweight CFG G2 built ([`crate::flag_check::Cfg`], spec §11
//! Q1 — reused, not duplicated), tracking a symbolic stack:
//!
//! - a **slot map** — the sp-relative save area as a stack of slots, each tagged
//!   with which register's entry value it holds (or opaque);
//! - per register, whether it currently holds its **entry value**.
//!
//! Saves (`move.l aN,-(sp)`, `movem.l <list>,-(sp)`) push slots; restores
//! (`movea.l (sp)+,aN`, `movem.l (sp)+,<list>`) pop and match; the `(sp)` PEEK
//! (`movea.l (sp),aN`) restores from the top without popping. A call clobbers
//! every register (conservative — no callee contract locally; that is the
//! closure's transitive job) but nets zero on the stack (the return address it
//! pushes is popped by its own `rts`). Any generic write to rN clears its
//! entry-value bit; a restore re-sets it iff the slot holds rN's own value.
//!
//! **Soundness bailouts** (assembly is assembly, §5): a bare `a7` operand (sp's
//! VALUE used as a number / escaping into address math, or a computed
//! `adda #n,sp`), or a displaced/indexed sp access (`d(sp)` / `(sp,Xn)` — a store
//! that could alias a tracked slot) makes the stack model untrustworthy → the
//! analysis BAILS. A declared `preserves` on a written register whose proof bailed
//! is `[proc.preserves-unverifiable]` (error — a wrong contract is worse than
//! none, the D2.32 principle kept). The movem entry/exit pair is the trivial fast
//! path of this same analysis — D2.32 subsumed.

use crate::flag_check::{Cfg, Edge};
use crate::lower::instr_written_regs;
use crate::value::{CodeItem, CodeOperand, Reg};
use std::collections::{BTreeMap, VecDeque};

/// The proof outcome for one checked register.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreserveStatus {
    /// Proven preserved: on every return path the register holds its entry value.
    Verified,
    /// Proven NOT preserved: some return path leaves it clobbered (a declared
    /// `preserves` here is a false contract).
    NotPreserved,
    /// A soundness bailout was hit (computed sp / sp escape / aliasing store) and
    /// the register is written — the proof is unavailable in either direction.
    Unverifiable(String),
}

/// A stack slot: `Some(r)` holds register `r`'s entry value; `None` is opaque.
type Slot = Option<Reg>;

/// The abstract state at a program point: the symbolic stack + per-register
/// entry-value bits (indexed `d0`..`a7` = 0..16).
#[derive(Clone, PartialEq, Eq)]
struct State {
    stack: Vec<Slot>,
    entry: [bool; 16],
}

fn reg_idx(r: Reg) -> usize {
    r as usize
}

/// Expand a `movem` `RegList` bitmask to registers in canonical ASCENDING order
/// (`bit0=d0`..`bit7=d7`, `bit8=a0`..`bit15=a7` — the `CodeOperand::RegList`
/// convention shared with `check_preserves`).
fn expand_mask(mask: u16) -> Vec<Reg> {
    const ORDER: [Reg; 16] = [
        Reg::D0, Reg::D1, Reg::D2, Reg::D3, Reg::D4, Reg::D5, Reg::D6, Reg::D7,
        Reg::A0, Reg::A1, Reg::A2, Reg::A3, Reg::A4, Reg::A5, Reg::A6, Reg::A7,
    ];
    (0..16).filter(|b| mask & (1 << b) != 0).map(|b| ORDER[b]).collect()
}

/// The `RegList` mask among a movem's operands, if any.
fn reglist_mask(ops: &[CodeOperand]) -> Option<u16> {
    ops.iter().find_map(|o| match o {
        CodeOperand::RegList(m) => Some(*m),
        _ => None,
    })
}

/// Verify `preserves(rN)` for each register in `check` over a proc's evaluated
/// CodeBuf `items`. One forward dataflow serves all checked registers.
pub fn verify_preserved(items: &[CodeItem], check: &[Reg]) -> BTreeMap<Reg, PreserveStatus> {
    let cfg = Cfg::build(items);

    // The first instruction is the entry point; a body with no instructions
    // preserves everything vacuously.
    let Some(entry_idx) = items
        .iter()
        .position(|it| matches!(it, CodeItem::Instr { .. }))
    else {
        return check.iter().map(|r| (*r, PreserveStatus::Verified)).collect();
    };

    // Which registers are ever clobbered (a generic write, or ANY call — a call
    // conservatively clobbers all). Used only to let a NEVER-written register
    // stay Verified even past a bailout (its value cannot change).
    let mut ever_clobbered = [false; 16];
    let mut has_call = false;
    for it in items {
        if let CodeItem::Instr { mnemonic, ops, .. } = it {
            if is_call(mnemonic) {
                has_call = true;
            }
            // Skip the clean restore/peek destination write — that RE-sets the
            // entry value, it is not a clobber.
            if is_pop(ops) || is_peek(mnemonic, ops) {
                continue;
            }
            for r in instr_written_regs(mnemonic, ops) {
                if r != Reg::A7 {
                    ever_clobbered[reg_idx(r)] = true;
                }
            }
        }
    }
    if has_call {
        ever_clobbered = [true; 16];
    }

    // Forward dataflow with joins. `in_state[idx]` = state on entry to instr idx.
    let mut in_state: BTreeMap<usize, State> = BTreeMap::new();
    in_state.insert(
        entry_idx,
        State { stack: Vec::new(), entry: [true; 16] },
    );
    let mut work: VecDeque<usize> = VecDeque::from([entry_idx]);
    let mut bailed: Option<String> = None;

    // For each checked register: does EVERY return path see it at its entry
    // value? Starts true; a return with a clobbered value flips it false.
    let mut all_returns_preserve: BTreeMap<Reg, bool> =
        check.iter().map(|r| (*r, true)).collect();
    let mut saw_return = false;

    while let Some(idx) = work.pop_front() {
        let mut st = in_state[&idx].clone();
        // Apply the instruction's effect to `st` (in place).
        if let Some(reason) = transfer(&cfg, idx, &mut st) {
            bailed.get_or_insert(reason);
            // A bail corrupts the stack model globally; stop propagating.
            continue;
        }
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
                            if join(&mut merged, &st).is_err() {
                                bailed.get_or_insert(
                                    "unbalanced stack at a control-flow join".to_string(),
                                );
                                false
                            } else if merged != *existing {
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
                    // A return / fall-off-end: checkpoint every checked register.
                    saw_return = true;
                    for r in check {
                        if !st.entry[reg_idx(*r)] {
                            *all_returns_preserve.get_mut(r).unwrap() = false;
                        }
                    }
                }
                Edge::Defer => {
                    // External tail transfer: the register's final value depends
                    // on code we cannot see — not locally preservable.
                    saw_return = true;
                    for r in check {
                        *all_returns_preserve.get_mut(r).unwrap() = false;
                    }
                }
            }
        }
    }

    // Resolve each checked register's status.
    check
        .iter()
        .map(|r| {
            let status = if let Some(reason) = &bailed {
                if ever_clobbered[reg_idx(*r)] {
                    PreserveStatus::Unverifiable(reason.clone())
                } else {
                    // Never written → immutable regardless of the stack model.
                    PreserveStatus::Verified
                }
            } else if !saw_return || all_returns_preserve[r] {
                PreserveStatus::Verified
            } else {
                PreserveStatus::NotPreserved
            };
            (*r, status)
        })
        .collect()
}

fn is_call(mnem: &str) -> bool {
    matches!(mnem, "jsr" | "jbsr" | "bsr")
}

/// A PUSH: some operand predecrements a7 (`-(sp)`).
fn is_push(ops: &[CodeOperand]) -> bool {
    ops.iter().any(|o| matches!(o, CodeOperand::PreDec(Reg::A7)))
}

/// A POP: some operand postincrements a7 (`(sp)+`).
fn is_pop(ops: &[CodeOperand]) -> bool {
    ops.iter().any(|o| matches!(o, CodeOperand::PostInc(Reg::A7)))
}

/// A PEEK: `move`/`movea` with a plain `(sp)` source and a register destination
/// (restore from the top without popping — the park/unpark shape).
fn is_peek(mnem: &str, ops: &[CodeOperand]) -> bool {
    (mnem == "move" || mnem == "movea")
        && matches!(ops.first(), Some(CodeOperand::Ind(Reg::A7)))
        && matches!(ops.last(), Some(CodeOperand::Reg(_)))
}

/// An unmodeled sp hazard → bail: a bare `a7` operand (sp's value used directly —
/// escapes into address math or a computed `adda #n,sp`), or a displaced/indexed
/// sp access (`d(sp)` / `(sp,Xn)` — could alias a tracked slot). The clean stack
/// forms use `PreDec`/`PostInc`/`Ind` of a7, never these.
fn sp_hazard(ops: &[CodeOperand]) -> bool {
    ops.iter().any(|o| {
        matches!(
            o,
            CodeOperand::Reg(Reg::A7)
                | CodeOperand::DispInd { reg: Reg::A7, .. }
                | CodeOperand::IndIdx { reg: Reg::A7, .. }
                | CodeOperand::IndIdx { xn: Reg::A7, .. }
        )
    })
}

/// Apply instruction `idx`'s effect to `st`. Returns `Some(reason)` on a
/// soundness bailout.
fn transfer(cfg: &Cfg, idx: usize, st: &mut State) -> Option<String> {
    let (mnem, ops) = cfg.instr(idx)?;

    // A call clobbers every register (no local callee contract) but nets zero on
    // the stack (its pushed return address is popped by its own rts).
    if is_call(mnem) {
        st.entry = [false; 16];
        return None;
    }

    // Unmodeled sp manipulation corrupts the slot model.
    if sp_hazard(ops) {
        return Some(sp_hazard_reason(ops));
    }

    // PUSH — `-(sp)`.
    if is_push(ops) {
        if let Some(mask) = reglist_mask(ops) {
            // movem save: push in REVERSE canonical order so the lowest register
            // (d0) lands on top, matching the (sp)+ restore order.
            for r in expand_mask(mask).into_iter().rev() {
                st.stack.push(tag(st, r));
            }
        } else {
            // Single push: the SOURCE (first operand) if it is a plain register.
            let slot = match ops.first() {
                Some(CodeOperand::Reg(r)) => tag(st, *r),
                _ => None, // pushing a non-register value → opaque slot
            };
            st.stack.push(slot);
        }
        return None;
    }

    // POP — `(sp)+`.
    if is_pop(ops) {
        if let Some(mask) = reglist_mask(ops) {
            // movem restore: pop in canonical ASCENDING order.
            for r in expand_mask(mask) {
                let slot = st.stack.pop().flatten();
                st.entry[reg_idx(r)] = slot == Some(r);
            }
        } else if let Some(CodeOperand::Reg(dst)) = ops.last() {
            let slot = st.stack.pop().flatten();
            st.entry[reg_idx(*dst)] = slot == Some(*dst);
        } else {
            // `(sp)+` into a non-register destination just drops a slot.
            st.stack.pop();
        }
        return None;
    }

    // PEEK — `(sp)` (no stack change).
    if is_peek(mnem, ops) {
        if let Some(CodeOperand::Reg(dst)) = ops.last() {
            let top = st.stack.last().copied().flatten();
            st.entry[reg_idx(*dst)] = top == Some(*dst);
        }
        return None;
    }

    // Plain instruction: every generic write (except a7) clears the register's
    // entry-value bit.
    for r in instr_written_regs(mnem, ops) {
        if r != Reg::A7 {
            st.entry[reg_idx(r)] = false;
        }
    }
    None
}

/// The slot tag for register `r` at the current state: `Some(r)` iff `r` still
/// holds its entry value.
fn tag(st: &State, r: Reg) -> Slot {
    if st.entry[reg_idx(r)] {
        Some(r)
    } else {
        None
    }
}

fn sp_hazard_reason(ops: &[CodeOperand]) -> String {
    if ops.iter().any(|o| matches!(o, CodeOperand::Reg(Reg::A7))) {
        "sp used as a value (computed sp / escape into address math)".to_string()
    } else {
        "displaced/indexed sp access could alias a saved slot".to_string()
    }
}

/// Join `other` into `acc` (both on entry to the same node). Entry bits meet by
/// AND (a register is entry-valued only if BOTH paths agree). Slots meet
/// pointwise (`Some(r)` iff both agree). Differing stack DEPTHS mean an ambiguous
/// sp at the merge → `Err` (bail).
fn join(acc: &mut State, other: &State) -> Result<(), ()> {
    if acc.stack.len() != other.stack.len() {
        return Err(());
    }
    for (a, b) in acc.stack.iter_mut().zip(other.stack.iter()) {
        if *a != *b {
            *a = None;
        }
    }
    for i in 0..16 {
        acc.entry[i] = acc.entry[i] && other.entry[i];
    }
    Ok(())
}
