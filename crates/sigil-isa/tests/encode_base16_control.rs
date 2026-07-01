//! Task 4 — base group: 16-bit ops + control flow.
//! Every expected byte string is the exact output of `tools/asl` (asl 1.42) for the
//! corresponding `cpu z80 / phase 0` snippet (see the plan's asl-provenance recipe).

use sigil_isa::z80::{encode, Cond, Instruction, IsaError, Mnemonic, Operand, Reg16};

/// Build an `Instruction` and unwrap its encoded bytes.
fn enc(mnemonic: Mnemonic, ops: Vec<Operand>) -> Vec<u8> {
    encode(&Instruction { mnemonic, ops }).unwrap()
}

#[test]
fn no_operand_fixed_opcodes() {
    // asl: nop=00  exx=D9  rrca=0F  scf=37  ei=FB  di=F3
    assert_eq!(enc(Mnemonic::Nop, vec![]), vec![0x00]);
    assert_eq!(enc(Mnemonic::Exx, vec![]), vec![0xD9]);
    assert_eq!(enc(Mnemonic::Rrca, vec![]), vec![0x0F]);
    assert_eq!(enc(Mnemonic::Scf, vec![]), vec![0x37]);
    assert_eq!(enc(Mnemonic::Ei, vec![]), vec![0xFB]);
    assert_eq!(enc(Mnemonic::Di, vec![]), vec![0xF3]);
}

#[test]
fn exchange_and_hl_transfer() {
    // asl: ex de,hl=EB  ex (sp),hl=E3  ex af,af'=08  ld sp,hl=F9  jp (hl)=E9
    assert_eq!(
        enc(Mnemonic::Ex, vec![Operand::Pair(Reg16::De), Operand::Pair(Reg16::Hl)]),
        vec![0xEB]
    );
    // (sp) is modeled as Pair(Sp): there is no `ex sp,hl` instruction, so this is unambiguous.
    assert_eq!(
        enc(Mnemonic::Ex, vec![Operand::Pair(Reg16::Sp), Operand::Pair(Reg16::Hl)]),
        vec![0xE3]
    );
    assert_eq!(
        enc(Mnemonic::Ex, vec![Operand::Pair(Reg16::Af), Operand::AfShadow]),
        vec![0x08]
    );
    assert_eq!(
        enc(Mnemonic::Ld, vec![Operand::Pair(Reg16::Sp), Operand::Pair(Reg16::Hl)]),
        vec![0xF9]
    );
    assert_eq!(enc(Mnemonic::Jp, vec![Operand::IndHl]), vec![0xE9]);
}

// (additional Task 4 tests are appended below)
