//! Spec 2, C2 — local typed-register binding `let <reg>: <Type>`.
//!
//! A `let` in a proc/asm body types a register that already holds its value: it
//! emits ZERO bytes and, from the binding point to the end of the enclosing
//! block, makes a bare field displacement `field(reg)` resolve in the type's
//! field space EXACTLY like a typed proc param — including the tranche-7b
//! namespace closure (a bare const does not resolve in the displacement slot on
//! a typed register). Scope is lexical: a subsequent `let` rebinds, and a
//! binding made inside a comptime-`if` branch does not leak past it. The bar is
//! byte-neutrality with the param form.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};

/// The base struct: a pitcher-plant-shaped SST with a byte-array `sst_custom`
/// window at `$2E`. Mirrors `overlay.rs`.
const SST: &str = "struct Sst (size: $50) {\n    \
    id: u16,\n    \
    _pad0: [u8; 14] @ $2,\n    \
    x_pos: u16 @ $10,\n    \
    _pad1: [u8; 8] @ $12,\n    \
    y_vel: u16 @ $1A,\n    \
    _pad2: [u8; 18] @ $1C,\n    \
    sst_custom: [u8; 34] @ $2E,\n\
}\n";

fn lower_errors(src: &str) -> (Module, Vec<String>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    let errs = diags
        .into_iter()
        .filter(|d| d.level == sigil_span::Level::Error)
        .map(|d| d.message)
        .collect();
    (module, errs)
}

fn linked_bytes(m: &Module) -> Vec<u8> {
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    m.sections
        .iter()
        .find_map(|s| linked.section(&s.name).map(|ls| ls.bytes.clone()))
        .expect("a linked section")
}

// ---- 1. headline: a `let` binding gives bare field-displacement access -------

#[test]
fn let_binding_headline_field_access() {
    // `let a0: *Sst` then `subq.b #1, timer(a0)` lowers byte-identically to the
    // literal `$2E(a0)`: SUBQ.B #1,(d16,A0) = 0x5328, ext $002E, RTS.
    let src = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8 }}\n\
         proc p () {{\n    let a0: *Sst\n    subq.b #1, timer(a0)\n    rts\n}}\n"
    );
    let (module, errs) = lower_errors(&src);
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
    assert_eq!(linked_bytes(&module), vec![0x53, 0x28, 0x00, 0x2E, 0x4E, 0x75]);
}

// ---- 2. byte-neutral with the typed-param form -------------------------------

#[test]
fn let_binding_is_byte_neutral_with_param() {
    // A `let a0: *Sst` after (no) load produces the SAME image as declaring
    // `(a0: *Sst)` as a param — the binding emits zero bytes and types the
    // register identically. This is the acceptance bar in miniature.
    let via_let = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8 }}\n\
         proc p () {{\n    let a0: *Sst\n    move.w x_pos(a0), d0\n    subq.b #1, timer(a0)\n    rts\n}}\n"
    );
    let via_param = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8 }}\n\
         proc p (a0: *Sst) {{\n    move.w x_pos(a0), d0\n    subq.b #1, timer(a0)\n    rts\n}}\n"
    );
    let (m_let, e_let) = lower_errors(&via_let);
    let (m_param, e_param) = lower_errors(&via_param);
    assert!(e_let.is_empty(), "let errors: {e_let:?}");
    assert!(e_param.is_empty(), "param errors: {e_param:?}");
    assert_eq!(
        linked_bytes(&m_let),
        linked_bytes(&m_param),
        "`let a0: *Sst` must be byte-identical to the `(a0: *Sst)` param form"
    );
}

// ---- 3. binding takes effect only AFTER the `let` (mirrors a self-load) -------

#[test]
fn let_binding_after_self_load() {
    // The TouchResponse shape: load the register with a real instruction, then
    // `let` it. `lea Sym, a0` (43 F9 + addr32 abs.l) then `move.w x_pos(a0), d0`
    // (30 28 00 10). Proves the binding rides after a preceding instruction.
    let src = format!(
        "module m\n{SST}data Sym: u8 = 0\n\
         proc p () {{\n    lea Sym, a0\n    let a0: *Sst\n    move.w x_pos(a0), d0\n    rts\n}}\n"
    );
    let (_module, errs) = lower_errors(&src);
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
}

// ---- 4. NEGATIVE: `let` on a non-register name is a loud error ----------------

#[test]
fn let_on_non_register_name_errors() {
    // `let foo: *Sst` — `foo` is not a register. The `let` is the decl site, so
    // it reports (unlike a param, which silently skips a non-register name).
    let src = format!(
        "module m\n{SST}\
         proc p () {{\n    let foo: *Sst\n    rts\n}}\n"
    );
    let (_module, errs) = lower_errors(&src);
    assert!(
        errs.iter().any(|m| m.contains("[asm.let-not-register]") && m.contains("foo")),
        "want [asm.let-not-register] naming foo, got: {errs:?}"
    );
}

// ---- 5. NEGATIVE: namespace closure — a bare const does not resolve (7b) ------

#[test]
fn let_binding_closes_the_displacement_namespace() {
    // A module-level `const timer` must NOT shadow field space: on a `let`-typed
    // register the displacement resolves ONLY in field space. With no overlay/
    // field `timer` in scope, this is `[operand.unknown-field]` naming `*Sst` —
    // exactly the tranche-7b closure a param gets, not a silent const fallback.
    let src = format!(
        "module m\n{SST}const timer: u8 = 9\n\
         proc p () {{\n    let a0: *Sst\n    tst.b timer(a0)\n    rts\n}}\n"
    );
    let (_module, errs) = lower_errors(&src);
    assert!(
        errs.iter().any(|m| m.contains("[operand.unknown-field]") && m.contains("*Sst")),
        "want [operand.unknown-field] naming *Sst (7b closure, no const fallback), got: {errs:?}"
    );
}

// ---- 6. rebinding: a later `let` on the same register re-types it -------------

#[test]
fn let_rebinding_retypes_the_register() {
    // `let a0: *Sst` then `let a0: *Other` — `x_pos` is an Sst field ($10),
    // `flag` an Other field ($2, chosen nonzero so it can't fold to a bare
    // `(a0)`). Each access resolves against the CURRENT binding: move.w
    // x_pos(a0),d0 = 30 28 00 10; tst.b flag(a0) = 4A 28 00 02; rts = 4E 75.
    let src = format!(
        "module m\n{SST}\
         struct Other (size: 4) {{ _pad0: [u8; 2], flag: u8 @ 2, _pad1: u8 @ 3 }}\n\
         proc p () {{\n    let a0: *Sst\n    move.w x_pos(a0), d0\n    \
         let a0: *Other\n    tst.b flag(a0)\n    rts\n}}\n"
    );
    let (module, errs) = lower_errors(&src);
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
    assert_eq!(
        linked_bytes(&module),
        vec![0x30, 0x28, 0x00, 0x10, 0x4A, 0x28, 0x00, 0x02, 0x4E, 0x75],
        "rebinding must resolve each site against the current binding"
    );
}

// ---- 7. NEGATIVE: a binding inside a comptime-`if` branch does not leak -------

#[test]
fn let_binding_in_comptime_if_does_not_leak() {
    // `let a0: *Sst` INSIDE `if 1 { }` types a0 only within the branch: the
    // branch's `timer(a0)` resolves in field space ($2E), but the SAME spelling
    // AFTER the branch falls to today's comptime eval and finds the module-level
    // `const timer` ($20). first tst.b timer(a0) = 4A 28 00 2E; second = 4A 28 00
    // 20; rts = 4E 75. A leaked binding would make the second resolve to $2E too.
    let src = format!(
        "module m\n{SST}vars V: sst_custom {{ timer: u8 }}\nconst timer: u8 = $20\n\
         proc p () {{\n    if 1 {{\n        let a0: *Sst\n        tst.b timer(a0)\n    }}\n    \
         tst.b timer(a0)\n    rts\n}}\n"
    );
    let (module, errs) = lower_errors(&src);
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
    assert_eq!(
        linked_bytes(&module),
        vec![0x4A, 0x28, 0x00, 0x2E, 0x4A, 0x28, 0x00, 0x20, 0x4E, 0x75],
        "the branch-local `let` must not leak past the `if`"
    );
}
