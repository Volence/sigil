//! T3 (Plan 4) — `asm { }` instantiation → `Value::Code` → Core IR. Evaluate an
//! `asm` block (with splice bindings) to a resolved `Value::Code`, lower it with
//! `lower_code_buf`, and byte-diff the linked image (mirroring T0/T2's link
//! helpers). Also exercises the `[asm.splice-kind]` and `[branch.missing-size]`
//! diagnostics and fresh-per-instantiation label hygiene.

use sigil_frontend_emp::ast::{self, Expr, Stmt};
use sigil_frontend_emp::eval::{Env, Evaluator};
use sigil_frontend_emp::lower::lower_code_buf;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::{Value, Width};
use sigil_ir::backend::{Cpu, IrStreamer};
use sigil_ir::{IrBuilder, SymbolTable};

/// Parse a one-fn module and pull the `asm { }` expression out of the fn's
/// `return asm { ... }` (or bare-expr) body — the smallest way to get an
/// `Expr::Asm` for the eval entry.
fn asm_expr(src: &str) -> Expr {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "unexpected parse diagnostics: {diags:?}");
    for item in file.items {
        if let ast::Item::ComptimeFn(f) = item {
            for stmt in f.body {
                match stmt {
                    Stmt::Return { value: Some(e), .. } if matches!(e, Expr::Asm { .. }) => return e,
                    Stmt::Expr(e) if matches!(e, Expr::Asm { .. }) => return e,
                    _ => {}
                }
            }
        }
    }
    panic!("no `asm {{ }}` expression found in source");
}

/// Evaluate `asm` source to a `Value::Code`, seeding `env` with the given
/// name→value bindings for its splices. Returns the value and the evaluator's
/// diagnostics.
fn eval_asm_with(src: &str, bindings: &[(&str, Value)]) -> (Value, Vec<sigil_span::Diagnostic>) {
    let e = asm_expr(src);
    let mut ev = Evaluator::new();
    let mut env = Env::new();
    for (name, v) in bindings {
        env.define(*name, v.clone(), false);
    }
    let v = ev.eval_expr(&e, &mut env);
    (v, ev.diags)
}

/// Lower a `Value::Code` into a single 68k section and link it to flat bytes.
fn lower_link_68k(code: &Value) -> Vec<u8> {
    let Value::Code(buf) = code else { panic!("expected Value::Code, got {code}") };
    let mut builder = IrBuilder::new();
    builder.switch_section("text", Cpu::M68000, None);
    let mut diags = Vec::new();
    lower_code_buf(buf, Cpu::M68000, &mut builder, &mut diags);
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");
    let (module, bdiags) = builder.finish();
    assert!(bdiags.is_empty(), "unexpected builder diagnostics: {bdiags:?}");
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

#[test]
fn const_immediate_splice_encodes_move_b() {
    // `move.b #{v}, d0` with v = 5 → the exact AS bytes for `move.b #5,d0`
    // (golden vector `move.b #$12,d0 => 10 3C 00 12`).
    let src = "module m\ncomptime fn f() -> Code {\n    return asm {\n        move.b #{v}, d0\n    }\n}\n";
    let (code, diags) = eval_asm_with(src, &[("v", Value::Int(5))]);
    assert!(diags.is_empty(), "unexpected eval diagnostics: {diags:?}");
    assert_eq!(lower_link_68k(&code), vec![0x10, 0x3C, 0x00, 0x05]);
}

#[test]
fn width_size_splice_selects_word() {
    // `cmp.{w} #1, d0` with width = Width::W selects word size → `cmp.w #1,d0`
    // (cmp base 0xB, d0 reg, word opmode 001, #imm EA 0x3C → B0 7C 00 01).
    let src = "module m\ncomptime fn f() -> Code {\n    return asm {\n        cmp.{w} #1, d0\n    }\n}\n";
    let (code, diags) = eval_asm_with(src, &[("w", Value::Width(Width::W))]);
    assert!(diags.is_empty(), "unexpected eval diagnostics: {diags:?}");
    assert_eq!(lower_link_68k(&code), vec![0xB0, 0x7C, 0x00, 0x01]);
}

#[test]
fn wrong_kind_size_splice_is_splice_kind_error() {
    // A size splice that evaluates to a string (where a `Width` is expected) is
    // the `[asm.splice-kind]` diagnostic, naming the expected class and got-type.
    let src = "module m\ncomptime fn f() -> Code {\n    return asm {\n        cmp.{bad} #1, d0\n    }\n}\n";
    let (_code, diags) = eval_asm_with(src, &[("bad", Value::Str("oops".into()))]);
    assert!(
        diags.iter().any(|d| d.message.contains("[asm.splice-kind]")
            && d.message.contains("Width")
            && d.message.contains("string")),
        "expected an [asm.splice-kind] diagnostic naming Width and string, got: {diags:?}"
    );
}

#[test]
fn bare_branch_without_size_is_missing_size_error() {
    // A `bra` with no `.s`/`.w` is `[branch.missing-size]` (D-P4.2) — surfaced by
    // the lowering half, not eval (the Code value is well-formed).
    let src = "module m\ncomptime fn f() -> Code {\n    return asm {\n    .loop:\n        bra .loop\n    }\n}\n";
    let (code, ediags) = eval_asm_with(src, &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let Value::Code(buf) = &code else { panic!("expected Value::Code") };
    let mut builder = IrBuilder::new();
    builder.switch_section("text", Cpu::M68000, None);
    let mut diags = Vec::new();
    lower_code_buf(buf, Cpu::M68000, &mut builder, &mut diags);
    assert!(
        diags.iter().any(|d| d.message.contains("[branch.missing-size]")),
        "expected a [branch.missing-size] diagnostic, got: {diags:?}"
    );
}

#[test]
fn disp_ind_valid_golden() {
    // `move.w 4(a0), d0` — a (d16,An) source. MOVE word, dest d0 (reg 000, mode
    // 000), source (d16,a0) (mode 101, reg 000) → 0x3028, then disp word 0x0004.
    let src = "module m\ncomptime fn f() -> Code {\n    return asm {\n        move.w 4(a0), d0\n    }\n}\n";
    let (code, diags) = eval_asm_with(src, &[]);
    assert!(diags.is_empty(), "unexpected eval diagnostics: {diags:?}");
    assert_eq!(lower_link_68k(&code), vec![0x30, 0x28, 0x00, 0x04]);
}

#[test]
fn disp_ind_out_of_range_diagnoses_not_truncates() {
    // `move.w 100000(a0), d0` — the displacement overflows i16. It MUST diagnose
    // (mirroring AS's "operand out of range"), NOT silently wrap to a wrong
    // displacement at the byte-exactness seam.
    let src = "module m\ncomptime fn f() -> Code {\n    return asm {\n        move.w 100000(a0), d0\n    }\n}\n";
    let (code, ediags) = eval_asm_with(src, &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let Value::Code(buf) = &code else { panic!("expected Value::Code") };
    let mut builder = IrBuilder::new();
    builder.switch_section("text", Cpu::M68000, None);
    let mut diags = Vec::new();
    lower_code_buf(buf, Cpu::M68000, &mut builder, &mut diags);
    assert!(
        diags.iter().any(|d| d.message.contains("out of range")),
        "expected a displacement-out-of-range diagnostic, got: {diags:?}"
    );
}

#[test]
fn intra_asm_branch_roundtrips_through_link() {
    // `bra.w .loop` targeting a `.loop:` in the SAME `asm {}` round-trips: the
    // non-export label is renamed fresh, the branch reference rewrites to the
    // same fresh symbol, and the linker resolves the displacement. Label and
    // branch both at offset 0 → PcRelDisp16 = 0 - (0+2) = -2 = 0xFFFE.
    let src = "module m\ncomptime fn f() -> Code {\n    return asm {\n    .loop:\n        bra.w .loop\n    }\n}\n";
    let (code, diags) = eval_asm_with(src, &[]);
    assert!(diags.is_empty(), "unexpected eval diagnostics: {diags:?}");
    assert_eq!(lower_link_68k(&code), vec![0x60, 0x00, 0xFF, 0xFE]);
}

#[test]
fn intra_asm_dbra_roundtrips_through_link() {
    // `dbra d0, .loop` targeting a `.loop:` in the SAME `asm {}` round-trips just
    // like `bra.w` above: dbf d0,* opcode word = 0x51C8, then the placeholder
    // displacement word gets a PcRelDisp16 fixup. Label and dbra both at offset 0
    // → PcRelDisp16 = 0 - (0+2) = -2 = 0xFFFE.
    let src = "module m\ncomptime fn f() -> Code {\n    return asm {\n    .loop:\n        dbra d0, .loop\n    }\n}\n";
    let (code, diags) = eval_asm_with(src, &[]);
    assert!(diags.is_empty(), "unexpected eval diagnostics: {diags:?}");
    assert_eq!(lower_link_68k(&code), vec![0x51, 0xC8, 0xFF, 0xFE]);
}
