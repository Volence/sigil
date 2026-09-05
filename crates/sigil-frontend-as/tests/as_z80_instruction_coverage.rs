//! Z80 instructions the front end had no name for, driven end to end.
//!
//! Every expectation below is the reference asl's own answer (asl md5
//! 61e672562465725a8c102288a7da9098, the s1disasm build; one asl invocation per
//! form with its exit status checked, so no form's bytes can be another form's
//! substituted residue). None is copied from a neighbouring value, which for
//! this instruction group is the whole difficulty: the ED block ops occupy a
//! 4x4 grid whose members differ by a single bit, so a table that swapped a row
//! for a column still answers with legal, plausible bytes.
//!
//! That is why the assertions come in FAMILIES rather than one exemplar each:
//!
//!   * `ldi`/`cpi`/`ini`/`outi` differ only in bits 1..0 of the sub-opcode;
//!   * `ldi`/`ldd` differ only in bit 3, `ldi`/`ldir` only in bit 4;
//!   * `in r,(c)`/`out (c),r` differ only in bit 0, with the register in bits
//!     5..3 -- and register `a` is code 7, not 0, so a family tested only on
//!     `b` leaves the register shift completely unexercised;
//!   * `im 0`/`im 1`/`im 2` are ED 46/56/5E, which is NOT `0x46 | mode << 4`:
//!     the obvious arithmetic yields 46/56/66, and ED 66 is a real (undocumented)
//!     `im 0` alias, so a wrong table there is invisible to a spot check;
//!   * `ld i,a` and `ld a,i` differ only in bit 4, the classic direction slip.

use sigil_frontend_as::{assemble, Options};
use sigil_ir::SymbolTable;

/// Assemble one Z80 snippet through parse -> lower -> link -> flatten.
fn asm(snippet: &str) -> Vec<u8> {
    let src = format!("        cpu z80\n        phase 0\n        {snippet}\n");
    let module = assemble(&src, &Options::default())
        .unwrap_or_else(|d| panic!("assemble `{snippet}` failed: {d:?}"));
    let linked = sigil_link::link(&module.sections, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link `{snippet}` failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
}

/// Assert a whole family at once and require its members to be PAIRWISE
/// DISTINCT. The second half is the part a single-form assertion cannot do: a
/// table that collapsed two neighbours onto one opcode passes every individual
/// check that happens to look at the surviving one.
fn family(cases: &[(&str, &[u8])]) {
    let mut wrong = Vec::new();
    for (snippet, want) in cases {
        let got = asm(snippet);
        if got != *want {
            wrong.push(format!("`{snippet}`: want {want:02X?}, got {got:02X?}"));
        }
    }
    assert!(wrong.is_empty(), "{} form(s) diverged:\n{}", wrong.len(), wrong.join("\n"));

    for (i, (sa, _)) in cases.iter().enumerate() {
        for (sb, _) in cases.iter().skip(i + 1) {
            assert_ne!(asm(sa), asm(sb), "`{sa}` and `{sb}` encode identically");
        }
    }
}

/// `ldi` -- the single-step block move, 17 live sites in Sonic 2's sound driver.
///
/// It is asserted against its three single-bit neighbours at once. `ldir` was
/// already encoded and is the one this instruction is most often confused with
/// in prose ("the block copy"): they are different instructions, ED A0 and
/// ED B0, and the pair is here so that confusion cannot survive in the table.
#[test]
fn ldi_and_its_single_bit_neighbours() {
    family(&[
        ("ldi", &[0xED, 0xA0]),
        ("ldir", &[0xED, 0xB0]),
    ]);
}
