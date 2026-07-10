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
    lower_code_buf(buf, Cpu::M68000, false, &mut builder, &mut diags);
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
fn bare_branch_without_size_is_missing_size_under_as_compat() {
    // §5.4: an unsized `bra` is `[branch.missing-size]` ONLY under `@as_compat`
    // (a faithful AS port pins branch widths). Without `@as_compat` it relaxes —
    // see `bare_branch_without_size_relaxes_without_as_compat` below.
    let src = "module m\ncomptime fn f() -> Code {\n    return asm {\n    .loop:\n        bra .loop\n    }\n}\n";
    let (code, ediags) = eval_asm_with(src, &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let Value::Code(buf) = &code else { panic!("expected Value::Code") };
    let mut builder = IrBuilder::new();
    builder.switch_section("text", Cpu::M68000, None);
    let mut diags = Vec::new();
    lower_code_buf(buf, Cpu::M68000, true, &mut builder, &mut diags);
    assert!(
        diags.iter().any(|d| d.message.contains("[branch.missing-size]")),
        "expected a [branch.missing-size] diagnostic under @as_compat, got: {diags:?}"
    );
}

#[test]
fn bare_branch_without_size_relaxes_without_as_compat() {
    // The §5.4 flip side: WITHOUT `@as_compat` an unsized `bra` lowers cleanly
    // (Core relaxes it), so there is NO `[branch.missing-size]` error. `bra .loop`
    // targets its own label (disp = 0 - 2 = -2 → bra.s FE), a legal reaching form.
    let src = "module m\ncomptime fn f() -> Code {\n    return asm {\n    .loop:\n        bra .loop\n    }\n}\n";
    let (code, ediags) = eval_asm_with(src, &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let Value::Code(buf) = &code else { panic!("expected Value::Code") };
    let mut builder = IrBuilder::new();
    builder.switch_section("text", Cpu::M68000, None);
    let mut diags = Vec::new();
    lower_code_buf(buf, Cpu::M68000, false, &mut builder, &mut diags);
    assert!(
        !diags.iter().any(|d| d.message.contains("[branch.missing-size]")),
        "an unsized branch must NOT error without @as_compat (it relaxes), got: {diags:?}"
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
    lower_code_buf(buf, Cpu::M68000, false, &mut builder, &mut diags);
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

// ---- symbolic absolute-address operands → Fragment::RelaxAbsSym -----------

/// Lower a `Value::Code` to a single 68k section's fragments (no link), so a
/// test can inspect the emitted `RelaxAbsSym` directly.
fn lower_module_68k(code: &Value) -> (sigil_ir::Module, Vec<sigil_span::Diagnostic>) {
    let Value::Code(buf) = code else { panic!("expected Value::Code, got {code}") };
    let mut builder = IrBuilder::new();
    builder.switch_section("text", Cpu::M68000, None);
    let mut diags = Vec::new();
    lower_code_buf(buf, Cpu::M68000, false, &mut builder, &mut diags);
    let (module, _bdiags) = builder.finish();
    (module, diags)
}

/// Assert the first (and only) fragment is a `RelaxAbsSym` whose candidates
/// match `short`/`long` bytes, whose fixups are `Abs16Be`/`Abs32Be` at the SAME
/// `offset`, and both referencing symbol `sym`.
fn assert_relax_abs(src: &str, sym: &str, short: &[u8], long: &[u8], offset: u32) {
    use sigil_ir::{Expr, Fixup, FixupKind, Fragment};
    let (code, ediags) = eval_asm_with(src, &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let (module, diags) = lower_module_68k(&code);
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");
    assert_eq!(module.sections[0].fragments.len(), 1, "expected one fragment");
    match &module.sections[0].fragments[0] {
        Fragment::RelaxAbsSym { short: s, long: l, target, .. } => {
            assert_eq!(s.bytes, short, "short (abs.w) bytes");
            assert_eq!(l.bytes, long, "long (abs.l) bytes");
            assert_eq!(s.fixup, Fixup { kind: FixupKind::Abs16Be, offset, target: Expr::Sym(sym.into()) });
            assert_eq!(l.fixup, Fixup { kind: FixupKind::Abs32Be, offset, target: Expr::Sym(sym.into()) });
            assert_eq!(*target, Expr::Sym(sym.into()));
        }
        other => panic!("expected RelaxAbsSym, got {other:?}"),
    }
}

fn asm_1(instr: &str) -> String {
    format!("module m\ncomptime fn f() -> Code {{\n    return asm {{\n        {instr}\n    }}\n}}\n")
}

#[test]
fn move_w_abs_src_emits_relax() {
    // move.w Foo, d0 — src abs, dst Dn (no ext). abs.w 30 38, abs.l 30 39.
    assert_relax_abs(&asm_1("move.w Foo, d0"), "Foo", &[0x30, 0x38, 0x00, 0x00], &[0x30, 0x39, 0x00, 0x00, 0x00, 0x00], 2);
}

#[test]
fn move_w_abs_dst_emits_relax() {
    // move.w d0, Foo — dst abs (matches the linker's hand-built relax_move).
    assert_relax_abs(&asm_1("move.w d0, Foo"), "Foo", &[0x31, 0xC0, 0x00, 0x00], &[0x33, 0xC0, 0x00, 0x00, 0x00, 0x00], 2);
}

#[test]
fn move_l_abs_src_emits_relax() {
    // move.l Foo, d0 — long-size data move; opcode word still one word, offset 2.
    assert_relax_abs(&asm_1("move.l Foo, d0"), "Foo", &[0x20, 0x38, 0x00, 0x00], &[0x20, 0x39, 0x00, 0x00, 0x00, 0x00], 2);
}

#[test]
fn lea_abs_emits_relax() {
    // lea Foo, a0 → 41 F8 / 41 F9 (abs source, An dest has no ext).
    assert_relax_abs(&asm_1("lea Foo, a0"), "Foo", &[0x41, 0xF8, 0x00, 0x00], &[0x41, 0xF9, 0x00, 0x00, 0x00, 0x00], 2);
}

#[test]
fn tst_w_abs_emits_relax() {
    // tst.w Foo → 4A 78 / 4A 79.
    assert_relax_abs(&asm_1("tst.w Foo"), "Foo", &[0x4A, 0x78, 0x00, 0x00], &[0x4A, 0x79, 0x00, 0x00, 0x00, 0x00], 2);
}

#[test]
fn clr_w_abs_emits_relax() {
    // clr.w Foo → 42 78 / 42 79.
    assert_relax_abs(&asm_1("clr.w Foo"), "Foo", &[0x42, 0x78, 0x00, 0x00], &[0x42, 0x79, 0x00, 0x00, 0x00, 0x00], 2);
}

#[test]
fn abs_sym_selects_width_and_links() {
    // End-to-end: move.w d0, Foo with Foo at a low (abs.w) address resolves to the
    // short candidate and the linker patches the Abs16Be operand (0x1000).
    let (code, ediags) = eval_asm_with(&asm_1("move.w d0, Foo"), &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let Value::Code(buf) = &code else { panic!("expected Value::Code") };
    let mut builder = IrBuilder::new();
    builder.switch_section("text", Cpu::M68000, None);
    let mut diags = Vec::new();
    lower_code_buf(buf, Cpu::M68000, false, &mut builder, &mut diags);
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");
    let (module, _b) = builder.finish();
    let mut syms = SymbolTable::new();
    syms.define("Foo", sigil_ir::SymbolValue::Int(0x1000));
    let resolved = sigil_link::resolve_layout(&module.sections, &syms, true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &syms).expect("link");
    assert_eq!(linked.section("text").unwrap().bytes, vec![0x31, 0xC0, 0x10, 0x00]);
}

#[test]
fn abs_sym_high_target_selects_long() {
    // move.w d0, Foo with Foo above the abs.w range resolves to the abs.l (long)
    // candidate: 6 bytes, Abs32Be operand patched with the full address.
    let (code, ediags) = eval_asm_with(&asm_1("move.w d0, Foo"), &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let Value::Code(buf) = &code else { panic!("expected Value::Code") };
    let mut builder = IrBuilder::new();
    builder.switch_section("text", Cpu::M68000, None);
    let mut diags = Vec::new();
    lower_code_buf(buf, Cpu::M68000, false, &mut builder, &mut diags);
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");
    let (module, _b) = builder.finish();
    let mut syms = SymbolTable::new();
    syms.define("Foo", sigil_ir::SymbolValue::Int(0x12345678));
    let resolved = sigil_link::resolve_layout(&module.sections, &syms, true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &syms).expect("link");
    assert_eq!(linked.section("text").unwrap().bytes, vec![0x33, 0xC0, 0x12, 0x34, 0x56, 0x78]);
}

#[test]
fn two_symbolic_operands_diagnose() {
    // move.w Foo, Bar — two symbolic operands is deferred, must diagnose (no panic).
    let (code, ediags) = eval_asm_with(&asm_1("move.w Foo, Bar"), &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let (_module, diags) = lower_module_68k(&code);
    assert!(
        diags.iter().any(|d| d.message.contains("two symbolic")),
        "expected a two-symbolic-operands diagnostic, got: {diags:?}"
    );
}

#[test]
fn abs_sym_with_preceding_immediate_relaxes() {
    // move.w #5, Foo — an immediate BEFORE the sym operand is allowed since
    // tranche 5 (its ext word precedes the abs field, which stays LAST): the
    // unpinned form emits the RelaxAbsSym pair with the imm word inside both
    // candidates and the fixup offsets end-anchored past it.
    use sigil_ir::Fragment;
    let (code, ediags) = eval_asm_with(&asm_1("move.w #5, Foo"), &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let (module, diags) = lower_module_68k(&code);
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");
    match &module.sections[0].fragments[0] {
        Fragment::RelaxAbsSym { short, long, .. } => {
            // short: 31FC 0005 + abs.w hole (6 B, fixup at 4);
            // long:  33FC 0005 + abs.l hole (8 B, fixup at 4).
            assert_eq!(short.bytes.len(), 6, "abs.w candidate length");
            assert_eq!(long.bytes.len(), 8, "abs.l candidate length");
            assert_eq!(short.fixup.offset, 4, "abs.w fixup past the imm word");
            assert_eq!(long.fixup.offset, 4, "abs.l fixup past the imm word");
        }
        other => panic!("expected RelaxAbsSym, got {other:?}"),
    }
}

#[test]
fn abs_sym_with_displacement_operand_diagnoses() {
    // move.w Foo, 4(a1) — the (d16,An) dest carries an extension word too: out of scope.
    let (code, ediags) = eval_asm_with(&asm_1("move.w Foo, 4(a1)"), &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let (_module, diags) = lower_module_68k(&code);
    assert!(
        diags.iter().any(|d| d.message.contains("extension-word operand")),
        "expected an extension-word-combination diagnostic, got: {diags:?}"
    );
}

// ---- `sp` alias (GAP 1) + `movem` register lists (GAP 2) ------------------
//
// Port #1 (hblank) recon found two operand-grammar gaps in the `.emp`
// front-end (the ISA layer is complete): `sp` is not accepted as the `a7`
// spelling anywhere an address register parses, and `movem`'s register-list
// operand (`d0-d1/a0`) has no grammar at all (it parses as arithmetic over
// unknown names). Reference bytes are the real hblank dispatcher (both
// shapes) plus AS-front-end-verified parity vectors (`eval.rs`
// `parse_reg_list_builds_canonical_masks` / `m68k_movem_*` tests).

#[test]
fn movem_store_predec_sp_matches_hblank_reference() {
    // `movem.l d0-d1/a0, -(sp)` — the hblank dispatcher's register-save line.
    // Canonical mask d0|d1|a0 = 0x0103; STORE to -(An) reverses it to 0xC080.
    let (code, diags) = eval_asm_with(&asm_1("movem.l d0-d1/a0, -(sp)"), &[]);
    assert!(diags.is_empty(), "unexpected eval diagnostics: {diags:?}");
    assert_eq!(lower_link_68k(&code), vec![0x48, 0xE7, 0xC0, 0x80]);
}

#[test]
fn movem_load_postinc_sp_matches_hblank_reference() {
    // `movem.l (sp)+, d0-d1/a0` — the hblank dispatcher's register-restore line.
    // LOAD from (An)+ emits the canonical mask as-is (no reversal): 0x0103.
    let (code, diags) = eval_asm_with(&asm_1("movem.l (sp)+, d0-d1/a0"), &[]);
    assert!(diags.is_empty(), "unexpected eval diagnostics: {diags:?}");
    assert_eq!(lower_link_68k(&code), vec![0x4C, 0xDF, 0x01, 0x03]);
}

#[test]
fn movem_word_size_form() {
    // `movem.w d0-d3, (a1)` — word size, plain (An) indirect (no reversal).
    // Canonical mask d0-d3 = 0x000F. movem.w opcode = 0x4890 | reg(a1)=1 = 0x4891.
    let (code, diags) = eval_asm_with(&asm_1("movem.w d0-d3, (a1)"), &[]);
    assert!(diags.is_empty(), "unexpected eval diagnostics: {diags:?}");
    assert_eq!(lower_link_68k(&code), vec![0x48, 0x91, 0x00, 0x0F]);
}

#[test]
fn movem_single_register_predec_sp() {
    // `movem.l d0, -(sp)` — single-register list. Canonical mask d0 = 0x0001;
    // STORE to -(An) reverses to 0x8000.
    let (code, diags) = eval_asm_with(&asm_1("movem.l d0, -(sp)"), &[]);
    assert!(diags.is_empty(), "unexpected eval diagnostics: {diags:?}");
    assert_eq!(lower_link_68k(&code), vec![0x48, 0xE7, 0x80, 0x00]);
}

#[test]
fn movem_wide_mixed_range_list_predec_sp() {
    // `movem.l d0-d7/a0-a6, -(sp)` — wide mixed list. Canonical mask = 0x7FFF
    // (all of d0-d7 and a0-a6, not a7); STORE to -(An) reverses to 0xFFFE.
    let (code, diags) = eval_asm_with(&asm_1("movem.l d0-d7/a0-a6, -(sp)"), &[]);
    assert!(diags.is_empty(), "unexpected eval diagnostics: {diags:?}");
    assert_eq!(lower_link_68k(&code), vec![0x48, 0xE7, 0xFF, 0xFE]);
}

#[test]
fn sp_alias_predec_matches_a7_spelling() {
    // `move.l d0, -(sp)` must be byte-identical to `move.l d0, -(a7)`.
    let (sp_code, sp_diags) = eval_asm_with(&asm_1("move.l d0, -(sp)"), &[]);
    assert!(sp_diags.is_empty(), "unexpected eval diagnostics: {sp_diags:?}");
    let (a7_code, a7_diags) = eval_asm_with(&asm_1("move.l d0, -(a7)"), &[]);
    assert!(a7_diags.is_empty(), "unexpected eval diagnostics: {a7_diags:?}");
    assert_eq!(lower_link_68k(&sp_code), lower_link_68k(&a7_code));
}

#[test]
fn sp_alias_postinc_matches_a7_spelling() {
    // `move.l (sp)+, d0` must be byte-identical to `move.l (a7)+, d0`.
    let (sp_code, sp_diags) = eval_asm_with(&asm_1("move.l (sp)+, d0"), &[]);
    assert!(sp_diags.is_empty(), "unexpected eval diagnostics: {sp_diags:?}");
    let (a7_code, a7_diags) = eval_asm_with(&asm_1("move.l (a7)+, d0"), &[]);
    assert!(a7_diags.is_empty(), "unexpected eval diagnostics: {a7_diags:?}");
    assert_eq!(lower_link_68k(&sp_code), lower_link_68k(&a7_code));
}

#[test]
fn sp_alias_plain_register_matches_a7_spelling() {
    // `movea.l sp, a1` (plain register operand) must be byte-identical to
    // `movea.l a7, a1`.
    let (sp_code, sp_diags) = eval_asm_with(&asm_1("movea.l sp, a1"), &[]);
    assert!(sp_diags.is_empty(), "unexpected eval diagnostics: {sp_diags:?}");
    let (a7_code, a7_diags) = eval_asm_with(&asm_1("movea.l a7, a1"), &[]);
    assert!(a7_diags.is_empty(), "unexpected eval diagnostics: {a7_diags:?}");
    assert_eq!(lower_link_68k(&sp_code), lower_link_68k(&a7_code));
}

#[test]
fn sp_alias_ind_matches_a7_spelling() {
    // `move.l (sp), d0` (plain register-indirect) must be byte-identical to
    // `move.l (a7), d0`.
    let (sp_code, sp_diags) = eval_asm_with(&asm_1("move.l (sp), d0"), &[]);
    assert!(sp_diags.is_empty(), "unexpected eval diagnostics: {sp_diags:?}");
    let (a7_code, a7_diags) = eval_asm_with(&asm_1("move.l (a7), d0"), &[]);
    assert!(a7_diags.is_empty(), "unexpected eval diagnostics: {a7_diags:?}");
    assert_eq!(lower_link_68k(&sp_code), lower_link_68k(&a7_code));
}

#[test]
fn sp_alias_displacement_matches_a7_spelling() {
    // `move.w 4(sp), d0` (d16,An) displacement form must be byte-identical to
    // `move.w 4(a7), d0`.
    let (sp_code, sp_diags) = eval_asm_with(&asm_1("move.w 4(sp), d0"), &[]);
    assert!(sp_diags.is_empty(), "unexpected eval diagnostics: {sp_diags:?}");
    let (a7_code, a7_diags) = eval_asm_with(&asm_1("move.w 4(a7), d0"), &[]);
    assert!(a7_diags.is_empty(), "unexpected eval diagnostics: {a7_diags:?}");
    assert_eq!(lower_link_68k(&sp_code), lower_link_68k(&a7_code));
}

#[test]
fn sp_alias_in_movem_reglist_matches_a7_spelling() {
    // `sp` inside a movem register list must be byte-identical to `a7`.
    let (sp_code, sp_diags) = eval_asm_with(&asm_1("movem.l d0-d1/sp, -(a6)"), &[]);
    assert!(sp_diags.is_empty(), "unexpected eval diagnostics: {sp_diags:?}");
    let (a7_code, a7_diags) = eval_asm_with(&asm_1("movem.l d0-d1/a7, -(a6)"), &[]);
    assert!(a7_diags.is_empty(), "unexpected eval diagnostics: {a7_diags:?}");
    assert_eq!(lower_link_68k(&sp_code), lower_link_68k(&a7_code));
}

#[test]
fn movem_byte_size_diagnoses() {
    // `movem.b` — movem is word/long only (matches the AS front-end's
    // `lower_m68k_movem`, which rejects any size but W/L).
    let (code, ediags) = eval_asm_with(&asm_1("movem.b d0, -(sp)"), &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let (_module, diags) = lower_module_68k(&code);
    assert!(
        diags.iter().any(|d| d.message.contains("word") && d.message.contains("long")),
        "expected a movem word/long-only diagnostic, got: {diags:?}"
    );
}

#[test]
fn movem_empty_reglist_diagnoses_not_panics() {
    // `movem.l , -(sp)` — an empty register-list operand. Must diagnose
    // cleanly (at whichever stage — parse, eval, or lowering), never panic.
    let (_file, pdiags) = parse_str(&asm_1("movem.l , -(sp)"));
    assert!(!pdiags.is_empty(), "expected a parse diagnostic for an empty movem operand");
}

#[test]
fn movem_malformed_reglist_diagnoses_not_panics() {
    // `movem.l d0-, -(sp)` — a dangling range. Must diagnose cleanly (at
    // whichever stage), never panic.
    let (_file, pdiags) = parse_str(&asm_1("movem.l d0-, -(sp)"));
    assert!(!pdiags.is_empty(), "expected a parse diagnostic for a malformed movem reglist");
}

#[test]
fn movem_descending_range_diagnoses() {
    // `movem.l d3-d0, -(sp)` — a descending range. The AS front-end's
    // `parse_reg_list` rejects `lo > hi` outright (`d7-d0` → `None`, pinned by
    // `parse_reg_list_builds_canonical_masks`), so the `.emp` front-end matches:
    // refuse, don't silently normalize. `d3-d0` fails the reglist recognizer
    // entirely (neither operand parses as a list), so this diagnoses at EVAL
    // time (a clean "needs a register-list operand" error), not lowering time.
    let (_code, ediags) = eval_asm_with(&asm_1("movem.l d3-d0, -(sp)"), &[]);
    assert!(!ediags.is_empty(), "expected a diagnostic for a descending movem range, got none");
}

#[test]
fn movem_reglist_in_register_operand_position_diagnoses() {
    // `movem.l d0-d1, d0` — the non-list operand must be a memory EA per the
    // ISA; a bare register there is illegal. `d0` alone also parses as a
    // (single-register) reglist, so BOTH operands recognize as lists here —
    // a clean "two register lists" diagnostic at EVAL time, never a panic.
    let (_code, ediags) = eval_asm_with(&asm_1("movem.l d0-d1, d0"), &[]);
    assert!(
        !ediags.is_empty(),
        "expected a diagnostic for a movem reglist-as-register-operand, got none"
    );
}

#[test]
fn bare_reglist_shape_outside_movem_stays_unknown_names() {
    // `d0-d1` in a NON-movem context (e.g. as a `move` operand) must NOT leak
    // reglist parsing into the general operand grammar — it stays an arithmetic
    // expression over unknown names (D-P1H.2: mnemonic-directed, not general).
    // The failed operand's item is dropped (not the whole `asm{}`, per
    // `eval_asm_owned`'s per-statement error recovery), so the diagnostic lands
    // at EVAL time, not lowering time.
    let (_code, ediags) = eval_asm_with(&asm_1("move.w d0-d1, d2"), &[]);
    assert!(
        ediags.iter().any(|d| d.message.contains("unknown name")),
        "expected `d0-d1` outside movem to fail as unknown names at eval time, got: {ediags:?}"
    );
}

#[test]
fn const_named_sp_still_resolves_like_any_other_identifier() {
    // `sp` becoming a general address-register alias must not steal the
    // identifier from comptime/expression position — a `const sp = 5` keeps
    // working exactly like `const a7 = 5` does today (registers are recognized
    // only in operand-syntax positions, not general `eval_expr` path lookup).
    let src = "module m\nconst sp = 5\n";
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.is_empty(), "unexpected parse diagnostics: {pdiags:?}");
    let (v, diags) = sigil_frontend_emp::eval::eval_const(&file, "sp");
    assert!(diags.is_empty(), "unexpected eval diagnostics: {diags:?}");
    assert_eq!(v, Some(Value::Int(5)));
}

// ---- `(An,Xn)` / `d8(An,Xn)` indexed EAs (tranche 3, vdp_init recon) ------
//
// Port #3 (vdp_init) recon found the third operand-grammar gap: address-
// register indirect with index (68k `(d8,An,Xn)`, brief extension word).
// The ISA layer is complete (`Disp8AnXn`, corpus-pinned); only the `.emp`
// surface is missing — `ind_single_reg` rejects the two-part form. Reference
// bytes: the real vdp_init line (s4.lst:13139, `move.b (a0,d2.w), d0` at
// $1C48 → `10 30 2000`) plus the ISA corpus vectors for the disp/long forms.

#[test]
fn an_indexed_zero_disp_matches_vdp_init_reference() {
    // `move.b (a0,d2.w), d0` — the real Flush_VDP_Shadow shadow-value load.
    // Brief ext: d2.w, d=0 → 0x2000.
    let (code, diags) = eval_asm_with(&asm_1("move.b (a0,d2.w), d0"), &[]);
    assert!(diags.is_empty(), "unexpected eval diagnostics: {diags:?}");
    assert_eq!(lower_link_68k(&code), vec![0x10, 0x30, 0x20, 0x00]);
}

#[test]
fn an_indexed_unsuffixed_index_defaults_to_word() {
    // `(a0,d2)` must be byte-identical to `(a0,d2.w)` — AS's unsuffixed
    // default (same rule the pc-indexed form pinned in tranche 2).
    let (unsuf, d1) = eval_asm_with(&asm_1("move.b (a0,d2), d0"), &[]);
    assert!(d1.is_empty(), "unexpected eval diagnostics: {d1:?}");
    let (suf, d2) = eval_asm_with(&asm_1("move.b (a0,d2.w), d0"), &[]);
    assert!(d2.is_empty(), "unexpected eval diagnostics: {d2:?}");
    assert_eq!(lower_link_68k(&unsuf), lower_link_68k(&suf));
}

#[test]
fn an_indexed_with_displacement_and_long_index() {
    // `move.l 2(a3,a4.l), d0` — ISA corpus vector ("move.l (2,a3,a4.l),d0"):
    // opcode 0x2033, brief ext a4.l d=2 → 0xC802.
    let (code, diags) = eval_asm_with(&asm_1("move.l 2(a3,a4.l), d0"), &[]);
    assert!(diags.is_empty(), "unexpected eval diagnostics: {diags:?}");
    assert_eq!(lower_link_68k(&code), vec![0x20, 0x33, 0xC8, 0x02]);
}

#[test]
fn an_indexed_negative_displacement() {
    // `move.w -2(a2,d3.w), d0` — ISA corpus vector ("move.w (-2,a2,d3.w),d0"):
    // opcode 0x3032, brief ext d3.w d=-2 → 0x30FE.
    let (code, diags) = eval_asm_with(&asm_1("move.w -2(a2,d3.w), d0"), &[]);
    assert!(diags.is_empty(), "unexpected eval diagnostics: {diags:?}");
    assert_eq!(lower_link_68k(&code), vec![0x30, 0x32, 0x30, 0xFE]);
}

#[test]
fn an_indexed_as_destination() {
    // `move.b d0, (a0,d2.w)` — indexed EA in the DESTINATION position
    // (vdp_init only reads through it, but the form is position-agnostic).
    // move.b src d0, dest mode 110 reg 000 → opcode 0x1180, ext 0x2000.
    let (code, diags) = eval_asm_with(&asm_1("move.b d0, (a0,d2.w)"), &[]);
    assert!(diags.is_empty(), "unexpected eval diagnostics: {diags:?}");
    assert_eq!(lower_link_68k(&code), vec![0x11, 0x80, 0x20, 0x00]);
}

#[test]
fn an_indexed_rejects_byte_index_size() {
    // Brief-extension index widths are `.w`/`.l` only — same diagnostic
    // contract as the pc-indexed form.
    let (_code, diags) = eval_asm_with(&asm_1("move.b (a0,d2.b), d0"), &[]);
    assert!(
        diags.iter().any(|d| d.message.contains("index size must be `.w` or `.l`")),
        "expected an index-size diagnostic, got: {diags:?}"
    );
}

#[test]
fn an_indexed_rejects_out_of_range_displacement() {
    // The brief extension word carries an 8-bit displacement: -128..=127.
    let (_code, diags) = eval_asm_with(&asm_1("move.b 128(a0,d2.w), d0"), &[]);
    assert!(
        diags.iter().any(|d| d.message.contains("-128..=127") || d.message.contains("8-bit")),
        "expected a displacement-range diagnostic, got: {diags:?}"
    );
}

#[test]
fn an_indexed_base_size_suffix_is_rejected() {
    // Review finding (tranche 3): AS rejects a base size suffix on the
    // 68000 (`(a0.l,d2.w)` is 68020 syntax with different semantics) —
    // silently ignoring it is a byte-exactness hazard.
    let (_code, diags) = eval_asm_with(&asm_1("move.b (a0.l,d2.w), d0"), &[]);
    assert!(
        diags.iter().any(|d| d.message.contains("base register takes no size suffix")),
        "a base size suffix must be rejected, got: {diags:?}"
    );
}

#[test]
fn bare_pc_indexed_without_target_gets_a_steering_error() {
    // Review finding (tranche 3): `(pc,d2.w)` (no displacement) fell into
    // the An-indexed path and diagnosed `unknown name \`pc\`` — misleading.
    // Steer to the pc-relative spelling instead.
    let (_code, diags) = eval_asm_with(&asm_1("move.w (pc,d2.w), d0"), &[]);
    assert!(
        diags.iter().any(|d| d.message.contains("Sym(pc")),
        "bare (pc,Xn) must steer to the Sym(pc,Xn) spelling, got: {diags:?}"
    );
}

// ---- explicit-width absolute EAs `(expr).w` / `(expr).l` (tranche 3) ------
//
// Volence-ratified at the packet review (the former abs.l-destinations open):
// the AS-parity FORCED-width spelling, complementing the bare-symbol idiom
// (which relaxes via the width rule and stays the new-style default). Two
// shapes: a comptime integer address pins its bytes at lower time; a symbol
// pins the WIDTH and defers the address as a single fixed-width fixup (no
// RelaxAbsSym pair). Both positions (source and destination).

/// A pinned symbolic abs must emit ONE Fragment::Data with a single
/// fixed-width fixup — not a RelaxAbsSym candidate pair.
fn assert_pinned_abs(src: &str, sym: &str, bytes: &[u8], kind: sigil_ir::FixupKind, offset: u32) {
    use sigil_ir::{Expr, Fixup, Fragment};
    let (code, ediags) = eval_asm_with(src, &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let (module, diags) = lower_module_68k(&code);
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");
    assert_eq!(module.sections[0].fragments.len(), 1, "expected one fragment");
    match &module.sections[0].fragments[0] {
        Fragment::Data(df) => {
            assert_eq!(df.bytes, bytes, "pinned abs bytes");
            assert_eq!(
                df.fixups,
                vec![Fixup { kind, offset, target: Expr::Sym(sym.into()) }],
                "pinned abs fixup"
            );
        }
        other => panic!("expected a pinned Fragment::Data, got {other:?}"),
    }
}

#[test]
fn pinned_abs_w_int_destination() {
    // `move.w d0, ($FFFF8022).w` — the RAM-mirror idiom, forced word:
    // opcode 31C0 (dest abs.w), ext = low word $8022.
    let (code, diags) = eval_asm_with(&asm_1("move.w d0, ($FFFF8022).w"), &[]);
    assert!(diags.is_empty(), "unexpected eval diagnostics: {diags:?}");
    assert_eq!(lower_link_68k(&code), vec![0x31, 0xC0, 0x80, 0x22]);
}

#[test]
fn pinned_abs_l_int_source() {
    // `move.w ($C00004).l, d0` — reading the VDP control port, forced long:
    // opcode 3039 (src abs.l), ext = 00C0 0004.
    let (code, diags) = eval_asm_with(&asm_1("move.w ($C00004).l, d0"), &[]);
    assert!(diags.is_empty(), "unexpected eval diagnostics: {diags:?}");
    assert_eq!(lower_link_68k(&code), vec![0x30, 0x39, 0x00, 0xC0, 0x00, 0x04]);
}

#[test]
fn pinned_abs_l_int_lea() {
    // `lea.l ($C00004).l, a1` → 43F9 00C0 0004.
    let (code, diags) = eval_asm_with(&asm_1("lea.l ($C00004).l, a1"), &[]);
    assert!(diags.is_empty(), "unexpected eval diagnostics: {diags:?}");
    assert_eq!(lower_link_68k(&code), vec![0x43, 0xF9, 0x00, 0xC0, 0x00, 0x04]);
}

#[test]
fn pinned_abs_l_symbol_destination_defers_one_fixed_fixup() {
    // `move.w d0, (Foo).l` — width pinned by the author, address deferred:
    // opcode 33C0 (dest abs.l) + a 4-byte hole with ONE Abs32Be fixup at 2.
    assert_pinned_abs(
        &asm_1("move.w d0, (Foo).l"),
        "Foo",
        &[0x33, 0xC0, 0x00, 0x00, 0x00, 0x00],
        sigil_ir::FixupKind::Abs32Be,
        2,
    );
}

#[test]
fn pinned_abs_w_symbol_source_defers_one_fixed_fixup() {
    // `move.w (Foo).w, d0` → 3038 + Abs16Be at 2.
    assert_pinned_abs(
        &asm_1("move.w (Foo).w, d0"),
        "Foo",
        &[0x30, 0x38, 0x00, 0x00],
        sigil_ir::FixupKind::Abs16Be,
        2,
    );
}

#[test]
fn pinned_abs_w_out_of_window_int_is_rejected() {
    // $10000 has no abs.w spelling (outside asl's sign-extension window).
    let (_code, diags) = eval_asm_with(&asm_1("move.w d0, ($10000).w"), &[]);
    assert!(
        diags.iter().any(|d| d.message.contains("abs.w")),
        "an out-of-window .w address must be rejected naming abs.w, got: {diags:?}"
    );
}

#[test]
fn register_indirect_group_size_suffix_is_rejected() {
    // `(a0).w` is not a 68000 form — silently ignoring the suffix was the
    // same hazard class as the indexed base suffix.
    let (_code, diags) = eval_asm_with(&asm_1("move.w (a0).w, d0"), &[]);
    assert!(
        diags.iter().any(|d| d.message.contains("register indirect takes no size suffix")),
        "a sized register indirect must be rejected, got: {diags:?}"
    );
}

#[test]
fn pinned_abs_byte_width_is_rejected() {
    let (_code, diags) = eval_asm_with(&asm_1("move.w (Foo).b, d0"), &[]);
    assert!(
        diags.iter().any(|d| d.message.contains("absolute width must be `.w` or `.l`")),
        "a .b absolute width must be rejected, got: {diags:?}"
    );
}

#[test]
fn pinned_abs_l_with_preceding_immediate_defers() {
    // Tranche 5 (the stopZ80 shape): `move.w #$0100, (Foo).l` — an
    // extension-word operand BEFORE the sym operand is fine (the 68k emits
    // ext words in operand order, so the imm word precedes the abs field,
    // which stays LAST): opcode 33FC + imm 0100 + a 4-byte hole with ONE
    // Abs32Be fixup at 4.
    assert_pinned_abs(
        &asm_1("move.w #$0100, (Foo).l"),
        "Foo",
        &[0x33, 0xFC, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00],
        sigil_ir::FixupKind::Abs32Be,
        4,
    );
}

#[test]
fn pinned_abs_l_btst_immediate_defers() {
    // Tranche 5 (the stopZ80 poll): `btst #0, (Foo).l` → 0839 0000 + hole,
    // Abs32Be at 4.
    assert_pinned_abs(
        &asm_1("btst #0, (Foo).l"),
        "Foo",
        &[0x08, 0x39, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        sigil_ir::FixupKind::Abs32Be,
        4,
    );
}

#[test]
fn abs_sym_followed_by_ext_word_operand_still_deferred() {
    // The other order stays fenced: `move.w (Foo).l, $10(a0)` would put the
    // d16 ext word BEHIND the abs field, moving the fixup offset.
    let (code, ediags) = eval_asm_with(&asm_1("move.w (Foo).l, $10(a0)"), &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let (_module, diags) = lower_module_68k(&code);
    assert!(
        diags.iter().any(|d| d.message.contains("[lower.abs-sym-operand]")),
        "expected the abs-sym-followed-by-ext fence, got: {diags:?}"
    );
}

#[test]
fn imm_link_movea_defers_value32_fixup() {
    // Tranche 5 (the SongTable shape): `movea.l #extern("Tbl"), a0` — a
    // link-time imm32 encodes with a zero placeholder and ONE Value32Be
    // fixup at 2 (the emp mirror of the AS side's try_defer_long_imm).
    use sigil_ir::{Expr, Fragment};
    let (code, ediags) = eval_asm_with(&asm_1("movea.l #extern(\"Tbl\"), a0"), &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let (module, diags) = lower_module_68k(&code);
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");
    match &module.sections[0].fragments[0] {
        Fragment::Data(df) => {
            assert_eq!(df.bytes, vec![0x20, 0x7C, 0x00, 0x00, 0x00, 0x00], "movea.l #0, a0 hole");
            assert_eq!(df.fixups.len(), 1, "one fixup");
            assert_eq!(df.fixups[0].kind, sigil_ir::FixupKind::Value32Be);
            assert_eq!(df.fixups[0].offset, 2);
            assert_eq!(df.fixups[0].target, Expr::Sym("Tbl".into()));
        }
        other => panic!("expected Fragment::Data, got {other:?}"),
    }
}

#[test]
fn imm_link_word_defers_value16_fixup() {
    // Tranche 6 (the objroutine width): `move.w #extern("Tbl"), d0` — a
    // link-time imm16 encodes with a zero placeholder and ONE Value16Be
    // fixup at 2. (Supersedes tranche 5's refusal probe for this width.)
    use sigil_ir::{Expr, Fragment};
    let (code, ediags) = eval_asm_with(&asm_1("move.w #extern(\"Tbl\"), d0"), &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let (module, diags) = lower_module_68k(&code);
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");
    match &module.sections[0].fragments[0] {
        Fragment::Data(df) => {
            assert_eq!(df.bytes, vec![0x30, 0x3C, 0x00, 0x00], "move.w #0, d0 hole");
            assert_eq!(df.fixups.len(), 1, "one fixup");
            assert_eq!(df.fixups[0].kind, sigil_ir::FixupKind::Value16Be);
            assert_eq!(df.fixups[0].offset, 2);
            assert_eq!(df.fixups[0].target, Expr::Sym("Tbl".into()));
        }
        other => panic!("expected Fragment::Data, got {other:?}"),
    }
}

#[test]
fn imm_link_word_objroutine_store_collapses_zero_disp_dest() {
    // THE tranche-6 demand shape: `move.w #(extern("Main") - extern("Base")),
    // $0(a0)` — a link-time symbol DIFFERENCE in a word immediate, stored to
    // an offset-0 EA. The dest collapses to `(a0)` (asl's 30BC form, 4 bytes)
    // and the Sub target rides one Value16Be fixup at 2.
    use sigil_ir::{Expr, Fragment};
    let (code, ediags) = eval_asm_with(
        &asm_1("move.w #(extern(\"Main\") - extern(\"Base\")), $0(a0)"),
        &[],
    );
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let (module, diags) = lower_module_68k(&code);
    assert!(diags.is_empty(), "unexpected lowering diagnostics: {diags:?}");
    match &module.sections[0].fragments[0] {
        Fragment::Data(df) => {
            assert_eq!(df.bytes, vec![0x30, 0xBC, 0x00, 0x00], "move.w #0, (a0) hole");
            assert_eq!(df.fixups.len(), 1, "one fixup");
            assert_eq!(df.fixups[0].kind, sigil_ir::FixupKind::Value16Be);
            assert_eq!(df.fixups[0].offset, 2);
            match &df.fixups[0].target {
                Expr::Binary { .. } => {}
                other => panic!("expected a Sub link expr target, got {other:?}"),
            }
        }
        other => panic!("expected Fragment::Data, got {other:?}"),
    }
}

#[test]
fn imm_link_byte_size_is_refused_with_steering() {
    // A `.b` symbolic immediate has no deferral yet (the remaining width of
    // the ledgered extension gap) — steer, don't guess.
    let (code, ediags) = eval_asm_with(&asm_1("move.b #extern(\"Tbl\"), d0"), &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let (_module, diags) = lower_module_68k(&code);
    assert!(
        diags.iter().any(|d| d.message.contains("[lower.imm-link]")
            && d.message.contains("`.w` or `.l` size")),
        "expected the .b imm-link steering, got: {diags:?}"
    );
}

#[test]
fn zero_displacement_collapses_to_address_indirect() {
    // asl's zero-displacement optimization, mirrored (tranche 6): a
    // `(d16,An)` EA whose displacement is 0 encodes as plain `(An)` —
    // `move.w d0, $0(a0)` = 3080 (2 bytes), not 3140 0000. The demand class
    // is typed field access on an offset-0 field (`Sst.code_addr(a0)`).
    let (code, ediags) = eval_asm_with(&asm_1("move.w d0, $0(a0)"), &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    assert_eq!(lower_link_68k(&code), vec![0x30, 0x80]);
}

#[test]
fn movep_keeps_zero_displacement() {
    // The sole 68000 exception: movep has NO `(An)` mode — its d16 field is
    // load-bearing even at 0. `movep.w d0, $0(a0)` = 0188 0000.
    let (code, ediags) = eval_asm_with(&asm_1("movep.w d0, $0(a0)"), &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    assert_eq!(lower_link_68k(&code), vec![0x01, 0x88, 0x00, 0x00]);
}

#[test]
fn sr_operand_round_trips() {
    // The interrupt-mask idiom (sound_api): `move.w sr, -(sp)` = 40E7,
    // `move.w #$2700, sr` = 46FC 2700, `move.w (sp)+, sr` = 46DF.
    let src = "module m\ncomptime fn f() -> Code {\n    return asm {\n        move.w sr, -(sp)\n        move.w #$2700, sr\n        move.w (sp)+, sr\n    }\n}\n";
    let (code, ediags) = eval_asm_with(src, &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    assert_eq!(
        lower_link_68k(&code),
        vec![0x40, 0xE7, 0x46, 0xFC, 0x27, 0x00, 0x46, 0xDF]
    );
}

#[test]
fn move_sr_is_word_only() {
    // Tranche-5 adversarial F1: `move.l #$2700, sr` used to emit a LONG imm
    // the CPU reads as `sr := $0000` + `$2700` executing as an opcode —
    // silent and behavior-corrupting. Word-only is policed at the ISA level
    // (fixing BOTH frontends); the word form still round-trips.
    let (code, ediags) = eval_asm_with(&asm_1("move.l #$2700, sr"), &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let (_module, diags) = lower_module_68k(&code);
    assert!(
        diags.iter().any(|d| d.message.contains("word-only")),
        "expected the sr word-only refusal, got: {diags:?}"
    );
    let (code, ediags) = eval_asm_with(&asm_1("move.b sr, d0"), &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let (_module, diags) = lower_module_68k(&code);
    assert!(
        diags.iter().any(|d| d.message.contains("word-only")),
        "expected the sr word-only refusal for move-from too, got: {diags:?}"
    );
}

#[test]
fn quick_form_imm_link_steers_without_placeholder_leak() {
    // Tranche-5 adversarial F2: `addq.l #extern(A), d0` must steer with the
    // opcode-embedded-imm message, not leak the zero placeholder into the
    // backend's range error ("Addq data must be 1..=8, got 0").
    let (code, ediags) = eval_asm_with(&asm_1("addq.l #extern(\"A\"), d0"), &[]);
    assert!(ediags.is_empty(), "unexpected eval diagnostics: {ediags:?}");
    let (_module, diags) = lower_module_68k(&code);
    assert!(
        diags.iter().any(|d| d.message.contains("embeds its immediate in the opcode word")),
        "expected the opcode-embedded-imm steering, got: {diags:?}"
    );
    assert!(
        !diags.iter().any(|d| d.message.contains("got 0")),
        "the zero placeholder must not leak into a diagnostic: {diags:?}"
    );
}

#[test]
fn pinned_abs_sr_is_steered() {
    // Tranche-5 adversarial F3: `(sr).w` must steer early, not resolve as a
    // symbol named `sr` that dangles at link.
    let src = "module m\ncomptime fn f() -> Code {\n    return asm {\n        move.w (sr).w, d0\n    }\n}\n";
    let (_code, ediags) = eval_asm_with(src, &[]);
    assert!(
        ediags.iter().any(|d| d.message.contains("status-register operand, not an address")),
        "expected the (sr).w steering, got: {ediags:?}"
    );
}
