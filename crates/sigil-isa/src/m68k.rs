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

/// Instruction mnemonics. The full 68000 set Aeon uses; only `Move` is encoded
/// today — every other variant currently dispatches to `UnsupportedForm`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mnemonic {
    Move, Movea,
    Add, Adda, Sub, Suba, And, Or, Eor, Cmp, Cmpa, Muls,
    Addi, Subi, Andi, Ori, Eori, Cmpi,
    Moveq, Addq, Subq,
    Asl, Asr, Lsl, Lsr, Rol, Ror,
    Btst, Bset, Bclr,
    Clr, Neg, Not, Tst, Tas,
    Scc(Cond),
    Jmp, Jsr, Lea, Pea, Nop, Rts, Rte, Trap, Swap, Ext,
    Bra, Bsr, Bcc(Cond), Dbcc(Cond),
    Movem, Movep, Addx, Cmpm,
    MoveToSr, MoveFromSr, // move.w <ea>,sr / move.w sr,<ea>
    AndiCcr, OriCcr,      // andi.b #imm,ccr / ori.b #imm,ccr
}

/// 68000 condition codes; discriminant is the 4-bit cc field (bits 11–8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cond {
    T = 0x0, F = 0x1, Hi = 0x2, Ls = 0x3, Cc = 0x4, Cs = 0x5, Ne = 0x6, Eq = 0x7,
    Vc = 0x8, Vs = 0x9, Pl = 0xA, Mi = 0xB, Ge = 0xC, Lt = 0xD, Gt = 0xE, Le = 0xF,
}
impl Cond {
    #[inline]
    pub fn cc(self) -> u16 { self as u16 }
}

/// Operation size. `B`/`W`/`L` are the data sizes; `S` is the 8-bit short branch
/// displacement (never used by non-branch forms). Do not reorder `B,W,L`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    B,
    W,
    L,
    S,
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
    /// MOVEM register-list mask in canonical order bit0=D0..bit7=D7,bit8=A0..bit15=A7.
    /// The predecrement (-(An)) bit-order reversal is applied inside encode_movem.
    RegList(u16),
    /// Resolved branch / DBcc displacement (bytes measured as asl emits them).
    Disp(i32),
    /// The condition-code register (andi/ori to ccr).
    Ccr,
    /// The status register (move to/from sr).
    Sr,
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
        Mnemonic::Add | Mnemonic::Sub | Mnemonic::And | Mnemonic::Or
        | Mnemonic::Cmp | Mnemonic::Eor
        | Mnemonic::Cmpa | Mnemonic::Adda | Mnemonic::Suba
        | Mnemonic::Muls => encode_alu_ea(inst),
        other => Err(IsaError::UnsupportedForm(format!("{other:?}"))),
    }
}

/// The three data sizes as their 2-bit code (`.b`=0, `.w`=1, `.l`=2).
/// Errors on `.s`, which has no meaning for a data-processing form.
fn size_code(size: Size) -> Result<u16, IsaError> {
    match size {
        Size::B => Ok(0),
        Size::W => Ok(1),
        Size::L => Ok(2),
        Size::S => Err(IsaError::UnsupportedForm("ALU-EA form has no short (.s) size".into())),
    }
}

/// Encode the ALU-EA family (`add/sub/and/or/cmp/eor/cmpa/adda/suba/muls`).
///
/// Base word: `base<<12 | reg<<9 | opmode<<6 | (ea_mode<<3 | ea_reg)`, followed by
/// the source/dest `<ea>` extension words. The register field (bits 11–9) holds the
/// Dn (or, for `cmpa/adda/suba`, the An); the other operand supplies the EA.
fn encode_alu_ea(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    let (src, dst) = match inst.ops.as_slice() {
        [s, d] => (s, d),
        _ => {
            return Err(IsaError::OperandCount(format!(
                "{:?} expects 2 operands, got {}",
                inst.mnemonic,
                inst.ops.len()
            )))
        }
    };
    let sz = size_code(inst.size)?;

    // (base bits 15–12, register field, opmode, ea operand)
    let (base, reg, opmode, ea): (u16, u8, u16, &Operand) = match inst.mnemonic {
        // Address-register destination: reg = An, source is the EA.
        Mnemonic::Cmpa | Mnemonic::Adda | Mnemonic::Suba => {
            let an = match dst {
                Operand::An(n) => n & 0b111,
                other => {
                    return Err(IsaError::UnsupportedForm(format!(
                        "{:?} requires An destination, got {other:?}",
                        inst.mnemonic
                    )))
                }
            };
            let base = match inst.mnemonic {
                Mnemonic::Adda => 0b1101,
                Mnemonic::Suba => 0b1001,
                Mnemonic::Cmpa => 0b1011,
                _ => unreachable!(),
            };
            // opmode: .w=011, .l=111 — the size bit is bit 8 of the opmode.
            let opmode = match inst.size {
                Size::W => 0b011,
                Size::L => 0b111,
                _ => {
                    return Err(IsaError::UnsupportedForm(format!(
                        "{:?} is word/long only",
                        inst.mnemonic
                    )))
                }
            };
            (base, an, opmode, src)
        }
        // Word multiply: reg = Dn destination, opmode 111, source is the EA.
        Mnemonic::Muls => {
            let dn = match dst {
                Operand::Dn(n) => n & 0b111,
                other => {
                    return Err(IsaError::UnsupportedForm(format!(
                        "muls requires Dn destination, got {other:?}"
                    )))
                }
            };
            if inst.size != Size::W {
                return Err(IsaError::UnsupportedForm("muls is word only".into()));
            }
            (0b1100, dn, 0b111, src)
        }
        // `<ea>,Dn` only: reg = Dn destination.
        Mnemonic::Cmp => {
            let dn = match dst {
                Operand::Dn(n) => n & 0b111,
                other => {
                    return Err(IsaError::UnsupportedForm(format!(
                        "cmp requires Dn destination, got {other:?}"
                    )))
                }
            };
            (0b1011, dn, sz, src)
        }
        // `Dn,<ea>` only: reg = Dn source.
        Mnemonic::Eor => {
            let dn = match src {
                Operand::Dn(n) => n & 0b111,
                other => {
                    return Err(IsaError::UnsupportedForm(format!(
                        "eor requires Dn source, got {other:?}"
                    )))
                }
            };
            (0b1011, dn, sz + 0b100, dst)
        }
        // Bidirectional: `<ea>,Dn` (opmode 0xx) or `Dn,<ea>` (opmode 1xx).
        Mnemonic::Add | Mnemonic::Sub | Mnemonic::And | Mnemonic::Or => {
            let base = match inst.mnemonic {
                Mnemonic::Add => 0b1101,
                Mnemonic::Sub => 0b1001,
                Mnemonic::And => 0b1100,
                Mnemonic::Or => 0b1000,
                _ => unreachable!(),
            };
            match (src, dst) {
                // <ea>,Dn — reg is the Dn destination, opmode 0xx.
                (ea, Operand::Dn(n)) => (base, n & 0b111, sz, ea),
                // Dn,<ea> — reg is the Dn source, opmode 1xx.
                (Operand::Dn(n), ea) => (base, n & 0b111, sz + 0b100, ea),
                (s, d) => {
                    return Err(IsaError::UnsupportedForm(format!(
                        "{:?} requires a Dn operand, got {s:?},{d:?}",
                        inst.mnemonic
                    )))
                }
            }
        }
        _ => unreachable!(),
    };

    let (ea_mode, ea_reg, ea_ext) = encode_ea(ea, Field::Source, inst.size)?;
    let word: u16 = (base << 12)
        | ((reg as u16) << 9)
        | (opmode << 6)
        | ((ea_mode as u16) << 3)
        | (ea_reg as u16);
    let mut out = Vec::with_capacity(2 + 2 * ea_ext.len());
    out.extend_from_slice(&word.to_be_bytes());
    for w in ea_ext {
        out.extend_from_slice(&w.to_be_bytes());
    }
    Ok(out)
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
        Size::S => return Err(IsaError::UnsupportedForm("move has no short (.s) size".into())),
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
fn encode_ea(op: &Operand, field: Field, size: Size) -> Result<(u8, u8, Vec<u16>), IsaError> {
    let r = |n: u8| n & 0b111;
    Ok(match *op {
        Operand::Dn(n) => (0b000, r(n), vec![]),
        Operand::An(n) => (0b001, r(n), vec![]),
        Operand::Ind(n) => (0b010, r(n), vec![]),
        Operand::PostInc(n) => (0b011, r(n), vec![]),
        Operand::PreDec(n) => (0b100, r(n), vec![]),
        Operand::Disp16An(d, n) => (0b101, r(n), vec![d as u16]),
        Operand::Disp8AnXn { d, an, xn, long } => (0b110, r(an), vec![brief_ext(d, xn, long)]),
        Operand::AbsW(a) => (0b111, 0b000, vec![a as u16]),
        Operand::AbsL(a) => (0b111, 0b001, vec![(a >> 16) as u16, a as u16]),
        Operand::Pcd16(d) => {
            if let Field::Dest = field {
                return Err(IsaError::IllegalDest("(d16,PC)".into()));
            }
            (0b111, 0b010, vec![d as u16])
        }
        Operand::Imm(v) => {
            if let Field::Dest = field {
                return Err(IsaError::IllegalDest("#imm".into()));
            }
            let ext = match size {
                Size::B => vec![(v as u16) & 0x00FF],
                Size::W => vec![v as u16],
                Size::L => vec![(v >> 16) as u16, v as u16],
                Size::S => return Err(IsaError::UnsupportedForm("#imm has no short (.s) size".into())),
            };
            (0b111, 0b100, ext)
        }
        // These are handled by their family encoders (movem/branch/ccr/sr) in
        // later tasks, never resolved as a general EA.
        Operand::RegList(_) => return Err(IsaError::UnsupportedForm("register list is not a general EA".into())),
        Operand::Disp(_) => return Err(IsaError::UnsupportedForm("branch displacement is not a general EA".into())),
        Operand::Ccr => return Err(IsaError::UnsupportedForm("ccr is not a general EA".into())),
        Operand::Sr => return Err(IsaError::UnsupportedForm("sr is not a general EA".into())),
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

#[cfg(test)]
mod vocab_tests {
    use super::*;

    #[test]
    fn new_vocab_constructs_and_move_still_dispatches() {
        // New mnemonics/operands exist and compile.
        let _ = (Mnemonic::Add, Mnemonic::Bcc(Cond::Eq), Mnemonic::Movem, Mnemonic::Moveq);
        let _ = (Operand::RegList(0x0001), Operand::Disp(4), Operand::Ccr, Operand::Sr);
        let _ = Size::S;
        // Still-unimplemented mnemonics are dispatched but return UnsupportedForm.
        let moveq = Instruction { mnemonic: Mnemonic::Moveq, size: Size::L, ops: vec![Operand::Imm(1), Operand::Dn(0)] };
        assert!(matches!(encode(&moveq), Err(IsaError::UnsupportedForm(_))));
        // Move still works.
        let mv = Instruction { mnemonic: Mnemonic::Move, size: Size::W, ops: vec![Operand::Dn(1), Operand::Dn(0)] };
        assert_eq!(encode(&mv).unwrap(), vec![0x30, 0x01]);
    }
}
