//! `asm { }` instantiation (Spec 2, Plan 4 — T3, §6.2 / D-P4.3): evaluate an
//! `asm` block's statements to a RESOLVED [`Value::Code`]. Every `{splice}` is
//! evaluated and typed HERE (not deferred): a mnemonic/size splice must be a
//! [`Width`]/[`Cc`], an operand splice must be an int / [`Reg`] / label. The
//! "what operand class is expected here vs. what did we get" decision lives in
//! ONE place — this module (it inspects [`Value`], so it stays Core-free; the
//! backend-operand MAPPING is `lower/code.rs`, per D-P4.1). A wrong-kind splice
//! is the `[asm.splice-kind]` diagnostic, naming the expected class and the
//! value's [`type_name`](Value::type_name) for the "got" side (§6.2 `~describe`).
//!
//! Label hygiene (T5, D-P4.6, §5.2/§5.3) is delegated to
//! [`crate::lower::hygiene`]: a monotonic counter `k` on the
//! [`Evaluator`](super::Evaluator) gives each instantiation a unique id, and a
//! [`LabelScope`] maps each source label to its emitted symbol — a non-`export`
//! `.name:` to the fresh, hidden `$asm{k}$name` (two instantiations never
//! collide; an intra-body reference rewrites to the same fresh symbol so the
//! branch resolves), and an `export .name:` to the stable, caller-visible
//! `Owner.name` (§5.2). The owner is the `proc` name for a proc body and the
//! instantiation id for a raw `asm { }`. This module only chooses the operand
//! CLASS and consults the scope; the label-symbol spelling lives in ONE place
//! (the hygiene module).

use super::{Env, Evaluator};
use crate::ast::{self, AsmStmt, BinOp, InstrLine, Operand, TextOrSplice};
use crate::lower::hygiene::{LabelScope, Owner};
use crate::parser::expr_span;
use crate::value::{
    CodeBuf, CodeItem, CodeOperand, ContextPhase, Reg, Value, Width, Z80Cond, Z80Index, Z80Pair,
    Z80Reg8, Z80RegClass,
};
use sigil_span::{Level, Span};

/// Collect every source-label definition in `body`, RECURSIVELY through
/// comptime-`if` branches (tranche 5, H1) — the label scope is pure syntax,
/// built before any condition can be evaluated. See the call site in
/// [`Evaluator::eval_asm_owned`] for why both-arm duplicates are benign.
/// Spans ride along so the caller can diagnose export-flag disagreement
/// between arms (same name, different flavor — the scope maps a name to ONE
/// symbol, so the flavors cannot coexist).
fn collect_labels<'a>(body: &'a [AsmStmt], out: &mut Vec<(&'a str, bool, Span)>) {
    for stmt in body {
        match stmt {
            AsmStmt::Label { name, export, span } => out.push((name.as_str(), *export, *span)),
            AsmStmt::If { then, els, .. } => {
                collect_labels(then, out);
                if let Some(els) = els {
                    collect_labels(els, out);
                }
            }
            // A `with` body's labels are ordinary labels of the ENCLOSING body
            // (the bracket is not a hygiene domain — it splices inline), so they
            // must join the same scope. Missing this arm would leave a label
            // inside a bracket unmapped and its references unresolved.
            AsmStmt::With { body, .. } => collect_labels(body, out),
            _ => {}
        }
    }
}

/// The template name a statement-call / `{expr}` splice records on the items
/// it inlines ([`crate::value::ItemAuthor::Splice`]): the callee path for a
/// call, `<expr>` for any other splice expression. Carried, not consumed —
/// the `-> Code` fn-contract check is ledger row 1551's parcel.
fn splice_template_name(expr: &ast::Expr) -> String {
    match expr {
        ast::Expr::Call { callee, .. } => callee.segments.join("."),
        _ => "<expr>".to_string(),
    }
}

impl Evaluator<'_> {
    /// Evaluate a raw `asm { }` body to a [`Value::Code`]. Its owner scope is the
    /// instantiation itself (a fresh `k`), so an exported label is stable per
    /// §5.3 but not caller-nameable — see [`eval_asm_owned`](Self::eval_asm_owned)
    /// for the proc case (owner = the proc name).
    pub(super) fn eval_asm(&mut self, body: &[AsmStmt], span: Span, env: &mut Env) -> Value {
        self.eval_asm_owned(body, span, env, None)
    }

    /// Evaluate an `asm { }` / `proc` body to a [`Value::Code`]. `owner_name` is
    /// `Some(proc)` for a proc body (its exported labels are caller-visible as
    /// `proc.name`, §5.2) and `None` for a raw `asm { }` (owner = the
    /// instantiation id). Build the [`LabelScope`] first (mapping every source
    /// label to its emitted symbol via the hygiene model), then build one
    /// [`CodeItem`] per statement, resolving label references against that scope.
    /// A statement that fails to lower emits a diagnostic and is dropped (its
    /// `Poison`-equivalent), so one bad line does not abort the whole block.
    pub(super) fn eval_asm_owned(
        &mut self,
        body: &[AsmStmt],
        _span: Span,
        env: &mut Env,
        owner_name: Option<&str>,
    ) -> Value {
        let k = self.asm_counter;
        self.asm_counter += 1;
        let module = self.module_id.clone();
        let owner = match owner_name {
            Some(name) => Owner::Proc { module, name: name.to_string() },
            None => Owner::Asm { module, k },
        };

        // Resolve every source label to its emitted symbol up front (export →
        // `Owner.name`, non-export → owner-scoped hidden symbol). Labels are
        // collected RECURSIVELY through comptime-`if` branches: the scope is
        // pure syntax (built before any condition can be evaluated), and a
        // label spelled in BOTH arms maps to the same symbol — only the
        // chosen arm ever DEFINES it, so there is no collision; a reference
        // to a label in the unchosen arm resolves in scope and fails loudly
        // at link (symbol never defined).
        let mut labels: Vec<(&str, bool, Span)> = Vec::new();
        collect_labels(body, &mut labels);
        // Same name with DIFFERENT export flavors (one arm `export .x:`, the
        // other `.x:`) cannot share the one symbol the scope maps — diagnose
        // rather than silently taking whichever arm was collected first.
        for (i, (name, export, span)) in labels.iter().enumerate() {
            if labels[..i].iter().any(|(n, e, _)| n == name && e != export) {
                self.error(
                    *span,
                    format!(
                        "label `.{name}` is defined both `export` and non-`export` \
                         across comptime-`if` arms — pick one flavor"
                    ),
                );
            }
        }
        let scope = LabelScope::build(&owner, labels.iter().map(|(n, e, _)| (*n, *e)));

        // F2 (tranche 7): enter this body's hygiene context. A proc-LOCAL label
        // CALL ARGUMENT (`axis(…, .next)`) evaluated inside this body's statement
        // loop mangles through THIS owner (the caller's space), and every such
        // minted label is recorded for the end-of-body loudness check. Save and
        // restore so nesting (a template call whose body has its own owner)
        // resolves each `.name` arg in ITS enclosing body and never leaks.
        let prev_owner = self.enclosing_owner.replace(owner.clone());
        let prev_minted = std::mem::take(&mut self.minted_local_labels);

        // Build the resolved item list.
        let mut buf = CodeBuf::empty();
        for stmt in body {
            self.lower_asm_stmt(stmt, &scope, &mut buf, env);
        }

        // Loudness (F2): every proc-local label VALUE minted from a `.name` call
        // argument in THIS body must name a label actually DEFINED in this body.
        // A miss (a typo'd `.labell`) is a loud diagnostic naming it — not a
        // silent undefined link symbol the linker would reject with a mangled name.
        let minted = std::mem::replace(&mut self.minted_local_labels, prev_minted);
        for (name, span) in minted {
            if !labels.iter().any(|(n, _, _)| *n == name) {
                self.error(
                    span,
                    format!(
                        "unknown local label `.{name}` — no `.{name}:` is defined in this \
                         proc body"
                    ),
                );
            }
        }
        self.enclosing_owner = prev_owner;

        Value::Code(buf)
    }

    /// Lower ONE proc/asm statement into `buf`, resolving labels against
    /// `scope`. Factored out of [`Self::eval_asm_owned`]'s loop so a comptime
    /// `if`'s chosen branch can lower RECURSIVELY against the same scope and
    /// buffer (tranche 5, H1) — branch statements are ordinary statements of
    /// the enclosing body, not a nested hygiene domain.
    fn lower_asm_stmt(
        &mut self,
        stmt: &AsmStmt,
        scope: &LabelScope,
        buf: &mut CodeBuf,
        env: &mut Env,
    ) {
        match stmt {
            AsmStmt::Label { name, export, span } => {
                buf.push(CodeItem::Label {
                    name: scope.label_def(name),
                    export: *export,
                    span: *span,
                });
            }
            AsmStmt::Instr(instr) => {
                // D-PP.1: a leading bareword that is NOT a recognized mnemonic
                // for this section's CPU (and not jbra/jbsr) but RESOLVES to an
                // in-scope comptime fn is a bare directive-style STATEMENT CALL
                // — pure sugar for the paren form. The decision lives HERE (not
                // the parser): only at lowering are BOTH the CPU's mnemonic
                // table and the comptime-fn table in hand. Mnemonics win
                // unconditionally, so this fires only for a bareword the
                // mnemonic table rejects; a bareword that then resolves to
                // nothing falls through to the unchanged "not a recognized
                // mnemonic" error, and a non-fn value gets a specific error.
                if let Some(spliced) = self.try_bare_statement_call(instr, env) {
                    buf.items.extend(spliced.items);
                    return;
                }
                if let Some(item) = self.lower_instr_to_item(instr, scope, env) {
                    buf.push(item);
                } else {
                    // The instruction did not lower — a mnemonic/size/operand
                    // failed to resolve, so it was about to vanish from the buffer
                    // SILENTLY, under-approximating every downstream analysis (the
                    // substrate hazard). Make it LOUD: count it and report it. A
                    // more specific operand diagnostic may already have been
                    // emitted; this is the guaranteed floor so `dropped == 0` is
                    // pinnable. In a real build nothing drops (the resolve pass
                    // supplies all imported decls).
                    self.dropped_instrs += 1;
                    self.error(
                        instr.span,
                        "instruction dropped — a mnemonic, size, or operand did not resolve \
                         (a missing import or type in scope?)",
                    );
                }
            }
            AsmStmt::Call(expr) => {
                // A statement-position call splices a nested template's items
                // in (§6.2): it MUST evaluate to a `Code` value.
                //
                // Provenance (§9, D-P4.11) — the SMALLEST HONEST version of
                // `ProvFrame::Comptime`. Core does NOT reserve a provenance
                // *stack* on a `Diagnostic` (it carries a single `primary`
                // span) nor on a `DataFragment` (a single span), so a
                // structured `ProvFrame::Comptime { call_site, def_site }`
                // cannot be attached to the emitted fragment here — that is
                // FLAGGED for the checkpoint (see the T7 report). What we CAN
                // do, Core-free and at the splice site, is name the generator
                // CALL SITE: if evaluating the generator produced any
                // diagnostics (an out-of-range value in the generated table, a
                // failed `ensure`, a bad splice, …), follow them with a `Note`
                // pointing at THIS call, so an error inside a comptime-
                // generated table is traceable back to where it was
                // instantiated (call_site = this call; def_site = the span the
                // generator's own diagnostic already carries).
                let watermark = self.diags.len();
                // A body-position `ensure` may read `cycles(L1, L2)` (rung 4): give
                // it the partial CodeBuf lowered SO FAR so the T-state span sum sees
                // every instruction textually before this guard. Snapshotted only
                // for guard statements (cheap — body ensures are rare); cleared
                // after so no other call sees a stale scope.
                // The rung-4 cycle READERS: a body `ensure(cycles(...))` guard AND a
                // `pad_to_cycles(target, cycles(...) + k)` pad emitter both read the
                // partial CodeBuf via `cycle_scope`. Snapshot for those two spellings
                // only (cheap — both are rare); cleared after so no other call sees a
                // stale scope.
                let is_cycle_reader = matches!(
                    expr,
                    ast::Expr::Call { callee, .. }
                        if callee.segments.len() == 1
                            && matches!(
                                callee.segments[0].as_str(),
                                "ensure" | "ensure_fatal" | "pad_to_cycles"
                            )
                );
                if is_cycle_reader {
                    self.cycle_scope = Some(buf.items.clone());
                }
                let v = self.eval_expr(expr, env);
                if is_cycle_reader {
                    self.cycle_scope = None;
                }
                self.note_if_comptime_error(watermark, expr_span(expr));
                match v {
                    Value::Code(mut inner) => {
                        crate::value::reauthor_user_items(
                            &mut inner.items,
                            &crate::value::ItemAuthor::Splice {
                                template: splice_template_name(expr),
                            },
                        );
                        buf.items.extend(inner.items)
                    }
                    // A body-position guard (`ensure(...)`) yields Unit and emits
                    // no code — the rung-4 `ensure(cycles(...) == N)` site.
                    Value::Unit => {}
                    Value::Poison => {}
                    other => self.error(
                        expr_span(expr),
                        format!(
                            "an `asm` statement-call must evaluate to Code, got {}",
                            other.type_name()
                        ),
                    ),
                }
            }
            AsmStmt::Let { reg, ty, span } => {
                // Spec 2, C2: a local typed-register binding. Emits ZERO bytes —
                // it writes the SAME `reg_pointee_struct` overlay params seed
                // (D6.A3), so from here down a bare field displacement `f(reg)`
                // resolves in the type's field space exactly as a param does,
                // including the tranche-7b namespace closure (a bare const does
                // not resolve in the displacement slot on a typed register). The
                // binding rides to the end of the enclosing block — the `If` arm
                // snapshots/restores the map so a branch-local `let` cannot leak.
                //
                // Register-name validity is the ONE divergence from a param
                // (params silently skip a non-register name; `let` is the decl
                // site, so it reports): a non-register name is a loud error.
                let Some(r) = Reg::from_name(reg) else {
                    self.error(
                        *span,
                        format!(
                            "[asm.let-not-register] `{reg}` is not a register; \
                             `let <reg>: <Type>` types an address/data register (aN/dN)"
                        ),
                    );
                    return;
                };
                // Only a pointer-to-struct participates in field-space
                // resolution (param-identical: a value newtype on a data
                // register — `let d0: Angle` — is accepted but has no
                // displacement effect, since data registers take no field
                // displacement). Resolve SILENTLY: an unresolvable/non-struct
                // pointee simply does not bind — its diagnostics belong to the
                // type's own decl site, and reporting here would diverge from
                // the param path (which resolves on a scratch evaluator).
                if let ast::Type::Ptr(inner) = ty {
                    let watermark = self.diags.len();
                    let inner_ty = self.resolve_type(inner);
                    if let Some(sname) = self.struct_name_for_offsetof(&inner_ty, *span) {
                        self.reg_pointee_struct.insert(r, sname);
                    }
                    self.diags.truncate(watermark);
                }
            }
            AsmStmt::Trap { kind, message, span } => {
                // S2-D11(e): both spellings assemble to the 68k ILLEGAL
                // word — the file builds and RUNS to the hole. 68k-only
                // in v1 (the script/offsets precedent): Z80 has no
                // ratified trap word.
                if self.cpu == Some(sigil_ir::backend::Cpu::Z80) {
                    self.error(
                        *span,
                        "[todo.non-68k] `todo!`/`unreachable!` are 68k-only in v1 \
                         (they assemble to the 68k ILLEGAL word; no Z80 trap \
                         encoding is ratified)",
                    );
                    return;
                }
                if matches!(kind, crate::ast::TrapKind::Todo) {
                    // One [todo.present] per site — together they ARE the
                    // "list of all todos"; `--deny-todo` promotes them.
                    let msg = match message {
                        Some(m) => format!("[todo.present] todo!: {m}"),
                        None => "[todo.present] todo! left in the build".to_string(),
                    };
                    self.warn(*span, msg);
                }
                buf.push(CodeItem::Instr {
                    mnemonic: "illegal".to_string(),
                    size: None,
                    ops: vec![],
                    span: *span,
                    as_type: None,
                    author: self.item_author.clone(),
                });
            }
            AsmStmt::If { cond, then, els, span: _ } => {
                // Tranche 5, H1: the condition must be a comptime
                // bool/int (mt_bank's define pattern — `if DEFINE == 1`).
                // The chosen branch lowers inline against the SAME scope
                // and buffer; the unchosen branch is never lowered.
                //
                // A `Poison` condition discards BOTH arms — every instruction
                // in them is invisible to `dropped_instrs` and to every
                // downstream analysis. The watermark/harvest pair records the
                // names the condition failed to resolve onto the
                // `[comptime.unresolved]` surface at the SAME eval this arm
                // decision reads, so a lost/misspelled toggle define is loud
                // where the drop count is structurally silent.
                let mark = self.unresolved_mark();
                let cond_val = self.eval_expr(cond, env);
                self.harvest_comptime_unresolved(mark);
                let truthy = match cond_val {
                    Value::Bool(b) => b,
                    Value::Int(i) => i != 0,
                    Value::Poison => return,
                    other => {
                        self.error(
                            expr_span(cond),
                            format!(
                                "[asm.if-not-comptime] a statement-position `if` \
                                 condition must be a comptime bool or int, got {}",
                                other.type_name()
                            ),
                        );
                        return;
                    }
                };
                let branch: Option<&[AsmStmt]> =
                    if truthy { Some(then) } else { els.as_deref() };
                if let Some(stmts) = branch {
                    // C2 lexical scope: a `let` binding is scoped to its
                    // enclosing block. The branch is a nested block, so snapshot
                    // the register-type map and restore it after lowering — a
                    // binding made INSIDE the branch does not leak past it, while
                    // a binding made before the `if` (already in the map) stays
                    // visible within. Bindings emit no bytes, so this affects
                    // operand resolution only.
                    let saved_reg_types = self.reg_pointee_struct.clone();
                    for stmt in stmts {
                        self.lower_asm_stmt(stmt, scope, buf, env);
                    }
                    self.reg_pointee_struct = saved_reg_types;
                }
            }
            AsmStmt::Splice(expr) => {
                // 2026-07-11 mini-spec: `{expr}` inlines a Code value's items in
                // place — the SAME append path as `AsmStmt::Call`'s Code return,
                // one surface up (ANY expr, not just a call; the braces mark the
                // hole). An empty Code (`Code.empty()`) inlines nothing. A `Data`
                // value is a steering error (data belongs in `dc`/`bytes()`);
                // any other value is a type error naming Code. The `[prov.comptime]`
                // call-site note rides the same watermark as the Call arm.
                let watermark = self.diags.len();
                let v = self.eval_expr(expr, env);
                self.note_if_comptime_error(watermark, expr_span(expr));
                match v {
                    Value::Code(mut inner) => {
                        crate::value::reauthor_user_items(
                            &mut inner.items,
                            &crate::value::ItemAuthor::Splice {
                                template: splice_template_name(expr),
                            },
                        );
                        buf.items.extend(inner.items)
                    }
                    Value::Poison => {}
                    Value::Data(_) => self.error(
                        expr_span(expr),
                        "a `{expr}` splice must be Code — data belongs in `dc`/`bytes()`; \
                         a Data splice is unbuilt (ledger the demand if you hit this)"
                            .to_string(),
                    ),
                    other => self.error(
                        expr_span(expr),
                        format!(
                            "a `{{expr}}` splice must evaluate to Code, got {}",
                            other.type_name()
                        ),
                    ),
                }
            }
            // Diagnostics construct (spec §3, Task 3): desugar to the twin-parity
            // expansion, then lower it through THIS SAME statement loop — the
            // exact path a comptime-`if`'s chosen branch takes.
            AsmStmt::Assert { width, src, src_spelling, cond, dest, span } => {
                // Bundle the parsed pieces (the AST already pairs each operand
                // with its verbatim spelling) so they stay together through the
                // desugar.
                let parts = crate::eval::diag::AssertParts {
                    width: *width,
                    src: (**src).clone(),
                    src_spelling: src_spelling.clone(),
                    cond: cond.clone(),
                    dest: dest.as_ref().map(|(op, s)| ((**op).clone(), s.clone())),
                };
                self.lower_assert(&parts, *span, scope, buf, env);
            }
            AsmStmt::RaiseError { fstring, opts, span } => {
                self.lower_raise_error(fstring, *opts, /*from_exception=*/ false, *span, scope, buf, env);
            }
            AsmStmt::RaiseException { fstring, opts, span } => {
                self.lower_raise_error(fstring, *opts, /*from_exception=*/ true, *span, scope, buf, env);
            }
            AsmStmt::Invoke { iface, member, span } => {
                self.lower_invoke(iface, member, *span, scope, buf, env);
            }
            AsmStmt::With { ctx, cond, body, span } => {
                self.lower_with(ctx, cond.as_ref(), body, *span, scope, buf, env);
            }
        }
    }

    /// Lower a `with <ctx> [if cond] { … }` context bracket (§3.2): plant the
    /// `Enter` mark, splice the context's `acquire` Code, lower the body inline,
    /// plant `BodyEnd`, splice `release`, plant `Exit`.
    ///
    /// The bracket emits EXACTLY the bytes the manual `acquire ++ body ++
    /// release` spelling does — the marks are zero-byte
    /// ([`CodeItem::ContextMark`]) and the acquire/release exprs are evaluated
    /// here, at the use site, precisely as a hand-written call of the same
    /// comptime fn would be. That is what makes corpus adoption byte-neutral.
    ///
    /// A FALSE comptime gate lowers the body verbatim and plants nothing: the
    /// context is genuinely not held in that shape, so a region there would be a
    /// lie the checker would then "prove".
    #[allow(clippy::too_many_arguments)]
    fn lower_with(
        &mut self,
        ctx: &str,
        cond: Option<&ast::Expr>,
        body: &[AsmStmt],
        span: Span,
        scope: &LabelScope,
        buf: &mut CodeBuf,
        env: &mut Env,
    ) {
        // The comptime gate, if any — the same bool/int contract `AsmStmt::If`'s
        // condition has, and the same diagnostic class.
        if let Some(cond) = cond {
            // An UNEVALUABLE gate lowers the body BARE rather than dropping it.
            // Unlike an `if`'s branch, a bracket's body is real code in EVERY
            // shape — only the acquire/release are conditional — so dropping it
            // would silently shrink the analysis buffer of whatever proc holds
            // it (the whole VBlank pipeline sits inside one of these). The
            // diagnostic is already reported; this is the conservative recovery.
            let gated_on = match self.eval_expr(cond, env) {
                Value::Bool(b) => b,
                Value::Int(i) => i != 0,
                Value::Poison => {
                    self.lower_with_body(body, scope, buf, env);
                    return;
                }
                other => {
                    self.error(
                        expr_span(cond),
                        format!(
                            "[context.gate-not-comptime] a `with … if` gate must be a comptime \
                             bool or int, got {}",
                            other.type_name()
                        ),
                    );
                    self.lower_with_body(body, scope, buf, env);
                    return;
                }
            };
            if !gated_on {
                self.lower_with_body(body, scope, buf, env);
                return;
            }
        }
        let Some(decl) = self.contexts.get(ctx).copied() else {
            self.error(
                span,
                format!(
                    "[context.unknown] no context named `{ctx}` is in scope — declare \
                     `context {ctx} {{ … }}` or import it"
                ),
            );
            self.lower_with_body(body, scope, buf, env);
            return;
        };
        let ast::ContextKind::Acquired { acquire, release } = &decl.kind else {
            self.error(
                span,
                format!(
                    "[context.not-acquirable] `{ctx}` is a GRANTED context — it has no \
                     acquire/release pair to bracket with. A granted context is asserted at a \
                     root proc with `grants({ctx})`"
                ),
            );
            self.lower_with_body(body, scope, buf, env);
            return;
        };
        // Clone out of the borrow: evaluating the exprs needs `&mut self`.
        let decl_span = decl.span;
        let (acquire, release) = (acquire.clone(), release.clone());
        buf.push(CodeItem::ContextMark {
            ctx: ctx.to_string(),
            kind: crate::value::ContextMarkKind::Enter,
            span,
        });
        let acq_start = buf.items.len();
        let acq_ok = self.splice_context_code(&acquire, ctx, ContextPhase::Acquire, span, buf, env);
        let acq_end = buf.items.len();
        buf.push(CodeItem::ContextMark {
            ctx: ctx.to_string(),
            kind: crate::value::ContextMarkKind::AcquireEnd,
            span,
        });
        self.lower_with_body(body, scope, buf, env);
        buf.push(CodeItem::ContextMark {
            ctx: ctx.to_string(),
            kind: crate::value::ContextMarkKind::BodyEnd,
            span,
        });
        let rel_start = buf.items.len();
        let rel_ok = self.splice_context_code(&release, ctx, ContextPhase::Release, span, buf, env);
        let rel_end = buf.items.len();
        buf.push(CodeItem::ContextMark {
            ctx: ctx.to_string(),
            kind: crate::value::ContextMarkKind::Exit,
            span,
        });
        // THE CONTEXT'S OWN SR OBLIGATION, checked where the code lives. The
        // bracket's spliced SR traffic is `Context`-authored, so the consumer's
        // `[proc.sr-undeclared]` never charges it — which is sound only because
        // the round-trip proof runs HERE, against what was ACTUALLY SPLICED: a
        // context that masks and never restores fires at its own declaration.
        //
        // The check runs at EVERY bracket — never deduped — because the halves
        // evaluate per site in the consumer's env, so one context can splice a
        // round-tripping stream at one site and a non-round-tripping one at
        // another (a comptime fn whose param gates the halves); a checked-once
        // scheme would exempt the second site unproven. Only the REPORT is
        // deduped (`sr_reported_contexts`, one firing per context per
        // evaluator; cross-proc duplicates collapse at the collectors on the
        // (level, message, span) key — every adopter anchors the same message
        // at the same declaration span).
        //
        // SCOPE — why SR-only: a round-tripped SR is unobservable to the
        // consumer, so the bracket alone discharges it; a register the acquire
        // clobbered and the release did not restore IS observable and stays the
        // consumer's to declare. A context cannot declare its own net
        // perturbation today (`perturbs(...)` is demand-parked; ledgered).
        // ASSUMPTION, stated because nothing checks it: the round trip is a
        // caller-visible guarantee only if the body leaves the stack balanced
        // at the release, since the restore pops whatever is on top —
        // balanced-stack verification is S2-D7(b)'s dataflow job.
        //
        // A half that failed to splice as Code (Poison / wrong type) leaves a
        // truncated range: the check would stack a spurious "does not
        // round-trip" on the already-reported error, so that bracket is
        // skipped; a later bracket with clean halves is still checked.
        if acq_ok
            && rel_ok
            && !crate::lower::sr_writes_round_trip(
                buf.items[acq_start..acq_end].iter().chain(&buf.items[rel_start..rel_end]),
            )
            && self.sr_reported_contexts.insert(ctx.to_string())
        {
            self.warn(
                decl_span,
                format!(
                    "[proc.sr-undeclared] context `{ctx}`'s acquire and release do not \
                     round-trip `sr` — the spliced traffic leaves every consumer's \
                     interrupt mask changed; save SR in `acquire` and make the \
                     `move.w (sp)+, sr` restore the release's last SR write"
                ),
            );
        }
    }

    /// Lower a bracket's statements inline against the enclosing scope + buffer,
    /// with the C2 register-type map snapshotted exactly as an `if` branch does
    /// (the bracket is a nested BLOCK, so a `let` inside it does not leak past).
    fn lower_with_body(
        &mut self,
        body: &[AsmStmt],
        scope: &LabelScope,
        buf: &mut CodeBuf,
        env: &mut Env,
    ) {
        let saved_reg_types = self.reg_pointee_struct.clone();
        for stmt in body {
            self.lower_asm_stmt(stmt, scope, buf, env);
        }
        self.reg_pointee_struct = saved_reg_types;
    }

    /// Evaluate one half of a context's bracket (`acquire`/`release`) and splice
    /// its items, authored as the CONTEXT's ([`crate::value::ItemAuthor::Context`]
    /// — the bracket is the consumer's declaration, so the spliced lines are the
    /// context's to answer for). It MUST be Code — the same contract a
    /// statement-position call carries — and an empty Code splices nothing (a
    /// legal, if pointless, context). Returns `false` when the half did NOT
    /// splice as Code (Poison / wrong type): the caller's definition-site
    /// round-trip check reads truncated ranges in that case and would stack a
    /// spurious "does not round-trip" on the already-reported error.
    fn splice_context_code(
        &mut self,
        expr: &ast::Expr,
        ctx: &str,
        phase: ContextPhase,
        span: Span,
        buf: &mut CodeBuf,
        env: &mut Env,
    ) -> bool {
        let half = match phase {
            ContextPhase::Acquire => "acquire",
            ContextPhase::Release => "release",
        };
        let watermark = self.diags.len();
        let v = self.eval_expr(expr, env);
        self.note_if_comptime_error(watermark, span);
        match v {
            Value::Code(mut inner) => {
                crate::value::reauthor_user_items(
                    &mut inner.items,
                    &crate::value::ItemAuthor::Context { name: ctx.to_string(), phase },
                );
                buf.items.extend(inner.items);
                return true;
            }
            Value::Poison => {}
            other => self.error(
                expr_span(expr),
                format!(
                    "[context.not-code] `context {ctx}`'s `{half}` must evaluate to Code, got {}",
                    other.type_name()
                ),
            ),
        }
        false
    }

    /// Lower `invoke Iface.hook` (L1). Resolves the hook binding from the seeded
    /// interface environment: a BOUND hook lowers to an absolute `jsr (sym).l`
    /// (placement-independent by rule — an engine→game call), synthesized and
    /// routed through the same statement path a hand-written `jsr` takes; an
    /// `empty`/unbound hook emits NOTHING. A member that is not a hook is a
    /// member-kind error; an unknown interface/member is `[contract.unknown-member]`
    /// (the bind pass is the sole site for unimplemented/missing/signature errors).
    fn lower_invoke(
        &mut self,
        iface: &str,
        member: &str,
        span: Span,
        scope: &LabelScope,
        buf: &mut CodeBuf,
        env: &mut Env,
    ) {
        match self.interfaces.get(iface).and_then(|i| i.members.get(member)) {
            Some(crate::contract::ResolvedMember::Hook(Some(sym))) => {
                // `jsr (sym).l` — an absolute-long indirect naming the bound proc
                // (the `abs_l` operand shape). Synthesized and lowered recursively
                // through `lower_asm_stmt`, exactly as the diagnostics desugars do.
                let sym = sym.clone();
                let jsr = AsmStmt::Instr(InstrLine {
                    mnemonic: vec![TextOrSplice::Text("jsr".into())],
                    size: None,
                    operands: vec![Operand::Ind {
                        parts: vec![(
                            ast::Expr::Path(ast::Path { segments: vec![sym], span }),
                            None,
                        )],
                        size: Some(TextOrSplice::Text("l".into())),
                        span,
                    }],
                    span,
                    dispatch_bound: None,
                    discards: None,
                });
                self.lower_asm_stmt(&jsr, scope, buf, env);
            }
            // `empty`/unbound hook: zero bytes, by rule (the load-bearing case).
            Some(crate::contract::ResolvedMember::Hook(None)) => {}
            Some(_) => self.error(
                span,
                format!(
                    "[contract.member-kind] `{iface}.{member}` is not a hook — only a \
                     `hook` member may be `invoke`d"
                ),
            ),
            None => self.error(
                span,
                format!(
                    "[contract.unknown-member] `{iface}` has no hook member `{member}` \
                     (or the interface is unimplemented in this build)"
                ),
            ),
        }
    }

    /// The diag-label module scope: `"$<module_id>"` so separately-lowered
    /// modules never mint colliding `$diag{n}$…` symbols at link; empty in the
    /// no-file mode (unit-test expansions keep their historical names).
    fn diag_label_scope(&self) -> String {
        if self.module_id.is_empty() { String::new() } else { format!("${}", self.module_id) }
    }

    /// The 16 Bcc condition codes an `assert` accepts (spec §3/§5), lowercase.
    /// The `b<cond>.w` branch mnemonic is formed directly from the spelling, so
    /// this is the ONE membership gate (an unknown cond is caught at parse time,
    /// but re-validated here so the desugar never forms a bogus mnemonic).
    const ASSERT_CONDS: &'static [&'static str] = &[
        "eq", "ne", "cs", "cc", "pl", "mi", "hi", "hs", "ls", "lo", "gt", "ge", "le", "lt", "vs",
        "vc",
    ];

    /// Lower an `assert` (spec §4.2): read `DEBUG` from the comptime env exactly
    /// as the `If` arm reads its condition (undefined → the spec-§5 "shapes are
    /// explicit" error). `DEBUG == 1` → desugar the 11-step expansion and lower
    /// it RECURSIVELY through `lower_asm_stmt` (the same path the `If` arm's
    /// chosen branch takes); anything else → emit nothing. `src` must be a
    /// register (§5) — else the "move to a register first" steering error.
    fn lower_assert(
        &mut self,
        p: &crate::eval::diag::AssertParts,
        span: Span,
        scope: &LabelScope,
        buf: &mut CodeBuf,
        env: &mut Env,
    ) {
        // §5: cond membership (re-validated so the desugar can't form `b<xx>.w`).
        if !Self::ASSERT_CONDS.contains(&p.cond.as_str()) {
            self.error(
                span,
                format!(
                    "unknown assert condition `{}` — expected one of {}",
                    p.cond,
                    Self::ASSERT_CONDS.join(", ")
                ),
            );
            return;
        }
        // §5: src must be a register (dn/an) — the debugger.asm limitation
        // (a parenthesised memory operand assembles to AS error #1300; the
        // rings.emp comment names this precedent).
        if !operand_is_register(&p.src) {
            self.error(
                span,
                format!(
                    "assert `src` must be a register (dn/an) in v1, got `{}` — \
                     move the value to a register first (debugger.asm's message expansion \
                     cannot take a parenthesised memory operand; matches the rings.emp \
                     precedent / AS error #1300)",
                    p.src_spelling
                ),
            );
            return;
        }
        // Gate on DEBUG exactly like the `If` arm — but a MISSING `DEBUG` is the
        // spec-§5 explicit-shape error (an ordinary `if DEBUG == 1` would just
        // say "unknown name"; assert's contract is that the shape is explicit).
        match self.debug_gate(span, env) {
            Some(true) => {}
            Some(false) => return, // DEBUG != 1: emit ZERO bytes (§4.1).
            None => return,        // undefined: error already emitted.
        }

        let n = self.asm_counter;
        self.asm_counter += 1;
        let diag_scope = self.diag_label_scope();
        let stmts = crate::eval::diag::build_assert_expansion(n, &diag_scope, p, span);
        // Every expansion instruction is the DESUGAR's, not the asserting
        // proc's author's — recorded at emission so an effect lint charges the
        // compiler's own surface (the emission-site balance pin) instead of the
        // containing proc. Restored after: the expansion cannot nest an assert,
        // but the save/restore keeps the stamp scoped by construction.
        let saved_author = std::mem::replace(
            &mut self.item_author,
            crate::value::ItemAuthor::AssertDesugar,
        );
        for stmt in &stmts {
            self.lower_asm_stmt(stmt, scope, buf, env);
        }
        self.item_author = saved_author;
    }

    /// Lower a `raise_error` (spec §4.3) or `raise_exception` (t25): NO DEBUG
    /// gate, NO cmp/branch/CCR wrapper — just the raise tail with the user's
    /// fstring. `from_exception` selects the frame-omit (`raise_exception` = the
    /// CPU-vectored `__ErrorMessage` shape, no `pea`/`move.w sr`); `opts` folds
    /// the options-form flag bits into the exit-flag byte. Arg pushes are
    /// generated in REVERSE token order (matching `__FSTRING_GenerateArgumentsCode`);
    /// each arg operand is limited to a register or immediate (§5), else a
    /// steering error.
    #[allow(clippy::too_many_arguments)]
    fn lower_raise_error(
        &mut self,
        fstring: &str,
        opts: u8,
        from_exception: bool,
        span: Span,
        scope: &LabelScope,
        buf: &mut CodeBuf,
        env: &mut Env,
    ) {
        let encoded = match crate::eval::diag::encode_fstring(fstring) {
            Ok(e) => e,
            Err(msg) => {
                self.error(span, msg);
                return;
            }
        };
        // Arg pushes in REVERSE token order (§4.3): the last `%<...>` operand is
        // pushed first, so the handler pops them in string order.
        let mut arg_pushes = Vec::new();
        for arg in encoded.args.iter().rev() {
            let Some(operand) = crate::eval::diag::fstring_arg_operand(&arg.operand_spelling, span)
            else {
                let kw = if from_exception { "raise_exception" } else { "raise_error" };
                self.error(
                    span,
                    format!(
                        "{kw} argument `{}` must be a register or immediate in v1 \
                         (§5) — a memory/EA operand arg is a recorded extension",
                        arg.operand_spelling
                    ),
                );
                return;
            };
            arg_pushes.extend(crate::eval::diag::arg_push(arg.width, operand, span));
        }
        let stmts = if from_exception {
            // The exception form mints no self-label (no `pea self(pc)`), so it
            // needs no instantiation id.
            crate::eval::diag::build_raise_exception_expansion(&encoded.bytes, arg_pushes, opts, span)
        } else {
            let n = self.asm_counter;
            self.asm_counter += 1;
            let diag_scope = self.diag_label_scope();
            crate::eval::diag::build_raise_error_expansion(
                n,
                &diag_scope,
                &encoded.bytes,
                arg_pushes,
                opts,
                span,
            )
        };
        for stmt in &stmts {
            self.lower_asm_stmt(stmt, scope, buf, env);
        }
    }

    /// Read `DEBUG` from the comptime scope for the `assert` gate. Returns
    /// `Some(truthy)` when it resolves to an int/bool, and `None` (after
    /// emitting the spec-§5 "shapes are explicit" error) when it is undefined.
    /// A `DEBUG` that resolves to a non-int/bool value is also `None` with a
    /// steering error. Mirrors the `If` arm's truthiness read (int != 0, bool).
    fn debug_gate(&mut self, span: Span, env: &mut Env) -> Option<bool> {
        if !(self.defines.contains_key("DEBUG")
            || self.consts.contains_key("DEBUG")
            || self.equs.contains_key("DEBUG"))
        {
            self.error(
                span,
                "`assert` requires `DEBUG` to be defined (house convention: the debug \
                 shape is explicit) — define it (`-D DEBUG=1`, a `const DEBUG`, or an \
                 `equ`) so the gate is unambiguous",
            );
            return None;
        }
        let debug_path = ast::Expr::Path(ast::Path { segments: vec!["DEBUG".into()], span });
        match self.eval_expr(&debug_path, env) {
            Value::Int(i) => Some(i != 0),
            Value::Bool(b) => Some(b),
            Value::Poison => None,
            other => {
                self.error(
                    span,
                    format!(
                        "`DEBUG` must be a comptime int or bool for the assert gate, got {}",
                        other.type_name()
                    ),
                );
                None
            }
        }
    }

    /// Emit the `[prov.comptime]` call-site note if a comptime GENERATOR call
    /// produced any new ERROR past `watermark` (§9, D-P4.11 — the smallest honest
    /// `ProvFrame::Comptime`; see the `AsmStmt::Call` arm's comment for why a
    /// structured provenance frame cannot attach to the fragment yet). Shared by
    /// every statement-position call spelling (paren `AsmStmt::Call`, bare
    /// D-PP.1), so the note's wording and its errors-only trigger stay in ONE
    /// place. Only an ERROR warrants the note (the message speaks of an error);
    /// a stray warning would not.
    ///
    /// D-PP.4 (named call arguments) does NOT add a third spelling here: named
    /// args are paren-form only (see `operand_to_arg`'s doc comment below for
    /// why the bare spelling's operand grammar cannot represent `name: expr`
    /// without ambiguity) — `bind_args` in `eval/call.rs` is the one binder both
    /// existing spellings already share.
    fn note_if_comptime_error(&mut self, watermark: usize, call_site: Span) {
        let new_error = self.diags[watermark..].iter().any(|d| d.level == Level::Error);
        if new_error {
            self.note(
                call_site,
                "[prov.comptime] error is inside a table generated by this comptime call"
                    .to_string(),
            );
        }
    }

    /// Try to interpret `instr` as a bare directive-style statement call
    /// (D-PP.1). Returns:
    ///  - `Some(code)` — the line WAS a bare call: `code` is the instantiated
    ///    template (possibly empty on a non-Code / arg error, so the caller does
    ///    NOT also emit the "unrecognized mnemonic" error).
    ///  - `None` — the line is NOT a bare call (a real instruction, or a bareword
    ///    resolving to nothing): the caller lowers it as an instruction unchanged,
    ///    preserving today's diagnostics EXACTLY.
    ///
    /// A candidate is a leading single plain-text mnemonic with NO size suffix
    /// (`set_timer`, `nop_twice`) whose word the section's mnemonic table rejects.
    /// Mnemonics win unconditionally, so a size-suffixed or spliced mnemonic, or
    /// any recognized mnemonic, is never a bare call. The CPU must be known (a
    /// proc body, not a raw `asm {}` template) for the mnemonic decision.
    fn try_bare_statement_call(&mut self, instr: &InstrLine, env: &mut Env) -> Option<CodeBuf> {
        let cpu = self.cpu?;
        // Only a bare, single, literal mnemonic word with no size is a candidate.
        if instr.size.is_some() || instr.mnemonic.len() != 1 {
            return None;
        }
        let TextOrSplice::Text(name) = &instr.mnemonic[0] else { return None };
        // Mnemonics win unconditionally (tenet 3).
        if crate::lower::is_recognized_mnemonic(name, cpu) {
            return None;
        }
        // Not a mnemonic. What does the bareword resolve to?
        if self.fns.contains_key(name.as_str()) {
            // An in-scope comptime fn → a statement call. Convert each operand back
            // to a call-argument expression (the parser saw the line as an
            // instruction; reverse the operand normalization), then dispatch
            // through the SAME call machinery the paren form uses.
            let mut args = Vec::with_capacity(instr.operands.len());
            for op in &instr.operands {
                match operand_to_arg(op) {
                    Some(a) => args.push(a),
                    None => {
                        self.error(
                            operand_span(op),
                            "a bare statement-call argument must be a comptime expression \
                             (an addressing-mode operand is not a valid argument)",
                        );
                        return Some(CodeBuf::empty());
                    }
                }
            }
            let callee = ast::Path { segments: vec![name.clone()], span: instr.span };
            let watermark = self.diags.len();
            let v = self.eval_call(&callee, &args, instr.span, env);
            self.note_if_comptime_error(watermark, instr.span);
            match v {
                Value::Code(mut inner) => {
                    // The splice boundary claims the template's own lines
                    // ([`crate::value::ItemAuthor::Splice`]); a finer
                    // compiler-authored fact inside the template survives.
                    crate::value::reauthor_user_items(
                        &mut inner.items,
                        &crate::value::ItemAuthor::Splice { template: name.clone() },
                    );
                    return Some(inner);
                }
                Value::Poison => return Some(CodeBuf::empty()),
                other => {
                    self.error(
                        instr.span,
                        format!(
                            "a bare statement call must evaluate to Code, got {} — only a \
                             comptime fn returning Code is legal in statement position",
                            other.type_name()
                        ),
                    );
                    return Some(CodeBuf::empty());
                }
            }
        }
        // A non-fn comptime value (const/enum/struct/newtype name) in statement
        // position: a specific error naming what it is. Registers are NOT checked
        // here — a bare register word in statement position is not a call and
        // stays the mnemonic error (it names no fn).
        if let Some(kind) = self.bareword_non_fn_kind(name) {
            self.error(
                instr.span,
                format!(
                    "`{name}` names {kind}, not a comptime fn — only comptime fn calls are legal \
                     in statement position"
                ),
            );
            return Some(CodeBuf::empty());
        }
        // Resolves to nothing → NOT a bare call. Fall through to the unchanged
        // "not a recognized mnemonic" error.
        None
    }

    /// Classify a statement-position bareword that is NOT a comptime fn but IS a
    /// known non-fn comptime construct, for the specific "`X` names <kind>" error
    /// (D-PP.1). Returns `None` for a name that resolves to nothing (which keeps
    /// today's mnemonic error). Each kind carries its own article ("a const",
    /// "an enum") so the caller's message stays grammatical without an
    /// article-selection branch at the format site.
    fn bareword_non_fn_kind(&self, name: &str) -> Option<&'static str> {
        if self.consts.contains_key(name) {
            Some("a const")
        } else if self.structs.contains_key(name) {
            Some("a struct")
        } else if self.enums.contains_key(name) {
            Some("an enum")
        } else if self.newtypes.contains_key(name) {
            Some("a newtype")
        } else if self.bitfields.contains_key(name) {
            Some("a bitfield")
        } else {
            None
        }
    }

    /// Lower one [`InstrLine`] to a [`CodeItem::Instr`]: resolve the mnemonic and
    /// size (splices typed against [`Width`]/[`Cc`]) and map every operand. Any
    /// failure emits a diagnostic and yields `None` (the line is dropped).
    ///
    /// `movem` is special-cased (D-P1H.2): reglist parsing is MNEMONIC-DIRECTED,
    /// not a general operand-grammar form, so only here — once the resolved
    /// mnemonic string is in hand — do we try each operand as a register list
    /// before falling back to the ordinary operand mapper. This mirrors the AS
    /// front-end's `lower_m68k_movem` shape (parse both operands as reglists,
    /// exactly one must succeed) without leaking `d0-d1/a0` grammar anywhere else.
    fn lower_instr_to_item(
        &mut self,
        instr: &InstrLine,
        scope: &LabelScope,
        env: &mut Env,
    ) -> Option<CodeItem> {
        let mnemonic = self.resolve_mnemonic(&instr.mnemonic, env)?;
        let size = self.resolve_size(instr.size.as_ref(), instr.span, env)?;
        if mnemonic == "dc" {
            return self.lower_dc(instr, size, env);
        }
        if mnemonic == "movem" {
            let ops = self.map_movem_operands(instr, scope, env, size)?;
            return Some(CodeItem::Instr {
                mnemonic,
                size,
                ops,
                span: instr.span,
                as_type: instr.dispatch_bound.clone(),
                author: self.item_author.clone(),
            });
        }
        // `movep` transfers ALTERNATE bytes (d, d+2[, d+4, d+6]) — each touched
        // byte is byte-sized, and the bytes past the named field land on OTHER
        // fields BY DESIGN (the interleaved DMA-queue-entry layout is the whole
        // point of the instruction). Mapping its operands with the instruction's
        // `.w`/`.l` width would mis-fire the field-overrun lint on a 1-byte
        // field (`movep.w Ent.SizeH(a0), d1`), so the EFFECTIVE access width for
        // operand mapping is `.b`; the emitted CodeItem keeps the true size.
        let op_width = if mnemonic == "movep" { Some(Width::B) } else { size };
        // Z80 condition codes (rung-2 §13.3): under a control-flow mnemonic
        // (`jr`/`jp`/`call`/`ret`), a bare condition word in the FIRST operand
        // position is a `Z80Cc`, not an ordinary operand. This is the AS
        // front-end's `control_flow && i == 0` rule — the only place `c` (the
        // carry cc) is disambiguated from `c` (the C register): under jr/jp/
        // call/ret, position 0 is a cc. Every other position, and every other
        // mnemonic, reads `c` as the register (the `map_operand` path below).
        let z80_control_flow =
            self.cpu == Some(sigil_ir::backend::Cpu::Z80) && is_z80_control_flow(&mnemonic);
        let mut ops = Vec::with_capacity(instr.operands.len());
        for (i, op) in instr.operands.iter().enumerate() {
            if z80_control_flow && i == 0 {
                if let Some(cc) = z80_cc_operand(op) {
                    ops.push(CodeOperand::Z80Cc(cc));
                    continue;
                }
            }
            ops.push(self.map_operand(op, scope, env, op_width)?);
        }
        Some(CodeItem::Instr {
            mnemonic,
            size,
            ops,
            span: instr.span,
            as_type: instr.dispatch_bound.clone(),
            author: self.item_author.clone(),
        })
    }

    /// `dc.b`/`dc.w`/`dc.l` — code-embedded constant data (tranche 8, the
    /// rings-port H8 demand: an error handler's format-string bytes sit
    /// MID-PROC between a `jsr` and its resume label; no item-position
    /// construct can express them). Elements must be COMPTIME-KNOWN: ints
    /// (range-checked to the width's signed∪unsigned window, loud on
    /// overflow — never silent truncation) or, for `dc.b` only, raw-ASCII
    /// string literals (D2.16 — no implicit terminator). Link-expr cells in
    /// `dc` position are a recorded extension (ledger), not built until a
    /// real consumer demands them; typed `data` items remain the story for
    /// item-position data. Produces a [`CodeItem::Inline`] — scalar cells
    /// serialize in the section CPU's byte order at lowering (68k BE,
    /// Z80 LE), so the statement is CPU-neutral like the DataBuf it builds.
    fn lower_dc(
        &mut self,
        instr: &InstrLine,
        size: Option<Width>,
        env: &mut Env,
    ) -> Option<CodeItem> {
        use crate::value::{Cell, DataBuf};
        let Some(width) = size else {
            self.error(instr.span, "[dc.missing-size] `dc` needs an explicit width — `dc.b`, `dc.w`, or `dc.l`");
            return None;
        };
        let width_bytes: usize = match width {
            Width::B => 1,
            Width::W => 2,
            Width::L => 4,
            Width::S => {
                self.error(instr.span, "[dc.missing-size] `.s` is a branch width — `dc` takes `.b`, `.w`, or `.l`");
                return None;
            }
        };
        if instr.operands.is_empty() {
            self.error(instr.span, "[dc.empty] `dc` needs at least one element");
            return None;
        }
        let mut cells = Vec::with_capacity(instr.operands.len());
        let mut total = 0usize;
        for op in &instr.operands {
            let expr = match op {
                Operand::Plain { expr, size: None, .. } => expr,
                Operand::Splice(e) => e,
                _ => {
                    self.error(
                        instr.span,
                        "[dc.operand] `dc` elements are plain comptime expressions — addressing-mode operands have no meaning here",
                    );
                    return None;
                }
            };
            // Self-relative offset word: `dc.w .target - .base` where BOTH sides
            // are proc-LOCAL labels — the AS inline jump-table idiom. Extends the
            // symbol-difference semantic already shipped for `offsets`/`dispatch`
            // and the `extern("X") - extern("Y")` data form (all `Cell::RelOffset`)
            // to local labels in `dc` position. SCOPE (each an Option-A constraint,
            // t38): SUB of TWO LOCALS only — `label + label`/`label + int`/
            // `int - label`/a GLOBAL operand fall through to the general eval below,
            // which keeps the loud label-arithmetic error (a global difference is a
            // potential cross-section fold — not this form; `offsets`/`dispatch` own
            // the cross-module table). LOCAL-only is a structural same-section
            // guarantee (proc-locals share the enclosing proc's section). `dc.w`
            // only — a self-rel offset is a signed word, so `dc.b`/`dc.l` reject.
            if let ast::Expr::Binary { op: BinOp::Sub, lhs, rhs, .. } = expr {
                if let (Some(target), Some(base)) =
                    (self.dc_self_rel_local(lhs, env), self.dc_self_rel_local(rhs, env))
                {
                    if width != Width::W {
                        self.error(
                            expr_span(expr),
                            "[dc.self-rel-width] a `<local> - <local>` self-relative offset is a signed word — `dc.w`-only",
                        );
                        return None;
                    }
                    cells.push(Cell::RelOffset { base, target });
                    total += 2;
                    continue;
                }
            }
            // Evaluate the element in LABEL context so a bareword/dotted proc or
            // data name becomes a first-class `Value::Label` (a deferred link
            // symbol) rather than an `unknown name` error — the same rule a
            // pointer FIELD uses (`lower_ptr`, D-PP.3). This makes `dc.l <label>`
            // (an absolute symbol pointer, e.g. a jump/vector table entry) work;
            // existing name resolution (local → const → fn) still WINS, so a
            // const named the same as a label keeps its comptime value.
            match self.in_label_ctx(|this| this.eval_expr(expr, env)) {
                Value::Int(n) => {
                    let bits = (width_bytes * 8) as u32;
                    let lo = -(1i128 << (bits - 1));
                    let hi = (1i128 << bits) - 1;
                    if n < lo || n > hi {
                        self.error(
                            expr_span(expr),
                            format!("[dc.range] {n} does not fit a {width_bytes}-byte `dc` element (allowed {lo}..={hi})"),
                        );
                        return None;
                    }
                    cells.push(Cell::Scalar {
                        value: n,
                        width: width_bytes as u8,
                        signed: n < 0,
                        le: false,
                    });
                    total += width_bytes;
                }
                // A link-resolved symbol pointer: `dc.l Label` (or `.w`) emits a
                // `Cell::SymRef` of the element width, resolved at link — the
                // absolute-pointer form a vector/jump table needs. Mirrors
                // `lower_ptr`'s `Cell::SymRef` (a `*u8` field), here in raw `dc`
                // position rather than a typed `data` field. A REGISTER name is
                // not a valid `dc` element (a register has no data reading) —
                // keep it the loud `unknown name` error the corpus pins, not a
                // silent SymRef.
                Value::Label(name)
                    if reg_from_name(&name).is_some()
                        || matches!(name.as_str(), "sp" | "pc" | "sr" | "ccr") =>
                {
                    // A register-class word is not a valid `dc` element — keep it
                    // the loud `unknown name` error (consistent with
                    // `bare_symbol_seg`'s sr/ccr exclusion), not a silent SymRef.
                    self.error(expr_span(expr), format!("unknown name `{name}`"));
                    return None;
                }
                Value::Label(name) => {
                    cells.push(Cell::SymRef { name, width: width_bytes as u8, windowed: false });
                    total += width_bytes;
                }
                Value::Str(s) if width_bytes == 1 => {
                    total += s.len();
                    cells.push(Cell::Bytes(s.into_bytes()));
                }
                Value::Str(_) => {
                    self.error(
                        expr_span(expr),
                        "[dc.string-width] string elements are `dc.b`-only (a string is a run of bytes; it has no word/long reading)",
                    );
                    return None;
                }
                // A link-time expression value: a `dc` element naming an equ whose
                // value is a residual link tree (e.g. `extern("L") + off`). A bare
                // symbol keeps the `Cell::SymRef` address lowering; an arithmetic
                // tree emits a general link-expr VALUE cell (`Cell::Expr`) — the
                // same machinery the typed-data emit path (`lower_link_expr`) and
                // the `offsets`/`dispatch` constructs use. The linker folds it
                // post-placement (equ-off-link-external-base on the `.emp` side).
                Value::LinkExpr(e) => {
                    match e {
                        sigil_ir::expr::Expr::Sym(name) => {
                            cells.push(Cell::SymRef { name, width: width_bytes as u8, windowed: false });
                        }
                        other => {
                            cells.push(Cell::Expr { expr: other, width: width_bytes as u8, le: false });
                        }
                    }
                    total += width_bytes;
                }
                Value::Poison => return None,
                other => {
                    self.error(
                        expr_span(expr),
                        format!(
                            "[dc.comptime-only] `dc` elements must be comptime ints, strings, or link symbols, got {}",
                            other.type_name()
                        ),
                    );
                    return None;
                }
            }
        }
        Some(CodeItem::Inline(DataBuf { cells, size: total }, instr.span))
    }

    /// A `dc.w` self-relative-offset operand side: the mangled symbol name IF
    /// `e` is a proc-LOCAL label reference (a dotted `.name`), else `None`. A
    /// leading `.name` operand is an `Expr::Path[".name"]`; an interior `.name`
    /// is an `Expr::LocalLabel` — accept BOTH. Evaluates in label context so the
    /// name is owner-mangled and tracked for the end-of-body loudness check,
    /// exactly as a normal `.name` reference. A GLOBAL/imported label (bareword,
    /// no leading `.`) returns `None`: it is not this same-section-only form and
    /// falls through to the general path's loud label-arithmetic error.
    fn dc_self_rel_local(&mut self, e: &ast::Expr, env: &mut Env) -> Option<String> {
        let is_local = match e {
            ast::Expr::LocalLabel(..) => true,
            ast::Expr::Path(p) => {
                p.segments.len() == 1 && p.segments[0].starts_with('.')
            }
            _ => false,
        };
        if !is_local {
            return None;
        }
        match self.in_label_ctx(|this| this.eval_expr(e, env)) {
            Value::Label(name) => Some(name),
            _ => None,
        }
    }

    /// Map a `movem`'s two operands (D-P1H.2): exactly one must be a register
    /// list (`d0-d7/a0-a6`, a single reg, a `d`→`a`-crossing range, `sp` as an
    /// `a7` alias); the other is the ordinary memory-EA operand mapper. Operand
    /// ORDER is preserved (it selects STORE vs LOAD direction downstream in
    /// `lower/code.rs`'s `lower_m68k_movem`). Mirrors the AS front-end's
    /// `lower_m68k_movem`: try both as reglists, exactly one hit is legal.
    fn map_movem_operands(
        &mut self,
        instr: &InstrLine,
        scope: &LabelScope,
        env: &mut Env,
        size: Option<Width>,
    ) -> Option<Vec<CodeOperand>> {
        let [op0, op1] = instr.operands.as_slice() else {
            self.error(
                instr.span,
                "movem needs two operands: a register list and a memory EA",
            );
            return None;
        };
        let list0 = movem_reg_list(op0);
        let list1 = movem_reg_list(op1);
        match (list0, list1) {
            (Some(mask), None) => {
                let mem = self.map_operand(op1, scope, env, size)?;
                Some(vec![CodeOperand::RegList(mask), mem])
            }
            (None, Some(mask)) => {
                let mem = self.map_operand(op0, scope, env, size)?;
                Some(vec![mem, CodeOperand::RegList(mask)])
            }
            (Some(_), Some(_)) => {
                self.error(
                    instr.span,
                    "movem needs a memory EA operand, got two register lists",
                );
                None
            }
            (None, None) => {
                self.error(
                    instr.span,
                    "movem needs a register-list operand (e.g. `d0-d7/a0-a6`)",
                );
                None
            }
        }
    }

    /// Resolve a possibly-spliced mnemonic to its final string. A `{splice}` in
    /// the mnemonic must be a [`Width`] (`cmp.{w}`-style, spliced as its
    /// `Display`) or a [`Cc`] (`b{cc}` → `"bne"`); any other kind is
    /// `[asm.splice-kind]`. Returns `None` on a poison/mistyped splice.
    fn resolve_mnemonic(&mut self, parts: &[TextOrSplice], env: &mut Env) -> Option<String> {
        let mut out = String::new();
        for part in parts {
            match part {
                TextOrSplice::Text(t) => out.push_str(t),
                TextOrSplice::Splice(e) => {
                    let v = self.eval_expr(e, env);
                    match v {
                        Value::Width(w) => out.push_str(&w.to_string()),
                        Value::Cc(c) => out.push_str(&c.to_string()),
                        Value::Poison => return None,
                        other => {
                            self.splice_kind_err(expr_span(e), "Width or Cc", &other);
                            return None;
                        }
                    }
                }
            }
        }
        Some(out)
    }

    /// Resolve an optional size suffix to an `Option<Width>`. The outer `Option`
    /// distinguishes error (`None`) from "no size / a resolved size"
    /// (`Some(Option<Width>)`). A literal `b`/`w`/`l`/`s` maps directly; a
    /// `{splice}` must evaluate to a [`Width`] (`[asm.splice-kind]` otherwise).
    fn resolve_size(
        &mut self,
        size: Option<&TextOrSplice>,
        span: Span,
        env: &mut Env,
    ) -> Option<Option<Width>> {
        match size {
            None => Some(None),
            Some(TextOrSplice::Text(t)) => match width_from_text(t) {
                Some(w) => Some(Some(w)),
                None => {
                    self.error(span, format!("unknown size suffix `.{t}`"));
                    None
                }
            },
            Some(TextOrSplice::Splice(e)) => {
                let v = self.eval_expr(e, env);
                match v {
                    Value::Width(w) => Some(Some(w)),
                    Value::Poison => None,
                    other => {
                        self.splice_kind_err(expr_span(e), "Width", &other);
                        None
                    }
                }
            }
        }
    }

    /// Map one parsed [`ast::Operand`] to a resolved [`CodeOperand`]. Register and
    /// `.local`/symbol references resolve directly; an `#imm` / displacement
    /// evaluates its expr to an integer; a `{splice}` is typed against the operand
    /// classes (int / [`Reg`] / label). Returns `None` on any diagnosed failure.
    fn map_operand(
        &mut self,
        op: &Operand,
        scope: &LabelScope,
        env: &mut Env,
        width: Option<Width>,
    ) -> Option<CodeOperand> {
        // T1 (§3): under a Z80 section the operand grammar reads through a
        // CPU-aware mapper — a bare register word is a Z80 register, `(hl)` /
        // `(ix+d)` are Z80 indirects, a bare int is a Z80 immediate. The parser
        // stays CPU-neutral (`ast.rs`); only this classification differs. The 68k
        // path below is UNTOUCHED (byte-frozen). `width` steers the 68k
        // field-overrun lint only, so Z80 mapping ignores it.
        if self.cpu == Some(sigil_ir::backend::Cpu::Z80) {
            return self.map_operand_z80(op, scope, env);
        }
        match op {
            Operand::Imm(e) => {
                // C1 item 1: an immediate is the third and last deferral
                // position — an unresolved bareword (`#TestSolid_Main`) becomes
                // a deferred link symbol, and `label ± const` / `label − label`
                // fold into a link-time expr (the objroutine store shape). The
                // `in_imm_link_ctx` scope enables both the bareword→`Label`
                // fallback AND label arithmetic; a comptime bareword outside an
                // immediate keeps its loud `unknown name` (the totality fence).
                let v = self.in_imm_link_ctx(|this| this.eval_expr(e, env));
                if matches!(v, Value::Poison) {
                    return None;
                }
                // A bare single label (`#TestSolid_Main`) — the D-PP.3 label
                // value in immediate position — defers to the SAME imm fixup a
                // string/extern would (byte-identical to `#extern("…")`). Width
                // routing (`.w` → `ImmWord16Be`, `.l` → `Value32Be`) is policed
                // at lowering, where the resolved size is known.
                if let Value::Label(n) = &v {
                    return Some(CodeOperand::ImmLink {
                        target: sigil_ir::expr::Expr::Sym(n.clone()),
                    });
                }
                // A link-time immediate (`#extern(...)` / an equ-aliased
                // extern sum / label arithmetic) DEFERS to a `Value32Be`
                // imm32 fixup (tranche 5 — the emp mirror of the AS side's
                // `try_defer_long_imm`); the `.l`-only width policing lives
                // at lowering, where the resolved size is known. A
                // `bankid()`-derived value keeps its provisional rejection
                // (R7m.3 — the 9-bit-latch semantics need their own ruling
                // before they ride an instruction immediate).
                if let Value::LinkExpr(expr) = &v {
                    if !crate::eval::expr::expr_carries_bank_mask(expr) {
                        return Some(CodeOperand::ImmLink { target: expr.clone() });
                    }
                    self.reject_if_provisional(&v, expr_span(e));
                    return None;
                }
                // An `extern NAME: Type` (L8) reference is a link-deferred value
                // WRAPPED in its newtype — `Typed { val: Label | LinkExpr, .. }`.
                // The type is carried for enforcement in typed positions; in an
                // immediate it erases to the SAME link fixup a bare `Label` /
                // `extern(..)` takes, so `#SFXID_RING_RIGHT` encodes byte-identically
                // to the untyped link name it replaces.
                if let Value::Typed { val, .. } = &v {
                    match val.as_ref() {
                        Value::Label(n) => {
                            return Some(CodeOperand::ImmLink {
                                target: sigil_ir::expr::Expr::Sym(n.clone()),
                            });
                        }
                        Value::LinkExpr(expr) if !crate::eval::expr::expr_carries_bank_mask(expr) => {
                            return Some(CodeOperand::ImmLink { target: expr.clone() });
                        }
                        _ => {}
                    }
                }
                match v.as_stored_int() {
                    Some(n) => Some(CodeOperand::Imm(n)),
                    None => {
                        self.error(
                            expr_span(e),
                            format!("immediate must be an integer, got {}", v.type_name()),
                        );
                        None
                    }
                }
            }
            Operand::Plain { expr, .. } => self.map_plain(expr, scope, env),
            Operand::Ind { parts, size, span } => {
                // Two-part `(An,Xn[.size])` — An-indexed with zero displacement
                // (68k `(d8,An,Xn)`, d=0). Checked before the single-register
                // path so the two-part form stops diagnosing as unsupported.
                if parts.len() == 2 {
                    return self.map_an_indexed(parts, 0, *span, env);
                }
                // A GROUP size suffix on a one-part indirect is the
                // explicit-width absolute form: `(Sym).w` / `($C00004).l`
                // (Volence-ratified, tranche 3). A sized REGISTER indirect
                // (`(a0).w`) is not a 68000 form and is rejected inside.
                if size.is_some() && parts.len() == 1 {
                    return self.map_pinned_abs(&parts[0].0, size.as_ref(), scope, env, *span);
                }
                let r = self.ind_single_reg(parts, *span, env)?;
                Some(CodeOperand::Ind(r))
            }
            Operand::PreDec(inner) => {
                let r = self.inner_ind_reg(inner, env)?;
                Some(CodeOperand::PreDec(r))
            }
            Operand::PostInc(inner) => {
                let r = self.inner_ind_reg(inner, env)?;
                Some(CodeOperand::PostInc(r))
            }
            Operand::DispInd { disp, inner, disp_spliced, field_size_override, span } => {
                // Decode a `:b`/`:w`/`:l` sized override (overlay-write, only
                // meaningful on the field-space paths below) to its byte count.
                let override_bytes: Option<i128> = match field_size_override {
                    Some(ast::TextOrSplice::Text(s)) => match s.as_str() {
                        "b" => Some(1),
                        "w" => Some(2),
                        "l" => Some(4),
                        _ => None,
                    },
                    _ => None,
                };
                // PC-relative EAs: `Sym(pc)` / `Sym(pc,Xn.size)` — 68k `(d16,PC)` /
                // `(d8,PC,Xn)`. Keyed on the inner base being the LITERAL `pc`
                // token (never a valid address register, so this can't collide
                // with the field-space or `(aN)` paths below). Must run BEFORE
                // the field-space checks and the eager `eval_expr(disp, ..)` — a
                // pc-relative target is a link symbol, not a comptime-evaluable
                // displacement (that eager eval is exactly what makes
                // `Sym(pc,d0.w)` fail as "unknown name" today).
                if let Some(shape) = pc_rel_shape(inner) {
                    return self.map_pcrel_operand(disp, shape, scope, env, *span);
                }
                // A LOCAL-LABEL displacement over An (`jmp .jump_table(a1)` —
                // the jump-table dispatch idiom: the table's abs.w-reachable
                // ADDRESS rides the d16 field, the base register carries a
                // byte offset into the table). The parser spells a `.local`
                // operand as a single-segment Path whose segment keeps the
                // leading dot; resolve it through the label scope (hygiene)
                // to the emitted symbol and defer the address to link as a
                // symbolic d16. Must run BEFORE the eager `eval_expr` below —
                // a label is a link symbol, not a comptime displacement.
                if let ast::Expr::Path(p) = disp {
                    if p.segments.len() == 1 && p.segments[0].starts_with('.') {
                        let target = scope.resolve_ref(&p.segments[0]);
                        let r = self.inner_ind_reg(inner, env)?;
                        return Some(CodeOperand::DispSymInd { target, reg: r });
                    }
                }
                // D6.A3: a BARE single-segment displacement `f(aN)` on a register
                // whose declared param type bottoms out at `*S` resolves ONLY in
                // FIELD SPACE (S's direct fields ∪ in-scope overlays over S) — a
                // field name lowers to its byte offset, and a const never silently
                // shadows it. Peek the register WITHOUT reporting (a bad register
                // is diagnosed on the shared path below, preserving today's
                // diagnostics); only the field-space case diverges.
                if let Some(field) = single_segment_field(disp) {
                    if let Some(reg) = self.peek_inner_reg(inner, env) {
                        if let Some(base) = self.reg_pointee_struct.get(&reg).cloned() {
                            match self.field_disp_silent(&base, field, expr_span(disp)) {
                                Ok(Some((d, size))) => {
                                    // Overrun is diagnosed but the operand is emitted anyway
                                    // (deliberate error-recovery): the displacement is valid,
                                    // so downstream passes still see a well-formed operand.
                                    // struct size is only needed for the overlay end-bound
                                    // (override present) — skip the layout query otherwise.
                                    let ssize = if override_bytes.is_some() {
                                        self.layout_of_struct(&base, *span).size as i128
                                    } else { 0 };
                                    self.check_field_overrun(
                                        FieldAccessCheck { field, field_size: size, field_offset: d, struct_size: ssize, override_bytes },
                                        width, *span,
                                    );
                                    return Some(CodeOperand::DispInd { disp: d, reg });
                                }
                                // Ruling #1: `field` is NOT a field of `*base`. If it is an
                                // in-scope CONST, fall THROUGH to the const/comptime path
                                // below — a typed register is a strict SUPERSET of untyped
                                // (an imported offset const `Act_grid_w_lo(aN)` resolves as
                                // that const, not a drop). If it is NEITHER a field NOR a
                                // const, it is most likely a FIELD TYPO — keep the
                                // informative `[operand.unknown-field]` naming `*base`. So
                                // the tranche-7b typo-guard survives for every non-const
                                // name; only a real field/const NAME collision degrades it,
                                // and none exists in the corpus.
                                Ok(None) => {
                                    if !self.is_const(field) {
                                        self.error(
                                            expr_span(disp),
                                            format!(
                                                "[operand.unknown-field] `*{base}` has no field or in-scope overlay field `{field}`"
                                            ),
                                        );
                                        return None;
                                    }
                                }
                                // An AMBIGUOUS overlay field is a real error (already
                                // diagnosed) — drop, never launder to a const.
                                Err(()) => return None,
                            }
                        }
                    }
                }
                // D6.A4: a QUALIFIED two-segment displacement `Qual.field(aN)`
                // resolves in field space on ANY address register (the
                // qualification is the type assertion). If `Qual` names an overlay
                // or struct → field-space resolution; otherwise (e.g. an `offsets`
                // ordinal `T.B`) fall through to comptime eval unchanged.
                if let Some((qual, field)) = two_segment_field(disp) {
                    if self.overlays.contains_key(qual) || self.structs.contains_key(qual) {
                        if let Some(reg) = self.peek_inner_reg(inner, env) {
                            let (d, size) =
                                self.resolve_qualified_field(qual, field, expr_span(disp))?;
                            // Same deliberate error-recovery as the bare form. The
                            // struct-end bound uses the BASE struct's size (a field
                            // offset is base-relative even through an overlay window).
                            let ssize = if override_bytes.is_none() {
                                0 // end-bound unused without an override; skip the layout query
                            } else if self.structs.contains_key(qual) {
                                self.layout_of_struct(qual, *span).size as i128
                            } else {
                                let bs = self.overlay_layout(qual, *span).base_struct.clone();
                                self.layout_of_struct(&bs, *span).size as i128
                            };
                            self.check_field_overrun(
                                FieldAccessCheck { field, field_size: size, field_offset: d, struct_size: ssize, override_bytes },
                                width, *span,
                            );
                            return Some(CodeOperand::DispInd { disp: d, reg });
                        }
                    }
                }
                // All other shapes (multi-segment paths, non-path exprs, untyped
                // register) keep today's semantics: comptime-eval the disp, then
                // resolve the register — byte-for-byte unchanged.
                let dv = self.eval_expr(disp, env);
                if matches!(dv, Value::Poison) {
                    return None;
                }
                // A provisional here() displacement gets the SPECIFIC D-H.2
                // steering message.
                if self.reject_if_provisional(&dv, *span).is_some() {
                    return None;
                }
                let Some(d) = dv.as_stored_int() else {
                    // A spliced displacement (`{off}(aN)`, F1) that is not an int
                    // gets the operand-splice diagnostic naming the expected class
                    // (`[asm.splice-kind]`); a literal/field displacement keeps its
                    // generic "must be an integer" message.
                    if *disp_spliced {
                        self.splice_kind_err(expr_span(disp), "int", &dv);
                    } else {
                        self.error(
                            *span,
                            format!("displacement must be an integer, got {}", dv.type_name()),
                        );
                    }
                    return None;
                };
                // Two-part `d8(An,Xn[.size])` — An-indexed with a comptime
                // displacement. The pc-indexed shape was peeled off above, and
                // the D6.A3/A4 field-space peeks only match one-part inners, so
                // a two-part inner reaching here is exactly this form.
                if let Operand::Ind { parts, .. } = &**inner {
                    if parts.len() == 2 {
                        return self.map_an_indexed(parts, d, *span, env);
                    }
                }
                let r = self.inner_ind_reg(inner, env)?;
                Some(CodeOperand::DispInd { disp: d, reg: r })
            }
            Operand::Splice(e) => {
                let v = self.eval_expr(e, env);
                self.classify_operand_splice(v, expr_span(e))
            }
        }
    }

    /// Map one parsed [`ast::Operand`] to a [`CodeOperand`] under a Z80 section
    /// (T1, §3). The image of `z80.rs`'s operand surface, one abstraction layer
    /// up: a bare register/pair/`i`/`r`/`af'` word resolves to its Z80 operand,
    /// `(hl)`/`(bc)`/`(de)` to the register indirects, `(ix±d)` to the indexed
    /// form (i8-range-checked), `(nn)` to a comptime absolute address, a bare
    /// int / `#n` to a CPU-neutral [`CodeOperand::Imm`] (the Z80 lowering picks
    /// imm8 vs imm16 by form), and any other bareword to a link `Sym` (a label
    /// operand → symbolic imm16, §4). Forms with no Z80 meaning
    /// (`-(reg)`/`(reg)+`/`d(reg)`) are `[lower.z80-unsupported]`.
    fn map_operand_z80(
        &mut self,
        op: &Operand,
        scope: &LabelScope,
        env: &mut Env,
    ) -> Option<CodeOperand> {
        match op {
            // `#n` — the 68k-style immediate spelling. Rare on Z80 (bare `n` is
            // the usual form, handled by the `Plain` arm) but accepted.
            Operand::Imm(e) => {
                let v = self.eval_expr(e, env);
                match v.as_stored_int() {
                    Some(n) => Some(CodeOperand::Imm(n)),
                    None => {
                        self.error(
                            expr_span(e),
                            format!("immediate must be an integer, got {}", v.type_name()),
                        );
                        None
                    }
                }
            }
            Operand::Plain { expr, .. } => self.map_plain_z80(expr, scope, env),
            Operand::Ind { parts, size, span } => {
                if size.is_some() {
                    self.error(
                        *span,
                        "[lower.z80-unsupported] a size-suffixed indirect is not a Z80 operand form"
                            .to_string(),
                    );
                    return None;
                }
                self.map_ind_z80(parts, *span, env)
            }
            Operand::Splice(e) => {
                let v = self.eval_expr(e, env);
                self.classify_operand_splice(v, expr_span(e))
            }
            Operand::PreDec(_) | Operand::PostInc(_) | Operand::DispInd { .. } => {
                self.error(
                    operand_span(op),
                    "[lower.z80-unsupported] this addressing form has no Z80 operand encoding"
                        .to_string(),
                );
                None
            }
        }
    }

    /// The Z80 branch of [`Self::map_plain`] (§3): a single-segment path is a
    /// Z80 register class (`a`..`l` / a pair / `i` / `r` / `af'`), else a link
    /// symbol (a label operand); a bare int / comptime expr is a [`Value::Int`]
    /// → [`CodeOperand::Imm`].
    fn map_plain_z80(
        &mut self,
        expr: &ast::Expr,
        scope: &LabelScope,
        env: &mut Env,
    ) -> Option<CodeOperand> {
        if let ast::Expr::Path(p) = expr {
            if p.segments.len() == 1 {
                let seg = p.segments[0].as_str();
                if let Some(r) = Z80Reg8::from_name(seg) {
                    return Some(CodeOperand::Z80Reg8(r));
                }
                if let Some(pr) = Z80Pair::from_name(seg) {
                    return Some(CodeOperand::Z80Pair(pr));
                }
                match seg {
                    "af'" => return Some(CodeOperand::Z80AfShadow),
                    "i" => return Some(CodeOperand::Z80RegI),
                    "r" => return Some(CodeOperand::Z80RegR),
                    _ => {}
                }
                // A bare non-register word that resolves to a COMPTIME integer (a
                // file `const`/`equ` or a `-D` define) folds to a CPU-neutral
                // immediate — the Z80 lowering picks imm8/imm16 by form. Without
                // this the single-segment fast path skips the comptime fold that
                // every COMPOUND expr (`CONST+0`, `CONST-1`) already reaches below,
                // so a bare `sub CONST` / `ld b, CONST` / `set BIT, (ix+d)`
                // mis-routes to the symbolic-imm16 path ([lower.z80-unsupported] —
                // the psg port surfaced this, the first Z80 code with symbolic
                // 8-bit immediates). A word that is NOT a comptime int stays a
                // link `Sym` (a label operand), so `call`/`jp`/`ld rr,Label` and
                // local-label branch-target resolution are unchanged.
                if let Some(n) = self.z80_bareword_const(seg, p.span, env) {
                    return Some(CodeOperand::Imm(n));
                }
                // Not a register or comptime constant: a link symbol (a label in
                // operand position → the symbolic imm16 of §4, or a `jp`/`call`
                // target).
                return Some(CodeOperand::Sym(scope.resolve_ref(seg)));
            }
            // Multi-segment: an exported cross-body label (`Owner.label`).
            return Some(CodeOperand::Sym(scope.resolve_ref(&p.segments.join("."))));
        }
        // A bare integer literal (`ld a, 5`, `ld (hl), 0E9h`) or a comptime int
        // expr folds to a CPU-neutral immediate; a symbol-bearing expression
        // that CANNOT fold (`Label+1`, `CONST-Label-1` — z80_init's clear-count
        // and self-patch lines) defers to a link-time `ImmLink`, the Z80 analog
        // of the 68k `#(Label+1)` deferral in `map_operand`'s `Imm` arm.
        // `in_imm_link_ctx` enables the label-arithmetic fallback; the Z80
        // lowering (`lower_z80_abs16_sym`) turns the `ImmLink` into the SAME
        // `Value16Le` fixup a bare label takes, now carrying the residual Expr.
        // A comptime bareword outside this context keeps its loud `unknown name`.
        let v = self.in_imm_link_ctx(|this| this.eval_expr(expr, env));
        if matches!(v, Value::Poison) {
            return None;
        }
        if let Value::Label(n) = &v {
            return Some(CodeOperand::Sym(n.clone()));
        }
        if let Value::LinkExpr(e) = &v {
            if !crate::eval::expr::expr_carries_bank_mask(e) {
                return Some(CodeOperand::ImmLink { target: e.clone() });
            }
            self.reject_if_provisional(&v, expr_span(expr));
            return None;
        }
        self.classify_operand_splice(v, expr_span(expr))
    }

    /// A bare single-segment word that resolves to a COMPTIME integer (a file
    /// `const`/`equ` or a `-D` define), or `None` if it is not a known comptime
    /// constant — a register, a proc-local label, a `proc`/`data` link label, or
    /// an unknown name, all of which must stay a link `Sym` in Z80 operand
    /// position. Mirrors [`Self::eval_path`]'s single-segment const precedence
    /// (local binding → file const/equ → `-D` define) but WITHOUT its label /
    /// unknown-name fallback, so it never errors and never steals a label. A
    /// non-integer const (e.g. a string) yields `None` and stays a `Sym`.
    fn z80_bareword_const(&mut self, name: &str, span: Span, env: &Env) -> Option<i128> {
        if name.starts_with('.') {
            return None; // a proc-local label reference, never a comptime constant
        }
        let v = if let Some(v) = env.lookup(name) {
            v.clone()
        } else if self.consts.contains_key(name) || self.equs.contains_key(name) {
            self.resolve_const(name, span)
        } else {
            let d = self.defines.get(name)?;
            Value::Int(*d)
        };
        v.as_stored_int()
    }

    /// The Z80 branch of [`Operand::Ind`] mapping (§3): a single-part paren is
    /// `(hl)`/`(bc)`/`(de)` (a register), `(ix±d)`/`(iy±d)` (an index register ±
    /// comptime displacement, i8-range-checked), or `(nn)` (a comptime absolute
    /// address). The i8 check REUSES the same `i8::MIN..=i8::MAX` window the 68k
    /// brief-extension disp8 path runs (`map_an_indexed`), the one shared
    /// invariant "(ix+d) disp is i8".
    fn map_ind_z80(
        &mut self,
        parts: &[(ast::Expr, Option<TextOrSplice>)],
        span: Span,
        env: &mut Env,
    ) -> Option<CodeOperand> {
        if parts.len() != 1 {
            self.error(
                span,
                "[lower.z80-unsupported] a multi-part indirect is not a Z80 operand form"
                    .to_string(),
            );
            return None;
        }
        let (e, psize) = &parts[0];
        if psize.is_some() {
            self.error(
                span,
                "[lower.z80-unsupported] a size-suffixed indirect is not a Z80 operand form"
                    .to_string(),
            );
            return None;
        }
        // `(hl)`/`(bc)`/`(de)`.
        if let ast::Expr::Path(p) = e {
            if p.segments.len() == 1 {
                match p.segments[0].as_str() {
                    "hl" => return Some(CodeOperand::Z80IndHl),
                    "bc" => return Some(CodeOperand::Z80IndBc),
                    "de" => return Some(CodeOperand::Z80IndDe),
                    // `(sp)` — only `ex (sp),hl`/`ex (sp),ix` encode it; any other
                    // mnemonic pairing errors at the ISA encode gate. Distinct
                    // from a bare `sp` so `sp_hazard` sees the trampoline swap.
                    "sp" => return Some(CodeOperand::Z80IndSp),
                    _ => {}
                }
            }
        }
        // `(ix±d)` / `(iy±d)`.
        if let Some((reg, disp)) = self.z80_index_disp(e, env) {
            if disp < i8::MIN as i128 || disp > i8::MAX as i128 {
                self.error(
                    span,
                    format!(
                        "(ix+d)/(iy+d) displacement must fit the 8-bit signed field \
                         (-128..=127), got {disp}"
                    ),
                );
                return None;
            }
            return Some(CodeOperand::Z80Indexed { reg, disp });
        }
        // `(nn)` — a comptime absolute address.
        let v = self.eval_expr(e, env);
        match v.as_stored_int() {
            Some(addr) => Some(CodeOperand::Z80Mem { addr }),
            None => {
                self.error(
                    expr_span(e),
                    format!(
                        "[lower.z80-unsupported] a Z80 indirect base must be `hl`/`bc`/`de`, an \
                         index register `(ix±d)`, or a comptime address `(nn)`, got {}",
                        v.type_name()
                    ),
                );
                None
            }
        }
    }

    /// Match `ix`/`iy`, optionally `± <comptime int>`, for the `(ix±d)` indexed
    /// form. Returns the index register and its (signed) displacement, or `None`
    /// if the inner expression does not name an index register.
    fn z80_index_disp(
        &mut self,
        e: &ast::Expr,
        env: &mut Env,
    ) -> Option<(Z80Index, i128)> {
        if let ast::Expr::Path(p) = e {
            if p.segments.len() == 1 {
                if let Some(idx) = Z80Index::from_name(&p.segments[0]) {
                    return Some((idx, 0));
                }
            }
        }
        if let ast::Expr::Binary { op, lhs, rhs, .. } = e {
            if matches!(op, ast::BinOp::Add | ast::BinOp::Sub) {
                if let ast::Expr::Path(p) = &**lhs {
                    if p.segments.len() == 1 {
                        if let Some(idx) = Z80Index::from_name(&p.segments[0]) {
                            let d = self.eval_expr(rhs, env).as_stored_int()?;
                            let d = if matches!(op, ast::BinOp::Sub) { -d } else { d };
                            return Some((idx, d));
                        }
                    }
                }
            }
        }
        None
    }

    /// Resolve a `Sym(pc)` / `Sym(pc,Xn[.size])` operand (a [`PcRelShape`]
    /// already peeled off `inner`) to a [`CodeOperand::PcRel`] /
    /// [`CodeOperand::PcRelIdx`]. `disp` is the TARGET — a link symbol, exactly
    /// like a branch target or [`Self::map_plain`]'s bare `Sym`, NOT a
    /// comptime-evaluable displacement (the actual byte displacement is a
    /// link-time fixup: `FixupKind::PcRelDisp16`/`PcRelDisp8`, resolved by
    /// `sigil-link` from the same VMA-distance arithmetic `bra`/`bsr` already
    /// use — see the module-level doc on cross-section behavior).
    ///
    /// The target resolution mirrors `map_plain`'s `Sym` arm exactly (single
    /// segment → hygiene-scoped local/label lookup; multi-segment →
    /// `Owner.label` cross-body reference) rather than calling it directly,
    /// because `map_plain` returns a full `CodeOperand` (register/`Sym`/...)
    /// and we specifically need just the resolved symbol STRING here.
    fn map_pcrel_operand(
        &mut self,
        disp: &ast::Expr,
        shape: PcRelShape<'_>,
        scope: &LabelScope,
        env: &mut Env,
        span: Span,
    ) -> Option<CodeOperand> {
        // `Sym±n(pc[,Xn])` — a comptime addend on the symbolic target (the
        // dispatch-table anchor idiom, tranche 9: `jmp .cc_table-4(pc,d0.w)`
        // where the adjusted address lands INSIDE the jmp itself, so no
        // relocated label can absorb it). The symbol stays a link-time fixup;
        // only the addend folds here.
        let (path_expr, addend) = match disp {
            ast::Expr::Binary { op, lhs, rhs, .. }
                if matches!(op, ast::BinOp::Add | ast::BinOp::Sub)
                    && matches!(&**lhs, ast::Expr::Path(_)) =>
            {
                let nv = self.eval_expr(rhs, env);
                if matches!(nv, Value::Poison) {
                    return None;
                }
                let Some(n) = nv.as_stored_int() else {
                    self.error(
                        expr_span(rhs),
                        format!(
                            "a PC-relative target addend must be a comptime integer, got {}",
                            nv.type_name()
                        ),
                    );
                    return None;
                };
                let Ok(n) = i64::try_from(n) else {
                    self.error(expr_span(rhs), "PC-relative target addend out of range".to_string());
                    return None;
                };
                (&**lhs, if matches!(op, ast::BinOp::Sub) { -n } else { n })
            }
            other => (other, 0),
        };
        let ast::Expr::Path(p) = path_expr else {
            self.error(
                expr_span(disp),
                "a PC-relative target must be a label or symbol path (optionally ± a comptime integer)".to_string(),
            );
            return None;
        };
        let target = scope.resolve_ref(&p.segments.join("."));
        match shape {
            PcRelShape::Plain => Some(CodeOperand::PcRel { target, addend }),
            PcRelShape::Indexed { xn_expr, xn_size } => {
                let xn = match xn_expr {
                    ast::Expr::Path(xp) if xp.segments.len() == 1 => {
                        reg_from_name(&xp.segments[0])
                    }
                    _ => None,
                };
                let Some(xn) = xn else {
                    self.error(
                        expr_span(xn_expr),
                        "`pc`-relative indexed addressing needs a valid index register \
                         (d0-d7/a0-a7)"
                            .to_string(),
                    );
                    return None;
                };
                let xlong = self.resolve_index_size(xn_size, span, env, "`pc`-relative ")?;
                Some(CodeOperand::PcRelIdx { target, addend, xn, xlong })
            }
        }
    }

    /// Resolve a brief-extension index size suffix: `.l` → long, `.w` or
    /// unsuffixed → sign-extended word (the AS-matching default). Anything
    /// else diagnoses with `ctx` prefixed to the message (`"`pc`-relative "`
    /// for the pc-indexed form, `""` for the An-indexed form). Shared by
    /// [`Self::map_pcrel_operand`] and [`Self::map_an_indexed`].
    fn resolve_index_size(
        &mut self,
        xn_size: Option<&TextOrSplice>,
        span: Span,
        env: &mut Env,
        ctx: &str,
    ) -> Option<bool> {
        match self.resolve_size(xn_size, span, env) {
            Some(Some(Width::L)) => Some(true),
            Some(Some(Width::W)) | Some(None) => Some(false),
            Some(Some(other)) => {
                let suffix = match other {
                    Width::B => "b",
                    Width::S => "s",
                    Width::W | Width::L => unreachable!("handled above"),
                };
                self.error(
                    span,
                    format!("{ctx}index size must be `.w` or `.l`, got `.{suffix}`"),
                );
                None
            }
            None => None,
        }
    }

    /// Resolve a one-part sized indirect `(expr).w` / `(expr).l` to an
    /// explicit-width ABSOLUTE operand (Volence-ratified, tranche 3 — the
    /// AS-parity forced-width spelling; the bare-symbol relax-via-width-rule
    /// idiom stays the new-style default). A bare path is ALWAYS a symbol
    /// (`map_plain`'s rule — never a const read in operand position) →
    /// [`CodeOperand::AbsSym`], width pinned, address deferred as one
    /// fixed-width fixup. Anything else comptime-evaluates to an integer
    /// address → [`CodeOperand::AbsInt`], with the `.w` window validated
    /// against asl's sign-extension rule. A register (spelled or evaluated)
    /// is rejected: `(a0).w` is not a 68000 form, and silently dropping the
    /// suffix was the same hazard class as the indexed base suffix.
    fn map_pinned_abs(
        &mut self,
        expr: &ast::Expr,
        size: Option<&TextOrSplice>,
        scope: &LabelScope,
        env: &mut Env,
        span: Span,
    ) -> Option<CodeOperand> {
        let long = match self.resolve_size(size, span, env) {
            Some(Some(Width::L)) => true,
            Some(Some(Width::W)) => false,
            Some(Some(other)) => {
                let suffix = match other {
                    Width::B => "b",
                    Width::S => "s",
                    Width::W | Width::L => unreachable!("handled above"),
                };
                self.error(
                    span,
                    format!("absolute width must be `.w` or `.l`, got `.{suffix}`"),
                );
                return None;
            }
            Some(None) => {
                // Caller only routes here when a size suffix is present.
                self.error(span, "absolute width must be `.w` or `.l`".to_string());
                return None;
            }
            None => return None,
        };
        if let ast::Expr::Path(p) = expr {
            if p.segments.len() == 1 && reg_from_name(&p.segments[0]).is_some() {
                self.error(
                    span,
                    "register indirect takes no size suffix — `(a0).w` is not a 68000 form"
                        .to_string(),
                );
                return None;
            }
            // `(sr).w` would otherwise resolve as an ordinary SYMBOL named
            // `sr` and fail only at link — steer early, matching the
            // register-class-words-win rule the bare path applies.
            if p.segments.len() == 1 && matches!(p.segments[0].as_str(), "sr" | "ccr") {
                self.error(
                    span,
                    format!(
                        "`({0}).w` is not a 68000 form — `{0}` is the status-register operand, not an address",
                        p.segments[0]
                    ),
                );
                return None;
            }
            let target = scope.resolve_ref(&p.segments.join("."));
            return Some(CodeOperand::AbsSym { target, long });
        }
        let v = self.eval_expr(expr, env);
        if matches!(v, Value::Poison) {
            return None;
        }
        if matches!(v, Value::Reg(_)) {
            self.error(
                span,
                "register indirect takes no size suffix — `(a0).w` is not a 68000 form"
                    .to_string(),
            );
            return None;
        }
        if self.reject_if_provisional(&v, span).is_some() {
            return None;
        }
        let Some(addr) = v.as_stored_int() else {
            self.error(
                span,
                format!(
                    "an explicit-width absolute needs an integer address or a symbol, got {}",
                    v.type_name()
                ),
            );
            return None;
        };
        if !long {
            // asl's abs.w window: the 24-bit address must sign-extend
            // losslessly from 16 bits (shared rule, sigil-ir width.rs).
            let a = (addr as i64) & 0xFF_FFFF;
            if !(a <= 0x7FFF || a >= 0xFF_8000) {
                self.error(
                    span,
                    format!(
                        "address {addr:#X} has no abs.w spelling (the 24-bit address must \
                         sign-extend losslessly from 16 bits) — use `.l`"
                    ),
                );
                return None;
            }
        } else if addr < i32::MIN as i128 || addr > u32::MAX as i128 {
            self.error(span, format!("address out of range for abs.l: {addr}"));
            return None;
        }
        Some(CodeOperand::AbsInt { addr, long })
    }

    /// Resolve a two-part `(An,Xn[.size])` inner to an An-indexed operand
    /// (68k `(d8,An,Xn)`, brief extension word) with the given comptime
    /// displacement — `0` from the bare `(An,Xn)` spelling, the evaluated
    /// displacement from `d8(An,Xn)`. Everything here is comptime (register
    /// spellings + an integer), so unlike the pc-indexed sibling nothing
    /// defers to the linker; the displacement is range-checked to the brief
    /// extension's signed-8-bit field now, at the spelling's own span.
    fn map_an_indexed(
        &mut self,
        parts: &[(ast::Expr, Option<TextOrSplice>)],
        disp: i128,
        span: Span,
        env: &mut Env,
    ) -> Option<CodeOperand> {
        debug_assert_eq!(parts.len(), 2, "caller checked the two-part shape");
        // `(pc,Xn)` without a displacement is a plausible mis-spelling of the
        // pc-indexed form — steer to it instead of "unknown name `pc`".
        if let ast::Expr::Path(p) = &parts[0].0 {
            if p.segments.len() == 1 && p.segments[0] == "pc" {
                self.error(
                    span,
                    "`pc` cannot be an indexed base without a target — pc-relative \
                     indexed addressing is spelled `Sym(pc,Xn.size)`"
                        .to_string(),
                );
                return None;
            }
        }
        // AS rejects a base size suffix on the 68000 (`(a0.l,d2.w)` is 68020
        // syntax with different semantics) — silently ignoring it would be a
        // byte-exactness hazard.
        if parts[0].1.is_some() {
            self.error(
                span,
                "the base register takes no size suffix in `(An,Xn)` addressing".to_string(),
            );
            return None;
        }
        let (xn_expr, xn_size) = &parts[1];
        let reg = self.ind_single_reg(std::slice::from_ref(&parts[0]), span, env)?;
        // The index slot mirrors the base (`ind_single_reg`): a literal register
        // spelling (`d3`) resolves without evaluating; anything else — a param
        // naming a Reg (`{off}` lowers to `Path([off])`), a const, an arbitrary
        // expr — evaluates and must yield a `Reg`. This closes the base/index
        // asymmetry that blocked spliced-index helpers (frame_piece_count).
        let literal_xn = match xn_expr {
            ast::Expr::Path(xp) if xp.segments.len() == 1 => reg_from_name(&xp.segments[0]),
            _ => None,
        };
        let xn = match literal_xn {
            Some(r) => r,
            None => match self.eval_expr(xn_expr, env) {
                Value::Reg(r) => r,
                Value::Poison => return None,
                _ => {
                    self.error(
                        expr_span(xn_expr),
                        "indexed addressing needs a valid index register (d0-d7/a0-a7)"
                            .to_string(),
                    );
                    return None;
                }
            },
        };
        let xlong = self.resolve_index_size(xn_size.as_ref(), span, env, "")?;
        // The brief extension word carries a signed 8-bit displacement.
        if disp < i8::MIN as i128 || disp > i8::MAX as i128 {
            self.error(
                span,
                format!(
                    "indexed-addressing displacement must fit the brief extension's \
                     8-bit field (-128..=127), got {disp}"
                ),
            );
            return None;
        }
        Some(CodeOperand::IndIdx { reg, disp, xn, xlong })
    }

    /// Peek the base register of a `(aN)` inner operand WITHOUT emitting any
    /// diagnostic (D6.A3). Only a one-part register-indirect `(aN)` yields a
    /// register; anything else (indexed/absolute, a non-register base) yields
    /// `None` and the shared displacement path below re-derives it, reporting as
    /// today. This peek is SYNTACTIC only: it matches a LITERAL register spelling
    /// (`a0`) in the AST and never evaluates — an evaluated or aliased base (e.g.
    /// a `{splice}` or a const naming a register) yields `None` here and falls
    /// through to the shared [`inner_ind_reg`](Self::inner_ind_reg) path.
    fn peek_inner_reg(&self, inner: &Operand, env: &Env) -> Option<Reg> {
        let Operand::Ind { parts, .. } = inner else { return None };
        if parts.len() != 1 {
            return None;
        }
        if let ast::Expr::Path(p) = &parts[0].0 {
            if p.segments.len() == 1 {
                if let Some(r) = reg_from_name(&p.segments[0]) {
                    return Some(r);
                }
                // A SPLICED base register (`Sst.field({sst})` inside a
                // comptime-fn template): the splice lowers to a one-segment
                // path bound to a `Value::Reg` in the template's env. Field
                // space is exactly as well-defined over it as over a literal
                // register, so resolve it here — otherwise a field
                // displacement silently falls through to comptime eval and
                // dies as "unknown name `Struct.field`". The lookup is a
                // BINDING read, never an eval: this peek must stay silent
                // (its callers own the diagnostics), and no other expression
                // shape can name a register without evaluation.
                if let Some(Value::Reg(r)) = env.lookup(&p.segments[0]) {
                    return Some(*r);
                }
            }
        }
        None
    }

    /// Resolve a bare field name against struct `base`'s FIELD SPACE (D6.A3):
    /// `base`'s direct fields ∪ the fields of every in-scope overlay whose
    /// `base_struct` is `base`. Returns `(displacement, field-byte-size)` where
    /// the displacement is the direct field's struct offset or `window_offset +
    /// overlay-relative offset`. Zero hits → `[operand.unknown-field]` (NO const
    /// fallback on a typed register); ≥2 hits across distinct overlays →
    /// `[operand.ambiguous-field]` listing the qualified candidates.
    /// SILENT field-space lookup (direct field ∪ in-scope overlay fields over
    /// `base`) with NO not-found diagnostic — the caller decides what a miss
    /// means. `Ok(Some(hit))` = an unambiguous field; `Ok(None)` = NOT a field of
    /// `base` (the bare D6.A3 form falls back to const resolution, ruling #1;
    /// `resolve_field_disp` reports it); `Err(())` = an AMBIGUOUS overlay field, a
    /// real error emitted here (never laundered to a const).
    fn field_disp_silent(
        &mut self,
        base: &str,
        field: &str,
        span: Span,
    ) -> Result<Option<(i128, i128)>, ()> {
        // Direct field first (a direct field can never be shadowed by an overlay:
        // `[overlay.shadows-field]` rejects that at the overlay decl, D6.A7).
        if let Some(hit) = self.field_in_struct(base, field, span) {
            return Ok(Some(hit));
        }
        // Overlay fields: scan every in-scope overlay whose window belongs to
        // `base`. Collect qualified hits so an ambiguity can name them. The
        // overlay index is a HashMap; sort candidate names for a stable message.
        let mut overlay_names: Vec<String> = self.overlays.keys().map(|s| s.to_string()).collect();
        overlay_names.sort();
        let mut hits: Vec<(String, i128, i128)> = Vec::new();
        for oname in overlay_names {
            // Only overlays whose window belongs to `base` are candidates for the
            // bare form (the overlay-qualified form skips this base filter).
            if self.overlay_layout(&oname, span).base_struct != base {
                continue;
            }
            if let Some((disp, size)) = self.field_in_overlay(&oname, field, span) {
                hits.push((oname, disp, size));
            }
        }
        match hits.as_slice() {
            [] => Ok(None),
            [(_, disp, size)] => Ok(Some((*disp, *size))),
            many => {
                let candidates = many
                    .iter()
                    .map(|(o, _, _)| format!("{o}.{field}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.error(
                    span,
                    format!(
                        "[operand.ambiguous-field] field `{field}` is ambiguous across {candidates} — qualify it as `Overlay.{field}`"
                    ),
                );
                Err(())
            }
        }
    }

    pub(crate) fn resolve_field_disp(
        &mut self,
        base: &str,
        field: &str,
        span: Span,
    ) -> Option<(i128, i128)> {
        match self.field_disp_silent(base, field, span) {
            Ok(Some(hit)) => Some(hit),
            Ok(None) => {
                self.error(
                    span,
                    format!(
                        "[operand.unknown-field] `*{base}` has no field or in-scope overlay field `{field}`"
                    ),
                );
                None
            }
            Err(()) => None,
        }
    }

    /// Look up a DIRECT field of struct `base` by name, returning `(struct
    /// offset, field-byte-size)` if present. Shared by the bare field-space scan
    /// (D6.A3) and the struct-qualified form (D6.A4).
    fn field_in_struct(&mut self, base: &str, field: &str, span: Span) -> Option<(i128, i128)> {
        let layout = self.layout_of_struct(base, span);
        layout
            .fields
            .iter()
            .find(|f| f.name == field)
            .map(|f| (f.offset as i128, f.size as i128))
    }

    /// Look up a field of the indexed overlay named `overlay` by name, returning
    /// `(window_offset + overlay-relative offset, field-byte-size)` if present. A
    /// poisoned overlay layout yields `None`. Shared by the bare scan (via a
    /// window-match filter, D6.A3) and the overlay-qualified form (D6.A4).
    fn field_in_overlay(
        &mut self,
        overlay: &str,
        field: &str,
        span: Span,
    ) -> Option<(i128, i128)> {
        let info = self.overlay_layout(overlay, span);
        if info.poisoned {
            return None;
        }
        info.fields
            .iter()
            .find(|(n, _, _)| n == field)
            .map(|(_, rel, size)| (info.window_offset + rel, *size))
    }

    /// D6.A4 — QUALIFIED field access `Qual.field(aN)`: a two-segment
    /// displacement path resolves in field space explicitly and is legal on ANY
    /// address register (the qualification IS the type assertion). If `qual`
    /// names an indexed overlay → resolve `field` among ITS fields; else if it
    /// names a struct → resolve `field` among its DIRECT fields; ELSE → `None`
    /// (caller falls through to today's comptime eval, so an `offsets`/const
    /// first segment keeps its ordinal meaning). A recognized qualifier with an
    /// unknown field is `[operand.unknown-field]` naming the qualifier.
    fn resolve_qualified_field(
        &mut self,
        qual: &str,
        field: &str,
        span: Span,
    ) -> Option<(i128, i128)> {
        if self.overlays.contains_key(qual) {
            if let Some(hit) = self.field_in_overlay(qual, field, span) {
                return Some(hit);
            }
            self.error(
                span,
                format!("[operand.unknown-field] overlay `{qual}` has no field `{field}`"),
            );
            return None;
        }
        if self.structs.contains_key(qual) {
            if let Some(hit) = self.field_in_struct(qual, field, span) {
                return Some(hit);
            }
            self.error(
                span,
                format!("[operand.unknown-field] struct `{qual}` has no field `{field}`"),
            );
            return None;
        }
        None
    }

    /// D6.A6: an access WIDER than the resolved field crosses a named boundary →
    /// `[operand.field-overrun]`. Narrower or equal is legal (the big-endian
    /// high-byte idiom), no lint. An unsized instruction (no `.b/.w/.l`) carries
    /// no access width here, so the check is skipped — the width is decided later
    /// by the encoder and the field boundary cannot be judged at this seam.
    fn check_field_overrun(&mut self, c: FieldAccessCheck, width: Option<Width>, span: Span) {
        let FieldAccessCheck { field, field_size, field_offset, struct_size, override_bytes } = c;
        let access = match width {
            Some(Width::B) => 1,
            Some(Width::W) => 2,
            Some(Width::L) => 4,
            // `.s` is a branch-displacement size, never an operand access width;
            // and no-suffix means "decided later" — skip in both cases.
            Some(Width::S) | None => return,
        };
        // A `:size` override (`Sst.prev_anim:l`) DECLARES the overlay width the
        // access is checked against, replacing the field's own size. It is a
        // STATED width, not a mute switch: `move.l` against a `:w` override still
        // overruns (4 > 2).
        let effective = override_bytes.unwrap_or(field_size);
        if access > effective {
            let Some(w) = width else { return };
            let target = if override_bytes.is_some() {
                format!("the :{effective}-byte override on field `{field}`")
            } else {
                format!("field `{field}` is {field_size} byte{}", if field_size == 1 { "" } else { "s" })
            };
            self.error(
                span,
                format!("[operand.field-overrun] .{w} access reads {access} bytes but {target}"),
            );
            return;
        }
        // Struct-end bound (rider): a declared overlay may not run past the end
        // of the struct it overlays — even if the instruction width fits it.
        if let Some(ov) = override_bytes {
            if field_offset + ov > struct_size {
                self.error(
                    span,
                    format!(
                        "[operand.field-overrun] :{ov}-byte overlay at field `{field}` (offset {field_offset}) runs {} byte(s) past the end of the {struct_size}-byte struct",
                        field_offset + ov - struct_size,
                    ),
                );
            }
        }
    }

    /// Map a bare (`Plain`) operand expression. A single-segment path names a
    /// register (→ [`CodeOperand::Reg`]) or, failing that, a `.local` / global
    /// symbol (→ [`CodeOperand::Sym`], resolved against `scope`). A MULTI-segment
    /// path is an external label reference `Owner.label` (§5.2, e.g.
    /// `bra.w foo.entry`): join it dot-wise and resolve against `scope` — it is
    /// not a local label, so it passes through as the caller-visible symbol the
    /// defining owner exported. Anything else is evaluated and classified like an
    /// operand splice.
    ///
    /// NOTE — this CHANGED prior behavior (T5): before, a bare multi-segment path
    /// fell through to `eval_expr` / value-path evaluation; now ANY bare path is a
    /// symbol reference. A comptime VALUE path in operand position must be written
    /// as `#expr` (`Operand::Imm`) or a `{splice}` (`Operand::Splice`) — so a
    /// future reader wondering why `move.l some.const, d0` is treated as a symbol
    /// `some.const` rather than that const's value is oriented here.
    fn map_plain(
        &mut self,
        expr: &ast::Expr,
        scope: &LabelScope,
        env: &mut Env,
    ) -> Option<CodeOperand> {
        // D2.33 review M6 (Volence-ratified): a top-level comptime INDEX as a
        // bare operand is FENCED — `move.w Tbl[2], d0` would read memory at
        // the element's VALUE as an absolute address, one typo away from the
        // address arithmetic `Tbl+2` and not classic syntax on an
        // instruction line (tenet 3). The IMMEDIATE form (`#Tbl[2]`) is
        // BLESSED — a pure comptime value, same class as `sizeof(T)`.
        if matches!(expr, ast::Expr::Index { .. }) {
            self.error(
                expr_span(expr),
                "[asm.index-operand] comptime element indexing is not an address operand —                  use `#Tbl[i]` for the element's VALUE as an immediate, or bind the                  address math to a `const` first",
            );
            return None;
        }
        if let ast::Expr::Path(p) = expr {
            if p.segments.len() == 1 {
                let seg = &p.segments[0];
                if let Some(r) = reg_from_name(seg) {
                    return Some(CodeOperand::Reg(r));
                }
                // `sr`/`ccr` are register-class words too (the AS front-end's
                // rule): they win over ordinary names in operand position.
                if seg == "sr" {
                    return Some(CodeOperand::Sr);
                }
                if seg == "ccr" {
                    return Some(CodeOperand::Ccr);
                }
                // S2-D12(e) — the forgotten-`#`: this bare name is about to
                // become an ABSOLUTE ADDRESS. If it is a plain-valued
                // `const`/`equ` in scope, say so
                // (`[operand.const-as-address]`).
                self.check_const_as_address(seg, expr_span(expr));
                return Some(CodeOperand::Sym(scope.resolve_ref(seg)));
            }
            // D-PP.5 — `Item.field` field-ADDRESS operand: a two-segment path
            // whose FIRST segment names a known struct-typed data item (module-
            // local OR a cross-module type-only import) and whose SECOND segment
            // names a field of that struct denotes the FIELD'S ADDRESS. Lower it
            // like the bare symbolic operand but with target `Item + offsetof`
            // (a `SymOff`, which `lower_m68k_abs_sym` turns into an `Add` fixup).
            // ONE field segment only; unknown first segment / >2 segments fall
            // through to today's `Owner.label` link-symbol behavior below.
            if p.segments.len() == 2 {
                let (item, field) = (p.segments[0].as_str(), p.segments[1].as_str());
                if let Some(struct_name) = self.data_item_struct_name(item) {
                    // A known struct-typed item: the field MUST exist (a loud
                    // comptime error naming struct+field otherwise — NOT a silent
                    // link-symbol pass-through). `field_in_struct` reports nothing
                    // on a miss, so name it here.
                    match self.field_in_struct(&struct_name, field, expr_span(expr)) {
                        Some((off, _size)) => {
                            return Some(CodeOperand::SymOff {
                                sym: scope.resolve_ref(item),
                                off,
                            });
                        }
                        None => {
                            self.error(
                                expr_span(expr),
                                format!(
                                    "[operand.unknown-field] struct `{struct_name}` (of `{item}`) has no field `{field}`"
                                ),
                            );
                            return None;
                        }
                    }
                }
            }
            // `Owner.label` — a cross-body reference to an exported label. Join the
            // segments to the `Owner.label` spelling the defining owner emitted.
            return Some(CodeOperand::Sym(scope.resolve_ref(&p.segments.join("."))));
        }
        // A `Sym ± const` sum in operand position denotes the ABSOLUTE address
        // `sym + off` — the bare-label idiom extended with a constant byte
        // offset (`Sprite_Cycle_Counter+1`, the odd byte of a word RAM cell).
        // Route it to `SymOff` so it rides the SAME RelaxAbsSym width-rule seam
        // as a bare symbol (a RAM address widths to abs.w), rather than
        // comptime-folding the link-time symbol and failing "unknown name".
        if let Some(op) = self.sym_off_operand(expr, scope, env) {
            return Some(op);
        }
        let v = self.eval_expr(expr, env);
        self.classify_operand_splice(v, expr_span(expr))
    }

    /// Match `Sym ± const` (either operand order for `+`, symbol-left only for
    /// `-`) where one side is a bare non-register single-segment symbol path (a
    /// link symbol, per the bare-path-is-a-symbol rule) and the other folds to a
    /// comptime integer → a [`CodeOperand::SymOff`] absolute operand with fixup
    /// target `sym + off`. Returns `None` (fall through to the generic path)
    /// unless the shape matches exactly — so pure-const sums and register
    /// arithmetic are untouched.
    fn sym_off_operand(
        &mut self,
        expr: &ast::Expr,
        scope: &LabelScope,
        env: &mut Env,
    ) -> Option<CodeOperand> {
        let ast::Expr::Binary { op, lhs, rhs, .. } = expr else { return None };
        let op = *op;
        if !matches!(op, ast::BinOp::Add | ast::BinOp::Sub) {
            return None;
        }
        // `sym ± const`
        if let Some(seg) = bare_symbol_seg(lhs) {
            let off = self.eval_expr(rhs, env).as_stored_int()?;
            let off = if matches!(op, ast::BinOp::Sub) { -off } else { off };
            return Some(CodeOperand::SymOff { sym: scope.resolve_ref(seg), off });
        }
        // `const + sym` (address commutes only for `+`)
        if matches!(op, ast::BinOp::Add) {
            if let Some(seg) = bare_symbol_seg(rhs) {
                let off = self.eval_expr(lhs, env).as_stored_int()?;
                return Some(CodeOperand::SymOff { sym: scope.resolve_ref(seg), off });
            }
        }
        None
    }

    /// Extract the single address/data register naming an indirect base. Only a
    /// one-part `(An)` form is supported in T3 — indexed/absolute indirects
    /// (`(d,An,Xn)`, `(Label).w`) diagnose as not-yet-supported.
    fn ind_single_reg(
        &mut self,
        parts: &[(ast::Expr, Option<TextOrSplice>)],
        span: Span,
        env: &mut Env,
    ) -> Option<Reg> {
        if parts.len() != 1 {
            self.error(span, "indexed/absolute indirect addressing is not yet supported");
            return None;
        }
        let (e, _psize) = &parts[0];
        if let ast::Expr::Path(p) = e {
            if p.segments.len() == 1 {
                if let Some(r) = reg_from_name(&p.segments[0]) {
                    return Some(r);
                }
            }
        }
        let v = self.eval_expr(e, env);
        match v {
            Value::Reg(r) => Some(r),
            Value::Poison => None,
            other => {
                self.error(
                    expr_span(e),
                    format!("indirect base must be a register, got {}", other.type_name()),
                );
                None
            }
        }
    }

    /// Extract the base register of a `-(An)` / `(An)+` inner operand (an
    /// [`Operand::Ind`]).
    fn inner_ind_reg(&mut self, inner: &Operand, env: &mut Env) -> Option<Reg> {
        match inner {
            Operand::Ind { parts, span, .. } => self.ind_single_reg(parts, *span, env),
            other => {
                self.error(
                    operand_span(other),
                    "pre-decrement / post-increment needs a register-indirect base",
                );
                None
            }
        }
    }

    /// Type a resolved operand-splice value against the operand classes: an
    /// integer → `Imm`, a [`Reg`] → `Reg`, a label ([`Value::FnRef`]/[`Value::Str`])
    /// → `Sym`. Any other kind is `[asm.splice-kind]`. This is THE place operand
    /// classes are decided (used by both `{splice}` operands and evaluated
    /// non-path `Plain` operands).
    fn classify_operand_splice(&mut self, v: Value, span: Span) -> Option<CodeOperand> {
        let is_z80 = self.cpu == Some(sigil_ir::backend::Cpu::Z80);
        match v {
            Value::Poison => None,
            // CPU/kind agreement (T1, §2.2): a 68k register spliced into a Z80
            // section is a loud `[asm.splice-kind]` error — the free correctness
            // win the single-CPU-per-section fact hands us. In a 68k section it
            // is the ordinary register operand (byte-frozen path).
            Value::Reg(r) => {
                if is_z80 {
                    self.splice_kind_err(span, "a Z80 operand (this is a Z80 section)", &Value::Reg(r));
                    None
                } else {
                    Some(CodeOperand::Reg(r))
                }
            }
            // The mirror: a Z80 register (`{r}` in a Z80 template, §3.1) maps to
            // its Z80 operand in a Z80 section; the SAME value in a 68k section
            // is the `[asm.splice-kind]` error. (The producer — a Z80 proc
            // register param — arrives with the rung-2 files; this arm is wired
            // now so it is a one-line addition then, and the 68k-section guard
            // is defense-in-depth today.)
            Value::Z80Reg(rc) => {
                if is_z80 {
                    Some(z80_regclass_operand(rc))
                } else {
                    self.splice_kind_err(
                        span,
                        "a 68k operand (this is a 68000 section)",
                        &Value::Z80Reg(rc),
                    );
                    None
                }
            }
            // A Label param spliced into an operand position (`jsr {p}`,
            // `lea {p}, a1`) produces the same symbol operand as the string form
            // (D-PP.3) — a link-time reference, byte-identical to `jsr {t}` with
            // a `string` param.
            Value::FnRef(n) | Value::Str(n) | Value::Label(n) => Some(CodeOperand::Sym(n)),
            other => {
                if let Some(n) = other.as_stored_int() {
                    Some(CodeOperand::Imm(n))
                } else if self.reject_if_provisional(&other, span).is_some() {
                    // A provisional here() splice gets the SPECIFIC D-H.2
                    // steering message, not the generic `[asm.splice-kind]`.
                    None
                } else {
                    self.splice_kind_err(span, "int, Reg, or Sym", &other);
                    None
                }
            }
        }
    }

    /// Emit the `[asm.splice-kind]` diagnostic (§6.2 `~describe`): name the
    /// expected operand class and the value's `type_name()` for the got side.
    fn splice_kind_err(&mut self, span: Span, expected: &str, got: &Value) {
        self.error(
            span,
            format!("[asm.splice-kind] expected {expected}, got {}", got.type_name()),
        );
    }
}

/// Map a comptime Z80 register class (a `{r}` splice value) to its operand
/// (§3.1). An index register alone in operand position is its 16-bit pair
/// spelling (`ix`/`iy`); `(ix+d)` indexing is a distinct operand shape, not a
/// bare splice.
fn z80_regclass_operand(rc: Z80RegClass) -> CodeOperand {
    match rc {
        Z80RegClass::R8(r) => CodeOperand::Z80Reg8(r),
        Z80RegClass::Pair(p) => CodeOperand::Z80Pair(p),
        Z80RegClass::Index(Z80Index::Ix) => CodeOperand::Z80Pair(Z80Pair::Ix),
        Z80RegClass::Index(Z80Index::Iy) => CodeOperand::Z80Pair(Z80Pair::Iy),
    }
}

/// A literal size-suffix string (`b`/`w`/`l`/`s`) to its [`Width`].
fn width_from_text(t: &str) -> Option<Width> {
    Some(match t {
        "b" => Width::B,
        "w" => Width::W,
        "l" => Width::L,
        "s" => Width::S,
        _ => return None,
    })
}

/// A register name (`d0`..`d7`, `a0`..`a7`) to its [`Reg`], else `None`.
/// Thin alias for [`Reg::from_name`] (the canonical map), kept for the local
/// call sites' brevity.
fn reg_from_name(name: &str) -> Option<Reg> {
    Reg::from_name(name)
}

/// The Z80 control-flow mnemonics whose FIRST operand may be a condition code
/// (`jr z, …` / `jp nz, …` / `call c, …` / `ret nc`). The AS front-end's
/// `control_flow` set for the `i == 0 ⇒ cc` disambiguation (rung-2 §13.3).
fn is_z80_control_flow(mnemonic: &str) -> bool {
    matches!(mnemonic, "jr" | "jp" | "call" | "ret")
}

/// The [`Z80Cond`] a control-flow operand names, or `None` if it is not a bare
/// single-segment condition word (`nz`/`z`/`nc`/`c`/`po`/`pe`/`p`/`m`). A
/// `(hl)` indirect (`jp (hl)`), an address expression (`jp Label`), or a
/// comptime relative (`jr 5`) is NOT a bare cond path — those stay ordinary
/// operands. Only reached for the leading operand of a control-flow mnemonic,
/// so `c` here is unambiguously the carry cc (the C register never leads a
/// control-flow form).
fn z80_cc_operand(op: &Operand) -> Option<Z80Cond> {
    let Operand::Plain { expr: ast::Expr::Path(p), .. } = op else { return None };
    if p.segments.len() != 1 {
        return None;
    }
    Z80Cond::from_name(p.segments[0].as_str())
}

/// The bare-symbol operand a single-segment path denotes: a name that is NOT a
/// register-class word (`d0`..`a7`/`sp`/`sr`/`ccr`). `None` for multi-segment
/// paths and register words. Used to spot the symbol side of a `Sym ± const`
/// absolute-address operand.
fn bare_symbol_seg(e: &ast::Expr) -> Option<&str> {
    let ast::Expr::Path(p) = e else { return None };
    if p.segments.len() != 1 {
        return None;
    }
    let seg = p.segments[0].as_str();
    if reg_from_name(seg).is_some() || seg == "sr" || seg == "ccr" {
        return None;
    }
    Some(seg)
}

/// The register-list bit index of a single [`Reg`] in the CANONICAL `movem`
/// mask (bit0=D0..bit7=D7, bit8=A0..bit15=A7) — mirrors the AS front-end's
/// `reg_list_index`. `sp` is already folded to `Reg::A7` by [`reg_from_name`]
/// before this runs, so it lands at bit 15 like any other `a7` spelling.
fn reg_list_bit(r: Reg) -> u8 {
    match r {
        Reg::D0 => 0,
        Reg::D1 => 1,
        Reg::D2 => 2,
        Reg::D3 => 3,
        Reg::D4 => 4,
        Reg::D5 => 5,
        Reg::D6 => 6,
        Reg::D7 => 7,
        Reg::A0 => 8,
        Reg::A1 => 9,
        Reg::A2 => 10,
        Reg::A3 => 11,
        Reg::A4 => 12,
        Reg::A5 => 13,
        Reg::A6 => 14,
        Reg::A7 => 15,
    }
}

/// If `op` is a `movem` register-list operand (`d0-d7/a0-a6`, `a2/d2`, `d0`,
/// `sp`, a `d`→`a`-crossing range), return its CANONICAL 16-bit mask; else
/// `None` (so the caller can try the OTHER operand / fall back to the memory-EA
/// mapper — this is a total, side-effect-free recognizer, matching the AS
/// front-end's `parse_reg_list` contract).
///
/// A `movem` operand is always parsed as an [`Operand::Plain`] expression (no
/// EA parens) — `(sp)+`/`-(sp)` are the memory side, never the list side — so
/// only that shape is tried. The expression tree groups by ARITHMETIC
/// precedence (`/` binds tighter than `-`), which does NOT match reglist
/// grammar's flat `item (- item)? (/ item (- item)?)*` shape — `d0-d7/a0-a6`
/// parses as `Sub(Sub(d0, Div(d7,a0)), a6)`, not `Div(Sub(d0,d7), Sub(a0,a6))`.
/// [`flatten_reglist_expr`] walks the tree and re-linearizes it into the
/// original left-to-right token sequence before applying the range/union
/// grammar, sidestepping the precedence mismatch entirely.
fn movem_reg_list(op: &Operand) -> Option<u16> {
    let Operand::Plain { expr, size: None, .. } = op else { return None };
    let mut items = Vec::new();
    flatten_reglist_expr(expr, &mut items)?;
    // `items[i].1` says how item `i` is joined to item `i-1`: `Range` means
    // items `i-1..=i` form a `lo-hi` pair (item `i-1` was already added as a
    // plain single register above by the previous iteration — a range OVERWRITES
    // it into the full span, which is a harmless re-set of already-set bits).
    let mut mask: u16 = 0;
    for (idx, (reg, sep)) in items.iter().enumerate() {
        match sep {
            ReglistSep::Range => {
                let (prev_reg, _) = items[idx - 1];
                let lo = reg_list_bit(prev_reg);
                let hi = reg_list_bit(*reg);
                if lo > hi {
                    return None;
                }
                for b in lo..=hi {
                    mask |= 1u16 << b;
                }
            }
            ReglistSep::Union => {
                mask |= 1u16 << reg_list_bit(*reg);
            }
        }
    }
    Some(mask)
}

/// How a reglist item is joined to the PREVIOUS item: `Union` (`/`, the first
/// item's own separator is always `Union` — a leading sentinel, never
/// consumed) or `Range` (`-`, forms a `lo-hi` pair with its predecessor).
#[derive(Clone, Copy, PartialEq)]
enum ReglistSep {
    Union,
    Range,
}

/// Re-linearize a `movem` reglist expression tree back into the FLAT
/// left-to-right `(register, separator-before-it)` token sequence the source
/// text actually wrote, undoing the arithmetic-precedence grouping (`/` binds
/// tighter than `-`) that the general expression parser imposed. Returns
/// `None` if any leaf is not a bare single-segment register-name path (a
/// non-register identifier, a call, a literal, ...) — the caller then knows
/// `op` is not a register list at all (e.g. a `move` operand's real arithmetic
/// expression correctly falls through here).
///
/// The tree is always LEFT-DEEP (the parser's precedence climb is
/// left-associative at each tier): a `Sub`/`Div` node's `lhs` may itself be
/// `Sub`/`Div` (recurse), but `rhs` is a single leaf OR a `Div` chain (from a
/// `/`-group immediately following a name) — never a `Sub`. Walking `lhs`
/// first and `rhs` last, in that order, reproduces the original token order.
fn flatten_reglist_expr(expr: &ast::Expr, out: &mut Vec<(Reg, ReglistSep)>) -> Option<()> {
    match expr {
        ast::Expr::Path(p) if p.segments.len() == 1 => {
            let r = reg_from_name(&p.segments[0])?;
            let sep = if out.is_empty() { ReglistSep::Union } else { ReglistSep::Range };
            out.push((r, sep));
            Some(())
        }
        ast::Expr::Binary { op: BinOp::Sub, lhs, rhs, .. } => {
            flatten_reglist_expr(lhs, out)?;
            flatten_reglist_expr(rhs, out)
        }
        ast::Expr::Binary { op: BinOp::Div, lhs, rhs, .. } => {
            flatten_reglist_expr(lhs, out)?;
            // A name right after `/` starts a fresh union item, not a range
            // continuation, even though it is the `rhs` of a binary node —
            // override the "not-first → Range" default `flatten_reglist_expr`
            // would otherwise assign by re-marking it here.
            let before = out.len();
            flatten_reglist_expr(rhs, out)?;
            if let Some(entry) = out.get_mut(before) {
                entry.1 = ReglistSep::Union;
            }
            Some(())
        }
        _ => None,
    }
}

/// The PC-relative shape of a `Sym(...)` inner operand, if its FIRST part is
/// the literal bareword `pc` (never a valid address register, so this can't
/// misfire on a real `(aN)`/`(aN,Xn)` operand). `Plain` is `Sym(pc)` (one
/// part); `Indexed` is `Sym(pc,Xn[.size])` (two parts), carrying the index
/// part's raw expr + optional size suffix for the caller to resolve. Any other
/// shape (wrong part count, first part not literally `pc`) yields `None` and
/// the caller falls through to ordinary `(An[,Xn])` handling.
///
/// RESERVED-TOKEN consequence (the small-opens doc line, tranche 3): inside
/// operand parentheses `pc` is claimed by this carve-out, so a user symbol
/// literally named `pc` can never be the SOLE inner base — `x(pc)` always
/// means PC-relative, matching AS. Such a symbol still works everywhere
/// else: as a displacement over a real register (`pc(a0)`), in comptime
/// expressions, and as a plain operand.
enum PcRelShape<'a> {
    /// `Sym(pc)` — plain PC-relative, `(d16,PC)`.
    Plain,
    /// `Sym(pc,Xn[.size])` — PC-indexed, `(d8,PC,Xn)`.
    Indexed {
        /// The index operand's raw expression (a register bareword, or
        /// something else entirely if the source got it wrong).
        xn_expr: &'a ast::Expr,
        /// The index's size suffix, if any (`.w`/`.l`; `None` defaults to
        /// `.w`, matching AS).
        xn_size: Option<&'a TextOrSplice>,
    },
}

fn pc_rel_shape(inner: &Operand) -> Option<PcRelShape<'_>> {
    let Operand::Ind { parts, .. } = inner else { return None };
    let is_pc = |e: &ast::Expr| matches!(e, ast::Expr::Path(p) if p.segments == ["pc"]);
    match parts.as_slice() {
        [(disp, None)] if is_pc(disp) => Some(PcRelShape::Plain),
        [(disp, None), (xn_expr, xn_size)] if is_pc(disp) => {
            Some(PcRelShape::Indexed { xn_expr, xn_size: xn_size.as_ref() })
        }
        _ => None,
    }
}

/// The single bare identifier of a displacement expression, if it is exactly a
/// one-segment [`ast::Expr::Path`] (D6.A3/A5). A multi-segment path, a literal,
/// arithmetic, or a call yields `None` — those keep today's comptime-eval
/// semantics (field names participate only as the ENTIRE displacement).
/// A path segment that spells a register (`a0`) is NOT a field name; excluding
/// it keeps `a0(a0)` on the comptime path where it errors as today.
/// The field-space facts `check_field_overrun` needs, bundled (the field's own
/// size + offset, the containing struct's size, and any `:size` overlay override).
struct FieldAccessCheck<'a> {
    field: &'a str,
    field_size: i128,
    field_offset: i128,
    struct_size: i128,
    override_bytes: Option<i128>,
}

fn single_segment_field(disp: &ast::Expr) -> Option<&str> {
    if let ast::Expr::Path(p) = disp {
        if p.segments.len() == 1 && reg_from_name(&p.segments[0]).is_none() {
            return Some(&p.segments[0]);
        }
    }
    None
}

/// The `(qualifier, field)` of a displacement expression that is exactly a
/// TWO-segment [`ast::Expr::Path`] (`Qual.field`, D6.A4). A path of any other
/// arity yields `None`. The caller decides whether `qualifier` names an overlay
/// or struct (field space) or falls through to comptime eval (`offsets` ordinal,
/// dotted const, …) — this helper only splits the two segments.
fn two_segment_field(disp: &ast::Expr) -> Option<(&str, &str)> {
    if let ast::Expr::Path(p) = disp {
        if p.segments.len() == 2 {
            return Some((&p.segments[0], &p.segments[1]));
        }
    }
    None
}

/// Reverse a parsed instruction [`Operand`] back into a call-argument
/// [`ast::Arg`] for a bare statement call (D-PP.1). The parser parsed the line
/// as an instruction (it cannot know it is a call until lowering resolves the
/// mnemonic), so the arguments arrived as operands; this un-does the operand
/// normalization for the shapes a comptime call argument can legitimately take:
///
///  - `Plain { expr, size: None }` — a bare expression: a register (`d0`), an
///    enum path (`Ani.Shoot`), an int/arith (`1 + 2`), a call, a struct literal.
///  - `DispInd { disp: Path(p), inner: Ind{ parts, .. } }` — the parser folds an
///    all-positional call `inner(2)` into displacement-indexed addressing
///    (parser.rs `operand`); reverse it to the `Expr::Call` it came from so a
///    NESTED call argument (`outer inner(2), d0`) round-trips.
///
/// Any addressing-mode-only shape (`#imm`, `(a0)`, `-(a7)`, `(a0)+`, a
/// size-suffixed operand, a `{splice}`) is NOT a valid call argument → `None`,
/// which the caller turns into a clear diagnostic. The reconstructed `Arg` is
/// always positional.
///
/// NAMED bare-call arguments (D-PP.4) were investigated and NOT built: the
/// operand grammar has no shape for `name: expr`. `operand()` (parser.rs) hits
/// the bareword `name`, parses it as a `Plain` expression, and then the
/// trailing `:` fails `expect_line_end_or_rbrace` — a genuine (and already
/// LOUD, pre-existing) parse error, not a silent misparse. Teaching the
/// operand grammar to accept a trailing `:` after a bare ident is exactly the
/// token shape `.name:` (a local label definition, parsed at the STATEMENT
/// level, ahead of `instr_line`) already claims one dot away from — adding it
/// to `operand()` too would need a lookahead rule to keep the two apart, for
/// no real gain: the tranche's only named-arg call site (`spawn(...)`) is
/// already paren-form. Decision: named args are PAREN-FORM ONLY (see
/// `bind_args` in `eval/call.rs`); the bare spelling stays positional-only,
/// and `f name: v` keeps its existing loud parse error unchanged (pinned by
/// `bare_form_named_looking_arg_is_a_loud_parse_error` in `tests/bare_calls.rs`).
fn operand_to_arg(op: &Operand) -> Option<ast::Arg> {
    let value = match op {
        Operand::Plain { expr, size: None, .. } => expr.clone(),
        Operand::DispInd { disp: ast::Expr::Path(callee), inner, span, .. } => {
            // Only the folded-call shape reverses: a `(parts)` indirect with no
            // per-part or trailing size (an actual displacement `4(a0)` carries a
            // register base and IS an addressing mode, not a call).
            let Operand::Ind { parts, size: None, .. } = inner.as_ref() else { return None };
            let mut cargs = Vec::with_capacity(parts.len());
            for (e, psize) in parts {
                if psize.is_some() {
                    return None;
                }
                cargs.push(ast::Arg { name: None, value: e.clone(), span: expr_span(e) });
            }
            ast::Expr::Call { callee: callee.clone(), args: cargs, span: *span }
        }
        _ => return None,
    };
    let span = expr_span(&value);
    Some(ast::Arg { name: None, value, span })
}

/// Whether a parsed operand is a bare data/address register (`dn`/`an`) — the
/// only `assert` `src` form v1 accepts (spec §5). A register parses as a
/// single-segment `Operand::Plain { Path([reg]) }` with no size suffix; any
/// addressing mode (`(a0)`, `#imm`, displacement) or a non-register bareword is
/// rejected so the desugar never pushes a non-register comparand.
fn operand_is_register(op: &Operand) -> bool {
    let Operand::Plain { expr: ast::Expr::Path(p), size: None, .. } = op else { return false };
    p.segments.len() == 1 && reg_from_name(&p.segments[0]).is_some()
}

/// The span of an operand, for diagnostics on the inner-operand paths.
fn operand_span(op: &Operand) -> Span {
    match op {
        Operand::Imm(e) => expr_span(e),
        Operand::PreDec(inner) | Operand::PostInc(inner) => operand_span(inner),
        Operand::Ind { span, .. }
        | Operand::DispInd { span, .. }
        | Operand::Plain { span, .. } => *span,
        Operand::Splice(e) => expr_span(e),
    }
}
