//! `[context.*]` — the DECLARED machine-state tier (contract unification §3).
//!
//! A `with <ctx> { }` bracket splices the context's `acquire` before its body and
//! `release` after it, and PROVES the pairing. This module owns those proofs and
//! the three-point MUST lattice they share with the INFERRED tier
//! ([`crate::z80_bus`]).
//!
//! **Two tiers, one CFG, one lattice.** `z80_bus` infers bus ownership from the
//! instruction stream over [`crate::flag_check::Cfg`]; a declared context
//! generalizes that to a per-context [`Tri`] whose seed is a DECLARATION rather
//! than an observation. Both instantiate [`must_in_states`] — there is exactly
//! one worklist, one meet, and one CFG in the tree.
//!
//! **Why the declared tier is total where the inferred tier is not.** `z80_bus`
//! seeds proc entry `Unknown`: a caller may already hold the bus and that is not
//! locally provable, so it fires only where the code itself made the state
//! definite (its zero-false-positive stance). The consequence is a real blind
//! spot — at a JOIN of a held path and a released path the lattice falls to
//! `Unknown` and nothing fires, which is exactly the shape of a branch that
//! jumps out of a hand-written stop/start pair:
//!
//! ```text
//!     stop_z80                    ; Stopped
//!     bne     .skip               ; --> leaves the pair with the bus HELD
//!     …
//!     start_z80                   ; Running
//! .skip:                          ; meet(Stopped, Running) = Unknown
//!     rts                         ; Unknown -> [bus.stopped-at-return] does NOT fire
//! ```
//!
//! Inside a bracket the state is DECLARED, not inferred: the region's body is
//! seeded [`Tri::Held`] and, because the acquire is compiler-generated and no
//! branch may enter the region mid-way ([`ContextFiringKind::EntrySkip`]), no
//! path can reach the body without it. The lattice therefore never leaves the
//! `Held` point inside the region — there is no `Unknown` to bail on — so
//! "every path reaches the release" is checked on EVERY path. The branch above
//! becomes [`ContextFiringKind::Escape`], an error.
//!
//! The declared tier feeds the inferred one back: a proc declaring
//! `requires(<ctx>)` / `grants(<ctx>)` has a DEFINITE entry state (checked at
//! every call site by `[context.unsatisfied]`), which
//! [`crate::z80_bus::check_bus_state`] takes as its seed in place of `Unknown`.

use crate::flag_check::{branch_target, Cfg, Edge};
use crate::value::{CodeItem, CodeOperand, ContextMarkKind};
use sigil_ir::backend::Cpu;
use sigil_span::Span;
use std::collections::{BTreeMap, VecDeque};

/// The three-point MUST lattice every machine-state net in the tree shares:
/// a definite `Held`, a definite `NotHeld`, and `Unknown` for "caller-dependent,
/// or a join of disagreeing states". `Unknown` is absorbing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Tri {
    /// The state provably HOLDS at this point.
    Held,
    /// The state provably does NOT hold at this point.
    NotHeld,
    /// No definite fact — nothing that needs a definite fact may fire here.
    Unknown,
}

/// The meet (the join for a MUST analysis): agreeing states survive, any
/// disagreement falls to [`Tri::Unknown`].
pub(crate) fn tri_meet(a: Tri, b: Tri) -> Tri {
    if a == b {
        a
    } else {
        Tri::Unknown
    }
}

/// The per-instruction IN-state fixpoint: a forward MUST dataflow over the
/// shared CFG from `seeds`, transfer `step`, join = [`tri_meet`], worklist to a
/// fixpoint. Monotone on a three-point lattice, so it terminates.
///
/// `follow` GATES edge propagation. The inferred tier walks the whole proc
/// (`|_| true`); a region-scoped instantiation stops at its own boundary, which
/// is what makes an edge leaving the region an EVENT the caller reports rather
/// than a state it propagates.
///
/// `cpu` selects the CFG's successor model ([`Cfg::edges`] vs
/// [`Cfg::z80_edges`]) — the same choice every other consumer of this CFG makes.
pub(crate) fn must_in_states(
    cfg: &Cfg,
    cpu: Cpu,
    seeds: &[(usize, Tri)],
    step: &dyn Fn(Tri, &str, &[CodeOperand]) -> Tri,
    follow: &dyn Fn(usize) -> bool,
) -> BTreeMap<usize, Tri> {
    let mut in_state: BTreeMap<usize, Tri> = BTreeMap::new();
    let mut work: VecDeque<usize> = VecDeque::new();
    for (idx, st) in seeds {
        in_state.insert(*idx, *st);
        work.push_back(*idx);
    }
    while let Some(idx) = work.pop_front() {
        let st_in = in_state[&idx];
        let Some((mnem, ops)) = cfg.instr(idx) else { continue };
        let st_out = step(st_in, mnem, ops);
        for edge in edges_for(cfg, cpu, idx) {
            let Edge::Follow(succ) = edge else { continue };
            if !follow(succ) {
                continue;
            }
            let merged = match in_state.get(&succ) {
                None => st_out,
                Some(existing) => tri_meet(*existing, st_out),
            };
            if in_state.get(&succ) != Some(&merged) {
                in_state.insert(succ, merged);
                work.push_back(succ);
            }
        }
    }
    in_state
}

/// The CFG's successor edges for `idx` under `cpu` — the one place the
/// 68k/Z80 successor models are selected, so no consumer has to remember.
pub(crate) fn edges_for(cfg: &Cfg, cpu: Cpu, idx: usize) -> Vec<Edge> {
    match cpu {
        Cpu::Z80 => cfg.z80_edges(idx),
        _ => cfg.edges(idx),
    }
}

/// Which `with`-bracket contract a firing violated (§3.2 / §6 tier map — all
/// error-tier, none `@as_compat`-silenced: a context is always declared surface).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextFiringKind {
    /// `[context.escape]` — a path through the bracketed body leaves the region
    /// without reaching the release: a branch to a label outside it, a return, a
    /// tail transfer, or a fall-off-the-end.
    Escape,
    /// `[context.entry-skip]` — a branch/call from OUTSIDE the region targets a
    /// label inside it, so control can reach the body without the acquire.
    EntrySkip,
    /// `[context.reacquire]` — a bracket for a context that is already active.
    /// Acquired contexts are not reentrant by default (the Z80 bus request is
    /// not a counting lock: the inner release frees the outer hold).
    Reacquire,
}

/// One `with`-bracket contract firing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextFiring {
    /// The proc whose body carries the region.
    pub proc: String,
    /// The context named by the bracket.
    pub ctx: String,
    /// Which proof failed.
    pub kind: ContextFiringKind,
    /// The offending instruction (escape / entry-skip) or the inner `with`
    /// header (reacquire).
    pub span: Span,
}

/// One `with` region's item-index boundaries, recovered from the marks a
/// bracket plants. `enter < body_end < exit`, and the region OWNS the half-open
/// index range `(enter, exit)` — its acquire, its body, and its release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    /// The context's name.
    pub ctx: String,
    /// Index of the `Enter` mark (immediately before the acquire).
    pub enter: usize,
    /// Index of the `BodyEnd` mark (immediately before the release).
    pub body_end: usize,
    /// Index of the `Exit` mark (immediately after the release).
    pub exit: usize,
    /// The `with` header's span.
    pub span: Span,
}

impl Region {
    /// Is item index `i` INSIDE this region (acquire, body, or release)?
    pub fn contains(&self, i: usize) -> bool {
        i > self.enter && i < self.exit
    }

    /// Is item index `i` inside the acquire+BODY half — the range the escape
    /// proof ranges over? The release is excluded: its own fall-through out of
    /// the region is the legitimate exit, and inside the checked range it would
    /// be indistinguishable from an escape.
    fn in_body(&self, i: usize) -> bool {
        i > self.enter && i < self.body_end
    }
}

/// Recover every complete `with` region from a CodeBuf's marks, plus the
/// `[context.reacquire]` firings the mark nesting itself proves.
///
/// Regions are recovered from the ITEM STREAM rather than the AST, so a bracket
/// that arrived through a comptime splice is seen exactly like a lexical one —
/// and so is a bracket nested inside another module's template, which is the
/// only way a same-context reacquire can happen without being visible in one
/// file. An unmatched mark (only reachable from a lowering that already
/// errored) contributes no region.
pub fn regions_of(proc: &str, items: &[CodeItem]) -> (Vec<Region>, Vec<ContextFiring>) {
    let mut open: Vec<(String, usize, Option<usize>, Span)> = Vec::new();
    let mut regions = Vec::new();
    let mut firings = Vec::new();
    for (idx, it) in items.iter().enumerate() {
        let CodeItem::ContextMark { ctx, kind, span } = it else { continue };
        match kind {
            ContextMarkKind::Enter => {
                if open.iter().any(|(c, _, _, _)| c == ctx) {
                    firings.push(ContextFiring {
                        proc: proc.to_string(),
                        ctx: ctx.clone(),
                        kind: ContextFiringKind::Reacquire,
                        span: *span,
                    });
                }
                open.push((ctx.clone(), idx, None, *span));
            }
            ContextMarkKind::BodyEnd => {
                if let Some(slot) = open.iter_mut().rev().find(|(c, _, _, _)| c == ctx) {
                    slot.2 = Some(idx);
                }
            }
            ContextMarkKind::Exit => {
                let Some(pos) = open.iter().rposition(|(c, _, _, _)| c == ctx) else { continue };
                let (ctx, enter, body_end, span) = open.remove(pos);
                if let Some(body_end) = body_end {
                    regions.push(Region { ctx, enter, body_end, exit: idx, span });
                }
            }
        }
    }
    (regions, firings)
}

/// Run the `with`-bracket proofs over one proc's evaluated CodeBuf (§3.2).
///
/// Per region, seeded at the first instruction after the `Enter` mark with a
/// DECLARED [`Tri::Held`] and propagating only INSIDE the region:
///
/// - **escape** — for every reached instruction in the acquire+body range, an
///   edge that does not land inside the region is a path that skips the
///   release. `Abandon` (a return, or falling off the proc's end) and `Defer`
///   (a tail transfer to an external symbol) are escapes for the same reason.
/// - **entry-skip** — for every instruction OUTSIDE the region, a branch/call
///   whose local-label target lands inside the region enters it past the
///   acquire. Read off the target symbol directly rather than the CFG edges, so
///   a `jbsr .label` into the region is caught as well as a branch.
///
/// `[context.reacquire]` comes from [`regions_of`] (mark nesting).
pub fn check_contexts(proc: &str, items: &[CodeItem], cpu: Cpu) -> Vec<ContextFiring> {
    let (regions, mut firings) = regions_of(proc, items);
    if regions.is_empty() {
        return firings;
    }
    let cfg = Cfg::build(items);
    for region in &regions {
        // The region's declared entry point: the first INSTRUCTION after the
        // `Enter` mark (the acquire's own first instruction).
        let Some(entry) = (region.enter + 1..region.exit)
            .find(|&i| matches!(items.get(i), Some(CodeItem::Instr { .. })))
        else {
            continue; // an empty region (acquire, body and release all inert)
        };
        let state = must_in_states(
            &cfg,
            cpu,
            &[(entry, Tri::Held)],
            // Nothing inside a region changes the DECLARED state: the acquire
            // established it and the release is past the checked range. That the
            // lattice cannot leave `Held` here is the point, not an omission —
            // see the module header.
            &|st, _, _| st,
            &|i| region.contains(i),
        );
        for (&idx, &st) in &state {
            if st != Tri::Held || !region.in_body(idx) {
                continue;
            }
            let escapes = edges_for(&cfg, cpu, idx).into_iter().any(|e| match e {
                Edge::Follow(succ) => !region.contains(succ),
                Edge::Abandon | Edge::Defer => true,
            });
            if escapes {
                firings.push(ContextFiring {
                    proc: proc.to_string(),
                    ctx: region.ctx.clone(),
                    kind: ContextFiringKind::Escape,
                    span: instr_span(items, idx).unwrap_or(region.span),
                });
            }
        }
        for (idx, it) in items.iter().enumerate() {
            if region.contains(idx) {
                continue;
            }
            let CodeItem::Instr { ops, span, .. } = it else { continue };
            let target = branch_target(ops).and_then(|t| cfg.label_index(t));
            if target.is_some_and(|t| region.contains(t)) {
                firings.push(ContextFiring {
                    proc: proc.to_string(),
                    ctx: region.ctx.clone(),
                    kind: ContextFiringKind::EntrySkip,
                    span: *span,
                });
            }
        }
    }
    firings.sort_by(|a, b| (&a.proc, &a.ctx, a.span.start).cmp(&(&b.proc, &b.ctx, b.span.start)));
    firings.dedup();
    firings
}

/// The span of the instruction at item index `idx`.
fn instr_span(items: &[CodeItem], idx: usize) -> Option<Span> {
    match items.get(idx) {
        Some(CodeItem::Instr { span, .. }) => Some(*span),
        _ => None,
    }
}

/// The contexts a bracket makes active at item index `idx` (§3.3) — the
/// region-derived half of what discharges a call site's `requires`. The other
/// half is the caller's own `requires` ∪ `grants`, which hold over its whole
/// body; the caller unions the two.
pub fn bracketed_at(regions: &[Region], idx: usize) -> std::collections::BTreeSet<String> {
    regions.iter().filter(|r| r.contains(idx)).map(|r| r.ctx.clone()).collect()
}
