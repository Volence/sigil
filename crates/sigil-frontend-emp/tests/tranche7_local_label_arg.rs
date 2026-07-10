//! Tranche 7 — F2: proc-local label VALUES in call arguments.
//!
//! `axis_test(d4, ..., .next_object)` — a dot-prefixed proc-LOCAL label passed as
//! a `Label`-typed call argument. It parses as `Expr::LocalLabel`, and in
//! label-value context resolves through the ENCLOSING proc body's hygienic
//! local-label naming (the same mangled `$module$Proc$name` scheme a `.name:`
//! written in that proc gets), so the argument splices into branch position as a
//! `CodeOperand::Sym` exactly like a directly-written local branch. Forward
//! references work. A typo'd `.labell` is a LOUD compile error naming it. In a
//! pure comptime expression position (`const x = .foo`) the form is rejected —
//! it never leaks a silent Label.

use sigil_frontend_emp::ast::*;
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use sigil_span::{Diagnostic, Level};

fn lower(src: &str) -> (sigil_ir::Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    )
}

fn errors(diags: &[Diagnostic]) -> Vec<&str> {
    diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.as_str()).collect()
}

fn section<'a>(module: &'a sigil_ir::Module, name: &str) -> &'a Section {
    module.sections.iter().find(|s| s.name == name).unwrap_or_else(|| panic!("no section `{name}`"))
}

fn linked_section_bytes(module: &sigil_ir::Module, name: &str) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.section(name).expect("linked section").bytes.clone()
}

// ---- F2 parse: `.name` is an Expr::LocalLabel ------------------------------

#[test]
fn dot_name_parses_as_local_label_in_call_arg() {
    // A call argument `.next` must parse (today it is "expected an expression,
    // found Dot") as an Expr::LocalLabel.
    let (file, diags) = parse_str(concat!(
        "module m\n",
        "comptime fn t(mlab: Label) -> Code {\n",
        "    return asm { bra.s {mlab} }\n",
        "}\n",
        "pub proc P () {\n",
        "    t(.next)\n",
        ".next:\n",
        "    rts\n",
        "}\n",
    ));
    assert!(diags.is_empty(), "parse diagnostics: {diags:?}");
    // Find the `t(.next)` call in proc P's body and check the arg is a LocalLabel.
    let Item::Proc(p) = file.items.iter().find(|i| matches!(i, Item::Proc(_))).unwrap() else {
        panic!()
    };
    let mut saw = false;
    for stmt in &p.body {
        if let AsmStmt::Call(Expr::Call { args, .. }) = stmt {
            if let Some(a) = args.first() {
                if matches!(&a.value, Expr::LocalLabel(n, _) if n == "next") {
                    saw = true;
                }
            }
        }
    }
    assert!(saw, "expected an Expr::LocalLabel(\"next\") call argument");
}

// ---- F2 eval / bytes: local-label arg == directly-written local branch ------
//
// A template that branches to a caller-supplied local label must produce the
// SAME bytes as the caller writing that branch directly.

const ARG_SRC: &str = concat!(
    "module m\n",
    "comptime fn jump(mlab: Label) -> Code {\n",
    "    return asm {\n",
    "        bra.w   {mlab}\n",
    "    }\n",
    "}\n",
    "pub proc P () {\n",
    "    jump(.done)\n",
    "    nop\n",
    ".done:\n",
    "    rts\n",
    "}\n",
);

const DIRECT_SRC: &str = concat!(
    "module m\n",
    "pub proc P () {\n",
    "    bra.w   .done\n",
    "    nop\n",
    ".done:\n",
    "    rts\n",
    "}\n",
);

#[test]
fn local_label_arg_bytes_match_direct_branch() {
    let (am, ad) = lower(ARG_SRC);
    assert!(errors(&ad).is_empty(), "arg lower errors: {:?}", errors(&ad));
    let (dm, dd) = lower(DIRECT_SRC);
    assert!(errors(&dd).is_empty(), "direct lower errors: {:?}", errors(&dd));
    let _ = section(&am, "text");
    let abytes = linked_section_bytes(&am, "text");
    let dbytes = linked_section_bytes(&dm, "text");
    assert_eq!(abytes, dbytes, "local-label arg branch must equal direct branch bytes");
}

// ---- F2 forward reference ---------------------------------------------------
// The `.done` label is DEFINED AFTER the call site — resolution to the mangled
// name must not require the definition to have been seen. Already exercised by
// ARG_SRC above (`.done:` follows `jump(.done)`), pinned explicitly here.

#[test]
fn local_label_arg_forward_reference_links() {
    let (am, ad) = lower(ARG_SRC);
    assert!(errors(&ad).is_empty(), "forward-ref errors: {:?}", errors(&ad));
    // Link must succeed (the mangled symbol resolves against the later `.done:`).
    let _ = linked_section_bytes(&am, "text");
}

// ---- F2 hygiene: caller's label space is distinct from callee template's -----
//
// The template defines its OWN internal `.aov` label AND receives a caller's
// `.next` label as an arg. The two must live in DISTINCT label spaces: the
// template's `.aov` mangles under its fresh-per-instantiation asm-counter owner,
// the `.next` arg mangles under the CALLER proc's owner. If they collided, the
// caller's `.next` would resolve into the template's fresh space and mislink.

const HYGIENE_SRC: &str = concat!(
    "module m\n",
    "comptime fn axis(stmp: Reg, mlab: Label) -> Code {\n",
    "    return asm {\n",
    "        move.w  {stmp}, {stmp}\n",
    "        bpl.s   .aov\n",
    "        neg.w   {stmp}\n",
    "    .aov:\n",
    "        cmp.w   {stmp}, {stmp}\n",
    "        bhs.w   {mlab}\n",
    "    }\n",
    "}\n",
    "pub proc P () {\n",
    "    axis(d2, .next)\n",
    "    nop\n",
    ".next:\n",
    "    rts\n",
    "}\n",
);

#[test]
fn caller_local_label_distinct_from_template_internal_label() {
    let (m, d) = lower(HYGIENE_SRC);
    assert!(errors(&d).is_empty(), "hygiene lower errors: {:?}", errors(&d));
    // Link resolves both the template's internal `.aov` AND the caller's `.next`.
    let bytes = linked_section_bytes(&m, "text");
    assert!(!bytes.is_empty());
    // The section must carry BOTH distinct mangled symbols as labels.
    let s = section(&m, "text");
    let has_aov = s.labels.iter().any(|l| l.name.contains("aov"));
    let has_next = s.labels.iter().any(|l| l.name.contains("next"));
    assert!(has_aov, "template's internal .aov label missing: {:?}", s.labels);
    assert!(has_next, "caller's .next label missing: {:?}", s.labels);
    // And they are DIFFERENT symbols (distinct owners).
    let aov = s.labels.iter().find(|l| l.name.contains("aov")).unwrap();
    let next = s.labels.iter().find(|l| l.name.contains("next")).unwrap();
    assert_ne!(aov.name, next.name);
}

// ---- F2 negative: typo'd local-label arg is LOUD ----------------------------

#[test]
fn unknown_local_label_arg_is_loud_naming_it() {
    // `.donee` is never defined in the caller — must be a loud compile error
    // naming the label, NOT a silent undefined link symbol.
    let (_m, d) = lower(concat!(
        "module m\n",
        "comptime fn jump(mlab: Label) -> Code {\n",
        "    return asm {\n",
        "        bra.w   {mlab}\n",
        "    }\n",
        "}\n",
        "pub proc P () {\n",
        "    jump(.donee)\n",
        "    nop\n",
        ".done:\n",
        "    rts\n",
        "}\n",
    ));
    let errs = errors(&d);
    assert!(
        errs.iter().any(|e| e.contains("donee")),
        "expected a loud error naming `.donee`, got: {errs:?}"
    );
}

// ---- F2 negative: a const with a `.name` body, referenced from a CALL ARG ----
//
// A const initializer is a PURE comptime expression even when the const is
// referenced from a label-value context (`t(x)` where `const x = .foo`): the
// `.foo` must NOT fold into a silent Label there. Pins the label-ctx reset in
// `resolve_const`.

#[test]
fn const_with_local_label_body_referenced_from_call_arg_is_rejected() {
    let (_m, d) = lower(concat!(
        "module m\n",
        "const x = .foo\n",
        "comptime fn t(mlab: Label) -> Code {\n",
        "    return asm { bra.w {mlab} }\n",
        "}\n",
        "pub proc P () {\n",
        "    t(x)\n",
        ".foo:\n",
        "    rts\n",
        "}\n",
    ));
    let errs = errors(&d);
    assert!(
        errs.iter().any(|e| e.contains("comptime expression") || e.contains("proc-local")),
        "a const body `.foo` referenced from a call arg must still be rejected: {errs:?}"
    );
}

// ---- F2 negative: `.name` in a pure comptime expression is rejected ----------

#[test]
fn local_label_in_const_initializer_is_rejected() {
    // `const x = .foo` is NOT a label value — it is a pure comptime expression
    // position; the form must be rejected (parse or eval error), never a silent
    // Label leaking into an ordinary expression.
    let (_file, diags) = parse_str(concat!(
        "module m\n",
        "const x = .foo\n",
    ));
    // Either a parse error here, or (if it parses) an eval error later; at minimum
    // parse must not silently accept it as a valid ordinary expression that lowers.
    let had_parse_err = diags.iter().any(|d| d.level == Level::Error);
    if !had_parse_err {
        // It parsed — then, once the const is USED (consts are lazily resolved),
        // eval must reject it loudly rather than fold a silent Label into an
        // ordinary comptime expression. Reference `x` from a proc immediate so
        // its initializer is forced.
        let (_m, ld) = lower(concat!(
            "module m\n",
            "const x = .foo\n",
            "pub proc P () {\n",
            "    move.w  #x, d0\n",
            "    rts\n",
            "}\n",
        ));
        let errs = errors(&ld);
        assert!(
            errs.iter().any(|e| e.contains("comptime expression") || e.contains("proc-local")),
            "`.foo` in a const initializer must be rejected (not a silent Label): {errs:?}"
        );
    }
}
