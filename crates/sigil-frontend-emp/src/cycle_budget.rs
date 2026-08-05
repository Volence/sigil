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
//! Per-instruction cost comes from [`crate::z80_cycles::instr_cost`] — the one
//! Z80 T-state table, shared with the `cycles(L1, L2)` builtin. Control flow
//! comes from the shared [`Cfg`], so this walk sees the same edges every other
//! per-proc analysis does.
//!
//! An exact worst-case cost is only definable over a FINITE, LOCAL, TOTALLY
//! MODELED path set, so everything outside that is a loud refusal rather than a
//! guess — the same stance the T-state table itself takes toward an op it does
//! not enumerate. A budget is a claim its author opted into, and the honest
//! downgrade for a proc that cannot carry one is free: delete the attribute.
//!
//! | shape | diagnostic | why it cannot be bounded |
//! |---|---|---|
//! | a back edge (any loop) | `[cycles.unbounded-loop]` | the longest path through a cycle is unbounded; no trip count is declared |
//! | `call` / `rst` | `[cycles.opaque-call]` | the callee's cost is not a local fact |
//! | a tail transfer out, or control off the end of the body | `[cycles.unbounded-transfer]` | the path continues into code this walk cannot see |
//! | an op outside the T-state table | `[cycles.unknown-op]` | no cost is assignable |
//! | an outcome-split conditional whose two edges cannot be told apart | `[cycles.ambiguous-branch]` | the two costs cannot be routed to their edges |
//! | a 68000 body | `[cycles.unmodeled-cpu]` | no 68000 timing model exists yet |
//!
//! Only a RETURN instruction ends a charged path. That single rule is what makes
//! the bound sound in the one direction that matters: every other way out of the
//! body leaves cost unaccounted, so it is refused rather than silently treated as
//! zero.
//!
//! Nominal 68000/Z80 clocks are the unit. Bus contention (the Z80 losing the bus
//! to a 68000 DMA, VDP-port wait states) is NOT modeled and cannot be — it is a
//! whole-machine fact, not a proc-local one. A budget is therefore a bound on
//! ISSUED cycles, which is what the sound driver's own balance proof means by
//! T-states.

use crate::flag_check::{instr_span, Cfg, Edge};
use crate::value::CodeItem;
use crate::z80_cycles::{instr_cost, Cost};
use sigil_ir::backend::Cpu;
use sigil_span::Span;
use std::collections::BTreeMap;

/// Z80 mnemonics that RETURN from a proc — the only charged path ends.
const Z80_RETURN_MNEMONICS: [&str; 3] = ["ret", "reti", "retn"];

/// Z80 mnemonics that CALL and come back, whose callee cost is not local.
const Z80_CALL_MNEMONICS: [&str; 2] = ["call", "rst"];

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
    /// `[cycles.ambiguous-branch]` — an outcome-split conditional whose taken and
    /// fall-through edges cannot be told apart.
    AmbiguousBranch {
        /// The conditional's mnemonic.
        mnemonic: String,
    },
    /// `[cycles.unknown-op]` — an op outside the T-state table.
    UnknownOp {
        /// The unmodeled mnemonic.
        mnemonic: String,
    },
    /// `[cycles.unmodeled-cpu]` — no timing model exists for this CPU.
    UnmodeledCpu,
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
            Self::AmbiguousBranch { .. } => "cycles.ambiguous-branch",
            Self::UnknownOp { .. } => "cycles.unknown-op",
            Self::UnmodeledCpu => "cycles.unmodeled-cpu",
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
                 path cost is unbounded — a cycle budget needs a loop-free body"
                .to_string(),
            Self::OpaqueCall { mnemonic } => format!(
                "`{mnemonic}` costs whatever its callee costs, which is not a fact about \
                 this proc — a cycle budget needs a call-free body"
            ),
            Self::UnboundedTransfer { mnemonic } => format!(
                "`{mnemonic}` continues into code outside this proc, so the path's cost is \
                 not accounted here — a cycle budget needs every path to end at a return"
            ),
            Self::AmbiguousBranch { mnemonic } => format!(
                "`{mnemonic}` costs differently taken and not-taken, and its two edges are \
                 not distinguishable here"
            ),
            Self::UnknownOp { mnemonic } => format!(
                "`{mnemonic}` is not in the T-state table — add it to `z80_cycles` if a \
                 budgeted proc legitimately needs it"
            ),
            Self::UnmodeledCpu => "cycle budgets model the Z80 only; there is no 68000 \
                 timing model yet"
                .to_string(),
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathCosts {
    /// The cheapest path from entry to a return.
    pub min: u64,
    /// The dearest path from entry to a return.
    pub max: u64,
}

/// Measure a proc body's path costs, or say why it cannot be measured.
///
/// `items` is the proc's evaluated `CodeBuf`; `cpu` selects the timing model and
/// the edge model. A body with no instructions costs zero on its one empty path.
pub fn path_costs(items: &[CodeItem], cpu: Cpu) -> Result<PathCosts, BudgetFinding> {
    if cpu != Cpu::Z80 {
        let span = items
            .iter()
            .find_map(|it| match it {
                CodeItem::Instr { span, .. } => Some(*span),
                _ => None,
            })
            .unwrap_or(Span { source: sigil_span::SourceId(0), start: 0, end: 0 });
        return Err(BudgetFinding { kind: BudgetFindingKind::UnmodeledCpu, span });
    }
    let Some(entry) = entry_instr_idx(items) else {
        return Ok(PathCosts { min: 0, max: 0 });
    };
    let cfg = Cfg::build(items);
    // A DFS that (a) proves the reachable subgraph is acyclic and (b) leaves a
    // post-order, whose reverse is a topological order. The two jobs share one
    // walk because a back edge is exactly what makes the second impossible.
    let order = topo_order(&cfg, items, entry)?;
    // Reverse post-order gives every successor a cost before its predecessor,
    // so one backward pass over `order` fills the whole table.
    let mut best: BTreeMap<usize, (u64, u64)> = BTreeMap::new();
    for &idx in &order {
        let (mnem, ops) = cfg.instr(idx).expect("topo order holds instruction indices only");
        let cost = instr_cost(mnem, ops);
        let span = instr_span(items, idx).expect("an instruction carries a span");
        let charged = charged_edges(&cfg, idx, mnem, cost, span)?;
        let mut lo = u64::MAX;
        let mut hi = 0u64;
        for (edge_cost, succ) in charged {
            let (smin, smax) = match succ {
                // The path ends here: a return contributes only its own cost.
                None => (0, 0),
                Some(s) => *best.get(&s).expect("successors are ordered before predecessors"),
            };
            lo = lo.min(edge_cost + smin);
            hi = hi.max(edge_cost + smax);
        }
        best.insert(idx, (lo, hi));
    }
    let (min, max) = best[&entry];
    Ok(PathCosts { min, max })
}

/// Check a proc's declared cycle contract. `budget` is the `@budget(cycles: N)`
/// ceiling and `exact` the presence of `@cycles_exact`; a proc declaring neither
/// is not walked.
pub fn check_cycle_budget(
    items: &[CodeItem],
    cpu: Cpu,
    budget: Option<u64>,
    exact: bool,
) -> Vec<BudgetFinding> {
    if budget.is_none() && !exact {
        return Vec::new();
    }
    let costs = match path_costs(items, cpu) {
        Ok(c) => c,
        Err(f) => return vec![f],
    };
    let mut out = Vec::new();
    // The declaration itself is what is violated, so both findings point at the
    // proc's first instruction rather than at an arbitrary interior site.
    let span = items
        .iter()
        .find_map(|it| match it {
            CodeItem::Instr { span, .. } => Some(*span),
            _ => None,
        })
        .unwrap_or(Span { source: sigil_span::SourceId(0), start: 0, end: 0 });
    if let Some(n) = budget {
        if costs.max > n {
            out.push(BudgetFinding {
                kind: BudgetFindingKind::OverBudget { worst: costs.max, budget: n },
                span,
            });
        }
    }
    if exact && costs.min != costs.max {
        out.push(BudgetFinding {
            kind: BudgetFindingKind::PathMismatch { min: costs.min, max: costs.max },
            span,
        });
    }
    out
}

/// The item index of a body's FIRST instruction — the walk's entry point.
fn entry_instr_idx(items: &[CodeItem]) -> Option<usize> {
    items.iter().position(|it| matches!(it, CodeItem::Instr { .. }))
}

/// The charged successors of `idx`: `(edge cost, successor)`, where `None` means
/// the path ENDS here (a return). Every other way out of the body is refused.
///
/// An outcome-split conditional's two costs are routed by the edge model's
/// TAKEN-FIRST ordering: every conditional arm of `Cfg::z80_edges` pushes the
/// branch edge before the fall-through. A form that does not present exactly two
/// edges cannot be routed and is refused rather than charged the wrong number.
#[allow(clippy::type_complexity)]
fn charged_edges(
    cfg: &Cfg,
    idx: usize,
    mnem: &str,
    cost: Cost,
    span: Span,
) -> Result<Vec<(u64, Option<usize>)>, BudgetFinding> {
    let bail = |kind| Err(BudgetFinding { kind, span });
    if Z80_CALL_MNEMONICS.contains(&mnem) {
        return bail(BudgetFindingKind::OpaqueCall { mnemonic: mnem.to_string() });
    }
    let base = match cost {
        Cost::Fixed(n) => Some(u64::from(n)),
        Cost::Split { .. } => None,
        Cost::Unknown => {
            return bail(BudgetFindingKind::UnknownOp { mnemonic: mnem.to_string() })
        }
    };
    let is_return = Z80_RETURN_MNEMONICS.contains(&mnem);
    let edges = cfg.z80_edges(idx);
    if base.is_none() && edges.len() != 2 {
        return bail(BudgetFindingKind::AmbiguousBranch { mnemonic: mnem.to_string() });
    }
    let mut out = Vec::new();
    for (i, e) in edges.iter().enumerate() {
        let c = match (base, cost) {
            (Some(n), _) => n,
            // Edge 0 is the branch edge, edge 1 the fall-through.
            (None, Cost::Split { taken, not_taken }) => {
                u64::from(if i == 0 { taken } else { not_taken })
            }
            (None, _) => unreachable!("only Cost::Split leaves the base cost open"),
        };
        match e {
            Edge::Follow(s) => out.push((c, Some(*s))),
            // A return ENDS the path; control running off the end of the body
            // continues into whatever the section places next.
            Edge::Abandon if is_return => out.push((c, None)),
            Edge::Abandon => {
                return bail(BudgetFindingKind::UnboundedTransfer {
                    mnemonic: mnem.to_string(),
                })
            }
            Edge::Defer => {
                return bail(BudgetFindingKind::UnboundedTransfer {
                    mnemonic: mnem.to_string(),
                })
            }
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

/// A post-order over the instructions reachable from `entry`, proving on the way
/// that the reachable subgraph is ACYCLIC. A back edge (an edge to an instruction
/// still open on the DFS stack) is `[cycles.unbounded-loop]`, reported at the
/// instruction the loop re-enters.
fn topo_order(cfg: &Cfg, items: &[CodeItem], entry: usize) -> Result<Vec<usize>, BudgetFinding> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Open,
        Done,
    }
    let mut mark: BTreeMap<usize, Mark> = BTreeMap::new();
    let mut order = Vec::new();
    let succs = |i: usize| -> Vec<usize> {
        cfg.z80_edges(i)
            .into_iter()
            .filter_map(|e| match e {
                Edge::Follow(s) => Some(s),
                _ => None,
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
        let c = path_costs(&items, Cpu::Z80).unwrap();
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
        let c = path_costs(&items, Cpu::Z80).unwrap();
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
        let c = path_costs(&items, Cpu::Z80).unwrap();
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
        let c = path_costs(&items, Cpu::Z80).unwrap();
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
        let e = path_costs(&items, Cpu::Z80).unwrap_err();
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
        let c = path_costs(&items, Cpu::Z80).unwrap();
        // left: 10 + 4 + 10 + 10 = 34 ; right: 10 + 4 + 4 + 10 = 28
        assert_eq!((c.min, c.max), (28, 34));
    }

    // A call's cost is its callee's, which is not a local fact.
    #[test]
    fn a_call_is_opaque() {
        let items = vec![instr("call", vec![sym("Helper")]), instr("ret", vec![])];
        let e = path_costs(&items, Cpu::Z80).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::OpaqueCall { mnemonic: "call".into() });
    }

    // A tail transfer to an external symbol leaves the accounted region.
    #[test]
    fn a_tail_transfer_out_is_unbounded() {
        let items = vec![instr("nop", vec![]), instr("jp", vec![sym("Elsewhere")])];
        let e = path_costs(&items, Cpu::Z80).unwrap_err();
        assert_eq!(
            e.kind,
            BudgetFindingKind::UnboundedTransfer { mnemonic: "jp".into() }
        );
    }

    // Control running off the end of the body is the same hole as a tail transfer.
    #[test]
    fn a_fall_off_the_end_is_unbounded() {
        let items = vec![instr("nop", vec![]), instr("nop", vec![])];
        let e = path_costs(&items, Cpu::Z80).unwrap_err();
        assert_eq!(
            e.kind,
            BudgetFindingKind::UnboundedTransfer { mnemonic: "nop".into() }
        );
    }

    // An op outside the T-state table has no assignable cost.
    #[test]
    fn an_off_table_op_is_unknown() {
        let items = vec![instr("rlca", vec![]), instr("ret", vec![])];
        let e = path_costs(&items, Cpu::Z80).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::UnknownOp { mnemonic: "rlca".into() });
    }

    // There is no 68000 timing model, and the refusal says so rather than
    // producing a number from the Z80 table.
    #[test]
    fn a_68k_body_is_unmodeled() {
        let items = vec![instr("nop", vec![]), instr("rts", vec![])];
        let e = path_costs(&items, Cpu::M68000).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::UnmodeledCpu);
    }

    // A body with no instructions costs nothing on its one empty path.
    #[test]
    fn an_empty_body_costs_nothing() {
        let items = vec![label("only")];
        let c = path_costs(&items, Cpu::Z80).unwrap();
        assert_eq!((c.min, c.max), (0, 0));
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
        assert!(check_cycle_budget(&items, Cpu::Z80, Some(24), false).is_empty());
        let f = check_cycle_budget(&items, Cpu::Z80, Some(23), false);
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
        let f = check_cycle_budget(&uneven, Cpu::Z80, None, true);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, BudgetFindingKind::PathMismatch { min: 20, max: 24 });
        // Pad the short arm with one `nop` (4 T) — now both arms cost 24.
        let even = vec![
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
            check_cycle_budget(&even, Cpu::Z80, None, true)[0].kind,
            BudgetFindingKind::PathMismatch { min: 34, max: 36 }
        );
        // Spelling the short arm's join as `jr` (12 T) rather than `jp` (10 T)
        // buys the missing 2 T-states — the same substitution `pad_to_cycles`'s
        // dense mode makes, and here it is machine-checked instead of counted.
        let mut balanced = even.clone();
        balanced[2] = instr("jr", vec![sym("join")]);
        assert!(check_cycle_budget(&balanced, Cpu::Z80, None, true).is_empty());
    }

    // A proc declaring neither attribute is not walked at all, so an unbounded
    // shape stays silent until someone claims a budget for it.
    #[test]
    fn an_undeclared_proc_is_not_walked() {
        let items = vec![label("loop"), instr("jp", vec![sym("loop")])];
        assert!(check_cycle_budget(&items, Cpu::Z80, None, false).is_empty());
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
        let f = check_cycle_budget(&items, Cpu::Z80, Some(20), true);
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].kind.lint_id(), "cycles.over-budget");
        assert_eq!(f[1].kind.lint_id(), "cycles.path-mismatch");
    }

    // A bail wins over both conclusions: an unmeasurable proc reports why, and
    // never a number.
    #[test]
    fn a_bail_replaces_the_conclusions() {
        let items = vec![instr("call", vec![sym("Helper")]), instr("ret", vec![])];
        let f = check_cycle_budget(&items, Cpu::Z80, Some(1), true);
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
        let c = path_costs(&items, Cpu::Z80).unwrap();
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
        let e = path_costs(&items, Cpu::Z80).unwrap_err();
        assert_eq!(e.kind, BudgetFindingKind::UnboundedLoop);
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
        let c = path_costs(&items, Cpu::Z80).unwrap();
        assert_eq!((c.min, c.max), (28, 28));
    }
}
