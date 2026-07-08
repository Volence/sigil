//! `ensure` / `ensure_fatal` guards (§6.5) and guard-message string
//! interpolation (D-P2.19) — the only place `.emp` strings are interpolated.
use super::{Env, Evaluator};
use crate::ast;
use crate::value::Value;
use sigil_span::{Diagnostic, Span};

/// Evaluate one item-position guard (D5.2). Builds a fresh evaluator over the
/// file (the same harness as a data item: eval stack + `here_base` +
/// `include_root`), evaluates the stored call expression — the
/// `ensure`/`ensure_fatal` special-case in [`Evaluator::eval_expr`] (via
/// `eval/call.rs`) does arity/interpolation/abort — and returns
/// `(continue_lowering, diagnostics)`. `continue_lowering` is `false` only when a
/// failing `ensure_fatal` set the abort flag (D5.3), signalling the caller to
/// stop lowering the module's remaining items.
pub(crate) fn eval_item_guard(
    file: &crate::ast::File,
    decl: &crate::ast::EnsureDecl,
    here_base: u32,
    include_root: Option<&std::path::Path>,
) -> (bool, Vec<Diagnostic>) {
    crate::eval::run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        ev.set_here_base(here_base);
        if let Some(root) = include_root {
            ev.set_include_root(root.to_path_buf());
        }
        let mut env = Env::new();
        let _ = ev.eval_expr(&decl.call, &mut env);
        (!ev.was_aborted(), ev.diags)
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
