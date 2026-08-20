//! `sigil-isa` 68000 encoder (procedural EA / extension-word machinery).
//!
//! M1.A scope: the full 68000 instruction/EA set the Aeon source (@ aeon `c7aaca6`)
//! uses — ~46 mnemonic families dispatched from [`encode`], each splicing the shared,
//! MOVE-proven `encode_ea`/`brief_ext` into a fixed-field opcode word. All 12 EA modes
//! appear, brief-extension indexed form only (no 68020 extensions). Proven byte-for-byte
//! against `asl` by the committed golden corpus (`tests/m68k_golden_vectors.txt`),
//! including the §5.5 hazards (MOVE dest-EA field swap, MOVEM `-(An)` mask reversal,
//! 2-wide branches, DBcc non-relaxability, MOVE SR/CCR). Emits big-endian bytes.
//!
//! # Operand contract
//!
//! The encoder is a leaf that assumes **fully-resolved, well-formed** operands from a
//! validating front-end (sub-project C) and linker (sub-project B): register numbers in
//! `0..=7`, immediates within their size's range, and branch/PC displacements already
//! resolved to their stored value (measured from the extension-word address, like the
//! `Pcd16` convention) and non-degenerate. Out-of-range register/immediate values are
//! masked/truncated to their field width rather than diagnosed — that validation is the
//! front-end's responsibility, not the encoder's. The exceptions that *do* hard-check
//! (because the signed truncation would be silently dangerous) are `moveq`/`addq`/`subq`
//! data and the branch/`DBcc` displacement fit, which return an `IsaError` on overflow.
//!
//! Width *selection* (`abs.w` vs `abs.l` for bare-symbol `jmp`/`jsr`, §5.6) and
//! decode/disassembly (the ISA-sharing dual facet) are deferred — the encoder takes the
//! explicit EA form it is given.

/// Instruction mnemonics — the full 68000 set the Aeon source uses. Every variant
/// dispatches to a real encoder in [`encode`]; the dispatch `match` is exhaustive over
/// `Mnemonic` (no `UnsupportedForm` catch-all).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mnemonic {
    Move, Movea,
    Add, Adda, Sub, Suba, And, Or, Eor, Cmp, Cmpa, Muls, Mulu, Divs, Divu,
    Addi, Subi, Andi, Ori, Eori, Cmpi,
    Moveq, Addq, Subq,
    Asl, Asr, Lsl, Lsr, Rol, Ror,
    Btst, Bset, Bclr,
    Clr, Neg, Not, Tst, Tas,
    Scc(Cond),
    Jmp, Jsr, Lea, Pea, Nop, Rts, Rte, Trap, Swap, Ext, Illegal,
    Bra, Bsr, Bcc(Cond), Dbcc(Cond),
    Movem, Movep, Addx, Cmpm,
    MoveToSr, MoveFromSr, // move.w <ea>,sr / move.w sr,<ea>
    AndiCcr, OriCcr,      // andi.b #imm,ccr / ori.b #imm,ccr
}

/// Does this mnemonic WRITE its last operand when that operand is a register —
/// the "destination is the last operand, `dN`/`aN`" form the front-end's
/// clobber/out lints (`[proc.clobber-undeclared]` / `[proc.out-unwritten]`) key
/// off? This is the ISA-DERIVED write model (S2-D6 U1): it replaces the
/// front-end's hand-maintained mnemonic string list with an EXHAUSTIVE match
/// over [`Mnemonic`]. Because there is no `_` arm, ADDING a variant to
/// `Mnemonic` fails THIS match to compile — so a newly-supported write-form can
/// no longer silently escape the lint (the load-bearing hole the string list
/// left; see the front-end `writes_dest_register` doc).
///
/// # Scope — the last-operand-register effect only
///
/// `true` marks the effect the front-end's `instr_written_regs` applies as "the
/// last operand, if it is a register, is written". The OTHER register-write
/// effects are operand-shape-driven and stay in the front-end (they need no
/// per-mnemonic table): auto-increment / -decrement base advance (`(An)+` /
/// `-(An)`, any mnemonic), the `dbcc` first-operand counter, and the `movem`
/// register-list destination. So `Movem`/`Dbcc` return `false` HERE (their
/// writes are not "the last operand as a register") yet are still fully modeled.
///
/// # The named escapees (U1)
///
/// - `Movep` → `true`: `movep.w (d16,An),Dn` (LOAD) writes `Dn`; `movep.w
///   Dn,(d16,An)` (STORE) writes memory, so the last-operand-register test
///   correctly finds no register there. (Both directions handled by construction.)
/// - `Addx` → `true`: `addx Dy,Dx` writes `Dx`; the `-(Ay),-(Ax)` form's writes
///   are the auto-dec bases, caught by the operand-shape effect.
/// - `Movem` → `false`: reglist-destination effect (see scope above).
///
/// Mnemonics NOT YET in this enum but named by the spec — `exg` (writes BOTH
/// register operands), `link`/`unlk` (write `An` + `sp`), `bchg`, `roxl`/`roxr`,
/// `negx`, `abcd`/`sbcd`/`nbcd` — are "covered by construction": when one is
/// added to `Mnemonic`, this match stops compiling and forces its classification
/// HERE. The bit/rotate/decimal forms (`bchg`/`roxl`/`roxr`/`negx`/`abcd`/…) are
/// last-operand-register writes (`true`); `exg` and `link`/`unlk` write registers
/// NOT expressible as "the last operand" and additionally need an operand-shape
/// arm in the front-end's `instr_written_regs` (the doc there records this).
/// **A `true` here is not a promise that the write COVERS the operand size.**
/// `Ext` writes only the half above the bits it reads, and `Tas`/`Bset`/`Bclr`
/// write a single bit. A consumer reasoning about how much of a register a write
/// establishes must classify those itself — `sigil_frontend_emp::out_verify`'s
/// `writes_partial_bits` and `ext_promotion` are that classification, and they are
/// plain string matches with no exhaustiveness link back to this enum. Adding a
/// mnemonic breaks the match below and forces a decision HERE; it will not force
/// one there, so a new partial-coverage form must be added there by hand.
pub fn writes_last_operand(m: Mnemonic) -> bool {
    use Mnemonic::*;
    match m {
        // Data movement / arithmetic / logic / shift-rotate / bit-set-clear /
        // set-cc — the destination is the last operand.
        Move | Movea | Moveq | Add | Adda | Sub | Suba | And | Or | Eor | Muls
        | Mulu | Divs | Divu | Addi | Subi | Andi | Ori | Eori | Addq | Subq
        | Asl | Asr | Lsl | Lsr | Rol | Ror | Bset | Bclr | Clr | Neg | Not
        | Tas | Scc(_) | Lea | Swap | Ext | Movep | Addx | MoveFromSr => true,
        // Read-only / control / flags-or-SR-only / operand-shape-modeled forms.
        // `Cmp`/`Cmpa`/`Cmpi`/`Cmpm`/`Btst`/`Tst` compare or test (no write);
        // `Jmp`/`Jsr`/`Pea`/`Nop`/`Rts`/`Rte`/`Trap`/`Illegal`/`Bra`/`Bsr`/`Bcc`
        // are control; `Dbcc`/`Movem` are operand-shape-modeled (see scope);
        // `MoveToSr`/`AndiCcr`/`OriCcr` write SR/CCR (not a GP register — the
        // dedicated `[proc.sr-undeclared]` branch handles the SR case).
        Cmp | Cmpa | Cmpi | Cmpm | Btst | Tst | Jmp | Jsr | Pea | Nop | Rts
        | Rte | Trap | Illegal | Bra | Bsr | Bcc(_) | Dbcc(_) | Movem
        | MoveToSr | AndiCcr | OriCcr => false,
    }
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

    /// The condition whose 4-bit cc field is `cc & 0xF` — [`cc`](Self::cc)'s inverse.
    pub fn from_cc(cc: u16) -> Cond {
        match cc & 0xF {
            0x0 => Cond::T, 0x1 => Cond::F, 0x2 => Cond::Hi, 0x3 => Cond::Ls,
            0x4 => Cond::Cc, 0x5 => Cond::Cs, 0x6 => Cond::Ne, 0x7 => Cond::Eq,
            0x8 => Cond::Vc, 0x9 => Cond::Vs, 0xA => Cond::Pl, 0xB => Cond::Mi,
            0xC => Cond::Ge, 0xD => Cond::Lt, 0xE => Cond::Gt, _ => Cond::Le,
        }
    }
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
    /// (d8,PC,Xn) brief-extension PC-relative indexed — EA mode 111, reg 011.
    Pcd8Xn { d: i8, xn: Xn, long: bool },
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
///
/// Every successful encode is offered to [`capture`] (a no-op unless a capture
/// session is active) so the round-trip self-check can see the full emitted
/// stream — see `m68k_decode::roundtrip_check` for the decode-side assert.
pub fn encode(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    let result = encode_dispatch(inst);
    if let Ok(bytes) = &result {
        capture::record(inst, bytes);
    }
    result
}

/// The per-family dispatch behind [`encode`].
fn encode_dispatch(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    match inst.mnemonic {
        Mnemonic::Move => encode_move(inst),
        Mnemonic::Movea => encode_movea(inst),
        Mnemonic::Add | Mnemonic::Sub | Mnemonic::And | Mnemonic::Or
        | Mnemonic::Cmp | Mnemonic::Eor
        | Mnemonic::Cmpa | Mnemonic::Adda | Mnemonic::Suba
        | Mnemonic::Muls | Mnemonic::Mulu
        | Mnemonic::Divs | Mnemonic::Divu => encode_alu_ea(inst),
        Mnemonic::Addi | Mnemonic::Subi | Mnemonic::Andi
        | Mnemonic::Ori | Mnemonic::Eori | Mnemonic::Cmpi => encode_alu_imm(inst),
        Mnemonic::Moveq | Mnemonic::Addq | Mnemonic::Subq => encode_quick(inst),
        Mnemonic::Asl | Mnemonic::Asr | Mnemonic::Lsl | Mnemonic::Lsr
        | Mnemonic::Rol | Mnemonic::Ror => encode_shift(inst),
        Mnemonic::Btst | Mnemonic::Bset | Mnemonic::Bclr => encode_bit(inst),
        Mnemonic::Clr | Mnemonic::Neg | Mnemonic::Not | Mnemonic::Tst
        | Mnemonic::Tas | Mnemonic::Scc(_) => encode_single_ea(inst),
        Mnemonic::AndiCcr | Mnemonic::OriCcr => encode_ccr_imm(inst),
        Mnemonic::MoveToSr | Mnemonic::MoveFromSr => encode_move_sr(inst),
        Mnemonic::Jmp | Mnemonic::Jsr | Mnemonic::Lea | Mnemonic::Pea
        | Mnemonic::Nop | Mnemonic::Rts | Mnemonic::Rte | Mnemonic::Trap
        | Mnemonic::Swap | Mnemonic::Ext | Mnemonic::Illegal => encode_control(inst),
        Mnemonic::Bra | Mnemonic::Bsr | Mnemonic::Bcc(_) => encode_branch(inst),
        Mnemonic::Dbcc(_) => encode_dbcc(inst),
        Mnemonic::Movem => encode_movem(inst),
        Mnemonic::Movep => encode_movep(inst),
        Mnemonic::Addx => encode_addx(inst),
        Mnemonic::Cmpm => encode_cmpm(inst),
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

/// Encode the ALU-EA family (`add/sub/and/or/cmp/eor/cmpa/adda/suba/muls/mulu`).
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

    // (base bits 15–12, register field, opmode, ea operand, EA modes the ISA permits there)
    let (base, reg, opmode, ea, allowed): (u16, u8, u16, &Operand, EaSet) = match inst.mnemonic {
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
            // Address arithmetic takes ANY source mode, An included.
            (base, an, opmode, src, EaSet::ALL)
        }
        // Word multiply: reg = Dn destination, source is the EA. muls opmode 111
        // (signed), mulu opmode 011 (unsigned) — a one-bit (bit 8) distinction.
        Mnemonic::Muls | Mnemonic::Mulu => {
            let signed = inst.mnemonic == Mnemonic::Muls;
            let name = if signed { "muls" } else { "mulu" };
            let dn = match dst {
                Operand::Dn(n) => n & 0b111,
                other => {
                    return Err(IsaError::UnsupportedForm(format!(
                        "{name} requires Dn destination, got {other:?}"
                    )))
                }
            };
            if inst.size != Size::W {
                return Err(IsaError::UnsupportedForm(format!("{name} is word only")));
            }
            (0b1100, dn, if signed { 0b111 } else { 0b011 }, src, EaSet::DATA)
        }
        // Word divide: reg = Dn destination (32-bit dividend / 16-bit EA divisor
        // → quotient:remainder in Dn). Same shape as muls but base 0b1000; divs
        // opmode 111 (signed), divu 011 (unsigned). Word only on the 68000.
        Mnemonic::Divs | Mnemonic::Divu => {
            let signed = inst.mnemonic == Mnemonic::Divs;
            let name = if signed { "divs" } else { "divu" };
            let dn = match dst {
                Operand::Dn(n) => n & 0b111,
                other => {
                    return Err(IsaError::UnsupportedForm(format!(
                        "{name} requires Dn destination, got {other:?}"
                    )))
                }
            };
            if inst.size != Size::W {
                return Err(IsaError::UnsupportedForm(format!("{name} is word only")));
            }
            (0b1000, dn, if signed { 0b111 } else { 0b011 }, src, EaSet::DATA)
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
            // `cmp.w a0,d0` is legal; `cmp.b a0,d0` is not, and the An byte rule catches it.
            (0b1011, dn, sz, src, EaSet::ALL)
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
            // DATA ALTERABLE: `eor.w d0,a1` otherwise encodes B149 = `cmpm.w (a1)+,(a0)+`.
            (0b1011, dn, sz + 0b100, dst, EaSet::DATA_ALTERABLE)
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
                // <ea>,Dn — reg is the Dn destination, opmode 0xx. (An SOURCE is a
                // valid EA here — `add.w a0,d1` is fine.)
                (ea, Operand::Dn(n)) => {
                    // ADD/SUB take any source mode (An for .w/.l); AND/OR are DATA
                    // only — `and.w a0,d0` has no encoding.
                    let allowed = match inst.mnemonic {
                        Mnemonic::Add | Mnemonic::Sub => EaSet::ALL,
                        _ => EaSet::DATA,
                    };
                    (base, n & 0b111, sz, ea, allowed)
                }
                // An DESTINATION has NO encoding in this family: the `Dn,<ea>`
                // direction's EA field cannot name an address register (mode 001 is
                // not an alterable-memory EA). Silently taking the `Dn,<ea>` arm
                // below produced `D549`-style words that the 68000 decodes as
                // `ADDX -(An),-(An)` — a memory-corrupting garbage opcode (the
                // effects-P2 raster/palette corruption, 2026-08-12). ADD/SUB have the
                // `adda`/`suba` address-arithmetic instructions for this; AND/OR have
                // none. Reject LOUD, naming the spelling — matching AS's stated intent
                // ("left to fail loud … never silently mis-encoded", sigil-frontend-as
                // eval.rs), which this backstops now that it actually fails.
                (Operand::Dn(_), Operand::An(_)) => {
                    let hint = match inst.mnemonic {
                        Mnemonic::Add => {
                            "write `adda` (address arithmetic; word/long only, no `.b`)"
                        }
                        Mnemonic::Sub => {
                            "write `suba` (address arithmetic; word/long only, no `.b`)"
                        }
                        // AND/OR: no ANDA/ORA exists — the operation is genuinely
                        // undefined against an address register.
                        _ => "no address-register-destination form exists for this operation",
                    };
                    return Err(IsaError::UnsupportedForm(format!(
                        "{:?}.{:?} with an address-register destination is not encodable — {hint}",
                        inst.mnemonic, inst.size
                    )));
                }
                // Dn,<ea> — reg is the Dn source, opmode 1xx (ea is memory: An was
                // rejected above, Dn dest was taken by the first arm).
                (Operand::Dn(n), ea) => (base, n & 0b111, sz + 0b100, ea, EaSet::MEMORY_ALTERABLE),
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

    let (ea_mode, ea_reg, ea_ext) = encode_ea(ea, allowed, inst.size)?;
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
    // DATA ALTERABLE — the whole family, cmpi included: `addi.w #1,a0` is not
    // an encoding (write `adda`), and there is no ALU-immediate form against An.
    let (ea_mode, ea_reg, ea_ext) = encode_ea(dst, EaSet::DATA_ALTERABLE, inst.size)?;
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
///
/// The SR forms are WORD-ONLY on the 68000 (asl rejects `.b`/`.l` too), and a
/// non-word size is worse than illegal here: the old encoder keyed the imm
/// ext-word width to `inst.size`, so `move.l #$2700, sr` emitted a LONG imm
/// the CPU reads as `sr := $0000` (interrupts fully UNMASKED — the opposite
/// of the intent) followed by `$2700` executing as an opcode (tranche-5
/// adversarial review F1). Police the size before any bytes are built.
fn encode_move_sr(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    if inst.size != Size::W {
        return Err(IsaError::UnsupportedForm(format!(
            "move to/from sr is word-only on the 68000, got size {:?}",
            inst.size
        )));
    }
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
    let allowed = match inst.mnemonic {
        Mnemonic::MoveToSr => EaSet::DATA,
        _ => EaSet::DATA_ALTERABLE,
    };
    let (ea_mode, ea_reg, ea_ext) = encode_ea(ea, allowed, inst.size)?;
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
    // ALTERABLE, not DATA ALTERABLE: addq/subq are the one ALU family with a
    // legal An destination (`addq.w #1,a0`). The `.b` case is illegal and the
    // An byte rule in `encode_ea` rejects it.
    let (ea_mode, ea_reg, ea_ext) = encode_ea(dst, EaSet::ALTERABLE, inst.size)?;
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
    let (d, tt): (u16, u16) = match inst.mnemonic {
        Mnemonic::Asr => (0, 0b00),
        Mnemonic::Asl => (1, 0b00),
        Mnemonic::Lsr => (0, 0b01),
        Mnemonic::Lsl => (1, 0b01),
        Mnemonic::Ror => (0, 0b11),
        Mnemonic::Rol => (1, 0b11),
        _ => unreachable!(),
    };
    // Single-operand MEMORY shift: `<shift>.w <ea>` shifts a memory WORD by one
    // (asl-verified: `asr.w (a0)` = E0D0, `lsl.w (a1)` = E3D1, `asr.w 4(a0)` =
    // E0E8 0004). Base word `1110 0tt d 11 eeeeee` = 0xE0C0 | tt<<9 | d<<8 | ea.
    // Word-size only (no .b/.l memory-shift form exists on the 68000).
    if let [dst] = inst.ops.as_slice() {
        if inst.size != Size::W {
            return Err(IsaError::UnsupportedForm(format!(
                "{:?} memory-shift form is word-only, got size {:?}",
                inst.mnemonic, inst.size
            )));
        }
        // MEMORY ALTERABLE: the one-operand form shifts a memory word. `asl.w d0`
        // is not it — the register form is the two-operand `asl.w #1,d0`.
        let (ea_mode, ea_reg, ea_ext) = encode_ea(dst, EaSet::MEMORY_ALTERABLE, Size::W)?;
        let ea: u16 = ((ea_mode as u16) << 3) | (ea_reg as u16);
        let word: u16 = 0xE0C0 | (tt << 9) | (d << 8) | ea;
        let mut out = word.to_be_bytes().to_vec();
        for w in ea_ext {
            out.extend_from_slice(&w.to_be_bytes());
        }
        return Ok(out);
    }
    let (src, dst) = match inst.ops.as_slice() {
        [s, d] => (s, d),
        _ => {
            return Err(IsaError::OperandCount(format!(
                "{:?} expects 1 or 2 operands, got {}",
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
    // `btst` only READS its destination, so it takes DATA; `bset`/`bclr` write and
    // take DATA ALTERABLE. Neither admits An — `bset d0,a1` otherwise encodes 01C9
    // = `movep.l d0,$8(a1)`, a MEMORY WRITE, and emits 2 bytes where MOVEP consumes
    // 4, so the displacement it swallows is the next instruction's opcode word.
    let allowed = match inst.mnemonic {
        Mnemonic::Btst => EaSet::DATA,
        _ => EaSet::DATA_ALTERABLE,
    };
    let (ea_mode, ea_reg, ea_ext) = encode_ea(dst, allowed, size)?;
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

/// Encode the single-EA family (`clr/neg/not/tst/tas/Scc`).
///
/// One operand supplies the `<ea>` in bits 5–0 (plus its extension words):
/// - `clr`=`0x4200`, `neg`=`0x4400`, `not`=`0x4600`, `tst`=`0x4A00`, each
///   `base | (size_code<<6) | ea`.
/// - `tas`=`0x4AC0 | ea` — byte-fixed opcode (bits 7–6 = `11` are opcode, no size field).
/// - `Scc(cond)`=`0x50C0 | (cond.cc()<<8) | ea` — byte-fixed, `cc` in bits 11–8.
///
/// For `tas`/`Scc` the operation is byte-size in the opcode itself, so `inst.size`
/// is ignored (asl confirms the fixed bytes).
fn encode_single_ea(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    let ea = match inst.ops.as_slice() {
        [op] => op,
        _ => {
            return Err(IsaError::OperandCount(format!(
                "{:?} expects 1 operand, got {}",
                inst.mnemonic,
                inst.ops.len()
            )))
        }
    };
    // `tst` only reads (DATA); `clr`/`neg`/`not`/`tas`/`Scc` write (DATA ALTERABLE).
    // Neither admits An on the 68000 — `sne a3` otherwise encodes 56CB = `dbne d3,…`,
    // a backward branch to garbage. (`tst.w a0` became legal only on the 68020.)
    let allowed = match inst.mnemonic {
        Mnemonic::Tst => EaSet::DATA,
        _ => EaSet::DATA_ALTERABLE,
    };
    let (ea_mode, ea_reg, ea_ext) = encode_ea(ea, allowed, inst.size)?;
    let ea_bits: u16 = ((ea_mode as u16) << 3) | (ea_reg as u16);
    let word: u16 = match inst.mnemonic {
        Mnemonic::Clr | Mnemonic::Neg | Mnemonic::Not | Mnemonic::Tst => {
            let base: u16 = match inst.mnemonic {
                Mnemonic::Clr => 0x4200,
                Mnemonic::Neg => 0x4400,
                Mnemonic::Not => 0x4600,
                Mnemonic::Tst => 0x4A00,
                _ => unreachable!(),
            };
            base | (size_code(inst.size)? << 6) | ea_bits
        }
        Mnemonic::Tas => 0x4AC0 | ea_bits,
        Mnemonic::Scc(cond) => 0x50C0 | (cond.cc() << 8) | ea_bits,
        _ => unreachable!(),
    };
    let mut out = Vec::with_capacity(2 + 2 * ea_ext.len());
    out.extend_from_slice(&word.to_be_bytes());
    for w in ea_ext {
        out.extend_from_slice(&w.to_be_bytes());
    }
    Ok(out)
}

/// Encode the control / misc family (`jmp/jsr/lea/pea/nop/rts/rte/trap/swap/ext`).
///
/// - EA-only forms: `jmp`=`0x4EC0 | ea`, `jsr`=`0x4E80 | ea`, `pea`=`0x4840 | ea`,
///   each followed by the operand's extension words.
/// - `lea <ea>,An`=`0x41C0 | (an<<9) | ea` (+ ea ext words); dest must be `An`.
/// - Fixed no-operand words: `nop`=`0x4E71`, `rts`=`0x4E75`, `rte`=`0x4E73`,
///   `illegal`=`0x4AFC` (the architecturally-reserved illegal-instruction word).
/// - `trap #n`=`0x4E40 | (n & 0xF)`; `n` must be an `Imm` in `0..=15`.
/// - `swap Dn`=`0x4840 | dn` (shares its base word with `pea`; dispatched by mnemonic).
/// - `ext.w Dn`=`0x4880 | dn`, `ext.l Dn`=`0x48C0 | dn`.
fn encode_control(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    // No-operand fixed words first.
    let fixed: Option<u16> = match inst.mnemonic {
        Mnemonic::Nop => Some(0x4E71),
        Mnemonic::Rts => Some(0x4E75),
        Mnemonic::Rte => Some(0x4E73),
        Mnemonic::Illegal => Some(0x4AFC),
        _ => None,
    };
    if let Some(word) = fixed {
        if !inst.ops.is_empty() {
            return Err(IsaError::OperandCount(format!(
                "{:?} expects 0 operands, got {}",
                inst.mnemonic,
                inst.ops.len()
            )));
        }
        return Ok(word.to_be_bytes().to_vec());
    }

    // `lea` is the only two-operand form.
    if inst.mnemonic == Mnemonic::Lea {
        let (src, an) = match inst.ops.as_slice() {
            [src, Operand::An(n)] => (src, (n & 0b111) as u16),
            [_, other] => {
                return Err(IsaError::UnsupportedForm(format!(
                    "lea requires An destination, got {other:?}"
                )))
            }
            _ => {
                return Err(IsaError::OperandCount(format!(
                    "lea expects 2 operands, got {}",
                    inst.ops.len()
                )))
            }
        };
        // CONTROL: `lea` names an address, so no register direct, no auto-inc/dec
        // side effect, no immediate (`lea (a0)+,a1` and `lea #$1000,a0` are not forms).
        let (ea_mode, ea_reg, ea_ext) = encode_ea(src, EaSet::CONTROL, inst.size)?;
        let word: u16 = 0x41C0 | (an << 9) | ((ea_mode as u16) << 3) | (ea_reg as u16);
        let mut out = Vec::with_capacity(2 + 2 * ea_ext.len());
        out.extend_from_slice(&word.to_be_bytes());
        for w in ea_ext {
            out.extend_from_slice(&w.to_be_bytes());
        }
        return Ok(out);
    }

    // Remaining forms take exactly one operand.
    let op = match inst.ops.as_slice() {
        [op] => op,
        _ => {
            return Err(IsaError::OperandCount(format!(
                "{:?} expects 1 operand, got {}",
                inst.mnemonic,
                inst.ops.len()
            )))
        }
    };

    match inst.mnemonic {
        // EA-only forms.
        Mnemonic::Jmp | Mnemonic::Jsr | Mnemonic::Pea => {
            let base: u16 = match inst.mnemonic {
                Mnemonic::Jmp => 0x4EC0,
                Mnemonic::Jsr => 0x4E80,
                Mnemonic::Pea => 0x4840,
                _ => unreachable!(),
            };
            // CONTROL, all three. `pea d0` otherwise encodes 4840 = `swap d0`:
            // nothing is pushed and the stack depth is wrong for the rest of the
            // routine. `pea` and `swap` genuinely share this base word.
            let (ea_mode, ea_reg, ea_ext) = encode_ea(op, EaSet::CONTROL, inst.size)?;
            let word: u16 = base | ((ea_mode as u16) << 3) | (ea_reg as u16);
            let mut out = Vec::with_capacity(2 + 2 * ea_ext.len());
            out.extend_from_slice(&word.to_be_bytes());
            for w in ea_ext {
                out.extend_from_slice(&w.to_be_bytes());
            }
            Ok(out)
        }
        Mnemonic::Trap => {
            let n = match op {
                Operand::Imm(v) if (0..=15).contains(v) => *v as u16,
                Operand::Imm(v) => {
                    return Err(IsaError::UnsupportedForm(format!(
                        "trap vector must be 0..=15, got {v}"
                    )))
                }
                other => {
                    return Err(IsaError::UnsupportedForm(format!(
                        "trap requires #n immediate, got {other:?}"
                    )))
                }
            };
            let word: u16 = 0x4E40 | (n & 0xF);
            Ok(word.to_be_bytes().to_vec())
        }
        Mnemonic::Swap => {
            let dn = match op {
                Operand::Dn(n) => (n & 0b111) as u16,
                other => {
                    return Err(IsaError::UnsupportedForm(format!(
                        "swap requires Dn operand, got {other:?}"
                    )))
                }
            };
            Ok((0x4840 | dn).to_be_bytes().to_vec())
        }
        Mnemonic::Ext => {
            let dn = match op {
                Operand::Dn(n) => (n & 0b111) as u16,
                other => {
                    return Err(IsaError::UnsupportedForm(format!(
                        "ext requires Dn operand, got {other:?}"
                    )))
                }
            };
            let base: u16 = match inst.size {
                Size::W => 0x4880,
                Size::L => 0x48C0,
                _ => {
                    return Err(IsaError::UnsupportedForm(
                        "ext is word (.w) or long (.l) only".into(),
                    ))
                }
            };
            Ok((base | dn).to_be_bytes().to_vec())
        }
        _ => unreachable!(),
    }
}

/// Encode the branch family (`bra`/`bsr`/`Bcc`), 2-wide only (§5.5 byte hazard).
///
/// Base word: `0110 cccc dddddddd` = `0x6000 | (cc<<8) | low_byte`.
/// - `bra` = cc `0000`, `bsr` = cc `0001`, conditional `Bcc(cond)` uses `cond.cc()`.
/// - `.s` (Size::S): the signed 8-bit displacement occupies the low byte (must fit `i8`).
/// - `.w` (Size::W): the low byte is `0x00` and a 16-bit displacement word follows
///   (must fit `i16`).
/// - No `.l`/`.b` form — `Size::L`/`Size::B` are rejected with `UnsupportedForm`.
///
/// The `Disp` operand carries the already-resolved displacement (as asl emits it,
/// measured from `instruction_address + 2`); the encoder emits it verbatim.
fn encode_branch(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    let disp = match inst.ops.as_slice() {
        [Operand::Disp(d)] => *d,
        _ => {
            return Err(IsaError::UnsupportedForm(format!(
                "{:?} requires a single resolved displacement operand, got {:?}",
                inst.mnemonic, inst.ops
            )))
        }
    };
    let cc: u16 = match inst.mnemonic {
        Mnemonic::Bra => 0x0,
        Mnemonic::Bsr => 0x1,
        Mnemonic::Bcc(cond) => cond.cc(),
        _ => unreachable!(),
    };
    let base = 0x6000 | (cc << 8);
    match inst.size {
        Size::S => {
            let d = i8::try_from(disp).map_err(|_| {
                IsaError::UnsupportedForm(format!(
                    "{:?}.s displacement {disp} does not fit a signed byte",
                    inst.mnemonic
                ))
            })?;
            let word = base | (d as u8 as u16);
            Ok(word.to_be_bytes().to_vec())
        }
        Size::W => {
            let d = i16::try_from(disp).map_err(|_| {
                IsaError::UnsupportedForm(format!(
                    "{:?}.w displacement {disp} does not fit a signed word",
                    inst.mnemonic
                ))
            })?;
            let mut out = Vec::with_capacity(4);
            out.extend_from_slice(&base.to_be_bytes());
            out.extend_from_slice(&d.to_be_bytes());
            Ok(out)
        }
        Size::L | Size::B => Err(IsaError::UnsupportedForm(format!(
            "{:?} is short (.s) or word (.w) only — 2-wide branch, no .l/.b",
            inst.mnemonic
        ))),
    }
}

/// Encode `DBcc Dn,disp` (`dbf`/`dbeq`/…), always 4 bytes — NON-relaxable (§5.5).
///
/// Base word: `0101 cccc 11001 rrr` = `0x50C8 | (cc<<8) | dn`, followed by a fixed
/// 16-bit displacement word (must fit `i16`). `dbf` = `Cond::F`, `dbeq` = `Cond::Eq`.
/// The `Disp` operand carries the already-resolved displacement, emitted verbatim.
fn encode_dbcc(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    let (dn, disp) = match inst.ops.as_slice() {
        [Operand::Dn(n), Operand::Disp(d)] => ((n & 0b111) as u16, *d),
        _ => {
            return Err(IsaError::UnsupportedForm(format!(
                "{:?} requires Dn,disp operands, got {:?}",
                inst.mnemonic, inst.ops
            )))
        }
    };
    let cc: u16 = match inst.mnemonic {
        Mnemonic::Dbcc(cond) => cond.cc(),
        _ => unreachable!(),
    };
    let d = i16::try_from(disp).map_err(|_| {
        IsaError::UnsupportedForm(format!(
            "{:?} displacement {disp} does not fit a signed word",
            inst.mnemonic
        ))
    })?;
    let word = 0x50C8 | (cc << 8) | dn;
    let mut out = Vec::with_capacity(4);
    out.extend_from_slice(&word.to_be_bytes());
    out.extend_from_slice(&d.to_be_bytes());
    Ok(out)
}

/// Encode `MOVEM` (register↔memory multi-move) — including the §5.5 predecrement
/// register-mask bit-order reversal, the single most byte-hazardous form.
///
/// Base word: `0100 1d00 1s eeeeee` = `0x4880 | (dir<<10) | (sz<<6) | ea`:
/// - `dir` (bit 10): register→memory STORE = 0, memory→register LOAD = 1.
/// - `sz` (bit 6): `.w` = 0, `.l` = 1.
/// - Base words: STORE `.w`=`0x4880`, STORE `.l`=`0x48C0`, LOAD `.w`=`0x4C80`, LOAD `.l`=`0x4CC0`.
///
/// Direction comes from operand ORDER: `[RegList, mem]` = STORE, `[mem, RegList]` = LOAD.
///
/// After the opcode word comes the register-mask word FIRST, THEN the memory EA's own
/// extension words (e.g. the `(d16,An)` displacement). `RegList(mask)` always holds the
/// mask in canonical order (bit0=D0..bit7=D7, bit8=A0..bit15=A7). For the `-(An)`
/// predecrement mode ONLY, the emitted mask word is the canonical mask with all 16 bits
/// reversed (`u16::reverse_bits`); every other addressing mode emits the canonical mask.
fn encode_movem(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    // Direction from operand order: [RegList, mem] = STORE, [mem, RegList] = LOAD.
    let (mask, mem, dir): (u16, &Operand, u16) = match inst.ops.as_slice() {
        [Operand::RegList(m), mem] => (*m, mem, 0), // regs -> memory (STORE)
        [mem, Operand::RegList(m)] => (*m, mem, 1), // memory -> regs (LOAD)
        _ => {
            return Err(IsaError::UnsupportedForm(format!(
                "movem requires [RegList,mem] or [mem,RegList] operands, got {:?}",
                inst.ops
            )))
        }
    };
    let sz: u16 = match inst.size {
        Size::W => 0,
        Size::L => 1,
        _ => {
            return Err(IsaError::UnsupportedForm(
                "movem is word (.w) or long (.l) only".into(),
            ))
        }
    };
    // MOVEM's legal modes differ BY DIRECTION, and the old code passed a
    // destination field for both — which let the illegal store `movem.l d0-d7,(a0)+`
    // through and falsely rejected the legal load `movem.l (Lbl,pc),d0-d7`.
    //   STORE (regs -> mem): control alterable, plus `-(An)`.
    //   LOAD  (mem -> regs): control, plus `(An)+`.
    let allowed = if dir == 0 {
        EaSet::CONTROL_ALTERABLE.or(EaSet::of(EaClass::PreDec))
    } else {
        EaSet::CONTROL.or(EaSet::of(EaClass::PostInc))
    };
    let (ea_mode, ea_reg, ea_ext) = encode_ea(mem, allowed, inst.size)?;
    let word: u16 = 0x4880 | (dir << 10) | (sz << 6) | ((ea_mode as u16) << 3) | (ea_reg as u16);

    // Predecrement `-(An)` reverses the canonical mask's 16 bits; all others emit it as-is.
    let mask_word = if matches!(mem, Operand::PreDec(_)) {
        mask.reverse_bits()
    } else {
        mask
    };

    let mut out = Vec::with_capacity(4 + 2 * ea_ext.len());
    out.extend_from_slice(&word.to_be_bytes());
    out.extend_from_slice(&mask_word.to_be_bytes());
    for w in ea_ext {
        out.extend_from_slice(&w.to_be_bytes());
    }
    Ok(out)
}

/// Encode `MOVEP Dn,(d16,An)` / `(d16,An),Dn` — the register↔alternate-byte-memory move.
///
/// Word: `0000 rrr ooo 001 aaa` = `(Dn<<9) | (opmode<<6) | 0b001_000 | An`, followed by a
/// trailing 16-bit displacement word. `opmode`: word mem→reg=`100`, long mem→reg=`101`,
/// word reg→mem=`110`, long reg→mem=`111`. Direction from operand order:
/// `[(d16,An), Dn]` = mem→reg, `[Dn, (d16,An)]` = reg→mem. The displacement is emitted
/// as its own trailing word (MOVEP has its own format — it is NOT routed through `encode_ea`).
fn encode_movep(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    // Direction from operand order.
    let (dn, an, disp, to_mem): (u8, u8, i16, bool) = match inst.ops.as_slice() {
        [Operand::Disp16An(d, a), Operand::Dn(n)] => (n & 0b111, a & 0b111, *d, false),
        [Operand::Dn(n), Operand::Disp16An(d, a)] => (n & 0b111, a & 0b111, *d, true),
        _ => {
            return Err(IsaError::UnsupportedForm(format!(
                "movep requires Dn,(d16,An) or (d16,An),Dn operands, got {:?}",
                inst.ops
            )))
        }
    };
    // opmode: bit8 (of the 3-bit field) = direction (mem→reg 0 / reg→mem 1), bit above = size.
    let long = match inst.size {
        Size::W => false,
        Size::L => true,
        _ => {
            return Err(IsaError::UnsupportedForm(
                "movep is word (.w) or long (.l) only".into(),
            ))
        }
    };
    let opmode: u16 = match (to_mem, long) {
        (false, false) => 0b100, // word mem -> reg
        (false, true) => 0b101,  // long mem -> reg
        (true, false) => 0b110,  // word reg -> mem
        (true, true) => 0b111,   // long reg -> mem
    };
    let word: u16 = ((dn as u16) << 9) | (opmode << 6) | 0b001_000 | (an as u16);
    let mut out = Vec::with_capacity(4);
    out.extend_from_slice(&word.to_be_bytes());
    out.extend_from_slice(&disp.to_be_bytes());
    Ok(out)
}

/// Encode `ADDX Dn,Dn` — extended add (register form only; the Aeon corpus uses no other).
///
/// Word: `1101 xxx 1 ss 00 0 yyy` = `0xD100 | (Rx<<9) | (ss<<6) | Ry`, where `Rx` is the
/// destination Dn (second operand), `Ry` the source Dn (first operand), `ss` = `size_code`.
/// The `-(An),-(An)` memory form is rejected with `UnsupportedForm`.
fn encode_addx(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    let (ry, rx) = match inst.ops.as_slice() {
        [Operand::Dn(s), Operand::Dn(d)] => (s & 0b111, d & 0b111),
        _ => {
            return Err(IsaError::UnsupportedForm(format!(
                "addx supports only the Dn,Dn form, got {:?}",
                inst.ops
            )))
        }
    };
    let ss = size_code(inst.size)?;
    let word: u16 = 0xD100 | ((rx as u16) << 9) | (ss << 6) | (ry as u16);
    Ok(word.to_be_bytes().to_vec())
}

/// Encode `CMPM (Ay)+,(Ax)+` — compare memory (postincrement form only).
///
/// Word: `1011 xxx 1 ss 001 yyy` = `0xB108 | (Ax<<9) | (ss<<6) | Ay`, where the FIRST
/// operand `(Ay)+` is the source and the SECOND `(Ax)+` is the destination; `ss` = `size_code`.
fn encode_cmpm(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    let (ay, ax) = match inst.ops.as_slice() {
        [Operand::PostInc(y), Operand::PostInc(x)] => (y & 0b111, x & 0b111),
        _ => {
            return Err(IsaError::UnsupportedForm(format!(
                "cmpm requires (Ay)+,(Ax)+ operands, got {:?}",
                inst.ops
            )))
        }
    };
    let ss = size_code(inst.size)?;
    let word: u16 = 0xB108 | ((ax as u16) << 9) | (ss << 6) | (ay as u16);
    Ok(word.to_be_bytes().to_vec())
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
    // `move` reads any mode and writes ALTERABLE — data alterable PLUS An.
    //
    // An destination is not an exception grudgingly allowed: MOVEA *is* MOVE with
    // destination mode 001. The two share one opcode layout (`00 ss rrr mmm nnn`),
    // so the bytes this encoder emits for `move.l Sym,a0` are byte-for-byte the
    // MOVEA the CPU executes. `movea` has its own encoder only because the corpus
    // also spells it explicitly. The illegal case is `move.b <ea>,aN` (there is no
    // MOVEA.B), and the An byte rule in `encode_ea` rejects exactly that.
    let (src_mode, src_reg, src_ext) = encode_ea(src, EaSet::ALL, inst.size)?;
    let (dst_mode, dst_reg, dst_ext) = encode_ea(dst, EaSet::ALTERABLE, inst.size)?;
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

/// Encode `MOVEA <ea>,An` — byte-identical to a `MOVE` whose destination EA is an
/// address register (dest mode `001`, dest reg = the An number). Same size-field
/// encoding as `MOVE` (`.w`=0b11, `.l`=0b10); MOVEA has **no `.b` form** and its
/// destination must be an `An`.
///
/// Word: `size_bits<<12 | (an<<9) | (0b001<<6) | (src_mode<<3) | src_reg`, followed by
/// the source EA's extension words.
fn encode_movea(inst: &Instruction) -> Result<Vec<u8>, IsaError> {
    let (src, an) = match inst.ops.as_slice() {
        [src, Operand::An(n)] => (src, (n & 0b111) as u16),
        [_, other] => {
            return Err(IsaError::IllegalDest(format!(
                "movea requires An destination, got {other:?}"
            )))
        }
        _ => {
            return Err(IsaError::OperandCount(format!(
                "movea expects 2 operands, got {}",
                inst.ops.len()
            )))
        }
    };
    // MOVEA shares MOVE's size field but has no byte form.
    let size_bits: u16 = match inst.size {
        Size::W => 0b11,
        Size::L => 0b10,
        Size::B | Size::S => {
            return Err(IsaError::UnsupportedForm(
                "movea is word (.w) or long (.l) only — no byte form".into(),
            ))
        }
    };
    let (src_mode, src_reg, src_ext) = encode_ea(src, EaSet::ALL, inst.size)?;
    let word: u16 = (size_bits << 12)
        | (an << 9)
        | (0b001 << 6)
        | ((src_mode as u16) << 3)
        | (src_reg as u16);
    let mut out = Vec::with_capacity(2 + 2 * src_ext.len());
    out.extend_from_slice(&word.to_be_bytes());
    for w in src_ext {
        out.extend_from_slice(&w.to_be_bytes());
    }
    Ok(out)
}

/// The twelve 68000 effective-address modes, as the ISA's own operand tables name
/// them. `encode_ea` resolves an `Operand` to one of these and then checks it
/// against the `EaSet` the *instruction* permits in that position.
///
/// This layer exists because resolving a mode and permitting a mode are different
/// questions, and sigil used to answer only the first. `encode_ea` mapped
/// `Operand::An` to mode `001` unconditionally, so four instructions whose opcode
/// space has a neighbour reachable by exactly that mode silently assembled into
/// the neighbour — see `alias_classes_are_rejected` for the decoded evidence.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EaClass {
    /// `Dn` — mode 000
    Dn,
    /// `An` — mode 001
    An,
    /// `(An)` — mode 010
    Ind,
    /// `(An)+` — mode 011
    PostInc,
    /// `-(An)` — mode 100
    PreDec,
    /// `(d16,An)` — mode 101
    Disp16An,
    /// `(d8,An,Xn)` — mode 110
    Disp8AnXn,
    /// `(xxx).W` — mode 111 reg 000
    AbsW,
    /// `(xxx).L` — mode 111 reg 001
    AbsL,
    /// `(d16,PC)` — mode 111 reg 010
    Pcd16,
    /// `(d8,PC,Xn)` — mode 111 reg 011
    Pcd8Xn,
    /// `#imm` — mode 111 reg 100
    Imm,
}

impl EaClass {
    const fn bit(self) -> u16 {
        1 << (self as u16)
    }
    /// How the operand spells in a diagnostic.
    pub(crate) fn spelling(self) -> &'static str {
        match self {
            EaClass::Dn => "Dn",
            EaClass::An => "An",
            EaClass::Ind => "(An)",
            EaClass::PostInc => "(An)+",
            EaClass::PreDec => "-(An)",
            EaClass::Disp16An => "(d16,An)",
            EaClass::Disp8AnXn => "(d8,An,Xn)",
            EaClass::AbsW => "(xxx).W",
            EaClass::AbsL => "(xxx).L",
            EaClass::Pcd16 => "(d16,PC)",
            EaClass::Pcd8Xn => "(d8,PC,Xn)",
            EaClass::Imm => "#imm",
        }
    }
}

/// A set of permitted `EaClass`es — the "addressing modes" column of the 68000
/// manual's per-instruction operand table, made machine-readable.
#[derive(Clone, Copy)]
pub struct EaSet(u16);

impl EaSet {
    pub(crate) const fn of(c: EaClass) -> Self {
        EaSet(c.bit())
    }
    pub(crate) const fn or(self, other: Self) -> Self {
        EaSet(self.0 | other.0)
    }
    const fn without(self, other: Self) -> Self {
        EaSet(self.0 & !other.0)
    }
    const fn and(self, other: Self) -> Self {
        EaSet(self.0 & other.0)
    }
    pub(crate) fn contains(self, c: EaClass) -> bool {
        self.0 & c.bit() != 0
    }

    // --- the named classes, defined exactly as the 68000 manual defines them ---

    /// Every mode. The correct set only where the instruction really accepts all
    /// twelve (`move` source, `movea` source, `cmpa`/`adda`/`suba` source).
    pub(crate) const ALL: Self = EaSet(0x0FFF);
    /// All but `An`. An address register is not a *data* operand.
    pub(crate) const DATA: Self = Self::ALL.without(Self::of(EaClass::An));
    /// All but `Dn` and `An` — the operand is in memory.
    pub(crate) const MEMORY: Self = Self::DATA.without(Self::of(EaClass::Dn));
    /// Modes that name a memory ADDRESS: no register direct, no autoincrement/
    /// decrement (their side effect has no meaning for an address), no immediate.
    ///
    /// `#imm` must be subtracted explicitly — the manual counts immediate among the
    /// MEMORY modes, so deriving control from memory without this line silently
    /// leaves `lea #$1000,a0` encodable.
    pub(crate) const CONTROL: Self = Self::MEMORY
        .without(Self::of(EaClass::PostInc))
        .without(Self::of(EaClass::PreDec))
        .without(Self::of(EaClass::Imm));
    /// Modes that can be WRITTEN: excludes the two PC-relative modes and `#imm`.
    pub(crate) const ALTERABLE: Self = Self::ALL
        .without(Self::of(EaClass::Pcd16))
        .without(Self::of(EaClass::Pcd8Xn))
        .without(Self::of(EaClass::Imm));
    /// `DATA & ALTERABLE` — the commonest destination class (`clr`, `Scc`, `eor`,
    /// `bset`, the ALU-immediate family, `move` destination).
    pub(crate) const DATA_ALTERABLE: Self = Self::DATA.and(Self::ALTERABLE);
    /// `MEMORY & ALTERABLE` — the memory-shift form and the `Dn,<ea>` ALU direction.
    pub(crate) const MEMORY_ALTERABLE: Self = Self::MEMORY.and(Self::ALTERABLE);
    /// `CONTROL & ALTERABLE` — MOVEM's store destination, before `-(An)` is added back.
    pub(crate) const CONTROL_ALTERABLE: Self = Self::CONTROL.and(Self::ALTERABLE);
}

/// Which of the twelve modes this operand is, or `None` for the pseudo-operands
/// that their family encoders consume directly and that are never a general EA.
pub(crate) fn ea_class(op: &Operand) -> Option<EaClass> {
    Some(match op {
        Operand::Dn(_) => EaClass::Dn,
        Operand::An(_) => EaClass::An,
        Operand::Ind(_) => EaClass::Ind,
        Operand::PostInc(_) => EaClass::PostInc,
        Operand::PreDec(_) => EaClass::PreDec,
        Operand::Disp16An(..) => EaClass::Disp16An,
        Operand::Disp8AnXn { .. } => EaClass::Disp8AnXn,
        Operand::AbsW(_) => EaClass::AbsW,
        Operand::AbsL(_) => EaClass::AbsL,
        Operand::Pcd16(_) => EaClass::Pcd16,
        Operand::Pcd8Xn { .. } => EaClass::Pcd8Xn,
        Operand::Imm(_) => EaClass::Imm,
        Operand::RegList(_) | Operand::Disp(_) | Operand::Ccr | Operand::Sr => return None,
    })
}

/// Resolve one EA to `(mode3, reg3, extension_words)`, rejecting any mode the
/// instruction does not permit in this position.
///
/// `allowed` is the instruction's own operand-table row, supplied by the family
/// encoder — the caller is the only place that knows which position this is and
/// what the ISA permits there. It replaces an earlier `Field::Source`/`Dest`
/// parameter, which could express only "not `#imm`/PC-relative" and therefore let
/// `An` through everywhere.
///
/// `size` supplies the `#imm` extension-word width, and also drives the ISA's one
/// size-dependent EA rule: an address register is never a BYTE operand.
fn encode_ea(op: &Operand, allowed: EaSet, size: Size) -> Result<(u8, u8, Vec<u16>), IsaError> {
    let Some(class) = ea_class(op) else {
        // Handled by their family encoders (movem/branch/ccr/sr), never as a general EA.
        let what = match op {
            Operand::RegList(_) => "register list",
            Operand::Disp(_) => "branch displacement",
            Operand::Ccr => "ccr",
            _ => "sr",
        };
        return Err(IsaError::UnsupportedForm(format!("{what} is not a general EA")));
    };

    if !allowed.contains(class) {
        // The three non-alterable modes keep the historical `IllegalDest` tag and
        // wording: for a write position that is precisely what they are, and
        // callers match on it.
        return Err(match class {
            EaClass::Imm | EaClass::Pcd16 | EaClass::Pcd8Xn if allowed.0 & EaSet::ALTERABLE.0 != 0 => {
                IsaError::IllegalDest(class.spelling().into())
            }
            _ => IsaError::UnsupportedForm(format!(
                "{} is not a legal addressing mode for this operand position",
                class.spelling()
            )),
        });
    }

    // An address register is a word/long operand only — there is no byte access
    // to An anywhere in the 68000 (`add.b a0,d0`, `cmp.b a0,d0`, `addq.b #1,a0`).
    if class == EaClass::An && size == Size::B {
        return Err(IsaError::UnsupportedForm(
            "an address register has no byte-size form".into(),
        ));
    }

    let r = |n: u8| n & 0b111;
    Ok(match *op {
        Operand::Dn(n) => (0b000, r(n), vec![]),
        Operand::An(n) => (0b001, r(n), vec![]),
        Operand::Ind(n) => (0b010, r(n), vec![]),
        Operand::PostInc(n) => (0b011, r(n), vec![]),
        Operand::PreDec(n) => (0b100, r(n), vec![]),
        Operand::Disp16An(d, n) => (0b101, r(n), vec![d as u16]),
        Operand::Disp8AnXn { d, an, xn, long } => (0b110, r(an), vec![brief_ext(d, xn, long)]),
        Operand::Pcd8Xn { d, xn, long } => (0b111, 0b011, vec![brief_ext(d, xn, long)]),
        Operand::AbsW(a) => (0b111, 0b000, vec![a as u16]),
        Operand::AbsL(a) => (0b111, 0b001, vec![(a >> 16) as u16, a as u16]),
        Operand::Pcd16(d) => (0b111, 0b010, vec![d as u16]),
        Operand::Imm(v) => {
            let ext = match size {
                Size::B => vec![(v as u16) & 0x00FF],
                Size::W => vec![v as u16],
                Size::L => vec![(v >> 16) as u16, v as u16],
                Size::S => return Err(IsaError::UnsupportedForm("#imm has no short (.s) size".into())),
            };
            (0b111, 0b100, ext)
        }
        Operand::RegList(_) | Operand::Disp(_) | Operand::Ccr | Operand::Sr => unreachable!("ea_class returned None"),
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

/// The mnemonic's FAMILY name — the label coverage accounting groups by. Each
/// mnemonic is its own family except the condition-parameterized trios, which
/// collapse to `"scc"`/`"bcc"`/`"dbcc"` (the condition is a field, not a family).
/// The match is exhaustive over [`Mnemonic`] with no `_` arm, so adding a variant
/// fails THIS function to compile and forces a row in [`ALL_FAMILY_NAMES`] — the
/// same forcing pattern as [`writes_last_operand`].
pub fn family_name(m: Mnemonic) -> &'static str {
    use Mnemonic::*;
    match m {
        Move => "move", Movea => "movea",
        Add => "add", Adda => "adda", Sub => "sub", Suba => "suba",
        And => "and", Or => "or", Eor => "eor",
        Cmp => "cmp", Cmpa => "cmpa",
        Muls => "muls", Mulu => "mulu", Divs => "divs", Divu => "divu",
        Addi => "addi", Subi => "subi", Andi => "andi", Ori => "ori",
        Eori => "eori", Cmpi => "cmpi",
        Moveq => "moveq", Addq => "addq", Subq => "subq",
        Asl => "asl", Asr => "asr", Lsl => "lsl", Lsr => "lsr",
        Rol => "rol", Ror => "ror",
        Btst => "btst", Bset => "bset", Bclr => "bclr",
        Clr => "clr", Neg => "neg", Not => "not", Tst => "tst", Tas => "tas",
        Scc(_) => "scc",
        Jmp => "jmp", Jsr => "jsr", Lea => "lea", Pea => "pea",
        Nop => "nop", Rts => "rts", Rte => "rte", Trap => "trap",
        Swap => "swap", Ext => "ext", Illegal => "illegal",
        Bra => "bra", Bsr => "bsr", Bcc(_) => "bcc", Dbcc(_) => "dbcc",
        Movem => "movem", Movep => "movep", Addx => "addx", Cmpm => "cmpm",
        MoveToSr => "move-to-sr", MoveFromSr => "move-from-sr",
        AndiCcr => "andi-ccr", OriCcr => "ori-ccr",
    }
}

/// Every family name [`family_name`] can return, exactly once each. A coverage
/// gate that wants "the whole encodable family set" derives it from HERE (which
/// the exhaustive match above keeps compiler-locked to the enum), never from a
/// measured run. `family_name_is_onto_all_family_names` proves the two agree.
pub const ALL_FAMILY_NAMES: &[&str] = &[
    "move", "movea",
    "add", "adda", "sub", "suba", "and", "or", "eor", "cmp", "cmpa",
    "muls", "mulu", "divs", "divu",
    "addi", "subi", "andi", "ori", "eori", "cmpi",
    "moveq", "addq", "subq",
    "asl", "asr", "lsl", "lsr", "rol", "ror",
    "btst", "bset", "bclr",
    "clr", "neg", "not", "tst", "tas", "scc",
    "jmp", "jsr", "lea", "pea", "nop", "rts", "rte", "trap", "swap", "ext",
    "illegal",
    "bra", "bsr", "bcc", "dbcc",
    "movem", "movep", "addx", "cmpm",
    "move-to-sr", "move-from-sr", "andi-ccr", "ori-ccr",
];

/// Encode-stream capture: a process-global tap on [`encode`] for the round-trip
/// self-check. While a [`CaptureSession`](capture::CaptureSession) is live,
/// every successful encode anywhere in the process records its
/// `(Instruction, bytes)` pair; a harness test wraps a whole-ROM build in a
/// session and round-trip-decodes everything the build encoded. The tap is
/// global (not thread-local) on purpose: the front-ends run lowering work on
/// spawned big-stack threads, and a thread-local sink would silently miss those
/// encodes — the exact silent-coverage-loss failure mode the round-trip pass
/// exists to prevent.
///
/// When no session is live the cost is one relaxed atomic load per encode.
pub mod capture {
    use super::Instruction;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, MutexGuard};

    static ENABLED: AtomicBool = AtomicBool::new(false);
    static PAIRS: Mutex<Vec<(Instruction, Vec<u8>)>> = Mutex::new(Vec::new());
    /// Serializes sessions so two concurrent tests cannot interleave streams.
    static SESSION: Mutex<()> = Mutex::new(());

    /// Exclusive capture session. Recording starts at [`begin`](CaptureSession::begin)
    /// and stops (with the buffer cleared) when the session drops.
    pub struct CaptureSession {
        _lock: MutexGuard<'static, ()>,
    }

    impl CaptureSession {
        /// Start capturing. Blocks until any other live session ends.
        ///
        /// Test infrastructure ONLY. While a session is live, EVERY successful
        /// encode in the process lands in its buffer — a session held across
        /// unrelated concurrent work collects that work's encodes as if the
        /// session's build emitted them, and any encode-heavy caller left
        /// running for the session's lifetime pays the per-encode lock. Keep a
        /// session scoped tightly around the one build it measures.
        pub fn begin() -> CaptureSession {
            let lock = SESSION.lock().unwrap_or_else(|e| e.into_inner());
            lock_pairs().clear();
            ENABLED.store(true, Ordering::SeqCst);
            CaptureSession { _lock: lock }
        }

        /// Take every pair recorded since the session began (or since the last
        /// `drain`), leaving the session recording.
        pub fn drain(&self) -> Vec<(Instruction, Vec<u8>)> {
            std::mem::take(&mut *lock_pairs())
        }
    }

    impl Drop for CaptureSession {
        fn drop(&mut self) {
            ENABLED.store(false, Ordering::SeqCst);
            lock_pairs().clear();
        }
    }

    fn lock_pairs() -> MutexGuard<'static, Vec<(Instruction, Vec<u8>)>> {
        PAIRS.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Record one successful encode. No-op unless a session is live.
    pub(super) fn record(inst: &Instruction, bytes: &[u8]) {
        if ENABLED.load(Ordering::Relaxed) {
            lock_pairs().push((inst.clone(), bytes.to_vec()));
        }
    }
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
        // movea now encodes (move-to-An): movea.l a1,a0 = 20 49.
        let movea = Instruction { mnemonic: Mnemonic::Movea, size: Size::L, ops: vec![Operand::An(1), Operand::An(0)] };
        assert_eq!(encode(&movea).unwrap(), vec![0x20, 0x49]);
        // Negative paths still covered: an illegal *operand shape* errors.
        // #imm destination for a MOVE is illegal.
        let bad_dst = Instruction { mnemonic: Mnemonic::Move, size: Size::W, ops: vec![Operand::Dn(0), Operand::Imm(1)] };
        assert!(matches!(encode(&bad_dst), Err(IsaError::IllegalDest(_))));
        // movea has no byte form.
        let movea_b = Instruction { mnemonic: Mnemonic::Movea, size: Size::B, ops: vec![Operand::Dn(0), Operand::An(1)] };
        assert!(matches!(encode(&movea_b), Err(IsaError::UnsupportedForm(_))));
        // movep/addx/cmpm now encode (specials family): movep.w (4,a1),d0 = 0109 0004.
        let movep = Instruction { mnemonic: Mnemonic::Movep, size: Size::W, ops: vec![Operand::Disp16An(4, 1), Operand::Dn(0)] };
        assert_eq!(encode(&movep).unwrap(), vec![0x01, 0x09, 0x00, 0x04]);
        // movem is now implemented (§5.5 predecrement mask reversal): store d0 to -(sp) = 48E7 8000.
        let movem = Instruction { mnemonic: Mnemonic::Movem, size: Size::L, ops: vec![Operand::RegList(0x0001), Operand::PreDec(7)] };
        assert_eq!(encode(&movem).unwrap(), vec![0x48, 0xE7, 0x80, 0x00]);
        // swap is now implemented (control/misc family): 0x4840 | dn.
        let swap = Instruction { mnemonic: Mnemonic::Swap, size: Size::W, ops: vec![Operand::Dn(0)] };
        assert_eq!(encode(&swap).unwrap(), vec![0x48, 0x40]);
        // Move still works.
        let mv = Instruction { mnemonic: Mnemonic::Move, size: Size::W, ops: vec![Operand::Dn(1), Operand::Dn(0)] };
        assert_eq!(encode(&mv).unwrap(), vec![0x30, 0x01]);
    }

    /// Address-register-destination hygiene for the ALU-EA family (effects-P2
    /// corruption fix, 2026-08-12). `add/sub/and/or dN,aM` must NOT silently encode
    /// as the `Dn,<ea>` form with An as the EA — that emitted `D549`-style words the
    /// 68000 decodes as `ADDX -(An),-(An)`, corrupting memory. ADD/SUB reject with an
    /// `adda`/`suba` hint; AND/OR reject (no address-register form). The CORRECT
    /// spellings still encode, and the `<ea>,Dn` direction with an An SOURCE is
    /// untouched.
    #[test]
    fn alu_ea_rejects_address_register_destination() {
        // --- garbage forms now fail loud (were silent D549-style ADDX) ---
        let add_da = Instruction { mnemonic: Mnemonic::Add, size: Size::W, ops: vec![Operand::Dn(2), Operand::An(1)] };
        assert!(matches!(encode(&add_da), Err(IsaError::UnsupportedForm(_))), "add.w d2,a1 must be rejected");
        let sub_da = Instruction { mnemonic: Mnemonic::Sub, size: Size::L, ops: vec![Operand::Dn(0), Operand::An(1)] };
        assert!(matches!(encode(&sub_da), Err(IsaError::UnsupportedForm(_))), "sub.l d0,a1 must be rejected");
        let and_da = Instruction { mnemonic: Mnemonic::And, size: Size::W, ops: vec![Operand::Dn(0), Operand::An(1)] };
        assert!(matches!(encode(&and_da), Err(IsaError::UnsupportedForm(_))), "and.w d0,a1 must be rejected (no ANDA)");
        let or_da = Instruction { mnemonic: Mnemonic::Or, size: Size::W, ops: vec![Operand::Dn(0), Operand::An(1)] };
        assert!(matches!(encode(&or_da), Err(IsaError::UnsupportedForm(_))), "or.w d0,a1 must be rejected (no ORA)");
        // The ADD hint names `adda`.
        match encode(&add_da) {
            Err(IsaError::UnsupportedForm(msg)) => assert!(msg.contains("adda"), "hint should name adda: {msg}"),
            other => panic!("expected UnsupportedForm, got {other:?}"),
        }

        // --- the CORRECT spellings encode (adda/suba/cmpa) ---
        let adda = Instruction { mnemonic: Mnemonic::Adda, size: Size::W, ops: vec![Operand::An(0), Operand::An(1)] };
        assert_eq!(encode(&adda).unwrap(), vec![0xD2, 0xC8], "adda.w a0,a1");
        let suba = Instruction { mnemonic: Mnemonic::Suba, size: Size::L, ops: vec![Operand::An(0), Operand::An(1)] };
        assert_eq!(encode(&suba).unwrap(), vec![0x93, 0xC8], "suba.l a0,a1");
        let cmpa = Instruction { mnemonic: Mnemonic::Cmpa, size: Size::L, ops: vec![Operand::An(0), Operand::An(1)] };
        assert_eq!(encode(&cmpa).unwrap(), vec![0xB3, 0xC8], "cmpa.l a0,a1");

        // --- adda/suba/cmpa are word/long only: byte form errors ---
        let adda_b = Instruction { mnemonic: Mnemonic::Adda, size: Size::B, ops: vec![Operand::An(0), Operand::An(1)] };
        assert!(matches!(encode(&adda_b), Err(IsaError::UnsupportedForm(_))), "adda.b has no byte form");
        let cmpa_b = Instruction { mnemonic: Mnemonic::Cmpa, size: Size::B, ops: vec![Operand::An(0), Operand::An(1)] };
        assert!(matches!(encode(&cmpa_b), Err(IsaError::UnsupportedForm(_))), "cmpa.b has no byte form");

        // --- regression: An SOURCE with a Dn destination is STILL valid (add.w a0,d1) ---
        let add_ad = Instruction { mnemonic: Mnemonic::Add, size: Size::W, ops: vec![Operand::An(0), Operand::Dn(1)] };
        assert_eq!(encode(&add_ad).unwrap(), vec![0xD2, 0x48], "add.w a0,d1 (An source) still encodes");
        // --- regression: Dn,<memory> still encodes (add.w d0,(a1)) ---
        let add_dm = Instruction { mnemonic: Mnemonic::Add, size: Size::W, ops: vec![Operand::Dn(0), Operand::Ind(1)] };
        assert_eq!(encode(&add_dm).unwrap(), vec![0xD1, 0x51], "add.w d0,(a1) still encodes");
        // --- plain Dn,Dn still encodes (add.w d0,d1) ---
        let add_dd = Instruction { mnemonic: Mnemonic::Add, size: Size::W, ops: vec![Operand::Dn(0), Operand::Dn(1)] };
        assert_eq!(encode(&add_dd).unwrap(), vec![0xD2, 0x40], "add.w d0,d1 still encodes");
    }

    /// [`family_name`] maps ONTO [`ALL_FAMILY_NAMES`]: every listed name is
    /// reachable from some mnemonic and no mnemonic maps outside the list, so a
    /// coverage gate deriving "all families" from the constant sees exactly the
    /// enum's families (the match in `family_name` is compiler-locked to the enum).
    #[test]
    fn family_name_is_onto_all_family_names() {
        use Mnemonic::*;
        let every: Vec<Mnemonic> = vec![
            Move, Movea, Add, Adda, Sub, Suba, And, Or, Eor, Cmp, Cmpa, Muls,
            Mulu, Divs, Divu, Addi, Subi, Andi, Ori, Eori, Cmpi, Moveq, Addq,
            Subq, Asl, Asr, Lsl, Lsr, Rol, Ror, Btst, Bset, Bclr, Clr, Neg,
            Not, Tst, Tas, Scc(Cond::Eq), Jmp, Jsr, Lea, Pea, Nop, Rts, Rte,
            Trap, Swap, Ext, Illegal, Bra, Bsr, Bcc(Cond::Eq), Dbcc(Cond::Eq),
            Movem, Movep, Addx, Cmpm, MoveToSr, MoveFromSr, AndiCcr, OriCcr,
        ];
        let reached: std::collections::BTreeSet<&str> =
            every.iter().map(|m| family_name(*m)).collect();
        let listed: std::collections::BTreeSet<&str> =
            ALL_FAMILY_NAMES.iter().copied().collect();
        assert_eq!(reached, listed, "family_name and ALL_FAMILY_NAMES disagree");
        assert_eq!(
            ALL_FAMILY_NAMES.len(),
            listed.len(),
            "ALL_FAMILY_NAMES carries a duplicate"
        );
    }

    /// S2-D6 U1 — the ISA-derived write model. Every write-form mnemonic the
    /// front-end clobber/out lints depend on is classified `true`; every
    /// read-only / control / operand-shape-modeled form is `false`. The match is
    /// exhaustive over `Mnemonic`, so this also proves the classifier covers the
    /// whole enum (a new variant would fail to compile, not silently escape).
    #[test]
    fn writes_last_operand_classifies_the_whole_mnemonic_set() {
        use Mnemonic::*;
        // Write-forms (destination is the last operand).
        for m in [
            Move, Movea, Moveq, Add, Adda, Sub, Suba, And, Or, Eor, Muls, Mulu,
            Divs, Divu, Addi, Subi, Andi, Ori, Eori, Addq, Subq, Asl, Asr, Lsl,
            Lsr, Rol, Ror, Bset, Bclr, Clr, Neg, Not, Tas, Lea, Swap, Ext,
            MoveFromSr,
        ] {
            assert!(writes_last_operand(m), "{m:?} should be a write-form");
        }
        // The named U1 escapees now covered by the ISA model.
        assert!(writes_last_operand(Movep), "movep (load form writes Dn)");
        assert!(writes_last_operand(Addx), "addx (Dy,Dx writes Dx)");
        // The set-cc family (every condition).
        for c in [Cond::T, Cond::Eq, Cond::Ne, Cond::Cs, Cond::Vc, Cond::Le] {
            assert!(writes_last_operand(Scc(c)), "s{c:?} should be a write-form");
        }
        // Read-only / control / operand-shape-modeled / SR-CCR forms.
        for m in [
            Cmp, Cmpa, Cmpi, Cmpm, Btst, Tst, Jmp, Jsr, Pea, Nop, Rts, Rte,
            Trap, Illegal, Bra, Bsr, MoveToSr, AndiCcr, OriCcr,
            // Movem/Dbcc are modeled by the front-end's operand-shape effects,
            // not the last-operand predicate.
            Movem,
        ] {
            assert!(!writes_last_operand(m), "{m:?} must not be a last-operand write-form");
        }
        for c in [Cond::Eq, Cond::Ne] {
            assert!(!writes_last_operand(Bcc(c)), "branch is not a write-form");
            assert!(!writes_last_operand(Dbcc(c)), "dbcc is operand-shape-modeled");
        }
    }
}
