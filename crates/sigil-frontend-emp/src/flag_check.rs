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
use crate::value::{CodeItem, CodeOperand, Reg, Z80Cond};
use sigil_ir::backend::Cpu;
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
const Z80_RETURN_MNEMONICS: [&str; 3] = ["ret", "reti", "retn"];

/// Does `mnem` RETURN from the proc? **An edge BUILDER's classifier**, plus one
/// caller with no CFG to ask: a walk over a straight-line item slice
/// ([`crate::z80_cycles::span_cost`]) has no edges, so it tests the mnemonic
/// directly and must name the CPU whose terminator set it means. Anyone holding
/// an [`Edge`] reads [`Edge::Return`] instead — a builder already decided, with
/// the CPU in hand.
pub(crate) fn is_return_mnemonic(mnem: &str, cpu: Cpu) -> bool {
    match cpu {
        Cpu::Z80 => Z80_RETURN_MNEMONICS.contains(&mnem),
        _ => RETURN_MNEMONICS.contains(&mnem),
    }
}

/// The item index of a body's FIRST instruction — the entry point every per-proc
/// walk over a `CodeBuf` starts from. `None` for a body with no instructions at
/// all (labels and inline data only).
pub(crate) fn entry_instr_idx(items: &[CodeItem]) -> Option<usize> {
    items.iter().position(|it| matches!(it, CodeItem::Instr { .. }))
}

/// The span of the instruction at item index `idx`, or `None` when that item is
/// not an instruction.
pub(crate) fn instr_span(items: &[CodeItem], idx: usize) -> Option<Span> {
    match items.get(idx) {
        Some(CodeItem::Instr { span, .. }) => Some(*span),
        _ => None,
    }
}

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
fn consumes_carry(mnem: &str, ops: &[CodeOperand], cpu: Cpu) -> bool {
    // Z80 (rung-2 §13.3 sub-part 2): a carry-testing conditional control-flow
    // form (`jr c`/`jr nc`/`ret c`/`jp nc`/`call c`) READS carry — the Z80
    // sibling of the 68k `bcs`/`bcc` consumers. The condition rides the leading
    // `Z80Cc` operand (the §13.3 producer), so the mnemonic alone is
    // insufficient. NEW Z80 arm; the 68k allowlist below is byte-unchanged.
    if cpu == Cpu::Z80 {
        return z80_reads_carry(mnem, ops);
    }
    matches!(
        mnem,
        "bcs" | "bcc" | "bhi" | "bls" | "blo" | "bhs"
            | "scs" | "scc" | "shi" | "sls" | "slo" | "shs"
            | "dbcs" | "dbcc" | "dbhi" | "dbls"
    )
}

/// A Z80 carry-testing control-flow form CONSUMES carry: `jr`/`jp`/`call`/`ret`
/// whose leading `Z80Cc` is `c` (carry set) or `nc` (carry clear). A `z`/`nz`
/// (Zero) form tests a DIFFERENT flag — it neither consumes nor redefines carry.
fn z80_reads_carry(mnem: &str, ops: &[CodeOperand]) -> bool {
    matches!(mnem, "jr" | "jp" | "call" | "ret")
        && matches!(ops.first(), Some(CodeOperand::Z80Cc(Z80Cond::C | Z80Cond::Nc)))
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
fn writes_carry(mnem: &str, cpu: Cpu) -> bool {
    // Z80 (rung-2 §13.3 sub-part 2): its own carry-writer allowlist (incl. an
    // intervening `call`/`rst`, which clobbers carry — no local callee CC
    // contract). NEW Z80 arm; the 68k `CALL_MNEMONICS`/allowlist below is
    // byte-unchanged.
    if cpu == Cpu::Z80 {
        return z80_writes_carry(mnem);
    }
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

/// The Z80 carry-writer allowlist — the mnemonics that REDEFINE carry, ending a
/// flag-result must-use window (the Z80 sibling of the 68k [`writes_carry`]
/// allowlist). A curated set (false-negative-leaning, like the 68k detector: an
/// unmodeled mnemonic is CC-transparent). Includes the ALU/shift/rotate carry
/// writers, `scf`/`ccf`, and `call`/`rst` (a callee clobbers carry). NOT here —
/// hence transparent: `ld`/`push`/`pop`/`inc`/`dec` (8-bit inc/dec leave carry,
/// 16-bit inc/dec touch no flags), `bit`/`set`/`res`, `cpl` (writes H/N, not C).
fn z80_writes_carry(mnem: &str) -> bool {
    matches!(
        mnem,
        "add" | "adc" | "sub" | "sbc" | "cp" | "and" | "or" | "xor"
            | "neg" | "daa" | "ccf" | "scf"
            | "rlca" | "rrca" | "rla" | "rra"
            | "rlc" | "rrc" | "rl" | "rr" | "sla" | "sra" | "srl" | "sll"
            | "call" | "rst"
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
pub(crate) fn branch_target(ops: &[CodeOperand]) -> Option<&str> {
    ops.iter().rev().find_map(|o| match o {
        CodeOperand::Sym(name) => Some(name.as_str()),
        _ => None,
    })
}

/// The link symbol a transfer NAMES, across every direct-target spelling: the
/// bare `Sym` a branch/tail takes, plus the pinned/offset absolute forms
/// (`jmp (Sym).l` → `AbsSym`, `jmp Item.field` → `SymOff`) the abs seam lowers.
/// Wider than [`branch_target`] (which sees only `Sym`), so a `@noreturn` match
/// or a computed-target test reads `jmp (Diverge).l` the same as `jbra Diverge`.
/// `None` for a register-indirect (`jsr (a1)`) or a computed dispatch.
pub(crate) fn transfer_target_sym(ops: &[CodeOperand]) -> Option<&str> {
    ops.iter().rev().find_map(|o| match o {
        CodeOperand::Sym(name) => Some(name.as_str()),
        CodeOperand::SymOff { sym, .. } => Some(sym.as_str()),
        CodeOperand::AbsSym { target, .. } => Some(target.as_str()),
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

    /// Whether `name` labels this proc's OWN body — any label DEFINED among these
    /// items, INCLUDING a body-closing one at the very end with no following
    /// instruction, which [`Cfg::label_index`] returns `None` for. The shared
    /// [`Cfg::branch_edge`] three-way needs this on BOTH CPUs to tell a jump to a
    /// body-closing local label (a fall-off exit) from a transfer to an EXTERNAL
    /// symbol (a tail call): a `jbra .end` / `jr cc, .end` whose `.end:` closes
    /// the proc must not be read as an external tail transfer.
    pub(crate) fn is_local_label(&self, name: &str) -> bool {
        self.items.iter().any(|it| matches!(it, CodeItem::Label { name: n, .. } if n == name))
    }

    /// The successor edges of the instruction at `idx` under 68k terminator
    /// semantics: [`Edge::Follow`] stays in the proc, [`Edge::Return`] is one of
    /// `RETURN_MNEMONICS`, [`Edge::FallOff`] is control running past the last
    /// instruction, and the transfer-out pair leaves for a target local analysis
    /// cannot judge — [`Edge::TailOut`] from an unconditional terminator,
    /// [`Edge::BranchOut`] from a conditional one's taken side.
    ///
    /// A branch's taken edge is resolved by the shared [`Self::branch_edge`]
    /// three-way, so a trailing local label — one that CLOSES the body with no
    /// instruction after it — is a [`Edge::FallOff`], NOT a transfer out:
    /// control reaches the fall-off point where this proc's own analysis still
    /// applies, and that is a deliberate, named case, not an external transfer.
    ///
    /// The variant choice is made in an edge BUILDER and nowhere else — every
    /// consumer reads it off the edge.
    pub(crate) fn edges(&self, idx: usize) -> Vec<Edge> {
        let Some((mnem, ops)) = self.instr(idx) else { return vec![] };
        if RETURN_MNEMONICS.contains(&mnem) {
            return vec![Edge::Return];
        }
        let fallthrough = self.next_instr.get(&idx).copied();
        if UNCOND_MNEMONICS.contains(&mnem) {
            // An unconditional tail transfer, resolved by the shared three-way: a
            // local label → follow it; a local label that CLOSES the body → fall
            // off it; anything else leaves the proc unconditionally → `TailOut`.
            return vec![self.branch_edge(ops, OutFlavor::TailOut)];
        }
        // A conditional branch (`bXX`/`dbXX`) that is NOT a carry consumer:
        // fall-through PLUS the taken edge. (Carry consumers are handled by the
        // caller before edges() is consulted.) `dbf`/`dbra` (Cond::F) and
        // Z/N/V-testing branches land here.
        //
        // `bsr` is spelled like one (three letters, leading `b`) and is NOT one:
        // it CALLS and comes back, so its only successor is the fall-through.
        // Giving it the branch's taken edge would splice the callee's body into
        // this proc's flow at the caller's state — for a LOCAL `bsr .helper` that
        // analyzes the helper with the caller's stack still on it.
        let is_cond_branch = (mnem.starts_with('b') && mnem.len() == 3 && mnem != "bsr")
            || mnem.starts_with("db");
        if is_cond_branch {
            // The taken edge goes through the shared three-way (out of the body →
            // `BranchOut`, trailing local → fall off, in-body local → follow); the
            // fall-through is the next instruction, or the end of the body.
            let mut v = vec![self.branch_edge(ops, OutFlavor::BranchOut)];
            match fallthrough {
                Some(f) => v.push(Edge::Follow(f)),
                None => v.push(Edge::FallOff),
            }
            return v;
        }
        // A plain instruction: fall through, or run off the end of the body.
        match fallthrough {
            Some(f) => vec![Edge::Follow(f)],
            None => vec![Edge::FallOff],
        }
    }

    /// The Z80 successor edges of instruction `idx` — the CPU-parametric sibling
    /// of [`Self::edges`] (rung-2 §13.3). The shared `edges` bakes in the 68k
    /// terminators (`rts`/`bra`/…); Z80 has its own (`ret`/`reti`/`retn`,
    /// `jp`/`jr`, conditional `jr cc`/`jp cc`/`ret cc`, `djnz`), so the
    /// carry-tracking walk consults THIS when the proc is a Z80 module — leaving
    /// the 68k `edges` byte-untouched. A conditional BRANCH (a leading `Z80Cc`
    /// on a `jp`/`jr`) contributes BOTH its taken and fall-through edges (a
    /// carry-CONSUMING conditional is pruned by the caller before `z80_edges` is
    /// reached, so only Z/parity-testing conditionals arrive here, each a genuine
    /// two-way split). `call cc` is NOT such a form — it calls and comes back, so
    /// its only successor is the fall-through. An external `jp`/`jr` target is a
    /// transfer out (the flag flows out): [`Edge::TailOut`] unconditionally,
    /// [`Edge::BranchOut`] on a `jp cc`/`jr cc`/`djnz` taken leg.
    pub(crate) fn z80_edges(&self, idx: usize) -> Vec<Edge> {
        let Some((mnem, ops)) = self.instr(idx) else { return vec![] };
        let leads_cc = matches!(ops.first(), Some(CodeOperand::Z80Cc(_)));
        let fallthrough = self.next_instr.get(&idx).copied();
        // Unconditional return.
        if is_return_mnemonic(mnem, Cpu::Z80) && !leads_cc {
            return vec![Edge::Return];
        }
        // Conditional `ret cc`: the TAKEN edge returns; the fall-through stays in
        // the proc, or runs off the end when the `ret cc` closes the body. The two
        // ends of that pair are different facts, and the pair names them
        // separately so no rule keyed on the shared mnemonic can close both.
        if is_return_mnemonic(mnem, Cpu::Z80) && leads_cc {
            return match fallthrough {
                Some(f) => vec![Edge::Return, Edge::Follow(f)],
                None => vec![Edge::Return, Edge::FallOff],
            };
        }
        // Unconditional `jp`/`jr`: the taken edge, classified by its target.
        if matches!(mnem, "jp" | "jr") && !leads_cc {
            return vec![self.branch_edge(ops, OutFlavor::TailOut)];
        }
        // Conditional `jr cc`/`jp cc`: the taken edge PLUS the fall-through.
        //
        // `call cc` is NOT one of these. It CALLS and comes back, so its only
        // successor is the fall-through — the same fact that keeps 68k `bsr` out
        // of the conditional-branch arm above. Giving a `call nz, .helper` the
        // branch's taken edge splices the helper's body into this proc's flow at
        // the caller's state; giving `call nz, External` a transfer-out edge
        // claims the flag and the register file leave the proc, which they do not.
        if matches!(mnem, "jp" | "jr") && leads_cc {
            let mut v = vec![self.branch_edge(ops, OutFlavor::BranchOut)];
            match fallthrough {
                Some(f) => v.push(Edge::Follow(f)),
                None => v.push(Edge::FallOff),
            }
            return v;
        }
        // `djnz` — a counting loop: its taken leg PLUS the fall-through. The leg
        // goes through the SAME three-way as every other conditional taken edge,
        // so a target this body does not define keeps an edge (`BranchOut`) and a
        // body-closing local label reads as the fall-off it is. A raw
        // `label_target` lookup here would emit NO edge for either shape, and a
        // path that no walk can see is a path no analysis can charge.
        if mnem == "djnz" {
            let mut v = vec![self.branch_edge(ops, OutFlavor::BranchOut)];
            match fallthrough {
                Some(f) => v.push(Edge::Follow(f)),
                None => v.push(Edge::FallOff),
            }
            return v;
        }
        // Everything else (incl. `call Name` and `call cc, Name`, which return)
        // falls through; the end of the body runs off it.
        match fallthrough {
            Some(f) => vec![Edge::Follow(f)],
            None => vec![Edge::FallOff],
        }
    }

    /// The taken edge of a branch — 68k `bra`/`jmp`/`bXX`/`dbXX` OR Z80
    /// `jp`/`jr` — classified by its TARGET. This is the ONE branch-edge
    /// resolution both CPUs share: a local label with an instruction after it is
    /// followed; a local label that CLOSES the proc is a fall-off exit, not a
    /// transfer out (the trailing `jbra .done` / `jr z, .div_ok` that ends a body
    /// before its `falls_into` successor); an external symbol, or a computed
    /// `jmp (a0)`/`jp (hl)` naming no symbol, is a transfer out of the proc.
    ///
    /// The three-way is CPU-independent, so both builders route their target
    /// edges through here. A closing label has no entry in the `label_target`
    /// map (that map holds the first instruction AT or after a label, and a
    /// closing label has none), so the map alone cannot tell it from an external
    /// symbol — `is_local_label` is what supplies the fall-off/transfer-out split.
    ///
    /// `flavor` carries the one fact this function cannot see: whether the
    /// TERMINATOR is conditional. That axis lives at the call site, which holds
    /// the mnemonic and the CPU's terminator set, and it decides which
    /// transfer-out variant a leaving edge takes.
    fn branch_edge(&self, ops: &[CodeOperand], flavor: OutFlavor) -> Edge {
        match branch_target(ops) {
            Some(t) => match self.label_target.get(t) {
                Some(&tgt) => Edge::Follow(tgt),
                None if self.is_local_label(t) => Edge::FallOff,
                None => flavor.out_edge(),
            },
            // A target the operands NAME no symbol for. TWO routes reach here, and
            // they differ in how reachable a CONDITIONAL is:
            //   * a COMPUTED transfer (`jmp (a0)`, `jp (hl)`) — a conditional form
            //     is unconstructible on both ISAs today (68k `bXX`/`dbXX` take a
            //     label; Z80 `jp cc` takes `nn`), so this route yields `BranchOut`
            //     only if the parser grows such a form;
            //   * a symbol-offset or absolute form [`branch_target`] does not read
            //     (`SymOff`, `AbsSym`) — nothing STOPS a conditional from lowering
            //     to one, and this route is checked against no corpus instance, so
            //     treat it as reachable.
            // Either way the edge takes the caller's flavor like any other leaving
            // edge, which is why this arm maps rather than asserting unreachable on
            // an operand shape.
            None => flavor.out_edge(),
        }
    }

    /// From the call at `call_idx`, walk the fall-through chain to the first
    /// branch that tests `cc` (or its negation) and return the item index that
    /// begins the INVALID edge (where `cc` does NOT hold). `None` when the guard
    /// is redefined first, the path returns, or an unrelated branch is reached —
    /// forward machinery bails rather than guess.
    fn invalid_edge(&self, call_idx: usize, cc: &str) -> Option<usize> {
        // `branch_cond` yields the CANONICAL code, so the declared guard must be
        // canonicalized before it can be compared with one. Raw, a `bhs` guard
        // matches neither `cc` (`"cc" != "hs"`) nor `neg` (`"cc" != "cs"`) and the
        // walk bails, so `[call.result-invalid-path]` never fires for the `hs`/`lo`
        // spellings of a guard it fires for as `cc`/`cs`.
        let cc = crate::ast::canonical_cc(cc);
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
            if RETURN_MNEMONICS.contains(&mnem)
                || writes_carry(mnem, Cpu::M68000)
                || writes_ccr_operand(ops)
            {
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
    ///
    /// **Label bail (Fable review 2026-07-21).** A LABEL anywhere between the
    /// call and the guard (inclusive of one immediately before the guard) is a
    /// potential JOIN: a bypass path can enter the chain there without having
    /// executed the call, so the guard's cc no longer implies the callee ran —
    /// crediting the guard's success edge would hand the bypass path the credit
    /// (a must-def FALSE NEGATIVE, the §3-forbidden polarity). `next_instr`
    /// chains instruction items and steps over labels invisibly, so the walk
    /// checks the RAW item range between consecutive steps and bails on any
    /// `CodeItem::Label` — even a currently-unreferenced local one (a referrer
    /// added later must not silently create the hole). NOTE the asymmetry:
    /// §6's [`Self::invalid_edge`] keeps the label-skip deliberately — its
    /// over-fire polarity makes a join harmless there; do not "unify" them.
    pub(crate) fn valid_edge(&self, call_idx: usize, cc: &str) -> Option<(usize, usize)> {
        let cc = crate::ast::canonical_cc(cc); // `branch_cond` yields canonical codes
        let neg = negate_cc(cc)?;
        let mut prev = call_idx;
        let mut idx = *self.next_instr.get(&call_idx)?;
        loop {
            if self.items[prev + 1..idx].iter().any(|it| matches!(it, CodeItem::Label { .. })) {
                return None; // a join point between call and guard — bypass paths would inherit the credit
            }
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
            prev = idx;
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

/// A control-flow edge out of one instruction. Built by [`Cfg::edges`] (68k) and
/// [`Cfg::z80_edges`] — those two builders and nowhere else.
///
/// **The builder decides, once.** Which variant an edge carries is settled where
/// the edge is CONSTRUCTED — the only place that holds the mnemonic, the CPU's
/// terminator set, and whether a conditional terminator's taken or fall-through
/// side is being emitted. A consumer that reads the mnemonic back off the
/// instruction to tell a return from a fall-off, or a tail transfer from a
/// conditional branch out, is re-deriving a fact the edge already states, and it
/// must re-supply the CPU to do it: read through the wrong CPU's table a Z80
/// `ret` is a fall-off-end, and keyed on the shared mnemonic both edges of an
/// end-of-body `ret cc` are returns.
///
/// **No variant claims anything about a transfer's TARGET** — not whether it
/// returns, diverges, or falls onward. [`Edge::Return`] and [`Edge::FallOff`]
/// are facts about the MACHINE; what a target does is not builder-visible and is
/// a consumer's own charge to make through its callee oracle. What IS
/// builder-visible is the conditional/unconditional axis of the TERMINATOR, and
/// that axis is the entire content of the
/// [`Edge::TailOut`]/[`Edge::BranchOut`] split.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Edge {
    /// Control stays in this proc, arriving at item index `.0`.
    Follow(usize),
    /// The path RETURNED to the caller: it executed one of this CPU's return
    /// instructions (68k `rts`/`rte`/`rtr`/`rtd`; Z80 `ret`/`reti`/`retn`,
    /// including the TAKEN edge of a `ret cc`).
    Return,
    /// Control ran off the END of the body — no next instruction, and no return
    /// instruction executed. Whether that is a return is a per-proc POLICY (a
    /// declared `falls_into` end is a continuation), so a consumer that cares
    /// carries its own flag; the edge states only the machine fact.
    FallOff,
    /// The successor of an UNCONDITIONAL transfer that leaves the body: an
    /// external symbol (`jbra Foo`, `jp Foo`), a symbol-offset or absolute form
    /// the abs seam lowers, or a computed target (`jmp (a0)`, `jp (hl)`).
    /// Control leaves the proc on every execution that reaches the instruction,
    /// so a `TailOut` is that instruction's ONLY edge.
    TailOut,
    /// The TAKEN successor of a CONDITIONAL terminator whose target is outside
    /// the body. Always accompanied by a sibling local edge ([`Edge::Follow`] or
    /// [`Edge::FallOff`]) — never an instruction's only edge.
    BranchOut,
}

/// Which transfer-out variant a terminator's leaving edge carries.
///
/// The conditional/unconditional axis is a property of the TERMINATOR, and the
/// shared [`Cfg::branch_edge`] three-way sees only operands — so the axis is
/// passed IN from the call site that already knows it. Making it a type rather
/// than a bare [`Edge`] parameter keeps a caller from handing the three-way a
/// local edge as a "flavor".
#[derive(Clone, Copy)]
enum OutFlavor {
    /// An unconditional transfer: the instruction's only edge leaves the body.
    /// Produces [`Edge::TailOut`].
    TailOut,
    /// A conditional terminator's taken side: a sibling local edge follows it.
    /// Produces [`Edge::BranchOut`].
    BranchOut,
}

impl OutFlavor {
    /// The 1:1 map onto the leaving edge. A method rather than a bare cast so the
    /// three-way's leaving arms name one thing, and a caller cannot hand it a
    /// local edge in place of a flavor.
    fn out_edge(self) -> Edge {
        match self {
            OutFlavor::TailOut => Edge::TailOut,
            OutFlavor::BranchOut => Edge::BranchOut,
        }
    }
}

/// Whether a mnemonic names a DIRECT subroutine call — the site whose flag
/// result must be consumed. 68k `jsr`/`bsr`/`jbsr`; Z80 `call` (rung-2 §13.3
/// sub-part 2).
fn is_call_site(mnem: &str, cpu: Cpu) -> bool {
    match cpu {
        Cpu::Z80 => mnem == "call",
        _ => CALL_MNEMONICS.contains(&mnem),
    }
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
    cpu: Cpu,
) -> Vec<FlagFiring> {
    let cfg = Cfg::build(items);
    let mut firings = Vec::new();

    for (idx, it) in items.iter().enumerate() {
        let CodeItem::Instr { mnemonic, ops, span, .. } = it else { continue };
        if !is_call_site(mnemonic, cpu) {
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
            if abandons_flag(&cfg, idx, cpu) {
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
pub(crate) fn branch_cond(mnem: &str) -> Option<&'static str> {
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
            | CodeOperand::DispInd { reg: r, .. }
            | CodeOperand::DispSymInd { reg: r, .. } => push(*r),
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
            // Return / FallOff / TailOut / BranchOut: the path leaves without a
            // read — safe here.
        }
    }
    false
}

/// Breadth-first reachability from the successors of the call at `call_idx`: is
/// there a path that REACHES a redefine / return / proc-end ([`Edge::Return`] or
/// [`Edge::FallOff`]) without first crossing a carry consumer? Consumers PRUNE
/// (that path is
/// satisfied); a transfer-out edge in either flavor also prunes (the flag flows
/// out of the proc — not a local abandonment). The visited set gives the CFG
/// real joins so loops terminate.
fn abandons_flag(cfg: &Cfg, call_idx: usize, cpu: Cpu) -> bool {
    // The Z80 terminator/edge model diverges from 68k (`ret` vs `rts`, `jr`/`jp`
    // vs `bra`/`jmp`, conditional `jr cc`), so the carry-tracking walk consults
    // the matching edge builder — leaving the 68k `edges` byte-untouched.
    let edges = |idx: usize| match cpu {
        Cpu::Z80 => cfg.z80_edges(idx),
        _ => cfg.edges(idx),
    };
    let mut visited: BTreeSet<usize> = BTreeSet::new();
    let mut queue: VecDeque<Edge> = VecDeque::new();
    // Seed from the call's own fall-through (the call is never re-examined).
    for e in edges(call_idx) {
        queue.push_back(e);
    }
    while let Some(edge) = queue.pop_front() {
        let idx = match edge {
            // Both ways of leaving without a consumer abandon the flag: a return
            // hands it to a caller that never asked for it, and running off the
            // end drops it. This check does not care which.
            Edge::Return | Edge::FallOff => return true,
            // Both flavors flow out of the proc — not a local abandonment. The
            // conditional's sibling `Follow` edge is walked separately.
            Edge::TailOut | Edge::BranchOut => continue,
            Edge::Follow(i) => i,
        };
        if !visited.insert(idx) {
            continue; // join / back-edge already explored
        }
        let Some((mnem, ops)) = cfg.instr(idx) else { continue };
        if consumes_carry(mnem, ops, cpu) {
            continue; // this path is satisfied
        }
        if writes_carry(mnem, cpu) || writes_ccr_operand(ops) {
            return true; // carry redefined before any consumer
        }
        for e in edges(idx) {
            queue.push_back(e);
        }
    }
    false
}

#[cfg(test)]
mod edge_model_tests {
    //! The edge model, stated over the EDGES rather than over any consumer's
    //! output: a builder decides `Return` vs `FallOff` vs `Follow`, and no
    //! consumer is in a position to express a different answer.

    use super::*;
    use crate::value::Z80Cond;
    use sigil_span::SourceId;

    fn sp() -> Span {
        Span { source: SourceId(0), start: 0, end: 0 }
    }
    fn instr(mnemonic: &str, ops: Vec<CodeOperand>) -> CodeItem {
        CodeItem::Instr {
            mnemonic: mnemonic.to_string(),
            size: None,
            ops,
            span: sp(),
            as_type: None,
            targets: Vec::new(),
            author: crate::value::ItemAuthor::User,
        }
    }
    fn label(name: &str) -> CodeItem {
        CodeItem::Label { name: name.to_string(), export: false, span: sp() }
    }
    fn sym(n: &str) -> CodeOperand {
        CodeOperand::Sym(n.to_string())
    }
    fn cc(c: Z80Cond) -> CodeOperand {
        CodeOperand::Z80Cc(c)
    }

    /// A `ret cc` that CLOSES a body has two successors carrying opposite facts:
    /// the taken edge returns to the caller, the other runs off the end with the
    /// path still live. They must be DIFFERENT values — a rule that cannot tell
    /// them apart closes the escaping side as a return at zero cost, and reports
    /// a cycle bound that is too low on an unsuppressible ERROR contract.
    #[test]
    fn a_tail_conditional_return_names_its_two_ends_apart() {
        let items = vec![instr("nop", vec![]), instr("ret", vec![cc(Z80Cond::Z)])];
        let cfg = Cfg::build(&items);
        let edges = cfg.z80_edges(1);
        assert_eq!(edges.len(), 2);
        assert_ne!(edges[0], edges[1], "the two ends of a tail `ret cc` are not the same fact");
        assert_eq!(edges, vec![Edge::Return, Edge::FallOff]);
    }

    /// A CALL comes back, so its only successor is the fall-through. Giving it a
    /// branch's taken edge splices the callee's body into the caller's flow AT
    /// THE CALLER'S STATE, charging a local helper's `rts` the caller's stack
    /// depth. `bsr` is spelled like a conditional branch — three letters, leading
    /// `b` — so the shape test must except it by name.
    #[test]
    fn a_bsr_calls_it_does_not_branch() {
        let items = vec![
            instr("bsr", vec![sym(".helper")]),
            instr("rts", vec![]),
            label(".helper"),
            instr("nop", vec![]),
            instr("rts", vec![]),
        ];
        let cfg = Cfg::build(&items);
        assert_eq!(cfg.edges(0), vec![Edge::Follow(1)], "`bsr` calls; it does not branch");
    }

    /// The Z80 twin: `call cc` is spelled like a conditional branch and is not
    /// one, for the same reason `bsr` is not one. Listed beside `jp cc`/`jr cc`
    /// it splices a local target into the caller's flow, and hands an external
    /// target a transfer-out edge claiming control left the proc.
    ///
    /// The external and tail cases pin the invariant `context.rs` relies on: NO
    /// call mnemonic yields a transfer-out edge, so no consumer needs a call
    /// exception in its escape check.
    #[test]
    fn a_conditional_z80_call_calls_it_does_not_branch() {
        let local = vec![
            instr("call", vec![cc(Z80Cond::Nz), sym(".helper")]),
            instr("ret", vec![]),
            label(".helper"),
            instr("nop", vec![]),
            instr("ret", vec![]),
        ];
        let cfg = Cfg::build(&local);
        assert_eq!(
            cfg.z80_edges(0),
            vec![Edge::Follow(1)],
            "the callee is not a successor of the caller"
        );

        let external = vec![
            instr("call", vec![cc(Z80Cond::Nz), sym("Fm_NoteOff")]),
            instr("ret", vec![]),
        ];
        let cfg = Cfg::build(&external);
        assert_eq!(cfg.z80_edges(0), vec![Edge::Follow(1)], "a call comes back");

        // A `call cc` closing a body still falls off it — it does not return, and
        // it does not transfer out.
        let tail = vec![instr("call", vec![cc(Z80Cond::Nz), sym("Fm_NoteOff")])];
        let cfg = Cfg::build(&tail);
        assert_eq!(cfg.z80_edges(0), vec![Edge::FallOff]);
    }

    /// The rest of the call vocabulary, held to the same invariant. Each is safe
    /// today because no arm of its builder matches it — but what keeps them out is
    /// a mnemonic-SHAPE predicate, which has already had to except one spelling by
    /// name, so the property is pinned rather than inferred.
    #[test]
    fn no_call_mnemonic_yields_a_transfer_out_edge() {
        for m in ["jsr", "jbsr", "bsr"] {
            let items = vec![instr(m, vec![sym("Elsewhere")]), instr("rts", vec![])];
            let cfg = Cfg::build(&items);
            assert_eq!(cfg.edges(0), vec![Edge::Follow(1)], "68k `{m}` comes back");
        }
        for m in ["call", "rst"] {
            let items = vec![instr(m, vec![sym("Elsewhere")]), instr("ret", vec![])];
            let cfg = Cfg::build(&items);
            assert_eq!(cfg.z80_edges(0), vec![Edge::Follow(1)], "z80 `{m}` comes back");
        }
    }

    /// A Z80 `jp`/`jr` to a LOCAL label that CLOSES the proc leaves the body
    /// without leaving the program's knowledge: control arrives at the fall-off
    /// point, where this proc's own analysis still applies. Reading it as a
    /// transfer OUT hands the path to a callee that does not exist, and every
    /// obligation the path carried — an unconsumed flag, a clobbered register —
    /// is discharged against nothing.
    ///
    /// `Cfg::build` maps a label to the first instruction at or after it, so a
    /// closing label has no mapping at all and is indistinguishable from an
    /// external symbol by that map alone. `Cfg::is_local_label` is what tells
    /// them apart.
    #[test]
    fn a_jump_to_a_closing_label_falls_off_it_does_not_transfer_out() {
        let items = vec![
            instr("call", vec![sym("FlagCallee")]),
            instr("jr", vec![sym(".done")]),
            label(".done"),
        ];
        let cfg = Cfg::build(&items);
        assert_eq!(cfg.z80_edges(1), vec![Edge::FallOff], "`.done` closes this proc");

        let guarded = vec![
            instr("call", vec![sym("FlagCallee")]),
            instr("jr", vec![cc(Z80Cond::Z), sym(".done")]),
            instr("ret", vec![]),
            label(".done"),
        ];
        let cfg = Cfg::build(&guarded);
        assert_eq!(cfg.z80_edges(1), vec![Edge::FallOff, Edge::Follow(2)]);

        // The contrast that gives the rule its content: a label this body does
        // NOT define really is a transfer out.
        let external = vec![
            instr("call", vec![sym("FlagCallee")]),
            instr("jr", vec![sym("Elsewhere")]),
        ];
        let cfg = Cfg::build(&external);
        assert_eq!(cfg.z80_edges(1), vec![Edge::TailOut]);
    }

    /// The 68k twin of the closing-label rule, routed through the SAME shared
    /// `branch_edge` three-way: a `jbra .done` whose `.done:` closes the body is
    /// a `FallOff` (control reaches the fall-off point where this proc's analysis
    /// still applies), never a transfer out to an external tail callee.
    /// This holds on BOTH the unconditional arm and a conditional branch's TAKEN
    /// edge — a `beq .done` closing the body falls off on the taken side and
    /// falls through on the other. The contrast: an external symbol is a genuine
    /// transfer out, and an in-body local is followed.
    #[test]
    fn a_68k_jump_to_a_closing_label_falls_off_it_does_not_transfer_out() {
        // Unconditional: `.done:` closes the proc.
        let uncond = vec![
            instr("jsr", vec![sym("FlagCallee")]),
            instr("jbra", vec![sym(".done")]),
            label(".done"),
        ];
        let cfg = Cfg::build(&uncond);
        assert_eq!(cfg.edges(1), vec![Edge::FallOff], "`.done` closes this 68k proc");

        // Conditional-taken: `beq .done` where `.done:` closes the body — the
        // taken edge falls off, the fall-through stays in the body.
        let guarded = vec![
            instr("jsr", vec![sym("FlagCallee")]),
            instr("beq", vec![sym(".done")]),
            instr("nop", vec![]),
            label(".done"),
        ];
        let cfg = Cfg::build(&guarded);
        assert_eq!(cfg.edges(1), vec![Edge::FallOff, Edge::Follow(2)]);

        // The contrast that gives the rule its content: a label this body does
        // NOT define is a transfer out, on both arms.
        let external_uncond = vec![instr("jbra", vec![sym("Elsewhere")])];
        assert_eq!(Cfg::build(&external_uncond).edges(0), vec![Edge::TailOut]);
        let external_cond = vec![instr("beq", vec![sym("Elsewhere")])];
        assert_eq!(
            Cfg::build(&external_cond).edges(0),
            vec![Edge::BranchOut, Edge::FallOff],
            "the conditional's taken edge leaves as a `BranchOut`; the fall-through \
             runs off the end"
        );

        // An in-body local label (an instruction after it) is followed, not
        // fallen off — the third leg of the three-way.
        let in_body = vec![
            instr("jbra", vec![sym(".loop")]),
            label(".loop"),
            instr("nop", vec![]),
            instr("rts", vec![]),
        ];
        // `.loop` maps to the first instruction AT or after it — the `nop` at
        // item index 2.
        assert_eq!(Cfg::build(&in_body).edges(0), vec![Edge::Follow(2)]);
    }

    /// Every mnemonic in a CPU's return table is classified by that CPU's return
    /// arms, conditional forms included — the classification is TOTAL over the
    /// table. A form that escaped to a later arm would be read as whatever that
    /// arm makes of it, which for an end-of-body instruction is a `FallOff`.
    #[test]
    fn the_z80_return_table_is_classified_whole() {
        for m in ["ret", "reti", "retn"] {
            let bare = vec![instr(m, vec![])];
            assert_eq!(Cfg::build(&bare).z80_edges(0), vec![Edge::Return], "`{m}`");
            let guarded = vec![instr(m, vec![cc(Z80Cond::Z)])];
            assert_eq!(
                Cfg::build(&guarded).z80_edges(0),
                vec![Edge::Return, Edge::FallOff],
                "`{m} cc`"
            );
        }
    }

    /// The CPU whose return table applies is settled by WHICH BUILDER produced
    /// the edge, so a consumer cannot pick the wrong one — there is no table left
    /// for it to pick. `ret` is a return to the Z80 builder and an ordinary
    /// instruction to the 68k builder, and the edges say so in both directions.
    #[test]
    fn the_builder_owns_the_return_table_not_its_consumers() {
        let items = vec![instr("ret", vec![])];
        let cfg = Cfg::build(&items);
        assert_eq!(cfg.z80_edges(0), vec![Edge::Return]);
        assert_eq!(cfg.edges(0), vec![Edge::FallOff], "`ret` is no 68k terminator");

        let items = vec![instr("rts", vec![])];
        let cfg = Cfg::build(&items);
        assert_eq!(cfg.edges(0), vec![Edge::Return]);
        assert_eq!(cfg.z80_edges(0), vec![Edge::FallOff], "`rts` is no Z80 terminator");
    }

    // ---- the transfer-out split ------------------------------------------

    /// The structural invariant the singleton-pattern consumers rest on
    /// (`cycle_budget::enumerated_succs`, `lower::proc::terminal_external_tail`):
    /// a `BranchOut` is the taken side of a CONDITIONAL terminator, so it always
    /// has a sibling local edge and is NEVER an instruction's only edge — while a
    /// `TailOut` always IS. A `[Edge::TailOut]` pattern therefore admits exactly
    /// the unconditional transfers out, and no future conditional shape can slip
    /// through it.
    ///
    /// Swept over every terminator shape on both CPUs whose target leaves the
    /// body, with a count on each side so the sweep cannot pass by observing
    /// nothing.
    #[test]
    fn a_branch_out_always_has_a_sibling_and_a_tail_out_never_does() {
        let conditional: Vec<Vec<CodeItem>> = vec![
            // 68k conditional branch / dbcc, external target.
            vec![instr("beq", vec![sym("Elsewhere")]), instr("rts", vec![])],
            // `dbcc` in its real two-operand form (`dbra dN, label`) — the counter
            // register is what makes it one, and the probe spells it.
            vec![
                instr("dbra", vec![CodeOperand::Reg(Reg::D0), sym("Elsewhere")]),
                instr("rts", vec![]),
            ],
            // The same, CLOSING the body (the sibling is a `FallOff`, not a
            // `Follow`) — a shape that would read as a singleton if the sibling
            // were dropped.
            vec![instr("bne", vec![sym("Elsewhere")])],
            // Z80 `jr cc` / `jp cc` / `djnz`, external target.
            vec![instr("jr", vec![cc(Z80Cond::Z), sym("Elsewhere")]), instr("ret", vec![])],
            vec![instr("jp", vec![cc(Z80Cond::Nz), sym("Elsewhere")]), instr("ret", vec![])],
            vec![instr("djnz", vec![sym("Elsewhere")]), instr("ret", vec![])],
            vec![instr("djnz", vec![sym("Elsewhere")])],
        ];
        let mut branch_outs = 0;
        for items in &conditional {
            let cfg = Cfg::build(items);
            for edges in [cfg.edges(0), cfg.z80_edges(0)] {
                if edges.contains(&Edge::BranchOut) {
                    branch_outs += 1;
                    assert!(
                        edges.len() >= 2,
                        "a `BranchOut` must carry a sibling local edge, got {edges:?}"
                    );
                    assert!(
                        !edges.contains(&Edge::TailOut),
                        "the two flavors never share an instruction: {edges:?}"
                    );
                }
            }
        }
        assert!(branch_outs >= 7, "the sweep observed only {branch_outs} `BranchOut` edges");

        let unconditional: Vec<Vec<CodeItem>> = vec![
            vec![instr("jbra", vec![sym("Elsewhere")])],
            vec![instr("bra", vec![sym("Elsewhere")])],
            vec![instr("jmp", vec![CodeOperand::Ind(Reg::A0)])],
            vec![instr("jp", vec![sym("Elsewhere")])],
            vec![instr("jr", vec![sym("Elsewhere")])],
            vec![instr("jp", vec![CodeOperand::Z80IndHl])],
        ];
        let mut tail_outs = 0;
        for items in &unconditional {
            let cfg = Cfg::build(items);
            for edges in [cfg.edges(0), cfg.z80_edges(0)] {
                if edges.contains(&Edge::TailOut) {
                    tail_outs += 1;
                    assert_eq!(edges, vec![Edge::TailOut], "a `TailOut` is the only edge");
                }
            }
        }
        assert!(tail_outs >= 6, "the sweep observed only {tail_outs} `TailOut` edges");
    }

    /// A COMPUTED transfer — one whose operands name no symbol at all — leaves the
    /// proc unconditionally, so it is a singleton `TailOut` on both CPUs. This is
    /// the shape the enumerated-dispatch `targets(...)` clause is legal on, and
    /// the pattern that admits it is the singleton one.
    #[test]
    fn a_computed_transfer_is_a_singleton_tail_out() {
        let m68k = vec![instr("jmp", vec![CodeOperand::Ind(Reg::A0)])];
        assert_eq!(Cfg::build(&m68k).edges(0), vec![Edge::TailOut]);

        let z80 = vec![instr("jp", vec![CodeOperand::Z80IndHl])];
        assert_eq!(Cfg::build(&z80).z80_edges(0), vec![Edge::TailOut]);
    }

    /// A `djnz` taken leg goes through the SAME three-way as every other
    /// conditional taken edge, so no shape of target loses its edge.
    ///
    /// The in-body backward loop — the only shape the corpus writes — is a
    /// `Follow`, unchanged. The two shapes a raw `label_target` lookup would drop
    /// keep an edge: a target this body does not define is a `BranchOut`, and a
    /// local label that CLOSES the body is the `FallOff` it is on every other
    /// terminator. A dropped leg is a path no walk can see and no analysis can
    /// charge.
    #[test]
    fn a_djnz_leg_that_leaves_the_body_keeps_its_edge() {
        // In-body backward loop: `Follow` the label, then fall through.
        let loop_back = vec![
            label(".spin"),
            instr("nop", vec![]),
            instr("djnz", vec![sym(".spin")]),
            instr("ret", vec![]),
        ];
        let cfg = Cfg::build(&loop_back);
        assert_eq!(cfg.z80_edges(2), vec![Edge::Follow(1), Edge::Follow(3)]);

        // A target this body does not define: the leg leaves as a `BranchOut`,
        // and the fall-through stays.
        let external = vec![instr("djnz", vec![sym("Elsewhere")]), instr("ret", vec![])];
        let cfg = Cfg::build(&external);
        assert_eq!(cfg.z80_edges(0), vec![Edge::BranchOut, Edge::Follow(1)]);

        // A local label that CLOSES the body: the leg reaches the fall-off point,
        // where this proc's own analysis still applies.
        let trailing = vec![
            instr("djnz", vec![sym(".done")]),
            instr("ret", vec![]),
            label(".done"),
        ];
        let cfg = Cfg::build(&trailing);
        assert_eq!(cfg.z80_edges(0), vec![Edge::FallOff, Edge::Follow(1)]);

        // Both leaving shapes closing the body too — the fall-through becomes a
        // `FallOff` and the leg is still there.
        let external_closing = vec![instr("djnz", vec![sym("Elsewhere")])];
        assert_eq!(
            Cfg::build(&external_closing).z80_edges(0),
            vec![Edge::BranchOut, Edge::FallOff]
        );
    }
}
