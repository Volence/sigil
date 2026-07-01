//! Z80 instruction encoder for the Sigil assembler's supported subset.

/// An 8-bit Z80 register operand.
///
/// The discriminants are the Z80 register codes used within opcode bytes.
/// `Hl` (code 6) denotes the `(HL)` memory operand and is intentionally
/// rejected by [`encode`] for now.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Reg8 {
    B = 0,
    C = 1,
    D = 2,
    E = 3,
    H = 4,
    L = 5,
    Hl = 6,
    A = 7,
}

/// A single Z80 instruction in the supported subset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Instruction {
    /// `nop`
    Nop,
    /// `ld dst, src` (register-to-register)
    LdRegReg { dst: Reg8, src: Reg8 },
    /// `ld dst, imm` (immediate load)
    LdRegImm { dst: Reg8, imm: u8 },
    /// `add a, src`
    AddAReg { src: Reg8 },
    /// `jp addr` (absolute jump)
    JpImm { addr: u16 },
}

/// An error produced while encoding an [`Instruction`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IsaError {
    /// An operand is not supported by the current encoder.
    UnsupportedOperand(String),
}

impl std::fmt::Display for IsaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IsaError::UnsupportedOperand(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for IsaError {}

/// Reject the `(HL)` memory operand, which is not yet supported.
fn reject_hl(reg: Reg8) -> Result<(), IsaError> {
    if reg == Reg8::Hl {
        return Err(IsaError::UnsupportedOperand(
            "(HL) memory operand is not supported".into(),
        ));
    }
    Ok(())
}

/// Encode a single [`Instruction`] into its Z80 machine-code bytes.
pub fn encode(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    match inst {
        Instruction::Nop => Ok(vec![0x00]),
        Instruction::LdRegReg { dst, src } => {
            reject_hl(*dst)?;
            reject_hl(*src)?;
            Ok(vec![0x40 | ((*dst as u8) << 3) | (*src as u8)])
        }
        Instruction::LdRegImm { dst, imm } => {
            reject_hl(*dst)?;
            Ok(vec![0x06 | ((*dst as u8) << 3), *imm])
        }
        Instruction::AddAReg { src } => {
            reject_hl(*src)?;
            Ok(vec![0x80 | (*src as u8)])
        }
        Instruction::JpImm { addr } => {
            let [lo, hi] = addr.to_le_bytes();
            Ok(vec![0xC3, lo, hi])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_hl_operand() {
        assert!(matches!(
            encode(&Instruction::LdRegReg { dst: Reg8::Hl, src: Reg8::B }),
            Err(IsaError::UnsupportedOperand(_))
        ));
        assert!(matches!(
            encode(&Instruction::LdRegReg { dst: Reg8::B, src: Reg8::Hl }),
            Err(IsaError::UnsupportedOperand(_))
        ));
        assert!(matches!(
            encode(&Instruction::LdRegImm { dst: Reg8::Hl, imm: 0 }),
            Err(IsaError::UnsupportedOperand(_))
        ));
        assert!(matches!(
            encode(&Instruction::AddAReg { src: Reg8::Hl }),
            Err(IsaError::UnsupportedOperand(_))
        ));
    }

    #[test]
    fn encodes_supported_forms() {
        assert_eq!(encode(&Instruction::Nop).unwrap(), vec![0x00]);
        assert_eq!(
            encode(&Instruction::LdRegReg { dst: Reg8::B, src: Reg8::C }).unwrap(),
            vec![0x41]
        );
        assert_eq!(
            encode(&Instruction::LdRegReg { dst: Reg8::A, src: Reg8::A }).unwrap(),
            vec![0x7F]
        );
        assert_eq!(
            encode(&Instruction::LdRegImm { dst: Reg8::A, imm: 5 }).unwrap(),
            vec![0x3E, 0x05]
        );
        assert_eq!(
            encode(&Instruction::LdRegImm { dst: Reg8::B, imm: 10 }).unwrap(),
            vec![0x06, 0x0A]
        );
        assert_eq!(
            encode(&Instruction::AddAReg { src: Reg8::B }).unwrap(),
            vec![0x80]
        );
        assert_eq!(
            encode(&Instruction::AddAReg { src: Reg8::A }).unwrap(),
            vec![0x87]
        );
        assert_eq!(
            encode(&Instruction::JpImm { addr: 0x1234 }).unwrap(),
            vec![0xC3, 0x34, 0x12]
        );
    }
}
