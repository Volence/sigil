//! Which 68000 interrupt levels are possible at a point, and which saved levels
//! sit on the stack (step A piece 2a). This module is the STATE a path-following
//! check carries, its combine rule ([`Lattice::meet`]) and the per-instruction
//! [`step`], and [`mask_findings`], which walks one routine with them and
//! reports every way out that is not provably at the caller's level.
//!
//! Order (for [`crate::context::forward_in_states`]): `meet` goes DOWN. A larger
//! level set is lower than a smaller one, `Level::Unknown` is the bottom of the
//! level part, and `Stack::Lost` is the bottom of the stack part. Every value has
//! one representation: a set is a bitmask, and `Unknown` is NOT the same as "all
//! eight levels plus ENTRY" (they never meet into each other, because `Unknown`
//! absorbs and a union never produces `Unknown`).
//!
//! Termination: level sets are finite; a stack's depth can grow without bound
//! along one path, but a `meet` of two different depths is `Lost`, so a loop
//! that pushes settles at `Lost` after one extra pass.
//!
//! Entries: the walk starts at the routine's entries and at every place an
//! entry it cannot see could land (see [`mask_findings`]). Out of the model, on
//! purpose: an interior address formed OUTSIDE the routine without an `export`
//! label (another routine computing `P` plus an offset).

use crate::context::{edges_for, forward_in_states, Lattice};
use crate::flag_check::{Cfg, Edge};
use crate::preserves::{exit_diverges, sp_effect, SpEffect};
use crate::value::{CodeItem, CodeOperand, ItemAuthor, Reg};
use sigil_ir::backend::Cpu;
use sigil_span::Span;
use std::collections::{BTreeMap, BTreeSet};

/// A set of possible interrupt levels. Bit n (0..=7): level n. Bit 8 (`ENTRY`):
/// whatever the level was when the routine was entered.
///
/// Never empty: the field is private to this module and no constructor makes
/// an empty set (`only`, `ENTRY`, `union` of non-empty sets, and the mapping
/// in [`logic_on_set`], which maps each member to at least one member).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LevelSet(u16);

const ENTRY_BIT: u16 = 1 << 8;

impl LevelSet {
    pub(crate) const ENTRY: LevelSet = LevelSet(ENTRY_BIT);

    /// The set holding just `level & 7`.
    pub(crate) fn only(level: u8) -> LevelSet {
        LevelSet(1 << (level & 7))
    }

    pub(crate) fn union(self, o: LevelSet) -> LevelSet {
        LevelSet(self.0 | o.0)
    }

    /// True when the only possibility is the level the routine was entered at.
    pub(crate) fn is_entry_only(self) -> bool {
        self.0 == ENTRY_BIT
    }

    /// The fixed levels in the set, ascending, `ENTRY` left out.
    pub(crate) fn fixed_levels(self) -> Vec<u8> {
        (0u8..8).filter(|n| self.0 & (1 << n) != 0).collect()
    }

    pub(crate) fn has_entry(self) -> bool {
        self.0 & ENTRY_BIT != 0
    }

    /// Raw bitmask of fixed levels 0..=7 (`ENTRY_BIT` stripped). May be 0 when
    /// `self` is `ENTRY` only, so this returns `u16` rather than a `LevelSet`.
    pub(crate) fn fixed_bits(self) -> u16 {
        self.0 & !ENTRY_BIT
    }

    /// True when every member of `self` is also in `allowed`.
    pub(crate) fn subset_of(self, allowed: LevelSet) -> bool {
        (self.0 & !allowed.0) == 0
    }

    pub(crate) fn contains_fixed(self, level: u8) -> bool {
        self.0 & (1 << (level & 7)) != 0
    }

    fn is_non_empty(self) -> bool {
        self.0 != 0
    }
}

/// What the level could be at one point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level {
    Known(LevelSet),
    Unknown,
}

impl Level {
    pub(crate) fn meet(self, o: Level) -> Level {
        match (self, o) {
            (Level::Known(a), Level::Known(b)) => Level::Known(a.union(b)),
            _ => Level::Unknown,
        }
    }

    pub(crate) fn subset_of(self, allowed: LevelSet) -> bool {
        match self {
            Level::Known(s) => s.subset_of(allowed),
            Level::Unknown => false,
        }
    }

    fn is_well_formed(self) -> bool {
        match self {
            Level::Known(s) => s.is_non_empty(),
            Level::Unknown => true,
        }
    }
}

/// Compose a caller's current interrupt level with a callee's level transformer.
///
/// A callee summary's `ENTRY_BIT` stands for "whatever level the caller passed
/// in"; fixed bits 0..=7 stand for concrete levels the callee establishes.
/// Monotone in both arguments, associative, with `Level::Known(LevelSet::ENTRY)`
/// as the two-sided identity.
pub(crate) fn compose_level(caller: Level, callee: Level) -> Level {
    match callee {
        Level::Unknown => Level::Unknown,
        Level::Known(cs) => {
            let fixed = cs.fixed_bits();
            if cs.has_entry() {
                match caller {
                    Level::Unknown => Level::Unknown,
                    Level::Known(cl) => Level::Known(LevelSet(cl.0 | fixed)),
                }
            } else {
                // `cs` is non-empty and lacks ENTRY, so `fixed != 0`.
                Level::Known(LevelSet(fixed))
            }
        }
    }
}

pub(crate) fn meet_opt_level(a: Option<Level>, b: Option<Level>) -> Option<Level> {
    match (a, b) {
        (None, x) | (x, None) => x,
        (Some(x), Some(y)) => Some(x.meet(y)),
    }
}

/// Optional reachability wrapper over [`SrState`] for [`forward_in_states`]:
/// `Reach(None)` is the top identity of `meet` (an unreachable point, e.g.
/// after a divergent call), letting divergent calls cleanly drop their
/// fall-through edge without losing fixpoint monotonicity.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Reach(Option<SrState>);

impl Lattice for Reach {
    fn meet(&self, other: &Self) -> Self {
        match (&self.0, &other.0) {
            (None, x) | (x, None) => Reach(x.clone()),
            (Some(a), Some(b)) => Reach(Some(a.meet(b))),
        }
    }
}

/// Summary of what entering a routine (or exported label / local helper) does
/// to the 68000 interrupt level and stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProcSummary {
    /// Meet of outgoing interrupt levels across all non-diverging exits.
    /// `None` means no path returns (diverges / `@noreturn` / infinite loop).
    pub(crate) exit_level: Option<Level>,
    /// Meet of interrupt levels observed at any reachable non-diverging point
    /// inside the callee (including transitively across its own calls). Used by
    /// Rule 3 (`with` brackets) to catch a callee that lowers the level mid-call
    /// even if it restores before returning.
    pub(crate) body_levels: Option<Level>,
    /// True when any non-diverging exit loses stack tracking (`Stack::Lost`) or
    /// exits with nonzero stack delta, meaning the callee may have overwritten
    /// or popped past its caller's stack frame.
    pub(crate) touches_caller_stack: bool,
}

impl ProcSummary {
    pub(crate) const DIVERGES: ProcSummary =
        ProcSummary { exit_level: None, body_levels: None, touches_caller_stack: false };

    pub(crate) const IDENTITY: ProcSummary = ProcSummary {
        exit_level: Some(Level::Known(LevelSet::ENTRY)),
        body_levels: Some(Level::Known(LevelSet::ENTRY)),
        touches_caller_stack: false,
    };

    pub(crate) const UNKNOWN: ProcSummary = ProcSummary {
        exit_level: Some(Level::Unknown),
        body_levels: Some(Level::Unknown),
        touches_caller_stack: false,
    };

    pub(crate) fn meet(self, other: ProcSummary) -> ProcSummary {
        ProcSummary {
            exit_level: meet_opt_level(self.exit_level, other.exit_level),
            body_levels: meet_opt_level(self.body_levels, other.body_levels),
            touches_caller_stack: self.touches_caller_stack || other.touches_caller_stack,
        }
    }
}

/// Apply a callee summary at a call site. Returns `None` when the callee
/// diverges (dropping the fall-through edge). Only poisons the caller's stack
/// when the caller currently holds at least one saved SR on the stack: without
/// a live SR save, a callee's internal stack loss cannot corrupt a save that
/// has not yet been pushed.
pub(crate) fn apply_summary(st: &SrState, summary: ProcSummary) -> Option<SrState> {
    let callee_lvl = summary.exit_level?;
    let level = compose_level(st.level, callee_lvl);
    let stack = match &st.stack {
        Stack::Known { saves, .. } if summary.touches_caller_stack && !saves.is_empty() => Stack::Lost,
        other => other.clone(),
    };
    Some(SrState { level, stack })
}

/// One saved level on the stack. `below` is the depth in bytes before the push.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Save {
    pub(crate) below: u32,
    pub(crate) level: Level,
}

/// The stack, as far as saved levels need it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Stack {
    /// `depth`: bytes pushed since entry. `saves`: bottom first, strictly rising `below`.
    Known { depth: u32, saves: Vec<Save> },
    Lost,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SrState {
    pub(crate) level: Level,
    pub(crate) stack: Stack,
}

impl SrState {
    /// The state on entry to a routine: the entry level, nothing pushed.
    pub(crate) fn at_entry() -> SrState {
        SrState { level: Level::Known(LevelSet::ENTRY), stack: Stack::Known { depth: 0, saves: Vec::new() } }
    }

    fn is_well_formed(&self) -> bool {
        self.level.is_well_formed()
            && match &self.stack {
                Stack::Known { saves, .. } => saves.iter().all(|s| s.level.is_well_formed()),
                Stack::Lost => true,
            }
    }
}

impl Lattice for SrState {
    fn meet(&self, other: &Self) -> Self {
        debug_assert!(self.is_well_formed() && other.is_well_formed(), "empty LevelSet in meet");
        let level = self.level.meet(other.level);
        let stack = match (&self.stack, &other.stack) {
            (Stack::Known { depth: da, saves: sa }, Stack::Known { depth: db, saves: sb })
                if da == db
                    && sa.len() == sb.len()
                    && sa.iter().zip(sb).all(|(x, y)| x.below == y.below) =>
            {
                Stack::Known {
                    depth: *da,
                    saves: sa
                        .iter()
                        .zip(sb)
                        .map(|(x, y)| Save { below: x.below, level: x.level.meet(y.level) })
                        .collect(),
                }
            }
            _ => Stack::Lost,
        };
        SrState { level, stack }
    }
}

/// What a call does to the level when no summary table is supplied (unit-test
/// helper for [`step`]). Full call propagation uses [`step_with_summary`].
fn call_effect(level: Level) -> Level {
    level
}

fn is_call(mnem: &str) -> bool {
    matches!(mnem, "jsr" | "bsr" | "jbsr")
}

/// One 68000 procedure fed into [`SummaryTable::compute`] and inter-procedural
/// interrupt-level checks (step A piece 2b).
pub(crate) struct SrProcInput<'a> {
    pub(crate) name: &'a str,
    pub(crate) items: &'a [CodeItem],
    pub(crate) falls_into: Option<&'a str>,
    pub(crate) is_resumable: bool,
    pub(crate) is_continuation: bool,
    pub(crate) span: Span,
}

/// External facts shared across every procedure in an inter-procedural walk.
pub(crate) struct SrExternalEnv<'a> {
    pub(crate) noreturn: &'a BTreeSet<String>,
    /// Pre-built summaries for `extern proc` declarations.
    pub(crate) extern_summaries: &'a BTreeMap<String, ProcSummary>,
    /// Contract-type names (`type T = proc ...`) that explicitly declare
    /// `clobbers(sr)` or `clobbers(sr.mask)`.
    pub(crate) sr_clobber_types: &'a BTreeSet<String>,
    /// True in whole-program mode (`corpus_contracts`); false in single-file
    /// mode (where cross-file callees are unseen and deferred to whole-program).
    pub(crate) whole_program: bool,
}

/// What a call or non-return transfer instruction targets.
#[derive(Clone, Debug, PartialEq, Eq)]
enum TransferTarget {
    /// Divergent rail (`AssertDesugar`) or `@noreturn` target.
    Divergent,
    /// A local code label inside the same proc (`bsr .h`, `jsr .h(pc)`, `jmp .h(pc)`).
    Local(usize),
    /// A body-closing local code label at the end of the proc (delegates to `falls_into`).
    ClosingLocal,
    /// A named external symbol (proc head or `Owner.label` export).
    External(String),
    /// A non-local symbol with a nonzero offset (`Foo+6`), whose landing point
    /// is outside the entry-0 model.
    OffsetUnknown,
    /// A `@resumable` proc's untyped `jmp (aN)` (`aN != a7`) continuation exit.
    ResumableReturn,
    /// An indirect call/tail carrying an explicit `as Type` contract bound.
    TypedIndirect(String),
    /// An untyped indirect/computed call or jump (left to piece 4).
    UntypedIndirect,
}

fn classify_symbol_target(
    items: &[CodeItem],
    cfg: &Cfg,
    name: &str,
    noreturn: &BTreeSet<String>,
) -> TransferTarget {
    if let Some(local_idx) = code_label_index(items, cfg, name) {
        return TransferTarget::Local(local_idx);
    }
    if is_closing_code_label(items, name) {
        return TransferTarget::ClosingLocal;
    }
    if noreturn.contains(name) {
        return TransferTarget::Divergent;
    }
    TransferTarget::External(name.to_string())
}

/// Classify a call or transfer instruction at `idx`.
fn classify_transfer(
    items: &[CodeItem],
    cfg: &Cfg,
    idx: usize,
    is_resumable: bool,
    noreturn: &BTreeSet<String>,
) -> TransferTarget {
    use CodeOperand as O;
    let CodeItem::Instr { mnemonic, ops, as_type, author, .. } = &items[idx] else {
        return TransferTarget::UntypedIndirect;
    };
    if matches!(author, ItemAuthor::AssertDesugar) {
        return TransferTarget::Divergent;
    }
    if is_resumable
        && mnemonic == "jmp"
        && as_type.is_none()
        && matches!(ops.as_slice(), [O::Ind(r)] if *r != Reg::A7)
    {
        return TransferTarget::ResumableReturn;
    }
    // Scan from the last operand so 2-operand `dbcc dN, Target` finds Target.
    if let Some(op) = ops.last() {
        match op {
            O::Sym(name) => return classify_symbol_target(items, cfg, name, noreturn),
            O::AbsSym { target, .. } => return classify_symbol_target(items, cfg, target, noreturn),
            O::SymOff { sym, off } => {
                if *off == 0 {
                    return classify_symbol_target(items, cfg, sym, noreturn);
                }
                return TransferTarget::OffsetUnknown;
            }
            O::PcRel { target, addend } => {
                if *addend == 0 {
                    return classify_symbol_target(items, cfg, target, noreturn);
                }
                return TransferTarget::OffsetUnknown;
            }
            _ => {}
        }
    }
    if let Some(ty) = as_type {
        return TransferTarget::TypedIndirect(ty.clone());
    }
    TransferTarget::UntypedIndirect
}

/// Shared read-only context for walking a single 68000 procedure.
#[derive(Clone, Copy)]
struct WalkCtx<'a> {
    proc_idx: usize,
    items: &'a [CodeItem],
    cfg: &'a Cfg<'a>,
    is_resumable: bool,
    table: &'a SummaryTable,
    env: &'a SrExternalEnv<'a>,
}

/// Converged interrupt-level summaries for every entry point (proc heads,
/// exported code labels, and local helper labels) across one or more 68000
/// procedures.
#[derive(Clone, Debug)]
pub(crate) struct SummaryTable {
    /// `(proc_idx, entry_instr_idx)` -> converged summary.
    pub(crate) entries: BTreeMap<(usize, usize), ProcSummary>,
    /// Empty-body procs or body-closing labels (delegating to `falls_into`) keyed by `proc_idx`.
    pub(crate) closing_procs: BTreeMap<usize, ProcSummary>,
    /// Global symbol name (`ProcName` or `Owner.label`) -> `(proc_idx, Option<entry_instr_idx>)`.
    pub(crate) by_name: BTreeMap<String, (usize, Option<usize>)>,
    /// Per-proc local `address_taken` code label indices (for `c02` indirect stack check).
    taken_per_proc: Vec<BTreeSet<usize>>,
}

impl SummaryTable {
    /// Compute the least fixpoint of [`ProcSummary`] across all `procs`.
    /// Each entry is walked from ONLY that entry (no `hidden_entries` or
    /// `first_unreached` during fixpoint iteration).
    pub(crate) fn compute(procs: &[SrProcInput<'_>], env: &SrExternalEnv<'_>) -> SummaryTable {
        let cfgs: Vec<Cfg> = procs.iter().map(|p| Cfg::build(p.items)).collect();
        let taken_per_proc: Vec<BTreeSet<usize>> =
            procs.iter().zip(&cfgs).map(|(p, cfg)| address_taken(p.items, cfg)).collect();

        let mut by_name: BTreeMap<String, (usize, Option<usize>)> = BTreeMap::new();
        let mut entries: BTreeMap<(usize, usize), ProcSummary> = BTreeMap::new();
        let mut closing_procs: BTreeMap<usize, ProcSummary> = BTreeMap::new();

        for (pi, (proc, cfg)) in procs.iter().zip(&cfgs).enumerate() {
            let first_instr = proc.items.iter().position(|it| matches!(it, CodeItem::Instr { .. }));
            by_name.insert(proc.name.to_string(), (pi, first_instr));
            match first_instr {
                Some(idx) => {
                    entries.insert((pi, idx), ProcSummary::DIVERGES);
                }
                None => {
                    closing_procs.insert(pi, ProcSummary::DIVERGES);
                }
            }
            for item in proc.items {
                match item {
                    CodeItem::Label { name, export, .. } => {
                        if let Some(idx) = code_label_index(proc.items, cfg, name) {
                            if *export {
                                by_name.insert(name.clone(), (pi, Some(idx)));
                                entries.insert((pi, idx), ProcSummary::DIVERGES);
                            }
                        } else if is_closing_code_label(proc.items, name) {
                            closing_procs.insert(pi, ProcSummary::DIVERGES);
                            if *export {
                                by_name.insert(name.clone(), (pi, None));
                            }
                        }
                    }
                    CodeItem::Instr { mnemonic, ops, targets, .. } => {
                        for t in targets {
                            if let Some(idx) = code_label_index(proc.items, cfg, t) {
                                entries.insert((pi, idx), ProcSummary::DIVERGES);
                            }
                        }
                        if is_transfer(mnemonic) {
                            for op in ops {
                                for name in operand_symbols(op) {
                                    if let Some(idx) = code_label_index(proc.items, cfg, &name) {
                                        entries.insert((pi, idx), ProcSummary::DIVERGES);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            for &idx in &taken_per_proc[pi] {
                entries.insert((pi, idx), ProcSummary::DIVERGES);
            }
        }

        let mut table = SummaryTable { entries, closing_procs, by_name, taken_per_proc };

        // Monotone fixpoint: height per entry is <= 11 (exit) + 11 (body) + 1 (stack),
        // so iteration terminates quickly even with arbitrary mutual recursion.
        let entry_keys: Vec<(usize, usize)> = table.entries.keys().copied().collect();
        let closing_keys: Vec<usize> = table.closing_procs.keys().copied().collect();
        loop {
            let mut changed = false;
            for &pi in &closing_keys {
                let next = match procs[pi].falls_into {
                    Some(target) => table.resolve_external(target, env),
                    None => ProcSummary::IDENTITY,
                };
                let slot = table.closing_procs.get_mut(&pi).expect("closing proc key");
                let merged = slot.meet(next);
                if merged != *slot {
                    *slot = merged;
                    changed = true;
                }
            }
            for &(pi, entry_idx) in &entry_keys {
                let next = summarize_entry(pi, entry_idx, &procs[pi], &cfgs[pi], &table, env);
                let slot = table.entries.get_mut(&(pi, entry_idx)).expect("entry key");
                let merged = slot.meet(next);
                if merged != *slot {
                    *slot = merged;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        table
    }

    pub(crate) fn resolve_external(&self, name: &str, env: &SrExternalEnv<'_>) -> ProcSummary {
        if env.noreturn.contains(name) {
            return ProcSummary::DIVERGES;
        }
        if let Some(&(pi, entry)) = self.by_name.get(name) {
            return match entry {
                Some(idx) => self.entries.get(&(pi, idx)).copied().unwrap_or(ProcSummary::UNKNOWN),
                None => self.closing_procs.get(&pi).copied().unwrap_or(ProcSummary::UNKNOWN),
            };
        }
        if let Some(&ext) = env.extern_summaries.get(name) {
            return ext;
        }
        if env.whole_program {
            ProcSummary::UNKNOWN
        } else {
            ProcSummary::IDENTITY
        }
    }

    /// Summary of a call instruction at `idx`.
    fn call_summary(&self, ctx: WalkCtx<'_>, idx: usize) -> ProcSummary {
        match classify_transfer(ctx.items, ctx.cfg, idx, ctx.is_resumable, ctx.env.noreturn) {
            TransferTarget::Divergent => ProcSummary::DIVERGES,
            TransferTarget::Local(local_idx) => {
                self.entries.get(&(ctx.proc_idx, local_idx)).copied().unwrap_or(ProcSummary::UNKNOWN)
            }
            TransferTarget::ClosingLocal => {
                self.closing_procs.get(&ctx.proc_idx).copied().unwrap_or(ProcSummary::IDENTITY)
            }
            TransferTarget::External(name) => self.resolve_external(&name, ctx.env),
            TransferTarget::OffsetUnknown => ProcSummary::UNKNOWN,
            TransferTarget::TypedIndirect(ty) => {
                if ctx.env.sr_clobber_types.contains(&ty) {
                    ProcSummary::UNKNOWN
                } else {
                    ProcSummary::IDENTITY
                }
            }
            TransferTarget::UntypedIndirect => {
                // Close `c02_helper_drops_return_ptr`: if this proc takes the
                // address of a local helper whose summary touches its caller's
                // stack, an untyped indirect call may invoke it. Keep `exit_level`
                // and `body_levels` as IDENTITY (piece 4 owns blind-call levels).
                let mut s = ProcSummary::IDENTITY;
                if let Some(taken) = self.taken_per_proc.get(ctx.proc_idx) {
                    if taken.iter().any(|&t| {
                        self.entries.get(&(ctx.proc_idx, t)).is_some_and(|e| e.touches_caller_stack)
                    }) {
                        s.touches_caller_stack = true;
                    }
                }
                s
            }
            TransferTarget::ResumableReturn => ProcSummary::IDENTITY,
        }
    }

    /// Summary of a leaving `TailOut` / `BranchOut` edge at `idx`.
    /// Returns `None` when the transfer diverges (skipped as an exit).
    fn exit_transfer_summary(&self, ctx: WalkCtx<'_>, idx: usize) -> Option<ProcSummary> {
        if let CodeItem::Instr { targets, .. } = &ctx.items[idx] {
            if !targets.is_empty() {
                let mut acc = ProcSummary::DIVERGES;
                for t in targets {
                    let s = if let Some(local_idx) = code_label_index(ctx.items, ctx.cfg, t) {
                        self.entries
                            .get(&(ctx.proc_idx, local_idx))
                            .copied()
                            .unwrap_or(ProcSummary::UNKNOWN)
                    } else if is_closing_code_label(ctx.items, t) {
                        self.closing_procs
                            .get(&ctx.proc_idx)
                            .copied()
                            .unwrap_or(ProcSummary::IDENTITY)
                    } else {
                        self.resolve_external(t, ctx.env)
                    };
                    acc = acc.meet(s);
                }
                return Some(acc);
            }
        }
        match classify_transfer(ctx.items, ctx.cfg, idx, ctx.is_resumable, ctx.env.noreturn) {
            TransferTarget::Divergent => None,
            TransferTarget::ResumableReturn => Some(ProcSummary::IDENTITY),
            TransferTarget::Local(local_idx) => Some(
                self.entries
                    .get(&(ctx.proc_idx, local_idx))
                    .copied()
                    .unwrap_or(ProcSummary::UNKNOWN),
            ),
            TransferTarget::ClosingLocal => Some(
                self.closing_procs.get(&ctx.proc_idx).copied().unwrap_or(ProcSummary::IDENTITY),
            ),
            TransferTarget::External(name) => Some(self.resolve_external(&name, ctx.env)),
            TransferTarget::OffsetUnknown => Some(ProcSummary::UNKNOWN),
            TransferTarget::TypedIndirect(ty) => Some(if ctx.env.sr_clobber_types.contains(&ty) {
                ProcSummary::UNKNOWN
            } else {
                ProcSummary::IDENTITY
            }),
            TransferTarget::UntypedIndirect => Some(ProcSummary::IDENTITY),
        }
    }
}

/// Step one instruction with an optional callee summary table.
fn step_reach(
    ctx: WalkCtx<'_>,
    reach: &Reach,
    idx: usize,
    mnem: &str,
    ops: &[CodeOperand],
) -> Reach {
    let Some(st) = &reach.0 else { return Reach(None) };
    if is_call(mnem) {
        let callee = ctx.table.call_summary(ctx, idx);
        return Reach(apply_summary(st, callee));
    }
    Reach(Some(step(ctx.items, st, idx, mnem, ops)))
}

/// Compute one iteration of [`ProcSummary`] starting ONLY from `entry_idx`.
fn summarize_entry(
    proc_idx: usize,
    entry_idx: usize,
    proc: &SrProcInput<'_>,
    cfg: &Cfg,
    table: &SummaryTable,
    env: &SrExternalEnv<'_>,
) -> ProcSummary {
    let ctx = WalkCtx { proc_idx, items: proc.items, cfg, is_resumable: proc.is_resumable, table, env };
    let seeds = [(entry_idx, Reach(Some(SrState::at_entry())))];
    let states = forward_in_states(
        cfg,
        Cpu::M68000,
        &seeds,
        &|r: &Reach, idx, mnem, ops| step_reach(ctx, r, idx, mnem, ops),
        &|_| true,
    );

    let mut summary = ProcSummary::DIVERGES;
    for (&idx, reach_in) in &states {
        let Some(st_in) = &reach_in.0 else { continue };
        let CodeItem::Instr { mnemonic, ops, .. } = &proc.items[idx] else { continue };

        // Record body levels observed inside any callee, propagate transitive
        // `touches_caller_stack` (review C2), and model early return past the
        // caller when a stack-corrupting callee returns (review C3).
        if is_call(mnemonic) {
            let callee = table.call_summary(ctx, idx);
            if let Some(b) = callee.body_levels {
                summary.body_levels =
                    meet_opt_level(summary.body_levels, Some(compose_level(st_in.level, b)));
            }
            if callee.touches_caller_stack {
                if let Some(e) = callee.exit_level {
                    summary.touches_caller_stack = true;
                    summary.exit_level =
                        meet_opt_level(summary.exit_level, Some(compose_level(st_in.level, e)));
                }
            }
        }
        let reach_out = step_reach(ctx, reach_in, idx, mnemonic, ops);
        let Some(out) = reach_out.0 else { continue };
        summary.body_levels = meet_opt_level(summary.body_levels, Some(out.level));

        let stack_corrupted = |s: &Stack| !matches!(s, Stack::Known { depth: 0, .. });

        for edge in edges_for(cfg, Cpu::M68000, idx) {
            match edge {
                Edge::Follow(_) => {}
                Edge::Return if mnemonic == "rte" => {
                    if proc.is_continuation {
                        // Decision 1: `@continuation` manufactured `rte` trusts
                        // the restored level (does not poison caller stack).
                        summary.exit_level = meet_opt_level(summary.exit_level, Some(out.level));
                    } else {
                        summary.exit_level = meet_opt_level(summary.exit_level, Some(Level::Unknown));
                        if stack_corrupted(&out.stack) {
                            summary.touches_caller_stack = true;
                        }
                    }
                }
                Edge::Return => {
                    summary.exit_level = meet_opt_level(summary.exit_level, Some(out.level));
                    if stack_corrupted(&out.stack) {
                        summary.touches_caller_stack = true;
                    }
                }
                Edge::TailOut | Edge::BranchOut => {
                    if let Some(target_sum) = table.exit_transfer_summary(ctx, idx) {
                        if let Some(b) = target_sum.body_levels {
                            summary.body_levels = meet_opt_level(
                                summary.body_levels,
                                Some(compose_level(out.level, b)),
                            );
                        }
                        if let Some(e) = target_sum.exit_level {
                            summary.exit_level = meet_opt_level(
                                summary.exit_level,
                                Some(compose_level(out.level, e)),
                            );
                            if stack_corrupted(&out.stack) || target_sum.touches_caller_stack {
                                summary.touches_caller_stack = true;
                            }
                        }
                    }
                }
                Edge::FallOff => {
                    if let Some(next_name) = proc.falls_into {
                        let target_sum = table.resolve_external(next_name, env);
                        if let Some(b) = target_sum.body_levels {
                            summary.body_levels = meet_opt_level(
                                summary.body_levels,
                                Some(compose_level(out.level, b)),
                            );
                        }
                        if let Some(e) = target_sum.exit_level {
                            summary.exit_level = meet_opt_level(
                                summary.exit_level,
                                Some(compose_level(out.level, e)),
                            );
                            if stack_corrupted(&out.stack) || target_sum.touches_caller_stack {
                                summary.touches_caller_stack = true;
                            }
                        }
                    } else {
                        summary.exit_level = meet_opt_level(summary.exit_level, Some(out.level));
                        if stack_corrupted(&out.stack) {
                            summary.touches_caller_stack = true;
                        }
                    }
                }
            }
        }
    }
    summary
}

/// Does this instruction set the level? `stop #imm` loads `sr` from its
/// immediate; every other write to `sr` has `sr` as its last operand (`sr` can
/// only be a destination in the 68000's write forms: `move`, `ori`, `andi`,
/// `eori`).
fn sets_level(mnem: &str, ops: &[CodeOperand]) -> bool {
    mnem == "stop" || matches!(ops.last(), Some(CodeOperand::Sr))
}

/// The interrupt-mask field (bits 8..=10) of an immediate, read as 16 bits:
/// `Imm` is an unmasked signed `i128` (`#~$0700` arrives as `-1793`).
fn mask_bits(imm: i128) -> u8 {
    (((imm as u16) >> 8) & 7) as u8
}

/// `ori`/`andi`/`eori` with mask bits `k` applied to a level.
fn logic_on_level(mnem: &str, k: u8, level: Level) -> Level {
    match level {
        Level::Known(s) => logic_on_set(mnem, k, s),
        // An unknown level becomes known only when the result does not depend
        // on it: every bit set (`ori` 7) or every bit cleared (`andi` 0).
        Level::Unknown => match (mnem, k) {
            ("ori", 7) => Level::Known(LevelSet::only(7)),
            ("andi", 0) => Level::Known(LevelSet::only(0)),
            _ => Level::Unknown,
        },
    }
}

fn logic_on_set(mnem: &str, k: u8, s: LevelSet) -> Level {
    let op = |n: u8| match mnem {
        "ori" => n | k,
        "andi" => n & k,
        _ => n ^ k,
    };
    // Each fixed level maps to exactly one level, so a non-empty set of fixed
    // levels stays non-empty.
    let mut bits: u16 = s.fixed_levels().into_iter().fold(0, |acc, n| acc | (1 << op(n)));
    if s.has_entry() {
        // ENTRY → ENTRY when the op leaves every bit alone; → a fixed level when
        // it forces every bit; otherwise the result depends on the entry level.
        let entry_to = match (mnem, k) {
            ("ori", 0) | ("andi", 7) | ("eori", 0) => ENTRY_BIT,
            ("ori", 7) => 1 << 7,
            ("andi", 0) => 1,
            _ => return Level::Unknown,
        };
        bits |= entry_to;
    }
    Level::Known(LevelSet(bits))
}

/// Apply a generic stack effect (`sp_effect`) to `stack`.
fn apply_sp(stack: Stack, eff: SpEffect) -> Stack {
    let Stack::Known { depth, saves } = stack else { return Stack::Lost };
    match eff {
        SpEffect::None => Stack::Known { depth, saves },
        SpEffect::Lost => Stack::Lost,
        SpEffect::Push(n) => match depth.checked_add(n) {
            Some(depth) => Stack::Known { depth, saves },
            None => Stack::Lost,
        },
        SpEffect::Drop(n) => drop_bytes(depth, saves, n),
    }
}

/// `n` bytes leave the stack: underflow (`n > depth`) → `Lost`; a save cut in
/// half (`below < new < below + 2`) → `Lost`; every save at or above the new
/// depth is discarded whole.
fn drop_bytes(depth: u32, mut saves: Vec<Save>, n: u32) -> Stack {
    if n > depth {
        return Stack::Lost;
    }
    let new = depth - n;
    if saves.iter().any(|s| s.below < new && new < s.below + 2) {
        return Stack::Lost;
    }
    saves.retain(|s| s.below < new);
    Stack::Known { depth: new, saves }
}

/// The state after the instruction at `idx`, given the state arriving at it.
/// Pure: it reads only its arguments and records nothing.
pub(crate) fn step(items: &[CodeItem], st: &SrState, idx: usize, mnem: &str, ops: &[CodeOperand]) -> SrState {
    let out = step_inner(items, st, idx, mnem, ops);
    debug_assert!(out.is_well_formed(), "empty LevelSet after step at {idx}");
    out
}

fn step_inner(items: &[CodeItem], st: &SrState, idx: usize, mnem: &str, ops: &[CodeOperand]) -> SrState {
    use CodeOperand as O;
    let level = st.level;
    let stack = st.stack.clone();

    if is_call(mnem) {
        return SrState { level: call_effect(level), stack };
    }

    // `move.w sr, -(sp)`: save the level.
    if mnem == "move" && matches!(ops, [O::Sr, O::PreDec(Reg::A7)]) {
        let stack = match stack {
            Stack::Known { depth, mut saves } => match depth.checked_add(2) {
                Some(new) => {
                    saves.push(Save { below: depth, level });
                    Stack::Known { depth: new, saves }
                }
                None => Stack::Lost,
            },
            Stack::Lost => Stack::Lost,
        };
        return SrState { level, stack };
    }

    if sets_level(mnem, ops) {
        // `stop #imm` loads sr from its immediate; it has no `sr` operand.
        if mnem == "stop" {
            let level = match ops.first() {
                Some(O::Imm(v)) => Level::Known(LevelSet::only(mask_bits(*v))),
                _ => Level::Unknown,
            };
            return SrState { level, stack };
        }
        // Every other write: `sr` is the last operand.
        match (mnem, ops) {
            // `move.w (sp)+, sr`: restore a saved level.
            ("move", [O::PostInc(Reg::A7), O::Sr]) => {
                if let Stack::Known { depth, saves } = &stack {
                    if let Some(top) = saves.last() {
                        if depth.checked_sub(2) == Some(top.below) {
                            let mut saves = saves.clone();
                            let top = saves.pop().expect("a top save");
                            return SrState {
                                level: top.level,
                                stack: Stack::Known { depth: depth - 2, saves },
                            };
                        }
                    }
                }
                // The word popped is not a save: unknown level, generic Drop(2).
                return SrState { level: Level::Unknown, stack: apply_sp(stack, SpEffect::Drop(2)) };
            }
            ("move", [O::Imm(v), O::Sr]) => {
                return SrState { level: Level::Known(LevelSet::only(mask_bits(*v))), stack };
            }
            ("ori" | "andi" | "eori", [O::Imm(v), O::Sr]) => {
                return SrState { level: logic_on_level(mnem, mask_bits(*v), level), stack };
            }
            ("ori" | "andi" | "eori", _) => {
                return SrState { level: Level::Unknown, stack };
            }
            // `move` from anything else, or a write form the table does not name.
            _ => {
                return SrState { level: Level::Unknown, stack: apply_sp(stack, sp_effect(items, idx)) };
            }
        }
    }

    SrState { level, stack: apply_sp(stack, sp_effect(items, idx)) }
}

/// Why one way out of a routine is not provably at the caller's level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MaskFindingKind {
    /// This way out can be taken at these fixed levels (and perhaps the caller's too).
    NotTheCallers { levels: Vec<u8> },
    /// The level cannot be followed to this way out.
    Untracked,
    /// The routine leaves by `rte`, which loads the level from the stack.
    Rte,
}

/// One way out and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MaskFinding {
    pub(crate) idx: usize,
    pub(crate) span: Span,
    pub(crate) kind: MaskFindingKind,
}

/// Every way out of a 68000 routine that is not provably at the caller's level.
/// Sorted by `idx`, at most one finding per instruction.
///
/// Walks from every entry ([`entry_seeds`]) with [`step`], then judges each
/// leaving edge of each instruction on that instruction's OUT state.
/// Findings are read from the walk's result, never recorded inside `step`.
/// The stack is not judged here (the stack-balance check owns that); a lost
/// stack matters only when it makes a restore unknown.
///
/// Entries the walk cannot see (a pointer, a `targets(...)` jump) can only land
/// on a label, so after a first walk from the ordinary entries, these labels are
/// entries too, each at [`SrState::at_entry`] ([`hidden_entries`]): every code
/// label in code that walk did not reach, every label a `targets(...)` list
/// names, every code label whose address the routine takes, and every code label
/// a transfer names in an operand the CFG does not follow. Then, while
/// some instruction is still not reached, the lowest-index one becomes one more
/// entry and the walk runs again, so every instruction is judged.
///
/// Why `at_entry` is sound there: a jump that enters is itself a way out
/// judged here, so it can only be taken cleanly at the caller's level; at the
/// label the entry state meets whatever else arrives, and a differing depth or
/// save list loses the stack, so a restore after it is `Unknown` (a finding).
/// A call that enters (`jsr (a0)`, `jsr .h(pc)`) is NOT judged: there the seed
/// reads the helper as a routine of its own whose entry level is its caller's,
/// and the call as leaving the level alone ([`call_effect`]). A helper that
/// throws away its return address and returns past its caller breaks that and
/// is not modelled (piece 2b, `docs/QUEUE.md` STEPA-P2B-CALLS).
/// A data label is not an entry ([`code_label_index`]).
///
/// An entry reached by address arithmetic lands on no label. When a routine has
/// that hazard ([`offset_entry_hazard`]), EVERY instruction is an entry at
/// [`SrState::at_entry`] as well. Out of the model, on purpose: an interior
/// address formed OUTSIDE the routine without an `export` label.
pub(crate) fn mask_findings(items: &[CodeItem], noreturn: &BTreeSet<String>) -> Vec<MaskFinding> {
    let empty_ext = BTreeMap::new();
    let empty_types = BTreeSet::new();
    let env = SrExternalEnv {
        noreturn,
        extern_summaries: &empty_ext,
        sr_clobber_types: &empty_types,
        whole_program: false,
    };
    let proc = SrProcInput {
        name: "",
        items,
        falls_into: None,
        is_resumable: false,
        is_continuation: false,
        span: Span { source: sigil_span::SourceId(0), start: 0, end: 0 },
    };
    let procs = [proc];
    let table = SummaryTable::compute(&procs, &env);
    mask_findings_for_proc(0, &procs[0], &table, &env)
}

pub(crate) fn mask_findings_for_decl(
    decl: &crate::ast::ProcDecl,
    items: &[CodeItem],
    noreturn: &BTreeSet<String>,
) -> Vec<MaskFinding> {
    let empty_ext = BTreeMap::new();
    let empty_types = BTreeSet::new();
    let env = SrExternalEnv {
        noreturn,
        extern_summaries: &empty_ext,
        sr_clobber_types: &empty_types,
        whole_program: false,
    };
    let proc = SrProcInput {
        name: &decl.name,
        items,
        falls_into: decl.falls_into.as_deref(),
        is_resumable: decl.is_resumable(),
        is_continuation: decl.is_continuation(),
        span: decl.span,
    };
    let procs = [proc];
    let table = SummaryTable::compute(&procs, &env);
    mask_findings_for_proc(0, &procs[0], &table, &env)
}

/// Every way out of `proc` that is not provably at the caller's interrupt
/// level, using the converged [`SummaryTable`] for local helpers, direct calls,
/// mid-body tails, branches out, and `falls_into` (step A piece 2b).
pub(crate) fn mask_findings_for_proc(
    proc_idx: usize,
    proc: &SrProcInput<'_>,
    table: &SummaryTable,
    env: &SrExternalEnv<'_>,
) -> Vec<MaskFinding> {
    let items = proc.items;
    let cfg = Cfg::build(items);
    let ctx = WalkCtx { proc_idx, items, cfg: &cfg, is_resumable: proc.is_resumable, table, env };
    let walk = |seeds: &[(usize, Reach)]| {
        forward_in_states(
            &cfg,
            Cpu::M68000,
            seeds,
            &|r: &Reach, idx, mnem, ops| step_reach(ctx, r, idx, mnem, ops),
            &|_| true,
        )
    };
    let mut at = entry_seeds(items, &cfg);
    let first = walk(&at_entry_reach(&at));
    let taken = address_taken(items, &cfg);
    at.extend(hidden_entries_reach(items, &cfg, &first, &taken));
    if offset_entry_hazard(items, &cfg, &taken) {
        at.extend(
            items.iter().enumerate().filter(|(_, it)| matches!(it, CodeItem::Instr { .. })).map(|(idx, _)| idx),
        );
    }
    let mut seeds = at_entry_reach(&at);
    let mut states = walk(&seeds);
    while let Some(idx) = first_unreached_reach(items, &states) {
        seeds.push((idx, Reach(Some(SrState::at_entry()))));
        states = walk(&seeds);
    }

    let mut found: BTreeMap<usize, MaskFinding> = BTreeMap::new();
    for (&idx, reach_in) in &states {
        let Some(st_in) = &reach_in.0 else { continue };
        let CodeItem::Instr { mnemonic, ops, span, .. } = &items[idx] else { continue };
        // Review C3: a callee that touches its caller's stack can discard its
        // return address and return directly past `proc` at `st_in.level`.
        if is_call(mnemonic) {
            let callee = table.call_summary(ctx, idx);
            if callee.touches_caller_stack {
                if let Some(e) = callee.exit_level {
                    if let Some(kind) = judge_level(compose_level(st_in.level, e)) {
                        found.entry(idx).or_insert(MaskFinding { idx, span: *span, kind });
                    }
                }
            }
        }
        let reach_out = step_reach(ctx, reach_in, idx, mnemonic, ops);
        let Some(out) = reach_out.0 else { continue };
        for edge in edges_for(&cfg, Cpu::M68000, idx) {
            let kind = match edge {
                Edge::Follow(_) => None,
                Edge::Return if mnemonic == "rte" => {
                    if proc.is_continuation {
                        judge_level(out.level)
                    } else {
                        Some(MaskFindingKind::Rte)
                    }
                }
                Edge::Return => judge_level(out.level),
                Edge::TailOut | Edge::BranchOut => {
                    if exit_diverges(&items[idx], env.noreturn) {
                        None
                    } else if let Some(target_sum) = table.exit_transfer_summary(ctx, idx) {
                        target_sum
                            .exit_level
                            .and_then(|e| judge_level(compose_level(out.level, e)))
                    } else {
                        None
                    }
                }
                Edge::FallOff => {
                    if let Some(next_name) = proc.falls_into {
                        let target_sum = table.resolve_external(next_name, env);
                        target_sum
                            .exit_level
                            .and_then(|e| judge_level(compose_level(out.level, e)))
                    } else {
                        judge_level(out.level)
                    }
                }
            };
            if let Some(kind) = kind {
                found.entry(idx).or_insert(MaskFinding { idx, span: *span, kind });
            }
        }
    }
    // Empty proc with `falls_into`: delegate exit verdict to `falls_into`.
    if items.iter().all(|it| !matches!(it, CodeItem::Instr { .. })) {
        if let Some(next_name) = proc.falls_into {
            let target_sum = table.resolve_external(next_name, env);
            if let Some(kind) = target_sum.exit_level.and_then(judge_level) {
                found.insert(0, MaskFinding { idx: 0, span: proc.span, kind });
            }
        }
    }
    found.into_values().collect()
}

/// The lowest-index instruction the walk has no live (`Reach(Some(_))`) state for.
fn first_unreached_reach(items: &[CodeItem], states: &BTreeMap<usize, Reach>) -> Option<usize> {
    items
        .iter()
        .enumerate()
        .find(|(idx, it)| {
            matches!(it, CodeItem::Instr { .. })
                && !matches!(states.get(idx), Some(Reach(Some(_))))
        })
        .map(|(idx, _)| idx)
}

/// Every place a routine can be entered, each at [`SrState::at_entry`]: the
/// first instruction; the instruction at every exported code label; the
/// instruction at every local code label a `jsr`/`bsr`/`jbsr` names.
fn entry_seeds(items: &[CodeItem], cfg: &Cfg) -> BTreeSet<usize> {
    let mut at: BTreeSet<usize> = BTreeSet::new();
    if let Some(first) = items.iter().position(|it| matches!(it, CodeItem::Instr { .. })) {
        at.insert(first);
    }
    for item in items {
        match item {
            CodeItem::Label { name, export: true, .. } => at.extend(code_label_index(items, cfg, name)),
            CodeItem::Instr { mnemonic, ops, .. } if is_call(mnemonic) => {
                for op in ops {
                    if let CodeOperand::Sym(name) = op {
                        at.extend(code_label_index(items, cfg, name));
                    }
                }
            }
            _ => {}
        }
    }
    at
}

fn at_entry_reach(at: &BTreeSet<usize>) -> Vec<(usize, Reach)> {
    at.iter().map(|&idx| (idx, Reach(Some(SrState::at_entry())))).collect()
}

/// The labels an entry the CFG cannot see may land on, as instruction indices.
fn hidden_entries_reach(
    items: &[CodeItem],
    cfg: &Cfg,
    first: &BTreeMap<usize, Reach>,
    taken: &BTreeSet<usize>,
) -> BTreeSet<usize> {
    let mut at = BTreeSet::new();
    for item in items {
        match item {
            CodeItem::Label { name, .. } => {
                if let Some(idx) = code_label_index(items, cfg, name) {
                    if !matches!(first.get(&idx), Some(Reach(Some(_)))) {
                        at.insert(idx);
                    }
                }
            }
            CodeItem::Instr { mnemonic, ops, targets, .. } => {
                for t in targets {
                    at.extend(code_label_index(items, cfg, t));
                }
                if is_transfer(mnemonic) {
                    for op in ops.iter().filter(|op| !matches!(op, CodeOperand::Sym(_))) {
                        for name in operand_symbols(op) {
                            at.extend(code_label_index(items, cfg, &name));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    at.extend(taken.iter().copied());
    at
}

/// Class (c) of [`hidden_entries`]: the code labels whose address the routine
/// takes, a name in an operand of an instruction that is not a branch, a call
/// or a return.
fn address_taken(items: &[CodeItem], cfg: &Cfg) -> BTreeSet<usize> {
    let mut at = BTreeSet::new();
    for item in items {
        if let CodeItem::Instr { mnemonic, ops, .. } = item {
            if !is_transfer(mnemonic) {
                for op in ops {
                    for name in operand_symbols(op) {
                        at.extend(code_label_index(items, cfg, &name));
                    }
                }
            }
        }
    }
    at
}

/// Can an entry land on an instruction that has no label? (1) An operand names
/// a local code label with an offset that is not provably zero: `.x+6(pc)`
/// (`PcRel` addend), `.x(pc, d0.w)` (`PcRelIdx`: the index is an offset),
/// `.x+6` absolute (`SymOff`), `.x(a1)` (`DispSymInd`: the register is an
/// offset), or an immediate (`ImmLink`/`Z80MemSym`) expression that is not just
/// the bare symbol. `Sym` and `AbsSym` carry no offset. (2) The routine takes
/// a local code label's address and has an indirect `jmp`/`jsr`/`jbsr` with a
/// displacement or an index (`d(aN)` with d not 0, `d(aN, Xn)`, `d(pc, Xn)`,
/// `sym(aN)`) and no `targets` list.
/// `taken` is [`address_taken`].
fn offset_entry_hazard(items: &[CodeItem], cfg: &Cfg, taken: &BTreeSet<usize>) -> bool {
    use CodeOperand as O;
    let is_code = |name: &str| code_label_index(items, cfg, name).is_some();
    let names_code_with_offset = |op: &CodeOperand| match op {
        O::PcRel { target, addend } => *addend != 0 && is_code(target),
        O::SymOff { sym, off } => *off != 0 && is_code(sym),
        O::PcRelIdx { target, .. } | O::DispSymInd { target, .. } => is_code(target),
        O::ImmLink { target } | O::Z80MemSym { target } => {
            !matches!(target, sigil_ir::expr::Expr::Sym(_)) && operand_symbols(op).iter().any(|n| is_code(n))
        }
        _ => false,
    };
    let offset_named = items.iter().any(|it| match it {
        CodeItem::Instr { ops, .. } => ops.iter().any(names_code_with_offset),
        _ => false,
    });
    if offset_named {
        return true;
    }
    let displaced = |op: &CodeOperand| match op {
        O::DispInd { disp, .. } => *disp != 0,
        O::IndIdx { .. } | O::PcRelIdx { .. } | O::DispSymInd { .. } => true,
        _ => false,
    };
    let displaced_jump = items.iter().any(|it| match it {
        CodeItem::Instr { mnemonic, ops, targets, .. } => {
            matches!(mnemonic.as_str(), "jmp" | "jsr" | "jbsr") && targets.is_empty() && ops.iter().any(displaced)
        }
        _ => false,
    });
    displaced_jump && !taken.is_empty()
}

/// The instruction a local label leads to, when only labels and context marks
/// sit between them. [`Cfg::label_index`] skips data too (the first instruction
/// at or after the label); a label on data is not a code entry, so `None`.
fn code_label_index(items: &[CodeItem], cfg: &Cfg, name: &str) -> Option<usize> {
    let target = cfg.label_index(name)?;
    let pos = items.iter().position(|it| matches!(it, CodeItem::Label { name: n, .. } if n == name))?;
    let between_is_code = items[pos..target]
        .iter()
        .all(|it| matches!(it, CodeItem::Label { .. } | CodeItem::ContextMark { .. }));
    between_is_code.then_some(target)
}

/// True when `name` is a local label at the end of `items` (only labels and
/// context marks follow it, no instructions or inline data).
fn is_closing_code_label(items: &[CodeItem], name: &str) -> bool {
    let Some(pos) =
        items.iter().position(|it| matches!(it, CodeItem::Label { name: n, .. } if n == name))
    else {
        return false;
    };
    items[pos..].iter().all(|it| matches!(it, CodeItem::Label { .. } | CodeItem::ContextMark { .. }))
}

/// A branch, a call or a return: a mnemonic whose symbol operand is where
/// control goes, not an address the routine takes.
fn is_transfer(mnem: &str) -> bool {
    matches!(mnem, "jmp" | "jsr" | "jbsr" | "jbra" | "jra" | "rts" | "rte" | "rtr" | "rtd")
        || (mnem.starts_with('b') && mnem.len() == 3)
        || mnem.starts_with("db")
        || mnem.starts_with("jb")
}

/// Every symbol name an operand mentions. Covered: `Sym`, `SymOff`, `PcRel`,
/// `PcRelIdx`, `AbsSym`, `DispSymInd`, and the `Expr::Sym` leaves of `ImmLink`
/// and `Z80MemSym`. Every other variant names no symbol.
fn operand_symbols(op: &CodeOperand) -> Vec<String> {
    use CodeOperand as O;
    match op {
        O::Sym(n) | O::SymOff { sym: n, .. } => vec![n.clone()],
        O::PcRel { target, .. }
        | O::PcRelIdx { target, .. }
        | O::AbsSym { target, .. }
        | O::DispSymInd { target, .. } => vec![target.clone()],
        O::ImmLink { target } | O::Z80MemSym { target } => {
            let mut out = Vec::new();
            expr_symbols(target, &mut out);
            out
        }
        _ => Vec::new(),
    }
}

fn expr_symbols(e: &sigil_ir::expr::Expr, out: &mut Vec<String>) {
    use sigil_ir::expr::Expr as E;
    match e {
        E::Sym(n) => out.push(n.clone()),
        E::Binary { lhs, rhs, .. } => {
            expr_symbols(lhs, out);
            expr_symbols(rhs, out);
        }
        E::Unary { operand, .. } => expr_symbols(operand, out),
        E::Int(_) => {}
    }
}

/// The verdict on the level a way out leaves at.
fn judge_level(level: Level) -> Option<MaskFindingKind> {
    match level {
        Level::Known(s) if s.is_entry_only() => None,
        Level::Known(s) => Some(MaskFindingKind::NotTheCallers { levels: s.fixed_levels() }),
        Level::Unknown => Some(MaskFindingKind::Untracked),
    }
}

/// True when a `clobbers(...)` token list names `sr` or `sr.mask`.
pub(crate) fn tokens_clobber_sr_mask(tokens: &[(String, Option<String>)]) -> bool {
    tokens.iter().any(|(lo, hi)| {
        hi.is_none() && (lo.eq_ignore_ascii_case("sr") || lo.eq_ignore_ascii_case("sr.mask"))
    })
}

/// The mask-preserving spelling (`"sr"` or `"sr.mask"`) declared in a
/// `preserves(...)` list, if any. Bare `"sr"` wins if both appear.
pub(crate) fn preserves_sr_token(tokens: &[(String, Option<String>)]) -> Option<&'static str> {
    let mut has_bare = false;
    let mut has_mask = false;
    for (lo, hi) in tokens {
        if hi.is_some() {
            continue;
        }
        if lo.eq_ignore_ascii_case("sr") {
            has_bare = true;
        } else if lo.eq_ignore_ascii_case("sr.mask") {
            has_mask = true;
        }
    }
    if has_bare {
        Some("sr")
    } else if has_mask {
        Some("sr.mask")
    } else {
        None
    }
}

/// True when a proc/extern/contract-type signature explicitly licenses writing
/// `sr` or `sr.mask` via `clobbers(...)`, `out(...)`, `out(... if cc)`, or `inout(...)`.
pub(crate) fn sig_modifies_sr_mask(sig: &crate::ast::ProcSig) -> bool {
    sig.clobbers.as_deref().is_some_and(tokens_clobber_sr_mask)
        || sig.out.as_deref().is_some_and(tokens_clobber_sr_mask)
        || sig.inout.as_deref().is_some_and(tokens_clobber_sr_mask)
        || sig
            .out_cond
            .iter()
            .any(|c| c.reg.eq_ignore_ascii_case("sr") || c.reg.eq_ignore_ascii_case("sr.mask"))
}

/// Build an interrupt-level summary from an `extern proc` declaration.
pub(crate) fn extern_proc_summary(decl: &crate::ast::ExternProcDecl) -> ProcSummary {
    if decl.is_noreturn() {
        return ProcSummary::DIVERGES;
    }
    if preserves_sr_token(&decl.sig.preserves).is_some() {
        return ProcSummary::IDENTITY;
    }
    if sig_modifies_sr_mask(&decl.sig) {
        return ProcSummary::UNKNOWN;
    }
    ProcSummary::IDENTITY
}

/// Format a `[proc.preserves-sr-unbalanced]` diagnostic message.
pub(crate) fn format_mask_finding(name: &str, token: &str, kind: &MaskFindingKind) -> String {
    match kind {
        MaskFindingKind::NotTheCallers { levels } => {
            let levels = levels.iter().map(u8::to_string).collect::<Vec<_>>().join(" or ");
            format!(
                "[proc.preserves-sr-unbalanced] `{name}` declares `preserves({token})` but this \
                 way out can be reached with the interrupt level at {levels}, not the caller's \
                 (a path masks interrupts and does not restore the saved level)"
            )
        }
        MaskFindingKind::Untracked => format!(
            "[proc.preserves-sr-unbalanced] `{name}` declares `preserves({token})` but the \
             interrupt level cannot be followed to this way out (restore it with \
             `move.w (sp)+, sr` from the matching `move.w sr, -(sp)`, with the stack as it \
             was at the save)"
        ),
        MaskFindingKind::Rte => format!(
            "[proc.preserves-sr-unbalanced] `{name}` declares `preserves({token})` but leaves \
             by `rte`, which loads the interrupt level from the stack"
        ),
    }
}

/// Why a `with <ctx> { }` bracket's held interrupt level was broken (Rule 3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BracketFindingKind {
    /// Instruction or callee can reach fixed levels outside the held set.
    LeftHeldLevel { observed: Vec<u8> },
    /// Instruction or callee makes the interrupt level untracked.
    Untracked,
}

/// One Rule 3 (`[proc.with-sr-level-broken]`) finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BracketFinding {
    pub(crate) idx: usize,
    pub(crate) span: Span,
    pub(crate) ctx: String,
    pub(crate) held: Vec<u8>,
    pub(crate) callee: Option<String>,
    pub(crate) kind: BracketFindingKind,
}

fn judge_bracket_level(obs: Level, allowed: LevelSet) -> Option<BracketFindingKind> {
    if obs.subset_of(allowed) {
        return None;
    }
    match obs {
        Level::Known(s) => {
            let outside: Vec<u8> =
                s.fixed_levels().into_iter().filter(|&n| !allowed.contains_fixed(n)).collect();
            if outside.is_empty() {
                Some(BracketFindingKind::Untracked)
            } else {
                Some(BracketFindingKind::LeftHeldLevel { observed: outside })
            }
        }
        Level::Unknown => Some(BracketFindingKind::Untracked),
    }
}

/// Rule 3: for every complete `with <ctx> { }` region whose `acquire`
/// establishes a concrete interrupt level (no `ENTRY` bit), verify that every
/// reachable body instruction and callee stays within that held level.
pub(crate) fn check_bracket_levels(
    proc_idx: usize,
    proc: &SrProcInput<'_>,
    table: &SummaryTable,
    env: &SrExternalEnv<'_>,
) -> Vec<BracketFinding> {
    let items = proc.items;
    if !items.iter().any(|it| matches!(it, CodeItem::ContextMark { .. })) {
        return Vec::new();
    }
    let (regions, _) = crate::context::regions_of(proc.name, items);
    if regions.is_empty() {
        return Vec::new();
    }
    let cfg = Cfg::build(items);
    let ctx = WalkCtx { proc_idx, items, cfg: &cfg, is_resumable: proc.is_resumable, table, env };
    let mut findings = Vec::new();

    for region in &regions {
        let Some(first_acq) = (region.enter + 1..region.acquire_end)
            .find(|&i| matches!(items[i], CodeItem::Instr { .. }))
        else {
            continue;
        };
        let acq_seeds = [(first_acq, Reach(Some(SrState::at_entry())))];
        let acq_states = forward_in_states(
            &cfg,
            Cpu::M68000,
            &acq_seeds,
            &|r: &Reach, idx, mnem, ops| step_reach(ctx, r, idx, mnem, ops),
            &|succ| region.in_acquire(succ),
        );

        let mut acq_level: Option<Level> = None;
        let mut body_entries: BTreeSet<usize> = BTreeSet::new();
        for (&idx, reach_in) in &acq_states {
            if reach_in.0.is_none() {
                continue;
            }
            let CodeItem::Instr { mnemonic, ops, .. } = &items[idx] else { continue };
            let reach_out = step_reach(ctx, reach_in, idx, mnemonic, ops);
            let Some(out) = reach_out.0 else { continue };
            for edge in edges_for(&cfg, Cpu::M68000, idx) {
                if let Edge::Follow(succ) = edge {
                    if succ > region.acquire_end && succ < region.body_end {
                        acq_level = meet_opt_level(acq_level, Some(out.level));
                        body_entries.insert(succ);
                    }
                }
            }
        }

        let Some(Level::Known(allowed)) = acq_level else { continue };
        if allowed.has_entry() || body_entries.is_empty() {
            continue;
        }
        let held = allowed.fixed_levels();

        let seed_state = SrState {
            level: Level::Known(allowed),
            stack: Stack::Known { depth: 0, saves: Vec::new() },
        };
        let body_seeds: Vec<(usize, Reach)> =
            body_entries.iter().map(|&i| (i, Reach(Some(seed_state.clone())))).collect();
        let body_states = forward_in_states(
            &cfg,
            Cpu::M68000,
            &body_seeds,
            &|r: &Reach, idx, mnem, ops| step_reach(ctx, r, idx, mnem, ops),
            &|succ| succ > region.acquire_end && succ < region.body_end,
        );

        for (&idx, reach_in) in &body_states {
            let Some(st_in) = &reach_in.0 else { continue };
            let CodeItem::Instr { mnemonic, ops, span, author, .. } = &items[idx] else { continue };
            if matches!(author, ItemAuthor::AssertDesugar) {
                continue;
            }
            if is_call(mnemonic) {
                let callee = table.call_summary(ctx, idx);
                let mut combined: Option<Level> = None;
                if let Some(b) = callee.body_levels {
                    combined =
                        meet_opt_level(combined, Some(compose_level(Level::Known(allowed), b)));
                }
                if let Some(e) = callee.exit_level {
                    combined =
                        meet_opt_level(combined, Some(compose_level(Level::Known(allowed), e)));
                }
                if let Some(obs) = combined {
                    if let Some(kind) = judge_bracket_level(obs, allowed) {
                        let callee_name = match classify_transfer(
                            items,
                            &cfg,
                            idx,
                            proc.is_resumable,
                            env.noreturn,
                        ) {
                            TransferTarget::External(name) => Some(name),
                            TransferTarget::TypedIndirect(ty) => Some(format!("({ty})")),
                            TransferTarget::Local(_) | TransferTarget::ClosingLocal => {
                                ops.iter().find_map(|o| match o {
                                    CodeOperand::Sym(n)
                                    | CodeOperand::PcRel { target: n, .. }
                                    | CodeOperand::AbsSym { target: n, .. }
                                    | CodeOperand::SymOff { sym: n, .. } => Some(n.clone()),
                                    _ => None,
                                })
                            }
                            _ => None,
                        };
                        findings.push(BracketFinding {
                            idx,
                            span: *span,
                            ctx: region.ctx.clone(),
                            held: held.clone(),
                            callee: callee_name,
                            kind,
                        });
                    }
                }
                continue;
            }
            if sets_level(mnemonic, ops) {
                let probe_in = if st_in.level.subset_of(allowed) {
                    st_in.clone()
                } else {
                    SrState { level: Level::Known(allowed), stack: st_in.stack.clone() }
                };
                let out = step(items, &probe_in, idx, mnemonic, ops);
                if let Some(kind) = judge_bracket_level(out.level, allowed) {
                    findings.push(BracketFinding {
                        idx,
                        span: *span,
                        ctx: region.ctx.clone(),
                        held: held.clone(),
                        callee: None,
                        kind,
                    });
                }
            }
        }
    }

    findings
}

/// Format a `[proc.with-sr-level-broken]` diagnostic message.
pub(crate) fn format_bracket_finding(proc_name: &str, f: &BracketFinding) -> String {
    let held_str = f.held.iter().map(u8::to_string).collect::<Vec<_>>().join(" or ");
    let ctx = &f.ctx;
    match (&f.callee, &f.kind) {
        (Some(callee), BracketFindingKind::LeftHeldLevel { observed }) => {
            let obs_str = observed.iter().map(u8::to_string).collect::<Vec<_>>().join(" or ");
            format!(
                "[proc.with-sr-level-broken] `{proc_name}` holds interrupt level {held_str} \
                 inside `with {ctx}`, but calling `{callee}` can reach interrupt level {obs_str}"
            )
        }
        (Some(callee), BracketFindingKind::Untracked) => format!(
            "[proc.with-sr-level-broken] `{proc_name}` holds interrupt level {held_str} \
             inside `with {ctx}`, but calling `{callee}` leaves the interrupt level untracked"
        ),
        (None, BracketFindingKind::LeftHeldLevel { observed }) => {
            let obs_str = observed.iter().map(u8::to_string).collect::<Vec<_>>().join(" or ");
            format!(
                "[proc.with-sr-level-broken] `{proc_name}` holds interrupt level {held_str} \
                 inside `with {ctx}`, but this instruction sets the interrupt level to {obs_str}"
            )
        }
        (None, BracketFindingKind::Untracked) => format!(
            "[proc.with-sr-level-broken] `{proc_name}` holds interrupt level {held_str} \
             inside `with {ctx}`, but this instruction leaves the interrupt level untracked"
        ),
    }
}

/// True when a context's acquire + release items preserve the caller's
/// interrupt level on every way out (replaces the linear `sr_writes_round_trip`
/// check in `eval/asm.rs`).
pub(crate) fn context_pair_round_trips_sr<'a>(
    items: impl Iterator<Item = &'a CodeItem>,
) -> bool {
    let mut seq: Vec<CodeItem> = items.cloned().collect();
    seq.push(CodeItem::Instr {
        mnemonic: "rts".to_string(),
        size: None,
        ops: Vec::new(),
        span: Span { source: sigil_span::SourceId(0), start: 0, end: 0 },
        as_type: None,
        targets: Vec::new(),
        author: ItemAuthor::User,
    });
    let empty = BTreeSet::new();
    mask_findings(&seq, &empty).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{ItemAuthor, Width};
    use sigil_span::SourceId;
    use CodeOperand as O;

    fn ins(idx: u32, mnemonic: &str, size: Option<Width>, ops: Vec<CodeOperand>) -> CodeItem {
        CodeItem::Instr {
            mnemonic: mnemonic.to_string(),
            size,
            ops,
            span: Span { source: SourceId(0), start: idx, end: idx + 1 },
            as_type: None,
            targets: Vec::new(),
            author: ItemAuthor::User,
        }
    }
    fn label(name: &str) -> CodeItem {
        CodeItem::Label {
            name: name.to_string(),
            export: false,
            span: Span { source: SourceId(0), start: 0, end: 0 },
        }
    }

    const W: Option<Width> = Some(Width::W);
    const L: Option<Width> = Some(Width::L);
    const SP: Reg = Reg::A7;

    // The instruction forms the tests use.
    fn save() -> CodeItem {
        ins(0, "move", W, vec![O::Sr, O::PreDec(SP)])
    }
    fn restore() -> CodeItem {
        ins(0, "move", W, vec![O::PostInc(SP), O::Sr])
    }
    fn set_sr(imm: i128) -> CodeItem {
        ins(0, "move", W, vec![O::Imm(imm), O::Sr])
    }
    fn logic_sr(mnem: &str, imm: i128) -> CodeItem {
        ins(0, mnem, W, vec![O::Imm(imm), O::Sr])
    }
    fn push_d0() -> CodeItem {
        ins(0, "move", W, vec![O::Reg(Reg::D0), O::PreDec(SP)])
    }
    fn addq_sp(n: i128) -> CodeItem {
        ins(0, "addq", L, vec![O::Imm(n), O::Reg(SP)])
    }

    /// Step every item in order, straight line, from `st`.
    fn run(items: &[CodeItem], st: SrState) -> SrState {
        let mut st = st;
        for (idx, it) in items.iter().enumerate() {
            if let CodeItem::Instr { mnemonic, ops, .. } = it {
                st = step(items, &st, idx, mnemonic, ops);
            }
        }
        st
    }

    fn set(levels: &[u8]) -> LevelSet {
        let mut it = levels.iter();
        let first = LevelSet::only(*it.next().expect("a level"));
        it.fold(first, |acc, l| acc.union(LevelSet::only(*l)))
    }
    fn known(s: LevelSet) -> Level {
        Level::Known(s)
    }
    fn entry() -> Level {
        Level::Known(LevelSet::ENTRY)
    }
    fn st(level: Level, depth: u32, saves: Vec<Save>) -> SrState {
        SrState { level, stack: Stack::Known { depth, saves } }
    }
    fn lost(level: Level) -> SrState {
        SrState { level, stack: Stack::Lost }
    }
    fn sv(below: u32, level: Level) -> Save {
        Save { below, level }
    }

    #[test]
    fn levelset_helpers_read_the_bits() {
        let s = set(&[4, 2]).union(LevelSet::ENTRY);
        assert_eq!(s.fixed_levels(), vec![2, 4]);
        assert!(s.has_entry());
        assert!(!s.is_entry_only());
        assert!(LevelSet::ENTRY.is_entry_only());
        assert!(LevelSet::ENTRY.fixed_levels().is_empty());
        assert_eq!(LevelSet::only(9), LevelSet::only(1));
        assert!(!LevelSet::only(3).has_entry());
        assert_eq!(SrState::at_entry(), st(entry(), 0, vec![]));
    }

    #[test]
    fn an_immediate_move_sets_the_level() {
        for (imm, k) in [(0x2700, 7u8), (0x2300, 3), (0x2000, 0)] {
            let items = vec![set_sr(imm)];
            assert_eq!(run(&items, SrState::at_entry()), st(known(LevelSet::only(k)), 0, vec![]), "{imm:#x}");
        }
        // Also from an unknown level, and the stack is left alone.
        let items = vec![set_sr(0x2700)];
        let from = st(Level::Unknown, 2, vec![sv(0, entry())]);
        assert_eq!(run(&items, from), st(known(LevelSet::only(7)), 2, vec![sv(0, entry())]));
    }

    #[test]
    fn an_unmasked_negative_immediate_is_read_as_sixteen_bits() {
        let items = vec![logic_sr("andi", -1793)];
        assert_eq!(
            run(&items, st(known(LevelSet::only(7)), 0, vec![])),
            st(known(LevelSet::only(0)), 0, vec![])
        );
        assert_eq!(run(&items, SrState::at_entry()), st(known(LevelSet::only(0)), 0, vec![]));
    }

    #[test]
    fn ori_seven_masks_from_any_level() {
        let items = vec![logic_sr("ori", 0x0700)];
        let seven = known(LevelSet::only(7));
        assert_eq!(run(&items, SrState::at_entry()), st(seven, 0, vec![]));
        assert_eq!(run(&items, st(known(set(&[2, 4])), 0, vec![])), st(seven, 0, vec![]));
        assert_eq!(
            run(&items, st(known(set(&[1]).union(LevelSet::ENTRY)), 0, vec![])),
            st(seven, 0, vec![])
        );
        assert_eq!(run(&items, st(Level::Unknown, 0, vec![])), st(seven, 0, vec![]));
    }

    #[test]
    fn ori_partial_on_the_entry_level_is_unknown() {
        let items = vec![logic_sr("ori", 0x0300)];
        assert_eq!(run(&items, SrState::at_entry()), st(Level::Unknown, 0, vec![]));
        assert_eq!(
            run(&items, st(known(set(&[2]).union(LevelSet::ENTRY)), 0, vec![])),
            st(Level::Unknown, 0, vec![])
        );
        // k = 0 keeps the entry level.
        let items = vec![logic_sr("ori", 0)];
        assert_eq!(run(&items, SrState::at_entry()), SrState::at_entry());
    }

    #[test]
    fn ori_partial_on_fixed_levels_is_exact() {
        let items = vec![logic_sr("ori", 0x0100)];
        assert_eq!(run(&items, st(known(set(&[2, 4])), 0, vec![])), st(known(set(&[3, 5])), 0, vec![]));
    }

    #[test]
    fn andi_and_eori_follow_the_bits() {
        // andi k = 7 keeps ENTRY and every fixed level.
        let items = vec![logic_sr("andi", 0x0700)];
        assert_eq!(run(&items, SrState::at_entry()), SrState::at_entry());
        assert_eq!(run(&items, st(known(set(&[2, 5])), 0, vec![])), st(known(set(&[2, 5])), 0, vec![]));
        // andi k = 3 on fixed levels is exact; on ENTRY it is unknown.
        let items = vec![logic_sr("andi", 0x0300)];
        assert_eq!(run(&items, st(known(set(&[5, 6])), 0, vec![])), st(known(set(&[1, 2])), 0, vec![]));
        assert_eq!(run(&items, SrState::at_entry()), st(Level::Unknown, 0, vec![]));
        // andi k = 0 sets level 0 even from unknown.
        let items = vec![logic_sr("andi", 0)];
        assert_eq!(run(&items, st(Level::Unknown, 0, vec![])), st(known(LevelSet::only(0)), 0, vec![]));
        // eori k = 0 keeps ENTRY.
        let items = vec![logic_sr("eori", 0)];
        assert_eq!(run(&items, SrState::at_entry()), SrState::at_entry());
        // eori k = 7 on {0} gives {7}; on ENTRY it is unknown.
        let items = vec![logic_sr("eori", 0x0700)];
        assert_eq!(run(&items, st(known(LevelSet::only(0)), 0, vec![])), st(known(LevelSet::only(7)), 0, vec![]));
        assert_eq!(run(&items, SrState::at_entry()), st(Level::Unknown, 0, vec![]));
        // eori from unknown stays unknown.
        assert_eq!(run(&items, st(Level::Unknown, 0, vec![])), st(Level::Unknown, 0, vec![]));
    }

    #[test]
    fn a_non_immediate_logic_source_is_unknown() {
        for m in ["ori", "andi", "eori"] {
            let items = vec![ins(0, m, W, vec![O::Reg(Reg::D0), O::Sr])];
            assert_eq!(run(&items, SrState::at_entry()), st(Level::Unknown, 0, vec![]), "{m}");
        }
    }

    #[test]
    fn stop_sets_the_level() {
        let items = vec![ins(0, "stop", None, vec![O::Imm(0x2500)])];
        assert_eq!(run(&items, SrState::at_entry()), st(known(LevelSet::only(5)), 0, vec![]));
    }

    #[test]
    fn a_register_source_is_unknown() {
        let items = vec![ins(0, "move", W, vec![O::Reg(Reg::D0), O::Sr])];
        assert_eq!(run(&items, SrState::at_entry()), st(Level::Unknown, 0, vec![]));
        // A memory source that pops: unknown, and the stack moves by sp_effect.
        let items = vec![ins(0, "move", W, vec![O::PostInc(Reg::A0), O::Sr])];
        assert_eq!(run(&items, st(entry(), 4, vec![])), st(Level::Unknown, 4, vec![]));
    }

    #[test]
    fn a_ccr_destination_does_not_change_the_level() {
        let items = vec![ins(0, "move", W, vec![O::Imm(0), O::Ccr])];
        assert_eq!(run(&items, SrState::at_entry()), SrState::at_entry());
    }

    #[test]
    fn save_then_restore_returns_the_saved_level() {
        let items = vec![save(), set_sr(0x2700), restore()];
        assert_eq!(run(&items, SrState::at_entry()), st(entry(), 0, vec![]));
        // Midway: the save is on the stack with the entry level.
        let mid = run(&items[..2], SrState::at_entry());
        assert_eq!(mid, st(known(LevelSet::only(7)), 2, vec![sv(0, entry())]));
    }

    #[test]
    fn nested_saves_restore_in_order() {
        let items = vec![save(), set_sr(0x2700), save(), set_sr(0x2300), restore()];
        let after_one = run(&items, SrState::at_entry());
        assert_eq!(after_one, st(known(LevelSet::only(7)), 2, vec![sv(0, entry())]));
        let mut items2 = items.clone();
        items2.push(restore());
        assert_eq!(run(&items2, SrState::at_entry()), st(entry(), 0, vec![]));
    }

    #[test]
    fn freeing_the_saved_word_discards_it_and_keeps_the_level() {
        let items = vec![save(), set_sr(0x2700), addq_sp(2)];
        assert_eq!(run(&items, SrState::at_entry()), st(known(LevelSet::only(7)), 0, vec![]));
    }

    #[test]
    fn popping_the_saved_word_elsewhere_discards_it() {
        let items = vec![save(), set_sr(0x2700), ins(0, "move", W, vec![O::PostInc(SP), O::Reg(Reg::D0)])];
        assert_eq!(run(&items, SrState::at_entry()), st(known(LevelSet::only(7)), 0, vec![]));
        let items = vec![save(), set_sr(0x2700), ins(0, "move", W, vec![O::PostInc(SP), O::Ccr])];
        assert_eq!(run(&items, SrState::at_entry()), st(known(LevelSet::only(7)), 0, vec![]));
    }

    #[test]
    fn a_long_pop_that_covers_the_saved_word_discards_it() {
        let items = vec![push_d0(), save(), ins(0, "move", L, vec![O::PostInc(SP), O::Reg(Reg::D1)])];
        let mid = run(&items[..2], SrState::at_entry());
        assert_eq!(mid, st(entry(), 4, vec![sv(2, entry())]));
        assert_eq!(run(&items, SrState::at_entry()), st(entry(), 0, vec![]));
    }

    #[test]
    fn a_pop_that_cuts_the_saved_word_in_half_loses_the_stack() {
        let items = vec![save(), addq_sp(1)];
        assert_eq!(run(&items, SrState::at_entry()), lost(entry()));
    }

    #[test]
    fn a_drop_past_entry_loses_the_stack() {
        let items = vec![addq_sp(2)];
        assert_eq!(run(&items, SrState::at_entry()), lost(entry()));
    }

    #[test]
    fn a_restore_with_something_pushed_on_top_is_unknown() {
        let items = vec![save(), push_d0(), restore()];
        assert_eq!(run(&items, SrState::at_entry()), st(Level::Unknown, 2, vec![sv(0, entry())]));
    }

    #[test]
    fn a_restore_with_a_lost_stack_is_unknown() {
        let items = vec![restore()];
        assert_eq!(run(&items, lost(entry())), lost(Level::Unknown));
        // A save on a lost stack records nothing and the stack stays lost.
        let items = vec![save()];
        assert_eq!(run(&items, lost(entry())), lost(entry()));
    }

    #[test]
    fn a_call_changes_nothing() {
        let from = st(known(LevelSet::only(7)), 2, vec![sv(0, entry())]);
        for m in ["jsr", "bsr", "jbsr"] {
            let items = vec![ins(0, m, None, vec![O::Sym("Foo".to_string())])];
            assert_eq!(run(&items, from.clone()), from, "{m}");
        }
    }

    #[test]
    fn meet_unions_levels() {
        let a = st(known(set(&[2])), 0, vec![]);
        let b = st(entry(), 0, vec![]);
        assert_eq!(a.meet(&b), st(known(set(&[2]).union(LevelSet::ENTRY)), 0, vec![]));
    }

    #[test]
    fn meet_of_different_depths_loses_the_stack() {
        let a = st(entry(), 0, vec![]);
        let b = st(entry(), 2, vec![]);
        assert_eq!(a.meet(&b), lost(entry()));
        // Same depth, different save positions: lost too.
        let a = st(entry(), 4, vec![sv(0, entry())]);
        let b = st(entry(), 4, vec![sv(2, entry())]);
        assert_eq!(a.meet(&b), lost(entry()));
        // Same depth, different save counts: lost.
        let a = st(entry(), 4, vec![sv(0, entry())]);
        let b = st(entry(), 4, vec![]);
        assert_eq!(a.meet(&b), lost(entry()));
    }

    #[test]
    fn meet_of_equal_stacks_combines_saved_levels() {
        let a = st(entry(), 4, vec![sv(0, entry()), sv(2, known(LevelSet::only(7)))]);
        let b = st(entry(), 4, vec![sv(0, known(LevelSet::only(3))), sv(2, Level::Unknown)]);
        assert_eq!(
            a.meet(&b),
            st(entry(), 4, vec![sv(0, known(LevelSet::only(3).union(LevelSet::ENTRY))), sv(2, Level::Unknown)])
        );
    }

    #[test]
    fn meet_with_unknown_is_unknown() {
        let a = st(Level::Unknown, 0, vec![]);
        let b = st(known(LevelSet::only(7)), 0, vec![]);
        assert_eq!(a.meet(&b), st(Level::Unknown, 0, vec![]));
        assert_eq!(b.meet(&a), st(Level::Unknown, 0, vec![]));
        // Both parts absorb at once: unknown level, lost stack.
        assert_eq!(lost(entry()).meet(&a), lost(Level::Unknown));
        // A save holding an unknown level absorbs pairwise.
        let x = st(entry(), 2, vec![sv(0, Level::Unknown)]);
        let y = st(entry(), 2, vec![sv(0, known(LevelSet::only(4)))]);
        assert_eq!(x.meet(&y), x);
    }

    fn sample() -> Vec<SrState> {
        vec![
            SrState::at_entry(),
            st(known(LevelSet::only(7)), 0, vec![]),
            st(Level::Unknown, 0, vec![]),
            st(entry(), 2, vec![sv(0, entry())]),
            st(known(set(&[3, 5])), 2, vec![sv(0, known(LevelSet::only(7)))]),
            st(entry(), 2, vec![sv(0, Level::Unknown)]),
            lost(known(LevelSet::only(0))),
            st(known(set(&[1]).union(LevelSet::ENTRY)), 4, vec![sv(0, entry()), sv(2, entry())]),
        ]
    }

    #[test]
    fn meet_is_commutative_associative_and_idempotent_on_a_sample() {
        let s = sample();
        for a in &s {
            assert_eq!(a.meet(a), *a, "idempotent {a:?}");
            for b in &s {
                assert_eq!(a.meet(b), b.meet(a), "commutative {a:?} {b:?}");
                for c in &s {
                    assert_eq!(a.meet(b).meet(c), a.meet(&b.meet(c)), "associative {a:?} {b:?} {c:?}");
                }
            }
        }
    }

    #[test]
    fn a_restore_at_an_odd_depth_that_cuts_a_save_loses_the_stack() {
        let items = vec![save(), addq_sp(1), restore()];
        assert_eq!(run(&items, SrState::at_entry()), lost(Level::Unknown));
        // The mismatched restore's own Drop(2) cutting a save: `move.w d0,-(sp)`,
        // save, `addq.l #1,sp` already loses it; from a hand-built odd-depth
        // state (depth 3, save at 0) the restore's Drop(2) lands at 1, inside it.
        let items = vec![restore()];
        assert_eq!(run(&items, st(entry(), 3, vec![sv(0, entry())])), lost(Level::Unknown));
    }

    #[test]
    fn a_push_in_a_loop_settles() {
        let items = vec![
            label(".top"),
            push_d0(),
            ins(1, "nop", None, vec![]),
            ins(2, "bne", None, vec![O::Sym(".top".to_string())]),
            ins(3, "rts", None, vec![]),
        ];
        let cfg = Cfg::build(&items);
        let head = 1;
        let got = forward_in_states(
            &cfg,
            Cpu::M68000,
            &[(head, SrState::at_entry())],
            &|s: &SrState, idx, m, ops| step(&items, s, idx, m, ops),
            &|_| true,
        );
        assert_eq!(got.get(&head), Some(&lost(entry())));
        assert_eq!(got.get(&4), Some(&lost(entry())));
    }

    // ---- mask_findings (Task 3) ----

    fn sym(n: &str) -> CodeOperand {
        O::Sym(n.to_string())
    }
    fn op(mnem: &str, ops: Vec<CodeOperand>) -> CodeItem {
        ins(0, mnem, None, ops)
    }
    fn exported(name: &str) -> CodeItem {
        CodeItem::Label { name: name.to_string(), export: true, span: Span { source: SourceId(0), start: 0, end: 0 } }
    }
    fn tst_d0() -> CodeItem {
        ins(0, "tst", W, vec![O::Reg(Reg::D0)])
    }
    fn by_assert(it: CodeItem) -> CodeItem {
        match it {
            CodeItem::Instr { mnemonic, size, ops, span, as_type, targets, .. } => CodeItem::Instr {
                mnemonic,
                size,
                ops,
                span,
                as_type,
                targets,
                author: ItemAuthor::AssertDesugar,
            },
            other => other,
        }
    }
    /// Give every instruction the span `idx..idx + 1`, so a finding's span names
    /// its item.
    fn numbered(items: Vec<CodeItem>) -> Vec<CodeItem> {
        items
            .into_iter()
            .enumerate()
            .map(|(i, it)| match it {
                CodeItem::Instr { mnemonic, size, ops, as_type, targets, author, .. } => CodeItem::Instr {
                    mnemonic,
                    size,
                    ops,
                    span: span_of(i),
                    as_type,
                    targets,
                    author,
                },
                other => other,
            })
            .collect()
    }
    fn span_of(i: usize) -> Span {
        Span { source: SourceId(0), start: i as u32, end: i as u32 + 1 }
    }
    fn found_with(items: Vec<CodeItem>, noreturn: &[&str]) -> Vec<(usize, MaskFindingKind)> {
        let items = numbered(items);
        let noreturn: BTreeSet<String> = noreturn.iter().map(|s| s.to_string()).collect();
        let got = mask_findings(&items, &noreturn);
        for f in &got {
            assert_eq!(f.span, span_of(f.idx), "a finding carries its own instruction's span");
        }
        got.into_iter().map(|f| (f.idx, f.kind)).collect()
    }
    fn found(items: Vec<CodeItem>) -> Vec<(usize, MaskFindingKind)> {
        found_with(items, &[])
    }
    fn not_callers(levels: &[u8]) -> MaskFindingKind {
        MaskFindingKind::NotTheCallers { levels: levels.to_vec() }
    }

    #[test]
    fn the_w2_shape_is_found() {
        let items = vec![
            save(),                         // 0
            set_sr(0x2700),                 // 1
            tst_d0(),                       // 2
            op("beq", vec![sym(".full")]),  // 3
            addq_sp(2),                     // 4
            op("rts", vec![]),              // 5
            label(".full"),                 // 6
            restore(),                      // 7
            op("rts", vec![]),              // 8
        ];
        assert_eq!(found(items), vec![(5, not_callers(&[7]))]);
    }

    #[test]
    fn both_paths_restoring_is_clean() {
        let items = vec![
            save(),
            set_sr(0x2700),
            tst_d0(),
            op("beq", vec![sym(".a")]),
            restore(),
            op("rts", vec![]),
            label(".a"),
            restore(),
            op("rts", vec![]),
        ];
        assert_eq!(found(items), vec![]);
    }

    #[test]
    fn a_routine_that_never_touches_sr_is_clean() {
        let items = vec![
            ins(0, "move", W, vec![O::Reg(Reg::D0), O::Reg(Reg::D1)]),
            tst_d0(),
            op("beq", vec![sym(".x")]),
            op("nop", vec![]),
            label(".x"),
            op("rts", vec![]),
        ];
        assert_eq!(found(items), vec![]);
    }

    #[test]
    fn a_remask_in_a_loop_under_one_save_is_clean() {
        let items = vec![
            save(),
            label(".loop"),
            set_sr(0x2700),
            tst_d0(),
            op("bne", vec![sym(".loop")]),
            restore(),
            op("rts", vec![]),
        ];
        assert_eq!(found(items), vec![]);
    }

    #[test]
    fn an_early_exit_before_the_save_is_clean() {
        let items = vec![
            tst_d0(),
            op("beq", vec![sym(".done")]),
            save(),
            set_sr(0x2700),
            op("nop", vec![]),
            restore(),
            label(".done"),
            op("rts", vec![]),
        ];
        assert_eq!(found(items), vec![]);
    }

    #[test]
    fn an_export_label_before_the_save_is_an_entry() {
        let items = vec![
            tst_d0(),
            op("beq", vec![sym(".out")]),
            exported("Owner.transfer"),
            save(),
            set_sr(0x2700),
            op("nop", vec![]),
            restore(),
            label(".out"),
            op("rts", vec![]),
        ];
        assert_eq!(found(items), vec![]);
    }

    #[test]
    fn an_export_label_after_the_save_is_refused() {
        let items = vec![
            save(),                     // 0
            set_sr(0x2700),             // 1
            exported("Owner.mid"),      // 2
            restore(),                  // 3
            op("rts", vec![]),          // 4
        ];
        // An entrant at `Owner.mid` pops a word it never pushed: the restore's
        // level is unknown.
        assert_eq!(found(items), vec![(4, MaskFindingKind::Untracked)]);
    }

    #[test]
    fn a_local_helper_reached_only_by_bsr_is_walked() {
        let items = vec![
            op("bsr", vec![sym(".h")]), // 0
            op("rts", vec![]),          // 1
            label(".h"),                // 2
            set_sr(0x2700),             // 3
            op("rts", vec![]),          // 4
        ];
        // Piece 2b propagates `.h`'s `{7}` summary back into the caller's `rts` (1) as well.
        assert_eq!(found(items), vec![(1, not_callers(&[7])), (4, not_callers(&[7]))]);
    }

    #[test]
    fn a_register_restore_is_untracked() {
        let items = vec![
            ins(0, "move", W, vec![O::Sr, O::Reg(Reg::D1)]), // 0
            set_sr(0x2700),                                   // 1
            ins(0, "move", W, vec![O::Reg(Reg::D1), O::Sr]),  // 2
            op("rts", vec![]),                                // 3
        ];
        assert_eq!(found(items), vec![(3, MaskFindingKind::Untracked)]);
    }

    #[test]
    fn an_rte_is_its_own_finding() {
        assert_eq!(found(vec![op("rte", vec![])]), vec![(0, MaskFindingKind::Rte)]);
        // Masked or not, an `rte` is judged as an `rte`.
        let items = vec![save(), set_sr(0x2700), op("rte", vec![])];
        assert_eq!(found(items), vec![(2, MaskFindingKind::Rte)]);
    }

    #[test]
    fn a_diverging_rail_inside_a_masked_section_is_not_a_finding() {
        let items = vec![
            save(),
            set_sr(0x2700),
            save(),
            ins(0, "cmp", W, vec![O::Reg(Reg::D0), O::Reg(Reg::D1)]),
            op("beq", vec![sym(".ok")]),
            ins(0, "subq", L, vec![O::Imm(2), O::Reg(SP)]),
            ins(0, "move", W, vec![O::Reg(Reg::D0), O::DispInd { disp: 1, reg: SP }]),
            by_assert(op("jmp", vec![sym("AssertFail")])),
            label(".ok"),
            restore(),
            restore(),
            op("rts", vec![]),
        ];
        assert_eq!(found(items), vec![]);
    }

    #[test]
    fn a_masked_branch_to_the_closing_label_is_found() {
        let items = vec![
            save(),                         // 0
            set_sr(0x2700),                 // 1
            tst_d0(),                       // 2
            op("beq", vec![sym(".full")]),  // 3
            addq_sp(2),                     // 4
            op("bra", vec![sym(".end")]),   // 5
            label(".full"),                 // 6
            restore(),                      // 7
            op("rts", vec![]),              // 8
            label(".end"),                  // 9
        ];
        assert_eq!(found(items), vec![(5, not_callers(&[7]))]);
    }

    fn lea_pc(target: &str) -> CodeItem {
        ins(0, "lea", L, vec![O::PcRel { target: target.to_string(), addend: 0 }, O::Reg(Reg::A0)])
    }

    #[test]
    fn a_masking_helper_entered_through_a_pointer_is_found_on_its_own_return() {
        let items = vec![
            lea_pc(".h"),                     // 0
            op("jsr", vec![O::Ind(Reg::A0)]), // 1
            op("rts", vec![]),                // 2
            label(".h"),                      // 3
            set_sr(0x2700),                   // 4
            op("rts", vec![]),                // 5
        ];
        assert_eq!(found(items), vec![(5, not_callers(&[7]))]);
    }

    #[test]
    fn a_balanced_helper_entered_only_through_a_pointer_is_clean() {
        let items = vec![
            lea_pc(".h"),                     // 0
            op("jsr", vec![O::Ind(Reg::A0)]), // 1
            op("rts", vec![]),                // 2
            label(".h"),                      // 3
            save(),                           // 4
            set_sr(0x2700),                   // 5
            restore(),                        // 6
            op("rts", vec![]),                // 7
        ];
        assert_eq!(found(items), vec![]);
    }

    /// The review's I1 shape (probe `c02_ptr_entry_feeds_restore`): a block
    /// reached only through `jmp (a0)` pushes `$2700` and joins the shared
    /// restore, which then pops the pushed word, not the save.
    #[test]
    fn a_pointer_entered_block_feeding_the_restore_is_found() {
        let items = vec![
            tst_d0(),                                               // 0
            op("beq", vec![sym(".via")]),                           // 1
            save(),                                                 // 2
            set_sr(0x2700),                                         // 3
            label(".r"),                                            // 4
            restore(),                                              // 5
            op("rts", vec![]),                                      // 6
            label(".via"),                                          // 7
            lea_pc(".b"),                                           // 8
            op("jmp", vec![O::Ind(Reg::A0)]),                       // 9
            label(".b"),                                            // 10
            ins(0, "move", W, vec![O::Imm(0x2700), O::PreDec(SP)]), // 11
            op("bra", vec![sym(".r")]),                             // 12
        ];
        assert_eq!(found(items), vec![(6, MaskFindingKind::Untracked)]);
    }

    /// The same, entered through `jmp .tbl(pc, d1.w) targets(.b)` (probe
    /// `c01_targets_entry_feeds_restore`; its `dc.w .b-.tbl` table is left out,
    /// data is not an instruction).
    #[test]
    fn a_targets_entered_block_feeding_the_restore_is_found() {
        let mut jmp = op("jmp", vec![O::PcRelIdx { target: ".tbl".to_string(), addend: 0, xn: Reg::D1, xlong: false }]);
        if let CodeItem::Instr { targets, .. } = &mut jmp {
            targets.push(".b".to_string());
        }
        let items = vec![
            tst_d0(),                                                // 0
            op("beq", vec![sym(".via")]),                            // 1
            save(),                                                  // 2
            set_sr(0x2700),                                          // 3
            label(".r"),                                             // 4
            restore(),                                               // 5
            op("rts", vec![]),                                       // 6
            label(".via"),                                           // 7
            ins(0, "moveq", None, vec![O::Imm(0), O::Reg(Reg::D1)]), // 8
            jmp,                                                     // 9
            label(".tbl"),                                           // 10
            label(".b"),                                             // 11
            ins(0, "move", W, vec![O::Imm(0x2700), O::PreDec(SP)]),  // 12
            op("bra", vec![sym(".r")]),                              // 13
        ];
        assert_eq!(found(items), vec![(6, MaskFindingKind::Untracked), (9, MaskFindingKind::Untracked)]);
    }

    /// Every instruction is walked: an unreached `stop` then `rte` is judged as
    /// an `rte`, and the unreached `ori` after it (a second extra entry) falls
    /// off the end at level 7.
    #[test]
    fn unreached_rte_stop_and_ori_are_walked() {
        let items = vec![
            op("rts", vec![]),                // 0
            op("stop", vec![O::Imm(0x2700)]), // 1
            op("rte", vec![]),                // 2
            logic_sr("ori", 0x0700),          // 3
        ];
        assert_eq!(found(items), vec![(2, MaskFindingKind::Rte), (3, not_callers(&[7]))]);
    }

    #[test]
    fn an_unreached_rte_under_a_mask_is_still_an_rte() {
        let items = vec![op("rts", vec![]), save(), set_sr(0x2700), op("rte", vec![])];
        assert_eq!(found(items), vec![(3, MaskFindingKind::Rte)]);
    }

    #[test]
    fn unreached_code_that_never_touches_the_level_or_the_stack_is_clean() {
        let items = vec![
            op("rts", vec![]),
            ins(0, "moveq", None, vec![O::Imm(0), O::Reg(Reg::D0)]),
            op("nop", vec![]),
            op("rts", vec![]),
        ];
        assert_eq!(found(items), vec![]);
    }

    /// The review's I3(a) probe (`s02_copy_top_over_save`): the pushed `$2700`
    /// is copied over the saved word, then "restored".
    #[test]
    fn copying_the_top_over_the_saved_word_is_found() {
        let items = vec![
            save(),                                                                          // 0
            ins(0, "move", W, vec![O::Imm(0x2700), O::PreDec(SP)]),                          // 1
            ins(0, "move", W, vec![O::Ind(SP), O::DispInd { disp: 2, reg: SP }]),            // 2
            addq_sp(2),                                                                      // 3
            restore(),                                                                       // 4
            op("rts", vec![]),                                                               // 5
        ];
        assert_eq!(found(items), vec![(5, MaskFindingKind::Untracked)]);
    }

    /// The review's I3(d) probe (`c03_cmpm_double_pop`): each `cmpm.w (sp)+,
    /// (sp)+` pops four bytes, so the restore loads the pushed `$2700`.
    #[test]
    fn a_double_pop_on_one_line_is_found() {
        let cmpm = || ins(0, "cmpm", W, vec![O::PostInc(SP), O::PostInc(SP)]);
        let items = vec![
            ins(0, "move", W, vec![O::Imm(0x2700), O::PreDec(SP)]), // 0
            save(),                                                 // 1
            push_d0(),                                              // 2
            cmpm(),                                                 // 3
            restore(),                                              // 4
            ins(0, "pea", L, vec![O::Ind(Reg::A0)]),                // 5
            cmpm(),                                                 // 6
            op("rts", vec![]),                                      // 7
        ];
        assert_eq!(found(items), vec![(7, MaskFindingKind::Untracked)]);
    }

    // ---- F1b: which labels are extra entries ----

    fn data() -> CodeItem {
        CodeItem::Inline(crate::value::DataBuf { cells: Vec::new(), size: 0 }, Span { source: SourceId(0), start: 0, end: 0 })
    }
    fn jmp_targets(table: &str, targets: &[&str]) -> CodeItem {
        let mut it = op("jmp", vec![O::PcRelIdx { target: table.to_string(), addend: 0, xn: Reg::D1, xlong: false }]);
        if let CodeItem::Instr { targets: t, .. } = &mut it {
            t.extend(targets.iter().map(|s| s.to_string()));
        }
        it
    }

    /// Probe x1: a pointer lands at `.mid`, after a dead save and mask. Entered
    /// there, the restore pops a word nobody saved.
    #[test]
    fn a_pointer_into_the_middle_of_dead_code_is_found() {
        let items = vec![
            lea_pc(".mid"),                   // 0
            op("jmp", vec![O::Ind(Reg::A0)]), // 1
            label(".top"),                    // 2
            save(),                           // 3
            set_sr(0x2700),                   // 4
            label(".mid"),                    // 5
            restore(),                        // 6
            op("rts", vec![]),                // 7
        ];
        assert_eq!(found(items), vec![(7, MaskFindingKind::Untracked)]);
    }

    /// Probe x2: the dead code before `.mid` only saves.
    #[test]
    fn a_pointer_past_a_dead_save_is_found() {
        let items = vec![
            lea_pc(".mid"),                   // 0
            op("jmp", vec![O::Ind(Reg::A0)]), // 1
            label(".top"),                    // 2
            save(),                           // 3
            label(".mid"),                    // 4
            restore(),                        // 5
            op("rts", vec![]),                // 6
        ];
        assert_eq!(found(items), vec![(6, MaskFindingKind::Untracked)]);
    }

    /// Probe x3: the same landing through `targets(.mid)`.
    #[test]
    fn a_targets_jump_into_the_middle_of_dead_code_is_found() {
        let items = vec![
            ins(0, "moveq", None, vec![O::Imm(0), O::Reg(Reg::D1)]), // 0
            jmp_targets(".tbl", &[".mid"]),                          // 1
            label(".tbl"),                                           // 2
            data(),                                                  // 3
            label(".top"),                                           // 4
            save(),                                                  // 5
            set_sr(0x2700),                                          // 6
            label(".mid"),                                           // 7
            restore(),                                               // 8
            op("rts", vec![]),                                       // 9
        ];
        assert_eq!(found(items), vec![(1, MaskFindingKind::Untracked), (9, MaskFindingKind::Untracked)]);
    }

    /// Probe x4: a pointer into a REACHED restore. The ordinary path arrives
    /// with the save on top, the pointer path with nothing: the stack is lost.
    #[test]
    fn a_pointer_into_a_reached_restore_is_found() {
        let items = vec![
            tst_d0(),                         // 0
            op("beq", vec![sym(".x")]),       // 1
            save(),                           // 2
            set_sr(0x2700),                   // 3
            label(".mid"),                    // 4
            restore(),                        // 5
            op("rts", vec![]),                // 6
            label(".x"),                      // 7
            lea_pc(".mid"),                   // 8
            op("jmp", vec![O::Ind(Reg::A0)]), // 9
        ];
        assert_eq!(found(items), vec![(6, MaskFindingKind::Untracked)]);
    }

    /// A block entered by a pointer in the middle of an unreached region, with its
    /// own save and restore after the label, and a dead prefix that touches
    /// neither the level nor the stack.
    #[test]
    fn a_balanced_block_entered_in_the_middle_of_dead_code_is_clean() {
        let items = vec![
            lea_pc(".mid"),                                          // 0
            op("jmp", vec![O::Ind(Reg::A0)]),                        // 1
            label(".top"),                                           // 2
            ins(0, "moveq", None, vec![O::Imm(0), O::Reg(Reg::D0)]), // 3
            label(".mid"),                                           // 4
            save(),                                                  // 5
            set_sr(0x2700),                                          // 6
            restore(),                                               // 7
            op("rts", vec![]),                                       // 8
        ];
        assert_eq!(found(items), vec![]);
    }

    /// `.tbl` names data. `label_index` would resolve it to the instruction after
    /// the data (the restore at `.r`); seeded there at the caller's level it
    /// would meet the masked path's save and lose the stack. Not an entry.
    #[test]
    fn a_data_label_whose_address_is_taken_is_not_an_entry() {
        let items = vec![
            save(),                     // 0
            set_sr(0x2700),             // 1
            lea_pc(".tbl"),             // 2
            op("bra", vec![sym(".r")]), // 3
            label(".tbl"),              // 4
            data(),                     // 5
            label(".r"),                // 6
            restore(),                  // 7
            op("rts", vec![]),          // 8
        ];
        assert_eq!(found(items), vec![]);
    }

    /// A masked `targets(...)` dispatch whose arms each restore, one arm also
    /// reached by an ordinary branch. Pinned as it is: the `jmp` leaves at 7 (the
    /// walk does not follow `targets`), and each arm, entered at the caller's
    /// level with nothing saved, pops a word it did not save. Recorded in
    /// `docs/QUEUE.md` STEPA-P2A-FALSE-REFUSALS.
    #[test]
    fn a_masked_targets_dispatch_whose_arms_restore_is_refused() {
        let items = vec![
            save(),                                                  // 0
            set_sr(0x2700),                                          // 1
            tst_d0(),                                                // 2
            op("beq", vec![sym(".b")]),                              // 3
            ins(0, "moveq", None, vec![O::Imm(0), O::Reg(Reg::D1)]), // 4
            jmp_targets(".tbl", &[".a", ".b"]),                      // 5
            label(".tbl"),                                           // 6
            data(),                                                  // 7
            label(".a"),                                             // 8
            restore(),                                               // 9
            op("rts", vec![]),                                       // 10
            label(".b"),                                             // 11
            restore(),                                               // 12
            op("rts", vec![]),                                       // 13
        ];
        assert_eq!(
            found(items),
            vec![(5, MaskFindingKind::Untracked), (10, MaskFindingKind::Untracked), (13, MaskFindingKind::Untracked)]
        );
    }

    /// Seed class (b) on its own: `.b` is reached by an ordinary branch with the
    /// save on top and is also a `targets` landing. Without the seed the
    /// restore at `.b` would be trusted.
    #[test]
    fn a_reached_targets_label_is_an_entry() {
        let items = vec![
            save(),                                                  // 0
            tst_d0(),                                                // 1
            op("beq", vec![sym(".b")]),                              // 2
            ins(0, "moveq", None, vec![O::Imm(0), O::Reg(Reg::D1)]), // 3
            jmp_targets(".tbl", &[".b"]),                            // 4
            label(".tbl"),                                           // 5
            data(),                                                  // 6
            label(".b"),                                             // 7
            restore(),                                               // 8
            op("rts", vec![]),                                       // 9
        ];
        assert_eq!(found(items), vec![(4, MaskFindingKind::Untracked), (9, MaskFindingKind::Untracked)]);
    }

    /// Seed class (a) on its own: `a0` comes from the caller, so nothing names
    /// `.mid`, but a label in code the first walk did not reach is an entry.
    #[test]
    fn a_label_in_unreached_code_is_an_entry() {
        let items = vec![
            op("jmp", vec![O::Ind(Reg::A0)]), // 0
            label(".top"),                    // 1
            save(),                           // 2
            label(".mid"),                    // 3
            restore(),                        // 4
            op("rts", vec![]),                // 5
        ];
        assert_eq!(found(items), vec![(5, MaskFindingKind::Untracked)]);
    }

    // ---- F1c: the offset-entry hazard seeds every instruction ----

    fn jmp_disp(mnem: &str, disp: i128, reg: Reg) -> CodeItem {
        op(mnem, vec![O::DispInd { disp, reg }])
    }
    fn lea_pc_plus(target: &str, addend: i64) -> CodeItem {
        ins(0, "lea", L, vec![O::PcRel { target: target.to_string(), addend }, O::Reg(Reg::A0)])
    }
    /// `.x: save; mask; restore; rts`, the body the y probes land inside.
    fn bracket_at_x() -> Vec<CodeItem> {
        vec![label(".x"), save(), set_sr(0x2700), restore(), op("rts", vec![])]
    }

    /// Probe y1: `jmp 6(a0)` after taking `.x`'s address lands on the restore.
    #[test]
    fn a_displaced_jump_after_taking_an_address_is_found() {
        let mut items = vec![lea_pc(".x"), jmp_disp("jmp", 6, Reg::A0)]; // 0, 1
        items.extend(bracket_at_x()); // 2 label, 3 save, 4 mask, 5 restore, 6 rts
        assert_eq!(found(items), vec![(6, MaskFindingKind::Untracked)]);
    }

    /// Probe y2: the offset is in the taken address, `lea .x+6(pc), a0`.
    #[test]
    fn an_address_with_an_offset_is_found() {
        let mut items = vec![lea_pc_plus(".x", 6), op("jmp", vec![O::Ind(Reg::A0)])]; // 0, 1
        items.extend(bracket_at_x()); // 2..6
        assert_eq!(found(items), vec![(6, MaskFindingKind::Untracked)]);
    }

    /// Probe y3: `jmp .x+6(pc)` on one path.
    #[test]
    fn a_pc_relative_jump_with_an_offset_is_found() {
        let mut items = vec![
            tst_d0(),                                                         // 0
            op("beq", vec![sym(".y")]),                                       // 1
            op("jmp", vec![O::PcRel { target: ".x".to_string(), addend: 6 }]), // 2
            label(".y"),                                                      // 3
            op("rts", vec![]),                                                // 4
        ];
        items.extend(bracket_at_x()); // 5 label, 6 save, 7 mask, 8 restore, 9 rts
        assert_eq!(found(items), vec![(2, MaskFindingKind::Untracked), (9, MaskFindingKind::Untracked)]);
    }

    #[test]
    fn a_hazard_routine_that_never_touches_the_level_or_the_stack_is_clean() {
        let items = vec![
            lea_pc(".x"),                                            // 0
            jmp_disp("jmp", 2, Reg::A0),                             // 1
            label(".x"),                                             // 2
            ins(0, "moveq", None, vec![O::Imm(0), O::Reg(Reg::D0)]), // 3
            op("nop", vec![]),                                       // 4
            op("rts", vec![]),                                       // 5
        ];
        assert_eq!(found(items), vec![]);
    }

    /// A taken address and a plain `jmp (a0)`: no hazard (F1b's label seed covers
    /// the landing), so a balanced block at the label stays clean.
    #[test]
    fn a_plain_indirect_jump_to_a_taken_label_has_no_hazard() {
        let mut items = vec![lea_pc(".x"), op("jmp", vec![O::Ind(Reg::A0)])];
        items.extend(bracket_at_x());
        assert_eq!(found(items), vec![]);
    }

    /// The object-dispatch shape: `jsr 4(a1)` in a masked, balanced routine that
    /// takes no local label's address. No hazard; clean.
    #[test]
    fn a_displaced_call_with_no_taken_address_has_no_hazard() {
        let items = vec![
            save(),                      // 0
            set_sr(0x2700),              // 1
            jmp_disp("jsr", 4, Reg::A1), // 2
            restore(),                   // 3
            op("rts", vec![]),           // 4
        ];
        assert_eq!(found(items), vec![]);
    }

    // ---- Re-review N2: a transfer that names a label in a non-`Sym` operand ----

    /// Probe b01: `jmp .r(pc)` straight into a reached restore. The CFG follows
    /// only a plain `Sym`, so the jump is a tail out and `.r` was no entry.
    #[test]
    fn a_pc_relative_jump_into_a_reached_restore_is_found() {
        let items = vec![
            tst_d0(),                                                          // 0
            op("beq", vec![sym(".via")]),                                      // 1
            save(),                                                            // 2
            set_sr(0x2700),                                                    // 3
            label(".r"),                                                       // 4
            restore(),                                                         // 5
            op("rts", vec![]),                                                 // 6
            label(".via"),                                                     // 7
            op("jmp", vec![O::PcRel { target: ".r".to_string(), addend: 0 }]), // 8
        ];
        assert_eq!(found(items), vec![(6, MaskFindingKind::Untracked), (8, MaskFindingKind::Untracked)]);
    }

    /// Probe r2: a balanced helper called as `jsr .h(pc)` stays clean.
    #[test]
    fn a_balanced_helper_called_pc_relative_is_clean() {
        let items = vec![
            op("jsr", vec![O::PcRel { target: ".h".to_string(), addend: 0 }]), // 0
            op("rts", vec![]),                                                 // 1
            label(".h"),                                                       // 2
            save(),                                                            // 3
            set_sr(0x2700),                                                    // 4
            restore(),                                                         // 5
            op("rts", vec![]),                                                 // 6
        ];
        assert_eq!(found(items), vec![]);
    }

    #[test]
    fn unreached_code_that_never_sets_the_level_is_not() {
        let items = vec![
            op("rts", vec![]),
            ins(0, "move", W, vec![O::Sr, O::Reg(Reg::D0)]),
            save(),
            ins(0, "move", W, vec![O::Imm(0), O::Ccr]),
            op("nop", vec![]),
            op("rtr", vec![]),
        ];
        assert_eq!(found(items), vec![]);
    }

    #[test]
    fn a_closing_export_label_and_an_empty_body_do_not_panic() {
        assert_eq!(found(vec![op("nop", vec![]), op("rts", vec![]), exported("Owner.end")]), vec![]);
        assert_eq!(found(vec![]), vec![]);
        assert_eq!(found(vec![exported("Owner.only")]), vec![]);
        assert_eq!(found(vec![label(".only")]), vec![]);
        // A call to a body-closing label: no instruction there, no seed.
        assert_eq!(found(vec![op("bsr", vec![sym(".end")]), op("rts", vec![]), label(".end")]), vec![]);
        // A label followed by another label: the seed is the instruction after both.
        let items = vec![
            op("bsr", vec![sym(".h")]), // 0
            op("rts", vec![]),          // 1
            label(".h"),                // 2
            exported("Owner.h"),        // 3
            set_sr(0x2700),             // 4
            op("rts", vec![]),          // 5
        ];
        assert_eq!(found(items), vec![(1, not_callers(&[7])), (5, not_callers(&[7]))]);
    }

    #[test]
    fn a_tail_out_while_masked_is_found() {
        let items = vec![save(), set_sr(0x2700), op("jmp", vec![sym("Other")])];
        assert_eq!(found(items), vec![(2, not_callers(&[7]))]);
    }

    #[test]
    fn a_tail_to_a_noreturn_target_while_masked_is_clean() {
        let items = vec![save(), set_sr(0x2700), op("jmp", vec![sym("Fatal")])];
        assert_eq!(found_with(items.clone(), &["Fatal"]), vec![]);
        // The same tail to a target that returns is found.
        assert_eq!(found_with(items, &["Elsewhere"]), vec![(2, not_callers(&[7]))]);
    }

    #[test]
    fn a_conditional_branch_out_while_masked_is_found() {
        let items = vec![
            save(),                          // 0
            set_sr(0x2700),                  // 1
            tst_d0(),                        // 2
            op("beq", vec![sym("Other")]),   // 3
            restore(),                       // 4
            op("rts", vec![]),               // 5
        ];
        assert_eq!(found(items), vec![(3, not_callers(&[7]))]);
    }

    #[test]
    fn a_balanced_stop_is_clean() {
        let items = vec![save(), op("stop", vec![O::Imm(0x2500)]), restore(), op("rts", vec![])];
        assert_eq!(found(items), vec![]);
    }

    #[test]
    fn a_return_is_judged_after_its_own_effect() {
        // A TailOut after the restore: clean; the mirror without it: found.
        let items = vec![save(), set_sr(0x2700), restore(), op("jmp", vec![sym("Other")])];
        assert_eq!(found(items), vec![]);
        let items = vec![save(), set_sr(0x2700), op("jmp", vec![sym("Other")])];
        assert_eq!(found(items), vec![(2, not_callers(&[7]))]);
        // One instruction that both writes `sr` and leaves: the body's last
        // instruction leaves by `FallOff`. Judged on its OUT state, not its IN.
        // The restore arrives at level 7 and leaves at the entry level: clean.
        let items = vec![save(), set_sr(0x2700), restore()];
        assert_eq!(found(items), vec![]);
        // The mask arrives at the entry level and leaves at 7: found.
        let items = vec![save(), set_sr(0x2700)];
        assert_eq!(found(items), vec![(1, not_callers(&[7]))]);
        // `rtr`'s Drop(2) loses the stack but not the level: still clean.
        assert_eq!(found(vec![op("rtr", vec![])]), vec![]);
        let items = vec![save(), set_sr(0x2700), restore(), op("rtr", vec![])];
        assert_eq!(found(items), vec![]);
    }

    #[test]
    fn a_known_level_with_the_entry_is_judged_by_its_fixed_levels() {
        // One path sets level 3, the other does not: {ENTRY, 3} at the return.
        let items = vec![
            tst_d0(),                       // 0
            op("beq", vec![sym(".r")]),     // 1
            set_sr(0x2300),                 // 2
            label(".r"),                    // 3
            op("rts", vec![]),              // 4
        ];
        assert_eq!(found(items), vec![(4, not_callers(&[3]))]);
    }

    #[test]
    fn one_finding_per_instruction_sorted_by_index() {
        let items = vec![
            op("bra", vec![sym(".go")]),     // 0
            set_sr(0x2700),                  // 1: reached only as an extra entry
            label(".go"),                    // 2
            set_sr(0x2700),                  // 3
            op("beq", vec![sym("Other")]),   // 4: BranchOut + Follow
            op("beq", vec![sym("Other2")]),  // 5: BranchOut + FallOff, one finding
        ];
        // The unreached `move` (1) is walked as an extra entry; its only edge
        // is a Follow, so it is not a finding itself.
        assert_eq!(found(items), vec![(4, not_callers(&[7])), (5, not_callers(&[7]))]);
    }

    #[test]
    fn a_known_set_of_all_nine_bits_is_not_untracked() {
        let all = (0u8..8).fold(LevelSet::ENTRY, |acc, l| acc.union(LevelSet::only(l)));
        assert_eq!(
            judge_level(Level::Known(all)),
            Some(MaskFindingKind::NotTheCallers { levels: vec![0, 1, 2, 3, 4, 5, 6, 7] })
        );
        assert_eq!(judge_level(Level::Unknown), Some(MaskFindingKind::Untracked));
    }

    /// save; level 7 (or not); an assert expansion that is the body's last
    /// statement: its `bcc` skips the rail to the closing label.
    fn body_ending_in_an_assert(masked: bool) -> Vec<CodeItem> {
        let mut items = vec![save()]; // 0
        if masked {
            items.push(set_sr(0x2700)); // 1
        } else {
            items.push(op("nop", vec![])); // 1
        }
        items.extend([
            restore_if(!masked),                                         // 2
            by_assert(op("bcc", vec![sym(".ok")])),                      // 3: FallOff + Follow
            by_assert(ins(0, "move", W, vec![O::Reg(Reg::D0), O::PreDec(SP)])), // 4
            by_assert(op("jmp", vec![sym("AssertFail")])),               // 5: TailOut
            label(".ok"),                                                // 6
        ]);
        items
    }
    /// The restore when `yes`, else a `nop` holding its place.
    fn restore_if(yes: bool) -> CodeItem {
        if yes {
            restore()
        } else {
            op("nop", vec![])
        }
    }

    #[test]
    fn a_masked_body_ending_in_a_passing_assert_is_found() {
        assert_eq!(found(body_ending_in_an_assert(true)), vec![(3, not_callers(&[7]))]);
    }

    #[test]
    fn an_unmasked_body_ending_in_a_passing_assert_is_clean() {
        assert_eq!(found(body_ending_in_an_assert(false)), vec![]);
    }

    #[test]
    fn a_masked_last_branch_to_a_noreturn_target_still_judges_the_fall_through() {
        let items = vec![
            save(),                         // 0
            set_sr(0x2700),                 // 1
            op("beq", vec![sym("Fatal")]),  // 2: BranchOut (diverges) + FallOff
        ];
        assert_eq!(found_with(items, &["Fatal"]), vec![(2, not_callers(&[7]))]);
    }

    #[test]
    fn compose_level_is_associative_with_entry_identity_and_overwrites_unknown() {
        let entry = Level::Known(LevelSet::ENTRY);
        let l3 = Level::Known(LevelSet::only(3));
        let l7 = Level::Known(LevelSet::only(7));
        let e_or_7 = Level::Known(LevelSet::ENTRY.union(LevelSet::only(7)));
        let sample = [entry, l3, l7, e_or_7, Level::Unknown];

        for a in sample {
            assert_eq!(compose_level(a, entry), a);
            assert_eq!(compose_level(entry, a), a);
            for b in sample {
                for c in sample {
                    assert_eq!(
                        compose_level(compose_level(a, b), c),
                        compose_level(a, compose_level(b, c)),
                        "associativity failed on ({a:?}, {b:?}, {c:?})"
                    );
                }
            }
        }
        // Unconditional callee setting level 7 overrides even Unknown caller level.
        assert_eq!(compose_level(Level::Unknown, l7), l7);
        // Conditional callee preserving entry or setting 7 keeps Unknown Unknown.
        assert_eq!(compose_level(Level::Unknown, e_or_7), Level::Unknown);
        assert_eq!(compose_level(l3, e_or_7), Level::Known(LevelSet::only(3).union(LevelSet::only(7))));
    }

    #[test]
    fn apply_summary_guards_stack_poison_on_non_empty_saves_and_handles_divergence() {
        let mut st_no_saves = SrState::at_entry();
        st_no_saves.stack = Stack::Known { depth: 4, saves: vec![] };
        let stack_corrupt = ProcSummary {
            exit_level: Some(Level::Known(LevelSet::ENTRY)),
            body_levels: Some(Level::Known(LevelSet::ENTRY)),
            touches_caller_stack: true,
        };
        // No saved SR on stack: stack stays Known (C1 fix).
        let after = apply_summary(&st_no_saves, stack_corrupt).expect("returns");
        assert_eq!(after.stack, st_no_saves.stack);

        // With a saved SR on stack: stack is poisoned to Lost.
        let st_saved = SrState {
            level: Level::Known(LevelSet::only(7)),
            stack: Stack::Known { depth: 2, saves: vec![Save { below: 0, level: Level::Known(LevelSet::ENTRY) }] },
        };
        let after_saved = apply_summary(&st_saved, stack_corrupt).expect("returns");
        assert_eq!(after_saved.stack, Stack::Lost);

        // Divergent callee returns None (drops fall-through).
        assert_eq!(apply_summary(&st_saved, ProcSummary::DIVERGES), None);
    }

    #[test]
    fn mutually_recursive_local_helpers_converge_and_detect_mask_leak() {
        // .a calls .b or returns; .b sets level 7 and calls .a or returns.
        let items = vec![
            op("bsr", vec![sym(".a")]), // 0
            op("rts", vec![]),          // 1
            label(".a"),                // 2
            tst_d0(),                   // 3
            op("beq", vec![sym(".a_ret")]), // 4
            op("bsr", vec![sym(".b")]), // 5
            label(".a_ret"),            // 6
            op("rts", vec![]),          // 7
            label(".b"),                // 8
            set_sr(0x2700),             // 9
            tst_d0(),                   // 10
            op("beq", vec![sym(".b_ret")]), // 11
            op("bsr", vec![sym(".a")]), // 12
            label(".b_ret"),            // 13
            op("rts", vec![]),          // 14
        ];
        assert_eq!(
            found(items),
            vec![
                (1, not_callers(&[7])),
                (7, not_callers(&[7])),
                (14, not_callers(&[7])),
            ]
        );
    }
}
