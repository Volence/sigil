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
        Mnemonic::Addi | Mnemonic::Subi | Mnemonic::Andi
        | Mnemonic::Ori | Mnemonic::Eori | Mnemonic::Cmpi => encode_alu_imm(inst),
        Mnemonic::Moveq | Mnemonic::Addq | Mnemonic::Subq => encode_quick(inst),
        Mnemonic::Asl | Mnemonic::Asr | Mnemonic::Lsl | Mnemonic::Lsr
        | Mnemonic::Rol | Mnemonic::Ror => encode_shift(inst),
        Mnemonic::Btst | Mnemonic::Bset | Mnemonic::Bclr => encode_bit(inst),
        Mnemonic::AndiCcr | Mnemonic::OriCcr => encode_ccr_imm(inst),
        Mnemonic::MoveToSr | Mnemonic::MoveFromSr => encode_move_sr(inst),
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

/// Build the immediate extension word(s) for an ALU-immediate operand.
/// `.b` → one word (value in the low byte), `.w` → one word, `.l` → two words (high first).
fn imm_ext_words(size: Size, v: i32) -> Result<Vec<u16>, IsaError> {
    Ok(match size {
        Size::B => vec![(v as u16) & 0x00FF],
        Size::W => vec![v as u16],
        Size::L => vec![(v >> 16) as u16, v as u16],
        Size::S => return Err(IsaError::UnsupportedForm("ALU-immediate has no short (.s) size".into())),
    })
}

/// Encode the ALU-immediate family (`addi/subi/andi/ori/eori/cmpi`).
///
/// Base word: `0000 oooo ss eeeeee`, followed by the immediate extension word(s)
/// **before** the destination EA's extension words.
fn encode_alu_imm(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    let (imm, dst) = match inst.ops.as_slice() {
        [Operand::Imm(v), d] => (*v, d),
        [s, d] => {
            return Err(IsaError::UnsupportedForm(format!(
                "{:?} requires #imm source, got {s:?},{d:?}",
                inst.mnemonic
            )))
        }
        _ => {
            return Err(IsaError::OperandCount(format!(
                "{:?} expects 2 operands, got {}",
                inst.mnemonic,
                inst.ops.len()
            )))
        }
    };
    let sz = size_code(inst.size)?;
    let op: u16 = match inst.mnemonic {
        Mnemonic::Ori => 0b0000,
        Mnemonic::Andi => 0b0010,
        Mnemonic::Subi => 0b0100,
        Mnemonic::Addi => 0b0110,
        Mnemonic::Eori => 0b1010,
        Mnemonic::Cmpi => 0b1100,
        _ => unreachable!(),
    };
    let (ea_mode, ea_reg, ea_ext) = encode_ea(dst, Field::Dest, inst.size)?;
    let word: u16 = (op << 8) | (sz << 6) | ((ea_mode as u16) << 3) | (ea_reg as u16);
    let imm_ext = imm_ext_words(inst.size, imm)?;

    let mut out = Vec::with_capacity(2 + 2 * (imm_ext.len() + ea_ext.len()));
    out.extend_from_slice(&word.to_be_bytes());
    for w in imm_ext {
        out.extend_from_slice(&w.to_be_bytes());
    }
    for w in ea_ext {
        out.extend_from_slice(&w.to_be_bytes());
    }
    Ok(out)
}

/// Encode `andi.b #imm,ccr` (`023C`) / `ori.b #imm,ccr` (`003C`) + one imm word.
fn encode_ccr_imm(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    let imm = match inst.ops.as_slice() {
        [Operand::Imm(v), Operand::Ccr] => *v,
        _ => {
            return Err(IsaError::UnsupportedForm(format!(
                "{:?} requires #imm,ccr operands, got {:?}",
                inst.mnemonic, inst.ops
            )))
        }
    };
    let opcode: u16 = match inst.mnemonic {
        Mnemonic::AndiCcr => 0x023C,
        Mnemonic::OriCcr => 0x003C,
        _ => unreachable!(),
    };
    let imm_word = (imm as u16) & 0x00FF;
    let mut out = Vec::with_capacity(4);
    out.extend_from_slice(&opcode.to_be_bytes());
    out.extend_from_slice(&imm_word.to_be_bytes());
    Ok(out)
}

/// Encode `move.w <ea>,sr` (`46C0 | ea`) / `move.w sr,<ea>` (`40C0 | ea`) + EA ext words.
fn encode_move_sr(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    let (base, ea): (u16, &Operand) = match inst.mnemonic {
        Mnemonic::MoveToSr => match inst.ops.as_slice() {
            [src, Operand::Sr] => (0x46C0, src),
            _ => {
                return Err(IsaError::UnsupportedForm(format!(
                    "move to sr requires <ea>,sr operands, got {:?}",
                    inst.ops
                )))
            }
        },
        Mnemonic::MoveFromSr => match inst.ops.as_slice() {
            [Operand::Sr, dst] => (0x40C0, dst),
            _ => {
                return Err(IsaError::UnsupportedForm(format!(
                    "move from sr requires sr,<ea> operands, got {:?}",
                    inst.ops
                )))
            }
        },
        _ => unreachable!(),
    };
    let field = match inst.mnemonic {
        Mnemonic::MoveToSr => Field::Source,
        _ => Field::Dest,
    };
    let (ea_mode, ea_reg, ea_ext) = encode_ea(ea, field, inst.size)?;
    let word: u16 = base | ((ea_mode as u16) << 3) | (ea_reg as u16);
    let mut out = Vec::with_capacity(2 + 2 * ea_ext.len());
    out.extend_from_slice(&word.to_be_bytes());
    for w in ea_ext {
        out.extend_from_slice(&w.to_be_bytes());
    }
    Ok(out)
}

/// Encode the "quick" family (`moveq`/`addq`/`subq`).
///
/// - `moveq #d,Dn` = `0111 rrr 0 dddddddd` (long; `d` is 8-bit signed data).
/// - `addq #d,<ea>` = `0101 ddd 0 ss eeeeee`; `subq` = `0101 ddd 1 ss eeeeee`.
///   `ddd` = data 1..=8 with **8 encoded as `000`**; `ss` = `size_code`.
fn encode_quick(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    let (imm, dst) = match inst.ops.as_slice() {
        [Operand::Imm(v), d] => (*v, d),
        [s, d] => {
            return Err(IsaError::UnsupportedForm(format!(
                "{:?} requires #imm source, got {s:?},{d:?}",
                inst.mnemonic
            )))
        }
        _ => {
            return Err(IsaError::OperandCount(format!(
                "{:?} expects 2 operands, got {}",
                inst.mnemonic,
                inst.ops.len()
            )))
        }
    };

    if inst.mnemonic == Mnemonic::Moveq {
        let dn = match dst {
            Operand::Dn(n) => (n & 0b111) as u16,
            other => {
                return Err(IsaError::UnsupportedForm(format!(
                    "moveq requires Dn destination, got {other:?}"
                )))
            }
        };
        let data: i8 = i8::try_from(imm).map_err(|_| {
            IsaError::UnsupportedForm(format!("moveq data {imm} does not fit in a signed byte"))
        })?;
        let word = 0x7000u16 | (dn << 9) | (data as u8 as u16);
        return Ok(word.to_be_bytes().to_vec());
    }

    // addq / subq: data 1..=8, with 8 encoded as 000.
    if !(1..=8).contains(&imm) {
        return Err(IsaError::UnsupportedForm(format!(
            "{:?} data must be 1..=8, got {imm}",
            inst.mnemonic
        )));
    }
    let ddd = (imm as u16) & 0b111; // 8 -> 000, else the value itself
    let op_bit: u16 = match inst.mnemonic {
        Mnemonic::Addq => 0,
        Mnemonic::Subq => 1,
        _ => unreachable!(),
    };
    let sz = size_code(inst.size)?;
    let (ea_mode, ea_reg, ea_ext) = encode_ea(dst, Field::Dest, inst.size)?;
    let word: u16 = (0b0101 << 12)
        | (ddd << 9)
        | (op_bit << 8)
        | (sz << 6)
        | ((ea_mode as u16) << 3)
        | (ea_reg as u16);
    let mut out = Vec::with_capacity(2 + 2 * ea_ext.len());
    out.extend_from_slice(&word.to_be_bytes());
    for w in ea_ext {
        out.extend_from_slice(&w.to_be_bytes());
    }
    Ok(out)
}

/// Encode the register shift/rotate family (`asl/asr/lsl/lsr/rol/ror`).
///
/// Base word: `1110 ccc d ss i tt rrr` =
/// `0xE000 | ccc<<9 | d<<8 | ss<<6 | i<<5 | tt<<3 | dst_dn`.
/// - `ccc`: immediate count 1..=8 (**8 → `000`**) when source is `#imm` (`i`=0),
///   or the source Dn number when source is `Dn` (`i`=1).
/// - `(d, tt)` by mnemonic: `asr`=(0,00), `asl`=(1,00), `lsr`=(0,01), `lsl`=(1,01),
///   `ror`=(0,11), `rol`=(1,11).
///
/// Only the register (Dn destination) form is supported; the word memory-shift form
/// is unused by the Aeon corpus.
fn encode_shift(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
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
    let dst_dn = match dst {
        Operand::Dn(n) => (n & 0b111) as u16,
        other => {
            return Err(IsaError::UnsupportedForm(format!(
                "{:?} requires Dn destination, got {other:?}",
                inst.mnemonic
            )))
        }
    };
    let (d, tt): (u16, u16) = match inst.mnemonic {
        Mnemonic::Asr => (0, 0b00),
        Mnemonic::Asl => (1, 0b00),
        Mnemonic::Lsr => (0, 0b01),
        Mnemonic::Lsl => (1, 0b01),
        Mnemonic::Ror => (0, 0b11),
        Mnemonic::Rol => (1, 0b11),
        _ => unreachable!(),
    };
    // Discriminate immediate-count (i=0) vs register-count (i=1) by the source.
    let (ccc, i): (u16, u16) = match src {
        Operand::Imm(v) => {
            if !(1..=8).contains(v) {
                return Err(IsaError::UnsupportedForm(format!(
                    "{:?} immediate count must be 1..=8, got {v}",
                    inst.mnemonic
                )));
            }
            ((*v as u16) & 0b111, 0) // 8 -> 000, else the value itself
        }
        Operand::Dn(n) => ((*n & 0b111) as u16, 1),
        other => {
            return Err(IsaError::UnsupportedForm(format!(
                "{:?} source must be #imm count or Dn, got {other:?}",
                inst.mnemonic
            )))
        }
    };
    let sz = size_code(inst.size)?;
    let word: u16 = 0xE000 | (ccc << 9) | (d << 8) | (sz << 6) | (i << 5) | (tt << 3) | dst_dn;
    Ok(word.to_be_bytes().to_vec())
}

/// Encode the bit-manipulation family (`btst/bset/bclr`, static `#n` and dynamic `Dn`).
///
/// - Static (`#n,<ea>`): `0000 1000 tt eeeeee` then the bit-number word `(#n as u16)`,
///   then the destination EA's extension words (bit-number word comes first).
/// - Dynamic (`Dn,<ea>`): `0000 rrr 1 tt eeeeee` (`rrr`=source Dn), then the EA ext words.
/// - `tt`: btst=00, bclr=10, bset=11.
///
/// Size is implicit (byte for a memory destination, long for a Dn destination); asl picks
/// it from the destination, so the corpus `size` field is informational. The bit-number
/// extension word is always a single word; the destination EA's own extension words depend
/// only on the EA form, so the size passed to `encode_ea` does not affect them here.
fn encode_bit(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
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
    let tt: u16 = match inst.mnemonic {
        Mnemonic::Btst => 0b00,
        Mnemonic::Bclr => 0b10,
        Mnemonic::Bset => 0b11,
        _ => unreachable!(),
    };
    // Implicit size: long for a Dn destination, byte for a memory destination.
    let size = match dst {
        Operand::Dn(_) => Size::L,
        _ => Size::B,
    };
    let (ea_mode, ea_reg, ea_ext) = encode_ea(dst, Field::Dest, size)?;
    let ea: u16 = ((ea_mode as u16) << 3) | (ea_reg as u16);

    let mut out = Vec::new();
    match src {
        // Static form: bit number is an immediate carried in an extension word.
        Operand::Imm(v) => {
            let bit_word = u16::try_from(*v).map_err(|_| {
                IsaError::UnsupportedForm(format!(
                    "{:?} bit number {v} does not fit a word",
                    inst.mnemonic
                ))
            })?;
            let word: u16 = 0x0800 | (tt << 6) | ea;
            out.extend_from_slice(&word.to_be_bytes());
            out.extend_from_slice(&bit_word.to_be_bytes());
        }
        // Dynamic form: bit number lives in a Dn selected by bits 11-9.
        Operand::Dn(n) => {
            let dn = (n & 0b111) as u16;
            let word: u16 = 0x0100 | (dn << 9) | (tt << 6) | ea;
            out.extend_from_slice(&word.to_be_bytes());
        }
        other => {
            return Err(IsaError::UnsupportedForm(format!(
                "{:?} source must be #imm or Dn, got {other:?}",
                inst.mnemonic
            )))
        }
    }
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
        let swap = Instruction { mnemonic: Mnemonic::Swap, size: Size::W, ops: vec![Operand::Dn(0)] };
        assert!(matches!(encode(&swap), Err(IsaError::UnsupportedForm(_))));
        // Move still works.
        let mv = Instruction { mnemonic: Mnemonic::Move, size: Size::W, ops: vec![Operand::Dn(1), Operand::Dn(0)] };
        assert_eq!(encode(&mv).unwrap(), vec![0x30, 0x01]);
    }
}
