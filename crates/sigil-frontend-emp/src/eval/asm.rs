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
use sigil_span::{Level, Span};

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
        let module = self.module_id.clone();
        let owner = match owner_name {
            Some(name) => Owner::Proc { module, name: name.to_string() },
            None => Owner::Asm { module, k },
        };

        // Resolve every source label to its emitted symbol up front (export →
        // `Owner.name`, non-export → owner-scoped hidden symbol).
        let scope = LabelScope::build(
            &owner,
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
                    // D-PP.1: a leading bareword that is NOT a recognized mnemonic
                    // for this section's CPU (and not jbra/jbsr) but RESOLVES to an
                    // in-scope comptime fn is a bare directive-style STATEMENT CALL
                    // — pure sugar for the paren form. The decision lives HERE (not
                    // the parser): only at lowering are BOTH the CPU's mnemonic
                    // table and the comptime-fn table in hand. Mnemonics win
                    // unconditionally, so this fires only for a bareword the
                    // mnemonic table rejects; a bareword that then resolves to
                    // nothing falls through to the unchanged "not a recognized
                    // mnemonic" error, and a non-fn value gets a specific error.
                    if let Some(spliced) = self.try_bare_statement_call(instr, env) {
                        buf.items.extend(spliced.items);
                        continue;
                    }
                    if let Some(item) = self.lower_instr_to_item(instr, &scope, env) {
                        buf.push(item);
                    }
                }
                AsmStmt::Call(expr) => {
                    // A statement-position call splices a nested template's items
                    // in (§6.2): it MUST evaluate to a `Code` value.
                    //
                    // Provenance (§9, D-P4.11) — the SMALLEST HONEST version of
                    // `ProvFrame::Comptime`. Core does NOT reserve a provenance
                    // *stack* on a `Diagnostic` (it carries a single `primary`
                    // span) nor on a `DataFragment` (a single span), so a
                    // structured `ProvFrame::Comptime { call_site, def_site }`
                    // cannot be attached to the emitted fragment here — that is
                    // FLAGGED for the checkpoint (see the T7 report). What we CAN
                    // do, Core-free and at the splice site, is name the generator
                    // CALL SITE: if evaluating the generator produced any
                    // diagnostics (an out-of-range value in the generated table, a
                    // failed `ensure`, a bad splice, …), follow them with a `Note`
                    // pointing at THIS call, so an error inside a comptime-
                    // generated table is traceable back to where it was
                    // instantiated (call_site = this call; def_site = the span the
                    // generator's own diagnostic already carries).
                    let watermark = self.diags.len();
                    let v = self.eval_expr(expr, env);
                    self.note_if_comptime_error(watermark, expr_span(expr));
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

    /// Emit the `[prov.comptime]` call-site note if a comptime GENERATOR call
    /// produced any new ERROR past `watermark` (§9, D-P4.11 — the smallest honest
    /// `ProvFrame::Comptime`; see the `AsmStmt::Call` arm's comment for why a
    /// structured provenance frame cannot attach to the fragment yet). Shared by
    /// every statement-position call spelling (paren `AsmStmt::Call`, bare
    /// D-PP.1), so the note's wording and its errors-only trigger stay in ONE
    /// place. Only an ERROR warrants the note (the message speaks of an error);
    /// a stray warning would not.
    ///
    /// D-PP.4 (named call arguments) does NOT add a third spelling here: named
    /// args are paren-form only (see `operand_to_arg`'s doc comment below for
    /// why the bare spelling's operand grammar cannot represent `name: expr`
    /// without ambiguity) — `bind_args` in `eval/call.rs` is the one binder both
    /// existing spellings already share.
    fn note_if_comptime_error(&mut self, watermark: usize, call_site: Span) {
        let new_error = self.diags[watermark..].iter().any(|d| d.level == Level::Error);
        if new_error {
            self.note(
                call_site,
                "[prov.comptime] error is inside a table generated by this comptime call"
                    .to_string(),
            );
        }
    }

    /// Try to interpret `instr` as a bare directive-style statement call
    /// (D-PP.1). Returns:
    ///  - `Some(code)` — the line WAS a bare call: `code` is the instantiated
    ///    template (possibly empty on a non-Code / arg error, so the caller does
    ///    NOT also emit the "unrecognized mnemonic" error).
    ///  - `None` — the line is NOT a bare call (a real instruction, or a bareword
    ///    resolving to nothing): the caller lowers it as an instruction unchanged,
    ///    preserving today's diagnostics EXACTLY.
    ///
    /// A candidate is a leading single plain-text mnemonic with NO size suffix
    /// (`set_timer`, `nop_twice`) whose word the section's mnemonic table rejects.
    /// Mnemonics win unconditionally, so a size-suffixed or spliced mnemonic, or
    /// any recognized mnemonic, is never a bare call. The CPU must be known (a
    /// proc body, not a raw `asm {}` template) for the mnemonic decision.
    fn try_bare_statement_call(&mut self, instr: &InstrLine, env: &mut Env) -> Option<CodeBuf> {
        let cpu = self.cpu?;
        // Only a bare, single, literal mnemonic word with no size is a candidate.
        if instr.size.is_some() || instr.mnemonic.len() != 1 {
            return None;
        }
        let TextOrSplice::Text(name) = &instr.mnemonic[0] else { return None };
        // Mnemonics win unconditionally (tenet 3).
        if crate::lower::is_recognized_mnemonic(name, cpu) {
            return None;
        }
        // Not a mnemonic. What does the bareword resolve to?
        if self.fns.contains_key(name.as_str()) {
            // An in-scope comptime fn → a statement call. Convert each operand back
            // to a call-argument expression (the parser saw the line as an
            // instruction; reverse the operand normalization), then dispatch
            // through the SAME call machinery the paren form uses.
            let mut args = Vec::with_capacity(instr.operands.len());
            for op in &instr.operands {
                match operand_to_arg(op) {
                    Some(a) => args.push(a),
                    None => {
                        self.error(
                            operand_span(op),
                            "a bare statement-call argument must be a comptime expression \
                             (an addressing-mode operand is not a valid argument)",
                        );
                        return Some(CodeBuf::empty());
                    }
                }
            }
            let callee = ast::Path { segments: vec![name.clone()], span: instr.span };
            let watermark = self.diags.len();
            let v = self.eval_call(&callee, &args, instr.span, env);
            self.note_if_comptime_error(watermark, instr.span);
            match v {
                Value::Code(inner) => return Some(inner),
                Value::Poison => return Some(CodeBuf::empty()),
                other => {
                    self.error(
                        instr.span,
                        format!(
                            "a bare statement call must evaluate to Code, got {} — only a \
                             comptime fn returning Code is legal in statement position",
                            other.type_name()
                        ),
                    );
                    return Some(CodeBuf::empty());
                }
            }
        }
        // A non-fn comptime value (const/enum/struct/newtype name) in statement
        // position: a specific error naming what it is. Registers are NOT checked
        // here — a bare register word in statement position is not a call and
        // stays the mnemonic error (it names no fn).
        if let Some(kind) = self.bareword_non_fn_kind(name) {
            self.error(
                instr.span,
                format!(
                    "`{name}` names {kind}, not a comptime fn — only comptime fn calls are legal \
                     in statement position"
                ),
            );
            return Some(CodeBuf::empty());
        }
        // Resolves to nothing → NOT a bare call. Fall through to the unchanged
        // "not a recognized mnemonic" error.
        None
    }

    /// Classify a statement-position bareword that is NOT a comptime fn but IS a
    /// known non-fn comptime construct, for the specific "`X` names <kind>" error
    /// (D-PP.1). Returns `None` for a name that resolves to nothing (which keeps
    /// today's mnemonic error). Each kind carries its own article ("a const",
    /// "an enum") so the caller's message stays grammatical without an
    /// article-selection branch at the format site.
    fn bareword_non_fn_kind(&self, name: &str) -> Option<&'static str> {
        if self.consts.contains_key(name) {
            Some("a const")
        } else if self.structs.contains_key(name) {
            Some("a struct")
        } else if self.enums.contains_key(name) {
            Some("an enum")
        } else if self.newtypes.contains_key(name) {
            Some("a newtype")
        } else if self.bitfields.contains_key(name) {
            Some("a bitfield")
        } else {
            None
        }
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
            ops.push(self.map_operand(op, scope, env, size)?);
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
        width: Option<Width>,
    ) -> Option<CodeOperand> {
        match op {
            Operand::Imm(e) => {
                let v = self.eval_expr(e, env);
                if matches!(v, Value::Poison) {
                    return None;
                }
                // A provisional here() immediate gets the SPECIFIC D-H.2
                // steering message, not the generic "must be an integer".
                if self.reject_if_provisional(&v, expr_span(e)).is_some() {
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
                // D6.A3: a BARE single-segment displacement `f(aN)` on a register
                // whose declared param type bottoms out at `*S` resolves ONLY in
                // FIELD SPACE (S's direct fields ∪ in-scope overlays over S) — a
                // field name lowers to its byte offset, and a const never silently
                // shadows it. Peek the register WITHOUT reporting (a bad register
                // is diagnosed on the shared path below, preserving today's
                // diagnostics); only the field-space case diverges.
                if let Some(field) = single_segment_field(disp) {
                    if let Some(reg) = self.peek_inner_reg(inner) {
                        if let Some(base) = self.reg_pointee_struct.get(&reg).cloned() {
                            let (d, size) =
                                self.resolve_field_disp(&base, field, expr_span(disp))?;
                            // Overrun is diagnosed but the operand is emitted anyway
                            // (deliberate error-recovery): the displacement is valid,
                            // so downstream passes still see a well-formed operand.
                            self.check_field_overrun(field, size, width, *span);
                            return Some(CodeOperand::DispInd { disp: d, reg });
                        }
                    }
                }
                // D6.A4: a QUALIFIED two-segment displacement `Qual.field(aN)`
                // resolves in field space on ANY address register (the
                // qualification is the type assertion). If `Qual` names an overlay
                // or struct → field-space resolution; otherwise (e.g. an `offsets`
                // ordinal `T.B`) fall through to comptime eval unchanged.
                if let Some((qual, field)) = two_segment_field(disp) {
                    if self.overlays.contains_key(qual) || self.structs.contains_key(qual) {
                        if let Some(reg) = self.peek_inner_reg(inner) {
                            let (d, size) =
                                self.resolve_qualified_field(qual, field, expr_span(disp))?;
                            // Same deliberate error-recovery as the bare form.
                            self.check_field_overrun(field, size, width, *span);
                            return Some(CodeOperand::DispInd { disp: d, reg });
                        }
                    }
                }
                // All other shapes (multi-segment paths, non-path exprs, untyped
                // register) keep today's semantics: comptime-eval the disp, then
                // resolve the register — byte-for-byte unchanged.
                let dv = self.eval_expr(disp, env);
                if matches!(dv, Value::Poison) {
                    return None;
                }
                // A provisional here() displacement gets the SPECIFIC D-H.2
                // steering message.
                if self.reject_if_provisional(&dv, *span).is_some() {
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

    /// Peek the base register of a `(aN)` inner operand WITHOUT emitting any
    /// diagnostic (D6.A3). Only a one-part register-indirect `(aN)` yields a
    /// register; anything else (indexed/absolute, a non-register base) yields
    /// `None` and the shared displacement path below re-derives it, reporting as
    /// today. This peek is SYNTACTIC only: it matches a LITERAL register spelling
    /// (`a0`) in the AST and never evaluates — an evaluated or aliased base (e.g.
    /// a `{splice}` or a const naming a register) yields `None` here and falls
    /// through to the shared [`inner_ind_reg`](Self::inner_ind_reg) path.
    fn peek_inner_reg(&self, inner: &Operand) -> Option<Reg> {
        let Operand::Ind { parts, .. } = inner else { return None };
        if parts.len() != 1 {
            return None;
        }
        if let ast::Expr::Path(p) = &parts[0].0 {
            if p.segments.len() == 1 {
                return reg_from_name(&p.segments[0]);
            }
        }
        None
    }

    /// Resolve a bare field name against struct `base`'s FIELD SPACE (D6.A3):
    /// `base`'s direct fields ∪ the fields of every in-scope overlay whose
    /// `base_struct` is `base`. Returns `(displacement, field-byte-size)` where
    /// the displacement is the direct field's struct offset or `window_offset +
    /// overlay-relative offset`. Zero hits → `[operand.unknown-field]` (NO const
    /// fallback on a typed register); ≥2 hits across distinct overlays →
    /// `[operand.ambiguous-field]` listing the qualified candidates.
    fn resolve_field_disp(
        &mut self,
        base: &str,
        field: &str,
        span: Span,
    ) -> Option<(i128, i128)> {
        // Direct field first (a direct field can never be shadowed by an overlay:
        // `[overlay.shadows-field]` rejects that at the overlay decl, D6.A7).
        if let Some(hit) = self.field_in_struct(base, field, span) {
            return Some(hit);
        }
        // Overlay fields: scan every in-scope overlay whose window belongs to
        // `base`. Collect qualified hits so an ambiguity can name them. The
        // overlay index is a HashMap; sort candidate names for a stable message.
        let mut overlay_names: Vec<String> = self.overlays.keys().map(|s| s.to_string()).collect();
        overlay_names.sort();
        let mut hits: Vec<(String, i128, i128)> = Vec::new();
        for oname in overlay_names {
            // Only overlays whose window belongs to `base` are candidates for the
            // bare form (the overlay-qualified form skips this base filter).
            if self.overlay_layout(&oname, span).base_struct != base {
                continue;
            }
            if let Some((disp, size)) = self.field_in_overlay(&oname, field, span) {
                hits.push((oname, disp, size));
            }
        }
        match hits.as_slice() {
            [] => {
                self.error(
                    span,
                    format!(
                        "[operand.unknown-field] `*{base}` has no field or in-scope overlay field `{field}`"
                    ),
                );
                None
            }
            [(_, disp, size)] => Some((*disp, *size)),
            many => {
                let candidates = many
                    .iter()
                    .map(|(o, _, _)| format!("{o}.{field}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.error(
                    span,
                    format!(
                        "[operand.ambiguous-field] field `{field}` is ambiguous across {candidates} — qualify it as `Overlay.{field}`"
                    ),
                );
                None
            }
        }
    }

    /// Look up a DIRECT field of struct `base` by name, returning `(struct
    /// offset, field-byte-size)` if present. Shared by the bare field-space scan
    /// (D6.A3) and the struct-qualified form (D6.A4).
    fn field_in_struct(&mut self, base: &str, field: &str, span: Span) -> Option<(i128, i128)> {
        let layout = self.layout_of_struct(base, span);
        layout
            .fields
            .iter()
            .find(|f| f.name == field)
            .map(|f| (f.offset as i128, f.size as i128))
    }

    /// Look up a field of the indexed overlay named `overlay` by name, returning
    /// `(window_offset + overlay-relative offset, field-byte-size)` if present. A
    /// poisoned overlay layout yields `None`. Shared by the bare scan (via a
    /// window-match filter, D6.A3) and the overlay-qualified form (D6.A4).
    fn field_in_overlay(
        &mut self,
        overlay: &str,
        field: &str,
        span: Span,
    ) -> Option<(i128, i128)> {
        let info = self.overlay_layout(overlay, span);
        if info.poisoned {
            return None;
        }
        info.fields
            .iter()
            .find(|(n, _, _)| n == field)
            .map(|(_, rel, size)| (info.window_offset + rel, *size))
    }

    /// D6.A4 — QUALIFIED field access `Qual.field(aN)`: a two-segment
    /// displacement path resolves in field space explicitly and is legal on ANY
    /// address register (the qualification IS the type assertion). If `qual`
    /// names an indexed overlay → resolve `field` among ITS fields; else if it
    /// names a struct → resolve `field` among its DIRECT fields; ELSE → `None`
    /// (caller falls through to today's comptime eval, so an `offsets`/const
    /// first segment keeps its ordinal meaning). A recognized qualifier with an
    /// unknown field is `[operand.unknown-field]` naming the qualifier.
    fn resolve_qualified_field(
        &mut self,
        qual: &str,
        field: &str,
        span: Span,
    ) -> Option<(i128, i128)> {
        if self.overlays.contains_key(qual) {
            if let Some(hit) = self.field_in_overlay(qual, field, span) {
                return Some(hit);
            }
            self.error(
                span,
                format!("[operand.unknown-field] overlay `{qual}` has no field `{field}`"),
            );
            return None;
        }
        if self.structs.contains_key(qual) {
            if let Some(hit) = self.field_in_struct(qual, field, span) {
                return Some(hit);
            }
            self.error(
                span,
                format!("[operand.unknown-field] struct `{qual}` has no field `{field}`"),
            );
            return None;
        }
        None
    }

    /// D6.A6: an access WIDER than the resolved field crosses a named boundary →
    /// `[operand.field-overrun]`. Narrower or equal is legal (the big-endian
    /// high-byte idiom), no lint. An unsized instruction (no `.b/.w/.l`) carries
    /// no access width here, so the check is skipped — the width is decided later
    /// by the encoder and the field boundary cannot be judged at this seam.
    fn check_field_overrun(
        &mut self,
        field: &str,
        field_size: i128,
        width: Option<Width>,
        span: Span,
    ) {
        let access = match width {
            Some(Width::B) => 1,
            Some(Width::W) => 2,
            Some(Width::L) => 4,
            // `.s` is a branch-displacement size, never an operand access width;
            // and no-suffix means "decided later" — skip in both cases.
            Some(Width::S) | None => return,
        };
        if access > field_size {
            // `width` is `Some(_)` here: the `None`/`.s` arms above already
            // returned, so match it out rather than `unwrap()`.
            let Some(w) = width else { return };
            self.error(
                span,
                format!(
                    "[operand.field-overrun] .{w} access reads {access} bytes but field `{field}` is {field_size} byte{}",
                    if field_size == 1 { "" } else { "s" },
                ),
            );
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
            // D-PP.5 — `Item.field` field-ADDRESS operand: a two-segment path
            // whose FIRST segment names a known struct-typed data item (module-
            // local OR a cross-module type-only import) and whose SECOND segment
            // names a field of that struct denotes the FIELD'S ADDRESS. Lower it
            // like the bare symbolic operand but with target `Item + offsetof`
            // (a `SymOff`, which `lower_m68k_abs_sym` turns into an `Add` fixup).
            // ONE field segment only; unknown first segment / >2 segments fall
            // through to today's `Owner.label` link-symbol behavior below.
            if p.segments.len() == 2 {
                let (item, field) = (p.segments[0].as_str(), p.segments[1].as_str());
                if let Some(struct_name) = self.data_item_struct_name(item) {
                    // A known struct-typed item: the field MUST exist (a loud
                    // comptime error naming struct+field otherwise — NOT a silent
                    // link-symbol pass-through). `field_in_struct` reports nothing
                    // on a miss, so name it here.
                    match self.field_in_struct(&struct_name, field, expr_span(expr)) {
                        Some((off, _size)) => {
                            return Some(CodeOperand::SymOff {
                                sym: scope.resolve_ref(item),
                                off,
                            });
                        }
                        None => {
                            self.error(
                                expr_span(expr),
                                format!(
                                    "[operand.unknown-field] struct `{struct_name}` (of `{item}`) has no field `{field}`"
                                ),
                            );
                            return None;
                        }
                    }
                }
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
            // A Label param spliced into an operand position (`jsr {p}`,
            // `lea {p}, a1`) produces the same symbol operand as the string form
            // (D-PP.3) — a link-time reference, byte-identical to `jsr {t}` with
            // a `string` param.
            Value::FnRef(n) | Value::Str(n) | Value::Label(n) => Some(CodeOperand::Sym(n)),
            other => {
                if let Some(n) = other.as_stored_int() {
                    Some(CodeOperand::Imm(n))
                } else if self.reject_if_provisional(&other, span).is_some() {
                    // A provisional here() splice gets the SPECIFIC D-H.2
                    // steering message, not the generic `[asm.splice-kind]`.
                    None
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
/// Thin alias for [`Reg::from_name`] (the canonical map), kept for the local
/// call sites' brevity.
fn reg_from_name(name: &str) -> Option<Reg> {
    Reg::from_name(name)
}

/// The single bare identifier of a displacement expression, if it is exactly a
/// one-segment [`ast::Expr::Path`] (D6.A3/A5). A multi-segment path, a literal,
/// arithmetic, or a call yields `None` — those keep today's comptime-eval
/// semantics (field names participate only as the ENTIRE displacement).
/// A path segment that spells a register (`a0`) is NOT a field name; excluding
/// it keeps `a0(a0)` on the comptime path where it errors as today.
fn single_segment_field(disp: &ast::Expr) -> Option<&str> {
    if let ast::Expr::Path(p) = disp {
        if p.segments.len() == 1 && reg_from_name(&p.segments[0]).is_none() {
            return Some(&p.segments[0]);
        }
    }
    None
}

/// The `(qualifier, field)` of a displacement expression that is exactly a
/// TWO-segment [`ast::Expr::Path`] (`Qual.field`, D6.A4). A path of any other
/// arity yields `None`. The caller decides whether `qualifier` names an overlay
/// or struct (field space) or falls through to comptime eval (`offsets` ordinal,
/// dotted const, …) — this helper only splits the two segments.
fn two_segment_field(disp: &ast::Expr) -> Option<(&str, &str)> {
    if let ast::Expr::Path(p) = disp {
        if p.segments.len() == 2 {
            return Some((&p.segments[0], &p.segments[1]));
        }
    }
    None
}

/// Reverse a parsed instruction [`Operand`] back into a call-argument
/// [`ast::Arg`] for a bare statement call (D-PP.1). The parser parsed the line
/// as an instruction (it cannot know it is a call until lowering resolves the
/// mnemonic), so the arguments arrived as operands; this un-does the operand
/// normalization for the shapes a comptime call argument can legitimately take:
///
///  - `Plain { expr, size: None }` — a bare expression: a register (`d0`), an
///    enum path (`Ani.Shoot`), an int/arith (`1 + 2`), a call, a struct literal.
///  - `DispInd { disp: Path(p), inner: Ind{ parts, .. } }` — the parser folds an
///    all-positional call `inner(2)` into displacement-indexed addressing
///    (parser.rs `operand`); reverse it to the `Expr::Call` it came from so a
///    NESTED call argument (`outer inner(2), d0`) round-trips.
///
/// Any addressing-mode-only shape (`#imm`, `(a0)`, `-(a7)`, `(a0)+`, a
/// size-suffixed operand, a `{splice}`) is NOT a valid call argument → `None`,
/// which the caller turns into a clear diagnostic. The reconstructed `Arg` is
/// always positional.
///
/// NAMED bare-call arguments (D-PP.4) were investigated and NOT built: the
/// operand grammar has no shape for `name: expr`. `operand()` (parser.rs) hits
/// the bareword `name`, parses it as a `Plain` expression, and then the
/// trailing `:` fails `expect_line_end_or_rbrace` — a genuine (and already
/// LOUD, pre-existing) parse error, not a silent misparse. Teaching the
/// operand grammar to accept a trailing `:` after a bare ident is exactly the
/// token shape `.name:` (a local label definition, parsed at the STATEMENT
/// level, ahead of `instr_line`) already claims one dot away from — adding it
/// to `operand()` too would need a lookahead rule to keep the two apart, for
/// no real gain: the tranche's only named-arg call site (`spawn(...)`) is
/// already paren-form. Decision: named args are PAREN-FORM ONLY (see
/// `bind_args` in `eval/call.rs`); the bare spelling stays positional-only,
/// and `f name: v` keeps its existing loud parse error unchanged (pinned by
/// `bare_form_named_looking_arg_is_a_loud_parse_error` in `tests/bare_calls.rs`).
fn operand_to_arg(op: &Operand) -> Option<ast::Arg> {
    let value = match op {
        Operand::Plain { expr, size: None, .. } => expr.clone(),
        Operand::DispInd { disp: ast::Expr::Path(callee), inner, span } => {
            // Only the folded-call shape reverses: a `(parts)` indirect with no
            // per-part or trailing size (an actual displacement `4(a0)` carries a
            // register base and IS an addressing mode, not a call).
            let Operand::Ind { parts, size: None, .. } = inner.as_ref() else { return None };
            let mut cargs = Vec::with_capacity(parts.len());
            for (e, psize) in parts {
                if psize.is_some() {
                    return None;
                }
                cargs.push(ast::Arg { name: None, value: e.clone(), span: expr_span(e) });
            }
            ast::Expr::Call { callee: callee.clone(), args: cargs, span: *span }
        }
        _ => return None,
    };
    let span = expr_span(&value);
    Some(ast::Arg { name: None, value, span })
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
