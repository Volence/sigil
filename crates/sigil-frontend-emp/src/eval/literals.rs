//! Checked literal construction (D-P2.14/§4.5, T4, T7): struct literals
//! (value-level, or CHECKED against a declared `struct`) and bitfield
//! literals.
use super::{Env, Evaluator};
use crate::ast;
use crate::value::Value;
use sigil_span::Span;

impl<'a> Evaluator<'a> {
    /// Build a struct or bitfield value from a written literal (D-P2.14 / §4.5).
    /// A literal whose type name resolves to a `bitfield` (T4) packs its fields
    /// to the erased repr integer, per [`eval_bitfield_lit`](Self::eval_bitfield_lit).
    /// A literal naming a DECLARED `struct` is CHECKED (T7, D-P3.12) via
    /// [`eval_checked_struct_lit`](Self::eval_checked_struct_lit): unknown/
    /// duplicate fields, missing no-default fields (NO silent zero-fill), and the
    /// layout `(size:)`/`@offset` checks all fire here. A literal naming an
    /// UNDECLARED type stays value-level (the Plan-2 behaviour): each field is
    /// evaluated in order and the value is tagged with the type's last path
    /// segment, with no existence/field/size checks — comptime-only structs and
    /// forward-compatible names rely on this.
    pub(super) fn eval_struct_lit(
        &mut self,
        ty: &ast::Path,
        fields: &[(String, ast::Expr)],
        rest: bool,
        span: Span,
        env: &mut Env,
    ) -> Value {
        let ty_name = ty.segments.last().cloned().unwrap_or_default();
        if self.bitfields.contains_key(ty_name.as_str()) {
            return self.eval_bitfield_lit(&ty_name, fields, span, env);
        }
        if self.structs.contains_key(ty_name.as_str()) {
            return self.eval_checked_struct_lit(&ty_name, fields, rest, span, env);
        }
        // Undeclared type name → value-level only (Plan 2). Poison field values
        // are preserved as-is (propagate, no new diagnostic). A field initializer
        // is a comptime VALUE position, so a bareword naming a proc/data item
        // resolves to a label value (D-PP.3) — `in_label_ctx` enables that
        // fallback for the field exprs.
        let fields = self.in_label_ctx(|this| {
            fields.iter().map(|(name, e)| (name.clone(), this.eval_expr(e, env))).collect()
        });
        Value::Struct { ty_name, fields }
    }

    /// The CHECKED struct literal (T7, §4.5 / D-P3.12) for a DECLARED struct.
    ///
    /// - Each PROVIDED field must be a declared field (else `[struct.unknown-field]`)
    ///   and must not be given twice (else a duplicate diagnostic).
    /// - Each DECLARED field NOT provided uses its `= default` if it has one; a
    ///   field with NO default is `[struct.missing-field]` — there is NO silent
    ///   zero-fill (the only "zero" is a field that literally declares `= 0`,
    ///   which the default path already covers).
    /// - The struct's layout is computed ([`layout_of_struct`](Self::layout_of_struct))
    ///   so a bad `(size:)`/`@offset` still surfaces at the literal site.
    ///
    /// Field VALUE range-checking is NOT done here — that happens later at
    /// emission ([`lower_to_data`](Self::lower_to_data)). Returns a
    /// [`Value::Struct`] whose fields are in DECLARATION order (defaults filled,
    /// a missing field filled with [`Poison`](Value::Poison) so downstream
    /// lowering stays silent) so it lines up with the byte layout.
    fn eval_checked_struct_lit(
        &mut self,
        ty_name: &str,
        provided: &[(String, ast::Expr)],
        rest: bool,
        span: Span,
        env: &mut Env,
    ) -> Value {
        // Construction cycle guard (T7 review): a field's `= default` can
        // construct this SAME struct (`struct A { x: A = A{} }`), which would
        // recurse forever. This is DISTINCT from the cyclic-LAYOUT check in
        // `layout_of_struct` (a by-value self-reference is also an infinite
        // layout, but the default-eval recursion fires independently and needs
        // its own guard). On a repeat, report and poison this construction.
        //
        // NOTE: this reports the chain from the WHOLE in-progress stack (not
        // sliced from where `ty_name` first appeared, unlike the shared
        // `with_cycle_guard` helper used just below) — kept exactly as before
        // so the diagnostic text is unchanged.
        if self.struct_construct_in_progress.iter().any(|n| n == ty_name) {
            let mut chain: Vec<&str> =
                self.struct_construct_in_progress.iter().map(|s| s.as_str()).collect();
            chain.push(ty_name);
            self.error(span, format!("cyclic struct construction: {}", chain.join(" -> ")));
            return Value::Poison;
        }
        // Copy the `&'a StructDecl` out so `self` is free to be mutated across
        // the field/default eval below (mirrors `layout_of_struct`).
        let decl: &'a ast::StructDecl =
            self.structs.get(ty_name).copied().expect("caller checked the struct exists");
        // The check above already ruled out `ty_name` being in progress, and
        // nothing runs between there and here that could push it — so this
        // guard's own (re-)check can never fire. It exists to guarantee the pop
        // on every path out of `body`, replacing the three hand-written pop
        // sites (T7/T8 review) with one.
        let built = self.with_cycle_guard(
            super::CycleStack::Construct,
            ty_name,
            span,
            "struct construction",
            |this| {
                // Evaluate provided fields, checking existence and duplication.
                // Keep the evaluated values keyed by name for the
                // declaration-order rebuild.
                let mut provided_vals: Vec<(String, Value)> = Vec::with_capacity(provided.len());
                for (fname, expr) in provided {
                    // A checked-struct field initializer is a comptime VALUE
                    // position: a bareword naming a proc/data item becomes a
                    // label value (D-PP.3). `code: init` in `ObjDef{ … }` is the
                    // motivating case.
                    let v = this.in_label_ctx(|this| this.eval_expr(expr, env));
                    // A `return` (or abort) inside a field expr propagates
                    // uniformly — mirroring the sibling construction sites (T8
                    // review, Minor 3).
                    if this.aborted || this.pending_return.is_some() {
                        return None;
                    }
                    let fspan = crate::parser::expr_span(expr);
                    if !decl.fields.iter().any(|f| &f.name == fname) {
                        this.error(
                            fspan,
                            format!("[struct.unknown-field] struct {ty_name} has no field `{fname}`"),
                        );
                        continue;
                    }
                    if provided_vals.iter().any(|(n, _)| n == fname) {
                        this.error(fspan, format!("struct {ty_name}: field `{fname}` given more than once"));
                        continue;
                    }
                    provided_vals.push((fname.clone(), v));
                }
                // Rebuild in declaration order: provided value, else default,
                // else a missing-field diagnostic (no silent zero-fill).
                let mut out_fields = Vec::with_capacity(decl.fields.len());
                for field in &decl.fields {
                    if let Some((_, v)) = provided_vals.iter().find(|(n, _)| n == &field.name) {
                        out_fields.push((field.name.clone(), v.clone()));
                    } else if let (Some(default), true) = (&field.default, rest) {
                        // A `= default` field expr is likewise a value position
                        // (a default `code: init` would resolve to a label).
                        let dv = this.in_label_ctx(|this| this.eval_expr(default, env));
                        // Same leaked-return / abort bail as the provided-field loop.
                        if this.aborted || this.pending_return.is_some() {
                            return None;
                        }
                        out_fields.push((field.name.clone(), dv));
                    } else {
                        // Elision is an EXPLICIT act (S2-D13(h)): a defaulted
                        // field may only be omitted under the `..` marker, so
                        // the message offers the spelling when it applies.
                        let msg = if field.default.is_some() {
                            format!(
                                "[struct.missing-field] struct {ty_name}: field `{}` was not \
                                 provided — add it, or elide it explicitly with `..` (it has \
                                 a default)",
                                field.name
                            )
                        } else {
                            format!(
                                "[struct.missing-field] struct {ty_name}: field `{}` has no \
                                 default and was not provided",
                                field.name
                            )
                        };
                        this.error(span, msg);
                        out_fields.push((field.name.clone(), Value::Poison));
                    }
                }
                Some(out_fields)
            },
        );
        // `built` is `Some(Some(fields))` on a normal build, `Some(None)` on a
        // leaked return/abort mid-construction, and (unreachable here, since
        // the check above already passed) `None` on a detected cycle.
        let Some(out_fields) = built.flatten() else {
            return Value::Poison;
        };
        // Trigger the layout `(size:)`/`@offset`/odd-field checks at the literal
        // site (memoized, so this reports at most once per struct).
        let _ = self.layout_of_struct(ty_name, span);
        Value::Struct { ty_name: ty_name.to_string(), fields: out_fields }
    }

    /// Build a bitfield value from a written literal (T4, §4.4): each provided
    /// field is evaluated to an `Int` and range-checked against `0..=(2^bits-1)`
    /// via [`check_in_range`](Self::check_in_range) — a bitfield field's width
    /// IS a refinement, the same shared mechanism as newtype/enum bounds
    /// (D-P3.6), not a special case. A field omitted from the literal defaults
    /// to 0 (unused/omitted bits are 0). An unknown field name is a
    /// diagnostic. On success, packs to `Σ field_val << field.lsb` and returns
    /// the erased repr integer (bitfields have no runtime representation
    /// beyond their packed value, §8.3) — a failure anywhere yields `Poison`
    /// (evaluation still visits every field first, so multiple bad fields each
    /// get their own diagnostic rather than short-circuiting on the first).
    fn eval_bitfield_lit(
        &mut self,
        ty_name: &str,
        fields: &[(String, ast::Expr)],
        span: Span,
        env: &mut Env,
    ) -> Value {
        let layout = self.layout_of_bitfield(ty_name, span);
        let mut packed: i128 = 0;
        let mut poisoned = false;
        for (fname, expr) in fields {
            let v = self.eval_expr(expr, env);
            // A `return` (or abort) inside a field expr propagates uniformly —
            // mirroring the sibling construction sites (`construct_enum_payload`,
            // `eval_single_int_arg`, `eval_byte`/`eval_bytes`) so a leaked return
            // is handled the same everywhere (T8 review, Minor 3).
            if self.aborted || self.pending_return.is_some() {
                return Value::Poison;
            }
            let fspan = crate::parser::expr_span(expr);
            let Some(fl) = layout.fields.iter().find(|f| &f.name == fname) else {
                self.error(fspan, format!("bitfield {ty_name} has no field `{fname}`"));
                poisoned = true;
                continue;
            };
            // A `Value::Typed` field value erases to its stored int (§8.3).
            if let Some(n) = v.as_stored_int() {
                let max = (1i128 << fl.bits) - 1;
                if self.check_in_range(n, 0, max, fspan, &format!("bitfield field '{fname}'")) {
                    packed |= n << fl.lsb;
                } else {
                    poisoned = true;
                }
                continue;
            }
            // A provisional here() field value gets the SPECIFIC D-H.2 steering
            // message, not the generic "must be an integer".
            if self.reject_if_provisional(&v, fspan).is_some() {
                poisoned = true;
                continue;
            }
            match v {
                Value::Poison => poisoned = true,
                other => {
                    self.error(
                        fspan,
                        format!(
                            "bitfield field '{fname}' must be an integer, got {}",
                            other.type_name()
                        ),
                    );
                    poisoned = true;
                }
            }
        }
        if poisoned {
            Value::Poison
        } else {
            Value::Int(packed)
        }
    }
}
