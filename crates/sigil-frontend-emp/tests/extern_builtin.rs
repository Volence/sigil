//! Task B2 (seam re-eval) — the `extern(name)` builtin: RAW passthrough of a
//! link-symbol reference as a [`sigil_ir::Expr::Sym`] `Value::LinkExpr`, no
//! mask/shift (unlike `bankid`/`winptr`). Closes the AS-equ read seam
//! together with Task B1 (`directive_equate`'s `EquSym` export): an AS-side
//! `equ`/`=` now reaches the linker's symbol table, and `extern(name)` is how
//! `.emp` code reads it back.
//!
//! Diagnostics/argument-form contract mirrors `bankid`'s exactly (R7m.3 (e)
//! precedent, `banks.rs`): wrong arity, non-string argument. A comptime-
//! required position hits the SAME `reject_if_provisional` choke point every
//! other `Value::LinkExpr` does — `extern(...)`'s residual (`Expr::Sym`
//! alone) carries no bank mask, so it falls into the generic
//! `[here.provisional]` bucket (see `eval_bankid`'s doc / `reject_if_provisional`
//! in `sigil-frontend-emp/src/eval/expr.rs`), exactly like `winptr`'s residual
//! already does today — NOT a new message class.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Expr, Module};

fn lower_ok(src: &str) -> Module {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (m, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    let errs: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(errs.is_empty(), "lower errors: {errs:?}");
    m
}

fn lower_diags(src: &str) -> Vec<sigil_span::Diagnostic> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (_m, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    diags
}

/// The folded `EquSym` named `name` across all sections of `resolved`
/// (post `resolve_layout`), or panic. Mirrors `equ_link.rs`'s helper.
fn folded_equ(resolved: &[sigil_ir::Section], name: &str) -> Expr {
    resolved
        .iter()
        .flat_map(|s| s.equ_syms.iter())
        .find(|e| e.name == name)
        .unwrap_or_else(|| panic!("no equ `{name}` found"))
        .expr
        .clone()
}

// ---- shape: extern(name) lowers to a raw Expr::Sym LinkExpr, no mask/shift ----

/// `extern("Foo")` used in an `equ` — the SIMPLEST way to observe the raw
/// residual tree without a data-cell width/endianness detour — must fold to
/// exactly `Expr::Sym("Foo")`, no `& mask`/`>> shift`/`| base` wrapping (the
/// defining difference from `bankid`/`winptr`).
#[test]
fn extern_of_label_is_raw_symbol_passthrough() {
    let src = "module m\n\
               section s (cpu: m68000, vma: $8000) {\n\
                 data L: u8 = 0\n\
                 equ E = extern(\"L\")\n\
               }\n";
    let module = lower_ok(src);
    let resolved =
        sigil_link::resolve_layout(&module.sections, &sigil_ir::SymbolTable::new(), true).expect("layout");
    assert_eq!(folded_equ(&resolved, "E"), Expr::Int(0x8000), "extern(\"L\") folds to L's own VMA, unmasked");
}

/// A bare `comptime fn` name (not a quoted string) works too — `Value::FnRef`,
/// exactly like `bankid`/`winptr` accept both forms (mirrors `banks.rs`'s
/// `bankid_of_fn_ref_captures_name`, which also uses a `comptime fn` name —
/// `eval_expr` on a bare identifier resolves a defined comptime fn to a
/// `Value::FnRef`, not an arbitrary link label).
#[test]
fn extern_of_fn_ref_argument_captures_the_name() {
    use sigil_frontend_emp::layout::eval_data_with_root;
    let src = "module m\n\
               comptime fn sfx() -> u8 { 0 }\n\
               data B: u16 = extern(sfx)\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (buf, _asserts, diags) = eval_data_with_root(&file, "B", None, None, &[]);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    // A BARE `Expr::Sym` LinkExpr (no mask/shift wrapping) lowers to the
    // pre-existing `Cell::SymRef` kind, not the generic `Cell::Expr` — the
    // data-cell lowering normalizes a trivial link-expr tree (just a Sym) back
    // to the plain symbol-reference cell it's equivalent to. This is the
    // observable proof `extern` truly built a RAW passthrough: no mask/shift
    // survived to need `Cell::Expr`'s general arithmetic-fixup machinery.
    assert_eq!(
        buf.expect("data buf").cells,
        vec![sigil_frontend_emp::value::Cell::SymRef { name: "sfx".into(), width: 2, windowed: false }],
        "extern(sfx) must build a bare Sym(\"sfx\") — no mask/shift",
    );
}

// ---- diagnostics: mirror bankid's exact taxonomy (R7m.3 (e) precedent) ------

/// Wrong arity (0 args) is diagnosed like `bankid`'s (`banks.rs`:
/// `bankid_wrong_arity_is_diagnosed`).
#[test]
fn extern_wrong_arity_zero_args_is_diagnosed() {
    let src = "module m\ndata X: u8 = 0\nequ E = extern()\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.message == "`extern` expects exactly 1 argument, got 0"),
        "expected the arity diagnostic, got: {diags:?}"
    );
}

/// Wrong arity (2 args) is diagnosed like `bankid`'s.
#[test]
fn extern_wrong_arity_two_args_is_diagnosed() {
    let src = "module m\ndata X: u8 = 0\nequ E = extern(A, B)\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.message == "`extern` expects exactly 1 argument, got 2"),
        "expected the arity diagnostic, got: {diags:?}"
    );
}

/// A non-symbol argument (an integer) is diagnosed like `bankid`'s "needs a
/// symbol reference" error.
#[test]
fn extern_non_string_argument_is_diagnosed() {
    let src = "module m\ndata X: u8 = 0\nequ E = extern(42)\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.message.contains("`extern` needs a symbol reference")),
        "expected the symbol-reference diagnostic, got: {diags:?}"
    );
}

/// An empty string argument is still a STRING (a valid `Value::Str` shape),
/// so it does NOT hit the "needs a symbol reference" type error — it reaches
/// `Expr::Sym("")`, an empty-name symbol reference that will fail later at
/// link (unresolved symbol), not at lower time. This pins that `extern("")`
/// does not panic and does not silently diagnose the WRONG error class.
#[test]
fn extern_empty_string_argument_lowers_without_a_type_error() {
    let src = "module m\ndata X: u8 = 0\nequ E = extern(\"\")\n";
    let diags = lower_diags(src);
    assert!(
        !diags.iter().any(|d| d.message.contains("needs a symbol reference")),
        "an empty STRING is still a valid extern() argument shape, got: {diags:?}"
    );
}

// ---- comptime-required refusal: same choke point as bankid/winptr/here() ---

/// `extern(...)` used as a comptime array length must refuse loudly via the
/// existing `reject_if_provisional` choke point — NOT silently size against a
/// wrong/stale value. Its residual carries no bank mask, so it surfaces the
/// GENERIC `[here.provisional]` message (the same one `winptr`'s residual
/// gets today), not `[bank.provisional]` (that one is bankid-specific
/// provenance steering — see `reject_if_provisional`'s doc).
#[test]
fn extern_as_array_length_refuses_via_reject_if_provisional() {
    let src = "module m\n\
               section s (cpu: m68000, vma: $8000) {\n\
                 data L: u8 = 0\n\
                 data Bad: [u8; extern(\"L\")] = []\n\
               }\n";
    let diags = lower_diags(src);
    assert!(
        diags.iter().any(|d| d.message.contains("[here.provisional]")),
        "expected the generic [here.provisional] refusal (extern carries no bank mask), got: {diags:?}"
    );
}
