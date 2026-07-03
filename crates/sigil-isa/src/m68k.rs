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
        Mnemonic::Move => encode_move(inst),
    }
}

/// Which MOVE field an EA occupies. Only affects word-bit placement + legal-dest checks.
#[derive(Clone, Copy)]
enum Field {
    Source,
    Dest,
}

fn encode_move(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    let (src, dst) = match inst.ops.as_slice() {
        [s, d] => (s, d),
        _ => return Err(IsaError::OperandCount(format!("move expects 2 operands, got {}", inst.ops.len()))),
    };
    let size_bits: u16 = match inst.size {
        Size::B => 0b01,
        Size::W => 0b11,
        Size::L => 0b10,
    };
    let (src_mode, src_reg, src_ext) = encode_ea(src, Field::Source, inst.size)?;
    let (dst_mode, dst_reg, dst_ext) = encode_ea(dst, Field::Dest, inst.size)?;
    let word: u16 = (size_bits << 12)
        | ((dst_reg as u16) << 9)
        | ((dst_mode as u16) << 6)
        | ((src_mode as u16) << 3)
        | (src_reg as u16);
    let mut out = Vec::with_capacity(2 + 2 * (src_ext.len() + dst_ext.len()));
    out.extend_from_slice(&word.to_be_bytes());
    for w in src_ext {
        out.extend_from_slice(&w.to_be_bytes());
    }
    for w in dst_ext {
        out.extend_from_slice(&w.to_be_bytes());
    }
    Ok(out)
}

/// Resolve one EA to `(mode3, reg3, extension_words)`. Used for both source and dest —
/// the field-order swap lives in `encode_move`, not here.
fn encode_ea(op: &Operand, _field: Field, _size: Size) -> Result<(u8, u8, Vec<u16>), IsaError> {
    let r = |n: u8| n & 0b111;
    Ok(match *op {
        Operand::Dn(n) => (0b000, r(n), vec![]),
        Operand::An(n) => (0b001, r(n), vec![]),
        Operand::Ind(n) => (0b010, r(n), vec![]),
        Operand::PostInc(n) => (0b011, r(n), vec![]),
        Operand::PreDec(n) => (0b100, r(n), vec![]),
        Operand::Disp16An(d, n) => (0b101, r(n), vec![d as u16]),
        Operand::Disp8AnXn { d, an, xn, long } => (0b110, r(an), vec![brief_ext(d, xn, long)]),
        other => return Err(IsaError::UnsupportedForm(format!("{other:?}"))),
    })
}

/// Build a 68000 brief-format extension word for `(d8,An,Xn)`.
/// bit15 index type (0=Dn,1=An); bits14–12 index reg; bit11 index size (0=`.w`,1=`.l`);
/// bits10–9 scale (`00`, 68020+); bit8 `0` (brief); bits7–0 signed displacement.
fn brief_ext(d: i8, xn: Xn, long: bool) -> u16 {
    let (ty, num) = match xn {
        Xn::D(n) => (0u16, (n & 0b111) as u16),
        Xn::A(n) => (1u16, (n & 0b111) as u16),
    };
    (ty << 15) | (num << 12) | ((long as u16) << 11) | ((d as u8) as u16)
}
