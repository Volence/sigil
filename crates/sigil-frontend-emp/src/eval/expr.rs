//! Pure expression evaluation (T2/T3): literals, paths, unary/binary
//! operators, ranges, array/tuple/struct literals, and the `eval_expr`
//! dispatch that ties them together.
use super::{Env, Evaluator, Flow};
use crate::ast::{self, BinOp, UnOp};
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
    /// `Call`, user-struct `StructLit`, `If`, and `For` are handled by other
    /// tasks; `Asm` produces a [`Value::Code`](crate::value::Value::Code) (T3).
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
            // `asm { }` evaluates to a resolved `Value::Code` (T3, D-P4.3): each
            // statement becomes a `CodeItem` with its `{splice}`s resolved and
            // typed HERE (not deferred), and non-`export` labels renamed fresh
            // per instantiation for hygiene.
            ast::Expr::Asm { body, span } => self.eval_asm(body, *span, env),
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
            // `Data.empty` — the `Data` monoid's identity (T7, §6.8). A bare path
            // (payload-carrying `byte`/`bytes` parse as calls); `Data` is not a
            // user type, so this cannot be shadowed by an enum/const.
            if a == "Data" && b == "empty" {
                return Value::Data(crate::value::DataBuf::empty());
            }
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
    pub(super) fn eval_equality(&self, op: BinOp, lhs: &Value, rhs: &Value) -> Value {
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
            // The `Data` monoid `++` (T7, §6.8): append cell lists, sum sizes.
            (Value::Data(a), Value::Data(b)) => {
                Value::Data(crate::value::DataBuf::concat(a, b))
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

    // ---- diagnostic helpers ------------------------------------------------

    /// Report an integer-overflow error for operator `sym` and return `Poison`.
    pub(super) fn arith_overflow(&mut self, span: Span, sym: &str) -> Value {
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
    pub(super) fn binop_type_error(&mut self, span: Span, sym: &str, lhs: &Value, rhs: &Value) -> Value {
        self.error(
            span,
            format!("`{sym}` not defined for {} and {}", lhs.type_name(), rhs.type_name()),
        );
        Value::Poison
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

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Int(x), Value::Float(y)) | (Value::Float(y), Value::Int(x)) => (*x as f64) == *y,
        _ => a == b,
    }
}

/// The source spelling of a binary operator, for diagnostics.
pub(super) fn binop_symbol(op: BinOp) -> &'static str {
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
