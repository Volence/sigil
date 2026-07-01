//! Round-trip + coverage tests for the Z80 disassembler over the five migrated
//! Plan-1 forms (Task 1). Full-ISA disassembly is deferred to later tasks.

use sigil_isa::z80::{disassemble, encode, Instruction, IsaError, Mnemonic, Operand, Reg8};

/// Build an `ld dst, src` instruction in the canonical model.
fn ld(dst: Operand, src: Operand) -> Instruction {
    Instruction { mnemonic: Mnemonic::Ld, ops: vec![dst, src] }
}

/// Build an `add a, src` instruction in the canonical model.
fn add_a(src: Operand) -> Instruction {
    Instruction { mnemonic: Mnemonic::Add, ops: vec![Operand::Reg(Reg8::A), src] }
}

#[test]
fn disassemble_nop() {
    let (inst, len) = disassemble(&[0x00]).unwrap();
    assert_eq!(inst, Instruction { mnemonic: Mnemonic::Nop, ops: vec![] });
    assert_eq!(len, 1);
}

#[test]
fn disassemble_ld_reg_reg() {
    // ld b, c = 0x41  (0x40 | (B<<3) | C)
    let (inst, len) = disassemble(&[0x41]).unwrap();
    assert_eq!(inst, ld(Operand::Reg(Reg8::B), Operand::Reg(Reg8::C)));
    assert_eq!(len, 1);
    // ld a, a = 0x7F
    let (inst, len) = disassemble(&[0x7F]).unwrap();
    assert_eq!(inst, ld(Operand::Reg(Reg8::A), Operand::Reg(Reg8::A)));
    assert_eq!(len, 1);
}

#[test]
fn disassemble_ld_reg_imm() {
    // ld a, 5 = 0x3E, 0x05
    let (inst, len) = disassemble(&[0x3E, 0x05]).unwrap();
    assert_eq!(inst, ld(Operand::Reg(Reg8::A), Operand::Imm8(5)));
    assert_eq!(len, 2);
    // ld b, 10 = 0x06, 0x0A
    let (inst, len) = disassemble(&[0x06, 0x0A]).unwrap();
    assert_eq!(inst, ld(Operand::Reg(Reg8::B), Operand::Imm8(10)));
    assert_eq!(len, 2);
}

#[test]
fn disassemble_add_a_reg() {
    // add a, b = 0x80
    let (inst, len) = disassemble(&[0x80]).unwrap();
    assert_eq!(inst, add_a(Operand::Reg(Reg8::B)));
    assert_eq!(len, 1);
    // add a, a = 0x87
    let (inst, len) = disassemble(&[0x87]).unwrap();
    assert_eq!(inst, add_a(Operand::Reg(Reg8::A)));
    assert_eq!(len, 1);
}

#[test]
fn disassemble_jp_imm() {
    // jp 1234h = 0xC3, 0x34, 0x12 (little-endian)
    let (inst, len) = disassemble(&[0xC3, 0x34, 0x12]).unwrap();
    assert_eq!(inst, Instruction { mnemonic: Mnemonic::Jp, ops: vec![Operand::Imm16(0x1234)] });
    assert_eq!(len, 3);
    // jp 00FFh = 0xC3, 0xFF, 0x00
    let (inst, len) = disassemble(&[0xC3, 0xFF, 0x00]).unwrap();
    assert_eq!(inst, Instruction { mnemonic: Mnemonic::Jp, ops: vec![Operand::Imm16(0x00FF)] });
    assert_eq!(len, 3);
}

#[test]
fn disassemble_consumes_only_its_own_bytes() {
    // 0x00 followed by trailing bytes -> Nop, len 1
    let (inst, len) = disassemble(&[0x00, 0xC3, 0x34, 0x12]).unwrap();
    assert_eq!(inst, Instruction { mnemonic: Mnemonic::Nop, ops: vec![] });
    assert_eq!(len, 1);
}

#[test]
fn unknown_opcode_is_err() {
    // 0xD3 = OUT, 0xFF = RST 38h — both outside the migrated subset
    assert!(matches!(disassemble(&[0xD3]), Err(IsaError::UnsupportedForm(_))));
    assert!(matches!(disassemble(&[0xFF]), Err(IsaError::UnsupportedForm(_))));
}

#[test]
fn hl_dst_ld_reg_reg_is_err() {
    // ld (hl), b = 0x70 -> reg code 6 in dst; (HL) forms deferred
    assert!(matches!(disassemble(&[0x70]), Err(IsaError::UnsupportedForm(_))));
}

#[test]
fn hl_src_ld_reg_reg_is_err() {
    // ld b, (hl) = 0x46 -> reg code 6 in src
    assert!(matches!(disassemble(&[0x46]), Err(IsaError::UnsupportedForm(_))));
    // halt = 0x76 -> both dst and src are code 6
    assert!(matches!(disassemble(&[0x76]), Err(IsaError::UnsupportedForm(_))));
}

#[test]
fn hl_ld_reg_imm_is_err() {
    // ld (hl), n = 0x36, 0x00 -> reg code 6 in dst
    assert!(matches!(disassemble(&[0x36, 0x00]), Err(IsaError::UnsupportedForm(_))));
}

#[test]
fn hl_add_a_reg_is_err() {
    // add a, (hl) = 0x86 -> reg code 6 in src
    assert!(matches!(disassemble(&[0x86]), Err(IsaError::UnsupportedForm(_))));
}

#[test]
fn round_trip_representative_list() {
    let list = vec![
        Instruction { mnemonic: Mnemonic::Nop, ops: vec![] },
        ld(Operand::Reg(Reg8::B), Operand::Reg(Reg8::C)),
        ld(Operand::Reg(Reg8::A), Operand::Reg(Reg8::A)),
        ld(Operand::Reg(Reg8::H), Operand::Reg(Reg8::L)),
        ld(Operand::Reg(Reg8::A), Operand::Imm8(5)),
        ld(Operand::Reg(Reg8::B), Operand::Imm8(10)),
        ld(Operand::Reg(Reg8::L), Operand::Imm8(0xFF)),
        add_a(Operand::Reg(Reg8::B)),
        add_a(Operand::Reg(Reg8::A)),
        Instruction { mnemonic: Mnemonic::Jp, ops: vec![Operand::Imm16(0x1234)] },
        Instruction { mnemonic: Mnemonic::Jp, ops: vec![Operand::Imm16(0x00FF)] },
        Instruction { mnemonic: Mnemonic::Jp, ops: vec![Operand::Imm16(0x0000)] },
    ];
    for inst in list {
        let bytes = encode(&inst).unwrap();
        let (decoded, len) = disassemble(&bytes).unwrap();
        assert_eq!(decoded, inst, "decoded mismatch for {inst:?}");
        assert_eq!(len, bytes.len(), "length mismatch for {inst:?}");
    }
}
