//! Z80 `Backend` implementation: binds the CPU-agnostic `sigil_ir::Backend`
//! trait to `sigil_isa::z80` and turns fully-resolved instructions into
//! `DataFragment`s. Fixed-length forms go straight through `z80::encode`;
//! `jr`/`djnz` emit `[opcode, 0x00]` + a `Z80JrRel8` fixup for the linker.

use sigil_ir::backend::{Backend, Cpu, LowerError};
use sigil_ir::{DataFragment, Expr, Fixup, FixupKind};
use sigil_isa::z80::{Cond, Instruction, Mnemonic, Operand};
use sigil_span::Span;

/// Re-export the Z80 operand/mnemonic vocabulary so downstream crates (the AS
/// front-end) can construct instructions without a *direct* `sigil-isa`
/// dependency — keeping `sigil-isa` a transitive-only edge (crate-graph guard).
pub use sigil_isa::z80;

/// The Z80 backend. Stateless.
pub struct Z80Backend;

impl Backend for Z80Backend {
    type Mnemonic = Mnemonic;
    type Operand = Operand;

    fn cpu(&self) -> Cpu {
        Cpu::Z80
    }

    fn lower(&self, mnemonic: Mnemonic, operands: &[Operand], span: Span) -> Result<DataFragment, LowerError> {
        let inst = Instruction { mnemonic, ops: operands.to_vec() };
        let bytes = z80::encode(&inst).map_err(|e| LowerError { message: e.to_string() })?;
        Ok(DataFragment { bytes, fixups: vec![], span })
    }
}

impl Z80Backend {
    /// Lower a `jr`/`djnz`/`jr cc` whose target is not yet resolved: emit the
    /// opcode byte + a placeholder displacement + a `Z80JrRel8` fixup carrying
    /// `target`. The linker computes and range-checks the displacement.
    ///
    /// `cond` is `Some(cc)` for `jr cc,e`; `None` for `jr e` / `djnz e`.
    pub fn lower_rel(
        &self,
        mnemonic: Mnemonic,
        cond: Option<Cond>,
        target: Expr,
        span: Span,
    ) -> Result<DataFragment, LowerError> {
        // Reuse the encoder for the opcode byte by encoding with a zero
        // displacement, then keep byte[0] and attach the fixup at offset 1.
        let ops: Vec<Operand> = match cond {
            Some(cc) => vec![Operand::Cc(cc), Operand::Rel(0)],
            None => vec![Operand::Rel(0)],
        };
        let inst = Instruction { mnemonic, ops };
        let encoded = z80::encode(&inst).map_err(|e| LowerError { message: e.to_string() })?;
        // jr/djnz forms are exactly 2 bytes: [opcode, disp].
        if encoded.len() != 2 {
            return Err(LowerError {
                message: format!("expected 2-byte relative form, got {} bytes", encoded.len()),
            });
        }
        // byte[1] is a placeholder the linker overwrites when it resolves the Z80JrRel8 fixup; hardcode 0x00 rather than depend on the encoder's disp byte.
        Ok(DataFragment {
            bytes: vec![encoded[0], 0x00],
            fixups: vec![Fixup { kind: FixupKind::Z80JrRel8, offset: 1, target }],
            span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_isa::z80::Reg8;
    use sigil_span::SourceId;

    fn span() -> Span {
        Span { source: SourceId(0), start: 0, end: 0 }
    }

    #[test]
    fn lowers_fixed_instruction_via_encode() {
        let b = Z80Backend;
        // ld a,0Ch  →  3E 0C
        let frag = b.lower(Mnemonic::Ld, &[Operand::Reg(Reg8::A), Operand::Imm8(0x0C)], span()).unwrap();
        assert_eq!(frag.bytes, vec![0x3E, 0x0C]);
        assert!(frag.fixups.is_empty());
    }

    #[test]
    fn unsupported_form_becomes_lower_error() {
        let b = Z80Backend;
        // A deliberately malformed operand list for `ex` (0 operands).
        let err = b.lower(Mnemonic::Ex, &[], span()).unwrap_err();
        assert!(err.message.contains("unsupported form"));
    }

    #[test]
    fn jr_emits_opcode_plus_z80jrrel8_fixup() {
        let b = Z80Backend;
        // jr .target  →  opcode 18, placeholder 00, one Z80JrRel8 fixup at offset 1.
        let frag = b.lower_rel(Mnemonic::Jr, None, Expr::Sym(".target".to_string()), span()).unwrap();
        assert_eq!(frag.bytes, vec![0x18, 0x00]);
        assert_eq!(frag.fixups.len(), 1);
        assert_eq!(frag.fixups[0].kind, FixupKind::Z80JrRel8);
        assert_eq!(frag.fixups[0].offset, 1);
    }

    #[test]
    fn jr_cc_uses_conditional_opcode() {
        let b = Z80Backend;
        // jr z,.target  →  opcode 28.
        let frag = b.lower_rel(Mnemonic::Jr, Some(Cond::Z), Expr::Sym(".target".to_string()), span()).unwrap();
        assert_eq!(frag.bytes[0], 0x28);
        assert_eq!(frag.fixups[0].kind, FixupKind::Z80JrRel8);
    }

    #[test]
    fn reexports_z80_vocabulary() {
        // The front-end constructs instructions via this re-export, not a direct
        // sigil-isa dep. Naming these types through `crate::z80` must compile.
        use crate::z80::{Cond, IndexReg, Mnemonic, Operand, Reg16, Reg8};
        let _ = (Mnemonic::Ld, Operand::Reg(Reg8::A), Reg16::Hl, Cond::Z, IndexReg::Ix);
    }
}
