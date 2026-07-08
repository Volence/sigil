//! `ensure` / `ensure_fatal` guards (§6.5) and guard-message string
//! interpolation (D-P2.19) — the only place `.emp` strings are interpolated.
use super::{Env, Evaluator};
use crate::ast;
use crate::value::Value;
use sigil_span::{Diagnostic, Span};

/// The outcome of an item-position guard evaluation (D5.2 / D-H.4). `cont` is
/// `false` only when a failing comptime `ensure_fatal` aborted (stop the module's
/// remaining items, D5.3). `link_asserts` are any deferred guards (D-H.4) to drain
/// onto the module. `anchor_used` records whether a PROVISIONAL `here()` in the
/// guard actually referenced its anonymous anchor, so the lowering pass defines
/// the anchor label only on use (D-H.8).
pub(crate) struct ItemGuardOutcome {
    /// Keep lowering the module's remaining items? (false = a fatal comptime abort.)
    pub cont: bool,
    /// Diagnostics from the guard evaluation.
    pub diags: Vec<Diagnostic>,
    /// Deferred link-time assertions produced by this guard (D-H.4).
    pub link_asserts: Vec<sigil_ir::LinkAssert>,
    /// Whether a provisional `here()` referenced the guard's anonymous anchor.
    pub anchor_used: bool,
}

/// Evaluate one item-position guard (D5.2 / D-H.4). Builds a fresh evaluator over
/// the file (the same harness as a data item: eval stack + `here` position +
/// `include_root`), evaluates the stored call expression — the
/// `ensure`/`ensure_fatal` special-case in [`Evaluator::eval_expr`] (via
/// `eval/call.rs`) does arity/interpolation/abort/deferral — and returns an
/// [`ItemGuardOutcome`]. At an EXACT position the guard passes/fails at comptime
/// exactly as before; at a PROVISIONAL one a `here()` condition DEFERS to a
/// `LinkAssert` (D-H.4) and the guard's `here()` anchor may be used (D-H.8).
pub(crate) fn eval_item_guard(
    file: &crate::ast::File,
    decl: &crate::ast::EnsureDecl,
    here: crate::layout::HerePos,
    include_root: Option<&std::path::Path>,
) -> ItemGuardOutcome {
    // `decl.fatal` records which keyword the parser saw; dispatch itself keys
    // off the stored call's CALLEE name (`eval/call.rs`). Assert the two agree
    // so any future parser divergence between the keyword and the captured call
    // is caught in debug builds instead of silently mis-dispatching.
    debug_assert!(
        matches!(&decl.call, ast::Expr::Call { callee, .. }
            if callee.segments.len() == 1
                && callee.segments[0] == if decl.fatal { "ensure_fatal" } else { "ensure" }),
        "EnsureDecl.fatal disagrees with the stored call's callee"
    );
    crate::eval::run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        ev.apply_here_pos(Some(here));
        if let Some(root) = include_root {
            ev.set_include_root(root.to_path_buf());
        }
        let mut env = Env::new();
        let _ = ev.eval_expr(&decl.call, &mut env);
        ItemGuardOutcome {
            cont: !ev.was_aborted(),
            anchor_used: ev.here_anchor_used(),
            link_asserts: ev.take_link_asserts(),
            diags: ev.diags,
        }
    })
}

impl<'a> Evaluator<'a> {
    /// Evaluate an `ensure` / `ensure_fatal` guard (§6.5). `fatal` selects the
    /// variant. Both take exactly two positional args `(cond, message)`.
    ///
    /// A passing guard (`cond == Bool(true)`) is silent and cheap: the message is
    /// never evaluated or interpolated, and the guard yields [`Value::Unit`]. A
    /// failing guard evaluates the message (which must be a string), interpolates
    /// its `{expr}` placeholders against the *current* `env` (see
    /// [`interpolate`](Self::interpolate)), and reports it — as a plain error for
    /// `ensure` (returning `Poison`, so downstream use suppresses per D-P2.8) or
    /// as an evaluation-halting abort for `ensure_fatal`. A `Poison` condition or
    /// message propagates silently; a non-bool condition or non-string message is
    /// an error.
    pub(super) fn eval_guard(&mut self, fatal: bool, args: &[ast::Arg], span: Span, env: &mut Env) -> Value {
        let kw = if fatal { "ensure_fatal" } else { "ensure" };
        // Guards take positional args only; a named arg is a diagnostic (but we
        // press on, so a mislabeled call still gets its cond/message checked).
        for a in args {
            if a.name.is_some() {
                self.error(a.span, format!("`{kw}` takes positional arguments (cond, message)"));
            }
        }
        if args.len() != 2 {
            self.error(
                span,
                format!("`{kw}` expects 2 arguments (cond, message), got {}", args.len()),
            );
            return Value::Poison;
        }
        let cond = self.eval_expr(&args[0].value, env);
        // A `return` surfaced from the condition, or an abort, unwinds silently.
        if self.aborted || self.pending_return.is_some() {
            return Value::Poison;
        }
        // D-H.4: a PROVISIONAL `here()` condition is a link-time value — the guard
        // is neither passed nor failed at comptime. DEFER: freeze the message
        // (D-H.5) and record a `LinkAssert` the linker evaluates against the
        // post-relaxation symbol table. Returns `Unit` (the guard yields nothing).
        if let Value::LinkExpr(cond_expr) = cond {
            return self.defer_guard(fatal, cond_expr, &args[1].value, span, env);
        }
        match cond {
            // Already-reported error in the condition: stay silent (D-P2.9).
            Value::Poison => Value::Poison,
            // Passing guard: silent and cheap — the message is never touched.
            Value::Bool(true) => Value::Unit,
            Value::Bool(false) => {
                let msg = self.eval_expr(&args[1].value, env);
                if self.aborted || self.pending_return.is_some() {
                    return Value::Poison;
                }
                let template = match msg {
                    Value::Str(s) => s,
                    Value::Poison => return Value::Poison,
                    other => {
                        self.error(
                            span,
                            format!("`{kw}` message must be a string, got {}", other.type_name()),
                        );
                        return Value::Poison;
                    }
                };
                let text = self.interpolate(&template, env, span);
                if fatal {
                    // Halt evaluation with the interpolated guard text as the sole
                    // reason (NOT the budget/recursion chain that `abort` formats).
                    // Mirror `abort`'s contract: emit once, then set the flag so
                    // in-flight evaluation unwinds silently.
                    if !self.aborted {
                        self.aborted = true;
                        self.error(span, text);
                    }
                } else {
                    self.error(span, text);
                }
                Value::Poison
            }
            other => {
                self.error(
                    span,
                    format!("`{kw}` condition must be bool, got {}", other.type_name()),
                );
                Value::Poison
            }
        }
    }

    /// Defer an `ensure`/`ensure_fatal` guard whose condition became a link-time
    /// value (D-H.4). Evaluates and freezes the message (D-H.5) — comptime parts
    /// to `MsgPart::Text`, a link-time `{expr}` placeholder to `MsgPart::Expr` —
    /// and records a [`LinkAssert`](sigil_ir::LinkAssert) on the evaluator, which
    /// the lowering pass drains onto the module. Returns [`Value::Unit`]: the
    /// guard is neither passed nor failed here. A non-string message / arity error
    /// stays a comptime error exactly as an eager guard (it never defers).
    fn defer_guard(
        &mut self,
        fatal: bool,
        cond: sigil_ir::expr::Expr,
        message_expr: &ast::Expr,
        span: Span,
        env: &mut Env,
    ) -> Value {
        let kw = if fatal { "ensure_fatal" } else { "ensure" };
        let msg = self.eval_expr(message_expr, env);
        if self.aborted || self.pending_return.is_some() {
            return Value::Poison;
        }
        let template = match msg {
            Value::Str(s) => s,
            Value::Poison => return Value::Poison,
            other => {
                self.error(
                    span,
                    format!("`{kw}` message must be a string, got {}", other.type_name()),
                );
                return Value::Poison;
            }
        };
        let message = self.interpolate_parts(&template, env, span);
        self.link_asserts.push(sigil_ir::LinkAssert { cond, message, fatal, span });
        Value::Unit
    }

    /// Interpolate a DEFERRED guard message into [`MsgPart`](sigil_ir::MsgPart)
    /// runs (D-H.5): literal text and comptime `{expr}` placeholders freeze to
    /// [`Text`](sigil_ir::MsgPart::Text) NOW (the comptime env is about to
    /// disappear), while a placeholder whose value is itself a provisional
    /// `here()` [`LinkExpr`](Value::LinkExpr) becomes an
    /// [`Expr`](sigil_ir::MsgPart::Expr) folded and rendered at link on failure.
    /// Adjacent `Text` runs are coalesced. Mirrors [`interpolate`](Self::interpolate)'s
    /// lexing of `{{`/`}}`/`{…}`; the ONLY difference is the per-placeholder
    /// eager-vs-lazy split.
    fn interpolate_parts(&mut self, s: &str, env: &mut Env, span: Span) -> Vec<sigil_ir::MsgPart> {
        use sigil_ir::MsgPart;
        let mut parts: Vec<MsgPart> = Vec::new();
        let mut lit = String::new();
        // Push a literal run (coalescing) and clear the accumulator.
        let flush = |lit: &mut String, parts: &mut Vec<MsgPart>| {
            if !lit.is_empty() {
                match parts.last_mut() {
                    Some(MsgPart::Text(t)) => t.push_str(lit),
                    _ => parts.push(MsgPart::Text(std::mem::take(lit))),
                }
                lit.clear();
            }
        };
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '{' if chars.peek() == Some(&'{') => {
                    chars.next();
                    lit.push('{');
                }
                '}' if chars.peek() == Some(&'}') => {
                    chars.next();
                    lit.push('}');
                }
                '{' => {
                    let mut inner = String::new();
                    let mut closed = false;
                    for nc in chars.by_ref() {
                        if nc == '}' {
                            closed = true;
                            break;
                        }
                        inner.push(nc);
                    }
                    if !closed {
                        self.error(span, "unterminated `{` in guard message");
                        lit.push('{');
                        lit.push_str(&inner);
                        break;
                    }
                    match self.interp_one_part(&inner, env, span) {
                        // A link-time placeholder becomes an Expr part (folded at
                        // link); flush the pending literal first to preserve order.
                        MsgPart::Expr(e) => {
                            flush(&mut lit, &mut parts);
                            parts.push(MsgPart::Expr(e));
                        }
                        // A comptime placeholder is already rendered text.
                        MsgPart::Text(t) => lit.push_str(&t),
                    }
                }
                _ => lit.push(c),
            }
        }
        flush(&mut lit, &mut parts);
        parts
    }

    /// Evaluate one deferred-message placeholder to a [`MsgPart`](sigil_ir::MsgPart)
    /// (D-H.5): a provisional `here()` [`LinkExpr`](Value::LinkExpr) becomes a
    /// lazy [`Expr`](sigil_ir::MsgPart::Expr) part; every other value is rendered
    /// to [`Text`](sigil_ir::MsgPart::Text) eagerly (its `Display`, a string
    /// unquoted — same rule as [`interp_one`](Self::interp_one)). A lex/parse
    /// failure or a `Poison` value renders the `<?>` placeholder with one
    /// diagnostic, exactly as the eager path.
    fn interp_one_part(&mut self, inner: &str, env: &mut Env, span: Span) -> sigil_ir::MsgPart {
        use sigil_ir::MsgPart;
        let bad = |ev: &mut Self, reason: &str| -> MsgPart {
            ev.error(span, format!("cannot interpolate `{{{inner}}}`: {reason}"));
            MsgPart::Text("<?>".to_string())
        };
        let (toks, lex_errs) = crate::lexer::lex(inner, span.source);
        if !lex_errs.is_empty() {
            return bad(self, "lex error");
        }
        let mut p = crate::parser::Parser::new(toks);
        let expr = p.expr();
        if !p.into_diagnostics().is_empty() {
            return bad(self, "parse error");
        }
        match self.eval_expr(&expr, env) {
            Value::Poison => bad(self, "evaluation failed"),
            // A provisional `here()` placeholder folds at link — keep it lazy so
            // the message reports the REAL final address (D-H.5).
            Value::LinkExpr(e) => MsgPart::Expr(e),
            Value::Str(s) => MsgPart::Text(s),
            other => MsgPart::Text(other.to_string()),
        }
    }

    /// Interpolate a guard message's `{expr}` placeholders (D-P2.19). This is the
    /// ONLY place `.emp` strings are interpolated — a plain string elsewhere keeps
    /// its `{...}` text literally.
    ///
    /// `{{` and `}}` are literal `{`/`}`. A `{ ... }` placeholder lexes+parses its
    /// inner text as a single expression, evaluates it in `env`, and appends the
    /// resulting [`Value`]'s `Display`. Best-effort: a lex/parse failure or a
    /// `Poison` result appends `<?>` and pushes one diagnostic, then interpolation
    /// continues. An unterminated `{` (no closing `}`) pushes one diagnostic and
    /// the remaining text — including the `{` — is appended literally.
    fn interpolate(&mut self, s: &str, env: &mut Env, span: Span) -> String {
        let mut out = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '{' if chars.peek() == Some(&'{') => {
                    chars.next();
                    out.push('{');
                }
                '}' if chars.peek() == Some(&'}') => {
                    chars.next();
                    out.push('}');
                }
                '{' => {
                    let mut inner = String::new();
                    let mut closed = false;
                    for nc in chars.by_ref() {
                        if nc == '}' {
                            closed = true;
                            break;
                        }
                        inner.push(nc);
                    }
                    if !closed {
                        self.error(span, "unterminated `{` in guard message");
                        out.push('{');
                        out.push_str(&inner);
                        break;
                    }
                    out.push_str(&self.interp_one(&inner, env, span));
                }
                _ => out.push(c),
            }
        }
        out
    }

    /// Lex+parse `inner` as a single expression, evaluate it in `env`, and return
    /// its `Display`. On any lex/parse failure — or a `Poison` result — returns
    /// the `<?>` placeholder and pushes one "cannot interpolate" diagnostic at
    /// `span`, so interpolation stays best-effort (never a crash). The inner
    /// expression is parsed with a fresh [`Parser`](crate::parser::Parser), the
    /// same single-expression entry the test helper uses.
    fn interp_one(&mut self, inner: &str, env: &mut Env, span: Span) -> String {
        let bad = |ev: &mut Self, reason: &str| -> String {
            ev.error(span, format!("cannot interpolate `{{{inner}}}`: {reason}"));
            "<?>".to_string()
        };
        let (toks, lex_errs) = crate::lexer::lex(inner, span.source);
        if !lex_errs.is_empty() {
            return bad(self, "lex error");
        }
        let mut p = crate::parser::Parser::new(toks);
        let expr = p.expr();
        if !p.into_diagnostics().is_empty() {
            return bad(self, "parse error");
        }
        // Evaluating the inner expr may itself report (e.g. an unknown name); that
        // diagnostic plus this best-effort one are both acceptable — the guard is
        // already failing, so a noisy interpolation is not a cascade concern.
        match self.eval_expr(&expr, env) {
            Value::Poison => bad(self, "evaluation failed"),
            // A string interpolates as its bare contents: `Value`'s `Display`
            // quotes strings (`{s:?}`) for diagnostics, which reads wrong
            // mid-sentence in a user-facing guard message. Every other value kind
            // keeps its `Display`.
            Value::Str(s) => s,
            other => other.to_string(),
        }
    }
}
