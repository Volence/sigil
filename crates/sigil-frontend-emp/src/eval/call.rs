//! `comptime fn` calls (T4/T6): dispatch of a call expression, positional/
//! named argument binding, the recursion/step-budget/call-stack machinery,
//! and applying a first-class callable ([`Value::Lambda`]/[`Value::FnRef`]).
use super::builtins::is_builtin;
use super::{Env, Evaluator, Flow, MAX_CALL_DEPTH};
use crate::ast;
use crate::value::Value;
use sigil_span::Span;

impl<'a> Evaluator<'a> {
    /// Evaluate a call expression. Dispatch order (D-P2.10): if the callee's
    /// last segment is a §6.8 builtin (`len`/`map`/`filter`/`fold`/`find`/
    /// `slice`/`val`), it is a builtin method call — builtins are *not*
    /// user-shadowable, so this is checked before any user fn. Otherwise a
    /// single-segment callee names a `comptime fn`; an unknown single name is an
    /// error, and any other multi-segment callee (e.g. an enum payload
    /// constructor, a later plan) is a silent `Poison`.
    pub(super) fn eval_call(&mut self, callee: &ast::Path, args: &[ast::Arg], span: Span, env: &mut Env) -> Value {
        // Guards (`ensure`/`ensure_fatal`, §6.5) are special calls, not user fns:
        // they are the only calls whose message string is `{interp}`-interpolated
        // (D-P2.19), and a passing guard skips its message entirely. Handled ahead
        // of the fn/builtin dispatch so they cannot be shadowed.
        if callee.segments.len() == 1 {
            match callee.segments[0].as_str() {
                "ensure" => return self.eval_guard(false, args, span, env),
                "ensure_fatal" => return self.eval_guard(true, args, span, env),
                _ => {}
            }
        }
        // Builtins win over user fns and are the only method-form (`a.b(..)`)
        // calls handled here.
        if let Some(method) = callee.segments.last() {
            if is_builtin(method) {
                return self.eval_builtin_call(callee, method.clone(), args, span, env);
            }
        }
        // Non-builtin, non-single-segment callee: an enum payload constructor or
        // module path, both later plans. Silently poison for now (no diagnostic).
        if callee.segments.len() != 1 {
            return Value::Poison;
        }
        let name = callee.segments[0].as_str();
        // A single-segment callee may name a local/const *callable value* — a
        // lambda bound by `let`, or a fn-ref (`const G = dbl`). Resolve it as a
        // value first (locals shadow consts, matching `eval_path`); if callable,
        // apply it. Full dispatch order: builtin → local/const callable value →
        // newtype/refined construction or enum cast (T4) → user fn → unknown.
        // Newtypes/enums live in their own tables (`self.newtypes`/`self.enums`),
        // disjoint from `self.fns`, so this new step can never shadow an
        // existing fn call — it only fires for a `name` that is NOT a callable
        // local/const AND IS declared as a newtype or enum.
        let callable_val = if let Some(v) = env.lookup(name) {
            Some(v.clone())
        } else if self.consts.contains_key(name) {
            Some(self.resolve_const(name, span))
        } else {
            None
        };
        if let Some(v) = callable_val {
            match &v {
                // An already-reported error propagates silently (D-P2.9).
                Value::Poison => return Value::Poison,
                Value::Lambda { .. } | Value::FnRef(_) => {
                    let arg_values = self.eval_value_call_args(args, env);
                    // A `return`/abort surfaced from an argument belongs to the
                    // caller; bail before applying (as the user-fn path does).
                    if self.aborted || self.pending_return.is_some() {
                        return Value::Poison;
                    }
                    return self.apply_callable(v, arg_values, span);
                }
                other => {
                    self.error(
                        span,
                        format!("value of type {} is not callable", other.type_name()),
                    );
                    return Value::Poison;
                }
            }
        }
        // Newtype/refined construction (T4): `PaletteLine(40)`. Erases to the
        // bare underlying value on success (no `Value::Typed` — that's T5).
        if let Some(decl) = self.newtypes.get(name).copied() {
            return self.construct_newtype(decl, args, span, env);
        }
        // Enum cast (T4): `Anim(1)`. The grammar has no `unchecked` escape-hatch
        // cast yet (§4.4) — an out-of-range integer is simply an error for now.
        if let Some(decl) = self.enums.get(name).copied() {
            return self.cast_enum(decl, args, span, env);
        }
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
        self.call_fn_with_values(decl, bound, span)
    }

    /// Invoke a `comptime fn` with already-evaluated positional argument values
    /// (D-P2.16). Factored out of [`eval_call`](Self::eval_call) so a first-class
    /// [`FnRef`](Value::FnRef) applied via [`apply_callable`](Self::apply_callable)
    /// runs through the exact same call machinery: arity check, depth/step
    /// budgets, a fresh pure env seeing only the params, and `Flow::Return`
    /// handling. `arg_values` must already be free of any pending return.
    fn call_fn_with_values(
        &mut self,
        decl: &'a ast::ComptimeFnDecl,
        arg_values: Vec<Value>,
        call_span: Span,
    ) -> Value {
        // Arity gate. From `eval_call` this is redundant (`bind_args` already
        // returns exactly `params.len()` values), but it is the LIVE check for
        // the `apply_callable`/`FnRef` path — `xs.map(some_fn)` reaches here with
        // whatever arity the builtin supplied. Do not delete it.
        if arg_values.len() != decl.params.len() {
            self.error(
                call_span,
                format!(
                    "function `{}` expects {} argument(s), got {}",
                    decl.name,
                    decl.params.len(),
                    arg_values.len()
                ),
            );
            return Value::Poison;
        }
        // Recursion / stack safety (D-P2.16): bound the depth *before* recursing
        // so runaway recursion is named, not a native stack overflow.
        if self.call_stack.len() >= MAX_CALL_DEPTH {
            self.abort(call_span, "recursion too deep");
            return Value::Poison;
        }
        if !self.bump_step() {
            self.abort(call_span, "step budget exceeded");
            return Value::Poison;
        }
        self.call_stack.push((decl.name.clone(), call_span));
        // Comptime fns are pure: a fresh env, seeing only their params (and, via
        // `self`, file consts/fns) — never the caller's locals.
        let mut fenv = Env::new();
        for ((pname, _, _), v) in decl.params.iter().zip(arg_values) {
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

    /// Apply a callable [`Value`] to already-evaluated arguments (D2.12): a
    /// [`Lambda`](Value::Lambda) (arity-checked, run in its captured env plus a
    /// fresh scope binding the params) or a [`FnRef`](Value::FnRef) (dispatched
    /// through [`call_fn_with_values`](Self::call_fn_with_values)). A `Poison`
    /// callable propagates silently; any other value type is "not callable".
    pub(super) fn apply_callable(&mut self, callable: Value, arg_values: Vec<Value>, call_span: Span) -> Value {
        if self.aborted {
            return Value::Poison;
        }
        match callable {
            Value::Poison => Value::Poison,
            Value::Lambda { params, body, captured } => {
                if params.len() != arg_values.len() {
                    self.error(
                        call_span,
                        format!(
                            "lambda expects {} argument(s), got {}",
                            params.len(),
                            arg_values.len()
                        ),
                    );
                    return Value::Poison;
                }
                if !self.bump_step() {
                    self.abort(call_span, "step budget exceeded");
                    return Value::Poison;
                }
                // Run in the captured env (owned via the moved `Value`) plus a
                // fresh scope holding the immutable params.
                let mut lenv = captured;
                lenv.push_scope();
                for (p, v) in params.iter().zip(arg_values) {
                    lenv.define(p.clone(), v, false);
                }
                let v = self.eval_expr(&body, &mut lenv);
                // A `return` reached through an expression-position `if`/`for` in
                // the body sets `pending_return`. `return` yields FROM the lambda
                // (the intuitive reading), so consume it here — otherwise it would
                // leak out through map/filter/fold → `eval_call` → the caller's
                // `eval_operand` and become a `Flow::Return` for the WRONG fn.
                if let Some(rv) = self.pending_return.take() {
                    return rv;
                }
                v
            }
            Value::FnRef(name) => match self.fns.get(name.as_str()).copied() {
                Some(decl) => self.call_fn_with_values(decl, arg_values, call_span),
                None => {
                    self.error(call_span, format!("unknown function `{name}`"));
                    Value::Poison
                }
            },
            other => {
                self.error(
                    call_span,
                    format!("value of type {} is not callable", other.type_name()),
                );
                Value::Poison
            }
        }
    }

    /// Evaluate the positional arguments of a call to a *callable value* (a
    /// lambda or fn-ref named at a single-segment callee). Named arguments are
    /// not supported for value calls (there is no parameter list to bind them
    /// to), so a named arg is a diagnostic; its value is still evaluated.
    fn eval_value_call_args(&mut self, args: &[ast::Arg], env: &mut Env) -> Vec<Value> {
        args.iter()
            .map(|a| {
                if a.name.is_some() {
                    self.error(a.span, "a call to a lambda or fn value takes positional arguments only");
                }
                self.eval_expr(&a.value, env)
            })
            .collect()
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

    /// `Name(x)` where `Name` is a `newtype` (T4): comptime construction.
    /// Evaluates the single argument, checks it against the newtype's
    /// effective bounds via the shared [`check_value_fits_ty`](Self::check_value_fits_ty)
    /// mechanism (D-P3.6), and returns the ERASED underlying value on success —
    /// no `Value::Typed` wrapper (that's T5, which extends this exact call
    /// site to add the type tag and newtype arithmetic).
    fn construct_newtype(
        &mut self,
        decl: &'a ast::NewtypeDecl,
        args: &[ast::Arg],
        span: Span,
        env: &mut Env,
    ) -> Value {
        let Some(arg_val) = self.eval_single_arg(&decl.name, args, span, env) else {
            return Value::Poison;
        };
        let n = match arg_val {
            Value::Int(n) => n,
            Value::Poison => return Value::Poison,
            other => {
                self.error(
                    span,
                    format!(
                        "newtype `{}` construction expects an integer, got {}",
                        decl.name,
                        other.type_name()
                    ),
                );
                return Value::Poison;
            }
        };
        let ty = crate::layout::Ty::Newtype(decl.name.clone());
        if self.check_value_fits_ty(&ty, n, span) {
            Value::Int(n)
        } else {
            Value::Poison
        }
    }

    /// `Name(x)` where `Name` is an `enum` (T4): a closed cast. Evaluates the
    /// single argument and matches it against each variant's comptime
    /// discriminant (see [`enum_variant_value`](Self::enum_variant_value)); a
    /// match yields that nullary [`Value::Enum`], and no match is
    /// `[enum.out-of-range]`. There is no `unchecked` escape-hatch cast in the
    /// grammar yet (§4.4) — deferred to whichever later task adds it.
    fn cast_enum(
        &mut self,
        decl: &'a ast::EnumDecl,
        args: &[ast::Arg],
        span: Span,
        env: &mut Env,
    ) -> Value {
        let Some(arg_val) = self.eval_single_arg(&decl.name, args, span, env) else {
            return Value::Poison;
        };
        let n = match arg_val {
            Value::Int(n) => n,
            Value::Poison => return Value::Poison,
            other => {
                self.error(
                    span,
                    format!(
                        "enum `{}` cast expects an integer, got {}",
                        decl.name,
                        other.type_name()
                    ),
                );
                return Value::Poison;
            }
        };
        for idx in 0..decl.variants.len() {
            if self.enum_variant_value(decl, idx) == Some(n) {
                return Value::Enum {
                    ty_name: decl.name.clone(),
                    variant: decl.variants[idx].name.clone(),
                    payload: vec![],
                };
            }
        }
        self.error(span, format!("[enum.out-of-range] {n} is not a variant of {}", decl.name));
        Value::Poison
    }

    /// Evaluate the comptime integer value of `decl`'s `idx`-th variant: its
    /// explicit discriminant expression (`Idle = 0`) if given, else one more
    /// than the previous variant's value (starting at 0 for the first variant)
    /// — the conventional C/Rust-style auto-increment. A non-int discriminant
    /// expression is a diagnostic (returns `None`); an already-`Poison` result
    /// stays silent (D-P2.9).
    fn enum_variant_value(&mut self, decl: &'a ast::EnumDecl, idx: usize) -> Option<i128> {
        match &decl.variants[idx].value {
            Some(expr) => match self.eval_expr(expr, &mut Env::new()) {
                Value::Int(n) => Some(n),
                Value::Poison => None,
                other => {
                    self.error(
                        crate::parser::expr_span(expr),
                        format!(
                            "enum variant discriminant must be an integer, got {}",
                            other.type_name()
                        ),
                    );
                    None
                }
            },
            None if idx == 0 => Some(0),
            None => self.enum_variant_value(decl, idx - 1).map(|v| v + 1),
        }
    }

    /// Evaluate the exactly-one positional argument a newtype/enum
    /// construction-or-cast call takes (`Name(x)`), reporting and returning
    /// `None` for the wrong arity; a named argument (`Name(x: 40)`) is also a
    /// diagnostic but its value is still evaluated and returned (so a `Poison`
    /// downstream propagates silently rather than compounding).
    fn eval_single_arg(
        &mut self,
        ty_name: &str,
        args: &[ast::Arg],
        span: Span,
        env: &mut Env,
    ) -> Option<Value> {
        if args.len() != 1 {
            self.error(
                span,
                format!("`{ty_name}` construction/cast expects exactly 1 argument, got {}", args.len()),
            );
            return None;
        }
        let arg = &args[0];
        if arg.name.is_some() {
            self.error(arg.span, format!("`{ty_name}` construction/cast takes a positional argument"));
        }
        Some(self.eval_expr(&arg.value, env))
    }
}
