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

// NOTE (for the AS front-end): the base ALU ops accept BOTH `<op> a,src` (two operands,
// first must be reg A) and `<op> src` (one operand); both lower to identical bytes. asl
// emits `or a`=B7 and `xor a`=AF (reg A's code is 7, not 0) — this is correct, not a bug.
/// `(register-form base opcode, immediate-form opcode)` for the eight base 8-bit
/// accumulator ALU operations; `None` for any other mnemonic.
///
/// Byte-verified against `tools/asl` (`add a,b`=80, `sub c`=91, `or a`=B7, `cp b`=B8, …).
fn alu8_opcodes(m: Mnemonic) -> Option<(u8, u8)> {
    Some(match m {
        Mnemonic::Add => (0x80, 0xC6),
        Mnemonic::Adc => (0x88, 0xCE),
        Mnemonic::Sub => (0x90, 0xD6),
        Mnemonic::Sbc => (0x98, 0xDE),
        Mnemonic::And => (0xA0, 0xE6),
        Mnemonic::Xor => (0xA8, 0xEE),
        Mnemonic::Or => (0xB0, 0xF6),
        Mnemonic::Cp => (0xB8, 0xFE),
        _ => return None,
    })
}

/// True when `op` is a source the base 8-bit ALU forms accept: a plain register or
/// `(hl)`. `(ix+d)`/`(iy+d)` sources are the DD/FD tasks' responsibility and are
/// deliberately excluded so their match arms are reached instead. (Extended to accept
/// `Imm8` in Step 3.5.)
fn is_alu8_src(op: &Operand) -> bool {
    matches!(op, Operand::Reg(_) | Operand::IndHl | Operand::Imm8(_))
}

/// Encode a base 8-bit accumulator ALU op (`<op> a,src` or `<op> src`) over a
/// register or `(hl)` source. (Immediate source added in Step 3.5.)
fn encode_alu8(m: Mnemonic, src: &Operand) -> Result<Vec<u8>, IsaError> {
    let (base, imm_op) = alu8_opcodes(m)
        .ok_or_else(|| IsaError::UnsupportedForm(format!("{m:?} is not a base 8-bit ALU op")))?;
    match src {
        Operand::Reg(r) => Ok(vec![base | reg8_code(*r)]),
        Operand::IndHl => Ok(vec![base | 0x06]),
        Operand::Imm8(n) => Ok(vec![imm_op, *n]),
        other => Err(IsaError::UnsupportedForm(format!("ALU source {other:?}"))),
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
        (Mnemonic::Ld, [Operand::Reg(dst), Operand::IndHl]) => {
            Ok(vec![0x46 | (reg8_code(*dst) << 3)])
        }
        (Mnemonic::Ld, [Operand::IndHl, Operand::Reg(src)]) => Ok(vec![0x70 | reg8_code(*src)]),
        (Mnemonic::Ld, [Operand::IndHl, Operand::Imm8(n)]) => Ok(vec![0x36, *n]),
        (Mnemonic::Ld, [Operand::IndDe, Operand::Reg(Reg8::A)]) => Ok(vec![0x12]),
        (Mnemonic::Ld, [Operand::Reg(Reg8::A), Operand::IndDe]) => Ok(vec![0x1A]),
        (Mnemonic::Ld, [Operand::IndBc, Operand::Reg(Reg8::A)]) => Ok(vec![0x02]),
        (Mnemonic::Ld, [Operand::Reg(Reg8::A), Operand::IndBc]) => Ok(vec![0x0A]),
        (Mnemonic::Ld, [Operand::Reg(Reg8::A), Operand::Mem(nn)]) => {
            let [lo, hi] = le16(*nn);
            Ok(vec![0x3A, lo, hi])
        }
        (Mnemonic::Ld, [Operand::Mem(nn), Operand::Reg(Reg8::A)]) => {
            let [lo, hi] = le16(*nn);
            Ok(vec![0x32, lo, hi])
        }
        // -- Task 3: base 8-bit ALU (accepts both `<op> a,src` and `<op> src`) --
        (m, [Operand::Reg(Reg8::A), src])
            if alu8_opcodes(m).is_some() && is_alu8_src(src) =>
        {
            encode_alu8(m, src)
        }
        (m, [src]) if alu8_opcodes(m).is_some() && is_alu8_src(src) => encode_alu8(m, src),
        // -- Task 3: base 8-bit inc/dec (r and (hl)) --------------------------
        (Mnemonic::Inc, [Operand::Reg(r)]) => Ok(vec![0x04 | (reg8_code(*r) << 3)]),
        (Mnemonic::Dec, [Operand::Reg(r)]) => Ok(vec![0x05 | (reg8_code(*r) << 3)]),
        (Mnemonic::Inc, [Operand::IndHl]) => Ok(vec![0x34]),
        (Mnemonic::Dec, [Operand::IndHl]) => Ok(vec![0x35]),
        // -- Task 3: end base group (insert new base arms above this line) -----
        // ---- Task 4: base group - 16-bit ops + control flow ----
        // (`nop` already encodes above via the migrated Plan-1 arm.)
        (Mnemonic::Ex, [Operand::Pair(Reg16::De), Operand::Pair(Reg16::Hl)]) => Ok(vec![0xEB]),
        (Mnemonic::Ex, [Operand::Pair(Reg16::Sp), Operand::Pair(Reg16::Hl)]) => Ok(vec![0xE3]),
        (Mnemonic::Ex, [Operand::Pair(Reg16::Af), Operand::AfShadow]) => Ok(vec![0x08]),
        (Mnemonic::Ld, [Operand::Pair(Reg16::Sp), Operand::Pair(Reg16::Hl)]) => Ok(vec![0xF9]),
        (Mnemonic::Jp, [Operand::IndHl]) => Ok(vec![0xE9]),
        (Mnemonic::Ld, [Operand::Pair(rr), Operand::Imm16(nn)])
            if matches!(rr, Reg16::Bc | Reg16::De | Reg16::Hl | Reg16::Sp) =>
        {
            let [lo, hi] = le16(*nn);
            Ok(vec![0x01 | (rp_code(*rr) << 4), lo, hi])
        }
        (Mnemonic::Ld, [Operand::Pair(Reg16::Hl), Operand::Mem(nn)]) => {
            let [lo, hi] = le16(*nn);
            Ok(vec![0x2A, lo, hi])
        }
        (Mnemonic::Ld, [Operand::Mem(nn), Operand::Pair(Reg16::Hl)]) => {
            let [lo, hi] = le16(*nn);
            Ok(vec![0x22, lo, hi])
        }
        (Mnemonic::Push, [Operand::Pair(rr)])
            if matches!(rr, Reg16::Bc | Reg16::De | Reg16::Hl | Reg16::Af) =>
        {
            Ok(vec![0xC5 | (push_pop_code(*rr) << 4)])
        }
        (Mnemonic::Pop, [Operand::Pair(rr)])
            if matches!(rr, Reg16::Bc | Reg16::De | Reg16::Hl | Reg16::Af) =>
        {
            Ok(vec![0xC1 | (push_pop_code(*rr) << 4)])
        }
        (Mnemonic::Add, [Operand::Pair(Reg16::Hl), Operand::Pair(rr)])
            if matches!(rr, Reg16::Bc | Reg16::De | Reg16::Hl | Reg16::Sp) =>
        {
            Ok(vec![0x09 | (rp_code(*rr) << 4)])
        }
        (Mnemonic::Inc, [Operand::Pair(rr)])
            if matches!(rr, Reg16::Bc | Reg16::De | Reg16::Hl | Reg16::Sp) =>
        {
            Ok(vec![0x03 | (rp_code(*rr) << 4)])
        }
        (Mnemonic::Dec, [Operand::Pair(rr)])
            if matches!(rr, Reg16::Bc | Reg16::De | Reg16::Hl | Reg16::Sp) =>
        {
            Ok(vec![0x0B | (rp_code(*rr) << 4)])
        }
        (Mnemonic::Ret, []) => Ok(vec![0xC9]),
        (Mnemonic::Ret, [Operand::Cc(cc)]) => Ok(vec![0xC0 | (cond_code(*cc) << 3)]),
        // (`jp nn` already encodes above via the migrated Plan-1 arm.)
        (Mnemonic::Jp, [Operand::Cc(cc), Operand::Imm16(nn)]) => {
            let [lo, hi] = le16(*nn);
            Ok(vec![0xC2 | (cond_code(*cc) << 3), lo, hi])
        }
        (Mnemonic::Call, [Operand::Imm16(nn)]) => {
            let [lo, hi] = le16(*nn);
            Ok(vec![0xCD, lo, hi])
        }
        (Mnemonic::Call, [Operand::Cc(cc), Operand::Imm16(nn)]) => {
            let [lo, hi] = le16(*nn);
            Ok(vec![0xC4 | (cond_code(*cc) << 3), lo, hi])
        }
        (Mnemonic::Exx, []) => Ok(vec![0xD9]),
        (Mnemonic::Rrca, []) => Ok(vec![0x0F]),
        (Mnemonic::Scf, []) => Ok(vec![0x37]),
        (Mnemonic::Ei, []) => Ok(vec![0xFB]),
        (Mnemonic::Di, []) => Ok(vec![0xF3]),
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
    fn ind_hl_forms_encode_in_base_group() {
        // The base group (Task 3) now encodes the (HL) load/ALU forms, each byte
        // verified against tools/asl: ld a,(hl)=7E, ld (hl),b=70, add a,(hl)=86.
        // (These superseded the Task-1 "for now unsupported" placeholder.)
        assert_eq!(encode(&ld(Operand::Reg(Reg8::A), Operand::IndHl)).unwrap(), vec![0x7E]);
        assert_eq!(encode(&ld(Operand::IndHl, Operand::Reg(Reg8::B))).unwrap(), vec![0x70]);
        assert_eq!(encode(&add_a(Operand::IndHl)).unwrap(), vec![0x86]);
        // `ld (hl),(hl)` (HALT, 0x76) is outside the driver ISA and still reports
        // UnsupportedForm — the (HL) operand itself is valid, the pairing is not.
        assert!(matches!(
            encode(&ld(Operand::IndHl, Operand::IndHl)),
            Err(IsaError::UnsupportedForm(_))
        ));
    }
}
