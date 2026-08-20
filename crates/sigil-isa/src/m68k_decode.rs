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
        return Err(unknown(
            u16::from_be_bytes([bytes[0], bytes[1]]),
            format!("instruction uses {used} of {} bytes — trailing bytes are not one instruction", bytes.len()),
        ));
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

/// `tt` field (bits 7–6) → bit-op mnemonic + its destination class. `bchg`
/// (`01`) is a real 68000 instruction sigil never emits.
fn bit_op(w: u16, tt: u16) -> Result<(Mnemonic, EaSet), DecodeError> {
    match tt {
        0b00 => Ok((Mnemonic::Btst, EaSet::DATA)),
        0b10 => Ok((Mnemonic::Bclr, EaSet::DATA_ALTERABLE)),
        0b11 => Ok((Mnemonic::Bset, EaSet::DATA_ALTERABLE)),
        _ => Err(unknown(w, "bchg is not in sigil's emitted set")),
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
    match w & 0xFFC0 {
        0x40C0 => {
            let dst = ea(rd, mode, r0, Size::W, EaSet::DATA_ALTERABLE)?;
            return Ok(inst(Mnemonic::MoveFromSr, Size::W, vec![Operand::Sr, dst]));
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
            _ => (Mnemonic::Tst, EaSet::DATA),
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
            0b000 | 0b001 => Err(unknown(w, "abcd/exg are not in sigil's emitted set")),
            _ => {
                let dst = ea(rd, mode, r0, size, EaSet::MEMORY_ALTERABLE)?;
                Ok(inst(Mnemonic::And, size, vec![Operand::Dn(reg9 as u8), dst]))
            }
        },
    }
}

/// Line 1110: shifts/rotates — the word memory-shift form (bits 7–6 = 11) and
/// the register form. The `rox` pair (`tt`=10) and the 68020 bitfield region
/// (memory form with bit 11 set) are outside sigil's emitted set.
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

fn shift_mnemonic(tt: u16, d: u16, w: u16) -> Result<Mnemonic, DecodeError> {
    Ok(match (tt, d) {
        (0b00, 0) => Mnemonic::Asr,
        (0b00, _) => Mnemonic::Asl,
        (0b01, 0) => Mnemonic::Lsr,
        (0b01, _) => Mnemonic::Lsl,
        (0b11, 0) => Mnemonic::Ror,
        (0b11, _) => Mnemonic::Rol,
        _ => return Err(unknown(w, "roxl/roxr are not in sigil's emitted set")),
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
        Btst | Bset | Bclr => Some(match ops.last() {
            Some(Operand::Dn(_)) => Size::L,
            _ => Size::B,
        }),
        Tas | Scc(_) | AndiCcr | OriCcr => Some(Size::B),
        MoveToSr | MoveFromSr => Some(Size::W),
        Jmp | Jsr | Lea | Pea => Some(Size::L),
        Nop | Rts | Rte | Trap | Swap | Illegal => Some(Size::W),
        Dbcc(_) => Some(Size::W),
        _ => None,
    }
}

/// Reduce an `Instruction` to the normal form the round-trip equality compares.
/// This IS the equivalence relation — see [`roundtrip_check`] for the rules and
/// why each is safe.
pub fn canonicalize(i: &Instruction) -> Instruction {
    let mut mnemonic = i.mnemonic;
    let mut size = i.size;
    let mut ops = i.ops.clone();

    // Register numbers occupy 3-bit fields; the encoder masks, so the normal
    // form does too (out-of-range registers are a front-end contract breach,
    // not an encoding distinction).
    for op in &mut ops {
        mask_regs(op);
    }

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
    // Rule S: size-less encodings get their canonical size.
    if let Some(s) = canonical_size(mnemonic, &ops) {
        size = s;
    }
    // Rule I: immediates stored at the operation width compare modulo that
    // width (the encoder truncates to the field; the decoder zero-extends).
    canonicalize_imms(mnemonic, size, &mut ops);

    Instruction { mnemonic, size, ops }
}

fn mask_regs(op: &mut Operand) {
    match op {
        Operand::Dn(n)
        | Operand::An(n)
        | Operand::Ind(n)
        | Operand::PostInc(n)
        | Operand::PreDec(n) => *n &= 0b111,
        Operand::Disp16An(_, n) => *n &= 0b111,
        Operand::Disp8AnXn { an, xn, .. } => {
            *an &= 0b111;
            mask_xn(xn);
        }
        Operand::Pcd8Xn { xn, .. } => mask_xn(xn),
        _ => {}
    }
}

fn mask_xn(xn: &mut Xn) {
    match xn {
        Xn::D(n) | Xn::A(n) => *n &= 0b111,
    }
}

/// Mask each immediate that the encoding stores at the operation width. The
/// width-independent immediates (quick data, shift counts, trap vectors, bit
/// numbers) are hard-range-checked by the encoder and pass through unchanged.
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
    if matches!(m, Btst | Bset | Bclr) {
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
/// - **Rule I** — an immediate stored at the operation width compares modulo
///   that width (`#-1` and `#$FFFF` are the same word-immediate field), and
///   3-bit register numbers compare masked. Everything else — EA mode AND
///   register of every operand, displacement values, masks, conditions, the
///   mnemonic family — must match exactly, which is precisely what catches a
///   wrong EA field or an aliased opcode word (the `D549` class: the bytes
///   decode as `Unknown`/a different family and the check fails loudly).
///
/// A decode failure is ALWAYS a check failure — an emitted instruction the
/// mirror cannot read is the loudest possible signal, never a skip.
pub fn roundtrip_check(inst: &Instruction, bytes: &[u8]) -> Result<(), String> {
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
        // subx.w d1,d0 (9141), abcd d1,d0 (C101), exg d0,d1 (C141),
        // roxl.w #1,d0 (E350), bchg #0,d0 (0840 0000), chk.w d0,d1 (4380),
        // line-A (A000), line-F (F000).
        for bytes in [
            &[0x91, 0x41][..],
            &[0xC1, 0x01],
            &[0xC1, 0x41],
            &[0xE3, 0x50],
            &[0x08, 0x40, 0x00, 0x00],
            &[0x43, 0x80],
            &[0xA0, 0x00],
            &[0xF0, 0x00],
        ] {
            assert!(
                matches!(decode_exact(bytes), Err(DecodeError::Unknown { .. })),
                "{bytes:02X?} must be Unknown"
            );
        }
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
        // nop + one stray word is not one instruction.
        assert!(decode_exact(&[0x4E, 0x71, 0x00, 0x00]).is_err());
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
