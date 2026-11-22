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
    fn meet(self, o: Level) -> Level {
        match (self, o) {
            (Level::Known(a), Level::Known(b)) => Level::Known(a.union(b)),
            _ => Level::Unknown,
        }
    }

    fn is_well_formed(self) -> bool {
        match self {
            Level::Known(s) => s.is_non_empty(),
            Level::Unknown => true,
        }
    }
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

/// What a call does to the level. 2a: nothing. Piece 2b changes this one place.
fn call_effect(level: Level) -> Level {
    level
}

fn is_call(mnem: &str) -> bool {
    matches!(mnem, "jsr" | "bsr" | "jbsr")
}

/// Does this instruction set the level? `stop #imm` loads `sr` from its
/// immediate; every other write to `sr` has `sr` as its last operand (`sr` can
/// only be a destination in the 68000's write forms: `move`, `ori`, `andi`,
/// `eori`). [`step`] and the unreached pass of [`mask_findings`] both ask this.
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
    /// The level cannot be followed to this way out. `lost_at`: the lowest-index
    /// reached instruction, not authored `AssertDesugar`, whose arriving level is
    /// known and leaving level unknown, or whose arriving stack is known and
    /// leaving stack lost. Its index, mnemonic and span.
    Untracked { lost_at: Option<(usize, String, Span)> },
    /// The routine leaves by `rte`, which loads the level from the stack.
    Rte,
    /// An instruction that sets the level (`sr` as last operand of a writing
    /// mnemonic, `stop`, `rte`) that the walk never reached. The walk cannot
    /// vouch for it: it may be entered through a pointer or a `targets(...)` jump.
    Unreached,
}

/// One way out (or one unreached level-setting instruction) and why.
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
/// leaving edge of each reached instruction on that instruction's OUT state.
/// Findings are read from the walk's result, never recorded inside `step`.
/// The stack is not judged here (the stack-balance check owns that); a lost
/// stack matters only when it makes a restore unknown.
pub(crate) fn mask_findings(items: &[CodeItem], noreturn: &BTreeSet<String>) -> Vec<MaskFinding> {
    let cfg = Cfg::build(items);
    let seeds = entry_seeds(items, &cfg);
    let states = forward_in_states(
        &cfg,
        Cpu::M68000,
        &seeds,
        &|st: &SrState, idx, mnem, ops| step(items, st, idx, mnem, ops),
        &|_| true,
    );
    let lost_at = loss_point(items, &states);

    let mut found: BTreeMap<usize, MaskFinding> = BTreeMap::new();
    for (&idx, st_in) in &states {
        let CodeItem::Instr { mnemonic, ops, span, .. } = &items[idx] else { continue };
        let out = step(items, st_in, idx, mnemonic, ops);
        for edge in edges_for(&cfg, Cpu::M68000, idx) {
            let kind = match edge {
                Edge::Follow(_) => None,
                Edge::Return if mnemonic == "rte" => Some(MaskFindingKind::Rte),
                Edge::Return => judge_level(out.level, &lost_at),
                // A transfer out: skipped when it diverges (an assert rail, a
                // noreturn target), as the stack check's `ExitKind::Defer` does.
                Edge::TailOut | Edge::BranchOut => {
                    if exit_diverges(&items[idx], noreturn) {
                        None
                    } else {
                        judge_level(out.level, &lost_at)
                    }
                }
                // Running off the end is an ordinary way out and never diverges.
                // `exit_diverges` judges the whole instruction, not the edge: it
                // would drop a passing assert's `bcc` to the closing label, or the
                // fall-through of a last `beq Fatal`.
                Edge::FallOff => judge_level(out.level, &lost_at),
            };
            // Several leaving edges of one instruction are judged on the one OUT
            // state, so their findings are equal: keep the first.
            if let Some(kind) = kind {
                found.entry(idx).or_insert(MaskFinding { idx, span: *span, kind });
            }
        }
    }

    // The unreached pass: a level-setting instruction the walk never reached.
    for (idx, item) in items.iter().enumerate() {
        let CodeItem::Instr { mnemonic, ops, span, .. } = item else { continue };
        if !states.contains_key(&idx) && (sets_level(mnemonic, ops) || mnemonic == "rte") {
            found.insert(idx, MaskFinding { idx, span: *span, kind: MaskFindingKind::Unreached });
        }
    }
    found.into_values().collect()
}

/// Every place a routine can be entered, each at [`SrState::at_entry`]: the
/// first instruction; the instruction at every exported label; the instruction
/// at every local label a `jsr`/`bsr`/`jbsr` names (a local helper has its own
/// caller and its own return, and the CFG's call edge does not enter it).
/// "The instruction at a label" is [`Cfg::label_index`]: the first instruction
/// at or after the label, so a label followed by another label resolves to the
/// instruction after both, and a body-closing label is `None` and seeds nothing.
/// De-duplicated: one seed per instruction.
fn entry_seeds(items: &[CodeItem], cfg: &Cfg) -> Vec<(usize, SrState)> {
    let mut at: BTreeSet<usize> = BTreeSet::new();
    if let Some(first) = items.iter().position(|it| matches!(it, CodeItem::Instr { .. })) {
        at.insert(first);
    }
    for item in items {
        match item {
            CodeItem::Label { name, export: true, .. } => at.extend(cfg.label_index(name)),
            CodeItem::Instr { mnemonic, ops, .. } if is_call(mnemonic) => {
                for op in ops {
                    if let CodeOperand::Sym(name) = op {
                        at.extend(cfg.label_index(name));
                    }
                }
            }
            _ => {}
        }
    }
    at.into_iter().map(|idx| (idx, SrState::at_entry())).collect()
}

/// The lowest-index reached instruction, not authored `AssertDesugar`, whose
/// arriving level is known and leaving level unknown, or whose arriving stack
/// is known and leaving stack lost. `None` when there is none.
fn loss_point(items: &[CodeItem], states: &BTreeMap<usize, SrState>) -> Option<(usize, String, Span)> {
    states.iter().find_map(|(&idx, st_in)| {
        let CodeItem::Instr { mnemonic, ops, span, author, .. } = &items[idx] else { return None };
        if matches!(author, ItemAuthor::AssertDesugar) {
            return None;
        }
        let out = step(items, st_in, idx, mnemonic, ops);
        let level_lost = matches!(st_in.level, Level::Known(_)) && out.level == Level::Unknown;
        let stack_lost = matches!(st_in.stack, Stack::Known { .. }) && out.stack == Stack::Lost;
        (level_lost || stack_lost).then(|| (idx, mnemonic.clone(), *span))
    })
}

/// The verdict on the level a way out leaves at.
fn judge_level(level: Level, lost_at: &Option<(usize, String, Span)>) -> Option<MaskFindingKind> {
    match level {
        Level::Known(s) if s.is_entry_only() => None,
        Level::Known(s) => Some(MaskFindingKind::NotTheCallers { levels: s.fixed_levels() }),
        Level::Unknown => Some(MaskFindingKind::Untracked { lost_at: lost_at.clone() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Width;
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
        // level is unknown, and it is the place the level was lost.
        assert_eq!(
            found(items),
            vec![(4, MaskFindingKind::Untracked { lost_at: Some((3, "move".to_string(), span_of(3))) })]
        );
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
        assert_eq!(found(items), vec![(4, not_callers(&[7]))]);
    }

    #[test]
    fn a_register_restore_is_untracked_and_names_where_it_was_lost() {
        let items = vec![
            ins(0, "move", W, vec![O::Sr, O::Reg(Reg::D1)]), // 0
            set_sr(0x2700),                                   // 1
            ins(0, "move", W, vec![O::Reg(Reg::D1), O::Sr]),  // 2
            op("rts", vec![]),                                // 3
        ];
        assert_eq!(
            found(items),
            vec![(3, MaskFindingKind::Untracked { lost_at: Some((2, "move".to_string(), span_of(2))) })]
        );
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

    #[test]
    fn a_helper_entered_through_a_pointer_is_unreached() {
        let items = vec![
            ins(0, "lea", L, vec![O::PcRel { target: ".h".to_string(), addend: 0 }, O::Reg(Reg::A0)]), // 0
            op("jsr", vec![O::Ind(Reg::A0)]), // 1
            op("rts", vec![]),                // 2
            label(".h"),                      // 3
            set_sr(0x2700),                   // 4
            op("rts", vec![]),                // 5
        ];
        assert_eq!(found(items), vec![(4, MaskFindingKind::Unreached)]);
    }

    #[test]
    fn an_unreached_rte_and_stop_are_findings() {
        let items = vec![
            op("rts", vec![]),
            op("stop", vec![O::Imm(0x2700)]),
            op("rte", vec![]),
            logic_sr("ori", 0x0700),
        ];
        assert_eq!(
            found(items),
            vec![
                (1, MaskFindingKind::Unreached),
                (2, MaskFindingKind::Unreached),
                (3, MaskFindingKind::Unreached),
            ]
        );
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
        assert_eq!(found(items), vec![(5, not_callers(&[7]))]);
    }

    #[test]
    fn the_loss_point_skips_assert_lines() {
        let items = vec![
            save(),                                                        // 0
            set_sr(0x2700),                                                // 1
            save(),                                                        // 2
            ins(0, "cmp", W, vec![O::Reg(Reg::D0), O::Reg(Reg::D1)]),      // 3
            op("beq", vec![sym(".ok")]),                                   // 4
            by_assert(ins(0, "subq", L, vec![O::Imm(2), O::Reg(SP)])),     // 5: stack known → lost
            by_assert(op("jmp", vec![sym("AssertFail")])),                 // 6
            label(".ok"),                                                  // 7
            restore(),                                                     // 8
            restore(),                                                     // 9
            ins(0, "move", W, vec![O::Reg(Reg::D1), O::Sr]),               // 10
            op("rts", vec![]),                                             // 11
        ];
        assert_eq!(
            found(items),
            vec![(11, MaskFindingKind::Untracked { lost_at: Some((10, "move".to_string(), span_of(10))) })]
        );
        // When the only loss is on an assert line, there is no place to name.
        let items = vec![by_assert(ins(0, "move", W, vec![O::Reg(Reg::D1), O::Sr])), op("rts", vec![])];
        assert_eq!(found(items), vec![(1, MaskFindingKind::Untracked { lost_at: None })]);
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
            set_sr(0x2700),                  // 1: unreached
            label(".go"),                    // 2
            set_sr(0x2700),                  // 3
            op("beq", vec![sym("Other")]),   // 4: BranchOut + Follow
            op("beq", vec![sym("Other2")]),  // 5: BranchOut + FallOff, one finding
        ];
        assert_eq!(
            found(items),
            vec![(1, MaskFindingKind::Unreached), (4, not_callers(&[7])), (5, not_callers(&[7]))]
        );
    }

    #[test]
    fn a_known_set_of_all_nine_bits_is_not_untracked() {
        let all = (0u8..8).fold(LevelSet::ENTRY, |acc, l| acc.union(LevelSet::only(l)));
        assert_eq!(
            judge_level(Level::Known(all), &None),
            Some(MaskFindingKind::NotTheCallers { levels: vec![0, 1, 2, 3, 4, 5, 6, 7] })
        );
        assert_eq!(judge_level(Level::Unknown, &None), Some(MaskFindingKind::Untracked { lost_at: None }));
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
}
