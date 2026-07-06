//! T6 — `section {}` placement (§7.1), the `here()` builtin, and cross-CPU
//! fixup-kind selection exercised ACROSS real sections (§7.2 / D-P4.11).
//!
//! Placement policy (emp's own, map-file regions being S2-D3-deferred): a
//! section's bytes land at the next physical LMA (a continuous counter across
//! sections in declaration order), while its labels/PC compute at its explicit
//! `vma:` base. Top-level items stay in the default `text` section.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Fixup, FixupKind, Fragment, Module, Section, SymbolTable};

/// Find a section by name in a lowered module (the default `text` sections are
/// interleaved between placed sections, so `first()` is not enough here).
fn section<'a>(module: &'a Module, name: &str) -> &'a Section {
    module
        .sections
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no section `{name}` in {:?}", module.sections.iter().map(|s| &s.name).collect::<Vec<_>>()))
}

/// All fixups across a named section's data fragments.
fn fixups_of(module: &Module, name: &str) -> Vec<Fixup> {
    section(module, name)
        .fragments
        .iter()
        .filter_map(|f| match f {
            Fragment::Data(d) => Some(d.fixups.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// Link a module and return a named section's resolved bytes.
fn linked_section_bytes(module: &Module, name: &str) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.section(name).expect("linked section").bytes.clone()
}

// ---- 1. placement: vma bases, continuous LMA, cross-section reference --------

#[test]
fn two_sections_place_at_vma_and_continuous_lma() {
    // Section `a` (68k, vma 0) emits a u16 (2 bytes); section `b` (z80, vma
    // $8000) emits a u16. The physical LMA counter is continuous: `a` at lma 0,
    // `b` at lma 2. VMA bases are the explicit `vma:` values.
    let src = "module m\n\
               section a (cpu: m68000, vma: $0) {\n\
                 data Aval: u16 = $1111\n\
               }\n\
               section b (cpu: z80, vma: $8000) {\n\
                 data Bval: u16 = $2222\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 });
    assert!(diags.is_empty(), "lower: {diags:?}");

    let a = section(&module, "a");
    assert_eq!(a.vma_base, Some(0x0000));
    assert_eq!(a.lma, 0);
    assert_eq!(a.cpu, Cpu::M68000);

    let b = section(&module, "b");
    assert_eq!(b.vma_base, Some(0x8000));
    assert_eq!(b.lma, 2, "b's bytes follow a's 2 bytes physically");
    assert_eq!(b.cpu, Cpu::Z80);

    // Label VMAs: Aval at $0, Bval at $8000 (section-relative offset 0 + origin).
    assert_eq!(a.labels[0].name, "Aval");
    assert_eq!(a.vma_origin() + a.labels[0].offset, 0x0000);
    assert_eq!(b.labels[0].name, "Bval");
    assert_eq!(b.vma_origin() + b.labels[0].offset, 0x8000);

    // z80 section serializes little-endian: $2222 → 22 22 (palindrome-safe: use
    // the byte order check below on a non-palindrome elsewhere). Here just prove
    // the byte order split: 68k `a` is big-endian $1111 → 11 11.
    assert_eq!(linked_section_bytes(&module, "a"), vec![0x11, 0x11]);
    assert_eq!(linked_section_bytes(&module, "b"), vec![0x22, 0x22]);
}

#[test]
fn cross_section_pointer_resolves_to_target_vma() {
    // Section `a` (68k) holds a pointer to `Bval`, defined in section `b` at VMA
    // $8000. The Abs32Be fixup resolves across sections to $00008000.
    let src = "module m\n\
               section a (cpu: m68000, vma: $0) {\n\
                 data Ptr: *u8 = \"Bval\"\n\
               }\n\
               section b (cpu: z80, vma: $8000) {\n\
                 data Bval: u16 = $2222\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 });
    assert!(diags.is_empty(), "lower: {diags:?}");

    assert_eq!(
        fixups_of(&module, "a"),
        vec![Fixup {
            kind: FixupKind::Abs32Be,
            offset: 0,
            target: sigil_ir::Expr::Sym("Bval".into()),
        }]
    );
    // Linked: Bval's VMA $00008000, big-endian.
    assert_eq!(linked_section_bytes(&module, "a"), vec![0x00, 0x00, 0x80, 0x00]);
}

#[test]
fn z80_section_serializes_little_endian() {
    // A non-palindrome value in a z80 section proves the CPU flows to the
    // streamer's byte order: $1234 → 34 12 (LE).
    let src = "module m\n\
               section z (cpu: z80, vma: $8000) {\n\
                 data W: u16 = $1234\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 });
    assert!(diags.is_empty(), "lower: {diags:?}");
    assert_eq!(linked_section_bytes(&module, "z"), vec![0x34, 0x12]);
}

// ---- 2. here() -------------------------------------------------------------

#[test]
fn here_resolves_to_item_start_vma_and_advances() {
    // Two data items in a vma:$8000 section. `here()` in the first reads $8000;
    // in the second, $8002 (advanced past the first's 2 bytes). 68k big-endian.
    let src = "module m\n\
               section s (cpu: m68000, vma: $8000) {\n\
                 data H0: u16 = here()\n\
                 data H1: u16 = here()\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 });
    assert!(diags.is_empty(), "lower: {diags:?}");
    // H0 = $8000 → 80 00 ; H1 = $8002 → 80 02.
    assert_eq!(linked_section_bytes(&module, "s"), vec![0x80, 0x00, 0x80, 0x02]);
}

#[test]
fn here_outside_a_placed_section_uses_default_origin() {
    // A top-level `data` (default `text` section, vma == lma == 0): here() == 0.
    let src = "module m\n\
               data H: u16 = here()\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 });
    assert!(diags.is_empty(), "lower: {diags:?}");
    assert_eq!(linked_section_bytes(&module, "text"), vec![0x00, 0x00]);
}

// ---- 3. cross-CPU fixup selection across real sections (§7.2) ----------------

#[test]
fn cross_cpu_bank_pointers_pick_le_and_be_by_section() {
    // One module, three sections: a z80 `Sfx` symbol, a z80 table with a
    // windowed pointer to it (→ BankPtr16Le), and a 68k table with the SAME
    // windowed pointer (→ BankPtr16Be, the new Core kind). The two write the
    // resolved low-16 in OPPOSITE byte orders.
    let src = "module m\n\
               section sdata (cpu: z80, vma: $1234) {\n\
                 data Sfx: u8 = $00\n\
               }\n\
               section ztab (cpu: z80, vma: $8000) {\n\
                 data ZP = winptr(\"Sfx\")\n\
               }\n\
               section mtab (cpu: m68000, vma: $C000) {\n\
                 data MP = winptr(\"Sfx\")\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 });
    assert!(diags.is_empty(), "lower: {diags:?}");

    // z80 windowed pointer → BankPtr16Le.
    assert_eq!(
        fixups_of(&module, "ztab"),
        vec![Fixup { kind: FixupKind::BankPtr16Le, offset: 0, target: sigil_ir::Expr::Sym("Sfx".into()) }]
    );
    // 68k reference to a bank pointer → BankPtr16Be (the new Core kind).
    assert_eq!(
        fixups_of(&module, "mtab"),
        vec![Fixup { kind: FixupKind::BankPtr16Be, offset: 0, target: sigil_ir::Expr::Sym("Sfx".into()) }]
    );

    // Linked: Sfx VMA $1234 → LE 34 12 (ztab), BE 12 34 (mtab).
    assert_eq!(linked_section_bytes(&module, "ztab"), vec![0x34, 0x12]);
    assert_eq!(linked_section_bytes(&module, "mtab"), vec![0x12, 0x34]);
}

// ---- negative paths: attribute + here() diagnostics -------------------------

#[test]
fn unknown_section_attribute_is_diagnosed() {
    let src = "module m\n\
               section s (foo: 1, vma: $8000) {\n\
                 data X: u8 = 0\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 });
    assert!(
        diags.iter().any(|d| d.message.contains("unknown attribute `foo`")),
        "expected an unknown-attribute diagnostic, got: {diags:?}"
    );
}

#[test]
fn non_integer_vma_is_diagnosed() {
    // A non-integer `vma:` value → the "is not a comptime integer" diagnostic,
    // pointed at the value expression's own span.
    let src = "module m\n\
               section s (vma: \"nope\") {\n\
                 data X: u8 = 0\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 });
    assert!(
        diags.iter().any(|d| d.message.contains("`vma:` is not a comptime integer")),
        "expected a non-integer vma diagnostic, got: {diags:?}"
    );
}

#[test]
fn here_with_arguments_is_arity_error() {
    // `here()` takes no arguments; `here(5)` is an arity error. Reached with a
    // valid `here_base` (the data item is inside a placed section).
    let src = "module m\n\
               section s (cpu: m68000, vma: $8000) {\n\
                 data H: u16 = here(5)\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 });
    assert!(
        diags.iter().any(|d| d.message.contains("`here` takes no arguments")),
        "expected a here() arity diagnostic, got: {diags:?}"
    );
}

#[test]
fn here_outside_a_lowering_context_is_error() {
    // `here()` in a section `vma:` attribute runs through `eval_attr_int`, whose
    // evaluator has no `here_base` set — the genuinely-None path. It reports the
    // "only valid inside a section during lowering" error (and, downstream, the
    // non-integer-vma error since `here()` poisons to a non-int).
    let src = "module m\n\
               section s (vma: here()) {\n\
                 data X: u8 = 0\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 });
    assert!(
        diags.iter().any(|d| d.message.contains("only valid inside a section during lowering")),
        "expected a here()-outside-lowering diagnostic, got: {diags:?}"
    );
}

#[test]
fn unwindowed_pointer_in_z80_section_is_error() {
    // A plain (un-windowed, width-4) pointer in a z80 section is the
    // [cross-cpu.unwindowed-pointer] error naming the symbol (§7.2 / D-P4.5).
    let src = "module m\n\
               section z (cpu: z80, vma: $8000) {\n\
                 data BadP: *u8 = \"Target\"\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 });
    assert!(
        diags.iter().any(|d| d.message.contains("[cross-cpu.unwindowed-pointer]")
            && d.message.contains("Target")),
        "expected an unwindowed-pointer diagnostic naming `Target`, got: {diags:?}"
    );
}
