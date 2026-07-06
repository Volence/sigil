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
//! Label hygiene (T5, D-P4.6, §5.2/§5.3) is delegated to
//! [`crate::lower::hygiene`]: a monotonic counter `k` on the
//! [`Evaluator`](super::Evaluator) gives each instantiation a unique id, and a
//! [`LabelScope`] maps each source label to its emitted symbol — a non-`export`
//! `.name:` to the fresh, hidden `$asm{k}$name` (two instantiations never
//! collide; an intra-body reference rewrites to the same fresh symbol so the
//! branch resolves), and an `export .name:` to the stable, caller-visible
//! `Owner.name` (§5.2). The owner is the `proc` name for a proc body and the
//! instantiation id for a raw `asm { }`. This module only chooses the operand
//! CLASS and consults the scope; the label-symbol spelling lives in ONE place
//! (the hygiene module).

use super::{Env, Evaluator};
use crate::ast::{self, AsmStmt, InstrLine, Operand, TextOrSplice};
use crate::lower::hygiene::{LabelScope, Owner};
use crate::parser::expr_span;
use crate::value::{CodeBuf, CodeItem, CodeOperand, Reg, Value, Width};
use sigil_span::Span;

impl Evaluator<'_> {
    /// Evaluate a raw `asm { }` body to a [`Value::Code`]. Its owner scope is the
    /// instantiation itself (a fresh `k`), so an exported label is stable per
    /// §5.3 but not caller-nameable — see [`eval_asm_owned`](Self::eval_asm_owned)
    /// for the proc case (owner = the proc name).
    pub(super) fn eval_asm(&mut self, body: &[AsmStmt], span: Span, env: &mut Env) -> Value {
        self.eval_asm_owned(body, span, env, None)
    }

    /// Evaluate an `asm { }` / `proc` body to a [`Value::Code`]. `owner_name` is
    /// `Some(proc)` for a proc body (its exported labels are caller-visible as
    /// `proc.name`, §5.2) and `None` for a raw `asm { }` (owner = the
    /// instantiation id). Build the [`LabelScope`] first (mapping every source
    /// label to its emitted symbol via the hygiene model), then build one
    /// [`CodeItem`] per statement, resolving label references against that scope.
    /// A statement that fails to lower emits a diagnostic and is dropped (its
    /// `Poison`-equivalent), so one bad line does not abort the whole block.
    pub(super) fn eval_asm_owned(
        &mut self,
        body: &[AsmStmt],
        _span: Span,
        env: &mut Env,
        owner_name: Option<&str>,
    ) -> Value {
        let k = self.asm_counter;
        self.asm_counter += 1;
        let owner = match owner_name {
            Some(name) => Owner::Proc(name.to_string()),
            None => Owner::Asm(k),
        };

        // Resolve every source label to its emitted symbol up front (export →
        // `Owner.name`, non-export → fresh `$asm{k}$name`).
        let scope = LabelScope::build(
            &owner,
            k,
            body.iter().filter_map(|stmt| match stmt {
                AsmStmt::Label { name, export, .. } => Some((name.as_str(), *export)),
                _ => None,
            }),
        );

        // Build the resolved item list.
        let mut buf = CodeBuf::empty();
        for stmt in body {
            match stmt {
                AsmStmt::Label { name, export, span } => {
                    buf.push(CodeItem::Label {
                        name: scope.label_def(name),
                        export: *export,
                        span: *span,
                    });
                }
                AsmStmt::Instr(instr) => {
                    if let Some(item) = self.lower_instr_to_item(instr, &scope, env) {
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
        scope: &LabelScope,
        env: &mut Env,
    ) -> Option<CodeItem> {
        let mnemonic = self.resolve_mnemonic(&instr.mnemonic, env)?;
        let size = self.resolve_size(instr.size.as_ref(), instr.span, env)?;
        let mut ops = Vec::with_capacity(instr.operands.len());
        for op in &instr.operands {
            ops.push(self.map_operand(op, scope, env)?);
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
        scope: &LabelScope,
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
            Operand::Plain { expr, .. } => self.map_plain(expr, scope, env),
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
    /// register (→ [`CodeOperand::Reg`]) or, failing that, a `.local` / global
    /// symbol (→ [`CodeOperand::Sym`], resolved against `scope`). A MULTI-segment
    /// path is an external label reference `Owner.label` (§5.2, e.g.
    /// `bra.w foo.entry`): join it dot-wise and resolve against `scope` — it is
    /// not a local label, so it passes through as the caller-visible symbol the
    /// defining owner exported. Anything else is evaluated and classified like an
    /// operand splice.
    ///
    /// NOTE — this CHANGED prior behavior (T5): before, a bare multi-segment path
    /// fell through to `eval_expr` / value-path evaluation; now ANY bare path is a
    /// symbol reference. A comptime VALUE path in operand position must be written
    /// as `#expr` (`Operand::Imm`) or a `{splice}` (`Operand::Splice`) — so a
    /// future reader wondering why `move.l some.const, d0` is treated as a symbol
    /// `some.const` rather than that const's value is oriented here.
    fn map_plain(
        &mut self,
        expr: &ast::Expr,
        scope: &LabelScope,
        env: &mut Env,
    ) -> Option<CodeOperand> {
        if let ast::Expr::Path(p) = expr {
            if p.segments.len() == 1 {
                let seg = &p.segments[0];
                if let Some(r) = reg_from_name(seg) {
                    return Some(CodeOperand::Reg(r));
                }
                return Some(CodeOperand::Sym(scope.resolve_ref(seg)));
            }
            // `Owner.label` — a cross-body reference to an exported label. Join the
            // segments to the `Owner.label` spelling the defining owner emitted.
            return Some(CodeOperand::Sym(scope.resolve_ref(&p.segments.join("."))));
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
