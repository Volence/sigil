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
//! than an observation. Both machine-state tiers instantiate [`must_in_states`]
//! over the one shared `Cfg` — the private lattice, meet and worklist `z80_bus`
//! used to carry are gone. (Other nets in the tree — `branch_const`,
//! `type_slice`, `out_verify`, `preserves` — still carry their own worklists over
//! that same CFG with their own lattices; unifying THOSE is a separate, wider
//! job and is ledgered, not claimed here.)
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
//! path can reach the body without it. So the shape above becomes
//! [`ContextFiringKind::Escape`], an error, and there is no `Unknown` for the
//! zero-FP stance to bail on.
//!
//! **Be precise about what the region walk proves, because it is not what the
//! word "lattice" suggests.** The transfer inside a region is the IDENTITY —
//! nothing in a region changes the declared state — so the walk is REACHABILITY
//! wearing the shared lattice's plumbing, and `NotHeld`/`Unknown` are
//! unreachable there. The property checked is exactly:
//!
//! > every instruction reachable from the acquire WITHIN the region has all its
//! > out-edges landing back inside the region.
//!
//! That is a reachability property, not a dominance one, and the difference is
//! load-bearing: it is why the entry-skip and back-edge-into-the-acquire rules
//! are separate CHECKS rather than consequences. The shared factoring earns its
//! keep on the inferred side, where the transfer is real and `Unknown` carries
//! weight; here it buys uniformity, not power.
//!
//! The declared tier feeds the inferred one back: a proc declaring
//! `requires(<ctx>)` / `grants(<ctx>)` has a DEFINITE entry state (checked at
//! every call site by `[context.unsatisfied]`), which
//! [`crate::z80_bus::check_bus_state`] takes as its seed in place of `Unknown`.

use crate::flag_check::{branch_target, instr_span, Cfg, Edge};
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
fn tri_meet(a: Tri, b: Tri) -> Tri {
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
fn edges_for(cfg: &Cfg, cpu: Cpu, idx: usize) -> Vec<Edge> {
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
/// bracket plants: `enter < acquire_end < body_end < exit`. The region OWNS the
/// open index range `(enter, exit)` — its acquire, its body, and its release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    /// The context's name.
    pub ctx: String,
    /// Index of the `Enter` mark (immediately before the acquire).
    pub enter: usize,
    /// Index of the `AcquireEnd` mark (immediately after the acquire).
    pub acquire_end: usize,
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

    /// Is item index `i` inside the spliced ACQUIRE? Landing here from outside
    /// the acquire re-runs it with no matching release; the acquire's OWN
    /// internal branch (a `bne .wait_z80` spin) does not, which is why the rule
    /// is about the edge's SOURCE as well as its target. (Which half a spliced
    /// item belongs to is otherwise the item's own
    /// [`ItemAuthor::Context`](crate::value::ItemAuthor) `phase` — the range
    /// form exists for the reacquire edge rule, whose question is about an
    /// edge's TARGET index, not an item.)
    pub fn in_acquire(&self, i: usize) -> bool {
        i > self.enter && i < self.acquire_end
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
    /// One region under construction: `(ctx, enter, acquire_end, body_end, span)`.
    type Open = (String, usize, Option<usize>, Option<usize>, Span);
    let mut open: Vec<Open> = Vec::new();
    let mut regions = Vec::new();
    let mut firings = Vec::new();
    for (idx, it) in items.iter().enumerate() {
        let CodeItem::ContextMark { ctx, kind, span } = it else { continue };
        match kind {
            ContextMarkKind::Enter => {
                if open.iter().any(|(c, ..)| c == ctx) {
                    firings.push(ContextFiring {
                        proc: proc.to_string(),
                        ctx: ctx.clone(),
                        kind: ContextFiringKind::Reacquire,
                        span: *span,
                    });
                }
                open.push((ctx.clone(), idx, None, None, *span));
            }
            ContextMarkKind::AcquireEnd => {
                if let Some(slot) = open.iter_mut().rev().find(|(c, ..)| c == ctx) {
                    slot.2 = Some(idx);
                }
            }
            ContextMarkKind::BodyEnd => {
                if let Some(slot) = open.iter_mut().rev().find(|(c, ..)| c == ctx) {
                    slot.3 = Some(idx);
                }
            }
            ContextMarkKind::Exit => {
                let Some(pos) = open.iter().rposition(|(c, ..)| c == ctx) else { continue };
                let (ctx, enter, acquire_end, body_end, span) = open.remove(pos);
                if let (Some(acquire_end), Some(body_end)) = (acquire_end, body_end) {
                    regions.push(Region { ctx, enter, acquire_end, body_end, exit: idx, span });
                }
            }
        }
    }
    (regions, firings)
}

/// Run the `with`-bracket proofs over one proc's evaluated CodeBuf (§3.2).
///
/// Per region, over the instructions reachable from the acquire WITHIN the
/// region (the [`must_in_states`] walk seeded [`Tri::Held`] and gated by
/// [`Region::contains`]):
///
/// - **escape** — an edge out of the acquire+body range that does not land back
///   inside the region is a path that skips the release. `Return`, `FallOff` and
///   a `Defer` (a tail transfer, e.g. an external `jbra`) are escapes for the
///   same reason. A call is not among them: a call comes back, and the edge
///   builders say so — every call mnemonic gets its fall-through and nothing
///   else.
/// - **reacquire (by branch)** — an edge from the body or the release back INTO
///   the acquire re-runs it with no matching release. That is the unbounded
///   `move.w sr,-(sp)` leak in bracket form, and it is why the acquire needs its
///   own fence: the acquire's own internal spin (`bne .wait_z80`) branches into
///   the same range and is correct.
/// - **entry-skip** — a transfer from OUTSIDE the region whose local-label target
///   lands inside it, or an `export` label inside it (reachable from any other
///   proc, which this proc's item list cannot see). Read off the target symbol
///   rather than the CFG edges, so a `jbsr .label` into the region is caught as
///   well as a branch.
///
/// `[context.reacquire]` for a NESTED bracket comes from [`regions_of`] (mark
/// nesting).
pub fn check_contexts(proc: &str, items: &[CodeItem], cpu: Cpu) -> Vec<ContextFiring> {
    let (regions, firings) = regions_of(proc, items);
    check_regions(proc, items, cpu, &regions, firings)
}

/// The bracket proofs against ALREADY-RECOVERED regions — the entry the corpus
/// walk uses, so one `regions_of` scan feeds both the census and the firings.
pub fn check_regions(
    proc: &str,
    items: &[CodeItem],
    cpu: Cpu,
    regions: &[Region],
    mut firings: Vec<ContextFiring>,
) -> Vec<ContextFiring> {
    if regions.is_empty() {
        firings.sort_by(|a, b| (&a.proc, &a.ctx, a.span.start).cmp(&(&b.proc, &b.ctx, b.span.start)));
        return firings;
    }
    let cfg = Cfg::build(items);
    for region in regions {
        // The region's declared entry point: the first INSTRUCTION after the
        // `Enter` mark (the acquire's own first instruction).
        let Some(entry) = (region.enter + 1..region.exit)
            .find(|&i| matches!(items.get(i), Some(CodeItem::Instr { .. })))
        else {
            continue; // an empty region (acquire, body and release all inert)
        };
        let mut fire = |kind: ContextFiringKind, span: Span| {
            firings.push(ContextFiring {
                proc: proc.to_string(),
                ctx: region.ctx.clone(),
                kind,
                span,
            });
        };
        let state = must_in_states(
            &cfg,
            cpu,
            &[(entry, Tri::Held)],
            // Nothing inside a region changes the DECLARED state, so this walk is
            // reachability with the shared lattice's plumbing — the `Held` seed is
            // a declaration and no transfer can move it. What is actually proven
            // is stated in the module header, and it is a REACHABILITY property,
            // not a dominance one.
            &|st, _, _| st,
            &|i| region.contains(i),
        );
        for &idx in state.keys() {
            if !region.in_body(idx) {
                continue;
            }
            for edge in edges_for(&cfg, cpu, idx) {
                match edge {
                    // A branch BACK into the acquire re-runs it. Excluded when
                    // the source is itself inside the acquire — that is the
                    // request-and-spin loop the acquire is made of.
                    Edge::Follow(succ)
                        if region.in_acquire(succ) && !region.in_acquire(idx) =>
                    {
                        fire(ContextFiringKind::Reacquire, span_at(items, idx, region));
                    }
                    Edge::Follow(succ) if !region.contains(succ) => {
                        fire(ContextFiringKind::Escape, span_at(items, idx, region));
                    }
                    // A `Defer` here is a genuine transfer out. No call reaches
                    // it: both builders give every call mnemonic its fall-through
                    // and nothing else — or, at the end of a body, only the
                    // fall-off, which IS an escape and fires as one.
                    Edge::Return | Edge::FallOff | Edge::Defer => {
                        fire(ContextFiringKind::Escape, span_at(items, idx, region));
                    }
                    Edge::Follow(_) => {}
                }
            }
        }
        for (idx, it) in items.iter().enumerate() {
            match it {
                // A transfer from OUTSIDE the region into it. Landing on the
                // region's FIRST instruction is not a skip — it takes the whole
                // acquire, which is the corpus's spin-probe idiom (`.await_slot:`
                // written immediately before the bracket, branched back to from
                // below; a label targets the first instruction at or after it).
                CodeItem::Instr { mnemonic, ops, span, .. } if !region.contains(idx) => {
                    if !is_transfer_mnemonic(mnemonic.as_str(), cpu) {
                        continue;
                    }
                    let target = branch_target(ops).and_then(|t| cfg.label_index(t));
                    if target.is_some_and(|t| region.contains(t) && t != entry) {
                        fire(ContextFiringKind::EntrySkip, *span);
                    }
                }
                // An EXPORTED label inside the region is an entry point this
                // proc's item list cannot see: it takes the stable `Owner.name`
                // symbol, so any other proc — or a residual `.asm` — can branch
                // straight past the acquire.
                CodeItem::Label { export: true, name, span }
                    if region.contains(idx) && cfg.label_index(name) != Some(entry) =>
                {
                    fire(ContextFiringKind::EntrySkip, *span);
                }
                _ => {}
            }
        }
    }
    firings.sort_by(|a, b| (&a.proc, &a.ctx, a.span.start).cmp(&(&b.proc, &b.ctx, b.span.start)));
    firings.dedup();
    firings
}

/// The span to anchor a region firing at: the offending instruction's, falling
/// back to the `with` header when the item carries none.
fn span_at(items: &[CodeItem], idx: usize, region: &Region) -> Span {
    instr_span(items, idx).unwrap_or(region.span)
}

/// Does `mnem` name a subroutine CALL — a transfer whose control returns?
pub(crate) fn is_call_mnemonic(mnem: &str, cpu: Cpu) -> bool {
    match cpu {
        Cpu::Z80 => mnem == "call" || mnem == "rst",
        _ => matches!(mnem, "jsr" | "bsr" | "jbsr"),
    }
}

/// Does `mnem` TRANSFER control to its symbolic operand — a branch, a tail
/// transfer, or a call? The entry-skip scan gates on this so an instruction that
/// merely NAMES a local label in a data position (`lea .table(pc), a0`) is not
/// read as a branch into the region.
fn is_transfer_mnemonic(mnem: &str, cpu: Cpu) -> bool {
    if is_call_mnemonic(mnem, cpu) {
        return true;
    }
    match cpu {
        Cpu::Z80 => matches!(mnem, "jp" | "jr" | "djnz" | "ret" | "reti" | "retn"),
        _ => {
            matches!(mnem, "jmp" | "jra" | "jbra" | "bra")
                || (mnem.starts_with('b') && mnem.len() == 3)
                || mnem.starts_with("db")
        }
    }
}

/// The contexts a bracket makes active at item index `idx` (§3.3) — the
/// region-derived half of what discharges a call site's `requires`. The other
/// half is the caller's own `requires` ∪ `grants`, which hold over its whole
/// body; the caller unions the two.
pub fn bracketed_at(regions: &[Region], idx: usize) -> std::collections::BTreeSet<String> {
    regions.iter().filter(|r| r.contains(idx)).map(|r| r.ctx.clone()).collect()
}
