//! Width selection + (next task) the bounded layout fixpoint (spec §5.4/§5.6).
//! The only length-variable fragment in Aeon is bare-symbol `jmp`/`jsr`.

/// The chosen absolute-addressing width for a `jmp`/`jsr`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbsWidth {
    /// `abs.w`: opcode word + 2-byte operand (4 bytes total).
    W,
    /// `abs.l`: opcode word + 4-byte operand (6 bytes total).
    L,
}

impl AbsWidth {
    /// Total instruction length in bytes for this width.
    pub fn inst_len(self) -> u32 {
        match self {
            AbsWidth::W => 4,
            AbsWidth::L => 6,
        }
    }
}

/// asl's `abs.w` vs `abs.l` selection for a `jmp`/`jsr` target address. Confirmed
/// byte-for-byte against asl 1.42 by a boundary sweep of `jmp $ADDR` (with AND
/// without `-A` — identical results, so `-A` is irrelevant to jmp/jsr width).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_rule_matches_asl_boundary_sweep() {
        assert_eq!(asl_width_rule(0x0000, true), AbsWidth::W);
        assert_eq!(asl_width_rule(0x7FFF, true), AbsWidth::W);
        assert_eq!(asl_width_rule(0x8000, true), AbsWidth::L);
        assert_eq!(asl_width_rule(0xFFFF, true), AbsWidth::L);
        assert_eq!(asl_width_rule(0x1_0000, true), AbsWidth::L);
        assert_eq!(asl_width_rule(0xFF_8000, true), AbsWidth::W);
        assert_eq!(asl_width_rule(0xFF_FFFF, true), AbsWidth::W);
    }

    #[test]
    fn dash_a_does_not_change_width() {
        // -A is irrelevant to jmp/jsr width (confirmed by the asl sweep).
        for addr in [0x0000i64, 0x7FFF, 0x8000, 0xFF_8000, 0xFF_FFFF] {
            assert_eq!(asl_width_rule(addr, true), asl_width_rule(addr, false));
        }
    }

    #[test]
    fn inst_len_is_4_for_w_and_6_for_l() {
        assert_eq!(AbsWidth::W.inst_len(), 4);
        assert_eq!(AbsWidth::L.inst_len(), 6);
    }
}
