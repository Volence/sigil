//! T5 (Plan 4) — label hygiene finalization (§5.2/§5.3, D-P4.6). Proves the
//! finished model end-to-end through link:
//!
//! - two instantiations of the SAME `asm { }` template define DISTINCT internal
//!   (non-export) labels — no collision — and both round-trip through link;
//! - an `export .entry:` in a `proc foo` is caller-visible as `foo.entry` and a
//!   `bra.w foo.entry` from another proc resolves to it;
//! - a reference to a NON-export label from outside its scope does NOT resolve
//!   (link reports it unresolved) — hygiene actually hides it.

use sigil_frontend_emp::ast::{self, Expr, Stmt};
use sigil_frontend_emp::eval::{Env, Evaluator};
use sigil_frontend_emp::lower::{lower_code_buf, lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::{CodeBuf, CodeItem, Value};
use sigil_ir::backend::{Cpu, IrStreamer};
use sigil_ir::{IrBuilder, Module, SymbolTable};

/// Pull the `asm { }` expression out of a one-fn module's `return asm { ... }`.
fn asm_expr(src: &str) -> Expr {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "unexpected parse diagnostics: {diags:?}");
    for item in file.items {
        if let ast::Item::ComptimeFn(f) = item {
            for stmt in f.body {
                if let Stmt::Return { value: Some(e), .. } = stmt {
                    if matches!(e, Expr::Asm { .. }) {
                        return e;
                    }
                }
            }
        }
    }
    panic!("no `asm {{ }}` expression found in source");
}

/// The label-definition symbols of a `CodeBuf`, in order.
fn label_names(buf: &CodeBuf) -> Vec<String> {
    buf.items
        .iter()
        .filter_map(|it| match it {
            CodeItem::Label { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

/// Lower a module (parse asserted clean) for the 68k.
fn lower(src: &str) -> (Module, Vec<sigil_span::Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000 })
}

#[test]
fn two_instantiations_define_distinct_internal_labels_and_both_link() {
    // The SAME template evaluated twice (shared evaluator → the instantiation
    // counter advances) must mint TWO distinct `.wait` symbols, so concatenating
    // both fragments into one section links WITHOUT a duplicate-symbol error.
    let src = "module m\ncomptime fn f() -> Code {\n    return asm {\n    .wait:\n        bra.w .wait\n    }\n}\n";
    let e = asm_expr(src);
    let mut ev = Evaluator::new();
    let mut env = Env::new();
    let v1 = ev.eval_expr(&e, &mut env);
    let v2 = ev.eval_expr(&e, &mut env);
    assert!(ev.diags.is_empty(), "unexpected eval diagnostics: {:?}", ev.diags);
    let (Value::Code(a), Value::Code(b)) = (&v1, &v2) else {
        panic!("expected two Value::Code");
    };

    // Distinct internal labels — the hygiene guarantee.
    let (la, lb) = (label_names(a), label_names(b));
    assert_eq!(la.len(), 1);
    assert_eq!(lb.len(), 1);
    assert_ne!(la[0], lb[0], "two instantiations must not share a label symbol: {la:?} vs {lb:?}");
    assert!(la[0].starts_with("$asm"), "non-export label should be mangled: {}", la[0]);

    // Both fragments in ONE section link cleanly (no collision, each branch
    // resolves to its own `.wait`).
    let mut builder = IrBuilder::new();
    builder.switch_section("text", Cpu::M68000, None);
    let mut diags = Vec::new();
    lower_code_buf(a, Cpu::M68000, &mut builder, &mut diags);
    lower_code_buf(b, Cpu::M68000, &mut builder, &mut diags);
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");
    let (module, bdiags) = builder.finish();
    assert!(bdiags.is_empty(), "unexpected builder diagnostics: {bdiags:?}");
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    // buf1: .wait@0, bra.w@0 (disp -2). buf2: .wait@4, bra.w@4 (disp -2).
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link must succeed");
    let bytes = sigil_link::flatten(&linked, 0x00);
    assert_eq!(bytes, vec![0x60, 0x00, 0xFF, 0xFE, 0x60, 0x00, 0xFF, 0xFE]);
}

#[test]
fn exported_proc_label_is_referenceable_as_owner_dot_name() {
    // `export .entry:` in `proc foo` is caller-visible as `foo.entry`; a
    // `bra.w foo.entry` in `proc bar` resolves to it and links to the right
    // displacement.
    let src = "module m\n\
               proc foo() {\n    export .entry:\n    rts\n}\n\
               proc bar() {\n    bra.w foo.entry\n    rts\n}\n";
    let (module, diags) = lower(src);
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");

    // The exported label is emitted under its stable `foo.entry` symbol.
    let section = module.sections.first().expect("one section");
    assert!(
        section.labels.iter().any(|l| l.name == "foo.entry"),
        "expected a `foo.entry` label, got {:?}",
        section.labels
    );

    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link must succeed");
    let bytes = sigil_link::flatten(&linked, 0x00);
    // foo: rts (4E 75) @0, foo.entry@0. bar@2: bra.w foo.entry (60 00 + disp16),
    // disp word @4, target @0 → disp = 0 - (2+2) = -4 = 0xFFFC; then rts @6.
    assert_eq!(bytes, vec![0x4E, 0x75, 0x60, 0x00, 0xFF, 0xFC, 0x4E, 0x75]);
}

#[test]
fn reference_to_non_export_label_from_outside_does_not_resolve() {
    // `.wait:` in `proc foo` is NOT exported, so it is hidden under a mangled
    // symbol — a `bra.w foo.wait` from `proc bar` references a `foo.wait` that
    // was never defined, so link reports it unresolved.
    let src = "module m\n\
               proc foo() {\n.wait:\n    bra.w .wait\n}\n\
               proc bar() {\n    bra.w foo.wait\n    rts\n}\n";
    let (module, _diags) = lower(src);

    // The hidden label must NOT be emitted under the caller-spellable name.
    let section = module.sections.first().expect("one section");
    assert!(
        !section.labels.iter().any(|l| l.name == "foo.wait"),
        "a non-export label must not be caller-visible as `foo.wait`: {:?}",
        section.labels
    );

    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let err = sigil_link::link(&resolved, &SymbolTable::new())
        .expect_err("link must fail: `foo.wait` is hidden and unresolved");
    assert!(
        err.iter().any(|d| d.message.contains("unresolved")),
        "expected an unresolved-symbol diagnostic, got: {err:?}"
    );
}
