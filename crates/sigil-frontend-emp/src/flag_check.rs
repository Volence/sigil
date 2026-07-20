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

use crate::calls::call_unconditional_outs;
use crate::lower::instr_written_regs;
use crate::out_verify::cc_transparent;
use crate::value::{CodeItem, CodeOperand, Reg};
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
/// discharges the must-use obligation? ONLY the carry-testing conditional
/// branches and their set/dbcc forms (`bcs`/`bcc`/`bhi`/`bls` + the `hs`/`lo`
/// aliases): a branch READS the condition codes without writing them, so a
/// carry-reading branch consumes; a Z-reading branch (`beq`/`bne`) neither
/// consumes nor redefines carry (it just adds CFG edges).
///
/// The ADDX-class (`addx`/`subx`/`negx`/`abcd`/`sbcd`/`roxl`/`roxr`) is
/// DELIBERATELY NOT here (G2.6 Fable rider): those read the EXTEND flag (X), not
/// the callee's carry (C), and they CLOBBER C — so for a carry result they are
/// redefines (`writes_carry`), not consumers. (The spec's "ADDX-class consumer"
/// language is about an `out(extend:)` result; a carry result is discharged only
/// by a carry-reading branch.)
fn consumes_carry(mnem: &str) -> bool {
    matches!(
        mnem,
        "bcs" | "bcc" | "bhi" | "bls" | "blo" | "bhs"
            | "scs" | "scc" | "shi" | "sls" | "slo" | "shs"
            | "dbcs" | "dbcc" | "dbhi" | "dbls"
    )
}

/// Does this resolved mnemonic REDEFINE (write) the carry flag, ending the
/// must-use window? A curated ALLOWLIST of CC-writing 68000 data operations plus
/// the call mnemonics (a subroutine clobbers CC unless it preserves it, which is
/// not locally provable). Includes the ADDX-class (`addx`/`subx`/`negx`/`abcd`/
/// `sbcd`/`roxl`/`roxr`): they read X but WRITE C, so an `addx` between a call
/// and its `bcs` ends the real window (G2.6 rider). Move-to-ccr/move-to-sr are
/// caught by [`writes_ccr_operand`] (operand-directed, independent of the
/// mnemonic).
///
/// NOT here — hence CC-TRANSPARENT: `movem`/`movea`/`lea`/`pea`/`adda`/`suba`/
/// `exg`/branches/`nop`, and — DELIBERATELY — the bit tests `btst`/`bset`/
/// `bclr`/`bchg`, which write ONLY the Z flag and never touch C (do not "fix"
/// this by adding them). `move` writes CC; `movea` (address-register move) does
/// not — the evaluator spells them distinctly.
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
            | "add" | "addi" | "addq" | "addx"
            | "sub" | "subi" | "subq" | "subx"
            | "cmp" | "cmpi" | "cmpm" | "cmpa"
            | "and" | "andi" | "or" | "ori" | "eor" | "eori" | "not"
            | "neg" | "negx" | "muls" | "mulu" | "divs" | "divu"
            | "tst" | "ext" | "extb" | "swap" | "tas"
            | "nbcd" | "abcd" | "sbcd"
            | "asl" | "asr" | "lsl" | "lsr" | "rol" | "ror" | "roxl" | "roxr"
    )
}

/// A redefine reached through the OPERAND: an instruction whose destination is
/// CCR or SR writes the carry directly (`move #imm, ccr` / `move #imm, sr` /
/// `andi/ori/eori #imm, ccr|sr`). Operand-directed so it holds regardless of how
/// the mnemonic is classified (G2.6 rider — the move-to-ccr/sr forms).
fn writes_ccr_operand(ops: &[CodeOperand]) -> bool {
    matches!(ops.last(), Some(CodeOperand::Ccr) | Some(CodeOperand::Sr))
}

/// The target label of a branch/tail/call instruction — the LAST `Sym` operand.
/// For most forms (`bcc label`, `bra label`, `jbsr Callee`) the label is the
/// sole/first operand; for the `dbcc dN, label` counting-loop form it is the
/// SECOND (the register comes first), so scanning from the end catches both.
/// `None` for a register-indirect form (`jsr (a1)`) with no symbolic target.
fn branch_target(ops: &[CodeOperand]) -> Option<&str> {
    ops.iter().rev().find_map(|o| match o {
        CodeOperand::Sym(name) => Some(name.as_str()),
        _ => None,
    })
}

/// A resolved per-proc control-flow view over a CodeBuf's items. Exposed
/// `pub(crate)` so the §5 verified-`preserves` dataflow ([`crate::preserves`])
/// REUSES this exact CFG substrate (spec §11 Q1: extend G2's CFG, do not
/// duplicate) — same `next_instr`/`label_target`/`edges` joins.
pub(crate) struct Cfg<'a> {
    items: &'a [CodeItem],
    /// For each item index that is an instruction, the item index of the next
    /// instruction (fall-through), or `None` if it falls off the end.
    next_instr: BTreeMap<usize, usize>,
    /// Label name → the item index of the first instruction at/after it.
    label_target: BTreeMap<String, usize>,
}

impl<'a> Cfg<'a> {
    pub(crate) fn build(items: &'a [CodeItem]) -> Self {
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
    pub(crate) fn instr(&self, idx: usize) -> Option<(&str, &[CodeOperand])> {
        match &self.items[idx] {
            CodeItem::Instr { mnemonic, ops, .. } => Some((mnemonic.as_str(), ops)),
            _ => None,
        }
    }

    /// The fall-through instruction index after `idx` (the textually next
    /// instruction), or `None` at the end of the body. Exposed for the §G4.5
    /// out-verifier's branch-split (distinguishing a conditional branch's taken
    /// edge from its fall-through).
    pub(crate) fn next_instr(&self, idx: usize) -> Option<usize> {
        self.next_instr.get(&idx).copied()
    }

    /// The instruction index a LOCAL label targets, or `None` for a non-local /
    /// unknown symbol. Exposed for the §G4.5 out-verifier's branch-split.
    pub(crate) fn label_index(&self, name: &str) -> Option<usize> {
        self.label_target.get(name).copied()
    }

    /// The successor edges of the instruction at `idx`, for carry-tracking. An
    /// edge is either `Follow(next_idx)` (stay in the proc) or `Abandon` (the
    /// flag is left unconsumed: a return, a fall-off-end, or a redefine reached).
    /// A call/tail transfer to an EXTERNAL target (not a local label) is
    /// `Defer` — the flag flows out of this proc and local analysis cannot judge
    /// it, so it neither follows nor abandons.
    pub(crate) fn edges(&self, idx: usize) -> Vec<Edge> {
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

    /// From the call at `call_idx`, walk the fall-through chain to the first
    /// branch that tests `cc` (or its negation) and return the item index that
    /// begins the INVALID edge (where `cc` does NOT hold). `None` when the guard
    /// is redefined first, the path returns, or an unrelated branch is reached —
    /// forward machinery bails rather than guess.
    fn invalid_edge(&self, call_idx: usize, cc: &str) -> Option<usize> {
        let neg = negate_cc(cc)?;
        let mut idx = *self.next_instr.get(&call_idx)?;
        loop {
            let (mnem, ops) = self.instr(idx)?;
            if let Some(bc) = branch_cond(mnem) {
                let taken = branch_target(ops).and_then(|t| self.label_target.get(t)).copied();
                let fall = self.next_instr.get(&idx).copied();
                return if bc == cc {
                    fall // cc holds on the taken edge → fall-through is INVALID
                } else if bc == neg {
                    taken // cc holds on the fall-through → the taken edge is INVALID
                } else {
                    None // an unrelated branch — bail
                };
            }
            if RETURN_MNEMONICS.contains(&mnem) || writes_carry(mnem) || writes_ccr_operand(ops) {
                return None; // guard never tested (returned / CC redefined)
            }
            idx = *self.next_instr.get(&idx)?; // fall through to the next instr
        }
    }

    /// From the call at `call_idx`, walk the fall-through chain to the guard branch
    /// testing `cc` and return `(guard_idx, success_idx)` — the guard instruction
    /// and the item index that BEGINS the cc-SUCCESS edge (where `cc` provably
    /// HOLDS). The edge-identification primitive for item #2's conditional-out
    /// crediting; the MIRROR of [`Self::invalid_edge`] but with the opposite
    /// conservative default and a SOUND-COMPLETE bail (the corrected spec banner):
    ///
    /// - The intervening-clobber bail is [`crate::out_verify::cc_transparent`], NOT
    ///   [`writes_carry`]: #2's cc is `eq`(Z), so a Z-only writer
    ///   (`btst`/`bset`/`bclr`/`bchg`) or ANY unmodeled mnemonic between the call
    ///   and the guard must BAIL. `writes_carry` deliberately treats those as
    ///   transparent (sound for §6's over-fire polarity, a false negative here).
    /// - **Exact-cc fence:** credit only when the guard tests the callee's EXACT
    ///   `cc` (success = the taken edge) or its EXACT negation (success = the
    ///   fall-through). Any other — even a correlated condition — bails.
    /// - A return or an unconditional transfer (`bra`/`jmp`/`jra`/`jbra`) before
    ///   the guard diverts / ends the straight-line path → bail.
    ///
    /// `None` on ANY bail. The load-bearing rule (§2): bail → the caller does NOT
    /// credit → a residual false positive may remain (acceptable), never a silent
    /// must-def miss. `Scc`/`dbcc` are not guards here — they fall to the
    /// transparency check and bail (neither is CC-transparent).
    pub(crate) fn valid_edge(&self, call_idx: usize, cc: &str) -> Option<(usize, usize)> {
        let neg = negate_cc(cc)?; // canonical negation
        let cc = negate_cc(neg)?; // canonical cc (double-negate folds hs/lo aliases)
        let mut idx = *self.next_instr.get(&call_idx)?;
        loop {
            let (mnem, ops) = self.instr(idx)?;
            // A real conditional branch (`bXX`, 3-char) is the candidate guard.
            if mnem.starts_with('b') && mnem.len() == 3 {
                let Some(bc) = branch_cond(mnem) else {
                    return None; // `bra` — unconditional; control diverts before a guard
                };
                let taken = branch_target(ops).and_then(|t| self.label_target.get(t)).copied();
                let fall = self.next_instr.get(&idx).copied();
                return if bc == cc {
                    taken.map(|t| (idx, t)) // cc holds on the TAKEN edge → success = taken
                } else if bc == neg {
                    fall.map(|f| (idx, f)) // cc holds on the FALL-THROUGH → success = fall
                } else {
                    None // an unrelated / correlated-but-different condition — bail
                };
            }
            // Not a guard: to keep walking the straight-line fall-through the
            // instruction must be PROVABLY CC-transparent and must not return or
            // divert. UNCOND is checked before the transparency allowlist because
            // `cc_transparent` treats `jmp`/`jra` as transparent (they don't WRITE
            // the CC) — but they still divert control off the fall-through.
            if RETURN_MNEMONICS.contains(&mnem) {
                return None;
            }
            if UNCOND_MNEMONICS.contains(&mnem) {
                return None; // jmp/jra/jbra divert — the guard is not straight-line-reachable
            }
            if !cc_transparent(mnem) {
                return None; // a Z-clobber (btst/…) or unmodeled mnemonic — sound-complete bail
            }
            idx = *self.next_instr.get(&idx)?; // fall through
        }
    }
}

/// For every direct CALL in `items` to a callee declaring conditional outs
/// (`cond_callees`), identify the caller's cc-SUCCESS edge via
/// [`Cfg::valid_edge`] and map that edge `(guard_idx, succ_idx)` to the credited
/// register(s). The SHARED edge-credit primitive (spec §4): must-def (D1b) and
/// the out-verifier both consume this so they cannot disagree on which edge is
/// cc-success. Keyed by the EDGE, not the successor node — each consumer applies
/// these as a per-edge transfer into its OWN forward must-analysis and re-joins
/// by intersection at merges (§3). A `valid_edge` bail contributes nothing (the
/// conservative default). Register names are canonicalized to the `d0`..`a7`
/// spelling the def/produce sets use.
pub(crate) fn conditional_out_edge_credits(
    cfg: &Cfg,
    items: &[CodeItem],
    cond_callees: &BTreeMap<String, Vec<(String, String)>>,
) -> BTreeMap<(usize, usize), BTreeSet<String>> {
    let mut credits: BTreeMap<(usize, usize), BTreeSet<String>> = BTreeMap::new();
    for (idx, it) in items.iter().enumerate() {
        let CodeItem::Instr { mnemonic, ops, .. } = it else { continue };
        if !CALL_MNEMONICS.contains(&mnemonic.as_str()) {
            continue;
        }
        let Some(callee) = branch_target(ops) else { continue };
        let Some(conds) = cond_callees.get(callee) else { continue };
        for (reg, cc) in conds {
            let Some(reg) = Reg::from_name(reg) else { continue };
            if let Some(edge) = cfg.valid_edge(idx, cc) {
                credits.entry(edge).or_default().insert(reg.to_string());
            }
        }
    }
    credits
}

/// A carry-tracking control-flow edge (see [`Cfg::edges`]).
pub(crate) enum Edge {
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

// ---------------------------------------------------------------------------
// §6 / G2.4 — [call.result-invalid-path] for out(rN if cc) conditional register
// results. D2.35's deferred sibling, riding the SAME CFG. A conditional
// register result `rN` is valid only on the path where the guard `cc` holds;
// reading `rN` on the other (invalid) path is an error. Forward machinery: no
// corpus site declares a conditional register result today (like G1's
// subcontract check — built + TDD'd against synthetic cases, inert on the real
// corpus until the first such contract appears).
// ---------------------------------------------------------------------------

/// The condition a `bXX`/`sXX` branch/set tests, stripped of the mnemonic prefix
/// (`bcc`→`cc`, `bhs`→`cc`, `blo`→`cs`, `beq`→`eq`, …). `None` for a non-branch,
/// an unconditional `bra`, or `dbf`/`dbra` (Cond::F).
fn branch_cond(mnem: &str) -> Option<&'static str> {
    let bare = mnem.strip_prefix('b').or_else(|| mnem.strip_prefix('s'))?;
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

/// The negation of a condition code (`cc`↔`cs`, `eq`↔`ne`, …). Canonicalizes the
/// `hs`/`lo` aliases to `cc`/`cs` first.
fn negate_cc(cc: &str) -> Option<&'static str> {
    Some(match cc {
        "cc" | "hs" => "cs",
        "cs" | "lo" => "cc",
        "eq" => "ne",
        "ne" => "eq",
        "hi" => "ls",
        "ls" => "hi",
        "pl" => "mi",
        "mi" => "pl",
        "vc" => "vs",
        "vs" => "vc",
        "ge" => "lt",
        "lt" => "ge",
        "gt" => "le",
        "le" => "gt",
        _ => return None,
    })
}

/// Every register a `move`/EA operand list MENTIONS (any position, incl. an
/// indirect base or index), so `mentioned − written` is the READ set.
fn regs_mentioned(ops: &[CodeOperand]) -> Vec<Reg> {
    let mut regs = Vec::new();
    let mut push = |r: Reg| {
        if !regs.contains(&r) {
            regs.push(r);
        }
    };
    for op in ops {
        match op {
            CodeOperand::Reg(r)
            | CodeOperand::Ind(r)
            | CodeOperand::PreDec(r)
            | CodeOperand::PostInc(r)
            | CodeOperand::DispInd { reg: r, .. } => push(*r),
            CodeOperand::IndIdx { reg, xn, .. } => {
                push(*reg);
                push(*xn);
            }
            _ => {}
        }
    }
    regs
}

/// Run `[call.result-invalid-path]` over one proc's CodeBuf. For each call to a
/// callee declaring `out(rN if cc)` results, find the branch that tests `cc`,
/// take the INVALID edge (where `cc` does not hold), and fire if `rN` is read
/// there before it is redefined.
pub fn check_result_invalid_path(
    proc_name: &str,
    items: &[CodeItem],
    cond_callees: &BTreeMap<String, Vec<(String, String)>>,
    callee_uncond_out: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<FlagFiring> {
    let cfg = Cfg::build(items);
    let mut firings = Vec::new();

    for (idx, it) in items.iter().enumerate() {
        let CodeItem::Instr { mnemonic, ops, span, .. } = it else { continue };
        if !CALL_MNEMONICS.contains(&mnemonic.as_str()) {
            continue;
        }
        let Some(callee) = branch_target(ops) else { continue };
        let Some(conds) = cond_callees.get(callee) else { continue };
        for (reg_name, cc) in conds {
            let Some(reg) = Reg::from_name(reg_name) else { continue };
            let Some(invalid_start) = cfg.invalid_edge(idx, cc) else { continue };
            if reads_reg_before_redefine(&cfg, invalid_start, reg, callee_uncond_out) {
                firings.push(FlagFiring {
                    proc: proc_name.to_string(),
                    callee: callee.to_string(),
                    flag: cc.clone(),
                    span: *span,
                    kind: FlagFiringKind::InvalidPathRead {
                        reg: reg_name.clone(),
                        cc: cc.clone(),
                    },
                });
            }
        }
    }
    firings
}

/// Breadth-first: does any path from `start` READ `reg` (as a source / address
/// base) before `reg` is redefined (written) or the path exits? Visited-set for
/// joins.
fn reads_reg_before_redefine(
    cfg: &Cfg,
    start: usize,
    reg: Reg,
    callee_uncond_out: &BTreeMap<String, BTreeSet<String>>,
) -> bool {
    let mut visited: BTreeSet<usize> = BTreeSet::new();
    let mut queue: VecDeque<usize> = VecDeque::from([start]);
    while let Some(idx) = queue.pop_front() {
        if !visited.insert(idx) {
            continue;
        }
        let Some((mnem, ops)) = cfg.instr(idx) else { continue };
        let written = instr_written_regs(mnem, ops);
        let mentioned = regs_mentioned(ops);
        // A READ = mentioned but not (only) written this instruction.
        if mentioned.contains(&reg) && !written.contains(&reg) {
            return true;
        }
        // A CALL that UNCONDITIONALLY redefines reg kills the conditional taint on
        // this path (the SAME shared fact must-def credits as a definition): reg
        // holds a produced value on every return edge, so a downstream read sees
        // the fresh value, not the invalid-path trash. UNCONDITIONAL only — a
        // conditional out(rM if cc2) is still trash on its !cc2 edge and must
        // NOT count as a redefine (else a real invalid-path read ships unflagged).
        if call_unconditional_outs(mnem, ops, callee_uncond_out)
            .is_some_and(|outs| outs.contains(&reg.to_string()))
        {
            continue;
        }
        // A pure redefine kills the invalid result on this path.
        if written.contains(&reg) {
            continue;
        }
        for e in cfg.edges(idx) {
            if let Edge::Follow(i) = e {
                queue.push_back(i);
            }
            // Abandon / Defer: the path leaves without a read — safe here.
        }
    }
    false
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
        let Some((mnem, ops)) = cfg.instr(idx) else { continue };
        if consumes_carry(mnem) {
            continue; // this path is satisfied
        }
        if writes_carry(mnem) || writes_ccr_operand(ops) {
            return true; // carry redefined before any consumer
        }
        for e in cfg.edges(idx) {
            queue.push_back(e);
        }
    }
    false
}
