//! asl's `abs.w` vs `abs.l` selection for a 68000 absolute address. Shared by
//! the front-end (bare-symbol absolute EA + jmp/jsr width, M1.D T2/T3) and the
//! linker's `resolve_layout` (M1.B). Single source of truth — the front-end
//! cannot depend on `sigil-link`, and a second copy would be drift-prone.

/// The chosen absolute-addressing width for a width-variable 68000 form
/// (`jmp`/`jsr` target, or a bare-symbol absolute EA).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbsWidth {
    /// `abs.w`: opcode word + 2-byte operand.
    W,
    /// `abs.l`: opcode word + 4-byte operand.
    L,
}

impl AbsWidth {
    /// Total length in bytes of a 2-byte-opcode `jmp`/`jsr` at this width
    /// (4 for `.w`, 6 for `.l`). Used by the linker's fragment-length math.
    pub fn inst_len(self) -> u32 {
        match self {
            AbsWidth::W => 4,
            AbsWidth::L => 6,
        }
    }
}

/// asl's `abs.w` vs `abs.l` selection for a 68000 absolute address. Confirmed
/// byte-for-byte against asl 1.42 by a boundary sweep of `jmp $ADDR` (with AND
/// without `-A` — identical results, so `-A` is irrelevant to width) and
/// re-confirmed for the general absolute EA in M1.D T2 (`lea`/`move` probes).
/// `abs.w` iff the 24-bit address sign-extends losslessly from 16 bits:
/// `[0, 0x7FFF] ∪ [0xFF_8000, 0xFF_FFFF]`. Examples: $7FFF→.w, $8000→.l,
/// $FF8000→.w (= -$8000 sign-extended), $FFFFFE→.w.
pub fn asl_width_rule(target: i64, _dash_a: bool) -> AbsWidth {
    let a = (target & 0xFF_FFFF) as u32;
    if a <= 0x7FFF || a >= 0xFF_8000 {
        AbsWidth::W
    } else {
        AbsWidth::L
    }
}
