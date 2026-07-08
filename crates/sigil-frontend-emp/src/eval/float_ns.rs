//! The `as.*` / `math.*` comptime float namespaces (Spec 2, Plan 5 — Task 4,
//! §6.6).
//!
//! **Intent of the split.** `as.*` is the byte-compatible surface for porting
//! `asl 1.42 Bld 212` numeric routines: it exists so a ported deform table
//! (or any other AS-derived float computation) can be reproduced byte-exact,
//! including AS's specific rounding quirks. `math.*` is the namespace NEW
//! `.emp` code should reach for — ordinary IEEE-754 float math, with no
//! promise of matching any particular assembler's historical behavior.
//!
//! **Today they share one backing.** Both namespaces are currently
//! implemented on top of Rust's std `f64` (`sin`/`cos`), because Core §7.1 /
//! M1.C Spike 0 already proved that std `f64::sin()` bit-matches `asl`'s sine
//! tables. So `as.sin(x) == math.sin(x)` for every `x` today — the split is
//! purely a **greppable, eventually-deletable compat surface**: if a future
//! `asl` quirk is found that std `f64` does NOT reproduce, only `as.sin` (not
//! `math.sin`) would grow a compensating tweak. The one place the two
//! namespaces genuinely diverge already is `int`: `as.int` is `asl`'s verified
//! floor-toward-negative-infinity semantic (`int()` in AS is `floor`, NOT
//! truncate — see Core §7.1), and is deliberately `as`-only. New code should
//! spell out its rounding mode explicitly (there is no `math.int` — see
//! `eval_float_ns`'s unknown-function arm) rather than inherit a compat name.
//!
//! **Determinism follow-up (deferred).** The goldens this module is graded
//! against were captured against std `f64` on this host/toolchain. Bit-exact
//! `sin`/`cos` across *arbitrary* platforms is not guaranteed by IEEE 754 (the
//! standard fixes rounding for `+ - * / sqrt`, not transcendental functions),
//! so a future determinism task may need to swap this backing for a
//! fixed-implementation library (e.g. the `libm` crate) if Sigil ever needs to
//! guarantee identical output across host platforms. That swap is out of
//! scope here: std `f64` is what Task 0's goldens were captured against, and
//! is what this task's acceptance gate requires.
use super::expr::num_f64;
use super::{Env, Evaluator};
use crate::ast;
use crate::value::Value;
use sigil_span::Span;

impl<'a> Evaluator<'a> {
    /// Evaluate an `as.{fn_name}(...)` / `math.{fn_name}(...)` call (`ns` is
    /// `"as"` or `"math"` — the call-site guard in `eval_call` never routes
    /// any other namespace head here). Every function in this table takes
    /// exactly one positional numeric argument.
    pub(super) fn eval_float_ns(
        &mut self,
        ns: &str,
        fn_name: &str,
        args: &[ast::Arg],
        span: Span,
        env: &mut Env,
    ) -> Value {
        if args.len() != 1 {
            self.error(
                span,
                format!("`{ns}.{fn_name}` expects exactly 1 argument, got {}", args.len()),
            );
            return Value::Poison;
        }
        let arg = &args[0];
        if arg.name.is_some() {
            self.error(span, format!("`{ns}.{fn_name}` takes a positional argument"));
            return Value::Poison;
        }
        let arg_val = self.eval_expr(&arg.value, env);
        // A `return`/abort surfaced from the argument belongs to the caller
        // (the `eval_operand` invariant — see `call.rs`'s other arg-eval
        // sites), not this call.
        if self.aborted || self.pending_return.is_some() {
            return Value::Poison;
        }
        // An already-reported error propagates silently (D-P2.9) — no new
        // diagnostic for a `Poison` argument.
        if matches!(arg_val, Value::Poison) {
            return Value::Poison;
        }
        // A provisional `here()` cannot feed a comptime float function (D-H.2).
        if let Some(v) = self.reject_if_provisional(&arg_val, span) {
            return v;
        }
        let x = match num_f64(&arg_val) {
            Some(x) => x,
            None => {
                self.error(
                    span,
                    format!(
                        "[float-ns.arg] {ns}.{fn_name} expects a number, got {}",
                        arg_val.type_name()
                    ),
                );
                return Value::Poison;
            }
        };
        match (ns, fn_name) {
            ("as", "sin") | ("math", "sin") => Value::Float(x.sin()),
            ("as", "cos") | ("math", "cos") => Value::Float(x.cos()),
            // ONLY `as.int` — the verified `asl` floor semantic (Core §7.1):
            // `int()` in AS floors toward -infinity, it does not truncate
            // toward zero. New code (`math.*`) has no `int` at all; see the
            // module doc comment.
            ("as", "int") => {
                let f = x.floor();
                // Guard the narrowing cast explicitly (D-P2.1 style: overflow
                // is an error, never a silent saturate/wrap) rather than rely
                // on Rust's `as` float-to-int cast, which saturates instead of
                // reporting a fitness problem.
                // NOTE the EXCLUSIVE upper bound: `MAX_I128_AS_F64` rounds UP to
                // 2^127 (one past `i128::MAX`), so an inclusive bound would admit
                // `f == 2^127`, which then saturates under `f as i128` to
                // `i128::MAX` — a silent wrong value. Excluding the endpoint
                // rejects `2^127` while still admitting the largest representable
                // f64 strictly below it (`2^127 − 2^75`), which casts cleanly.
                if f.is_finite() && (MIN_I128_AS_F64..MAX_I128_AS_F64).contains(&f) {
                    Value::Int(f as i128)
                } else {
                    self.error(
                        span,
                        format!("[float-ns.int-range] as.int({x}) does not fit in a 128-bit integer"),
                    );
                    Value::Poison
                }
            }
            _ => {
                self.error(span, format!("[float-ns.unknown] no float function `{ns}.{fn_name}`"));
                Value::Poison
            }
        }
    }
}

/// `i128::MIN` as an `f64` — the low end of the `as.int` range guard. Computed
/// once as a const rather than re-cast per call.
const MIN_I128_AS_F64: f64 = i128::MIN as f64;
/// `i128::MAX` as an `f64` — the high end of the `as.int` range guard. This
/// rounds UP from the true `i128::MAX` (an `f64` mantissa cannot represent it
/// exactly) to exactly `2^127`, so the guard uses it as an EXCLUSIVE upper
/// bound: `2^127` itself does not fit `i128` and must be rejected, while every
/// representable f64 strictly below it does fit.
const MAX_I128_AS_F64: f64 = i128::MAX as f64;
