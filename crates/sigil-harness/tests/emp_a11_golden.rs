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
    for line in ["rtr.w", "rtr.l", "unlk.l a6", "link.l a6, #-8\n    unlk a6", "stop.w #$2700", "reset.w", "trapv.w", "chk.l d0, d1", "chk.b d0, d1"] {
        let e = emp(line).unwrap_err();
        assert!(e[0].starts_with("[lower.size-suffix] "), "`{line}`: {e:?}");
        let a = asl(line).unwrap_err();
        // The AS route names the written mnemonic, suffix included (`unlk.l`).
        // asl itself assembles `chk.l` (the MC68020 word `4300`); the AS route
        // refuses it with its own reason, a 68000 does not decode that word.
        let word = line.split_whitespace().next().unwrap();
        let want = if word == "chk.l" {
            "`chk.l` is the MC68020's long form, which a 68000 does not decode".to_string()
        } else {
            format!("`{word}` is refused, as asl refuses it")
        };
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
    // A LONG index (final review triage (b)): the brief extension word is
    // D/A=1 (an address register), register 001 (a1), W/L=1 (long), so its
    // high byte is %1001_1000 = $98; the displacement byte is again -6 = $FA.
    let pcd8l = [0x4C, 0xFB, 0x00, 0x18, 0x98, 0xFA];
    let xn = m68k::Xn::A(1);
    let enc = encode(&i(Mnemonic::Movem, Size::L, &[Pcd8Xn { d: -6, xn, long: true }, RegList(0x0018)]));
    assert_eq!(enc.as_deref(), Ok(&pcd8l[..]), "shared encoder, (d8,pc,a1.l)");
    assert_eq!(emp("movem.l .tgt(pc,a1.l), d3-d4"), Ok(framed(&pcd8l)));
    assert_eq!(asl("movem.l tgt(pc,a1.l),d3-d4"), Ok(framed(&pcd8l)));
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

#[test]
fn link_and_unlk_assemble() {
    use Operand::*;
    assert_eq!(encode(&i(Mnemonic::Link, Size::W, &[An(6), Imm(-8)])).as_deref(), Ok(&[0x4E, 0x56, 0xFF, 0xF8][..]));
    assert_eq!(encode(&i(Mnemonic::Unlk, Size::W, &[An(6)])).as_deref(), Ok(&[0x4E, 0x5E][..]));
    let pair = framed(&[0x4E, 0x56, 0xFF, 0xF8, 0x4E, 0x5E]);
    assert_eq!(emp("link a6, #-8\n    unlk a6"), Ok(pair.clone()));
    assert_eq!(emp("link.w a6, #-8\n    unlk a6"), Ok(pair.clone()));
    assert_eq!(asl("link a6,#-8\n\tunlk a6"), Ok(pair));
    // A second frame register and a displacement with every nibble distinct.
    // LINK.W is `0100 1110 0101 0rrr` + the 16-bit displacement: a3 -> $4E53,
    // -$1234 -> $EDCC. UNLK is `0100 1110 0101 1rrr`: a3 -> $4E5B.
    let link = [0x4E, 0x53, 0xED, 0xCC];
    assert_eq!(encode(&i(Mnemonic::Link, Size::W, &[An(3), Imm(-0x1234)])).as_deref(), Ok(&link[..]));
    assert_eq!(encode(&i(Mnemonic::Unlk, Size::W, &[An(3)])).as_deref(), Ok(&[0x4E, 0x5B][..]));
    let pair = framed(&[0x4E, 0x53, 0xED, 0xCC, 0x4E, 0x5B]);
    assert_eq!(emp("link a3, #-$1234\n    unlk a3"), Ok(pair.clone()));
    assert_eq!(asl("link a3,#-$1234\n\tunlk a3"), Ok(pair));
}

#[test]
fn stop_and_the_sr_immediates_assemble() {
    use Operand::*;
    golden("stop #$2700", i(Mnemonic::Stop, Size::W, &[Imm(0x2700)]), &[0x4E, 0x72, 0x27, 0x00]);
    golden("ori.w #$0700, sr", i(Mnemonic::OriSr, Size::W, &[Imm(0x0700), Sr]), &[0x00, 0x7C, 0x07, 0x00]);
    golden("andi.w #$F8FF, sr", i(Mnemonic::AndiSr, Size::W, &[Imm(0xF8FF), Sr]), &[0x02, 0x7C, 0xF8, 0xFF]);
    golden("eori.w #$0700, sr", i(Mnemonic::EoriSr, Size::W, &[Imm(0x0700), Sr]), &[0x0A, 0x7C, 0x07, 0x00]);
}

/// RESET is the fixed word `0100 1110 0111 0000` = $4E70 (no operand field).
#[test]
fn reset_assembles() {
    golden("reset", i(Mnemonic::Reset, Size::W, &[]), &[0x4E, 0x70]);
}

#[test]
fn chk_and_trapv_assemble() {
    use Operand::*;
    golden("chk.w d0, d1", i(Mnemonic::Chk, Size::W, &[Dn(0), Dn(1)]), &[0x43, 0x80]);
    golden("chk.w (a0), d1", i(Mnemonic::Chk, Size::W, &[Ind(0), Dn(1)]), &[0x43, 0x90]);
    golden("trapv", i(Mnemonic::Trapv, Size::W, &[]), &[0x4E, 0x76]);
    // `chk` has no default size on either route.
    assert_eq!(emp("chk d0, d1").unwrap_err(), ["instruction needs an explicit size suffix (.b/.w/.l)"]);
    assert!(asl("chk d0,d1").is_err());
}

/// CHK.W is `0100 DDD 110 MMMRRR` (bound register DDD, bound source EA
/// mode/register), TRAPV the fixed word `0100 1110 0111 0110` = $4E76. With
/// distinct non-zero registers in every field:
/// - `chk.w d3, d5`: DDD=101, EA=000 011 -> 0100 1011 1000 0011 = $4B83;
/// - `chk.w (a2), d6`: DDD=110, EA=010 010 -> 0100 1101 1001 0010 = $4D92;
/// - `chk.w #$1234, d2`: DDD=010, EA=111 100 -> 0100 0101 1011 1100 = $45BC,
///   then the immediate word $1234;
/// - `chk.w 6(a4), d7`: DDD=111, EA=101 100 -> 0100 1111 1010 1100 = $4FAC,
///   then the displacement word $0006.
#[test]
fn chk_assembles_with_distinct_registers_in_every_field() {
    use Operand::*;
    golden("chk.w d3, d5", i(Mnemonic::Chk, Size::W, &[Dn(3), Dn(5)]), &[0x4B, 0x83]);
    golden("chk.w (a2), d6", i(Mnemonic::Chk, Size::W, &[Ind(2), Dn(6)]), &[0x4D, 0x92]);
    golden("chk.w #$1234, d2", i(Mnemonic::Chk, Size::W, &[Imm(0x1234), Dn(2)]), &[0x45, 0xBC, 0x12, 0x34]);
    golden("chk.w 6(a4), d7", i(Mnemonic::Chk, Size::W, &[Disp16An(6, 4), Dn(7)]), &[0x4F, 0xAC, 0x00, 0x06]);
}
