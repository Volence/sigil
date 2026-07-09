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
                // An overlay name (`sizeof(V)`) is not a data-layout `Ty`, so
                // check the overlay index FIRST (D6.A9) — its laid-out size is
                // the answer. Anything else falls through to type sizing.
                if let Some(oname) = overlay_name(ty) {
                    if self.overlays.contains_key(oname.as_str()) {
                        let info = self.overlay_layout(&oname, *span);
                        return if info.poisoned { Value::Poison } else { Value::Int(info.size) };
                    }
                }
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
                // `offsetof(V, f)` on an overlay name = the field's offset WITHIN
                // the overlay (NOT including the window offset — D6.A9). Checked
                // before type resolution, since `V` is not a data-layout `Ty`.
                if let Some(oname) = overlay_name(ty) {
                    if self.overlays.contains_key(oname.as_str()) {
                        let info = self.overlay_layout(&oname, *span);
                        if info.poisoned {
                            return Value::Poison;
                        }
                        return match info.fields.iter().find(|(n, _, _)| n == field_name) {
                            Some((_, off, _)) => Value::Int(*off),
                            None => {
                                self.error(
                                    *span,
                                    format!("offsetof: overlay {oname} has no field {field_name}"),
                                );
                                Value::Poison
                            }
                        };
                    }
                }
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
                    if self.consts.contains_key(name) || self.equs.contains_key(name) {
                        return self.resolve_const(name, path.span);
                    }
                    // A `-D NAME=INT` comptime define (sound-migration T2
                    // Task 1): an already-resolved int seeded by
                    // `seed_defines`, returned directly — there is no backing
                    // `ast::ConstDecl` to route through `resolve_const`'s
                    // memo/cycle machinery (a resolved int can't cycle).
                    if let Some(v) = self.defines.get(name) {
                        return Value::Int(*v);
                    }
                    if self.fns.contains_key(name) {
                        return Value::FnRef(name.to_string());
                    }
                    // D-PP.3 label-value FALLBACK: in comptime VALUE position
                    // (a data-item field initializer or a call argument), a name
                    // the evaluator does not know is a DEFERRED LINK SYMBOL — a
                    // `proc`/`data` reference resolved at link, exactly as the
                    // string form `"init"` is. Confined to `label_ctx` so a pure
                    // comptime expression keeps its loud `unknown name`. Existing
                    // name resolution (local → const → fn, above) WINS, so a
                    // same-named const shadows the label interpretation (D-PP.3
                    // precedence). Registers win earlier still, in `eval_call_arg`.
                    if self.label_ctx_active() {
                        return Value::Label(name.to_string());
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
            // Step 1: does `a` resolve to a *value* (local binding, then const,
            // then a module-local DATA ITEM's comptime value)? Local/const win
            // first (existing precedence, D2.12). The data-item receiver is the
            // D-PP.5 VALUE half (`Def.art`): a module-local data item with a
            // struct-literal initializer is evaluated lazily (with cycle
            // detection) and its field read — this WINS before the U3 label
            // fallback, so a data item named like a proc resolves to its field
            // value, not a link symbol.
            let a_val = if let Some(v) = env.lookup(a) {
                Some(v.clone())
            } else if self.consts.contains_key(a) {
                Some(self.resolve_const(a, path.span))
            } else if self.data_value_readable(a) {
                Some(self.resolve_data_value(a, path.span))
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
            // Step 3: an `Offsets.Member` comptime ordinal, or `Offsets.count`
            // (Spec 2 Plan 7 backlog #3, reverse direction). Unlike an
            // `Enum.Variant`, these are plain comptime ints (`Value::Int`), NOT a
            // distinct value/type — `offsets` introduces no new type, only named
            // constants. Forward emission (`dc.w target - Name`) is a separate,
            // later task; this is purely name resolution.
            if let Some(decl) = self.offsets.get(a) {
                if b == "count" {
                    return Value::Int(decl.members.len() as i128);
                }
                if let Some(index) = decl.members.iter().position(|m| m.name == b) {
                    return Value::Int(index as i128);
                }
                self.error(path.span, format!("offsets `{a}` has no member `{b}`"));
                return Value::Poison;
            }
            // Step 3b: a `Dispatch.Member` PRE-SCALED comptime ordinal, or
            // `Dispatch.count` (Spec 2 Plan 7 backlog #6, Part B — D6.B3).
            // `Name.Member` = ordinal × encoding.scale() (×2 for `word_offsets`,
            // ×4 for `long_ptrs`): the routine byte S3K stores in `routine(a0)`
            // and `add.w`s into the table. `Name.count` = member count, UNSCALED.
            // Plain comptime ints (`Value::Int`), like the `offsets` ordinals —
            // `dispatch` introduces no new type in v1 (a state newtype is #9).
            if let Some(decl) = self.dispatches.get(a) {
                if b == "count" {
                    return Value::Int(decl.members.len() as i128);
                }
                if let Some(index) = decl.members.iter().position(|m| m.name == b) {
                    return Value::Int(index as i128 * decl.encoding.scale());
                }
                self.error(path.span, format!("dispatch `{a}` has no member `{b}`"));
                return Value::Poison;
            }
        }
        // Any other multi-segment path (module paths, unknown enums).
        let full = path.segments.join(".");
        // D-PP.3: a DOTTED bareword in comptime VALUE position that resolves to
        // nothing above (not a value field-access, not an Enum/offsets/dispatch
        // member) is a module-qualified LINK SYMBOL — `pitcher_plant.init`,
        // `badniks.pitcher_plant.init` — deferred to link exactly as the string
        // form `"pitcher_plant.init"` is (both resolve through the same
        // `canonicalize_name` module-suffix rule). Confined to `label_ctx`.
        if self.label_ctx_active() {
            return Value::Label(full);
        }
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
            // `data.len` (R7m.7): the comptime BYTE length of a `Value::Data`
            // buffer — the same answer the call form gives (`eval_builtin`), so a
            // bare-path `Kick.len` and a `Kick.len()` call agree. `DataBuf::size`
            // is kept in step with the cells by `push`/`concat`.
            Value::Data(buf) if field == "len" => Value::Int(buf.size as i128),
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
        // D-H.2: a provisional operand lifts. `-x` → IR `Neg`; `~x` → IR `Not`
        // (bitwise complement); logical `!x` on a link value has no direct IR
        // node, so it becomes `x == 0` (its neutral 0/1 truth value at link).
        if let Value::LinkExpr(e) = v {
            use sigil_ir::expr::{BinOp as IrBin, Expr as IrExpr, UnOp as IrUn};
            let lifted = match op {
                UnOp::Neg => IrExpr::Unary { op: IrUn::Neg, operand: Box::new(e) },
                UnOp::BitNot => IrExpr::Unary { op: IrUn::Not, operand: Box::new(e) },
                UnOp::Not => IrExpr::Binary {
                    op: IrBin::Eq,
                    lhs: Box::new(e),
                    rhs: Box::new(IrExpr::Int(0)),
                },
            };
            return Value::LinkExpr(lifted);
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
        // Short-circuit `&&`/`||` cannot short-circuit on a link-time operand
        // (its truth value is unknown until link), so a `LinkExpr` on either side
        // must build a residual `LogAnd`/`LogOr` tree instead — route them through
        // the same non-short-circuit lift path as every other operator by falling
        // through when a provisional operand is present. `eval_logical` still owns
        // the fully-comptime case.
        if matches!(op, BinOp::And | BinOp::Or) {
            let lhs = self.eval_expr(lhs_e, env);
            if matches!(lhs, Value::LinkExpr(_)) {
                let rhs = self.eval_expr(rhs_e, env);
                return self.lift_binary(op, lhs, rhs, span);
            }
            // Not a provisional LHS: replay the standard short-circuit path with
            // the already-evaluated LHS, so a `LinkExpr` RHS after a comptime LHS
            // still lifts (via `lift_binary` from inside `eval_logical_with_lhs`).
            return self.eval_logical_with_lhs(op, lhs, rhs_e, span, env);
        }
        let lhs = self.eval_expr(lhs_e, env);
        let rhs = self.eval_expr(rhs_e, env);
        // D-P2.9: poison in either operand yields poison with no new diagnostic.
        if matches!(lhs, Value::Poison) || matches!(rhs, Value::Poison) {
            return Value::Poison;
        }
        // D-H.2: a PROVISIONAL `here()` operand (a `LinkExpr`) makes the whole op
        // a residual link-time expression — build the `Expr` tree instead of
        // folding. Any operand mix where at least one side is `LinkExpr` lifts;
        // an `Int` lifts via `Expr::Int` (range-checked i128→i64).
        if matches!(lhs, Value::LinkExpr(_)) || matches!(rhs, Value::LinkExpr(_)) {
            return self.lift_binary(op, lhs, rhs, span);
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
    fn eval_logical_with_lhs(
        &mut self,
        op: BinOp,
        lhs: Value,
        rhs_e: &ast::Expr,
        span: Span,
        env: &mut Env,
    ) -> Value {
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
        // A provisional RHS (`true && here() < N`, the LHS did not short-circuit)
        // builds a residual tree — the comptime LHS is a known `Bool` here, so the
        // whole op reduces to the RHS's link-time truth value (D-H.2).
        if matches!(rhs, Value::LinkExpr(_)) {
            return self.lift_binary(op, Value::Bool(lb), rhs, span);
        }
        match rhs {
            Value::Bool(b) => Value::Bool(b),
            other => self.operand_type_error(span, binop_symbol(op), &other),
        }
    }

    /// Build a residual link-time expression for `lhs op rhs` where at least one
    /// operand is a [`Value::LinkExpr`] (D-H.2). Lifts each operand to an IR
    /// [`Expr`](sigil_ir::expr::Expr) (an `Int`/`Bool` via a literal, a `LinkExpr`
    /// verbatim), maps the operator, and wraps the tree back in a `LinkExpr`. A
    /// non-liftable operand (a non-integer mixed with a link value, an i64
    /// overflow, or the `++` operator IR cannot carry) is a loud error.
    fn lift_binary(&mut self, op: BinOp, lhs: Value, rhs: Value, span: Span) -> Value {
        if matches!(lhs, Value::Poison) || matches!(rhs, Value::Poison) {
            return Value::Poison;
        }
        let Some(ir_op) = ast_binop_to_ir(op) else {
            return self.binop_type_error(span, binop_symbol(op), &lhs, &rhs);
        };
        let l = match lift_to_link_expr(&lhs) {
            Ok(e) => e,
            Err(reason) => {
                self.error(span, format!("[here.provisional] {reason}"));
                return Value::Poison;
            }
        };
        let r = match lift_to_link_expr(&rhs) {
            Ok(e) => e,
            Err(reason) => {
                self.error(span, format!("[here.provisional] {reason}"));
                return Value::Poison;
            }
        };
        Value::LinkExpr(sigil_ir::expr::Expr::Binary {
            op: ir_op,
            lhs: Box::new(l),
            rhs: Box::new(r),
        })
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
        // A provisional `here()` bound cannot size a comptime range (D-H.2) —
        // e.g. the §7.1 `rept $38 - here()` gap-fill idiom after a relaxable.
        if let Some(v) = self.reject_if_provisional(&lo_v, span) {
            return v;
        }
        if let Some(v) = self.reject_if_provisional(&hi_v, span) {
            return v;
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

    /// Emit the `[here.provisional]` error (D-H.2) and return `Poison`. A
    /// PROVISIONAL `here()` — one after a size-relaxable instruction — is a
    /// [`Value::LinkExpr`], an integer known only after `resolve_layout`. It may
    /// only be LIFTED through the comptime operators IR `Expr` represents, EMITTED
    /// plainly (D-H.3), or GUARDED (D-H.4); every other site that consumes it as a
    /// concrete comptime integer (array length, `rept`/`if`/`while`, index, slice,
    /// `.map`, `max_size`, charmap, float ops, an arithmetic-then-emit) refuses
    /// loudly here rather than silently checking or sizing against a stale value.
    pub(crate) fn here_provisional_error(&mut self, span: Span) -> Value {
        self.error(
            span,
            "[here.provisional] `here()` after a size-relaxable instruction (jbra/jbsr, an \
             unsized branch, or a bare jmp/jsr) is a link-time value; it cannot size or steer \
             comptime evaluation — pin branch sizes (bra.s/bra.w, jmp) before this point, or \
             restructure so the value is only emitted or guarded"
                .to_string(),
        );
        Value::Poison
    }

    /// The `bankid()`-specific variant of [`here_provisional_error`](Self::here_provisional_error)
    /// (D7.3/R7m.3): a `bankid()` value in a comptime-required position gets a
    /// message that STEERS toward emission/guarding rather than the `here()`
    /// branch-sizing advice (which does not apply — bankid is link-time by
    /// construction, not because of an unsized branch). Same `[bank.provisional]`
    /// refusal class, its own steering text.
    pub(crate) fn bank_provisional_error(&mut self, span: Span) -> Value {
        self.error(
            span,
            "[bank.provisional] bankid() is a link-time value; it cannot size or steer comptime \
             evaluation — emit it into a data cell or guard it with ensure"
                .to_string(),
        );
        Value::Poison
    }

    /// If `v` is a [`Value::LinkExpr`], emit the refusal error and return
    /// `Some(Poison)` — the caller (a site that needs a concrete comptime value)
    /// short-circuits on it. `None` means `v` is not a provisional link value, so
    /// the caller proceeds unchanged. The single choke point every comptime
    /// consumer routes a possibly-provisional value through.
    ///
    /// The message STEERS by provenance (R7m.3): a residual tree carrying the
    /// Genesis bank-latch mask (`$7F8000`) is a `bankid()`-derived value — that
    /// mask appears ONLY in `eval_bankid` (D7.3), so its presence is a reliable
    /// provenance marker without threading a tag through every `LinkExpr` site.
    /// Such a value gets `[bank.provisional]`; every other link-time value
    /// (chiefly a provisional `here()`) keeps `[here.provisional]`.
    pub(crate) fn reject_if_provisional(&mut self, v: &Value, span: Span) -> Option<Value> {
        match v {
            Value::LinkExpr(e) if expr_carries_bank_mask(e) => {
                Some(self.bank_provisional_error(span))
            }
            Value::LinkExpr(_) => Some(self.here_provisional_error(span)),
            _ => None,
        }
    }
}

/// The Genesis cartridge bank-latch mask: bits 15–22 of a 68k ROM address form
/// the 9-bit-latch bank id (D7.3). Shared by `eval_bankid` (which builds it into
/// the residual tree) and `expr_carries_bank_mask` (which scans for it), so "the
/// mask appears in one place" is a compile-time fact, not a comment.
pub(crate) const BANK_MASK: i64 = 0x7F8000;

/// Whether a residual link-time tree carries the Genesis bank-latch mask
/// (`$7F8000`) — the marker of a `bankid()`-derived value (D7.3/R7m.3). The mask
/// is built into the tree ONLY by `eval_bankid` (both sites share [`BANK_MASK`],
/// so the one-place invariant is compiler-enforced), making a structural scan an
/// honest, non-invasive provenance check (no tag threaded through the tuple
/// variant). Recurses through the operator tree so a composed value
/// (`bankid(A) == bankid(B)`) is still recognized. Accepted trade: a USER-written
/// `& $7F8000` over a provisional value also matches, yielding the bank-flavored
/// refusal on an already-erroring path — wrong wording at worst, never wrong
/// behavior.
fn expr_carries_bank_mask(e: &sigil_ir::expr::Expr) -> bool {
    use sigil_ir::expr::Expr;
    match e {
        Expr::Int(n) => *n == BANK_MASK,
        Expr::Sym(_) => false,
        Expr::Unary { operand, .. } => expr_carries_bank_mask(operand),
        Expr::Binary { lhs, rhs, .. } => {
            expr_carries_bank_mask(lhs) || expr_carries_bank_mask(rhs)
        }
    }
}

/// Lift a comptime [`Value`] into an IR [`Expr`](sigil_ir::expr::Expr) operand of
/// a residual link-time expression (D-H.2). A [`Value::LinkExpr`] contributes its
/// residual tree; a bare integer (`Int`, or a `Typed` erasing to one) lifts via
/// [`Expr::Int`] after a checked i128 → i64 narrowing. `Err` carries a human
/// reason: a value that is neither (a non-integer mixed with a `LinkExpr`), or an
/// integer that does not fit i64 — the caller turns it into a diagnostic.
pub(super) fn lift_to_link_expr(v: &Value) -> Result<sigil_ir::expr::Expr, String> {
    if let Value::LinkExpr(e) = v {
        return Ok(e.clone());
    }
    match v.as_stored_int() {
        Some(n) => match i64::try_from(n) {
            Ok(i) => Ok(sigil_ir::expr::Expr::Int(i)),
            Err(_) => Err(format!(
                "value {n} does not fit a 64-bit link-time expression operand"
            )),
        },
        None => Err(format!(
            "a {} value cannot combine with a link-time `here()` value",
            v.type_name()
        )),
    }
}

/// Map an AST [`BinOp`] to its IR [`BinOp`](sigil_ir::expr::BinOp) counterpart for
/// building a residual link expression (D-H.2). Returns `None` for the operators
/// IR `Expr` cannot carry (`++` concat) — those never combine with a `LinkExpr`.
pub(super) fn ast_binop_to_ir(op: BinOp) -> Option<sigil_ir::expr::BinOp> {
    use sigil_ir::expr::BinOp as Ir;
    Some(match op {
        BinOp::Add => Ir::Add,
        BinOp::Sub => Ir::Sub,
        BinOp::Mul => Ir::Mul,
        BinOp::Div => Ir::Div,
        BinOp::Mod => Ir::Mod,
        BinOp::Shl => Ir::Shl,
        BinOp::Shr => Ir::Shr,
        BinOp::BitAnd => Ir::And,
        BinOp::BitOr => Ir::Or,
        BinOp::BitXor => Ir::Xor,
        BinOp::Eq => Ir::Eq,
        BinOp::Ne => Ir::Ne,
        BinOp::Lt => Ir::Lt,
        BinOp::Le => Ir::Le,
        BinOp::Gt => Ir::Gt,
        BinOp::Ge => Ir::Ge,
        BinOp::And => Ir::LogAnd,
        BinOp::Or => Ir::LogOr,
        BinOp::Concat => return None,
    })
}

/// Coerce a numeric value to `f64` for mixed Int/Float promotion; `None` for
/// non-numeric kinds. `pub(super)` so [`float_ns`](super::float_ns) reuses the
/// exact same coercion for `as.*`/`math.*` argument evaluation (Spec 2,
/// Plan 5 — Task 4) instead of duplicating it.
pub(super) fn num_f64(v: &Value) -> Option<f64> {
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

/// The single-segment name of a `sizeof(T)`/`offsetof(T, f)` type argument, if it
/// is a bare `Named` path — the only shape an overlay name can take. Returns
/// `None` for pointers, arrays, multi-segment paths, etc., so the caller falls
/// through to ordinary type resolution (Spec 2, Plan 7 #6 — D6.A9).
fn overlay_name(ty: &ast::Type) -> Option<String> {
    match ty {
        ast::Type::Named(path) if path.segments.len() == 1 => Some(path.segments[0].clone()),
        _ => None,
    }
}

#[cfg(test)]
mod link_expr_tests {
    use super::{ast_binop_to_ir, lift_to_link_expr};
    use crate::ast::BinOp;
    use crate::value::Value;
    use sigil_ir::expr::{BinOp as IrBin, Expr as IrExpr};

    #[test]
    fn lift_int_narrows_and_link_expr_passes_through() {
        // A bare int lifts via Expr::Int.
        assert_eq!(lift_to_link_expr(&Value::Int(4)).unwrap(), IrExpr::Int(4));
        // A LinkExpr contributes its residual tree verbatim.
        let sym = IrExpr::Sym("__here$m$0".into());
        assert_eq!(
            lift_to_link_expr(&Value::LinkExpr(sym.clone())).unwrap(),
            sym
        );
    }

    #[test]
    fn lift_rejects_out_of_i64_range_and_non_int() {
        assert!(lift_to_link_expr(&Value::Int(i128::from(i64::MAX) + 1)).is_err());
        assert!(lift_to_link_expr(&Value::Str("x".into())).is_err());
    }

    #[test]
    fn ast_binop_maps_all_arith_logic_ops_and_refuses_concat() {
        assert_eq!(ast_binop_to_ir(BinOp::Add), Some(IrBin::Add));
        assert_eq!(ast_binop_to_ir(BinOp::Le), Some(IrBin::Le));
        assert_eq!(ast_binop_to_ir(BinOp::And), Some(IrBin::LogAnd));
        assert_eq!(ast_binop_to_ir(BinOp::Concat), None);
    }
}
