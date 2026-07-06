//! T0 — prove the Core IR seam end-to-end with the smallest possible slice:
//! a `.emp` `data` item lowers to a `Module` whose linked image round-trips to
//! bytes, and a pointer field lands an `Abs32Be` fixup targeting the symbol.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::expr::BinOp;
use sigil_ir::{Expr, Fixup, FixupKind, Fragment, SymbolTable};

/// The masked fixup target a `winptr(sym)` lowers to: `(sym & 0x7FFF) | 0x8000`
/// (AS `sfx_winptr`). Used by the windowed-pointer fixup-shape assertions below.
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

#[test]
fn roundtrip_bytes() {
    let (file, perrs) = parse_str("module m\ndata X: [u8; 3] = [1, 2, 3]\n");
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 });
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");

    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    let bytes = sigil_link::flatten(&linked, 0x00);
    assert_eq!(bytes, vec![0x01, 0x02, 0x03]);
}

#[test]
fn multibyte_scalar_is_big_endian() {
    // The seam's whole point: a width>1 scalar must serialize big-endian
    // (M68000 order). `u16 = $1234` → [0x12, 0x34].
    let (file, perrs) = parse_str("module m\ndata W: u16 = $1234\n");
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 });
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");

    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    let bytes = sigil_link::flatten(&linked, 0x00);
    assert_eq!(bytes, vec![0x12, 0x34]);
}

#[test]
fn symref_makes_abs32_fixup() {
    let src = "module m\n\
               comptime fn init() -> u8 { 0 }\n\
               struct Obj { code: *u8, flags: u8 }\n\
               data D: Obj = Obj{ code: init, flags: 3 }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (module, _diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 });

    // The point is the fixup SHAPE, not its resolution: an Abs32Be fixup at
    // offset 0 of the data fragment, targeting the symbol `init`.
    let section = module.sections.first().expect("one section");
    let fixups: Vec<&Fixup> = section
        .fragments
        .iter()
        .filter_map(|f| match f {
            Fragment::Data(d) => Some(&d.fixups),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(
        fixups,
        vec![&Fixup {
            kind: FixupKind::Abs32Be,
            offset: 0,
            target: Expr::Sym("init".into()),
        }]
    );
}

// ---- T2: per-CPU byte order and the SymRef → Fixup table ----------------

/// Collect all fixups across a module's first section's data fragments.
fn section_fixups(module: &sigil_ir::Module) -> Vec<Fixup> {
    module
        .sections
        .first()
        .expect("one section")
        .fragments
        .iter()
        .filter_map(|f| match f {
            Fragment::Data(d) => Some(d.fixups.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// Concatenate the raw (pre-link) bytes of a module's first section's data
/// fragments. Used when a fixup targets an undefined external symbol, so the
/// module cannot be linked but its serialized image (holes zero-filled) still
/// byte-diffs.
fn raw_data_bytes(module: &sigil_ir::Module) -> Vec<u8> {
    module
        .sections
        .first()
        .expect("one section")
        .fragments
        .iter()
        .filter_map(|f| match f {
            Fragment::Data(d) => Some(d.bytes.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// Link a module to its flat image bytes (mirrors T0's link pattern).
fn linked_bytes(module: &sigil_ir::Module) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

#[test]
fn scalar_byte_order_per_cpu() {
    // The seam's whole point: the SAME `DataBuf` serializes big-endian on a
    // 68000 section and little-endian on a Z80 section (§4.5 / §7.2).
    // `u16 = 258` (0x0102) → [0x01, 0x02] on 68k, [0x02, 0x01] on Z80.
    let (file, perrs) = parse_str("module m\ndata W: u16 = 258\n");
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (be_mod, be_diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 });
    assert!(be_diags.is_empty(), "unexpected 68k diagnostics: {be_diags:?}");
    assert_eq!(linked_bytes(&be_mod), vec![0x01, 0x02]);

    let (le_mod, le_diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::Z80 });
    assert!(le_diags.is_empty(), "unexpected Z80 diagnostics: {le_diags:?}");
    assert_eq!(linked_bytes(&le_mod), vec![0x02, 0x01]);
}

#[test]
fn symref_width4_68k_is_abs32be() {
    // A width-4 pointer in a 68000 section is an Abs32Be fixup (D-P4.5 row 1).
    let src = "module m\n\
               comptime fn init() -> u8 { 0 }\n\
               struct Obj { code: *u8, flags: u8 }\n\
               data D: Obj = Obj{ code: init, flags: 3 }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 });
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(
        section_fixups(&module),
        vec![Fixup { kind: FixupKind::Abs32Be, offset: 0, target: Expr::Sym("init".into()) }]
    );
}

#[test]
fn winptr_in_z80_is_bankptr16le() {
    // `winptr(sym)` in a Z80 section → 2 zero bytes + a BankPtr16Le fixup at
    // offset 0 targeting the WINDOW-MASKED symbol `(sfx & 0x7FFF) | 0x8000`
    // (D-P4.5 row 3, matching AS `sfx_winptr`).
    let src = "module m\n\
               comptime fn sfx() -> u8 { 0 }\n\
               data P = winptr(sfx)\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::Z80 });
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(
        section_fixups(&module),
        vec![Fixup { kind: FixupKind::BankPtr16Le, offset: 0, target: winptr_target("sfx") }]
    );
    // The 2-byte hole is present in the image (zero-filled before linking).
    let section = module.sections.first().expect("one section");
    let bytes: usize = section
        .fragments
        .iter()
        .filter_map(|f| match f {
            Fragment::Data(d) => Some(d.bytes.len()),
            _ => None,
        })
        .sum();
    assert_eq!(bytes, 2, "winptr reserves a 2-byte window pointer");
}

#[test]
fn winptr_in_68k_is_bankptr16be() {
    // A `winptr(sym)` in a 68000 section hits `(M68000, 2, true)` — a 68k
    // reference to a Z80 bank pointer, which T6 now represents with the new Core
    // `BankPtr16Be` kind (§7.2 / D-P4.7), the big-endian counterpart of
    // `BankPtr16Le`. (Was the T2 tripwire `winptr_in_68k_is_unsupported_...`.)
    let src = "module m\n\
               comptime fn sfx() -> u8 { 0 }\n\
               data P = winptr(sfx)\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 });
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(
        section_fixups(&module),
        vec![Fixup { kind: FixupKind::BankPtr16Be, offset: 0, target: winptr_target("sfx") }]
    );
    // The 2-byte hole is present in the image (zero-filled before linking).
    assert_eq!(raw_data_bytes(&module), vec![0x00, 0x00]);
}

#[test]
fn unwindowed_pointer_in_z80_is_error() {
    // A plain (un-windowed, width-4) pointer in a Z80 section is the
    // [cross-cpu.unwindowed-pointer] error naming the symbol (§7.2 / D-P4.5).
    let src = "module m\n\
               comptime fn init() -> u8 { 0 }\n\
               struct Obj { code: *u8, flags: u8 }\n\
               data D: Obj = Obj{ code: init, flags: 3 }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (_module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::Z80 });
    assert!(
        diags.iter().any(|d| d.message.contains("[cross-cpu.unwindowed-pointer]")
            && d.message.contains("init")),
        "expected an unwindowed-pointer diagnostic naming `init`, got: {diags:?}"
    );
}

#[test]
fn mixed_table_byte_diff_68k() {
    // A small mixed `data` table byte-diffed against a hand-computed reference
    // (§8.3): a u16, a pointer (kept at an even offset to avoid the odd-field
    // layout warning), then a u8. On 68k: u16 big-endian, then a 4-byte Abs32
    // hole (zero-filled pre-link), then the u8 → 7 bytes total.
    let src = "module m\n\
               comptime fn init() -> u8 { 0 }\n\
               struct Row { tag: u16, code: *u8, flag: u8 }\n\
               data R: Row = Row{ tag: $1234, code: init, flag: $7F }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");

    let (module, diags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 });
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    // tag=0x1234 (BE) | code=0x00000000 (Abs32 hole) | flag=0x7F. The pointer
    // targets an external `init`, so byte-diff the raw pre-link image.
    assert_eq!(raw_data_bytes(&module), vec![0x12, 0x34, 0x00, 0x00, 0x00, 0x00, 0x7F]);
    assert_eq!(
        section_fixups(&module),
        vec![Fixup { kind: FixupKind::Abs32Be, offset: 2, target: Expr::Sym("init".into()) }]
    );
}
