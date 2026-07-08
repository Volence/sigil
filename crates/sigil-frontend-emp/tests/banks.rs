//! Plan 7 #7-main Task 1 — the `bank:` section attribute (R7m.1): parses
//! beside `cpu:`/`vma:`, evaluates as a comptime positive power-of-two
//! integer, and threads to `ir::Section.bank`. The field is INERT here —
//! nothing reads it until Task 2's placement seam.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, Section};

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
