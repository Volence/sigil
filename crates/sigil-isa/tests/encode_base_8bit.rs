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

#[test]
fn ld_indirect_pair_and_absolute() {
    // ld (de),a = 12 ; ld a,(de) = 1A ; ld (bc),a = 02 ; ld a,(bc) = 0A
    assert_eq!(
        encode(&inst(Mnemonic::Ld, vec![Operand::IndDe, Operand::Reg(Reg8::A)])).unwrap(),
        vec![0x12]
    );
    assert_eq!(
        encode(&inst(Mnemonic::Ld, vec![Operand::Reg(Reg8::A), Operand::IndDe])).unwrap(),
        vec![0x1A]
    );
    assert_eq!(
        encode(&inst(Mnemonic::Ld, vec![Operand::IndBc, Operand::Reg(Reg8::A)])).unwrap(),
        vec![0x02]
    );
    assert_eq!(
        encode(&inst(Mnemonic::Ld, vec![Operand::Reg(Reg8::A), Operand::IndBc])).unwrap(),
        vec![0x0A]
    );
    // ld a,(8DFCh) = 3A FC 8D ; ld (8DFCh),a = 32 FC 8D   (imm16 little-endian)
    assert_eq!(
        encode(&inst(Mnemonic::Ld, vec![Operand::Reg(Reg8::A), Operand::Mem(0x8DFC)])).unwrap(),
        vec![0x3A, 0xFC, 0x8D]
    );
    assert_eq!(
        encode(&inst(Mnemonic::Ld, vec![Operand::Mem(0x8DFC), Operand::Reg(Reg8::A)])).unwrap(),
        vec![0x32, 0xFC, 0x8D]
    );
}

#[test]
fn alu_reg_and_hl() {
    // Single-operand shape (asl: `sub c`, `or a`, `cp b`, …)
    let cases: &[(Mnemonic, Reg8, u8)] = &[
        (Mnemonic::Add, Reg8::A, 0x87), // add a,a
        (Mnemonic::Add, Reg8::B, 0x80), // add a,b
        (Mnemonic::Adc, Reg8::H, 0x8C), // adc a,h
        (Mnemonic::Sub, Reg8::C, 0x91), // sub c
        (Mnemonic::Sbc, Reg8::A, 0x9F), // sbc a,a
        (Mnemonic::And, Reg8::C, 0xA1), // and c
        (Mnemonic::Or, Reg8::A, 0xB7),  // or a   (A=7 -> B0|7=B7, NOT B0)
        (Mnemonic::Or, Reg8::C, 0xB1),  // or c
        (Mnemonic::Xor, Reg8::A, 0xAF), // xor a  (A8|7=AF)
        (Mnemonic::Cp, Reg8::B, 0xB8),  // cp b
    ];
    for &(m, r, byte) in cases {
        assert_eq!(encode(&inst(m, vec![Operand::Reg(r)])).unwrap(), vec![byte], "{m:?} {r:?}");
    }

    // Two-operand `<op> a,r` shape must encode identically to `<op> r`.
    assert_eq!(encode(&inst(Mnemonic::Sub, vec![Operand::Reg(Reg8::A), Operand::Reg(Reg8::C)])).unwrap(), vec![0x91]);
    assert_eq!(encode(&inst(Mnemonic::And, vec![Operand::Reg(Reg8::A), Operand::Reg(Reg8::C)])).unwrap(), vec![0xA1]);
    assert_eq!(encode(&inst(Mnemonic::Or, vec![Operand::Reg(Reg8::A), Operand::Reg(Reg8::A)])).unwrap(), vec![0xB7]);
    assert_eq!(encode(&inst(Mnemonic::Xor, vec![Operand::Reg(Reg8::A), Operand::Reg(Reg8::A)])).unwrap(), vec![0xAF]);
    assert_eq!(encode(&inst(Mnemonic::Cp, vec![Operand::Reg(Reg8::A), Operand::Reg(Reg8::B)])).unwrap(), vec![0xB8]);

    // `<op> a,(hl)` / `<op> (hl)`
    let hl_cases: &[(Mnemonic, u8)] = &[
        (Mnemonic::Add, 0x86),
        (Mnemonic::Adc, 0x8E),
        (Mnemonic::Sub, 0x96),
        (Mnemonic::Sbc, 0x9E),
        (Mnemonic::And, 0xA6),
        (Mnemonic::Or, 0xB6),
        (Mnemonic::Xor, 0xAE),
        (Mnemonic::Cp, 0xBE),
    ];
    for &(m, byte) in hl_cases {
        assert_eq!(encode(&inst(m, vec![Operand::IndHl])).unwrap(), vec![byte], "{m:?} (hl)");
        assert_eq!(encode(&inst(m, vec![Operand::Reg(Reg8::A), Operand::IndHl])).unwrap(), vec![byte], "{m:?} a,(hl)");
    }
}

// (further base-8bit vectors appended by later steps)
