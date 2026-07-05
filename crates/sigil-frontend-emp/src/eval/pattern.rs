//! `match` / sum-type destructuring (T6): exhaustiveness checking and
//! pattern binding against a [`Value::Enum`] scrutinee.
use super::{Env, Evaluator};
use crate::ast;
use crate::value::Value;
use sigil_span::Span;

impl<'a> Evaluator<'a> {
    /// `match scrutinee { pat => body, ... }` (T6). The scrutinee must
    /// evaluate to a [`Value::Enum`] — matching on any other value kind is
    /// unsupported in v1 (a clear diagnostic; a future plan may add matching
    /// on other shapes). Exhaustiveness (D-P3.10) is checked statically
    /// against the scrutinee's enum decl BEFORE arm selection, and always
    /// runs regardless of which arm ultimately fires. Arms are then tried
    /// top-to-bottom; the first whose pattern matches the scrutinee wins, runs
    /// its body in a fresh scope holding the pattern's bindings, and its value
    /// is returned. A `return` reached through a nested expression-position
    /// `if` inside the winning arm's body sets `pending_return` exactly as it
    /// would for any other expression (T4's mechanism) — `eval_match` does
    /// nothing special to propagate it; the caller's usual
    /// `eval_operand`/`Expr::If`-wrapping machinery picks it up unchanged.
    pub(super) fn eval_match(
        &mut self,
        scrutinee: &ast::Expr,
        arms: &[ast::MatchArm],
        span: Span,
        env: &mut Env,
    ) -> Value {
        let scrutinee_span = crate::parser::expr_span(scrutinee);
        let sv = self.eval_expr(scrutinee, env);
        // A `return`/abort surfaced while evaluating the scrutinee belongs to
        // the caller (the `eval_operand` invariant) — bail before touching arms.
        if self.aborted || self.pending_return.is_some() {
            return Value::Poison;
        }
        let (ty_name, variant) = match &sv {
            Value::Enum { ty_name, variant, .. } => (ty_name.clone(), variant.clone()),
            Value::Poison => return Value::Poison,
            other => {
                self.error(
                    scrutinee_span,
                    format!("match on non-enum value ({}) is unsupported", other.type_name()),
                );
                return Value::Poison;
            }
        };
        // Exhaustiveness + variant-name validation is a STATIC check — it
        // always runs, independent of which arm (if any) matches at runtime,
        // and reports whether it emitted any diagnostic (so the runtime
        // "no arm matched" fallback below does not double-report on the
        // common non-exhaustive-hit path).
        let statically_reported = self.check_match_exhaustive(&ty_name, arms, span);
        for arm in arms {
            env.push_scope();
            let outcome = self.bind_pattern(&arm.pat, &sv, env);
            match outcome {
                // Matched (and any bindings applied) — run the arm body in
                // this scope and return its value; first match wins.
                Some(true) => {
                    let v = self.eval_expr(&arm.body, env);
                    env.pop_scope();
                    return v;
                }
                // Did not match this arm (e.g. a different variant, or a
                // nested subpattern mismatch) — drop the scope and try the
                // next arm; no diagnostic, this is an ordinary non-match.
                Some(false) => {
                    env.pop_scope();
                }
                // A structural pattern error (arity mismatch) was already
                // diagnosed inside `bind_pattern` — poison the whole match
                // immediately rather than silently trying another arm, since
                // the mismatch is fixed by the pattern's shape, not the data.
                None => {
                    env.pop_scope();
                    return Value::Poison;
                }
            }
        }
        // No arm matched at runtime. When the static check already reported
        // (a non-exhaustive match, or a typo'd variant), this is the SAME
        // event — do not double-report; just poison per D-P2.9. The fallback
        // diagnostic is reserved for the genuinely-shouldn't-happen case where
        // exhaustiveness passed yet nothing matched.
        if !statically_reported {
            self.error(span, format!("no arm matched enum value `{ty_name}.{variant}`"));
        }
        Value::Poison
    }

    /// Static validation of a `match` on enum `ty_name` (D-P3.10): variant-name
    /// checking AND exhaustiveness. Returns `true` iff it emitted any
    /// diagnostic (so [`eval_match`](Self::eval_match) can suppress its runtime
    /// "no arm matched" fallback rather than double-report the same event).
    ///
    /// Because comptime enums are closed (a fixed, fully-known variant set — no
    /// solver needed), both checks are simple scans over the declared variants:
    /// - Every [`Pattern::Variant`] names a variant by its path's LAST segment;
    ///   a segment naming NO declared variant is a typo — reported
    ///   (`no variant \`X\` on enum \`Y\``) EVEN when a catch-all is present, so
    ///   a typo is never silently swallowed by the `_` arm.
    /// - A `Variant` arm covers its named variant; a [`Pattern::Wildcard`] or
    ///   [`Pattern::Binding`] is a catch-all covering every remaining variant.
    ///   Any declared variant covered by neither, once all arms are scanned, is
    ///   reported by name in one `[match.non-exhaustive]` diagnostic.
    ///
    /// Silently returns `false` if `ty_name` isn't a known enum (should not
    /// happen — the scrutinee already produced a `Value::Enum` naming it).
    fn check_match_exhaustive(&mut self, ty_name: &str, arms: &[ast::MatchArm], span: Span) -> bool {
        let Some(decl) = self.enums.get(ty_name).copied() else {
            return false;
        };
        let mut reported = false;
        let mut covered: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut catch_all = false;
        for arm in arms {
            match &arm.pat {
                ast::Pattern::Variant { path, span: pat_span, .. } => {
                    if let Some(seg) = path.segments.last() {
                        // Validate the variant name against the enum (fires
                        // regardless of any catch-all, so a typo is caught).
                        if decl.variants.iter().any(|v| &v.name == seg) {
                            covered.insert(seg.as_str());
                        } else {
                            self.error(
                                *pat_span,
                                format!("no variant `{seg}` on enum `{ty_name}`"),
                            );
                            reported = true;
                        }
                    }
                }
                ast::Pattern::Wildcard(_) | ast::Pattern::Binding(_, _) => catch_all = true,
            }
        }
        if catch_all {
            return reported;
        }
        let missing: Vec<&str> =
            decl.variants.iter().map(|v| v.name.as_str()).filter(|n| !covered.contains(n)).collect();
        if !missing.is_empty() {
            self.error(span, format!("[match.non-exhaustive] missing {}", missing.join(", ")));
            reported = true;
        }
        reported
    }

    /// Try to match (and, on success, bind) `pat` against a value `val` —
    /// either the whole scrutinee (top-level arm pattern) or one payload
    /// value (a nested [`Pattern::Variant`] subpattern) — recursing uniformly
    /// for both, since a nested subpattern is matched exactly like a
    /// top-level one against its corresponding payload value.
    ///
    /// Returns:
    /// - `Some(true)` — matched; any bindings introduced are defined in `env`.
    /// - `Some(false)` — did not match (a different variant, or a nested
    ///   subpattern mismatch) — an ordinary non-match, no diagnostic; the
    ///   caller should try the next arm.
    /// - `None` — a STRUCTURAL pattern error (subpattern arity does not match
    ///   the payload's declared arity) already diagnosed here — this is fixed
    ///   by the pattern's shape, not by the runtime data, so the caller should
    ///   poison the whole match rather than keep trying arms.
    fn bind_pattern(&mut self, pat: &ast::Pattern, val: &Value, env: &mut Env) -> Option<bool> {
        match pat {
            ast::Pattern::Wildcard(_) => Some(true),
            ast::Pattern::Binding(name, _) => {
                env.define(name.clone(), val.clone(), false);
                Some(true)
            }
            ast::Pattern::Variant { path, subpats, span } => {
                let vname = path.segments.last().map(String::as_str).unwrap_or("");
                match val {
                    Value::Enum { variant, payload, .. } => {
                        if variant != vname {
                            return Some(false);
                        }
                        if subpats.len() != payload.len() {
                            self.error(
                                *span,
                                format!(
                                    "[match.pattern-arity] pattern `{vname}` expects {} payload value(s), got {}",
                                    subpats.len(),
                                    payload.len()
                                ),
                            );
                            return None;
                        }
                        for (sp, pv) in subpats.iter().zip(payload.iter()) {
                            match self.bind_pattern(sp, pv, env) {
                                Some(true) => {}
                                Some(false) => return Some(false),
                                None => return None,
                            }
                        }
                        Some(true)
                    }
                    // A payload value that already failed evaluation (T6:
                    // constructing the enum itself may have poisoned a payload
                    // slot) propagates silently (D-P2.9). But the subpatterns'
                    // inner names must STILL be bound — to `Poison`, mirroring
                    // the `Binding` case — otherwise the arm body's references
                    // to them fire a spurious `unknown name` cascade off the
                    // one real (already-reported) error.
                    Value::Poison => {
                        self.bind_subpats_poison(subpats, env);
                        Some(true)
                    }
                    // Matching a `Variant` pattern against a non-enum value is
                    // a static/data mismatch this task does not deep-check
                    // (payload TYPES are loose at comptime, D-P3.10 scope) —
                    // treat as an ordinary non-match rather than erroring.
                    _ => Some(false),
                }
            }
        }
    }

    /// Bind every name introduced by `subpats` to [`Value::Poison`], recursing
    /// through nested [`Pattern::Variant`] subpatterns. Used when a `Variant`
    /// pattern matches against an already-`Poison` payload value (D-P2.9): the
    /// inner names must be defined so the arm body does not fire a spurious
    /// `unknown name` cascade off the one real error, but they carry no usable
    /// value. Wildcards bind nothing; arity is not checked (the value is
    /// already poisoned, so any further diagnostic would itself be a cascade).
    fn bind_subpats_poison(&mut self, subpats: &[ast::Pattern], env: &mut Env) {
        for sp in subpats {
            match sp {
                ast::Pattern::Wildcard(_) => {}
                ast::Pattern::Binding(name, _) => env.define(name.clone(), Value::Poison, false),
                ast::Pattern::Variant { subpats, .. } => self.bind_subpats_poison(subpats, env),
            }
        }
    }
}
