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

// (additional Task 4 tests are appended below)
