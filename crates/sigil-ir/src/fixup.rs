//! Relocation records patched by the linker. Fully fleshed out in Task 4.

use crate::expr::Expr;

/// How the linker turns a resolved target value into patched bytes.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FixupKind {
    /// Resolve target to a VMA; write it as a little-endian `u16`.
    BankPtr16Le,
    /// Z80 `jr`/`djnz`: write `(target_vma - (site_vma + 2))` as a range-checked `i8`.
    Z80JrRel8,
    /// Scaffolding for the 68000 backend (M1); unused in M0.
    Abs16Be,
    /// Scaffolding for the 68000 backend (M1); unused in M0.
    Abs32Be,
}

impl FixupKind {
    /// Number of image bytes this fixup writes, starting at its offset.
    /// Used by the linker to verify a fixup fits entirely within its fragment.
    pub fn byte_width(&self) -> u32 {
        match self {
            FixupKind::BankPtr16Le | FixupKind::Abs16Be => 2,
            FixupKind::Z80JrRel8 => 1,
            FixupKind::Abs32Be => 4,
        }
    }
}

/// A patch to apply after layout: `kind` determines the byte format, `offset` is
/// the byte position **within the owning `DataFragment`**, `target` is the
/// (possibly symbolic) expression to resolve.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fixup {
    pub kind: FixupKind,
    pub offset: u32,
    pub target: Expr,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_width_matches_kind() {
        assert_eq!(FixupKind::BankPtr16Le.byte_width(), 2);
        assert_eq!(FixupKind::Abs16Be.byte_width(), 2);
        assert_eq!(FixupKind::Z80JrRel8.byte_width(), 1);
        assert_eq!(FixupKind::Abs32Be.byte_width(), 4);
    }
}
