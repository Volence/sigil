//! `[branch.condition-constant]` — a sound "the branch's condition is statically
//! decided" check (contract-grammar item-4 rider, §D backlog).
//!
//! A conditional branch (`Bcc`/`Scc`) reads the condition codes set by the
//! nearest preceding CCR-writer. When that writer is a **compile-time constant**
//! source (`move #imm`, `moveq #imm`, `clr`) on EVERY reaching path — and every
//! intervening instruction is provably CC-transparent — the branch outcome does
//! not depend on runtime state: it is either always-taken or never-taken. That is
//! dead code, or — as with the `Sound_PlayMusic.await_slot` repost gate — a
//! condition clobbered by an interposed constant-flag write (`startZ80` is
//! `move.w #$0000, Z80_BUS_REQUEST`, forcing `Z=1`, so the following `bne` can
//! never loop). Same silent-wrong-behaviour family as the MigrateMasks stride bug.
//!
//! **Soundness stance (the opposite polarity to [`crate::flag_check`]):** this
//! check must NOT false-fire, so an instruction is treated as a CCR clobber
//! (`Dyn`) unless it is PROVABLY CC-transparent or a recognised constant writer.
//! An unmodelled mnemonic degrades the state to `Dyn` (a false NEGATIVE, safe) —
//! never keeps a stale constant. Full CCR liveness stays S2-D7; this is the sound
//! constant-fold slice of it.
//!
//! Mechanism: a forward MUST dataflow over `flag_check::Cfg` with a 3-point
//! lattice `Top ⊒ Const{z,n} ⊒ Dyn`, join = meet (disagreeing constants ⇒ `Dyn`),
//! worklist to a fixpoint, then a post-fixpoint walk fires once per branch whose
//! IN-state is `Const`. The `(z,n)` granularity is exact: `move`/`moveq`/`clr`
//! always clear V and C, so two constants agreeing on `(z,n)` decide every cc
//! identically — merging them loses nothing.

use crate::flag_check::{branch_cond, Cfg, Edge};
use crate::value::{CodeItem, CodeOperand, Width};
use sigil_span::Span;
use std::collections::{BTreeMap, VecDeque};

/// One `[branch.condition-constant]` firing: in `proc`, a conditional branch
/// testing `cc` has a statically-determined outcome (`always_taken`) because its
/// reaching CCR-definition is a compile-time constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchConstFiring {
    pub proc: String,
    /// The condition the branch tests (`"ne"`, `"eq"`, …).
    pub cc: String,
    /// `true` = the branch always transfers; `false` = it never does (the
    /// fall-through is unconditional). Either way the runtime state is ignored.
    pub always_taken: bool,
    pub span: Span,
}

/// The CCR-constancy lattice value at a program point.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CcState {
    /// No information yet (entry / not-yet-reached). The join identity.
    Top,
    /// CCR is a compile-time constant with these `Z`/`N` bits (V and C are 0 for
    /// every constant writer we model).
    Const { z: bool, n: bool },
    /// CCR is runtime-dependent.
    Dyn,
}

/// `Z`/`N` for a value masked to `w` bits (V, C are 0 for `move`/`moveq`/`clr`).
fn flags_of(v: i128, w: Width) -> (bool, bool) {
    let bits = match w {
        Width::B => 8,
        Width::W => 16,
        Width::L => 32,
        // `.s` is a short-BRANCH size; it never annotates a `move`/`moveq`/`clr`
        // data value, so this arm is unreachable for a real constant writer.
        Width::S => 8,
    };
    let mask: i128 = (1i128 << bits) - 1;
    let m = v & mask;
    let z = m == 0;
    let n = (m >> (bits - 1)) & 1 == 1;
    (z, n)
}

/// If `instr` sets CCR from a compile-time-constant source, its known `(z, n)`.
/// The modelled writers: `clr` (Z=1), `moveq #imm` (`.l`, byte sign-extended),
/// and `move #imm, <normal dst>` (immediate source, not to `ccr`/`sr`). A
/// non-immediate `move` (`move d0, d1` / a memory load) is runtime — not here.
fn const_flag_writer(mnem: &str, size: Option<Width>, ops: &[CodeOperand]) -> Option<(bool, bool)> {
    match mnem {
        "clr" => Some((true, false)),
        "moveq" => match ops.first() {
            // moveq loads the byte immediate sign-extended into a long.
            Some(CodeOperand::Imm(v)) => Some(flags_of((*v as i8) as i128, Width::L)),
            _ => None,
        },
        "move" => match (ops.first(), ops.last()) {
            (Some(CodeOperand::Imm(v)), Some(dst))
                if !matches!(dst, CodeOperand::Ccr | CodeOperand::Sr) =>
            {
                Some(flags_of(*v, size.unwrap_or(Width::W)))
            }
            _ => None,
        },
        _ => None,
    }
}

/// Is `mnem` PROVABLY CC-transparent — it cannot change any condition code? The
/// conservative allowlist: `Bcc`/`Scc` (read CCR, never write it), the
/// unconditional/`dbcc` transfers, and the address-register / move-multiple / misc
/// ops that leave CCR untouched. Everything NOT here is assumed to clobber CCR
/// (→ `Dyn`), which is the sound (false-negative) default. `branch_cond` matching
/// keeps `sub`/`swap`-style arithmetic OUT (they write CCR).
fn cc_transparent(mnem: &str) -> bool {
    if branch_cond(mnem).is_some() {
        return true; // Bcc / Scc read CCR only (sub → branch_cond None, stays out)
    }
    matches!(
        mnem,
        "bra" | "jbra" | "jra" | "jmp"
            | "dbra" | "dbf" | "dbt" | "dbcc" | "dbcs" | "dbeq" | "dbne" | "dbhi" | "dbls"
            | "dbpl" | "dbmi" | "dbvc" | "dbvs" | "dbge" | "dblt" | "dbgt" | "dble"
            | "movea" | "lea" | "pea" | "adda" | "suba" | "movem" | "exg" | "nop" | "link"
            | "unlk"
    )
}

/// Apply instruction `idx`'s effect to the CCR-constancy state.
fn step(st: CcState, mnem: &str, size: Option<Width>, ops: &[CodeOperand]) -> CcState {
    if let Some((z, n)) = const_flag_writer(mnem, size, ops) {
        return CcState::Const { z, n };
    }
    if cc_transparent(mnem) {
        return st;
    }
    CcState::Dyn // any other (including calls, tst/cmp, btst, memory loads) clobbers CCR
}

/// The meet (join for this MUST analysis): agreeing constants survive, everything
/// else falls to `Dyn`; `Top` is the identity.
fn meet(a: CcState, b: CcState) -> CcState {
    match (a, b) {
        (CcState::Top, x) | (x, CcState::Top) => x,
        (CcState::Const { z: z1, n: n1 }, CcState::Const { z: z2, n: n2 })
            if z1 == z2 && n1 == n2 =>
        {
            CcState::Const { z: z1, n: n1 }
        }
        _ => CcState::Dyn,
    }
}

/// Evaluate whether condition `cc` holds given a constant `(z, n)` (V = C = 0).
fn cc_holds(cc: &str, z: bool, n: bool) -> bool {
    match cc {
        "eq" => z,
        "ne" => !z,
        "cc" | "hs" => true,  // C == 0
        "cs" | "lo" => false, // C == 0
        "pl" => !n,
        "mi" => n,
        "vc" => true,  // V == 0
        "vs" => false, // V == 0
        "hi" => !z,    // !C & !Z == !Z
        "ls" => z,     // C | Z == Z
        "ge" => !n,    // N == V == 0
        "lt" => n,     // N != V
        "gt" => !z && !n,
        "le" => z || n,
        _ => false,
    }
}

/// The per-instruction IN-state fixpoint: forward MUST dataflow, join = meet,
/// worklist. Seed = `Top` at entry (a proc's entry CCR is caller-dependent =
/// unknown, so no branch fires purely off entry).
fn in_states(cfg: &Cfg, entry: usize, items: &[CodeItem]) -> BTreeMap<usize, CcState> {
    let mut in_state: BTreeMap<usize, CcState> = BTreeMap::new();
    in_state.insert(entry, CcState::Top);
    let mut work: VecDeque<usize> = VecDeque::from([entry]);
    while let Some(idx) = work.pop_front() {
        let st_in = in_state[&idx];
        let (mnem, ops) = match cfg.instr(idx) {
            Some(x) => x,
            None => continue,
        };
        let size = match &items[idx] {
            CodeItem::Instr { size, .. } => *size,
            _ => None,
        };
        let st_out = step(st_in, mnem, size, ops);
        for edge in cfg.edges(idx) {
            let Edge::Follow(succ) = edge else { continue };
            let merged = match in_state.get(&succ) {
                None => st_out,
                Some(existing) => meet(*existing, st_out),
            };
            let changed = in_state.get(&succ) != Some(&merged);
            if changed {
                in_state.insert(succ, merged);
                work.push_back(succ);
            }
        }
    }
    in_state
}

/// Run `[branch.condition-constant]` over one proc's evaluated CodeBuf. Fires once
/// per conditional branch whose reaching CCR-definition is provably constant.
pub fn check_branch_const(proc: &str, items: &[CodeItem]) -> Vec<BranchConstFiring> {
    let mut firings = Vec::new();
    let cfg = Cfg::build(items);
    let Some(entry) = items.iter().position(|it| matches!(it, CodeItem::Instr { .. })) else {
        return firings;
    };
    let in_state = in_states(&cfg, entry, items);

    for (idx, it) in items.iter().enumerate() {
        let CodeItem::Instr { mnemonic, span, .. } = it else { continue };
        // Only real conditional branches/sets consume CCR; `bra`/`dbf`/`dbra`
        // (Cond::T/F) test no condition (`branch_cond` → None) and are skipped.
        let Some(cc) = branch_cond(mnemonic) else { continue };
        let Some(CcState::Const { z, n }) = in_state.get(&idx).copied() else { continue };
        firings.push(BranchConstFiring {
            proc: proc.to_string(),
            cc: cc.to_string(),
            always_taken: cc_holds(cc, z, n),
            span: *span,
        });
    }
    firings
}
