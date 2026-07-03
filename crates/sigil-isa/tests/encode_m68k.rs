//! Encoder tests: `encode(corpus form) == asl golden`, byte-for-byte. Grows one
//! per-mode test per implementation task; the final `all_forms_match_golden` gate
//! covers the entire corpus.

mod corpus_m68k;
mod m68k_common;

use m68k_common::{golden_bytes, parse_golden_m68k};
use sigil_isa::m68k::encode;

const GOLDEN: &str = include_str!("m68k_golden_vectors.txt");

/// Encode every corpus form whose snippet is in `snippets` and assert it matches golden.
fn check(snippets: &[&str]) {
    let golden = parse_golden_m68k(GOLDEN);
    let corpus = corpus_m68k::corpus_m68k();
    for snip in snippets {
        let inst = corpus
            .iter()
            .find(|(s, _)| s == snip)
            .unwrap_or_else(|| panic!("snippet {snip:?} not in corpus"))
            .1
            .clone();
        let want = golden_bytes(&golden, snip);
        let got = encode(&inst).unwrap_or_else(|e| panic!("encode {snip:?}: {e}"));
        assert_eq!(got, want, "snippet {snip:?}");
    }
}

#[test]
fn reg_direct() {
    check(&["move.w d1,d0", "move.l a1,d0"]);
}

#[test]
fn memory_indirect_source_and_dest() {
    check(&[
        "move.w (a1),d0",
        "move.w (a1)+,d0",
        "move.w -(a1),d0",
        "move.w d1,(a0)",
        "move.w d1,(a0)+",
        "move.w d1,-(a0)",
    ]);
}

#[test]
fn displacement_and_brief_extension_word() {
    check(&[
        "move.w (4,a1),d0",
        "move.w d1,(4,a0)",
        "move.w (6,a1,d2.w),d0",
        "move.l (2,a3,a4.l),d0",
    ]);
}

#[test]
fn absolute_pcrel_immediate() {
    check(&[
        "move.w ($1234).w,d0",
        "move.w ($12345678).l,d0",
        "move.w (8,pc),d0",
        "move.w #$1234,d0",
        "move.w d1,($1234).w",
        "move.w d1,($12345678).l",
    ]);
}

#[test]
fn immediate_and_pcrel_are_illegal_dest() {
    use sigil_isa::m68k::{encode, Instruction, Mnemonic, Operand, Size};
    let imm_dst = Instruction {
        mnemonic: Mnemonic::Move,
        size: Size::W,
        ops: vec![Operand::Dn(0), Operand::Imm(1)],
    };
    assert!(matches!(encode(&imm_dst), Err(sigil_isa::m68k::IsaError::IllegalDest(_))));
    let pc_dst = Instruction {
        mnemonic: Mnemonic::Move,
        size: Size::W,
        ops: vec![Operand::Dn(0), Operand::Pcd16(4)],
    };
    assert!(matches!(encode(&pc_dst), Err(sigil_isa::m68k::IsaError::IllegalDest(_))));
}

#[test]
fn alu_ea_family() {
    check(&[
        "add.w d1,d0", "add.w (a1),d0", "add.l d0,(a1)", "sub.w d1,d0",
        "and.w d1,d0", "or.b d1,d0", "eor.w d0,d1", "cmp.w (a1),d0",
        "cmpa.l a1,a0", "adda.w d0,a1", "suba.l a2,a3", "muls.w d1,d0",
    ]);
}

#[test]
fn alu_immediate_family() {
    check(&[
        "addi.w #$10,d0", "subi.l #$1000,d1", "andi.w #$00FF,d0", "ori.b #$01,d0",
        "eori.w #$FFFF,d0", "cmpi.w #$0010,(a1)", "andi.b #$FE,ccr", "ori.b #$01,ccr",
        "move.w #$2700,sr", "move.w sr,-(sp)",
    ]);
}

#[test]
fn quick_family() {
    check(&["moveq #1,d0", "moveq #-1,d3", "addq.w #1,d0", "addq.l #8,a1", "subq.w #2,d1"]);
}

#[test]
fn all_forms_match_golden() {
    let golden = parse_golden_m68k(GOLDEN);
    let mut mismatches = Vec::new();
    for (snip, inst) in corpus_m68k::corpus_m68k() {
        let want = golden_bytes(&golden, snip);
        match encode(&inst) {
            Ok(got) if got == want => {}
            Ok(got) => mismatches.push(format!("{snip}: got {got:02X?}, want {want:02X?}")),
            Err(e) => mismatches.push(format!("{snip}: error {e}")),
        }
    }
    assert!(mismatches.is_empty(), "mismatches:\n{}", mismatches.join("\n"));
}

#[test]
fn dest_ea_field_swap_is_proven() {
    // `move.w d1,(a0)` (dest indirect) must differ from `move.w (a0),d1` shape:
    // dest EA sits in bits 11-6 as register:mode, source in bits 5-0 as mode:register.
    // Concretely: dest (a0) => dst_reg=000, dst_mode=010 => bits 11-6 = 000_010.
    use sigil_isa::m68k::{encode, Instruction, Mnemonic, Operand, Size};
    let inst = Instruction {
        mnemonic: Mnemonic::Move,
        size: Size::W,
        ops: vec![Operand::Dn(1), Operand::Ind(0)],
    };
    let bytes = encode(&inst).unwrap();
    let word = u16::from_be_bytes([bytes[0], bytes[1]]);
    assert_eq!((word >> 6) & 0b111111, 0b000_010, "dest EA field (reg:mode) wrong");
    assert_eq!(word & 0b111111, 0b000_001, "source EA field (mode:reg) wrong for d1");
}
