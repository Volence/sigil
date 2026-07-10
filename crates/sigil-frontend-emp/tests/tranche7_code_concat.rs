//! The `Code` monoid `++` (§6.2, tranche 7): `asm { } ++ asm { }` composes
//! fragments in emission order. Demanded by the aabb template's conditional
//! lead instruction (`let head = if aliased { asm { } } else { asm { move … } };
//! return head ++ asm { …body… }`) — the emp twin of an AS macro `if` guard.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_span::Level;

fn emp_bytes(src: &str) -> Vec<u8> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    let errs: Vec<_> = diags.iter().filter(|d| d.level == Level::Error).collect();
    assert!(errs.is_empty(), "lower diagnostics: {errs:?}");
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.sections.iter().flat_map(|s| s.bytes.clone()).collect()
}

/// Concat of two non-empty fragments emits both, in order, byte-identical to
/// the single-block spelling.
#[test]
fn code_concat_appends_in_emission_order() {
    let split = emp_bytes(
        "module t in t\n\
         comptime fn f() -> Code {\n\
             let head = asm { moveq   #1, d0 }\n\
             return head ++ asm { moveq   #2, d1\n\
                 rts }\n\
         }\n\
         pub proc P () {\n\
             f()\n\
         }\n",
    );
    let joined = emp_bytes(
        "module t in t\n\
         pub proc P () {\n\
                 moveq   #1, d0\n\
                 moveq   #2, d1\n\
                 rts\n\
         }\n",
    );
    assert_eq!(split, joined, "split ++ spelling must equal the single block");
}

/// The conditional-head shape: an EMPTY `asm { }` head is the monoid identity —
/// concat with it emits only the tail.
#[test]
fn empty_code_is_concat_identity() {
    let with_empty_head = emp_bytes(
        "module t in t\n\
         comptime fn f(skip: bool) -> Code {\n\
             let head = if skip { asm { } } else { asm { moveq   #1, d0 } }\n\
             return head ++ asm { rts }\n\
         }\n\
         pub proc P () {\n\
             f(true)\n\
         }\n",
    );
    let bare = emp_bytes(
        "module t in t\n\
         pub proc P () {\n\
                 rts\n\
         }\n",
    );
    assert_eq!(with_empty_head, bare, "empty head must vanish");
}

/// Each `asm { }` block is its own hygiene scope, and `++` composes ITEMS, not
/// label spaces: a tail fragment's branch to a label defined in the head
/// fragment is a LOUD unresolved-symbol error, never silent wrong bytes.
/// (Pinned semantics — fn-call-scoped hygiene is a ledgered ask; a template
/// needing a shared label keeps it in ONE fragment, like the aabb head shape.)
#[test]
fn cross_fragment_label_fails_loudly() {
    let src = "module t in t\n\
         comptime fn f() -> Code {\n\
             let head = asm { .top:\n\
                 nop }\n\
             return head ++ asm { bra.s   .top }\n\
         }\n\
         pub proc P () {\n\
             f()\n\
         }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    let errs: Vec<_> = diags.iter().filter(|d| d.level == Level::Error).collect();
    if errs.is_empty() {
        let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
            .expect("resolve_layout");
        let err = sigil_link::link(&resolved, &SymbolTable::new())
            .expect_err("cross-fragment label must NOT silently resolve");
        assert!(
            format!("{err:?}").contains("unresolved symbol"),
            "must fail as an unresolved symbol, got: {err:?}"
        );
    }
}
