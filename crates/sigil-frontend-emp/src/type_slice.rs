//! G5 §7 tier 5 — the caller-side DOMAIN-NEWTYPE slot check (`[call.slot-type-mismatch]`).
//!
//! A "strict-degrade reaching-definition slice": a forward MUST dataflow over a
//! proc's evaluated CodeBuf that tracks, per register, the DOMAIN NEWTYPE it
//! provably holds (`Untyped | T`). At a direct call/tail to a proc whose params
//! carry a domain newtype (`Section_FlatIDXY (d2: GridX, d3: GridY, …)`), each
//! typed slot must hold exactly that newtype at the call — otherwise a
//! `[call.slot-type-mismatch]` ERROR fires (the sec_x/sec_y swap class, the same
//! silent-wrong-answer family as the MigrateMasks stride bug).
//!
//! The lattice (per register): `None` (Untyped) ⊒ `Some(T)`. Transfer:
//! - a PLAIN register copy `move`/`movea` rX→rY PROPAGATES rX's type to rY;
//! - an `as Type` annotation on a PRODUCING instruction BLESSES the destination
//!   register with that newtype (the boundary escape hatch — Q3);
//! - ANY other write (arithmetic, logic, shift, memory load, moveq) DEGRADES the
//!   written register(s) to `Untyped`;
//! - a call UPDATES per the callee's contract: `out(dN: T)` births T, any other
//!   written/clobbered register degrades to `Untyped`, preserved registers keep
//!   their type.
//!
//! Control-flow JOIN is the meet: both edges agree on `T` ⇒ `T`, else `Untyped`.
//! An `Untyped` (or wrong-newtype) value reaching a typed slot is the loud site;
//! an UNTYPED callee slot checks nothing (§7 no-ceremony rule).
//!
//! Mirrors `out_verify.rs`/`calls.rs`: the shared `flag_check::Cfg` substrate, a
//! worklist forward fixpoint (join = meet ⇒ monotone ⇒ terminates), then a
//! single post-fixpoint walk emits one firing per offending slot per call site.

use crate::flag_check::{Cfg, Edge};
use crate::lower::instr_written_regs;
use crate::value::{CodeItem, CodeOperand, Reg};
use sigil_span::Span;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Mnemonics that CALL (the callee returns to us — its out/clobber effect applies
/// after the call) and TAIL-TRANSFER (control leaves; only the CHECK applies).
const CALL_MNEMONICS: [&str; 3] = ["jsr", "bsr", "jbsr"];
const TAIL_MNEMONICS: [&str; 4] = ["jmp", "bra", "jbra", "jra"];

/// One `[call.slot-type-mismatch]` firing: at `span`, `proc` calls `callee`
/// passing `found` (a wrong newtype, or `None` = an untyped/undefined value) in
/// register `reg`, but `callee` declares that slot as `expected`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotTypeMismatch {
    pub proc: String,
    pub callee: String,
    pub reg: String,
    pub expected: String,
    pub found: Option<String>,
    pub span: Span,
}

/// The per-program-point abstract state: each register's provable domain newtype,
/// `None` = Untyped. Index d0..d7 = 0..7, a0..a7 = 8..15 (`Reg as usize`).
type TypeState = [Option<String>; 16];

const UNTYPED: Option<String> = None;

fn reg_idx(r: Reg) -> usize {
    r as usize
}

/// A plain register-to-register copy `move`/`movea rX, rY` — the only transfer
/// that PROPAGATES a domain type. `moveq #imm, dY` is a different mnemonic (first
/// operand `Imm`) and correctly falls through to the degrade path; an auto-inc
/// source (`move (a0)+, d0`) is a `PostInc`, not a bare `Reg`, so it is not a
/// plain copy either. Size is intentionally not restricted — a `.w`/`.b` copy of
/// a byte-width grid coord preserves the value (Q3's `move.w d4, d2` case).
fn plain_reg_copy(mnem: &str, ops: &[CodeOperand]) -> Option<(Reg, Reg)> {
    if mnem != "move" && mnem != "movea" {
        return None;
    }
    match ops {
        [CodeOperand::Reg(src), CodeOperand::Reg(dst)] => Some((*src, *dst)),
        _ => None,
    }
}

/// The destination register an `as Type` bless applies to — the last operand,
/// when it is a bare register.
fn dest_reg(ops: &[CodeOperand]) -> Option<Reg> {
    match ops.last() {
        Some(CodeOperand::Reg(r)) => Some(*r),
        _ => None,
    }
}

/// The bare `Sym` target of a direct call/tail, or `None` for an indirect /
/// local-label (`$`-mangled) target.
fn direct_target(ops: &[CodeOperand]) -> Option<&str> {
    match ops {
        [CodeOperand::Sym(name)] if !name.contains('$') => Some(name.as_str()),
        _ => None,
    }
}

/// The registers a returning call leaves in a KNOWN state: `out(dN: T)` slots
/// (→ `Some(T)`) and every other register the callee may write/clobber
/// (→ `Untyped`). The degrade set is the callee's DECLARED `clobbers(...)` ∪ its
/// `out(...)` — the caller-facing CONTRACT, which the S2-D6 transitive-clobber
/// ERROR gate proves is a sound over-approximation of actual effect (a register
/// the body writes but SAVE/RESTOREs is verified-preserved and stays out of the
/// declared set — this is exactly what lets `Section_GetSecPtrXY`'s tightened
/// `clobbers(d1)` carry a `GridX` in d2 across the call). A callee with NO
/// declared clobber contract (`None` — half-ported) or an unknown/indirect
/// target degrades ALL non-typed-out registers (fully conservative).
fn apply_call_effect(
    st: &mut TypeState,
    callee: &str,
    typed_out: &BTreeMap<String, Vec<(usize, String)>>,
    callee_out: &BTreeMap<String, BTreeSet<String>>,
    callee_clobbers: &BTreeMap<String, Option<BTreeSet<String>>>,
) {
    // Degrade set: every register the callee may write (declared clobber ∪
    // declared out). A typed-out slot is re-blessed afterwards.
    let mut clobbered = [false; 16];
    match callee_clobbers.get(callee) {
        Some(Some(set)) => {
            for name in set {
                if let Some(r) = Reg::from_name(name) {
                    clobbered[reg_idx(r)] = true;
                }
            }
        }
        // No declared contract, or an unknown callee: assume every register is
        // clobbered.
        _ => clobbered = [true; 16],
    }
    if let Some(outs) = callee_out.get(callee) {
        for name in outs {
            if let Some(r) = Reg::from_name(name) {
                clobbered[reg_idx(r)] = true;
            }
        }
    }
    for (i, hit) in clobbered.iter().enumerate() {
        if *hit {
            st[i] = UNTYPED;
        }
    }
    // Born-typed outs override the degrade.
    if let Some(outs) = typed_out.get(callee) {
        for (idx, nt) in outs {
            st[*idx] = Some(nt.clone());
        }
    }
}

/// Apply instruction `idx`'s effect to `st` (state update only — firings are
/// emitted in a separate post-fixpoint walk so a re-visited call site fires once).
#[allow(clippy::too_many_arguments)]
fn transfer(
    cfg: &Cfg,
    idx: usize,
    st: &mut TypeState,
    items: &[CodeItem],
    newtypes: &BTreeSet<String>,
    typed_out: &BTreeMap<String, Vec<(usize, String)>>,
    callee_out: &BTreeMap<String, BTreeSet<String>>,
    callee_clobbers: &BTreeMap<String, Option<BTreeSet<String>>>,
) {
    let Some((mnem, ops)) = cfg.instr(idx) else { return };

    // A returning call: apply the callee's contract effect. (A TAIL leaves the
    // proc — its post-state never propagates, so no update is needed.)
    if CALL_MNEMONICS.contains(&mnem) {
        if let Some(callee) = direct_target(ops) {
            apply_call_effect(st, callee, typed_out, callee_out, callee_clobbers);
            return;
        }
        // Indirect call: conservatively clobber every register.
        *st = std::array::from_fn(|_| UNTYPED);
        return;
    }
    if TAIL_MNEMONICS.contains(&mnem) {
        return;
    }

    // A producing instruction. A plain reg copy propagates; anything else
    // degrades every written register. An `as Type` bless then overrides the dest.
    let as_type = items.get(idx).and_then(|it| match it {
        CodeItem::Instr { as_type, .. } => as_type.as_ref(),
        _ => None,
    });

    if let Some((src, dst)) = plain_reg_copy(mnem, ops) {
        st[reg_idx(dst)] = st[reg_idx(src)].clone();
    } else {
        for r in instr_written_regs(mnem, ops) {
            st[reg_idx(r)] = UNTYPED;
        }
    }

    if let Some(nt) = as_type {
        if newtypes.contains(nt) {
            if let Some(dst) = dest_reg(ops) {
                st[reg_idx(dst)] = Some(nt.clone());
            }
        }
    }
}

/// The control-flow JOIN — the meet: a register keeps its type only where both
/// incoming edges agree, else it degrades to Untyped.
fn join(acc: &mut TypeState, other: &TypeState) {
    for i in 0..16 {
        if acc[i] != other[i] {
            acc[i] = UNTYPED;
        }
    }
}

/// Compute the per-instruction IN type-state fixpoint. Forward MUST dataflow,
/// join = meet, worklist to a fixpoint. `seed` is the entry state (the proc's own
/// typed params).
#[allow(clippy::too_many_arguments)]
fn type_state_in(
    cfg: &Cfg,
    entry: usize,
    seed: TypeState,
    items: &[CodeItem],
    newtypes: &BTreeSet<String>,
    typed_out: &BTreeMap<String, Vec<(usize, String)>>,
    callee_out: &BTreeMap<String, BTreeSet<String>>,
    callee_clobbers: &BTreeMap<String, Option<BTreeSet<String>>>,
) -> BTreeMap<usize, TypeState> {
    let mut in_state: BTreeMap<usize, TypeState> = BTreeMap::new();
    in_state.insert(entry, seed);
    let mut work: VecDeque<usize> = VecDeque::from([entry]);
    while let Some(idx) = work.pop_front() {
        let mut st = in_state[&idx].clone();
        transfer(cfg, idx, &mut st, items, newtypes, typed_out, callee_out, callee_clobbers);
        for edge in cfg.edges(idx) {
            let Edge::Follow(succ) = edge else { continue };
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
    }
    in_state
}

/// Check every direct call/tail in `proc`'s body against the callees' typed
/// param slots, emitting a `[call.slot-type-mismatch]` for each slot whose
/// provable type at the call is not the declared newtype (Untyped included).
#[allow(clippy::too_many_arguments)]
pub fn check_slot_types(
    proc: &str,
    items: &[CodeItem],
    typed_params: &BTreeMap<String, Vec<(usize, String)>>,
    typed_out: &BTreeMap<String, Vec<(usize, String)>>,
    callee_out: &BTreeMap<String, BTreeSet<String>>,
    callee_clobbers: &BTreeMap<String, Option<BTreeSet<String>>>,
    newtypes: &BTreeSet<String>,
    own_params: &[(usize, String)],
) -> Vec<SlotTypeMismatch> {
    let mut firings = Vec::new();
    // Only procs that actually call a domain-typed callee can fire; skip cheaply.
    let cfg = Cfg::build(items);
    let Some(entry) = items.iter().position(|it| matches!(it, CodeItem::Instr { .. })) else {
        return firings;
    };

    // Entry seed = the proc's own typed params (a pass-through caller keeps them).
    let mut seed: TypeState = std::array::from_fn(|_| UNTYPED);
    for (idx, nt) in own_params {
        seed[*idx] = Some(nt.clone());
    }

    let in_state = type_state_in(
        &cfg, entry, seed, items, newtypes, typed_out, callee_out, callee_clobbers,
    );

    for (idx, it) in items.iter().enumerate() {
        let CodeItem::Instr { mnemonic, ops, span, .. } = it else { continue };
        if !CALL_MNEMONICS.contains(&mnemonic.as_str())
            && !TAIL_MNEMONICS.contains(&mnemonic.as_str())
        {
            continue;
        }
        let Some(callee) = direct_target(ops) else { continue };
        let Some(slots) = typed_params.get(callee) else { continue };
        let Some(st) = in_state.get(&idx) else { continue };
        for (reg_i, expected) in slots {
            let found = &st[*reg_i];
            if found.as_deref() != Some(expected.as_str()) {
                firings.push(SlotTypeMismatch {
                    proc: proc.to_string(),
                    callee: callee.to_string(),
                    reg: reg_name(*reg_i),
                    expected: expected.clone(),
                    found: found.clone(),
                    span: *span,
                });
            }
        }
    }
    firings
}

/// The canonical spelling of a register slot index (for diagnostics).
fn reg_name(i: usize) -> String {
    const NAMES: [&str; 16] = [
        "d0", "d1", "d2", "d3", "d4", "d5", "d6", "d7", "a0", "a1", "a2", "a3", "a4", "a5", "a6",
        "a7",
    ];
    NAMES.get(i).copied().unwrap_or("d0").to_string()
}
