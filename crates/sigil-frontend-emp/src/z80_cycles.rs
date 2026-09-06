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
//!   * `[cycles.opaque-call]` — a CALL (`call`, `rst`) inside a span. The
//!     instruction is priced, but its callee is not in the slice, so the sum
//!     would be a true T-state count of less code than runs.
//!
//! SCOPE. The table is no longer the driver-demand subset: it prices every
//! instruction FORM the Z80 encoder can emit, so `[cycles.unknown-op]` now means
//! "this assembler cannot assemble that either" rather than "nobody has needed it
//! yet". `tests::encoder_coverage` is what holds that: it asks the encoder which
//! forms it accepts and requires this table to price each one, so an encoding
//! added later cannot land unpriced and silent. Three form classes are priced
//! nowhere and each says so at its arm: the `(c)` port forms (no `CodeOperand`
//! spelling exists), the DDCB shift column (`rlc (ix+d)` — the encoder refuses
//! it), and the index-page forms the encoder does not reach (`jp (ix)`,
//! `ex (sp),ix`).
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

/// The type a span sum is accumulated and returned in.
///
/// The width here is a PROOF OBLIGATION, not a preference. [`MAX_SPAN_T_STATES`]
/// is the largest value [`span_cost`] can ever produce and is computed in this
/// same type; const evaluation makes arithmetic overflow a COMPILE error, so
/// narrowing this alias to a type the bound does not fit fails the build rather
/// than silently re-introducing a ceiling. That is why the sum below adds with a
/// plain `+` and never saturates, clamps, or checks: the overflow is
/// unrepresentable, not watched for. A saturating sum would answer 65 535 to a
/// span that costs more — a number where the model owes either the cost or a
/// refusal, which is the one thing a cost model must never do.
pub type TStates = u128;

/// The arithmetic ceiling on a span sum, and the compile-time proof that
/// [`TStates`] holds it. Two structural facts bound it:
///
///   * every [`Cost::Fixed`] payload is a `u16`, so one instruction site
///     contributes at most `u16::MAX` — a bound on the TYPE, so it survives any
///     future growth of the table;
///   * a `&[CodeItem]` is backed by an allocation and Rust caps an allocation at
///     `isize::MAX` bytes, so the slice holds at most
///     `isize::MAX / size_of::<CodeItem>()` items, instructions included.
///
/// The product is the ceiling. It is not a limit anything refuses at — nothing
/// can reach it — it is the number the accumulator width is chosen to hold.
pub const MAX_SPAN_T_STATES: TStates =
    (isize::MAX as TStates / std::mem::size_of::<CodeItem>() as TStates) * (u16::MAX as TStates);

/// `span_cost`'s result reaches `Value::Int`, which is an `i128`. The conversion
/// is lossless BY CONSTRUCTION and this is where that is proved.
const _: () = assert!(MAX_SPAN_T_STATES <= i128::MAX as TStates);

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
    /// A subroutine CALL (`call nn`, `rst p`) sits inside the span. The
    /// instruction's own cost is known and this table states it, but the callee's
    /// is not in the slice: a sum that charged only the call would answer 17 for
    /// a span that runs for however long the callee runs. The number would be a
    /// real T-state count of the wrong thing, which is the worst shape an answer
    /// can take, so the span refuses instead.
    OpaqueCall { mnemonic: String, span: Span },
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
/// A 16-bit absolute target, however it is spelled. `jp`/`call`/`ld rr,nn` take
/// either a link-time symbol or a folded constant, and the T-state count is the
/// same for both — the operand is an immediate word in the encoding either way.
fn is_abs16(op: &CodeOperand) -> bool {
    is_sym(op) || is_imm(op)
}
/// A CB-group bit NUMBER. Lowering accepts both spellings in that slot
/// (`map_z80_bit_number`): the classified [`CodeOperand::Z80Bit`] and a folded
/// `Imm` in `0..=7`, so the cost table has to recognise both or it prices only
/// half of the sites the encoder accepts.
fn is_bitnum(op: &CodeOperand) -> bool {
    matches!(op, CodeOperand::Z80Bit(_)) || matches!(op, CodeOperand::Imm(n) if (0..=7).contains(n))
}
/// The source of a base 8-bit accumulator ALU op (`add a,src` / `or src`).
///
/// Zilog UM0080 "8-Bit Arithmetic and Logical Group": every one of the eight ops
/// costs the same for a given SOURCE shape, so the cost is a function of the
/// source alone — register 4, immediate 7, `(hl)` 7, `(ix+d)` 19.
fn alu8_src_cost(src: &CodeOperand) -> Cost {
    match src {
        _ if is_reg8(src) => Cost::Fixed(4),
        _ if is_imm(src) => Cost::Fixed(7),
        CodeOperand::Z80IndHl => Cost::Fixed(7),
        CodeOperand::Z80Indexed { .. } => Cost::Fixed(19),
        _ => Cost::Unknown,
    }
}
/// The CB-group rotate/shift cost for a target shape (Zilog UM0080 "Rotate and
/// Shift Group"): `r` 8, `(hl)` 15. The `(ix+d)` form is 23 T on the machine but
/// has NO encoding here — `encode_cb_shift` refuses an `Indexed` target, so the
/// DDCB shift column cannot be assembled and pricing it would be a number for an
/// instruction this assembler cannot emit.
fn cb_shift_cost(target: &CodeOperand) -> Cost {
    match target {
        _ if is_reg8(target) => Cost::Fixed(8),
        CodeOperand::Z80IndHl => Cost::Fixed(15),
        _ => Cost::Unknown,
    }
}

/// The T-state cost of one resolved Z80 instruction FORM (mnemonic + operand
/// shapes). Anything not enumerated is [`Cost::Unknown`] (loud bail) — an ABSENT
/// cost, never a default one.
///
/// Every count is the Zilog T-state figure from the UM0080 instruction tables,
/// cited per group at the arms. The coverage obligation is stated and tested
/// rather than intended: `tests::encoder_coverage` derives the form set from
/// `z80::encode` itself and fails on any form this function answers `Unknown` for.
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
        // ld (ix+d), n                                19
        // Four bytes and five M-cycles, yet the SAME 19 T as the register form
        // above: the extra operand byte rides an M-cycle the displacement fetch
        // already needs. Zilog UM0080, "8-Bit Load Group".
        ("ld", [Z80Indexed { .. }, n]) if is_imm(n) => Cost::Fixed(19),
        // ld a,i / ld a,r / ld i,a / ld r,a           9
        // ED-prefixed, and the odd 9 (4 + 5) is the interrupt-register access,
        // not the 8 an ED pair usually costs. Zilog UM0080, "8-Bit Load Group".
        ("ld", [a, Z80RegI]) | ("ld", [a, Z80RegR]) if is_a(a) => Cost::Fixed(9),
        ("ld", [Z80RegI, b]) | ("ld", [Z80RegR, b]) if is_a(b) => Cost::Fixed(9),

        // --- 16-bit loads (Zilog UM0080, "16-Bit Load Group") ---
        // ld sp, hl                                    6
        // Matched before the pair-immediate arms below because its SOURCE is a
        // pair, not an immediate; the guards are disjoint either way.
        ("ld", [Z80Pair(crate::value::Z80Pair::Sp), Z80Pair(crate::value::Z80Pair::Hl)]) => {
            Cost::Fixed(6)
        }
        // ld dd,nn 10  ;  ld ix,nn 14 (the DD/FD prefix costs the extra 4)
        ("ld", [Z80Pair(p), n]) if is_abs16(n) => {
            Cost::Fixed(if is_index_pair(*p) { 14 } else { 10 })
        }
        // ld hl,(nn) 16 (base 2A)  ;  ld dd,(nn) 20 (ED-prefixed)  ;  ld ix,(nn) 20
        // The 4 T between them is the prefix byte, and `hl` is the ONLY pair with
        // an unprefixed form — which is exactly why the encoder never emits ED 6B.
        ("ld", [Z80Pair(crate::value::Z80Pair::Hl), Z80Mem { .. }]) => Cost::Fixed(16),
        ("ld", [Z80Pair(_), Z80Mem { .. }]) => Cost::Fixed(20),
        // ld (nn),hl 16  ;  ld (nn),dd 20  ;  ld (nn),ix 20 — the same split.
        ("ld", [Z80Mem { .. }, Z80Pair(crate::value::Z80Pair::Hl)]) => Cost::Fixed(16),
        ("ld", [Z80Mem { .. }, Z80Pair(_)]) => Cost::Fixed(20),
        // push qq 11 / pop qq 10 — the ONE place a push and its matching pop
        // differ, because the push writes two bytes after an extra idle T-state.
        // push ix 15 / pop ix 14 add the prefix's 4. Zilog UM0080, "16-Bit Load".
        ("push", [Z80Pair(p)]) => Cost::Fixed(if is_index_pair(*p) { 15 } else { 11 }),
        ("pop", [Z80Pair(p)]) => Cost::Fixed(if is_index_pair(*p) { 14 } else { 10 }),

        // --- 8-bit arithmetic / logic on the accumulator ---
        // The eight base ALU ops in both spellings the encoder accepts — the
        // two-operand `add a,src` and the one-operand `or src` — priced by the
        // SOURCE shape alone (see [`alu8_src_cost`]). `add a,n` and `or (hl)` are
        // the same instruction as `add a,b` with a dearer operand fetch.
        ("add" | "adc" | "sub" | "sbc" | "and" | "or" | "xor" | "cp", [a, src]) if is_a(a) => {
            alu8_src_cost(src)
        }
        ("add" | "adc" | "sub" | "sbc" | "and" | "or" | "xor" | "cp", [src]) => {
            alu8_src_cost(src)
        }

        // --- 16-bit arithmetic (Zilog UM0080, "16-Bit Arithmetic Group") ---
        // add hl,ss 11 (unprefixed 09)  vs  adc/sbc hl,ss 15 (ED-prefixed).
        // The carry-aware pair is FOUR T dearer for the prefix alone, which is the
        // discrimination a table built by copying `add hl,ss` across the family
        // would lose.
        ("add", [Z80Pair(crate::value::Z80Pair::Hl), Z80Pair(_)]) => Cost::Fixed(11),
        ("adc" | "sbc", [Z80Pair(crate::value::Z80Pair::Hl), Z80Pair(_)]) => Cost::Fixed(15),
        // add ix,pp 15 — `add hl,ss`'s 11 plus the DD/FD prefix's 4.
        ("add", [Z80Pair(p), Z80Pair(_)]) if is_index_pair(*p) => Cost::Fixed(15),

        // --- inc / dec ---
        // 8-bit: inc/dec r                            4
        ("inc" | "dec", [r]) if is_reg8(r) => Cost::Fixed(4),
        // 16-bit pair: inc/dec bc|de|hl|sp|af         6   (FILL's `dec hl` shadow len--)
        ("inc" | "dec", [CodeOperand::Z80Pair(p)]) if !is_index_pair(*p) => Cost::Fixed(6),
        // 16-bit index: inc/dec ix|iy                 10  (FILL's `inc ix` ROM++)
        ("inc" | "dec", [CodeOperand::Z80Pair(p)]) if is_index_pair(*p) => Cost::Fixed(10),
        // READ-MODIFY-WRITE through memory: inc (hl) 11, inc (ix+d) 23. Nothing
        // like the 4 T of the register form and nothing like the 6 T of the
        // 16-bit pair form — this is the arm a table that treated `inc` as one
        // instruction would get wrong by a factor of five.
        ("inc" | "dec", [Z80IndHl]) => Cost::Fixed(11),
        ("inc" | "dec", [Z80Indexed { .. }]) => Cost::Fixed(23),

        // --- exchanges (Zilog UM0080, "Exchange, Block Transfer, and Search") ---
        ("exx", []) => Cost::Fixed(4),
        // ex de,hl and ex af,af' are register renames: 4 T, one M-cycle.
        ("ex", [Z80Pair(crate::value::Z80Pair::De), Z80Pair(crate::value::Z80Pair::Hl)]) => {
            Cost::Fixed(4)
        }
        ("ex", [Z80Pair(crate::value::Z80Pair::Af), Z80AfShadow]) => Cost::Fixed(4),
        // ex (sp),hl is 19 T — it moves two bytes each way through the stack.
        // `(sp)` reaches here in BOTH emp spellings: the distinct `Z80IndSp` the
        // operand mapper produces, and the `Z80Pair(Sp)` it lowers to.
        ("ex", [Z80IndSp | Z80Pair(crate::value::Z80Pair::Sp), Z80Pair(crate::value::Z80Pair::Hl)]) => {
            Cost::Fixed(19)
        }

        // --- the ED block-transfer / search / I-O grid ---
        // The SINGLE-STEP members are flat 16 T: they move one byte and stop.
        // Zilog UM0080, "Exchange, Block Transfer, and Search" and "Input and
        // Output Group".
        ("ldi" | "ldd" | "cpi" | "cpd" | "ini" | "ind" | "outi" | "outd", _) => Cost::Fixed(16),
        // The REPEATING members carry the same 16 T final step plus a 21 T body:
        // the machine decrements PC by two and re-executes, so 21 is paid for
        // every iteration that repeats and 16 for the one that leaves. That is
        // exactly [`Cost::Split`]'s meaning — two outcome-keyed numbers — and it
        // is why they do not get a mechanism of their own.
        //
        // HOW THE CONSUMERS READ IT, which is the part worth stating out loud:
        // neither charges 21 to a loop edge, because a repeat is not an edge in
        // either CFG. `span_cost` bails `[cycles.ambiguous-branch]`, and the
        // budget walk refuses the same way (a split cost over an instruction
        // presenting one edge is not chargeable). Both are REFUSALS, which is the
        // honest answer: the true cost is 16 + 21*(BC-1) and BC is a run-time
        // value, so no single number is assignable.
        ("ldir" | "lddr" | "cpir" | "cpdr" | "inir" | "indr" | "otir" | "otdr", _) => {
            Cost::Split { taken: 21, not_taken: 16 }
        }

        // --- no-op (the pad primitive) ---
        ("nop", []) => Cost::Fixed(4),

        // --- 1-byte accumulator/flag primitives: 4 T each, like `nop`/`exx` ---
        ("rlca" | "rrca" | "rla" | "rra" | "daa" | "cpl" | "ccf" | "scf" | "halt", []) => {
            Cost::Fixed(4)
        }
        // `ei`/`di` are 4 T like the flag primitives, and are one byte each.
        ("ei" | "di", []) => Cost::Fixed(4),
        // `neg` and `im n` are ED-prefixed two-byte ops at 8 T. The operand-free
        // `_` on `neg` mirrors the encoder's own arm, which also ignores operands.
        ("neg", _) => Cost::Fixed(8),
        ("im", [n]) if is_imm(n) => Cost::Fixed(8),
        // `rrd`/`rld` rotate a BCD digit through `(hl)`: 18 T, the dearest ED
        // two-byte form, because they read, rotate and write memory back.
        ("rrd" | "rld", []) => Cost::Fixed(18),
        // `rst p` — an 11 T call to a page-zero vector (1 byte, vs `call nn`'s
        // 3 bytes / 17 T). That density is why it is worth having.
        ("rst", [_]) => Cost::Fixed(11),

        // --- CB-group rotates / shifts and bit operations ---
        // Rotate/shift: r 8, (hl) 15 (see [`cb_shift_cost`]).
        ("rlc" | "rrc" | "rl" | "rr" | "sla" | "sra" | "srl", [t]) => cb_shift_cost(t),
        // `bit` only TESTS, so it is 12 T through `(hl)` and 20 T through
        // `(ix+d)`; `set`/`res` WRITE THE BYTE BACK and pay 15 and 23 for the same
        // addressing. The register forms are 8 T for all three. Pricing the three
        // mnemonics alike is the wrong answer that looks right, because it agrees
        // on the register column, which is where a casual test would look.
        ("bit", [b, t]) if is_bitnum(b) => match t {
            _ if is_reg8(t) => Cost::Fixed(8),
            Z80IndHl => Cost::Fixed(12),
            Z80Indexed { .. } => Cost::Fixed(20),
            _ => Cost::Unknown,
        },
        ("res" | "set", [b, t]) if is_bitnum(b) => match t {
            _ if is_reg8(t) => Cost::Fixed(8),
            Z80IndHl => Cost::Fixed(15),
            Z80Indexed { .. } => Cost::Fixed(23),
            _ => Cost::Unknown,
        },

        // --- the direct-port I/O forms (Zilog UM0080, "Input and Output Group") ---
        // `in a,(n)` / `out (n),a` are 11 T. The register-indirect `(c)` forms are
        // 12 T on the machine and are NOT priced here, because they have no
        // spelling to price: `CodeOperand` has no `(c)` variant, so no emp
        // instruction can carry that operand shape to this table at all.
        ("in", [a, Z80Mem { .. }]) if is_a(a) => Cost::Fixed(11),
        ("out", [Z80Mem { .. }, a]) if is_a(a) => Cost::Fixed(11),

        // --- unconditional return: ret 10 ; reti/retn 14 (both ED-prefixed) ---
        ("ret", []) => Cost::Fixed(10),
        ("reti" | "retn", []) => Cost::Fixed(14),

        // --- unconditional call (17) — see `span_cost`'s OpaqueCall refusal for
        // why pricing it does not make a straight-line sum reach past it.
        ("call", [t]) if is_abs16(t) => Cost::Fixed(17),

        // --- unconditional jump (10) vs CONDITIONAL jp cc (10 either outcome) ---
        ("jp", [t]) if is_abs16(t) => Cost::Fixed(10),
        ("jp", [cc, t]) if is_cc(cc) && is_abs16(t) => Cost::Fixed(10),
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
///
/// The total is EXACT for every input the function can be handed: see
/// [`TStates`] and [`MAX_SPAN_T_STATES`] for why the accumulator cannot overflow.
/// Every path out of here is therefore the span's true cost or a named refusal —
/// there is no third answer.
pub fn span_cost(items: &[CodeItem]) -> Result<TStates, CycleBail> {
    let mut total: TStates = 0;
    for it in items {
        let CodeItem::Instr { mnemonic, ops, span, .. } = it else { continue };
        // A return has a cost (the budget walk charges it), but inside a
        // straight-line span it is a path END: everything after it in the slice is
        // unreachable, so summing on would answer a question nobody asked.
        if crate::flag_check::is_return_mnemonic(mnemonic, sigil_ir::backend::Cpu::Z80) {
            return Err(CycleBail::PathEnd { mnemonic: mnemonic.clone(), span: *span });
        }
        // A call's own cost is priced, but the callee is not in this slice. The
        // same classifier the budget walk uses for its `OpaqueCall` refusal is
        // asked here, so the two consumers agree on what a call is (`call` and
        // `rst` on the Z80) rather than each carrying its own idea.
        if crate::context::is_call_mnemonic(mnemonic, sigil_ir::backend::Cpu::Z80) {
            return Err(CycleBail::OpaqueCall { mnemonic: mnemonic.clone(), span: *span });
        }
        match instr_cost(mnemonic, ops) {
            // Plain `+`: the width proof above makes an overflow here
            // unrepresentable, so there is nothing to saturate or check.
            Cost::Fixed(n) => total += TStates::from(n),
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

    // An off-table FORM bails Unknown.
    #[test]
    fn off_table_bails_unknown() {
        // `ex (sp),ix` is a REAL, documented Z80 instruction (23 T) that this
        // table genuinely does not price, and it is unpriced for a stated reason
        // rather than by neglect: `encode_index` has no `ex` arm, so this
        // assembler cannot emit those bytes at all, and a cost for an instruction
        // that cannot be assembled is a number with nothing behind it.
        //
        // The fixture has now been `rlca`, then `ldir`, and each became priced.
        // Both times the test was left asserting that a KNOWN op was unknown.
        // Pick the next one the same way if this one is ever priced: a real op
        // the encoder refuses, never a fake name, and check `encoder_coverage`
        // below still passes afterwards.
        let items = vec![instr(
            "ex",
            vec![
                CodeOperand::Z80IndSp,
                CodeOperand::Z80Pair(crate::value::Z80Pair::Ix),
            ],
        )];
        assert!(matches!(span_cost(&items), Err(CycleBail::UnknownOp { .. })));
        // The DISCRIMINATING neighbour: the same mnemonic and the same `(sp)`
        // spelling with `hl` instead IS priced, so the refusal above is about the
        // form and not about `ex` being missing from the table.
        let hl = vec![instr(
            "ex",
            vec![
                CodeOperand::Z80IndSp,
                CodeOperand::Z80Pair(crate::value::Z80Pair::Hl),
            ],
        )];
        assert_eq!(span_cost(&hl).unwrap(), 19);
    }

    // A `call` inside a straight-line span is a REFUSAL, not a 17. The
    // instruction is priced — the budget walk charges it — but the callee is not
    // in the slice, so a sum would be a true count of less code than runs.
    #[test]
    fn call_and_rst_bail_opaque_in_a_span() {
        let c = vec![instr("call", vec![CodeOperand::Sym("Sub".into())])];
        assert!(matches!(span_cost(&c), Err(CycleBail::OpaqueCall { .. })));
        let r = vec![instr("rst", vec![CodeOperand::Imm(0x18)])];
        assert!(matches!(span_cost(&r), Err(CycleBail::OpaqueCall { .. })));
        // The table itself still states both costs (this is a SPAN rule, not a
        // hole in the model), and the two differ, so neither number is the other.
        assert_eq!(instr_cost("call", &[CodeOperand::Sym("Sub".into())]), Cost::Fixed(17));
        assert_eq!(instr_cost("rst", &[CodeOperand::Imm(0x18)]), Cost::Fixed(11));
    }

    // exx + a nop pad: 4 + 4 = 8.
    #[test]
    fn exx_and_nop() {
        assert_eq!(span_cost(&[instr("exx", vec![]), instr("nop", vec![])]).unwrap(), 8);
    }

    // A span whose TRUE cost passes 65 535 T-states must return that true cost.
    // The accumulator's own ceiling is not an answer: reporting it would be the
    // model saying a number where it owes either the sum or a refusal.
    //
    // Nothing here is a copied literal. The per-instruction cost comes from
    // `instr_cost` itself (so a table correction moves the expectation with it),
    // and the site count is derived from `u16::MAX` — the width being escaped.
    #[test]
    fn span_cost_past_65535_t_returns_the_true_sum() {
        // The dearest fixed form the table prices: `ld a,(ix+0)` at 19 T.
        let dearest = instr(
            "ld",
            vec![a(), CodeOperand::Z80Indexed { reg: crate::value::Z80Index::Ix, disp: 0 }],
        );
        let CodeItem::Instr { mnemonic, ops, .. } = &dearest else {
            unreachable!("the fixture is an instruction")
        };
        let per: u16 = match instr_cost(mnemonic, ops) {
            Cost::Fixed(n) => n,
            other => panic!("the fixture form must be a FIXED cost to sum; got {other:?}"),
        };
        assert!(per > 0, "a zero-cost fixture could never cross any ceiling");
        // One site past the old u16 ceiling, plus a margin so the excess is
        // unmistakable rather than a rounding.
        let sites = (u16::MAX as usize / per as usize) + 64;
        let items: Vec<CodeItem> = std::iter::repeat_n(dearest.clone(), sites).collect();
        let expect = TStates::from(per) * sites as TStates;
        assert!(
            expect > TStates::from(u16::MAX),
            "the fixture must actually cross u16::MAX or this test proves nothing; got {expect}"
        );
        assert_eq!(span_cost(&items).unwrap(), expect);
    }

    // The overflow bound is a COMPILE-time fact (`MAX_SPAN_T_STATES` is computed
    // in `TStates` and const arithmetic overflow is a build error). This checks
    // the bound is the one documented — the product of the widest single cost and
    // the most instructions a `&[CodeItem]` can hold — so a later edit that
    // loosens the derivation is loud here rather than silently unproving the type.
    #[test]
    fn max_span_bound_is_derived_from_the_two_structural_limits() {
        let widest_cost = TStates::from(u16::MAX);
        let most_items =
            isize::MAX as TStates / std::mem::size_of::<CodeItem>() as TStates;
        assert!(most_items > 0, "CodeItem must be a sized non-ZST for the bound to hold");
        assert_eq!(MAX_SPAN_T_STATES, most_items * widest_cost);
        assert!(
            MAX_SPAN_T_STATES > TStates::from(u16::MAX),
            "a bound at or below u16::MAX would mean the old ceiling was never escaped"
        );
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

    // ---- the newly priced groups, on cases whose NEIGHBOURS DIFFER ------------
    //
    // Most Z80 instructions cost 4, 7, 8 or 11 T, so a transposed or copy-pasted
    // table entry agrees with the right one by coincidence more often than not.
    // Every assertion below is chosen so the value it checks is NOT shared with
    // the form a plausible mistake would have produced, and each test says which
    // mistake it is separating from.

    fn hl() -> CodeOperand {
        CodeOperand::Z80Pair(crate::value::Z80Pair::Hl)
    }
    fn bc() -> CodeOperand {
        CodeOperand::Z80Pair(crate::value::Z80Pair::Bc)
    }
    fn ix() -> CodeOperand {
        CodeOperand::Z80Pair(crate::value::Z80Pair::Ix)
    }
    fn ixd() -> CodeOperand {
        CodeOperand::Z80Indexed { reg: crate::value::Z80Index::Ix, disp: 0 }
    }
    fn mem() -> CodeOperand {
        CodeOperand::Z80Mem { addr: 0x40FE }
    }

    // `inc` answers FOUR different numbers depending only on its operand. A table
    // that treated a mnemonic as an instruction would give one of them four times.
    #[test]
    fn inc_costs_four_different_things() {
        assert_eq!(instr_cost("inc", &[c()]), Cost::Fixed(4));
        assert_eq!(instr_cost("inc", &[hl()]), Cost::Fixed(6));
        assert_eq!(instr_cost("inc", &[CodeOperand::Z80IndHl]), Cost::Fixed(11));
        assert_eq!(instr_cost("inc", &[ixd()]), Cost::Fixed(23));
        assert_eq!(instr_cost("inc", &[ix()]), Cost::Fixed(10));
    }

    // The 16-bit adds separate on the ED prefix, which is worth 4 T. `add hl,ss`
    // is the ONLY unprefixed one; copying its 11 across the family would be
    // wrong three times and the arithmetic would look consistent.
    #[test]
    fn sixteen_bit_adds_separate_on_the_prefix() {
        assert_eq!(instr_cost("add", &[hl(), bc()]), Cost::Fixed(11));
        assert_eq!(instr_cost("adc", &[hl(), bc()]), Cost::Fixed(15));
        assert_eq!(instr_cost("sbc", &[hl(), bc()]), Cost::Fixed(15));
        assert_eq!(instr_cost("add", &[ix(), bc()]), Cost::Fixed(15));
        // …and the 8-bit `add a,r` is a different instruction with the same
        // mnemonic, at 4. The operand shape is the whole distinction.
        assert_eq!(instr_cost("add", &[a(), c()]), Cost::Fixed(4));
    }

    // push and pop are the one stack pair whose costs are NOT equal: 11 against
    // 10. A table that assumed symmetry passes every other check.
    #[test]
    fn push_costs_one_more_than_pop() {
        assert_eq!(instr_cost("push", &[bc()]), Cost::Fixed(11));
        assert_eq!(instr_cost("pop", &[bc()]), Cost::Fixed(10));
        assert_eq!(instr_cost("push", &[ix()]), Cost::Fixed(15));
        assert_eq!(instr_cost("pop", &[ix()]), Cost::Fixed(14));
    }

    // `hl` is the only pair with an UNPREFIXED (nn) load/store, so it is 16 where
    // every other pair is 20. That asymmetry is the encoder's reason for never
    // emitting ED 63 / ED 6B, and it has to survive into the pricing.
    #[test]
    fn hl_is_the_cheap_pair_through_memory() {
        assert_eq!(instr_cost("ld", &[hl(), mem()]), Cost::Fixed(16));
        assert_eq!(instr_cost("ld", &[bc(), mem()]), Cost::Fixed(20));
        assert_eq!(instr_cost("ld", &[ix(), mem()]), Cost::Fixed(20));
        assert_eq!(instr_cost("ld", &[mem(), hl()]), Cost::Fixed(16));
        assert_eq!(instr_cost("ld", &[mem(), bc()]), Cost::Fixed(20));
        // The 8-bit accumulator form through the same `(nn)` is 13 — a third
        // number, so a wrong arm here cannot land on a right one.
        assert_eq!(instr_cost("ld", &[a(), mem()]), Cost::Fixed(13));
    }

    // `bit` only READS; `set`/`res` write the byte back. The three agree on the
    // register column (8 T) and diverge everywhere else, so a test that checked
    // only `bit 0,b` would clear a table that priced all three alike.
    #[test]
    fn bit_is_cheaper_than_set_because_it_writes_nothing() {
        let n = CodeOperand::Z80Bit(0);
        assert_eq!(instr_cost("bit", &[n.clone(), c()]), Cost::Fixed(8));
        assert_eq!(instr_cost("set", &[n.clone(), c()]), Cost::Fixed(8));
        assert_eq!(instr_cost("bit", &[n.clone(), CodeOperand::Z80IndHl]), Cost::Fixed(12));
        assert_eq!(instr_cost("set", &[n.clone(), CodeOperand::Z80IndHl]), Cost::Fixed(15));
        assert_eq!(instr_cost("res", &[n.clone(), CodeOperand::Z80IndHl]), Cost::Fixed(15));
        assert_eq!(instr_cost("bit", &[n.clone(), ixd()]), Cost::Fixed(20));
        assert_eq!(instr_cost("set", &[n.clone(), ixd()]), Cost::Fixed(23));
        // A folded `Imm` in the bit slot is the same instruction as `Z80Bit`;
        // lowering accepts both spellings, so the table must not price only one.
        assert_eq!(instr_cost("bit", &[CodeOperand::Imm(0), CodeOperand::Z80IndHl]), Cost::Fixed(12));
    }

    // The CB rotate through `(hl)` is 15, its register form 8, and the
    // single-byte accumulator rotate that shares the name-shape is 4. Three
    // numbers, none of them reachable from another by a plausible slip.
    #[test]
    fn cb_rotates_are_not_the_accumulator_rotates() {
        assert_eq!(instr_cost("rlc", &[c()]), Cost::Fixed(8));
        assert_eq!(instr_cost("rlc", &[CodeOperand::Z80IndHl]), Cost::Fixed(15));
        assert_eq!(instr_cost("rlca", &[]), Cost::Fixed(4));
        assert_eq!(instr_cost("rr", &[c()]), Cost::Fixed(8));
        assert_eq!(instr_cost("rra", &[]), Cost::Fixed(4));
        // `rlc (ix+d)` is 23 T on the machine and is deliberately UNPRICED: the
        // encoder refuses the DDCB shift column, so there is no such instruction
        // to cost here.
        assert_eq!(instr_cost("rlc", &[ixd()]), Cost::Unknown);
    }

    // The block grid: the single-step members are flat 16, the repeating ones
    // carry a second number. Asserting the whole grid, not one member, is what
    // catches a family priced by copying one row.
    #[test]
    fn the_block_grid_splits_into_stepping_and_repeating() {
        for m in ["ldi", "ldd", "cpi", "cpd", "ini", "ind", "outi", "outd"] {
            assert_eq!(instr_cost(m, &[]), Cost::Fixed(16), "`{m}` steps once");
        }
        for m in ["ldir", "lddr", "cpir", "cpdr", "inir", "indr", "otir", "otdr"] {
            assert_eq!(
                instr_cost(m, &[]),
                Cost::Split { taken: 21, not_taken: 16 },
                "`{m}` repeats"
            );
        }
        // And the repeat is a REFUSAL in a straight-line span, because the true
        // cost is 16 + 21*(BC-1) and BC is a run-time value.
        let items = vec![instr("ldir", vec![])];
        assert!(matches!(span_cost(&items), Err(CycleBail::AmbiguousBranch { .. })));
    }

    // The interrupt-register loads are 9, not the 8 an ED pair usually costs and
    // not the 4 of `ld a,b`. Both neighbours are asserted beside them.
    #[test]
    fn the_i_and_r_loads_are_nine() {
        assert_eq!(instr_cost("ld", &[a(), CodeOperand::Z80RegI]), Cost::Fixed(9));
        assert_eq!(instr_cost("ld", &[CodeOperand::Z80RegR, a()]), Cost::Fixed(9));
        assert_eq!(instr_cost("ld", &[a(), c()]), Cost::Fixed(4));
        assert_eq!(instr_cost("neg", &[]), Cost::Fixed(8));
    }

    // The direct port is 11 and the memory load through the same `(nn)` spelling
    // is 13. These two differ ONLY by mnemonic, which is exactly the confusion a
    // shared operand shape invites.
    #[test]
    fn the_direct_port_is_not_a_memory_load() {
        let port = CodeOperand::Z80Mem { addr: 0x00FE };
        assert_eq!(instr_cost("in", &[a(), port.clone()]), Cost::Fixed(11));
        assert_eq!(instr_cost("out", &[port.clone(), a()]), Cost::Fixed(11));
        assert_eq!(instr_cost("ld", &[a(), port]), Cost::Fixed(13));
    }

    // `ex (sp),hl` moves four bytes and is 19; the two register renames that
    // share the mnemonic are 4. `ld sp,hl` is 6 and shares the operand pair with
    // `ex (sp),hl` under one of the two `(sp)` spellings, so both are asserted.
    #[test]
    fn the_exchanges_are_not_all_renames() {
        let de = CodeOperand::Z80Pair(crate::value::Z80Pair::De);
        let af = CodeOperand::Z80Pair(crate::value::Z80Pair::Af);
        let sp = CodeOperand::Z80Pair(crate::value::Z80Pair::Sp);
        assert_eq!(instr_cost("ex", &[de, hl()]), Cost::Fixed(4));
        assert_eq!(instr_cost("ex", &[af, CodeOperand::Z80AfShadow]), Cost::Fixed(4));
        assert_eq!(instr_cost("ex", &[CodeOperand::Z80IndSp, hl()]), Cost::Fixed(19));
        assert_eq!(instr_cost("ex", &[sp.clone(), hl()]), Cost::Fixed(19));
        assert_eq!(instr_cost("ld", &[sp, hl()]), Cost::Fixed(6));
    }

    /// THE COVERAGE GUARD: what the encoder can emit, the table must be able to
    /// price.
    ///
    /// The population is NOT a list of mnemonics kept up by hand. A hand list's
    /// failure mode is "green because nobody maintained it", which is precisely
    /// the failure this guard exists to prevent — the twenty-one mnemonics that
    /// prompted it became assemblable and stayed unpriced for exactly that
    /// reason. Two mechanisms replace the list:
    ///
    ///   * the [`mnemonics!`] macro writes the vocabulary ONCE and expands it
    ///     into both a lookup and an exhaustive `match` on the encoder's own
    ///     `Mnemonic` enum, so a variant added later fails to COMPILE here;
    ///   * which FORMS exist is asked of `z80::encode` itself, by offering every
    ///     mnemonic every operand shape in a fixed pool and keeping the ones it
    ///     accepts. Nothing declares what is encodable; the encoder answers.
    mod encoder_coverage {
        use super::super::{instr_cost, Cost};
        use crate::value::{CodeOperand, Z80Cond, Z80Index, Z80Pair, Z80Reg8};
        use sigil_backend_z80::z80::{
            self, Cond as IsaCond, IndexReg as IsaIndex, Instruction, Mnemonic, Operand as IsaOp,
            Reg16 as IsaPair, Reg8 as IsaReg8,
        };

        /// One list, two consumers: an exhaustive `match` (the compile-time
        /// completeness proof) and the vector the test walks (so the walked
        /// population cannot be a subset of the proved one).
        macro_rules! mnemonics {
            ($($v:ident => $s:expr),* $(,)?) => {
                /// The spelling the cost table keys on. Exhaustive over the
                /// encoder's enum by construction.
                #[allow(dead_code)]
                fn spelling(m: Mnemonic) -> &'static str {
                    match m { $(Mnemonic::$v => $s),* }
                }
                fn all_mnemonics() -> Vec<(Mnemonic, &'static str)> {
                    vec![$((Mnemonic::$v, $s)),*]
                }
            };
        }

        mnemonics! {
            Nop => "nop", Ld => "ld", Add => "add", Adc => "adc", Sub => "sub",
            Sbc => "sbc", And => "and", Or => "or", Xor => "xor", Cp => "cp",
            Inc => "inc", Dec => "dec", Push => "push", Pop => "pop", Ex => "ex",
            Exx => "exx", Ret => "ret", Jr => "jr", Jp => "jp", Call => "call",
            Djnz => "djnz", Rrca => "rrca", Scf => "scf", Ei => "ei", Di => "di",
            Bit => "bit", Res => "res", Set => "set", Srl => "srl", Rr => "rr",
            Sla => "sla", Rlc => "rlc", Rrc => "rrc", Rl => "rl", Sra => "sra",
            Neg => "neg", Im => "im",
            Ldi => "ldi", Ldir => "ldir", Ldd => "ldd", Lddr => "lddr",
            Cpi => "cpi", Cpd => "cpd", Cpir => "cpir", Cpdr => "cpdr",
            Ini => "ini", Ind => "ind", Inir => "inir", Indr => "indr",
            Outi => "outi", Outd => "outd", Otir => "otir", Otdr => "otdr",
            In => "in", Out => "out", Reti => "reti", Retn => "retn",
            Rrd => "rrd", Rld => "rld",
            // The two `ld a,i` / `ld a,r` marker variants. `encode` has no arm
            // for either — those bytes come from the ordinary `Ld` variant with
            // a `RegI`/`RegR` operand — so the pool below finds them
            // unencodable and they place no demand on the table.
            LdIA => "ld", LdRA => "ld",
            Rlca => "rlca", Rla => "rla", Rra => "rra", Daa => "daa",
            Cpl => "cpl", Ccf => "ccf", Halt => "halt", Rst => "rst",
        }

        /// The emp-side image of one ISA operand, or `None` when emp has no
        /// spelling for it. Exhaustive on the ISA's own operand enum, so a new
        /// operand form fails to compile here rather than silently dropping
        /// every instruction that uses it out of the demand.
        fn emp_image(op: &IsaOp) -> Option<CodeOperand> {
            Some(match op {
                IsaOp::Reg(r) => CodeOperand::Z80Reg8(match r {
                    IsaReg8::A => Z80Reg8::A,
                    IsaReg8::B => Z80Reg8::B,
                    IsaReg8::C => Z80Reg8::C,
                    IsaReg8::D => Z80Reg8::D,
                    IsaReg8::E => Z80Reg8::E,
                    IsaReg8::H => Z80Reg8::H,
                    IsaReg8::L => Z80Reg8::L,
                }),
                IsaOp::Pair(p) => CodeOperand::Z80Pair(match p {
                    IsaPair::Bc => Z80Pair::Bc,
                    IsaPair::De => Z80Pair::De,
                    IsaPair::Hl => Z80Pair::Hl,
                    IsaPair::Sp => Z80Pair::Sp,
                    IsaPair::Af => Z80Pair::Af,
                    IsaPair::Ix => Z80Pair::Ix,
                    IsaPair::Iy => Z80Pair::Iy,
                }),
                IsaOp::IndHl => CodeOperand::Z80IndHl,
                IsaOp::IndBc => CodeOperand::Z80IndBc,
                IsaOp::IndDe => CodeOperand::Z80IndDe,
                IsaOp::Indexed { reg, disp } => CodeOperand::Z80Indexed {
                    reg: match reg {
                        IsaIndex::Ix => Z80Index::Ix,
                        IsaIndex::Iy => Z80Index::Iy,
                    },
                    disp: i128::from(*disp),
                },
                IsaOp::Imm8(n) => CodeOperand::Imm(i128::from(*n)),
                IsaOp::Imm16(n) => CodeOperand::Imm(i128::from(*n)),
                IsaOp::Mem(a) => CodeOperand::Z80Mem { addr: i128::from(*a) },
                IsaOp::Cc(c) => CodeOperand::Z80Cc(match c {
                    IsaCond::Nz => Z80Cond::Nz,
                    IsaCond::Z => Z80Cond::Z,
                    IsaCond::Nc => Z80Cond::Nc,
                    IsaCond::C => Z80Cond::C,
                    IsaCond::Po => Z80Cond::Po,
                    IsaCond::Pe => Z80Cond::Pe,
                    IsaCond::P => Z80Cond::P,
                    IsaCond::M => Z80Cond::M,
                }),
                IsaOp::Bit(b) => CodeOperand::Z80Bit(*b),
                // A resolved displacement on the ISA side; on the emp side a
                // branch target is always the SYMBOL, resolved at link.
                IsaOp::Rel(_) => CodeOperand::Sym(".target".to_string()),
                IsaOp::AfShadow => CodeOperand::Z80AfShadow,
                IsaOp::RegI => CodeOperand::Z80RegI,
                IsaOp::RegR => CodeOperand::Z80RegR,
                // `(c)`. There is NO `CodeOperand` for the C-addressed port, so
                // no emp instruction can carry this shape to the cost table, and
                // the table cannot have an arm that matches it. Forms containing
                // it are skipped, and the test asserts that this is the ONLY
                // reason anything is skipped.
                IsaOp::IndC => return None,
            })
        }

        /// The operand shapes offered to every mnemonic. Over-covering is the
        /// safe direction here: a shape the encoder rejects costs one `Err` and
        /// nothing else, while a shape left out silently removes real forms from
        /// the demand — so the immediates are chosen to be ACCEPTABLE wherever a
        /// narrow range exists (`rst $18`, `im 1`, a port that fits in 8 bits)
        /// rather than merely typical.
        fn shape_pool() -> Vec<Vec<IsaOp>> {
            use IsaOp::*;
            let a = Reg(IsaReg8::A);
            let b = Reg(IsaReg8::B);
            let idx = Indexed { reg: IsaIndex::Ix, disp: 5 };
            let idy = Indexed { reg: IsaIndex::Iy, disp: -1 };
            let big = Mem(0x4000);
            let port = Mem(0x00FE);
            vec![
                vec![],
                // one operand
                vec![a],
                vec![b],
                vec![IndHl],
                vec![idx],
                vec![Imm8(0x12)],
                vec![Imm8(0x00)],
                vec![Imm8(0x01)],
                vec![Imm8(0x02)],
                vec![Imm8(0x18)],
                vec![Imm16(0x1234)],
                vec![big],
                vec![Pair(IsaPair::Bc)],
                vec![Pair(IsaPair::De)],
                vec![Pair(IsaPair::Hl)],
                vec![Pair(IsaPair::Sp)],
                vec![Pair(IsaPair::Af)],
                vec![Pair(IsaPair::Ix)],
                vec![Pair(IsaPair::Iy)],
                vec![Cc(IsaCond::Nz)],
                vec![Cc(IsaCond::Po)],
                vec![Rel(2)],
                // two operands — 8-bit
                vec![a, b],
                vec![b, a],
                vec![a, Imm8(0x12)],
                vec![b, Imm8(0x12)],
                vec![a, IndHl],
                vec![b, IndHl],
                vec![IndHl, a],
                vec![IndHl, Imm8(0x12)],
                vec![a, IndBc],
                vec![IndBc, a],
                vec![a, IndDe],
                vec![IndDe, a],
                vec![a, big],
                vec![big, a],
                vec![a, port],
                vec![port, a],
                vec![a, idx],
                vec![idx, a],
                vec![idx, Imm8(0x12)],
                vec![a, idy],
                vec![idy, a],
                vec![a, RegI],
                vec![a, RegR],
                vec![RegI, a],
                vec![RegR, a],
                vec![a, IndC],
                vec![b, IndC],
                vec![IndC, a],
                vec![IndC, b],
                // two operands — 16-bit
                vec![Pair(IsaPair::Hl), Imm16(0x1234)],
                vec![Pair(IsaPair::Bc), Imm16(0x1234)],
                vec![Pair(IsaPair::Sp), Imm16(0x1234)],
                vec![Pair(IsaPair::Ix), Imm16(0x1234)],
                vec![Pair(IsaPair::Hl), big],
                vec![Pair(IsaPair::Bc), big],
                vec![Pair(IsaPair::Sp), big],
                vec![Pair(IsaPair::Ix), big],
                vec![big, Pair(IsaPair::Hl)],
                vec![big, Pair(IsaPair::Bc)],
                vec![big, Pair(IsaPair::Sp)],
                vec![big, Pair(IsaPair::Ix)],
                vec![Pair(IsaPair::Hl), Pair(IsaPair::Bc)],
                vec![Pair(IsaPair::Hl), Pair(IsaPair::Hl)],
                vec![Pair(IsaPair::Hl), Pair(IsaPair::Sp)],
                vec![Pair(IsaPair::Ix), Pair(IsaPair::Bc)],
                vec![Pair(IsaPair::Ix), Pair(IsaPair::Ix)],
                vec![Pair(IsaPair::De), Pair(IsaPair::Hl)],
                vec![Pair(IsaPair::Sp), Pair(IsaPair::Hl)],
                vec![Pair(IsaPair::Af), AfShadow],
                // control transfer
                vec![Cc(IsaCond::Nz), Imm16(0x1234)],
                vec![Cc(IsaCond::Po), Imm16(0x1234)],
                vec![Cc(IsaCond::Nz), Rel(2)],
                vec![Cc(IsaCond::Po), Rel(2)],
                // CB group
                vec![Bit(0), a],
                vec![Bit(7), b],
                vec![Bit(0), IndHl],
                vec![Bit(7), idx],
                vec![Bit(0), idy],
            ]
        }

        #[test]
        fn every_encodable_form_is_priced() {
            let pool = shape_pool();
            // GUARD 1: the pool itself. A pool that shrank to nothing would make
            // every later loop examine no forms and pass.
            assert!(
                pool.len() >= 40,
                "the operand-shape pool has {} entries; a shrunken pool makes this \
                 guard pass by examining almost nothing",
                pool.len()
            );

            // Ask the ENCODER which (mnemonic, shape) pairs exist.
            let mut encodable: Vec<(&'static str, Vec<IsaOp>)> = Vec::new();
            let mut unencodable: Vec<Mnemonic> = Vec::new();
            for (m, name) in all_mnemonics() {
                let mut any = false;
                for ops in &pool {
                    let inst = Instruction { mnemonic: m, ops: ops.clone() };
                    if z80::encode(&inst).is_ok() {
                        any = true;
                        encodable.push((name, ops.clone()));
                    }
                }
                if !any {
                    unencodable.push(m);
                }
            }

            // GUARD 2: the population. Non-empty is the floor, and it is not
            // enough on its own — a pool that had lost every ED or CB shape
            // would still be non-empty, so the anchors below require the
            // discovery to have reached each ENCODING PAGE: base one-byte, base
            // two-operand, CB, ED block, ED I/O, DD/FD index, and the two
            // narrow-immediate forms whose operands must be in range to encode
            // at all.
            assert!(
                !encodable.is_empty(),
                "no encodable form was discovered at all; the encoder or the pool is broken"
            );
            for anchor in ["nop", "ld", "srl", "ldir", "in", "push", "rst", "im"] {
                assert!(
                    encodable.iter().any(|(n, _)| *n == anchor),
                    "`{anchor}` encodes nothing against this pool, so the pool no longer \
                     reaches its encoding page and the demand below is short by a whole group"
                );
            }

            // Require the TABLE to cover what the encoder produced.
            let mut checked = 0usize;
            let mut skipped = 0usize;
            let mut unpriced: Vec<String> = Vec::new();
            for (name, ops) in &encodable {
                match ops.iter().map(emp_image).collect::<Option<Vec<CodeOperand>>>() {
                    Some(emp) => {
                        checked += 1;
                        if instr_cost(name, &emp) == Cost::Unknown {
                            unpriced.push(format!("{name} {ops:?}"));
                        }
                    }
                    None => {
                        // The ONLY admissible reason to skip: the form carries
                        // `(c)`, which has no emp operand at all. Anything else
                        // skipped would be the demand quietly shrinking.
                        assert!(
                            ops.contains(&IsaOp::IndC),
                            "form `{name} {ops:?}` has no emp image for a reason other than \
                             the `(c)` port; the coverage demand must not shrink silently"
                        );
                        skipped += 1;
                    }
                }
            }

            // GUARD 3: something was actually examined. An empty population and
            // an unapplied mutation both print `ok`, and this is the assertion
            // that tells them apart.
            assert!(
                checked > 0,
                "the coverage check examined NO form: {} encodable form(s) were found and \
                 all {skipped} of them were skipped for want of an emp operand image",
                encodable.len()
            );

            assert!(
                unpriced.is_empty(),
                "{} encodable Z80 form(s) resolve to Cost::Unknown, out of {checked} checked \
                 ({skipped} skipped as `(c)` forms; mnemonics with no encoding at all: \
                 {unencodable:?}):\n  {}",
                unpriced.len(),
                unpriced.join("\n  ")
            );
        }
    }
}
