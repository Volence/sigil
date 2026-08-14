//! Z80 T-state accounting — rung 4 (`2026-07-29-t40-step0-design.md` §3).
//!
//! The single Zilog T-state cost model, and the SOLE authority on what a Z80
//! instruction costs. [`instr_cost`] is the model; the two consumers differ only
//! in what they conclude from it:
//!
//!   * [`span_cost`] — the `cycles(L1, L2)` comptime builtin. Sums a
//!     STRAIGHT-LINE instruction span (label `L1` up to label `L2`) in one proc's
//!     evaluated `CodeBuf`. Its ruled scope (recon §4.3 ruling 2) is
//!     single-path accounting ONLY, so an outcome-split conditional is a bail.
//!   * [`crate::cycle_budget`] — the `@budget(cycles: N)` / `@cycles_exact`
//!     whole-proc path walk. It CAN tell a branch edge from a fall-through, so it
//!     charges [`Cost::Split`]'s two numbers to their two edges.
//!
//! A second table would be free to drift from this one, so there is not one:
//! both consumers key on [`instr_cost`], and a form it does not enumerate is
//! loud in both.
//!
//! The DAC loop makes this scope EXACT rather than approximate: every hot-loop
//! conditional is `jp cc` (10 T-states taken OR not-taken), so a span's cost is a
//! plain SUM with no branch-outcome dependence. Two HARD bails keep the accounting
//! honest and turn the driver's prose disciplines into compile errors:
//!
//!   * `[cycles.ambiguous-branch]` — a `jr cc` / `djnz` / `ret cc` / `call cc` inside
//!     a span has DIFFERING taken/not-taken cost, so no single cost is assignable.
//!     This is the `jp`-never-`jr`-on-the-hot-path discipline as a type error.
//!   * `[cycles.unknown-op]` — any op/form outside the driver-demand table. The table
//!     is the timed-region subset ONLY; a future timed region adds its ops
//!     explicitly, never a silent default cost.
//!   * `[cycles.path-end]` — a RETURN inside a span. A straight-line sum cannot
//!     represent a path ending, so it would go on costing instructions the
//!     machine never reaches.
//!
//! Table values are the asl/Zilog T-states, cross-checked against the driver's own
//! CYCLE-BALANCE PROOF arithmetic (`z80_sound_driver.asm:48-110`): FILL = 195,
//! DRAIN = 195, DRAINING_TAIL = 194.

use crate::value::{CodeItem, CodeOperand, Z80Reg8};
use sigil_span::Span;

/// The T-state cost of one instruction form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cost {
    /// A single, outcome-independent T-state count.
    Fixed(u16),
    /// A conditional whose taken and not-taken costs DIFFER (`jr cc`, `djnz`,
    /// `ret cc`, `call cc`). The two costs are OUTCOME-KEYED, not unknown: a
    /// consumer that can tell the branch edge from the fall-through edge charges
    /// each its own number. Straight-line accounting cannot, so [`span_cost`]
    /// treats this as the `[cycles.ambiguous-branch]` bail.
    Split {
        /// T-states when the branch is taken.
        taken: u16,
        /// T-states when it falls through.
        not_taken: u16,
    },
    /// An op/form outside the driver-demand timed-region table.
    Unknown,
}

/// Why a span-cost sum could not be produced.
#[derive(Debug, Clone)]
pub enum CycleBail {
    /// A variable-timing conditional sits inside the span.
    AmbiguousBranch { mnemonic: String, span: Span },
    /// An op/form outside the table sits inside the span.
    UnknownOp { mnemonic: String, span: Span },
    /// A RETURN sits inside the span. A straight-line sum has no notion of a path
    /// ending, so it would keep costing instructions the machine never reaches.
    PathEnd { mnemonic: String, span: Span },
}

/// `ix`/`iy` (a 16-bit inc/dec on an index reg costs 10, a plain pair 6)?
fn is_index_pair(p: crate::value::Z80Pair) -> bool {
    matches!(p, crate::value::Z80Pair::Ix | crate::value::Z80Pair::Iy)
}
/// Is this operand the 8-bit accumulator `a`?
fn is_a(op: &CodeOperand) -> bool {
    matches!(op, CodeOperand::Z80Reg8(Z80Reg8::A))
}
/// An 8-bit register operand (`a`..`l`)?
fn is_reg8(op: &CodeOperand) -> bool {
    matches!(op, CodeOperand::Z80Reg8(_))
}
/// An immediate (folded const or literal)?
fn is_imm(op: &CodeOperand) -> bool {
    matches!(op, CodeOperand::Imm(_))
}
/// A condition-code operand (`nz`/`z`/`nc`/`c`/…)?
fn is_cc(op: &CodeOperand) -> bool {
    matches!(op, CodeOperand::Z80Cc(_))
}
/// A symbolic branch target (`jp .loop`)?
fn is_sym(op: &CodeOperand) -> bool {
    matches!(op, CodeOperand::Sym(_) | CodeOperand::SymOff { .. })
}

/// The T-state cost of one resolved Z80 instruction FORM (mnemonic + operand
/// shapes), for the driver's timed-region op subset. Anything not enumerated is
/// [`Cost::Unknown`] (loud bail) — the table is the DEMAND subset, never a default.
pub fn instr_cost(mnemonic: &str, ops: &[CodeOperand]) -> Cost {
    use CodeOperand::*;
    match (mnemonic, ops) {
        // The niche-option `assume_some` extraction marker emits no bytes and
        // executes nothing — zero cycles, exactly. Without this arm a Z80
        // `@budget` proc containing a marker hard-bails `[cycles.unknown-op]` on
        // an instruction that is not in the ROM.
        ("assume_some", _) => Cost::Fixed(0),
        // --- 8-bit loads ---
        // ld r, r'                                    4
        ("ld", [a, b]) if is_reg8(a) && is_reg8(b) => Cost::Fixed(4),
        // ld r, n                                     7
        ("ld", [a, b]) if is_reg8(a) && is_imm(b) => Cost::Fixed(7),
        // ld a,(hl) / ld (hl),r / ld a,(de) / ld (de),a   7
        ("ld", [a, Z80IndHl]) | ("ld", [a, Z80IndDe]) | ("ld", [a, Z80IndBc]) if is_reg8(a) => {
            Cost::Fixed(7)
        }
        ("ld", [Z80IndHl, b]) | ("ld", [Z80IndDe, b]) | ("ld", [Z80IndBc, b]) if is_reg8(b) => {
            Cost::Fixed(7)
        }
        // ld (hl), n                                  10
        ("ld", [Z80IndHl, b]) if is_imm(b) => Cost::Fixed(10),
        // ld a,(nn) / ld (nn),a                       13
        ("ld", [a, Z80Mem { .. }]) if is_a(a) => Cost::Fixed(13),
        ("ld", [Z80Mem { .. }, b]) if is_a(b) => Cost::Fixed(13),
        // ld r,(ix+d) / ld (ix+d),r                   19
        ("ld", [a, Z80Indexed { .. }]) if is_reg8(a) => Cost::Fixed(19),
        ("ld", [Z80Indexed { .. }, b]) if is_reg8(b) => Cost::Fixed(19),

        // --- 8-bit arithmetic / logic on the accumulator ---
        // and/or/xor/cp/sub/add a, r                  4
        ("and" | "or" | "xor" | "cp" | "sub", [r]) if is_reg8(r) => Cost::Fixed(4),
        ("add" | "adc" | "sbc", [a, r]) if is_a(a) && is_reg8(r) => Cost::Fixed(4),
        // and/or/xor/cp/sub n                         7
        ("and" | "or" | "xor" | "cp" | "sub", [n]) if is_imm(n) => Cost::Fixed(7),

        // --- inc / dec ---
        // 8-bit: inc/dec r                            4
        ("inc" | "dec", [r]) if is_reg8(r) => Cost::Fixed(4),
        // 16-bit pair: inc/dec bc|de|hl|sp|af         6   (FILL's `dec hl` shadow len--)
        ("inc" | "dec", [CodeOperand::Z80Pair(p)]) if !is_index_pair(*p) => Cost::Fixed(6),
        // 16-bit index: inc/dec ix|iy                 10  (FILL's `inc ix` ROM++)
        ("inc" | "dec", [CodeOperand::Z80Pair(p)]) if is_index_pair(*p) => Cost::Fixed(10),

        // --- shadow-set exchange ---
        ("exx", []) => Cost::Fixed(4),

        // --- no-op (the pad primitive) ---
        ("nop", []) => Cost::Fixed(4),

        // --- 1-byte accumulator/flag primitives: 4 T each, like `nop`/`exx` ---
        ("rlca" | "rrca" | "rla" | "rra" | "daa" | "cpl" | "ccf" | "halt", []) => Cost::Fixed(4),
        // `rst p` — an 11 T call to a page-zero vector (1 byte, vs `call nn`'s
        // 3 bytes / 17 T). That density is why it is worth having.
        ("rst", [_]) => Cost::Fixed(11),

        // --- unconditional return: ret 10 ; reti/retn 14 (both ED-prefixed) ---
        ("ret", []) => Cost::Fixed(10),
        ("reti" | "retn", []) => Cost::Fixed(14),

        // --- unconditional jump (10) vs CONDITIONAL jp cc (10 either outcome) ---
        ("jp", [t]) if is_sym(t) => Cost::Fixed(10),
        ("jp", [cc, t]) if is_cc(cc) && is_sym(t) => Cost::Fixed(10),
        // `jp (hl)` — the register-indirect computed transfer (4 T, 1 M-cycle),
        // the Z80 twin of the 68k `jmp (a1)` dispatch. Priced so an enumerated
        // `jp (hl) targets(...)` can carry a budget; unenumerated it is still a
        // `[cycles.computed-transfer]` refusal (the structural check comes first).
        ("jp", [Z80IndHl]) => Cost::Fixed(4),

        // --- UNCONDITIONAL `jr e` (12 T, 2 bytes) — the DENSE pad primitive.
        // Unambiguous (there is no not-taken outcome), unlike `jr cc` below. It is
        // the 3x-denser pad unit: 12 T in 2 bytes vs a `nop`'s 4 T in 1 byte, which
        // is what `pad_to_cycles(..., dense: true)` emits. It is NOT admissible on
        // the hot streaming path for its own sake — the jp-not-jr discipline is
        // about the CONDITIONAL forms, whose taken/not-taken costs differ.
        ("jr", [t]) if is_sym(t) => Cost::Fixed(12),

        // --- the OUTCOME-SPLIT conditionals: taken/not-taken DIFFER ---
        // A straight-line span cannot assign these one cost (hence the
        // `[cycles.ambiguous-branch]` bail in `span_cost`); a path walk charges
        // `taken` on the branch edge and `not_taken` on the fall-through.
        ("jr", [cc, _]) if is_cc(cc) => Cost::Split { taken: 12, not_taken: 7 },
        ("djnz", _) => Cost::Split { taken: 13, not_taken: 8 },
        ("ret", [cc]) if is_cc(cc) => Cost::Split { taken: 11, not_taken: 5 },
        ("call", [cc, _]) if is_cc(cc) => Cost::Split { taken: 17, not_taken: 10 },

        // Everything else in a timed span is a loud bail.
        _ => Cost::Unknown,
    }
}

/// Sum the T-states of the instructions in `items` (a straight-line span). Labels
/// are skipped (zero cost). Returns the total, or the FIRST bail encountered.
pub fn span_cost(items: &[CodeItem]) -> Result<u16, CycleBail> {
    let mut total: u16 = 0;
    for it in items {
        let CodeItem::Instr { mnemonic, ops, span, .. } = it else { continue };
        // A return has a cost (the budget walk charges it), but inside a
        // straight-line span it is a path END: everything after it in the slice is
        // unreachable, so summing on would answer a question nobody asked.
        if crate::flag_check::is_return_mnemonic(mnemonic, sigil_ir::backend::Cpu::Z80) {
            return Err(CycleBail::PathEnd { mnemonic: mnemonic.clone(), span: *span });
        }
        match instr_cost(mnemonic, ops) {
            Cost::Fixed(n) => total = total.saturating_add(n),
            Cost::Split { .. } => {
                return Err(CycleBail::AmbiguousBranch {
                    mnemonic: mnemonic.clone(),
                    span: *span,
                })
            }
            Cost::Unknown => {
                return Err(CycleBail::UnknownOp { mnemonic: mnemonic.clone(), span: *span })
            }
        }
    }
    Ok(total)
}

/// Locate the half-open instruction span between two labels in a CodeBuf: from the
/// item AFTER `start` up to (not including) `end`. `None` if either label is absent
/// or `end` precedes `start`. (The span EXCLUDES the `end` label's own instruction,
/// matching the `[L1, L2)` reach convention — `cycles(.loop, .exhaust)` counts
/// `.loop`'s body up to the `.exhaust:` label.)
pub fn label_span<'a>(items: &'a [CodeItem], start: &str, end: &str) -> Option<&'a [CodeItem]> {
    let li = |name: &str| {
        items.iter().position(
            |it| matches!(it, CodeItem::Label { name: n, .. } if n == name),
        )
    };
    let s = li(start)?;
    let e = li(end)?;
    if e < s {
        return None;
    }
    Some(&items[s + 1..e])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{Z80Cond, Z80Reg8};
    use sigil_span::{SourceId, Span};

    fn sp() -> Span {
        Span { source: SourceId(0), start: 0, end: 0 }
    }
    fn instr(mnemonic: &str, ops: Vec<CodeOperand>) -> CodeItem {
        CodeItem::Instr {
            mnemonic: mnemonic.to_string(),
            size: None,
            ops,
            span: sp(),
            as_type: None,
            targets: Vec::new(),
            author: crate::value::ItemAuthor::User,
        }
    }
    fn label(name: &str) -> CodeItem {
        CodeItem::Label { name: name.to_string(), export: false, span: sp() }
    }
    fn a() -> CodeOperand {
        CodeOperand::Z80Reg8(Z80Reg8::A)
    }
    fn c() -> CodeOperand {
        CodeOperand::Z80Reg8(Z80Reg8::C)
    }
    fn l() -> CodeOperand {
        CodeOperand::Z80Reg8(Z80Reg8::L)
    }

    // The driver's FILL "consumer" block: ld l,c / ld a,(hl) / ld (de),a / inc c = 22.
    #[test]
    fn consumer_block_is_22() {
        let items = vec![
            instr("ld", vec![l(), c()]),                       // 4
            instr("ld", vec![a(), CodeOperand::Z80IndHl]),     // 7
            instr("ld", vec![CodeOperand::Z80IndDe, a()]),     // 7
            instr("inc", vec![c()]),                           // 4
        ];
        assert_eq!(span_cost(&items).unwrap(), 22);
    }

    // The FILL "timer poll": ld a,(nn) / and n / jp nz,sym = 13 + 7 + 10 = 30.
    #[test]
    fn timer_poll_is_30() {
        let items = vec![
            instr("ld", vec![a(), CodeOperand::Z80Mem { addr: 0x4000 }]),
            instr("and", vec![CodeOperand::Imm(1)]),
            instr("jp", vec![CodeOperand::Z80Cc(Z80Cond::Nz), CodeOperand::Sym("t".into())]),
        ];
        assert_eq!(span_cost(&items).unwrap(), 30);
    }

    // FILL's 16-bit pointer/len ops: `inc ix` (10, ROM++) and `dec hl` (6, shadow
    // len--) — both in the demand subset, both fixed-cost.
    #[test]
    fn pair_inc_dec_costs() {
        let inc_ix = vec![instr("inc", vec![CodeOperand::Z80Pair(crate::value::Z80Pair::Ix)])];
        assert_eq!(span_cost(&inc_ix).unwrap(), 10);
        let dec_hl = vec![instr("dec", vec![CodeOperand::Z80Pair(crate::value::Z80Pair::Hl)])];
        assert_eq!(span_cost(&dec_hl).unwrap(), 6);
    }

    // The whole FILL body block: ld a,(ix+0)/ld l,b/ld (hl),a/inc b/inc ix = 44.
    #[test]
    fn fill_body_is_44() {
        let items = vec![
            instr("ld", vec![a(), CodeOperand::Z80Indexed { reg: crate::value::Z80Index::Ix, disp: 0 }]), // 19
            instr("ld", vec![l(), CodeOperand::Z80Reg8(Z80Reg8::B)]),  // 4
            instr("ld", vec![CodeOperand::Z80IndHl, a()]),             // 7
            instr("inc", vec![CodeOperand::Z80Reg8(Z80Reg8::B)]),      // 4
            instr("inc", vec![CodeOperand::Z80Pair(crate::value::Z80Pair::Ix)]), // 10
        ];
        assert_eq!(span_cost(&items).unwrap(), 44);
    }

    // A `jr cc` inside a span is the HARD ambiguous bail (the jp-not-jr discipline).
    #[test]
    fn jr_cc_bails_ambiguous() {
        let items = vec![
            instr("nop", vec![]),
            instr("jr", vec![CodeOperand::Z80Cc(Z80Cond::Z), CodeOperand::Sym("x".into())]),
        ];
        assert!(matches!(span_cost(&items), Err(CycleBail::AmbiguousBranch { .. })));
    }

    // The POSITIVE control (t24): the SAME span with `jp cc` sums cleanly.
    #[test]
    fn jp_cc_is_fixed_positive_control() {
        let items = vec![
            instr("nop", vec![]),
            instr("jp", vec![CodeOperand::Z80Cc(Z80Cond::Z), CodeOperand::Sym("x".into())]),
        ];
        assert_eq!(span_cost(&items).unwrap(), 14); // 4 + 10
    }

    // djnz is ambiguous too (13/8).
    #[test]
    fn djnz_bails_ambiguous() {
        let items = vec![instr("djnz", vec![CodeOperand::Sym("x".into())])];
        assert!(matches!(span_cost(&items), Err(CycleBail::AmbiguousBranch { .. })));
    }

    // An off-table op (`rlca`) bails Unknown.
    #[test]
    fn off_table_bails_unknown() {
        // `ldir` is a REAL Z80 instruction that this table genuinely does not
        // price, which is the point: the bail must fire on an unmodeled op rather
        // than guess a cost. (It used to be `rlca` — which became priced when the
        // eight missing primitives landed, so the test was asserting that a
        // now-known op was unknown. If `ldir` is ever priced, move this to another
        // unpriced real op, not to a fake name.)
        let items = vec![instr("ldir", vec![])];
        assert!(matches!(span_cost(&items), Err(CycleBail::UnknownOp { .. })));
    }

    // exx + a nop pad: 4 + 4 = 8.
    #[test]
    fn exx_and_nop() {
        assert_eq!(span_cost(&[instr("exx", vec![]), instr("nop", vec![])]).unwrap(), 8);
    }

    // label_span carves [start.body .. end) and excludes the labels themselves.
    #[test]
    fn label_span_is_half_open() {
        let items = vec![
            label("loop"),
            instr("nop", vec![]),
            instr("nop", vec![]),
            label("exhaust"),
            instr("ret", vec![]),
        ];
        let span = label_span(&items, "loop", "exhaust").unwrap();
        assert_eq!(span_cost(span).unwrap(), 8); // two nops, ret excluded
    }
}
