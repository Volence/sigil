//! Relocation records patched by the linker. Fully fleshed out in Task 4.

use crate::expr::Expr;

/// How the linker turns a resolved target value into patched bytes.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FixupKind {
    /// Resolve target to a VMA; write it as a little-endian `u16`.
    BankPtr16Le,
    /// Resolve target to a VMA; write it as a big-endian `u16`. The 68k-section
    /// counterpart to [`BankPtr16Le`](Self::BankPtr16Le): a 68000 reference to a
    /// Z80 bank pointer (§7.2 / D-P4.7).
    BankPtr16Be,
    /// Z80 `jr`/`djnz`: write `(target_vma - (site_vma + 2))` as a range-checked `i8`.
    Z80JrRel8,
    /// Scaffolding for the 68000 backend (M1); unused in M0.
    Abs16Be,
    /// Scaffolding for the 68000 backend (M1); unused in M0.
    Abs32Be,
    /// A self-relative signed **word** offset (`dc.w Target-Base`): the offset
    /// table idiom. Unlike [`Abs16Be`](Self::Abs16Be) (an absolute address
    /// truncated to 16 bits), this writes a *signed relative displacement* — the
    /// [`Fixup`]'s `target` is a symbol **difference** (`Sub(Sym(t), Sym(base))`),
    /// so the folded value IS the offset. Range i16 (`[-0x8000, 0x7FFF]`),
    /// big-endian; overflow is an error (totality). Fixed width — no relaxation.
    RelWord16Be,
    /// 8-bit branch displacement in the opcode low byte (`bra.s`/`bsr.s`/`Bcc.s`).
    /// `disp = target - (site_vma + 1)` (PC ref = op+2, byte at op+1); range i8.
    PcRel8,
    /// 16-bit displacement in an extension word (`bra.w`/`bsr.w`/`Bcc.w`, `(d16,PC)`).
    /// `disp = target - site_vma` (the disp word's own VMA); range i16, big-endian.
    PcRelDisp16,
    /// 8-bit displacement in a brief extension word (`(d8,PC,Xn)`). The disp is
    /// the LOW byte of the ext word, but the 68k PC reference is the ext word's
    /// own VMA (one byte before): `disp = target - (site_vma - 1)`; range i8.
    /// The fixup offset points at the disp (low) byte.
    PcRelDisp8,
    /// Synthetic Sega header checksum. Applied as a final post-image pass over
    /// the whole file, NOT through `apply_fixup`; present here for `byte_width`
    /// bookkeeping and future in-fragment modelling.
    HeaderChecksum,
    /// A general link-expr data VALUE, width 1 (Spec 2, S2-D13f / R7m.4). Writes
    /// the folded target integer VERBATIM after an UNSIGNED-window range check
    /// (`0 ≤ v < 2^8`). Deliberately DISTINCT from the address kinds
    /// ([`Abs16Be`](Self::Abs16Be) range-checks as a *signed* address;
    /// [`BankPtr16Le`](Self::BankPtr16Le) *masks*) — a value cell inherits neither
    /// semantics. Byte order is irrelevant at width 1. Any CPU.
    Value8,
    /// A general link-expr data VALUE, width 2, big-endian (68k sections).
    /// Unsigned-window range check (`0 ≤ v < 2^16`), then the folded integer is
    /// written verbatim big-endian.
    Value16Be,
    /// A general link-expr data VALUE, width 2, little-endian (Z80 sections).
    /// Unsigned-window range check (`0 ≤ v < 2^16`), then written verbatim
    /// little-endian.
    Value16Le,
    /// A general link-expr data VALUE, width 4, big-endian (68k sections).
    /// Unsigned-window range check (`0 ≤ v < 2^32`), then written verbatim
    /// big-endian.
    Value32Be,
    /// A general link-expr data VALUE, width 4, little-endian (Z80 sections).
    /// Unsigned-window range check (`0 ≤ v < 2^32`), then written verbatim
    /// little-endian.
    Value32Le,
}

impl FixupKind {
    /// Number of image bytes this fixup writes, starting at its offset.
    /// Used by the linker to verify a fixup fits entirely within its fragment.
    pub fn byte_width(&self) -> u32 {
        match self {
            FixupKind::BankPtr16Le
            | FixupKind::BankPtr16Be
            | FixupKind::Abs16Be
            | FixupKind::RelWord16Be => 2,
            FixupKind::PcRelDisp16 | FixupKind::HeaderChecksum => 2,
            FixupKind::Z80JrRel8 | FixupKind::PcRel8 | FixupKind::PcRelDisp8 => 1,
            FixupKind::Abs32Be => 4,
            FixupKind::Value8 => 1,
            FixupKind::Value16Be | FixupKind::Value16Le => 2,
            FixupKind::Value32Be | FixupKind::Value32Le => 4,
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
        assert_eq!(FixupKind::BankPtr16Be.byte_width(), 2);
        assert_eq!(FixupKind::Abs16Be.byte_width(), 2);
        assert_eq!(FixupKind::Z80JrRel8.byte_width(), 1);
        assert_eq!(FixupKind::Abs32Be.byte_width(), 4);
    }

    #[test]
    fn byte_width_of_new_68k_kinds() {
        assert_eq!(FixupKind::PcRel8.byte_width(), 1);
        assert_eq!(FixupKind::PcRelDisp16.byte_width(), 2);
        assert_eq!(FixupKind::PcRelDisp8.byte_width(), 1);
        assert_eq!(FixupKind::HeaderChecksum.byte_width(), 2);
    }

    #[test]
    fn rel_word_16_be_is_two_bytes() {
        assert_eq!(FixupKind::RelWord16Be.byte_width(), 2);
    }
}
