//! Comptime evaluator scaffold (Spec 2, Plan 2): the lexical [`Env`] and the
//! [`Evaluator`] state. Expression evaluation, const resolution, control flow,
//! builtins, and lambda parsing arrive in Tasks 2–6; this module only provides
//! the environment, the evaluator's bookkeeping, and a stub [`eval_const`].
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

/// The comptime evaluator's mutable state, threaded through evaluation.
pub struct Evaluator {
    /// Diagnostics collected during evaluation.
    pub diags: Vec<Diagnostic>,
    /// Steps consumed so far, capped by [`STEP_BUDGET`].
    pub steps: u64,
    /// The active call stack as `(fn name, call-site span)`, for budget and
    /// recursion-cycle reporting in later tasks.
    pub call_stack: Vec<(String, Span)>,
}

impl Evaluator {
    /// Create a fresh evaluator with an empty diagnostic list and step count.
    pub fn new() -> Self {
        Evaluator { diags: Vec::new(), steps: 0, call_stack: Vec::new() }
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
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// Evaluate the `const` item named `name` in `file` to a comptime [`Value`].
///
/// STUB: real const evaluation is Task 3. For now this only wires the module
/// entry point: it locates a matching `const` item and returns `(None, diags)`.
// TODO(Task 3): resolve the const's value expression via the evaluator and
// return `(Some(value), diags)`.
pub fn eval_const(file: &crate::ast::File, name: &str) -> (Option<Value>, Vec<Diagnostic>) {
    let mut ev = Evaluator::new();
    let _found = file.items.iter().any(|item| {
        matches!(item, crate::ast::Item::Const(c) if c.name == name)
    });
    // TODO(Task 3): actually evaluate `_found`'s value expression.
    (None, std::mem::take(&mut ev.diags))
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
    fn eval_const_stub_returns_none() {
        let (v, diags) = crate::eval::eval_const(&empty_file(), "MISSING");
        assert!(v.is_none());
        assert!(diags.is_empty());
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
