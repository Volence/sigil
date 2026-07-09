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
use sigil_ir::expr::BinOp;
use sigil_ir::{Expr, Fixup, FixupKind, Fragment, Module, Section, SymbolTable};

/// The masked fixup target a `winptr(sym)` lowers to: `(sym & 0x7FFF) | 0x8000`
/// (AS `sfx_winptr`, `SFX_WIN_MASK`/`SFX_WIN_BASE`).
fn winptr_target(name: &str) -> Expr {
    Expr::Binary {
        op: BinOp::Or,
        lhs: Box::new(Expr::Binary {
            op: BinOp::And,
            lhs: Box::new(Expr::Sym(name.into())),
            rhs: Box::new(Expr::Int(0x7FFF)),
        }),
        rhs: Box::new(Expr::Int(0x8000)),
    }
}

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
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
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
fn named_section_without_vma_has_no_pinned_vma_base() {
    // R7p.5 (Plan 7 item-7-pre Task 6): a NAMED section with NO `vma:` attribute
    // must get `vma_base == None` — the SAME "follow wherever it's placed"
    // contract the default (top-level items) section already has — never a
    // silently-defaulted `Some(0)` pin. `a` here has an explicit `vma: $0`
    // (a genuine pin, still `Some(0)` — unchanged by this fix); `b` omits
    // `vma:` entirely and must be `None`, with `vma_origin()` falling back to
    // its physical `lma` (2, right after `a`'s 2 bytes) rather than 0.
    let src = "module m\n\
               section a (cpu: m68000, vma: $0) {\n\
                 data Aval: u16 = $1111\n\
               }\n\
               section b (cpu: m68000) {\n\
                 data Bval: u16 = $2222\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
    assert!(diags.is_empty(), "lower: {diags:?}");

    let a = section(&module, "a");
    assert_eq!(a.vma_base, Some(0x0000), "explicit `vma: $0` stays a pin");

    let b = section(&module, "b");
    assert_eq!(b.vma_base, None, "no `vma:` attribute -> vma_base is None, not Some(0)");
    assert_eq!(b.lma, 2, "b's bytes physically follow a's 2 bytes");
    assert_eq!(
        b.vma_origin(),
        2,
        "vma_origin() falls back to lma (2) — Bval must NOT resolve from address 0"
    );
    assert_eq!(b.labels[0].name, "Bval");
    assert_eq!(b.vma_origin() + b.labels[0].offset, 2);
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
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
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
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
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
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
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
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
    assert!(diags.is_empty(), "lower: {diags:?}");
    assert_eq!(linked_section_bytes(&module, "text"), vec![0x00, 0x00]);
}

// ---- 3. cross-CPU fixup selection across real sections (§7.2) ----------------

#[test]
fn cross_cpu_bank_pointers_pick_le_and_be_by_section() {
    // One module, three sections: a `Sfx` symbol at a 68k-ROM-blob address
    // ($6569A — an out-of-window address, so the SFX bank-window mask is actually
    // EXERCISED), a z80 table with a windowed pointer to it (→ Value16Le, R-T0.5),
    // and a 68k table with the SAME windowed pointer (→ Value16Be). Each fixup
    // targets the MASKED tree `(Sfx & 0x7FFF) | 0x8000` (AS `sfx_winptr`); the two
    // write the resolved windowed value $D69A in OPPOSITE byte orders — IDENTICAL
    // bytes to the pre-R-T0.5 BankPtr16Le/Be path.
    let src = "module m\n\
               section sdata (cpu: z80, vma: $6569A) {\n\
                 data Sfx: u8 = $00\n\
               }\n\
               section ztab (cpu: z80, vma: $8000) {\n\
                 data ZP: u16 = winptr(\"Sfx\")\n\
               }\n\
               section mtab (cpu: m68000, vma: $C000) {\n\
                 data MP: u16 = winptr(\"Sfx\")\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
    assert!(diags.is_empty(), "lower: {diags:?}");

    // z80 windowed pointer → Value16Le, targeting the masked tree.
    assert_eq!(
        fixups_of(&module, "ztab"),
        vec![Fixup { kind: FixupKind::Value16Le, offset: 0, target: winptr_target("Sfx") }]
    );
    // 68k windowed pointer → Value16Be.
    assert_eq!(
        fixups_of(&module, "mtab"),
        vec![Fixup { kind: FixupKind::Value16Be, offset: 0, target: winptr_target("Sfx") }]
    );

    // Linked: Sfx VMA $6569A → ($6569A & 0x7FFF) | 0x8000 = $D69A.
    // LE $D69A → 9A D6 (ztab), BE $D69A → D6 9A (mtab). Unchanged bytes.
    assert_eq!(linked_section_bytes(&module, "ztab"), vec![0x9A, 0xD6]);
    assert_eq!(linked_section_bytes(&module, "mtab"), vec![0xD6, 0x9A]);
}


/// R-T0.5 — the L7.3 byte-identity condition: `data P: u16 = winptr("L")` in
/// BOTH a 68k and a Z80 section must produce IDENTICAL bytes before AND after
/// the winptr-over-link-exprs switch. The constants below are the bytes the
/// PRE-change winptr (`Cell::SymRef{windowed}` → `BankPtr16Be`/`BankPtr16Le`)
/// produced, captured on HEAD~ and PINNED here; the post-change winptr
/// (`Value::LinkExpr` → `Cell::Expr` → `Value16Be`/`Value16Le`) must match them
/// byte for byte.
///
/// `L` sits in a `vma: $6569A` section — an OUT-of-window address, so the mask
/// `(L & $7FFF) | $8000` = ($569A | $8000) = $D69A is genuinely exercised (not a
/// no-op on an already-windowed address). 68k writes it big-endian (`D6 9A`);
/// Z80 writes it little-endian (`9A D6`).
#[test]
fn winptr_data_cell_byte_identical_via_linkexpr() {
    let src = "module m\n\
               section anchor (cpu: z80, vma: $6569A) {\n\
                 data L: u8 = $00\n\
               }\n\
               section mtab (cpu: m68000, vma: $C000) {\n\
                 data MP: u16 = winptr(\"L\")\n\
               }\n\
               section ztab (cpu: z80, vma: $E000) {\n\
                 data ZP: u16 = winptr(\"L\")\n\
               }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
    assert!(diags.is_empty(), "lower: {diags:?}");

    // PINNED pre-change bytes (winptr($6569A) = $D69A): 68k big-endian, Z80
    // little-endian. These are the exact bytes HEAD~'s BankPtr16Be/Le path wrote.
    const MP_68K_BE: [u8; 2] = [0xD6, 0x9A];
    const ZP_Z80_LE: [u8; 2] = [0x9A, 0xD6];
    assert_eq!(linked_section_bytes(&module, "mtab"), MP_68K_BE, "68k winptr bytes must not drift");
    assert_eq!(linked_section_bytes(&module, "ztab"), ZP_Z80_LE, "Z80 winptr bytes must not drift");
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
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
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
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
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
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
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
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
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
    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
    assert!(
        diags.iter().any(|d| d.message.contains("[cross-cpu.unwindowed-pointer]")
            && d.message.contains("Target")),
        "expected an unwindowed-pointer diagnostic naming `Target`, got: {diags:?}"
    );
}

// ---- module `in <section>`: default section naming (Plan 7 #4) -------------

#[test]
fn module_in_section_names_default_section() {
    // `module m in obj_bank` with a top-level `data` item must lower that item
    // into a section NAMED `obj_bank` (not the literal default `text`).
    let src = "module m in obj_bank\ndata Blob: [u8; 2] = [1, 2]\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    assert!(diags.is_empty(), "lower: {diags:?}");
    // The default section carries the `in_section` name.
    let s = section(&module, "obj_bank");
    assert_eq!(s.image_len(), 2);
    // ...and there is NO literal `text` section.
    assert!(
        !module.sections.iter().any(|s| s.name == "text"),
        "expected no `text` section, got: {:?}",
        module.sections.iter().map(|s| &s.name).collect::<Vec<_>>()
    );
}
