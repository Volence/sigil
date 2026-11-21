//! A11 golden vectors: each instruction A11 makes writable, three ways. The
//! literal bytes, the shared encoder (`sigil_isa::m68k::encode`) and both front
//! ends (`.emp`, and the asl-compatible AS route) must agree.
use m68k::{encode, Instruction, Mnemonic, Operand, Size};
use sigil_isa::m68k;

#[path = "../../sigil-cli/tests/support/frontend_diff.rs"]
mod frontend_diff;

fn emp(body: &str) -> Result<Vec<u8>, Vec<String>> {
    frontend_diff::emp_assemble(&format!("module m\nproc f() {{\n.tgt:\n    nop\n    {body}\n    rts\n}}\n"))
}

fn asl(body: &str) -> Result<Vec<u8>, Vec<String>> {
    frontend_diff::as_assemble(&format!("\tcpu 68000\ntgt:\n\tnop\n\t{body}\n\trts\n"))
}

fn framed(mid: &[u8]) -> Vec<u8> {
    [&[0x4E, 0x71][..], mid, &[0x4E, 0x75][..]].concat()
}

/// `line` assembles to `hex` on both routes, and `hex` is what the shared
/// encoder gives for `inst`.
#[track_caller]
fn golden(line: &str, inst: Instruction, hex: &[u8]) {
    assert_eq!(encode(&inst).as_deref(), Ok(hex), "shared encoder, `{line}`");
    assert_eq!(emp(line), Ok(framed(hex)), "`.emp`, `{line}`");
    assert_eq!(asl(line), Ok(framed(hex)), "AS route, `{line}`");
}

fn i(mnemonic: Mnemonic, size: Size, ops: &[Operand]) -> Instruction {
    Instruction { mnemonic, size, ops: ops.to_vec() }
}

#[test]
fn the_extended_and_decimal_arithmetic_assembles() {
    use Operand::*;
    golden("abcd d0, d1", i(Mnemonic::Abcd, Size::B, &[Dn(0), Dn(1)]), &[0xC3, 0x00]);
    golden("abcd -(a0), -(a1)", i(Mnemonic::Abcd, Size::B, &[PreDec(0), PreDec(1)]), &[0xC3, 0x08]);
    golden("sbcd d0, d1", i(Mnemonic::Sbcd, Size::B, &[Dn(0), Dn(1)]), &[0x83, 0x00]);
    golden("nbcd d0", i(Mnemonic::Nbcd, Size::B, &[Dn(0)]), &[0x48, 0x00]);
    golden("nbcd (a0)", i(Mnemonic::Nbcd, Size::B, &[Ind(0)]), &[0x48, 0x10]);
    golden("negx.w d0", i(Mnemonic::Negx, Size::W, &[Dn(0)]), &[0x40, 0x40]);
    golden("negx.b (a0)", i(Mnemonic::Negx, Size::B, &[Ind(0)]), &[0x40, 0x10]);
    golden("subx.w d0, d1", i(Mnemonic::Subx, Size::W, &[Dn(0), Dn(1)]), &[0x93, 0x40]);
    golden("subx.l -(a0), -(a1)", i(Mnemonic::Subx, Size::L, &[PreDec(0), PreDec(1)]), &[0x93, 0x88]);
}

#[test]
fn exg_assembles_with_and_without_its_suffix() {
    use Operand::*;
    for line in ["exg d0, d1", "exg.l d0, d1"] {
        golden(line, i(Mnemonic::Exg, Size::L, &[Dn(0), Dn(1)]), &[0xC1, 0x41]);
    }
    golden("exg a0, a1", i(Mnemonic::Exg, Size::L, &[An(0), An(1)]), &[0xC1, 0x49]);
    golden("exg d0, a1", i(Mnemonic::Exg, Size::L, &[Dn(0), An(1)]), &[0xC1, 0x89]);
    let e = emp("exg.w d0, d1").unwrap_err();
    assert!(e[0].contains("exg is long-only"), "{e:?}");
}

#[test]
fn rtr_assembles() {
    golden("rtr", i(Mnemonic::Rtr, Size::W, &[]), &[0x4E, 0x77]);
}

/// A suffix the instruction does not have is refused, on both routes.
#[test]
fn a_size_suffix_the_instruction_lacks_is_refused() {
    for line in ["rtr.w", "rtr.l"] {
        let e = emp(line).unwrap_err();
        assert!(e[0].starts_with("[lower.size-suffix] "), "`{line}`: {e:?}");
        assert!(asl(&line.replace("\n    ", "\n\t")).is_err(), "AS route accepts `{line}`");
    }
}
