//! Comptime integer expressions and the folding pass. Fleshed out in Task 2.

/// A build-time integer expression. (Task 2 adds operators and folding.)
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
    /// A literal integer.
    Int(i64),
}
