//! T0 — prove the Core IR seam end-to-end with the smallest possible slice:
//! a `.emp` `data` item lowers to a `Module` whose linked image round-trips to
//! bytes, and a pointer field lands an `Abs32Be` fixup targeting the symbol.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Expr, Fixup, FixupKind, Fragment, SymbolTable};

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
