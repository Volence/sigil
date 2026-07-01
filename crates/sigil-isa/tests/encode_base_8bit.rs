//! Golden encode vectors for the base (unprefixed) 8-bit load / ALU / inc-dec forms.
//!
//! Every expected byte string was produced by `tools/asl -cpu 68000 -q -L -U` on a
//! `cpu z80 / phase 0` snippet (ground truth: /home/volence/sonic_hacks/aeon). See the
//! reference table in the Plan 2 / Task 3 section.
use sigil_isa::z80::{encode, Instruction, Mnemonic, Operand, Reg8};

/// Build an instruction from a mnemonic and an operand list.
fn inst(mnemonic: Mnemonic, ops: Vec<Operand>) -> Instruction {
    Instruction { mnemonic, ops }
}

#[test]
fn ld_reg_reg_and_reg_imm() {
    // ld b,a = 47 ; ld a,a = 7F ; ld h,l = 65 ; ld e,b = 58   (dst<<3 | src field math)
    assert_eq!(
        encode(&inst(Mnemonic::Ld, vec![Operand::Reg(Reg8::B), Operand::Reg(Reg8::A)])).unwrap(),
        vec![0x47]
    );
    assert_eq!(
        encode(&inst(Mnemonic::Ld, vec![Operand::Reg(Reg8::A), Operand::Reg(Reg8::A)])).unwrap(),
        vec![0x7F]
    );
    assert_eq!(
        encode(&inst(Mnemonic::Ld, vec![Operand::Reg(Reg8::H), Operand::Reg(Reg8::L)])).unwrap(),
        vec![0x65]
    );
    assert_eq!(
        encode(&inst(Mnemonic::Ld, vec![Operand::Reg(Reg8::E), Operand::Reg(Reg8::B)])).unwrap(),
        vec![0x58]
    );
    // ld d,0 = 16 00 ; ld l,0FFh = 2E FF
    assert_eq!(
        encode(&inst(Mnemonic::Ld, vec![Operand::Reg(Reg8::D), Operand::Imm8(0)])).unwrap(),
        vec![0x16, 0x00]
    );
    assert_eq!(
        encode(&inst(Mnemonic::Ld, vec![Operand::Reg(Reg8::L), Operand::Imm8(0xFF)])).unwrap(),
        vec![0x2E, 0xFF]
    );
}

#[test]
fn ld_hl_indirect() {
    // ld a,(hl) = 7E ; ld c,(hl) = 4E   (dst<<3 | 0x46)
    assert_eq!(
        encode(&inst(Mnemonic::Ld, vec![Operand::Reg(Reg8::A), Operand::IndHl])).unwrap(),
        vec![0x7E]
    );
    assert_eq!(
        encode(&inst(Mnemonic::Ld, vec![Operand::Reg(Reg8::C), Operand::IndHl])).unwrap(),
        vec![0x4E]
    );
    // ld (hl),c = 71 ; ld (hl),a = 77   (0x70 | src)
    assert_eq!(
        encode(&inst(Mnemonic::Ld, vec![Operand::IndHl, Operand::Reg(Reg8::C)])).unwrap(),
        vec![0x71]
    );
    assert_eq!(
        encode(&inst(Mnemonic::Ld, vec![Operand::IndHl, Operand::Reg(Reg8::A)])).unwrap(),
        vec![0x77]
    );
    // ld (hl),0 = 36 00
    assert_eq!(
        encode(&inst(Mnemonic::Ld, vec![Operand::IndHl, Operand::Imm8(0)])).unwrap(),
        vec![0x36, 0x00]
    );
}

// (further base-8bit vectors appended by later steps)
