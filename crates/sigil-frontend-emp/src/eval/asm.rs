//! `asm { }` instantiation (Spec 2, Plan 4 — T3, §6.2 / D-P4.3): evaluate an
//! `asm` block's statements to a RESOLVED [`Value::Code`]. Every `{splice}` is
//! evaluated and typed HERE (not deferred): a mnemonic/size splice must be a
//! [`Width`]/[`Cc`], an operand splice must be an int / [`Reg`] / label. The
//! "what operand class is expected here vs. what did we get" decision lives in
//! ONE place — this module (it inspects [`Value`], so it stays Core-free; the
//! backend-operand MAPPING is `lower/code.rs`, per D-P4.1). A wrong-kind splice
//! is the `[asm.splice-kind]` diagnostic, naming the expected class and the
//! value's [`type_name`](Value::type_name) for the "got" side (§6.2 `~describe`).
//!
//! Non-`export` labels are renamed fresh per instantiation for hygiene (D-P4.6,
//! minimal for T3): a monotonic counter `k` on the [`Evaluator`](super::Evaluator)
//! gives each block a unique id, `.name` → `$asm{k}$name`, and references to
//! `.name` within the same block rewrite to the same fresh symbol so an
//! intra-`asm{}` branch resolves. The FULL hygiene model (`export` opt-out,
//! cross-`asm{}` refs) is T5 — here `export` is only recorded on the label.

use super::{Env, Evaluator};
use crate::ast::{self, AsmStmt, InstrLine, Operand, TextOrSplice};
use crate::parser::expr_span;
use crate::value::{CodeBuf, CodeItem, CodeOperand, Reg, Value, Width};
use sigil_span::Span;
use std::collections::HashMap;

impl Evaluator<'_> {
    /// Evaluate an `asm { }` body to a [`Value::Code`]. Two passes: first collect
    /// this instantiation's non-`export` local labels and assign each a fresh
    /// renamed symbol; then build one [`CodeItem`] per statement, rewriting label
    /// references against that rename map. A statement that fails to lower emits a
    /// diagnostic and is dropped (its `Poison`-equivalent), so one bad line does
    /// not abort the whole block.
    pub(super) fn eval_asm(&mut self, body: &[AsmStmt], _span: Span, env: &mut Env) -> Value {
        let k = self.asm_counter;
        self.asm_counter += 1;

        // Pass 1: non-`export` label names → fresh unique symbols.
        let mut renames: HashMap<String, String> = HashMap::new();
        for stmt in body {
            if let AsmStmt::Label { name, export, .. } = stmt {
                if !export {
                    renames.entry(name.clone()).or_insert_with(|| format!("$asm{k}${name}"));
                }
            }
        }

        // Pass 2: build the resolved item list.
        let mut buf = CodeBuf::empty();
        for stmt in body {
            match stmt {
                AsmStmt::Label { name, export, span } => {
                    let out_name = if *export {
                        // T5 owns the caller-visible `Owner.name` spelling; T3 only
                        // records `export` and keeps the label's own name.
                        name.clone()
                    } else {
                        renames[name].clone()
                    };
                    buf.push(CodeItem::Label { name: out_name, export: *export, span: *span });
                }
                AsmStmt::Instr(instr) => {
                    if let Some(item) = self.lower_instr_to_item(instr, &renames, env) {
                        buf.push(item);
                    }
                }
                AsmStmt::Call(expr) => {
                    // A statement-position call splices a nested template's items
                    // in (§6.2): it MUST evaluate to a `Code` value.
                    let v = self.eval_expr(expr, env);
                    match v {
                        Value::Code(inner) => buf.items.extend(inner.items),
                        Value::Poison => {}
                        other => self.error(
                            expr_span(expr),
                            format!(
                                "an `asm` statement-call must evaluate to Code, got {}",
                                other.type_name()
                            ),
                        ),
                    }
                }
            }
        }
        Value::Code(buf)
    }

    /// Lower one [`InstrLine`] to a [`CodeItem::Instr`]: resolve the mnemonic and
    /// size (splices typed against [`Width`]/[`Cc`]) and map every operand. Any
    /// failure emits a diagnostic and yields `None` (the line is dropped).
    fn lower_instr_to_item(
        &mut self,
        instr: &InstrLine,
        renames: &HashMap<String, String>,
        env: &mut Env,
    ) -> Option<CodeItem> {
        let mnemonic = self.resolve_mnemonic(&instr.mnemonic, env)?;
        let size = self.resolve_size(instr.size.as_ref(), instr.span, env)?;
        let mut ops = Vec::with_capacity(instr.operands.len());
        for op in &instr.operands {
            ops.push(self.map_operand(op, renames, env)?);
        }
        Some(CodeItem::Instr { mnemonic, size, ops, span: instr.span })
    }

    /// Resolve a possibly-spliced mnemonic to its final string. A `{splice}` in
    /// the mnemonic must be a [`Width`] (`cmp.{w}`-style, spliced as its
    /// `Display`) or a [`Cc`] (`b{cc}` → `"bne"`); any other kind is
    /// `[asm.splice-kind]`. Returns `None` on a poison/mistyped splice.
    fn resolve_mnemonic(&mut self, parts: &[TextOrSplice], env: &mut Env) -> Option<String> {
        let mut out = String::new();
        for part in parts {
            match part {
                TextOrSplice::Text(t) => out.push_str(t),
                TextOrSplice::Splice(e) => {
                    let v = self.eval_expr(e, env);
                    match v {
                        Value::Width(w) => out.push_str(&w.to_string()),
                        Value::Cc(c) => out.push_str(&c.to_string()),
                        Value::Poison => return None,
                        other => {
                            self.splice_kind_err(expr_span(e), "Width or Cc", &other);
                            return None;
                        }
                    }
                }
            }
        }
        Some(out)
    }

    /// Resolve an optional size suffix to an `Option<Width>`. The outer `Option`
    /// distinguishes error (`None`) from "no size / a resolved size"
    /// (`Some(Option<Width>)`). A literal `b`/`w`/`l`/`s` maps directly; a
    /// `{splice}` must evaluate to a [`Width`] (`[asm.splice-kind]` otherwise).
    fn resolve_size(
        &mut self,
        size: Option<&TextOrSplice>,
        span: Span,
        env: &mut Env,
    ) -> Option<Option<Width>> {
        match size {
            None => Some(None),
            Some(TextOrSplice::Text(t)) => match width_from_text(t) {
                Some(w) => Some(Some(w)),
                None => {
                    self.error(span, format!("unknown size suffix `.{t}`"));
                    None
                }
            },
            Some(TextOrSplice::Splice(e)) => {
                let v = self.eval_expr(e, env);
                match v {
                    Value::Width(w) => Some(Some(w)),
                    Value::Poison => None,
                    other => {
                        self.splice_kind_err(expr_span(e), "Width", &other);
                        None
                    }
                }
            }
        }
    }

    /// Map one parsed [`ast::Operand`] to a resolved [`CodeOperand`]. Register and
    /// `.local`/symbol references resolve directly; an `#imm` / displacement
    /// evaluates its expr to an integer; a `{splice}` is typed against the operand
    /// classes (int / [`Reg`] / label). Returns `None` on any diagnosed failure.
    fn map_operand(
        &mut self,
        op: &Operand,
        renames: &HashMap<String, String>,
        env: &mut Env,
    ) -> Option<CodeOperand> {
        match op {
            Operand::Imm(e) => {
                let v = self.eval_expr(e, env);
                if matches!(v, Value::Poison) {
                    return None;
                }
                match v.as_stored_int() {
                    Some(n) => Some(CodeOperand::Imm(n)),
                    None => {
                        self.error(
                            expr_span(e),
                            format!("immediate must be an integer, got {}", v.type_name()),
                        );
                        None
                    }
                }
            }
            Operand::Plain { expr, .. } => self.map_plain(expr, renames, env),
            Operand::Ind { parts, span, .. } => {
                let r = self.ind_single_reg(parts, *span, env)?;
                Some(CodeOperand::Ind(r))
            }
            Operand::PreDec(inner) => {
                let r = self.inner_ind_reg(inner, env)?;
                Some(CodeOperand::PreDec(r))
            }
            Operand::PostInc(inner) => {
                let r = self.inner_ind_reg(inner, env)?;
                Some(CodeOperand::PostInc(r))
            }
            Operand::DispInd { disp, inner, span } => {
                let dv = self.eval_expr(disp, env);
                if matches!(dv, Value::Poison) {
                    return None;
                }
                let Some(d) = dv.as_stored_int() else {
                    self.error(
                        *span,
                        format!("displacement must be an integer, got {}", dv.type_name()),
                    );
                    return None;
                };
                let r = self.inner_ind_reg(inner, env)?;
                Some(CodeOperand::DispInd { disp: d, reg: r })
            }
            Operand::Splice(e) => {
                let v = self.eval_expr(e, env);
                self.classify_operand_splice(v, expr_span(e))
            }
        }
    }

    /// Map a bare (`Plain`) operand expression. A single-segment path names a
    /// register (→ [`CodeOperand::Reg`]) or, failing that, a symbol / `.local`
    /// label (→ [`CodeOperand::Sym`], rewritten against `renames`). Anything else
    /// is evaluated and classified like an operand splice.
    fn map_plain(
        &mut self,
        expr: &ast::Expr,
        renames: &HashMap<String, String>,
        env: &mut Env,
    ) -> Option<CodeOperand> {
        if let ast::Expr::Path(p) = expr {
            if p.segments.len() == 1 {
                let seg = &p.segments[0];
                if let Some(r) = reg_from_name(seg) {
                    return Some(CodeOperand::Reg(r));
                }
                return Some(CodeOperand::Sym(resolve_sym(seg, renames)));
            }
        }
        let v = self.eval_expr(expr, env);
        self.classify_operand_splice(v, expr_span(expr))
    }

    /// Extract the single address/data register naming an indirect base. Only a
    /// one-part `(An)` form is supported in T3 — indexed/absolute indirects
    /// (`(d,An,Xn)`, `(Label).w`) diagnose as not-yet-supported.
    fn ind_single_reg(
        &mut self,
        parts: &[(ast::Expr, Option<TextOrSplice>)],
        span: Span,
        env: &mut Env,
    ) -> Option<Reg> {
        if parts.len() != 1 {
            self.error(span, "indexed/absolute indirect addressing is not yet supported");
            return None;
        }
        let (e, _psize) = &parts[0];
        if let ast::Expr::Path(p) = e {
            if p.segments.len() == 1 {
                if let Some(r) = reg_from_name(&p.segments[0]) {
                    return Some(r);
                }
            }
        }
        let v = self.eval_expr(e, env);
        match v {
            Value::Reg(r) => Some(r),
            Value::Poison => None,
            other => {
                self.error(
                    expr_span(e),
                    format!("indirect base must be a register, got {}", other.type_name()),
                );
                None
            }
        }
    }

    /// Extract the base register of a `-(An)` / `(An)+` inner operand (an
    /// [`Operand::Ind`]).
    fn inner_ind_reg(&mut self, inner: &Operand, env: &mut Env) -> Option<Reg> {
        match inner {
            Operand::Ind { parts, span, .. } => self.ind_single_reg(parts, *span, env),
            other => {
                self.error(
                    operand_span(other),
                    "pre-decrement / post-increment needs a register-indirect base",
                );
                None
            }
        }
    }

    /// Type a resolved operand-splice value against the operand classes: an
    /// integer → `Imm`, a [`Reg`] → `Reg`, a label ([`Value::FnRef`]/[`Value::Str`])
    /// → `Sym`. Any other kind is `[asm.splice-kind]`. This is THE place operand
    /// classes are decided (used by both `{splice}` operands and evaluated
    /// non-path `Plain` operands).
    fn classify_operand_splice(&mut self, v: Value, span: Span) -> Option<CodeOperand> {
        match v {
            Value::Poison => None,
            Value::Reg(r) => Some(CodeOperand::Reg(r)),
            Value::FnRef(n) | Value::Str(n) => Some(CodeOperand::Sym(n)),
            other => {
                if let Some(n) = other.as_stored_int() {
                    Some(CodeOperand::Imm(n))
                } else {
                    self.splice_kind_err(span, "int, Reg, or Sym", &other);
                    None
                }
            }
        }
    }

    /// Emit the `[asm.splice-kind]` diagnostic (§6.2 `~describe`): name the
    /// expected operand class and the value's `type_name()` for the got side.
    fn splice_kind_err(&mut self, span: Span, expected: &str, got: &Value) {
        self.error(
            span,
            format!("[asm.splice-kind] expected {expected}, got {}", got.type_name()),
        );
    }
}

/// A literal size-suffix string (`b`/`w`/`l`/`s`) to its [`Width`].
fn width_from_text(t: &str) -> Option<Width> {
    Some(match t {
        "b" => Width::B,
        "w" => Width::W,
        "l" => Width::L,
        "s" => Width::S,
        _ => return None,
    })
}

/// A register name (`d0`..`d7`, `a0`..`a7`) to its [`Reg`], else `None`.
fn reg_from_name(name: &str) -> Option<Reg> {
    Some(match name {
        "d0" => Reg::D0,
        "d1" => Reg::D1,
        "d2" => Reg::D2,
        "d3" => Reg::D3,
        "d4" => Reg::D4,
        "d5" => Reg::D5,
        "d6" => Reg::D6,
        "d7" => Reg::D7,
        "a0" => Reg::A0,
        "a1" => Reg::A1,
        "a2" => Reg::A2,
        "a3" => Reg::A3,
        "a4" => Reg::A4,
        "a5" => Reg::A5,
        "a6" => Reg::A6,
        "a7" => Reg::A7,
        _ => return None,
    })
}

/// Rewrite a symbol/label reference against the instantiation's rename map: a
/// `.local` (or bare) name whose key is a renamed local label resolves to the
/// fresh symbol; an external symbol passes through unchanged.
fn resolve_sym(name: &str, renames: &HashMap<String, String>) -> String {
    let key = name.strip_prefix('.').unwrap_or(name);
    // TODO(T5 hygiene): an undefined `.local` reference (no matching label in
    // this instantiation) currently keeps its bare `.name` and surfaces only as
    // an unresolved symbol at link. Full hygiene (T5) should diagnose it at eval
    // time as an unknown local label.
    renames.get(key).cloned().unwrap_or_else(|| name.to_string())
}

/// The span of an operand, for diagnostics on the inner-operand paths.
fn operand_span(op: &Operand) -> Span {
    match op {
        Operand::Imm(e) => expr_span(e),
        Operand::PreDec(inner) | Operand::PostInc(inner) => operand_span(inner),
        Operand::Ind { span, .. }
        | Operand::DispInd { span, .. }
        | Operand::Plain { span, .. } => *span,
        Operand::Splice(e) => expr_span(e),
    }
}
