//! Acceptance vector 2 for the `table` construct (Plan 7 T2-d): the
//! RECORD-LIST emission shape, proving the construct is born GENERAL and not
//! sfx-shaped (the R1/R2 requirement). A `header: u16(count - 1)` PLC-shaped
//! table (`plrlistheader`/`plreq`: a count word then 6-byte `dc.l art, dc.w
//! vram` records) is byte-diffed against the equivalent AS-macro output
//! assembled through the AS front-end.
//!
//! The AS side computes the header via the stride division the macro uses
//! (`(End - Start)/6 - 1`); the `.emp` side spells it `header: u16(count - 1)`.
//! Equal bytes prove the division collapses to the row count (design §1).

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};

fn linked_all(m: &Module) -> Vec<u8> {
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    // One section in each fixture; return its bytes.
    m.sections
        .iter()
        .find_map(|s| linked.section(&s.name).map(|ls| ls.bytes.clone()))
        .expect("a linked section")
}

/// The AS-macro reference: `plrlistheader` (a `dc.w (End-Start)/6 - 1` count
/// word) then three `plreq art, vram` records (`dc.l art, dc.w vram`), with the
/// three art blobs immediately after so the pointer targets are deterministic.
fn as_reference() -> Vec<u8> {
    let asm = "\
cpu 68000
phase 0
Plc:
        dc.w (PlcEnd-PlcStart)/6-1
PlcStart:
        dc.l ArtA
        dc.w $6000
        dc.l ArtB
        dc.w $6800
        dc.l ArtC
        dc.w $6C00
PlcEnd:
ArtA:   dc.b $AA
ArtB:   dc.b $BB
ArtC:   dc.b $CC
";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let m = assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble: {d:?}"));
    linked_all(&m)
}

/// The `.emp` candidate: the same PLC list as a record-list `table`.
fn emp_candidate() -> Vec<u8> {
    let src = "\
module m
struct PlcReq { art: *u8, vram: u16 }
section s (cpu: m68000) {
    table Plc: [PlcReq] (header: u16(count - 1)) {
        PlcReq { art: \"ArtA\", vram: $6000 },
        PlcReq { art: \"ArtB\", vram: $6800 },
        PlcReq { art: \"ArtC\", vram: $6C00 },
    }
    data ArtA: u8 = $AA
    data ArtB: u8 = $BB
    data ArtC: u8 = $CC
}
";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (m, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "lower: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    linked_all(&m)
}

#[test]
fn plc_record_list_matches_as_macro() {
    let expected = as_reference();
    let candidate = emp_candidate();
    // Sanity: header 00 02 (count-1 = 2), then 3 six-byte records, then art.
    assert_eq!(&expected[0..2], &[0x00, 0x02], "AS header = count-1");
    assert_eq!(
        expected,
        vec![
            0x00, 0x02, // header (count - 1)
            0x00, 0x00, 0x00, 0x14, 0x60, 0x00, // ArtA @0x14, vram $6000
            0x00, 0x00, 0x00, 0x15, 0x68, 0x00, // ArtB @0x15, vram $6800
            0x00, 0x00, 0x00, 0x16, 0x6C, 0x00, // ArtC @0x16, vram $6C00
            0xAA, 0xBB, 0xCC, // the three art blobs
        ],
        "AS reference layout"
    );
    assert_eq!(candidate, expected, "table record-list bytes must equal the AS-macro output");
}
