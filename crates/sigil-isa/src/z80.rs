// crates/sigil-isa/src/z80.rs

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Instruction {
    Nop,
    LdRegReg { dst: Reg8, src: Reg8 },
    LdRegImm { dst: Reg8, imm: u8 },
    AddAReg { src: Reg8 },
    JpImm { addr: u16 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IsaError {
    UnsupportedOperand(String),
}

pub fn encode(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    match inst {
        Instruction::Nop => Ok(vec![0x00]),
        Instruction::LdRegReg { dst, src } => {
            Ok(vec![0x40 | ((*dst as u8) << 3) | (*src as u8)])
        }
        Instruction::LdRegImm { dst, imm } => {
            Ok(vec![0x06 | ((*dst as u8) << 3), *imm])
        }
        Instruction::AddAReg { src } => Ok(vec![0x80 | (*src as u8)]),
        Instruction::JpImm { addr } => {
            Ok(vec![0xC3, (*addr & 0x00FF) as u8, (*addr >> 8) as u8])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
