//! Plan 7 #7-main Task 1 — the `bank:` section attribute (R7m.1): parses
//! beside `cpu:`/`vma:`, evaluates as a comptime positive power-of-two
//! integer, and threads to `ir::Section.bank`. The field is INERT here —
//! nothing reads it until Task 2's placement seam.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, Section, SymbolTable};

/// Find a section by name in a lowered module (mirrors `lower_sections.rs`'s
/// helper — the default `text` sections are interleaved between placed ones).
fn section<'a>(module: &'a Module, name: &str) -> &'a Section {
    module
        .sections
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no section `{name}` in {:?}", module.sections.iter().map(|s| &s.name).collect::<Vec<_>>()))
}

#[test]
fn bank_attr_threads_to_section_bank() {
    let src = "module m\n\
               section s (bank: $8000) {\n\
                 data X: u8 = 0\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(diags.is_empty(), "lower: {diags:?}");
    let s = section(&module, "s");
    assert_eq!(s.bank, Some(0x8000));
}

#[test]
fn bank_attr_composes_with_cpu_and_vma_in_any_order() {
    let src = "module m\n\
               section s (vma: $8000, bank: $4000, cpu: z80) {\n\
                 data X: u8 = 0\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(diags.is_empty(), "lower: {diags:?}");
    let s = section(&module, "s");
    assert_eq!(s.bank, Some(0x4000));
    assert_eq!(s.vma_base, Some(0x8000));
    assert_eq!(s.cpu, Cpu::Z80);
}

#[test]
fn section_without_bank_attr_has_none() {
    let src = "module m\n\
               section s (vma: $8000) {\n\
                 data X: u8 = 0\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(diags.is_empty(), "lower: {diags:?}");
    let s = section(&module, "s");
    assert_eq!(s.bank, None);
}

#[test]
fn bank_attr_non_power_of_two_is_diagnosed() {
    let src = "module m\n\
               section s (bank: 3) {\n\
                 data X: u8 = 0\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(
        diags.iter().any(|d| d.message
            == "section `s` `bank:` must be a positive power-of-two comptime integer"),
        "expected the R7m.1 bank: diagnostic, got: {diags:?}"
    );
}

#[test]
fn bank_attr_zero_is_diagnosed() {
    let src = "module m\n\
               section s (bank: 0) {\n\
                 data X: u8 = 0\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(
        diags.iter().any(|d| d.message
            == "section `s` `bank:` must be a positive power-of-two comptime integer"),
        "expected the R7m.1 bank: diagnostic, got: {diags:?}"
    );
}

// ---- Task 3: general link-expr data cells (Cell::Expr + ValueN kinds, S2-D13f)
//
// A `Value::LinkExpr` landing in a data cell of declared width w ∈ {1,2,4} now
// lowers to `Cell::Expr` → a width/CPU-selected VALUE fixup folded at link and
// unsigned-window range-checked on write (R7m.4). A provisional here() is minted
// by a `jbra` to a far label; arithmetic on it (`here() + N`, `here() >> N`)
// produces the residual LinkExpr these tests emit. ------------------------------

/// Lower `src` requiring no lower-time errors (mirrors here_provisional.rs).
fn lower_ok(src: &str) -> Module {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (m, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    let errs: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(errs.is_empty(), "lower errors: {errs:?}");
    m
}

/// Full compile+link path: resolve_layout then link, return one section's bytes.
fn linked_bytes(m: &Module, section: &str) -> Vec<u8> {
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.section(section).map(|s| s.bytes.clone()).unwrap_or_default()
}

/// (a) An arithmetic LinkExpr (`here() + 2`) emitted at width 2 in a 68k section
/// produces `Value16Be` bytes of the FINAL folded value — byte-asserted through
/// the full resolve_layout+link path. The jbra grows to bra.w (4 bytes), so the
/// data item H sits at $8000 + 4 = $8004; here()+2 folds to $8006, big-endian.
#[test]
fn link_expr_width2_68k_folds_big_endian() {
    let src = "module m\n\
               section s (cpu: m68000, vma: $8000) {\n\
                 proc p () {\n\
                   jbra Far\n\
                 }\n\
                 data H: u16 = here() + 2\n\
                 data Pad = bytes(for i in 0..200 { 0 })\n\
                 proc Far () {\n\
                   rts\n\
                 }\n\
               }\n";
    let m = lower_ok(src);
    let bytes = linked_bytes(&m, "s");
    // bytes[0..4] = jbra bra.w; bytes[4..6] = H = ($8004 + 2) big-endian.
    assert_eq!(&bytes[4..6], &[0x80, 0x06], "here()+2 must fold to $8006 BE; got {:02X?}", &bytes[4..6]);
}

/// (b) Width 1 works (Value8): `here() >> 15` — the bank-id shift — into a u8
/// cell. H at $8004; $8004 >> 15 == 1.
#[test]
fn link_expr_width1_emits_value8() {
    let src = "module m\n\
               section s (cpu: m68000, vma: $8000) {\n\
                 proc p () {\n\
                   jbra Far\n\
                 }\n\
                 data H: u8 = here() >> 15\n\
                 data Pad = bytes(for i in 0..200 { 0 })\n\
                 proc Far () {\n\
                   rts\n\
                 }\n\
               }\n";
    let m = lower_ok(src);
    let bytes = linked_bytes(&m, "s");
    assert_eq!(bytes[4], 0x01, "here()>>15 at $8004 must fold to 1; got {:#X}", bytes[4]);
}

/// (c) A fold overflowing the width window is an Error naming the cell and the
/// value. A u16 cell holding `here() + $8000` folds to $8004 + $8000 = $10004,
/// which is ≥ $10000 (does not fit an unsigned 16-bit cell).
#[test]
fn link_expr_overflow_is_range_error() {
    let src = "module m\n\
               section s (cpu: m68000, vma: $8000) {\n\
                 proc p () {\n\
                   jbra Far\n\
                 }\n\
                 data H: u16 = here() + $8000\n\
                 data Pad = bytes(for i in 0..200 { 0 })\n\
                 proc Far () {\n\
                   rts\n\
                 }\n\
               }\n";
    let m = lower_ok(src);
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let err = sigil_link::link(&resolved, &SymbolTable::new()).expect_err("link must fail");
    assert!(
        err.iter().any(|d| d.message.contains("[value.out-of-range]")
            && d.message.contains("65540") // $10004
            && d.message.contains("16-bit")),
        "expected an unsigned-window range error naming the value, got: {err:?}"
    );
}

// (d) A Z80 section width-2 LinkExpr cell writes LITTLE-endian (Value16Le) — the
// R7m.5 Z80 probe. Z80 has no `jbra` (no relaxable → no provisional here() in a
// Z80 section until Task 4's `bankid`), so the CPU→endianness selection is
// proven at the two seams it actually lives in:
//   - the FRONTEND `stream_data` selection (a Z80 `Cell::Expr` → Value16Le), a
//     unit test in `lower/data.rs`;
//   - the LINKER `apply_fixup` write (Value16Le folds LE), a unit test in
//     `sigil-link/src/lib.rs`.
// Both are in-crate `#[cfg(test)]` modules (they touch pub(super)/private items);
// this comment records where the (d) evidence lives per the Task-3 plan.

#[test]
fn unknown_attr_diagnostics_unchanged_alongside_bank() {
    // Unknown-attribute diagnostics (naming the offending attr) must still fire
    // exactly as before, even in a section that also carries a `bank:`.
    let src = "module m\n\
               section s (bank: $8000, bogus: 1) {\n\
                 data X: u8 = 0\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(
        diags.iter().any(|d| d.message.contains("unknown attribute `bogus`")),
        "expected an unknown-attribute diagnostic, got: {diags:?}"
    );
}
