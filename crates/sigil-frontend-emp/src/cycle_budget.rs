//! Cycle budgets — `@budget(cycles: N)` and `@cycles_exact` (contract
//! unification §4-cycles / P5, delta spec §4).
//!
//! Two conclusions off ONE walk of a proc's evaluated `CodeBuf`:
//!
//!   * `@budget(cycles: N)` — the WORST-CASE path from proc entry to a return
//!     must cost at most `N`. `[cycles.over-budget]`.
//!   * `@cycles_exact` — every path must cost the SAME, which is the machine-
//!     checked form of a hand-counted pad (the driver's `nop`/`jr` fillers).
//!     `[cycles.path-mismatch]`.
//!
//! ## The model, and what it refuses
//!
//! Per-instruction cost comes from the CPU's one cost table —
//! [`crate::z80_cycles::instr_cost`] (shared with the `cycles(L1, L2)` builtin)
//! for the Z80, [`crate::m68k_cycles::instr_cost`] over the
//! `sigil_isa::m68k_cycles` tables for the 68000. Control flow comes from the
//! shared [`Cfg`], so this walk sees the same edges every other per-proc
//! analysis does.
//!
//! An exact worst-case cost is only definable over a FINITE, LOCAL, TOTALLY
//! MODELED path set, so everything outside that is a loud refusal rather than a
//! guess — the same stance the cost tables themselves take toward an op they do
//! not enumerate. A budget is a claim its author opted into, and the honest
//! downgrade for a proc that cannot carry one is free: delete the attribute.
//!
//! | shape | diagnostic | why it cannot be bounded |
//! |---|---|---|
//! | a back edge (any loop) | `[cycles.unbounded-loop]` | the longest path through a cycle is unbounded; no trip count is declared |
//! | a call (`call`/`rst`; `jsr`/`bsr`/`jbsr`) | `[cycles.opaque-call]` | the callee's cost is not a local fact |
//! | a tail transfer out, or control off the end of the body | `[cycles.unbounded-transfer]` | the path continues into code this walk cannot see |
//! | a transfer to a COMPUTED target (`jp (hl)`, `jmp .table(a1)`) | `[cycles.computed-transfer]` | the destination set is data, not structure — UNLESS a `targets(...)` clause enumerates the reachable local labels (see below) |
//! | an op outside the CPU's cost table | `[cycles.unknown-op]` | no cost is assignable |
//! | an outcome-split conditional whose two edges cannot be told apart | `[cycles.ambiguous-branch]` | the two costs cannot be routed to their edges (a defensive guard with no input from THIS walk — see [`BudgetFindingKind::AmbiguousBranch`]; the id's live producer is the `cycles(L1, L2)` span builtin) |
//! | inline data in the code stream | `[cycles.inline-data]` | those bytes DECODE if control reaches them, and the CFG does not model them as instructions |
//! | a body with no instructions | `[cycles.empty-body]` | its one path never returns, so there is no path cost to bound |
//! | `@cycles_exact` over an instruction whose cost is a CEILING | `[cycles.inexact-cost]` | a maximum can bound a budget but cannot prove an equality |
//!
//! On the 68000 some charged costs are MAXIMA rather than exact counts — a
//! data-dependent form (`mulu`, a register-count shift) or a linker-relaxed
//! width (a bare symbolic operand, `jbra`'s rung ladder — see the
//! [`crate::m68k_cycles`] ruling). A ceiling keeps `@budget` sound: the walk's
//! worst path is at or above the machine's. It cannot carry `@cycles_exact`,
//! which is why the last refusal in the table exists and fires only for that
//! attribute.
//!
//! Only a RETURN instruction ends a charged path, and only on the EDGE that
//! actually returns. That single rule is what makes the bound sound in the one
//! direction that matters: every other way out of the body leaves cost
//! unaccounted, so it is refused rather than silently treated as zero.
//!
//! ## Enumerated dispatch — the one way through a computed transfer
//!
//! A computed transfer (`jmp .table(a1)`) is refused because its destination set
//! is DATA. The one exception is an author-written `targets(.a, .b, …)` clause
//! (enumerated-dispatch design §2): it names the finite set of LOCAL labels the
//! transfer can land on, and the walk turns the `TailOut` into that many `Follow`
//! edges — the fixed cost charged once, then a max/min over the arms, exactly as
//! for a two-edge branch ([`enumerated_succs`]). Exhaustiveness is the AUTHOR's
//! claim, verified only for existence/locality/distinctness — so ONLY this opt-in
//! walk reads the clause; a wrong enumeration mis-measures the budget it feeds and
//! nothing else. Enumerated cycles fall to the unbounded-loop refusal naturally.
//! The clause's own validity refusals live in [`check_dispatch_targets`].
//!
//! ## Three things a budget does NOT say
//!
//!   * **Nominal cycles, not elapsed time.** Bus contention (the Z80 losing the
//!     bus to a 68000 DMA, either CPU stalling on a VDP-port FIFO) is a
//!     whole-machine fact, not a proc-local one, and is not modeled. A budget
//!     bounds ISSUED cycles — the same unit the sound driver's own balance proof
//!     uses, so the two agree.
//!   * **Interrupts are not counted.** A proc interrupted mid-body spends the
//!     handler's time on top of its budget. The bound is over the proc's own
//!     instruction stream; masking is the caller's business.
//!   * **Entry-point zero only.** The walk roots at the body's first instruction,
//!     so a path reachable only from an `export`ed mid-body label is outside the
//!     claim.

use crate::flag_check::{entry_instr_idx, instr_span, Cfg, Edge};
use crate::value::{CodeItem, CodeOperand, ItemAuthor, Width};
use crate::z80_cycles::Cost;
use sigil_backend_m68k::m68k_cycles::CycleCost;
use sigil_ir::backend::Cpu;
use sigil_span::Span;
use std::collections::{BTreeMap, BTreeSet};

/// What a cycle-budget walk concluded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BudgetFindingKind {
    /// `[cycles.over-budget]` — the worst-case path costs more than the ceiling.
    OverBudget {
        /// The worst-case path cost.
        worst: u64,
        /// The declared ceiling.
        budget: u64,
    },
    /// `[cycles.path-mismatch]` — `@cycles_exact` with paths of unequal cost.
    PathMismatch {
        /// The cheapest path's cost.
        min: u64,
        /// The dearest path's cost.
        max: u64,
    },
    /// `[cycles.unbounded-loop]` — a back edge reaches an instruction already on
    /// the path, so the longest path is unbounded.
    UnboundedLoop,
    /// `[cycles.opaque-call]` — a call whose callee cost is not a local fact.
    OpaqueCall {
        /// The call mnemonic.
        mnemonic: String,
    },
    /// `[cycles.unbounded-transfer]` — control leaves the body other than by
    /// returning (a tail transfer, or running off the end).
    UnboundedTransfer {
        /// The transferring mnemonic.
        mnemonic: String,
    },
    /// `[cycles.computed-transfer]` — a transfer whose target is computed
    /// (`jp (hl)`, `jmp .table(a1)`): the destination set is data, and the walk
    /// refuses to enumerate what the program text does not.
    ComputedTransfer {
        /// The transferring mnemonic.
        mnemonic: String,
    },
    /// `[cycles.ambiguous-branch]` — an outcome-split conditional whose taken and
    /// fall-through edges cannot be told apart.
    ///
    /// **THIS VARIANT NOW HAS A REACHABLE INPUT**, and it acquired one exactly the
    /// way the kill condition below predicted a table change could: the eight
    /// REPEATING block ops (`ldir`, `lddr`, `cpir`, `cpdr`, `inir`, `indr`,
    /// `otir`, `otdr`) carry a `Cost::Split` of 21/16 and present ONE edge,
    /// because a block repeat re-executes the same instruction rather than
    /// branching. So the `two_way` guard fires on them, and the refusal is the
    /// right answer: the true cost is `16 + 21*(BC-1)` with `BC` a run-time
    /// value, and charging either number to the single edge would state a bound
    /// that is wrong by up to 21 T per iteration. Pinned by this module's
    /// `a_block_repeat_is_an_ambiguous_branch_not_an_unknown_op`.
    ///
    /// The same id is also emitted by a second producer —
    /// [`crate::z80_cycles::CycleBail::AmbiguousBranch`], reached through the
    /// `cycles(L1, L2)` span builtin ([`crate::eval::builtins`]) and pinned by
    /// `z80_cycles`'s own `mod tests` and `tests/t40_cycles.rs`.
    ///
    /// The walk's OTHER producer remains inputless: the enumerated-dispatch arm
    /// in [`charged_edges`] — a dispatch mnemonic (`jmp`/`jp`, the only shapes a
    /// `targets(...)` clause is legal on) never carries a Split table cost, so
    /// that arm's `WalkCost::Split` case has no input. It would be a cost-table
    /// defect, refused rather than routed blind.
    ///
    /// The near miss, and the ordering it depends on: `call cc` DOES carry a split
    /// cost with a single edge (it calls and comes back, so its only successor is
    /// the fall-through). It never reaches the guard because
    /// [`crate::context::is_call_mnemonic`] refuses it as `[cycles.opaque-call]`
    /// FIRST — and that is structural, not a corpus accident: the call bail is the
    /// first refusal [`charged_edges`] makes, before any edge or cost is consulted.
    /// Reorder those two and this variant acquires an input (measured: moving the
    /// call bail below the `two_way` guard makes `call nz, Helper` earn
    /// `[cycles.ambiguous-branch]`; `tests/cycle_budget.rs`'s `a_call_is_refused`
    /// is the pin that would catch it).
    ///
    /// The counted enumeration of TERMINATOR shapes is still there
    /// (`tests/cycle_budget.rs`,
    /// `a_split_cost_conditional_is_refused_before_its_edges_are_counted`, with the
    /// edge-count invariant pinned crate-side in this module's `mod tests`,
    /// `a_split_cost_terminator_presents_exactly_two_edges`), and it still holds
    /// for terminators. What it never covered — and what the block repeats are —
    /// is a split-cost instruction that is not a terminator at all.
    ///
    /// **Kill condition:** none now. The variant is load-bearing, not defensive:
    /// deleting it would make the block repeats route one of two numbers to their
    /// single edge.
    AmbiguousBranch {
        /// The conditional's mnemonic.
        mnemonic: String,
    },
    /// `[cycles.unknown-op]` — an op outside the T-state table.
    UnknownOp {
        /// The unmodeled mnemonic.
        mnemonic: String,
    },
    /// `[cycles.inline-data]` — a `DataBuf` spliced into the code stream.
    InlineData,
    /// `[cycles.empty-body]` — a body with no instructions at all. Its one path
    /// never executes a return, so there is no path cost to bound: control
    /// entering it continues into whatever follows.
    EmptyBody,
    /// `[cycles.inexact-cost]` — `@cycles_exact` over a body containing an
    /// instruction whose charged cost is a CEILING (data-dependent, or
    /// linker-relaxed): a maximum bounds a budget but cannot prove an equality.
    InexactCost {
        /// The first ceiling-charged mnemonic, in item order.
        mnemonic: String,
    },
}

impl BudgetFindingKind {
    /// The lint id this kind reports under.
    pub fn lint_id(&self) -> &'static str {
        match self {
            Self::OverBudget { .. } => "cycles.over-budget",
            Self::PathMismatch { .. } => "cycles.path-mismatch",
            Self::UnboundedLoop => "cycles.unbounded-loop",
            Self::OpaqueCall { .. } => "cycles.opaque-call",
            Self::UnboundedTransfer { .. } => "cycles.unbounded-transfer",
            Self::ComputedTransfer { .. } => "cycles.computed-transfer",
            Self::AmbiguousBranch { .. } => "cycles.ambiguous-branch",
            Self::UnknownOp { .. } => "cycles.unknown-op",
            Self::InlineData => "cycles.inline-data",
            Self::EmptyBody => "cycles.empty-body",
            Self::InexactCost { .. } => "cycles.inexact-cost",
        }
    }

    /// The finding's body text, after the `[id] in `proc`: ` prefix.
    pub fn message(&self) -> String {
        match self {
            Self::OverBudget { worst, budget } => format!(
                "the worst-case path costs {worst} cycles, over the declared budget of {budget}"
            ),
            Self::PathMismatch { min, max } => format!(
                "`@cycles_exact` requires every path to cost the same, but they range \
                 from {min} to {max} cycles"
            ),
            Self::UnboundedLoop => "a loop reaches this instruction again, so the worst-case \
                 path cost is unbounded, a cycle budget needs a loop-free body"
                .to_string(),
            Self::OpaqueCall { mnemonic } => format!(
                "`{mnemonic}` costs whatever its callee costs, which is not a fact about \
                 this proc, a cycle budget needs a call-free body"
            ),
            Self::UnboundedTransfer { mnemonic } => format!(
                "`{mnemonic}` continues into code outside this proc, so the path's cost is \
                 not accounted here, a cycle budget needs every path to end at a return \
                 (a computed dispatch landing on LOCAL labels can be enumerated with a \
                 `targets(...)` clause; a transfer to a `@noreturn` target ends the path)"
            ),
            Self::ComputedTransfer { mnemonic } => format!(
                "`{mnemonic}` transfers to a COMPUTED target, so where this path goes is \
                 data, not structure, the walk cannot enumerate destinations the program \
                 text does not name; name the reachable LOCAL labels with a `targets(...)` \
                 clause on the transfer to budget it"
            ),
            Self::AmbiguousBranch { mnemonic } => format!(
                "`{mnemonic}` costs differently taken and not-taken, and its two edges are \
                 not distinguishable here"
            ),
            Self::UnknownOp { mnemonic } => format!(
                "`{mnemonic}` is not in this CPU's cycle table, add it to `z80_cycles` / \
                 `m68k_cycles` if a budgeted proc legitimately needs it"
            ),
            Self::InlineData => "this body splices data into the code stream, and those \
                 bytes DECODE as instructions if control reaches them, the control-flow \
                 model steps over them, so no path through this proc can be costed"
                .to_string(),
            Self::EmptyBody => "this body has no instructions, so its one path never \
                 executes a return and there is no path cost to bound, control entering \
                 it continues into whatever follows"
                .to_string(),
            Self::InexactCost { mnemonic } => format!(
                "`@cycles_exact` needs every instruction to cost one exact number, but \
                 `{mnemonic}`'s charge is a ceiling (data-dependent, or decided by the \
                 linker's width choice), a maximum can hold a budget, not an equality"
            ),
        }
    }
}

/// One cycle-budget finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BudgetFinding {
    /// What was concluded.
    pub kind: BudgetFindingKind,
    /// Where to point.
    pub span: Span,
}

/// The cost extremes of a proc's path set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathCosts {
    /// The cheapest path from entry to a return.
    pub min: u64,
    /// The dearest path from entry to a return.
    pub max: u64,
    /// The MNEMONIC of the first (lowest item index) reachable instruction whose
    /// charged cost is a CEILING rather than an exact count — `None` when every
    /// charge is exact.
    /// `max` is then an upper bound on the machine's worst path (sound for
    /// `@budget`); an equality proof over it is not a proof (`@cycles_exact`
    /// refuses through [`BudgetFindingKind::InexactCost`]).
    pub inexact: Option<String>,
}

/// One instruction's charge, normalized across the two CPU tables.
enum WalkCost {
    /// One cost for every way out. `exact: false` marks a ceiling.
    Fixed { t: u64, exact: bool },
    /// An outcome-split conditional: edge 0 (taken) and edge 1 (fall-through)
    /// charge differently.
    Split { taken: u64, not_taken: u64, exact: bool },
    /// No entry in the CPU's table.
    Unknown,
}

/// The CPU's per-instruction charge. The Z80 table is exact everywhere it is
/// defined; the 68000 table marks its data-dependent and linker-relaxed maxima.
fn walk_cost(cpu: Cpu, mnem: &str, size: Option<Width>, ops: &[CodeOperand]) -> WalkCost {
    match cpu {
        Cpu::Z80 => match crate::z80_cycles::instr_cost(mnem, ops) {
            Cost::Fixed(n) => WalkCost::Fixed { t: u64::from(n), exact: true },
            Cost::Split { taken, not_taken } => WalkCost::Split {
                taken: u64::from(taken),
                not_taken: u64::from(not_taken),
                exact: true,
            },
            Cost::Unknown => WalkCost::Unknown,
        },
        Cpu::M68000 => match crate::m68k_cycles::instr_cost(mnem, size, ops) {
            CycleCost::Fixed { cycles, exact } => {
                WalkCost::Fixed { t: u64::from(cycles), exact }
            }
            CycleCost::Branch { taken, not_taken, exact } => WalkCost::Split {
                taken: u64::from(taken),
                not_taken: u64::from(not_taken),
                exact,
            },
            CycleCost::Unmodeled => WalkCost::Unknown,
        },
    }
}

/// The CPU's successor edges — the same per-CPU builders every other consumer of
/// the shared [`Cfg`] reads.
fn cpu_edges(cfg: &Cfg, cpu: Cpu, idx: usize) -> Vec<Edge> {
    match cpu {
        Cpu::Z80 => cfg.z80_edges(idx),
        Cpu::M68000 => cfg.edges(idx),
    }
}

/// Measure a proc body's path costs, or say why it cannot be measured.
///
/// `items` is the proc's evaluated `CodeBuf`; `cpu` selects the timing model and
/// the edge model. A body with no instructions is refused ([`BudgetFindingKind::
/// EmptyBody`]): only a return ends a charged path, and an empty body has none.
pub fn path_costs(
    items: &[CodeItem],
    cpu: Cpu,
    decl_span: Span,
    noreturn: &BTreeSet<String>,
) -> Result<PathCosts, BudgetFinding> {
    // Inline data is not an `Instr`, so the CFG links straight across it and would
    // charge it nothing — while on hardware those bytes decode and execute if
    // control reaches them. Only a reachability proof could tell the two apart,
    // and a budget does not get to assume the author put the table somewhere safe.
    if let Some(i) = items.iter().position(|it| matches!(it, CodeItem::Inline(..))) {
        let span = match &items[i] {
            CodeItem::Inline(_, sp) => *sp,
            _ => decl_span,
        };
        return Err(BudgetFinding { kind: BudgetFindingKind::InlineData, span });
    }
    let Some(entry) = entry_instr_idx(items) else {
        return Err(BudgetFinding { kind: BudgetFindingKind::EmptyBody, span: decl_span });
    };
    let cfg = Cfg::build(items);
    // A DFS that (a) proves the reachable subgraph is acyclic and (b) leaves a
    // post-order, whose reverse is a topological order. The two jobs share one
    // walk because a back edge is exactly what makes the second impossible.
    let order = postorder(&cfg, items, entry, cpu)?;
    // POST-order visits every successor before its predecessor, so a single
    // forward pass over `order` fills the whole table with no revisits.
    let mut best: BTreeMap<usize, (u64, u64)> = BTreeMap::new();
    // The inexact witness: the LOWEST-indexed reachable ceiling charge, so the
    // diagnostic is deterministic and names the first offender in reading order.
    let mut inexact: Option<(usize, String)> = None;
    for &idx in &order {
        let CodeItem::Instr { mnemonic, size, ops, span, author, .. } = &items[idx] else {
            unreachable!("postorder holds instruction indices only");
        };
        let cost = walk_cost(cpu, mnemonic, *size, ops);
        let ceiling = matches!(
            cost,
            WalkCost::Fixed { exact: false, .. } | WalkCost::Split { exact: false, .. }
        );
        if ceiling && inexact.as_ref().is_none_or(|(i, _)| idx < *i) {
            inexact = Some((idx, mnemonic.clone()));
        }
        let charged =
            charged_edges(&cfg, cpu, items, idx, mnemonic, ops, author, &cost, noreturn, *span)?;
        let mut lo = u64::MAX;
        let mut hi = 0u64;
        for e in charged {
            let (smin, smax) = match e.succ {
                // The path ends here: a return contributes only its own cost.
                None => (0, 0),
                Some(s) => *best.get(&s).expect("successors are ordered before predecessors"),
            };
            lo = lo.min(e.cost + smin);
            hi = hi.max(e.cost + smax);
        }
        best.insert(idx, (lo, hi));
    }
    let (min, max) = best[&entry];
    Ok(PathCosts { min, max, inexact: inexact.map(|(_, m)| m) })
}

/// Check a proc's declared cycle contract. `budget` is the `@budget(cycles: N)`
/// ceiling and `exact` the presence of `@cycles_exact`; a proc declaring neither
/// is not walked.
pub fn check_cycle_budget(
    items: &[CodeItem],
    cpu: Cpu,
    decl_span: Span,
    budget: Option<u64>,
    exact: bool,
    noreturn: &BTreeSet<String>,
) -> Vec<BudgetFinding> {
    if budget.is_none() && !exact {
        return Vec::new();
    }
    let costs = match path_costs(items, cpu, decl_span, noreturn) {
        Ok(c) => c,
        Err(f) => return vec![f],
    };
    let mut out = Vec::new();
    // Both verdicts point at the DECLARATION. The body may be spliced from a
    // comptime template or a `with <ctx> { }` bracket, whose instructions carry
    // their own file's spans — so an interior site can land in a file that does
    // not contain the attribute being violated.
    let span = decl_span;
    if let Some(n) = budget {
        if costs.max > n {
            out.push(BudgetFinding {
                kind: BudgetFindingKind::OverBudget { worst: costs.max, budget: n },
                span,
            });
        }
    }
    if exact {
        match &costs.inexact {
            // A ceiling charge can hold a budget but cannot prove an equality:
            // the exactness half refuses (naming the first offender) while the
            // budget half, if also declared, still concludes above.
            Some(mnemonic) => out.push(BudgetFinding {
                kind: BudgetFindingKind::InexactCost { mnemonic: mnemonic.clone() },
                span,
            }),
            None if costs.min != costs.max => out.push(BudgetFinding {
                kind: BudgetFindingKind::PathMismatch { min: costs.min, max: costs.max },
                span,
            }),
            None => {}
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Enumerated dispatch — the `targets(...)` clause's own validity refusals
// (enumerated-dispatch design §1). These are decidable from ONE proc body and
// fire whether or not a budget is declared: a `targets(...)` clause that names
// the wrong thing reads as a checked claim and is not one. The cycle-budget walk
// (above) is the only CONSUMER; this is the GATEKEEPER.
// ---------------------------------------------------------------------------

/// What a `targets(...)` clause got wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DispatchFindingKind {
    /// `[dispatch.targets-on-call]` — a clause on a call (`jsr`/`bsr`/`jbsr`). A
    /// call's cost is callee-cost composition, deliberately NOT this form.
    OnCall {
        /// The call mnemonic.
        mnemonic: String,
    },
    /// `[dispatch.targets-redundant]` — a clause on an instruction whose control
    /// target is already exact (a direct `jmp .label` / `jmp External`) or which
    /// is not an unconditional computed transfer at all. The enumeration buys
    /// nothing where the edge is already known.
    Redundant {
        /// The annotated mnemonic.
        mnemonic: String,
    },
    /// `[dispatch.target-unknown]` — a named `.local` label is defined nowhere in
    /// this proc body.
    Unknown {
        /// The unresolved label, as written.
        label: String,
    },
    /// `[dispatch.target-nonlocal]` — a named target resolves to a symbol that is
    /// not a LOCAL label of this proc (a cross-proc / global name). A nonlocal arm
    /// would also need callee costs, so v1 refuses it to keep the form's meaning
    /// sharp.
    Nonlocal {
        /// The nonlocal name, as written.
        label: String,
    },
    /// `[dispatch.target-duplicate]` — the same label is named twice.
    Duplicate {
        /// The repeated label.
        label: String,
    },
    /// `[dispatch.target-trailing]` — a named label is LOCAL but has no instruction
    /// at or after it: it closes the body, so it is a fall-off in disguise, not a
    /// landing point. Left un-refused it produces contradictory diagnostics — the
    /// checker accepts it, then the budget walk refuses the transfer as computed,
    /// telling the author to add the clause they already wrote.
    Trailing {
        /// The trailing label.
        label: String,
    },
}

impl DispatchFindingKind {
    /// The lint id this kind reports under.
    pub fn lint_id(&self) -> &'static str {
        match self {
            Self::OnCall { .. } => "dispatch.targets-on-call",
            Self::Redundant { .. } => "dispatch.targets-redundant",
            Self::Unknown { .. } => "dispatch.target-unknown",
            Self::Nonlocal { .. } => "dispatch.target-nonlocal",
            Self::Duplicate { .. } => "dispatch.target-duplicate",
            Self::Trailing { .. } => "dispatch.target-trailing",
        }
    }

    /// The finding's body text, after the `[id] in `proc`: ` prefix.
    pub fn message(&self) -> String {
        match self {
            Self::OnCall { mnemonic } => format!(
                "`targets(...)` names where control GOES, but `{mnemonic}` is a call, its \
                 cost is the callee's, which is the opaque-call problem, not this form"
            ),
            Self::Redundant { mnemonic } => format!(
                "`targets(...)` applies only to an unconditional COMPUTED transfer \
                 (`jmp .table(a1)`); `{mnemonic}` here already has an exact control target, so \
                 the enumeration is redundant"
            ),
            Self::Unknown { label } => format!(
                "`targets(...)` names `{label}`, but no such local label is defined in this proc"
            ),
            Self::Nonlocal { label } => format!(
                "`targets(...)` names `{label}`, which is not a LOCAL label of this proc, a \
                 cross-proc target would also need the callee's cost, so v1 refuses it"
            ),
            Self::Duplicate { label } => format!(
                "`targets(...)` names `{label}` more than once, an enumeration lists each \
                 reachable label once"
            ),
            Self::Trailing { label } => format!(
                "`targets(...)` names `{label}`, which closes the proc with no instruction \
                 after it, a trailing label is a fall-off, not a landing point, so control \
                 never arrives there through the dispatch"
            ),
        }
    }
}

/// One `targets(...)` validity refusal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchFinding {
    /// What the clause got wrong.
    pub kind: DispatchFindingKind,
    /// The annotated instruction's span.
    pub span: Span,
}

/// Check every `targets(...)` clause in one proc body for the §1 refusals. Runs
/// unconditionally (no attribute gates it): a wrong enumeration is a wrong claim
/// whether or not a `@budget` reads it. Cheap when no clause is present — the
/// per-instruction scan below only builds a [`Cfg`] when at least one exists.
pub fn check_dispatch_targets(items: &[CodeItem], cpu: Cpu) -> Vec<DispatchFinding> {
    let any = items
        .iter()
        .any(|it| matches!(it, CodeItem::Instr { targets, .. } if !targets.is_empty()));
    if !any {
        return Vec::new();
    }
    let cfg = Cfg::build(items);
    let mut out = Vec::new();
    for (idx, it) in items.iter().enumerate() {
        let CodeItem::Instr { mnemonic, ops, span, targets, .. } = it else { continue };
        if targets.is_empty() {
            continue;
        }
        if crate::context::is_call_mnemonic(mnemonic, cpu) {
            out.push(DispatchFinding {
                kind: DispatchFindingKind::OnCall { mnemonic: mnemonic.clone() },
                span: *span,
            });
            continue;
        }
        // Legal only on a BARE computed transfer: exactly one `TailOut` edge and no
        // operand naming a symbol. Everything else already has an exact edge (a
        // direct `jmp .label`, a tail `jmp External`) or is no transfer at all.
        let names_a_target = ops.iter().any(|o| {
            matches!(
                o,
                CodeOperand::Sym(_) | CodeOperand::SymOff { .. } | CodeOperand::AbsSym { .. }
            )
        });
        if !matches!(cpu_edges(&cfg, cpu, idx).as_slice(), [Edge::TailOut]) || names_a_target {
            out.push(DispatchFinding {
                kind: DispatchFindingKind::Redundant { mnemonic: mnemonic.clone() },
                span: *span,
            });
            continue;
        }
        // The label set: each must be distinct and a LOCAL label of this proc.
        // Names arrive resolved through the label scope, so a valid local reads as
        // its mangled `CodeItem::Label` symbol; an undefined `.local` kept its dot
        // (unknown), a cross-proc/global name kept its bare spelling (nonlocal).
        let mut seen: std::collections::BTreeSet<&String> = std::collections::BTreeSet::new();
        for label in targets {
            if !seen.insert(label) {
                out.push(DispatchFinding {
                    kind: DispatchFindingKind::Duplicate { label: label.clone() },
                    span: *span,
                });
                continue;
            }
            if cfg.is_local_label(label) {
                // A local label with an instruction at/after it is a real landing
                // point; one with NONE closes the body and is a fall-off, not a
                // landing point (the b8 trailing-label reading).
                if cfg.label_index(label).is_some() {
                    continue;
                }
                out.push(DispatchFinding {
                    kind: DispatchFindingKind::Trailing { label: label.clone() },
                    span: *span,
                });
                continue;
            }
            let kind = if label.starts_with('.') {
                DispatchFindingKind::Unknown { label: label.clone() }
            } else {
                DispatchFindingKind::Nonlocal { label: label.clone() }
            };
            out.push(DispatchFinding { kind, span: *span });
        }
    }
    out
}

/// One charged successor of an instruction.
struct ChargedEdge {
    /// T-states charged for taking THIS edge out of the instruction.
    cost: u64,
    /// The next instruction, or `None` when the path ENDS here (a return).
    succ: Option<usize>,
}

/// The successor item indices an enumerated-dispatch `targets(...)` clause names,
/// when the clause is present and every named label resolves LOCALLY. `None` when
/// there is no clause to honor here — the instruction carries no targets, OR it is
/// not a bare COMPUTED transfer (a single `TailOut` naming no symbol), OR a name
/// does not resolve to a local label — and the caller falls back to the ordinary
/// edge model (which then refuses the computed transfer as it did before this
/// form).
///
/// This is the ONE consumer of `targets`. It turns a `TailOut` into `Follow` edges
/// and nothing else: no `Cfg` edge builder changes, so the preserves prover, the
/// flag walks, and the clobbers closure keep treating the instruction as an opaque
/// computed transfer. A wrong enumeration can therefore mis-measure only the budget
/// its author opted into, and can corrupt no soundness-bearing analysis.
fn enumerated_succs(cfg: &Cfg, cpu: Cpu, items: &[CodeItem], idx: usize) -> Option<Vec<usize>> {
    let CodeItem::Instr { targets, ops, .. } = &items[idx] else { return None };
    if targets.is_empty() {
        return None;
    }
    // Only a BARE computed transfer is enumerable: exactly one `TailOut` edge and no
    // operand naming a symbol. A direct `jmp .label` (or `jmp External`) already
    // has an exact edge — the redundant case the per-proc dispatch check refuses —
    // so the clause is ignored here and the exact edge stands.
    //
    // The singleton pattern needs no sibling test of its own: a `BranchOut` is never
    // an instruction's only edge, so `[TailOut]` admits exactly the unconditional
    // transfers out, and the `names_a_target` clause narrows those to the computed
    // ones.
    let names_a_target = ops.iter().any(|o| {
        matches!(
            o,
            CodeOperand::Sym(_) | CodeOperand::SymOff { .. } | CodeOperand::AbsSym { .. }
        )
    });
    if !matches!(cpu_edges(cfg, cpu, idx).as_slice(), [Edge::TailOut]) || names_a_target {
        return None;
    }
    // Resolve every name to its local instruction index; a single miss abandons
    // the whole enumeration (the walk falls back to a `ComputedTransfer` refusal
    // rather than silently under-count a malformed clause — the dispatch check
    // owns that diagnostic).
    targets.iter().map(|t| cfg.label_index(t)).collect()
}

/// The charged successors of `idx`. Every way out of the body except an
/// [`Edge::Return`] is refused, so a path can never be closed with cost left
/// unaccounted. An end-of-body `ret cc` presents `[Return, FallOff]`, so the
/// escaping side refuses the whole bound from the SAME list that names the
/// returning side — no positional or mnemonic rule decides which is which.
///
/// One edge-model fact is still read POSITIONALLY, because `Edge` does not record
/// it: **taken-first**. Every conditional arm of both edge builders
/// ([`Cfg::z80_edges`] and the 68k [`Cfg::edges`]) pushes the branch edge before
/// the fall-through, so an outcome-split conditional's `taken` cost belongs to
/// edge 0. A form presenting anything but exactly two edges does not satisfy
/// that reading, so a split cost over it is refused rather than charged a number
/// the rule picked blind.
#[allow(clippy::too_many_arguments)] // the walk's per-instruction facts; each is load-bearing
fn charged_edges(
    cfg: &Cfg,
    cpu: Cpu,
    items: &[CodeItem],
    idx: usize,
    mnem: &str,
    ops: &[CodeOperand],
    author: &ItemAuthor,
    cost: &WalkCost,
    noreturn: &BTreeSet<String>,
    span: Span,
) -> Result<Vec<ChargedEdge>, BudgetFinding> {
    let bail = |kind| Err(BudgetFinding { kind, span });
    if crate::context::is_call_mnemonic(mnem, cpu) {
        return bail(BudgetFindingKind::OpaqueCall { mnemonic: mnem.to_string() });
    }
    // Enumerated dispatch: an unconditional computed transfer carrying a
    // `targets(...)` clause is the ONE way this walk sees THROUGH a computed jump.
    // The `TailOut` its edge model produces becomes N `Follow` edges here — the
    // instruction's own fixed cost charged once, then a fan-out to each named
    // local label, exactly like a two-edge branch fans out. This arm is
    // self-contained (it consumes the clause and returns), leaving the ordinary
    // edge refusals below untouched for every other shape.
    if let Some(succs) = enumerated_succs(cfg, cpu, items, idx) {
        let t = match cost {
            WalkCost::Fixed { t, .. } => *t,
            // A computed jmp is unconditional, so its charge is Fixed. A Split or
            // an off-table cost would be a table defect at a dispatch mnemonic —
            // refuse honestly rather than route a number the arm picked blind.
            WalkCost::Split { .. } => {
                return bail(BudgetFindingKind::AmbiguousBranch { mnemonic: mnem.to_string() })
            }
            WalkCost::Unknown => {
                return bail(BudgetFindingKind::UnknownOp { mnemonic: mnem.to_string() })
            }
        };
        return Ok(succs.into_iter().map(|s| ChargedEdge { cost: t, succ: Some(s) }).collect());
    }
    let edges = cpu_edges(cfg, cpu, idx);
    // A DIVERGENT terminal transfer ends the path exactly like a return: nothing
    // after it runs in THIS proc, so its cost need not be accounted. Two forms,
    // and only these — a transfer out is otherwise unbounded (below):
    //   * an `ItemAuthor::AssertDesugar`-authored unconditional transfer — the
    //     assert / raise-error / raise-exception rail's `jmp (pages).l`, which the
    //     compiler emitted knowing it never returns. A HAND-written `jmp` to the
    //     same blob stays a plain `TailOut` (authorship is the distinguisher).
    //   * a transfer whose named target is declared `@noreturn`.
    // Computed once (an instruction-level fact) and consulted at the transfer-out arms.
    // The target is read through the UNIFIED extractor so `jmp (Diverge).l` (an
    // `AbsSym`) matches a `@noreturn` symbol, not only a bare `jbra Diverge`.
    let target = crate::flag_check::transfer_target_sym(ops);
    let names_a_target = target.is_some();
    let is_uncond_transfer = matches!(mnem, "bra" | "jbra" | "jmp" | "jra");
    // The authored-rail arm is CONJOINED with `names_a_target` so it is
    // order-independent with the coming enum-dispatch `targets()` arm by
    // construction: an authored terminal that names nothing is not a rail.
    let divergent_terminal =
        (matches!(author, ItemAuthor::AssertDesugar) && is_uncond_transfer && names_a_target)
            || target.is_some_and(|t| noreturn.contains(t));
    // The STRUCTURAL refusals come before the cost-table one: a path that
    // escapes the body is unboundable whatever the instruction costs, and for a
    // computed transfer (`jp (hl)`) an "add it to the table" refusal would be a
    // misleading invitation — a table entry would not make it boundable.
    // `names_a_target` (above) is true when a transfer NAMES its target — the bare
    // `Sym`, or the pinned/offset absolutes; only a target the program text does
    // not name at all (`jp (hl)`, `jmp .table(a1)`) is computed.
    for e in &edges {
        match e {
            // A divergent terminal (`@noreturn` target, or an authored rail's
            // `jmp`) closes the path like a return — charged below, not refused.
            Edge::TailOut | Edge::BranchOut if divergent_terminal => {}
            // Transferring out with no symbolic target is a COMPUTED transfer
            // (`jp (hl)`, `jmp .table(a1)`): the honest refusal names the shape,
            // because "code outside this proc" may be false — a jump-table
            // dispatch lands inside its own body, through addresses the walk
            // cannot enumerate.
            Edge::TailOut | Edge::BranchOut if !names_a_target => {
                return bail(BudgetFindingKind::ComputedTransfer {
                    mnemonic: mnem.to_string(),
                })
            }
            // Running off the end, or transferring out, leaves cost this bound
            // cannot see. Refuse rather than report a ceiling that is too low.
            Edge::FallOff | Edge::TailOut | Edge::BranchOut => {
                return bail(BudgetFindingKind::UnboundedTransfer {
                    mnemonic: mnem.to_string(),
                })
            }
            Edge::Follow(_) | Edge::Return => {}
        }
    }
    if matches!(cost, WalkCost::Unknown) {
        return bail(BudgetFindingKind::UnknownOp { mnemonic: mnem.to_string() });
    }
    let two_way = edges.len() == 2;
    // REACHED, by the eight repeating block ops. Every split-cost TERMINATOR on
    // both CPUs presents exactly two edges, and the one split-cost terminator that
    // presents a single edge (`call cc`) is refused as an opaque call at the top of
    // this function — but `ldir` and its seven siblings are split-cost
    // NON-terminators with one edge, because a block repeat re-executes itself
    // instead of branching. They land here, and refusing is correct: their true
    // cost is `16 + 21*(BC-1)` for a run-time `BC`. See
    // [`BudgetFindingKind::AmbiguousBranch`].
    if matches!(cost, WalkCost::Split { .. }) && !two_way {
        return bail(BudgetFindingKind::AmbiguousBranch { mnemonic: mnem.to_string() });
    }
    let mut out = Vec::new();
    for (i, e) in edges.iter().enumerate() {
        let cost = match cost {
            WalkCost::Fixed { t, .. } => *t,
            WalkCost::Split { taken, not_taken, .. } => {
                if i == 0 {
                    *taken
                } else {
                    *not_taken
                }
            }
            WalkCost::Unknown => unreachable!("refused above"),
        };
        match e {
            Edge::Follow(s) => out.push(ChargedEdge { cost, succ: Some(*s) }),
            // A return — or a divergent terminal transfer — CLOSES the path: the
            // caller owns everything after it (a divergent transfer owns nothing).
            Edge::Return => out.push(ChargedEdge { cost, succ: None }),
            Edge::TailOut | Edge::BranchOut if divergent_terminal => {
                out.push(ChargedEdge { cost, succ: None })
            }
            Edge::FallOff | Edge::TailOut | Edge::BranchOut => unreachable!("refused above"),
        }
    }
    // An instruction with no successors at all cannot happen for a modeled op
    // (every `z80_edges` arm yields at least one edge), but treating it as a path
    // end would invent a zero-cost exit — refuse instead.
    if out.is_empty() {
        return bail(BudgetFindingKind::UnboundedTransfer { mnemonic: mnem.to_string() });
    }
    Ok(out)
}

/// A POST-ORDER over the instructions reachable from `entry`, proving on the way
/// that the reachable subgraph is ACYCLIC. On a DAG a post-order lists every
/// successor before its predecessor, which is exactly the order the cost pass
/// needs. A back edge (an edge to an instruction still open on the DFS stack) is
/// `[cycles.unbounded-loop]`, reported at the instruction the loop re-enters.
fn postorder(
    cfg: &Cfg,
    items: &[CodeItem],
    entry: usize,
    cpu: Cpu,
) -> Result<Vec<usize>, BudgetFinding> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Open,
        Done,
    }
    let mut mark: BTreeMap<usize, Mark> = BTreeMap::new();
    let mut order = Vec::new();
    // Exhaustive rather than a catch-all: every way of LEAVING the body is
    // named, so an edge kind that stays inside one cannot be dropped here by
    // default. A dropped in-proc successor loses a path, and a lost path makes
    // the bound too LOW — the one direction a budget must not err in.
    let succs = |i: usize| -> Vec<usize> {
        // An enumerated computed transfer resolves to its named local labels; the
        // topo walk must see the SAME successors the cost pass charges, or a
        // drain arm would go unvisited and its cost unfilled.
        if let Some(ts) = enumerated_succs(cfg, cpu, items, i) {
            return ts;
        }
        cpu_edges(cfg, cpu, i)
            .into_iter()
            .filter_map(|e| match e {
                Edge::Follow(s) => Some(s),
                Edge::Return | Edge::FallOff | Edge::TailOut | Edge::BranchOut => None,
            })
            .collect()
    };
    // An explicit stack of (node, its successors, how many are consumed) —
    // recursion depth would otherwise track proc length.
    let mut stack: Vec<(usize, Vec<usize>, usize)> = vec![(entry, succs(entry), 0)];
    mark.insert(entry, Mark::Open);
    while !stack.is_empty() {
        let top = stack.len() - 1;
        let (node, kids, at) = &mut stack[top];
        if *at < kids.len() {
            let k = kids[*at];
            let node = *node;
            *at += 1;
            match mark.get(&k) {
                Some(Mark::Open) => {
                    let span = instr_span(items, k)
                        .or_else(|| instr_span(items, node))
                        .expect("a reachable instruction carries a span");
                    return Err(BudgetFinding { kind: BudgetFindingKind::UnboundedLoop, span });
                }
                Some(Mark::Done) => continue,
                None => {
                    mark.insert(k, Mark::Open);
                    let ks = succs(k);
                    stack.push((k, ks, 0));
                }
            }
        } else {
            let n = *node;
            mark.insert(n, Mark::Done);
            order.push(n);
            stack.pop();
        }
    }
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{CodeOperand, Z80Cond, Z80Reg8};
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
    /// An instruction stamped with a specific author — used to build an
    /// `AssertDesugar` divergent rail terminal.
    fn instr_authored(mnemonic: &str, ops: Vec<CodeOperand>, author: ItemAuthor) -> CodeItem {
        CodeItem::Instr { mnemonic: mnemonic.to_string(), size: None, ops, span: sp(), as_type: None, targets: Vec::new(), author }
    }
    /// The empty `@noreturn` set (the default for every pre-existing fixture).
    fn nr() -> BTreeSet<String> {
        BTreeSet::new()
    }
    /// A `@noreturn` set naming `names`.
    fn nr_of(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|s| s.to_string()).collect()
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
    fn a() -> CodeOperand {
        CodeOperand::Z80Reg8(Z80Reg8::A)
    }

    // A straight line of four nops and a `ret`: 4*4 + 10 = 26, one path.
    #[test]
    fn a_straight_line_has_one_cost() {
        let items = vec![
            instr("nop", vec![]),
            instr("nop", vec![]),
            instr("nop", vec![]),
            instr("nop", vec![]),
            instr("ret", vec![]),
        ];
        let c = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap();
        assert_eq!((c.min, c.max), (26, 26));
    }

    // `jr z, .skip` splits: taken 12 + ret 10 = 22; not-taken 7 + nop 4 + ret 10 = 21.
    // The SPLIT cost is what makes them differ by one T-state rather than by five.
    #[test]
    fn a_split_conditional_charges_each_edge_its_own_cost() {
        let items = vec![
            instr("jr", vec![cc(Z80Cond::Z), sym("skip")]),
            instr("nop", vec![]),
            label("skip"),
            instr("ret", vec![]),
        ];
        let c = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap();
        assert_eq!((c.min, c.max), (21, 22));
    }

    // The same shape with `jp cc` (10 either outcome): 10+10 vs 10+4+10.
    #[test]
    fn a_fixed_conditional_charges_both_edges_the_same() {
        let items = vec![
            instr("jp", vec![cc(Z80Cond::Z), sym("skip")]),
            instr("nop", vec![]),
            label("skip"),
            instr("ret", vec![]),
        ];
        let c = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap();
        assert_eq!((c.min, c.max), (20, 24));
    }

    // `ret cc` returns on the taken edge (11) and falls through on the other (5).
    #[test]
    fn a_conditional_return_ends_one_path_and_continues_the_other() {
        let items = vec![
            instr("ret", vec![cc(Z80Cond::Z)]),
            instr("nop", vec![]),
            instr("ret", vec![]),
        ];
        let c = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap();
        assert_eq!((c.min, c.max), (11, 19)); // 11 ; 5 + 4 + 10
    }

    // A back edge is refused, not approximated.
    #[test]
    fn a_loop_is_unbounded() {
        let items = vec![
            label("loop"),
            instr("nop", vec![]),
            instr("jp", vec![sym("loop")]),
        ];
        let e = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::UnboundedLoop);
    }

    // A DIAMOND is not a loop: both arms rejoin and the walk bounds it.
    #[test]
    fn a_diamond_is_not_a_loop() {
        let items = vec![
            instr("jp", vec![cc(Z80Cond::Z), sym("right")]),
            instr("nop", vec![]),
            instr("jp", vec![sym("join")]),
            label("right"),
            instr("nop", vec![]),
            instr("nop", vec![]),
            label("join"),
            instr("ret", vec![]),
        ];
        let c = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap();
        // left: 10 + 4 + 10 + 10 = 34 ; right: 10 + 4 + 4 + 10 = 28
        assert_eq!((c.min, c.max), (28, 34));
    }

    /// The EDGE-COUNT invariant over TERMINATORS: every split-cost terminator on
    /// both CPUs presents exactly two edges, so a Split cost on one always has two
    /// places to be routed. Read off the edge builders directly, because the
    /// diagnostic cannot show it — the structural transfer-out refusal in
    /// [`charged_edges`] fires on the first leaving edge, BEFORE `two_way` is ever
    /// computed, so a one-edge shape would still be refused and a source-level
    /// sweep would still be green.
    ///
    /// THIS INVARIANT IS ABOUT TERMINATORS AND ONLY TERMINATORS, which is the
    /// scope its earlier prose left implicit and the `two_way` guard was read as
    /// having no input because of. A split-cost NON-terminator with a single edge
    /// exists — the eight repeating block ops — and is asserted at the end of this
    /// sweep as its own polarity.
    ///
    /// Counted, and paired with both polarities: a `jbra`/`jp` tail has ONE edge
    /// and is Fixed-cost (a split with one edge is what the guard fears, and no
    /// terminator here is one), while `call cc` IS a Split cost with a single edge
    /// — saved only because [`charged_edges`] refuses calls before it looks at
    /// edges or cost at all.
    #[test]
    fn a_split_cost_terminator_presents_exactly_two_edges() {
        // (cpu, items) — every split-cost terminator shape both builders produce:
        // out of the body, to a body-closing local, and with no fall-through.
        let d0 = || CodeOperand::Reg(crate::value::Reg::D0);
        let shapes: Vec<(Cpu, Vec<CodeItem>)> = vec![
            (Cpu::Z80, vec![instr("djnz", vec![sym("Elsewhere")]), instr("ret", vec![])]),
            (
                Cpu::Z80,
                vec![instr("djnz", vec![sym(".done")]), instr("ret", vec![]), label(".done")],
            ),
            (Cpu::Z80, vec![instr("djnz", vec![sym("Elsewhere")])]),
            (
                Cpu::Z80,
                vec![instr("jr", vec![cc(Z80Cond::Z), sym("Elsewhere")]), instr("ret", vec![])],
            ),
            (
                Cpu::Z80,
                vec![
                    instr("jr", vec![cc(Z80Cond::Z), sym(".done")]),
                    instr("ret", vec![]),
                    label(".done"),
                ],
            ),
            (Cpu::Z80, vec![instr("jr", vec![cc(Z80Cond::Z), sym("Elsewhere")])]),
            (Cpu::Z80, vec![instr("ret", vec![cc(Z80Cond::Z)])]),
            (Cpu::M68000, vec![instr("beq", vec![sym("Elsewhere")]), instr("rts", vec![])]),
            (Cpu::M68000, vec![instr("beq", vec![sym("Elsewhere")])]),
            (
                Cpu::M68000,
                vec![instr("dbra", vec![d0(), sym("Elsewhere")]), instr("rts", vec![])],
            ),
        ];
        let mut swept = 0;
        for (cpu, items) in &shapes {
            let CodeItem::Instr { mnemonic, ops, .. } = &items[0] else { unreachable!() };
            let cost = walk_cost(*cpu, mnemonic, None, ops);
            assert!(
                matches!(cost, WalkCost::Split { .. }),
                "`{mnemonic}` must be a SPLIT cost for this sweep to mean anything"
            );
            let edges = cpu_edges(&Cfg::build(items), *cpu, 0);
            assert_eq!(edges.len(), 2, "`{mnemonic}` must present two edges, got {edges:?}");
            swept += 1;
        }
        assert_eq!(swept, 10, "the sweep covered {swept} shapes");

        // The other polarity: a single-edge terminator is not a split cost.
        for (cpu, items) in [
            (Cpu::M68000, vec![instr("jbra", vec![sym("Elsewhere")])]),
            (Cpu::Z80, vec![instr("jp", vec![sym("Elsewhere")])]),
        ] {
            let CodeItem::Instr { mnemonic, ops, .. } = &items[0] else { unreachable!() };
            assert_eq!(cpu_edges(&Cfg::build(&items), cpu, 0).len(), 1, "`{mnemonic}` is a tail");
            assert!(
                matches!(walk_cost(cpu, mnemonic, None, ops), WalkCost::Fixed { .. }),
                "an unconditional tail charges one number"
            );
        }

        // The near miss, machine-checked: `call cc` DOES carry a split cost with a
        // single edge. The guard never sees it only because the call bail is the
        // first refusal `charged_edges` makes — an ORDERING dependency, not a
        // property of the shape.
        let call_cc =
            vec![instr("call", vec![cc(Z80Cond::Nz), sym("Helper")]), instr("ret", vec![])];
        let CodeItem::Instr { mnemonic, ops, .. } = &call_cc[0] else { unreachable!() };
        assert!(matches!(walk_cost(Cpu::Z80, mnemonic, None, ops), WalkCost::Split { .. }));
        assert_eq!(cpu_edges(&Cfg::build(&call_cc), Cpu::Z80, 0).len(), 1);
        assert!(crate::context::is_call_mnemonic(mnemonic, Cpu::Z80), "the bail that saves it");

        // THE THIRD POLARITY, and the one that is NOT a terminator: each repeating
        // block op is a split cost presenting a single edge, with no call bail
        // ahead of it. These are the guard's live input. Swept over all eight, so
        // a family priced by copying one row cannot leave seven unexercised.
        let mut repeats = 0;
        for m in ["ldir", "lddr", "cpir", "cpdr", "inir", "indr", "otir", "otdr"] {
            let items = vec![instr(m, vec![]), instr("ret", vec![])];
            let CodeItem::Instr { mnemonic, ops, .. } = &items[0] else { unreachable!() };
            assert!(
                matches!(walk_cost(Cpu::Z80, mnemonic, None, ops), WalkCost::Split { .. }),
                "`{m}` must be a split cost for this polarity to mean anything"
            );
            assert_eq!(
                cpu_edges(&Cfg::build(&items), Cpu::Z80, 0).len(),
                1,
                "`{m}` re-executes itself; that is not an edge in this CFG"
            );
            assert!(!crate::context::is_call_mnemonic(mnemonic, Cpu::Z80), "no bail ahead of it");
            repeats += 1;
        }
        assert_eq!(repeats, 8, "the repeat sweep covered {repeats} of the eight block repeats");
    }

    // A call's cost is its callee's, which is not a local fact.
    #[test]
    fn a_call_is_opaque() {
        let items = vec![instr("call", vec![sym("Helper")]), instr("ret", vec![])];
        let e = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::OpaqueCall { mnemonic: "call".into() });
    }

    // A tail transfer to an external symbol leaves the accounted region.
    #[test]
    fn a_tail_transfer_out_is_unbounded() {
        let items = vec![instr("nop", vec![]), instr("jp", vec![sym("Elsewhere")])];
        let e = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap_err();
        assert_eq!(
            e.kind,
            BudgetFindingKind::UnboundedTransfer { mnemonic: "jp".into() }
        );
    }

    // Control running off the end of the body is the same hole as a tail transfer.
    #[test]
    fn a_fall_off_the_end_is_unbounded() {
        let items = vec![instr("nop", vec![]), instr("nop", vec![])];
        let e = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap_err();
        assert_eq!(
            e.kind,
            BudgetFindingKind::UnboundedTransfer { mnemonic: "nop".into() }
        );
    }

    // An op outside the T-state table has no assignable cost.
    //
    // The fixture was `ldir`, which is now PRICED — as a `Cost::Split`, because a
    // block repeat costs 21 T per iteration and 16 T on the one that leaves. It
    // is still a refusal here, and the arm below records which one: a split cost
    // over an instruction presenting a single edge is an ambiguous branch, not an
    // unknown op. `ex (sp),ix` replaces it because the encoder genuinely cannot
    // emit that form, so no future pricing pass can quietly make it known.
    #[test]
    fn an_off_table_op_is_unknown() {
        let ex_sp_ix = instr(
            "ex",
            vec![
                CodeOperand::Z80IndSp,
                CodeOperand::Z80Pair(crate::value::Z80Pair::Ix),
            ],
        );
        let items = vec![ex_sp_ix, instr("ret", vec![])];
        let e = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::UnknownOp { mnemonic: "ex".into() });
    }

    // A BLOCK REPEAT is refused, and the refusal names the right reason. The walk
    // charges `taken` to a branch edge and `not_taken` to the fall-through, and
    // `ldir` has neither: it re-executes itself, which is not an edge in this
    // CFG. So the two-edge precondition fails and the split cost is refused
    // rather than charged 16 to the single edge — an undercount by 21 T per
    // iteration, of which there may be 65 535.
    #[test]
    fn a_block_repeat_is_an_ambiguous_branch_not_an_unknown_op() {
        let items = vec![instr("ldir", vec![]), instr("ret", vec![])];
        let e = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::AmbiguousBranch { mnemonic: "ldir".into() });
        // The DISCRIMINATING neighbour: the single-step `ldi` costs a flat 16 and
        // walks cleanly, so the refusal above is about the repeat and not about
        // the block family being off-table.
        let stepped = vec![instr("ldi", vec![]), instr("ret", vec![])];
        let c = path_costs(&stepped, Cpu::Z80, sp(), &nr()).unwrap();
        assert_eq!((c.min, c.max), (26, 26)); // ldi 16 + ret 10
    }

    // A 68000 straight line measures through the M68000UM table: nop 4 + rts 16.
    #[test]
    fn a_68k_straight_line_measures() {
        let items = vec![instr("nop", vec![]), instr("rts", vec![])];
        let c = path_costs(&items, Cpu::M68000, sp(), &nr()).unwrap();
        assert_eq!((c.min, c.max), (20, 20));
        assert_eq!(c.inexact, None);
    }

    // A sized 68k conditional charges its own edges: beq.s taken 10 + rts 16 =
    // 26; not-taken 8 + moveq 4 + rts 16 = 28. Exact at a pinned width.
    #[test]
    fn a_68k_sized_conditional_charges_each_edge() {
        let items = vec![
            CodeItem::Instr {
                mnemonic: "beq".into(),
                size: Some(crate::value::Width::S),
                ops: vec![sym("skip")],
                span: sp(),
                as_type: None,
                targets: Vec::new(),
                author: crate::value::ItemAuthor::User,
            },
            instr("moveq", vec![CodeOperand::Imm(1), CodeOperand::Reg(crate::value::Reg::D0)]),
            label("skip"),
            instr("rts", vec![]),
        ];
        let c = path_costs(&items, Cpu::M68000, sp(), &nr()).unwrap();
        assert_eq!((c.min, c.max), (26, 28));
        assert_eq!(c.inexact, None);
    }

    // An UNSIZED 68k conditional relaxes at link time (.s/.w): the fall-through
    // is charged its `.w` ceiling and the walk records the inexact witness.
    #[test]
    fn an_unsized_68k_conditional_is_a_ceiling() {
        let items = vec![
            instr("beq", vec![sym("skip")]),
            instr("moveq", vec![CodeOperand::Imm(1), CodeOperand::Reg(crate::value::Reg::D0)]),
            label("skip"),
            instr("rts", vec![]),
        ];
        let c = path_costs(&items, Cpu::M68000, sp(), &nr()).unwrap();
        // taken 10 + 16 = 26; not-taken ceiling 12 + 4 + 16 = 32.
        assert_eq!((c.min, c.max), (26, 32));
        assert_eq!(c.inexact.as_deref(), Some("beq"));
    }

    // Every 68000 call form is opaque, `jbsr` included.
    #[test]
    fn a_68k_call_is_opaque() {
        for m in ["jsr", "bsr", "jbsr"] {
            let items = vec![instr(m, vec![sym("Helper")]), instr("rts", vec![])];
            let e = path_costs(&items, Cpu::M68000, sp(), &nr()).unwrap_err();
            assert_eq!(e.kind, BudgetFindingKind::OpaqueCall { mnemonic: m.into() });
        }
    }

    // A 68k back edge is refused like a Z80 one; `dbf` is a loop first.
    #[test]
    fn a_68k_loop_is_unbounded() {
        let items = vec![label("loop"), instr("nop", vec![]), instr("bra", vec![sym("loop")])];
        let e = path_costs(&items, Cpu::M68000, sp(), &nr()).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::UnboundedLoop);
        let items = vec![
            label("loop"),
            instr("nop", vec![]),
            instr("dbf", vec![CodeOperand::Reg(crate::value::Reg::D0), sym("loop")]),
            instr("rts", vec![]),
        ];
        let e = path_costs(&items, Cpu::M68000, sp(), &nr()).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::UnboundedLoop);
    }

    // A computed dispatch (`jmp .table(a1)` — the DMA-queue drain shape) is
    // refused BY NAME: its destination set is data, and calling it a transfer to
    // "code outside this proc" would be false — the jump table is right here.
    #[test]
    fn a_computed_transfer_is_refused_by_its_own_name() {
        let items = vec![
            instr(
                "jmp",
                vec![CodeOperand::DispSymInd {
                    target: "table".into(),
                    reg: crate::value::Reg::A1,
                }],
            ),
            label("table"),
            instr("rts", vec![]),
        ];
        let e = path_costs(&items, Cpu::M68000, sp(), &nr()).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::ComputedTransfer { mnemonic: "jmp".into() });
        // The Z80 twin: `jp (hl)` names no symbol either.
        let items = vec![instr("jp", vec![CodeOperand::Z80IndHl])];
        let e = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::ComputedTransfer { mnemonic: "jp".into() });
    }

    // A ceiling charge holds a budget and refuses an exactness proof — from the
    // SAME walk, each attribute concluding for itself.
    #[test]
    fn a_ceiling_holds_a_budget_but_not_an_exactness_proof() {
        // jbra to a local join: charged its dearest rung (12), then rts 16.
        let items = vec![
            instr("jbra", vec![sym("join")]),
            label("join"),
            instr("rts", vec![]),
        ];
        assert!(check_cycle_budget(&items, Cpu::M68000, sp(), Some(28), false, &nr()).is_empty());
        let f = check_cycle_budget(&items, Cpu::M68000, sp(), Some(27), true, &nr());
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].kind, BudgetFindingKind::OverBudget { worst: 28, budget: 27 });
        assert_eq!(f[1].kind, BudgetFindingKind::InexactCost { mnemonic: "jbra".into() });
    }

    // The 68000 fall-off refusal matches the Z80 one: the body ends without a
    // return, and closing that path at zero cost would under-report.
    #[test]
    fn a_68k_fall_off_the_end_is_unbounded() {
        let items = vec![instr("nop", vec![]), instr("nop", vec![])];
        let e = path_costs(&items, Cpu::M68000, sp(), &nr()).unwrap_err();
        assert_eq!(
            e.kind,
            BudgetFindingKind::UnboundedTransfer { mnemonic: "nop".into() }
        );
    }

    // An op the 68000 table does not price is refused by name (`link` is real
    // 68000 but absent from the corpus and the table).
    #[test]
    fn an_off_table_68k_op_is_unknown() {
        let items = vec![instr("link", vec![]), instr("rts", vec![])];
        let e = path_costs(&items, Cpu::M68000, sp(), &nr()).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::UnknownOp { mnemonic: "link".into() });
    }

    // The DMA drain group's own arithmetic, through the walk: 3× move.l
    // (a1)+,(a5) at 20 + move.w at 12 = 72 per entry — the dma_queue comment's
    // "72 cycles/entry" — plus rts 16.
    #[test]
    fn the_dma_drain_group_measures_72_per_entry() {
        let a1 = crate::value::Reg::A1;
        let a5 = crate::value::Reg::A5;
        let entry = |w: crate::value::Width| CodeItem::Instr {
            mnemonic: "move".into(),
            size: Some(w),
            ops: vec![CodeOperand::PostInc(a1), CodeOperand::Ind(a5)],
            span: sp(),
            as_type: None,
            targets: Vec::new(),
            author: crate::value::ItemAuthor::User,
        };
        let items = vec![
            entry(crate::value::Width::L),
            entry(crate::value::Width::L),
            entry(crate::value::Width::L),
            entry(crate::value::Width::W),
            instr("rts", vec![]),
        ];
        let c = path_costs(&items, Cpu::M68000, sp(), &nr()).unwrap();
        assert_eq!((c.min, c.max), (88, 88)); // 72 + 16, exact
        assert_eq!(c.inexact, None);
    }

    // A body with no instructions cannot hold a budget: its one path never
    // returns, and a zero-cost vacuous pass would certify paths that escape.
    #[test]
    fn an_empty_body_cannot_hold_a_budget() {
        let items = vec![label("only")];
        let e = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::EmptyBody);
        assert_eq!(
            path_costs(&[], Cpu::M68000, sp(), &nr()).unwrap_err().kind,
            BudgetFindingKind::EmptyBody
        );
    }

    // The budget compares against the WORST path, not the best.
    #[test]
    fn the_budget_is_checked_against_the_worst_path() {
        let items = vec![
            instr("jp", vec![cc(Z80Cond::Z), sym("skip")]),
            instr("nop", vec![]),
            label("skip"),
            instr("ret", vec![]),
        ];
        assert!(check_cycle_budget(&items, Cpu::Z80, sp(), Some(24), false, &nr()).is_empty());
        let f = check_cycle_budget(&items, Cpu::Z80, sp(), Some(23), false, &nr());
        assert_eq!(f.len(), 1);
        assert_eq!(
            f[0].kind,
            BudgetFindingKind::OverBudget { worst: 24, budget: 23 }
        );
    }

    // `@cycles_exact` fires on unequal paths and is silent on a padded pair.
    #[test]
    fn cycles_exact_proves_equal_paths() {
        let uneven = vec![
            instr("jp", vec![cc(Z80Cond::Z), sym("skip")]),
            instr("nop", vec![]),
            label("skip"),
            instr("ret", vec![]),
        ];
        let f = check_cycle_budget(&uneven, Cpu::Z80, sp(), None, true, &nr());
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, BudgetFindingKind::PathMismatch { min: 20, max: 24 });
        // Rejoining the arms costs the fall-through a `jp .join` (10 T), so the
        // arms are 34 and 36 — closer, still unequal.
        let rejoined = vec![
            instr("jp", vec![cc(Z80Cond::Z), sym("skip")]),
            instr("nop", vec![]),
            instr("jp", vec![sym("join")]),
            label("skip"),
            instr("nop", vec![]),
            instr("nop", vec![]),
            instr("nop", vec![]),
            instr("nop", vec![]),
            label("join"),
            instr("ret", vec![]),
        ];
        // taken: 10 + 4*4 + 10 = 36 ; not-taken: 10 + 4 + 10 + 10 = 34.
        assert_eq!(
            check_cycle_budget(&rejoined, Cpu::Z80, sp(), None, true, &nr())[0].kind,
            BudgetFindingKind::PathMismatch { min: 34, max: 36 }
        );
        // Spelling the short arm's join as `jr` (12 T) rather than `jp` (10 T)
        // buys the missing 2 T-states — the same substitution `pad_to_cycles`'s
        // dense mode makes, and here it is machine-checked instead of counted.
        let mut balanced = rejoined.clone();
        balanced[2] = instr("jr", vec![sym("join")]);
        assert!(check_cycle_budget(&balanced, Cpu::Z80, sp(), None, true, &nr()).is_empty());
    }

    // A proc declaring neither attribute is not walked at all, so an unbounded
    // shape stays silent until someone claims a budget for it.
    #[test]
    fn an_undeclared_proc_is_not_walked() {
        let items = vec![label("loop"), instr("jp", vec![sym("loop")])];
        assert!(check_cycle_budget(&items, Cpu::Z80, sp(), None, false, &nr()).is_empty());
    }

    // Both conclusions come off ONE walk, so a proc carrying both attributes
    // reports both.
    #[test]
    fn both_attributes_report_together() {
        let items = vec![
            instr("jp", vec![cc(Z80Cond::Z), sym("skip")]),
            instr("nop", vec![]),
            label("skip"),
            instr("ret", vec![]),
        ];
        let f = check_cycle_budget(&items, Cpu::Z80, sp(), Some(20), true, &nr());
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].kind.lint_id(), "cycles.over-budget");
        assert_eq!(f[1].kind.lint_id(), "cycles.path-mismatch");
    }

    // A bail wins over both conclusions: an unmeasurable proc reports why, and
    // never a number.
    #[test]
    fn a_bail_replaces_the_conclusions() {
        let items = vec![instr("call", vec![sym("Helper")]), instr("ret", vec![])];
        let f = check_cycle_budget(&items, Cpu::Z80, sp(), Some(1), true, &nr());
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind.lint_id(), "cycles.opaque-call");
    }

    // A shared join is costed once, not once per path reaching it — the walk is
    // a DAG longest/shortest path, not a path enumeration.
    #[test]
    fn a_reconverging_chain_does_not_blow_up() {
        // Ten nested diamonds: 2^10 paths, but 41 instructions.
        let mut items = Vec::new();
        for i in 0..10 {
            items.push(instr("jp", vec![cc(Z80Cond::Z), sym(&format!("r{i}"))]));
            items.push(instr("nop", vec![]));
            items.push(instr("jp", vec![sym(&format!("j{i}"))]));
            items.push(label(&format!("r{i}")));
            items.push(instr("nop", vec![]));
            items.push(instr("nop", vec![]));
            items.push(label(&format!("j{i}")));
        }
        items.push(instr("ret", vec![]));
        let c = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap();
        // per diamond: left 10+4+10 = 24, right 10+4+4 = 18; + ret 10.
        assert_eq!((c.min, c.max), (18 * 10 + 10, 24 * 10 + 10));
    }

    // A `djnz` is both outcome-split AND a loop; the loop refusal is the one that
    // matters and it is what the walk reports.
    #[test]
    fn a_djnz_loop_is_reported_as_a_loop() {
        let items = vec![
            label("loop"),
            instr("nop", vec![]),
            instr("djnz", vec![sym("loop")]),
            instr("ret", vec![]),
        ];
        let e = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::UnboundedLoop);
    }

    // A CONDITIONAL return at the very end of a body presents [Return, FallOff]:
    // the taken edge returns, the other runs off the end. Only the first is a path
    // end — charging both would close an escaping path at zero cost and report a
    // bound that is TOO LOW, which is the one error direction a budget must not
    // make.
    #[test]
    fn a_tail_conditional_return_refuses_its_fall_through() {
        let items = vec![instr("nop", vec![]), instr("ret", vec![cc(Z80Cond::Z)])];
        let e = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap_err();
        assert_eq!(
            e.kind,
            BudgetFindingKind::UnboundedTransfer { mnemonic: "ret".into() }
        );
        // The twin: give the fall-through somewhere to go and the same shape
        // measures — 4 + 11 = 15 taken, 4 + 5 + 10 = 19 through.
        let closed = vec![
            instr("nop", vec![]),
            instr("ret", vec![cc(Z80Cond::Z)]),
            instr("ret", vec![]),
        ];
        let c = path_costs(&closed, Cpu::Z80, sp(), &nr()).unwrap();
        assert_eq!((c.min, c.max), (15, 19));
    }

    // Inline data in a code stream emits BYTES that decode as instructions if
    // control reaches them, and the CFG links straight across it — so a body
    // carrying any is refused rather than costed as if the bytes were free.
    #[test]
    fn inline_data_is_refused() {
        let items = vec![
            instr("nop", vec![]),
            CodeItem::Inline(crate::value::DataBuf { cells: Vec::new(), size: 4 }, sp()),
            instr("ret", vec![]),
        ];
        let e = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::InlineData);
    }

    // The T-state table is shared with `cycles(L1, L2)`: the same span costs the
    // same through both consumers.
    #[test]
    fn the_walk_and_the_span_builtin_agree() {
        let body = vec![
            instr("ld", vec![a(), CodeOperand::Z80IndHl]), // 7
            instr("ld", vec![CodeOperand::Z80IndDe, a()]), // 7
            instr("nop", vec![]),                          // 4
        ];
        assert_eq!(crate::z80_cycles::span_cost(&body).unwrap(), 18);
        let mut items = body;
        items.push(instr("ret", vec![])); // 10
        let c = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap();
        assert_eq!((c.min, c.max), (28, 28));
    }

    // ---- enumerated dispatch — `targets(...)` -----------------------------

    use crate::value::{ItemAuthor, Reg, Width};

    /// A `jmp .disp(reg)` computed transfer carrying a `targets(...)` clause. The
    /// `DispSymInd` operand names no symbol, so its edge is a bare `TailOut` — the
    /// enumerable shape.
    fn jmp_targets(disp: &str, reg: Reg, targets: &[&str]) -> CodeItem {
        CodeItem::Instr {
            mnemonic: "jmp".into(),
            size: None,
            ops: vec![CodeOperand::DispSymInd { target: disp.into(), reg }],
            span: sp(),
            as_type: None,
            targets: targets.iter().map(|s| s.to_string()).collect(),
            author: ItemAuthor::User,
        }
    }

    /// `move.l (a1)+, (a5)` — one drain send word, 20 cycles exact.
    fn drain_move() -> CodeItem {
        CodeItem::Instr {
            mnemonic: "move".into(),
            size: Some(Width::L),
            ops: vec![CodeOperand::PostInc(Reg::A1), CodeOperand::Ind(Reg::A5)],
            span: sp(),
            as_type: None,
            targets: Vec::new(),
            author: ItemAuthor::User,
        }
    }

    /// The dma-drain shape in miniature: a computed `jmp` enumerated over three
    /// local labels, the drain groups falling through each other. The walk sees
    /// THROUGH the dispatch — `jmp (d16,An)` 10, then max over the three arms.
    fn dispatch_fixture() -> Vec<CodeItem> {
        vec![
            jmp_targets("table", Reg::A1, &["done", "drain_2", "drain_1"]), // 0: entry, 10
            label("table"),
            instr("rts", vec![]),  // unreachable — reached only by an address the walk can't name
            label("drain_2"),
            drain_move(), // 20
            label("drain_1"),
            drain_move(), // 20
            instr("rts", vec![]), // 16
            label("done"),
            instr("rts", vec![]), // 16
        ]
    }

    // The enumerated dispatch is MEASURED: the dearest arm (`drain_2`) drains two
    // groups then returns (10 + 20 + 20 + 16 = 66); the `done` arm is 10 + 16 = 26.
    #[test]
    fn enumerated_targets_measures_a_computed_dispatch() {
        let c = path_costs(&dispatch_fixture(), Cpu::M68000, sp(), &nr()).unwrap();
        assert_eq!((c.min, c.max), (26, 66));
        assert_eq!(c.inexact, None);
    }

    // The SAME `jmp` WITHOUT the clause is the pre-form behavior: a computed
    // transfer the walk refuses BY NAME. The clause is the ONLY thing that lets
    // the budget see through it.
    #[test]
    fn without_targets_the_same_jmp_refuses() {
        let mut items = dispatch_fixture();
        // Strip the clause off the entry jmp; everything else is identical.
        if let CodeItem::Instr { targets, .. } = &mut items[0] {
            targets.clear();
        }
        let e = path_costs(&items, Cpu::M68000, sp(), &nr()).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::ComputedTransfer { mnemonic: "jmp".into() });
    }

    // ORTHOGONALITY (spec §2): the clause changes NOTHING the other analyses see.
    // Not just the edge model — the actual VERDICTS. The preserves prover, the
    // flag def-use walk, and the stack-balance verdict all follow `flag_check::Cfg`
    // edges, and those are a single `TailOut` for the computed jmp with the clause
    // and without it. A future consumer that read `targets` directly (turning the
    // `TailOut` into `Follow` edges into the drains) would change all three verdicts
    // — this pin flips red the day that happens.
    #[test]
    fn enumerated_targets_leave_the_base_analyses_untouched() {
        use crate::preserves::{check_stack_balance, verify_preserved, CallPolicy};
        let with = dispatch_fixture();
        let mut without = with.clone();
        if let CodeItem::Instr { targets, .. } = &mut without[0] {
            targets.clear();
        }
        // The edge model itself: identical, and a single `TailOut`.
        let e_with = crate::flag_check::Cfg::build(&with).edges(0);
        assert_eq!(e_with, crate::flag_check::Cfg::build(&without).edges(0));
        assert_eq!(e_with, vec![Edge::TailOut]);
        // The preserves prover's verdict on the drain registers.
        let regs = [crate::value::Reg::A1, crate::value::Reg::A5];
        assert_eq!(
            verify_preserved(&with, &regs, CallPolicy::ClobberAll, None, &BTreeSet::new()),
            verify_preserved(&without, &regs, CallPolicy::ClobberAll, None, &BTreeSet::new()),
        );
        // The flag def-use walk's verdict.
        let no_callees = BTreeMap::new();
        assert_eq!(
            crate::flag_check::check_flag_unused("f", &with, &no_callees, &[], Cpu::M68000),
            crate::flag_check::check_flag_unused("f", &without, &no_callees, &[], Cpu::M68000),
        );
        // The stack-balance verdict.
        assert_eq!(check_stack_balance(&with, true), check_stack_balance(&without, true));
    }

    // PERTURBATION (spec §5): add one send group to the dearest arm and the worst
    // path rises by exactly that group's 20 cycles — a budget pinned at the old
    // ceiling now fires. The enumeration tracks the arms, not a frozen number.
    #[test]
    fn a_perturbed_drain_arm_moves_the_budget() {
        assert!(check_cycle_budget(&dispatch_fixture(), Cpu::M68000, sp(), Some(66), false, &nr()).is_empty());
        let mut items = dispatch_fixture();
        items.insert(4, drain_move()); // an extra group at the head of `drain_2`
        let c = path_costs(&items, Cpu::M68000, sp(), &nr()).unwrap();
        assert_eq!(c.max, 86);
        let f = check_cycle_budget(&items, Cpu::M68000, sp(), Some(66), false, &nr());
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, BudgetFindingKind::OverBudget { worst: 86, budget: 66 });
    }

    // THE DRAIN-LABEL TRAP (soundness regression pin): a `targets(...)` label must
    // name the PHYSICAL landing point, not a label DOWNSTREAM of it. A jump-table
    // dispatch lands on a slot whose prologue (here two `nop`s standing in for the
    // real `lea/lea`) executes BEFORE control reaches the drain. Naming the landing
    // label charges that prologue; naming the drain label DOWNSTREAM of it fans the
    // walk PAST the prologue and under-counts by exactly its cost — an unsound
    // ceiling the hardware exceeds. The pin proves the walk charges landed-on code:
    // the landing enumeration MUST measure dearer than the downstream one.
    #[test]
    fn targets_charge_the_landed_on_code_not_a_downstream_label() {
        let build = |target: &str| {
            vec![
                jmp_targets("slot", Reg::A1, &[target]),
                label("landing"),
                instr("nop", vec![]), // 4 — the slot's dispatch prologue
                instr("nop", vec![]), // 4
                label("drain"),
                drain_move(),         // 20
                instr("rts", vec![]), // 16
            ]
        };
        let landing = path_costs(&build("landing"), Cpu::M68000, sp(), &nr()).unwrap();
        let downstream = path_costs(&build("drain"), Cpu::M68000, sp(), &nr()).unwrap();
        // jmp 10 + [prologue 4+4] + drain 20 + rts 16 vs jmp 10 + drain 20 + rts 16.
        assert_eq!((landing.max, downstream.max), (54, 46));
        assert_eq!(landing.max - downstream.max, 8, "the drain-label spelling skips the 8-cycle prologue");
    }

    // An enumerated target that leads back onto the path is a cycle, and falls to
    // the existing unbounded-loop refusal with no new rule — the topo walk sees
    // the enumerated successor exactly as the cost pass does.
    #[test]
    fn an_enumerated_cycle_is_unbounded() {
        let items = vec![
            label("top"),
            instr("nop", vec![]),
            jmp_targets("t", Reg::A1, &["top"]),
        ];
        let e = path_costs(&items, Cpu::M68000, sp(), &nr()).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::UnboundedLoop);
    }

    // ---- the `targets(...)` validity refusals (§1) ------------------------

    // A clause on a CALL is refused: a call's cost is callee-cost composition, not
    // this form.
    #[test]
    fn targets_on_a_call_is_refused() {
        let items = vec![
            CodeItem::Instr {
                mnemonic: "jsr".into(),
                size: None,
                ops: vec![CodeOperand::Ind(Reg::A1)],
                span: sp(),
                as_type: None,
                targets: vec!["done".into()],
                author: ItemAuthor::User,
            },
            label("done"),
            instr("rts", vec![]),
        ];
        let f = check_dispatch_targets(&items, Cpu::M68000);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, DispatchFindingKind::OnCall { mnemonic: "jsr".into() });
    }

    // A clause on a DIRECT `jmp .label` (an already-exact edge) is redundant.
    #[test]
    fn targets_on_a_direct_jmp_is_redundant() {
        let items = vec![
            CodeItem::Instr {
                mnemonic: "jmp".into(),
                size: None,
                ops: vec![sym("done")],
                span: sp(),
                as_type: None,
                targets: vec!["done".into()],
                author: ItemAuthor::User,
            },
            label("done"),
            instr("rts", vec![]),
        ];
        let f = check_dispatch_targets(&items, Cpu::M68000);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, DispatchFindingKind::Redundant { mnemonic: "jmp".into() });
    }

    // An unknown local label, a nonlocal target, and a duplicate are each named.
    #[test]
    fn targets_name_checks_fire() {
        let unknown = vec![jmp_targets("t", Reg::A1, &[".typo"]), instr("rts", vec![])];
        let f = check_dispatch_targets(&unknown, Cpu::M68000);
        assert_eq!(f, vec![DispatchFinding {
            kind: DispatchFindingKind::Unknown { label: ".typo".into() },
            span: sp(),
        }]);

        let nonlocal = vec![jmp_targets("t", Reg::A1, &["OtherProc"]), instr("rts", vec![])];
        let f = check_dispatch_targets(&nonlocal, Cpu::M68000);
        assert_eq!(f, vec![DispatchFinding {
            kind: DispatchFindingKind::Nonlocal { label: "OtherProc".into() },
            span: sp(),
        }]);

        let dup = vec![
            jmp_targets("t", Reg::A1, &["done", "done"]),
            label("done"),
            instr("rts", vec![]),
        ];
        let f = check_dispatch_targets(&dup, Cpu::M68000);
        assert_eq!(f, vec![DispatchFinding {
            kind: DispatchFindingKind::Duplicate { label: "done".into() },
            span: sp(),
        }]);
    }

    // A well-formed enumeration draws no refusal — the passing twin, so a green
    // cannot mean the check is dead.
    #[test]
    fn a_valid_enumeration_is_silent() {
        assert!(check_dispatch_targets(&dispatch_fixture(), Cpu::M68000).is_empty());
    }

    // A target naming a TRAILING label (one that closes the body, no instruction
    // after it) is a fall-off in disguise, refused BY NAME — otherwise the checker
    // accepts it and the walk then refuses the transfer as computed, a contradiction.
    #[test]
    fn a_trailing_label_target_is_refused() {
        let items = vec![
            jmp_targets("t", Reg::A1, &["done"]),
            drain_move(),
            instr("rts", vec![]),
            label("done"), // trailing: closes the proc, no instruction after it
        ];
        let f = check_dispatch_targets(&items, Cpu::M68000);
        assert_eq!(
            f,
            vec![DispatchFinding {
                kind: DispatchFindingKind::Trailing { label: "done".into() },
                span: sp(),
            }]
        );
    }

    // The Z80 twin: `jp (hl) targets(.a)` — a computed transfer naming no symbol —
    // enumerates just like the 68k `jmp .table(a1)`. `jp (hl)` 4 + nop 4 + ret 10.
    #[test]
    fn a_z80_computed_jp_enumerates() {
        let items = vec![
            CodeItem::Instr {
                mnemonic: "jp".into(),
                size: None,
                ops: vec![CodeOperand::Z80IndHl],
                span: sp(),
                as_type: None,
                targets: vec!["a".into()],
                author: ItemAuthor::User,
            },
            label("a"),
            instr("nop", vec![]),
            instr("ret", vec![]),
        ];
        assert!(check_dispatch_targets(&items, Cpu::Z80).is_empty());
        let c = path_costs(&items, Cpu::Z80, sp(), &nr()).unwrap();
        assert_eq!((c.min, c.max), (18, 18));
    }

    // ---- noreturn-tail model: divergent terminals close the budget walk -------

    // An assert / raise rail's `jmp (pages).l` DIVERGES: the compiler authored it
    // (`ItemAuthor::AssertDesugar`) knowing it never returns, so its terminal
    // transfer closes the path like a return and the proc stays MEASURABLE. A
    // HAND-written `jmp` to the same symbol stays a plain unbounded transfer —
    // authorship is the only distinguisher (spec §1(b)). The refuses-before /
    // verifies-after pin the spec's §5 bar names.
    #[test]
    fn an_authored_rail_terminal_closes_the_budget_where_a_hand_jmp_refuses() {
        // BEFORE: a User `jmp (pages)` is an unbounded transfer.
        let user = vec![instr("nop", vec![]), instr("jmp", vec![sym("pages")])];
        let e = path_costs(&user, Cpu::M68000, sp(), &nr()).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::UnboundedTransfer { mnemonic: "jmp".into() });
        // AFTER: the SAME jmp stamped as the desugar's closes the path — nop (4)
        // + jmp abs.l ceiling (12) = 16, and the budget verifies.
        let rail = vec![
            instr("nop", vec![]),
            instr_authored("jmp", vec![sym("pages")], ItemAuthor::AssertDesugar),
        ];
        let c = path_costs(&rail, Cpu::M68000, sp(), &nr()).unwrap();
        assert_eq!((c.min, c.max), (16, 16));
        assert!(check_cycle_budget(&rail, Cpu::M68000, sp(), Some(16), false, &nr()).is_empty());
        // A User jmp to the same symbol is unmeasurable — no `@noreturn`, no author.
        let over = check_cycle_budget(&user, Cpu::M68000, sp(), Some(16), false, &nr());
        assert_eq!(over[0].kind, BudgetFindingKind::UnboundedTransfer { mnemonic: "jmp".into() });
    }

    // A tail transfer to a `@noreturn`-declared target closes the path: the
    // divergence is stated by the target's contract, not by authorship, so a
    // HAND-written `jbra GameLoop` measures when `GameLoop` is `@noreturn` and
    // refuses when it is not.
    #[test]
    fn a_tail_to_a_noreturn_target_closes_the_path() {
        let items = vec![instr("nop", vec![]), instr("jbra", vec![sym("Diverge")])];
        // Not marked: an unbounded transfer.
        let e = path_costs(&items, Cpu::M68000, sp(), &nr()).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::UnboundedTransfer { mnemonic: "jbra".into() });
        // Marked `@noreturn`: the transfer is a terminal, the path closes.
        let c = path_costs(&items, Cpu::M68000, sp(), &nr_of(&["Diverge"])).unwrap();
        // nop (4) + jbra's `jmp abs.l` rung ceiling (12).
        assert_eq!((c.min, c.max), (16, 16));
        assert!(check_cycle_budget(&items, Cpu::M68000, sp(), Some(16), false, &nr_of(&["Diverge"])).is_empty());
    }

    // S3: the `@noreturn` match reads the UNIFIED target extractor, so an absolute
    // long `jmp (Diverge).l` (an `AbsSym`, which `branch_target` cannot see)
    // matches a `@noreturn` symbol and closes the path.
    #[test]
    fn a_tail_via_abs_long_to_a_noreturn_target_closes_the_path() {
        let items = vec![
            instr("nop", vec![]),
            instr("jmp", vec![CodeOperand::AbsSym { target: "Diverge".into(), long: true }]),
        ];
        // Not marked: unbounded (an AbsSym still names a target, so not computed).
        assert!(path_costs(&items, Cpu::M68000, sp(), &nr()).is_err());
        // Marked: the AbsSym target matches, the path closes and measures.
        let c = path_costs(&items, Cpu::M68000, sp(), &nr_of(&["Diverge"])).unwrap();
        assert_eq!((c.min, c.max), (16, 16)); // nop 4 + jmp abs.l 12
    }

    // A CONDITIONAL branch's taken edge to a `@noreturn` target diverges while its
    // fall-through continues — the two-edge split still routes each edge its own
    // cost, and only the diverging side closes.
    #[test]
    fn a_conditional_branch_to_a_noreturn_target_closes_only_the_taken_edge() {
        let items = vec![
            instr("beq", vec![sym("Diverge")]), // taken -> @noreturn (closes); fall -> rts
            instr("rts", vec![]),
        ];
        // Without the mark the taken edge is an unbounded transfer.
        assert!(path_costs(&items, Cpu::M68000, sp(), &nr()).is_err());
        // With it, the walk measures: the fall-through path (beq not-taken + rts).
        let c = path_costs(&items, Cpu::M68000, sp(), &nr_of(&["Diverge"])).unwrap();
        // The path that RETURNS is beq(not-taken) + rts; the diverging taken edge
        // closes with its own cost and contributes no successor. Both are finite.
        assert!(c.max >= c.min && c.min > 0);
    }
}
