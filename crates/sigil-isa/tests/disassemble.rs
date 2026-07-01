use sigil_isa::z80::{disassemble, encode, Instruction, IsaError, Reg8};

#[test]
fn disassemble_nop() {
    let (inst, len) = disassemble(&[0x00]).unwrap();
    assert_eq!(inst, Instruction::Nop);
    assert_eq!(len, 1);
}

#[test]
fn disassemble_ld_reg_reg() {
    // ld b, c = 0x41  (0x40 | (B<<3) | C)
    let (inst, len) = disassemble(&[0x41]).unwrap();
    assert_eq!(inst, Instruction::LdRegReg { dst: Reg8::B, src: Reg8::C });
    assert_eq!(len, 1);
    // ld a, a = 0x7F
    let (inst, len) = disassemble(&[0x7F]).unwrap();
    assert_eq!(inst, Instruction::LdRegReg { dst: Reg8::A, src: Reg8::A });
    assert_eq!(len, 1);
}

#[test]
fn disassemble_ld_reg_imm() {
    // ld a, 5 = 0x3E, 0x05
    let (inst, len) = disassemble(&[0x3E, 0x05]).unwrap();
    assert_eq!(inst, Instruction::LdRegImm { dst: Reg8::A, imm: 5 });
    assert_eq!(len, 2);
    // ld b, 10 = 0x06, 0x0A
    let (inst, len) = disassemble(&[0x06, 0x0A]).unwrap();
    assert_eq!(inst, Instruction::LdRegImm { dst: Reg8::B, imm: 10 });
    assert_eq!(len, 2);
}

#[test]
fn disassemble_add_a_reg() {
    // add a, b = 0x80
    let (inst, len) = disassemble(&[0x80]).unwrap();
    assert_eq!(inst, Instruction::AddAReg { src: Reg8::B });
    assert_eq!(len, 1);
    // add a, a = 0x87
    let (inst, len) = disassemble(&[0x87]).unwrap();
    assert_eq!(inst, Instruction::AddAReg { src: Reg8::A });
    assert_eq!(len, 1);
}

#[test]
fn disassemble_jp_imm() {
    // jp $1234 = 0xC3, 0x34, 0x12 (little-endian)
    let (inst, len) = disassemble(&[0xC3, 0x34, 0x12]).unwrap();
    assert_eq!(inst, Instruction::JpImm { addr: 0x1234 });
    assert_eq!(len, 3);
    // jp 0x00FF = 0xC3, 0xFF, 0x00
    let (inst, len) = disassemble(&[0xC3, 0xFF, 0x00]).unwrap();
    assert_eq!(inst, Instruction::JpImm { addr: 0x00FF });
    assert_eq!(len, 3);
}

#[test]
fn disassemble_consumes_only_its_own_bytes() {
    // 0x00 followed by trailing bytes -> Nop, len 1
    let (inst, len) = disassemble(&[0x00, 0xC3, 0x34, 0x12]).unwrap();
    assert_eq!(inst, Instruction::Nop);
    assert_eq!(len, 1);
}

#[test]
fn unknown_opcode_is_err() {
    // 0xD3 = OUT, 0xFF = RST 38h — both outside the subset
    assert!(matches!(disassemble(&[0xD3]), Err(IsaError::UnsupportedOperand(_))));
    assert!(matches!(disassemble(&[0xFF]), Err(IsaError::UnsupportedOperand(_))));
}

#[test]
fn hl_dst_ld_reg_reg_is_err() {
    // ld (hl), b = 0x70 -> reg code 6 in dst
    assert!(matches!(disassemble(&[0x70]), Err(IsaError::UnsupportedOperand(_))));
}

#[test]
fn hl_src_ld_reg_reg_is_err() {
    // ld b, (hl) = 0x46 -> reg code 6 in src
    assert!(matches!(disassemble(&[0x46]), Err(IsaError::UnsupportedOperand(_))));
    // halt = 0x76 -> both dst and src are code 6
    assert!(matches!(disassemble(&[0x76]), Err(IsaError::UnsupportedOperand(_))));
}

#[test]
fn hl_ld_reg_imm_is_err() {
    // ld (hl), n = 0x36, 0x00 -> reg code 6 in dst
    assert!(matches!(disassemble(&[0x36, 0x00]), Err(IsaError::UnsupportedOperand(_))));
}

#[test]
fn hl_add_a_reg_is_err() {
    // add a, (hl) = 0x86 -> reg code 6 in src
    assert!(matches!(disassemble(&[0x86]), Err(IsaError::UnsupportedOperand(_))));
}

#[test]
fn round_trip_representative_list() {
    let list = vec![
        Instruction::Nop,
        Instruction::LdRegReg { dst: Reg8::B, src: Reg8::C },
        Instruction::LdRegReg { dst: Reg8::A, src: Reg8::A },
        Instruction::LdRegReg { dst: Reg8::H, src: Reg8::L },
        Instruction::LdRegImm { dst: Reg8::A, imm: 5 },
        Instruction::LdRegImm { dst: Reg8::B, imm: 10 },
        Instruction::LdRegImm { dst: Reg8::L, imm: 0xFF },
        Instruction::AddAReg { src: Reg8::B },
        Instruction::AddAReg { src: Reg8::A },
        Instruction::JpImm { addr: 0x1234 },
        Instruction::JpImm { addr: 0x00FF },
        Instruction::JpImm { addr: 0x0000 },
    ];
    for inst in list {
        let bytes = encode(&inst).unwrap();
        let (decoded, len) = disassemble(&bytes).unwrap();
        assert_eq!(decoded, inst, "decoded mismatch for {:?}", inst);
        assert_eq!(len, bytes.len(), "length mismatch for {:?}", inst);
    }
}
