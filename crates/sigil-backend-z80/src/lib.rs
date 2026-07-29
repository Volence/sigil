//! Z80 `Backend` implementation: binds the CPU-agnostic `sigil_ir::Backend`
//! trait to `sigil_isa::z80` and turns fully-resolved instructions into
//! `DataFragment`s. Fixed-length forms go straight through `z80::encode`;
//! `jr`/`djnz` emit `[opcode, 0x00]` + a `Z80JrRel8` fixup for the linker.

use sigil_ir::backend::{Backend, Cpu, LowerError};
use sigil_ir::{DataFragment, Expr, Fixup, FixupKind, RelaxCandidate};
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

    /// Lower a fixed-opcode instruction whose 16-bit immediate is an unresolved
    /// symbolic *absolute* address (`jp`/`call`/`ld rr,nn`/`ld ix,nn` with a
    /// label target). The immediate is the last 2 bytes for every such form;
    /// emit the opcode with a `00 00` placeholder + a `BankPtr16Le` fixup there.
    /// `operands` must carry `Imm16(0)` in the immediate slot (the placeholder).
    pub fn lower_abs16(
        &self,
        mnemonic: Mnemonic,
        operands: &[Operand],
        target: Expr,
        span: Span,
    ) -> Result<DataFragment, LowerError> {
        let inst = Instruction { mnemonic, ops: operands.to_vec() };
        let mut bytes = z80::encode(&inst).map_err(|e| LowerError { message: e.to_string() })?;
        if bytes.len() < 2 {
            return Err(LowerError { message: "abs16 form shorter than 2 bytes".into() });
        }
        let off = bytes.len() as u32 - 2;
        let n = bytes.len();
        bytes[n - 2] = 0x00;
        bytes[n - 1] = 0x00;
        Ok(DataFragment {
            bytes,
            fixups: vec![Fixup { kind: FixupKind::BankPtr16Le, offset: off, target }],
            span,
        })
    }

    /// The two rungs of the `jr → jp` relaxation ladder (rung-2 §5) for an
    /// UNCONDITIONAL symbolic `jr Label`, smallest → largest:
    ///
    /// - rung 0 (2 B): `jr e` — `[0x18, 0x00]`, `Z80JrRel8` disp at offset 1;
    /// - rung 1 (3 B): `jp nn` — `[0xC3, 0x00, 0x00]`, `Value16Le` LE absolute at
    ///   offset 1 (byte-identical to an explicit `jp Label` via
    ///   [`lower_z80_abs16_sym`](../../sigil_frontend_emp/index.html)).
    ///
    /// The linker's `resolve_layout` selects the smallest reaching rung, so an
    /// in-reach target stays `jr` (2 B, asl's choice) and only a genuinely
    /// out-of-reach target grows to `jp`. `djnz` (short-only, no long form) and
    /// `call` (long-only, no short form) are NEVER laddered — see the design §5.
    pub fn lower_jr_jp_candidates(&self, target: Expr, _span: Span) -> Vec<RelaxCandidate> {
        vec![
            // rung 0: jr e (2 bytes) — the opcode 0x18 is `jr` (unconditional).
            RelaxCandidate {
                bytes: vec![0x18, 0x00],
                fixup: Fixup { kind: FixupKind::Z80JrRel8, offset: 1, target: target.clone() },
            },
            // rung 1: jp nn (3 bytes) — opcode 0xC3, little-endian absolute.
            RelaxCandidate {
                bytes: vec![0xC3, 0x00, 0x00],
                fixup: Fixup { kind: FixupKind::Value16Le, offset: 1, target },
            },
        ]
    }

    /// The two rungs of the CONDITIONAL `jr cc → jp cc` ladder (rung-2 §5 / §13.3
    /// item 4) for a symbolic `jr cc, Label`, smallest → largest:
    ///
    /// - rung 0 (2 B): `jr cc, e` — `Z80JrRel8` disp at offset 1;
    /// - rung 1 (3 B): `jp cc, nn` — `Value16Le` LE absolute at offset 1.
    ///
    /// The cc-adjusted opcodes are drawn from the encoder (a zero-operand encode
    /// for the opcode byte, exactly as [`Self::lower_rel`] does) rather than
    /// hardcoded — so the `jr cc` cond-field (`nz`/`z`/`nc`/`c` only) and the
    /// `jp cc` cond-field stay the ISA's single source of truth. A cc the short
    /// `jr` cannot encode (`po`/`pe`/`p`/`m`) surfaces the encoder's own
    /// `jr condition must be nz, z, nc, or c` as a `LowerError` — never a panic,
    /// never a silent narrowing. The linker selects the smallest reaching rung
    /// (an in-reach `jr cc` stays 2 B — asl's choice; every psg/fm `jr cc`
    /// reaches), so the ladder is byte-neutral latent capacity.
    pub fn lower_jr_jp_cc_candidates(
        &self,
        cc: Cond,
        target: Expr,
        _span: Span,
    ) -> Result<Vec<RelaxCandidate>, LowerError> {
        let jr = z80::encode(&Instruction {
            mnemonic: Mnemonic::Jr,
            ops: vec![Operand::Cc(cc), Operand::Rel(0)],
        })
        .map_err(|e| LowerError { message: e.to_string() })?;
        let jp = z80::encode(&Instruction {
            mnemonic: Mnemonic::Jp,
            ops: vec![Operand::Cc(cc), Operand::Imm16(0)],
        })
        .map_err(|e| LowerError { message: e.to_string() })?;
        Ok(vec![
            // rung 0: jr cc, e (2 bytes) — opcode from the encoder, disp placeholder.
            RelaxCandidate {
                bytes: vec![jr[0], 0x00],
                fixup: Fixup { kind: FixupKind::Z80JrRel8, offset: 1, target: target.clone() },
            },
            // rung 1: jp cc, nn (3 bytes) — opcode from the encoder, LE absolute.
            RelaxCandidate {
                bytes: vec![jp[0], 0x00, 0x00],
                fixup: Fixup { kind: FixupKind::Value16Le, offset: 1, target },
            },
        ])
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
    fn abs16_emits_bankptr_fixup_at_last_two_bytes() {
        use sigil_isa::z80::Reg16;
        let b = Z80Backend;
        let f = b.lower_abs16(Mnemonic::Jp, &[Operand::Imm16(0)], Expr::Sym("Label".into()), span()).unwrap();
        assert_eq!(f.bytes, vec![0xC3, 0x00, 0x00]);
        assert_eq!(f.fixups.len(), 1);
        assert_eq!(f.fixups[0].kind, FixupKind::BankPtr16Le);
        assert_eq!(f.fixups[0].offset, 1);
        let g = b.lower_abs16(Mnemonic::Ld, &[Operand::Pair(Reg16::Ix), Operand::Imm16(0)], Expr::Sym("L".into()), span()).unwrap();
        assert_eq!(g.bytes, vec![0xDD, 0x21, 0x00, 0x00]);
        assert_eq!(g.fixups[0].offset, 2);
    }

    #[test]
    fn reexports_z80_vocabulary() {
        // The front-end constructs instructions via this re-export, not a direct
        // sigil-isa dep. Naming these types through `crate::z80` must compile.
        use crate::z80::{Cond, IndexReg, Mnemonic, Operand, Reg16, Reg8};
        let _ = (Mnemonic::Ld, Operand::Reg(Reg8::A), Reg16::Hl, Cond::Z, IndexReg::Ix);
    }
}
