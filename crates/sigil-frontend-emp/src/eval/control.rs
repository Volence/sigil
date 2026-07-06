//! Statement execution and control flow (T4/T5): the block executor
//! (`exec_stmts`), scope helpers, `if`/`else`, `for`/`while`, `comptime`
//! blocks, and `comptime var`/assignment.
use super::{AssignError, Env, Evaluator, Flow};
use crate::ast;
use crate::value::Value;
use sigil_span::Span;

impl<'a> Evaluator<'a> {
    /// Eval `expr`; if it left a pending return, surface it as `Err(Flow::Return)`
    /// so the calling stmt arm can bail. Centralizes the check EVERY statement arm
    /// that evaluates an operand must perform (this is the invariant that prevents
    /// the call-arg return-leak class of bug).
    fn eval_operand(&mut self, expr: &ast::Expr, env: &mut Env) -> Result<Value, Flow> {
        let v = self.eval_expr(expr, env);
        match self.pending_return.take() {
            Some(r) => Err(Flow::Return(r)),
            None => Ok(v),
        }
    }

    /// Execute a statement block in order in `env`'s *current* scope, returning
    /// a [`Flow`]: `Normal(v)` if the block fell off its end (with `v` the
    /// trailing value), or `Return(v)` the moment an explicit `return` — or a
    /// `return` inside a nested `if` — fires.
    ///
    /// The block's trailing value is the value of its final statement iff that
    /// statement is a bare expression, else [`Value::Unit`]. Explicit `return`
    /// is the primary idiom; trailing-expression is the fallback.
    ///
    /// Statements deferred to T5 (`for`/`while`/`comptime` blocks, `comptime var`,
    /// assignment, `patch`, `bind`) are no-ops here so the executor stays total;
    /// their semantics land with control flow in the next task.
    pub(super) fn exec_stmts(&mut self, stmts: &[ast::Stmt], env: &mut Env) -> Flow {
        if self.aborted {
            return Flow::Normal(Value::Poison);
        }
        let mut last = Value::Unit;
        for stmt in stmts {
            if self.aborted {
                return Flow::Normal(Value::Poison);
            }
            match stmt {
                ast::Stmt::Let { name, value, .. } => {
                    let v = match self.eval_operand(value, env) {
                        Ok(v) => v,
                        Err(f) => return f,
                    };
                    env.define(name.clone(), v, false);
                    last = Value::Unit;
                }
                ast::Stmt::LetTuple { names, value, span } => {
                    let v = match self.eval_operand(value, env) {
                        Ok(v) => v,
                        Err(f) => return f,
                    };
                    self.bind_tuple(names, v, *span, env);
                    last = Value::Unit;
                }
                ast::Stmt::Return { value, .. } => {
                    let v = match value {
                        // A `return` nested in the returned expression wins (it
                        // fired first); `eval_operand` surfaces it as `Err`.
                        Some(e) => match self.eval_operand(e, env) {
                            Ok(v) => v,
                            Err(f) => return f,
                        },
                        None => Value::Unit,
                    };
                    return Flow::Return(v);
                }
                ast::Stmt::Expr(e) => {
                    last = match self.eval_operand(e, env) {
                        Ok(v) => v,
                        Err(f) => return f,
                    };
                }
                ast::Stmt::If(e) => {
                    // Statement-position `if`: run it, propagate any `return`,
                    // and (like all non-expression statements) contribute no
                    // trailing value.
                    if let ast::Expr::If { cond, then, els, .. } = e {
                        match self.eval_if(cond, then, els.as_deref(), env) {
                            Flow::Return(v) => return Flow::Return(v),
                            Flow::Normal(_) => {}
                        }
                    }
                    last = Value::Unit;
                }
                ast::Stmt::Var { name, value, span, .. } => {
                    // Evaluate the initializer first (a nested `return` wins and
                    // bails before we bind or diagnose).
                    let v = match self.eval_operand(value, env) {
                        Ok(v) => v,
                        Err(f) => return f,
                    };
                    // `comptime var` needs a comptime-mutable context (D-P2.5).
                    // Outside one it is an error — but we still bind it (mutable)
                    // so later references/assignments don't cascade extra
                    // unknown-name/immutable diagnostics off the one real error.
                    if self.comptime_ctx == 0 {
                        self.error(
                            *span,
                            "comptime var is only allowed inside a comptime block or comptime fn body",
                        );
                    }
                    env.define(name.clone(), v, true);
                    last = Value::Unit;
                }
                ast::Stmt::Assign { target, value, span } => {
                    let v = match self.eval_operand(value, env) {
                        Ok(v) => v,
                        Err(f) => return f,
                    };
                    // Field assignment (`a.b = ..`) is Plan 3+; only a plain
                    // single-segment target is assignable here.
                    if target.segments.len() > 1 {
                        self.error(*span, "field assignment not yet supported");
                    } else {
                        let name = target.segments[0].as_str();
                        match env.assign(name, v) {
                            Ok(()) => {}
                            Err(AssignError::NotFound) => {
                                self.error(*span, format!("cannot assign to unbound name `{name}`"));
                            }
                            Err(AssignError::Immutable) => self.error(
                                *span,
                                format!(
                                    "cannot assign to immutable binding `{name}` (declared with `let`)"
                                ),
                            ),
                        }
                    }
                    last = Value::Unit;
                }
                ast::Stmt::ComptimeBlock { body, .. } => {
                    // A nested comptime block is its own scope and comptime
                    // context: a `comptime var` declared inside is dead at the
                    // closing brace (the scope pop drops it). The block is a
                    // side-effect statement — it yields Unit — but an inner
                    // `return` still propagates out to the enclosing fn.
                    if let Flow::Return(v) = self.exec_comptime_scoped(body, env) {
                        return Flow::Return(v);
                    }
                    last = Value::Unit;
                }
                ast::Stmt::While { cond, body, span } => {
                    match self.eval_while(cond, body, *span, env) {
                        Flow::Return(v) => return Flow::Return(v),
                        Flow::Normal(_) => {}
                    }
                    last = Value::Unit;
                }
                ast::Stmt::For(e) => {
                    // A `for` at statement position runs for its side effects
                    // (mutating comptime vars); its Array value is discarded.
                    // `eval_operand` surfaces any body/iter `return` as `Err`.
                    match self.eval_operand(e, env) {
                        Ok(_) => {}
                        Err(f) => return f,
                    }
                    last = Value::Unit;
                }
                // `patch` / `bind` (§6.4, D-P4.10): the slot + back-patch
                // mechanism lives in `lower::patch::PatchTable` (T5). These
                // statements stay no-ops in the Core-free evaluator because the
                // surface does not yet give a comptime body a section-emission
                // position to route them into — wiring `Stmt::Patch`/`Stmt::Bind`
                // to a live section table is a T6/T7 surface-integration concern
                // (see the `lower::patch` module doc).
                ast::Stmt::Patch { .. } | ast::Stmt::Bind { .. } => {
                    last = Value::Unit;
                }
            }
        }
        Flow::Normal(last)
    }

    /// Execute `body` in a fresh nested scope, returning its [`Flow`]. Pushes a
    /// scope, runs the block, then pops — centralizing the push/exec/pop idiom so
    /// the scope is always dropped, including on the `Return` path.
    fn exec_scoped(&mut self, body: &[ast::Stmt], env: &mut Env) -> Flow {
        env.push_scope();
        let f = self.exec_stmts(body, env);
        env.pop_scope();
        f
    }

    /// Like [`exec_scoped`](Self::exec_scoped) but also enters a comptime-mutable
    /// context (D-P2.5) for the duration, so `comptime var`/assignment are legal
    /// inside `body`. Folding the depth bump into this helper keeps it balanced:
    /// there is no path between the increment and its matching decrement, so a
    /// future early return in the body cannot leave `comptime_ctx` unbalanced.
    pub(super) fn exec_comptime_scoped(&mut self, body: &[ast::Stmt], env: &mut Env) -> Flow {
        self.comptime_ctx += 1;
        let f = self.exec_scoped(body, env);
        self.comptime_ctx -= 1;
        f
    }

    /// Evaluate an `if` in either statement or expression position (D-P2.15).
    ///
    /// The condition must be `Bool`; a non-bool is an error (yielding `Poison`)
    /// and a `Poison` condition propagates silently. The taken branch runs in a
    /// fresh nested scope and its [`Flow`] (including a `Return`) is returned
    /// as-is; a false condition with no `else` yields `Normal(Unit)`.
    pub(super) fn eval_if(
        &mut self,
        cond: &ast::Expr,
        then: &[ast::Stmt],
        els: Option<&[ast::Stmt]>,
        env: &mut Env,
    ) -> Flow {
        if self.aborted {
            return Flow::Normal(Value::Poison);
        }
        // A `return` fired while evaluating the condition itself — propagate it.
        let c = match self.eval_operand(cond, env) {
            Ok(v) => v,
            Err(f) => return f,
        };
        match c {
            Value::Poison => Flow::Normal(Value::Poison),
            Value::Bool(true) => self.exec_scoped(then, env),
            Value::Bool(false) => match els {
                Some(e) => self.exec_scoped(e, env),
                None => Flow::Normal(Value::Unit),
            },
            other => {
                self.error(
                    crate::parser::expr_span(cond),
                    format!("if condition must be bool, got {}", other.type_name()),
                );
                Flow::Normal(Value::Poison)
            }
        }
    }

    /// Evaluate a `for var in iter { body }` expression (D-P2.6, §6.8): iterate
    /// `iter`, running `body` in a fresh scope per element with `var` bound, and
    /// collect each iteration's value into an [`Array`](Value::Array).
    ///
    /// `iter` must be a [`Range`](Value::Range) (half-open `lo..hi`) or an
    /// [`Array`](Value::Array); any other type is an error yielding `Poison`.
    /// One step is charged per iteration, so even a huge range stays bounded by
    /// [`super::STEP_BUDGET`]. A `return` inside the body stops the loop and is
    /// stashed in `pending_return` so the enclosing `exec_stmts` turns it into a
    /// fn-level [`Flow::Return`]; a `Poison` iterable propagates silently.
    pub(super) fn eval_for(
        &mut self,
        var: &str,
        iter: &ast::Expr,
        body: &[ast::Stmt],
        span: Span,
        env: &mut Env,
    ) -> Value {
        // `eval_expr`'s top guard guarantees no pending return on entry; a
        // return fired *while evaluating `iter`* leaves one set, so bail.
        let iter_v = self.eval_expr(iter, env);
        if self.aborted || self.pending_return.is_some() {
            return Value::Poison;
        }
        // One element stream for both iterables. `Range` stays lazy — it is
        // never materialized into a `Vec` — so a huge range costs no memory and
        // is bounded purely by the per-iteration step budget below.
        let items: Box<dyn Iterator<Item = Value>> = match iter_v {
            Value::Range { lo, hi } => Box::new((lo..hi).map(Value::Int)),
            Value::Array(elems) => Box::new(elems.into_iter()),
            Value::Poison => return Value::Poison,
            other => {
                self.error(
                    crate::parser::expr_span(iter),
                    format!("for expects a range or array, got {}", other.type_name()),
                );
                return Value::Poison;
            }
        };
        let mut collected = Vec::new();
        for elem in items {
            if !self.bump_step() {
                self.abort(span, "step budget exceeded");
                return Value::Poison;
            }
            match self.run_loop_body(var, elem, body, env) {
                Flow::Normal(v) => collected.push(v),
                Flow::Return(r) => {
                    // Stash the body's return so the enclosing `exec_stmts`
                    // surfaces it as a fn-level `Flow::Return`.
                    self.pending_return = Some(r);
                    return Value::Poison;
                }
            }
            if self.aborted {
                return Value::Poison;
            }
        }
        Value::Array(collected)
    }

    /// Run one `for` iteration: bind `var` to `elem` (immutably) in a fresh
    /// scope, then run `body` via [`exec_scoped`](Self::exec_scoped). The loop
    /// variable lives only for this iteration (dropped when the scope pops), and
    /// the body's own locals are dropped by `exec_scoped`. The [`Flow`] —
    /// including a `Return` — is returned so the caller can collect the value or
    /// propagate the return.
    fn run_loop_body(
        &mut self,
        var: &str,
        elem: Value,
        body: &[ast::Stmt],
        env: &mut Env,
    ) -> Flow {
        env.push_scope();
        env.define(var.to_string(), elem, false);
        let f = self.exec_scoped(body, env);
        env.pop_scope();
        f
    }

    /// Evaluate a `while cond { body }` statement (D-P2.6): repeatedly run `body`
    /// (in a fresh scope) while `cond` is `Bool(true)`, yielding `Normal(Unit)`.
    ///
    /// A step is charged per iteration so an otherwise-infinite loop is bounded
    /// by [`super::STEP_BUDGET`] and aborts rather than hanging. A non-bool
    /// condition is an error that stops the loop; a `Poison` condition stops
    /// silently. A `return` in the body (or surfaced from the condition)
    /// propagates outward.
    fn eval_while(
        &mut self,
        cond: &ast::Expr,
        body: &[ast::Stmt],
        span: Span,
        env: &mut Env,
    ) -> Flow {
        loop {
            if self.aborted {
                return Flow::Normal(Value::Poison);
            }
            if !self.bump_step() {
                self.abort(span, "step budget exceeded");
                return Flow::Normal(Value::Poison);
            }
            let c = match self.eval_operand(cond, env) {
                Ok(v) => v,
                Err(f) => return f,
            };
            match c {
                Value::Bool(true) => {
                    if let Flow::Return(v) = self.exec_scoped(body, env) {
                        return Flow::Return(v);
                    }
                }
                Value::Bool(false) => return Flow::Normal(Value::Unit),
                // A poisoned condition already reported its own error upstream.
                Value::Poison => return Flow::Normal(Value::Unit),
                other => {
                    self.error(
                        crate::parser::expr_span(cond),
                        format!("while condition must be bool, got {}", other.type_name()),
                    );
                    return Flow::Normal(Value::Unit);
                }
            }
        }
    }

    /// Bind a tuple-destructuring `let (a, b, ...) = e`. The value must be a
    /// [`Value::Tuple`] whose arity matches `names`; a mismatch (wrong arity or
    /// non-tuple) is an error and every name is bound to `Poison` so downstream
    /// use suppresses. A `Poison` value propagates silently (no new diagnostic).
    fn bind_tuple(&mut self, names: &[String], value: Value, span: Span, env: &mut Env) {
        match value {
            Value::Tuple(elems) if elems.len() == names.len() => {
                for (n, e) in names.iter().zip(elems) {
                    env.define(n.clone(), e, false);
                }
                return;
            }
            Value::Poison => {}
            ref other => {
                let got = match other {
                    Value::Tuple(elems) => format!("{}-tuple", elems.len()),
                    v => v.type_name().to_string(),
                };
                self.error(
                    span,
                    format!("expected a {}-tuple to destructure, got {got}", names.len()),
                );
            }
        }
        for n in names {
            env.define(n.clone(), Value::Poison, false);
        }
    }
}
