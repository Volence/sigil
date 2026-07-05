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
        self.bump_step();
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
            // TODO(T4): evaluate comptime-fn / builtin calls.
            ast::Expr::Call { .. } => Value::Poison,
            // TODO(T4/T6): construct user structs.
            ast::Expr::StructLit { .. } => Value::Poison,
            // TODO(T5): control flow.
            ast::Expr::If { .. } => Value::Poison,
            ast::Expr::For { .. } => Value::Poison,
            // TODO(Plan 3/4): `asm { }` lowers to a `Code` value.
            ast::Expr::Asm { .. } => Value::Poison,
        }
    }

    /// Resolve a path expression: the boolean/`none` keywords, then an `Env`
    /// lookup; unknown names are an error.
    fn eval_path(&mut self, path: &ast::Path, env: &Env) -> Value {
        if path.segments.len() == 1 {
            match path.segments[0].as_str() {
                // Booleans are single-segment paths (there is no `Expr::Bool`).
                "true" => return Value::Bool(true),
                "false" => return Value::Bool(false),
                // `none` maps to Unit for now; revisit if a later task
                // introduces a first-class Option value.
                "none" => return Value::Unit,
                name => {
                    if let Some(v) = env.lookup(name) {
                        return v.clone();
                    }
                    // TODO(T3): fall back to resolving file-level consts before
                    // reporting the name as unknown.
                    self.error(path.span, format!("unknown name `{name}`"));
                    return Value::Poison;
                }
            }
        }
        // TODO(T3): resolve multi-segment paths (module/enum paths). For now
        // any such path is reported as an unknown name.
        let full = path.segments.join(".");
        self.error(path.span, format!("unknown name `{full}`"));
        Value::Poison
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
                other => self.unop_type_error(span, "-", &other),
            },
            UnOp::Not => match v {
                Value::Bool(b) => Value::Bool(!b),
                other => self.unop_type_error(span, "!", &other),
            },
            UnOp::BitNot => match v {
                Value::Int(n) => Value::Int(!n),
                other => self.unop_type_error(span, "~", &other),
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
    fn eval_equality(&mut self, op: BinOp, lhs: &Value, rhs: &Value) -> Value {
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
            other => return self.binop_lhs_type_error(span, binop_symbol(op), &other),
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
            other => self.binop_lhs_type_error(span, binop_symbol(op), &other),
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

    /// Report a unary type error and return `Poison`.
    fn unop_type_error(&mut self, span: Span, sym: &str, operand: &Value) -> Value {
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

    /// Report a type error naming a single (LHS) operand and return `Poison`.
    fn binop_lhs_type_error(&mut self, span: Span, sym: &str, operand: &Value) -> Value {
        self.error(span, format!("`{sym}` not defined for {}", operand.type_name()));
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
