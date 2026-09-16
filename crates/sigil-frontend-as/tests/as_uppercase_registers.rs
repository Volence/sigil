//! asl's register names are case-insensitive even under `-U`, AND they are
//! per-CPU. Both halves of that sentence are load-bearing, and this file pins
//! each of them against the reference assembler.
//!
//! ## The silent half, which is why this exists
//!
//! With `A0: equ $1234` in scope, `move.w A0,d0`:
//!
//! ```text
//!     asl     3008              a0 direct: the REGISTER wins over the symbol
//!     sigil   3038 1234         absolute, through the symbol
//! ```
//!
//! Two different instructions, both toolchains exit 0, neither says a word.
//! The classifier matched register spellings in lower case only, so an
//! uppercase one fell through to the ordinary-symbol path, and if a symbol of
//! that name happened to exist it was silently used. That is the wrong-ROM
//! shape, and no corpus build option reaches it (nothing in Sonic 1 or Sonic 2
//! spells a register in upper case), so no sweep could have found it.
//!
//! ## The per-CPU half, which decides the implementation
//!
//! Under `cpu z80`, with `A0: equ 05678h` in scope, asl assembles
//! `ld a,(A0)` as `3A 78 56`: an ABSOLUTE load through the symbol, because the
//! Z80 has no register called `A0`. Under `cpu 68000` the same parenthesised
//! name is `3010`, a0 indirect. So a case fold that did not know the CPU would
//! be right in one corpus and wrong in the other. That is what
//! `a_z80_symbol_spelled_like_a_68k_register_is_still_a_memory_reference`
//! measures, and it is the test that fails if the fold is ever made
//! CPU-agnostic.
//!
//! ## Where every byte below comes from
//!
//! asl, reference build md5 `61e672562465725a8c102288a7da9098`, invoked through
//! `asl_run` (`docs/superpowers/notes/asl-reference/asl_ref.sh`) with
//! `-xx -n -q -A -L -U`, exit 0 and `ASL_DIAG=complete` on every probe quoted.
//! The probes and their listings are committed beside the note, in
//! `docs/superpowers/notes/2026-09-16-as-uppercase-registers/probes/`: `p9`
//! (68000), `p10` (Z80), `p15` (`z80undoc` halves) and `p16` (PC-relative and
//! the control-register `move` forms). `p9` and `p10` are assembled with NO
//! `org`, which is the origin the assertions below compare at. Every symbol in
//! `p9`'s listing is marked unused (`*`), which is asl saying in its own output
//! that it never consulted the equates.

use sigil_frontend_as::{assemble, Options};
use sigil_ir::SymbolTable;

/// Assemble one probe through parse, lower, link and flatten.
fn build(src: &str) -> Result<Vec<u8>, String> {
    let module = assemble(src, &Options::default()).map_err(|d| format!("{d:?}"))?;
    let linked =
        sigil_link::link(&module.sections, &SymbolTable::new()).map_err(|d| format!("{d:?}"))?;
    sigil_link::flatten(&linked, 0x00).map_err(|d| format!("{d:?}"))
}

fn bytes(src: &str) -> Vec<u8> {
    build(src).unwrap_or_else(|e| panic!("front end refused:\n{src}\n{e}"))
}

fn refused(src: &str, needle: &str) {
    match build(src) {
        Ok(b) => panic!("assembled to {b:02X?} instead of refusing:\n{src}"),
        Err(e) => assert!(e.contains(needle), "no diagnostic naming {needle:?}: {e}"),
    }
}

/// `b` is asl's byte column, written as it appears in the listing.
fn hex(b: &str) -> Vec<u8> {
    let s: String = b.chars().filter(|c| !c.is_whitespace()).collect();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex pair"))
        .collect()
}

/// Every equate the 68000 probe defines, so that each assertion below is the
/// SHADOWED case: a symbol of the register's own name is in scope and holds a
/// value that would be visible in the bytes if sigil consulted it.
const M68K: &str = "\tcpu 68000\nA0:\tequ $1234\nSP:\tequ $5678\nD0:\tequ $9abc\n\
                    SR:\tequ $4444\nCCR:\tequ $2222\nUSP:\tequ $3333\n";

/// The Z80 probe's equates, same idea: `HL` is a register on this CPU and `A0`
/// is not, and both have a value that would show up in the bytes.
const Z80: &str = "\tcpu z80\nHL:\tequ 01234h\nA0:\tequ 05678h\n";

/// THE DEFECT: a bare uppercase register in an EA position is the register, not
/// the symbol that shares its name. asl p9.lst 9, 11, 12, 13.
#[test]
fn a_bare_uppercase_register_is_the_register_and_not_the_shadowing_symbol() {
    for (src, asl) in [
        ("\tmove.w A0,d0\n", "3008"),
        ("\tmove.w SP,d0\n", "300F"),
        ("\tmove.w Sp,d0\n", "300F"),
        ("\tmove.w D0,d1\n", "3200"),
    ] {
        assert_eq!(bytes(&format!("{M68K}{src}")), hex(asl), "{src}");
    }
}

/// And the same for the parenthesised forms, which were loud before this and
/// are now simply right. asl p9.lst 8, 10, 14, 15.
#[test]
fn uppercase_register_indirect_encodes_as_the_register() {
    for (src, asl) in [
        ("\tmove.w (A0),d0\n", "3010"),
        ("\tmove.w (SP),d0\n", "3017"),
        ("\tmove.w (A0)+,d0\n", "3018"),
        ("\tmove.w -(A0),d0\n", "3020"),
        ("\tlea (A0),A1\n", "43D0"),
    ] {
        assert_eq!(bytes(&format!("{M68K}{src}")), hex(asl), "{src}");
    }
}

/// The displacement and index forms, in both the AS `(d,An)` spelling and the
/// Motorola `disp(An)` one, with the index-size suffix in upper case too.
/// asl p9.lst 16 to 20.
#[test]
fn uppercase_displacement_and_index_forms_encode_as_registers() {
    for (src, asl) in [
        ("\tmove.w (4,A0),d0\n", "3028 0004"),
        ("\tmove.w 4(A0),d0\n", "3028 0004"),
        ("\tmove.w (4,A0,D1.W),d0\n", "3030 1004"),
        ("\tmove.w (A0,D1),d0\n", "3030 1000"),
        ("\tmove.w (A0,D1.L),d0\n", "3030 1800"),
    ] {
        assert_eq!(bytes(&format!("{M68K}{src}")), hex(asl), "{src}");
    }
}

/// A MOVEM register list is spelled out of the same names and folds the same
/// way: `D0/A0` is bit 0 and bit 8, reversed by the `-(An)` rule into `8080`.
/// asl p9.lst 21.
#[test]
fn an_uppercase_movem_register_list_is_a_register_list() {
    assert_eq!(
        bytes(&format!("{M68K}\tmovem.w D0/A0,-(sp)\n")),
        hex("48A7 8080")
    );
}

/// The control registers are register names too, and the shadowing equates make
/// the difference visible: `move.w SR,d0` reading `SR` as `$4444` would be a
/// `3038 4444`, not a `40C0`. asl p9.lst 23, 24, 25.
#[test]
fn the_control_registers_fold_case_as_well() {
    for (src, asl) in [
        ("\tmove.w SR,d0\n", "40C0"),
        ("\tmove.b d0,CCR\n", "44C0"),
        ("\tmove.l a0,USP\n", "4E60"),
    ] {
        assert_eq!(bytes(&format!("{M68K}{src}")), hex(asl), "{src}");
    }
}

/// THE CPU-KEYED HALF, and the one assertion here that a case fold without a
/// CPU to hand cannot satisfy. `A0` is not a Z80 register, so `(A0)` is an
/// ordinary memory reference through the symbol, and asl loads from `$5678`.
/// `(HL)` on the same line of the same file IS a register, in either case.
/// asl p10.lst 4 to 8.
#[test]
fn a_z80_symbol_spelled_like_a_68k_register_is_still_a_memory_reference() {
    assert_eq!(bytes(&format!("{Z80}\tld a,(A0)\n")), hex("3A 78 56"));
    assert_eq!(bytes(&format!("{Z80}\tld a,(HL)\n")), hex("7E"));
}

/// And the Z80 register names themselves fold, which is the same rule seen from
/// the other CPU. asl p10.lst 5, 6, 7, 9, 10, 11.
#[test]
fn z80_register_condition_and_index_names_fold_case() {
    for (src, asl) in [
        ("\tld A,(HL)\n", "7E"),
        ("\tld B,C\n", "41"),
        ("\tld a,(IX+3)\n", "DD 7E 03"),
        ("\tld BC,01234h\n", "01 34 12"),
        ("\tjr NZ,$\n", "20 FE"),
        ("\tex (SP),HL\n", "E3"),
    ] {
        assert_eq!(bytes(&format!("{Z80}{src}")), hex(asl), "{src}");
    }
}

/// What asl REFUSES stays refused, in either case. A fold that widened into
/// these would be an over-acceptance rather than a fix.
///
/// * `(dN)` is asl's `error #1505: addressing mode not supported on 68000`.
/// * `(A0).w` is asl's `error #1146: expected integer or string` -- a register
///   in parens is not an address expression, so a width suffix cannot apply to
///   it. sigil's own wording for this shape is a parse refusal.
/// * `(PC)` asl DOES assemble, as PC-relative at zero displacement
///   (`303A EFFE` at `$1000`), and sigil has no operand for it: refused, which
///   is a documented gap and not an agreement.
#[test]
fn the_shapes_asl_refuses_are_still_refused_in_upper_case() {
    for spelling in ["d0", "D0", "d7", "D7"] {
        refused(
            &format!("{M68K}\tmove.w ({spelling}),d1\n"),
            "names a 68k register",
        );
    }
    for spelling in ["pc", "PC", "Pc"] {
        refused(
            &format!("{M68K}\tmove.w ({spelling}),d0\n"),
            "names a 68k register",
        );
    }
    for spelling in ["a0", "A0", "sp", "SP"] {
        refused(&format!("{M68K}\tmove.w ({spelling}).w,d0\n"), "operand");
    }
}

/// PC-relative addressing is spelled out of the same register table, and the
/// implicit size of the control-register `move` forms is read off the operand
/// name before the operand is converted, so both had to learn the fold or the
/// line dies asking for a suffix asl never needed. asl `probes/p16.asm` 4, 6,
/// 7, 8, exit 0 and `ASL_DIAG=complete`.
#[test]
fn pc_relative_and_the_implicit_control_register_sizes_fold_case() {
    assert_eq!(
        bytes("\tcpu 68000\n\torg 0\nTbl:\tdc.w 1,2\n\tmove.w (Tbl,PC),d1\n"),
        hex("0001 0002 323A FFFA")
    );
    for (src, asl) in [
        ("\tmove D6,CCR\n", "44C6"),
        ("\tmove #$2700,SR\n", "46FC 2700"),
        ("\tmove A6,USP\n", "4E66"),
    ] {
        assert_eq!(bytes(&format!("\tcpu 68000\n{src}")), hex(asl), "{src}");
    }
}

/// The `z80undoc` index-register halves take a plain register beside them, and
/// that register folds too. asl `probes/p15.asm` 3 to 6.
#[test]
fn an_index_register_half_takes_an_uppercase_plain_register() {
    for (src, asl) in [
        ("\tld A,ixl\n", "DD 7D"),
        ("\tld IXU,B\n", "DD 60"),
        ("\tld a,IXL\n", "DD 7D"),
        ("\tld IYU,A\n", "FD 67"),
    ] {
        assert_eq!(
            bytes(&format!("\tcpu z80undoc\n\torg 0\n{src}")),
            hex(asl),
            "{src}"
        );
    }
}

/// The lower-case readings are untouched. This is the half a fold is most
/// likely to move by accident, so it is asserted rather than assumed: every
/// spelling below is what it was before the fold existed.
#[test]
fn the_lower_case_readings_did_not_move() {
    for (src, asl) in [
        ("\tmove.w a0,d0\n", "3008"),
        ("\tmove.w sp,d0\n", "300F"),
        ("\tmove.w (a0),d0\n", "3010"),
        ("\tmove.w (sp),d0\n", "3017"),
        ("\tmove.w (4,a0,d1.w),d0\n", "3030 1004"),
        ("\tmovem.w d0/a0,-(sp)\n", "48A7 8080"),
    ] {
        assert_eq!(bytes(&format!("{M68K}{src}")), hex(asl), "{src}");
    }
    for (src, asl) in [
        ("\tld a,(hl)\n", "7E"),
        ("\tld b,c\n", "41"),
        ("\tld a,(ix+3)\n", "DD 7E 03"),
    ] {
        assert_eq!(bytes(&format!("{Z80}{src}")), hex(asl), "{src}");
    }
}
