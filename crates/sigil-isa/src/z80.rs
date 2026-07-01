//! Z80 instruction encoder/decoder for the Sigil assembler.
//!
//! This module is a **dependency-free, extraction-ready** codec. It takes
//! RESOLVED integer operands (displacements, immediates, addresses); symbol
//! resolution belongs to the front-end, not here.
//!
//! Plan 2 rebuilds this from a five-variant enum into a table-driven encoder
//! covering the full driver ISA. Task 1 lands the canonical operand/instruction
//! model and migrates the five Plan-1 forms (`nop`; `ld r,r'`; `ld r,n`;
//! `add a,r`; `jp nn`) onto it; the remaining ~69 catalog forms are added by
//! later tasks. Full-ISA disassembly is DEFERRED — [`disassemble`] keeps only
//! the five migrated forms.

/// Pure 8-bit register (NO `(HL)` — that is an [`Operand`]).
///
/// The discriminant is the Z80 register-field code (B=0 … L=5, A=7). Code 6 is
/// reserved for `(HL)` and is intentionally **absent** here — it is
/// [`Operand::IndHl`]. This is the breaking API change flagged in Plan 1's
/// `Reg8` doc note, now realised.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Reg8 {
    B = 0,
    C = 1,
    D = 2,
    E = 3,
    H = 4,
    L = 5,
    A = 7,
}

/// 16-bit register pairs. The field encoding is context-dependent — see the
/// `*_code` helpers ([`rp_code`] vs [`push_pop_code`]) and [`index_prefix`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Reg16 {
    Bc,
    De,
    Hl,
    Sp,
    Af,
    Ix,
    Iy,
}

/// Condition codes. Discriminant = Z80 condition-field code.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Cond {
    Nz = 0,
    Z = 1,
    Nc = 2,
    C = 3,
    Po = 4,
    Pe = 5,
    P = 6,
    M = 7,
}

/// Index register for `(IX+d)` / `(IY+d)`.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum IndexReg {
    Ix,
    Iy,
}

/// A single operand. Immediates/displacements/addresses are already RESOLVED.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Operand {
    /// `b,c,d,e,h,l,a`
    Reg(Reg8),
    /// `bc,de,hl,sp,af,ix,iy`
    Pair(Reg16),
    /// `(hl)`
    IndHl,
    /// `(bc)`
    IndBc,
    /// `(de)`
    IndDe,
    /// `(ix+d)` / `(iy+d)`, displacement already resolved to -128..=127.
    Indexed { reg: IndexReg, disp: i8 },
    /// `n` — 8-bit immediate.
    Imm8(u8),
    /// `nn` — 16-bit immediate.
    Imm16(u16),
    /// `(nn)` — absolute memory address.
    Mem(u16),
    /// A condition code.
    Cc(Cond),
    /// A bit number 0..=7.
    Bit(u8),
    /// `jr`/`djnz` relative displacement (already resolved).
    Rel(i8),
    /// `af'` (only in `ex af,af'`).
    AfShadow,
    /// `i` (only in `ld i,a`).
    RegI,
    /// `r` (only in `ld r,a`).
    RegR,
}

/// The mnemonic set the driver uses.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Mnemonic {
    Nop,
    Ld,
    Add,
    Adc,
    Sub,
    Sbc,
    And,
    Or,
    Xor,
    Cp,
    Inc,
    Dec,
    Push,
    Pop,
    Ex,
    Exx,
    Ret,
    Jr,
    Jp,
    Call,
    Djnz,
    Rrca,
    Scf,
    Ei,
    Di,
    Bit,
    Res,
    Set,
    Srl,
    Rr,
    Sla,
    Rlc,
    Rrc,
    Rl,
    Sra,
    Neg,
    Im,
    Ldir,
    LdIA,
    LdRA,
}

/// A decoded instruction: mnemonic + 0..2 operands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Instruction {
    /// The instruction's mnemonic.
    pub mnemonic: Mnemonic,
    /// The operands, in source order (0..=2 for the driver ISA).
    pub ops: Vec<Operand>,
}

/// An error produced while encoding or decoding an [`Instruction`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IsaError {
    /// The `(mnemonic, operand-shape)` pair is not (yet) a supported form.
    UnsupportedForm(String),
    /// An operand value is outside the range the encoding allows.
    OperandRange(String),
}

impl std::fmt::Display for IsaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IsaError::UnsupportedForm(msg) => write!(f, "unsupported form: {msg}"),
            IsaError::OperandRange(msg) => write!(f, "operand out of range: {msg}"),
        }
    }
}

impl std::error::Error for IsaError {}

// ── Shared primitives ─────────────────────────────────────────────────────
// Later encoder-group tasks CALL these; they never redefine them. The four not
// yet used by the five migrated forms carry `#[allow(dead_code)]` until Task 2
// wires them in.

/// Z80 register-field code for a pure 8-bit register (B=0 … L=5, A=7).
fn reg8_code(r: Reg8) -> u8 {
    r as u8
}

/// Register-pair field for `ld rr,nn` / `add hl,rr` / `inc`/`dec rr` /
/// ED `(nn),rr`: BC=0, DE=1, HL=2, SP=3. IX/IY encode as HL's code (2) behind a
/// DD/FD prefix. `Af` shares slot 3 with SP and is never valid in these
/// contexts (use [`push_pop_code`] for `push`/`pop`).
#[allow(dead_code)] // consumed by later encoder-group tasks (16-bit forms)
fn rp_code(r: Reg16) -> u8 {
    match r {
        Reg16::Bc => 0,
        Reg16::De => 1,
        Reg16::Hl | Reg16::Ix | Reg16::Iy => 2,
        Reg16::Sp | Reg16::Af => 3,
    }
}

/// Register-pair field for `push`/`pop`: BC=0, DE=1, HL=2, AF=3. IX/IY encode as
/// HL's code (2) behind a DD/FD prefix. `Sp` shares slot 3 with AF and is never
/// valid in these contexts (use [`rp_code`] for 16-bit arithmetic/loads).
#[allow(dead_code)] // consumed by later encoder-group tasks (push/pop)
fn push_pop_code(r: Reg16) -> u8 {
    match r {
        Reg16::Bc => 0,
        Reg16::De => 1,
        Reg16::Hl | Reg16::Ix | Reg16::Iy => 2,
        Reg16::Af | Reg16::Sp => 3,
    }
}

/// Z80 condition-field code.
#[allow(dead_code)] // consumed by later encoder-group tasks (jr/jp/ret/call cc)
fn cond_code(c: Cond) -> u8 {
    c as u8
}

/// Prefix byte for an index register: IX => 0xDD, IY => 0xFD.
#[allow(dead_code)] // consumed by later encoder-group tasks (DD/FD groups)
fn index_prefix(r: IndexReg) -> u8 {
    match r {
        IndexReg::Ix => 0xDD,
        IndexReg::Iy => 0xFD,
    }
}

/// Little-endian split of a 16-bit value: `[lo, hi]`.
fn le16(v: u16) -> [u8; 2] {
    [v as u8, (v >> 8) as u8]
}

/// Map a Z80 register-field code (0..=7) to a pure [`Reg8`].
///
/// Code 6 denotes `(HL)`, which is NOT among Task 1's migrated forms, so it is
/// rejected here (full-ISA disassembly of `(HL)` forms is deferred).
fn reg8_from_code(code: u8) -> Result<Reg8, IsaError> {
    match code {
        0 => Ok(Reg8::B),
        1 => Ok(Reg8::C),
        2 => Ok(Reg8::D),
        3 => Ok(Reg8::E),
        4 => Ok(Reg8::H),
        5 => Ok(Reg8::L),
        7 => Ok(Reg8::A),
        _ => Err(IsaError::UnsupportedForm(
            "register code 6 ((HL)) is not in Task 1 disassembly coverage".into(),
        )),
    }
}

/// Encode a single [`Instruction`] into its Z80 machine-code bytes.
///
/// Dispatches on the mnemonic then the operand shape. Task 1 covers the five
/// migrated Plan-1 forms; every other shape returns [`IsaError::UnsupportedForm`]
/// until a later task adds it.
pub fn encode(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    use Operand::*;
    match (inst.mnemonic, inst.ops.as_slice()) {
        // nop
        (Mnemonic::Nop, []) => Ok(vec![0x00]),
        // ld r, r'
        (Mnemonic::Ld, [Reg(dst), Reg(src)]) => {
            Ok(vec![0x40 | (reg8_code(*dst) << 3) | reg8_code(*src)])
        }
        // ld r, n
        (Mnemonic::Ld, [Reg(dst), Imm8(n)]) => Ok(vec![0x06 | (reg8_code(*dst) << 3), *n]),
        // add a, r
        (Mnemonic::Add, [Reg(Reg8::A), Reg(src)]) => Ok(vec![0x80 | reg8_code(*src)]),
        // jp nn  (0xC3, lo, hi — little-endian)
        (Mnemonic::Jp, [Imm16(nn)]) => {
            let [lo, hi] = le16(*nn);
            Ok(vec![0xC3, lo, hi])
        }
        _ => Err(IsaError::UnsupportedForm(format!("{inst:?}"))),
    }
}

/// Decode the first instruction in `bytes`, returning it and the number of
/// bytes consumed. Inverse of [`encode`] over the five migrated forms only.
///
/// Full-ISA disassembly is DEFERRED: any opcode outside the migrated subset
/// (and register code 6 = `(HL)`) yields [`IsaError::UnsupportedForm`].
pub fn disassemble(bytes: &[u8]) -> Result<(Instruction, usize), IsaError> {
    let opcode = match bytes.first() {
        Some(&b) => b,
        None => return Err(IsaError::UnsupportedForm("empty input".into())),
    };
    match opcode {
        // nop
        0x00 => Ok((Instruction { mnemonic: Mnemonic::Nop, ops: vec![] }, 1)),
        // ld r, r'  (0x40..=0x7F; code 6 / HALT rejected via reg8_from_code)
        0x40..=0x7F => {
            let dst = reg8_from_code((opcode >> 3) & 0x07)?;
            let src = reg8_from_code(opcode & 0x07)?;
            Ok((
                Instruction {
                    mnemonic: Mnemonic::Ld,
                    ops: vec![Operand::Reg(dst), Operand::Reg(src)],
                },
                1,
            ))
        }
        // ld r, n  (0x06/0x0E/0x16/0x1E/0x26/0x2E/0x36/0x3E)
        _ if opcode & 0xC7 == 0x06 => {
            let dst = reg8_from_code((opcode >> 3) & 0x07)?;
            let imm = match bytes.get(1) {
                Some(&b) => b,
                None => return Err(IsaError::UnsupportedForm("truncated ld r, n".into())),
            };
            Ok((
                Instruction {
                    mnemonic: Mnemonic::Ld,
                    ops: vec![Operand::Reg(dst), Operand::Imm8(imm)],
                },
                2,
            ))
        }
        // add a, r  (0x80..=0x87)
        0x80..=0x87 => {
            let src = reg8_from_code(opcode & 0x07)?;
            Ok((
                Instruction {
                    mnemonic: Mnemonic::Add,
                    ops: vec![Operand::Reg(Reg8::A), Operand::Reg(src)],
                },
                1,
            ))
        }
        // jp nn  (0xC3, lo, hi — little-endian)
        0xC3 => {
            let lo = match bytes.get(1) {
                Some(&b) => b as u16,
                None => return Err(IsaError::UnsupportedForm("truncated jp nn".into())),
            };
            let hi = match bytes.get(2) {
                Some(&b) => b as u16,
                None => return Err(IsaError::UnsupportedForm("truncated jp nn".into())),
            };
            Ok((
                Instruction {
                    mnemonic: Mnemonic::Jp,
                    ops: vec![Operand::Imm16(lo | (hi << 8))],
                },
                3,
            ))
        }
        _ => Err(IsaError::UnsupportedForm(format!(
            "unknown opcode {opcode:#04X}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ld(dst: Operand, src: Operand) -> Instruction {
        Instruction { mnemonic: Mnemonic::Ld, ops: vec![dst, src] }
    }

    fn add_a(src: Operand) -> Instruction {
        Instruction { mnemonic: Mnemonic::Add, ops: vec![Operand::Reg(Reg8::A), src] }
    }

    #[test]
    fn primitive_codes_match_z80_fields() {
        assert_eq!(reg8_code(Reg8::B), 0);
        assert_eq!(reg8_code(Reg8::L), 5);
        assert_eq!(reg8_code(Reg8::A), 7);

        assert_eq!(rp_code(Reg16::Bc), 0);
        assert_eq!(rp_code(Reg16::De), 1);
        assert_eq!(rp_code(Reg16::Hl), 2);
        assert_eq!(rp_code(Reg16::Sp), 3);
        assert_eq!(rp_code(Reg16::Ix), 2);
        assert_eq!(rp_code(Reg16::Iy), 2);

        assert_eq!(push_pop_code(Reg16::Bc), 0);
        assert_eq!(push_pop_code(Reg16::De), 1);
        assert_eq!(push_pop_code(Reg16::Hl), 2);
        assert_eq!(push_pop_code(Reg16::Af), 3);

        assert_eq!(cond_code(Cond::Nz), 0);
        assert_eq!(cond_code(Cond::C), 3);
        assert_eq!(cond_code(Cond::M), 7);

        assert_eq!(index_prefix(IndexReg::Ix), 0xDD);
        assert_eq!(index_prefix(IndexReg::Iy), 0xFD);

        assert_eq!(le16(0x1234), [0x34, 0x12]);
        assert_eq!(le16(0x00FF), [0xFF, 0x00]);
    }

    #[test]
    fn encodes_migrated_forms() {
        assert_eq!(
            encode(&Instruction { mnemonic: Mnemonic::Nop, ops: vec![] }).unwrap(),
            vec![0x00]
        );
        assert_eq!(encode(&ld(Operand::Reg(Reg8::B), Operand::Reg(Reg8::C))).unwrap(), vec![0x41]);
        assert_eq!(encode(&ld(Operand::Reg(Reg8::A), Operand::Reg(Reg8::A))).unwrap(), vec![0x7F]);
        assert_eq!(encode(&ld(Operand::Reg(Reg8::A), Operand::Imm8(5))).unwrap(), vec![0x3E, 0x05]);
        assert_eq!(encode(&ld(Operand::Reg(Reg8::B), Operand::Imm8(10))).unwrap(), vec![0x06, 0x0A]);
        assert_eq!(encode(&add_a(Operand::Reg(Reg8::B))).unwrap(), vec![0x80]);
        assert_eq!(encode(&add_a(Operand::Reg(Reg8::A))).unwrap(), vec![0x87]);
        assert_eq!(
            encode(&Instruction { mnemonic: Mnemonic::Jp, ops: vec![Operand::Imm16(0x1234)] }).unwrap(),
            vec![0xC3, 0x34, 0x12]
        );
    }

    #[test]
    fn ind_hl_forms_are_unsupported_for_now() {
        // (HL) is a valid Operand, but ld/add with (HL) are not among Task 1's
        // migrated forms, so encode reports UnsupportedForm (NOT a bad register).
        assert!(matches!(
            encode(&ld(Operand::Reg(Reg8::A), Operand::IndHl)),
            Err(IsaError::UnsupportedForm(_))
        ));
        assert!(matches!(
            encode(&ld(Operand::IndHl, Operand::Reg(Reg8::B))),
            Err(IsaError::UnsupportedForm(_))
        ));
        assert!(matches!(
            encode(&add_a(Operand::IndHl)),
            Err(IsaError::UnsupportedForm(_))
        ));
    }
}
