//! The Plan 3 acceptance gate: a hand-built two-section Z80 module, lowered via
//! the real Z80 backend and linked, lays out at $400/$60000 with an A→B
//! cross-fixup resolving to the phased VMA $845F (little-endian).

use sigil_backend_z80::Z80Backend;
use sigil_ir::backend::Backend;
use sigil_ir::{Cpu, DataFragment, Expr, Fixup, FixupKind, Fragment, Label, Section, SymbolTable};
use sigil_isa::z80::{Mnemonic, Operand, Reg16};
use sigil_link::{flatten, link};
use sigil_span::{SourceId, Span};

fn span() -> Span {
    Span { source: SourceId(0), start: 0, end: 0 }
}

#[test]
fn two_section_ab_layout_and_cross_fixup() {
    let backend = Z80Backend;

    // Region A: `ld de,SfxBlobWinTab`. Lower the *opcode/skeleton* via the
    // backend using a placeholder immediate, then attach the cross-region fixup.
    // ld de,nn = 11 lo hi. Encode with nn=0 → 11 00 00, then fix offset 1.
    let a_ld = backend
        .lower(Mnemonic::Ld, &[Operand::Pair(Reg16::De), Operand::Imm16(0)], span())
        .expect("lower ld de,nn");
    assert_eq!(a_ld.bytes, vec![0x11, 0x00, 0x00]);
    let a_frag = DataFragment {
        bytes: a_ld.bytes,
        fixups: vec![Fixup {
            kind: FixupKind::BankPtr16Le,
            offset: 1,
            target: Expr::Sym("SfxBlobWinTab".to_string()),
        }],
        span: span(),
    };
    let region_a = Section {
        name: "regionA".to_string(),
        cpu: Cpu::Z80,
        vma_base: Some(0x0000),
        lma: 0x400,
        labels: vec![],
        fragments: vec![Fragment::Data(a_frag)],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
    };

    // Region B: SfxBlobWinTab at VMA $8000 + $45F = $845F.
    let region_b = Section {
        name: "regionB".to_string(),
        cpu: Cpu::Z80,
        vma_base: Some(0x8000),
        lma: 0x60000,
        labels: vec![Label { name: "SfxBlobWinTab".to_string(), offset: 0x45F }],
        fragments: vec![
            Fragment::Fill { value: 0x00, count: 0x45F, span: span() },
            Fragment::Data(DataFragment { bytes: vec![0x9A, 0xD6], fixups: vec![], span: span() }),
        ],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
    };

    let linked = link(&[region_a, region_b], &SymbolTable::new()).expect("link ok");

    // A→B fixup resolved to $845F little-endian.
    let a = linked.section("regionA").unwrap();
    assert_eq!(a.bytes, vec![0x11, 0x5F, 0x84], "ld de,SfxBlobWinTab must resolve to $845F LE");
    assert_eq!(a.lma, 0x400);

    let b = linked.section("regionB").unwrap();
    assert_eq!(b.lma, 0x60000);
    assert_eq!(b.bytes.len(), 0x45F + 2);
    assert_eq!(&b.bytes[0x45F..], &[0x9A, 0xD6]);

    // The materialized image places A at $400 and B at $60000.
    let image = flatten(&linked, 0x00);
    assert_eq!(&image[0x400..0x403], &[0x11, 0x5F, 0x84]);
    assert_eq!(&image[0x6045F..0x60461], &[0x9A, 0xD6]);
}
