//! The eight Z80 primitives that lived in the ANALYZERS with no encoder arm
//! (lens sweep, seat CGb, finding S5).
//!
//! `rst` was the known instance — it blocked ~116 B of sound-driver size reclaim.
//! It had seven siblings in the same half-built state: `rlca`, `rla`, `rra`,
//! `daa`, `cpl`, `ccf`, `halt`. All appear in `z80_preserves`'s write sets and
//! `flag_check`'s F-writer table; none had a `Mnemonic` variant, a name mapping,
//! or an `encode()` arm, so writing one was a loud parse error — never wrong
//! bytes, but never usable either. A size-reclaim pass reaches for exactly these:
//! `rla` is 4 T in 1 byte against the CB-prefixed `rl a` at 8 T in 2.
//!
//! ORACLE. `asl` left the pipeline at the flip, so these bytes could not be
//! minted the way the rest of the corpus was, and there is no Z80 assembler and no
//! Capstone Z80 mode on this machine. They are instead cross-checked against the
//! sibling `oracle-next` Z80 CPU core — an INDEPENDENT implementation, by other
//! hands, verified against the external SingleStepTests suite. Every opcode and
//! T-state below was read out of its dispatch:
//!
//!   0x07 RLCA · 0x17 RLA · 0x1F RRA · 0x27 DAA · 0x2F CPL · 0x3F CCF   (4 T each)
//!   0x76 HALT                                                          (4 T)
//!   0xC7|0xCF|0xD7|0xDF|0xE7|0xEF|0xF7|0xFF RST, vector = opcode & 0x38 (11 T)
//!
//! The pre-existing `rrca` = 0x0F and `scf` = 0x37 arms agree with that core too,
//! which is what makes it a shared oracle rather than an assumption.
use sigil_isa::z80::{encode, Instruction, Mnemonic, Operand};

fn enc(m: Mnemonic, ops: Vec<Operand>) -> Vec<u8> {
    encode(&Instruction { mnemonic: m, ops }).expect("must encode")
}

#[test]
fn the_one_byte_accumulator_and_flag_primitives_encode() {
    assert_eq!(enc(Mnemonic::Rlca, vec![]), vec![0x07]);
    assert_eq!(enc(Mnemonic::Rla, vec![]), vec![0x17]);
    assert_eq!(enc(Mnemonic::Rra, vec![]), vec![0x1F]);
    assert_eq!(enc(Mnemonic::Daa, vec![]), vec![0x27]);
    assert_eq!(enc(Mnemonic::Cpl, vec![]), vec![0x2F]);
    assert_eq!(enc(Mnemonic::Ccf, vec![]), vec![0x3F]);
    assert_eq!(enc(Mnemonic::Halt, vec![]), vec![0x76]);
    // The two that were already here, asserted alongside: they anchor the rotate
    // column (0x07/0x0F/0x17/0x1F) and the flag column that the new arms join.
    assert_eq!(enc(Mnemonic::Rrca, vec![]), vec![0x0F]);
    assert_eq!(enc(Mnemonic::Scf, vec![]), vec![0x37]);
}

#[test]
fn rst_encodes_every_page_zero_vector() {
    for (p, want) in
        [(0x00, 0xC7), (0x08, 0xCF), (0x10, 0xD7), (0x18, 0xDF),
         (0x20, 0xE7), (0x28, 0xEF), (0x30, 0xF7), (0x38, 0xFF)]
    {
        assert_eq!(
            enc(Mnemonic::Rst, vec![Operand::Imm8(p)]),
            vec![want],
            "rst ${p:02X}"
        );
    }
}

/// An off-vector `rst` must be REFUSED, not masked into a different valid
/// restart. Masking is the silently-wrong-instruction class the 68k EA work
/// closed; `0xC7 | (p & 0x38)` would happily turn `rst $05` into `rst $00`.
#[test]
fn rst_refuses_a_target_that_is_not_a_vector() {
    for p in [0x01u8, 0x05, 0x07, 0x09, 0x3F, 0x40, 0xFF] {
        assert!(
            encode(&Instruction { mnemonic: Mnemonic::Rst, ops: vec![Operand::Imm8(p)] }).is_err(),
            "rst ${p:02X} is not a page-zero vector and must be refused"
        );
    }
}
