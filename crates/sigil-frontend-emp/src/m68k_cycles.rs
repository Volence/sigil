//! The 68000 cost classifier — [`crate::value::CodeOperand`] forms to the
//! [`sigil_isa::m68k_cycles`] timing table (B′-3b).
//!
//! [`instr_cost`] is the 68000 sibling of [`crate::z80_cycles::instr_cost`]: the
//! one place a 68k instruction form is priced, consumed by the
//! [`crate::cycle_budget`] path walk. The table's NUMBERS live in the ISA crate;
//! what lives here is the mapping from surface operand forms to timing
//! categories — and with it the one ruling the ISA table cannot make:
//!
//! ## The relaxation ruling: charge the LONGEST rung
//!
//! A bare symbolic operand does not have one encoding at lowering time. The
//! LINKER picks it: a bare `Sym`/`SymOff` absolute relaxes across an
//! abs.w/abs.l pair, `jmp`/`jsr Sym` across the same pair, `jbra`/`jbsr` across
//! a four-rung ladder (`bra.s`/`bra.w`/`jmp abs.w`/`jmp abs.l`), and an unsized
//! `Bcc` across `.s`/`.w`. The rungs differ in LENGTH and therefore in CYCLES,
//! and this walk runs before the choice is made — so every such form is charged
//! its DEAREST rung and marked inexact. Charged cheapest, a budget would pass at
//! compile time and be wrong on the real ROM; refused outright, the ~2700 bare
//! symbolic operands in the corpus would put nearly every 68k proc out of reach
//! for the cost of at most 4 cycles of ceiling slack per operand. A width the
//! AUTHOR pinned (`(Sym).w`, an explicit `.s`/`.w` branch) has one encoding and
//! prices exactly.
//!
//! The maxima per relaxable form:
//!
//! | form | rungs (cycles) | charged |
//! |---|---|---|
//! | bare `Sym`/`SymOff` operand | abs.w (+8/+12) / abs.l (+12/+16) | abs.l |
//! | `jmp Sym` | 10 / 12 | 12 |
//! | `jbra Sym` | 10 / 10 / 10 / 12 | 12 |
//! | unsized `Bcc` | taken 10 both; not-taken 8 (`.s`) / 12 (`.w`) | 10 / 12 |
//! | `bra Sym` (any width) | 10 / 10 | 10, exact |
//!
//! `jsr`/`jbsr`/`bsr` are priced for table completeness (20 / 20 / 18) but the
//! walk refuses every call before consulting a cost.

use crate::lower::{m68k_default_size, m68k_mnemonic, reg_kind};
use crate::value::{CodeOperand, Width};
use sigil_backend_m68k::m68k::{Mnemonic, Size};
use sigil_backend_m68k::m68k_cycles::{instr_cycles, CatOp, CycleCost, EaCat};

/// The cost of one 68000 instruction form, or [`CycleCost::Unmodeled`] for a
/// form outside the table — the consumer refuses, never defaults.
pub fn instr_cost(mnemonic: &str, size: Option<Width>, ops: &[CodeOperand]) -> CycleCost {
    // The emp-only auto-reaching branches never enter the ISA mnemonic table;
    // their ladders are priced here (the module-header ruling).
    match mnemonic {
        "jbra" | "jra" => return CycleCost::Fixed { cycles: 12, exact: false },
        "jbsr" => return CycleCost::Fixed { cycles: 20, exact: false },
        _ => {}
    }
    let Some(m) = m68k_mnemonic(mnemonic) else {
        return CycleCost::Unmodeled;
    };
    // The branch families price on width, and an ABSENT width is the relax
    // ladder, not a default. Their symbolic target is a DISPLACEMENT, not an
    // effective address, so it must not classify (a bare-`Sym` EA would wrongly
    // clear `exact`) — each arm asks the ISA table with no operands.
    match m {
        Mnemonic::Bra | Mnemonic::Bsr => return instr_cycles(m, Size::S, &[]),
        Mnemonic::Bcc(_) => {
            return match size {
                Some(w) => instr_cycles(m, isa_size(w), &[]),
                // Unsized: the linker picks `.s` or `.w`. The ceiling is derived
                // from the table's own two rungs — taken agrees on both, the
                // not-taken side is charged the dearer.
                None => {
                    match (instr_cycles(m, Size::S, &[]), instr_cycles(m, Size::W, &[])) {
                        (
                            CycleCost::Branch { taken, not_taken: nt_s, .. },
                            CycleCost::Branch { not_taken: nt_w, .. },
                        ) => CycleCost::Branch {
                            taken,
                            not_taken: nt_s.max(nt_w),
                            exact: false,
                        },
                        _ => CycleCost::Unmodeled,
                    }
                }
            };
        }
        // DBcc is non-relaxable (one d16 form): one rung, one row.
        Mnemonic::Dbcc(_) => {
            return instr_cycles(m, Size::W, &[]);
        }
        _ => {}
    }
    let isa_size = match size {
        Some(w) => isa_size(w),
        None => match m68k_default_size(m) {
            Some(s) => s,
            None => return CycleCost::Unmodeled,
        },
    };
    let mut exact = true;
    let mut cats = Vec::with_capacity(ops.len());
    for op in ops {
        let Some(cat) = classify(op, &mut exact) else {
            return CycleCost::Unmodeled;
        };
        cats.push(cat);
    }
    match instr_cycles(m, isa_size, &cats) {
        CycleCost::Fixed { cycles, exact: e } => CycleCost::Fixed { cycles, exact: e && exact },
        CycleCost::Branch { taken, not_taken, exact: e } => {
            CycleCost::Branch { taken, not_taken, exact: e && exact }
        }
        CycleCost::Unmodeled => CycleCost::Unmodeled,
    }
}

/// The ISA-side size for a surface width suffix.
fn isa_size(w: Width) -> Size {
    match w {
        Width::B => Size::B,
        Width::W => Size::W,
        Width::L => Size::L,
        Width::S => Size::S,
    }
}

/// One operand's timing category. Clears `exact` for the linker-relaxed forms
/// (charged at their abs.l maximum); returns `None` for a form the table cannot
/// price.
fn classify(op: &CodeOperand, exact: &mut bool) -> Option<CatOp> {
    use CodeOperand as O;
    Some(match op {
        // The value rides along for the forms that price on it (shift counts);
        // out-of-range immediates never lower, so the clamp is unreachable
        // defense, not a semantic.
        O::Imm(n) => CatOp::ImmVal((*n).clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32),
        // A link-time immediate fetches like any other immediate; its VALUE
        // never prices (no shift takes one).
        O::ImmLink { .. } => CatOp::Ea(EaCat::Imm),
        O::Reg(r) => match reg_kind(*r) {
            (false, _) => CatOp::Ea(EaCat::Dn),
            (true, _) => CatOp::Ea(EaCat::An),
        },
        O::Sr => CatOp::Sr,
        O::Ccr => CatOp::Ccr,
        // THE RULING (module header): a bare symbolic absolute relaxes at link
        // time across abs.w/abs.l — charge the dearest rung, mark inexact.
        O::Sym(_) | O::SymOff { .. } => {
            *exact = false;
            CatOp::Ea(EaCat::AbsL)
        }
        // An author-pinned width has one encoding: exact.
        O::AbsSym { long, .. } | O::AbsInt { long, .. } => {
            CatOp::Ea(if *long { EaCat::AbsL } else { EaCat::AbsW })
        }
        // The symbolic d16 displacement is a FIXED-width (d16,An) encoding.
        O::DispSymInd { .. } | O::DispInd { .. } => CatOp::Ea(EaCat::Disp16An),
        O::Ind(_) => CatOp::Ea(EaCat::Ind),
        O::PreDec(_) => CatOp::Ea(EaCat::PreDec),
        O::PostInc(_) => CatOp::Ea(EaCat::PostInc),
        O::IndIdx { .. } => CatOp::Ea(EaCat::Disp8AnXn),
        O::PcRel { .. } => CatOp::Ea(EaCat::Pcd16),
        O::PcRelIdx { .. } => CatOp::Ea(EaCat::Pcd8Xn),
        O::RegList(mask) => CatOp::RegList(mask.count_ones() as u8),
        // A condition-code operand, and every Z80 form: not a 68000 EA.
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Reg;

    fn dn() -> CodeOperand {
        CodeOperand::Reg(Reg::D0)
    }
    fn an() -> CodeOperand {
        CodeOperand::Reg(Reg::A0)
    }
    fn sym(s: &str) -> CodeOperand {
        CodeOperand::Sym(s.to_string())
    }

    // The relaxation ruling, pinned form by form: a bare symbolic absolute is
    // charged its abs.l rung and marked inexact; the author-pinned width of the
    // same address is exact at its own rung.
    #[test]
    fn a_bare_symbol_charges_the_long_rung_and_is_inexact() {
        // move.w Sym, d0: 4 + abs.l(12) + 0 = 16, inexact.
        assert_eq!(
            instr_cost("move", Some(Width::W), &[sym("Var"), dn()]),
            CycleCost::Fixed { cycles: 16, exact: false }
        );
        // move.w (Sym).w, d0: 4 + abs.w(8) = 12, exact.
        assert_eq!(
            instr_cost(
                "move",
                Some(Width::W),
                &[CodeOperand::AbsSym { target: "Var".into(), long: false }, dn()]
            ),
            CycleCost::Fixed { cycles: 12, exact: true }
        );
        // move.w (Sym).l, d0: 4 + abs.l(12) = 16, exact — same number as the
        // bare form's ceiling, but a claim, not a bound.
        assert_eq!(
            instr_cost(
                "move",
                Some(Width::W),
                &[CodeOperand::AbsSym { target: "Var".into(), long: true }, dn()]
            ),
            CycleCost::Fixed { cycles: 16, exact: true }
        );
    }

    // The transfer ladders: `jbra` charges its `jmp abs.l` rung; `jmp Sym` its
    // abs.l rung; a `bra` costs 10 on every rung, so relaxation cannot move it
    // and it stays exact.
    #[test]
    fn transfer_ladders_charge_their_dearest_rung() {
        assert_eq!(
            instr_cost("jbra", None, &[sym("Elsewhere")]),
            CycleCost::Fixed { cycles: 12, exact: false }
        );
        assert_eq!(
            instr_cost("jmp", None, &[sym("Elsewhere")]),
            CycleCost::Fixed { cycles: 12, exact: false }
        );
        assert_eq!(
            instr_cost("bra", None, &[sym(".join")]),
            CycleCost::Fixed { cycles: 10, exact: true }
        );
        // jmp (a0) has ONE encoding: 8, exact.
        assert_eq!(
            instr_cost("jmp", None, &[CodeOperand::Ind(Reg::A0)]),
            CycleCost::Fixed { cycles: 8, exact: true }
        );
    }

    // An unsized conditional: the taken side is 10 on both rungs (exact as a
    // number, but the pair is marked inexact because the not-taken side is the
    // `.w` ceiling). Explicit widths price their own rung exactly.
    #[test]
    fn an_unsized_conditional_charges_the_word_fall_through() {
        assert_eq!(
            instr_cost("beq", None, &[sym(".skip")]),
            CycleCost::Branch { taken: 10, not_taken: 12, exact: false }
        );
        assert_eq!(
            instr_cost("beq", Some(Width::S), &[sym(".skip")]),
            CycleCost::Branch { taken: 10, not_taken: 8, exact: true }
        );
        assert_eq!(
            instr_cost("beq", Some(Width::W), &[sym(".skip")]),
            CycleCost::Branch { taken: 10, not_taken: 12, exact: true }
        );
    }

    // Default sizes resolve before pricing: `lea` is implicitly long (but
    // prices its own row), `moveq` long, `dbf` word — and `dbf`'s fall-through
    // is exact where a live condition's is a two-reason maximum.
    #[test]
    fn default_sizes_resolve_before_pricing() {
        assert_eq!(
            instr_cost("lea", None, &[sym("Table"), an()]),
            // LEA abs.l row: 12 — inexact through the bare symbol.
            CycleCost::Fixed { cycles: 12, exact: false }
        );
        assert_eq!(
            instr_cost("moveq", None, &[CodeOperand::Imm(1), dn()]),
            CycleCost::Fixed { cycles: 4, exact: true }
        );
        assert_eq!(
            instr_cost("dbf", None, &[dn(), sym(".loop")]),
            CycleCost::Branch { taken: 10, not_taken: 14, exact: true }
        );
        assert_eq!(
            instr_cost("dbeq", None, &[dn(), sym(".loop")]),
            CycleCost::Branch { taken: 10, not_taken: 14, exact: false }
        );
    }

    // The census's own hot forms, priced end to end.
    #[test]
    fn corpus_shapes_price_through_the_table() {
        // movem.l d0-d1/a0, -(sp): reg-to-mem long, 3 regs: 8 + 8*3 = 32.
        assert_eq!(
            instr_cost(
                "movem",
                Some(Width::L),
                &[CodeOperand::RegList(0b0000_0001_0000_0011), CodeOperand::PreDec(Reg::A7)]
            ),
            CycleCost::Fixed { cycles: 32, exact: true }
        );
        // move.l (a1)+, (a5): 4 + 8 + 8 = 20 — the DMA drain group's word.
        assert_eq!(
            instr_cost(
                "move",
                Some(Width::L),
                &[CodeOperand::PostInc(Reg::A1), CodeOperand::Ind(Reg::A5)]
            ),
            CycleCost::Fixed { cycles: 20, exact: true }
        );
        // move.w (a1)+, (a5): 4 + 4 + 4 = 12.
        assert_eq!(
            instr_cost(
                "move",
                Some(Width::W),
                &[CodeOperand::PostInc(Reg::A1), CodeOperand::Ind(Reg::A5)]
            ),
            CycleCost::Fixed { cycles: 12, exact: true }
        );
        // lsr.w #2, d0: 6 + 4 = 10, exact; lsr.w d1, d0: the modulo-64 ceiling.
        assert_eq!(
            instr_cost("lsr", Some(Width::W), &[CodeOperand::Imm(2), dn()]),
            CycleCost::Fixed { cycles: 10, exact: true }
        );
        assert_eq!(
            instr_cost("lsr", Some(Width::W), &[CodeOperand::Reg(Reg::D1), dn()]),
            CycleCost::Fixed { cycles: 132, exact: false }
        );
        // mulu (a0), d0: the 70-cycle worst case plus the 4-cycle fetch.
        assert_eq!(
            instr_cost("mulu", Some(Width::W), &[CodeOperand::Ind(Reg::A0), dn()]),
            CycleCost::Fixed { cycles: 74, exact: false }
        );
    }

    // Forms the table refuses: an unrecognized mnemonic, a Z80 operand, a trap.
    #[test]
    fn off_table_forms_refuse() {
        assert_eq!(instr_cost("link", Some(Width::W), &[an()]), CycleCost::Unmodeled);
        assert_eq!(instr_cost("trap", None, &[CodeOperand::Imm(0)]), CycleCost::Unmodeled);
        assert_eq!(
            instr_cost("move", Some(Width::W), &[CodeOperand::Z80IndHl, dn()]),
            CycleCost::Unmodeled
        );
    }
}
