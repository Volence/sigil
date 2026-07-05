//! Comptime evaluator scaffold (Spec 2, Plan 2): the lexical [`Env`] and the
//! [`Evaluator`] state. Expression evaluation, const resolution, control flow,
//! builtins, and lambda parsing arrive in Tasks 2–6; this module only provides
//! the environment, the evaluator's bookkeeping, and a stub [`eval_const`].
use crate::ast::{self, BinOp, UnOp};
use crate::value::Value;
use sigil_span::{Diagnostic, Level, Span};
use std::collections::HashMap;

/// Why an [`Env::assign`] failed, so the caller can phrase the right
/// diagnostic (wording is a later task's concern).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignError {
    /// No binding of that name exists in any enclosing scope.
    NotFound,
    /// The nearest binding exists but is immutable (`let`, not `comptime var`).
    Immutable,
}

/// A single name binding within a scope.
#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
    /// The bound value.
    pub value: Value,
    /// Whether the binding may be reassigned (`comptime var` vs `let`).
    pub mutable: bool,
}

/// A lexical scope chain: a stack of scopes, innermost last.
///
/// Represented as `Vec<HashMap<String, Binding>>`. Cloning deep-copies every
/// scope, so a clone is fully independent of the original — a lambda that
/// captures an `Env` snapshots it by value and is unaffected by later mutation
/// of the defining scope.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Env {
    scopes: Vec<HashMap<String, Binding>>,
}

impl Env {
    /// Create a fresh environment with a single (global) scope.
    pub fn new() -> Self {
        Env { scopes: vec![HashMap::new()] }
    }

    /// Push a new innermost scope.
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the innermost scope, discarding its bindings. No-op if only the
    /// global scope remains (the chain is never left empty).
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Bind `name` to `value` in the innermost scope, shadowing any outer
    /// binding of the same name. Re-defining a name already in the innermost
    /// scope overwrites it.
    pub fn define(&mut self, name: impl Into<String>, value: Value, mutable: bool) {
        let scope = self.scopes.last_mut().expect("env always has a scope");
        scope.insert(name.into(), Binding { value, mutable });
    }

    /// Look up `name`, searching innermost scope outward. Returns the nearest
    /// binding's value, or `None` if unbound.
    pub fn lookup(&self, name: &str) -> Option<&Value> {
        self.scopes.iter().rev().find_map(|s| s.get(name)).map(|b| &b.value)
    }

    /// Assign `value` to the nearest existing binding of `name`.
    ///
    /// Returns [`AssignError`] if `name` is unbound or its binding is
    /// immutable; the caller decides how to phrase the diagnostic. On success
    /// the binding's value is replaced.
    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), AssignError> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(binding) = scope.get_mut(name) {
                if !binding.mutable {
                    return Err(AssignError::Immutable);
                }
                binding.value = value;
                return Ok(());
            }
        }
        Err(AssignError::NotFound)
    }
}

/// Comptime step budget (D-P2.7): a coarse upper bound on evaluation work,
/// guarding against runaway loops/recursion. Later tasks act on exhaustion.
pub const STEP_BUDGET: u64 = 5_000_000;

/// Maximum comptime-fn call depth (D-P2.16). A hard bound below the native
/// stack limit so unbounded recursion is caught and *named* (see
/// [`Evaluator::abort`]) instead of overflowing the process stack.
pub const MAX_CALL_DEPTH: usize = 512;

/// How many innermost call-stack frames an abort message names before it
/// truncates with a leading `...` (keeps a deep repeated chain readable).
const MAX_CHAIN_FRAMES: usize = 12;

/// A control-flow signal threaded out of [`Evaluator::exec_stmts`].
///
/// A statement block either falls off its end (`Normal`, carrying the block's
/// value — the last bare expression statement's value, or `Unit`) or hits an
/// explicit `return` (`Return`, carrying the returned value, which stops the
/// block and bubbles up to the enclosing `comptime fn` boundary).
enum Flow {
    /// The block ran to completion; the payload is its trailing value.
    Normal(Value),
    /// An explicit `return` fired; the payload is the returned value.
    Return(Value),
}

/// The comptime evaluator's mutable state, threaded through evaluation.
///
/// The `'a` lifetime ties the evaluator to a borrowed [`ast::File`]'s items:
/// [`Evaluator::with_file`] indexes the file's `const` and `enum` decls so bare
/// names and `Enum.Variant` paths resolve to them. [`Value`] carries no lifetime
/// (a [`Value::Lambda`] owns its body), so borrowing the file here is free of
/// friction with the `&mut self` mutation during evaluation — the borrowed
/// index is a distinct object from the mutated `diags`/memo.
///
/// [`Evaluator::new`] builds the empty-program evaluator (no file): in that mode
/// there are no file consts/enums, so unknown names still error. This keeps the
/// T2 pure-expression tests working unchanged.
pub struct Evaluator<'a> {
    /// Diagnostics collected during evaluation.
    pub diags: Vec<Diagnostic>,
    /// Steps consumed so far, capped by [`STEP_BUDGET`].
    pub steps: u64,
    /// The active call stack as `(fn name, call-site span)`, for budget and
    /// recursion-cycle reporting in later tasks.
    pub call_stack: Vec<(String, Span)>,
    /// File-level `const` decls, indexed by name (empty in the no-file mode).
    consts: HashMap<&'a str, &'a ast::ConstDecl>,
    /// File-level `enum` decls, indexed by name (empty in the no-file mode).
    enums: HashMap<&'a str, &'a ast::EnumDecl>,
    /// File-level `comptime fn` decls, indexed by name (empty in no-file mode).
    fns: HashMap<&'a str, &'a ast::ComptimeFnDecl>,
    /// Set once a hard limit (step budget or call depth) is hit (D-P2.16). While
    /// set, [`eval_expr`](Evaluator::eval_expr) / [`exec_stmts`](Evaluator::exec_stmts)
    /// short-circuit to `Poison` so evaluation unwinds without further work or
    /// diagnostics.
    aborted: bool,
    /// A `return` that fired inside an *expression-position* `if` and must still
    /// exit the enclosing fn. `eval_expr` sets it; the next `exec_stmts` step
    /// picks it up and turns it into a [`Flow::Return`]. (Statement-position
    /// `return`/`if` never need this — they flow through `exec_stmts` directly.)
    ///
    /// INVARIANT: every statement arm that evaluates an operand MUST route it
    /// through [`eval_operand`](Evaluator::eval_operand) and bail on
    /// `Err(Flow::Return)`. Bypassing that check lets a caller's pending return
    /// leak into a callee (the call-arg return-leak bug class).
    pending_return: Option<Value>,
    /// Depth of enclosing comptime-mutable contexts (D-P2.5). A `comptime var`
    /// and its reassignment are only legal where this is non-zero: inside a
    /// `comptime fn` body (bumped in [`eval_call`](Evaluator::eval_call)) or a
    /// nested `comptime block { }` (bumped in the [`Stmt::ComptimeBlock`] arm).
    /// Module-level `const` value expressions run with `comptime_ctx == 0`, so
    /// they have no mutable state.
    comptime_ctx: u32,
    /// Memoized const values, keyed by const name. A `Poison` entry records a
    /// const that already failed (cycle or error) so the failure does not
    /// re-report on subsequent references.
    const_memo: HashMap<String, Value>,
    /// The names of consts whose value expressions are currently being
    /// evaluated, in reference order — the in-progress stack used to detect and
    /// name cyclic const definitions.
    in_progress: Vec<String>,
}

impl<'a> Evaluator<'a> {
    /// Create a fresh evaluator with no file context: an empty diagnostic list,
    /// step count, and const/enum index. Bare names resolve only against the
    /// local [`Env`]; there are no file-level consts or enums to fall back to.
    pub fn new() -> Self {
        Evaluator {
            diags: Vec::new(),
            steps: 0,
            call_stack: Vec::new(),
            consts: HashMap::new(),
            enums: HashMap::new(),
            fns: HashMap::new(),
            aborted: false,
            pending_return: None,
            comptime_ctx: 0,
            const_memo: HashMap::new(),
            in_progress: Vec::new(),
        }
    }

    /// Create an evaluator that can resolve names against `file`'s top-level
    /// `const` and `enum` items. Later duplicate names (a parse-level concern)
    /// are resolved last-wins by the index build; duplicate diagnosis is not
    /// this task's job.
    pub fn with_file(file: &'a ast::File) -> Self {
        let mut ev = Evaluator::new();
        for item in &file.items {
            match item {
                ast::Item::Const(c) => {
                    ev.consts.insert(c.name.as_str(), c);
                }
                ast::Item::Enum(e) => {
                    ev.enums.insert(e.name.as_str(), e);
                }
                ast::Item::ComptimeFn(f) => {
                    ev.fns.insert(f.name.as_str(), f);
                }
                _ => {}
            }
        }
        ev
    }

    /// Push an [`Error`](Level::Error) diagnostic at `span`.
    pub fn error(&mut self, span: Span, msg: impl Into<String>) {
        self.diags.push(Diagnostic { level: Level::Error, message: msg.into(), primary: span });
    }

    /// Charge one evaluation step. Returns `false` once [`STEP_BUDGET`] is
    /// exceeded so callers can bail out; keeps counting otherwise.
    pub fn bump_step(&mut self) -> bool {
        self.steps += 1;
        self.steps <= STEP_BUDGET
    }

    /// Evaluate a pure `.emp` expression to a comptime [`Value`] (T2).
    ///
    /// Charges one step per node. On any type or arithmetic error it emits a
    /// diagnostic and returns [`Value::Poison`]; per D-P2.9, operating on an
    /// already-[`Poison`](Value::Poison) operand yields `Poison` *silently*, so
    /// one bad subexpression never fans out into a cascade of diagnostics.
    ///
    /// `Call`, user-struct `StructLit`, `If`, `For`, and `Asm` are handled by
    /// later tasks (T4–T6); here they return `Poison` without a diagnostic.
    pub fn eval_expr(&mut self, expr: &ast::Expr, env: &mut Env) -> Value {
        // Once evaluation has aborted (D-P2.16) or a `return` is pending out of an
        // expression-position `if`, short-circuit so the tree unwinds silently.
        if self.aborted || self.pending_return.is_some() {
            return Value::Poison;
        }
        if !self.bump_step() {
            self.abort(crate::parser::expr_span(expr), "step budget exceeded");
            return Value::Poison;
        }
        match expr {
            // Int literals are `i64` in the AST; widen to the `i128` comptime
            // domain (D-P2.13).
            ast::Expr::Int(n, _) => Value::Int(i128::from(*n)),
            ast::Expr::Float(x, _) => Value::Float(*x),
            ast::Expr::Str(s, _) => Value::Str(s.clone()),
            ast::Expr::Path(path) => self.eval_path(path, env),
            ast::Expr::Unary { op, expr, span } => {
                let v = self.eval_expr(expr, env);
                self.eval_unary(*op, v, *span)
            }
            ast::Expr::Binary { op, lhs, rhs, span } => {
                self.eval_binary(*op, lhs, rhs, *span, env)
            }
            ast::Expr::Range { lo, hi, span } => self.eval_range(lo, hi, *span, env),
            ast::Expr::ArrayLit { elems, .. } => {
                // Poison elements are preserved as-is (no extra diagnostics).
                Value::Array(elems.iter().map(|e| self.eval_expr(e, env)).collect())
            }
            ast::Expr::TupleLit { elems, .. } => {
                Value::Tuple(elems.iter().map(|e| self.eval_expr(e, env)).collect())
            }
            ast::Expr::Call { callee, args, span } => self.eval_call(callee, args, *span, env),
            ast::Expr::StructLit { ty, fields, .. } => self.eval_struct_lit(ty, fields, env),
            ast::Expr::If { cond, then, els, .. } => {
                // As an expression, an `if` yields its chosen branch's value. If
                // that branch hit `return`, stash it in `pending_return` so the
                // enclosing `exec_stmts` turns it into a fn-level `Flow::Return`.
                match self.eval_if(cond, then, els.as_deref(), env) {
                    Flow::Normal(v) => v,
                    Flow::Return(v) => {
                        self.pending_return = Some(v.clone());
                        v
                    }
                }
            }
            ast::Expr::For { var, iter, body, span } => {
                self.eval_for(var, iter, body, *span, env)
            }
            // TODO(Plan 3/4): `asm { }` lowers to a `Code` value.
            ast::Expr::Asm { .. } => Value::Poison,
        }
    }

    /// Resolve a path expression: the boolean/`none` keywords; a single name
    /// (local `Env` binding, then a file-level `const`); or a two-segment
    /// `Enum.Variant` path. Local bindings shadow file consts. Unknown names,
    /// and `Enum.Variant` for a known enum with no such variant, are errors.
    fn eval_path(&mut self, path: &ast::Path, env: &Env) -> Value {
        if path.segments.len() == 1 {
            return match path.segments[0].as_str() {
                // Booleans are single-segment paths (there is no `Expr::Bool`).
                "true" => Value::Bool(true),
                "false" => Value::Bool(false),
                // `none` maps to Unit for now; revisit if a later task
                // introduces a first-class Option value.
                "none" => Value::Unit,
                name => {
                    // Local bindings shadow file-level consts: check `env` first.
                    if let Some(v) = env.lookup(name) {
                        return v.clone();
                    }
                    // Then fall back to a file-level const of the same name.
                    if self.consts.contains_key(name) {
                        return self.resolve_const(name, path.span);
                    }
                    self.error(path.span, format!("unknown name `{name}`"));
                    Value::Poison
                }
            };
        }
        // A two-segment `Enum.Variant` path resolves to a nullary enum value
        // when `Enum` names a file-level enum. Payload-carrying construction
        // (`Enum.Variant(x)`) parses as a `Call`, not a plain path.
        if path.segments.len() == 2 {
            let (ty, variant) = (path.segments[0].as_str(), path.segments[1].as_str());
            if let Some(decl) = self.enums.get(ty) {
                if decl.variants.iter().any(|(v, _, _)| v == variant) {
                    return Value::Enum {
                        ty_name: ty.to_string(),
                        variant: variant.to_string(),
                        payload: vec![],
                    };
                }
                self.error(path.span, format!("enum `{ty}` has no variant `{variant}`"));
                return Value::Poison;
            }
        }
        // Any other multi-segment path (module paths, unknown enums) is an
        // unknown name for now; later plans resolve `use`d/module paths.
        let full = path.segments.join(".");
        self.error(path.span, format!("unknown name `{full}`"));
        Value::Poison
    }

    /// Build a struct value from a written literal (D-P2.14, value level only):
    /// evaluate each field in order and tag the value with the type's last path
    /// segment. No existence/field/size/default checks — those are Plan 3.
    fn eval_struct_lit(
        &mut self,
        ty: &ast::Path,
        fields: &[(String, ast::Expr)],
        env: &mut Env,
    ) -> Value {
        let ty_name = ty.segments.last().cloned().unwrap_or_default();
        // Poison field values are preserved as-is (propagate, no new diagnostic).
        let fields =
            fields.iter().map(|(name, e)| (name.clone(), self.eval_expr(e, env))).collect();
        Value::Struct { ty_name, fields }
    }

    // ---- statement execution / control flow (T4) ---------------------------

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
    fn exec_stmts(&mut self, stmts: &[ast::Stmt], env: &mut Env) -> Flow {
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
                // TODO(Plan 4): `patch` / `bind`. No-op for now (kept total).
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
    fn exec_comptime_scoped(&mut self, body: &[ast::Stmt], env: &mut Env) -> Flow {
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
    fn eval_if(
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
    /// [`STEP_BUDGET`]. A `return` inside the body stops the loop and is stashed
    /// in `pending_return` so the enclosing `exec_stmts` turns it into a
    /// fn-level [`Flow::Return`]; a `Poison` iterable propagates silently.
    fn eval_for(
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
    /// by [`STEP_BUDGET`] and aborts rather than hanging. A non-bool condition is
    /// an error that stops the loop; a `Poison` condition stops silently. A
    /// `return` in the body (or surfaced from the condition) propagates outward.
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

    /// Evaluate a call expression. A single-segment callee names a `comptime fn`;
    /// an unknown single name is an error. A multi-segment callee is a
    /// method-style receiver call (`xs.map(..)`) handled by T6 — returned as
    /// silent `Poison` here.
    fn eval_call(&mut self, callee: &ast::Path, args: &[ast::Arg], span: Span, env: &mut Env) -> Value {
        // Multi-segment callee: a `recv.method(..)` builtin or enum constructor,
        // both of which arrive in T6. Silently poison for now (no diagnostic).
        if callee.segments.len() != 1 {
            return Value::Poison;
        }
        let name = callee.segments[0].as_str();
        // Copy the `&'a` decl out of the index so its body/params are borrowed
        // from the file, leaving `self` free to mutate across the body eval.
        let decl: &'a ast::ComptimeFnDecl = match self.fns.get(name).copied() {
            Some(d) => d,
            None => {
                self.error(span, format!("unknown function `{name}`"));
                return Value::Poison;
            }
        };
        // Bind arguments (evaluated in the caller's env) to a positional slot
        // vector aligned with the params.
        let bound = self.bind_args(decl, args, span, env);
        if self.aborted {
            return Value::Poison;
        }
        // A `return` fired inside an argument expression (e.g. `f(if c { return 7 })`)
        // belongs to the *caller*, not the callee. Bail before running the callee
        // body so the enclosing `exec_stmts` arm takes `pending_return` and yields
        // the caller's `Flow::Return`; otherwise the callee's first statement would
        // steal it.
        if self.pending_return.is_some() {
            return Value::Poison;
        }
        // Recursion / stack safety (D-P2.16): bound the depth *before* recursing
        // so runaway recursion is named, not a native stack overflow.
        if self.call_stack.len() >= MAX_CALL_DEPTH {
            self.abort(span, "recursion too deep");
            return Value::Poison;
        }
        if !self.bump_step() {
            self.abort(span, "step budget exceeded");
            return Value::Poison;
        }
        self.call_stack.push((name.to_string(), span));
        // Comptime fns are pure: a fresh env, seeing only their params (and, via
        // `self`, file consts/fns) — never the caller's locals.
        let mut fenv = Env::new();
        for ((pname, _, _), v) in decl.params.iter().zip(bound) {
            fenv.define(pname.clone(), v, false);
        }
        // A comptime-fn body IS a comptime-mutable context (D-P2.5): `comptime
        // var` and reassignment are legal inside it. `exec_comptime_scoped`
        // enters (and always restores) that context around the body.
        let flow = self.exec_comptime_scoped(&decl.body, &mut fenv);
        self.call_stack.pop();
        match flow {
            Flow::Return(v) | Flow::Normal(v) => v,
        }
    }

    /// Bind call `args` to `decl`'s parameters, returning a value per parameter
    /// (in parameter order), `Poison`-filled where an argument is missing or a
    /// binding error occurred — so a single clear diagnostic is emitted and the
    /// call still proceeds without a crash.
    ///
    /// Positional args fill parameters left-to-right by position; named args fill
    /// the parameter of that name. Errors: an unknown named parameter, a
    /// parameter filled twice (positionally then by name, or twice by name), a
    /// positional arg past the last parameter (`too many arguments`), and any
    /// parameter left unfilled (`missing argument`).
    fn bind_args(
        &mut self,
        decl: &ast::ComptimeFnDecl,
        args: &[ast::Arg],
        span: Span,
        env: &mut Env,
    ) -> Vec<Value> {
        let n = decl.params.len();
        let mut slots: Vec<Option<Value>> = vec![None; n];
        let mut pos = 0usize;
        for arg in args {
            // A `return` fired in an earlier arg (its value belongs to the
            // caller) or an abort — stop binding so we don't pile spurious
            // arity diagnostics onto the real event. The caller discards these
            // bindings.
            if self.aborted || self.pending_return.is_some() {
                break;
            }
            let v = self.eval_expr(&arg.value, env);
            match &arg.name {
                None => {
                    if pos >= n {
                        self.error(arg.span, "too many arguments");
                    } else if slots[pos].is_some() {
                        let pname = &decl.params[pos].0;
                        self.error(
                            arg.span,
                            format!("parameter `{pname}` given more than once"),
                        );
                        pos += 1;
                    } else {
                        slots[pos] = Some(v);
                        pos += 1;
                    }
                }
                Some(pname) => match decl.params.iter().position(|(p, _, _)| p == pname) {
                    None => {
                        self.error(arg.span, format!("unknown named parameter `{pname}`"));
                    }
                    Some(idx) => {
                        if slots[idx].is_some() {
                            self.error(
                                arg.span,
                                format!("parameter `{pname}` given more than once"),
                            );
                        } else {
                            slots[idx] = Some(v);
                        }
                    }
                },
            }
        }
        // If a return/abort interrupted arg binding, the slots are incomplete by
        // design; skip missing-arg reporting (spurious) — the caller discards
        // this result anyway.
        if self.aborted || self.pending_return.is_some() {
            return vec![Value::Poison; n];
        }
        slots
            .into_iter()
            .enumerate()
            .map(|(i, s)| match s {
                Some(v) => v,
                None => {
                    let pname = &decl.params[i].0;
                    self.error(span, format!("missing argument `{pname}`"));
                    Value::Poison
                }
            })
            .collect()
    }

    /// Abort evaluation on a hard limit (step budget or call depth, D-P2.16).
    ///
    /// Sets the [`aborted`](Self::aborted) flag (so all in-flight evaluation
    /// short-circuits and unwinds) and emits *one* error naming the active call
    /// chain — the innermost non-terminating callees, not an opaque quota. Only
    /// the first abort reports; later triggers during unwinding are ignored.
    fn abort(&mut self, span: Span, reason: &str) {
        if self.aborted {
            return;
        }
        self.aborted = true;
        let names: Vec<&str> = self.call_stack.iter().map(|(n, _)| n.as_str()).collect();
        // Keep the message bounded when a deep chain repeats the same callee:
        // show only the innermost `MAX_CHAIN_FRAMES`, prefixed with `...`.
        let chain = if names.len() > MAX_CHAIN_FRAMES {
            format!("... -> {}", names[names.len() - MAX_CHAIN_FRAMES..].join(" -> "))
        } else {
            names.join(" -> ")
        };
        let msg = if chain.is_empty() {
            reason.to_string()
        } else {
            format!("{reason}: in {chain}")
        };
        self.error(span, msg);
    }

    /// Resolve the file-level const named `name`, evaluating it lazily and
    /// memoizing the result. `ref_span` is the reference site, used to locate a
    /// cyclic-definition error.
    ///
    /// - A memoized value (including a memoized `Poison`) is returned directly.
    /// - If `name` is already on the in-progress stack, this reference closes a
    ///   cycle: report `cyclic const definition: <chain>` at `ref_span`, memoize
    ///   `Poison` for `name` so the cascade suppresses, and return `Poison`.
    /// - Otherwise push `name`, evaluate its value expr in a fresh global-only
    ///   env (consts see each other only by name, never each other's locals),
    ///   pop, memoize, and return.
    ///
    /// Callers must only invoke this for a `name` known to be in `self.consts`.
    fn resolve_const(&mut self, name: &str, ref_span: Span) -> Value {
        if let Some(v) = self.const_memo.get(name) {
            return v.clone();
        }
        if let Some(start) = self.in_progress.iter().position(|n| n == name) {
            // Name the cycle as the chain from where it was first entered back
            // to this repeated reference, e.g. `A -> B -> A`.
            let mut chain: Vec<&str> = self.in_progress[start..].iter().map(|s| s.as_str()).collect();
            chain.push(name);
            self.error(ref_span, format!("cyclic const definition: {}", chain.join(" -> ")));
            self.const_memo.insert(name.to_string(), Value::Poison);
            return Value::Poison;
        }
        // Copy the `&'a ConstDecl` out of the index so its `value` expr is
        // borrowed from the file (lifetime `'a`), not from `self`. That leaves
        // `self` free to be mutated (diags/memo/in_progress) across the
        // recursive `eval_expr` below.
        let decl: &'a ast::ConstDecl =
            self.consts.get(name).copied().expect("caller ensures the const exists");
        self.in_progress.push(name.to_string());
        let mut env = Env::new();
        let v = self.eval_expr(&decl.value, &mut env);
        self.in_progress.pop();
        self.const_memo.insert(name.to_string(), v.clone());
        v
    }

    /// Apply a unary operator (D-P2.3). A `Poison` operand propagates silently.
    fn eval_unary(&mut self, op: UnOp, v: Value, span: Span) -> Value {
        if matches!(v, Value::Poison) {
            return Value::Poison;
        }
        match op {
            UnOp::Neg => match v {
                // Checked negation: `i128::MIN` has no positive counterpart, so
                // negating it is a comptime overflow error (D-P2.1).
                Value::Int(n) => match n.checked_neg() {
                    Some(r) => Value::Int(r),
                    None => self.arith_overflow(span, "-"),
                },
                Value::Float(x) => Value::Float(-x),
                other => self.operand_type_error(span, "-", &other),
            },
            UnOp::Not => match v {
                Value::Bool(b) => Value::Bool(!b),
                other => self.operand_type_error(span, "!", &other),
            },
            UnOp::BitNot => match v {
                Value::Int(n) => Value::Int(!n),
                other => self.operand_type_error(span, "~", &other),
            },
        }
    }

    /// Evaluate a binary operation. Short-circuiting `&&`/`||` are dispatched
    /// before either operand's poison state is consulted so the RHS is not
    /// evaluated needlessly.
    fn eval_binary(
        &mut self,
        op: BinOp,
        lhs_e: &ast::Expr,
        rhs_e: &ast::Expr,
        span: Span,
        env: &mut Env,
    ) -> Value {
        if matches!(op, BinOp::And | BinOp::Or) {
            return self.eval_logical(op, lhs_e, rhs_e, span, env);
        }
        let lhs = self.eval_expr(lhs_e, env);
        let rhs = self.eval_expr(rhs_e, env);
        // D-P2.9: poison in either operand yields poison with no new diagnostic.
        if matches!(lhs, Value::Poison) || matches!(rhs, Value::Poison) {
            return Value::Poison;
        }
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                self.eval_arith(op, lhs, rhs, span)
            }
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => self.eval_bitwise(op, lhs, rhs, span),
            BinOp::Shl | BinOp::Shr => self.eval_shift(op, lhs, rhs, span),
            BinOp::Eq | BinOp::Ne => self.eval_equality(op, &lhs, &rhs),
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => self.eval_ordering(op, lhs, rhs, span),
            BinOp::Concat => self.eval_concat(lhs, rhs, span),
            // Short-circuit operators were dispatched above.
            BinOp::And | BinOp::Or => unreachable!("logical ops handled by eval_logical"),
        }
    }

    /// Arithmetic `+ - * / %` (D-P2.3). `Int op Int` stays an exact `Int` and
    /// overflow is an error (D-P2.1, never a wrap); a `Float` on either side
    /// promotes the other to `f64`.
    fn eval_arith(&mut self, op: BinOp, lhs: Value, rhs: Value, span: Span) -> Value {
        if let (Value::Int(a), Value::Int(b)) = (&lhs, &rhs) {
            let (a, b) = (*a, *b);
            let checked = match op {
                BinOp::Add => a.checked_add(b),
                BinOp::Sub => a.checked_sub(b),
                BinOp::Mul => a.checked_mul(b),
                BinOp::Div => {
                    if b == 0 {
                        self.error(span, "division by zero");
                        return Value::Poison;
                    }
                    // `checked_div` also catches `i128::MIN / -1` (overflow).
                    // Integer `/` truncates toward zero (Rust semantics — this
                    // matches AS for the non-negative constants in practice).
                    a.checked_div(b)
                }
                BinOp::Mod => {
                    if b == 0 {
                        self.error(span, "modulo by zero");
                        return Value::Poison;
                    }
                    // `%` is the remainder, taking the sign of the dividend.
                    a.checked_rem(b)
                }
                _ => unreachable!("non-arithmetic op in eval_arith"),
            };
            return match checked {
                Some(v) => Value::Int(v),
                None => self.arith_overflow(span, binop_symbol(op)),
            };
        }
        // Mixed Int/Float or Float/Float: promote to f64.
        match (num_f64(&lhs), num_f64(&rhs)) {
            (Some(a), Some(b)) => {
                let r = match op {
                    BinOp::Add => a + b,
                    BinOp::Sub => a - b,
                    BinOp::Mul => a * b,
                    BinOp::Div => a / b,
                    // Float `%` is f64 remainder; rarely used at comptime.
                    BinOp::Mod => a % b,
                    _ => unreachable!("non-arithmetic op in eval_arith"),
                };
                Value::Float(r)
            }
            _ => self.binop_type_error(span, binop_symbol(op), &lhs, &rhs),
        }
    }

    /// Bitwise `& | ^` — defined only on `Int op Int`.
    fn eval_bitwise(&mut self, op: BinOp, lhs: Value, rhs: Value, span: Span) -> Value {
        match (&lhs, &rhs) {
            (Value::Int(a), Value::Int(b)) => {
                let r = match op {
                    BinOp::BitAnd => a & b,
                    BinOp::BitOr => a | b,
                    BinOp::BitXor => a ^ b,
                    _ => unreachable!("non-bitwise op in eval_bitwise"),
                };
                Value::Int(r)
            }
            _ => self.binop_type_error(span, binop_symbol(op), &lhs, &rhs),
        }
    }

    /// Shifts `<< >>` on `Int op Int`. The shift amount must be in `[0, 128)`
    /// (i128 is 128 bits); a left shift that loses the sign/high bits is an
    /// overflow error (D-P2.1). `>>` is arithmetic (sign-extending), matching
    /// AS behavior on signed comptime values.
    fn eval_shift(&mut self, op: BinOp, lhs: Value, rhs: Value, span: Span) -> Value {
        let (Value::Int(a), Value::Int(b)) = (&lhs, &rhs) else {
            return self.binop_type_error(span, binop_symbol(op), &lhs, &rhs);
        };
        let (a, b) = (*a, *b);
        if !(0..128).contains(&b) {
            self.error(span, format!("shift amount out of range: {b}"));
            return Value::Poison;
        }
        let n = b as u32;
        match op {
            BinOp::Shl => match a.checked_shl(n) {
                // `checked_shl` only validates the shift amount (already
                // guarded), not value overflow — verify the shift round-trips.
                Some(r) if (r >> n) == a => Value::Int(r),
                _ => self.arith_overflow(span, "<<"),
            },
            // Shift amount is guarded to `< 128`, so `>>` cannot overflow.
            BinOp::Shr => Value::Int(a >> n),
            _ => unreachable!("non-shift op in eval_shift"),
        }
    }

    /// Structural equality `== !=` (D-P2.3), always yielding a `Bool`. Numeric
    /// `Int`/`Float` compare by value; distinct non-numeric kinds are simply
    /// not equal (so `==` is total and never spuriously errors — genuine type
    /// mismatches are the type checker's job in a later plan).
    fn eval_equality(&self, op: BinOp, lhs: &Value, rhs: &Value) -> Value {
        let eq = values_equal(lhs, rhs);
        Value::Bool(if op == BinOp::Ne { !eq } else { eq })
    }

    /// Ordering `< <= > >=` (D-P2.3): numeric (`Int`/`Float`, with promotion)
    /// or lexicographic on `Str`. Any other operand kinds are a type error.
    fn eval_ordering(&mut self, op: BinOp, lhs: Value, rhs: Value, span: Span) -> Value {
        use std::cmp::Ordering;
        let ord = match (&lhs, &rhs) {
            (Value::Str(a), Value::Str(b)) => a.cmp(b),
            _ => match (num_f64(&lhs), num_f64(&rhs)) {
                // NaN is unordered: every comparison against it is false.
                (Some(a), Some(b)) => match a.partial_cmp(&b) {
                    Some(o) => o,
                    None => return Value::Bool(false),
                },
                _ => return self.binop_type_error(span, binop_symbol(op), &lhs, &rhs),
            },
        };
        let res = match op {
            BinOp::Lt => ord == Ordering::Less,
            BinOp::Le => ord != Ordering::Greater,
            BinOp::Gt => ord == Ordering::Greater,
            BinOp::Ge => ord != Ordering::Less,
            _ => unreachable!("non-ordering op in eval_ordering"),
        };
        Value::Bool(res)
    }

    /// Short-circuiting `&&`/`||`. The LHS must be `Bool`; the RHS is evaluated
    /// only when the result is not already determined (so a guarding/erroring
    /// RHS is skipped). A `Poison` operand propagates silently.
    fn eval_logical(
        &mut self,
        op: BinOp,
        lhs_e: &ast::Expr,
        rhs_e: &ast::Expr,
        span: Span,
        env: &mut Env,
    ) -> Value {
        let lhs = self.eval_expr(lhs_e, env);
        if matches!(lhs, Value::Poison) {
            return Value::Poison;
        }
        let lb = match lhs {
            Value::Bool(b) => b,
            other => return self.operand_type_error(span, binop_symbol(op), &other),
        };
        match op {
            BinOp::And if !lb => return Value::Bool(false),
            BinOp::Or if lb => return Value::Bool(true),
            _ => {}
        }
        let rhs = self.eval_expr(rhs_e, env);
        if matches!(rhs, Value::Poison) {
            return Value::Poison;
        }
        match rhs {
            Value::Bool(b) => Value::Bool(b),
            other => self.operand_type_error(span, binop_symbol(op), &other),
        }
    }

    /// Concatenation `++` (D-P2.4): `Str ++ Str` or `Array ++ Array` only.
    fn eval_concat(&mut self, lhs: Value, rhs: Value, span: Span) -> Value {
        match (lhs, rhs) {
            (Value::Str(mut a), Value::Str(b)) => {
                a.push_str(&b);
                Value::Str(a)
            }
            (Value::Array(mut a), Value::Array(b)) => {
                a.extend(b);
                Value::Array(a)
            }
            (a, b) => self.binop_type_error(span, "++", &a, &b),
        }
    }

    /// A half-open `lo..hi` range; both bounds must be `Int`.
    fn eval_range(&mut self, lo: &ast::Expr, hi: &ast::Expr, span: Span, env: &mut Env) -> Value {
        let lo_v = self.eval_expr(lo, env);
        let hi_v = self.eval_expr(hi, env);
        if matches!(lo_v, Value::Poison) || matches!(hi_v, Value::Poison) {
            return Value::Poison;
        }
        match (lo_v, hi_v) {
            // An empty/negative range (`lo >= hi`) is allowed here; whether it
            // iterates to nothing is decided when the range is consumed.
            (Value::Int(lo), Value::Int(hi)) => Value::Range { lo, hi },
            (l, h) => {
                self.error(
                    span,
                    format!("range bounds must be int, got {} and {}", l.type_name(), h.type_name()),
                );
                Value::Poison
            }
        }
    }

    // ---- diagnostic helpers ------------------------------------------------

    /// Report an integer-overflow error for operator `sym` and return `Poison`.
    fn arith_overflow(&mut self, span: Span, sym: &str) -> Value {
        self.error(span, format!("integer overflow in `{sym}`"));
        Value::Poison
    }

    /// Report a type error naming a single operand and return `Poison`. Used
    /// for unary operators and for a single offending operand of a logical op.
    fn operand_type_error(&mut self, span: Span, sym: &str, operand: &Value) -> Value {
        self.error(span, format!("`{sym}` not defined for {}", operand.type_name()));
        Value::Poison
    }

    /// Report a binary type error and return `Poison`.
    fn binop_type_error(&mut self, span: Span, sym: &str, lhs: &Value, rhs: &Value) -> Value {
        self.error(
            span,
            format!("`{sym}` not defined for {} and {}", lhs.type_name(), rhs.type_name()),
        );
        Value::Poison
    }
}

/// Coerce a numeric value to `f64` for mixed Int/Float promotion; `None` for
/// non-numeric kinds.
fn num_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Int(n) => Some(*n as f64),
        Value::Float(x) => Some(*x),
        _ => None,
    }
}

/// Structural value equality with numeric `Int`/`Float` promotion at the top
/// level. Distinct kinds are unequal; same-kind aggregates use the derived
/// structural `PartialEq` (nested numbers are *not* cross-promoted).
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Int(x), Value::Float(y)) | (Value::Float(y), Value::Int(x)) => (*x as f64) == *y,
        _ => a == b,
    }
}

/// The source spelling of a binary operator, for diagnostics.
fn binop_symbol(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::Concat => "++",
    }
}

impl Default for Evaluator<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// Evaluate the top-level `const` named `name` in `file` to a comptime
/// [`Value`], returning it alongside every diagnostic emitted.
///
/// If no const of that name exists, returns `(None, [error])` reporting
/// `no const named `<name>``. Otherwise resolution is lazy and memoized: the
/// named const's value expression is evaluated, resolving referenced consts on
/// demand and detecting cyclic definitions (which yield [`Value::Poison`] plus a
/// diagnostic naming the cycle). A successful evaluation returns
/// `(Some(value), diags)` — `diags` may still be non-empty if the value
/// contains a reported error (its `Poison` is surfaced as `Some(Poison)`).
pub fn eval_const(file: &crate::ast::File, name: &str) -> (Option<Value>, Vec<Diagnostic>) {
    // Run on a dedicated thread with a large stack so the native call stack has
    // headroom for [`MAX_CALL_DEPTH`] comptime frames (D-P2.16): the depth bound,
    // not a native stack overflow, is what stops runaway recursion. A scoped
    // thread lets the closure borrow `file`/`name` without a `'static` bound.
    // (A per-call thread is cheap enough at comptime; a future task may hoist it
    // to one evaluator-owned worker.)
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .stack_size(EVAL_STACK_BYTES)
            .spawn_scoped(scope, || eval_const_inner(file, name))
            .expect("failed to spawn comptime evaluation thread");
        match handle.join() {
            Ok(v) => v,
            // Re-raise the original panic on the caller's thread so its payload,
            // message, and backtrace are preserved rather than flattened.
            Err(payload) => std::panic::resume_unwind(payload),
        }
    })
}

/// Stack size for the comptime-evaluation thread (see [`eval_const`]). Sized to
/// comfortably hold [`MAX_CALL_DEPTH`] comptime frames even in unoptimized
/// debug builds, where per-frame stack usage is large.
const EVAL_STACK_BYTES: usize = 64 * 1024 * 1024;

/// The body of [`eval_const`], run on the large-stack evaluation thread.
fn eval_const_inner(file: &crate::ast::File, name: &str) -> (Option<Value>, Vec<Diagnostic>) {
    let mut ev = Evaluator::with_file(file);
    if !ev.consts.contains_key(name) {
        // Anchor the error at the module header — there is no const span to use.
        ev.error(file.module.span, format!("no const named `{name}`"));
        return (None, ev.diags);
    }
    let value = ev.resolve_const(name, file.module.span);
    (Some(value), ev.diags)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn i(n: i128) -> Value {
        Value::Int(n)
    }

    #[test]
    fn define_and_lookup() {
        let mut env = Env::new();
        env.define("x", i(1), false);
        assert_eq!(env.lookup("x"), Some(&i(1)));
    }

    #[test]
    fn lookup_undefined_is_none() {
        let env = Env::new();
        assert_eq!(env.lookup("nope"), None);
    }

    #[test]
    fn inner_scope_shadows_outer() {
        let mut env = Env::new();
        env.define("x", i(1), false);
        env.push_scope();
        env.define("x", i(2), false);
        assert_eq!(env.lookup("x"), Some(&i(2)));
    }

    #[test]
    fn pop_scope_restores_outer_binding() {
        let mut env = Env::new();
        env.define("x", i(1), false);
        env.push_scope();
        env.define("x", i(2), false);
        env.pop_scope();
        assert_eq!(env.lookup("x"), Some(&i(1)));
    }

    #[test]
    fn pop_scope_never_empties_chain() {
        let mut env = Env::new();
        env.define("g", i(1), false);
        // Extra pops past the global scope are harmless no-ops.
        env.pop_scope();
        env.pop_scope();
        assert_eq!(env.lookup("g"), Some(&i(1)));
    }

    #[test]
    fn assign_mutable_updates() {
        let mut env = Env::new();
        env.define("x", i(1), true);
        assert!(env.assign("x", i(9)).is_ok());
        assert_eq!(env.lookup("x"), Some(&i(9)));
    }

    #[test]
    fn assign_immutable_errs() {
        let mut env = Env::new();
        env.define("x", i(1), false);
        assert_eq!(env.assign("x", i(9)), Err(AssignError::Immutable));
        assert_eq!(env.lookup("x"), Some(&i(1)));
    }

    #[test]
    fn assign_undefined_errs() {
        let mut env = Env::new();
        assert_eq!(env.assign("nope", i(1)), Err(AssignError::NotFound));
    }

    #[test]
    fn assign_targets_nearest_binding() {
        let mut env = Env::new();
        env.define("x", i(1), true);
        env.push_scope();
        env.define("x", i(2), true);
        assert!(env.assign("x", i(3)).is_ok());
        assert_eq!(env.lookup("x"), Some(&i(3)));
        env.pop_scope();
        // The outer binding is untouched.
        assert_eq!(env.lookup("x"), Some(&i(1)));
    }

    #[test]
    fn clone_is_independent() {
        let mut env = Env::new();
        env.define("x", i(1), true);
        let mut cloned = env.clone();
        cloned.assign("x", i(99)).unwrap();
        // Mutating the clone does not affect the original (deep-copy clone).
        assert_eq!(env.lookup("x"), Some(&i(1)));
        assert_eq!(cloned.lookup("x"), Some(&i(99)));
    }

    #[test]
    fn evaluator_error_collects_diagnostic() {
        let mut ev = Evaluator::new();
        let span = Span { source: sigil_span::SourceId(0), start: 1, end: 2 };
        ev.error(span, "boom");
        assert_eq!(ev.diags.len(), 1);
        assert_eq!(ev.diags[0].level, Level::Error);
        assert_eq!(ev.diags[0].message, "boom");
    }

    #[test]
    fn bump_step_reports_budget_exhaustion() {
        let mut ev = Evaluator::new();
        assert!(ev.bump_step());
        ev.steps = STEP_BUDGET - 1;
        // The step that reaches exactly the budget is still allowed...
        assert!(ev.bump_step());
        assert_eq!(ev.steps, STEP_BUDGET);
        // ...the next one exceeds it.
        assert!(!ev.bump_step());
    }

    #[test]
    fn eval_const_missing_reports_error() {
        let (v, diags) = crate::eval::eval_const(&empty_file(), "MISSING");
        assert!(v.is_none());
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("no const named `MISSING`"));
    }

    #[test]
    fn comptime_var_outside_context_is_diagnosed_but_still_bound() {
        // `comptime var` at `comptime_ctx == 0` (module/const level) is illegal.
        // Surface syntax can't reach this (a `comptime var` only parses inside a
        // fn/comptime-block body, which bump the context), so drive `exec_stmts`
        // directly to prove the guard fires — and that the name is still bound
        // (mutable) so downstream references don't cascade.
        let mut ev = Evaluator::new();
        let mut env = Env::new();
        let span = Span { source: sigil_span::SourceId(0), start: 0, end: 0 };
        let stmts = vec![ast::Stmt::Var {
            name: "x".to_string(),
            ty: None,
            value: ast::Expr::Int(7, span),
            span,
        }];
        assert_eq!(ev.comptime_ctx, 0);
        let _ = ev.exec_stmts(&stmts, &mut env);
        assert!(
            ev.diags.iter().any(|d| d.message.contains("comptime var is only allowed")),
            "diagnostics were {:?}",
            ev.diags
        );
        assert_eq!(env.lookup("x"), Some(&i(7)));
    }

    fn empty_file() -> crate::ast::File {
        use crate::ast::*;
        let span = Span { source: sigil_span::SourceId(0), start: 0, end: 0 };
        File {
            module: ModuleDecl {
                path: Path { segments: vec!["m".into()], span },
                in_section: None,
                span,
            },
            attrs: vec![],
            items: vec![],
        }
    }
}
