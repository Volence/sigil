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

#[test]
fn add_hl_pair() {
    // asl: add hl,bc=09  add hl,de=19  add hl,hl=29  add hl,sp=39
    assert_eq!(
        enc(Mnemonic::Add, vec![Operand::Pair(Reg16::Hl), Operand::Pair(Reg16::Bc)]),
        vec![0x09]
    );
    assert_eq!(
        enc(Mnemonic::Add, vec![Operand::Pair(Reg16::Hl), Operand::Pair(Reg16::De)]),
        vec![0x19]
    );
    assert_eq!(
        enc(Mnemonic::Add, vec![Operand::Pair(Reg16::Hl), Operand::Pair(Reg16::Hl)]),
        vec![0x29]
    );
    assert_eq!(
        enc(Mnemonic::Add, vec![Operand::Pair(Reg16::Hl), Operand::Pair(Reg16::Sp)]),
        vec![0x39]
    );
}

#[test]
fn inc_dec_pair() {
    // asl: inc bc/de/hl/sp = 03 13 23 33 ; dec bc/de/hl/sp = 0B 1B 2B 3B
    assert_eq!(enc(Mnemonic::Inc, vec![Operand::Pair(Reg16::Bc)]), vec![0x03]);
    assert_eq!(enc(Mnemonic::Inc, vec![Operand::Pair(Reg16::De)]), vec![0x13]);
    assert_eq!(enc(Mnemonic::Inc, vec![Operand::Pair(Reg16::Hl)]), vec![0x23]);
    assert_eq!(enc(Mnemonic::Inc, vec![Operand::Pair(Reg16::Sp)]), vec![0x33]);
    assert_eq!(enc(Mnemonic::Dec, vec![Operand::Pair(Reg16::Bc)]), vec![0x0B]);
    assert_eq!(enc(Mnemonic::Dec, vec![Operand::Pair(Reg16::De)]), vec![0x1B]);
    assert_eq!(enc(Mnemonic::Dec, vec![Operand::Pair(Reg16::Hl)]), vec![0x2B]);
    assert_eq!(enc(Mnemonic::Dec, vec![Operand::Pair(Reg16::Sp)]), vec![0x3B]);
}

#[test]
fn ret_and_ret_cc() {
    // asl: ret=C9 ; ret nz/z/nc/c/po/pe/p/m = C0 C8 D0 D8 E0 E8 F0 F8
    assert_eq!(enc(Mnemonic::Ret, vec![]), vec![0xC9]);
    assert_eq!(enc(Mnemonic::Ret, vec![Operand::Cc(Cond::Nz)]), vec![0xC0]);
    assert_eq!(enc(Mnemonic::Ret, vec![Operand::Cc(Cond::Z)]), vec![0xC8]);
    assert_eq!(enc(Mnemonic::Ret, vec![Operand::Cc(Cond::Nc)]), vec![0xD0]);
    assert_eq!(enc(Mnemonic::Ret, vec![Operand::Cc(Cond::C)]), vec![0xD8]);
    assert_eq!(enc(Mnemonic::Ret, vec![Operand::Cc(Cond::Po)]), vec![0xE0]);
    assert_eq!(enc(Mnemonic::Ret, vec![Operand::Cc(Cond::Pe)]), vec![0xE8]);
    assert_eq!(enc(Mnemonic::Ret, vec![Operand::Cc(Cond::P)]), vec![0xF0]);
    assert_eq!(enc(Mnemonic::Ret, vec![Operand::Cc(Cond::M)]), vec![0xF8]);
}

#[test]
fn jp_and_jp_cc() {
    // asl: jp 1234h=C3 34 12 ; jp nz/z/nc/c/po/pe/p/m,1234h = C2 CA D2 DA E2 EA F2 FA (+34 12)
    assert_eq!(enc(Mnemonic::Jp, vec![Operand::Imm16(0x1234)]), vec![0xC3, 0x34, 0x12]);
    assert_eq!(enc(Mnemonic::Jp, vec![Operand::Cc(Cond::Nz), Operand::Imm16(0x1234)]), vec![0xC2, 0x34, 0x12]);
    assert_eq!(enc(Mnemonic::Jp, vec![Operand::Cc(Cond::Z), Operand::Imm16(0x1234)]), vec![0xCA, 0x34, 0x12]);
    assert_eq!(enc(Mnemonic::Jp, vec![Operand::Cc(Cond::Nc), Operand::Imm16(0x1234)]), vec![0xD2, 0x34, 0x12]);
    assert_eq!(enc(Mnemonic::Jp, vec![Operand::Cc(Cond::C), Operand::Imm16(0x1234)]), vec![0xDA, 0x34, 0x12]);
    assert_eq!(enc(Mnemonic::Jp, vec![Operand::Cc(Cond::Po), Operand::Imm16(0x1234)]), vec![0xE2, 0x34, 0x12]);
    assert_eq!(enc(Mnemonic::Jp, vec![Operand::Cc(Cond::Pe), Operand::Imm16(0x1234)]), vec![0xEA, 0x34, 0x12]);
    assert_eq!(enc(Mnemonic::Jp, vec![Operand::Cc(Cond::P), Operand::Imm16(0x1234)]), vec![0xF2, 0x34, 0x12]);
    assert_eq!(enc(Mnemonic::Jp, vec![Operand::Cc(Cond::M), Operand::Imm16(0x1234)]), vec![0xFA, 0x34, 0x12]);
}

#[test]
fn call_and_call_cc() {
    // asl: call 1234h=CD 34 12 ; call nz/z/nc/c/po/pe/p/m,1234h = C4 CC D4 DC E4 EC F4 FC (+34 12)
    assert_eq!(enc(Mnemonic::Call, vec![Operand::Imm16(0x1234)]), vec![0xCD, 0x34, 0x12]);
    assert_eq!(enc(Mnemonic::Call, vec![Operand::Cc(Cond::Nz), Operand::Imm16(0x1234)]), vec![0xC4, 0x34, 0x12]);
    assert_eq!(enc(Mnemonic::Call, vec![Operand::Cc(Cond::Z), Operand::Imm16(0x1234)]), vec![0xCC, 0x34, 0x12]);
    assert_eq!(enc(Mnemonic::Call, vec![Operand::Cc(Cond::Nc), Operand::Imm16(0x1234)]), vec![0xD4, 0x34, 0x12]);
    assert_eq!(enc(Mnemonic::Call, vec![Operand::Cc(Cond::C), Operand::Imm16(0x1234)]), vec![0xDC, 0x34, 0x12]);
    assert_eq!(enc(Mnemonic::Call, vec![Operand::Cc(Cond::Po), Operand::Imm16(0x1234)]), vec![0xE4, 0x34, 0x12]);
    assert_eq!(enc(Mnemonic::Call, vec![Operand::Cc(Cond::Pe), Operand::Imm16(0x1234)]), vec![0xEC, 0x34, 0x12]);
    assert_eq!(enc(Mnemonic::Call, vec![Operand::Cc(Cond::P), Operand::Imm16(0x1234)]), vec![0xF4, 0x34, 0x12]);
    assert_eq!(enc(Mnemonic::Call, vec![Operand::Cc(Cond::M), Operand::Imm16(0x1234)]), vec![0xFC, 0x34, 0x12]);
}

#[test]
fn jr_relative() {
    // asl `jr $` / `jr cc,$` at own-PC → disp -2 (FE): jr=18 nz=20 z=28 nc=30 c=38
    assert_eq!(enc(Mnemonic::Jr, vec![Operand::Rel(-2)]), vec![0x18, 0xFE]);
    assert_eq!(enc(Mnemonic::Jr, vec![Operand::Cc(Cond::Nz), Operand::Rel(-2)]), vec![0x20, 0xFE]);
    assert_eq!(enc(Mnemonic::Jr, vec![Operand::Cc(Cond::Z), Operand::Rel(-2)]), vec![0x28, 0xFE]);
    assert_eq!(enc(Mnemonic::Jr, vec![Operand::Cc(Cond::Nc), Operand::Rel(-2)]), vec![0x30, 0xFE]);
    assert_eq!(enc(Mnemonic::Jr, vec![Operand::Cc(Cond::C), Operand::Rel(-2)]), vec![0x38, 0xFE]);
    // A positive, already-resolved displacement passes through verbatim (no relaxation).
    assert_eq!(enc(Mnemonic::Jr, vec![Operand::Rel(5)]), vec![0x18, 0x05]);
}

#[test]
fn jr_rejects_non_flag_conditions() {
    // Only nz/z/nc/c are legal for jr; po/pe/p/m must be rejected (asl cannot encode them).
    for cc in [Cond::Po, Cond::Pe, Cond::P, Cond::M] {
        let r = encode(&Instruction {
            mnemonic: Mnemonic::Jr,
            ops: vec![Operand::Cc(cc), Operand::Rel(0)],
        });
        assert!(matches!(r, Err(IsaError::OperandRange(_))), "jr {cc:?} should be rejected");
    }
}

#[test]
fn djnz_relative() {
    // asl `djnz $` at own-PC → disp -2: 10 FE. Displacement passes through as i8 -> u8.
    assert_eq!(enc(Mnemonic::Djnz, vec![Operand::Rel(-2)]), vec![0x10, 0xFE]);
    assert_eq!(enc(Mnemonic::Djnz, vec![Operand::Rel(-16)]), vec![0x10, 0xF0]);
}

// (additional Task 4 tests are appended below)
