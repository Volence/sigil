//! R-T0.3 — `equ NAME = expr` becomes a link-level symbol folded POST-placement.
//!
//! The emp front-end lowers each `equ` to an `ir::EquSym` on its carrier section;
//! `sigil_link::resolve_layout` folds them against the FINAL label VMAs (after the
//! placement⇄relaxation fixpoint), and `sigil_link::link` defines them as
//! `SymbolValue::Int` before applying fixups. These tests drive the whole
//! front-end → link seam.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Expr, Module, SymbolTable};

/// Lower `src`, asserting no diagnostics, and return the module.
fn lower_ok(src: &str) -> Module {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
    assert!(diags.is_empty(), "lower: {diags:?}");
    module
}

/// The single folded `EquSym` named `name` across all sections of `resolved`
/// (post `resolve_layout`), or panic.
fn folded_equ(resolved: &[sigil_ir::Section], name: &str) -> Expr {
    resolved
        .iter()
        .flat_map(|s| s.equ_syms.iter())
        .find(|e| e.name == name)
        .unwrap_or_else(|| panic!("no equ `{name}` found"))
        .expr
        .clone()
}

#[test]
fn equ_bankid_folds_to_symbol_at_link() {
    // A `(cpu: m68000, vma: $58000)` section with a data label `L` (VMA pinned at
    // $58000), plus three equs referencing it:
    //   equ B = bankid("L")   → ($58000 & $7F8000) >> 15 = 0xB
    //   equ P = winptr("L")    → ($58000 & $7FFF) | $8000  = $8000
    //   equ N = 3 + 3          → 6 (a comptime int)
    let src = "module m\n\
               section blob (cpu: m68000, vma: $58000) {\n\
                 data L: [u8;6] = [$11,$22,$33,$44,$55,$66]\n\
                 equ B = bankid(\"L\")\n\
                 equ P = winptr(\"L\")\n\
                 equ N = 3 + 3\n\
               }\n";
    let module = lower_ok(src);

    // Three equ_syms attached to the `blob` section, un-folded (raw trees) here.
    let blob = module.sections.iter().find(|s| s.name == "blob").expect("blob section");
    assert_eq!(blob.equ_syms.len(), 3, "three equs attach to blob");

    // After resolve_layout they fold to concrete ints against L's final VMA.
    let resolved =
        sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true).expect("layout");
    assert_eq!(folded_equ(&resolved, "B"), Expr::Int(0xB), "B = bankid($58000) = 0xB");
    assert_eq!(folded_equ(&resolved, "P"), Expr::Int(0x8000), "P = winptr($58000) = $8000");
    assert_eq!(folded_equ(&resolved, "N"), Expr::Int(6), "N = 3 + 3 = 6");

    // And link defines them (Pass-1b) without error.
    let _linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
}

#[test]
fn equ_string_value_is_equ_value_diagnostic() {
    // `equ S = "x"` — a string is neither an integer nor a link-time expression.
    let src = "module m\n\
               equ S = \"x\"\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_module, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] });
    assert!(
        diags.iter().any(|d| d.message.contains("[equ.value]")
            && d.message.contains("integer or a link-time expression")),
        "expected an [equ.value] diagnostic, got: {diags:?}"
    );
}

#[test]
fn plain_comptime_int_equ_folds_to_int() {
    // A top-level `equ` (attaches to the default `text` carrier) with a pure
    // comptime int value.
    let src = "module m\n\
               equ K = $2A\n\
               data D: u8 = 0\n";
    let module = lower_ok(src);
    let resolved =
        sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true).expect("layout");
    assert_eq!(folded_equ(&resolved, "K"), Expr::Int(0x2A));
}
