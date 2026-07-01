//! Z80 instruction encoder for the Sigil assembler's supported subset.

/// An 8-bit Z80 register operand.
///
/// The discriminants are the Z80 register codes used within opcode bytes.
/// `Hl` (code 6) denotes the `(HL)` memory operand and is intentionally
/// rejected by [`encode`] for now.
///
/// # Design note — `Hl = 6` and future breaking change
///
/// The `Hl` variant models the Z80 `(HL)` memory-indirect operand, whose
/// register-code is 6.  It is kept inside `Reg8` deliberately so that the
/// Plan 1 encoder and disassembler can use `(reg as u8)` register-code
/// arithmetic uniformly across all eight register slots (0 = B … 7 = A).
/// [`encode`] guards against accidental use of this variant via `reject_hl`.
///
/// A future revision that adds real memory-indirect or IX/IY-prefixed operands
/// **should** split this into a pure-register enum (B/C/D/E/H/L/A) plus a
/// separate operand type that covers `(HL)`, `(IX+d)`, `(IY+d)`, etc.  That
/// split is a **known breaking change** to the (extraction-ready) public API of
/// this crate and must be coordinated with all downstream crates.
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

/// Map a 3-bit Z80 register code (0..=7) back to [`Reg8`].
/// Code 6 represents `(HL)`; callers that reject it should pass the result
/// through [`reject_hl`] before using it.
fn reg_from_code(code: u8) -> Reg8 {
    match code {
        0 => Reg8::B,
        1 => Reg8::C,
        2 => Reg8::D,
        3 => Reg8::E,
        4 => Reg8::H,
        5 => Reg8::L,
        6 => Reg8::Hl,
        _ => Reg8::A,
    }
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

/// Decode the first instruction in `bytes`, returning it and the number of
/// bytes consumed. Exact inverse of [`encode`] over the five supported forms.
///
/// Returns `Err` for any opcode outside the supported subset, for `(HL)`
/// register operands (code 6), or for a truncated multi-byte instruction.
pub fn disassemble(bytes: &[u8]) -> Result<(Instruction, usize), IsaError> {
    let opcode = match bytes.first() {
        Some(&b) => b,
        None => return Err(IsaError::UnsupportedOperand("empty input".into())),
    };
    match opcode {
        // nop
        0x00 => Ok((Instruction::Nop, 1)),
        // ld r, r'  (0x40..=0x7F, excluding HALT 0x76 and (HL) variants)
        0x40..=0x7F => {
            let dst = reg_from_code((opcode >> 3) & 0x07);
            let src = reg_from_code(opcode & 0x07);
            reject_hl(dst)?;
            reject_hl(src)?;
            Ok((Instruction::LdRegReg { dst, src }, 1))
        }
        // ld r, n  (0x06, 0x0E, 0x16, 0x1E, 0x26, 0x2E, 0x36, 0x3E)
        _ if opcode & 0xC7 == 0x06 => {
            let dst = reg_from_code((opcode >> 3) & 0x07);
            reject_hl(dst)?;
            let imm = match bytes.get(1) {
                Some(&b) => b,
                None => return Err(IsaError::UnsupportedOperand("truncated ld r, imm".into())),
            };
            Ok((Instruction::LdRegImm { dst, imm }, 2))
        }
        // add a, r  (0x80..=0x87)
        0x80..=0x87 => {
            let src = reg_from_code(opcode & 0x07);
            reject_hl(src)?;
            Ok((Instruction::AddAReg { src }, 1))
        }
        // jp nn  (0xC3, lo, hi — little-endian)
        0xC3 => {
            let lo = match bytes.get(1) {
                Some(&b) => b as u16,
                None => return Err(IsaError::UnsupportedOperand("truncated jp imm16".into())),
            };
            let hi = match bytes.get(2) {
                Some(&b) => b as u16,
                None => return Err(IsaError::UnsupportedOperand("truncated jp imm16".into())),
            };
            Ok((Instruction::JpImm { addr: lo | (hi << 8) }, 3))
        }
        _ => Err(IsaError::UnsupportedOperand(format!(
            "unknown opcode {opcode:#04X}"
        ))),
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
