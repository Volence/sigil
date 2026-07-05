//! Pure expression evaluation (T2/T3): literals, paths, unary/binary
//! operators, ranges, array/tuple/struct literals, and the `eval_expr`
//! dispatch that ties them together.
use super::{Env, Evaluator, Flow};
use crate::ast::{self, BinOp, UnOp};
use crate::layout::Ty;
use crate::value::Value;
use sigil_span::Span;

impl<'a> Evaluator<'a> {
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
        // Once evaluation has aborted (D-P2.16) or a `return` is pending out of an
        // expression-position `if`, short-circuit so the tree unwinds silently.
        if self.aborted || self.pending_return.is_some() {
            return Value::Poison;
        }
        if !self.bump_step() {
            self.abort(crate::parser::expr_span(expr), "step budget exceeded");
            return Value::Poison;
        }
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
            ast::Expr::Call { callee, args, span } => self.eval_call(callee, args, *span, env),
            ast::Expr::StructLit { ty, fields, span } => self.eval_struct_lit(ty, fields, *span, env),
            ast::Expr::If { cond, then, els, .. } => {
                // As an expression, an `if` yields its chosen branch's value. If
                // that branch hit `return`, stash it in `pending_return` so the
                // enclosing `exec_stmts` turns it into a fn-level `Flow::Return`.
                match self.eval_if(cond, then, els.as_deref(), env) {
                    Flow::Normal(v) => v,
                    Flow::Return(v) => {
                        self.pending_return = Some(v.clone());
                        v
                    }
                }
            }
            ast::Expr::For { var, iter, body, span } => {
                self.eval_for(var, iter, body, *span, env)
            }
            // TODO(Plan 3/4): `asm { }` lowers to a `Code` value.
            ast::Expr::Asm { .. } => Value::Poison,
            // A lambda captures the *current* env by value (D2.12): the clone
            // snapshots the defining scope, so later mutation of it cannot leak
            // into an already-constructed lambda (matches `Env`'s clone contract).
            ast::Expr::Lambda { params, body, .. } => Value::Lambda {
                params: params.clone(),
                body: body.clone(),
                captured: env.clone(),
            },
            // `sizeof(T)` (D-P3.9, T3): resolve `T` against the file's type
            // tables and size it via the layout engine. Resolution failure is
            // already diagnosed by `resolve_type`, so an already-`Poison`
            // resolved type stays silent here.
            ast::Expr::SizeOf(ty, span) => {
                let resolved = self.resolve_type(ty);
                if matches!(resolved, crate::layout::Ty::Poison) {
                    return Value::Poison;
                }
                Value::Int(self.size_of_ty(&resolved, *span) as i128)
            }
            // `offsetof(T, field)` (D-P3.9, T3): `T` must bottom out (through
            // `Refined`/`Newtype` wrappers) at a struct; `field` must be one of
            // its fields. Either failure is a diagnostic + `Poison`.
            ast::Expr::OffsetOf(ty, field_name, span) => {
                let resolved = self.resolve_type(ty);
                if matches!(resolved, crate::layout::Ty::Poison) {
                    return Value::Poison;
                }
                // `struct_name_for_offsetof` may itself report (a newtype
                // cycle) — only add the generic "not a struct" message when it
                // stayed silent, so a cyclic newtype yields one specific error.
                let before = self.diags.len();
                let Some(struct_name) = self.struct_name_for_offsetof(&resolved, *span) else {
                    if self.diags.len() == before {
                        self.error(
                            *span,
                            format!("offsetof: {} is not a struct", resolved.describe()),
                        );
                    }
                    return Value::Poison;
                };
                let layout = self.layout_of_struct(&struct_name, *span);
                match layout.fields.iter().find(|f| &f.name == field_name) {
                    Some(f) => Value::Int(f.offset as i128),
                    None => {
                        self.error(
                            *span,
                            format!("offsetof: struct {struct_name} has no field {field_name}"),
                        );
                        Value::Poison
                    }
                }
            }
            // `rescale<I,F>(x)` (T5, D2.10): retag a fixed-point value to the
            // target scale, shifting its stored int by the fraction-bit delta.
            ast::Expr::Rescale { i, f, arg, span } => self.eval_rescale(*i, *f, arg, *span, env),
            // `match` / sum-type destructuring (T6, D-P3.10-ish).
            ast::Expr::Match { scrutinee, arms, span } => {
                self.eval_match(scrutinee, arms, *span, env)
            }
        }
    }

    /// Resolve a path expression: the boolean/`none` keywords; a single name
    /// (local `Env` binding, then a file-level `const`); or a two-segment
    /// `Enum.Variant` path. Local bindings shadow file consts. Unknown names,
    /// and `Enum.Variant` for a known enum with no such variant, are errors.
    fn eval_path(&mut self, path: &ast::Path, env: &Env) -> Value {
        if path.segments.len() == 1 {
            return match path.segments[0].as_str() {
                // Booleans are single-segment paths (there is no `Expr::Bool`).
                "true" => Value::Bool(true),
                "false" => Value::Bool(false),
                // `none` maps to Unit for now; revisit if a later task
                // introduces a first-class Option value.
                "none" => Value::Unit,
                name => {
                    // Precedence (D2.12): local binding → file const → fn-ref.
                    // A bare `comptime fn` name becomes a first-class `FnRef` so
                    // it can be passed as a value (`xs.map(band_entry)`); env
                    // vars and consts still shadow a same-named fn.
                    if let Some(v) = env.lookup(name) {
                        return v.clone();
                    }
                    if self.consts.contains_key(name) {
                        return self.resolve_const(name, path.span);
                    }
                    if self.fns.contains_key(name) {
                        return Value::FnRef(name.to_string());
                    }
                    self.error(path.span, format!("unknown name `{name}`"));
                    Value::Poison
                }
            };
        }
        // A two-segment `a.b` path is, in precedence order: field access / `.len`
        // on a value `a` (struct field, or the length of an array/string/range),
        // then an `Enum.Variant` nullary value. Payload-carrying construction
        // (`Enum.Variant(x)`) parses as a `Call`, not a plain path.
        if path.segments.len() == 2 {
            let (a, b) = (path.segments[0].as_str(), path.segments[1].as_str());
            // Step 1: does `a` resolve to a *value* (local binding, then const)?
            let a_val = if let Some(v) = env.lookup(a) {
                Some(v.clone())
            } else if self.consts.contains_key(a) {
                Some(self.resolve_const(a, path.span))
            } else {
                None
            };
            if let Some(v) = a_val {
                return self.field_or_len(v, b, path.span);
            }
            // Step 2: a nullary `Enum.Variant` value. A variant that DOES
            // declare a payload (T6) cannot be referenced bare — it must be
            // called (`Enum.Variant(...)`, parsed as an `Expr::Call` and
            // handled by `eval_call`'s `construct_enum_payload`) so its
            // payload values are actually supplied.
            if let Some(decl) = self.enums.get(a) {
                if let Some(variant) = decl.variants.iter().find(|v| v.name == b) {
                    if !variant.payload.is_empty() {
                        self.error(
                            path.span,
                            format!(
                                "variant `{b}` takes {} payload value(s); use `{a}.{b}(...)`",
                                variant.payload.len()
                            ),
                        );
                        return Value::Poison;
                    }
                    return Value::Enum {
                        ty_name: a.to_string(),
                        variant: b.to_string(),
                        payload: vec![],
                    };
                }
                self.error(path.span, format!("enum `{a}` has no variant `{b}`"));
                return Value::Poison;
            }
        }
        // Any other multi-segment path (module paths, unknown enums) is an
        // unknown name for now; later plans resolve `use`d/module paths.
        let full = path.segments.join(".");
        self.error(path.span, format!("unknown name `{full}`"));
        Value::Poison
    }

    /// Resolve a bare `a.b` where `a` is a value (D-P2.17/D-P2.18): a struct
    /// field access, the `.len` of an array/string/range, or a string's `.val`
    /// (the no-arg integer-parse builtin, so `s.val` and `s.val()` are
    /// equivalent — mirroring `s.len`/`s.len` on a length). Anything else is an
    /// error yielding `Poison`; a `Poison` receiver propagates silently.
    ///
    /// Note the ordering: on a struct, `b` is *always* a field name (so a struct
    /// with a field literally named `len` reads that field, not a length).
    fn field_or_len(&mut self, v: Value, field: &str, span: Span) -> Value {
        match v {
            Value::Poison => Value::Poison,
            Value::Struct { ty_name, fields } => {
                match fields.iter().find(|(n, _)| n == field) {
                    Some((_, val)) => val.clone(),
                    None => {
                        self.error(span, format!("struct `{ty_name}` has no field `{field}`"));
                        Value::Poison
                    }
                }
            }
            Value::Array(elems) if field == "len" => Value::Int(elems.len() as i128),
            Value::Str(s) if field == "len" => Value::Int(s.chars().count() as i128),
            // The no-arg `val` builtin also reads as a bare path (`s.val`).
            Value::Str(s) if field == "val" => self.str_val(&s, span),
            // A half-open `lo..hi` has `max(0, hi - lo)` elements.
            Value::Range { lo, hi } if field == "len" => Value::Int((hi - lo).max(0)),
            other => {
                self.error(
                    span,
                    format!("`{field}` is not a field or `.len` of {}", other.type_name()),
                );
                Value::Poison
            }
        }
    }

    /// Build a struct or bitfield value from a written literal (D-P2.14). A
    /// literal whose type name resolves to a `bitfield` (T4) packs its fields
    /// to the erased repr integer, per [`eval_bitfield_lit`](Self::eval_bitfield_lit).
    /// Otherwise (a plain struct, value level only): evaluate each field in
    /// order and tag the value with the type's last path segment — no
    /// existence/field/size/default checks (that is T7).
    fn eval_struct_lit(
        &mut self,
        ty: &ast::Path,
        fields: &[(String, ast::Expr)],
        span: Span,
        env: &mut Env,
    ) -> Value {
        let ty_name = ty.segments.last().cloned().unwrap_or_default();
        if self.bitfields.contains_key(ty_name.as_str()) {
            return self.eval_bitfield_lit(&ty_name, fields, span, env);
        }
        // Poison field values are preserved as-is (propagate, no new diagnostic).
        let fields =
            fields.iter().map(|(name, e)| (name.clone(), self.eval_expr(e, env))).collect();
        Value::Struct { ty_name, fields }
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
                other => self.operand_type_error(span, "-", &other),
            },
            UnOp::Not => match v {
                Value::Bool(b) => Value::Bool(!b),
                other => self.operand_type_error(span, "!", &other),
            },
            UnOp::BitNot => match v {
                Value::Int(n) => Value::Int(!n),
                other => self.operand_type_error(span, "~", &other),
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
        // T5 (D-P3.3): if EITHER operand carries a sized nominal type, the whole
        // op is type-aware — it wraps at the underlying's width/scale and stays
        // typed. This is the ONLY place comptime arithmetic wraps; bare `Int op
        // Int` keeps the Plan-2 overflow-is-error behaviour below, untouched.
        if matches!(lhs, Value::Typed { .. }) || matches!(rhs, Value::Typed { .. }) {
            return self.eval_typed_binary(op, lhs, rhs, span);
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

    /// Type-aware binary op (T5, D-P3.3): at least one operand is a
    /// [`Value::Typed`]. Resolves both operands to `(Ty, i128)` — coercing a
    /// bare `Int` into the typed operand's type (the ergonomic "typed + literal"
    /// case) — then dispatches on the effective underlying type. A `Typed` beside
    /// a non-int, non-typed operand (a `Float`, `Bool`, `Str`, …) is a plain type
    /// error.
    fn eval_typed_binary(&mut self, op: BinOp, lhs: Value, rhs: Value, span: Span) -> Value {
        let (tl, nl, tr, nr) = match (lhs, rhs) {
            (Value::Typed { ty: tl, val: vl }, Value::Typed { ty: tr, val: vr }) => {
                match (vl.as_stored_int(), vr.as_stored_int()) {
                    (Some(a), Some(b)) => (*tl, a, *tr, b),
                    // A typed value whose stored payload is not an integer is a
                    // malformed value (already-reported upstream); stay silent.
                    _ => return Value::Poison,
                }
            }
            // `Typed ⊕ bare Int`: coerce the literal into the typed operand's
            // type, then treat it as a same-type op.
            (Value::Typed { ty, val }, other) => match (val.as_stored_int(), other.as_stored_int()) {
                (Some(a), Some(b)) => (*ty.clone(), a, *ty, b),
                _ => return self.binop_type_error(span, binop_symbol(op), &Value::Typed { ty, val }, &other),
            },
            (other, Value::Typed { ty, val }) => match (other.as_stored_int(), val.as_stored_int()) {
                (Some(a), Some(b)) => (*ty.clone(), a, *ty, b),
                _ => return self.binop_type_error(span, binop_symbol(op), &other, &Value::Typed { ty, val }),
            },
            // Unreachable: the caller only routes here when one side is `Typed`.
            (l, r) => return self.binop_type_error(span, binop_symbol(op), &l, &r),
        };
        self.typed_op(op, tl, nl, tr, nr, span)
    }

    /// Dispatch a same-or-cross-type typed op on two resolved `(Ty, i128)`
    /// operands. Resolves each nominal type to its effective underlying
    /// ([`Ty::Prim`] or [`Ty::Fixed`]) and routes to the width-wrapping prim
    /// path or the scale-aware fixed path. Cross-type mixing (different nominal
    /// types that are not a fixed/fixed scale mismatch) is `[cross-type mix]`.
    fn typed_op(&mut self, op: BinOp, tl: Ty, nl: i128, tr: Ty, nr: i128, span: Span) -> Value {
        let ul = self.effective_underlying(&tl, span);
        // Bail on a poisoned (e.g. cyclic) left underlying before resolving the
        // right — so a cycle present on both operands reports once, not twice.
        if matches!(ul, Ty::Poison) {
            return Value::Poison;
        }
        let ur = self.effective_underlying(&tr, span);
        match (&ul, &ur) {
            (Ty::Poison, _) | (_, Ty::Poison) => Value::Poison,
            (Ty::Fixed { i: il, f: fl }, Ty::Fixed { i: ir, f: fr }) => {
                self.fixed_op(op, &tl, (*il, *fl), nl, &tr, (*ir, *fr), nr, span)
            }
            (Ty::Prim { width, signed }, Ty::Prim { .. }) => {
                // Prim-underlying values must share the SAME nominal type (there
                // is no meaningful cross-newtype arithmetic — D2.9 / Appendix E).
                if tl != tr {
                    return self.cross_type_mix(&tl, &tr, span);
                }
                self.prim_op(op, tl, *width, *signed, nl, nr, span)
            }
            // A fixed mixed with a prim (or any other underlying) at different
            // nominal types is a cross-type mix.
            _ => self.cross_type_mix(&tl, &tr, span),
        }
    }

    /// Resolve a nominal [`Ty`] to its effective underlying — following
    /// [`Ty::Newtype`] chains (and [`Ty::Refined`] wrappers) down to a
    /// [`Ty::Prim`] or [`Ty::Fixed`] — for arithmetic dispatch. Cycle-guarded
    /// against the shared [`layout_in_progress`](Evaluator) stack (a
    /// `newtype A = B; newtype B = A` chain), reporting `cyclic type: {chain}`
    /// (matching the sibling guards in [`size_of_newtype`](Evaluator) etc.) and
    /// returning [`Ty::Poison`] on a detected cycle. A construction already
    /// validated the value, so this normally bottoms out cleanly.
    fn effective_underlying(&mut self, ty: &Ty, span: Span) -> Ty {
        match ty {
            Ty::Newtype(name) => {
                if let Some(start) = self.layout_in_progress.iter().position(|n| n == name) {
                    let mut chain: Vec<&str> =
                        self.layout_in_progress[start..].iter().map(|s| s.as_str()).collect();
                    chain.push(name);
                    self.error(span, format!("cyclic type: {}", chain.join(" -> ")));
                    return Ty::Poison;
                }
                let Some(decl) = self.newtypes.get(name.as_str()).copied() else {
                    return Ty::Poison;
                };
                self.layout_in_progress.push(name.to_string());
                let underlying = self.resolve_type(&decl.underlying);
                let result = self.effective_underlying(&underlying, span);
                self.layout_in_progress.pop();
                result
            }
            Ty::Refined { inner, .. } => self.effective_underlying(inner, span),
            other => other.clone(),
        }
    }

    /// A prim-underlying typed op (D2.9): `+ - * / %` and bitwise/shift compute
    /// on the stored ints then WRAP at the underlying's `width*8` bits (two's
    /// complement, respecting `signed`), staying [`Value::Typed`] with the same
    /// nominal type. Comparisons compare the stored ints and yield a bare
    /// [`Value::Bool`]. `++` is not defined on a scalar.
    #[allow(clippy::too_many_arguments)]
    fn prim_op(&mut self, op: BinOp, ty: Ty, width: u8, signed: bool, nl: i128, nr: i128, span: Span) -> Value {
        let bits = u32::from(width) * 8;
        let typed = |n: i128| Value::Typed { ty: Box::new(ty.clone()), val: Box::new(Value::Int(n)) };
        match op {
            BinOp::Add => typed(wrap_bits(nl.wrapping_add(nr), bits, signed)),
            BinOp::Sub => typed(wrap_bits(nl.wrapping_sub(nr), bits, signed)),
            BinOp::Mul => typed(wrap_bits(nl.wrapping_mul(nr), bits, signed)),
            BinOp::Div => {
                if nr == 0 {
                    self.error(span, "division by zero");
                    return Value::Poison;
                }
                typed(wrap_bits(nl.wrapping_div(nr), bits, signed))
            }
            BinOp::Mod => {
                if nr == 0 {
                    self.error(span, "modulo by zero");
                    return Value::Poison;
                }
                typed(wrap_bits(nl.wrapping_rem(nr), bits, signed))
            }
            BinOp::BitAnd => typed(wrap_bits(nl & nr, bits, signed)),
            BinOp::BitOr => typed(wrap_bits(nl | nr, bits, signed)),
            BinOp::BitXor => typed(wrap_bits(nl ^ nr, bits, signed)),
            // Shift count must be in `0..width` (the TYPE's width) — shifting an
            // 8-bit value by >= 8 is out of range, mirroring bare-int `eval_shift`
            // which errors rather than silently wrapping/clamping the count.
            BinOp::Shl | BinOp::Shr => {
                if !(0..i128::from(bits)).contains(&nr) {
                    self.error(span, format!("shift amount out of range: {nr}"));
                    return Value::Poison;
                }
                let n = nr as u32;
                if op == BinOp::Shl {
                    typed(wrap_bits(nl.wrapping_shl(n), bits, signed))
                } else {
                    typed(wrap_bits(nl >> n, bits, signed))
                }
            }
            BinOp::Eq => Value::Bool(nl == nr),
            BinOp::Ne => Value::Bool(nl != nr),
            BinOp::Lt => Value::Bool(nl < nr),
            BinOp::Le => Value::Bool(nl <= nr),
            BinOp::Gt => Value::Bool(nl > nr),
            BinOp::Ge => Value::Bool(nl >= nr),
            BinOp::Concat => {
                self.error(span, format!("`++` not defined for {}", ty.describe()));
                Value::Poison
            }
            BinOp::And | BinOp::Or => unreachable!("logical ops never reach typed_op"),
        }
    }

    /// A fixed-underlying typed op (D2.10, Appendix E case 2):
    /// - `+`/`-`: transparent at the SAME scale (wrap at `I+F` bits, signed);
    ///   DIFFERENT scale is `[scale mismatch]` naming the required `rescale`.
    ///   Same scale but a DIFFERENT nominal type (e.g. `newtype Fix =
    ///   fixed<16,16>` vs a bare `fixed<16,16>`) is a `[cross-type mix]`.
    /// - `*`: the scale COMBINES — `fixed<Il,Fl> × fixed<Ir,Fr>` →
    ///   `fixed<Il+Ir, Fl+Fr>` (so same-scale squares to `fixed<2I,2F>`), no
    ///   wrap (the result widened).
    /// - comparisons: same-scale compare stored ints; different scale is a
    ///   scale mismatch.
    /// - `/`, `%`, bitwise, shift, `++`: not defined on `fixed<>` (use `rescale`
    ///   + integer ops).
    #[allow(clippy::too_many_arguments)]
    fn fixed_op(
        &mut self,
        op: BinOp,
        tl: &Ty,
        (il, fl): (u32, u32),
        nl: i128,
        tr: &Ty,
        (ir, fr): (u32, u32),
        nr: i128,
        span: Span,
    ) -> Value {
        // Multiply combines scales regardless of nominal wrapper, producing a
        // bare `fixed<Il+Ir, Fl+Fr>` — the one op whose result type differs from
        // its operands'.
        if op == BinOp::Mul {
            let (ni, nf) = (il.saturating_add(ir), fl.saturating_add(fr));
            // The combined scale must stay representable in the i128 domain —
            // e.g. `fixed<32,32> × fixed<32,32>` would be a 128-bit fixed, past
            // where `wrap_bits` can wrap. Reject it rather than widen silently.
            if self.fixed_width_bits(ni, nf, span).is_none() {
                return Value::Poison;
            }
            let Some(prod) = nl.checked_mul(nr) else {
                return self.arith_overflow(span, "*");
            };
            return Value::Typed {
                ty: Box::new(Ty::Fixed { i: ni, f: nf }),
                val: Box::new(Value::Int(prod)),
            };
        }
        let same_scale = (il, fl) == (ir, fr);
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                if !same_scale {
                    return self.scale_mismatch(tl, (il, fl), tr, (ir, fr), span);
                }
                if tl != tr {
                    // Same scale, distinct nominal types — a real nominal mix.
                    return self.cross_type_mix(tl, tr, span);
                }
                let bits = il.saturating_add(fl);
                match op {
                    BinOp::Add => Value::Typed {
                        ty: Box::new(tl.clone()),
                        val: Box::new(Value::Int(wrap_bits(nl.wrapping_add(nr), bits, true))),
                    },
                    BinOp::Sub => Value::Typed {
                        ty: Box::new(tl.clone()),
                        val: Box::new(Value::Int(wrap_bits(nl.wrapping_sub(nr), bits, true))),
                    },
                    BinOp::Eq => Value::Bool(nl == nr),
                    BinOp::Ne => Value::Bool(nl != nr),
                    BinOp::Lt => Value::Bool(nl < nr),
                    BinOp::Le => Value::Bool(nl <= nr),
                    BinOp::Gt => Value::Bool(nl > nr),
                    BinOp::Ge => Value::Bool(nl >= nr),
                    _ => unreachable!(),
                }
            }
            _ => {
                self.error(
                    span,
                    format!(
                        "`{}` is not defined on {} (use rescale + integer ops)",
                        binop_symbol(op),
                        tl.describe()
                    ),
                );
                Value::Poison
            }
        }
    }

    /// The `[cross-type mix]` diagnostic (Appendix E case 3): two distinct
    /// nominal types cannot be combined arithmetically.
    fn cross_type_mix(&mut self, tl: &Ty, tr: &Ty, span: Span) -> Value {
        self.error(
            span,
            format!("[cross-type mix] cannot mix {} and {}", tl.describe(), tr.describe()),
        );
        Value::Poison
    }

    /// The `[scale mismatch]` diagnostic (D2.10): two `fixed<>` values of
    /// different scale met in an add/sub/comparison. Names an explicit `rescale`
    /// to a common scale — the shift the author must write themselves (never a
    /// silent shift).
    fn scale_mismatch(&mut self, tl: &Ty, (il, fl): (u32, u32), tr: &Ty, (ir, fr): (u32, u32), span: Span) -> Value {
        self.error(
            span,
            format!(
                "[scale mismatch] cannot combine {} (fixed<{il},{fl}>) and {} (fixed<{ir},{fr}>) — \
                 rescale one to a common scale, e.g. rescale<{ir},{fr}>(..)",
                tl.describe(),
                tr.describe()
            ),
        );
        Value::Poison
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
    fn eval_equality(&self, op: BinOp, lhs: &Value, rhs: &Value) -> Value {
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
            other => return self.operand_type_error(span, binop_symbol(op), &other),
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
            other => self.operand_type_error(span, binop_symbol(op), &other),
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
        // Range bounds erase a `Value::Typed` to its stored int (§8.3). An
        // empty/negative range (`lo >= hi`) is allowed here; whether it iterates
        // to nothing is decided when the range is consumed.
        match (lo_v.as_stored_int(), hi_v.as_stored_int()) {
            (Some(lo), Some(hi)) => Value::Range { lo, hi },
            _ => {
                self.error(
                    span,
                    format!(
                        "range bounds must be int, got {} and {}",
                        lo_v.type_name(),
                        hi_v.type_name()
                    ),
                );
                Value::Poison
            }
        }
    }

    /// `rescale<I,F>(x)` (T5, D2.10): reinterpret a fixed-point value under a
    /// new `fixed<I,F>` scale. `x` must be a [`Value::Typed`] whose effective
    /// underlying is [`Ty::Fixed`] (a bare `fixed<>` value or a newtype over
    /// one); anything else is a diagnostic. The stored int is shifted by the
    /// fraction-bit delta — arithmetic right shift (the `asr` the author needs)
    /// when narrowing the fraction (`F_src > f`), left shift when widening — and
    /// the result is retagged as a bare `fixed<I,F>`. So
    /// `rescale<16,16>(fixed<32,32>_value)` shifts right by 16.
    fn eval_rescale(&mut self, i: u32, f: u32, arg: &ast::Expr, span: Span, env: &mut Env) -> Value {
        let v = self.eval_expr(arg, env);
        if matches!(v, Value::Poison) {
            return Value::Poison;
        }
        let Value::Typed { ty, val } = &v else {
            self.error(span, format!("rescale expects a fixed<> value, got {}", v.type_name()));
            return Value::Poison;
        };
        let Some(stored) = val.as_stored_int() else {
            return Value::Poison;
        };
        // The target `fixed<I,F>` must itself be a usable width.
        if self.fixed_width_bits(i, f, span).is_none() {
            return Value::Poison;
        }
        let Ty::Fixed { f: src_f, .. } = self.effective_underlying(ty, span) else {
            self.error(span, format!("rescale expects a fixed<> value, got {}", ty.describe()));
            return Value::Poison;
        };
        // Shift by the fraction-bit delta: right (arithmetic) when narrowing the
        // fraction, left when widening. A pathological delta ≥ 128 saturates the
        // shifted value to 0 rather than panicking on an out-of-range shift.
        let shifted = if src_f > f {
            let d = src_f - f;
            if d >= 128 { 0 } else { stored >> d }
        } else {
            let d = f - src_f;
            if d >= 128 { 0 } else { stored << d }
        };
        Value::Typed { ty: Box::new(Ty::Fixed { i, f }), val: Box::new(Value::Int(shifted)) }
    }

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
    fn eval_match(
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

    // ---- diagnostic helpers ------------------------------------------------

    /// Report an integer-overflow error for operator `sym` and return `Poison`.
    fn arith_overflow(&mut self, span: Span, sym: &str) -> Value {
        self.error(span, format!("integer overflow in `{sym}`"));
        Value::Poison
    }

    /// Report a type error naming a single operand and return `Poison`. Used
    /// for unary operators and for a single offending operand of a logical op.
    fn operand_type_error(&mut self, span: Span, sym: &str, operand: &Value) -> Value {
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
}

/// Wrap `value` to `bits` low bits, two's-complement, honoring `signed` (T5,
/// D-P3.3 / D2.9). This is the underlying-width wrap for sized/typed arithmetic:
/// `Angle(200) + Angle(100)` (u8) → `300 & 0xFF` = 44; a signed i8 `100 + 100` →
/// 200 → sign-extends to −56. `bits` is a whole underlying width (8/16/32 for a
/// prim, `I+F` for a fixed) and always ≤ 127 in practice, so `1i128 << bits`
/// never overflows.
fn wrap_bits(value: i128, bits: u32, signed: bool) -> i128 {
    if bits == 0 || bits >= 128 {
        return value;
    }
    let mask = (1i128 << bits) - 1;
    let low = value & mask;
    if signed && (low & (1i128 << (bits - 1))) != 0 {
        // The sign bit is set — subtract 2^bits to sign-extend into `i128`.
        low - (1i128 << bits)
    } else {
        low
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
