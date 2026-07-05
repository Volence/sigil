//! Comptime evaluator (Spec 2, Plan 2): the lexical [`Env`] and the
//! [`Evaluator`] state, split across focused (private) submodules —
//! `env` (the scope chain), `expr` (pure expression evaluation),
//! `control` (statement execution / control flow), `call` (`comptime fn`
//! calls and applying callables), `builtins` (the §6.8 builtin methods),
//! and `guards` (`ensure`/`ensure_fatal` and string interpolation).
//!
//! This module (`mod.rs`) owns the [`Evaluator`] struct itself, its
//! constructors, and the crate's top-level entry points ([`eval_const`]);
//! the submodules contribute method groups via additional `impl Evaluator`
//! blocks.
mod builtins;
mod call;
mod control;
mod env;
mod expr;
mod guards;

pub use env::{AssignError, Binding, Env};

use crate::ast;
use crate::value::Value;
use sigil_span::{Diagnostic, Level, Span};
use std::collections::HashMap;

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
