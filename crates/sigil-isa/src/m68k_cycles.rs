//! 68000 instruction timing — the M68000UM Section 8 tables as data (B′-3b).
//!
//! One claim per entry: the number of clock periods a 68000 spends executing one
//! instruction form, assuming no wait states and no arbitration loss. Sources,
//! in order of authority:
//!
//!   * **M68000UM Section 8** (Tables 8-1..8-9) — the vendor's published counts;
//!     every entry here is that table's number.
//!   * **Exodus** (`oracle/Devices/M68000/*.h`) — transcribes the same tables per
//!     opcode; used as the per-family cross-check. Its `DIVS` charges a flat 168,
//!     which DISAGREES with the UM's 158 maximum — see [`instr_cycles`]'s div arm.
//!   * **oracle-next** (`oracle-core/src/m68000/`), SingleStepTests-validated —
//!     the second opinion for the data-dependent forms: its exact
//!     `divs_cycles`/`divu_cycles` drivers bound every real division at or below
//!     the UM maxima charged here.
//!
//! A form outside this table is [`CycleCost::Unmodeled`] — a loud refusal in the
//! consumer, never a default cost. The table is size- and addressing-mode-keyed
//! because that is where the numbers vary and therefore where errors hide.
//!
//! Costs marked inexact ([`CycleCost::Fixed`]`.exact == false`) are MAXIMA over a
//! data-dependent execution (a multiply's bit pattern, a shift count read from a
//! register, a `DBcc` fall-through's two reasons): sound as a ceiling, unusable
//! as an equality. The linker-relaxation maxima (a bare symbolic operand's
//! abs.w/abs.l pair, `jbra`'s rung ladder) are the FRONT-END's ruling, applied
//! before this table is consulted — this table prices only resolved categories.

use crate::m68k::{Cond, Mnemonic, Size};

/// An effective-address CATEGORY — the M68000UM Table 8-2 row an operand's
/// addressing mode falls in. Carries no value: the calculation time depends on
/// the mode and the operation size only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EaCat {
    /// Data register direct `Dn`.
    Dn,
    /// Address register direct `An`.
    An,
    /// Address register indirect `(An)`.
    Ind,
    /// Postincrement `(An)+`.
    PostInc,
    /// Predecrement `-(An)`.
    PreDec,
    /// Displacement `(d16,An)`.
    Disp16An,
    /// Indexed `(d8,An,Xn)`.
    Disp8AnXn,
    /// Absolute short `(xxx).W`.
    AbsW,
    /// Absolute long `(xxx).L`.
    AbsL,
    /// PC-relative `(d16,PC)`.
    Pcd16,
    /// PC-relative indexed `(d8,PC,Xn)`.
    Pcd8Xn,
    /// Immediate `#xxx`.
    Imm,
}

/// One operand, reduced to what timing depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatOp {
    /// An effective address of the given category.
    Ea(EaCat),
    /// An immediate whose VALUE is known — the shift-count and quick-data forms
    /// price on it. Category [`EaCat::Imm`] wherever only the category matters.
    ImmVal(i32),
    /// A `movem` register list with this many registers set.
    RegList(u8),
    /// The status register `sr`.
    Sr,
    /// The condition-code register `ccr`.
    Ccr,
}

impl CatOp {
    /// The Table 8-2 category, when the operand is an effective address.
    fn cat(self) -> Option<EaCat> {
        match self {
            CatOp::Ea(c) => Some(c),
            CatOp::ImmVal(_) => Some(EaCat::Imm),
            CatOp::RegList(_) | CatOp::Sr | CatOp::Ccr => None,
        }
    }
}

/// What one instruction form costs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleCost {
    /// A single per-execution count. `exact: false` marks a MAXIMUM over a
    /// data-dependent execution — sound as a ceiling, unusable as an equality.
    Fixed {
        /// Clock periods.
        cycles: u16,
        /// Whether every execution costs exactly this many.
        exact: bool,
    },
    /// A conditional whose taken and not-taken sides cost differently. The pair
    /// is outcome-keyed; `exact: false` marks a not-taken side that is itself a
    /// maximum (`DBcc`'s two distinct fall-through reasons).
    Branch {
        /// Clock periods when the branch is taken.
        taken: u16,
        /// Clock periods when it falls through.
        not_taken: u16,
        /// Whether both sides are exact.
        exact: bool,
    },
    /// Not in the table. The consumer refuses; there is no default cost.
    Unmodeled,
}

/// Table 8-2 — effective-address calculation time, added to a family's base
/// cost wherever a family row says `+ ea`. `long` selects the second column
/// pair (long-word operands fetch one more word).
pub fn ea_time(cat: EaCat, long: bool) -> u16 {
    use EaCat::*;
    match (cat, long) {
        (Dn | An, _) => 0,
        (Ind | PostInc, false) => 4,
        (Ind | PostInc, true) => 8,
        (PreDec, false) => 6,
        (PreDec, true) => 10,
        (Disp16An | AbsW | Pcd16, false) => 8,
        (Disp16An | AbsW | Pcd16, true) => 12,
        (Disp8AnXn | Pcd8Xn, false) => 10,
        (Disp8AnXn | Pcd8Xn, true) => 14,
        (AbsL, false) => 12,
        (AbsL, true) => 16,
        (Imm, false) => 4,
        (Imm, true) => 8,
    }
}

/// Table 8-1 — the MOVE source × destination matrix, as its generating rule:
/// `4 + source fetch + destination store`, where the destination store is the
/// Table 8-2 time EXCEPT `-(An)`, which stores in 4 (byte/word) / 8 (long) —
/// two fewer than its fetch time, because the decrement overlaps the store.
/// That quirk is the one cell rule-vs-table disagreement would hide in, so the
/// tests pin matrix cells directly, corner rows and the `-(An)` column included.
pub fn move_cycles(long: bool, src: EaCat, dst: EaCat) -> u16 {
    let dst_time = match dst {
        EaCat::PreDec => {
            if long {
                8
            } else {
                4
            }
        }
        d => ea_time(d, long),
    };
    4 + ea_time(src, long) + dst_time
}

/// The cost of one resolved 68000 instruction form, or [`CycleCost::Unmodeled`].
///
/// `ops` is the operand list in source order, reduced to timing categories. The
/// caller resolves default sizes and linker relaxation FIRST — every category
/// arriving here prices as itself.
pub fn instr_cycles(m: Mnemonic, size: Size, ops: &[CatOp]) -> CycleCost {
    use Mnemonic::*;
    let long = size == Size::L;
    let exact = |cycles: u16| CycleCost::Fixed { cycles, exact: true };
    let at_most = |cycles: u16| CycleCost::Fixed { cycles, exact: false };
    match (m, ops) {
        // --- data movement -------------------------------------------------
        // Table 8-1 (MOVE, MOVEA). `move` to/from SR/CCR is Table 8-6 and is
        // matched on the operand, not the mnemonic, before the matrix.
        (Move, [src, CatOp::Sr | CatOp::Ccr]) => match src.cat() {
            // MOVE to SR / MOVE to CCR: 12 + ea (both word-sized operations).
            Some(s) => exact(12 + ea_time(s, false)),
            None => CycleCost::Unmodeled,
        },
        (Move, [CatOp::Sr, CatOp::Ea(dst)]) => {
            // MOVE from SR: Dn 6; memory 8 + ea.
            match dst {
                EaCat::Dn => exact(6),
                d => exact(8 + ea_time(*d, false)),
            }
        }
        (Move | Movea, [src, dst]) => match (src.cat(), dst.cat()) {
            (Some(s), Some(d)) => exact(move_cycles(long, s, d)),
            _ => CycleCost::Unmodeled,
        },
        (Moveq, _) => exact(4),
        // Table 8-9: MOVEP word 16, long 24.
        (Movep, _) => exact(if long { 24 } else { 16 }),
        // Table 8-9: MOVEM, keyed by direction, size, mode and register count.
        (Movem, [CatOp::RegList(n), CatOp::Ea(dst)]) => movem_reg_to_mem(long, *dst, *n),
        (Movem, [CatOp::Ea(src), CatOp::RegList(n)]) => movem_mem_to_reg(long, *src, *n),
        // Table 8-4: LEA prices as its own row, not 4 + Table 8-2 (the indexed
        // form is 12, not 14).
        (Lea, [CatOp::Ea(src), _]) => match src {
            EaCat::Ind => exact(4),
            EaCat::Disp16An | EaCat::AbsW | EaCat::Pcd16 => exact(8),
            EaCat::Disp8AnXn | EaCat::AbsL | EaCat::Pcd8Xn => exact(12),
            _ => CycleCost::Unmodeled,
        },
        (Pea, [CatOp::Ea(src)]) => match src {
            EaCat::Ind => exact(12),
            EaCat::Disp16An | EaCat::AbsW | EaCat::Pcd16 => exact(16),
            EaCat::Disp8AnXn | EaCat::AbsL | EaCat::Pcd8Xn => exact(20),
            _ => CycleCost::Unmodeled,
        },

        // --- integer arithmetic / logic ------------------------------------
        // The base spellings the encoder REFINES to their immediate/address
        // forms (`#imm, <mem>` to `addi`-class; `cmp` to an address register to
        // `cmpa` — the front-end's `refine_m68k_mnemonic`) price AS the refined
        // form, by recursing into its arm: the numbers keep one owner.
        (Add | Sub | And | Or | Eor | Cmp, [CatOp::ImmVal(_) | CatOp::Ea(EaCat::Imm), CatOp::Ea(d)])
            if is_mem(*d) =>
        {
            let refined = match m {
                Add => Addi,
                Sub => Subi,
                And => Andi,
                Or => Ori,
                Eor => Eori,
                Cmp => Cmpi,
                _ => unreachable!("the pattern names these six"),
            };
            instr_cycles(refined, size, ops)
        }
        (Cmp, [_, CatOp::Ea(EaCat::An)]) => instr_cycles(Cmpa, size, ops),
        // Table 8-4 (standard two-operand). `<ea>, Dn`: byte/word 4 + ea; long
        // 6 + ea, +2 when the source is a register or immediate (the UM's `**`
        // footnote). `Dn, <ea>` (memory destination): byte/word 8 + ea; long
        // 12 + ea. CMP never writes back: long stays 6 + ea, no footnote.
        (Add | Sub | And | Or, [src, CatOp::Ea(EaCat::Dn)]) => {
            standard_to_reg(src.cat(), long, /*plus2_on_reg_imm=*/ true)
        }
        (Cmp, [src, CatOp::Ea(EaCat::Dn)]) => {
            standard_to_reg(src.cat(), long, /*plus2_on_reg_imm=*/ false)
        }
        (Add | Sub | And | Or | Eor, [CatOp::Ea(EaCat::Dn), CatOp::Ea(dst)]) => match dst {
            // EOR to a data register: byte/word 4, long 8 (Table 8-4's Dn row).
            EaCat::Dn => exact(if long { 8 } else { 4 }),
            EaCat::An => CycleCost::Unmodeled,
            d => exact(if long { 12 } else { 8 } + ea_time(*d, long)),
        },
        // Table 8-4: ADDA/SUBA word 8 + ea (the sign-extending form); long
        // 6 + ea with the same register/immediate `+2` footnote. CMPA is
        // 6 + ea at both sizes.
        (Adda | Suba, [src, CatOp::Ea(EaCat::An)]) => match src.cat() {
            Some(s) if !long => exact(8 + ea_time(s, false)),
            Some(s) => standard_to_reg(Some(s), true, true),
            None => CycleCost::Unmodeled,
        },
        (Cmpa, [src, CatOp::Ea(EaCat::An)]) => match src.cat() {
            Some(s) => exact(6 + ea_time(s, long)),
            None => CycleCost::Unmodeled,
        },
        // Table 8-5: immediate forms. To Dn: byte/word 8, long 16 (CMPI 14).
        // To memory: byte/word 12 + ea, long 20 + ea (CMPI reads only: 8 + ea /
        // 12 + ea). ANDI/ORI/EORI to CCR (Table 8-6): 20.
        (Andi | Ori | Eori, [_, CatOp::Ccr]) | (AndiCcr | OriCcr, _) => exact(20),
        (Andi | Ori | Eori, [_, CatOp::Sr]) => exact(20),
        (Addi | Subi | Andi | Ori | Eori, [_, CatOp::Ea(dst)]) => match dst {
            EaCat::Dn => exact(if long { 16 } else { 8 }),
            EaCat::An => CycleCost::Unmodeled,
            d => exact(if long { 20 } else { 12 } + ea_time(*d, long)),
        },
        (Cmpi, [_, CatOp::Ea(dst)]) => match dst {
            EaCat::Dn => exact(if long { 14 } else { 8 }),
            EaCat::An => CycleCost::Unmodeled,
            d => exact(if long { 12 } else { 8 } + ea_time(*d, long)),
        },
        // Table 8-5: quick forms. Dn: byte/word 4, long 8. An: 8 at word and
        // long (no CCR update, full sign-extended add). Memory: byte/word
        // 8 + ea, long 12 + ea.
        (Addq | Subq, [_, CatOp::Ea(dst)]) => match dst {
            EaCat::Dn => exact(if long { 8 } else { 4 }),
            EaCat::An => exact(8),
            d => exact(if long { 12 } else { 8 } + ea_time(*d, long)),
        },
        // Table 8-4/8-9: ADDX Dn,Dn 4 / 8; -(An),-(An) 18 / 30.
        (Addx, [CatOp::Ea(EaCat::Dn), CatOp::Ea(EaCat::Dn)]) => exact(if long { 8 } else { 4 }),
        (Addx, [CatOp::Ea(EaCat::PreDec), CatOp::Ea(EaCat::PreDec)]) => {
            exact(if long { 30 } else { 18 })
        }
        // Table 8-9: CMPM (An)+,(An)+ 12 / 20.
        (Cmpm, _) => exact(if long { 20 } else { 12 }),
        // Table 8-4 (MULS/MULU/DIVS/DIVU): data-dependent, charged at the UM
        // MAXIMUM plus the source fetch. MULU/MULS: 38 + 2n ≤ 70 (n ≤ 16 bit
        // positions). DIVU ≤ 140; DIVS ≤ 158. oracle-next's SingleStepTests-
        // validated exact drivers stay at or below these (DIVS normal path
        // ≤ 152, trailing refill included). Exodus charges DIVS a flat
        // 168 — ABOVE the UM maximum; recorded as a source disagreement, not
        // adopted: two independent sources agree 158 bounds the hardware
        // (oracle-next's returned range already INCLUDES its trailing refill).
        // MULU with a KNOWN immediate source prices EXACTLY: the UM's `38+2n`
        // has n = the number of ones in the source operand, and an immediate
        // source's ones are visible at compile time. Add the Table 8-2
        // immediate fetch. The all-ones source (n=16) reproduces the ceiling
        // row below, so the exact form and the maximum cannot drift apart.
        // MULS stays a ceiling even with a known immediate: its n counts the
        // 01/10 transitions of the 17-bit sign-extended source — value-aware
        // pricing is one line of popcount away but has no consumer yet, and an
        // unconsumed exact row is an untested claim.
        (Mulu, [CatOp::ImmVal(v), CatOp::Ea(EaCat::Dn)]) => {
            let ones = (*v as u16).count_ones() as u16;
            exact(38 + 2 * ones + ea_time(EaCat::Imm, false))
        }
        (Mulu | Muls, [src, CatOp::Ea(EaCat::Dn)]) => match src.cat() {
            Some(s) => at_most(70 + ea_time(s, false)),
            None => CycleCost::Unmodeled,
        },
        (Divu, [src, CatOp::Ea(EaCat::Dn)]) => match src.cat() {
            Some(s) => at_most(140 + ea_time(s, false)),
            None => CycleCost::Unmodeled,
        },
        (Divs, [src, CatOp::Ea(EaCat::Dn)]) => match src.cat() {
            Some(s) => at_most(158 + ea_time(s, false)),
            None => CycleCost::Unmodeled,
        },

        // --- single-operand (Table 8-6) ------------------------------------
        (Clr | Neg | Not, [CatOp::Ea(dst)]) => match dst {
            EaCat::Dn => exact(if long { 6 } else { 4 }),
            EaCat::An | EaCat::Imm | EaCat::Pcd16 | EaCat::Pcd8Xn => CycleCost::Unmodeled,
            d => exact(if long { 12 } else { 8 } + ea_time(*d, long)),
        },
        (Tst, [CatOp::Ea(dst)]) => match dst {
            EaCat::An | EaCat::Imm => CycleCost::Unmodeled,
            d => exact(4 + ea_time(*d, long)),
        },
        (Tas, [CatOp::Ea(dst)]) => match dst {
            EaCat::Dn => exact(4),
            EaCat::An | EaCat::Imm | EaCat::Pcd16 | EaCat::Pcd8Xn => CycleCost::Unmodeled,
            d => exact(10 + ea_time(*d, false)),
        },
        (Ext, _) => exact(4),
        (Swap, _) => exact(4),
        // Table 8-8: Scc. On Dn: 4 when the condition is false, 6 when true —
        // `st` (always true) and `sf` (always false) are therefore exact; every
        // real condition is a data-dependent maximum. In memory: 8 + ea, both
        // outcomes (the store happens either way).
        (Scc(c), [CatOp::Ea(dst)]) => match (dst, c) {
            (EaCat::Dn, Cond::T) => exact(6),
            (EaCat::Dn, Cond::F) => exact(4),
            (EaCat::Dn, _) => at_most(6),
            (EaCat::An | EaCat::Imm | EaCat::Pcd16 | EaCat::Pcd8Xn, _) => CycleCost::Unmodeled,
            (d, _) => exact(8 + ea_time(*d, false)),
        },

        // --- shifts / rotates (Table 8-7) ----------------------------------
        // Register form: 6 + 2n (byte/word) / 8 + 2n (long). An immediate count
        // is 1..8 and exact; a count in a register is taken modulo 64 by the
        // hardware, so its ceiling — n = 63 — is a machine fact, not a guess.
        // Memory form (shift by one, word only): 8 + ea.
        (Asl | Asr | Lsl | Lsr | Rol | Ror, [CatOp::ImmVal(n), CatOp::Ea(EaCat::Dn)]) => {
            let n = (*n).clamp(0, 8) as u16;
            exact(if long { 8 } else { 6 } + 2 * n)
        }
        (Asl | Asr | Lsl | Lsr | Rol | Ror, [CatOp::Ea(EaCat::Dn), CatOp::Ea(EaCat::Dn)]) => {
            at_most(if long { 8 } else { 6 } + 2 * 63)
        }
        (Asl | Asr | Lsl | Lsr | Rol | Ror, [CatOp::Ea(dst)]) => match dst {
            EaCat::Dn | EaCat::An | EaCat::Imm | EaCat::Pcd16 | EaCat::Pcd8Xn => {
                CycleCost::Unmodeled
            }
            d => exact(8 + ea_time(*d, false)),
        },

        // --- bit operations (Table 8-8) ------------------------------------
        // Byte-sized in memory (exact); long on Dn, where the cost depends on
        // the bit number — the UM marks the Dn cells as maxima. BTST never
        // writes, so its Dn forms are exact.
        (Btst, [CatOp::ImmVal(_), CatOp::Ea(dst)]) => match dst {
            EaCat::Dn => exact(10),
            EaCat::An | EaCat::Imm => CycleCost::Unmodeled,
            d => exact(8 + ea_time(*d, false)),
        },
        (Btst, [CatOp::Ea(EaCat::Dn), CatOp::Ea(dst)]) => match dst {
            EaCat::Dn => exact(6),
            EaCat::An => CycleCost::Unmodeled,
            d => exact(4 + ea_time(*d, false)),
        },
        (Bclr, [CatOp::ImmVal(_), CatOp::Ea(dst)]) => match dst {
            EaCat::Dn => at_most(14),
            EaCat::An | EaCat::Imm | EaCat::Pcd16 | EaCat::Pcd8Xn => CycleCost::Unmodeled,
            d => exact(12 + ea_time(*d, false)),
        },
        (Bclr, [CatOp::Ea(EaCat::Dn), CatOp::Ea(dst)]) => match dst {
            EaCat::Dn => at_most(10),
            EaCat::An | EaCat::Imm | EaCat::Pcd16 | EaCat::Pcd8Xn => CycleCost::Unmodeled,
            d => exact(8 + ea_time(*d, false)),
        },
        (Bset, [CatOp::ImmVal(_), CatOp::Ea(dst)]) => match dst {
            EaCat::Dn => at_most(12),
            EaCat::An | EaCat::Imm | EaCat::Pcd16 | EaCat::Pcd8Xn => CycleCost::Unmodeled,
            d => exact(12 + ea_time(*d, false)),
        },
        (Bset, [CatOp::Ea(EaCat::Dn), CatOp::Ea(dst)]) => match dst {
            EaCat::Dn => at_most(8),
            EaCat::An | EaCat::Imm | EaCat::Pcd16 | EaCat::Pcd8Xn => CycleCost::Unmodeled,
            d => exact(8 + ea_time(*d, false)),
        },

        // --- control (Tables 8-6, 8-9) -------------------------------------
        // Bcc: taken 10 at either width; not-taken 8 (`.s`) / 12 (`.w`). BRA is
        // always taken: 10 at both widths, so width relaxation cannot move it.
        (Bra, _) => exact(10),
        (Bsr, _) => exact(18),
        (Bcc(_), _) => match size {
            Size::S => CycleCost::Branch { taken: 10, not_taken: 8, exact: true },
            Size::W => CycleCost::Branch { taken: 10, not_taken: 12, exact: true },
            _ => CycleCost::Unmodeled,
        },
        // DBcc: branch taken 10; fall-through 14 when the counter expires, 12
        // when the condition is true. `dbf`/`dbra` (condition never true) fall
        // through only by expiry — exact; any real condition's fall-through is
        // a two-reason maximum.
        (Dbcc(c), _) => CycleCost::Branch { taken: 10, not_taken: 14, exact: c == Cond::F },
        // JMP / JSR price per target mode (Table 8-6's own rows, not 8-2).
        (Jmp, [CatOp::Ea(t)]) => match t {
            EaCat::Ind => exact(8),
            EaCat::Disp16An | EaCat::AbsW | EaCat::Pcd16 => exact(10),
            EaCat::AbsL => exact(12),
            EaCat::Disp8AnXn | EaCat::Pcd8Xn => exact(14),
            _ => CycleCost::Unmodeled,
        },
        (Jsr, [CatOp::Ea(t)]) => match t {
            EaCat::Ind => exact(16),
            EaCat::Disp16An | EaCat::AbsW | EaCat::Pcd16 => exact(18),
            EaCat::AbsL => exact(20),
            EaCat::Disp8AnXn | EaCat::Pcd8Xn => exact(22),
            _ => CycleCost::Unmodeled,
        },
        (Nop, _) => exact(4),
        (Rts, _) => exact(16),
        (Rte, _) => exact(20),
        // TRAP and ILLEGAL transfer into an exception handler whose cost is not
        // this instruction's to state; MOVE from/to USP and STOP never appear in
        // the corpus. All refuse.
        _ => CycleCost::Unmodeled,
    }
}

/// A memory effective-address DESTINATION — the categories the `#imm, <mem>`
/// refinement applies to. Mirrors the front-end encoder's `is_mem_dest`.
fn is_mem(d: EaCat) -> bool {
    use EaCat::*;
    matches!(d, Ind | PostInc | PreDec | Disp16An | Disp8AnXn | AbsW | AbsL)
}

/// Table 8-4's `<ea>, Dn` row: byte/word 4 + ea; long 6 + ea, plus 2 when the
/// long source is a register or immediate (the UM `**` footnote — the write-back
/// idle overlaps a memory fetch but not a register one).
fn standard_to_reg(src: Option<EaCat>, long: bool, plus2_on_reg_imm: bool) -> CycleCost {
    let Some(s) = src else { return CycleCost::Unmodeled };
    let cycles = if long {
        let plus2 = plus2_on_reg_imm && matches!(s, EaCat::Dn | EaCat::An | EaCat::Imm);
        6 + ea_time(s, true) + if plus2 { 2 } else { 0 }
    } else {
        4 + ea_time(s, false)
    };
    CycleCost::Fixed { cycles, exact: true }
}

/// Table 8-9's MOVEM register-to-memory rows: `8 + 4n` words / `8 + 8n` longs
/// at `(An)`/`-(An)`, plus the extension-word fetch for the displaced and
/// absolute modes. `n` is the register count — static in the instruction, so
/// the cost is exact.
fn movem_reg_to_mem(long: bool, dst: EaCat, n: u8) -> CycleCost {
    let n = u16::from(n);
    let per = if long { 8 } else { 4 };
    let base = match dst {
        EaCat::Ind | EaCat::PreDec => 8,
        EaCat::Disp16An | EaCat::AbsW => 12,
        EaCat::Disp8AnXn => 14,
        EaCat::AbsL => 16,
        _ => return CycleCost::Unmodeled,
    };
    CycleCost::Fixed { cycles: base + per * n, exact: true }
}

/// Table 8-9's MOVEM memory-to-register rows: `12 + 4n` / `12 + 8n` at `(An)`/
/// `(An)+`, plus the extension-word fetch for the other modes.
fn movem_mem_to_reg(long: bool, src: EaCat, n: u8) -> CycleCost {
    let n = u16::from(n);
    let per = if long { 8 } else { 4 };
    let base = match src {
        EaCat::Ind | EaCat::PostInc => 12,
        EaCat::Disp16An | EaCat::AbsW | EaCat::Pcd16 => 16,
        EaCat::Disp8AnXn | EaCat::Pcd8Xn => 18,
        EaCat::AbsL => 20,
        _ => return CycleCost::Unmodeled,
    };
    CycleCost::Fixed { cycles: base + per * n, exact: true }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::m68k::{Cond, Mnemonic::*, Size};

    fn ea(c: EaCat) -> CatOp {
        CatOp::Ea(c)
    }
    fn exact(m: Mnemonic, size: Size, ops: &[CatOp]) -> u16 {
        match instr_cycles(m, size, ops) {
            CycleCost::Fixed { cycles, exact: true } => cycles,
            other => panic!("expected an exact fixed cost, got {other:?}"),
        }
    }
    fn at_most(m: Mnemonic, size: Size, ops: &[CatOp]) -> u16 {
        match instr_cycles(m, size, ops) {
            CycleCost::Fixed { cycles, exact: false } => cycles,
            other => panic!("expected a maximum, got {other:?}"),
        }
    }

    // Table 8-2 — one pin per row, both columns, including every cell the
    // corpus's own EA histogram exercises.
    #[test]
    fn ea_calculation_times_match_table_8_2() {
        use EaCat::*;
        let bw: &[(EaCat, u16)] = &[
            (Dn, 0),
            (An, 0),
            (Ind, 4),
            (PostInc, 4),
            (PreDec, 6),
            (Disp16An, 8),
            (Disp8AnXn, 10),
            (AbsW, 8),
            (AbsL, 12),
            (Pcd16, 8),
            (Pcd8Xn, 10),
            (Imm, 4),
        ];
        let l: &[(EaCat, u16)] = &[
            (Dn, 0),
            (An, 0),
            (Ind, 8),
            (PostInc, 8),
            (PreDec, 10),
            (Disp16An, 12),
            (Disp8AnXn, 14),
            (AbsW, 12),
            (AbsL, 16),
            (Pcd16, 12),
            (Pcd8Xn, 14),
            (Imm, 8),
        ];
        for &(c, t) in bw {
            assert_eq!(ea_time(c, false), t, "{c:?} b/w");
        }
        for &(c, t) in l {
            assert_eq!(ea_time(c, true), t, "{c:?} long");
        }
    }

    // Table 8-1 — matrix cells pinned DIRECTLY (not through the generating
    // rule), corners plus the `-(An)`-destination column where the rule and a
    // naive `4 + ea + ea` differ by 2.
    #[test]
    fn move_matrix_cells_match_table_8_1() {
        use EaCat::*;
        // (long, src, dst, cycles) — byte/word rows first.
        let cells: &[(bool, EaCat, EaCat, u16)] = &[
            (false, Dn, Dn, 4),
            (false, Dn, Ind, 8),
            (false, Dn, PreDec, 8),      // the quirk: NOT 4 + 0 + 6
            (false, Dn, AbsW, 12),
            (false, Dn, AbsL, 16),
            (false, PreDec, Dn, 10),     // fetch side keeps its 6
            (false, Disp16An, Dn, 12),
            (false, Imm, Dn, 8),
            (false, Imm, AbsW, 16),
            (false, AbsL, AbsL, 28),
            (false, PostInc, Ind, 12),
            (false, Pcd16, Dn, 12),
            (true, Dn, Dn, 4),
            (true, Dn, Ind, 12),
            (true, Dn, PreDec, 12),      // the quirk at long: NOT 4 + 0 + 10
            (true, Ind, Dn, 12),
            (true, PreDec, Ind, 22),
            (true, AbsL, AbsL, 36),
            (true, Imm, Dn, 12),
            (true, Disp8AnXn, Dn, 18),
        ];
        for &(long, s, d, t) in cells {
            assert_eq!(move_cycles(long, s, d), t, "move long={long} {s:?} -> {d:?}");
        }
    }

    // Table 8-4 — the standard two-operand rows, including the long-size `**`
    // footnote (+2 on a register/immediate source) and CMP's exemption from it.
    #[test]
    fn standard_alu_rows_match_table_8_4() {
        use EaCat::*;
        assert_eq!(exact(Add, Size::W, &[ea(Dn), ea(Dn)]), 4);
        assert_eq!(exact(Add, Size::L, &[ea(Dn), ea(Dn)]), 8); // 6 + 0 + 2
        assert_eq!(exact(Add, Size::L, &[CatOp::ImmVal(5), ea(Dn)]), 16); // 6 + 8 + 2
        assert_eq!(exact(Add, Size::L, &[ea(Ind), ea(Dn)]), 14); // 6 + 8, no +2
        assert_eq!(exact(Add, Size::W, &[ea(PostInc), ea(Dn)]), 8);
        assert_eq!(exact(Cmp, Size::L, &[ea(Dn), ea(Dn)]), 6); // CMP: no +2
        assert_eq!(exact(Cmp, Size::W, &[ea(Disp16An), ea(Dn)]), 12);
        assert_eq!(exact(Add, Size::W, &[ea(Dn), ea(Ind)]), 12); // 8 + 4
        assert_eq!(exact(Add, Size::L, &[ea(Dn), ea(Ind)]), 20); // 12 + 8
        assert_eq!(exact(Eor, Size::W, &[ea(Dn), ea(Dn)]), 4);
        assert_eq!(exact(Eor, Size::L, &[ea(Dn), ea(Dn)]), 8);
        assert_eq!(exact(Eor, Size::W, &[ea(Dn), ea(Ind)]), 12);
        assert_eq!(exact(Adda, Size::W, &[ea(Dn), ea(An)]), 8); // word ADDA: 8 + ea
        assert_eq!(exact(Adda, Size::L, &[ea(Dn), ea(An)]), 8); // 6 + 0 + 2
        assert_eq!(exact(Adda, Size::L, &[ea(Ind), ea(An)]), 14);
        assert_eq!(exact(Cmpa, Size::W, &[ea(Dn), ea(An)]), 6);
        assert_eq!(exact(Cmpa, Size::L, &[ea(AbsW), ea(An)]), 18); // 6 + 12
    }

    // Table 8-5 — immediate and quick rows; CMPI's cheaper read-only column.
    #[test]
    fn immediate_and_quick_rows_match_table_8_5() {
        use EaCat::*;
        assert_eq!(exact(Addi, Size::W, &[CatOp::ImmVal(1), ea(Dn)]), 8);
        assert_eq!(exact(Addi, Size::L, &[CatOp::ImmVal(1), ea(Dn)]), 16);
        assert_eq!(exact(Addi, Size::W, &[CatOp::ImmVal(1), ea(Ind)]), 16); // 12 + 4
        assert_eq!(exact(Addi, Size::L, &[CatOp::ImmVal(1), ea(Ind)]), 28); // 20 + 8
        assert_eq!(exact(Cmpi, Size::W, &[CatOp::ImmVal(1), ea(Dn)]), 8);
        assert_eq!(exact(Cmpi, Size::L, &[CatOp::ImmVal(1), ea(Dn)]), 14);
        assert_eq!(exact(Cmpi, Size::W, &[CatOp::ImmVal(1), ea(Ind)]), 12); // 8 + 4
        assert_eq!(exact(Cmpi, Size::L, &[CatOp::ImmVal(1), ea(Ind)]), 20); // 12 + 8
        assert_eq!(exact(Addq, Size::W, &[CatOp::ImmVal(1), ea(Dn)]), 4);
        assert_eq!(exact(Addq, Size::L, &[CatOp::ImmVal(1), ea(Dn)]), 8);
        assert_eq!(exact(Addq, Size::W, &[CatOp::ImmVal(1), ea(An)]), 8);
        assert_eq!(exact(Subq, Size::L, &[CatOp::ImmVal(1), ea(An)]), 8);
        assert_eq!(exact(Addq, Size::W, &[CatOp::ImmVal(1), ea(Ind)]), 12); // 8 + 4
        assert_eq!(exact(Andi, Size::W, &[CatOp::ImmVal(1), ea(AbsW)]), 20); // 12 + 8
    }

    // Table 8-6 — single-operand rows.
    #[test]
    fn single_operand_rows_match_table_8_6() {
        use EaCat::*;
        assert_eq!(exact(Clr, Size::W, &[ea(Dn)]), 4);
        assert_eq!(exact(Clr, Size::L, &[ea(Dn)]), 6);
        assert_eq!(exact(Clr, Size::W, &[ea(Ind)]), 12); // 8 + 4
        assert_eq!(exact(Clr, Size::L, &[ea(AbsW)]), 24); // 12 + 12
        assert_eq!(exact(Neg, Size::W, &[ea(Dn)]), 4);
        assert_eq!(exact(Not, Size::L, &[ea(Dn)]), 6);
        assert_eq!(exact(Tst, Size::W, &[ea(Dn)]), 4);
        assert_eq!(exact(Tst, Size::L, &[ea(Ind)]), 12); // 4 + 8
        assert_eq!(exact(Tst, Size::W, &[ea(AbsW)]), 12); // 4 + 8
        assert_eq!(exact(Tas, Size::B, &[ea(Dn)]), 4);
        assert_eq!(exact(Tas, Size::B, &[ea(Ind)]), 14); // 10 + 4
        assert_eq!(exact(Ext, Size::W, &[ea(Dn)]), 4);
        assert_eq!(exact(Swap, Size::W, &[ea(Dn)]), 4);
    }

    // Table 8-7 — shifts: immediate counts price exactly; a register count is
    // the modulo-64 ceiling; the memory form shifts by one.
    #[test]
    fn shift_rows_match_table_8_7() {
        use EaCat::*;
        assert_eq!(exact(Lsr, Size::W, &[CatOp::ImmVal(1), ea(Dn)]), 8); // 6 + 2
        assert_eq!(exact(Lsl, Size::W, &[CatOp::ImmVal(8), ea(Dn)]), 22); // 6 + 16
        assert_eq!(exact(Asl, Size::L, &[CatOp::ImmVal(4), ea(Dn)]), 16); // 8 + 8
        assert_eq!(at_most(Lsr, Size::W, &[ea(Dn), ea(Dn)]), 132); // 6 + 2*63
        assert_eq!(at_most(Rol, Size::L, &[ea(Dn), ea(Dn)]), 134); // 8 + 2*63
        assert_eq!(exact(Asr, Size::W, &[ea(Ind)]), 12); // 8 + 4
    }

    // Table 8-8 — bit operations: BTST reads only (exact everywhere); the
    // writing forms' Dn cells are UM-marked maxima.
    #[test]
    fn bit_op_rows_match_table_8_8() {
        use EaCat::*;
        assert_eq!(exact(Btst, Size::B, &[CatOp::ImmVal(3), ea(Ind)]), 12); // 8 + 4
        assert_eq!(exact(Btst, Size::L, &[CatOp::ImmVal(3), ea(Dn)]), 10);
        assert_eq!(exact(Btst, Size::B, &[ea(Dn), ea(Ind)]), 8); // 4 + 4
        assert_eq!(exact(Btst, Size::L, &[ea(Dn), ea(Dn)]), 6);
        assert_eq!(at_most(Bclr, Size::L, &[CatOp::ImmVal(3), ea(Dn)]), 14);
        assert_eq!(at_most(Bclr, Size::L, &[ea(Dn), ea(Dn)]), 10);
        assert_eq!(exact(Bclr, Size::B, &[CatOp::ImmVal(3), ea(Ind)]), 16); // 12 + 4
        assert_eq!(at_most(Bset, Size::L, &[CatOp::ImmVal(3), ea(Dn)]), 12);
        assert_eq!(at_most(Bset, Size::L, &[ea(Dn), ea(Dn)]), 8);
        assert_eq!(exact(Bset, Size::B, &[ea(Dn), ea(Ind)]), 12); // 8 + 4
    }

    // Tables 8-6/8-9 — control flow: the branch pairs, both widths; DBcc's
    // exact-only-for-false rule; JMP/JSR per target mode; the returns.
    #[test]
    fn control_rows_match_tables_8_6_and_8_9() {
        use EaCat::*;
        assert_eq!(instr_cycles(Bra, Size::S, &[]), CycleCost::Fixed { cycles: 10, exact: true });
        assert_eq!(instr_cycles(Bra, Size::W, &[]), CycleCost::Fixed { cycles: 10, exact: true });
        assert_eq!(
            instr_cycles(Bcc(Cond::Eq), Size::S, &[]),
            CycleCost::Branch { taken: 10, not_taken: 8, exact: true }
        );
        assert_eq!(
            instr_cycles(Bcc(Cond::Eq), Size::W, &[]),
            CycleCost::Branch { taken: 10, not_taken: 12, exact: true }
        );
        assert_eq!(
            instr_cycles(Dbcc(Cond::F), Size::W, &[]),
            CycleCost::Branch { taken: 10, not_taken: 14, exact: true }
        );
        assert_eq!(
            instr_cycles(Dbcc(Cond::Eq), Size::W, &[]),
            CycleCost::Branch { taken: 10, not_taken: 14, exact: false }
        );
        assert_eq!(exact(Jmp, Size::W, &[ea(Ind)]), 8);
        assert_eq!(exact(Jmp, Size::W, &[ea(Disp16An)]), 10);
        assert_eq!(exact(Jmp, Size::W, &[ea(AbsW)]), 10);
        assert_eq!(exact(Jmp, Size::W, &[ea(AbsL)]), 12);
        assert_eq!(exact(Jmp, Size::W, &[ea(Disp8AnXn)]), 14);
        assert_eq!(exact(Jsr, Size::W, &[ea(Ind)]), 16);
        assert_eq!(exact(Jsr, Size::W, &[ea(AbsW)]), 18);
        assert_eq!(exact(Jsr, Size::W, &[ea(AbsL)]), 20);
        assert_eq!(exact(Bsr, Size::W, &[]), 18);
        assert_eq!(exact(Nop, Size::W, &[]), 4);
        assert_eq!(exact(Rts, Size::W, &[]), 16);
        assert_eq!(exact(Rte, Size::W, &[]), 20);
    }

    // Table 8-1/8-6 — the MOVE special destinations and sources (SR/CCR) and
    // the movement families' fixed rows.
    #[test]
    fn move_families_match_tables_8_1_and_8_6() {
        use EaCat::*;
        assert_eq!(exact(Move, Size::W, &[ea(Dn), ea(Ind)]), 8);
        assert_eq!(exact(Movea, Size::L, &[ea(PostInc), ea(An)]), 12);
        assert_eq!(exact(Moveq, Size::L, &[CatOp::ImmVal(1), ea(Dn)]), 4);
        assert_eq!(exact(Move, Size::W, &[CatOp::ImmVal(0x2700), CatOp::Sr]), 16); // 12 + 4
        assert_eq!(exact(Move, Size::W, &[ea(Dn), CatOp::Sr]), 12);
        assert_eq!(exact(Move, Size::W, &[CatOp::Sr, ea(Dn)]), 6);
        assert_eq!(exact(Move, Size::W, &[CatOp::Sr, ea(Ind)]), 12); // 8 + 4
        assert_eq!(exact(AndiCcr, Size::B, &[CatOp::ImmVal(1), CatOp::Ccr]), 20);
        assert_eq!(exact(Movep, Size::W, &[ea(Dn), ea(Disp16An)]), 16);
        assert_eq!(exact(Movep, Size::L, &[ea(Disp16An), ea(Dn)]), 24);
    }

    // Table 8-9 — MOVEM: both directions, both sizes, the per-mode bases, and
    // the register-count term.
    #[test]
    fn movem_rows_match_table_8_9() {
        use EaCat::*;
        let f = |m, s, ops: &[CatOp]| exact(m, s, ops);
        assert_eq!(f(Movem, Size::W, &[CatOp::RegList(3), ea(PreDec)]), 20); // 8 + 4*3
        assert_eq!(f(Movem, Size::L, &[CatOp::RegList(3), ea(PreDec)]), 32); // 8 + 8*3
        assert_eq!(f(Movem, Size::W, &[CatOp::RegList(2), ea(Ind)]), 16);
        assert_eq!(f(Movem, Size::L, &[CatOp::RegList(2), ea(Disp16An)]), 28); // 12 + 16
        assert_eq!(f(Movem, Size::W, &[ea(PostInc), CatOp::RegList(3)]), 24); // 12 + 4*3
        assert_eq!(f(Movem, Size::L, &[ea(PostInc), CatOp::RegList(3)]), 36); // 12 + 8*3
        assert_eq!(f(Movem, Size::W, &[ea(AbsW), CatOp::RegList(2)]), 24); // 16 + 8
        assert_eq!(f(Movem, Size::L, &[ea(AbsL), CatOp::RegList(1)]), 28); // 20 + 8
    }

    // Table 8-4 with the UM maxima — the data-dependent forms: sound ceilings,
    // never exact, and the DIVS entry is the one the two emulator cores
    // disagree on (Exodus 168 vs UM/oracle-next ≤ 158) — the UM number wins.
    // Table 8-4's MULU footnote — `38 + 2n`, n = ones in the source: a known
    // immediate prices exactly, and the all-ones immediate meets the ceiling
    // row, pinning the exact form and the maximum together.
    #[test]
    fn mulu_immediate_prices_on_its_ones_count() {
        use EaCat::*;
        assert_eq!(exact(Mulu, Size::W, &[CatOp::ImmVal(0), ea(Dn)]), 42); // 38 + 0 + 4
        assert_eq!(exact(Mulu, Size::W, &[CatOp::ImmVal(1), ea(Dn)]), 44);
        assert_eq!(exact(Mulu, Size::W, &[CatOp::ImmVal(66), ea(Dn)]), 46); // two ones
        assert_eq!(exact(Mulu, Size::W, &[CatOp::ImmVal(0x00FF), ea(Dn)]), 58); // eight ones
        assert_eq!(exact(Mulu, Size::W, &[CatOp::ImmVal(0xFFFF), ea(Dn)]), 74); // = the ceiling
        // MULS keeps its ceiling with a known immediate (transition count unmodeled).
        assert_eq!(at_most(Muls, Size::W, &[CatOp::ImmVal(66), ea(Dn)]), 74);
    }

    #[test]
    fn data_dependent_maxima_are_ceilings_not_exact() {
        use EaCat::*;
        assert_eq!(at_most(Mulu, Size::W, &[ea(Dn), ea(Dn)]), 70);
        assert_eq!(at_most(Muls, Size::W, &[ea(Ind), ea(Dn)]), 74); // 70 + 4
        assert_eq!(at_most(Divu, Size::W, &[ea(Dn), ea(Dn)]), 140);
        assert_eq!(at_most(Divs, Size::W, &[ea(Dn), ea(Dn)]), 158);
        assert_eq!(at_most(Divs, Size::W, &[ea(Disp16An), ea(Dn)]), 166); // 158 + 8
        assert_eq!(at_most(Scc(Cond::Eq), Size::B, &[ea(Dn)]), 6);
        assert_eq!(exact(Scc(Cond::T), Size::B, &[ea(Dn)]), 6);
        assert_eq!(exact(Scc(Cond::F), Size::B, &[ea(Dn)]), 4);
        assert_eq!(exact(Scc(Cond::Eq), Size::B, &[ea(Ind)]), 12); // 8 + 4
    }

    // The base spellings of the refined forms price as the refined form: the
    // encoder rewrites `#imm, <mem>` to the `addi` class and `cmp …, An` to
    // `cmpa`, so the table answers for the instruction that actually encodes.
    #[test]
    fn base_spellings_price_as_their_refined_forms() {
        use EaCat::*;
        assert_eq!(
            instr_cycles(Cmp, Size::W, &[CatOp::ImmVal(1), ea(Ind)]),
            instr_cycles(Cmpi, Size::W, &[CatOp::ImmVal(1), ea(Ind)])
        );
        assert_eq!(exact(Cmp, Size::W, &[CatOp::ImmVal(1), ea(Ind)]), 12); // 8 + 4
        assert_eq!(exact(Add, Size::L, &[CatOp::ImmVal(1), ea(Ind)]), 28); // 20 + 8
        assert_eq!(exact(And, Size::W, &[CatOp::ImmVal(1), ea(AbsW)]), 20); // 12 + 8
        assert_eq!(exact(Cmp, Size::W, &[ea(Dn), ea(An)]), 6); // cmpa row
        assert_eq!(exact(Cmp, Size::L, &[ea(Ind), ea(An)]), 14); // 6 + 8
    }

    // Off-table forms refuse: no default cost exists.
    #[test]
    fn off_table_forms_are_unmodeled() {
        use EaCat::*;
        assert_eq!(instr_cycles(Trap, Size::W, &[CatOp::ImmVal(0)]), CycleCost::Unmodeled);
        assert_eq!(instr_cycles(Illegal, Size::W, &[]), CycleCost::Unmodeled);
        assert_eq!(instr_cycles(Clr, Size::W, &[ea(An)]), CycleCost::Unmodeled);
        assert_eq!(instr_cycles(Jmp, Size::W, &[ea(Dn)]), CycleCost::Unmodeled);
    }
}
