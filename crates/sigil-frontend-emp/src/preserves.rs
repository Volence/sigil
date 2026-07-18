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

use crate::closure::RegEffect;
use crate::flag_check::{Cfg, Edge};
use crate::lower::instr_written_regs;
use crate::value::{CodeItem, CodeOperand, Reg, Width};
use sigil_span::Span;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

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
    /// This path hit a soundness bailout (computed sp / escape / aliasing store /
    /// underflow / unbalanced-depth join). Rides the CFG; only matters if it
    /// reaches a return.
    bailed: bool,
}

fn reg_idx(r: Reg) -> usize {
    r as usize
}

/// The resolved operand size of instruction item `idx`, if any.
fn instr_size(items: &[CodeItem], idx: usize) -> Option<Width> {
    match items.get(idx) {
        Some(CodeItem::Instr { size, .. }) => *size,
        _ => None,
    }
}

/// Expand a `movem` `RegList` bitmask to registers in canonical ASCENDING order
/// (`bit0=d0`..`bit7=d7`, `bit8=a0`..`bit15=a7` — the `CodeOperand::RegList`
/// convention shared with `check_preserves`).
pub(crate) fn expand_mask(mask: u16) -> Vec<Reg> {
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
    // A register is "written" if ANY instruction touches it — INCLUDING a pop/peek
    // (which writes the register, cleanly or, under a bailout like an underflow,
    // with garbage). Only a register never mentioned as a write anywhere stays
    // preserved past a bailout (its value is immutable).
    let mut ever_clobbered = [false; 16];
    let mut has_call = false;
    for it in items {
        if let CodeItem::Instr { mnemonic, ops, .. } = it {
            if is_call(mnemonic) {
                has_call = true;
            }
            for r in instr_written_regs(mnemonic, ops) {
                if r != Reg::A7 {
                    ever_clobbered[reg_idx(r)] = true;
                }
            }
            // `instr_written_regs` does not expand a movem reglist — a register
            // that only ever appears in a save/restore movem still participates
            // in stack traffic, so it is NOT "never written".
            if let Some(mask) = reglist_mask(ops) {
                for r in expand_mask(mask) {
                    if r != Reg::A7 {
                        ever_clobbered[reg_idx(r)] = true;
                    }
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
        State { stack: Vec::new(), entry: [true; 16], bailed: false },
    );
    let mut work: VecDeque<usize> = VecDeque::from([entry_idx]);
    // A bailout is PATH-LOCAL: it taints the state (`State::bailed`) and rides the
    // CFG. Only a bail that REACHES a return (`rts`/fall-off) makes a written
    // register unverifiable — a bail on a noreturn/`Defer` path (the DEBUG
    // `raise_error` `subq #2,sp`→`jmp handler` shape) never constrains a return.
    let mut bail_reason: Option<String> = None;
    let mut bailed_reached_return = false;

    // For each checked register: does EVERY return path see it at its entry
    // value? Starts true; a return with a clobbered value flips it false.
    let mut all_returns_preserve: BTreeMap<Reg, bool> =
        check.iter().map(|r| (*r, true)).collect();
    let mut saw_return = false;

    while let Some(idx) = work.pop_front() {
        let mut st = in_state[&idx].clone();
        // Apply the instruction's effect. A hazard taints this path (bailed) but
        // does NOT stop the dataflow — the path must still reach its terminator so
        // we can tell a returning bail from a diverging one.
        if !st.bailed {
            if let Some(reason) = transfer(&cfg, idx, &mut st, items) {
                st.bailed = true;
                bail_reason.get_or_insert(reason);
            }
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
                            join(&mut merged, &st); // depth mismatch → merged.bailed
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
                    // A return / fall-off-end: checkpoint every checked register.
                    saw_return = true;
                    if st.bailed {
                        bailed_reached_return = true;
                    } else {
                        for r in check {
                            if !st.entry[reg_idx(*r)] {
                                *all_returns_preserve.get_mut(r).unwrap() = false;
                            }
                        }
                    }
                }
                Edge::Defer => {
                    // An external tail transfer (`jmp`/`bra` to a non-local
                    // symbol) is NOT an `rts` of THIS proc. `preserves(rN)`
                    // constrains the proc's own return (`rts`/`rte`) paths; a tail
                    // transfer either diverges (a noreturn `raise_error`/error
                    // handler — no return obligation at all) or is a real tail
                    // call whose preservation is a TRANSITIVE property the closure
                    // accounts for via its tail edge (corpus `TAIL_MNEMONICS`).
                    // Either way it is not a local counterexample — ignore it,
                    // bailed or not. (This mirrors D2.32, which verified the movem
                    // pair ignoring control flow.)
                }
            }
        }
    }

    // Resolve each checked register's status.
    check
        .iter()
        .map(|r| {
            let clobbered = ever_clobbered[reg_idx(*r)];
            let status = if bailed_reached_return && clobbered {
                // A bail reached a return and this register is written somewhere —
                // the stack model can't prove the round-trip. (A never-written
                // register is immutable and stays Verified below.)
                PreserveStatus::Unverifiable(
                    bail_reason.clone().unwrap_or_else(|| "unverifiable stack".to_string()),
                )
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
fn transfer(cfg: &Cfg, idx: usize, st: &mut State, items: &[CodeItem]) -> Option<String> {
    let (mnem, ops) = cfg.instr(idx)?;
    // Only a FULL (`.l`) transfer round-trips an address/data register; a `.w`/`.b`
    // restore moves or sign-extends a fragment and preserves nothing.
    let is_long = matches!(instr_size(items, idx), Some(Width::L));

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

    // POP — `(sp)+`. Popping more slots than are tracked underflows into the
    // caller's frame / return address — the model is inconsistent → bail.
    if is_pop(ops) {
        let regs: Vec<Reg> = match reglist_mask(ops) {
            Some(mask) => expand_mask(mask),
            None => match ops.last() {
                Some(CodeOperand::Reg(dst)) => vec![*dst],
                _ => {
                    st.stack.pop();
                    return None;
                }
            },
        };
        if st.stack.len() < regs.len() {
            return Some("stack underflow — pop drains more than was pushed".to_string());
        }
        // movem restores in canonical ascending order; a single pop is one reg.
        for r in regs {
            let slot = st.stack.pop().flatten();
            // A `.w`/`.b` restore does not round-trip the full register.
            st.entry[reg_idx(r)] = is_long && slot == Some(r);
        }
        return None;
    }

    // PEEK — `(sp)` (no stack change), a full `.l` read of the top slot.
    if is_peek(mnem, ops) {
        if let Some(CodeOperand::Reg(dst)) = ops.last() {
            let top = st.stack.last().copied().flatten();
            st.entry[reg_idx(*dst)] = is_long && top == Some(*dst);
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
/// pointwise (`Some(r)` iff both agree). `bailed` propagates (OR). Differing stack
/// DEPTHS mean an ambiguous sp at the merge → taint the merged path `bailed`
/// (path-local, so a diverging bailed path never poisons a returning one).
fn join(acc: &mut State, other: &State) {
    acc.bailed |= other.bailed;
    if acc.stack.len() != other.stack.len() {
        acc.bailed = true;
        return;
    }
    for (a, b) in acc.stack.iter_mut().zip(other.stack.iter()) {
        if *a != *b {
            *a = None;
        }
    }
    for i in 0..16 {
        acc.entry[i] = acc.entry[i] && other.entry[i];
    }
}

// ===========================================================================
// §6 / D1d — [proc.dead-save]: a verified save/restore pair for a register the
// bracketed callee provably preserves. The pass-3 dead-save worklist.
// ===========================================================================

/// One dead save: proc `proc` saves `reg` across the call(s) `callees` (all of
/// which preserve `reg`), so the save/restore is redundant. `span` is the saving
/// instruction's site (the worklist's file:line).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeadSave {
    pub proc: String,
    pub reg: Reg,
    pub callees: Vec<String>,
    pub span: Span,
}

/// A dead-save stack slot: which register's value it holds, the push site, and —
/// accumulated over the span — whether the register was clobbered (a direct write
/// or a non-preserving call ⇒ the save is genuinely needed) and which callees it
/// bracketed.
#[derive(Clone, PartialEq, Eq)]
struct DsSlot {
    reg: Option<Reg>,
    push_idx: usize,
    clobbered: bool,
    callees: BTreeSet<String>,
}

#[derive(Clone, PartialEq, Eq)]
struct DsState {
    stack: Vec<DsSlot>,
}

/// Does callee `c` clobber `r`, per the closure's VERIFIED `effective` set? A
/// callee absent from the map is a hole → conservatively clobbers everything
/// (never report a dead save across an unknown callee). `⊤` likewise clobbers all.
fn callee_clobbers(effective: &BTreeMap<String, RegEffect>, c: &str, r: Reg) -> bool {
    match effective.get(c) {
        None => true,
        Some(e) => e.top || e.regs.contains(&reg_name(r)),
    }
}

fn reg_name(r: Reg) -> String {
    r.to_string()
}

/// A pending dead-save record, merged across every restore site of one push.
struct DeadRec {
    callees: BTreeSet<String>,
    any_clobbered: bool,
    span: Span,
}

/// Find every `[proc.dead-save]` in a proc's evaluated CodeBuf: a verified
/// save/restore of a register that nothing in its span clobbers (every bracketed
/// call PRESERVES it, per `effective`, and there is no direct scratch write). One
/// clobbering path anywhere suppresses the firing (the save is needed there).
pub fn find_dead_saves(
    proc_name: &str,
    items: &[CodeItem],
    effective: &BTreeMap<String, RegEffect>,
) -> Vec<DeadSave> {
    let cfg = Cfg::build(items);
    let Some(entry_idx) = items
        .iter()
        .position(|it| matches!(it, CodeItem::Instr { .. }))
    else {
        return Vec::new();
    };

    // Restore sites merge into this map, keyed by (reg, push site).
    let mut recs: BTreeMap<(Reg, usize), DeadRec> = BTreeMap::new();
    let mut in_state: BTreeMap<usize, DsState> = BTreeMap::new();
    in_state.insert(entry_idx, DsState { stack: Vec::new() });
    let mut work: VecDeque<usize> = VecDeque::from([entry_idx]);
    let mut bailed = false;

    while let Some(idx) = work.pop_front() {
        let mut st = in_state[&idx].clone();
        if ds_transfer(&cfg, idx, &mut st, effective, items, &mut recs) {
            bailed = true;
            break; // an unmodeled sp op — the stack model is unreliable
        }
        for edge in cfg.edges(idx) {
            if let Edge::Follow(succ) = edge {
                let changed = match in_state.get(&succ) {
                    None => {
                        in_state.insert(succ, st.clone());
                        true
                    }
                    Some(existing) => {
                        let mut merged = existing.clone();
                        if ds_join(&mut merged, &st).is_err() {
                            bailed = true;
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
            // Abandon / Defer: a return / external transfer — the save's fate is
            // decided at its pop, not here.
        }
        if bailed {
            break;
        }
    }

    if bailed {
        return Vec::new();
    }

    // Fire for saves un-clobbered on EVERY restore path that bracket ≥1 call.
    let mut out: Vec<DeadSave> = recs
        .into_iter()
        .filter(|((_, _), rec)| !rec.any_clobbered && !rec.callees.is_empty())
        .map(|((reg, _), rec)| DeadSave {
            proc: proc_name.to_string(),
            reg,
            callees: rec.callees.into_iter().collect(),
            span: rec.span,
        })
        .collect();
    out.sort_by_key(|d| (d.reg, d.span.start));
    out
}

/// Apply instruction `idx` to the dead-save state. Returns `true` on a soundness
/// bailout (unmodeled sp).
fn ds_transfer(
    cfg: &Cfg,
    idx: usize,
    st: &mut DsState,
    effective: &BTreeMap<String, RegEffect>,
    items: &[CodeItem],
    recs: &mut BTreeMap<(Reg, usize), DeadRec>,
) -> bool {
    let Some((mnem, ops)) = cfg.instr(idx) else {
        return false;
    };

    // A call: it clobbers every saved register in its effective set (that save is
    // then needed); it PRESERVES the rest (recorded as a bracketed callee). Nets
    // zero on the stack.
    if is_call(mnem) {
        if let Some(callee) = call_target(ops) {
            for slot in st.stack.iter_mut() {
                slot.callees.insert(callee.to_string());
                if let Some(r) = slot.reg {
                    if callee_clobbers(effective, callee, r) {
                        slot.clobbered = true;
                    }
                }
            }
        } else {
            // An indirect call — unknown effect; every live save is needed.
            for slot in st.stack.iter_mut() {
                slot.clobbered = true;
            }
        }
        return false;
    }

    if sp_hazard(ops) {
        return true;
    }

    // PUSH.
    if is_push(ops) {
        if let Some(mask) = reglist_mask(ops) {
            for r in expand_mask(mask).into_iter().rev() {
                st.stack.push(DsSlot { reg: Some(r), push_idx: idx, clobbered: false, callees: BTreeSet::new() });
            }
        } else {
            let reg = match ops.first() {
                Some(CodeOperand::Reg(r)) => Some(*r),
                _ => None,
            };
            st.stack.push(DsSlot { reg, push_idx: idx, clobbered: false, callees: BTreeSet::new() });
        }
        return false;
    }

    // POP — record each clean restore into the dead-save map.
    if is_pop(ops) {
        if let Some(mask) = reglist_mask(ops) {
            for r in expand_mask(mask) {
                if let Some(slot) = st.stack.pop() {
                    record_restore(slot, r, items, recs);
                }
            }
        } else if let Some(CodeOperand::Reg(dst)) = ops.last() {
            if let Some(slot) = st.stack.pop() {
                record_restore(slot, *dst, items, recs);
            }
        } else {
            st.stack.pop();
        }
        return false;
    }

    // PEEK — restores mid-span without popping; not a clobber.
    if is_peek(mnem, ops) {
        return false;
    }

    // A plain instruction: a direct write to a saved register means it is used as
    // scratch → the save is needed.
    for r in instr_written_regs(mnem, ops) {
        if r == Reg::A7 {
            continue;
        }
        for slot in st.stack.iter_mut() {
            if slot.reg == Some(r) {
                slot.clobbered = true;
            }
        }
    }
    false
}

/// A clean restore of `r` from `slot` (slot held `r`'s value) merges into the
/// dead-save record for that (reg, push) pair. A mismatched restore (the slot
/// held a different register — a deliberate move-through-stack like
/// load_object's a1→a2) is NOT a preservation and is ignored.
fn record_restore(
    slot: DsSlot,
    r: Reg,
    items: &[CodeItem],
    recs: &mut BTreeMap<(Reg, usize), DeadRec>,
) {
    if slot.reg != Some(r) {
        return;
    }
    let span = match &items[slot.push_idx] {
        CodeItem::Instr { span, .. } => *span,
        _ => return,
    };
    let rec = recs.entry((r, slot.push_idx)).or_insert(DeadRec {
        callees: BTreeSet::new(),
        any_clobbered: false,
        span,
    });
    rec.callees.extend(slot.callees.iter().cloned());
    rec.any_clobbered |= slot.clobbered;
}

/// The call target symbol (the last `Sym` operand), or `None` for an indirect
/// call.
fn call_target(ops: &[CodeOperand]) -> Option<&str> {
    ops.iter().rev().find_map(|o| match o {
        CodeOperand::Sym(name) => Some(name.as_str()),
        _ => None,
    })
}

/// Join `other` into `acc` for the dead-save dataflow. Differing depth ⇒ bail.
/// Slots meet: register identity/push-site must agree (else disable the slot);
/// `clobbered` ORs (needed on either path ⇒ needed) and `callees` unions — both
/// the SAFE direction for a code-cutting worklist.
fn ds_join(acc: &mut DsState, other: &DsState) -> Result<(), ()> {
    if acc.stack.len() != other.stack.len() {
        return Err(());
    }
    for (a, b) in acc.stack.iter_mut().zip(other.stack.iter()) {
        if a.reg != b.reg || a.push_idx != b.push_idx {
            a.reg = None;
        }
        a.clobbered |= b.clobbered;
        a.callees.extend(b.callees.iter().cloned());
    }
    Ok(())
}
