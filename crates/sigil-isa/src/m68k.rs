//! `sigil-isa` 68000 encoder (procedural EA / extension-word machinery).
//!
//! M0.5 spike scope: `MOVE` across the effective-address matrix, proving the
//! procedural EA encoder and the §5.5 `MOVE` dest-EA mode/register field swap
//! byte-for-byte against `asl`. Operands carry **already-resolved** integers and
//! their **explicit** EA form (`AbsW` vs `AbsL` vs `Pcd16`) — width *selection*
//! (§5.6) is deliberately out of scope. Emits big-endian bytes.
//!
//! Decode/disassembly (the ISA-sharing dual-facet) is deferred, as full Z80
//! disassembly was in M0.

/// Instruction mnemonics. Grown in M1; the spike covers `Move` only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mnemonic {
    Move,
}

/// Operation size. `B` is included for completeness; the MOVE slice uses `W`/`L`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    B,
    W,
    L,
}

/// Index register for the `(d8,An,Xn)` brief extension word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Xn {
    D(u8),
    A(u8),
}

/// A fully-resolved effective address. Each variant is one explicit EA form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    Dn(u8),
    An(u8),
    Ind(u8),
    PostInc(u8),
    PreDec(u8),
    Disp16An(i16, u8),
    Disp8AnXn { d: i8, an: u8, xn: Xn, long: bool },
    AbsW(i16),
    AbsL(i32),
    Pcd16(i16),
    Imm(i32),
}

/// A decoded instruction: mnemonic + size + operands (source, dest order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub mnemonic: Mnemonic,
    pub size: Size,
    pub ops: Vec<Operand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IsaError {
    /// The `(mnemonic, operand-shape)` pair is not (yet) a supported form.
    UnsupportedForm(String),
    /// An EA form is illegal in the destination position (e.g. `#imm`, `(d16,PC)`).
    IllegalDest(String),
    /// Wrong operand count for the mnemonic.
    OperandCount(String),
}

impl std::fmt::Display for IsaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IsaError::UnsupportedForm(m) => write!(f, "unsupported form: {m}"),
            IsaError::IllegalDest(m) => write!(f, "illegal destination EA: {m}"),
            IsaError::OperandCount(m) => write!(f, "operand count: {m}"),
        }
    }
}

impl std::error::Error for IsaError {}

/// Encode one instruction to big-endian machine-code bytes.
pub fn encode(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    match inst.mnemonic {
        Mnemonic::Move => Err(IsaError::UnsupportedForm("move (not yet implemented)".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_encode_move_is_unsupported_for_now() {
        let inst = Instruction {
            mnemonic: Mnemonic::Move,
            size: Size::W,
            ops: vec![Operand::Dn(1), Operand::Dn(0)],
        };
        assert!(matches!(encode(&inst), Err(IsaError::UnsupportedForm(_))));
    }
}
