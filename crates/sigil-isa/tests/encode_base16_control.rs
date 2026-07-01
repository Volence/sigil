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

#[test]
fn ld_pair_imm16() {
    // asl: ld bc,1234h=01 34 12  ld de,8DFCh=11 FC 8D  ld hl,1234h=21 34 12  ld sp,1234h=31 34 12
    assert_eq!(
        enc(Mnemonic::Ld, vec![Operand::Pair(Reg16::Bc), Operand::Imm16(0x1234)]),
        vec![0x01, 0x34, 0x12]
    );
    assert_eq!(
        enc(Mnemonic::Ld, vec![Operand::Pair(Reg16::De), Operand::Imm16(0x8DFC)]),
        vec![0x11, 0xFC, 0x8D]
    );
    assert_eq!(
        enc(Mnemonic::Ld, vec![Operand::Pair(Reg16::Hl), Operand::Imm16(0x1234)]),
        vec![0x21, 0x34, 0x12]
    );
    assert_eq!(
        enc(Mnemonic::Ld, vec![Operand::Pair(Reg16::Sp), Operand::Imm16(0x1234)]),
        vec![0x31, 0x34, 0x12]
    );
}

#[test]
fn ld_hl_absolute_memory() {
    // asl: ld hl,(1234h)=2A 34 12   ld (1234h),hl=22 34 12  (HL-only base opcodes)
    assert_eq!(
        enc(Mnemonic::Ld, vec![Operand::Pair(Reg16::Hl), Operand::Mem(0x1234)]),
        vec![0x2A, 0x34, 0x12]
    );
    assert_eq!(
        enc(Mnemonic::Ld, vec![Operand::Mem(0x1234), Operand::Pair(Reg16::Hl)]),
        vec![0x22, 0x34, 0x12]
    );
}

#[test]
fn push_and_pop() {
    // asl: push bc/de/hl/af = C5 D5 E5 F5 ; pop bc/de/hl/af = C1 D1 E1 F1
    assert_eq!(enc(Mnemonic::Push, vec![Operand::Pair(Reg16::Bc)]), vec![0xC5]);
    assert_eq!(enc(Mnemonic::Push, vec![Operand::Pair(Reg16::De)]), vec![0xD5]);
    assert_eq!(enc(Mnemonic::Push, vec![Operand::Pair(Reg16::Hl)]), vec![0xE5]);
    assert_eq!(enc(Mnemonic::Push, vec![Operand::Pair(Reg16::Af)]), vec![0xF5]);
    assert_eq!(enc(Mnemonic::Pop, vec![Operand::Pair(Reg16::Bc)]), vec![0xC1]);
    assert_eq!(enc(Mnemonic::Pop, vec![Operand::Pair(Reg16::De)]), vec![0xD1]);
    assert_eq!(enc(Mnemonic::Pop, vec![Operand::Pair(Reg16::Hl)]), vec![0xE1]);
    assert_eq!(enc(Mnemonic::Pop, vec![Operand::Pair(Reg16::Af)]), vec![0xF1]);
}

// (additional Task 4 tests are appended below)
