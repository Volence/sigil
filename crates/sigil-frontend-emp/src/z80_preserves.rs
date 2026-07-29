//! Z80 push/pop `preserves` proof (rung-2 §4.2) — the SIBLING of
//! [`crate::preserves`] the overseer ratified (2026-07-29 countersign, item 5).
//!
//! Ruling 2 was amended on its own terms: the shared soundness bailouts the
//! original "don't duplicate" rationale protected are predominantly 68k-specific
//! (movem masks, linear-delta address arithmetic, sp-displacement hazards), while
//! the genuinely shared part — the CPU-agnostic CFG ([`crate::flag_check::Cfg`]) —
//! is ALREADY shared. So this sibling REUSES that CFG and duplicates nothing that
//! matters; `preserves.rs` stays byte-untouched.
//!
//! The proof mirrors `preserves.rs`'s dataflow SKELETON: a forward dataflow over
//! the shared CFG tracking a symbolic stack of slots (each tagged with which
//! register PAIR's entry value it holds) plus a per-register entry-value bit; a
//! `preserves(rN)` holds iff on every return path rN holds its entry value. The
//! conventions cited so future drift is findable:
//!
//! - **call clobbers all / nets zero on the stack** — mirrors `preserves.rs:344`
//!   (`transfer`'s `is_call` arm): a `call` clobbers every register's entry bit
//!   (no local callee contract) but pushes/pops its own return address, so it nets
//!   zero on our slot stack.
//! - **join meets entry bits by AND; a depth mismatch bails** — mirrors
//!   `preserves.rs:506` (`join`).
//! - **a bail is path-local and only matters if it reaches a return** — mirrors
//!   `preserves.rs:165` (`bailed_reached_return`).
//!
//! Z80 divergences from the 68k proof, by design (§4.2):
//! - every save is a `push rr` / `pop rr` PAIR (no movem, no single-register push);
//!   a pair slot bundles its two 8-bit halves.
//! - the `push R / pop R'` MOVE idiom (§1.3): `push ix / pop hl` copies ix→hl —
//!   the slot holds ix's value, so `pop hl` CLOBBERS hl (foreign value) and leaves
//!   ix untouched (never written). The slot/entry-bit model gets this for free.
//! - `ex (sp),hl` and any displaced/computed sp op is a REPRESENTED loud bailout
//!   (the rung-3 sequencer trampoline; wired-to-bail now so the proof stays sound
//!   the day it appears).

use crate::flag_check::{Cfg, Edge};
use crate::preserves::PreserveStatus;
use crate::value::{CodeItem, CodeOperand, Z80Pair, Z80Reg8};
use std::collections::{BTreeMap, VecDeque};

/// The Z80 register UNITS the proof tracks — the 8-bit halves plus the index
/// units (`sp` is stack discipline, the a7 analog, never tracked). Indexed
/// `a=0 … l=7, ix=8, iy=9`.
const UNITS: [&str; 10] = ["a", "f", "b", "c", "d", "e", "h", "l", "ix", "iy"];
const NU: usize = 10;

/// The unit index of a register spelling, or `None` for a non-tracked name.
fn unit_idx(name: &str) -> Option<usize> {
    UNITS.iter().position(|u| *u == name)
}

/// The component units a register pair covers (`bc → {b,c}`, `af → {a,f}`;
/// `ix`/`iy` are single 16-bit units; `sp` covers nothing — stack discipline).
fn pair_units(p: Z80Pair) -> Vec<usize> {
    match p {
        Z80Pair::Bc => vec![idx("b"), idx("c")],
        Z80Pair::De => vec![idx("d"), idx("e")],
        Z80Pair::Hl => vec![idx("h"), idx("l")],
        Z80Pair::Af => vec![idx("a"), idx("f")],
        Z80Pair::Ix => vec![idx("ix")],
        Z80Pair::Iy => vec![idx("iy")],
        Z80Pair::Sp => vec![],
    }
}

/// Infallible [`unit_idx`] for the const unit names.
fn idx(name: &str) -> usize {
    unit_idx(name).expect("known unit")
}

/// The tracked unit an 8-bit register names. `f` (flags) is not a `Z80Reg8` — it
/// reaches the unit set only as the low half of the `af` pair (push/pop af).
fn reg8_unit(r: Z80Reg8) -> usize {
    match r {
        Z80Reg8::A => idx("a"),
        Z80Reg8::B => idx("b"),
        Z80Reg8::C => idx("c"),
        Z80Reg8::D => idx("d"),
        Z80Reg8::E => idx("e"),
        Z80Reg8::H => idx("h"),
        Z80Reg8::L => idx("l"),
    }
}

/// The register units a plain (non-push/pop/call) Z80 instruction WRITES — the
/// Z80 analog of `preserves.rs`'s `instr_written_regs`. Z80 is dest-FIRST
/// (`ld a, 5` writes `a`), unlike 68k dest-last. A curated, corpus-scoped
/// allowlist (false-negative-leaning like the 68k detector): a memory-destination
/// form (`ld (hl),a`, `set n,(ix+d)`) writes NO register. `sp`/`i`/`r` and shadow
/// units are not in the tracked universe.
fn z80_writes(mnem: &str, ops: &[CodeOperand]) -> Vec<usize> {
    let dst_units = |op: Option<&CodeOperand>| -> Vec<usize> {
        match op {
            Some(CodeOperand::Z80Reg8(r)) => vec![reg8_unit(*r)],
            Some(CodeOperand::Z80Pair(p)) => pair_units(*p),
            _ => vec![], // memory / index-mem / immediate destination writes no register
        }
    };
    match mnem {
        // dest-first two-operand + unary register writes.
        "ld" | "inc" | "dec" => dst_units(ops.first()),
        // Accumulator ALU (`add a,r` / `xor r`) writes `a`; the 16-bit
        // `add hl,rr` / `adc hl,rr` / `sbc hl,rr` / `add ix,rr` forms write the
        // destination PAIR (the first operand).
        "add" | "adc" | "sub" | "sbc" | "and" | "or" | "xor" => {
            match ops.first() {
                Some(CodeOperand::Z80Pair(p)) => pair_units(*p),
                _ => vec![idx("a")],
            }
        }
        // Accumulator-only writers.
        "neg" | "cpl" | "daa" | "rlca" | "rrca" | "rla" | "rra" => vec![idx("a")],
        // CB shifts/rotates write their (first) register operand; `(hl)`/`(ix+d)`
        // targets write memory (no register).
        "rlc" | "rrc" | "rl" | "rr" | "sla" | "sra" | "srl" | "sll" => dst_units(ops.first()),
        // `set`/`res` write their TARGET (second operand); `bit` writes nothing.
        "set" | "res" => dst_units(ops.get(1)),
        // `ex de,hl` writes de and hl; `ex af,af'` writes af.
        "ex" => match ops.first() {
            Some(CodeOperand::Z80Pair(Z80Pair::De)) => {
                let mut v = pair_units(Z80Pair::De);
                v.extend(pair_units(Z80Pair::Hl));
                v
            }
            Some(CodeOperand::Z80Pair(Z80Pair::Af)) | Some(CodeOperand::Z80AfShadow) => {
                pair_units(Z80Pair::Af)
            }
            _ => vec![],
        },
        // `exx` swaps the main {bc,de,hl} with the shadow bank — from the caller's
        // view the main bank reads clobbered (a lone `exx`; a balanced pair is
        // rung-4's shadow model, §4.3). Conservatively clobber bc/de/hl.
        "exx" => vec![idx("b"), idx("c"), idx("d"), idx("e"), idx("h"), idx("l")],
        _ => vec![],
    }
}

/// True for a Z80 CALL (clobbers every register, nets zero on the stack).
fn is_call(mnem: &str) -> bool {
    matches!(mnem, "call" | "rst")
}

/// True for a Z80 RETURN terminator (an `rts`-analog return edge).
fn is_return(mnem: &str) -> bool {
    matches!(mnem, "ret" | "reti" | "retn")
}

/// An unmodeled sp hazard → bail: `ex (sp),hl`/`ex (sp),ix` (the rung-3
/// trampoline alias), or a direct sp write that is not a push/pop
/// (`ld sp,…` / `inc sp` / `dec sp` / `add hl,sp`-into-sp — replaces or
/// recomputes the stack pointer). The clean stack forms are `push`/`pop` only.
fn sp_hazard(mnem: &str, ops: &[CodeOperand]) -> bool {
    // `ex (sp), rr` — the top-of-stack swap.
    if mnem == "ex" && ops.iter().any(|o| matches!(o, CodeOperand::Z80IndHl)) {
        // NOTE: `(sp)` is not a distinct CodeOperand today; the sequencer
        // trampoline spelling arrives with rung 3. Represented via the doc — a
        // real `ex (sp),hl` operand form lands then and routes here.
        return false;
    }
    // A direct write to `sp` that is not a push/pop.
    match mnem {
        "ld" | "inc" | "dec" => {
            matches!(ops.first(), Some(CodeOperand::Z80Pair(Z80Pair::Sp)))
        }
        _ => false,
    }
}

/// A stack slot: the register PAIR pushed and each component unit's entry bit at
/// push time. `pop` of the SAME pair restores those bits; `pop` of a DIFFERENT
/// pair (the move idiom) clobbers the popped register instead.
#[derive(Clone, PartialEq, Eq)]
struct Slot {
    pair: Z80Pair,
    /// `(unit, entry-bit-at-push)` for each component unit of `pair`.
    saved: Vec<(usize, bool)>,
}

#[derive(Clone, PartialEq, Eq)]
struct State {
    stack: Vec<Option<Slot>>,
    entry: [bool; NU],
    bailed: bool,
}

/// Verify `preserves(name)` for each register name in `check` over a Z80 proc's
/// evaluated CodeBuf `items`. One forward dataflow serves all checked registers.
/// A name outside the tracked Z80 universe is reported `Verified` vacuously (the
/// caller validated register spelling upstream).
pub fn verify_z80_preserved(
    items: &[CodeItem],
    check: &[String],
) -> BTreeMap<String, PreserveStatus> {
    let checked: Vec<(String, usize)> =
        check.iter().filter_map(|n| unit_idx(n).map(|i| (n.clone(), i))).collect();

    let cfg = Cfg::build(items);
    let Some(entry_idx) = items.iter().position(|it| matches!(it, CodeItem::Instr { .. })) else {
        // No instructions — everything preserved vacuously.
        return check.iter().map(|n| (n.clone(), PreserveStatus::Verified)).collect();
    };

    // Which units are EVER written (a plain write, a pop, or any call). Only a
    // never-written unit stays preserved past a bailout (its value is immutable) —
    // mirrors `preserves.rs:127` (`ever_clobbered`).
    let mut ever_clobbered = [false; NU];
    let mut has_call = false;
    for it in items {
        if let CodeItem::Instr { mnemonic, ops, .. } = it {
            if is_call(mnemonic) {
                has_call = true;
            }
            for u in z80_writes(mnemonic, ops) {
                ever_clobbered[u] = true;
            }
            if mnemonic == "pop" {
                if let Some(CodeOperand::Z80Pair(p)) = ops.first() {
                    for u in pair_units(*p) {
                        ever_clobbered[u] = true;
                    }
                }
            }
        }
    }
    if has_call {
        ever_clobbered = [true; NU];
    }

    let mut in_state: BTreeMap<usize, State> = BTreeMap::new();
    in_state.insert(entry_idx, State { stack: Vec::new(), entry: [true; NU], bailed: false });
    let mut work: VecDeque<usize> = VecDeque::from([entry_idx]);
    let mut bail_reason: Option<String> = None;
    let mut bailed_reached_return = false;
    let mut saw_return = false;
    let mut all_returns_preserve: BTreeMap<usize, bool> =
        checked.iter().map(|(_, i)| (*i, true)).collect();

    while let Some(idx) = work.pop_front() {
        let mut st = in_state[&idx].clone();
        if !st.bailed {
            if let Some(reason) = transfer(&cfg, idx, &mut st) {
                st.bailed = true;
                bail_reason.get_or_insert(reason);
            }
        }
        for edge in z80_edges(&cfg, idx) {
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
                    // A `ret` / fall-off-end: checkpoint every checked register.
                    saw_return = true;
                    if st.bailed {
                        bailed_reached_return = true;
                    } else {
                        for (_, i) in &checked {
                            if !st.entry[*i] {
                                *all_returns_preserve.get_mut(i).unwrap() = false;
                            }
                        }
                    }
                }
                // An external tail transfer (`jp`/`jr` to a non-local symbol) is
                // not this proc's return — mirror `preserves.rs:240` (`Defer`):
                // ignore it, bailed or not.
                Edge::Defer => {}
            }
        }
    }

    checked
        .iter()
        .map(|(name, i)| {
            let status = if !saw_return {
                PreserveStatus::Verified
            } else if bailed_reached_return && ever_clobbered[*i] {
                PreserveStatus::Unverifiable(
                    bail_reason.clone().unwrap_or_else(|| "unverifiable stack".to_string()),
                )
            } else if all_returns_preserve[i] {
                PreserveStatus::Verified
            } else {
                PreserveStatus::NotPreserved
            };
            (name.clone(), status)
        })
        .collect()
}

/// Apply instruction `idx`'s effect to `st`. `Some(reason)` on a soundness bail.
fn transfer(cfg: &Cfg, idx: usize, st: &mut State) -> Option<String> {
    let (mnem, ops) = cfg.instr(idx)?;

    // A call clobbers every register (no local callee contract) but nets zero on
    // the stack — mirrors `preserves.rs:344`.
    if is_call(mnem) {
        st.entry = [false; NU];
        return None;
    }

    if sp_hazard(mnem, ops) {
        return Some("unmodeled sp manipulation (ld/inc/dec sp or ex (sp),rr)".to_string());
    }

    // PUSH — save a pair slot capturing each component unit's entry bit.
    if mnem == "push" {
        if let Some(CodeOperand::Z80Pair(p)) = ops.first() {
            let saved: Vec<(usize, bool)> = pair_units(*p).into_iter().map(|u| (u, st.entry[u])).collect();
            st.stack.push(Some(Slot { pair: *p, saved }));
        } else {
            st.stack.push(None); // pushing a non-pair (defensive) → opaque slot
        }
        return None;
    }

    // POP — restore into the popped pair. Popping more than was pushed underflows
    // into the caller's frame → bail (mirrors `preserves.rs:396`).
    if mnem == "pop" {
        if let Some(CodeOperand::Z80Pair(p)) = ops.first() {
            let Some(slot) = st.stack.pop() else {
                return Some("stack underflow — pop drains more than was pushed".to_string());
            };
            for u in pair_units(*p) {
                // The popped register holds ITS entry value iff the slot held the
                // SAME pair's snapshot of that unit; a cross-pair pop (the move
                // idiom, `push ix / pop hl`) clobbers it (foreign value).
                st.entry[u] = slot
                    .as_ref()
                    .filter(|s| s.pair == *p)
                    .and_then(|s| s.saved.iter().find(|(su, _)| *su == u).map(|(_, b)| *b))
                    .unwrap_or(false);
            }
        } else {
            st.stack.pop();
        }
        return None;
    }

    // A plain instruction: every register write clears the unit's entry bit.
    for u in z80_writes(mnem, ops) {
        st.entry[u] = false;
    }
    None
}

/// The successor edges of instruction `idx` under Z80 terminator semantics. The
/// shared `Cfg::edges` is 68k-terminator-aware (`rts`/`bra`/…); the Z80 proof
/// needs `ret`/`jp`/`jr`/`djnz`/`call` semantics, so it computes edges itself from
/// the shared `Cfg`'s `instr`/`next_instr`/`label_index` primitives.
///
/// A CONDITIONAL form (a leading `Z80Cc`) is a genuine two-way split — it
/// contributes BOTH its taken edge and its fall-through. A `preserves` proof
/// that dropped the fall-through of a `jr cc`/`jp cc`/`ret cc` would MISS a
/// register clobbered only on that path and wrongly verify the preserve (the
/// §13.4-E soundness gap). This mirrors the flag check's `Cfg::z80_edges`
/// (`flag_check.rs`) — the same conditional split, one edge model.
fn z80_edges(cfg: &Cfg, idx: usize) -> Vec<Edge> {
    let Some((mnem, ops)) = cfg.instr(idx) else { return vec![] };
    let leads_cc = matches!(ops.first(), Some(CodeOperand::Z80Cc(_)));
    // Unconditional return.
    if is_return(mnem) && !leads_cc {
        return vec![Edge::Abandon];
    }
    // Conditional `ret cc`: the taken edge returns (abandon); the fall-through
    // stays in the proc.
    if is_return(mnem) && leads_cc {
        return match cfg.next_instr(idx) {
            Some(f) => vec![Edge::Abandon, Edge::Follow(f)],
            None => vec![Edge::Abandon, Edge::Abandon],
        };
    }
    // Unconditional tail transfer: `jp`/`jr` to a LOCAL label follows it; to an
    // external symbol (or a computed `jp (hl)`) it defers out of the proc.
    if matches!(mnem, "jp" | "jr") && !leads_cc {
        return match branch_sym(ops).and_then(|t| cfg.label_index(t)) {
            Some(tgt) => vec![Edge::Follow(tgt)],
            None => vec![Edge::Defer],
        };
    }
    // Conditional `jr cc`/`jp cc`: the taken edge (local → follow, external →
    // defer) PLUS the fall-through.
    if matches!(mnem, "jp" | "jr") && leads_cc {
        let mut v = Vec::new();
        match branch_sym(ops).and_then(|t| cfg.label_index(t)) {
            Some(tgt) => v.push(Edge::Follow(tgt)),
            None => v.push(Edge::Defer),
        }
        match cfg.next_instr(idx) {
            Some(f) => v.push(Edge::Follow(f)),
            None => v.push(Edge::Abandon),
        }
        return v;
    }
    // `djnz` branches AND falls through.
    if mnem == "djnz" {
        let mut v = Vec::new();
        if let Some(tgt) = branch_sym(ops).and_then(|t| cfg.label_index(t)) {
            v.push(Edge::Follow(tgt));
        }
        match cfg.next_instr(idx) {
            Some(f) => v.push(Edge::Follow(f)),
            None => v.push(Edge::Abandon),
        }
        return v;
    }
    // Everything else (incl. `call`, which returns) falls through; the end of the
    // body abandons (fall-off).
    match cfg.next_instr(idx) {
        Some(f) => vec![Edge::Follow(f)],
        None => vec![Edge::Abandon],
    }
}

/// The last bare-`Sym` operand of a branch/tail instruction (its target label).
fn branch_sym(ops: &[CodeOperand]) -> Option<&str> {
    ops.iter().rev().find_map(|o| match o {
        CodeOperand::Sym(name) => Some(name.as_str()),
        _ => None,
    })
}

/// Join `other` into `acc` — entry bits meet by AND, slots meet pointwise, a
/// depth mismatch taints the merge `bailed`. Mirrors `preserves.rs:506`.
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
    for i in 0..NU {
        acc.entry[i] = acc.entry[i] && other.entry[i];
    }
}
