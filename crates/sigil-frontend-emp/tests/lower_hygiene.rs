//! T5 (Plan 4) — label hygiene finalization (§5.2/§5.3, D-P4.6). Proves the
//! finished model end-to-end through `lower_module` + link:
//!
//! - two procs that each define a non-export `.loop:` get DISTINCT owner-scoped
//!   symbols (`$m$foo$loop` / `$m$bar$loop`, module-qualified per Plan 7 #4) — no
//!   collision — and the module links
//!   (the whole-branch review's CRITICAL: the most common Sonic-asm idiom);
//! - the same comptime-fn-generated `asm { }` (with a local label) called from
//!   two different procs links cleanly — the instantiation counter `k` is
//!   globally monotonic across procs, so the two internal labels stay distinct;
//! - an `export .entry:` in a `proc foo` is caller-visible as `foo.entry` and a
//!   `bra.w foo.entry` from another proc resolves to it;
//! - a reference to a NON-export label from outside its scope does NOT resolve
//!   (link reports it unresolved) — hygiene actually hides it.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};

/// Lower a module (parse asserted clean) for the 68k.
fn lower(src: &str) -> (Module, Vec<sigil_span::Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] })
}

/// The label names defined across every section of a lowered module.
fn all_labels(module: &Module) -> Vec<String> {
    module.sections.iter().flat_map(|s| s.labels.iter().map(|l| l.name.clone())).collect()
}

#[test]
fn two_procs_with_same_local_label_get_distinct_symbols_and_link() {
    // THE whole-branch CRITICAL: two procs that each define `.loop:` (the most
    // common Sonic idiom) must NOT collide. Each proc's local label is scoped to
    // its owner (`$foo$loop` / `$bar$loop`), so lowering emits distinct symbols
    // and the module links (no `redefined` error).
    let src = "module m\n\
               proc foo() {\n.loop:\n    bra.w .loop\n}\n\
               proc bar() {\n.loop:\n    bra.w .loop\n}\n";
    let (module, diags) = lower(src);
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");

    let labels = all_labels(&module);
    // Local labels are module-qualified (`module m` → `$m$…`) so identical proc
    // names in different modules don't collide (Plan 7 #4).
    assert!(labels.contains(&"$m$foo$loop".to_string()), "expected $m$foo$loop, got {labels:?}");
    assert!(labels.contains(&"$m$bar$loop".to_string()), "expected $m$bar$loop, got {labels:?}");

    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    // Both branches resolve intra-proc (label and bra at the same offset → -2).
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link must succeed");
    let bytes = sigil_link::flatten(&linked, 0x00);
    assert_eq!(bytes, vec![0x60, 0x00, 0xFF, 0xFE, 0x60, 0x00, 0xFF, 0xFE]);
}

#[test]
fn same_asm_template_from_two_procs_links_cleanly() {
    // A comptime fn returns an `asm { }` with an internal `.wait:`; two different
    // procs each splice it via a statement-call. Because the instantiation counter
    // is globally monotonic across procs (not reset per proc), the two `.wait`
    // instantiations mint DISTINCT `$asm{k}$wait` symbols — so the module links
    // (guards requirement 2: `asm {}`-in-two-procs must not collide).
    let src = "module m\n\
               comptime fn spin() -> Code {\n    return asm {\n.wait:\n    bra.w .wait\n    }\n}\n\
               proc foo() {\n    spin()\n    rts\n}\n\
               proc bar() {\n    spin()\n    rts\n}\n";
    let (module, diags) = lower(src);
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");

    // Two DISTINCT mangled `.wait` symbols (one per instantiation).
    let waits: Vec<String> =
        all_labels(&module).into_iter().filter(|n| n.ends_with("$wait")).collect();
    assert_eq!(waits.len(), 2, "expected two distinct wait labels, got {waits:?}");
    assert_ne!(waits[0], waits[1], "the two asm instantiations must not share a symbol: {waits:?}");

    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    sigil_link::link(&resolved, &SymbolTable::new()).expect("link must succeed");
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
