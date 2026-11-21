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
        let a = asl(line).unwrap_err();
        let want = format!("`{line}` is refused, as asl refuses it");
        assert!(a.iter().any(|m| m.contains(&want)), "AS route, `{line}`: {a:?}");
    }
}

#[test]
fn the_usp_moves_assemble() {
    use Operand::*;
    golden("move.l a3, usp", i(Mnemonic::MoveToUsp, Size::L, &[An(3), Usp]), &[0x4E, 0x63]);
    golden("move.l usp, a5", i(Mnemonic::MoveFromUsp, Size::L, &[Usp, An(5)]), &[0x4E, 0x6D]);
}

#[test]
fn the_ccr_forms_assemble() {
    use Operand::*;
    golden("move.w d3, ccr", i(Mnemonic::MoveToCcr, Size::W, &[Dn(3), Ccr]), &[0x44, 0xC3]);
    golden("move.w #$1F, ccr", i(Mnemonic::MoveToCcr, Size::W, &[Imm(0x1F), Ccr]), &[0x44, 0xFC, 0x00, 0x1F]);
    golden("eori.b #$01, ccr", i(Mnemonic::EoriCcr, Size::B, &[Imm(1), Ccr]), &[0x0A, 0x3C, 0x00, 0x01]);
}

#[test]
fn movem_loads_from_a_pc_relative_source() {
    use Operand::*;
    // `.tgt` is the frame's first byte (offset 0); the movem sits at 2 and its
    // displacement is taken from its extension word at 6: -6 = $FA / $FFFA.
    let pcd16 = [0x4C, 0xBA, 0x00, 0x18, 0xFF, 0xFA];
    let enc = encode(&i(Mnemonic::Movem, Size::W, &[Pcd16(-6), RegList(0x0018)]));
    assert_eq!(enc.as_deref(), Ok(&pcd16[..]), "shared encoder, (d16,pc)");
    assert_eq!(emp("movem.w .tgt(pc), d3-d4"), Ok(framed(&pcd16)));
    assert_eq!(asl("movem.w tgt(pc),d3-d4"), Ok(framed(&pcd16)));
    let pcd8 = [0x4C, 0xFB, 0x00, 0x18, 0xA0, 0xFA];
    let xn = m68k::Xn::A(2);
    let enc = encode(&i(Mnemonic::Movem, Size::L, &[Pcd8Xn { d: -6, xn, long: false }, RegList(0x0018)]));
    assert_eq!(enc.as_deref(), Ok(&pcd8[..]), "shared encoder, (d8,pc,a2.w)");
    assert_eq!(emp("movem.l .tgt(pc,a2.w), d3-d4"), Ok(framed(&pcd8)));
    assert_eq!(asl("movem.l tgt(pc,a2.w),d3-d4"), Ok(framed(&pcd8)));
}

/// A PC-relative `movem` DESTINATION is not a 68000 instruction (program-space
/// references are read-only): refused on both routes.
#[test]
fn movem_to_a_pc_relative_destination_is_refused() {
    for (e, a) in [("movem.l d3-d4, .tgt(pc)", "movem.l d3-d4,tgt(pc)"), ("movem.w d3-d4, .tgt(pc,a2.w)", "movem.w d3-d4,tgt(pc,a2.w)")] {
        let err = emp(e).unwrap_err();
        assert!(err.iter().any(|m| m.starts_with("illegal destination EA: (d")), "`{e}`: {err:?}");
        assert!(asl(a).is_err(), "AS route accepts `{a}`");
    }
}
