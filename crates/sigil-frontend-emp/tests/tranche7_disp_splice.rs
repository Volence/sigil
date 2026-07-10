//! Tranche 7 — F1: splice in displacement position `{off}({reg})` / `{off}(aN)`.
//!
//! A comptime `int` splice in the DISPLACEMENT slot of a `d16(An)` operand must
//! parse (continuing into the displacement-indirect grammar) and evaluate to the
//! same `CodeOperand::DispInd` a literal displacement produces — byte-identical.
//! A non-`int` splice (e.g. a `Reg`) in that slot is the `[asm.splice-kind]`
//! diagnostic naming the expected class.

use sigil_frontend_emp::ast::*;
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use sigil_span::{Diagnostic, Level};

fn parse_ok(src: &str) -> File {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "parse diagnostics: {diags:?}");
    file
}

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

// ---- F1 parse ---------------------------------------------------------------

/// `{boff}(a3)` — a bare splice in displacement position over a literal address
/// register must parse as `Operand::DispInd { disp: Splice-expr, inner }`.
#[test]
fn splice_disp_over_literal_reg_parses() {
    let f = parse_ok(concat!(
        "module m\n",
        "comptime fn t(boff: int) -> Code {\n",
        "    return asm {\n",
        "        sub.w   {boff}(a3), d1\n",
        "    }\n",
        "}\n",
    ));
    let Item::ComptimeFn(fun) = f.items.iter().find(|i| matches!(i, Item::ComptimeFn(_))).unwrap()
    else {
        panic!()
    };
    // Body: `return asm { ... }`.
    let instr = extract_single_instr(fun);
    let Operand::DispInd { disp, inner, .. } = &instr.operands[0] else {
        panic!("expected DispInd, got {:?}", instr.operands[0]);
    };
    // The displacement is the spliced `boff` expression.
    assert!(matches!(disp, Expr::Path(p) if p.segments == ["boff"]),
        "disp should be the splice expr, got {disp:?}");
    assert!(matches!(&**inner, Operand::Ind { .. }));
}

/// `{boff}({breg})` — splice in BOTH displacement and register position.
#[test]
fn splice_disp_over_splice_reg_parses() {
    let f = parse_ok(concat!(
        "module m\n",
        "comptime fn t(boff: int, breg: Reg) -> Code {\n",
        "    return asm {\n",
        "        sub.w   {boff}({breg}), d1\n",
        "    }\n",
        "}\n",
    ));
    let Item::ComptimeFn(fun) = f.items.iter().find(|i| matches!(i, Item::ComptimeFn(_))).unwrap()
    else {
        panic!()
    };
    let instr = extract_single_instr(fun);
    let Operand::DispInd { disp, inner, .. } = &instr.operands[0] else {
        panic!("expected DispInd, got {:?}", instr.operands[0]);
    };
    assert!(matches!(disp, Expr::Path(p) if p.segments == ["boff"]));
    // Inner register is itself a splice part.
    let Operand::Ind { parts, .. } = &**inner else { panic!() };
    assert!(matches!(&parts[0].0, Expr::Path(p) if p.segments == ["breg"]));
}

// A tiny helper: pull the single instruction out of `return asm { <one instr> }`.
fn extract_single_instr(fun: &ComptimeFnDecl) -> &InstrLine {
    // fn body is a block of stmts; find the `return asm {...}`.
    let asm = find_asm(&fun.body).expect("no asm block in fn body");
    let AsmStmt::Instr(instr) = &asm[0] else { panic!("first asm stmt not an instr") };
    instr
}

fn find_asm(stmts: &[Stmt]) -> Option<&[AsmStmt]> {
    for s in stmts {
        if let Some(a) = asm_of_stmt(s) {
            return Some(a);
        }
    }
    None
}

fn asm_of_stmt(s: &Stmt) -> Option<&[AsmStmt]> {
    let e = match s {
        Stmt::Return { value: Some(e), .. } => e,
        Stmt::Expr(e) => e,
        _ => return None,
    };
    if let Expr::Asm { body, .. } = e {
        Some(body)
    } else {
        None
    }
}

// ---- F1 eval / bytes: emp-internal parity -----------------------------------
//
// The spliced-displacement operand must lower to the SAME bytes as the literal
// displacement it evaluates to. Two procs, one using a template with a spliced
// displacement, one hand-writing the literal — byte-identical.

const SPLICE_SRC: &str = concat!(
    "module m\n",
    "comptime fn axis(boff: int, breg: Reg) -> Code {\n",
    "    return asm {\n",
    "        sub.w   {boff}({breg}), d1\n",
    "    }\n",
    "}\n",
    "pub proc Spliced () {\n",
    "    axis(2, a3)\n",
    "    rts\n",
    "}\n",
);

const LITERAL_SRC: &str = concat!(
    "module m\n",
    "pub proc Literal () {\n",
    "    sub.w   2(a3), d1\n",
    "    rts\n",
    "}\n",
);

#[test]
fn splice_disp_bytes_match_literal() {
    let (sm, sd) = lower(SPLICE_SRC);
    assert!(errors(&sd).is_empty(), "spliced lower errors: {:?}", errors(&sd));
    let (lm, ld) = lower(LITERAL_SRC);
    assert!(errors(&ld).is_empty(), "literal lower errors: {:?}", errors(&ld));

    // A single proc lands in the default `text` section; the proc name does not
    // affect emitted bytes, so the whole section is the instruction stream.
    let _ = section(&sm, "text");
    let sbytes = linked_section_bytes(&sm, "text");
    let lbytes = linked_section_bytes(&lm, "text");
    assert_eq!(sbytes, lbytes, "spliced-disp bytes must equal literal-disp bytes");
}

// ---- F1 indexed brief-extension form: `{d8}(An,Xn.w)` -----------------------
//
// The spliced-displacement grammar extends TRIVIALLY to the indexed brief form:
// `{off}(a3,d4.w)` routes through the same displacement-indirect parse, and eval
// dispatches a 2-part inner to `map_an_indexed`, which range-checks to i8. Pin
// byte-parity against the literal `4(a3,d4.w)`.

const SPLICE_IDX_SRC: &str = concat!(
    "module m\n",
    "comptime fn axisi(boff: int, breg: Reg) -> Code {\n",
    "    return asm {\n",
    "        move.w  {boff}({breg},d4.w), d1\n",
    "    }\n",
    "}\n",
    "pub proc SplicedIdx () {\n",
    "    axisi(4, a3)\n",
    "    rts\n",
    "}\n",
);

const LITERAL_IDX_SRC: &str = concat!(
    "module m\n",
    "pub proc LiteralIdx () {\n",
    "    move.w  4(a3,d4.w), d1\n",
    "    rts\n",
    "}\n",
);

#[test]
fn splice_disp_indexed_bytes_match_literal() {
    let (sm, sd) = lower(SPLICE_IDX_SRC);
    assert!(errors(&sd).is_empty(), "spliced-idx lower errors: {:?}", errors(&sd));
    let (lm, ld) = lower(LITERAL_IDX_SRC);
    assert!(errors(&ld).is_empty(), "literal-idx lower errors: {:?}", errors(&ld));
    let sbytes = linked_section_bytes(&sm, "text");
    let lbytes = linked_section_bytes(&lm, "text");
    assert_eq!(sbytes, lbytes, "spliced indexed-disp bytes must equal literal");
}

// ---- F1 negative: a Reg splice in displacement position ---------------------

#[test]
fn reg_splice_in_disp_position_is_splice_kind() {
    // `boff` is declared `Reg`, spliced into displacement position — must be the
    // `[asm.splice-kind]` diagnostic naming the expected int class.
    let (_m, d) = lower(concat!(
        "module m\n",
        "comptime fn bad(boff: Reg, breg: Reg) -> Code {\n",
        "    return asm {\n",
        "        sub.w   {boff}({breg}), d1\n",
        "    }\n",
        "}\n",
        "pub proc P () {\n",
        "    bad(d0, a3)\n",
        "    rts\n",
        "}\n",
    ));
    let errs = errors(&d);
    assert!(
        errs.iter().any(|e| e.contains("[asm.splice-kind]")),
        "expected [asm.splice-kind] diagnostic, got: {errs:?}"
    );
}
