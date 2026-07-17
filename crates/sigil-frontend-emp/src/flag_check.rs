//! Contract-grammar v2 §6 — the caller-side flag-result must-use check.
//!
//! A callee declaring `out(carry: name)` (§6) returns a status flag the caller
//! MUST consume. `[call.flag-result-unused]` verifies that, for every call to a
//! flag-result callee, the carry is READ (a `Bcc`/`Scc`/ADDX-class consumer)
//! before it is REDEFINED (a CC-writing instruction / an intervening call) or
//! the proc RETURNS — on EVERY path. A path that abandons the flag fires, unless
//! the call carries an explicit `@discards(name)`.
//!
//! The analysis is a lightweight CFG over a proc's *evaluated* CodeBuf — the §11
//! Q1 decision: a real CFG with joins (a visited-set breadth-first reachability),
//! never a straight-line approximation (the stale-row-1030 trap). It is
//! deliberately decoupled from the grammar: it consumes a `&[CodeItem]` plus a
//! flag-callee map and a discard set, both of which the corpus walk builds.
//!
//! **Modeling stance (soundness):** the redefine set (`writes_carry`) is a
//! curated ALLOWLIST of CC-writing 68000 operations; an unrecognized mnemonic is
//! treated as CC-TRANSPARENT so the check is false-NEGATIVE-leaning — it never
//! fires on an instruction it does not model. This is what the dplc
//! `movem.l (sp)+` between the call and its `bcs` requires (movem preserves
//! CCR). `sr`/full-CCR liveness stays S2-D7; this is per-call-site carry def-use
//! only (§6 scope fence).

use crate::value::{CodeItem, CodeOperand};
use sigil_span::Span;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// The kind of flag-result violation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlagFiringKind {
    /// `[call.flag-result-unused]` — a flag result reaches a redefine / return /
    /// proc-end on some path without being consumed.
    Unused,
    /// `[call.result-invalid-path]` — a conditional register result
    /// `out(rN if cc)` is read on the path where `cc` says it is invalid (§6,
    /// G2.4). `reg`/`cc` name the offending result and its validity guard.
    InvalidPathRead { reg: String, cc: String },
}

/// One flag-result must-use firing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlagFiring {
    /// The calling proc.
    pub proc: String,
    /// The flag-result callee whose result was abandoned / mis-read.
    pub callee: String,
    /// The flag (`carry`) that went unconsumed.
    pub flag: String,
    /// The call site (for the diagnostic span).
    pub span: Span,
    /// Which check fired.
    pub kind: FlagFiringKind,
}

/// Call/tail mnemonics (both the `.emp` `jbsr`/`jbra` idioms and their resolved
/// `bsr`/`bra` forms may appear in a CodeBuf).
const CALL_MNEMONICS: [&str; 3] = ["jsr", "jbsr", "bsr"];
const UNCOND_MNEMONICS: [&str; 4] = ["bra", "jbra", "jmp", "jra"];
const RETURN_MNEMONICS: [&str; 4] = ["rts", "rte", "rtr", "rtd"];

/// Does this resolved mnemonic CONSUME the carry flag — a reader whose presence
/// discharges the must-use obligation? The carry-testing conditional branches
/// and their set/dbcc forms (`bcs`/`bcc`/`bhi`/`bls` + the `hs`/`lo` aliases),
/// plus the ADDX-class extend readers (spec §6). A branch READS the condition
/// codes without writing them, so a carry-reading branch consumes; a Z-reading
/// branch (`beq`/`bne`) neither consumes nor redefines carry (it just adds CFG
/// edges). Checked BEFORE `writes_carry`, so the read+write rotates
/// (`roxl`/`roxr`) and `negx`/`abcd`/`sbcd` count as consumers.
fn consumes_carry(mnem: &str) -> bool {
    matches!(
        mnem,
        "bcs" | "bcc" | "bhi" | "bls" | "blo" | "bhs"
            | "scs" | "scc" | "shi" | "sls" | "slo" | "shs"
            | "dbcs" | "dbcc" | "dbhi" | "dbls"
            | "addx" | "subx" | "negx" | "abcd" | "sbcd" | "roxl" | "roxr"
    )
}

/// Does this resolved mnemonic REDEFINE (write) the carry flag, ending the
/// must-use window? A curated ALLOWLIST of CC-writing 68000 data operations plus
/// the call mnemonics (a subroutine clobbers CC unless it preserves it, which is
/// not locally provable). NOT here — hence CC-transparent: `movem`/`movea`/`lea`/
/// `pea`/`adda`/`suba`/`exg`/`swap`?/branches/`nop` and the bit tests
/// (`btst`/`bset`/`bclr`/`bchg`, which write only Z). `move` writes CC; `movea`
/// (address-register move) does not — the evaluator spells them distinctly.
fn writes_carry(mnem: &str) -> bool {
    if CALL_MNEMONICS.contains(&mnem) {
        // An intervening call clobbers the condition codes: the tracked carry
        // does not survive it (a `bcs` after an unrelated `jsr` tests the wrong
        // flag). Locally we cannot prove CC-preservation, so a call ends the
        // window. (The flag-result call that STARTS a window is never re-seen —
        // the walk begins at its successor.)
        return true;
    }
    matches!(
        mnem,
        "move" | "moveq" | "clr"
            | "add" | "addi" | "addq"
            | "sub" | "subi" | "subq"
            | "cmp" | "cmpi" | "cmpm" | "cmpa"
            | "and" | "andi" | "or" | "ori" | "eor" | "eori" | "not"
            | "neg" | "muls" | "mulu" | "divs" | "divu"
            | "tst" | "ext" | "extb" | "swap" | "tas"
            | "nbcd"
            | "asl" | "asr" | "lsl" | "lsr" | "rol" | "ror"
    )
}

/// The sole `Sym` operand of a branch/tail instruction (its target label), if
/// any. `None` for a register-indirect or multi-operand form.
fn branch_target(ops: &[CodeOperand]) -> Option<&str> {
    match ops.first() {
        Some(CodeOperand::Sym(name)) => Some(name.as_str()),
        _ => None,
    }
}

/// A resolved per-proc control-flow view over a CodeBuf's items.
struct Cfg<'a> {
    items: &'a [CodeItem],
    /// For each item index that is an instruction, the item index of the next
    /// instruction (fall-through), or `None` if it falls off the end.
    next_instr: BTreeMap<usize, usize>,
    /// Label name → the item index of the first instruction at/after it.
    label_target: BTreeMap<String, usize>,
}

impl<'a> Cfg<'a> {
    fn build(items: &'a [CodeItem]) -> Self {
        // The instruction item indices, in order.
        let instrs: Vec<usize> = items
            .iter()
            .enumerate()
            .filter(|(_, it)| matches!(it, CodeItem::Instr { .. }))
            .map(|(i, _)| i)
            .collect();
        let mut next_instr = BTreeMap::new();
        for w in instrs.windows(2) {
            next_instr.insert(w[0], w[1]);
        }
        // A label targets the first instruction at/after its position.
        let mut label_target = BTreeMap::new();
        for (i, it) in items.iter().enumerate() {
            if let CodeItem::Label { name, .. } = it {
                if let Some(&tgt) = instrs.iter().find(|&&j| j >= i) {
                    label_target.insert(name.clone(), tgt);
                }
            }
        }
        Cfg { items, next_instr, label_target }
    }

    /// The instruction at item index `idx`, as `(mnemonic, ops)`.
    fn instr(&self, idx: usize) -> Option<(&str, &[CodeOperand])> {
        match &self.items[idx] {
            CodeItem::Instr { mnemonic, ops, .. } => Some((mnemonic.as_str(), ops)),
            _ => None,
        }
    }

    /// The successor edges of the instruction at `idx`, for carry-tracking. An
    /// edge is either `Follow(next_idx)` (stay in the proc) or `Abandon` (the
    /// flag is left unconsumed: a return, a fall-off-end, or a redefine reached).
    /// A call/tail transfer to an EXTERNAL target (not a local label) is
    /// `Defer` — the flag flows out of this proc and local analysis cannot judge
    /// it, so it neither follows nor abandons.
    fn edges(&self, idx: usize) -> Vec<Edge> {
        let Some((mnem, ops)) = self.instr(idx) else { return vec![] };
        if RETURN_MNEMONICS.contains(&mnem) {
            return vec![Edge::Abandon];
        }
        let fallthrough = self.next_instr.get(&idx).copied();
        if UNCOND_MNEMONICS.contains(&mnem) {
            // An unconditional tail transfer: to a LOCAL label → follow it; to an
            // external symbol (a tail call) → defer.
            return match branch_target(ops).and_then(|t| self.label_target.get(t)) {
                Some(&tgt) => vec![Edge::Follow(tgt)],
                None => vec![Edge::Defer],
            };
        }
        // A conditional branch (`bXX`/`dbXX`) that is NOT a carry consumer:
        // fall-through PLUS the taken edge. (Carry consumers are handled by the
        // caller before edges() is consulted.) `dbf`/`dbra` (Cond::F) and
        // Z/N/V-testing branches land here.
        let is_cond_branch = (mnem.starts_with('b') && mnem.len() == 3)
            || mnem.starts_with("db");
        if is_cond_branch {
            let mut v = Vec::new();
            match branch_target(ops).and_then(|t| self.label_target.get(t)) {
                Some(&tgt) => v.push(Edge::Follow(tgt)),
                None => v.push(Edge::Defer), // branch to external symbol
            }
            match fallthrough {
                Some(f) => v.push(Edge::Follow(f)),
                None => v.push(Edge::Abandon),
            }
            return v;
        }
        // A plain instruction: fall through, or abandon if it falls off the end.
        match fallthrough {
            Some(f) => vec![Edge::Follow(f)],
            None => vec![Edge::Abandon],
        }
    }
}

/// A carry-tracking control-flow edge (see [`Cfg::edges`]).
enum Edge {
    Follow(usize),
    Abandon,
    Defer,
}

/// Run `[call.flag-result-unused]` over one proc's evaluated CodeBuf `items`.
/// For each call to a `flag_callees` member, verify every path consumes the
/// flag before a redefine / return. `discarded` is the set of call-site spans
/// carrying `@discards` (matched against the CodeBuf instruction's source span).
pub fn check_flag_unused(
    proc_name: &str,
    items: &[CodeItem],
    flag_callees: &BTreeMap<String, BTreeSet<String>>,
    discarded: &[Span],
) -> Vec<FlagFiring> {
    let cfg = Cfg::build(items);
    let mut firings = Vec::new();

    for (idx, it) in items.iter().enumerate() {
        let CodeItem::Instr { mnemonic, ops, span, .. } = it else { continue };
        if !CALL_MNEMONICS.contains(&mnemonic.as_str()) {
            continue;
        }
        // A DIRECT call whose sole operand is a bare symbol naming a flag-result
        // callee. (`branch_target` returns that symbol; an indirect `jsr (aN)`
        // has no bare Sym operand and is skipped.)
        let Some(callee) = branch_target(ops) else { continue };
        let Some(flags) = flag_callees.get(callee) else { continue };
        // The explicit opt-out.
        if discarded.contains(span) {
            continue;
        }
        // The carry flag is the only §6 must-use flag today; a callee may in
        // principle return several. Fire once per unconsumed flag.
        for flag in flags {
            if flag != "carry" {
                continue; // only carry has a consumer model today
            }
            if abandons_flag(&cfg, idx) {
                firings.push(FlagFiring {
                    proc: proc_name.to_string(),
                    callee: callee.to_string(),
                    flag: flag.clone(),
                    span: *span,
                    kind: FlagFiringKind::Unused,
                });
            }
        }
    }
    firings
}

/// Breadth-first reachability from the successors of the call at `call_idx`: is
/// there a path that REACHES a redefine / return / proc-end (an `Abandon`)
/// without first crossing a carry consumer? Consumers PRUNE (that path is
/// satisfied); a `Defer` edge (tail call to an external symbol) also prunes (the
/// flag flows out of the proc — not a local abandonment). The visited set gives
/// the CFG real joins so loops terminate.
fn abandons_flag(cfg: &Cfg, call_idx: usize) -> bool {
    let mut visited: BTreeSet<usize> = BTreeSet::new();
    let mut queue: VecDeque<Edge> = VecDeque::new();
    // Seed from the call's own fall-through (the call is never re-examined).
    for e in cfg.edges(call_idx) {
        queue.push_back(e);
    }
    while let Some(edge) = queue.pop_front() {
        let idx = match edge {
            Edge::Abandon => return true, // a path abandons the flag
            Edge::Defer => continue,      // flows out of the proc — not local
            Edge::Follow(i) => i,
        };
        if !visited.insert(idx) {
            continue; // join / back-edge already explored
        }
        let Some((mnem, _)) = cfg.instr(idx) else { continue };
        if consumes_carry(mnem) {
            continue; // this path is satisfied
        }
        if writes_carry(mnem) {
            return true; // carry redefined before any consumer
        }
        for e in cfg.edges(idx) {
            queue.push_back(e);
        }
    }
    false
}
