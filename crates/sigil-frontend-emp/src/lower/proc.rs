//! Lower an [`Item::Proc`](crate::ast::Item::Proc) to Core IR (Spec 2, Plan 4 —
//! T4, §5.1). A proc becomes: a label named after the proc, then its body
//! lowered through the SAME machinery `asm { }` uses — the body is evaluated to
//! a resolved [`CodeBuf`](crate::value::CodeBuf) by
//! [`eval_proc_body`](crate::eval::eval_proc_body) (reusing `eval_asm`) and
//! streamed by [`lower_code_buf`](super::lower_code_buf) (reusing T3's backend
//! dispatch). No instruction lowering is re-implemented here (D-P4.1).
//!
//! T4 also runs the §5.1 proc-contract checks over the resolved body — the three
//! below plus the contract-grammar v2 additions ([`check_preserves`],
//! [`check_out`], the context brackets, the survives claims, and stack balance),
//! numbered in [`lower_proc`] in the order they run:
//!
//! - **Declared fallthrough** (`falls_into next`): `next` must be the item
//!   IMMEDIATELY following this proc in the section (declaration order) — any
//!   item between them, an out-of-order target, or a non-proc target breaks the
//!   physical fallthrough and is the `[proc.fallthrough-separated]` error.
//! - **Undeclared fallthrough** (default-on warning): a proc with no
//!   `falls_into` whose body can reach its closing `}` without an unconditional
//!   terminator (`rts`/`rte`/`bra`/`jmp`/`jbra` on 68k; `ret`/`jp`/`jr` on Z80) warns
//!   `[proc.undeclared-fallthrough]`. T4's analysis is deliberately minimal — it
//!   inspects only the LAST instruction's mnemonic; the full control-flow
//!   reachability version is deferred (S2-D6/D7).
//! - **`clobbers` lint** (default-on, D-P4.9): a write to a register outside the
//!   declared `clobbers(...)` set ∪ params is `[proc.clobber-undeclared]`. This
//!   is NECESSARILY a heuristic (it is assembly): T4 flags the destination
//!   register operand of the standard write-form mnemonics (`move`, `add`,
//!   `moveq`, `clr`, …). Read-only / control forms (`cmp`, `tst`, `bra`, `jmp`)
//!   and memory-destination writes do not trigger it. The full register-dataflow
//!   contract is the deferred S2-D6 sub-milestone.

use crate::ast;
use crate::eval::eval_proc_body;
use crate::value::{CodeItem, CodeOperand, Reg};
use sigil_ir::backend::{Cpu, IrStreamer};
use sigil_ir::IrBuilder;
use sigil_span::{Diagnostic, Level, Span};
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// This proc's position among its declaration-order siblings — the context a
/// declared `falls_into` needs to check physical adjacency (§5.1). Bundling the
/// `(index, items)` pair keeps [`lower_proc`]'s signature within the arg budget
/// and reads as one concept ("where this proc sits").
pub(super) struct Siblings<'a> {
    /// This proc's index within `items`.
    pub index: usize,
    /// The declaration-order item list this proc belongs to (the module's items,
    /// or a `section {}` block's items).
    pub items: &'a [ast::Item],
}

/// How a proc lowers: its CPU (drives code encoding + the terminator table) and
/// whether the enclosing module is `@as_compat` (which silences the heuristic
/// modernization WARNINGs). Bundled so `lower_proc` stays under clippy's
/// argument-count lint (mirroring how [`Siblings`] bundles position).
pub(super) struct ProcCtx<'a> {
    /// The CPU this proc's body encodes for.
    pub cpu: Cpu,
    /// Module-level `@as_compat` — silence the faithful-port lints (D-P6.3).
    pub as_compat: bool,
    /// Comptime `-D NAME=INT` defines (sound-migration T2 Task 1, R1), seeded
    /// into this proc's evaluator so its body can reference one like any
    /// other name.
    pub defines: &'a [(String, i128)],
    /// The module-scope `invariant: preserves(...)` unit set every proc in a
    /// `(cpu: z80)` module INHERITS (rung-2 §3.2). Empty when the module carries
    /// no invariant (every 68k module, and a Z80 module without the attribute).
    pub invariant_regs: &'a [String],
    /// The callee-declared-preserves oracle (gap 2): each visible proc / `extern
    /// proc` name → the register UNITS its contract preserves, so the Z80
    /// `preserves` proof credits a `call`/tail-transfer to a preserving callee.
    /// Empty for every 68k module (the map is only consulted by the Z80 proof).
    pub callee_preserves: &'a crate::z80_preserves::CalleePreserves,
    /// The resolved game-contract interfaces (L1), seeded into this proc's
    /// evaluator so its body resolves `Iface.MEMBER` and lowers
    /// `invoke Iface.hook`. Empty for a contract-free build.
    pub contracts: &'a crate::contract::InterfaceEnv,
    /// The module's `@noreturn` symbol set (noreturn-tail model): every visible
    /// `proc` / `extern proc` declaring `@noreturn`. A divergent tail transfer to
    /// a name in this set is a TERMINAL edge, not an unbounded transfer — read by
    /// the cycle-budget and CCR-bracket walks. Empty when no decl carries it.
    pub noreturn: &'a std::collections::BTreeSet<String>,
    /// The interrupt-mask-preservers oracle (68k preserves-through-tail credit):
    /// every visible proc / `extern proc` declaring `preserves(sr)` or
    /// `preserves(sr.mask)`, mapped to its save-first-bracket export-label entries
    /// (see [`crate::lower::collect_sr_mask_preservers`]). The mask-claim tail
    /// credit consults it so an unconditional external tail to a preserving sibling
    /// round-trips the mask — a plain-name tail by key presence, an `Owner.label`
    /// tail only when the label is a save-first entry. Empty when no visible decl
    /// preserves the mask.
    pub sr_mask_preservers:
        &'a std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
}

/// Lower one proc: define its label, evaluate + lower its body, then run the
/// §5.1 fallthrough / clobber contract checks. `siblings` locates this proc in
/// declaration order so declared fallthrough can check adjacency. `asm_counter`
/// is the module-wide instantiation counter (D-P4.6): it seeds this proc's
/// evaluator and is advanced by however many `asm { }` bodies it instantiates, so
/// `k` stays globally monotonic across procs (a fresh evaluator per proc would
/// otherwise reset it and collide labels). `as_compat` (module `@as_compat`,
/// Plan 6 D-P6.3) silences the heuristic modernization WARNINGs — undeclared
/// fallthrough and the clobber lint — while leaving the hard `falls_into`
/// adjacency ERROR untouched.
pub(super) fn lower_proc(
    file: &ast::File,
    proc: &ast::ProcDecl,
    siblings: Siblings<'_>,
    ctx: ProcCtx,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
    asm_counter: &mut u32,
) {
    // 1. Label + body → IR. Params emit no code (declarative register bindings).
    builder.define_label(&proc.name);
    // D2.29 amendment: a 68k proc at an odd final address is an address-error
    // crash — error-tier [layout.odd-item] parity check on the proc's label.
    super::record_odd_item_assert(
        file,
        builder,
        ctx.cpu,
        ctx.as_compat,
        super::OddItemKind::Code,
        &proc.name,
        proc.span,
    );
    let (buf, mut ds, next_counter) = eval_proc_body(
        file,
        &proc.name,
        &proc.params,
        &proc.body,
        proc.span,
        *asm_counter,
        ctx.cpu,
        ctx.defines,
        ctx.contracts,
    );
    *asm_counter = next_counter;
    diags.append(&mut ds);
    let Some(buf) = buf else { return };
    super::lower_code_buf(&buf, ctx.cpu, ctx.as_compat, builder, diags);

    // Exported EXTENT symbol (bookmark ask 2): a `@resumable` proc gets a linkable
    // `Proc.__end` label at the byte immediately past its body, so a consumer
    // compiles a `[Proc, Proc.__end)` PC range check from toolchain symbols
    // instead of a hand-maintained sentinel. The builder cursor sits at the body
    // end right after `lower_code_buf`, so this is that position. It rides the
    // exported-label naming path (`Owner.local`, like `export .name:` → `foo.name`)
    // so `canonicalize_name`'s dotted-owner rule module-qualifies it exactly as the
    // proc's own symbol — no rename-table change, same cross-module visibility. A
    // label emits no bytes, and no existing corpus proc is `@resumable`, so this is
    // byte- and symbol-neutral everywhere but a resumable proc's own module.
    if proc.is_resumable() {
        let end_sym = format!("{}.__end", proc.name);
        // `.__end` is RESERVED for the generated extent label. A body-defined
        // `export .__end:` hygiene-resolves to the SAME `Proc.__end` symbol
        // (`Owner.name`) and would silently collide with a second definition at a
        // different offset. (A non-export `.__end:` mangles to `$mod$Proc$__end`
        // and does not collide, so the name compare below is exactly the collision
        // set.) Reject the source label rather than mint a duplicate symbol.
        let collides = buf
            .items
            .iter()
            .any(|it| matches!(it, CodeItem::Label { name, .. } if *name == end_sym));
        if collides {
            push(
                diags,
                Level::Error,
                proc.span,
                format!(
                    "[resumable.extent-reserved] `@resumable` proc `{}` defines an exported \
                     `.__end` label, but that name is reserved for the generated extent symbol \
                     `{end_sym}` (the `[Proc, Proc.__end)` range bound) — rename the label",
                    proc.name
                ),
            );
        } else {
            builder.define_label(&end_sym);
        }
    }

    // 2/3. Fallthrough contract. A declared `falls_into` demands adjacency (a
    // hard ERROR when broken — never silenced); an undeclared but reachable
    // fall-off the end is a modernization WARNING that `@as_compat` silences
    // (Plan 6, D-P6.3: a faithful port opts out of the faithful-port lints).
    match &proc.falls_into {
        Some(next) => {
            check_fallthrough_adjacent(proc, next, siblings.index, siblings.items, diags)
        }
        None if !ctx.as_compat => check_undeclared_fallthrough(proc, &buf, ctx.cpu, diags),
        None => {}
    }

    // The `with <ctx> { }` regions this body carries, recovered ONCE for the
    // bracket proofs (step 9); `check_regions` is `pub` precisely so
    // `corpus_contracts.rs` — the other consumer of the same seam — does not
    // re-scan the mark stream. A no-bracket body yields an empty vector from
    // one linear pass. (The clobbers lint does not read regions; its context
    // exemption is the typed `ItemAuthor::Context` check.)
    let (regions, mark_firings) = crate::context::regions_of(&proc.name, &buf.items);

    // 4. Clobbers lint (only when the proc declares a clobber set — the
    // explicit empty `clobbers()` counts: it declares "touches nothing", so
    // every register write is undeclared) — likewise a modernization warning
    // silenced under `@as_compat`.
    // A `@resumable` proc's register-state set IS this contract (params + clobbers
    // + out); "anything live outside the declared set is an error" (bookmark ask 1)
    // is exactly `[proc.clobber-undeclared]`, so the check is MANDATORY there —
    // it runs even under `@as_compat` (a resumable proc is a strict new contract,
    // not a faithful port). `check_resumable` separately requires the `clobbers`
    // set to be declared, so this gate's `is_some()` is met whenever it matters.
    if proc.clobbers.is_some() && (!ctx.as_compat || proc.is_resumable()) {
        check_clobbers(
            proc,
            &buf,
            ctx.cpu,
            ctx.noreturn,
            ctx.sr_mask_preservers,
            ctx.invariant_regs,
            ctx.callee_preserves,
            diags,
        );
    }

    // 5. Preserves contract. On Z80 (rung-2 §4.2) the push/pop `preserves` proof
    // (the `z80_preserves` sibling) replaces the 68k movem-pair slice, and the
    // module-scope `invariant` is UNIONED onto every proc's declared preserves
    // (§3.2 inheritance) — so a proc that breaks the invariant fires even with no
    // explicit `preserves`. On 68k, the S2-D6b syntactic slice stands unchanged
    // (byte-frozen — `preserves.rs` untouched). Both are opt-in declared
    // CONTRACTs, error-tier, NOT silenced by `@as_compat`.
    if ctx.cpu == Cpu::Z80 {
        check_z80_preserves(
            proc, &buf, ctx.invariant_regs, ctx.callee_preserves, ctx.noreturn, diags,
        );
    } else if !proc.preserves.is_empty() {
        check_preserves(proc, &buf, ctx.noreturn, ctx.sr_mask_preservers, diags);
    }

    // 5b. `@noreturn` contract (noreturn-tail model): a proc claiming it never
    // returns must contain NO return edge — every path leaves by transfer or
    // loop. A CHECKED declared claim, error-tier, never `@as_compat`-silenced.
    if proc.is_noreturn() {
        check_noreturn(proc, &buf, ctx.cpu, ctx.noreturn, diags);
    }

    // 6. Output contract (S2-D6e): a declared `out(...)` set. Like `preserves`,
    // an opt-in declared CONTRACT — error/warning tier, NOT silenced by
    // `@as_compat` (only the heuristic modernization lints are). Runs only when
    // a contract is declared (`Some(_)`; the explicit empty `out()` counts —
    // it declares "returns nothing", so any listed register would be moot but
    // the overlap/unwritten checks still apply to whatever IS listed).
    if proc.out.is_some() || proc.inout.is_some() {
        check_out(proc, &buf, ctx.cpu, diags);
    }

    // 7. Flag results (`out(carry: name)`) + conditional register results
    // (`out(rN if cc)`) — contract-grammar v2 §6. Validity only (the caller-side
    // must-use check `[call.flag-result-unused]` lives in the whole-corpus walk,
    // since it needs cross-module contract knowledge). Runs whenever a flag /
    // conditional result is declared. Not silenced by `@as_compat` — an opt-in
    // declared contract, like `preserves`/`out`.
    if !proc.out_flags.is_empty() || !proc.out_cond.is_empty() {
        check_out_flags_cond(&proc.name, &proc.out_flags, &proc.out_cond, diags);
    }

    // 8. The SURVIVES half of a conditional result (delta spec §7.1): a cond-out
    // register ABSENT from `clobbers` claims it still holds its entry value on
    // every `!cc` return path. Error-tier and not `@as_compat`-silenced, like
    // every declared contract. 68k only — `VALID_CCS` is the 68k set and is
    // applied to both CPUs, so a Z80 `out(a if z)` is rejected before this point.
    if ctx.cpu != Cpu::Z80 && !proc.out_cond.is_empty() && proc.clobbers.is_some() {
        check_survives_claims(proc, &buf, diags);
    }

    // 9. The `with <ctx> { }` bracket proofs (contract unification §3.2): every
    // path through a bracketed body reaches the release, no branch enters the
    // region past the acquire, and no acquired context is entered twice. These
    // need only ONE body, so they belong here rather than in the corpus walk (the
    // cross-proc half — `[context.unsatisfied]` — lives there). Error-tier and
    // NEVER `@as_compat`-silenced: a context is always declared surface (§6).
    check_context_brackets(&proc.name, &buf, &regions, mark_firings, ctx.cpu, diags);

    // 10. The `requires(...)` / `grants(...)` clauses' own validity. The
    // per-call-site satisfaction check is inherently cross-proc and lives in the
    // corpus walk; these two are decidable HERE, from the file's item list (which
    // carries every imported context, injected by the resolver), and they must
    // fail the build for the same reason `with` on an unknown context does — a
    // silently-ignored context clause reads as a checked claim and is not one.
    check_context_clauses(file, proc, diags);

    // 11. Stack discipline (delta spec §3 / U-spec §4-stack): sp is back at its
    // entry value on every path to a return, and paths that merge agree on where
    // sp is. This needs NO declaration — an imbalanced `rts` returns to whatever
    // word is on top of the stack, which is a defect in any proc — so it runs on
    // every 68k body. 68k only: `preserves.rs` and the `Cfg` edge model it walks
    // are the 68k pair (`z80_preserves` is the Z80 sibling and has no stack-delta
    // arm yet).
    // A `@continuation` proc is exempt (bookmark ask 4): it is entered by a
    // manufactured/hardware transfer and its manufactured `rte`/`rts` consume a
    // frame the model never saw pushed (the hijacked exception frame, or a
    // push-SR/push-PC pair the `rte` pops). Its frame discipline is the author's
    // responsibility — a trust root, like `grants(...)`. Gated on
    // `is_continuation()`, which no existing corpus proc carries, so this is
    // check-neutral everywhere else.
    if ctx.cpu != Cpu::Z80 && !proc.is_continuation() {
        check_stack_balance(file, proc, &buf, ctx.as_compat, diags);
    }

    // 11a. `@continuation` (bookmark ask 4): the manufactured-frame license's own
    // validity — a declared register-state set and the 68k scope guard. The
    // stack-balance exemption above is the license itself; this is its two guards.
    if proc.is_continuation() {
        check_continuation(proc, ctx.cpu, diags);
    }

    // 11b. `@resumable` (bookmark ask 1): the STACKLESS contract. The body must
    // touch sp NOWHERE — no call/frame/return mnemonic, no sp operand — so a
    // supervisor interrupt can bookmark and resume it. Build-fatal and NEVER
    // softened (`@as_compat` does not reach it): the whole VBlank-bookmark safety
    // argument rests on this property. The register-state half is the mandatory
    // `check_clobbers` above; this owns the stackless half + its two guards.
    if proc.is_resumable() {
        check_resumable(proc, &buf, ctx.cpu, diags);
    }

    // 12. Cycle budgets (delta spec §4 / U-spec §4-cycles): the declared
    // `@budget(cycles: N)` ceiling and the `@cycles_exact` equal-cost proof.
    // Purely declaration-driven — a proc carrying neither attribute is not walked.
    check_cycle_budget(file, proc, &buf, &ctx, diags);

    // The enumerated-dispatch `targets(...)` validity check (§1) is NOT here: it
    // runs in `lower_code_buf` (super::lower_code_buf, called above at #? for this
    // proc's buf), the single chokepoint EVERY code buf funnels through — a named
    // proc, a dispatch-table inline body, a script body, or an item-position
    // `asm {}` template. Running it there catches a `targets(...)` clause on any
    // path with no double-report.
}

/// Report the `[stack.*]` findings for one proc body.
///
/// Tier (U-spec §6): ERROR, softening to a WARNING under `@as_compat` and
/// suppressible per-module with `@allow("stack.unbalanced")`. Unlike a declared
/// contract — which the author opted into and which therefore never softens — this
/// gate reads raw ported assembly that no one annotated, so a faithful port keeps
/// the finding visible without failing the build.
///
/// The checker's own silence discipline does the soundness work
/// ([`crate::preserves::check_stack_balance`]): wherever the stack model bails,
/// nothing fires, so an ERROR here always names a delta the analysis followed
/// exactly.
fn check_stack_balance(
    file: &ast::File,
    proc: &ast::ProcDecl,
    buf: &crate::value::CodeBuf,
    as_compat: bool,
    diags: &mut Vec<Diagnostic>,
) {
    use crate::preserves::StackFindingKind as K;
    let level = if as_compat { Level::Warning } else { Level::Error };
    let allow_unbalanced = super::allows_lint(file, "stack.unbalanced");
    let allow_mismatch = super::allows_lint(file, "stack.merge-mismatch");
    // A declared `falls_into` continues into its successor rather than returning,
    // so control running off the end of THIS body is not a return and the pair may
    // legitimately share one frame across the boundary.
    let charge_fall_off_end = proc.falls_into.is_none();
    for f in crate::preserves::check_stack_balance(&buf.items, charge_fall_off_end) {
        if match f.kind {
            K::Unbalanced { .. } => allow_unbalanced,
            K::MergeMismatch { .. } => allow_mismatch,
        } {
            continue;
        }
        let (id, what) = match f.kind {
            K::Unbalanced { depth } => (
                "stack.unbalanced",
                format!(
                    "this path returns with {depth} bytes still on the stack — sp is below its \
                     entry value, so the return reads its address from the wrong word"
                ),
            ),
            K::MergeMismatch { existing, incoming } => (
                "stack.merge-mismatch",
                format!(
                    "paths reach this point holding different amounts of stack ({existing} and \
                     {incoming} bytes) — the code past the merge runs at an sp that depends on \
                     the branch taken"
                ),
            ),
        };
        push(diags, level, f.span, format!("[{id}] in `{}`: {what}", proc.name));
    }
}

/// Report the `@resumable` (stackless) contract failures for one proc body
/// (bookmark ask 1). Three diagnostics, all ERROR-tier and NEVER softened — the
/// VBlank-bookmark safety argument rests on the stackless property, so a faithful
/// port cannot opt out of it the way it opts out of modernization lints:
///
/// - `[resumable.z80-unsupported]` — the stack model and the bookmark mechanism
///   are 68k; `@resumable` on a Z80 proc is rejected (matching the `inout`
///   facet's Z80 scope guard). No stackless scan follows.
/// - `[resumable.contract-required]` — a `@resumable` proc MUST declare its
///   register-state set via `clobbers(...)`. Without it there is nothing to bound
///   the body's liveness against, so the "anything outside the set is an error"
///   half (the mandatory `check_clobbers`) has no set and is vacuous.
/// - `[resumable.stack-op]` — one per sp-touching instruction the body contains
///   ([`crate::resumable::scan_stack_ops`], which scans the evaluated/spliced
///   `CodeBuf` — post-`with`-splice, pre-backend-encoding — so a stack op
///   arriving via a `with` bracket or a template splice is caught too).
fn check_resumable(
    proc: &ast::ProcDecl,
    buf: &crate::value::CodeBuf,
    cpu: Cpu,
    diags: &mut Vec<Diagnostic>,
) {
    if cpu == Cpu::Z80 {
        push(
            diags,
            Level::Error,
            proc.span,
            format!(
                "[resumable.z80-unsupported] `{}` declares `@resumable`, but the stackless \
                 contract is 68k-only (the supervisor bookmark mechanism it enables is 68k)",
                proc.name
            ),
        );
        return;
    }
    if proc.clobbers.is_none() {
        push(
            diags,
            Level::Error,
            proc.span,
            format!(
                "[resumable.contract-required] `@resumable` proc `{}` must declare its \
                 register-state set with `clobbers(...)` — it is what bounds the body's \
                 liveness (a touch outside params/clobbers/out is `[proc.clobber-undeclared]`)",
                proc.name
            ),
        );
    }
    for f in crate::resumable::scan_stack_ops(&buf.items) {
        push(
            diags,
            Level::Error,
            f.span,
            format!(
                "[resumable.stack-op] in `{}`: {} — a `@resumable` proc must keep all live \
                 state in registers and touch the stack nowhere (it exits by `jmp (aN)`)",
                proc.name, f.what
            ),
        );
    }
}

/// Report the `@continuation` (manufactured-frame license) validity failures
/// (bookmark ask 4). The license itself — the stack-balance exemption — is applied
/// at the call site; this owns its two guards, both ERROR-tier and NEVER softened
/// (a manufactured-transfer trust root is a strict opt-in, not a faithful-port
/// lint):
///
/// - `[continuation.z80-unsupported]` — the manufactured-frame mechanism (the
///   68k exception frame, `rte`, the stack model it relaxes) is 68k; matching the
///   `@resumable`/`inout` Z80 scope guards.
/// - `[continuation.contract-required]` — a `@continuation` proc MUST declare its
///   register-state set with `clobbers(...)`. It is entered mid-transfer with live
///   registers the checker cannot prove; the declared set is the trusted contract
///   consumers compile against. An undeclared set would silently exempt the proc
///   from the clobber lint (`check_clobbers` gates on `clobbers.is_some()`) with no
///   register contract at all.
fn check_continuation(proc: &ast::ProcDecl, cpu: Cpu, diags: &mut Vec<Diagnostic>) {
    if cpu == Cpu::Z80 {
        push(
            diags,
            Level::Error,
            proc.span,
            format!(
                "[continuation.z80-unsupported] `{}` declares `@continuation`, but the \
                 manufactured-frame mechanism (the 68k exception frame + `rte` it licenses) is \
                 68k-only",
                proc.name
            ),
        );
        return;
    }
    if proc.clobbers.is_none() {
        push(
            diags,
            Level::Error,
            proc.span,
            format!(
                "[continuation.contract-required] `@continuation` proc `{}` must declare its \
                 register-state set with `clobbers(...)` — it is entered mid-transfer with live \
                 registers the checker cannot prove, so the declared set is the trusted contract",
                proc.name
            ),
        );
    }
}

/// Report `[noreturn.returns]` for a `@noreturn` proc whose body can return.
///
/// The claim is CHECKED locally: a `@noreturn` body must have no `Edge::Return`
/// (an `rts`/`rte`/`rtr`/`rtd`, or the Z80 `ret`-class — conditional returns
/// included, since the returning instruction carries the edge whatever branch
/// reaches it) and no `Edge::FallOff` (control off the end into whatever
/// follows). Every path must leave by a real tail transfer or a back edge (a
/// loop). Two refinements the panel proved necessary (amended spec §1, master
/// `ad670db4`):
///
///   * **`falls_into` composes.** A `FallOff` is honest IFF the proc's declared
///     `falls_into` names a symbol that is ITSELF `@noreturn` — the successor
///     never returns either, so neither does this proc. Any other fall-off
///     returns into its successor and is refused.
///   * **A trailing-local transfer is a fall-off.** A transfer to a LOCAL label
///     that CLOSES the body (no instruction after it) reaches the fall-off point
///     where this proc's analysis still applies. The shared `Cfg::branch_edge`
///     three-way classifies it as an `Edge::FallOff` on BOTH CPUs, so this walk
///     reads it off the edge like any other fall-off — no consumer-side
///     trailing-label test is needed.
///
/// What CANNOT be checked here — that a real transfer's TARGET never returns —
/// is the transitive claim, trusted exactly like every other declared contract
/// in the closure. Error-tier, unsuppressible: a false `@noreturn` misinforms
/// every consumer that trusts it.
fn check_noreturn(
    proc: &ast::ProcDecl,
    buf: &crate::value::CodeBuf,
    cpu: Cpu,
    noreturn: &BTreeSet<String>,
    diags: &mut Vec<Diagnostic>,
) {
    use crate::flag_check::{Cfg, Edge};
    let cfg = Cfg::build(&buf.items);
    // A fall-off is honest only when the declared successor is itself `@noreturn`.
    // A trailing-local transfer is one of those fall-offs — `Cfg::branch_edge`
    // classifies it as `Edge::FallOff`, so it needs no special case here.
    let falls_into_noreturn = proc.falls_into.as_deref().is_some_and(|s| noreturn.contains(s));
    for (idx, item) in buf.items.iter().enumerate() {
        let CodeItem::Instr { span, .. } = item else { continue };
        let edges = match cpu {
            Cpu::Z80 => cfg.z80_edges(idx),
            _ => cfg.edges(idx),
        };
        for e in edges {
            let why = match e {
                Edge::Return => Some("returns here (a return instruction)"),
                Edge::FallOff if !falls_into_noreturn => {
                    Some("runs off the end of the body into whatever follows")
                }
                Edge::FallOff | Edge::Follow(_) | Edge::TailOut | Edge::BranchOut => None,
            };
            if let Some(why) = why {
                push(
                    diags,
                    Level::Error,
                    *span,
                    format!(
                        "[noreturn.returns] `{}` is declared `@noreturn` but a path {why} — a \
                         `@noreturn` body must leave only by a tail transfer or a loop (a \
                         `falls_into` a `@noreturn` successor composes)",
                        proc.name
                    ),
                );
            }
        }
    }
}

/// Report the `[cycles.*]` findings for one proc body.
///
/// Tier (U-spec §6, "Budget overrun: error · `@as_compat` n/a · `@allow` no"):
/// ERROR, unsoftened and unsuppressible. Both halves of that follow from the
/// attribute being the opt-in: a budget is a claim its author wrote down, so it
/// never softens (a wrong contract is worse than none), and `@allow` would be a
/// contradiction — asking for the proof and then discarding it. Deleting the
/// attribute is the free, honest downgrade, and it is visible in the source.
///
/// The finding is a fact about ONE FILE — a proc's own instructions and its own
/// control flow — which is what licenses a hard build failure here (the
/// one-file/whole-corpus tier rule, delta spec §7.2). Anything needing knowledge
/// past the proc boundary is refused by the walk instead of guessed.
fn check_cycle_budget(
    file: &ast::File,
    proc: &ast::ProcDecl,
    buf: &crate::value::CodeBuf,
    ctx: &ProcCtx,
    diags: &mut Vec<Diagnostic>,
) {
    let declared: Vec<&ast::Attr> = proc
        .attrs
        .iter()
        .filter(|a| a.name == "budget" || a.name == "cycles_exact")
        .collect();
    let Some(&first) = declared.first() else {
        return;
    };
    // Each attribute states a whole contract for the proc, so a repeat is two
    // claims where the reader sees one. Reporting the extras keeps the surface
    // honest instead of picking one and discarding the rest.
    for dup in duplicate_attrs(&declared) {
        push(
            diags,
            Level::Error,
            dup.span,
            format!(
                "[cycles.form] in `{}`: `@{}` is declared more than once — one declaration \
                 states the whole contract",
                proc.name, dup.name
            ),
        );
    }
    let budget = match budget_cycles(file, proc, ctx, diags) {
        Ok(b) => b,
        // The declared ceiling did not fold to an integer; the form error is
        // already reported and walking would only add a second complaint about
        // the same broken declaration.
        Err(()) => return,
    };
    let exact = declared.iter().any(|a| a.name == "cycles_exact");
    // The DECLARATION is what a verdict is about, and it is always in this file —
    // a spliced body's instructions may carry another module's spans.
    let decl_span = first.span;
    for f in crate::cycle_budget::check_cycle_budget(
        &buf.items, ctx.cpu, decl_span, budget, exact, ctx.noreturn,
    ) {
        let what = match (&f.kind, &proc.falls_into) {
            // A declared fallthrough is not an unknown escape — the successor is
            // named and checked. Its COST still leaves this proc, so the refusal
            // stands, but the reason the reader gets should be the true one. An
            // EMPTY body with a declared successor is the same fact at zero
            // instructions.
            (
                crate::cycle_budget::BudgetFindingKind::UnboundedTransfer { .. }
                | crate::cycle_budget::BudgetFindingKind::EmptyBody,
                Some(next),
            ) => {
                format!(
                    "control falls through into `{next}`, so this proc's paths do not end \
                     here — a cycle budget needs every path to end at a return"
                )
            }
            _ => f.kind.message(),
        };
        push(
            diags,
            Level::Error,
            f.span,
            format!("[{}] in `{}`: {}", f.kind.lint_id(), proc.name, what),
        );
    }
}

/// The attributes after the FIRST of each name — the repeats.
fn duplicate_attrs<'a>(attrs: &[&'a ast::Attr]) -> Vec<&'a ast::Attr> {
    let mut seen: Vec<&str> = Vec::new();
    let mut dups = Vec::new();
    for a in attrs {
        if seen.contains(&a.name.as_str()) {
            dups.push(*a);
        } else {
            seen.push(a.name.as_str());
        }
    }
    dups
}

/// The `@budget(cycles: N)` ceiling on `proc`, folded to an integer. `Ok(None)`
/// means the proc declares no budget; `Err(())` means it declares one that is not
/// a non-negative comptime integer (already reported).
fn budget_cycles(
    file: &ast::File,
    proc: &ast::ProcDecl,
    ctx: &ProcCtx,
    diags: &mut Vec<Diagnostic>,
) -> Result<Option<u64>, ()> {
    let Some(attr) = proc.attrs.iter().find(|a| a.name == "budget") else {
        return Ok(None);
    };
    // The parser has already rejected any shape but a single `cycles:` argument.
    let Some(arg) = attr.args.first().filter(|a| a.name.as_deref() == Some("cycles")) else {
        return Err(());
    };
    let (val, mut ds) = crate::layout::eval_attr_int(file, &arg.value, ctx.defines);
    diags.append(&mut ds);
    match val {
        Some(n) if n >= 0 => Ok(Some(n as u64)),
        _ => {
            push(
                diags,
                Level::Error,
                attr.span,
                format!(
                    "[cycles.form] in `{}`: `@budget(cycles: N)` needs a non-negative \
                     comptime integer",
                    proc.name
                ),
            );
            Err(())
        }
    }
}

/// Validate a proc's `requires`/`grants` names against the contexts in scope.
///
/// - a name no `context` item declares is `[context.unknown]`;
/// - `grants` of an ACQUIRED context is `[context.not-grantable]` — the mirror of
///   `with` on a granted one. A grant asserts "this already holds when my body
///   runs", which for a context the compiler itself brackets is both unverifiable
///   and wrong at the obvious site (a proc that ESTABLISHES the context with a
///   `with` would then be analyzed as already holding it at entry, so its own
///   acquire reads as a double-take).
fn check_context_clauses(
    file: &ast::File,
    proc: &ast::ProcDecl,
    diags: &mut Vec<Diagnostic>,
) {
    if proc.requires.is_empty() && proc.grants.is_empty() {
        return;
    }
    let mut declared: std::collections::BTreeMap<&str, &ast::ContextKind> = Default::default();
    collect_context_decls(&file.items, &mut declared);
    // A name a `use m.{name}` brings in counts as in scope even when the DECL is
    // not present in this item list. The resolver normally injects the decl, but
    // a single module lowered standalone (the port harnesses' shape) has only its
    // own `use` lines — and an import the author wrote is not the typo this check
    // exists to catch.
    let mut imported: std::collections::BTreeSet<&str> = Default::default();
    collect_use_names(&file.items, &mut imported);
    for (ctx, span) in proc.requires.iter().chain(proc.grants.iter()) {
        if !declared.contains_key(ctx.as_str()) && !imported.contains(ctx.as_str()) {
            push(
                diags,
                Level::Error,
                *span,
                format!(
                    "[context.unknown] `{}` requires/grants `{ctx}`, which names no context \
                     in scope — declare `context {ctx} {{ … }}` or import it",
                    proc.name
                ),
            );
        }
    }
    for (ctx, span) in &proc.grants {
        if matches!(declared.get(ctx.as_str()), Some(ast::ContextKind::Acquired { .. })) {
            push(
                diags,
                Level::Error,
                *span,
                format!(
                    "[context.not-grantable] `{ctx}` is an ACQUIRED context — it is entered by \
                     a `with {ctx} {{ … }}` bracket, not asserted. Only a `granted` context is \
                     a trust root"
                ),
            );
        }
    }
}

/// Every name a `use m.{a, b}` list brings into this file, recursing `section {}`
/// bodies like every other item walk. A glob or whole-module `use` brings no
/// checkable name here, so a context behind one is accepted by the corpus gate
/// rather than this one.
fn collect_use_names<'a>(items: &'a [ast::Item], out: &mut std::collections::BTreeSet<&'a str>) {
    for item in items {
        match item {
            ast::Item::Use(u) => {
                if let ast::UseNames::List(names) = &u.names {
                    out.extend(names.iter().map(|n| n.as_str()));
                }
            }
            ast::Item::Section(s) => collect_use_names(&s.items, out),
            _ => {}
        }
    }
}

/// Every `context` name in scope for this file (its own + the resolver-injected
/// imports), recursing `section {}` bodies like every other item walk.
fn collect_context_decls<'a>(
    items: &'a [ast::Item],
    out: &mut std::collections::BTreeMap<&'a str, &'a ast::ContextKind>,
) {
    for item in items {
        match item {
            ast::Item::Context(c) => {
                out.insert(c.name.as_str(), &c.kind);
            }
            ast::Item::Section(s) => collect_context_decls(&s.items, out),
            _ => {}
        }
    }
}

/// Report the `[context.*]` bracket firings for one proc body, against the
/// regions and mark-nesting firings its caller already recovered.
fn check_context_brackets(
    name: &str,
    buf: &crate::value::CodeBuf,
    regions: &[crate::context::Region],
    mark_firings: Vec<crate::context::ContextFiring>,
    cpu: Cpu,
    diags: &mut Vec<Diagnostic>,
) {
    use crate::context::ContextFiringKind as K;
    for f in crate::context::check_regions(name, &buf.items, cpu, regions, mark_firings) {
        let (id, what) = match f.kind {
            K::Escape => (
                "context.escape",
                format!(
                    "this path leaves the `with {}` region without reaching its release — \
                     the context stays held past the bracket",
                    f.ctx
                ),
            ),
            K::EntrySkip => (
                "context.entry-skip",
                format!(
                    "this branch enters the `with {}` region past its acquire — the context \
                     would be released without ever being taken",
                    f.ctx
                ),
            ),
            K::Reacquire => (
                "context.reacquire",
                format!(
                    "`{}` is already active here — an acquired context is not reentrant, so \
                     the inner release would free the outer hold",
                    f.ctx
                ),
            ),
        };
        push(diags, Level::Error, f.span, format!("[{id}] in `{name}`: {what}"));
    }
}

/// Verify each conditional result's SURVIVES claim (delta spec §7.1): `out(rN if
/// cc)` with rN absent from `clobbers(...)` says rN is untouched on every `!cc`
/// return path, and [`crate::out_verify::check_cond_out_survives`] proves it by
/// the same machinery `preserves` uses, scoped to those returns.
///
/// **The caller gates this on a DECLARED `clobbers(...)` clause** (mirroring
/// `check_clobbers`): §7.1's rule reads clobbers MEMBERSHIP, so a proc with no
/// clobber contract states nothing about its failure edges and there is no claim
/// to check.
///
/// The DEFER pattern is `check_preserves`': prove under the conservative
/// `ClobberAll` (per-file lowering has no cross-file callee knowledge — a callee
/// here may be a synthetic, contract-less stub), and re-probe anything that fails
/// under the optimistic `PreserveAll`. A register that verifies only under the
/// probe is blocked SOLELY by calls, a fact only the whole-corpus closure can
/// settle — stay silent and let `corpus_contracts`' oracle run, which is the
/// final authority and gated by the strict suite.
fn check_survives_claims(
    proc: &ast::ProcDecl,
    buf: &crate::value::CodeBuf,
    diags: &mut Vec<Diagnostic>,
) {
    use crate::preserves::CallPolicy;
    // `cond_out_pairs` drops a register that is ALSO named unconditionally: its
    // out is unconditional, and forcing it into `clobbers` to discharge a claim
    // would trip `[proc.out-clobbers-overlap]` — a contract with no legal
    // spelling. It also owns the canonicalisation, so no consumer re-derives it.
    let cond: Vec<(Reg, String)> = proc
        .cond_out_pairs(crate::regfile::RegFile::M68k)
        .into_iter()
        .filter_map(|(reg, cc)| Reg::from_name(&reg).map(|r| (r, cc)))
        .collect();
    if cond.is_empty() {
        return;
    }
    let clobbers = expand_reglist_regs(proc.clobbers.as_deref().unwrap_or(&[]));
    let strict = crate::out_verify::check_cond_out_survives(
        &proc.name,
        &buf.items,
        &cond,
        &clobbers,
        CallPolicy::ClobberAll,
        proc.span,
    );
    if strict.is_empty() {
        return;
    }
    let still_failing: Vec<(Reg, String)> = cond
        .iter()
        .filter(|(r, _)| strict.iter().any(|f| f.reg == r.to_string()))
        .cloned()
        .collect();
    for f in crate::out_verify::check_cond_out_survives(
        &proc.name,
        &buf.items,
        &still_failing,
        &clobbers,
        CallPolicy::PreserveAll,
        proc.span,
    ) {
        push(diags, Level::Error, f.span, crate::out_verify::survives_message(&f));
    }
}

/// Verify a `(cpu: z80)` proc's `preserves` contract (rung-2 §4.2/§3.2) via the
/// push/pop [`z80_preserves`](crate::z80_preserves) proof. The CHECKED set is the
/// proc's own declared `preserves(...)` UNIONED with the module's inherited
/// `invariant: preserves(...)` (`invariant_regs`) — so an `invariant(ix)` module
/// makes EVERY proc prove it preserves `ix`, even one with no explicit contract
/// (the psg-header-line-60 bug class, now a compile error). Each reglist is read
/// through the Z80 register file (item 1), so a bad register is
/// `[contract.unknown-register]`. A register the proof cannot verify preserved is
/// `[proc.preserves-unverifiable]` (error — a wrong contract is worse than none,
/// the D2.32 principle kept). NOTHING to prove ⇒ nothing runs (the vacuous pass
/// the three landed Z80 modules take — item 7).
fn check_z80_preserves(
    proc: &ast::ProcDecl,
    buf: &crate::value::CodeBuf,
    invariant_regs: &[String],
    callee_preserves: &crate::z80_preserves::CalleePreserves,
    noreturn: &BTreeSet<String>,
    diags: &mut Vec<Diagnostic>,
) {
    use crate::preserves::PreserveStatus;
    // The declared preserves reglist → Z80 units (validated), plus the inherited
    // invariant units.
    let mut check: BTreeSet<String> = crate::regfile::expand_reglist(
        &proc.preserves,
        crate::regfile::RegFile::Z80,
        |reason| {
            push(
                diags,
                Level::Error,
                proc.span,
                format!("[proc.preserves-invalid] `{}` declares an invalid `preserves` register: {reason}", proc.name),
            )
        },
    );
    check.extend(invariant_regs.iter().cloned());
    if check.is_empty() {
        return;
    }
    let checklist: Vec<String> = check.into_iter().collect();
    let statuses =
        crate::z80_preserves::verify_z80_preserved(
            &buf.items,
            &checklist,
            invariant_regs,
            callee_preserves,
            proc.falls_into.as_deref(),
            noreturn,
        );
    for (reg, status) in statuses {
        // Whether `reg` is an INHERITED invariant (vs an explicit preserve) — for
        // a message that names WHY the proc must preserve it.
        let inherited = invariant_regs.contains(&reg);
        match status {
            PreserveStatus::Verified => {}
            PreserveStatus::NotPreserved => push(
                diags,
                Level::Error,
                proc.span,
                if inherited {
                    format!("[proc.preserves-unverifiable] `{}` breaks the module invariant `preserves({reg})` — `{reg}` is written and not restored", proc.name)
                } else {
                    format!("[proc.preserves-unverifiable] `{}` declares `preserves({reg})` but `{reg}` is written and not restored", proc.name)
                },
            ),
            PreserveStatus::Unverifiable(why) => push(
                diags,
                Level::Error,
                proc.span,
                if inherited {
                    format!("[proc.preserves-unverifiable] `{}` cannot verify the module invariant `preserves({reg})`: {why}", proc.name)
                } else {
                    format!("[proc.preserves-unverifiable] `{}` cannot verify `preserves({reg})`: {why}", proc.name)
                },
            ),
        }
    }
}

/// The status flags a `out(carry: name)` result may name — the 68000 CCR bits.
/// `carry` is the sole corpus demand; the rest are accepted for forward use.
const VALID_FLAGS: [&str; 5] = ["carry", "zero", "negative", "overflow", "extend"];

/// The 68000 condition codes a `out(rN if cc)` guard may name (incl. the `hs`/`lo`
/// aliases of `cc`/`cs`). `t`/`f` are legal cc encodings but nonsensical as a
/// result guard, so they are NOT accepted here.
const VALID_CCS: [&str; 16] = [
    "hi", "ls", "cc", "cs", "ne", "eq", "vc", "vs", "pl", "mi", "ge", "lt", "gt", "le", "hs", "lo",
];

/// Validate `out(carry: name)` flag results and `out(rN if cc)` conditional
/// register results (§6): a flag name outside [`VALID_FLAGS`] is
/// `[proc.out-flag-invalid]`; a condition code outside [`VALID_CCS`] or a
/// non-register `reg` is `[proc.out-cond-invalid]`. Both error-tier, mirroring
/// `[proc.out-invalid]`. (The conditional register's `reg` also rides the `out`
/// reglist, so its register-spelling validity is already covered by
/// `[proc.out-invalid]`; here we only police the `cc`.)
fn check_out_flags_cond(
    proc_name: &str,
    flags: &[ast::FlagResult],
    conds: &[ast::CondResult],
    diags: &mut Vec<Diagnostic>,
) {
    for f in flags {
        if !VALID_FLAGS.contains(&f.flag.as_str()) {
            push(
                diags,
                Level::Error,
                f.span,
                format!(
                    "[proc.out-flag-invalid] `{proc_name}` declares `out({}: …)` — `{}` is not a \
                     status flag (expected one of {})",
                    f.flag,
                    f.flag,
                    VALID_FLAGS.join(", "),
                ),
            );
        }
    }
    for c in conds {
        if !VALID_CCS.contains(&c.cc.as_str()) {
            push(
                diags,
                Level::Error,
                c.span,
                format!(
                    "[proc.out-cond-invalid] `{proc_name}` declares `out({} if {})` — `{}` is not a \
                     condition code",
                    c.reg, c.cc, c.cc,
                ),
            );
        }
    }
}

/// `falls_into next` requires `next` to be the item immediately following `proc`
/// in declaration order (§5.1) — otherwise the two procs are not physically
/// adjacent and the fall cannot happen. Any intervening item (proc or data), an
/// out-of-order target, or a non-proc / missing next item is
/// `[proc.fallthrough-separated]`.
fn check_fallthrough_adjacent(
    proc: &ast::ProcDecl,
    next: &str,
    index: usize,
    items: &[ast::Item],
    diags: &mut Vec<Diagnostic>,
) {
    let adjacent = matches!(items.get(index + 1), Some(ast::Item::Proc(p)) if p.name == next);
    if !adjacent {
        push(
            diags,
            Level::Error,
            proc.span,
            format!(
                "[proc.fallthrough-separated] `{}` declares `falls_into {next}`, but `{next}` is \
                 not the immediately-following proc in the section — declared fallthrough requires \
                 the two procs to be adjacent (nothing may sit between them)",
                proc.name
            ),
        );
    }
}

/// A proc with no `falls_into` whose body does not end in an unconditional
/// terminator can reach its closing `}` and run into whatever follows —
/// `[proc.undeclared-fallthrough]` (default-on warning, §5.1). T4 inspects only
/// the LAST `Instr` item's mnemonic (conditional branches like `bne` / `jr cc`
/// do NOT terminate); the full reachability analysis is deferred (S2-D6/D7).
fn check_undeclared_fallthrough(
    proc: &ast::ProcDecl,
    buf: &crate::value::CodeBuf,
    cpu: Cpu,
    diags: &mut Vec<Diagnostic>,
) {
    if !ends_in_terminator(buf, cpu) {
        push(
            diags,
            Level::Warning,
            proc.span,
            format!(
                "[proc.undeclared-fallthrough] `{}` can reach its closing `}}` without an \
                 unconditional terminator and does not declare `falls_into` — it will run into \
                 whatever follows it",
                proc.name
            ),
        );
    }
}

/// True when the buf's LAST instruction is an unconditional terminator — the
/// shared core of the proc- and dispatch-body fallthrough lints (same
/// last-mnemonic heuristic, S2-D6/D7 defers full reachability). Exposed
/// `pub(super)` so `lower/script.rs`'s `[script.fallthrough]` check (R9b.9) can
/// reuse the very same terminator recognition (D9.6: a script body that reaches
/// its closing `}` without a terminator runs into whatever follows).
pub(super) fn ends_in_terminator(buf: &crate::value::CodeBuf, cpu: Cpu) -> bool {
    buf.items
        .iter()
        .rev()
        .find_map(|it| match it {
            CodeItem::Instr { mnemonic, .. } => Some(mnemonic.as_str()),
            _ => None,
        })
        .is_some_and(|m| is_terminator(m, cpu))
}

/// 9a (R9a.4): a dispatch member's inline body is an anonymous proc with no
/// `falls_into` surface — a body that can reach its closing `}` without an
/// unconditional terminator runs into the next member's body (or whatever
/// follows the dispatch). Member-flavored mirror of
/// [`check_undeclared_fallthrough`]; silenced under `@as_compat` by the caller,
/// like every modernization lint.
pub(super) fn check_member_body_fallthrough(
    table: &str,
    member: &crate::ast::DispatchMember,
    buf: &crate::value::CodeBuf,
    cpu: Cpu,
    diags: &mut Vec<Diagnostic>,
) {
    if !ends_in_terminator(buf, cpu) {
        push(
            diags,
            Level::Warning,
            member.span,
            format!(
                "[dispatch.body-fallthrough] dispatch `{table}` member `{}`'s inline body can \
                 reach its closing `}}` without an unconditional terminator — it will run into \
                 whatever follows it",
                member.name
            ),
        );
    }
}

/// True for an UNCONDITIONAL control-transfer mnemonic that ends straight-line
/// flow. Conditional forms (`bcc`/`bne`, `jr cc`) and calls (`bsr`/`jsr`) are
/// deliberately excluded — they may fall through.
fn is_terminator(mnemonic: &str, cpu: Cpu) -> bool {
    match cpu {
        // `jbra` (emp auto-reaching branch, D2.18) is an unconditional transfer,
        // so it terminates like `bra`/`jmp`; `jbsr` (a call) is deliberately NOT
        // a terminator — control returns, mirroring `bsr`/`jsr`. `illegal`
        // terminates too: it is the S2-D11(e) `todo!`/`unreachable!` trap —
        // straight-line flow never continues past it (the error vector takes
        // over), so a proc ending in a hole must not ALSO warn fallthrough.
        Cpu::M68000 => matches!(mnemonic, "rts" | "rte" | "bra" | "jmp" | "jbra" | "illegal"),
        Cpu::Z80 => matches!(mnemonic, "ret" | "jp" | "jr"),
    }
}

/// Scan the resolved body for register writes outside `clobbers(...)` ∪ params
/// (§5.1, D-P4.9). HEURISTIC: for the standard write-form mnemonics, the
/// destination is the last operand; if it is a `Dn`/`An` not in the allowed set,
/// warn `[proc.clobber-undeclared]`. Non-writing / control mnemonics,
/// memory-destination writes, and compiler-authored SR writes (a `with` bracket's
/// spliced acquire/release, the `assert` desugar's save/restore — each checked at
/// its author's own surface) never trigger. The full register-dataflow contract
/// is the deferred S2-D6 sub-milestone.
///
/// This lint is **68k-only**: the write-form set and `Reg` display below are 68k
/// concepts, so a Z80 proc gets no clobber lint (mirroring the CPU asymmetry in
/// [`is_terminator`]). It also assumes param NAMES are register spellings
/// (`a0`/`d2`/…), which is today's model (§5.1); if params ever gain symbolic
/// names bound to registers, a write to that register would false-positive here.
fn check_clobbers(
    proc: &ast::ProcDecl,
    buf: &crate::value::CodeBuf,
    cpu: Cpu,
    noreturn: &BTreeSet<String>,
    sr_mask_preservers: &BTreeMap<String, BTreeSet<String>>,
    invariant_regs: &[String],
    callee_preserves: &crate::z80_preserves::CalleePreserves,
    diags: &mut Vec<Diagnostic>,
) {
    // On Z80 (rung-2 §2/§2.2, gap 1) the reglist validates against the Z80
    // register file via the CPU-aware recognizer, NOT the 68k universe — so
    // `clobbers(af, b)` is accepted and `clobbers(d0)` is `[proc.clobber-invalid]`.
    //
    // Reglist validation used to be the WHOLE job here, on the reasoning that the
    // undeclared-write lint below is 68k-shaped (it reads 68k `Reg` destinations).
    // That left `clobbers(...)` on Z80 checked for SPELLING only: a proc writing a
    // register it never declared compiled with zero diagnostics, while the
    // identical 68k shape fired immediately. The claim did not stay inside sigil —
    // aeon's `sound_psg.emp`/`sound_fm.emp` headers state the register contract is
    // "machine-checked" and cite a prior bug caused by a false clobber comment as
    // the reason to trust it, over ~9,600 lines of Z80 sound driver (lens sweep,
    // seat Vb, finding S4). What made it insidious is that the claim was two-thirds
    // true: `preserves(...)` and `out(...)` ARE verified on Z80.
    //
    // The write detector already existed — `z80_written_registers` is what the
    // transitive clobber closure uses as this proc's `local_writes`. So the closure
    // knew the writes all along; only the local per-proc lint was missing. Sharing
    // the one detector is deliberate: a clobber claim and a preserve proof must not
    // drift apart on the same instruction.
    if cpu == Cpu::Z80 {
        let clob = crate::regfile::expand_reglist(
            proc.clobbers.as_deref().unwrap_or(&[]),
            crate::regfile::RegFile::Z80,
            |reason| {
                push(
                    diags,
                    Level::Error,
                    proc.span,
                    format!(
                        "[proc.clobber-invalid] `{}` declares an invalid `clobber` register: {reason}",
                        proc.name
                    ),
                )
            },
        );
        let mut allowed: HashSet<String> = clob.into_iter().collect();
        // Same allowances the 68k arm grants, in the Z80 vocabulary: params are
        // declarative register bindings, `out`/`inout` are results the proc exists
        // to produce, and a pair name expands to its halves.
        let expand_quiet = |segs: &[(String, Option<String>)]| -> Vec<String> {
            crate::regfile::expand_reglist(segs, crate::regfile::RegFile::Z80, |_| {})
                .into_iter()
                .collect()
        };
        let param_segs: Vec<(String, Option<String>)> =
            proc.params.iter().map(|(n, _, _)| (n.clone(), None)).collect();
        allowed.extend(expand_quiet(&param_segs));
        allowed.extend(expand_quiet(proc.out.as_deref().unwrap_or(&[])));
        allowed.extend(expand_quiet(proc.inout.as_deref().unwrap_or(&[])));
        // VERIFIED preserves only — the same subtraction the 68k arm makes, and for
        // the same reason: a declared-but-unprovable `preserves` must subtract
        // nothing, so the lint keeps its teeth against a lying clause. Module
        // `invariant` regs are inherited onto every proc's preserve set (§3.2), so
        // they are checked here exactly as `check_z80_preserves` checks them.
        let mut check: Vec<String> = proc.preserves.iter().map(|(r, _)| r.clone()).collect();
        check.extend(invariant_regs.iter().cloned());
        if !check.is_empty() {
            let statuses = crate::z80_preserves::verify_z80_preserved(
                &buf.items,
                &check,
                invariant_regs,
                callee_preserves,
                proc.falls_into.as_deref(),
                noreturn,
            );
            for (reg, status) in statuses {
                if matches!(status, crate::preserves::PreserveStatus::Verified) {
                    allowed.insert(reg);
                }
            }
        }
        // Conservatism, stated so it is not mistaken for completeness: the shared
        // detector is a curated, FALSE-NEGATIVE-leaning allowlist (a
        // memory-destination form writes no register; `sp`/`i`/`r` and the shadow
        // bank are outside the tracked unit set). So this lint proves "every write
        // it models is declared", not "the declared set is exhaustive". That is the
        // same bound the Z80 preserve proof and the clobber closure already carry —
        // sharing one detector is what keeps the three from disagreeing about the
        // same instruction.
        for w in crate::z80_preserves::z80_written_registers(buf) {
            if allowed.contains(w.as_str()) {
                continue;
            }
            push(
                diags,
                Level::Warning,
                proc.span,
                format!(
                    "[proc.clobber-undeclared] `{}` writes `{w}`, which is not in its \
                     contract — add it to `clobbers(...)`, or `preserves({w})` if the \
                     body save/restores it",
                    proc.name
                ),
            );
        }
        return;
    }
    // Expand + validate the clobbers reglist (C1 items 2/6): ranges expand to
    // their register set, and an invalid entry (`clobbers(d9)`/typo) is a loud
    // `[proc.clobber-invalid]` error at THIS site (the primary owner).
    let clob = reglist_expand_checked(
        proc.clobbers.as_deref().unwrap_or(&[]),
        "clobber",
        &proc.name,
        proc.span,
        diags,
    );
    let mut allowed: HashSet<String> = clob.regs;
    // Params are declarative register bindings (§5.1): a write to a param
    // register is part of the proc's own contract, not an undeclared clobber.
    allowed.extend(proc.params.iter().map(|(name, _, _)| name.clone()));
    // Output registers (S2-D6e) are RESULTS: the proc writes them for the
    // caller to read, so a write to one is part of the contract, not an
    // undeclared clobber. THIS is the immediate win — it silences
    // clobber-undeclared on every declared output register. (Whether such a
    // register is actually written is a SEPARATE concern: `check_out`'s
    // `[proc.out-unwritten]` catches a declared-but-never-written output.)
    // `out`'s own validation runs in `check_out`, so expand it quietly here.
    let outs = reglist_set_quiet(proc.out.as_deref().unwrap_or(&[]));
    allowed.extend(outs.regs);
    // In-out registers are BOTH input and result: the proc advances/re-produces
    // them for the caller (`addq.w #1, d5` on an `inout(d5)` cursor), so a write is
    // part of the contract, not an undeclared clobber. They are also required to be
    // params (`[proc.inout-not-param]`), so this is belt-and-braces with the param
    // extend above — but it keeps the allowance even if that rule is violated.
    let inouts = reglist_set_quiet(proc.inout.as_deref().unwrap_or(&[]));
    allowed.extend(inouts.regs);
    // §5 VERIFIED preserves are allowed writes too (S2-D6 FP-kill): a register the
    // proc writes but provably SAVE/RESTORES round-trips to its entry value — it is
    // preserved, not clobbered, so it must not fire `[proc.clobber-undeclared]`.
    // This is the SAME subtraction the transitive closure already trusts
    // (`closure.rs`'s `− verifiedPreserved(P)`); aligning the local WARN lint with
    // it removes the dishonest-`clobbers()`-pressure FP class (AllocDynamic a0,
    // Collected_Park/UnparkSlot a0, EntityWindow_TrySpawn* d3/d5 — all honestly
    // `preserves(...)`-declared, §5-verified). `verified_preserves_regs` returns
    // ONLY the declared set that PASSES §5 (∅ on any preserves error), so a
    // declared-but-UNVERIFIABLE preserves subtracts nothing and the register still
    // fires — the lint keeps its teeth against a lying `preserves`.
    allowed.extend(verified_preserves_regs(proc, buf, noreturn, sr_mask_preservers));
    // §6 partial-width: a `preserves(dN.w)` licenses clobbering the FULL `dN` (the
    // upper word is unspecified, the whole register clobberable from a caller's
    // conservative-v1 view) — so it silences `[proc.clobber-undeclared]` on `dN`
    // exactly as `clobbers(dN)` would, WITHOUT crediting `dN` as preserved to the
    // closure. The low-word round-trip is verified separately (`check_preserves`).
    allowed.extend(preserve_word_regs(proc));
    // Whether any clause covers the MASK half — computed once, read per item.
    // A whole-SR destination always writes the mask, so only a mask-covering
    // token (`sr` or `sr.mask`, in any clause) addresses it; `sr.ccr` alone
    // does not silence a mask write.
    let mask_covered = clob.sr.mask || outs.sr.mask || SrCover::of(&proc.preserves).mask;

    for item in buf.items.iter() {
        let CodeItem::Instr { mnemonic, ops, span, author, .. } = item else { continue };
        // An SR destination is a machine-state clobber (tranche 5): undeclared
        // unless the contract covers the MASK half — bare `sr` or `sr.mask` in
        // `clobbers`/`out`/`preserves` (the preserves balance is checked
        // separately) — or the write is COMPILER-AUTHORED with its obligation
        // held at the author's own surface: a `Context`-authored write is the
        // bracket's declaration (its round-trip is proven at the context
        // DEFINITION, `lower_with`), and an `AssertDesugar` write is the
        // desugar's (its balance is pinned at the emission site). Authorship
        // redirects the check, never waives it. The bar is mask coverage
        // because a whole-SR destination always perturbs the mask:
        // `clobbers(sr.ccr)` alone leaves that perturbation undeclared and
        // still fires. (CCR-ONLY destinations never trip this lint at all —
        // ledgered.) A bracketed BODY's hand-written SR write is `User`-
        // authored and still fires. Only a write-form mnemonic can target SR.
        if writes_dest_register(mnemonic)
            && matches!(ops.last(), Some(CodeOperand::Sr))
            && !mask_covered
            && !matches!(
                author,
                crate::value::ItemAuthor::Context { .. } | crate::value::ItemAuthor::AssertDesugar
            )
        {
            push(
                diags,
                Level::Warning,
                *span,
                format!(
                    "[proc.sr-undeclared] `{}` writes `sr` (interrupt mask / condition \
                     codes), which is not in its contract — declare `clobbers(sr)`, or \
                     `preserves(sr)` if the body save/restores it",
                    proc.name
                ),
            );
            continue;
        }
        // A DECLARED or context-exempt SR write falls through to the
        // register-clobber arm below, exactly as it always has. Harmless: an SR
        // destination contributes no `Reg` write, and the `move.w (sp)+, sr`
        // restore's only register effect is the a7 auto-inc, which
        // `is_sp_discipline` exempts.
        // The written registers (write-form destination + — after the auto-inc
        // fix — `(An)+`/`-(An)` bases). Reuse `Reg`'s `Display` for the
        // canonical `d0`..`a7` spelling.
        for r in instr_written_regs(mnemonic, ops) {
            // Stack DISCIPLINE on a7 is not a register clobber — every
            // push/pop-balancing proc adjusts sp, and balanced-stack
            // verification is S2-D7(b)'s dataflow job. Two forms: ARITHMETIC
            // (`addq.l #2, sp` / `lea N(sp), sp` cleanup) and PUSH/POP
            // (`move.l x, -(sp)` / `(sp)+`, now that auto-inc/dec advances of
            // a7 are detected). Stack REPLACEMENT (`movea.l x, sp` — switching
            // stacks) stays a genuine a7 clobber and is NOT exempt (tranche-3
            // review scoping).
            if r == crate::value::Reg::A7 && is_sp_discipline(mnemonic, ops) {
                continue;
            }
            let name = r.to_string();
            if !allowed.contains(name.as_str()) {
                push(
                    diags,
                    Level::Warning,
                    *span,
                    format!(
                        "[proc.clobber-undeclared] `{}` writes `{name}`, which is not in its \
                         `clobbers(...)` set or parameter list (heuristic lint — full register \
                         dataflow is deferred to S2-D6)",
                        proc.name
                    ),
                );
            }
        }
    }
}

/// True for a 68k write-form mnemonic whose LAST operand is the written
/// destination (`move`/`add`/`lea`/`clr`/…, plus the `s<cc>` family and
/// `movep`/`addx`). Read-only / control forms (`cmp`, `tst`, `btst`, `bra`,
/// `bsr`, `jmp`, `jsr`, `pea`, `nop`, `rts`…) return `false` so they never trip
/// the lint.
///
/// `dbcc`/`dbra`/`dbf` decrement their FIRST operand, not the last, and `movem`
/// writes a register-LIST destination; both are covered by [`instr_written_regs`]
/// effects (3)/(4) directly, NOT this last-operand predicate — so they return
/// `false` here.
///
/// **ISA-DERIVED (S2-D6 U1).** This is no longer a parallel string list the
/// compiler cannot keep honest: it parses the mnemonic to the backend
/// [`sigil_isa::m68k::Mnemonic`] and defers to the EXHAUSTIVE
/// [`sigil_isa::m68k::writes_last_operand`] classifier. Adding a mnemonic to the
/// ISA enum fails that classifier's match to compile — so a newly-supported
/// write-form can no longer silently escape the lint. A string that does not
/// parse to a 68k mnemonic (a Z80 mnemonic, a pseudo-op) is not a 68k write-form
/// → `false` (the lint is 68k-only; see [`check_clobbers`]).
pub(crate) fn writes_dest_register(m: &str) -> bool {
    crate::lower::m68k_mnemonic(m).map(sigil_backend_m68k::m68k::writes_last_operand).unwrap_or(false)
}

/// Every REGISTER this instruction modifies, per the clobber/out write
/// heuristic — the single shared detector behind `check_clobbers`,
/// `check_out`, and the contract census ([`proc_written_registers`]). Two
/// disjoint effects:
///
/// 1. **Write-form destination.** For a [`writes_dest_register`] mnemonic whose
///    LAST operand is a register (`move x, d3` / `lea T, a0`), that register is
///    written. (An SR/CCR/memory destination is not a register write and is
///    handled by the callers separately.)
/// 2. **Auto-increment / -decrement address modification.** `(An)+` and `-(An)`
///    ADVANCE `An` as a side effect regardless of operand position (source OR
///    destination) and regardless of the mnemonic — `move.w (a4)+, d0` writes
///    BOTH `d0` (dest) and `a4` (post-increment); `tst.w (a0)+` writes `a0`
///    even though `tst` is read-only. This closes the auto-inc/dec
///    write-analysis gap ([out-clause, 2026-07-11] gap-ledger row): a pointer
///    result advanced only through `(a4)+` is a genuine write of `a4`, so it
///    can be declared `out(a4)` without a false `[proc.out-unwritten]`, and a
///    proc that scribbles `a4` via `(a4)+` no longer escapes
///    `[proc.clobber-undeclared]`. `a7` via `(sp)+`/`-(sp)` (push/pop) is stack
///    discipline — reported here for honesty but exempted by `check_clobbers`.
/// 3. **`dbcc`-family loop counter (S2-D6).** `dbf`/`dbra`/`dbeq`/… `dN, <label>`
///    DECREMENTS its first-operand data register `dN`; the "destination is the
///    last operand" model (effect 1) does not hold for it, so it is handled
///    explicitly. Closes the tranche-4 "dbcc clobber-lint blind spot": the write
///    set the closure's `local_writes` trusts no longer misses a counter register.
///    (Live corpus impact 0 — every `dbf` counter is `moveq`-initialized first,
///    already counted — but a completeness hole in an ERROR gate's input.)
/// 4. **`movem`-LOAD register list (S2-D6).** `movem <ea>, <reglist>` (reglist =
///    LAST operand = destination, e.g. `movem.l (a0)+, d0-d6/a2`) WRITES every
///    listed register (fresh values). **CLOBBER-LINT POLARITY — read before
///    touching:** a `(sp)+` stack RESTORE (`movem.l (sp)+, d0-d7`) is EXEMPTED
///    (its reglist is preserve-discipline, the direct parallel of the `a7`
///    push/pop exemption in effect 2 — counting a restored reglist would
///    false-positive a defensive over-save `movem d0-d7,-(sp)…(sp)+,d0-d7
///    clobbers(d0-d3)` into a d4-d7 clobber). So this detector is NOT ISA-true
///    for a `(sp)+` movem — it deliberately omits the restored reglist. **Any
///    consumer needing ISA-true movem-load semantics must mask-expand the reglist
///    ITSELF and not rely on this function.** The current such consumers already
///    do exactly that (and dedupe against this): `out_verify::produced_regs`,
///    `calls::written_names`, `preserves::ever_clobbered`, and
///    `preserves::transfer` (whose `is_pop` early-return handles the stack case
///    before it ever reaches this detector). The polarity lives here (not in a
///    caller) because `check_clobbers` consumes this directly for per-span
///    diagnostics and cannot route through `proc_written_registers`.
///
/// Registers are returned in encounter order (dest first, then operand order),
/// DEDUPED (an instruction that advances the same register twice reports it
/// once, so `check_clobbers` does not double-warn at one span). Still a
/// heuristic (this is assembly): the full register-dataflow contract is the
/// deferred S2-D6 sub-milestone.
pub(crate) fn instr_written_regs(mnemonic: &str, ops: &[CodeOperand]) -> Vec<Reg> {
    let mut regs: Vec<Reg> = Vec::new();
    // (1) Write-form destination register (last operand).
    if writes_dest_register(mnemonic) {
        if let Some(CodeOperand::Reg(r)) = ops.last() {
            regs.push(*r);
        }
    }
    // (2) Auto-inc/dec base registers — ANY operand position, ANY mnemonic.
    for op in ops {
        if let CodeOperand::PostInc(r) | CodeOperand::PreDec(r) = op {
            regs.push(*r);
        }
    }
    // (3) dbcc-family counter (first operand, a data register). `db<cc>` is the
    // only mnemonic family spelled `db*`, and the push is further gated on the
    // first operand being a register — matching the `starts_with("db")`
    // convention `flag_check`/`out_verify` already use.
    if mnemonic.starts_with("db") {
        if let Some(CodeOperand::Reg(r)) = ops.first() {
            regs.push(*r);
        }
    }
    // (4) movem-LOAD reglist (last operand = RegList destination), EXCEPT a
    // `(sp)+` stack restore (preserve-discipline exemption — see the doc above).
    if let Some(CodeOperand::RegList(mask)) = ops.last() {
        if !matches!(ops.first(), Some(CodeOperand::PostInc(Reg::A7))) {
            regs.extend(crate::preserves::expand_mask(*mask));
        }
    }
    // Dedup (order-preserving): one instruction may advance the same register
    // twice (`move.w (a0)+, (a0)+`) — report it once so `check_clobbers` does
    // not emit two identical warnings at one span.
    let mut seen = Vec::new();
    regs.retain(|r| if seen.contains(r) { false } else { seen.push(*r); true });
    regs
}

/// The union write set over a resolved body — "the lint's computed write set"
/// (§5.1). The contract census consumes this verbatim to diff a proc's
/// declared `clobbers`/`out` against what it actually writes; `check_out`
/// builds its own `written` set from the same [`instr_written_regs`] detector,
/// so the two never drift. Register spellings are canonical (`d0`..`a7`,
/// `sp`→`a7`) and sorted (BTreeSet) for a deterministic report.
pub fn proc_written_registers(buf: &crate::value::CodeBuf) -> BTreeSet<String> {
    let mut written = BTreeSet::new();
    for item in &buf.items {
        if let CodeItem::Instr { mnemonic, ops, .. } = item {
            for r in instr_written_regs(mnemonic, ops) {
                written.insert(r.to_string());
            }
        }
    }
    written
}

/// Verify the declared `preserves(...)` set against the literal movem
/// save/restore pair (S2-D6b, the SYNTACTIC slice — no dataflow; the full
/// register-contract batch stays gated on S2-D6). The rule: the body's FIRST
/// `movem <list>, -(sp)` and LAST `movem (sp)+, <list>` must both exist (save
/// before restore) and both lists must equal the declared set exactly.
/// A proc that preserves registers some other way (individual pushes) cannot
/// declare `preserves` yet — a missing pair is an error, not a shrug, because
/// a wrong contract is worse than none.
///
/// 68k-only, like the clobber lint (movem/`sp` are 68k concepts); a Z80 proc
/// declaring `preserves` gets the missing-pair error, which is honest — the
/// slice cannot verify it.
fn check_preserves(
    proc: &ast::ProcDecl,
    buf: &crate::value::CodeBuf,
    noreturn: &BTreeSet<String>,
    sr_mask_preservers: &BTreeMap<String, BTreeSet<String>>,
    diags: &mut Vec<Diagnostic>,
) {
    // Fold the declared segments to the canonical movem mask
    // (bit0=D0..bit7=D7, bit8=A0..bit15=A7 — the `CodeOperand::RegList`
    // convention).
    let mut declared: u16 = 0;
    let mut bad = false;
    // The SR family is contract vocabulary (tranche 5 / the sr split): not
    // movem registers, so each token rides its OWN verification below instead
    // of the mask fold. `sr` means both halves; `sr.mask`/`sr.ccr` name one.
    // The bare token is tracked apart from its halves because verification
    // dispatches on the SPELLING: bare `sr` keeps the historical round-trip
    // slice only (its CCR half stays unverified — ledgered, S2-D7), while an
    // explicit `sr.ccr` claim must prove its half or refuse. Alone only — an
    // SR token in a range falls through to the invalid-register error.
    let mut pres_sr = SrCover::default();
    let mut explicit_bare_sr = false;
    let mut explicit_sr_ccr = false;
    for (lo, hi) in &proc.preserves {
        if hi.is_none() && pres_sr.fold_token(lo) {
            explicit_bare_sr |= lo == "sr";
            explicit_sr_ccr |= lo == "sr.ccr";
            continue;
        }
        // Bare `ccr` is not a contract token — the CCR half of SR is spelled
        // `sr.ccr` (one spelling for the partition). Steer rather than accept a
        // synonym.
        if lo == "ccr" && hi.is_none() {
            push(
                diags,
                Level::Error,
                proc.span,
                format!(
                    "[proc.preserves-invalid] `{}` declares `preserves(ccr)` — the \
                     condition-code half of SR is spelled `preserves(sr.ccr)` (bare `sr` \
                     covers both halves)",
                    proc.name
                ),
            );
            bad = true;
            continue;
        }
        // A dotted REGISTER facet — `preserves(dN.w)` (§6 partial-width). The SR
        // half tokens (`sr.mask`/`sr.ccr`) were consumed above, so a remaining
        // dotted token is a register-facet attempt. The ONE facet with witnesses
        // is `.w` on a DATA register (the low word); every other spelling refuses
        // with its own arm. The mask itself is folded quietly by
        // [`preserve_word_mask`]; this arm only VALIDATES.
        if hi.is_none() {
            let facet = WordFacet::fold_token(lo);
            if !matches!(facet, WordFacet::NotAFacet) {
                if let WordFacet::Rejected(why) = facet {
                    let reason = match why {
                        WordFacetError::NotARegister => {
                            let regtok = lo.split_once('.').map(|(r, _)| r).unwrap_or(lo);
                            format!("`{regtok}` is not a register (d0-d7/a0-a7/sp)")
                        }
                        WordFacetError::AddressWord => {
                            "a word facet is a DATA-register form only — an address-register `.w` \
                             write sign-extends into the whole register, so it is not a \
                             partial-width claim"
                                .to_string()
                        }
                        WordFacetError::Byte => {
                            "there is no `.b` facet — the only partial-width facet is `.w`, the \
                             low word"
                                .to_string()
                        }
                        WordFacetError::Long => {
                            let regtok = lo.split_once('.').map(|(r, _)| r).unwrap_or(lo);
                            format!(
                                "bare `{regtok}` IS the full-width claim; `.l` is not a separate \
                                 facet spelling"
                            )
                        }
                        WordFacetError::Unknown => {
                            "the only partial-width facet is `.w`, the low word".to_string()
                        }
                    };
                    push(
                        diags,
                        Level::Error,
                        proc.span,
                        format!(
                            "[proc.preserves-invalid] `{}` declares `preserves({lo})` — {reason}",
                            proc.name
                        ),
                    );
                    bad = true;
                }
                continue;
            }
        }
        let Some(lo_bit) = preserves_reg_bit(lo) else {
            push(
                diags,
                Level::Error,
                proc.span,
                format!(
                    "[proc.preserves-invalid] `{}` declares `preserves({lo}{})` — `{lo}` is \
                     not a register (d0-d7/a0-a7/sp)",
                    proc.name,
                    hi.as_deref().map(|h| format!("-{h}")).unwrap_or_default(),
                ),
            );
            bad = true;
            continue;
        };
        let hi_bit = match hi {
            None => lo_bit,
            Some(h) => match preserves_reg_bit(h) {
                Some(b) if b >= lo_bit => b,
                Some(_) => {
                    push(
                        diags,
                        Level::Error,
                        proc.span,
                        format!(
                            "[proc.preserves-invalid] `{}` declares the reversed range \
                             `{lo}-{h}` — a reglist range runs low to high",
                            proc.name
                        ),
                    );
                    bad = true;
                    continue;
                }
                None => {
                    push(
                        diags,
                        Level::Error,
                        proc.span,
                        format!(
                            "[proc.preserves-invalid] `{}` declares `preserves({lo}-{h})` — \
                             `{h}` is not a register (d0-d7/a0-a7/sp)",
                            proc.name
                        ),
                    );
                    bad = true;
                    continue;
                }
            },
        };
        for bit in lo_bit..=hi_bit {
            declared |= 1 << bit;
        }
    }
    if bad {
        return;
    }

    // §6 partial-width word facets. A register full-preserved AND word-preserved
    // is redundant (full subsumes word) — drop it from the word set so the full
    // proof is the only obligation.
    let declared_word = preserve_word_mask(proc) & !declared;

    // A register cannot be both preserved and clobbered — a contradictory
    // contract is diagnosed, not resolved. Expand the clobbers reglist quietly
    // (C1 item 2 — `check_clobbers` owns its diagnostics). (The SR family
    // first: it has no mask bits.) The test is per HALF, so the honest
    // partition `preserves(sr.mask) clobbers(sr.ccr)` is legal while bare `sr`
    // against either half still contradicts (bare `sr` covers both).
    let clob = reglist_set_quiet(proc.clobbers.as_deref().unwrap_or(&[]));
    for (covered, half) in
        [(pres_sr.mask && clob.sr.mask, "sr.mask"), (pres_sr.ccr && clob.sr.ccr, "sr.ccr")]
    {
        if covered {
            push(
                diags,
                Level::Error,
                proc.span,
                format!(
                    "[proc.preserves-clobbers-overlap] `{}` declares the `{half}` half of SR \
                     both preserved and clobbered (bare `sr` covers both halves) — a machine \
                     state cannot be in both sets",
                    proc.name
                ),
            );
            return;
        }
    }
    for c in &clob.regs {
        if let Some(bit) = preserves_reg_bit(c) {
            if declared & (1 << bit) != 0 {
                push(
                    diags,
                    Level::Error,
                    proc.span,
                    format!(
                        "[proc.preserves-clobbers-overlap] `{}` declares `{c}` both preserved \
                         and clobbered — a register cannot be in both sets",
                        proc.name
                    ),
                );
                return;
            }
            // §6: the word facet is a preserve too — `preserves({c}.w) clobbers({c})`
            // claims the low word survives AND licenses destroying the register, a
            // contradiction. (The upper word is clobberable by the facet itself; a
            // separate `clobbers` entry is the incoherent part.)
            if declared_word & (1 << bit) != 0 {
                push(
                    diags,
                    Level::Error,
                    proc.span,
                    format!(
                        "[proc.preserves-clobbers-overlap] `{}` declares `{c}` both word-preserved \
                         (`{c}.w`) and clobbered — a register cannot be in both sets",
                        proc.name
                    ),
                );
                return;
            }
        }
    }

    // Verification dispatches on the declared SPELLING. A mask claim (bare `sr`
    // or `sr.mask`) runs the whole-SR round-trip slice — the save/restore pair
    // restores both halves, so the proof covers the mask wherever it covered
    // bare `sr`. An EXPLICIT `sr.ccr` additionally runs the CCR slice, which
    // demands every flag effect sit inside the bracket; bare `sr` runs only
    // the round-trip slice — its CCR half is UNCHECKED beyond the restore's
    // presence (a flag write after the restore is invisible here; the gap is
    // ledgered against S2-D7's dataflow half).
    if pres_sr.mask {
        let before = diags.len();
        check_preserves_sr(
            proc,
            buf,
            if explicit_bare_sr { "sr" } else { "sr.mask" },
            sr_mask_preservers,
            noreturn,
            diags,
        );
        // Advisory (noreturn-tail model, warn tier): bare `preserves(sr)` claims
        // BOTH halves, but only the mask round-trips through `check_preserves_sr`
        // — its CCR half is otherwise unverified (ledgered against S2-D7). The
        // CCR-bracket walk names a bare-`sr` proc whose condition codes are not
        // provably the caller's at return. Gated on the mask proof PASSING, so it
        // stays CCR-SPECIFIC: a proc that already failed the round-trip is not
        // also nagged about the half it never reached.
        if explicit_bare_sr && diags.len() == before {
            check_ccr_advisory(proc, buf, noreturn, diags);
        }
    }
    if explicit_sr_ccr {
        // This fn is the 68k arm of the `lower_proc` preserves dispatch (Z80
        // routes to `check_z80_preserves`), so the CPU is fixed here.
        check_preserves_sr_ccr(proc, buf, Cpu::M68000, noreturn, diags);
    }
    // §6 partial-width word facets (`preserves(dN.w)`). Same DEFER discipline as
    // the full check below — the byte gate proves what one file can (`ClobberAll`)
    // and DEFERS a call-blocked round-trip (verifies under `PreserveAll`) to the
    // corpus oracle; a LOCAL failure (no `.w`/`.l` round-trip, only a `.b`
    // fragment, an sp bailout) is a real error here. This runs independently of the
    // full mask below (a proc may declare only word facets).
    if declared_word != 0 {
        let wregs = crate::preserves::expand_mask(declared_word);
        let real = crate::preserves::verify_preserved_word(
            &buf.items,
            &wregs,
            crate::preserves::CallPolicy::ClobberAll,
            proc.falls_into.as_deref(),
            noreturn,
        );
        let not_verified: Vec<crate::value::Reg> = wregs
            .iter()
            .copied()
            .filter(|r| !matches!(real.get(r), Some(crate::preserves::PreserveStatus::Verified)))
            .collect();
        if !not_verified.is_empty() {
            let optimistic = crate::preserves::verify_preserved_word(
                &buf.items,
                &not_verified,
                crate::preserves::CallPolicy::PreserveAll,
                proc.falls_into.as_deref(),
                noreturn,
            );
            let unverifiable: Vec<String> = not_verified
                .iter()
                .filter(|r| {
                    !matches!(optimistic.get(r), Some(crate::preserves::PreserveStatus::Verified))
                })
                .map(|r| format!("{r}.w"))
                .collect();
            if !unverifiable.is_empty() {
                push(
                    diags,
                    Level::Error,
                    proc.span,
                    format!(
                        "[proc.preserves-unverifiable] `{}` declares `preserves({})` but {} not \
                         provably preserved — no `.w` (or `.l`) save/restore round-trips the low \
                         word on every return path (a `.b` restore round-trips only the byte, or an \
                         unmodeled sp op blocks the proof)",
                        proc.name,
                        unverifiable.join(", "),
                        if unverifiable.len() == 1 { "it is" } else { "they are" },
                    ),
                );
            }
        }
    }

    if declared == 0 {
        return; // no full mask (SR-family- or word-facet-only contract)
    }

    // §5 verified preserves (the dataflow upgrade — subsumes the D2.32 movem-pair
    // slice, which becomes its trivial fast path). Every declared register must be
    // provably preserved by symbolic stack tracking ([`crate::preserves`]): its
    // ENTRY value restored on every return path (individual push/pop, `(sp)` peek,
    // mid-body or entry/exit movem, or a superset save), or never written.
    //
    // The byte gate verifies under the CONSERVATIVE model (`ClobberAll` — it has no
    // cross-file contract knowledge; a callee here may be a synthetic, contract-less
    // stub). A register it cannot prove falls into two classes:
    //  - blocked ONLY by a call to a contract-less callee — a fact only the
    //    whole-corpus closure can settle. Re-verify under the OPTIMISTIC probe
    //    (`PreserveAll` — every call preserves everything); a register that verifies
    //    there is call-blocked, so the byte gate stays SILENT and DEFERS it to the
    //    corpus closure, which re-proves it with the verified-`effective` oracle and
    //    is the FINAL AUTHORITY. The strict suite ALWAYS runs that closure gate, so
    //    a genuinely-unprovable deferral still errors there — nothing silently ships.
    //  - blocked by a LOCAL hazard the closure cannot fix — an sp bailout (computed
    //    sp, sp escape, aliasing store, stack underflow), a `.w` restore
    //    (sign-extends), or a direct clobber with no restore. These fail the
    //    optimistic probe too and are a real `[proc.preserves-unverifiable]` here (a
    //    wrong contract is worse than none, the D2.32 principle kept).
    // `[proc.preserves-missing-pair]`/`-mismatch`/`-word-pair` retire, subsumed.
    let regs = crate::preserves::expand_mask(declared);
    let real = crate::preserves::verify_preserved(
        &buf.items,
        &regs,
        crate::preserves::CallPolicy::ClobberAll,
        proc.falls_into.as_deref(),
        noreturn,
    );
    let not_verified: Vec<crate::value::Reg> = regs
        .iter()
        .copied()
        .filter(|r| !matches!(real.get(r), Some(crate::preserves::PreserveStatus::Verified)))
        .collect();
    if not_verified.is_empty() {
        return;
    }
    let optimistic = crate::preserves::verify_preserved(
        &buf.items,
        &not_verified,
        crate::preserves::CallPolicy::PreserveAll,
        proc.falls_into.as_deref(),
        noreturn,
    );
    let unverifiable: Vec<String> = not_verified
        .iter()
        .filter(|r| {
            !matches!(optimistic.get(r), Some(crate::preserves::PreserveStatus::Verified))
        })
        .map(|r| r.to_string())
        .collect();
    if !unverifiable.is_empty() {
        push(
            diags,
            Level::Error,
            proc.span,
            format!(
                "[proc.preserves-unverifiable] `{}` declares `preserves({})` but {} not \
                 provably preserved — no `.l` save/restore round-trips {} FULL entry value on \
                 every return path (individual push/pop, `movem.l` pair, or `(sp)` peek), or an \
                 unmodeled sp op blocks the proof. A `.w` save/restore round-trips only the LOW \
                 WORD: if that is what the code guarantees, declare `preserves(dN.w)`",
                proc.name,
                mask_reglist(declared),
                unverifiable.join(", "),
                if unverifiable.len() == 1 { "its" } else { "their" },
            ),
        );
    }
}

/// Verify a declared `out(...)` set (S2-D6e — the third register-contract
/// partition member: returned results, beside `clobbers`' scratch and
/// `preserves`' untouched). Four checks, mirroring the `preserves` tiers:
///
/// - `[proc.out-invalid]` (ERROR) — a listed name that is not a register
///   spelling (`d0-d7`/`a0-a7`/`sp`), mirroring `[proc.preserves-invalid]`.
/// - `[proc.out-clobbers-overlap]` / `[proc.out-preserves-overlap]` (ERROR) — a
///   register in BOTH `out` and (`clobbers` | `preserves`) is a contradiction
///   (returned-and-scratch / returned-and-untouched). Preserves segments are
///   expanded to their register set for the membership test. A register whose
///   out is EXCLUSIVELY conditional (every mention carries an `if cc`) is exempt
///   from the clobbers half — result on the cc edge, scratch on the others is a
///   coherent contract, not a contradiction. `out(rN, rN if cc) clobbers(rN)`
///   keeps its unconditional mention and still errors. The preserves half has no
///   such exemption (written on any path still contradicts untouched on all
///   paths).
/// - `[proc.out-unwritten]` (WARN) — an `out`-declared register never written
///   on any path in the body is a false output claim (a stale `out()` after a
///   refactor). The dual of `[proc.clobber-undeclared]`; reuses the SAME
///   register-write detection (`writes_dest_register` → last-operand register
///   destination). Note this is a SEPARATE concern from the register being in
///   `check_clobbers`' `allowed` set: an output is allowed-to-write there AND
///   must-be-written here.
///
/// Register spelling is validated per name (unlike `preserves`, `out` names are
/// never ranges — D-out.1), so an invalid name is reported once and excluded
/// from the overlap/unwritten checks (a nonsense name has no meaningful set
/// membership). 68k + Z80 (D-out.5): outputs are a general calling-convention
/// concept, so this runs for both CPUs; the unwritten check reuses the 68k
/// write-form heuristic, which on Z80 simply finds no matching writes (a Z80
/// `out` currently cannot be verified-written — honest, like `preserves`).
/// The four in-out PARTITION rules, shared by the body-proc check ([`check_out`])
/// and the boundary-declaration pass ([`validate_boundary_inout`], for `extern
/// proc` / `type = proc`). Each is a membership test over pre-expanded sets, so both
/// callers compute the sets from their own AST shape and this is the one authority
/// for the four messages. The `[proc.inout-not-param]` rule is what routes the
/// in-out INPUT obligation through the existing param→D1b machinery, and its
/// enforcement on externs is what closes the extern-fold blessing path.
// Each parameter is one pre-expanded set the four rules test membership against;
// the arity IS the rule's input list, so it stays flat rather than bundled.
#[allow(clippy::too_many_arguments)]
pub(crate) fn check_inout_partition(
    owner: &str,
    span: Span,
    inout_valid: &[String],
    param_regs: &std::collections::HashSet<String>,
    clobbers_regs: &std::collections::HashSet<String>,
    preserved_mask: u16,
    word_mask: u16,
    out_reg_set: &std::collections::HashSet<String>,
    diags: &mut Vec<Diagnostic>,
) {
    for name in inout_valid {
        if !param_regs.contains(name) {
            push(
                diags,
                Level::Error,
                span,
                format!(
                    "[proc.inout-not-param] `{owner}` declares `inout({name})` but `{name}` is not \
                     a param — an in-out register is a caller-provided input, so it must be \
                     declared in the param list"
                ),
            );
        }
        if clobbers_regs.contains(name) {
            push(
                diags,
                Level::Error,
                span,
                format!(
                    "[proc.inout-clobbers-overlap] `{owner}` declares `{name}` both in-out and \
                     clobbered — an in-out register's exit value is promised to the caller, so it \
                     cannot also be destroyed scratch"
                ),
            );
        }
        if let Some(bit) = preserves_reg_bit(name) {
            if (preserved_mask | word_mask) & (1 << bit) != 0 {
                push(
                    diags,
                    Level::Error,
                    span,
                    format!(
                        "[proc.inout-preserves-overlap] `{owner}` declares `{name}` both in-out and \
                         preserved — an in-out register's exit value need not equal its entry \
                         value, so it cannot also be promised unchanged"
                    ),
                );
            }
        }
        if out_reg_set.contains(name) {
            push(
                diags,
                Level::Error,
                span,
                format!(
                    "[proc.inout-out-overlap] `{owner}` declares `{name}` both `inout` and `out` — \
                     an in-out register is checked as a threaded cursor (pass-through valid), an \
                     out as a produced result; a register takes exactly one"
                ),
            );
        }
    }
}

/// The full in-out structural check for a BOUNDARY declaration — an `extern proc`
/// or a `type = proc` contract type — whose body the closure never sees. Without
/// this, an `extern proc Q (a0) inout(d5)` (d5 not a param) would fold `d5` into the
/// caller-credit maps and seed it VERIFIED by §3 axiom, blessing a caller's
/// `out(d5)` with no gate firing (D1b stays silent because `d5 ∉ Q.params`). Running
/// the SAME five rules `check_out` runs on bodies closes that path at the
/// declaration. `is_z80` gates the 68k-only facet.
// One parameter per declared reglist facet, mirroring `check_out`'s body-side
// signature; the arity IS the declaration's facet list.
#[allow(clippy::too_many_arguments)]
pub(crate) fn validate_boundary_inout(
    owner: &str,
    span: Span,
    params: &std::collections::HashSet<String>,
    clobbers: Option<&[(String, Option<String>)]>,
    preserves: &[(String, Option<String>)],
    out: Option<&[(String, Option<String>)]>,
    inout: Option<&[(String, Option<String>)]>,
    is_z80: bool,
    diags: &mut Vec<Diagnostic>,
) {
    let inout_nonempty = inout.is_some_and(|v| !v.is_empty());
    if is_z80 {
        if inout_nonempty {
            push(
                diags,
                Level::Error,
                span,
                format!(
                    "[proc.inout-z80-unsupported] `{owner}` declares `inout(...)` on a Z80 \
                     declaration — the in-out facet is 68k-only"
                ),
            );
        }
        return;
    }
    let inout_set = reglist_expand_checked(inout.unwrap_or(&[]), "inout", owner, span, diags);
    let mut inout_valid: Vec<String> = inout_set.regs.into_iter().collect();
    inout_valid.sort();
    if inout_valid.is_empty() {
        return;
    }
    let clobbers_regs = reglist_set_quiet(clobbers.unwrap_or(&[])).regs;
    let out_reg_set = reglist_set_quiet(out.unwrap_or(&[])).regs;
    let mut preserved_mask: u16 = 0;
    for (lo, hi) in preserves {
        let Some(lo_bit) = preserves_reg_bit(lo) else { continue };
        let hi_bit = match hi {
            None => lo_bit,
            Some(h) => match preserves_reg_bit(h) {
                Some(b) if b >= lo_bit => b,
                _ => continue,
            },
        };
        for bit in lo_bit..=hi_bit {
            preserved_mask |= 1 << bit;
        }
    }
    check_inout_partition(
        owner,
        span,
        &inout_valid,
        params,
        &clobbers_regs,
        preserved_mask,
        /* word_mask */ 0,
        &out_reg_set,
        diags,
    );
}

fn check_out(
    proc: &ast::ProcDecl,
    buf: &crate::value::CodeBuf,
    cpu: Cpu,
    diags: &mut Vec<Diagnostic>,
) {
    // On Z80 (rung-2 §2.2, gap 1) validate the out reglist against the Z80
    // register file, then run the out∩clobbers / out∩preserves overlap checks with
    // Z80-expanded sets. The out-UNWRITTEN check is 68k-only: its write detector is
    // the 68k heuristic (`proc_written_registers`), which finds no Z80 writes, so a
    // Z80 out cannot be verified-written — honest, like `preserves` (an empty 68k
    // `written` set would otherwise false-fire unwritten on EVERY Z80 out).
    // A CONDITIONAL result (`out(rN if cc)`) is a result on the cc edge and
    // destroyed scratch on every other, so it may legitimately ALSO be declared
    // `clobbers(rN)` — that pair is not the result-or-scratch contradiction
    // `[proc.out-clobbers-overlap]` names. The parser lands such a register in
    // BOTH `out_cond` and the plain out reglist (out-verify needs it there), so
    // the overlap check subtracts it here. `out ∩ preserves` is NOT relaxed:
    // written on ANY path contradicts untouched on ALL paths.
    //
    // The exemption is keyed on (register, EXCLUSIVELY conditional): a register
    // ALSO mentioned unconditionally (`out(rN, rN if cc)`) keeps the
    // unconditional reading and still errors against `clobbers(rN)`.
    // [`ast::ProcDecl::cond_only_out_regs`] owns that keying — and the register
    // file expansion behind it, without which a raw-text set would miss `sp`/`a7`
    // on 68k and every Z80 pair spelling (`hl` expands to `h`+`l`).
    if cpu == Cpu::Z80 {
        // The in-out facet is 68k-only tonight — its exit-production verifier is
        // the 68k write/CFG machinery. A Z80 `inout(...)` is rejected rather than
        // silently unchecked.
        if proc.inout.as_deref().is_some_and(|v| !v.is_empty()) {
            push(
                diags,
                Level::Error,
                proc.span,
                format!(
                    "[proc.inout-z80-unsupported] `{}` declares `inout(...)` on a Z80 proc — the \
                     in-out facet is 68k-only",
                    proc.name
                ),
            );
        }
        let rf = crate::regfile::RegFile::Z80;
        let cond_guarded = proc.cond_only_out_regs(rf);
        let out_set = crate::regfile::expand_reglist(
            proc.out.as_deref().unwrap_or(&[]),
            rf,
            |reason| {
                push(
                    diags,
                    Level::Error,
                    proc.span,
                    format!(
                        "[proc.out-invalid] `{}` declares an invalid `out` register: {reason}",
                        proc.name
                    ),
                )
            },
        );
        let clob = crate::regfile::expand_reglist(proc.clobbers.as_deref().unwrap_or(&[]), rf, |_| {});
        let pres = crate::regfile::expand_reglist(&proc.preserves, rf, |_| {});
        for name in &out_set {
            if clob.contains(name) && !cond_guarded.contains(name) {
                push(
                    diags,
                    Level::Error,
                    proc.span,
                    format!(
                        "[proc.out-clobbers-overlap] `{}` declares `{name}` both output and \
                         clobbered — a register is either a returned result or destroyed scratch, \
                         not both",
                        proc.name
                    ),
                );
            }
            if pres.contains(name) {
                push(
                    diags,
                    Level::Error,
                    proc.span,
                    format!(
                        "[proc.out-preserves-overlap] `{}` declares `{name}` both output and \
                         preserved — a register is either a returned result or left untouched, \
                         not both",
                        proc.name
                    ),
                );
            }
        }
        // A Z80 flag result (`out(carry: …)`) or a conditional register result
        // (`out(rN if cc)`, whose `if cc` guard is read from the flags) lives in / is
        // read from `f`, so a contract that also PRESERVES the flags contradicts it:
        // `f` either carries the result to the caller or is restored to entry, not
        // both. `af` expands to {a, f}, so testing for the `f` unit in the expanded
        // preserves set covers both the `preserves(f)` and `preserves(af)` spellings.
        //
        // DIVERGENCE FROM 68k: `clobbers(f)` + `out(carry:)` REMAINS LEGAL on Z80,
        // unlike the 68k `out(carry:)` + `clobbers(sr.ccr)` error. Z80 has no
        // finer-than-`f` flag token, so clobbers-covering-`f` is the only honest
        // spelling of "flags are scratch except the carry result" — the shape 9 of the
        // corpus's 10 carry-returning procs take.
        if (!proc.out_flags.is_empty() || !proc.out_cond.is_empty()) && pres.contains("f") {
            let flag_result = proc
                .out_flags
                .first()
                .map(|fl| format!("`out({}: {})`", fl.flag, fl.name))
                .or_else(|| {
                    proc.out_cond.first().map(|c| format!("`out({} if {})`", c.reg, c.cc))
                })
                .unwrap_or_else(|| "a flag result".to_string());
            // Which preserves token covers the flags (`f` or `af`) — the one whose
            // Z80 expansion contains the `f` unit.
            let pres_token = proc
                .preserves
                .iter()
                .find(|seg| {
                    crate::regfile::expand_reglist(std::slice::from_ref(*seg), rf, |_| {})
                        .contains("f")
                })
                .map(|(lo, _)| lo.clone())
                .unwrap_or_else(|| "f".to_string());
            push(
                diags,
                Level::Error,
                proc.span,
                format!(
                    "[proc.out-preserves-overlap] `{}` declares {flag_result} and preserves the \
                     flags (`preserves({pres_token})` covers `f`) — a flag result lives in `f`, \
                     so `f` either carries the result or is restored to entry, not both",
                    proc.name
                ),
            );
        }
        return;
    }
    // Expand + validate the out reglist (C1 items 2/6): ranges (`out(d0-d1)`)
    // expand to their register set; a nonsense name is `[proc.out-invalid]` and
    // dropped from the downstream membership checks. Sorted for deterministic
    // diagnostic order.
    let out_set = reglist_expand_checked(
        proc.out.as_deref().unwrap_or(&[]),
        "out",
        &proc.name,
        proc.span,
        diags,
    );
    let mut valid: Vec<String> = out_set.regs.into_iter().collect();
    valid.sort();

    // out ∩ clobbers — returned AND scratch is contradictory. Expand the
    // clobbers reglist quietly (`check_clobbers` owns its diagnostics).
    let clobbers = reglist_set_quiet(proc.clobbers.as_deref().unwrap_or(&[]));
    let cond_guarded = proc.cond_only_out_regs(crate::regfile::RegFile::M68k);
    for name in &valid {
        if clobbers.regs.contains(name) && !cond_guarded.contains(name) {
            push(
                diags,
                Level::Error,
                proc.span,
                format!(
                    "[proc.out-clobbers-overlap] `{}` declares `{name}` both output and \
                     clobbered — a register is either a returned result or destroyed scratch, \
                     not both",
                    proc.name
                ),
            );
        }
    }

    // out ∩ preserves — returned AND untouched is contradictory. Preserves
    // stores movem-reglist segments (`(lo, Option<hi>)`): expand each to the
    // canonical mask bits and test the single output register's bit.
    let mut preserved_mask: u16 = 0;
    for (lo, hi) in &proc.preserves {
        let Some(lo_bit) = preserves_reg_bit(lo) else { continue };
        let hi_bit = match hi {
            None => lo_bit,
            Some(h) => match preserves_reg_bit(h) {
                Some(b) if b >= lo_bit => b,
                _ => continue,
            },
        };
        for bit in lo_bit..=hi_bit {
            preserved_mask |= 1 << bit;
        }
    }
    // §6 partial-width: the word facet is a PRESERVE too, so `out(dN)
    // preserves(dN.w)` is the same contradiction at half width — the low word
    // cannot both carry a returned result and hold its entry value. Read through
    // the same fold the validator uses, and reported on its own arm so the message
    // names the facet the contract actually spells.
    let word_mask = preserve_word_mask(proc);
    for name in &valid {
        if let Some(bit) = preserves_reg_bit(name) {
            if preserved_mask & (1 << bit) != 0 {
                push(
                    diags,
                    Level::Error,
                    proc.span,
                    format!(
                        "[proc.out-preserves-overlap] `{}` declares `{name}` both output and \
                         preserved — a register is either a returned result or left untouched, \
                         not both",
                        proc.name
                    ),
                );
            } else if word_mask & (1 << bit) != 0 {
                push(
                    diags,
                    Level::Error,
                    proc.span,
                    format!(
                        "[proc.out-preserves-overlap] `{}` declares `{name}` both output and \
                         word-preserved (`{name}.w`) — the low word is either a returned result \
                         or the caller's entry value, not both",
                        proc.name
                    ),
                );
            }
        }
    }

    // The in-out facet's partition rules. An `inout` register is a caller
    // obligation on BOTH sides — provided at entry, read at exit — so it is
    // mutually exclusive with every other disposition and must be a param.
    let inout_set = reglist_expand_checked(
        proc.inout.as_deref().unwrap_or(&[]),
        "inout",
        &proc.name,
        proc.span,
        diags,
    );
    let mut inout_valid: Vec<String> = inout_set.regs.into_iter().collect();
    inout_valid.sort();
    let param_regs: std::collections::HashSet<String> =
        proc.params.iter().map(|(n, _, _)| n.clone()).collect();
    let out_reg_set: std::collections::HashSet<String> = valid.iter().cloned().collect();
    check_inout_partition(
        &proc.name,
        proc.span,
        &inout_valid,
        &param_regs,
        &clobbers.regs,
        preserved_mask,
        word_mask,
        &out_reg_set,
        diags,
    );

    // The SR-half partition (the sr split). A flag result (`out(carry: …)`)
    // lives in CCR and a conditional result's `if cc` guard is read from CCR,
    // so either DEMANDS the condition codes carry this proc's exit state to
    // the caller — which contradicts a contract restoring them to entry
    // (`preserves` covering `sr.ccr`, including bare `sr`) or licensing their
    // destruction (`clobbers` covering `sr.ccr`). The mask half is disjoint
    // from every flag, so `preserves(sr.mask)` co-declares cleanly — that pair
    // of facts is what the split exists to surface. SR tokens in the out
    // reglist itself partition the same way, per half.
    let out_sr = SrCover {
        mask: out_set.sr.mask,
        ccr: out_set.sr.ccr || !proc.out_flags.is_empty() || !proc.out_cond.is_empty(),
    };
    let pres_sr = SrCover::of(&proc.preserves);
    // What the CCR demand should be NAMED as in a message: the flag result if
    // one exists, else the cond guard, else the CCR half of the declared `out`
    // SR token (spelled as the HALF, not as the user's token — bare `out(sr)`
    // reaches here too, and the parenthetical in each message states the
    // bare-token rule).
    let ccr_result = proc
        .out_flags
        .first()
        .map(|f| format!("`out({}: {})`", f.flag, f.name))
        .or_else(|| proc.out_cond.first().map(|c| format!("`out({} if {})`", c.reg, c.cc)))
        .unwrap_or_else(|| "the CCR half of its `out` SR token".to_string());
    if out_sr.ccr && pres_sr.ccr {
        push(
            diags,
            Level::Error,
            proc.span,
            format!(
                "[proc.out-preserves-overlap] `{}` declares {ccr_result} and preserves the \
                 condition codes (bare `sr` covers both halves) — a flag result lives in CCR, \
                 so CCR either carries the result or is restored to entry, not both (declare \
                 `preserves(sr.mask)` if only the interrupt mask round-trips)",
                proc.name
            ),
        );
    }
    if out_sr.ccr && clobbers.sr.ccr {
        push(
            diags,
            Level::Error,
            proc.span,
            format!(
                "[proc.out-clobbers-overlap] `{}` declares {ccr_result} and clobbers the \
                 condition codes (bare `sr` covers both halves) — a flag result lives in CCR, \
                 so CCR either carries the result or is destroyed scratch, not both",
                proc.name
            ),
        );
    }
    if out_sr.mask && pres_sr.mask {
        push(
            diags,
            Level::Error,
            proc.span,
            format!(
                "[proc.out-preserves-overlap] `{}` declares the `sr.mask` half of SR both \
                 output and preserved (bare `sr` covers both halves) — a machine state is \
                 either a returned result or left untouched, not both",
                proc.name
            ),
        );
    }
    if out_sr.mask && clobbers.sr.mask {
        push(
            diags,
            Level::Error,
            proc.span,
            format!(
                "[proc.out-clobbers-overlap] `{}` declares the `sr.mask` half of SR both \
                 output and clobbered (bare `sr` covers both halves) — a machine state is \
                 either a returned result or destroyed scratch, not both",
                proc.name
            ),
        );
    }

    // out-unwritten — a declared output never written on any path is a false
    // claim. Same write detection as check_clobbers (the shared
    // [`instr_written_regs`] detector via [`proc_written_registers`]).
    //
    // EXCEPT under a declared `falls_into`: control continues into the successor
    // inside the same call, so the output may legitimately be produced THERE and
    // never touched here — a body that only READS the register still has an honest
    // claim if the successor produces it.
    //
    // THE EXEMPTION ITSELF IS UNASSERTED, in this precise sense: pinning
    // `charge_unwritten` to TRUE — removing the exemption — is green across the
    // whole tree, so nothing states that a `falls_into` proc's declared out is
    // meant to be spared here. The opposite pin is NOT green, and the asymmetry is
    // the point: `charge_unwritten` gates this diagnostic for EVERY proc, so
    // pinning it false silences `[proc.out-unwritten]` corpus-wide and reddens
    // `lower_proc.rs`'s `out_unwritten_warns` and
    // `as_compat_does_not_silence_out_contract` plus `warn_tier_corpus.rs`'s
    // frozen id set. Those gates cover the CHECK; none covers the EXEMPTION.
    //
    // No 68k proc declares both `falls_into` and an `out`, so no corpus proc
    // reaches the spared branch. `tranche22_spelling_probes.rs`'s `Dict` fixture
    // does execute it, but that probe asserts only the absence of `Level::Error`
    // and this fires at `Level::Warning`. The frontend's other `falls_into` + `out`
    // synthetics assert over `out_verify::check_out`, a DIFFERENT function from
    // this one. The Z80 route is closed independently — the out-unwritten check is
    // 68k-only, so the single proc of that shape anywhere never reaches here.
    //
    // THIS TIER'S EXEMPTION IS WEAKER THAN THE CLOSURE'S, deliberately and
    // visibly. The closure does NOT exempt: `out_verify` charges the fall-off
    // against the SUCCESSOR's verified out, so a successor that fails to produce
    // the register still fires. Here the claim is simply dropped, proc-wide, for
    // every declared out — a single-file lint cannot see the successor's contract.
    // Do not read this as the shared line, and do not copy its shape into a
    // whole-program checker: dropping a value obligation is only safe where
    // something else re-charges it, and at this tier nothing does.
    //
    // (The stack checker's `check_stack_balance` flag IS sound to drop, because
    // the successor's own unconditional balance check discharges it. No value
    // claim has that discharge.)
    let written = proc_written_registers(buf);
    let charge_unwritten = proc.falls_into.is_none();
    for name in &valid {
        if charge_unwritten && !written.contains(name.as_str()) {
            push(
                diags,
                Level::Warning,
                proc.span,
                format!(
                    "[proc.out-unwritten] `{}` declares `out({name})` but never writes `{name}` \
                     — a declared output register must be written (a false result claim, or a \
                     stale `out()` after a refactor)",
                    proc.name
                ),
            );
        }
    }
}

/// The mask-claim save/restore-balance check (tranche 5 — S2-D7's first
/// syntactic slice; Sound_PostByte is the exhibit), run for `preserves(sr)`
/// AND `preserves(sr.mask)` (`token` names the declared spelling for the
/// message): if the body writes SR at all, a `move.w sr, -(sp)` save must
/// precede the FIRST SR write and the LAST SR write must be the
/// `move.w (sp)+, sr` restore. The full-SR round trip restores both halves,
/// so one proof serves either claim. Static order only — no path analysis (a
/// save/restore split across branches is S2-D7's dataflow half); a body with
/// NO SR writes preserves vacuously.
fn check_preserves_sr(
    proc: &ast::ProcDecl,
    buf: &crate::value::CodeBuf,
    token: &str,
    sr_mask_preservers: &BTreeMap<String, BTreeSet<String>>,
    noreturn: &BTreeSet<String>,
    diags: &mut Vec<Diagnostic>,
) {
    // The mnemonic-less tail refusals (68k preserves-through-tail credit, §2.4):
    // a declared `falls_into`, or a body that can run off its end, hands the
    // caller the successor's SR — which this slice cannot see. Applied to the
    // EXPLICIT `sr.mask` claim, whose mask round-trip nothing else guards, closing
    // the vacuity that let a fall-through / run-off body pass without proving the
    // mask survives the leave. A BARE `sr` proc leaves these two shapes to the CCR
    // advisory (WARN), which keeps its current tail behavior (§2.4) — its mask half
    // stays the S2-D7 deferral, unchanged.
    if token == "sr.mask" {
        if let Some(why) = sr_tail_refusal(proc, buf, Cpu::M68000) {
            push(
                diags,
                Level::Error,
                proc.span,
                format!(
                    "[proc.preserves-sr-unbalanced] `{}` declares `preserves({token})` but the \
                     interrupt mask is not provably the caller's at return: {why}",
                    proc.name
                ),
            );
            return;
        }
    }
    // An UNCONDITIONAL EXTERNAL tail (the QueueDMA_Critical → *.transfer shape):
    // the SR the caller finally sees is the tail-callee's, so the mask claim holds
    // only if that callee ITSELF preserves the mask. A tail to a provable
    // preserver is CREDITED (the tail edge discharged; the body's own SR writes
    // must still round-trip below); a tail to anything else is REFUSED, closing
    // the vacuity hole a mask-claiming tail-only body would pass through. A
    // transfer INTO a local label (the builder classifies it `Follow`/`FallOff`)
    // is not an external tail and stays with the round-trip slice.
    if let Some(target) = terminal_external_tail(buf, noreturn) {
        if !sr_mask_preservers_credit(sr_mask_preservers, &target) {
            push(
                diags,
                Level::Error,
                proc.span,
                format!(
                    "[proc.preserves-sr-unbalanced] `{}` declares `preserves({token})` but it \
                     leaves through an unconditional tail to `{target}`, which is not known to \
                     preserve the interrupt mask (declare `preserves(sr.mask)` on the tail \
                     target to credit the mask through the tail)",
                    proc.name
                ),
            );
            return;
        }
    }
    if !sr_writes_round_trip(buf.items.iter()) {
        push(
            diags,
            Level::Error,
            proc.span,
            format!(
                "[proc.preserves-sr-unbalanced] `{}` declares `preserves({token})` but its \
                 body's SR writes are not bracketed by the `move.w sr, -(sp)` … \
                 `move.w (sp)+, sr` pair (the syntactic slice checks static order; \
                 path-sensitive save/restore is S2-D7)",
                proc.name
            ),
        );
    }
}

/// The target symbol of a body's terminating UNCONDITIONAL EXTERNAL tail transfer
/// (`bra`/`jbra`/`jmp` to an external symbol), or `None` when the body returns,
/// runs off its end, transfers into a local label, or DIVERGES. The shape the
/// mask-claim tail credit consults: the caller-observed SR past this exit is
/// entirely the tail target's. The last instruction's SOLE edge being
/// `Edge::TailOut` identifies the unconditional external transfer — the unified
/// `Cfg::edges` builder classifies a transfer INTO a local label (including a
/// body-closing trailing label) as `Follow`/`FallOff`, so a `TailOut` here is
/// always a genuine external tail; no trailing-label special case is needed.
/// A conditional branch out is an `Edge::BranchOut` and never an instruction's
/// only edge, so the singleton test cannot admit one.
///
/// A DIVERGENT tail — an `AssertDesugar`-authored assert/raise rail, or a jump to
/// a `@noreturn` handler — never returns to the caller, so it carries no mask
/// obligation (the same `@noreturn` composition the register credit applies):
/// it reads as "no external tail" and the body's own SR discipline decides.
fn terminal_external_tail(buf: &crate::value::CodeBuf, noreturn: &BTreeSet<String>) -> Option<String> {
    use crate::flag_check::{transfer_target_sym, Cfg, Edge};
    let cfg = Cfg::build(&buf.items);
    let last = buf
        .items
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, it)| matches!(it, CodeItem::Instr { .. }).then_some(i))?;
    if cfg.edges(last) != vec![Edge::TailOut] {
        return None;
    }
    let CodeItem::Instr { ops, author, .. } = &buf.items[last] else { return None };
    if matches!(author, crate::value::ItemAuthor::AssertDesugar) {
        return None; // an authored divergent rail — never returns
    }
    let target = transfer_target_sym(ops)?;
    if noreturn.contains(target) {
        return None; // a `@noreturn` tail — never returns
    }
    Some(target.to_string())
}

/// Does the interrupt-mask-preservers oracle credit a tail to `target`? A PLAIN
/// name credits when it is a preserver — the tail enters at the owner's own entry,
/// exactly where `preserves(sr.mask)` was verified. An `Owner.label` target (the
/// shared-core `*.transfer` idiom) enters MID-body, so it credits ONLY when the
/// label is a SAVE-FIRST-BRACKET entry of the owner (`safe.contains(target)`) —
/// the SR round-trip is NOT a monotone property that survives entry-point
/// restriction (a subset-of-the-body argument, valid for `clobbers`, is INVALID
/// here): a label past the owner's save would let the entrant skip it and pop a
/// word it never pushed. [`crate::lower::collect_sr_mask_preservers`] computes the
/// safe-entry set from the owner's evaluated body.
fn sr_mask_preservers_credit(map: &BTreeMap<String, BTreeSet<String>>, target: &str) -> bool {
    if map.contains_key(target) {
        return true; // a plain-name tail into a preserver — the owner's own entry
    }
    match target.split_once('.') {
        Some((owner, _)) => map.get(owner).is_some_and(|safe| safe.contains(target)),
        None => false,
    }
}

/// The `preserves(sr.ccr)` slice: the condition codes at every return must be
/// the caller's own. Provable today in exactly one static shape — every
/// CCR-affecting instruction sits inside a `move.w sr, -(sp)` …
/// `move.w (sp)+, sr` bracket (the restore puts back the entry CCR the save
/// captured), and only CC-transparent instructions run outside it. Anything
/// this slice cannot see REFUSES with `[proc.preserves-unverifiable]` — a
/// wrong contract is worse than none (the D2.32 principle) — mirroring
/// `[proc.out-cond-survives-unverifiable]`'s stance of never trusting a flag
/// claim the analysis cannot discharge.
///
/// Refusal set, each named in the message: a flag effect outside the bracket
/// (including any call — the callee's flags are unknown), an unconditional
/// tail transfer anywhere (the flags the caller finally sees are the
/// target's) — INCLUDING the mnemonic-less tails: a declared `falls_into` and
/// a body that can run off its end, either of which hands control to a
/// successor whose flag traffic this walk cannot see — a return between save
/// and restore (that path skips the restore), nested saves, an unmatched
/// save, and the CCR-popping returns (`rtr`/`rte`). Sequential brackets are
/// legal — between them only transparent instructions can run, so each save
/// recaptures the entry CCR.
///
/// Static order only, like [`check_preserves_sr`], and with the same known
/// blindness: a conditional branch AROUND the restore is invisible (S2-D7's
/// dataflow half is the real answer), and the round trip assumes a balanced
/// stack at the restore (S2-D7(b), the same deferral the context
/// definition-site round-trip check rests on).
/// The mnemonic-less tail refusals shared by the `sr.ccr` ERROR check and the
/// bare-`sr` advisory: a declared `falls_into`, or a body that can run off its
/// end, hands control to a successor whose flag traffic this walk cannot see, so
/// the CCR the caller finally observes is not this proc's. Factored so the
/// advisory refuses these BEFORE walking, exactly as the ERROR check does (a
/// bare-`sr` proc falling into its successor must not read as silently green).
fn sr_tail_refusal(proc: &ast::ProcDecl, buf: &crate::value::CodeBuf, cpu: Cpu) -> Option<String> {
    if proc.falls_into.is_some() {
        Some("the proc falls into its successor — the flags the caller sees are the successor's".to_string())
    } else if !ends_in_terminator(buf, cpu) {
        Some("control can run off the end of the body into whatever follows".to_string())
    } else {
        None
    }
}

fn check_preserves_sr_ccr(
    proc: &ast::ProcDecl,
    buf: &crate::value::CodeBuf,
    cpu: Cpu,
    noreturn: &BTreeSet<String>,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(why) =
        sr_tail_refusal(proc, buf, cpu).or_else(|| ccr_bracket_refusal(&buf.items, noreturn))
    {
        push(
            diags,
            Level::Error,
            proc.span,
            format!(
                "[proc.preserves-unverifiable] `{}` declares `preserves(sr.ccr)` but the \
                 condition codes are not provably the caller's at return: {why} (the slice \
                 accepts only flag effects bracketed by the `move.w sr, -(sp)` … \
                 `move.w (sp)+, sr` pair; path-sensitive flag liveness is S2-D7)",
                proc.name
            ),
        );
    }
}

/// The bare-`preserves(sr)` CCR-half advisory (noreturn-tail model, warn tier).
///
/// Bare `sr` claims the mask AND the condition codes; `check_preserves_sr` proves
/// only the mask round-trip. This runs the SAME bracket walk the explicit
/// `sr.ccr` ERROR uses, at WARNING tier, so a bare-`sr` adopter whose CCR half is
/// not provably the caller's is NAMED — the interim the sr-split ledger row asked
/// for, short of S2-D7's dataflow. The walk is divergence- and local-label-aware
/// (an assert/raise rail and a `jbra .local` are not caller-visible CCR leaves),
/// so it does not false-positive on the DEBUG-shape rails the sr-split lane could
/// not enable it over. It ALSO refuses the mnemonic-less tails ([`sr_tail_refusal`])
/// the ERROR check does — a bare-`sr` proc that falls into its successor, or runs
/// off its end, must not read as silently green. WARN tier (non-blocking): the
/// honest downgrade is declaring `preserves(sr.mask)`, visible in the source.
fn check_ccr_advisory(
    proc: &ast::ProcDecl,
    buf: &crate::value::CodeBuf,
    noreturn: &BTreeSet<String>,
    diags: &mut Vec<Diagnostic>,
) {
    // The advisory is the 68k arm of the bare-`sr` preserves dispatch (Z80 routes
    // to `check_z80_preserves`), so the CPU for the tail refusal is fixed.
    if let Some(why) =
        sr_tail_refusal(proc, buf, Cpu::M68000).or_else(|| ccr_bracket_refusal(&buf.items, noreturn))
    {
        push(
            diags,
            Level::Warning,
            proc.span,
            format!(
                "[proc.ccr-advisory] `{}` declares bare `preserves(sr)` (both halves), but its \
                 condition codes are not provably the caller's at return: {why} — declare \
                 `preserves(sr.mask)` if only the interrupt mask round-trips (path-sensitive \
                 flag liveness is S2-D7)",
                proc.name
            ),
        );
    }
}

/// The CCR-bracket walk (shared by the explicit-`sr.ccr` ERROR and the bare-`sr`
/// advisory): `None` iff every CCR effect in `items` is bracketed by the SR
/// save/restore pair, else the first refusal reason.
///
/// Divergence- and local-label-aware (noreturn-tail model):
///   * `ItemAuthor::AssertDesugar` items are SKIPPED — an assert / raise rail is
///     the compiler's own divergent expansion, proven at the emission site; on
///     the success path the assert restores SR, and on the failure path the rail
///     never returns to the caller, so its internal SR save and flag effects are
///     invisible to the caller's condition codes.
///   * an unconditional transfer to a LOCAL label WITH A FOLLOWING INSTRUCTION is
///     intra-proc flow (the shared `Cfg` decides via `label_index`), not a leave;
///     a transfer to a `@noreturn` target diverges (its flags never reach the
///     caller); only a real EXTERNAL tail leaves with the target's flags. A
///     transfer to a TRAILING local label (one that closes the body) is NOT
///     intra-proc — control runs off the end — so `label_index` (None for a
///     trailing label), not `is_local_label`, is the gate. This walk reads
///     `label_index` DIRECTLY rather than the `Cfg::edges` builder, so the shared
///     `Cfg::branch_edge` three-way does not serve it; keyed on the SAME
///     `label_index` map the builder is, it is a reader-level gate that cannot
///     drift from the builder.
fn ccr_bracket_refusal(items: &[CodeItem], noreturn: &BTreeSet<String>) -> Option<String> {
    use crate::flag_check::{transfer_target_sym, Cfg};
    use crate::value::{CodeOperand, Reg};
    let cfg = Cfg::build(items);
    let mut saved = false;
    for item in items {
        let CodeItem::Instr { mnemonic, ops, author, .. } = item else { continue };
        // An assert/raise rail is the compiler's own divergent expansion — proven
        // at the emission site, invisible to the caller's CCR. Skip it whole (its
        // `move.w sr, -(sp)` frame push must NOT read as a nested bracket save).
        if matches!(author, crate::value::ItemAuthor::AssertDesugar) {
            continue;
        }
        // The bracket's own halves.
        if matches!(ops.as_slice(), [CodeOperand::Sr, CodeOperand::PreDec(Reg::A7)]) {
            if saved {
                return Some(
                    "a nested `move.w sr, -(sp)` save (the slice pairs one at a time)".into(),
                );
            }
            saved = true;
            continue;
        }
        if matches!(ops.as_slice(), [CodeOperand::PostInc(Reg::A7), CodeOperand::Sr]) {
            if !saved {
                return Some("a `move.w (sp)+, sr` restore with no prior save".into());
            }
            saved = false;
            continue;
        }
        match ccr_effect(mnemonic) {
            CcrEffect::Transparent => {}
            CcrEffect::Returns => {
                if saved {
                    return Some(format!(
                        "`{mnemonic}` returns between the save and the restore, skipping it"
                    ));
                }
            }
            CcrEffect::Leaves => {
                // A transfer to a LOCAL label with a following instruction is
                // intra-proc flow (not a leave); a transfer to a `@noreturn`
                // target diverges and its flags never reach the caller; only a
                // real external (or trailing-local) tail leaves. `label_index` is
                // None for a body-closing label, so a `jbra .end` at the end reads
                // as a leave, not intra-proc. The target is read through the
                // UNIFIED extractor so `jmp (Diverge).l` (an `AbsSym`) matches.
                match transfer_target_sym(ops) {
                    Some(t) if cfg.label_index(t).is_some() => {}
                    Some(t) if noreturn.contains(t) => {}
                    _ => {
                        return Some(format!(
                            "control leaves through `{mnemonic}` — the flags the caller sees \
                             are the target's"
                        ));
                    }
                }
            }
            CcrEffect::Writes => {
                if !saved {
                    return Some(format!(
                        "`{mnemonic}` affects the condition codes outside the bracket"
                    ));
                }
            }
        }
    }
    if saved {
        return Some("the `move.w sr, -(sp)` save is never restored".into());
    }
    None
}

/// How one mnemonic bears on the CCR slice's bracket walk.
enum CcrEffect {
    /// Provably does not write CCR and stays in the proc (the CC-inert data
    /// ops — [`crate::out_verify::cc_inert_data_op`] — plus conditional
    /// branches and `dbcc`).
    Transparent,
    /// A plain return (`rts`/`rtd`) — legal outside the bracket, a refusal
    /// inside it (that path would skip the restore).
    Returns,
    /// An unconditional tail transfer — the proc's final flags are the
    /// target's, which the slice cannot see.
    Leaves,
    /// Writes (or may write) CCR: every data operation, any call (the
    /// callee's flags are unknown), and the CCR-popping returns `rtr`/`rte`.
    Writes,
}

/// Classify `mnemonic` for [`ccr_bracket_refusal`]. Deliberately conservative,
/// like [`crate::out_verify::cc_transparent`] (whose base allowlist this
/// reuses): an unmodeled mnemonic WRITES rather than silently preserving a
/// stale flag. Divergences from that predicate, each because this slice asks
/// "is the ENTRY CCR intact" rather than "is a computed flag intact": calls
/// return with the callee's flags (writes), `rtr`/`rte` pop a stacked CCR
/// (writes), and tails/returns get their own arms so the walk can reason about
/// control leaving.
fn ccr_effect(mnemonic: &str) -> CcrEffect {
    match mnemonic {
        "jsr" | "jbsr" | "bsr" | "rtr" | "rte" => CcrEffect::Writes,
        "rts" | "rtd" => CcrEffect::Returns,
        "bra" | "jbra" | "jmp" | "jra" => CcrEffect::Leaves,
        m if crate::out_verify::cc_inert_data_op(m) => CcrEffect::Transparent,
        m if m.starts_with("db") => CcrEffect::Transparent,
        m if m.starts_with('b') && m.len() == 3 => CcrEffect::Transparent,
        _ => CcrEffect::Writes,
    }
}

/// Do the SR writes in `items` ROUND-TRIP — a `move.w sr, -(sp)` save before the
/// first write, and the `move.w (sp)+, sr` restore as the last? A stream with no
/// SR write round-trips vacuously. Static order only, no path analysis (a
/// save/restore split across branches is S2-D7's dataflow half).
///
/// `rte` loads SR wholesale from the stack with no `sr` operand, so it is the
/// one SR write the operand shapes cannot see — any `rte` fails the round trip
/// outright. (`rtr` pops only CCR; the mask survives it, so the MASK reading
/// this recognizer serves is unaffected.)
///
/// Two callers share this one recognizer so the two readings of the same idiom
/// cannot drift: [`check_preserves_sr`] reads a whole proc body against a declared
/// contract, and the context DEFINITION check
/// ([`Evaluator::lower_with`](crate::eval::Evaluator)) reads a `with` bracket's
/// spliced acquire and release together — the proof that grounds the
/// [`ItemAuthor::Context`](crate::value::ItemAuthor) exemption in
/// [`check_clobbers`].
pub(crate) fn sr_writes_round_trip<'a>(items: impl Iterator<Item = &'a CodeItem>) -> bool {
    use crate::value::{CodeOperand, Reg};
    let mut saved = false;
    let mut saved_before_first_write: Option<bool> = None;
    let mut last_write_is_restore = false;
    for item in items {
        let CodeItem::Instr { mnemonic, ops, .. } = item else { continue };
        if mnemonic == "rte" {
            return false;
        }
        // Save: `move.w sr, -(sp)` (reads SR — not an SR write).
        if matches!(ops.as_slice(), [CodeOperand::Sr, CodeOperand::PreDec(Reg::A7)]) {
            saved = true;
            continue;
        }
        // Any SR-destination form is an SR write; the restore is the
        // `move.w (sp)+, sr` spelling specifically.
        if matches!(ops.last(), Some(CodeOperand::Sr)) {
            saved_before_first_write.get_or_insert(saved);
            last_write_is_restore =
                matches!(ops.as_slice(), [CodeOperand::PostInc(Reg::A7), CodeOperand::Sr]);
        }
    }
    match saved_before_first_write {
        None => true,
        Some(saved_first) => saved_first && last_write_is_restore,
    }
}

/// True when an a7 write is stack DISCIPLINE (exempt from the clobber lint)
/// rather than stack REPLACEMENT: either sp arithmetic ([`is_sp_arithmetic`])
/// or a push/pop that advances a7 via `(sp)+`/`-(sp)`. Stack replacement stays a
/// genuine clobber (tranche-3 scoping) — including `movea.l x, sp` (a bare-a7
/// destination) AND the subtle `movea.l (sp)+, sp` (pop INTO sp), where the same
/// instruction both pops (a7 auto-inc, discipline) and loads a new SP (a bare-a7
/// destination). The auto-inc exemption must therefore NOT fire when a7 is also
/// the instruction's bare-register destination.
fn is_sp_discipline(mnemonic: &str, ops: &[CodeOperand]) -> bool {
    if is_sp_arithmetic(mnemonic, ops) {
        return true;
    }
    // A bare-a7 destination is stack REPLACEMENT — not exempt, even alongside a
    // `(sp)+`/`-(sp)` operand on the same instruction.
    let a7_is_dest =
        writes_dest_register(mnemonic) && matches!(ops.last(), Some(CodeOperand::Reg(Reg::A7)));
    !a7_is_dest
        && ops
            .iter()
            .any(|op| matches!(op, CodeOperand::PostInc(Reg::A7) | CodeOperand::PreDec(Reg::A7)))
}

/// True for an sp-DESTINATION write that is stack arithmetic rather than
/// stack replacement: the add/sub immediate families (`addq #2, sp`), or a
/// `lea` whose SOURCE is a displacement over sp itself (`lea N(sp), sp` —
/// the classic frame cleanup). `move`/`movea`-to-sp (stack switching) and
/// `lea Table, sp` do not qualify — those genuinely replace the stack.
fn is_sp_arithmetic(mnemonic: &str, ops: &[CodeOperand]) -> bool {
    match mnemonic {
        "add" | "adda" | "addi" | "addq" | "addx" | "sub" | "suba" | "subi" | "subq"
        | "subx" => true,
        "lea" => matches!(
            ops.first(),
            Some(CodeOperand::DispInd { reg: crate::value::Reg::A7, .. })
        ),
        _ => false,
    }
}

/// A register spelling to its canonical movem-mask bit (bit0=D0..bit7=D7,
/// bit8=A0..bit15=A7), via the shared spelling→register map (so `sp` works).
fn preserves_reg_bit(name: &str) -> Option<u8> {
    use crate::value::Reg;
    let r = Reg::from_name(name)?;
    let (is_a, n) = super::code::reg_kind(r);
    Some(if is_a { 8 + n } else { n })
}

/// A canonical movem-mask bit back to its register spelling (`d0`..`d7`,
/// `a0`..`a7`) — the inverse of [`preserves_reg_bit`], for expanding a range.
fn reg_bit_name(bit: u8) -> String {
    if bit < 8 { format!("d{bit}") } else { format!("a{}", bit - 8) }
}

/// Which halves of SR a contract clause covers. SR partitions into the system
/// byte (`sr.mask` — the interrupt mask, with the trace/supervisor bits it
/// shares the byte with) and the condition codes (`sr.ccr`); the bare `sr`
/// token means BOTH halves.
#[derive(Default, Clone, Copy)]
struct SrCover {
    mask: bool,
    ccr: bool,
}

impl SrCover {
    /// The cover a raw reglist declares — a zero-alloc segment scan for
    /// consumers that need only the SR halves, not the register set.
    fn of(segs: &[(String, Option<String>)]) -> SrCover {
        let mut cover = SrCover::default();
        for (lo, hi) in segs {
            if hi.is_none() {
                cover.fold_token(lo);
            }
        }
        cover
    }

    /// Fold one declared token into the cover. `true` iff `name` is an
    /// SR-family token (the caller then skips register handling).
    fn fold_token(&mut self, name: &str) -> bool {
        match name {
            "sr" => {
                self.mask = true;
                self.ccr = true;
            }
            "sr.mask" => self.mask = true,
            "sr.ccr" => self.ccr = true,
            _ => return false,
        }
        true
    }
}

/// The expansion of a `clobbers`/`out` reglist (C1 items 2 + 6): the canonical
/// register-name SET plus which SR halves were declared (the SR family is
/// machine state, not movem registers, so it rides its own cover rather than
/// mask bits). Overlapping segments FOLD — `preserves(sr, sr.ccr)` covers the
/// same halves as `preserves(sr)`, exactly as `d0, d0-d3` folds — because a
/// reglist denotes a set union everywhere in the grammar.
#[derive(Default)]
struct RegSet {
    regs: std::collections::HashSet<String>,
    sr: SrCover,
}

/// Expand + validate a `clobbers`/`out` reglist (C1 item 2 = ranges, item 6 =
/// validation), calling `on_error` with a human reason for each invalid segment.
/// Each segment is a single register (`sr` composes), or an inclusive `lo-hi`
/// movem range; a range endpoint that is not a movem register, or a reversed
/// range, is an error. Canonical names (`sp`→`a7`) so the set matches the
/// `Reg::Display` spelling `check_clobbers`/`check_out` compare against.
fn reglist_expand(segs: &[(String, Option<String>)], mut on_error: impl FnMut(String)) -> RegSet {
    let mut set = RegSet::default();
    for (lo, hi) in segs {
        match hi {
            // A single register or an SR-family token.
            None => {
                if set.sr.fold_token(lo) {
                    continue;
                }
                if lo == "ccr" {
                    on_error(
                        "`ccr` is not a contract token — the condition-code half of SR is \
                         spelled `sr.ccr` (bare `sr` covers both halves)"
                            .to_string(),
                    );
                } else if let Some(bit) = preserves_reg_bit(lo) {
                    set.regs.insert(reg_bit_name(bit));
                } else {
                    on_error(format!(
                        "`{lo}` is not a register (d0-d7/a0-a7/sp) or an SR token \
                         (sr/sr.mask/sr.ccr)"
                    ));
                }
            }
            // An inclusive `lo-hi` movem range (the SR family cannot appear in a
            // range).
            Some(h) => {
                let (Some(lo_bit), Some(hi_bit)) = (preserves_reg_bit(lo), preserves_reg_bit(h))
                else {
                    on_error(format!(
                        "`{lo}-{h}` has a non-register endpoint (a range runs d0-d7/a0-a7/sp)"
                    ));
                    continue;
                };
                if hi_bit < lo_bit {
                    on_error(format!("the range `{lo}-{h}` is reversed — a reglist range runs low to high"));
                    continue;
                }
                for bit in lo_bit..=hi_bit {
                    set.regs.insert(reg_bit_name(bit));
                }
            }
        }
    }
    set
}

/// [`reglist_expand`] that emits `[proc.{tag}-invalid]` diagnostics for each bad
/// segment (the primary validation site — C1 item 6).
fn reglist_expand_checked(
    segs: &[(String, Option<String>)],
    tag: &str,
    proc_name: &str,
    span: Span,
    diags: &mut Vec<Diagnostic>,
) -> RegSet {
    reglist_expand(segs, |reason| {
        push(
            diags,
            Level::Error,
            span,
            format!("[proc.{tag}-invalid] `{proc_name}` declares an invalid `{tag}` register: {reason}"),
        )
    })
}

/// [`reglist_expand`] with errors DISCARDED — for a secondary reader of an
/// attribute whose diagnostics another check already owns (so a bad register is
/// reported once, not thrice).
fn reglist_set_quiet(segs: &[(String, Option<String>)]) -> RegSet {
    reglist_expand(segs, |_| {})
}

/// Expand a `clobbers`/`out`/`preserves` reglist to its canonical register-name
/// SET (`d0`..`a7`), silently DROPPING the SR family (`sr`/`sr.mask`/`sr.ccr` —
/// the transitive closure is register-file only; SR stays the local
/// `[proc.sr-undeclared]` check).
/// Reused by the corpus contract walk ([`crate::corpus_contracts`]) so its
/// declared-register sets match `check_clobbers`' exactly.
pub fn expand_reglist_regs(segs: &[(String, Option<String>)]) -> BTreeSet<String> {
    reglist_set_quiet(segs).regs.into_iter().collect()
}

/// The declared `preserves` reglist folded to a canonical movem mask (bit0=d0..
/// bit15=a7), `sr` dropped and ranges expanded. Quiet — contract-shape validity
/// (invalid name / reversed range / overlap) is [`check_preserves`]' job.
fn preserve_mask(proc: &ast::ProcDecl) -> u16 {
    let mut mask = 0u16;
    for name in reglist_set_quiet(&proc.preserves).regs {
        if let Some(bit) = preserves_reg_bit(&name) {
            mask |= 1 << bit;
        }
    }
    mask
}

/// How one dotted REGISTER-facet token reads — the §6 partial-width fold, the
/// analog of [`SrCover::fold_token`] and, like it, the ONE place the spelling is
/// decided. Both the VALIDATOR (which refuses a bad facet) and the OBLIGATION fold
/// (which decides what must be proven) read this, so "accepted" and "obligated"
/// cannot disagree — a disagreement would be silent in the dangerous direction (a
/// facet accepted at the surface but never checked).
enum WordFacet {
    /// `dN.w` — the low-word facet on a data register, the one accepted form.
    /// Carries its canonical movem-mask bit.
    Word(u8),
    /// A dotted token that is NOT a legal facet, with the reason for its arm.
    Rejected(WordFacetError),
    /// Not a dotted token at all — a plain register / range endpoint.
    NotAFacet,
}

/// Why a dotted register-facet token is refused — one variant per message arm.
enum WordFacetError {
    /// The part before the dot is not a register (`foo.w`).
    NotARegister,
    /// `.w` on an ADDRESS register: an address-register word write sign-extends
    /// into the whole register, so a word facet there is not a partial claim.
    AddressWord,
    /// `.b` — no byte facet exists (only the low word has witnesses).
    Byte,
    /// `.l` — bare `dN` already IS the full-width claim.
    Long,
    /// Any other suffix.
    Unknown,
}

impl WordFacet {
    /// Read `token` as a register facet. Only `dN.w` is a facet; everything else is
    /// either rejected with its reason or is not a dotted token at all. SR halves
    /// (`sr.mask`/`sr.ccr`) are consumed as SR tokens BEFORE this is reached, so a
    /// dotted token arriving here is a register-facet attempt.
    fn fold_token(token: &str) -> WordFacet {
        let Some((regtok, suffix)) = token.split_once('.') else {
            return WordFacet::NotAFacet;
        };
        let Some(bit) = preserves_reg_bit(regtok) else {
            return WordFacet::Rejected(WordFacetError::NotARegister);
        };
        let is_data = bit < 8; // d0..d7 = bits 0..7
        match suffix {
            "w" if is_data => WordFacet::Word(bit),
            "w" => WordFacet::Rejected(WordFacetError::AddressWord),
            "b" => WordFacet::Rejected(WordFacetError::Byte),
            "l" => WordFacet::Rejected(WordFacetError::Long),
            _ => WordFacet::Rejected(WordFacetError::Unknown),
        }
    }
}

/// The `preserves(dN.w)` word-facet registers folded to a movem mask — the §6
/// partial-width facet. Quiet (spelling validity is [`check_preserves`]' job,
/// through the SAME [`WordFacet::fold_token`]): only a well-formed `dN.w`
/// contributes.
fn preserve_word_mask(proc: &ast::ProcDecl) -> u16 {
    let mut mask = 0u16;
    for (lo, hi) in &proc.preserves {
        if hi.is_some() {
            continue; // a facet is a single, never a range endpoint
        }
        if let WordFacet::Word(bit) = WordFacet::fold_token(lo) {
            mask |= 1 << bit;
        }
    }
    mask
}

/// The `preserves(dN.w)` word-facet register NAMES (`d0`..`d7`) — the clobber
/// PERMISSION half of the partial-width facet. Under conservative v1 a
/// `preserves(dN.w)` licenses clobbering the FULL `dN` for callers (the caller-
/// visible contract is identical to `clobbers(dN)`), so these names join the
/// "allowed to clobber" set everywhere a declared clobber does — the local
/// `[proc.clobber-undeclared]` allowed set and the closure's `declared_clobbers`.
/// The low-word ROUND-TRIP is the separate obligation [`check_preserves`] verifies
/// (and the corpus oracle re-verifies for a deferred, call-blocked claim).
pub fn preserve_word_regs(proc: &ast::ProcDecl) -> BTreeSet<String> {
    crate::preserves::expand_mask(preserve_word_mask(proc))
        .into_iter()
        .map(|r| r.to_string())
        .collect()
}

/// The register set a proc PROVABLY preserves with NO callee-contract knowledge —
/// the contract closure's oracle-FREE base `verifiedPreserved(P)` (§1): a register
/// the proc writes but save/restores by its OWN machinery does not escape it. The
/// closure SEED must stay oracle-free so the callee-preserves oracle it feeds
/// ([`crate::corpus_contracts`]' oracle round) cannot be circular; that round then
/// UPGRADES this monotonically with verified callee contracts.
///
/// Shape validity is checked by [`check_preserves`] against a throwaway sink (a
/// malformed contract credits nothing). The round-trip verdict itself is recomputed
/// here under `ClobberAll` rather than read off that sink, because `check_preserves`
/// now DEFERS a call-only failure (emitting no error) — the seed must NOT credit a
/// register whose only proof needs a callee contract. `sr` is dropped (out of the
/// register-file closure's scope).
pub fn verified_preserves_regs(
    proc: &ast::ProcDecl,
    buf: &crate::value::CodeBuf,
    noreturn: &BTreeSet<String>,
    sr_mask_preservers: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    if proc.preserves.is_empty() {
        return BTreeSet::new();
    }
    let mut sink = Vec::new();
    // This caller reads only the ERROR-tier round-trip verdict for the REGISTER
    // preserves (the `sr` token is dropped from the register-file closure); the
    // bare-`sr` CCR advisory (warn) is discarded. The `@noreturn` set and the
    // mask-preservers map are threaded from the caller — the SAME sources the
    // primary `check_preserves` call consumes — so this register verdict stays
    // consistent with the primary path when a proc both register-preserves AND
    // claims the mask through an external tail: a mask-tail REFUSAL is an error
    // here, and an error zeroes the credit outright.
    //
    // The threading is inert only while the mask-claiming proc's terminal tail is
    // REACHABLE — then the tail poisons every register under `ClobberAll` below and
    // the credit is empty either way. It is LOAD-BEARING when that terminal tail is
    // unreachable (dead code): `terminal_external_tail` classifies on the last
    // instruction regardless of reachability, while `verify_preserved` is
    // reachability-based, so the body's live return path can round-trip a register
    // that the empty-map mask refusal would have discarded. Measured inert on the
    // frozen corpus (byte-identical), not assumed.
    check_preserves(proc, buf, noreturn, sr_mask_preservers, &mut sink);
    if sink.iter().any(|d| matches!(d.level, Level::Error)) {
        return BTreeSet::new();
    }
    let regs = crate::preserves::expand_mask(preserve_mask(proc));
    // The oracle-free base SEED (ClobberAll): a `falls_into`/tail exit charges its
    // successor, which under ClobberAll preserves nothing — so a call-/tail-blocked
    // register seeds NotPreserved and the corpus oracle round re-credits it (the
    // empty `@noreturn` set only defers a divergent-tail credit to that round).
    let status = crate::preserves::verify_preserved(
        &buf.items,
        &regs,
        crate::preserves::CallPolicy::ClobberAll,
        proc.falls_into.as_deref(),
        &BTreeSet::new(),
    );
    if regs
        .iter()
        .all(|r| matches!(status.get(r), Some(crate::preserves::PreserveStatus::Verified)))
    {
        expand_reglist_regs(&proc.preserves)
    } else {
        BTreeSet::new()
    }
}

/// The inputs the corpus callee-preserves oracle round ([`crate::corpus_contracts`])
/// needs for one proc: `(check_regs, credit_names, word_check_regs)` — the declared
/// full registers to re-verify under the oracle, the full declared set to CREDIT iff
/// every `check_reg` round-trips, and the §6 word-facet registers whose low-word
/// round-trip the oracle must re-prove.
///
/// **The three outputs do not share a polarity, which is why they are computed
/// together.** `check_regs`/`credit_names` are a CREDIT: empty means "grant
/// nothing", so a malformed contract returning empty leaves the closure
/// conservative — safe, and the reason the shape-error guard returns empty for them.
/// `word_check_regs` is an OBLIGATION: empty means "check nothing", and a `.w` claim
/// dropped here is verified by NOBODY, because the per-file gate DEFERS a
/// call-blocked claim silently and the corpus gate is its final authority. So the
/// word set FAILS CLOSED — folded from the declared spelling and kept even when the
/// shape check errors, since that error can belong to a DIFFERENT clause of the same
/// contract (`preserves(sr.mask, d5.w)`) and losing a real obligation to an
/// unrelated diagnostic is the failure this note exists to prevent. A kept
/// obligation costs at most a firing the real Oracle re-proves away; a dropped one
/// costs an unverified contract.
///
/// The oracle round need not re-run [`check_preserves`]: an empty `check_regs`
/// already means "no credit".
pub fn preserve_oracle_inputs(
    proc: &ast::ProcDecl,
    buf: &crate::value::CodeBuf,
    noreturn: &BTreeSet<String>,
    sr_mask_preservers: &BTreeMap<String, BTreeSet<String>>,
) -> (Vec<crate::value::Reg>, BTreeSet<String>, Vec<crate::value::Reg>) {
    // §6 word facet, folded from the DECLARED spelling before any shape verdict —
    // the FAIL-CLOSED half (see the polarity note in the doc comment).
    let word_regs = crate::preserves::expand_mask(preserve_word_mask(proc));
    if proc.preserves.is_empty() {
        return (Vec::new(), BTreeSet::new(), Vec::new());
    }
    let mut sink = Vec::new();
    // ERROR-tier verdict only (see `verified_preserves_regs`); the `@noreturn` set
    // and mask-preservers map are threaded from the caller (the same sources the
    // primary path consumes), inert on the frozen corpus and measured so.
    check_preserves(proc, buf, noreturn, sr_mask_preservers, &mut sink);
    if sink.iter().any(|d| matches!(d.level, Level::Error)) {
        return (Vec::new(), BTreeSet::new(), word_regs);
    }
    (
        crate::preserves::expand_mask(preserve_mask(proc)),
        expand_reglist_regs(&proc.preserves),
        word_regs,
    )
}

/// Format a canonical movem mask back to its reglist spelling (`d0-d1/a0`) —
/// consecutive runs collapse to ranges, data registers before address
/// registers, `a7` spelled `a7`. Diagnostic-only (the inverse of the declared
/// fold, for naming masks in messages).
fn mask_reglist(mask: u16) -> String {
    let mut segs: Vec<String> = Vec::new();
    for (base, prefix) in [(0u8, 'd'), (8u8, 'a')] {
        let mut bit = 0u8;
        while bit < 8 {
            if mask & (1 << (base + bit)) == 0 {
                bit += 1;
                continue;
            }
            let start = bit;
            while bit + 1 < 8 && mask & (1 << (base + bit + 1)) != 0 {
                bit += 1;
            }
            segs.push(if start == bit {
                format!("{prefix}{start}")
            } else {
                format!("{prefix}{start}-{prefix}{bit}")
            });
            bit += 1;
        }
    }
    segs.join("/")
}

/// Push a diagnostic at `span`.
fn push(diags: &mut Vec<Diagnostic>, level: Level, span: Span, message: String) {
    diags.push(Diagnostic { level, message, primary: span });
}
