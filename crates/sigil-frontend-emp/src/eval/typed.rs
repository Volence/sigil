//! Type-aware arithmetic (T5, D-P3.3): dispatch for a binary op where at
//! least one operand is a [`Value::Typed`], the prim-underlying width-wrap
//! path, the fixed-point scale-aware path, and `rescale<I,F>(x)` (D2.10).
use super::{Env, Evaluator};
use crate::ast::{self, BinOp};
use crate::layout::Ty;
use crate::value::Value;
use sigil_span::Span;

impl<'a> Evaluator<'a> {
    /// Type-aware binary op (T5, D-P3.3): at least one operand is a
    /// [`Value::Typed`]. Resolves both operands to `(Ty, i128)` — coercing a
    /// bare `Int` into the typed operand's type (the ergonomic "typed + literal"
    /// case) — then dispatches on the effective underlying type. A `Typed` beside
    /// a non-int, non-typed operand (a `Float`, `Bool`, `Str`, …) is a plain type
    /// error.
    pub(super) fn eval_typed_binary(&mut self, op: BinOp, lhs: Value, rhs: Value, span: Span) -> Value {
        // `==`/`!=` are TOTAL (D-P2.3, `eval_equality`): they never error on a
        // type mismatch. T5's routing sends any op with a `Typed` operand here
        // first, which silently narrowed that contract — `Angle(5) == true` began
        // erroring. When a `Typed` operand is NOT numerically comparable to the
        // other (a non-int, non-same-type-`Typed` operand — e.g. `== true`, or a
        // cross-type `Angle(5) == Pos(..)`), fall back to structural (in)equality
        // rather than the width-aware numeric path (which would error). Same-type
        // `Typed == Typed` and `Typed`-vs-int coercion still take the numeric path.
        if matches!(op, BinOp::Eq | BinOp::Ne) && !typed_eq_is_numeric(&lhs, &rhs) {
            return self.eval_equality(op, &lhs, &rhs);
        }
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
    pub(crate) fn effective_underlying(&mut self, ty: &Ty, span: Span) -> Ty {
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

    /// `rescale<I,F>(x)` (T5, D2.10): reinterpret a fixed-point value under a
    /// new `fixed<I,F>` scale. `x` must be a [`Value::Typed`] whose effective
    /// underlying is [`Ty::Fixed`] (a bare `fixed<>` value or a newtype over
    /// one); anything else is a diagnostic. The stored int is shifted by the
    /// fraction-bit delta — arithmetic right shift (the `asr` the author needs)
    /// when narrowing the fraction (`F_src > f`), left shift when widening — and
    /// the result is retagged as a bare `fixed<I,F>`. So
    /// `rescale<16,16>(fixed<32,32>_value)` shifts right by 16.
    pub(super) fn eval_rescale(&mut self, i: u32, f: u32, arg: &ast::Expr, span: Span, env: &mut Env) -> Value {
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

/// Whether an `==`/`!=` between a `Typed` operand and another value should take
/// the width-aware NUMERIC comparison path (comparing stored ints) rather than
/// falling back to total structural equality. True only when both operands
/// erase to the SAME nominal type's stored int (same-type `Typed == Typed`) or
/// one is a `Typed` and the other a bare integer (the `Typed`-vs-int coercion).
/// Everything else (a `Typed` vs a `Bool`/`Str`/`Float`, or two DIFFERENT
/// nominal types) is not numerically comparable and must stay total.
fn typed_eq_is_numeric(lhs: &Value, rhs: &Value) -> bool {
    match (lhs, rhs) {
        (Value::Typed { ty: tl, val: vl }, Value::Typed { ty: tr, val: vr }) => {
            tl == tr && vl.as_stored_int().is_some() && vr.as_stored_int().is_some()
        }
        (Value::Typed { val, .. }, other) | (other, Value::Typed { val, .. }) => {
            val.as_stored_int().is_some() && other.as_stored_int().is_some()
        }
        _ => false,
    }
}

/// The source spelling of a binary operator, for diagnostics. Re-exported at
/// `pub(super)` from [`expr`](super::expr) so this module (and any other
/// `eval` sibling) can format the same operator symbol used by
/// [`eval_arith`](Evaluator::eval_arith) et al.
use super::expr::binop_symbol;
