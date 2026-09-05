//! Z80 instructions the front end had no name for, driven end to end.
//!
//! Every expectation below is the reference asl's own answer (asl md5
//! 61e672562465725a8c102288a7da9098, the s1disasm build; one asl invocation per
//! form with its exit status checked, so no form's bytes can be another form's
//! substituted residue). None is copied from a neighbouring value, which for
//! this instruction group is the whole difficulty: the ED block ops occupy a
//! 4x4 grid whose members differ by a single bit, so a table that swapped a row
//! for a column still answers with legal, plausible bytes.
//!
//! That is why the assertions come in FAMILIES rather than one exemplar each:
//!
//!   * `ldi`/`cpi`/`ini`/`outi` differ only in bits 1..0 of the sub-opcode;
//!   * `ldi`/`ldd` differ only in bit 3, `ldi`/`ldir` only in bit 4;
//!   * `in r,(c)`/`out (c),r` differ only in bit 0, with the register in bits
//!     5..3 -- and register `a` is code 7, not 0, so a family tested only on
//!     `b` leaves the register shift completely unexercised;
//!   * `im 0`/`im 1`/`im 2` are ED 46/56/5E, which is NOT `0x46 | mode << 4`:
//!     the obvious arithmetic yields 46/56/66, and ED 66 is a real (undocumented)
//!     `im 0` alias, so a wrong table there is invisible to a spot check;
//!   * `ld i,a` and `ld a,i` differ only in bit 4, the classic direction slip.

use sigil_frontend_as::{assemble, Options};
use sigil_ir::SymbolTable;

/// Assemble one Z80 snippet through parse -> lower -> link -> flatten.
fn asm(snippet: &str) -> Vec<u8> {
    let src = format!("        cpu z80\n        phase 0\n        {snippet}\n");
    let module = assemble(&src, &Options::default())
        .unwrap_or_else(|d| panic!("assemble `{snippet}` failed: {d:?}"));
    let linked = sigil_link::link(&module.sections, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link `{snippet}` failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
}

/// Assert a whole family at once and require its members to be PAIRWISE
/// DISTINCT. The second half is the part a single-form assertion cannot do: a
/// table that collapsed two neighbours onto one opcode passes every individual
/// check that happens to look at the surviving one.
fn family(cases: &[(&str, &[u8])]) {
    let mut wrong = Vec::new();
    for (snippet, want) in cases {
        let got = asm(snippet);
        if got != *want {
            wrong.push(format!("`{snippet}`: want {want:02X?}, got {got:02X?}"));
        }
    }
    assert!(wrong.is_empty(), "{} form(s) diverged:\n{}", wrong.len(), wrong.join("\n"));

    for (i, (sa, _)) in cases.iter().enumerate() {
        for (sb, _) in cases.iter().skip(i + 1) {
            assert_ne!(asm(sa), asm(sb), "`{sa}` and `{sb}` encode identically");
        }
    }
}

/// `ldi` -- the single-step block move, 17 live sites in Sonic 2's sound driver.
///
/// It is asserted against its three single-bit neighbours at once. `ldir` was
/// already encoded and is the one this instruction is most often confused with
/// in prose ("the block copy"): they are different instructions, ED A0 and
/// ED B0, and the pair is here so that confusion cannot survive in the table.
#[test]
fn ldi_and_its_single_bit_neighbours() {
    family(&[
        ("ldi", &[0xED, 0xA0]),
        ("ldir", &[0xED, 0xB0]),
    ]);
}

/// The whole ED block-op grid at once: four families (LD/CP/IN/OUT) crossed
/// with four steppings (step-up, step-down, repeat-up, repeat-down).
///
/// One assertion per instruction would not discriminate here. The grid is
/// `ED A0` plus `family | direction << 3 | repeat << 4`, so ANY transposition
/// of the two axes, and any off-by-one in either, still lands on a legal
/// instruction from the same grid. Requiring all sixteen simultaneously, and
/// requiring them pairwise distinct, is what makes a transposed table fail.
#[test]
fn the_ed_block_op_grid() {
    family(&[
        ("ldi", &[0xED, 0xA0]), ("cpi", &[0xED, 0xA1]),
        ("ini", &[0xED, 0xA2]), ("outi", &[0xED, 0xA3]),
        ("ldd", &[0xED, 0xA8]), ("cpd", &[0xED, 0xA9]),
        ("ind", &[0xED, 0xAA]), ("outd", &[0xED, 0xAB]),
        ("ldir", &[0xED, 0xB0]), ("cpir", &[0xED, 0xB1]),
        ("inir", &[0xED, 0xB2]), ("otir", &[0xED, 0xB3]),
        ("lddr", &[0xED, 0xB8]), ("cpdr", &[0xED, 0xB9]),
        ("indr", &[0xED, 0xBA]), ("otdr", &[0xED, 0xBB]),
    ]);
}

/// `reti`/`retn` and the two nibble rotates.
///
/// `reti` (ED 4D) and `retn` (ED 45) differ in one bit, and ED 45's column is
/// full of undocumented `retn` aliases, so a table that reached for the
/// "obvious" neighbour would still return a working return-from-interrupt.
/// `rrd`/`rld` (ED 67/6F) likewise differ in one bit and do OPPOSITE things to
/// the nibbles they rotate.
#[test]
fn the_returns_and_the_nibble_rotates() {
    family(&[
        ("retn", &[0xED, 0x45]),
        ("reti", &[0xED, 0x4D]),
        ("rrd", &[0xED, 0x67]),
        ("rld", &[0xED, 0x6F]),
    ]);
}

/// `in r,(c)` and `out (c),r` over every register.
///
/// Direction lives in bit 0 and the register in bits 5..3, so a single-register
/// check leaves both the shift and the direction unproven. Register `a` is code
/// SEVEN, not zero, which is exactly the trap: a family exercised only on `b`
/// (code 0) passes with the register field entirely ignored. All seven appear
/// below, and `f`/`(c)` (ED 70) and `out (c),0` (ED 71) are deliberately absent
/// because the reference asl refuses both spellings, so there is no asl answer
/// to hold them to.
#[test]
fn the_c_port_family_over_every_register() {
    family(&[
        ("in b,(c)", &[0xED, 0x40]), ("out (c),b", &[0xED, 0x41]),
        ("in c,(c)", &[0xED, 0x48]), ("out (c),c", &[0xED, 0x49]),
        ("in d,(c)", &[0xED, 0x50]), ("out (c),d", &[0xED, 0x51]),
        ("in e,(c)", &[0xED, 0x58]), ("out (c),e", &[0xED, 0x59]),
        ("in h,(c)", &[0xED, 0x60]), ("out (c),h", &[0xED, 0x61]),
        ("in l,(c)", &[0xED, 0x68]), ("out (c),l", &[0xED, 0x69]),
        ("in a,(c)", &[0xED, 0x78]), ("out (c),a", &[0xED, 0x79]),
    ]);
}

/// `in a,(n)` and `out (n),a`, the unprefixed direct-port pair.
///
/// The port literal is `0FEh` and not a single digit on purpose: a value whose
/// hex and decimal readings coincide cannot tell a radix error from a correct
/// one, and a value of zero cannot tell an emitted operand from a dropped one.
#[test]
fn the_direct_port_pair() {
    family(&[
        ("in a,(0FEh)", &[0xDB, 0xFE]),
        ("out (0FEh),a", &[0xD3, 0xFE]),
    ]);
}

/// A port above 255 has no encoding and must be REFUSED, never masked into a
/// different legal port. The reference asl answers `range overflow` and exits
/// 2 for `in a,(100h)` and `out (100h),a`; this front end must not be the more
/// permissive of the two.
///
/// Each refusal is paired with the SAME form at an in-range port, asserted to
/// assemble. Without that pair the test passes while `in` is not a mnemonic at
/// all, which is a refusal for a reason that has nothing to do with the range.
#[test]
fn a_port_above_255_is_refused() {
    for (bad, good) in
        [("in a,(100h)", "in a,(0FFh)"),
         ("out (100h),a", "out (0FFh),a"),
         ("in a,(1FFh)", "in a,(0FEh)")]
    {
        let src = format!("        cpu z80\n        phase 0\n        {good}\n");
        assemble(&src, &Options::default())
            .unwrap_or_else(|d| panic!("`{good}` is in range and must assemble: {d:?}"));
        let src = format!("        cpu z80\n        phase 0\n        {bad}\n");
        assert!(
            assemble(&src, &Options::default()).is_err(),
            "`{bad}` must be refused: asl gives range overflow"
        );
    }
}

/// The ED 16-bit accumulator arithmetic, both operations over all four pairs.
///
/// `sbc hl,rr` and `adc hl,rr` differ in bit 3 and the pair rides bits 5..4, so
/// the same two-axis confound as the block grid applies: swap the axes and
/// every result is still a legal 16-bit add-or-subtract on some pair. Note
/// there is no `add hl,rr` here, because that one is unprefixed base-page 09
/// and was already encoded; its presence in the same mnemonic family is why the
/// `adc`/`sbc` arms have to be reached by the PAIR operand shape.
#[test]
fn the_ed_sixteen_bit_arithmetic() {
    family(&[
        ("sbc hl,bc", &[0xED, 0x42]), ("adc hl,bc", &[0xED, 0x4A]),
        ("sbc hl,de", &[0xED, 0x52]), ("adc hl,de", &[0xED, 0x5A]),
        ("sbc hl,hl", &[0xED, 0x62]), ("adc hl,hl", &[0xED, 0x6A]),
        ("sbc hl,sp", &[0xED, 0x72]), ("adc hl,sp", &[0xED, 0x7A]),
    ]);
}

/// All three interrupt modes.
///
/// This is the family that most needs its literal answers: the modes encode as
/// ED 46 / ED 56 / ED 5E, which is NOT `0x46 | mode << 4`. That arithmetic
/// gives 46 / 56 / 66, and ED 66 is a genuine undocumented `im 0` alias, so a
/// table built from the pattern rather than from asl produces bytes a
/// disassembler will happily read back as an interrupt-mode instruction. Only
/// mode 1 was encoded before; modes 0 and 2 were refused.
#[test]
fn every_interrupt_mode() {
    family(&[
        ("im 0", &[0xED, 0x46]),
        ("im 1", &[0xED, 0x56]),
        ("im 2", &[0xED, 0x5E]),
    ]);
}

/// A mode outside 0..2 stays refused. asl answers `instruction not supported
/// on Z80` for `im 3`.
///
/// `im 2` is asserted to assemble in the same test. `im 3` was refused before
/// this coverage work too, back when EVERY mode but 1 was refused, so a lone
/// refusal assertion here would go on passing whether or not the boundary it
/// names is the boundary being enforced.
#[test]
fn an_out_of_range_interrupt_mode_is_refused() {
    let ok = "        cpu z80\n        phase 0\n        im 2\n";
    assemble(ok, &Options::default()).expect("`im 2` is a real mode and must assemble");
    let src = "        cpu z80\n        phase 0\n        im 3\n";
    assert!(assemble(src, &Options::default()).is_err(), "`im 3` must be refused");
}

/// `i` and `r`, both directions.
///
/// The write forms were already encoded and the READ forms were not, which is
/// the shape a direction slip takes: `ld i,a` is ED 47 and `ld a,i` is ED 57,
/// one bit apart, and getting them backwards silently swaps a load for a store.
/// Both directions of both registers are asserted together.
#[test]
fn the_i_and_r_registers_in_both_directions() {
    family(&[
        ("ld i,a", &[0xED, 0x47]),
        ("ld r,a", &[0xED, 0x4F]),
        ("ld a,i", &[0xED, 0x57]),
        ("ld a,r", &[0xED, 0x5F]),
    ]);
}
