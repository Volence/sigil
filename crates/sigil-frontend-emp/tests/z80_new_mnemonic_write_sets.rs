//! The clobber model for the Z80 instructions the ISA can now encode.
//!
//! `z80_writes_regs` is a match whose default arm is "writes nothing". That
//! default is the UNSAFE direction for a preserve proof: an instruction that is
//! assemblable but unlisted is claimed to preserve every register it destroys,
//! and the claim is silent. So an arm has to land in the same change as the
//! encoder that makes the mnemonic assemblable, and this file is the check that
//! it did.
//!
//! WHAT THIS DOES AND DOES NOT REACH. It drives [`z80_written_registers`] over
//! synthesised `CodeBuf`s rather than over `.emp` source, because the `.emp`
//! front end's own mnemonic table is deliberately not part of this change: the
//! `.emp` language surface is the owner's to extend. So this exercises the
//! TABLE, not the pipeline, and it says so rather than implying more coverage
//! than it has.
//!
//! Register semantics are Zilog's:
//!   * the LD block moves step hl and de and count down bc;
//!   * the CP block searches step hl and count down bc, and never touch de,
//!     which is the difference that makes them a separate arm and not a member
//!     of the LD one;
//!   * the IN and OUT block transfers step hl and decrement B ALONE, so they
//!     write b, h and l but not c;
//!   * `rrd`/`rld` rotate a nibble between the accumulator and `(hl)`: they
//!     write a, and their memory write is not a register;
//!   * `in r,(c)` and `in a,(n)` write their destination register;
//!   * `out` writes no register at all.

use sigil_frontend_emp::value::{CodeBuf, CodeItem, CodeOperand, ItemAuthor, Z80Reg8};
use sigil_frontend_emp::z80_preserves::z80_written_registers;
use sigil_span::{SourceId, Span};

fn span() -> Span {
    Span { source: SourceId(0), start: 0, end: 0 }
}

/// The register units one instruction writes, per the model, WITHOUT the flag
/// unit. Flag modelling is a separate allowlist with the opposite polarity and
/// is asserted on its own below.
fn writes(mnemonic: &str, ops: Vec<CodeOperand>) -> Vec<String> {
    let buf = CodeBuf {
        items: vec![CodeItem::Instr {
            mnemonic: mnemonic.to_string(),
            size: None,
            ops,
            span: span(),
            as_type: None,
            targets: vec![],
            author: ItemAuthor::User,
        }],
    };
    let mut v: Vec<String> =
        z80_written_registers(&buf).into_iter().filter(|u| u != "f").collect();
    v.sort();
    v
}

/// Does the model say this instruction writes the flag register?
fn writes_f(mnemonic: &str, ops: Vec<CodeOperand>) -> bool {
    let buf = CodeBuf {
        items: vec![CodeItem::Instr {
            mnemonic: mnemonic.to_string(),
            size: None,
            ops,
            span: span(),
            as_type: None,
            targets: vec![],
            author: ItemAuthor::User,
        }],
    };
    z80_written_registers(&buf).contains("f")
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

/// The LD block moves clobber the two pointers and the counter.
#[test]
fn ld_block_moves_clobber_bc_de_hl() {
    for m in ["ldi", "ldd", "ldir", "lddr"] {
        assert_eq!(writes(m, vec![]), s(&["b", "c", "d", "e", "h", "l"]), "`{m}`");
    }
}

/// The CP block searches clobber bc and hl but NOT de. Asserting the absence is
/// the whole point: an arm that lumped them in with the LD family would over-
/// report de, and one that forgot them entirely would under-report everything,
/// which is the direction that lets a false `preserves` through.
#[test]
fn cp_block_searches_clobber_bc_and_hl_but_not_de() {
    for m in ["cpi", "cpd", "cpir", "cpdr"] {
        assert_eq!(writes(m, vec![]), s(&["b", "c", "h", "l"]), "`{m}`");
    }
}

/// The IN and OUT block transfers decrement B alone, so `c` stays intact.
#[test]
fn io_block_transfers_clobber_b_and_hl_but_not_c() {
    for m in ["ini", "ind", "inir", "indr", "outi", "outd", "otir", "otdr"] {
        assert_eq!(writes(m, vec![]), s(&["b", "h", "l"]), "`{m}`");
    }
}

/// `rrd`/`rld` write the accumulator; the nibble they place in `(hl)` is a
/// memory write and no register.
#[test]
fn the_nibble_rotates_write_the_accumulator() {
    for m in ["rrd", "rld"] {
        assert_eq!(writes(m, vec![]), s(&["a"]), "`{m}`");
    }
}

/// `in` writes its destination register, over both port spellings; `out` writes
/// none. The destination is exercised on a register OTHER than `a` as well, so
/// a model that hard-coded the accumulator would fail.
#[test]
fn in_writes_its_destination_and_out_writes_none() {
    assert_eq!(writes("in", vec![CodeOperand::Z80Reg8(Z80Reg8::A)]), s(&["a"]));
    assert_eq!(writes("in", vec![CodeOperand::Z80Reg8(Z80Reg8::E)]), s(&["e"]));
    assert_eq!(writes("out", vec![CodeOperand::Z80Reg8(Z80Reg8::A)]), s(&[]));
}

/// The returns write no register.
#[test]
fn the_returns_write_nothing() {
    for m in ["reti", "retn"] {
        assert_eq!(writes(m, vec![]), s(&[]), "`{m}`");
    }
}

/// FLAGS. The block ops, the nibble rotates and `in` all write the flags; `out`
/// and the returns do not. The flag model is the COMPLEMENT of a neutral
/// allowlist, so an unlisted mnemonic reads as a writer, which is the safe
/// direction; these assertions pin that the listing did not accidentally put
/// one of them on the neutral side.
#[test]
fn the_flag_writers_and_the_flag_neutrals() {
    for m in ["ldi", "ldd", "ldir", "lddr", "cpi", "cpd", "cpir", "cpdr",
              "ini", "ind", "inir", "indr", "outi", "outd", "otir", "otdr",
              "rrd", "rld", "in"] {
        assert!(writes_f(m, vec![]), "`{m}` writes the flags");
    }
    for m in ["out", "reti", "retn"] {
        assert!(!writes_f(m, vec![]), "`{m}` leaves the flags");
    }
}

/// `ld a,i` and `ld a,r` READ `i`/`r` into the accumulator AND set S, Z and
/// P/V, so unlike every other `ld` they are flag WRITERS. The `ld` mnemonic is
/// on the flag-neutral allowlist, so this is the one form of it that has to be
/// carved back out; the write direction (`ld i,a`) stays neutral, and both are
/// asserted so the carve-out cannot be a blanket one.
#[test]
fn the_i_and_r_reads_write_flags_while_the_writes_do_not() {
    assert!(
        writes_f("ld", vec![CodeOperand::Z80Reg8(Z80Reg8::A), CodeOperand::Z80RegI]),
        "`ld a,i` sets S/Z/P-V"
    );
    assert!(
        writes_f("ld", vec![CodeOperand::Z80Reg8(Z80Reg8::A), CodeOperand::Z80RegR]),
        "`ld a,r` sets S/Z/P-V"
    );
    assert!(
        !writes_f("ld", vec![CodeOperand::Z80RegI, CodeOperand::Z80Reg8(Z80Reg8::A)]),
        "`ld i,a` leaves the flags"
    );
    assert!(
        !writes_f("ld", vec![CodeOperand::Z80Reg8(Z80Reg8::A), CodeOperand::Z80Reg8(Z80Reg8::B)]),
        "an ordinary `ld` leaves the flags"
    );
}

/// THE CYCLE TABLE, the other consumer keyed on these mnemonic strings.
///
/// It prices only the timed-region subset the driver demands, and its default
/// is `Cost::Unknown`, which is the LOUD direction: a span or budget walk
/// containing an unpriced op bails with `[cycles.unknown-op]` rather than
/// guessing a number. Every instruction this coverage work made assemblable
/// lands there, `ldir` included, and that is deliberately left alone: a wrong
/// T-state count is worse than an absent one, and pricing them belongs to
/// whichever timed region first needs them, with its own derivation.
///
/// This is a PIN, not an aspiration. If a later change prices one of these, the
/// test goes red and whoever priced it has to say so out loud.
///
/// TWO of them were ALREADY priced before they could be assembled: `reti` and
/// `retn` sat in this table at 14 T with no encoder arm and no name mapping, the
/// same half-built shape the eight one-byte primitives were found in. The
/// encoder has now caught up with the price rather than the other way round, so
/// they are asserted at their existing value instead of being pushed to Unknown.
#[test]
fn the_new_mnemonics_are_unpriced_in_the_cycle_table() {
    use sigil_frontend_emp::z80_cycles::{instr_cost, Cost};
    for m in ["ldi", "ldd", "ldir", "lddr", "cpi", "cpd", "cpir", "cpdr",
              "ini", "ind", "inir", "indr", "outi", "outd", "otir", "otdr",
              "rrd", "rld", "in", "out"] {
        assert_eq!(instr_cost(m, &[]), Cost::Unknown, "`{m}` must stay unpriced");
    }
    assert_eq!(instr_cost("reti", &[]), Cost::Fixed(14));
    assert_eq!(instr_cost("retn", &[]), Cost::Fixed(14));
    // The positive control: an op the table DOES price, so a broken lookup that
    // answered Unknown for everything could not pass the loop above.
    assert_eq!(instr_cost("nop", &[]), Cost::Fixed(4));
}
