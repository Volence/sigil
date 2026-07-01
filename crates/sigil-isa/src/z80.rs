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

/// Map a CB-group register/`(hl)` target to its 3-bit code (0..=7).
/// `(HL)` is code 6; a plain register uses its `reg8_code`. Anything else is
/// not a legal CB target.
fn cb_target_code(op: &Operand) -> Result<u8, IsaError> {
    match op {
        Operand::Reg(r) => Ok(reg8_code(*r)),
        Operand::IndHl => Ok(6),
        other => Err(IsaError::UnsupportedForm(format!(
            "CB group target must be r or (hl), got {other:?}"
        ))),
    }
}

/// Encode a CB shift/rotate: `[0xCB, (family << 3) | target_code]`.
fn encode_cb_shift(family: u8, ops: &[Operand]) -> Result<Vec<u8>, IsaError> {
    match ops {
        [target] => {
            let code = cb_target_code(target)?;
            Ok(vec![0xCB, (family << 3) | code])
        }
        _ => Err(IsaError::UnsupportedForm(format!(
            "CB shift/rotate expects one operand (r or (hl)), got {ops:?}"
        ))),
    }
}

/// Encode a CB bit op (`bit`/`res`/`set`): `[0xCB, base | (bit << 3) | target_code]`.
/// `base` is 0x40 (bit), 0x80 (res) or 0xC0 (set). The bit number must be 0..=7.
fn encode_cb_bit(base: u8, ops: &[Operand]) -> Result<Vec<u8>, IsaError> {
    match ops {
        [Operand::Bit(bit), target] => {
            if *bit > 7 {
                return Err(IsaError::OperandRange(format!(
                    "bit number {bit} out of range 0..=7"
                )));
            }
            let code = cb_target_code(target)?;
            Ok(vec![0xCB, base | (bit << 3) | code])
        }
        _ => Err(IsaError::UnsupportedForm(format!(
            "CB bit op expects `b,r` or `b,(hl)`, got {ops:?}"
        ))),
    }
}

/// The Z80 `ED` instruction-group prefix byte.
const ED_PREFIX: u8 = 0xED;

/// ED sub-opcode for `ld (nn),rr` (store): `0x43 | (rp_code(rp) << 4)`.
/// Yields bc=`43`, de=`53`, hl=`63`, sp=`73`. NOTE: the `hl` value (`63`) is
/// never emitted by Sigil — AS uses the base `22` short form for `ld (nn),hl`
/// (catalog §2.3/§6.7). The caller's or-pattern excludes `hl`.
fn ed_ld_mem_pair_sub(rp: Reg16) -> u8 {
    0x43 | (rp_code(rp) << 4)
}

/// ED sub-opcode for `ld rr,(nn)` (load): `0x4B | (rp_code(rp) << 4)`.
/// Yields bc=`4B`, de=`5B`, hl=`6B`, sp=`7B`. The `hl` value (`6B`) is never
/// emitted — AS uses the base `2A` short form for `ld hl,(nn)` (catalog §6.7).
fn ed_ld_pair_mem_sub(rp: Reg16) -> u8 {
    0x4B | (rp_code(rp) << 4)
}

/// Map an IX/IY 16-bit pair to its index register; `None` for any non-index pair.
///
/// The index group shares the `[Pair, ..]` operand shapes with the base group
/// (`ld rr,nn`, `add hl,rr`, `push`/`pop`). Returning `None` for BC/DE/HL/SP/AF keeps
/// those forms with the base group.
fn as_index_reg(r: Reg16) -> Option<IndexReg> {
    match r {
        Reg16::Ix => Some(IndexReg::Ix),
        Reg16::Iy => Some(IndexReg::Iy),
        _ => None,
    }
}

/// True for the DD/FD (IX/IY) non-CB indexed forms this task owns: any operand is
/// `Operand::Indexed`, or a 16-bit operand names IX/IY (`ld ix,nn`, `add ix,rr`,
/// `push`/`pop ix`, `ld ix,(nn)`). The mnemonic is unused — the operand shape suffices —
/// but is accepted so the call site reads `is_index_form(m, ops)`. The DDCB/FDCB
/// `[Bit, Indexed]` forms are intercepted by Task 8's guard at the top of `encode`, so
/// they never reach this one.
fn is_index_form(_m: Mnemonic, ops: &[Operand]) -> bool {
    ops.iter().any(|op| {
        matches!(
            op,
            Operand::Indexed { .. } | Operand::Pair(Reg16::Ix) | Operand::Pair(Reg16::Iy)
        )
    })
}

/// Encode the DD/FD (IX/IY) non-CB indexed forms (catalog §2.4/§2.5).
///
/// Layout is always `index_prefix(reg)` then the HL-equivalent base opcode, with the
/// displacement byte at the position asl emits it (for `ld (ix+d),n` that is
/// `DD 36 <disp> <n>`). Only called for `is_index_form` inputs; any shape it does not own
/// (e.g. the illegal `add ix,hl`) is a hard `Err`. Each later step inserts one arm above
/// the sentinel.
fn encode_index(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    let ops = inst.ops.as_slice();
    match (inst.mnemonic, ops) {
        // ld ix,nn / ld iy,nn  ->  <pfx> 21 lo hi   (base `ld hl,nn`)
        (Mnemonic::Ld, [Operand::Pair(rr), Operand::Imm16(nn)])
            if as_index_reg(*rr).is_some() =>
        {
            let ix = as_index_reg(*rr).unwrap();
            let [lo, hi] = le16(*nn);
            Ok(vec![index_prefix(ix), 0x21, lo, hi])
        }
        // ld r,(ix+d) / ld r,(iy+d)  ->  <pfx> (0x46 | r<<3) disp   (base `ld r,(hl)`)
        (Mnemonic::Ld, [Operand::Reg(r), Operand::Indexed { reg, disp }]) => {
            Ok(vec![index_prefix(*reg), 0x46 | (reg8_code(*r) << 3), *disp as u8])
        }
        // ld (ix+d),r / ld (iy+d),r  ->  <pfx> (0x70 | r) disp   (base `ld (hl),r`)
        (Mnemonic::Ld, [Operand::Indexed { reg, disp }, Operand::Reg(r)]) => {
            Ok(vec![index_prefix(*reg), 0x70 | reg8_code(*r), *disp as u8])
        }
        // ld (ix+d),n / ld (iy+d),n  ->  <pfx> 36 disp n   (base `ld (hl),n`)
        (Mnemonic::Ld, [Operand::Indexed { reg, disp }, Operand::Imm8(n)]) => {
            Ok(vec![index_prefix(*reg), 0x36, *disp as u8, *n])
        }
        // -- Task 7: end index group (insert new index arms above this line) --
        _ => Err(IsaError::UnsupportedForm(format!(
            "unsupported Z80 index form: {inst:?}"
        ))),
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
        // ED-prefixed ld rr,(nn) for bc/de/sp. hl uses base 2A (Task 4).
        (Mnemonic::Ld, [Operand::Pair(rp @ (Reg16::Bc | Reg16::De | Reg16::Sp)), Operand::Mem(nn)]) => {
            let [lo, hi] = le16(*nn);
            Ok(vec![ED_PREFIX, ed_ld_pair_mem_sub(*rp), lo, hi])
        }
        // ld i,a = ED 47 ; ld r,a = ED 4F  (catalog §2.3, A0 idle stub)
        (Mnemonic::Ld, [Operand::RegI, Operand::Reg(Reg8::A)]) => Ok(vec![ED_PREFIX, 0x47]),
        (Mnemonic::Ld, [Operand::RegR, Operand::Reg(Reg8::A)]) => Ok(vec![ED_PREFIX, 0x4F]),
        (Mnemonic::Ld, [Operand::Mem(nn), Operand::Pair(Reg16::Hl)]) => {
            let [lo, hi] = le16(*nn);
            Ok(vec![0x22, lo, hi])
        }
        // ED-prefixed ld (nn),rr for bc/de/sp. hl uses base 22 (Task 4); the
        // or-pattern excludes hl so ED 63 is never emitted (catalog §6.7).
        (Mnemonic::Ld, [Operand::Mem(nn), Operand::Pair(rp @ (Reg16::Bc | Reg16::De | Reg16::Sp))]) => {
            let [lo, hi] = le16(*nn);
            Ok(vec![ED_PREFIX, ed_ld_mem_pair_sub(*rp), lo, hi])
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
        (Mnemonic::Jr, [Operand::Cc(cc), Operand::Rel(d)]) if cond_code(*cc) < 4 => {
            Ok(vec![0x20 | (cond_code(*cc) << 3), *d as u8])
        }
        (Mnemonic::Jr, [Operand::Cc(_), Operand::Rel(_)]) => Err(IsaError::OperandRange(
            "jr condition must be nz, z, nc, or c".into(),
        )),
        (Mnemonic::Jr, [Operand::Rel(d)]) => Ok(vec![0x18, *d as u8]),
        (Mnemonic::Djnz, [Operand::Rel(d)]) => Ok(vec![0x10, *d as u8]),
        (Mnemonic::Exx, []) => Ok(vec![0xD9]),
        (Mnemonic::Rrca, []) => Ok(vec![0x0F]),
        (Mnemonic::Scf, []) => Ok(vec![0x37]),
        (Mnemonic::Ei, []) => Ok(vec![0xFB]),
        (Mnemonic::Di, []) => Ok(vec![0xF3]),
        // ---- Task 5: CB group — shifts/rotates on r and (hl) ----
        (Mnemonic::Rlc, _) => encode_cb_shift(0, &inst.ops),
        (Mnemonic::Rrc, _) => encode_cb_shift(1, &inst.ops),
        (Mnemonic::Rl, _) => encode_cb_shift(2, &inst.ops),
        (Mnemonic::Rr, _) => encode_cb_shift(3, &inst.ops),
        (Mnemonic::Sla, _) => encode_cb_shift(4, &inst.ops),
        (Mnemonic::Sra, _) => encode_cb_shift(5, &inst.ops),
        (Mnemonic::Srl, _) => encode_cb_shift(7, &inst.ops),
        // ---- Task 5: CB group — bit/res/set on r and (hl) ----
        (Mnemonic::Bit, _) => encode_cb_bit(0x40, &inst.ops),
        (Mnemonic::Res, _) => encode_cb_bit(0x80, &inst.ops),
        (Mnemonic::Set, _) => encode_cb_bit(0xC0, &inst.ops),
        // ---- Task 6: ED group — bare one-offs ----
        (Mnemonic::Neg, _) => Ok(vec![ED_PREFIX, 0x44]),
        (Mnemonic::Ldir, _) => Ok(vec![ED_PREFIX, 0xB0]),
        // im 1 = ED 56 (only mode 1 is in catalog scope; other modes not oracled).
        (Mnemonic::Im, ops) => match ops {
            [Operand::Imm8(1)] => Ok(vec![ED_PREFIX, 0x56]),
            _ => Err(IsaError::UnsupportedForm(format!("im {:?}", inst.ops))),
        },
        // Task 7: IX/IY indexed forms (DD/FD prefix)
        (m, ops) if is_index_form(m, ops) => encode_index(inst),
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

    #[test]
    fn cb_shifts_rotates_on_r_and_hl() {
        // Golden bytes from tools/asl (asl 1.42), cpu z80 / phase 0.
        // rlc — family 0 (CB 00 base)
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Rlc, ops: vec![Operand::Reg(Reg8::B)] }).unwrap(), vec![0xCB, 0x00]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Rlc, ops: vec![Operand::Reg(Reg8::C)] }).unwrap(), vec![0xCB, 0x01]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Rlc, ops: vec![Operand::Reg(Reg8::A)] }).unwrap(), vec![0xCB, 0x07]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Rlc, ops: vec![Operand::IndHl] }).unwrap(), vec![0xCB, 0x06]);
        // rrc — family 1 (CB 08 base)
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Rrc, ops: vec![Operand::Reg(Reg8::D)] }).unwrap(), vec![0xCB, 0x0A]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Rrc, ops: vec![Operand::Reg(Reg8::A)] }).unwrap(), vec![0xCB, 0x0F]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Rrc, ops: vec![Operand::IndHl] }).unwrap(), vec![0xCB, 0x0E]);
        // rl — family 2 (CB 10 base)
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Rl, ops: vec![Operand::Reg(Reg8::E)] }).unwrap(), vec![0xCB, 0x13]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Rl, ops: vec![Operand::Reg(Reg8::L)] }).unwrap(), vec![0xCB, 0x15]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Rl, ops: vec![Operand::Reg(Reg8::A)] }).unwrap(), vec![0xCB, 0x17]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Rl, ops: vec![Operand::IndHl] }).unwrap(), vec![0xCB, 0x16]);
        // rr — family 3 (CB 18 base)
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Rr, ops: vec![Operand::Reg(Reg8::L)] }).unwrap(), vec![0xCB, 0x1D]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Rr, ops: vec![Operand::Reg(Reg8::A)] }).unwrap(), vec![0xCB, 0x1F]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Rr, ops: vec![Operand::IndHl] }).unwrap(), vec![0xCB, 0x1E]);
        // sla — family 4 (CB 20 base)
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Sla, ops: vec![Operand::Reg(Reg8::C)] }).unwrap(), vec![0xCB, 0x21]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Sla, ops: vec![Operand::Reg(Reg8::H)] }).unwrap(), vec![0xCB, 0x24]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Sla, ops: vec![Operand::Reg(Reg8::A)] }).unwrap(), vec![0xCB, 0x27]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Sla, ops: vec![Operand::IndHl] }).unwrap(), vec![0xCB, 0x26]);
        // sra — family 5 (CB 28 base)
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Sra, ops: vec![Operand::Reg(Reg8::E)] }).unwrap(), vec![0xCB, 0x2B]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Sra, ops: vec![Operand::Reg(Reg8::A)] }).unwrap(), vec![0xCB, 0x2F]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Sra, ops: vec![Operand::IndHl] }).unwrap(), vec![0xCB, 0x2E]);
        // srl — family 7 (CB 38 base; family 6 = undocumented sll, NOT implemented)
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Srl, ops: vec![Operand::Reg(Reg8::A)] }).unwrap(), vec![0xCB, 0x3F]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Srl, ops: vec![Operand::Reg(Reg8::H)] }).unwrap(), vec![0xCB, 0x3C]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Srl, ops: vec![Operand::Reg(Reg8::B)] }).unwrap(), vec![0xCB, 0x38]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Srl, ops: vec![Operand::IndHl] }).unwrap(), vec![0xCB, 0x3E]);
    }

    #[test]
    fn cb_bit_res_set_on_r_and_hl() {
        // Golden bytes from tools/asl. bit = base 0x40, res = 0x80, set = 0xC0.
        // op = base | (bit << 3) | target_code
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Bit, ops: vec![Operand::Bit(0), Operand::Reg(Reg8::B)] }).unwrap(), vec![0xCB, 0x40]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Bit, ops: vec![Operand::Bit(7), Operand::Reg(Reg8::D)] }).unwrap(), vec![0xCB, 0x7A]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Bit, ops: vec![Operand::Bit(4), Operand::Reg(Reg8::A)] }).unwrap(), vec![0xCB, 0x67]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Bit, ops: vec![Operand::Bit(3), Operand::Reg(Reg8::L)] }).unwrap(), vec![0xCB, 0x5D]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Bit, ops: vec![Operand::Bit(7), Operand::IndHl] }).unwrap(), vec![0xCB, 0x7E]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Bit, ops: vec![Operand::Bit(0), Operand::IndHl] }).unwrap(), vec![0xCB, 0x46]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Res, ops: vec![Operand::Bit(0), Operand::Reg(Reg8::A)] }).unwrap(), vec![0xCB, 0x87]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Res, ops: vec![Operand::Bit(5), Operand::Reg(Reg8::C)] }).unwrap(), vec![0xCB, 0xA9]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Res, ops: vec![Operand::Bit(7), Operand::IndHl] }).unwrap(), vec![0xCB, 0xBE]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Set, ops: vec![Operand::Bit(3), Operand::Reg(Reg8::B)] }).unwrap(), vec![0xCB, 0xD8]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Set, ops: vec![Operand::Bit(6), Operand::Reg(Reg8::A)] }).unwrap(), vec![0xCB, 0xF7]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Set, ops: vec![Operand::Bit(0), Operand::IndHl] }).unwrap(), vec![0xCB, 0xC6]);
        assert_eq!(encode(&Instruction { mnemonic: Mnemonic::Set, ops: vec![Operand::Bit(7), Operand::IndHl] }).unwrap(), vec![0xCB, 0xFE]);
    }

    #[test]
    fn cb_bit_number_out_of_range_errors() {
        assert_eq!(
            encode(&Instruction { mnemonic: Mnemonic::Bit, ops: vec![Operand::Bit(8), Operand::Reg(Reg8::A)] }),
            Err(IsaError::OperandRange("bit number 8 out of range 0..=7".into()))
        );
        assert!(matches!(
            encode(&Instruction { mnemonic: Mnemonic::Set, ops: vec![Operand::Bit(9), Operand::IndHl] }),
            Err(IsaError::OperandRange(_))
        ));
    }

    #[test]
    fn cb_rejects_unsupported_targets_and_shapes() {
        // (bc)/(de) and immediates are not legal CB targets.
        assert!(matches!(
            encode(&Instruction { mnemonic: Mnemonic::Rlc, ops: vec![Operand::IndBc] }),
            Err(IsaError::UnsupportedForm(_))
        ));
        assert!(matches!(
            encode(&Instruction { mnemonic: Mnemonic::Bit, ops: vec![Operand::Bit(0), Operand::Imm8(5)] }),
            Err(IsaError::UnsupportedForm(_))
        ));
        // wrong operand count.
        assert!(matches!(
            encode(&Instruction { mnemonic: Mnemonic::Srl, ops: vec![] }),
            Err(IsaError::UnsupportedForm(_))
        ));
        assert!(matches!(
            encode(&Instruction { mnemonic: Mnemonic::Res, ops: vec![Operand::Reg(Reg8::A)] }),
            Err(IsaError::UnsupportedForm(_))
        ));
    }

    #[test]
    fn encodes_ed_ld_mem_pair_stores() {
        // ld (1234h),bc = ED 43 34 12  (asl-verified)
        assert_eq!(
            encode(&Instruction {
                mnemonic: Mnemonic::Ld,
                ops: vec![Operand::Mem(0x1234), Operand::Pair(Reg16::Bc)],
            })
            .unwrap(),
            vec![0xED, 0x43, 0x34, 0x12]
        );
        // ld (1234h),de = ED 53 34 12  (asl-verified) — de MUST use ED 53, NOT base
        assert_eq!(
            encode(&Instruction {
                mnemonic: Mnemonic::Ld,
                ops: vec![Operand::Mem(0x1234), Operand::Pair(Reg16::De)],
            })
            .unwrap(),
            vec![0xED, 0x53, 0x34, 0x12]
        );
        // ld (1234h),sp = ED 73 34 12  (asl-verified)
        assert_eq!(
            encode(&Instruction {
                mnemonic: Mnemonic::Ld,
                ops: vec![Operand::Mem(0x1234), Operand::Pair(Reg16::Sp)],
            })
            .unwrap(),
            vec![0xED, 0x73, 0x34, 0x12]
        );
        // little-endian immediate re-confirmed with a distinct address:
        // ld (8DFCh),de = ED 53 FC 8D  (asl-verified)
        assert_eq!(
            encode(&Instruction {
                mnemonic: Mnemonic::Ld,
                ops: vec![Operand::Mem(0x8DFC), Operand::Pair(Reg16::De)],
            })
            .unwrap(),
            vec![0xED, 0x53, 0xFC, 0x8D]
        );
        // REGRESSION GUARD (byte-exact risk §6.7): hl store MUST stay base 22, not ED 63.
        // This arm is owned by Task 4; Task 6's ED path must not shadow it.
        assert_eq!(
            encode(&Instruction {
                mnemonic: Mnemonic::Ld,
                ops: vec![Operand::Mem(0x1234), Operand::Pair(Reg16::Hl)],
            })
            .unwrap(),
            vec![0x22, 0x34, 0x12]
        );
    }

    #[test]
    fn encodes_ed_ld_pair_mem_loads() {
        // ld bc,(1234h) = ED 4B 34 12  (asl-verified)
        assert_eq!(
            encode(&Instruction {
                mnemonic: Mnemonic::Ld,
                ops: vec![Operand::Pair(Reg16::Bc), Operand::Mem(0x1234)],
            })
            .unwrap(),
            vec![0xED, 0x4B, 0x34, 0x12]
        );
        // ld de,(1234h) = ED 5B 34 12  (asl-verified) — de MUST use ED 5B, NOT base
        assert_eq!(
            encode(&Instruction {
                mnemonic: Mnemonic::Ld,
                ops: vec![Operand::Pair(Reg16::De), Operand::Mem(0x1234)],
            })
            .unwrap(),
            vec![0xED, 0x5B, 0x34, 0x12]
        );
        // ld sp,(0C000h) = ED 7B 00 C0  (asl-verified, distinct address for LE)
        assert_eq!(
            encode(&Instruction {
                mnemonic: Mnemonic::Ld,
                ops: vec![Operand::Pair(Reg16::Sp), Operand::Mem(0xC000)],
            })
            .unwrap(),
            vec![0xED, 0x7B, 0x00, 0xC0]
        );
        // REGRESSION GUARD (§6.7): hl load MUST stay base 2A, not ED 6B (Task 4 arm).
        assert_eq!(
            encode(&Instruction {
                mnemonic: Mnemonic::Ld,
                ops: vec![Operand::Pair(Reg16::Hl), Operand::Mem(0x1234)],
            })
            .unwrap(),
            vec![0x2A, 0x34, 0x12]
        );
    }

    #[test]
    fn encodes_ed_neg_and_ldir() {
        // neg  = ED 44  (asl-verified)
        assert_eq!(
            encode(&Instruction { mnemonic: Mnemonic::Neg, ops: vec![] }).unwrap(),
            vec![0xED, 0x44]
        );
        // ldir = ED B0  (asl-verified)
        assert_eq!(
            encode(&Instruction { mnemonic: Mnemonic::Ldir, ops: vec![] }).unwrap(),
            vec![0xED, 0xB0]
        );
    }

    #[test]
    fn encodes_ed_im1() {
        // im 1 = ED 56  (asl-verified)
        assert_eq!(
            encode(&Instruction {
                mnemonic: Mnemonic::Im,
                ops: vec![Operand::Imm8(1)],
            })
            .unwrap(),
            vec![0xED, 0x56]
        );
        // only mode 1 is in catalog scope; other modes are unsupported for M0.
        assert!(matches!(
            encode(&Instruction {
                mnemonic: Mnemonic::Im,
                ops: vec![Operand::Imm8(2)],
            }),
            Err(IsaError::UnsupportedForm(_))
        ));
    }

    #[test]
    fn encodes_ed_ld_i_and_r() {
        // ld i,a = ED 47  (asl-verified)
        assert_eq!(
            encode(&Instruction {
                mnemonic: Mnemonic::Ld,
                ops: vec![Operand::RegI, Operand::Reg(Reg8::A)],
            })
            .unwrap(),
            vec![0xED, 0x47]
        );
        // ld r,a = ED 4F  (asl-verified)
        assert_eq!(
            encode(&Instruction {
                mnemonic: Mnemonic::Ld,
                ops: vec![Operand::RegR, Operand::Reg(Reg8::A)],
            })
            .unwrap(),
            vec![0xED, 0x4F]
        );
    }
}

#[cfg(test)]
mod index_tests {
    use super::*;

    /// Encode via the public `encode` entry point (the same path the asl vector
    /// oracle exercises), unwrapping the produced byte vector.
    fn enc(mnemonic: Mnemonic, ops: Vec<Operand>) -> Vec<u8> {
        encode(&Instruction { mnemonic, ops }).unwrap()
    }

    #[test]
    fn ld_ix_iy_nn() {
        // asl: `ld ix,1234h` = DD 21 34 12 ; `ld iy,1234h` = FD 21 34 12
        assert_eq!(
            enc(Mnemonic::Ld, vec![Operand::Pair(Reg16::Ix), Operand::Imm16(0x1234)]),
            vec![0xDD, 0x21, 0x34, 0x12]
        );
        assert_eq!(
            enc(Mnemonic::Ld, vec![Operand::Pair(Reg16::Iy), Operand::Imm16(0x1234)]),
            vec![0xFD, 0x21, 0x34, 0x12]
        );
    }

    #[test]
    fn ld_reg_indexed() {
        // asl: `ld a,(ix+3)` = DD 7E 03 ; `ld l,(ix+10)` = DD 6E 0A (10 == resolved
        // struct-field displacement) ; `ld a,(iy+5)` = FD 7E 05 ; `ld a,(ix-1)` = DD 7E FF
        assert_eq!(
            enc(Mnemonic::Ld, vec![Operand::Reg(Reg8::A),
                Operand::Indexed { reg: IndexReg::Ix, disp: 3 }]),
            vec![0xDD, 0x7E, 0x03]
        );
        assert_eq!(
            enc(Mnemonic::Ld, vec![Operand::Reg(Reg8::L),
                Operand::Indexed { reg: IndexReg::Ix, disp: 10 }]),
            vec![0xDD, 0x6E, 0x0A]
        );
        assert_eq!(
            enc(Mnemonic::Ld, vec![Operand::Reg(Reg8::A),
                Operand::Indexed { reg: IndexReg::Iy, disp: 5 }]),
            vec![0xFD, 0x7E, 0x05]
        );
        assert_eq!(
            enc(Mnemonic::Ld, vec![Operand::Reg(Reg8::A),
                Operand::Indexed { reg: IndexReg::Ix, disp: -1 }]),
            vec![0xDD, 0x7E, 0xFF]
        );
    }

    #[test]
    fn ld_indexed_reg() {
        // asl: `ld (ix+3),l` = DD 75 03 ; `ld (ix+10),b` = DD 70 0A ;
        // `ld (iy+7),a` = FD 77 07 ; `ld (ix-2),c` = DD 71 FE
        assert_eq!(
            enc(Mnemonic::Ld, vec![Operand::Indexed { reg: IndexReg::Ix, disp: 3 },
                Operand::Reg(Reg8::L)]),
            vec![0xDD, 0x75, 0x03]
        );
        assert_eq!(
            enc(Mnemonic::Ld, vec![Operand::Indexed { reg: IndexReg::Ix, disp: 10 },
                Operand::Reg(Reg8::B)]),
            vec![0xDD, 0x70, 0x0A]
        );
        assert_eq!(
            enc(Mnemonic::Ld, vec![Operand::Indexed { reg: IndexReg::Iy, disp: 7 },
                Operand::Reg(Reg8::A)]),
            vec![0xFD, 0x77, 0x07]
        );
        assert_eq!(
            enc(Mnemonic::Ld, vec![Operand::Indexed { reg: IndexReg::Ix, disp: -2 },
                Operand::Reg(Reg8::C)]),
            vec![0xDD, 0x71, 0xFE]
        );
    }

    #[test]
    fn ld_indexed_imm() {
        // asl: `ld (ix+3),0` = DD 36 03 00 ; `ld (iy+3),0` = FD 36 03 00
        // Order is prefix, opcode 36, displacement, THEN the immediate.
        assert_eq!(
            enc(Mnemonic::Ld, vec![Operand::Indexed { reg: IndexReg::Ix, disp: 3 },
                Operand::Imm8(0)]),
            vec![0xDD, 0x36, 0x03, 0x00]
        );
        assert_eq!(
            enc(Mnemonic::Ld, vec![Operand::Indexed { reg: IndexReg::Iy, disp: 3 },
                Operand::Imm8(0)]),
            vec![0xFD, 0x36, 0x03, 0x00]
        );
    }

    // --- more index-group tests appended below ---
}
