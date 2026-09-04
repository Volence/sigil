//! `sigil-isa` 68000 decoder — the encoder's independent mirror, written from
//! the opcode-map direction (M68000PRM Section 8 bit patterns) rather than by
//! inverting `m68k::encode`'s code, so a single encoder bug cannot survive the
//! round trip. Scope: exactly the instruction/EA forms `m68k::encode` can emit.
//! Anything outside that set — including REAL 68000 instructions sigil never
//! emits (`addx -(An),-(An)`, `subx`, `abcd`, `exg`, `roxl`, `bchg`, `chk`, …)
//! — decodes to a loud [`DecodeError::Unknown`] naming the word. That strictness
//! is the point: the motivating defect class is an encoder arm silently emitting
//! a NEIGHBOUR opcode (`add.w d2,a1` → `D549` = `ADDX -(An),-(An)`), and a
//! decoder that "helpfully" understood the neighbour would wave it through.
//!
//! The public surface is [`decode_exact`] (one instruction, must consume the
//! whole slice), [`canonicalize`] (the equivalence-relation normal form), and
//! [`roundtrip_check`]/[`assert_roundtrip`] (the self-check the encoder tests
//! and the harness stream pass share).

use crate::m68k::{
    ea_class, encode, Cond, EaClass, EaSet, Instruction, Mnemonic, Operand, Size, Xn,
};

/// Why a byte slice failed to decode. Every variant names the opcode word so a
/// failure in a bulk pass is diagnosable without re-running anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// The slice ends before the instruction's extension words do.
    Truncated { word: u16, have: usize, need: usize },
    /// A word sigil's encoder can never emit — either not a 68000 instruction
    /// at all, or a real instruction outside sigil's emitted set.
    Unknown { word: u16, why: String },
    /// The opcode word is fine but an extension word carries bits the encoder
    /// never sets (brief-extension scale/full bits, a nonzero high byte on a
    /// byte-size immediate).
    BadExtension { word: u16, ext: u16, why: String },
    /// [`decode_exact`] only: the slice holds one complete instruction of
    /// `used` bytes plus leftover bytes that are not part of it.
    TrailingBytes { word: u16, used: usize, len: usize },
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::Truncated { word, have, need } => write!(
                f,
                "truncated: opcode {word:04X} needs {need} bytes, slice has {have}"
            ),
            DecodeError::Unknown { word, why } => write!(f, "unknown opcode {word:04X}: {why}"),
            DecodeError::BadExtension { word, ext, why } => {
                write!(f, "bad extension {ext:04X} under opcode {word:04X}: {why}")
            }
            DecodeError::TrailingBytes { word, used, len } => write!(
                f,
                "trailing bytes: opcode {word:04X} is a {used}-byte instruction but the slice has {len}"
            ),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Big-endian word reader over the instruction slice.
struct Rd<'a> {
    bytes: &'a [u8],
    pos: usize,
    /// The opcode word, for error context.
    word: u16,
}

impl<'a> Rd<'a> {
    fn word(&mut self) -> Result<u16, DecodeError> {
        if self.pos + 2 > self.bytes.len() {
            return Err(DecodeError::Truncated {
                word: self.word,
                have: self.bytes.len(),
                need: self.pos + 2,
            });
        }
        let w = u16::from_be_bytes([self.bytes[self.pos], self.bytes[self.pos + 1]]);
        self.pos += 2;
        Ok(w)
    }
}

fn unknown(word: u16, why: impl Into<String>) -> DecodeError {
    DecodeError::Unknown { word, why: why.into() }
}

/// Decode the brief-format extension word for `(d8,An,Xn)` / `(d8,PC,Xn)`.
/// Rejects the 68020 scale bits (10–9) and full-format bit (8), which the
/// encoder never sets.
fn brief(rd: &mut Rd) -> Result<(i8, Xn, bool), DecodeError> {
    let ext = rd.word()?;
    if ext & 0x0700 != 0 {
        return Err(DecodeError::BadExtension {
            word: rd.word,
            ext,
            why: "brief extension carries 68020 scale/full bits sigil never emits".into(),
        });
    }
    let num = ((ext >> 12) & 0b111) as u8;
    let xn = if ext & 0x8000 != 0 { Xn::A(num) } else { Xn::D(num) };
    let long = ext & 0x0800 != 0;
    Ok((ext as u8 as i8, xn, long))
}

/// Decode one EA (mode/reg fields + extension words) as an `Operand`, then
/// validate it against `allowed` — the same operand-table row the encoder's
/// family arm enforces — plus the An-is-never-a-byte-operand rule. A pattern
/// outside the set is a word the encoder cannot have produced, so it is
/// [`DecodeError::Unknown`], never a silent best-effort operand.
fn ea(rd: &mut Rd, mode: u16, reg: u16, size: Size, allowed: EaSet) -> Result<Operand, DecodeError> {
    let r = reg as u8;
    let op = match mode {
        0b000 => Operand::Dn(r),
        0b001 => Operand::An(r),
        0b010 => Operand::Ind(r),
        0b011 => Operand::PostInc(r),
        0b100 => Operand::PreDec(r),
        0b101 => Operand::Disp16An(rd.word()? as i16, r),
        0b110 => {
            let (d, xn, long) = brief(rd)?;
            Operand::Disp8AnXn { d, an: r, xn, long }
        }
        0b111 => match reg {
            0b000 => Operand::AbsW(rd.word()? as i16),
            0b001 => {
                let hi = rd.word()? as u32;
                let lo = rd.word()? as u32;
                Operand::AbsL(((hi << 16) | lo) as i32)
            }
            0b010 => Operand::Pcd16(rd.word()? as i16),
            0b011 => {
                let (d, xn, long) = brief(rd)?;
                Operand::Pcd8Xn { d, xn, long }
            }
            0b100 => Operand::Imm(imm_ext(rd, size)?),
            _ => return Err(unknown(rd.word, format!("EA mode 111 reg {reg:03b} is not a 68000 mode"))),
        },
        _ => unreachable!("mode is a 3-bit field"),
    };
    let class = ea_class(&op).expect("every decoded operand is a general EA");
    if !allowed.contains(class) {
        return Err(unknown(
            rd.word,
            format!("EA {} is not legal for this operand position — sigil never emits this word", class.spelling()),
        ));
    }
    if class == EaClass::An && size == Size::B {
        return Err(unknown(rd.word, "byte-size access to an address register does not exist"));
    }
    Ok(op)
}

/// Read an immediate extension at `size`'s width. A byte immediate lives in the
/// low byte of one word whose high byte the encoder always zeroes — a nonzero
/// high byte is a word sigil never emits. Word immediates decode zero-extended;
/// [`canonicalize`] masks the encode-side value the same way.
fn imm_ext(rd: &mut Rd, size: Size) -> Result<i32, DecodeError> {
    Ok(match size {
        Size::B => {
            let w = rd.word()?;
            if w & 0xFF00 != 0 {
                return Err(DecodeError::BadExtension {
                    word: rd.word,
                    ext: w,
                    why: "byte immediate with a nonzero high byte".into(),
                });
            }
            (w & 0x00FF) as i32
        }
        Size::W => rd.word()? as i32,
        Size::L => {
            let hi = rd.word()? as u32;
            let lo = rd.word()? as u32;
            ((hi << 16) | lo) as i32
        }
        Size::S => unreachable!("no decoder path requests a .s immediate"),
    })
}

fn data_size(ss: u16, word: u16) -> Result<Size, DecodeError> {
    match ss {
        0b00 => Ok(Size::B),
        0b01 => Ok(Size::W),
        0b10 => Ok(Size::L),
        _ => Err(unknown(word, "size field 11 has no data size")),
    }
}

/// Decode exactly one instruction that occupies the whole slice — the shape the
/// round-trip check needs, since the encoder hands back exactly one
/// instruction's bytes. Errors if bytes remain after the instruction.
///
/// One slice-delimited leniency, for the backend's fixup placeholders: a 2-byte
/// slice whose word is a branch with displacement byte `00` decodes as the
/// `.s`-branch placeholder (`Disp(0)`). In a real instruction STREAM `6x00`
/// always introduces a word-form branch, but `M68kBackend::lower_branch`
/// legitimately encodes `bra.s` with a zero placeholder the linker patches via
/// `PcRel8`, and the 2-byte slice — which cannot hold a word form — is the
/// unambiguous witness of that form here.
pub fn decode_exact(bytes: &[u8]) -> Result<Instruction, DecodeError> {
    if bytes.len() == 2 {
        let w = u16::from_be_bytes([bytes[0], bytes[1]]);
        if w & 0xF000 == 0x6000 && w & 0x00FF == 0 {
            return Ok(Instruction {
                mnemonic: branch_mnemonic(w),
                size: Size::S,
                ops: vec![Operand::Disp(0)],
            });
        }
    }
    let (inst, used) = decode_one(bytes)?;
    if used != bytes.len() {
        return Err(DecodeError::TrailingBytes {
            word: u16::from_be_bytes([bytes[0], bytes[1]]),
            used,
            len: bytes.len(),
        });
    }
    Ok(inst)
}

fn branch_mnemonic(w: u16) -> Mnemonic {
    match (w >> 8) & 0xF {
        0x0 => Mnemonic::Bra,
        0x1 => Mnemonic::Bsr,
        cc => Mnemonic::Bcc(Cond::from_cc(cc)),
    }
}

/// Decode one instruction from the front of `bytes`; returns it with the byte
/// count consumed.
pub fn decode_one(bytes: &[u8]) -> Result<(Instruction, usize), DecodeError> {
    if bytes.len() < 2 {
        return Err(DecodeError::Truncated { word: 0, have: bytes.len(), need: 2 });
    }
    let w = u16::from_be_bytes([bytes[0], bytes[1]]);
    let mut rd = Rd { bytes, pos: 2, word: w };
    let inst = decode_word(w, &mut rd)?;
    let used = rd.pos;
    Ok((inst, used))
}

fn inst(mnemonic: Mnemonic, size: Size, ops: Vec<Operand>) -> Instruction {
    Instruction { mnemonic, size, ops }
}

fn decode_word(w: u16, rd: &mut Rd) -> Result<Instruction, DecodeError> {
    let mode = (w >> 3) & 0b111;
    let reg9 = (w >> 9) & 0b111;
    let r0 = w & 0b111;
    match w >> 12 {
        0b0000 => decode_line0(w, rd, mode, reg9, r0),
        0b0001..=0b0011 => decode_move(w, rd),
        0b0100 => decode_line4(w, rd, mode, reg9, r0),
        0b0101 => decode_line5(w, rd, mode, reg9, r0),
        0b0110 => {
            let low = w & 0x00FF;
            let mn = branch_mnemonic(w);
            if low == 0 {
                let d = rd.word()? as i16;
                Ok(inst(mn, Size::W, vec![Operand::Disp(d as i32)]))
            } else {
                Ok(inst(mn, Size::S, vec![Operand::Disp(low as u8 as i8 as i32)]))
            }
        }
        0b0111 => {
            if w & 0x0100 != 0 {
                return Err(unknown(w, "line 7 with bit 8 set is not moveq"));
            }
            let data = (w & 0xFF) as u8 as i8 as i32;
            Ok(inst(Mnemonic::Moveq, Size::L, vec![Operand::Imm(data), Operand::Dn(reg9 as u8)]))
        }
        0b1000 => decode_alu_ea(w, rd, AluFamily::OrDiv),
        0b1001 => decode_alu_ea(w, rd, AluFamily::Sub),
        0b1010 => Err(unknown(w, "line-A (unimplemented trap line)")),
        0b1011 => decode_alu_ea(w, rd, AluFamily::CmpEor),
        0b1100 => decode_alu_ea(w, rd, AluFamily::AndMul),
        0b1101 => decode_alu_ea(w, rd, AluFamily::Add),
        0b1110 => decode_shift(w, rd),
        0b1111 => Err(unknown(w, "line-F (coprocessor line)")),
        _ => unreachable!("4-bit field"),
    }
}

/// Line 0000: ALU-immediate, CCR immediates, bit ops (static + dynamic), movep.
fn decode_line0(w: u16, rd: &mut Rd, mode: u16, reg9: u16, r0: u16) -> Result<Instruction, DecodeError> {
    // andi/ori to CCR: exact opcode words, one byte-immediate extension.
    if w == 0x023C || w == 0x003C {
        let mn = if w == 0x023C { Mnemonic::AndiCcr } else { Mnemonic::OriCcr };
        let imm = imm_ext(rd, Size::B)?;
        return Ok(inst(mn, Size::B, vec![Operand::Imm(imm), Operand::Ccr]));
    }
    if w & 0x0100 != 0 {
        // Bit 8 set: movep (EA-field 001) or a dynamic bit op.
        if mode == 0b001 {
            let opmode = (w >> 6) & 0b111; // 100..111 (bit 8 is the top bit)
            let size = if opmode & 0b001 != 0 { Size::L } else { Size::W };
            let to_mem = opmode & 0b010 != 0;
            let d = rd.word()? as i16;
            let (dn, an) = (Operand::Dn(reg9 as u8), Operand::Disp16An(d, r0 as u8));
            let ops = if to_mem { vec![dn, an] } else { vec![an, dn] };
            return Ok(inst(Mnemonic::Movep, size, ops));
        }
        let (mn, allowed) = bit_op(w, (w >> 6) & 0b11)?;
        let size = bit_op_size(mode);
        let dst = ea(rd, mode, r0, size, allowed)?;
        return Ok(inst(mn, size, vec![Operand::Dn(reg9 as u8), dst]));
    }
    if reg9 == 0b100 {
        // Static bit op: bit-number word first, then the destination EA.
        let (mn, allowed) = bit_op(w, (w >> 6) & 0b11)?;
        let bit = rd.word()? as i32;
        let size = bit_op_size(mode);
        let dst = ea(rd, mode, r0, size, allowed)?;
        return Ok(inst(mn, size, vec![Operand::Imm(bit), dst]));
    }
    let mn = match reg9 {
        0b000 => Mnemonic::Ori,
        0b001 => Mnemonic::Andi,
        0b010 => Mnemonic::Subi,
        0b011 => Mnemonic::Addi,
        0b101 => Mnemonic::Eori,
        0b110 => Mnemonic::Cmpi,
        _ => return Err(unknown(w, "line 0 op field is not an ALU-immediate sigil emits")),
    };
    let size = data_size((w >> 6) & 0b11, w)?;
    let imm = imm_ext(rd, size)?;
    let dst = ea(rd, mode, r0, size, EaSet::DATA_ALTERABLE)?;
    Ok(inst(mn, size, vec![Operand::Imm(imm), dst]))
}

/// `tt` field (bits 7–6) → bit-op mnemonic + its destination class. All four
/// rows are real 68000 instructions and all four are emitted; only `btst`
/// reads without writing, so only it takes the wider DATA row.
fn bit_op(_w: u16, tt: u16) -> Result<(Mnemonic, EaSet), DecodeError> {
    match tt {
        0b00 => Ok((Mnemonic::Btst, EaSet::DATA)),
        0b01 => Ok((Mnemonic::Bchg, EaSet::DATA_ALTERABLE)),
        0b10 => Ok((Mnemonic::Bclr, EaSet::DATA_ALTERABLE)),
        _ => Ok((Mnemonic::Bset, EaSet::DATA_ALTERABLE)),
    }
}

/// Bit ops size by destination: long for `Dn`, byte for memory (the ISA's
/// implicit rule — the encoder derives it the same way).
fn bit_op_size(mode: u16) -> Size {
    if mode == 0b000 { Size::L } else { Size::B }
}

/// Lines 0001/0010/0011: MOVE, with destination mode 001 decoding as MOVEA —
/// the two are one opcode layout, and MOVEA is the canonical spelling here.
fn decode_move(w: u16, rd: &mut Rd) -> Result<Instruction, DecodeError> {
    let size = match w >> 12 {
        0b0001 => Size::B,
        0b0011 => Size::W,
        _ => Size::L,
    };
    let src = ea(rd, (w >> 3) & 0b111, w & 0b111, size, EaSet::ALL)?;
    let dst_mode = (w >> 6) & 0b111;
    let dst_reg = (w >> 9) & 0b111;
    if dst_mode == 0b001 {
        if size == Size::B {
            return Err(unknown(w, "movea has no byte form"));
        }
        return Ok(inst(Mnemonic::Movea, size, vec![src, Operand::An(dst_reg as u8)]));
    }
    let dst = ea(rd, dst_mode, dst_reg, size, EaSet::DATA_ALTERABLE)?;
    Ok(inst(Mnemonic::Move, size, vec![src, dst]))
}

/// Line 0100: the misc group.
fn decode_line4(w: u16, rd: &mut Rd, mode: u16, reg9: u16, r0: u16) -> Result<Instruction, DecodeError> {
    match w {
        0x4E71 => return Ok(inst(Mnemonic::Nop, Size::W, vec![])),
        0x4E73 => return Ok(inst(Mnemonic::Rte, Size::W, vec![])),
        0x4E75 => return Ok(inst(Mnemonic::Rts, Size::W, vec![])),
        0x4AFC => return Ok(inst(Mnemonic::Illegal, Size::W, vec![])),
        _ => {}
    }
    if w & 0xFFF0 == 0x4E40 {
        return Ok(inst(Mnemonic::Trap, Size::W, vec![Operand::Imm((w & 0xF) as i32)]));
    }
    // `move.l An,usp` / `move.l usp,An` — one word, the An in bits 2-0, no EA.
    if w & 0xFFF0 == 0x4E60 {
        let an = Operand::An((w & 0b111) as u8);
        return Ok(if w & 0x0008 == 0 {
            inst(Mnemonic::MoveToUsp, Size::L, vec![an, Operand::Usp])
        } else {
            inst(Mnemonic::MoveFromUsp, Size::L, vec![Operand::Usp, an])
        });
    }
    match w & 0xFFC0 {
        0x40C0 => {
            let dst = ea(rd, mode, r0, Size::W, EaSet::DATA_ALTERABLE)?;
            return Ok(inst(Mnemonic::MoveFromSr, Size::W, vec![Operand::Sr, dst]));
        }
        0x44C0 => {
            let src = ea(rd, mode, r0, Size::W, EaSet::DATA)?;
            return Ok(inst(Mnemonic::MoveToCcr, Size::W, vec![src, Operand::Ccr]));
        }
        0x46C0 => {
            let src = ea(rd, mode, r0, Size::W, EaSet::DATA)?;
            return Ok(inst(Mnemonic::MoveToSr, Size::W, vec![src, Operand::Sr]));
        }
        0x4AC0 => {
            let dst = ea(rd, mode, r0, Size::B, EaSet::DATA_ALTERABLE)?;
            return Ok(inst(Mnemonic::Tas, Size::B, vec![dst]));
        }
        0x4E80 | 0x4EC0 => {
            let mn = if w & 0xFFC0 == 0x4E80 { Mnemonic::Jsr } else { Mnemonic::Jmp };
            let target = ea(rd, mode, r0, Size::L, EaSet::CONTROL)?;
            return Ok(inst(mn, Size::L, vec![target]));
        }
        0x4840 => {
            if mode == 0b000 {
                return Ok(inst(Mnemonic::Swap, Size::W, vec![Operand::Dn(r0 as u8)]));
            }
            let addr = ea(rd, mode, r0, Size::L, EaSet::CONTROL)?;
            return Ok(inst(Mnemonic::Pea, Size::L, vec![addr]));
        }
        0x4880 | 0x48C0 | 0x4C80 | 0x4CC0 => return decode_ext_movem(w, rd, mode, r0),
        _ => {}
    }
    if w & 0xF1C0 == 0x41C0 {
        let src = ea(rd, mode, r0, Size::L, EaSet::CONTROL)?;
        return Ok(inst(Mnemonic::Lea, Size::L, vec![src, Operand::An(reg9 as u8)]));
    }
    if let base @ (0x4200 | 0x4400 | 0x4600 | 0x4A00) = w & 0xFF00 {
        let size = data_size((w >> 6) & 0b11, w)?;
        let (mn, allowed) = match base {
            0x4200 => (Mnemonic::Clr, EaSet::DATA_ALTERABLE),
            0x4400 => (Mnemonic::Neg, EaSet::DATA_ALTERABLE),
            0x4600 => (Mnemonic::Not, EaSet::DATA_ALTERABLE),
            _ => (Mnemonic::Tst, EaSet::DATA_ALTERABLE),
        };
        let op = ea(rd, mode, r0, size, allowed)?;
        return Ok(inst(mn, size, vec![op]));
    }
    Err(unknown(w, "line 4 word outside sigil's emitted set"))
}

/// `0x4880/0x48C0/0x4C80/0x4CC0`: `ext` when the EA field is register-direct,
/// otherwise MOVEM (whose predecrement mask is emitted bit-reversed and is
/// reversed BACK here, so the decoded `RegList` is always canonical order).
fn decode_ext_movem(w: u16, rd: &mut Rd, mode: u16, r0: u16) -> Result<Instruction, DecodeError> {
    let load = w & 0x0400 != 0;
    let size = if w & 0x0040 != 0 { Size::L } else { Size::W };
    if mode == 0b000 {
        if load {
            return Err(unknown(w, "ext-shaped word on the movem-load base"));
        }
        return Ok(inst(Mnemonic::Ext, size, vec![Operand::Dn(r0 as u8)]));
    }
    let mask_word = rd.word()?;
    let allowed = if load {
        EaSet::CONTROL.or(EaSet::of(EaClass::PostInc))
    } else {
        EaSet::CONTROL_ALTERABLE.or(EaSet::of(EaClass::PreDec))
    };
    let mem = ea(rd, mode, r0, size, allowed)?;
    let mask = if matches!(mem, Operand::PreDec(_)) { mask_word.reverse_bits() } else { mask_word };
    let ops = if load {
        vec![mem, Operand::RegList(mask)]
    } else {
        vec![Operand::RegList(mask), mem]
    };
    Ok(inst(Mnemonic::Movem, size, ops))
}

/// Line 0101: DBcc, Scc, addq/subq.
fn decode_line5(w: u16, rd: &mut Rd, mode: u16, reg9: u16, r0: u16) -> Result<Instruction, DecodeError> {
    if (w >> 6) & 0b11 == 0b11 {
        let cond = Cond::from_cc((w >> 8) & 0xF);
        if mode == 0b001 {
            let d = rd.word()? as i16;
            return Ok(inst(
                Mnemonic::Dbcc(cond),
                Size::W,
                vec![Operand::Dn(r0 as u8), Operand::Disp(d as i32)],
            ));
        }
        let dst = ea(rd, mode, r0, Size::B, EaSet::DATA_ALTERABLE)?;
        return Ok(inst(Mnemonic::Scc(cond), Size::B, vec![dst]));
    }
    let mn = if w & 0x0100 != 0 { Mnemonic::Subq } else { Mnemonic::Addq };
    let size = data_size((w >> 6) & 0b11, w)?;
    let data = if reg9 == 0 { 8 } else { reg9 as i32 };
    let dst = ea(rd, mode, r0, size, EaSet::ALTERABLE)?;
    Ok(inst(mn, size, vec![Operand::Imm(data), dst]))
}

/// Which line-8/9/B/C/D family a word belongs to.
enum AluFamily {
    OrDiv,
    Sub,
    CmpEor,
    AndMul,
    Add,
}

/// Lines 1000/1001/1011/1100/1101 — the ALU-EA group, where the aliasing
/// neighbours live. Register-direct EA fields in the `Dn,<ea>` direction are
/// the extended/decimal/exchange forms (`addx -(An),-(An)` et al.); everything
/// sigil does not emit is a named [`DecodeError::Unknown`].
fn decode_alu_ea(w: u16, rd: &mut Rd, fam: AluFamily) -> Result<Instruction, DecodeError> {
    let reg9 = (w >> 9) & 0b111;
    let opmode = (w >> 6) & 0b111;
    let mode = (w >> 3) & 0b111;
    let r0 = w & 0b111;

    // opmode x11: the word-form specials (An-destination arithmetic, mul/div).
    if opmode & 0b011 == 0b011 {
        let signed = opmode & 0b100 != 0;
        return match fam {
            AluFamily::OrDiv => {
                let src = ea(rd, mode, r0, Size::W, EaSet::DATA)?;
                let mn = if signed { Mnemonic::Divs } else { Mnemonic::Divu };
                Ok(inst(mn, Size::W, vec![src, Operand::Dn(reg9 as u8)]))
            }
            AluFamily::AndMul => {
                let src = ea(rd, mode, r0, Size::W, EaSet::DATA)?;
                let mn = if signed { Mnemonic::Muls } else { Mnemonic::Mulu };
                Ok(inst(mn, Size::W, vec![src, Operand::Dn(reg9 as u8)]))
            }
            AluFamily::Sub | AluFamily::CmpEor | AluFamily::Add => {
                let size = if signed { Size::L } else { Size::W };
                let mn = match fam {
                    AluFamily::Sub => Mnemonic::Suba,
                    AluFamily::CmpEor => Mnemonic::Cmpa,
                    _ => Mnemonic::Adda,
                };
                let src = ea(rd, mode, r0, size, EaSet::ALL)?;
                Ok(inst(mn, size, vec![src, Operand::An(reg9 as u8)]))
            }
        };
    }

    let size = data_size(opmode & 0b011, w)?;
    if opmode & 0b100 == 0 {
        // `<ea>,Dn`: An source is a real EA for add/sub/cmp (word/long), not and/or.
        let (mn, allowed) = match fam {
            AluFamily::OrDiv => (Mnemonic::Or, EaSet::DATA),
            AluFamily::AndMul => (Mnemonic::And, EaSet::DATA),
            AluFamily::Sub => (Mnemonic::Sub, EaSet::ALL),
            AluFamily::CmpEor => (Mnemonic::Cmp, EaSet::ALL),
            AluFamily::Add => (Mnemonic::Add, EaSet::ALL),
        };
        let src = ea(rd, mode, r0, size, allowed)?;
        return Ok(inst(mn, size, vec![src, Operand::Dn(reg9 as u8)]));
    }

    // `Dn,<ea>` direction. Register-direct EA fields here are the specials.
    match fam {
        AluFamily::CmpEor => {
            if mode == 0b001 {
                // cmpm (Ay)+,(Ax)+ — reg9 is Ax (dest), r0 is Ay (source).
                return Ok(inst(
                    Mnemonic::Cmpm,
                    size,
                    vec![Operand::PostInc(r0 as u8), Operand::PostInc(reg9 as u8)],
                ));
            }
            let dst = ea(rd, mode, r0, size, EaSet::DATA_ALTERABLE)?;
            Ok(inst(Mnemonic::Eor, size, vec![Operand::Dn(reg9 as u8), dst]))
        }
        AluFamily::Add => match mode {
            // addx Dy,Dx — reg9 is Dx (dest), r0 is Dy (source).
            0b000 => Ok(inst(
                Mnemonic::Addx,
                size,
                vec![Operand::Dn(r0 as u8), Operand::Dn(reg9 as u8)],
            )),
            0b001 => Err(unknown(w, "addx -(Ay),-(Ax) is not in sigil's emitted set")),
            _ => {
                let dst = ea(rd, mode, r0, size, EaSet::MEMORY_ALTERABLE)?;
                Ok(inst(Mnemonic::Add, size, vec![Operand::Dn(reg9 as u8), dst]))
            }
        },
        AluFamily::Sub => match mode {
            0b000 | 0b001 => Err(unknown(w, "subx is not in sigil's emitted set")),
            _ => {
                let dst = ea(rd, mode, r0, size, EaSet::MEMORY_ALTERABLE)?;
                Ok(inst(Mnemonic::Sub, size, vec![Operand::Dn(reg9 as u8), dst]))
            }
        },
        AluFamily::OrDiv => match mode {
            0b000 | 0b001 => Err(unknown(w, "sbcd is not in sigil's emitted set")),
            _ => {
                let dst = ea(rd, mode, r0, size, EaSet::MEMORY_ALTERABLE)?;
                Ok(inst(Mnemonic::Or, size, vec![Operand::Dn(reg9 as u8), dst]))
            }
        },
        AluFamily::AndMul => match mode {
            // Line C, bit 8 set, register-direct EA: `abcd` (opmode3 100) and
            // the three `exg` pairs. `opmode` here is bits 8-6, so the ISA's
            // 5-bit opmode `01000/01001/10001` reads as
            // (opmode3, mode) = (101,000) / (101,001) / (110,001).
            0b000 if opmode == 0b101 => Ok(inst(
                Mnemonic::Exg,
                Size::L,
                vec![Operand::Dn(reg9 as u8), Operand::Dn(r0 as u8)],
            )),
            0b001 if opmode == 0b101 => Ok(inst(
                Mnemonic::Exg,
                Size::L,
                vec![Operand::An(reg9 as u8), Operand::An(r0 as u8)],
            )),
            0b001 if opmode == 0b110 => Ok(inst(
                Mnemonic::Exg,
                Size::L,
                vec![Operand::Dn(reg9 as u8), Operand::An(r0 as u8)],
            )),
            0b000 | 0b001 => Err(unknown(w, "abcd is not in sigil's emitted set")),
            _ => {
                let dst = ea(rd, mode, r0, size, EaSet::MEMORY_ALTERABLE)?;
                Ok(inst(Mnemonic::And, size, vec![Operand::Dn(reg9 as u8), dst]))
            }
        },
    }
}

/// Line 1110: shifts/rotates — the word memory-shift form (bits 7–6 = 11) and
/// the register form, over all four `tt` rows (`as`/`ls`/`rox`/`ro`). The 68020
/// bitfield region (memory form with bit 11 set) is outside sigil's emitted set.
fn decode_shift(w: u16, rd: &mut Rd) -> Result<Instruction, DecodeError> {
    let d = (w >> 8) & 1;
    if (w >> 6) & 0b11 == 0b11 {
        if w & 0x0800 != 0 {
            return Err(unknown(w, "line E memory form with bit 11 set (68020 bitfield region)"));
        }
        let mn = shift_mnemonic((w >> 9) & 0b11, d, w)?;
        let dst = ea(rd, (w >> 3) & 0b111, w & 0b111, Size::W, EaSet::MEMORY_ALTERABLE)?;
        return Ok(inst(mn, Size::W, vec![dst]));
    }
    let mn = shift_mnemonic((w >> 3) & 0b11, d, w)?;
    let size = data_size((w >> 6) & 0b11, w)?;
    let ccc = (w >> 9) & 0b111;
    let src = if w & 0x0020 != 0 {
        Operand::Dn(ccc as u8)
    } else {
        Operand::Imm(if ccc == 0 { 8 } else { ccc as i32 })
    };
    Ok(inst(mn, size, vec![src, Operand::Dn((w & 0b111) as u8)]))
}

fn shift_mnemonic(tt: u16, d: u16, _w: u16) -> Result<Mnemonic, DecodeError> {
    Ok(match (tt, d) {
        (0b00, 0) => Mnemonic::Asr,
        (0b00, _) => Mnemonic::Asl,
        (0b01, 0) => Mnemonic::Lsr,
        (0b01, _) => Mnemonic::Lsl,
        (0b10, 0) => Mnemonic::Roxr,
        (0b10, _) => Mnemonic::Roxl,
        (0b11, 0) => Mnemonic::Ror,
        _ => Mnemonic::Rol,
    })
}

/// The canonical size for a mnemonic whose ENCODING carries no size field (the
/// encoder ignores `inst.size` for these). `None` means the size field is real.
/// Both [`canonicalize`] and the decoder route through this one table, so the
/// two sides can never disagree about a size the bytes do not store.
fn canonical_size(m: Mnemonic, ops: &[Operand]) -> Option<Size> {
    use Mnemonic::*;
    match m {
        Moveq => Some(Size::L),
        // Bit ops: implicit long on a data register, byte on memory.
        Btst | Bset | Bclr | Bchg => Some(match ops.last() {
            Some(Operand::Dn(_)) => Size::L,
            _ => Size::B,
        }),
        Tas | Scc(_) | AndiCcr | OriCcr => Some(Size::B),
        MoveToSr | MoveFromSr | MoveToCcr => Some(Size::W),
        // USP moves and `exg` have no size field: long by construction.
        MoveToUsp | MoveFromUsp | Exg => Some(Size::L),
        Jmp | Jsr | Lea | Pea => Some(Size::L),
        Nop | Rts | Rte | Trap | Swap | Illegal => Some(Size::W),
        Dbcc(_) => Some(Size::W),
        _ => None,
    }
}

/// The line-1110 shift/rotate family — the eight mnemonics [`shift_mnemonic`]
/// can return. Used by [`canonicalize`]'s Rule R.
fn is_shift(m: Mnemonic) -> bool {
    use Mnemonic::*;
    matches!(m, Asl | Asr | Lsl | Lsr | Rol | Ror | Roxl | Roxr)
}

/// Reduce an `Instruction` to the normal form the round-trip equality compares.
/// This IS the equivalence relation — see [`roundtrip_check`] for the rules and
/// why each is safe.
pub fn canonicalize(i: &Instruction) -> Instruction {
    let mut mnemonic = i.mnemonic;
    let mut size = i.size;
    let mut ops = i.ops.clone();

    // Rule M: `move` to an address register IS `movea` (one opcode layout).
    if mnemonic == Mnemonic::Move {
        if let Some(Operand::An(_)) = ops.last() {
            mnemonic = Mnemonic::Movea;
        }
    }
    // Rule B: `bcc` with the T/F pseudo-conditions IS `bra`/`bsr` in the cc field.
    if let Mnemonic::Bcc(c) = mnemonic {
        match c {
            Cond::T => mnemonic = Mnemonic::Bra,
            Cond::F => mnemonic = Mnemonic::Bsr,
            _ => {}
        }
    }
    // Rule E: `exg An,Dn` IS `exg Dn,An` — the encoding has one data-register
    // slot and one address-register slot, so the written order carries no bits.
    // asl normalises the same way (`exg a0,d0` and `exg d0,a0` both = `C1 88`).
    if mnemonic == Mnemonic::Exg {
        if let [Operand::An(a), Operand::Dn(d)] = ops[..] {
            ops = vec![Operand::Dn(d), Operand::An(a)];
        }
    }
    // Rule R: the shift/rotate family's two alias spellings. `<shift> Dn` IS
    // `<shift> #1,Dn` (register form, count 1) and `<shift> #1,<mem>` IS the
    // one-operand memory form — asl accepts all of them and each pair emits
    // identical bytes, so the decoder can only ever produce one of each pair.
    if is_shift(mnemonic) {
        match ops[..] {
            [Operand::Dn(n)] => ops = vec![Operand::Imm(1), Operand::Dn(n)],
            [Operand::Imm(1), ref mem] if !matches!(mem, Operand::Dn(_)) => {
                ops = vec![mem.clone()]
            }
            _ => {}
        }
    }
    // Rule S: size-less encodings get their canonical size.
    if let Some(s) = canonical_size(mnemonic, &ops) {
        size = s;
    }
    // Rule I: immediates stored at the operation width compare modulo that
    // width (the encoder truncates to the field; the decoder zero-extends).
    canonicalize_imms(mnemonic, size, &mut ops);

    Instruction { mnemonic, size, ops }
}

/// The first out-of-range register number (>7) in `i`, rendered for a
/// diagnostic, or `None` when every register field is in `0..=7`. A register
/// outside the 3-bit field is a front-end contract breach the ENCODER forgives
/// by masking (`Dn(9)` silently becomes `d1`), so the round-trip check refuses
/// it loudly instead of comparing the masked ghost.
fn out_of_range_register(i: &Instruction) -> Option<String> {
    fn xn_bad(xn: &Xn) -> Option<String> {
        match xn {
            Xn::D(n) if *n > 7 => Some(format!("index register Xn::D({n})")),
            Xn::A(n) if *n > 7 => Some(format!("index register Xn::A({n})")),
            _ => None,
        }
    }
    for op in &i.ops {
        let bad = match op {
            Operand::Dn(n) if *n > 7 => Some(format!("Dn({n})")),
            Operand::An(n) if *n > 7 => Some(format!("An({n})")),
            Operand::Ind(n) if *n > 7 => Some(format!("Ind({n})")),
            Operand::PostInc(n) if *n > 7 => Some(format!("PostInc({n})")),
            Operand::PreDec(n) if *n > 7 => Some(format!("PreDec({n})")),
            Operand::Disp16An(_, n) if *n > 7 => Some(format!("Disp16An(_, {n})")),
            Operand::Disp8AnXn { an, xn, .. } => {
                if *an > 7 { Some(format!("Disp8AnXn base An({an})")) } else { xn_bad(xn) }
            }
            Operand::Pcd8Xn { xn, .. } => xn_bad(xn),
            _ => None,
        };
        if bad.is_some() {
            return bad;
        }
    }
    None
}

/// Mask each immediate that the encoding stores at the operation width. Quick
/// data, shift counts, and trap vectors are hard-range-checked by the encoder
/// and pass through unchanged; a static bit number is stored in a full u16
/// extension word, so it compares modulo that word like the CCR immediates
/// compare modulo their byte.
fn canonicalize_imms(m: Mnemonic, size: Size, ops: &mut [Operand]) {
    use Mnemonic::*;
    let width_checked = matches!(
        m,
        Move | Movea
            | Add | Adda | Sub | Suba | And | Or | Eor | Cmp | Cmpa
            | Muls | Mulu | Divs | Divu
            | Addi | Subi | Andi | Ori | Eori | Cmpi
            | MoveToSr
    );
    if width_checked {
        // Only a SOURCE-position immediate exists for these forms; the loop is
        // total over ops because a destination immediate never encodes.
        for op in ops.iter_mut() {
            if let Operand::Imm(v) = op {
                *v = match m {
                    // mul/div/move-to-sr are word ops regardless of `size`.
                    Muls | Mulu | Divs | Divu | MoveToSr => *v & 0xFFFF,
                    _ => match size {
                        Size::B => *v & 0xFF,
                        Size::W => *v & 0xFFFF,
                        _ => *v,
                    },
                };
            }
        }
    }
    if matches!(m, AndiCcr | OriCcr) {
        if let Some(Operand::Imm(v)) = ops.first_mut() {
            *v &= 0xFF;
        }
    }
    if matches!(m, Btst | Bset | Bclr | Bchg) {
        if let Some(Operand::Imm(v)) = ops.first_mut() {
            *v &= 0xFFFF;
        }
    }
}

/// Round-trip one already-encoded instruction: decode `bytes` with the
/// independent decoder and require canonical equality with `inst`. Returns a
/// diagnostic string on failure (bulk passes collect these; [`assert_roundtrip`]
/// panics with it).
///
/// # The equivalence relation
///
/// Equality is over [`canonicalize`]d instructions. The relation is exactly the
/// many-to-one freedom the ENCODING itself has — nothing looser:
///
/// - **Rule M** — `move.w/.l <ea>,An` ≡ `movea.w/.l <ea>,An`: MOVEA is MOVE
///   with destination mode 001, one opcode layout, so the bytes genuinely
///   cannot distinguish the spellings.
/// - **Rule B** — `Bcc(T)` ≡ `bra`, `Bcc(F)` ≡ `bsr`: cc values 0/1 in the
///   branch opcode ARE bra/bsr; no distinct `bt`/`bf` instruction exists.
/// - **Rule S** — for encodings with NO size field (`moveq`, bit ops, `tas`,
///   `Scc`, SR/CCR moves, `jmp`/`jsr`/`lea`/`pea`, fixed words, `dbcc`), the
///   `Instruction.size` value is not stored in the bytes; both sides normalize
///   to one canonical size per form.
/// - **Rule E** — `exg An,Dn` ≡ `exg Dn,An`: the mixed-pair encoding has one
///   slot for each register kind, so the written order is not stored (asl
///   normalises it identically).
/// - **Rule R** — `<shift> Dn` ≡ `<shift> #1,Dn`, and `<shift> #1,<mem>` ≡
///   `<shift> <mem>`: asl accepts all four spellings and each pair emits the
///   same bytes, so the decoder can produce only one member of each pair.
/// - **Rule I** — an immediate stored at the operation width compares modulo
///   that width (`#-1` and `#$FFFF` are the same word-immediate field): the
///   relation proves FIELD fidelity, not value fidelity, because the encoder
///   itself truncates without a fit check. Register numbers get no such
///   forgiveness: a register operand outside `0..=7` on the encode side FAILS
///   the check outright (the encoder masks, so `Dn(9)` would silently emit
///   `d1` — precisely the silent rewrite this check exists to refuse).
///   Everything else — EA mode AND register of every operand, displacement
///   values, masks, conditions, the mnemonic family — must match exactly,
///   which is precisely what catches a wrong EA field or an aliased opcode
///   word (the `D549` class: the bytes decode as `Unknown`/a different family
///   and the check fails loudly).
///
/// A decode failure is ALWAYS a check failure — an emitted instruction the
/// mirror cannot read is the loudest possible signal, never a skip.
pub fn roundtrip_check(inst: &Instruction, bytes: &[u8]) -> Result<(), String> {
    if let Some(bad) = out_of_range_register(inst) {
        return Err(format!(
            "round-trip REJECTED: {bad} is out of range (registers are 3-bit, 0..=7) — \
             the encoder masks it into a different register silently\n  instruction: {inst:?}\n  bytes: {bytes:02X?}"
        ));
    }
    let decoded = decode_exact(bytes).map_err(|e| {
        format!("round-trip DECODE failed: {e}\n  encoded from: {inst:?}\n  bytes: {bytes:02X?}")
    })?;
    let want = canonicalize(inst);
    let got = canonicalize(&decoded);
    if want != got {
        return Err(format!(
            "round-trip MISMATCH:\n  encoded:   {inst:?}\n  bytes:     {bytes:02X?}\n  decoded:   {decoded:?}\n  canonical encode-side: {want:?}\n  canonical decode-side: {got:?}"
        ));
    }
    Ok(())
}

/// Encode `inst`, then [`roundtrip_check`] the result. Panics with the full
/// diagnostic on any failure — the one-line self-check the encoder tests use.
///
/// Because this calls the real `encode`, invoking it while a
/// `m68k::capture::CaptureSession` is live records `inst` into that session's
/// buffer like any other encode; a capture-driven test must not interleave
/// with it.
#[track_caller]
pub fn assert_roundtrip(inst: &Instruction) {
    let bytes = encode(inst)
        .unwrap_or_else(|e| panic!("assert_roundtrip: encode failed for {inst:?}: {e}"));
    if let Err(msg) = roundtrip_check(inst, &bytes) {
        panic!("{msg}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Operand::*;

    fn dec(bytes: &[u8]) -> Instruction {
        decode_exact(bytes).unwrap_or_else(|e| panic!("decode {bytes:02X?}: {e}"))
    }

    /// The motivating defect class: the words the old encoder holes emitted must
    /// NOT decode as the instruction that was written — each is either a named
    /// `Unknown` or a DIFFERENT family the equality then rejects.
    #[test]
    fn alias_words_do_not_decode_as_their_misspelling() {
        // `add.w d2,a1` once emitted D549 = addx.w -(a1),-(a2): outside the set.
        assert!(matches!(
            decode_exact(&[0xD5, 0x49]),
            Err(DecodeError::Unknown { word: 0xD549, .. })
        ));
        // `eor.w d0,a1` once emitted B149 = cmpm.w (a1)+,(a0)+ — decodes as CMPM.
        let cmpm = dec(&[0xB1, 0x49]);
        assert_eq!(cmpm.mnemonic, Mnemonic::Cmpm);
        // `bset d0,a1` once emitted 01C9 = movep.l d0,(d16,a1) — the movep shape
        // consumes a displacement word, so the 2-byte emission is TRUNCATED.
        assert!(matches!(
            decode_exact(&[0x01, 0xC9]),
            Err(DecodeError::Truncated { word: 0x01C9, .. })
        ));
        // `sne a3` once emitted 56CB = dbne d3,<disp> — same truncation shape.
        assert!(matches!(
            decode_exact(&[0x56, 0xCB]),
            Err(DecodeError::Truncated { word: 0x56CB, .. })
        ));
        // ...and with a displacement attached it decodes as DBcc, not Scc.
        let dbne = dec(&[0x56, 0xCB, 0xFF, 0xFE]);
        assert_eq!(dbne.mnemonic, Mnemonic::Dbcc(Cond::Ne));
        // `pea d0` once emitted 4840 = swap d0 — decodes as SWAP.
        assert_eq!(dec(&[0x48, 0x40]).mnemonic, Mnemonic::Swap);
    }

    #[test]
    fn unknown_real_instructions_are_named_not_guessed() {
        // Real 68000 instructions still outside sigil's emitted set, plus the
        // two unassigned lines. Each must be a NAMED `Unknown`, never guessed
        // into a neighbouring family.
        //
        //   subx.w d1,d0 (9141), abcd d1,d0 (C101), sbcd d1,d0 (8101),
        //   nbcd d0 (4800), negx.w d0 (4040), chk.w d0,d1 (4380),
        //   link a0,#0 (4E50 0000), unlk a0 (4E58), stop #0 (4E72 0000),
        //   reset (4E70), trapv (4E76), rtr (4E77), line-A (A000), line-F (F000).
        for bytes in [
            &[0x91, 0x41][..],
            &[0xC1, 0x01],
            &[0x81, 0x01],
            &[0x48, 0x00],
            &[0x40, 0x40],
            &[0x43, 0x80],
            &[0x4E, 0x50, 0x00, 0x00],
            &[0x4E, 0x58],
            &[0x4E, 0x72, 0x00, 0x00],
            &[0x4E, 0x70],
            &[0x4E, 0x76],
            &[0x4E, 0x77],
            &[0xA0, 0x00],
            &[0xF0, 0x00],
        ] {
            assert!(
                matches!(decode_exact(bytes), Err(DecodeError::Unknown { .. })),
                "{bytes:02X?} must be Unknown"
            );
        }
    }

    /// The five instruction lines this parcel added. Each word is asl's own
    /// output for the snippet named beside it (`s1disasm/build_tools/
    /// Linux-x86_64/asl`, md5 `61e672562465725a8c102288a7da9098`), so this is a
    /// decode assertion against the assembler rather than against the encoder
    /// that produced it — the two halves are checked against one outside answer.
    #[test]
    fn newly_covered_lines_decode_to_their_instruction() {
        use Operand::*;
        // `exg.l d0,d1` = C1 41 — the Dx,Dy pair.
        assert_eq!(dec(&[0xC1, 0x41]), inst(Mnemonic::Exg, Size::L, vec![Dn(0), Dn(1)]));
        // `exg.l a0,a1` = C1 49 — the Ax,Ay pair.
        assert_eq!(dec(&[0xC1, 0x49]), inst(Mnemonic::Exg, Size::L, vec![An(0), An(1)]));
        // `exg.l d0,a0` = C1 88 — the mixed pair. asl spells `exg a0,d0` the
        // same way, which is what canonicalize's Rule E exists for.
        assert_eq!(dec(&[0xC1, 0x88]), inst(Mnemonic::Exg, Size::L, vec![Dn(0), An(0)]));
        // `roxl.w #1,d0` = E3 50 and `roxr.w #1,d0` = E2 50 — the tt=10 rows.
        assert_eq!(
            dec(&[0xE3, 0x50]),
            inst(Mnemonic::Roxl, Size::W, vec![Imm(1), Dn(0)])
        );
        assert_eq!(
            dec(&[0xE2, 0x50]),
            inst(Mnemonic::Roxr, Size::W, vec![Imm(1), Dn(0)])
        );
        // `roxl.w (a0)` = E5 D0 — the memory-shift form on the same row.
        assert_eq!(dec(&[0xE5, 0xD0]), inst(Mnemonic::Roxl, Size::W, vec![Ind(0)]));
        // `bchg #0,d0` = 08 40 00 00 (static) and `bchg d2,d0` = 05 40 (dynamic).
        assert_eq!(
            dec(&[0x08, 0x40, 0x00, 0x00]),
            inst(Mnemonic::Bchg, Size::L, vec![Imm(0), Dn(0)])
        );
        assert_eq!(dec(&[0x05, 0x40]), inst(Mnemonic::Bchg, Size::L, vec![Dn(2), Dn(0)]));
        // `move.w d6,ccr` = 44 C6.
        assert_eq!(
            dec(&[0x44, 0xC6]),
            inst(Mnemonic::MoveToCcr, Size::W, vec![Dn(6), Ccr])
        );
        // `move.l a6,usp` = 4E 66 and `move.l usp,a0` = 4E 68.
        assert_eq!(
            dec(&[0x4E, 0x66]),
            inst(Mnemonic::MoveToUsp, Size::L, vec![An(6), Usp])
        );
        assert_eq!(
            dec(&[0x4E, 0x68]),
            inst(Mnemonic::MoveFromUsp, Size::L, vec![Usp, An(0)])
        );
    }

    #[test]
    fn bad_extension_bits_are_loud() {
        // move.w (6,a1,d2.w),d0 with a 68020 scale bit set in the brief ext.
        assert!(matches!(
            decode_exact(&[0x30, 0x31, 0x22, 0x06]),
            Err(DecodeError::BadExtension { .. })
        ));
        // ori.b #imm,d0 with a nonzero immediate high byte.
        assert!(matches!(
            decode_exact(&[0x00, 0x00, 0x12, 0x01]),
            Err(DecodeError::BadExtension { .. })
        ));
    }

    #[test]
    fn trailing_bytes_fail_exact_decode() {
        // nop + one stray word is not one instruction — and the error is the
        // dedicated trailing-bytes shape, not a generic unknown.
        assert!(matches!(
            decode_exact(&[0x4E, 0x71, 0x00, 0x00]),
            Err(DecodeError::TrailingBytes { word: 0x4E71, used: 2, len: 4 })
        ));
    }

    /// Rule I registers: an out-of-range register on the encode side must FAIL
    /// the round trip, never be masked into a neighbour. (The encoder emits
    /// `Dn(9)` as `d1`; the check names the rewrite instead of blessing it.)
    #[test]
    fn out_of_range_register_fails_the_roundtrip() {
        let i = Instruction { mnemonic: Mnemonic::Move, size: Size::W, ops: vec![Dn(9), Dn(0)] };
        let bytes = encode(&i).expect("the encoder masks and encodes");
        let err = roundtrip_check(&i, &bytes).expect_err("Dn(9) must fail the round trip");
        assert!(err.contains("Dn(9)") && err.contains("out of range"), "message must name the register: {err}");
        // The same instruction with the register in range is fine.
        let ok = Instruction { mnemonic: Mnemonic::Move, size: Size::W, ops: vec![Dn(1), Dn(0)] };
        assert_roundtrip(&ok);
    }

    /// Rule I immediates: the relation proves FIELD fidelity, not value
    /// fidelity. `#$12345` does not fit a word, the encoder truncates it
    /// without a fit check (its documented operand contract), and both sides
    /// mask to the field width — so this round trip is GREEN by design. Value
    /// validation is the front-end's job; pinning the forgiveness here makes
    /// any future tightening a deliberate change to this test.
    #[test]
    fn oversized_width_stored_immediate_is_field_compared() {
        assert_roundtrip(&Instruction {
            mnemonic: Mnemonic::Cmpi,
            size: Size::W,
            ops: vec![Imm(0x12345), Dn(0)],
        });
    }

    #[test]
    fn short_branch_placeholder_is_slice_delimited() {
        // The backend's `bra.s` fixup placeholder: 2-byte slice, zero low byte.
        let i = dec(&[0x60, 0x00]);
        assert_eq!(
            i,
            Instruction { mnemonic: Mnemonic::Bra, size: Size::S, ops: vec![Disp(0)] }
        );
        // With a displacement word attached the same opcode is the WORD form.
        let w = dec(&[0x60, 0x00, 0x01, 0x00]);
        assert_eq!(w.size, Size::W);
        assert_eq!(w.ops, vec![Disp(0x100)]);
    }

    #[test]
    fn movem_predec_mask_reverses_back_to_canonical() {
        // movem.l d0-d7/a0-a6,-(sp) = 48E7 FFFE — canonical mask 0x7FFF.
        let store = dec(&[0x48, 0xE7, 0xFF, 0xFE]);
        assert_eq!(store.ops, vec![RegList(0x7FFF), PreDec(7)]);
        // movem.l (sp)+,d0-d7/a0-a6 = 4CDF 7FFF — mask unreversed.
        let load = dec(&[0x4C, 0xDF, 0x7F, 0xFF]);
        assert_eq!(load.ops, vec![PostInc(7), RegList(0x7FFF)]);
    }

    #[test]
    fn canonical_equivalences_hold_and_nothing_looser() {
        // Rule M: move.l d0,a0 and movea.l d0,a0 canonicalize identically.
        let mv = Instruction { mnemonic: Mnemonic::Move, size: Size::L, ops: vec![Dn(0), An(0)] };
        let mva = Instruction { mnemonic: Mnemonic::Movea, size: Size::L, ops: vec![Dn(0), An(0)] };
        assert_eq!(canonicalize(&mv), canonicalize(&mva));
        // Rule I: #-1 and #$FFFF are one word-immediate field.
        let neg = Instruction { mnemonic: Mnemonic::Cmpi, size: Size::W, ops: vec![Imm(-1), Dn(0)] };
        let pos = Instruction { mnemonic: Mnemonic::Cmpi, size: Size::W, ops: vec![Imm(0xFFFF), Dn(0)] };
        assert_eq!(canonicalize(&neg), canonicalize(&pos));
        // ...but a different register, EA mode, size, or family never collapses.
        let d1 = Instruction { mnemonic: Mnemonic::Cmpi, size: Size::W, ops: vec![Imm(-1), Dn(1)] };
        assert_ne!(canonicalize(&neg), canonicalize(&d1));
        let long = Instruction { mnemonic: Mnemonic::Cmpi, size: Size::L, ops: vec![Imm(-1), Dn(0)] };
        assert_ne!(canonicalize(&neg), canonicalize(&long));
        let ind = Instruction { mnemonic: Mnemonic::Cmpi, size: Size::W, ops: vec![Imm(-1), Ind(0)] };
        assert_ne!(canonicalize(&neg), canonicalize(&ind));
    }

    /// Forms the golden corpus does not cover but the encoder supports: prove
    /// the mirror reads them too.
    #[test]
    fn divide_and_memory_shift_roundtrip() {
        for i in [
            Instruction { mnemonic: Mnemonic::Divs, size: Size::W, ops: vec![Dn(1), Dn(0)] },
            Instruction { mnemonic: Mnemonic::Divu, size: Size::W, ops: vec![Ind(2), Dn(3)] },
            Instruction { mnemonic: Mnemonic::Asr, size: Size::W, ops: vec![Ind(0)] },
            Instruction { mnemonic: Mnemonic::Rol, size: Size::W, ops: vec![Disp16An(4, 3)] },
            Instruction { mnemonic: Mnemonic::Lsl, size: Size::B, ops: vec![Imm(8), Dn(5)] },
            Instruction { mnemonic: Mnemonic::Ror, size: Size::L, ops: vec![Dn(2), Dn(6)] },
        ] {
            assert_roundtrip(&i);
        }
    }
}
