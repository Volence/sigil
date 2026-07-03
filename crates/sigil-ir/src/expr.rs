//! Comptime integer expressions and the pure folding pass.
//!
//! Folding is a pure function of the expression tree and a symbol-lookup
//! closure. Any unresolved symbol or arithmetic error (e.g. divide-by-zero)
//! yields [`Fold::Poison`], which propagates through every operator — the
//! front-end turns a poisoned fold into a diagnostic (Plan 4).

/// Binary operators used by the Z80 driver's build-time math (catalog §3.10).
/// Comparisons appear only inside `if` and fold to `1`/`0`.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    /// Truncating integer division (toward zero), matching AS.
    Div,
    Shl,
    Shr,
    And,
    Or,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

/// Unary operators.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum UnOp {
    Neg,
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
    /// The expression could not be resolved (unknown symbol, arithmetic error).
    Poison,
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
                    Fold::Poison => return Fold::Poison,
                };
                match op {
                    UnOp::Neg => Fold::Value(v.wrapping_neg()),
                }
            }
            Expr::Binary { op, lhs, rhs } => {
                let a = match lhs.fold(lookup) {
                    Fold::Value(v) => v,
                    Fold::Poison => return Fold::Poison,
                };
                let b = match rhs.fold(lookup) {
                    Fold::Value(v) => v,
                    Fold::Poison => return Fold::Poison,
                };
                let bool_val = |t: bool| Fold::Value(if t { 1 } else { 0 });
                match op {
                    BinOp::Add => Fold::Value(a.wrapping_add(b)),
                    BinOp::Sub => Fold::Value(a.wrapping_sub(b)),
                    BinOp::Mul => Fold::Value(a.wrapping_mul(b)),
                    BinOp::Div => {
                        if b == 0 {
                            Fold::Poison
                        } else {
                            // i64 `/` truncates toward zero (matches AS).
                            Fold::Value(a.wrapping_div(b))
                        }
                    }
                    BinOp::Shl => Fold::Value(a.wrapping_shl(b as u32)),
                    BinOp::Shr => Fold::Value(a.wrapping_shr(b as u32)),
                    BinOp::And => Fold::Value(a & b),
                    BinOp::Or => Fold::Value(a | b),
                    BinOp::Eq => bool_val(a == b),
                    BinOp::Ne => bool_val(a != b),
                    BinOp::Lt => bool_val(a < b),
                    BinOp::Gt => bool_val(a > b),
                    BinOp::Le => bool_val(a <= b),
                    BinOp::Ge => bool_val(a >= b),
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
    fn division_truncates_toward_zero() {
        use BinOp::*;
        let div = |l, r| Expr::Binary { op: Div, lhs: Box::new(Expr::Int(l)), rhs: Box::new(Expr::Int(r)) };
        assert_eq!(fold_pure(&div(1_000_000_000, 1_107_607)), Fold::Value(902));
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
    fn poison_propagates_and_div_by_zero_poisons() {
        use BinOp::*;
        let e = Expr::Binary {
            op: Add,
            lhs: Box::new(Expr::Int(1)),
            rhs: Box::new(Expr::Sym("Nope".to_string())),
        };
        assert_eq!(fold_pure(&e), Fold::Poison);
        let dz = Expr::Binary { op: Div, lhs: Box::new(Expr::Int(1)), rhs: Box::new(Expr::Int(0)) };
        assert_eq!(fold_pure(&dz), Fold::Poison);
    }
}
