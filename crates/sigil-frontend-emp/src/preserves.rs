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
//! `link aN,#-d` opens a frame — the saved fp is an ordinary tagged slot and the
//! locals one opaque slot — and `unlk aN` truncates back to the matching mark and
//! pops the fp.
//!
//! **Soundness bailouts** (assembly is assembly, §5): a bare `a7` operand (sp's
//! VALUE used as a number / escaping into address math, or a computed
//! `adda #n,sp`), a displaced/indexed sp access (`d(sp)` / `(sp,Xn)` — a store
//! that could alias a tracked slot), or an `unlk` with no matching open `link`
//! (sp is set from a register this model never computed) makes the stack model
//! untrustworthy → the analysis BAILS. A declared `preserves` on a written
//! register whose proof bailed is `[proc.preserves-unverifiable]` (error — a wrong
//! contract is worse than none, the D2.32 principle kept). The movem entry/exit
//! pair is the trivial fast path of this same analysis — D2.32 subsumed.
//!
//! **Scoped exits** ([`ReturnScope`]): the same proof serves the ¬cc half of a
//! conditional out (delta spec §7.1). `out(rN if cc)` with rN absent from
//! `clobbers` claims rN survives the ¬cc return paths — a preservation proof
//! obligated on SOME exits, not all (rN is deliberately written on the cc edge).
//! Only the choice of exits differs; the round-trip, never-written and
//! callee-preserves proofs are the ones below, unchanged.
//!
//! **Stack balance** ([`check_stack_balance`], delta spec §3 / U-spec §4-stack):
//! the symbolic stack's own DEPTH is the sp delta, so `[stack.unbalanced]` and
//! `[stack.merge-mismatch]` read the SAME model rather than running a second one.
//! Both consumers walk through [`run_stack_dataflow`] — one worklist, one
//! [`transfer`], one [`join`], therefore one bailout set. Two analyses cannot
//! disagree about whether the stack model is trustworthy when there is only one
//! place that decides.

use crate::closure::RegEffect;
use crate::flag_check::{entry_instr_idx, instr_span, transfer_target_sym, Cfg, Edge};
use crate::lower::instr_written_regs;
use crate::value::{CodeItem, CodeOperand, ItemAuthor, Reg, Width};
use sigil_span::Span;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Registers in canonical mask-bit order (bit0=d0..bit15=a7); the array index
/// equals [`reg_idx`], so `REG_BY_IDX[i]` is the register the state arrays track
/// at slot `i`.
const REG_BY_IDX: [Reg; 16] = [
    Reg::D0, Reg::D1, Reg::D2, Reg::D3, Reg::D4, Reg::D5, Reg::D6, Reg::D7,
    Reg::A0, Reg::A1, Reg::A2, Reg::A3, Reg::A4, Reg::A5, Reg::A6, Reg::A7,
];

/// How a `jsr`/`bsr` call is modeled in the entry-value proof — the CALLEE-PRESERVES
/// ORACLE (§5). A call always ends the linear-delta tracking of every register
/// (the callee may recompute them past what the delta model follows), but its
/// effect on the ENTRY-value bits — the round-trip proof — depends on what is known
/// about the callee's contract.
///
/// **Soundness.** The oracle credits a caller's `preserves(rN)` across a call only
/// when the callee's OWN `preserves(rN)` is itself proven — by THIS SAME analysis,
/// transitively resolved into the closure's verified `effective` map. There is no
/// new fixpoint and no circular trust: the closure's monotone union fixpoint
/// ([`crate::closure::compute_closure`]) has already folded every call CYCLE /
/// SCC into `effective` conservatively (a member's effect is the union of the
/// cycle's), so an in-cycle or unknown callee reads as "clobbers" and the caller
/// stays conservative. `Oracle` consults `effective` through the EXACT convention
/// [`find_dead_saves`] already uses ([`callee_clobbers`]) — ONE convention for
/// "does this callee clobber rN?" across both the dead-save and preserve analyses.
#[derive(Clone, Copy)]
pub enum CallPolicy<'a> {
    /// No callee-contract knowledge: every call clobbers every register. The
    /// per-file byte gate's default (a callee there may be a synthetic,
    /// contract-less stub). The pre-oracle behavior, and the closure SEED's
    /// oracle-free base credit.
    ClobberAll,
    /// Every call preserves every register: the per-file byte gate's DEFER probe.
    /// A register that verifies under this but NOT under `ClobberAll` is blocked
    /// SOLELY by calls (no local sp/`.w`/direct-clobber hazard), so the byte gate
    /// defers it to the corpus closure rather than erroring.
    PreserveAll,
    /// The closure's VERIFIED effective clobber map: a call to `callee` preserves
    /// rN iff `!callee_clobbers(effective, callee, rN)`. An indirect/unresolved
    /// target, an absent callee, a ⊤ effect, or a cycle member whose effect still
    /// names rN all read as "clobbers" — conservative.
    Oracle(&'a BTreeMap<String, RegEffect>),
}

/// Does a call to `callee` (`None` = indirect / unresolved target) provably
/// PRESERVE register `r` under `policy`? The entry-value oracle for the call
/// transfer (see [`CallPolicy`]).
fn call_preserves(policy: &CallPolicy, callee: Option<&str>, r: Reg) -> bool {
    match policy {
        CallPolicy::ClobberAll => false,
        CallPolicy::PreserveAll => true,
        CallPolicy::Oracle(effective) => callee.is_some_and(|c| !callee_clobbers(effective, c, r)),
    }
}

/// The proof outcome for one checked register.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreserveStatus {
    /// Proven preserved: on every exit in scope ([`ReturnScope`]) the register
    /// holds its entry value.
    Verified,
    /// Proven NOT preserved: some in-scope exit leaves it clobbered (a declared
    /// `preserves` here is a false contract).
    NotPreserved,
    /// A soundness bailout was hit (computed sp / sp escape / aliasing store) and
    /// the register is written — the proof is unavailable in either direction.
    Unverifiable(String),
}

/// Which register FACET a preserve claim is about. The `preserves(dN.w)` word
/// facet (§6 partial-width) is proven against the low 16 bits; the bare claim
/// against the FULL register. One dataflow computes BOTH exit maps; the facet
/// only selects which the verdict reads. `Full` ⟹ `Word` (a proof of the full
/// register subsumes the low word), so the `Word` verdict is monotonically
/// weaker than the `Full` one.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Facet {
    /// The full register — the bare `preserves(dN)` claim.
    Full,
    /// The low 16 bits — the `preserves(dN.w)` claim.
    Word,
}

/// A stack slot. `reg = Some(r)` holds register `r`'s entry value; `None` is
/// opaque. `bytes` is the TRUE pushed width (`.l`=4, `.w`=2, byte-on-a7=2, each
/// `movem` member its own size, a `link` frame its whole allocation) — the
/// prerequisite for the immediate-sp-cleanup idiom: an `addq/adda #N,sp` drops
/// whole slots totaling exactly N bytes, and a drop that lands mid-slot bails. A
/// bare `Option<Reg>` carries no byte size, so it could not tell a 2-byte word
/// scratch from a 4-byte long save.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Slot {
    reg: Option<Reg>,
    bytes: u32,
    /// The WIDTH of the push that created this slot — the facet the slot can
    /// restore. A `.l` push (4 bytes) round-trips the FULL register; a `.w` push
    /// (2 bytes) only the low WORD; a `.b` push (also 2 bytes on a7, but carrying
    /// one meaningful byte) round-trips neither the word nor the full register, so
    /// it is distinguished from `.w` here even though `bytes` cannot tell them
    /// apart. `None` for an opaque / unsized push.
    save_width: Option<Width>,
}

/// The stack byte width a push of size `size` consumes on a7. Long=4, word=2; a
/// BYTE push to `-(a7)` decrements sp by 2 (the 68000 keeps a7 word-aligned), so
/// byte=2. An unsized push (defaults to a long-shaped transfer) is 4.
fn slot_bytes(size: Option<Width>) -> u32 {
    match size {
        Some(Width::L) => 4,
        Some(Width::W) => 2,
        Some(Width::B) => 2,
        _ => 4,
    }
}

/// The abstract state at a program point: the symbolic stack + per-register
/// entry-value bits (indexed `d0`..`a7` = 0..16).
#[derive(Clone, PartialEq, Eq)]
struct State {
    stack: Vec<Slot>,
    entry: [bool; 16],
    /// Per-register LOW-WORD facet: `true` iff the register's low 16 bits equal
    /// their entry value. Weaker than `entry` (the FULL register) and maintained
    /// so that `entry[i]` ⟹ `entry_word[i]`: a `.w` save/restore round-trip
    /// credits THIS without crediting `entry`, a `.l` round-trip credits both, and
    /// any generic write clears both. The `preserves(dN.w)` facet reads this map.
    /// (§6 partial-width.)
    entry_word: [bool; 16],
    /// Per-register LINEAR DELTA from its entry value (§5 register-arithmetic
    /// extension): `Some(d)` means the register currently holds `entry + d`,
    /// tracked through `(rN)+`/`-(rN)`, `lea d(rN), rN`, and `adda/suba #imm, rN`;
    /// `None` means the value is no longer a static offset of entry (a fresh load,
    /// a computed write, a call, or a loop-join of conflicting deltas). A register
    /// is delta-preserved iff `Some(0)` at every `rts`. This proves the
    /// pointer-walk-and-restore idiom the stack model cannot (a0 advanced by
    /// `(a0)+` then restored by `lea -N(a0), a0` — DeleteObject's `.clear_slot`).
    /// `a7` (index 15) is not meaningful here (stack discipline).
    delta: [Option<i64>; 16],
    /// The open `link` frames, innermost last: `(frame pointer, stack depth just
    /// above its saved-fp slot)`. A `unlk aN` truncates back to the innermost
    /// mark and pops the saved fp; an `unlk` that names a different register, or
    /// one with no open frame, leaves sp at an unmodeled value → bail.
    frames: Vec<(Reg, usize)>,
    /// This path hit a soundness bailout (computed sp / escape / aliasing store /
    /// underflow / unbalanced-depth join). Rides the CFG; only matters if it
    /// reaches a return.
    bailed: bool,
}

impl State {
    /// The state on entry to a proc: nothing pushed, no open frame, every register
    /// holding its entry value at delta 0.
    fn at_entry() -> State {
        State {
            stack: Vec::new(),
            entry: [true; 16],
            entry_word: [true; 16],
            delta: [Some(0); 16],
            frames: Vec::new(),
            bailed: false,
        }
    }

    /// Bytes of tracked save area — the magnitude of sp's displacement from its
    /// entry value at this program point.
    fn depth_bytes(&self) -> i64 {
        self.stack.iter().map(|s| s.bytes as i64).sum()
    }
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
    (0..16).filter(|b| mask & (1 << b) != 0).map(|b| REG_BY_IDX[b]).collect()
}

/// The `RegList` mask among a movem's operands, if any.
fn reglist_mask(ops: &[CodeOperand]) -> Option<u16> {
    ops.iter().find_map(|o| match o {
        CodeOperand::RegList(m) => Some(*m),
        _ => None,
    })
}

/// Which exits the entry-value proof is obligated on.
#[derive(Clone, Copy)]
pub enum ReturnScope<'a> {
    /// EVERY exit that leaves the body: a return, a fall-off-end, and a transfer
    /// out ([`Edge::TailOut`] or [`Edge::BranchOut`]). The `preserves(rN)`
    /// contract — a transfer-out exit is an obligation site charged through the
    /// callee-preserves oracle ([`PreserveObserver::exit`]), which is what closes
    /// the tail-only vacuity hole; a diverging (`@noreturn` / authored-rail)
    /// transfer carries no obligation.
    AllReturns,
    /// Exactly these instruction indices, counting ANY edge that leaves the proc
    /// — a return or a transfer out — charged at its RAW entry bits. The ¬cc
    /// exits of an `out(rN if cc)` survives-claim
    /// ([`out_verify::check_cond_out_survives`](crate::out_verify::check_cond_out_survives)),
    /// which must hold wherever control leaves, not only where it `rts`.
    Sites(&'a BTreeSet<usize>),
}

/// Verify `preserves(rN)` for each register in `check` over a proc's evaluated
/// CodeBuf `items`. One forward dataflow serves all checked registers.
///
/// `falls_into` names the proc's declared fall-through successor (if any) and
/// `noreturn` the visible `@noreturn` symbol set — both fed to the exit
/// checkpoints so a tail transfer / fall-through into a callee is credited
/// through the callee-preserves oracle (the 68k analog of the Z80 credit; see
/// [`PreserveObserver::exit`]). Pass `None` / an empty set for a body whose tail
/// edges carry no such credit.
pub fn verify_preserved(
    items: &[CodeItem],
    check: &[Reg],
    policy: CallPolicy,
    falls_into: Option<&str>,
    noreturn: &BTreeSet<String>,
) -> BTreeMap<Reg, PreserveStatus> {
    verify_preserved_on(items, check, policy, ReturnScope::AllReturns, Facet::Full, falls_into, noreturn)
}

/// [`verify_preserved`] for the LOW-WORD facet (`preserves(dN.w)`, §6): a register
/// is Verified iff its low 16 bits hold their entry value on every in-scope exit —
/// via a `.w` OR `.l` save/restore round-trip, a full-register proof (the delta
/// round-trip or a `.l` restore, both of which subsume the word), or never being
/// written. The full proof implies the word, so this verdict is monotonically
/// WEAKER than [`verify_preserved`]'s.
pub fn verify_preserved_word(
    items: &[CodeItem],
    check: &[Reg],
    policy: CallPolicy,
    falls_into: Option<&str>,
    noreturn: &BTreeSet<String>,
) -> BTreeMap<Reg, PreserveStatus> {
    verify_preserved_on(items, check, policy, ReturnScope::AllReturns, Facet::Word, falls_into, noreturn)
}

/// [`verify_preserved`] with the obligated exits chosen by `scope` — the entry
/// point the ¬cc survives-claim uses. Everything about the proof is identical;
/// only WHICH exits must show the entry value differs. `falls_into`/`noreturn`
/// carry the tail-credit context (see [`verify_preserved`]); the `Sites` scope
/// (the survives-claim) charges its named exits at their raw entry bits, so the
/// tail credit is applied ONLY under `AllReturns` (a `preserves` contract).
pub fn verify_preserved_on(
    items: &[CodeItem],
    check: &[Reg],
    policy: CallPolicy,
    scope: ReturnScope<'_>,
    facet: Facet,
    falls_into: Option<&str>,
    noreturn: &BTreeSet<String>,
) -> BTreeMap<Reg, PreserveStatus> {
    let cfg = Cfg::build(items);

    // The first instruction is the entry point; a body with no instructions
    // preserves everything vacuously — UNLESS the proc declares a `falls_into`
    // successor: then control flows straight through the empty body into that
    // successor's frame, so rN survives iff the successor itself preserves it
    // (mirrors `z80_preserves.rs`'s empty-body arm; an absent/indirect/unknown
    // successor preserves nothing, via the oracle). A `Sites` scope has no exits
    // in an empty body, so it is unaffected.
    let Some(entry_idx) = entry_instr_idx(items) else {
        return check
            .iter()
            .map(|r| {
                let status = match (scope, falls_into) {
                    (ReturnScope::AllReturns, Some(succ))
                        if !call_preserves(&policy, Some(succ), *r) =>
                    {
                        PreserveStatus::NotPreserved
                    }
                    _ => PreserveStatus::Verified,
                };
                (*r, status)
            })
            .collect();
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
            // `instr_written_regs` expands only NON-stack movem-load reglists
            // (it exempts `(sp)+` restores as preserve-discipline); this pass
            // needs the ISA-true set — a register that only ever appears in a
            // save/restore movem still participates in stack traffic, so it is
            // NOT "never written". Mask-expand every movem reglist ourselves
            // (idempotent with effect (4) for non-stack loads).
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
    // A transfer out of the body (either edge flavor) and the declared `falls_into`
    // successor exit into a callee that clobbers every register it does not
    // provably preserve — so a register the tail-callee destroys must not stay
    // credited past a bailout as "never written". Mirrors `z80_preserves.rs`'s
    // tail/falls_into marking. A DIVERGENT tail (an `@noreturn` target or an
    // `AssertDesugar`-authored rail) never returns to the caller, so it charges
    // nothing. AllReturns only — a `Sites` scope keeps its exact former set.
    if matches!(scope, ReturnScope::AllReturns) {
        for (idx, it) in items.iter().enumerate() {
            let CodeItem::Instr { ops, .. } = it else { continue };
            if !cfg.edges(idx).iter().any(|e| matches!(e, Edge::TailOut | Edge::BranchOut)) {
                continue;
            }
            if exit_diverges(it, noreturn) {
                continue; // never returns to the caller — charges nothing
            }
            let callee = transfer_target_sym(ops);
            for (i, r) in REG_BY_IDX.iter().enumerate() {
                if *r != Reg::A7 && !call_preserves(&policy, callee, *r) {
                    ever_clobbered[i] = true;
                }
            }
        }
        if let Some(succ) = falls_into {
            for (i, r) in REG_BY_IDX.iter().enumerate() {
                if *r != Reg::A7 && !call_preserves(&policy, Some(succ), *r) {
                    ever_clobbered[i] = true;
                }
            }
        }
    }

    // For each checked register: does EVERY in-scope exit see it at its entry
    // value? Starts true; an exit with a clobbered value flips it false. Plus, per
    // checked register, whether its linear delta is `Some(0)` at EVERY in-scope
    // exit — an independent POSITIVE proof (the register-arithmetic round-trip),
    // valid even past a stack bailout (a computed-sp hazard does not touch a
    // register whose value is a proven static offset of entry).
    let mut obs = PreserveObserver {
        check,
        scope,
        policy,
        items,
        falls_into,
        noreturn,
        all_exits_preserve: check.iter().map(|r| (*r, true)).collect(),
        all_exits_preserve_word: check.iter().map(|r| (*r, true)).collect(),
        delta_ok: check.iter().map(|r| (*r, true)).collect(),
        saw_exit: false,
        bailed_reached_exit: false,
    };
    let bail_reason = run_stack_dataflow(&cfg, entry_idx, items, &policy, &mut obs);
    let PreserveObserver {
        all_exits_preserve, all_exits_preserve_word, delta_ok, saw_exit, bailed_reached_exit, ..
    } = obs;

    // Resolve each checked register's status.
    check
        .iter()
        .map(|r| {
            let clobbered = ever_clobbered[reg_idx(*r)];
            // The exit map the facet reads: the FULL entry bits, or the low-WORD
            // ones. The delta round-trip and the bailout arms are facet-independent
            // (a delta proof of the full register subsumes the word; a bailout that
            // blocks the round-trip blocks it for either facet).
            let exits_preserve = match facet {
                Facet::Full => &all_exits_preserve,
                Facet::Word => &all_exits_preserve_word,
            };
            let status = if !saw_exit || delta_ok[r] {
                // No in-scope exit (vacuous), OR the linear-delta round-trip proves
                // the register holds its entry value at EVERY one — a positive proof
                // that overrides a stack bailout (the hazard is about sp, not this
                // register's static offset). This is the register-arithmetic
                // extension: DeleteObject's `(a0)+`…`lea -N(a0), a0` verifies here.
                PreserveStatus::Verified
            } else if bailed_reached_exit && clobbered {
                // A bail reached a return and this register is written somewhere,
                // with no delta proof — the stack model can't prove the round-trip.
                PreserveStatus::Unverifiable(
                    bail_reason.clone().unwrap_or_else(|| "unverifiable stack".to_string()),
                )
            } else if exits_preserve[r] {
                PreserveStatus::Verified
            } else {
                PreserveStatus::NotPreserved
            };
            (*r, status)
        })
        .collect()
}

/// Does the transfer-out at this item DIVERGE — i.e. never come back to this
/// proc's caller?
///
/// Two ways, and both must be here rather than in a consumer: an `@noreturn`
/// target, and an `AssertDesugar`-authored assert/raise rail (compiler-authored,
/// so it carries no symbol a consumer could look up). A divergent exit owes the
/// caller nothing — it is not a return path for ANY per-exit obligation.
///
/// THE SHARED SUCCESSOR-AWARE PREDICATE, and the two facts are NOT the same kind
/// of fact — conflating them is itself a defect this project has paid for:
///
/// - A divergent tail is not a return path AT ALL. The obligation DISAPPEARS,
///   because there is no caller to owe anything to. That is this predicate.
/// - A declared `falls_into` successor does NOT make a fall-off vanish. The
///   fall-off is still an exit; its obligation MOVES to the successor and is
///   charged against the successor's contract. `preserves` does this by deferring
///   to `checkpoint_after_tail(st, Some(succ))`, and `out_verify` by crediting the
///   successor's verified out — both exactly as they charge a tail transfer.
///
/// Reading the second as "the exit is exempt" re-opens the vacuity hole this file
/// documents closing: a proc whose only exit is a fall-off would have ZERO
/// obligation sites, so every claim it declares would verify on no evidence.
///
/// `preserves` carries both facts; `out_verify` reads this one and takes the
/// successor NAME for the other. The stack walk takes a `falls_into` BOOL, and it
/// is the one place where dropping the charge is sound — the successor's own
/// unconditional balance check discharges it. No value obligation has that
/// discharge, so the flag shape does not generalise.
pub(crate) fn exit_diverges(item: &CodeItem, noreturn: &BTreeSet<String>) -> bool {
    match item {
        CodeItem::Instr { ops, author, .. } => {
            matches!(author, ItemAuthor::AssertDesugar)
                || transfer_target_sym(ops).is_some_and(|t| noreturn.contains(t))
        }
        _ => false,
    }
}

/// How control left the proc at an exit the shared walk reports.
///
/// Carried straight through from the CFG builder, and NOT one variant per [`Edge`]:
/// the four leaving edges ([`Edge::Return`], [`Edge::FallOff`], [`Edge::TailOut`],
/// [`Edge::BranchOut`]) collapse to three kinds, because both transfer-out flavors
/// reach the same arm in both consumers (see [`ExitKind::Defer`]). The collapse is
/// deliberate: this enum carries the distinctions its consumers ACT on, and a
/// fourth kind no arm reads would be surface. Each consumer states its own POLICY
/// as a match over these — the two that differ (whether a fall-off-end is a return)
/// differ in the arm, not in a re-reading of the exit instruction's mnemonic.
#[derive(Clone, Copy)]
enum ExitKind {
    /// [`Edge::Return`] — an `rts`/`rte`/`rtr`/`rtd`.
    Return,
    /// [`Edge::FallOff`] — control ran past the last instruction of the body.
    FallOff,
    /// A transfer out of the body — [`Edge::TailOut`] or [`Edge::BranchOut`].
    /// One kind for both flavors: both of this enum's consumers (the entry-value
    /// proof and the stack-balance checker) charge the callee oracle identically
    /// whatever terminator produced the exit, so the distinction would be surface
    /// with no consumer.
    Defer,
}

/// What a consumer of the symbolic-stack dataflow observes as it walks a proc.
///
/// **The anti-drift seam.** Both consumers of [`State`] — the entry-value proof
/// and the stack-balance checker — run through [`run_stack_dataflow`], so there is
/// exactly ONE worklist, ONE [`transfer`] and ONE [`join`], therefore ONE bailout
/// set. A consumer chooses what to CONCLUDE from a state; it never gets to decide
/// whether the state is trustworthy. That is why the two cannot drift apart on
/// `State::bailed`. ([`find_dead_saves`] models the stack SEPARATELY, over its own
/// `DsState`, with a strictly more conservative bailout set — safe for a
/// code-cutting worklist, and outside this seam.)
trait StackObserver {
    /// Control leaves the proc at instruction `idx` carrying state `st`, by an
    /// exit of kind `exit`.
    fn exit(&mut self, idx: usize, st: &State, exit: ExitKind);

    /// Two paths meet on entry to instruction `succ`. Called with BOTH incoming
    /// states before [`join`] folds them, so a consumer can read a disagreement
    /// the join deliberately erases (it answers a mismatch with a bail).
    fn merge(&mut self, _succ: usize, _existing: &State, _incoming: &State) {}
}

/// The shared forward dataflow over the symbolic stack: walk `items` from
/// `entry_idx` to fixpoint, applying [`transfer`] per instruction and [`join`] at
/// every merge, reporting exits and merges to `obs`. Returns the FIRST bailout
/// reason encountered, or `None` if the model held everywhere.
///
/// A bailout RIDES THE CFG rather than stopping the walk: it taints the state
/// ([`State::bailed`]), so a path must still reach its terminator and consumers
/// can tell a returning bail from a diverging one. It reaches only what the
/// tainted path reaches — a bail on a diverging branch never touches a returning
/// one, though [`join`] does propagate it into any merge it participates in. Under
/// [`ReturnScope::AllReturns`] a bail on a noreturn `Defer` path (the DEBUG
/// `raise_error` `subq #2,sp` → `jmp handler` shape) constrains nothing.
fn run_stack_dataflow<O: StackObserver>(
    cfg: &Cfg,
    entry_idx: usize,
    items: &[CodeItem],
    policy: &CallPolicy,
    obs: &mut O,
) -> Option<String> {
    let mut in_state: BTreeMap<usize, State> = BTreeMap::new();
    in_state.insert(entry_idx, State::at_entry());
    let mut work: VecDeque<usize> = VecDeque::from([entry_idx]);
    let mut bail_reason: Option<String> = None;

    while let Some(idx) = work.pop_front() {
        let mut st = in_state[&idx].clone();
        if !st.bailed {
            if let Some(reason) = transfer(cfg, idx, &mut st, items, policy) {
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
                            obs.merge(succ, existing, &st);
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
                Edge::Return => obs.exit(idx, &st, ExitKind::Return),
                Edge::FallOff => obs.exit(idx, &st, ExitKind::FallOff),
                // An external tail transfer (`jmp`/`bra` to a non-local symbol) is
                // NOT an `rts` of THIS proc: it either diverges (a noreturn
                // `raise_error` / error handler, which owes the caller nothing) or
                // is a real tail call whose effect is a TRANSITIVE property the
                // closure accounts for via its own tail edge (corpus
                // `TAIL_MNEMONICS`). Consumers decide whether to charge it.
                // Both flavors of transfer out are charged identically: the exit
                // is an exit whatever terminator produced it, and every consumer
                // of `ExitKind::Defer` charges the callee oracle the same way.
                Edge::TailOut | Edge::BranchOut => obs.exit(idx, &st, ExitKind::Defer),
            }
        }
    }
    bail_reason
}

/// The entry-value proof's view of the shared walk: charge every IN-SCOPE exit
/// against the checked registers, ignore merges. Holds the tail-credit context
/// (`policy`/`items`/`falls_into`/`noreturn`) so a `Defer` or `falls_into`
/// exit consults the callee-preserves oracle at the point of consumption.
struct PreserveObserver<'a> {
    check: &'a [Reg],
    scope: ReturnScope<'a>,
    policy: CallPolicy<'a>,
    items: &'a [CodeItem],
    falls_into: Option<&'a str>,
    noreturn: &'a BTreeSet<String>,
    all_exits_preserve: BTreeMap<Reg, bool>,
    /// The low-WORD companion to `all_exits_preserve` — an exit that leaves a
    /// register's low word clobbered flips it false. Read for the `Facet::Word`
    /// verdict. (§6 partial-width.)
    all_exits_preserve_word: BTreeMap<Reg, bool>,
    delta_ok: BTreeMap<Reg, bool>,
    saw_exit: bool,
    bailed_reached_exit: bool,
}

impl StackObserver for PreserveObserver<'_> {
    fn exit(&mut self, idx: usize, st: &State, exit: ExitKind) {
        match self.scope {
            // A `Sites` scope (the ¬cc survives-claim) names its own exits and
            // charges each at its RAW entry bits — a register already destroyed
            // HERE is destroyed from the caller's view whatever the target does.
            // The tail credit is a `preserves`-contract concept and does not apply.
            ReturnScope::Sites(s) => {
                if s.contains(&idx) {
                    self.checkpoint(st);
                }
            }
            // A `preserves(rN)` contract. Every exit that ENDS THIS BODY is an
            // obligation site — the vacuity hole (a tail-only proc used to have
            // ZERO obligation sites) closes here, in the same motion that grants
            // the tail credit. This is the 68k mirror of `z80_preserves.rs`'s
            // transfer-out (`TailOut`/`BranchOut`) and `FallOff` arms.
            ReturnScope::AllReturns => match exit {
                // A plain return hands the register file straight back.
                ExitKind::Return => self.checkpoint(st),
                // Control off the end. With a declared `falls_into` successor the
                // closing `}` continues into that successor's frame — rN survives
                // iff it holds its entry value here AND the successor preserves it
                // (the same charge the transfer-out arm below makes to a
                // tail-callee). Without one, running off the end hands the register
                // file back like a return.
                ExitKind::FallOff => match self.falls_into {
                    Some(succ) => self.checkpoint_after_tail(st, Some(succ)),
                    None => self.checkpoint(st),
                },
                // A transfer out of the body — an unconditional tail
                // ([`Edge::TailOut`]) or a conditional terminator's taken leg
                // ([`Edge::BranchOut`]); both arrive here. It either DIVERGES — an
                // `@noreturn` target, or an `AssertDesugar`-authored assert/raise
                // rail, which never returns to the caller and so carries no
                // preserves obligation (the `@noreturn` composition) — or it is a
                // real transfer whose callee owns the rest of the path: rN survives
                // iff it holds its entry value AT the transfer AND the callee itself
                // preserves rN (unknown / indirect / equ-boundary target →
                // conservative clobber, via the oracle). A conditional leg is
                // charged the same way, which OVER-obligates rather than misses —
                // this check's safe polarity.
                ExitKind::Defer => {
                    let items = self.items;
                    let callee = match &items[idx] {
                        CodeItem::Instr { ops, author, .. }
                            if !matches!(author, ItemAuthor::AssertDesugar) =>
                        {
                            transfer_target_sym(ops)
                        }
                        _ => None,
                    };
                    let divergent = exit_diverges(&items[idx], self.noreturn);
                    if divergent {
                        return; // a diverging exit never returns — no obligation
                    }
                    self.checkpoint_after_tail(st, callee);
                }
            },
        }
    }
}

/// Apply a transfer of control to `callee` (`None` = indirect / unresolved) to
/// the entry-value bits: a register the callee does not provably PRESERVE under
/// `policy` no longer holds its entry value. Linear-delta tracking ends for every
/// register either way — the callee may recompute them past what the delta model
/// follows, conservative regardless of policy. Shared by the CALL transfer and
/// the scoped TAIL-exit charge, which owe the register file the same debt: both
/// hand control to code this proof does not read.
fn apply_callee_effect(st: &mut State, policy: &CallPolicy, callee: Option<&str>) {
    for (i, r) in REG_BY_IDX.iter().enumerate() {
        if !call_preserves(policy, callee, *r) {
            st.entry[i] = false;
            // Conservative v1: a callee's word facet is invisible here, so a
            // register the callee does not provably FULLY preserve is a clobber of
            // BOTH facets (the low word included). This is what makes a caller
            // claiming `preserves(dN)` OR `preserves(dN.w)` through a `.w`-preserving
            // callee still refuse — the callee reads as a full clobber.
            st.entry_word[i] = false;
        }
    }
    st.delta = [None; 16];
}

impl PreserveObserver<'_> {
    /// Charge a TAIL exit (a `Defer` transfer, or a `falls_into` fall-off) to
    /// `callee`: apply the callee-preserves oracle to a COPY of the exit state —
    /// a register the callee does not provably preserve loses its entry bit, and
    /// every register's linear delta ends (the callee may recompute it) — then
    /// checkpoint that state. So rN survives iff it held its entry value AT the
    /// jump AND the callee preserves it, exactly as the CALL transfer charges an
    /// in-body call. `callee = None` (indirect / equ boundary) preserves nothing.
    fn checkpoint_after_tail(&mut self, st: &State, callee: Option<&str>) {
        let mut st2 = st.clone();
        apply_callee_effect(&mut st2, &self.policy, callee);
        self.checkpoint(&st2);
    }

    /// Charge one in-scope exit against every checked register: the linear-delta
    /// proof needs Δ==0 on an unbailed path, and the stack/entry-bit model needs
    /// the entry bit still set.
    fn checkpoint(&mut self, st: &State) {
        self.saw_exit = true;
        for r in self.check {
            // The linear-delta proof holds here iff Δ==0 AND the path did NOT
            // bail: a bail FREEZES the transfer, so `delta` after it is STALE —
            // it cannot prove anything.
            if st.bailed || st.delta[reg_idx(*r)] != Some(0) {
                *self.delta_ok.get_mut(r).unwrap() = false;
            }
        }
        if st.bailed {
            self.bailed_reached_exit = true;
        } else {
            for r in self.check {
                if !st.entry[reg_idx(*r)] {
                    *self.all_exits_preserve.get_mut(r).unwrap() = false;
                }
                if !st.entry_word[reg_idx(*r)] {
                    *self.all_exits_preserve_word.get_mut(r).unwrap() = false;
                }
            }
        }
    }
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

/// A SAFE displaced-sp READ: `move`/`movea` with a `d(sp)` SOURCE and a plain
/// register destination that is NOT a7 — the movem-frame-slot recovery idiom
/// (`movea.l 8(sp), aN`, e.g. EntityWindow_TrySpawnObject). A load reads the frame
/// into a register; it cannot alter a tracked slot's contents (only a store could
/// alias). The displaced analogue of [`is_peek`], it takes the normal
/// register-write handling. Deliberately NARROW — move/movea only (a `d(sp)` write
/// target, an rmw / non-move mnemonic, an `(sp,Xn)` indexed form, or a
/// stack-replacing `d(sp) → sp` all stay hazards).
fn is_safe_sp_disp_read(mnem: &str, ops: &[CodeOperand]) -> bool {
    (mnem == "move" || mnem == "movea")
        && matches!(ops.first(), Some(CodeOperand::DispInd { reg: Reg::A7, .. }))
        && matches!(ops.last(), Some(CodeOperand::Reg(r)) if *r != Reg::A7)
}

/// A plain `(sp)` READ: the top slot is the SOURCE and the destination is
/// something else. A load cannot alter a slot's contents, so it is not an alias
/// hazard whatever the mnemonic — `adda.w (sp), a2` reads the top word exactly as
/// `movea.l (sp), a1` does. The STORE direction (`(sp)` as the destination, e.g.
/// `add.l d0, (sp)`) is the hazard, and so is a read-modify-write naming `(sp)`
/// on both ends. [`is_peek`] stays the NARROWER question of which loads also
/// RESTORE a register's entry value.
fn is_sp_top_read(ops: &[CodeOperand]) -> bool {
    matches!(ops.first(), Some(CodeOperand::Ind(Reg::A7)))
        && !matches!(ops.last(), Some(CodeOperand::Ind(Reg::A7)))
}

/// An unmodeled sp hazard → bail: a bare `a7` operand (sp's value used directly —
/// escapes into address math or a computed `adda #n,sp`), or an sp access that
/// could ALIAS a tracked slot — displaced (`d(sp)`), indexed (`(sp,Xn)`), or the
/// exact top-of-stack (`(sp)`). The clean stack forms use `PreDec`/`PostInc` of
/// a7, never these. Two READ forms are exempted at the call site, since a load
/// cannot alter a slot's contents: the `(sp)` peek ([`is_peek`]) and the
/// displaced-frame read ([`is_safe_sp_disp_read`]).
fn sp_hazard(ops: &[CodeOperand]) -> bool {
    ops.iter().any(|o| {
        matches!(
            o,
            CodeOperand::Reg(Reg::A7)
                | CodeOperand::Ind(Reg::A7)
                | CodeOperand::DispInd { reg: Reg::A7, .. }
                | CodeOperand::IndIdx { reg: Reg::A7, .. }
                | CodeOperand::IndIdx { xn: Reg::A7, .. }
        )
    })
}

/// If `(mnem, ops)` is an IMMEDIATE sp-INCREASE (`add`/`addq`/`adda #N, sp`), the
/// byte count `N` it deallocates; `None` otherwise. Deliberately NARROW:
/// - ADD family only — a `sub`/`subq`/`suba #N, sp` (scratch ALLOCATION, an
///   sp-DECREASE) is a hazard the slot model does not follow (constraint 1);
/// - IMMEDIATE source only — a register/index source (`adda dN, sp`) is a computed
///   advance with no static byte count (constraint 1);
/// - non-negative `N` — a negative immediate is not a real cleanup.
fn immediate_sp_cleanup_bytes(mnem: &str, ops: &[CodeOperand]) -> Option<i64> {
    if !matches!(mnem, "add" | "addq" | "adda") {
        return None;
    }
    // dest = sp.
    if !matches!(ops.last(), Some(CodeOperand::Reg(Reg::A7))) {
        return None;
    }
    match ops.first() {
        Some(CodeOperand::Imm(v)) if *v >= 0 => Some(*v as i64),
        _ => None,
    }
}

/// Drop exactly `n` BYTES of tracked slots off the top of `st.stack` (an immediate
/// sp-cleanup). Returns `Some(reason)` on a bail:
/// - the drop lands MID-SLOT (`n` does not consume whole slots) — a partial-slot
///   drop the model cannot represent (constraint 2);
/// - the drop drains MORE than is tracked — it reaches into the caller frame /
///   return address, so the model is inconsistent (the pop-underflow sibling).
///
/// Over-drop soundness (constraint 3) needs no special case: dropping the slot
/// that held a saved register simply removes it, and a later pop then reads
/// whatever slot is now on top — a different register's value, so the entry-bit
/// model REFUTES the preserve on its own.
fn apply_sp_cleanup(st: &mut State, n: i64) -> Option<String> {
    let mut remaining = n;
    while remaining > 0 {
        match st.stack.pop() {
            None => {
                return Some("sp cleanup drains more than was pushed".to_string());
            }
            Some(slot) => {
                remaining -= slot.bytes as i64;
                if remaining < 0 {
                    return Some(
                        "sp cleanup lands mid-slot — a partial-slot drop cannot be modeled"
                            .to_string(),
                    );
                }
            }
        }
    }
    None
}

/// Apply instruction `idx`'s effect to `st`. Returns `Some(reason)` on a
/// soundness bailout.
fn transfer(
    cfg: &Cfg,
    idx: usize,
    st: &mut State,
    items: &[CodeItem],
    policy: &CallPolicy,
) -> Option<String> {
    let (mnem, ops) = cfg.instr(idx)?;
    // Only a FULL (`.l`) transfer round-trips the WHOLE register; a `.w` restore
    // round-trips just the low WORD (crediting `entry_word`, the `preserves(dN.w)`
    // facet), and a `.b` restore neither. `is_long` gates the full-facet credit;
    // the word facet reads the push/pop widths through [`credit_restore`].
    let is_long = matches!(instr_size(items, idx), Some(Width::L));

    // A call nets zero on the stack (its pushed return address is popped by its own
    // rts) and always ends linear-delta tracking for every register (the callee may
    // recompute them past what the delta model follows — conservative regardless of
    // policy). Its effect on the ENTRY-value bits is the CALLEE-PRESERVES ORACLE
    // ([`CallPolicy`]): a register the callee provably preserves keeps its entry
    // bit; every other is clobbered.
    if is_call(mnem) {
        apply_callee_effect(st, policy, call_target(ops));
        return None;
    }

    // FRAME OPEN / CLOSE — `link aN, #-d` / `unlk aN`. Modeled BEFORE the
    // sp_hazard bail: `link` moves sp without naming it, so an unmodeled `link`
    // would leave the slot map claiming a depth the machine does not have.
    if mnem == "link" {
        return apply_link(st, ops);
    }
    if mnem == "unlk" {
        return apply_unlk(st, ops);
    }

    // IMMEDIATE sp-INCREASE cleanup (`addq/adda #N,sp`): deallocate a save area by
    // dropping exactly N BYTES of tracked slots off the top. Modeled BEFORE the
    // sp_hazard bail (its `sp` operand would otherwise read as an escape). A
    // constant sp-DECREASE (scratch alloc) and a COMPUTED (register/index) sp op
    // are NOT this idiom — they fall through to the hazard bail below.
    if let Some(n) = immediate_sp_cleanup_bytes(mnem, ops) {
        return apply_sp_cleanup(st, n);
    }

    // Unmodeled sp manipulation corrupts the slot model — EXCEPT a displaced-sp
    // READ into a plain register (`movea.l d(sp), aN`), the displaced analogue of
    // the `(sp)` peek: a load cannot alias a tracked slot (only a store could), so
    // it is safe and takes the normal register-write handling below (§5 grow —
    // sp_hazard's own "could alias" rationale is write-only).
    if sp_hazard(ops) && !is_sp_top_read(ops) && !is_safe_sp_disp_read(mnem, ops) {
        return Some(sp_hazard_reason(ops));
    }

    // PUSH — `-(sp)`.
    if is_push(ops) {
        let width = instr_size(items, idx);
        let bytes = slot_bytes(width);
        if let Some(mask) = reglist_mask(ops) {
            // movem save: push in REVERSE canonical order so the lowest register
            // (d0) lands on top, matching the (sp)+ restore order. Each member is
            // `bytes` wide (a `.w` movem stores 2 bytes per member).
            for r in expand_mask(mask).into_iter().rev() {
                st.stack.push(tag(st, r, bytes, width));
            }
        } else {
            // Single push: the SOURCE (first operand) if it is a plain register.
            let slot = match ops.first() {
                Some(CodeOperand::Reg(r)) => tag(st, *r, bytes, width),
                _ => Slot { reg: None, bytes, save_width: width }, // non-register → opaque
            };
            st.stack.push(slot);
        }
        return None;
    }

    // POP — `(sp)+`. The transfer's byte width is the truth: an `n`-register pop
    // of size `sz` consumes `n * sz` BYTES, and the slots it drains must total
    // exactly that. A pop whose width disagrees with the slots beneath it (a `.l`
    // pop over two `.w` pushes) leaves sp somewhere the slot map cannot name, and
    // a pop that drains more than is tracked reaches into the caller's frame —
    // both make the model inconsistent → bail. (Counting SLOTS instead of bytes
    // would let the mismatched case report a depth the machine does not have,
    // which at ERROR tier is a false positive.)
    if is_pop(ops) {
        let regs: Vec<Reg> = match reglist_mask(ops) {
            Some(mask) => expand_mask(mask),
            None => match ops.last() {
                Some(CodeOperand::Reg(dst)) => vec![*dst],
                // A non-register destination (`move.w (sp)+, sr`) restores no
                // tracked register but still drains one transfer's worth.
                _ => Vec::new(),
            },
        };
        let pop_w = instr_size(items, idx);
        let per = slot_bytes(pop_w) as i64;
        let want = per * regs.len().max(1) as i64;
        let mut got = 0i64;
        // movem restores in canonical ascending order; a single pop is one reg.
        // Drain first so a width disagreement is caught before any entry bit is
        // set from a slot the pop did not actually consume.
        let mut popped: Vec<Slot> = Vec::with_capacity(regs.len());
        while got < want {
            let Some(slot) = st.stack.pop() else {
                return Some("stack underflow — pop drains more than was pushed".to_string());
            };
            got += slot.bytes as i64;
            popped.push(slot);
        }
        if got != want {
            return Some(
                "pop width disagrees with the slots beneath it — sp lands mid-slot".to_string(),
            );
        }
        for (r, slot) in regs.iter().zip(popped) {
            // Credit the full and/or word facet per the pop and save widths; a pop
            // LOADS the register (a fresh value), so the delta tracker gives up.
            credit_restore(st, *r, slot, is_long, pop_w);
        }
        return None;
    }

    // PEEK — `(sp)` (no stack change), a read of the top slot.
    if is_peek(mnem, ops) {
        if let Some(CodeOperand::Reg(dst)) = ops.last() {
            let peek_w = instr_size(items, idx);
            let top = st.stack.last().copied().unwrap_or(EMPTY_SLOT);
            credit_restore(st, *dst, top, is_long, peek_w);
        }
        return None;
    }

    // Plain instruction: every generic write (except a7) clears the register's
    // entry-value bit and updates its linear delta. A TRACKABLE arithmetic write
    // (`(rN)+`/`-(rN)` advance, `lea d(rN), rN`, `adda/suba #imm, rN`) accumulates
    // the static offset; any other write to a register whose delta is still Some
    // makes it untrackable (a fresh/computed value).
    let width = instr_size(items, idx);
    for r in instr_written_regs(mnem, ops) {
        if r == Reg::A7 {
            continue;
        }
        st.entry[reg_idx(r)] = false;
        // Any write to the register changes at least its low word (a `.b`/`.w`
        // write touches the low bytes; a `.l`/`swap` the whole register), so the
        // word facet is cleared too — a later `.w` restore re-sets it.
        st.entry_word[reg_idx(r)] = false;
        if let Some(d) = st.delta[reg_idx(r)] {
            st.delta[reg_idx(r)] = linear_write_amount(mnem, ops, width, r).map(|amount| d + amount);
        }
    }
    None
}

/// A shared empty slot (no register, zero bytes) — the `(sp)` peek's fallback
/// when the stack is empty (an underflow the balance model reports separately).
const EMPTY_SLOT: Slot = Slot { reg: None, bytes: 0, save_width: None };

/// Credit a restore of `r` from `slot` (a pop or a `(sp)` peek of width `restore_w`,
/// `is_long` = the restore is `.l`) to the entry-value bits.
///
/// **The restore width must MATCH the save width**, for both facets. A round-trip
/// is only a round-trip when the bytes read back are the bytes written: on a
/// big-endian 68000 a `.w` read of a `.l`-saved slot loads the saved HIGH word into
/// the register's LOW word, so it restores nothing about the entry low word, and a
/// `.l` read of a `.w`-saved slot pulls in whatever sits beneath the slot. The POP
/// arm gets the same discipline from its byte-balance check only in TOTAL, and the
/// `(sp)` PEEK arm has no balance check at all, so the guard belongs here where
/// both consumers share it.
///
/// - FULL round-trip: a `.l` restore of a `.l`-saved slot holding `r`.
/// - WORD round-trip: an equal-width `.w`-or-`.l` save/restore pair holding `r` — a
///   `.b` pair round-trips only the byte, so it credits neither facet.
///
/// Full implies word (a matched `.l` pair satisfies both), keeping the
/// `entry ⟹ entry_word` invariant.
fn credit_restore(st: &mut State, r: Reg, slot: Slot, is_long: bool, restore_w: Option<Width>) {
    let matched = slot.reg == Some(r);
    // An UNSIZED push transfers a long — the same default `slot_bytes` charges it
    // 4 bytes for. The restore side keeps the strict `Some(Width::L)` test
    // (`is_long`), so an unsized restore credits nothing.
    let save_w = slot.save_width.unwrap_or(Width::L);
    let full = matched && is_long && save_w == Width::L;
    st.entry[reg_idx(r)] = full;
    let word_rt = matched
        && matches!(restore_w, Some(Width::W) | Some(Width::L))
        && restore_w == Some(save_w);
    st.entry_word[reg_idx(r)] = full || word_rt;
    st.delta[reg_idx(r)] = None;
}

/// Open a stack frame: `link aN, #-d` pushes aN (4 bytes), sets aN to the new sp,
/// then allocates `d` bytes of locals. The saved-fp push is an ordinary tagged
/// slot, so a frame is visible to the depth model like any other save; the
/// allocation is one opaque slot of the true byte size, and the `(aN, depth)`
/// mark lets the matching `unlk` truncate back.
///
/// Bails on the forms the model cannot represent: a non-register or `a7` frame
/// pointer, a non-immediate displacement, or a POSITIVE displacement (an sp
/// INCREASE, which would reach into the caller's frame rather than allocate).
fn apply_link(st: &mut State, ops: &[CodeOperand]) -> Option<String> {
    let Some(CodeOperand::Reg(fp)) = ops.first() else {
        return Some("`link` with a non-register frame pointer".to_string());
    };
    if *fp == Reg::A7 {
        return Some("`link a7` — the frame pointer is sp itself".to_string());
    }
    let Some(CodeOperand::Imm(disp)) = ops.last() else {
        return Some("`link` with a non-immediate frame size".to_string());
    };
    if *disp > 0 {
        return Some(
            "`link` with a positive displacement raises sp into the caller frame".to_string(),
        );
    }
    // The allocation must fit the slot's byte width to be tracked at all. This
    // rejects both a nonsense frame size and `i128::MIN`, whose negation does not
    // exist — `checked_neg` answers the second before the width test sees it.
    let Some(alloc) = disp.checked_neg().and_then(|n| u32::try_from(n).ok()) else {
        return Some("`link` frame size does not fit the tracked slot width".to_string());
    };
    let slot = tag(st, *fp, 4, Some(Width::L));
    st.stack.push(slot);
    st.frames.push((*fp, st.stack.len()));
    if alloc > 0 {
        st.stack.push(Slot { reg: None, bytes: alloc, save_width: None });
    }
    // aN now holds sp, not its entry value, and is no longer a static offset of it.
    st.entry[reg_idx(*fp)] = false;
    st.entry_word[reg_idx(*fp)] = false;
    st.delta[reg_idx(*fp)] = None;
    None
}

/// Close a stack frame: `unlk aN` sets sp back to aN (discarding the locals and
/// anything else pushed inside the frame), then pops the saved frame pointer.
///
/// The PAIRING RULE's provable direction. An `unlk` whose innermost open frame
/// names a different register — or which has no open frame at all — leaves sp at a
/// value this model never computed, so it BAILS rather than guessing; the
/// mispairing that IS decidable is a `link` whose frame is still on the stack when
/// control reaches a return, which the depth check reports on its own.
fn apply_unlk(st: &mut State, ops: &[CodeOperand]) -> Option<String> {
    let Some(CodeOperand::Reg(fp)) = ops.first() else {
        return Some("`unlk` with a non-register frame pointer".to_string());
    };
    match st.frames.pop() {
        Some((open, depth)) if open == *fp && depth <= st.stack.len() => {
            st.stack.truncate(depth);
            let saved = st.stack.pop().and_then(|s| s.reg);
            // The saved fp is a full `.l` slot (`link` pushed 4 bytes), so a match
            // restores both facets.
            st.entry[reg_idx(*fp)] = saved == Some(*fp);
            st.entry_word[reg_idx(*fp)] = saved == Some(*fp);
            st.delta[reg_idx(*fp)] = None;
            None
        }
        _ => Some(
            "`unlk` with no matching `link` on this path — sp becomes an unmodeled value"
                .to_string(),
        ),
    }
}

/// If instruction `(mnem, ops)` writes register `r` via a TRACKABLE linear
/// address-register operation, the static amount `r` advances by; `None` for any
/// other (untrackable) write of `r`. Trackable forms: an auto-increment `(r)+`
/// (+width) / auto-decrement `-(r)` (−width); a self-relative `lea d(r), r`
/// (+d); an immediate `adda #imm, r` (+imm) / `suba #imm, r` (−imm). A `lea`/
/// `adda`/`suba` whose base or source is not the reg itself (a computed value)
/// or any other mnemonic is untrackable.
fn linear_write_amount(mnem: &str, ops: &[CodeOperand], size: Option<Width>, r: Reg) -> Option<i64> {
    let sz: i64 = match size {
        Some(Width::L) => 4,
        Some(Width::W) => 2,
        Some(Width::B) => 1,
        _ => 0,
    };
    // Auto-inc / auto-dec of r.
    for op in ops {
        match op {
            CodeOperand::PostInc(x) if *x == r => return Some(sz),
            CodeOperand::PreDec(x) if *x == r => return Some(-sz),
            _ => {}
        }
    }
    let dest_is_r = matches!(ops.last(), Some(CodeOperand::Reg(x)) if *x == r);
    // `lea d(r), r` — self-relative displacement.
    if mnem == "lea" && dest_is_r {
        if let Some(CodeOperand::DispInd { disp, reg }) = ops.first() {
            if *reg == r {
                return Some(*disp as i64);
            }
        }
        return None; // lea from a different base/index → a fresh computed pointer
    }
    // `adda #imm, r` / `suba #imm, r`.
    if (mnem == "adda" || mnem == "suba") && dest_is_r {
        if let Some(CodeOperand::Imm(v)) = ops.first() {
            let d = *v as i64;
            return Some(if mnem == "adda" { d } else { -d });
        }
        return None; // adda rM, r → a computed advance
    }
    None
}

/// The slot tag for a `bytes`-wide push of register `r` at the current state:
/// `reg = Some(r)` iff `r` still holds its entry value.
fn tag(st: &State, r: Reg, bytes: u32, width: Option<Width>) -> Slot {
    // Which facet the push captures decides whether the slot holds `r`'s entry
    // value: a `.l` push captures the FULL register (valid iff `entry`), a
    // `.w`/`.b` push the low word/byte (valid iff `entry_word`). `save_width` rides
    // along so [`credit_restore`] can demand an EQUAL-width restore, which is what
    // confines a `.w`-tagged slot to the word facet: a `.l` pop over it credits
    // nothing, because a full round-trip requires a `.l`-saved slot.
    let holds = match width {
        Some(Width::W) | Some(Width::B) => st.entry_word[reg_idx(r)],
        _ => st.entry[reg_idx(r)],
    };
    Slot { reg: if holds { Some(r) } else { None }, bytes, save_width: width }
}

fn sp_hazard_reason(ops: &[CodeOperand]) -> String {
    if ops.iter().any(|o| matches!(o, CodeOperand::Reg(Reg::A7))) {
        "sp used as a value (computed sp / escape into address math)".to_string()
    } else {
        "sp-relative store could alias a saved slot".to_string()
    }
}

/// Join `other` into `acc` (both on entry to the same node). Entry bits meet by
/// AND (a register is entry-valued only if BOTH paths agree). Slots meet
/// pointwise (`Some(r)` iff both agree). Differing stack DEPTHS mean an ambiguous
/// sp at the merge → taint the merged path `bailed`.
///
/// `bailed` propagates by OR, so a bailed path DOES taint every path it merges
/// with — the safe direction (more silence), and the reason "path-local" means
/// only that a bail rides the CFG instead of aborting the walk: a bail on a
/// diverging path that never merges leaves returning paths untouched.
fn join(acc: &mut State, other: &State) {
    acc.bailed |= other.bailed;
    if acc.stack.len() != other.stack.len() {
        acc.bailed = true;
        return;
    }
    // Differing OPEN FRAMES mean the two paths disagree about which `link` an
    // `unlk` past this point would close — the frame chain's analogue of the
    // depth mismatch → taint the merge.
    if acc.frames != other.frames {
        acc.bailed = true;
        return;
    }
    for (a, b) in acc.stack.iter_mut().zip(other.stack.iter()) {
        // Differing slot BYTE widths at the same depth mean the two paths reached
        // this point with different sp offsets — an ambiguous stack geometry, the
        // depth-mismatch bail's finer-grained sibling → taint the merge.
        if a.bytes != b.bytes {
            acc.bailed = true;
            return;
        }
        // Registers meet pointwise: a slot holds a saved value only if both paths
        // agree it holds the SAME register's value AT THE SAME width. A width
        // disagreement (`.w` vs `.b` at equal `bytes`) means the two paths saved
        // different facets, so neither is creditable → drop the slot's tag.
        if a.reg != b.reg || a.save_width != b.save_width {
            a.reg = None;
        }
    }
    for i in 0..16 {
        acc.entry[i] = acc.entry[i] && other.entry[i];
        acc.entry_word[i] = acc.entry_word[i] && other.entry_word[i];
        // Delta meets by exact agreement: a register whose incoming deltas differ
        // (e.g. a loop's Δ=0 initial vs Δ=+4 back-edge) is no longer statically
        // known → untrackable. Two `None`s stay `None`; equal `Some`s survive.
        if acc.delta[i] != other.delta[i] {
            acc.delta[i] = None;
        }
    }
}

// ===========================================================================
// §4-stack / S2-D7(b) — [stack.unbalanced] and [stack.merge-mismatch]: sp is at
// its entry value on every path to `rts`, and merging paths agree on the delta.
// ===========================================================================

/// What a [`StackFinding`] reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackFindingKind {
    /// Control reaches a return with sp `depth` bytes BELOW its entry value —
    /// data pushed and never popped, so the caller's return address is not where
    /// the `rts` will look for it.
    ///
    /// Only that direction is representable: the slot map has no negative depth,
    /// so an sp RAISED past entry drains the tracked slots and hits the underflow
    /// bail instead. The opposite defect (popping more than was pushed) is
    /// therefore a blind spot, not a covered case — see the gap ledger.
    Unbalanced {
        /// Bytes of stack still held at the return.
        depth: i64,
    },
    /// Two paths meet with different sp deltas, so the code past the merge runs at
    /// an sp that depends on which branch was taken.
    MergeMismatch {
        /// The depth the already-recorded path arrives with.
        existing: i64,
        /// The depth the joining path arrives with.
        incoming: i64,
    },
}

impl StackFindingKind {
    /// The finding's CLASS, the dedup key's second half: one report per
    /// (site, class), so the fixpoint's repeated visits collapse while a site
    /// that is genuinely both an imbalance and a mismatch reports both.
    fn class(&self) -> u8 {
        match self {
            StackFindingKind::Unbalanced { .. } => 0,
            StackFindingKind::MergeMismatch { .. } => 1,
        }
    }
}

/// One stack-discipline finding at `span` — the return, or the merge point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StackFinding {
    pub kind: StackFindingKind,
    pub span: Span,
}

/// Every stack-discipline finding in a proc's evaluated CodeBuf: sp must be back
/// at its entry value on every path to a return, and paths that merge must agree
/// on where sp is.
///
/// **This is the SAME analysis as the entry-value proof**, reading a different
/// conclusion off the same states: the symbolic stack's byte depth IS sp's
/// displacement from entry, so the slot map that proves `preserves(rN)` already
/// carries the delta. It walks through [`run_stack_dataflow`], which owns the one
/// [`transfer`] and the one [`join`] — so it inherits, rather than restates, the
/// bailout set (a bare `a7` operand, a computed `adda #n,sp`, a displaced or
/// indexed sp access, a pop that underflows, an sp cleanup landing mid-slot, a
/// depth-ambiguous merge, an unmatched `unlk`).
///
/// **Silence where the model bails is the whole soundness story.** These are
/// ERROR-tier, so a false positive breaks the build on correct code; a bailed path
/// therefore reports NOTHING in either direction, and only the paths the model
/// tracked exactly can fire.
///
/// Scope: exits that END THIS BODY ([`Edge::Return`], and [`Edge::FallOff`] when
/// `charge_fall_off_end`). A transfer out of the proc ([`Edge::TailOut`] /
/// [`Edge::BranchOut`]) is deliberately not charged, for the reason the
/// entry-value proof gives — it may diverge (a noreturn error rail owes its caller
/// nothing), and nothing in the language marks that yet.
///
/// The policy is [`CallPolicy::ClobberAll`] because a call's STACK effect is
/// policy-independent (it nets zero — the return address it pushes is popped by
/// its own `rts`); only the entry-value bits, which this consumer never reads,
/// vary with the oracle.
pub fn check_stack_balance(items: &[CodeItem], charge_fall_off_end: bool) -> Vec<StackFinding> {
    let cfg = Cfg::build(items);
    let Some(entry_idx) = entry_instr_idx(items) else {
        return Vec::new(); // no instructions: nothing to balance
    };
    let mut obs = BalanceObserver { items, charge_fall_off_end, found: BTreeMap::new() };
    run_stack_dataflow(&cfg, entry_idx, items, &CallPolicy::ClobberAll, &mut obs);
    obs.found.into_values().collect()
}

/// The stack-balance checker's view of the shared walk. Findings are keyed by
/// `(span start, kind)` so the fixpoint's repeated visits to one site report it
/// once, in source order.
struct BalanceObserver<'a> {
    items: &'a [CodeItem],
    /// Charge control running off the END of the body as a return. False for a
    /// declared `falls_into`, whose end is a continuation, not a return.
    charge_fall_off_end: bool,
    /// Keyed `(source, span start, class)`. The SOURCE is load-bearing: a
    /// `with <ctx> { }` bracket splices the context module's instructions into
    /// this body carrying THEIR file's spans, so two findings in one proc can sit
    /// at the same byte offset in different files.
    found: BTreeMap<(u32, u32, u8), StackFinding>,
}

impl BalanceObserver<'_> {
    /// Is an exit of kind `exit` one this checker charges? A RETURN always is —
    /// an imbalanced `rts` returns to whatever word is on top of the stack.
    /// Control running off the END of the body is a return only when the proc did
    /// not declare a fallthrough: a declared `falls_into` continues into its
    /// successor, whose own check covers the shared frame, and charging it would
    /// reject the idiom with a message about an `rts` the body does not contain.
    /// A transfer OUT is the tail-callee's frame to answer for, not this proc's.
    fn charges(&self, exit: ExitKind) -> bool {
        match exit {
            ExitKind::Return => true,
            ExitKind::FallOff => self.charge_fall_off_end,
            ExitKind::Defer => false,
        }
    }

    /// Record `kind` at instruction `idx`, keeping the FIRST finding per
    /// (site, class).
    ///
    /// The walk reports during iteration rather than at the fixpoint, and that is
    /// deliberate: the state at a given visit is either ONE path's exactly (the
    /// first arrival) or a merge whose members AGREED on depth, so every recorded
    /// depth is a true fact about a real path. The fixpoint's converged state is
    /// strictly less informative — a later merge answers a disagreement with a
    /// bail, which would erase a genuine imbalance the model had already proven.
    fn record(&mut self, idx: usize, kind: StackFindingKind) {
        if let Some(span) = instr_span(self.items, idx) {
            self.found
                .entry((span.source.0, span.start, kind.class()))
                .or_insert(StackFinding { kind, span });
        }
    }
}

impl StackObserver for BalanceObserver<'_> {
    fn exit(&mut self, idx: usize, st: &State, exit: ExitKind) {
        if st.bailed || !self.charges(exit) {
            return;
        }
        let depth = st.depth_bytes();
        if depth != 0 {
            self.record(idx, StackFindingKind::Unbalanced { depth });
        }
    }

    fn merge(&mut self, succ: usize, existing: &State, incoming: &State) {
        if existing.bailed || incoming.bailed {
            return;
        }
        let (a, b) = (existing.depth_bytes(), incoming.depth_bytes());
        if a != b {
            self.record(succ, StackFindingKind::MergeMismatch { existing: a, incoming: b });
        }
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
    let Some(entry_idx) = entry_instr_idx(items) else {
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
            // Return / FallOff / TailOut / BranchOut: control leaves the body —
            // the save's fate is decided at its pop, not here.
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

    // `link`/`unlk` move sp without naming it. This walk does not model frames, so
    // it must BAIL rather than carry a depth the machine does not have — otherwise
    // a later restore pairs a slot to the wrong register and the worklist tells a
    // porter to delete a load-bearing save.
    if mnem == "link" || mnem == "unlk" {
        return true;
    }

    if sp_hazard(ops) && !is_sp_top_read(ops) {
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
    let Some(span) = instr_span(items, slot.push_idx) else { return };
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

#[cfg(test)]
mod frame_tests {
    //! `link` / `unlk` — the frame arm of the stack model.
    //!
    //! These are UNIT tests because the 68k backend has no encoder for either
    //! mnemonic, so no `.emp` source can reach the analysis through lowering. That
    //! is also the honest state of the corpus: it contains ZERO `link`/`unlk`
    //! sites. The rule is implemented for soundness — an unmodeled `link` would
    //! leave the slot map claiming a depth the machine does not have — and proven
    //! here, at the only level where it can be exercised.

    use super::*;
    use sigil_span::SourceId;

    /// One instruction item. Spans are distinct per index so the balance
    /// checker's per-site dedup key does not collapse two findings into one.
    fn instr(idx: u32, mnemonic: &str, ops: Vec<CodeOperand>) -> CodeItem {
        CodeItem::Instr {
            mnemonic: mnemonic.to_string(),
            size: Some(Width::L),
            ops,
            span: Span { source: SourceId(0), start: idx, end: idx + 1 },
            as_type: None,
            targets: Vec::new(),
            author: crate::value::ItemAuthor::User,
        }
    }

    /// `link fp, #disp` — `disp` is the frame DISPLACEMENT, negative to allocate,
    /// the ISA's own sign convention.
    fn link(idx: u32, fp: Reg, disp: i128) -> CodeItem {
        instr(idx, "link", vec![CodeOperand::Reg(fp), CodeOperand::Imm(disp)])
    }

    fn unlk(idx: u32, fp: Reg) -> CodeItem {
        instr(idx, "unlk", vec![CodeOperand::Reg(fp)])
    }

    fn rts(idx: u32) -> CodeItem {
        instr(idx, "rts", vec![])
    }

    /// The one thing the exit KIND does not settle: whether running off the end
    /// of a body is a return. That is a per-proc DECLARATION — `falls_into`
    /// continues into a successor that shares the frame — so the edge states the
    /// machine fact and `charge_fall_off_end` states the policy. A body that ends
    /// with an unmatched push and no `rts` is imbalanced under one and silent
    /// under the other, from the identical edge.
    #[test]
    fn a_fall_off_end_is_charged_only_when_the_proc_declares_no_fallthrough() {
        let push = instr(0, "move", vec![CodeOperand::Reg(Reg::D0), CodeOperand::PreDec(Reg::A7)]);
        let items = [push, instr(1, "nop", vec![])];
        assert_eq!(
            check_stack_balance(&items, false),
            Vec::new(),
            "a declared `falls_into` end continues into its successor's frame"
        );
        let found = check_stack_balance(&items, true);
        assert_eq!(found.len(), 1, "without the declaration the end IS a return");
        assert_eq!(found[0].kind, StackFindingKind::Unbalanced { depth: 4 });
    }

    /// A paired frame nets zero: the `unlk` drops the locals and pops the saved
    /// frame pointer.
    #[test]
    fn a_paired_frame_is_balanced() {
        let items = [link(0, Reg::A6, -8), unlk(1, Reg::A6), rts(2)];
        assert_eq!(check_stack_balance(&items, true), Vec::new(), "a closed frame nets zero");
    }

    /// The pairing rule's PROVABLE direction: a `link` whose frame is still on the
    /// stack at a return leaves sp 12 bytes low (4 saved fp + 8 locals).
    #[test]
    fn a_link_with_no_unlk_is_unbalanced() {
        let items = [link(0, Reg::A6, -8), rts(1)];
        let found = check_stack_balance(&items, true);
        assert_eq!(found.len(), 1, "expected one finding, got {found:?}");
        assert_eq!(found[0].kind, StackFindingKind::Unbalanced { depth: 12 });
    }

    /// An `unlk` with no open frame sets sp from a register this model never
    /// computed. That is a BAILOUT, not a finding: reporting a delta here would be
    /// a guess, and the tier does not allow guesses.
    #[test]
    fn an_unlk_with_no_link_bails_rather_than_guessing() {
        let items = [unlk(0, Reg::A6), rts(1)];
        assert_eq!(
            check_stack_balance(&items, true),
            Vec::new(),
            "an unmatched unlk leaves sp unmodeled — reporting a delta would be a guess"
        );
    }

    /// Likewise an `unlk` naming a register no open `link` opened — the frame
    /// chain disagrees, so sp is unmodeled from there.
    #[test]
    fn an_unlk_of_the_wrong_register_bails() {
        let items = [link(0, Reg::A6, -8), unlk(1, Reg::A5), rts(2)];
        assert_eq!(
            check_stack_balance(&items, true),
            Vec::new(),
            "a disagreeing frame chain leaves sp unmodeled"
        );
    }

    /// The frame arm serves the ENTRY-VALUE proof too, which is the point of
    /// putting it in the shared transfer: `unlk` restores the frame pointer from
    /// its own saved slot, so `preserves(a6)` verifies.
    #[test]
    fn a_paired_frame_preserves_its_frame_pointer() {
        let items = [link(0, Reg::A6, -8), unlk(1, Reg::A6), rts(2)];
        let got = verify_preserved(&items, &[Reg::A6], CallPolicy::ClobberAll, None, &BTreeSet::new());
        assert_eq!(got[&Reg::A6], PreserveStatus::Verified);
    }

    /// And refutes the claim when the frame is never closed: `link` writes the
    /// frame pointer, and nothing restores it.
    #[test]
    fn an_unclosed_frame_does_not_preserve_its_frame_pointer() {
        let items = [link(0, Reg::A6, -8), rts(1)];
        let got = verify_preserved(&items, &[Reg::A6], CallPolicy::ClobberAll, None, &BTreeSet::new());
        assert_eq!(got[&Reg::A6], PreserveStatus::NotPreserved);
    }

    /// A closed zero-size frame (`link aN, #0` — a frame pointer with no locals)
    /// is balanced.
    #[test]
    fn a_closed_zero_size_frame_is_balanced() {
        let items = [link(0, Reg::A6, 0), unlk(1, Reg::A6), rts(2)];
        assert_eq!(check_stack_balance(&items, true), Vec::new(), "a closed frame nets zero");
    }

    /// An unclosed zero-size frame still leaves its saved frame pointer behind.
    #[test]
    fn an_unclosed_zero_size_frame_still_carries_its_saved_pointer() {
        let found = check_stack_balance(&[link(0, Reg::A6, 0), rts(1)], true);
        assert_eq!(found.len(), 1, "expected one finding, got {found:?}");
        assert_eq!(found[0].kind, StackFindingKind::Unbalanced { depth: 4 });
    }

    /// A POSITIVE displacement would raise sp into the caller's frame rather than
    /// allocate — not the idiom, and not modeled, so it bails.
    #[test]
    fn a_positive_link_displacement_bails() {
        let items = [link(0, Reg::A6, 8), rts(1)];
        assert_eq!(
            check_stack_balance(&items, true),
            Vec::new(),
            "a positive displacement is not the allocate idiom, so it is not modeled"
        );
    }

    /// A frame size the slot's byte width cannot hold is untracked, so it bails
    /// rather than silently truncating to a depth the machine does not have.
    /// `i128::MIN` rides the same arm — its negation does not exist.
    #[test]
    fn an_unrepresentable_frame_size_bails() {
        for size in [-(u32::MAX as i128) - 1, i128::MIN] {
            assert_eq!(
                check_stack_balance(&[link(0, Reg::A6, size), rts(1)], true),
                Vec::new(),
                "frame size {size} must bail"
            );
        }
    }

    /// A `jmp` authored by the `assert`/`raise` desugar is a DIVERGENT rail — it
    /// never returns to the caller, so its tail carries NO preserves obligation
    /// even when its target preserves nothing (the `@noreturn`-composition sibling
    /// that needs no declared symbol). Here the sole exit is such a rail, so a0
    /// (untouched) verifies; the `User`-authored control below is charged.
    #[test]
    fn an_assert_desugar_tail_is_obligation_free() {
        let jmp = |author| CodeItem::Instr {
            mnemonic: "jmp".to_string(),
            size: None,
            ops: vec![CodeOperand::Sym("Handler".to_string())],
            span: Span { source: SourceId(0), start: 0, end: 1 },
            as_type: None,
            targets: Vec::new(),
            author,
        };
        // Handler preserves nothing (absent from the oracle map).
        let empty: BTreeMap<String, RegEffect> = BTreeMap::new();
        let noreturn = BTreeSet::new();
        let authored = verify_preserved(
            &[jmp(ItemAuthor::AssertDesugar)],
            &[Reg::A0],
            CallPolicy::Oracle(&empty),
            None,
            &noreturn,
        );
        assert_eq!(
            authored[&Reg::A0],
            PreserveStatus::Verified,
            "an AssertDesugar divergent tail carries no obligation"
        );
        // CONTROL: the same tail authored by the USER is a real tail call — a0 is
        // charged against Handler's (empty) contract → NotPreserved.
        let user = verify_preserved(
            &[jmp(ItemAuthor::User)],
            &[Reg::A0],
            CallPolicy::Oracle(&empty),
            None,
            &noreturn,
        );
        assert_eq!(
            user[&Reg::A0],
            PreserveStatus::NotPreserved,
            "a User-authored tail into a non-preserving target is charged"
        );
    }
}
