//! Comptime integer expressions and the pure folding pass.
//!
//! Folding is a pure function of the expression tree and a symbol-lookup
//! closure. An unresolved symbol yields [`Fold::Poison`]; an arithmetic
//! fault (64-bit overflow, division by zero, a shift amount outside
//! `0..=63`) yields [`Fold::Fault`] carrying the failed operation. Both
//! propagate through every operator, and a fault outranks a poison so it is
//! reported where it happened rather than deferred to a later resolution.
//! Every consumer of a fold reports a fault: the AS front end at the line, the
//! linker at the fixup, equ or assertion. This evaluator serves both the AS
//! front end at assembly time and the linker at link time, and it refuses
//! exactly what the `.emp` comptime evaluator refuses (D-P2.1), so a value
//! that reaches a narrowing was computed without a wrap on either route.

/// Binary operators used by the Z80 driver's build-time math (catalog §3.10).
/// Comparisons appear only inside `if` and fold to `1`/`0`.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    /// Truncating integer division (toward zero), matching AS.
    Div,
    /// `#` — modulo, remainder-of-truncating-division (sign follows the
    /// dividend, i.e. Rust's `%`). asl-verified: `256#64`=0, `100#7`=2,
    /// `255#256`=255, and with negatives `(-5)#3`=-2, `5#(-3)`=2,
    /// `(-5)#(-3)`=-2 — exactly Rust's `%` / `wrapping_rem`, not Euclidean
    /// modulo. Same precedence tier as `*`/`/` (asl-verified: `7#5*2`=4,
    /// `5+7#2`=6 — `#` binds tighter than `+`).
    Mod,
    Shl,
    Shr,
    And,
    Or,
    /// `!` — bitwise XOR (asl's infix `!`; probe-verified 2026-07-04:
    /// `1!1`=0, `3!1`=2, `5!3`=6 — NOT bitwise-OR).
    Xor,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    /// Logical OR (`||`). Folds to `1`/`0` (neutral truth value), not bitwise `Or`.
    LogOr,
    /// Logical AND (`&&`). Folds to `1`/`0` (neutral truth value), not bitwise `And`.
    LogAnd,
}

/// Unary operators.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum UnOp {
    Neg,
    /// Bitwise complement (`~x`), asl's one's-complement operator.
    Not,
    /// Logical NOT (`~~x`), asl's boolean-negation operator: `1` when the
    /// operand is zero, `0` otherwise. A SEPARATE operator from `~`, not two
    /// applications of it — `~~` is one greedy token in asl, and folding it as
    /// `!!x` cancels and returns `x` unchanged.
    LogNot,
}

/// A build-time integer expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
    /// A literal integer.
    Int(i64),
    /// A named symbol reference (global `Foo`, local `.bar`, or dotted `Foo.bar`).
    /// Scope qualification is the caller's concern via the lookup closure.
    Sym(String),
    /// A binary operation.
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    /// A unary operation.
    Unary { op: UnOp, operand: Box<Expr> },
}

/// The result of folding an [`Expr`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Fold {
    /// The expression folded to a concrete integer.
    Value(i64),
    /// The expression could not be resolved (unknown symbol).
    Poison,
    /// The expression has no 64-bit value: an operation overflowed, divided
    /// by zero, or shifted by an amount outside the value's width. Never a
    /// placeholder; every consumer reports it.
    Fault(ArithFault),
}

/// What went wrong in a folded operation.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FaultKind {
    /// The mathematical result does not fit an `i64`.
    Overflow,
    /// `/` or `#` with a zero divisor.
    DivideByZero,
    /// A shift amount outside `0..=63`, the only amounts a 64-bit value has.
    ShiftRange,
}

/// The failed operation, with the operand values it was applied to, so a
/// diagnostic can name the arithmetic rather than the symptom. `lhs` is the
/// single operand of a unary `-`.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ArithFault {
    pub kind: FaultKind,
    /// The operator as written: `+ - * / # << >>`, or `neg` for unary `-`.
    pub op: &'static str,
    pub lhs: i64,
    pub rhs: i64,
}

impl std::fmt::Display for ArithFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            FaultKind::Overflow if self.op == "neg" => {
                write!(f, "arithmetic overflow: -({}) does not fit a 64-bit value", self.lhs)
            }
            FaultKind::Overflow => write!(
                f,
                "arithmetic overflow: {} {} {} does not fit a 64-bit value",
                self.lhs, self.op, self.rhs
            ),
            FaultKind::DivideByZero => {
                write!(f, "division by zero: {} {} {}", self.lhs, self.op, self.rhs)
            }
            FaultKind::ShiftRange => write!(
                f,
                "shift amount {} out of range 0..=63 in {} {} {}",
                self.rhs, self.lhs, self.op, self.rhs
            ),
        }
    }
}

impl ArithFault {
    fn binary(kind: FaultKind, op: &'static str, lhs: i64, rhs: i64) -> Fold {
        Fold::Fault(ArithFault { kind, op, lhs, rhs })
    }
}

/// `checked` or the overflow fault for `lhs op rhs`.
fn checked_or_overflow(checked: Option<i64>, op: &'static str, lhs: i64, rhs: i64) -> Fold {
    match checked {
        Some(v) => Fold::Value(v),
        None => ArithFault::binary(FaultKind::Overflow, op, lhs, rhs),
    }
}

/// A shift by `rhs` of `lhs`: the amount must be one a 64-bit value has, and
/// a left shift must round-trip (`(a << n) >> n == a`), which is the
/// `.emp` comptime rule and what makes `1 << 63` an overflow rather than
/// `i64::MIN`.
fn shift(op: &'static str, lhs: i64, rhs: i64) -> Fold {
    if !(0..=63).contains(&rhs) {
        return ArithFault::binary(FaultKind::ShiftRange, op, lhs, rhs);
    }
    let n = rhs as u32;
    if op == "<<" {
        let r = lhs << n;
        if (r >> n) != lhs {
            return ArithFault::binary(FaultKind::Overflow, op, lhs, rhs);
        }
        Fold::Value(r)
    } else {
        Fold::Value(lhs >> n)
    }
}

impl Expr {
    /// Fold this expression to a concrete integer, resolving `Sym(name)` via
    /// `lookup`. `lookup` returns `None` for an unknown symbol (→ [`Fold::Poison`]).
    pub fn fold(&self, lookup: &dyn Fn(&str) -> Option<i64>) -> Fold {
        match self {
            Expr::Int(n) => Fold::Value(*n),
            Expr::Sym(name) => match lookup(name) {
                Some(v) => Fold::Value(v),
                None => Fold::Poison,
            },
            Expr::Unary { op, operand } => {
                let v = match operand.fold(lookup) {
                    Fold::Value(v) => v,
                    other => return other,
                };
                match op {
                    UnOp::Neg => match v.checked_neg() {
                        Some(r) => Fold::Value(r),
                        None => ArithFault::binary(FaultKind::Overflow, "neg", v, 0),
                    },
                    UnOp::Not => Fold::Value(!v),
                    UnOp::LogNot => Fold::Value(i64::from(v == 0)),
                }
            }
            Expr::Binary { op, lhs, rhs } => {
                // Both sides fold before either failure is returned, so a
                // fault on the right is reported even when the left is a
                // still-unresolved symbol: a fault is final, a poison is not.
                let (a, b) = match (lhs.fold(lookup), rhs.fold(lookup)) {
                    (Fold::Value(a), Fold::Value(b)) => (a, b),
                    (Fold::Fault(f), _) | (_, Fold::Fault(f)) => return Fold::Fault(f),
                    _ => return Fold::Poison,
                };
                let bool_val = |t: bool| Fold::Value(if t { 1 } else { 0 });
                match op {
                    BinOp::Add => checked_or_overflow(a.checked_add(b), "+", a, b),
                    BinOp::Sub => checked_or_overflow(a.checked_sub(b), "-", a, b),
                    BinOp::Mul => checked_or_overflow(a.checked_mul(b), "*", a, b),
                    BinOp::Div => {
                        if b == 0 {
                            ArithFault::binary(FaultKind::DivideByZero, "/", a, b)
                        } else {
                            // i64 `/` truncates toward zero (matches AS);
                            // `checked_div` also refuses `i64::MIN / -1`.
                            checked_or_overflow(a.checked_div(b), "/", a, b)
                        }
                    }
                    BinOp::Mod => {
                        if b == 0 {
                            ArithFault::binary(FaultKind::DivideByZero, "#", a, b)
                        } else {
                            checked_or_overflow(a.checked_rem(b), "#", a, b)
                        }
                    }
                    BinOp::Shl => shift("<<", a, b),
                    BinOp::Shr => shift(">>", a, b),
                    BinOp::And => Fold::Value(a & b),
                    BinOp::Or => Fold::Value(a | b),
                    BinOp::Xor => Fold::Value(a ^ b),
                    BinOp::Eq => bool_val(a == b),
                    BinOp::Ne => bool_val(a != b),
                    BinOp::Lt => bool_val(a < b),
                    BinOp::Gt => bool_val(a > b),
                    BinOp::Le => bool_val(a <= b),
                    BinOp::Ge => bool_val(a >= b),
                    BinOp::LogOr => bool_val(a != 0 || b != 0),
                    BinOp::LogAnd => bool_val(a != 0 && b != 0),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: fold with an empty symbol table (pure arithmetic).
    fn fold_pure(e: &Expr) -> Fold {
        e.fold(&|_name: &str| None)
    }

    #[test]
    fn int_folds_to_itself() {
        assert_eq!(fold_pure(&Expr::Int(42)), Fold::Value(42));
    }

    #[test]
    fn operators_fold() {
        use BinOp::*;
        let bin = |op, l: i64, r: i64| {
            Expr::Binary { op, lhs: Box::new(Expr::Int(l)), rhs: Box::new(Expr::Int(r)) }
        };
        assert_eq!(fold_pure(&bin(Add, 2, 3)), Fold::Value(5));
        assert_eq!(fold_pure(&bin(Sub, 2, 3)), Fold::Value(-1));
        assert_eq!(fold_pure(&bin(Mul, 4, 5)), Fold::Value(20));
        assert_eq!(fold_pure(&bin(Shr, 0xFF, 8)), Fold::Value(0));
        assert_eq!(fold_pure(&bin(Shl, 1, 4)), Fold::Value(16));
        assert_eq!(fold_pure(&bin(And, 0x6569A, 0x7FFF)), Fold::Value(0x569A));
        assert_eq!(fold_pure(&bin(Or, 0x569A, 0x8000)), Fold::Value(0xD69A));
        // Comparisons fold to 1 / 0 (used only in if-context by the front-end).
        assert_eq!(fold_pure(&bin(Eq, 3, 3)), Fold::Value(1));
        assert_eq!(fold_pure(&bin(Ne, 3, 3)), Fold::Value(0));
    }

    #[test]
    fn logical_or_and_and_fold_to_neutral_0_1() {
        use BinOp::*;
        let bin = |op, l: i64, r: i64| {
            Expr::Binary { op, lhs: Box::new(Expr::Int(l)), rhs: Box::new(Expr::Int(r)) }
        };
        // Matches real asl's truth table (verified against the `asl` binary):
        // both operators yield 1/0, never -1/0.
        assert_eq!(fold_pure(&bin(LogOr, 0, 0)), Fold::Value(0));
        assert_eq!(fold_pure(&bin(LogOr, 0, 1)), Fold::Value(1));
        assert_eq!(fold_pure(&bin(LogOr, 1, 0)), Fold::Value(1));
        assert_eq!(fold_pure(&bin(LogOr, 1, 1)), Fold::Value(1));
        assert_eq!(fold_pure(&bin(LogAnd, 0, 0)), Fold::Value(0));
        assert_eq!(fold_pure(&bin(LogAnd, 0, 1)), Fold::Value(0));
        assert_eq!(fold_pure(&bin(LogAnd, 1, 0)), Fold::Value(0));
        assert_eq!(fold_pure(&bin(LogAnd, 1, 1)), Fold::Value(1));
        // Non-zero operands other than 1 still normalize to the truth value.
        assert_eq!(fold_pure(&bin(LogOr, 5, 0)), Fold::Value(1));
        assert_eq!(fold_pure(&bin(LogAnd, 5, 2)), Fold::Value(1));
    }

    #[test]
    fn xor_folds() {
        use BinOp::*;
        let bin = |op, l: i64, r: i64| {
            Expr::Binary { op, lhs: Box::new(Expr::Int(l)), rhs: Box::new(Expr::Int(r)) }
        };
        // asl-verified infix `!`: 1!1=0, 3!1=2, 5!3=6 (probe 2026-07-04).
        assert_eq!(fold_pure(&bin(Xor, 1, 1)), Fold::Value(0));
        assert_eq!(fold_pure(&bin(Xor, 3, 1)), Fold::Value(2));
        assert_eq!(fold_pure(&bin(Xor, 5, 3)), Fold::Value(6));
    }

    #[test]
    fn division_truncates_toward_zero() {
        use BinOp::*;
        let div = |l, r| Expr::Binary { op: Div, lhs: Box::new(Expr::Int(l)), rhs: Box::new(Expr::Int(r)) };
        assert_eq!(fold_pure(&div(1_000_000_000, 1_107_607)), Fold::Value(902));
    }

    #[test]
    fn modulo_matches_asl_hash_operator() {
        // asl-verified truth table (probed against real `asl` — see BinOp::Mod doc).
        use BinOp::*;
        let m = |l, r| Expr::Binary { op: Mod, lhs: Box::new(Expr::Int(l)), rhs: Box::new(Expr::Int(r)) };
        assert_eq!(fold_pure(&m(256, 64)), Fold::Value(0));
        assert_eq!(fold_pure(&m(100, 7)), Fold::Value(2));
        assert_eq!(fold_pure(&m(255, 256)), Fold::Value(255));
        assert_eq!(fold_pure(&m(-5, 3)), Fold::Value(-2));
        assert_eq!(fold_pure(&m(5, -3)), Fold::Value(2));
        assert_eq!(fold_pure(&m(-5, -3)), Fold::Value(-2));
    }

    #[test]
    fn modulo_by_zero_faults() {
        use BinOp::*;
        let m = Expr::Binary { op: Mod, lhs: Box::new(Expr::Int(5)), rhs: Box::new(Expr::Int(0)) };
        let f = ArithFault { kind: FaultKind::DivideByZero, op: "#", lhs: 5, rhs: 0 };
        assert_eq!(fold_pure(&m), Fold::Fault(f));
        assert_eq!(f.to_string(), "division by zero: 5 # 0");
    }

    /// Every fault kind, with the text a consumer prints, and the edge each
    /// one sits beside: the value one step inside still folds.
    #[test]
    fn overflow_and_shift_faults_name_the_operation() {
        use BinOp::*;
        let bin = |op, l: i64, r: i64| Expr::Binary { op, lhs: Box::new(Expr::Int(l)), rhs: Box::new(Expr::Int(r)) };
        let fault = |e: &Expr| match fold_pure(e) {
            Fold::Fault(f) => f.to_string(),
            other => panic!("{e:?} folded to {other:?}, not a fault"),
        };
        assert_eq!(fault(&bin(Add, i64::MAX, 2)), "arithmetic overflow: 9223372036854775807 + 2 does not fit a 64-bit value");
        assert_eq!(fold_pure(&bin(Add, i64::MAX, 0)), Fold::Value(i64::MAX));
        assert_eq!(fault(&bin(Sub, i64::MIN, 1)), "arithmetic overflow: -9223372036854775808 - 1 does not fit a 64-bit value");
        assert_eq!(fault(&bin(Mul, i64::MAX, 2)), "arithmetic overflow: 9223372036854775807 * 2 does not fit a 64-bit value");
        assert_eq!(fold_pure(&bin(Mul, i64::MAX, 1)), Fold::Value(i64::MAX));
        assert_eq!(fault(&bin(Div, i64::MIN, -1)), "arithmetic overflow: -9223372036854775808 / -1 does not fit a 64-bit value");
        assert_eq!(fault(&bin(Div, 5, 0)), "division by zero: 5 / 0");
        assert_eq!(fault(&bin(Shl, 1, 64)), "shift amount 64 out of range 0..=63 in 1 << 64");
        assert_eq!(fault(&bin(Shr, 1, 64)), "shift amount 64 out of range 0..=63 in 1 >> 64");
        assert_eq!(fault(&bin(Shl, 1, -1)), "shift amount -1 out of range 0..=63 in 1 << -1");
        assert_eq!(fault(&bin(Shl, 1, 63)), "arithmetic overflow: 1 << 63 does not fit a 64-bit value");
        assert_eq!(fault(&bin(Shl, 0x100, 62)), "arithmetic overflow: 256 << 62 does not fit a 64-bit value");
        assert_eq!(fold_pure(&bin(Shl, 1, 62)), Fold::Value(1 << 62));
        assert_eq!(fold_pure(&bin(Shl, -1, 63)), Fold::Value(i64::MIN));
        assert_eq!(fold_pure(&bin(Shr, -1, 63)), Fold::Value(-1));
        assert_eq!(fold_pure(&bin(Shr, i64::MIN, 63)), Fold::Value(-1));
        let neg = Expr::Unary { op: UnOp::Neg, operand: Box::new(Expr::Int(i64::MIN)) };
        assert_eq!(fault(&neg), "arithmetic overflow: -(-9223372036854775808) does not fit a 64-bit value");
        let neg_ok = Expr::Unary { op: UnOp::Neg, operand: Box::new(Expr::Int(i64::MIN + 1)) };
        assert_eq!(fold_pure(&neg_ok), Fold::Value(i64::MAX));
    }

    /// A fault on one side outranks a poison on the other: the fault is
    /// final and is reported where it happened, while the poison may resolve
    /// on a later pass.
    #[test]
    fn fault_outranks_poison_on_either_side() {
        use BinOp::*;
        let dz = Expr::Binary { op: Div, lhs: Box::new(Expr::Int(1)), rhs: Box::new(Expr::Int(0)) };
        let sym = Expr::Sym("Later".to_string());
        let l = Expr::Binary { op: Add, lhs: Box::new(sym.clone()), rhs: Box::new(dz.clone()) };
        let r = Expr::Binary { op: Add, lhs: Box::new(dz), rhs: Box::new(sym) };
        assert!(matches!(fold_pure(&l), Fold::Fault(_)));
        assert!(matches!(fold_pure(&r), Fold::Fault(_)));
    }

    #[test]
    fn timer_a_reload_59_folds_to_122() {
        use BinOp::*;
        // 1024 - (1000000000 / (59 * 18773))
        let hz_times = Expr::Binary {
            op: Mul,
            lhs: Box::new(Expr::Int(59)),
            rhs: Box::new(Expr::Int(18773)),
        };
        let quotient = Expr::Binary {
            op: Div,
            lhs: Box::new(Expr::Int(1_000_000_000)),
            rhs: Box::new(hz_times),
        };
        let expr = Expr::Binary {
            op: Sub,
            lhs: Box::new(Expr::Int(1024)),
            rhs: Box::new(quotient),
        };
        assert_eq!(fold_pure(&expr), Fold::Value(122));
    }

    #[test]
    fn symbol_resolves_via_lookup() {
        let e = Expr::Sym("Sfx_33".to_string());
        let resolved = e.fold(&|name| if name == "Sfx_33" { Some(0x6569A) } else { None });
        assert_eq!(resolved, Fold::Value(0x6569A));
    }

    #[test]
    fn unknown_symbol_poisons() {
        let e = Expr::Sym("Nope".to_string());
        assert_eq!(fold_pure(&e), Fold::Poison);
    }

    #[test]
    fn poison_propagates_and_div_by_zero_faults() {
        use BinOp::*;
        let e = Expr::Binary {
            op: Add,
            lhs: Box::new(Expr::Int(1)),
            rhs: Box::new(Expr::Sym("Nope".to_string())),
        };
        assert_eq!(fold_pure(&e), Fold::Poison);
        let dz = Expr::Binary { op: Div, lhs: Box::new(Expr::Int(1)), rhs: Box::new(Expr::Int(0)) };
        assert_eq!(fold_pure(&dz), Fold::Fault(ArithFault { kind: FaultKind::DivideByZero, op: "/", lhs: 1, rhs: 0 }));
    }
}
