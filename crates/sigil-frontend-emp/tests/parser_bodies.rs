use sigil_frontend_emp::ast::*;
use sigil_frontend_emp::parse_str;

fn ok(src: &str) -> File {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "diagnostics: {diags:?}");
    file
}

fn first_proc(f: &File) -> &ProcDecl {
    for it in &f.items { if let Item::Proc(p) = it { return p; } }
    panic!("no proc");
}

#[test]
fn proc_header_forms() {
    let f = ok("module m\nproc init (a0: *Sst) falls_into wait {\n}\nproc wait (a0: *Sst) clobbers(d0, d1) {\n}\n");
    let Item::Proc(p) = &f.items[0] else { panic!() };
    assert_eq!(p.falls_into.as_deref(), Some("wait"));
    assert_eq!(p.params[0].0, "a0");
    assert!(matches!(p.params[0].1, Type::Ptr(_)));
    let Item::Proc(p) = &f.items[1] else { panic!() };
    assert_eq!(p.clobbers, vec!["d0", "d1"]);
}

#[test]
fn instructions_and_labels() {
    let f = ok(concat!(
        "module m\n",
        "proc wait (a0: *Sst) {\n",
        "    subq.b  #1, timer(a0)\n",
        "    bne     .draw\n",
        ".draw:\n",
        "    jmp     Draw_Sprite\n",
        "}\n"));
    let p = first_proc(&f);
    assert_eq!(p.body.len(), 4);

    let AsmStmt::Instr(i) = &p.body[0] else { panic!() };
    assert_eq!(i.mnemonic, vec![TextOrSplice::Text("subq".into())]);
    assert_eq!(i.size, Some(TextOrSplice::Text("b".into())));
    assert!(matches!(&i.operands[0], Operand::Imm(Expr::Int(1, _))));
    assert!(matches!(&i.operands[1], Operand::DispInd { .. }));

    let AsmStmt::Instr(i) = &p.body[1] else { panic!() };
    assert!(i.size.is_none());
    // `.draw` target is a Plain operand whose expr is the local-label path ".draw"
    assert!(matches!(&i.operands[0], Operand::Plain { .. }));

    assert!(matches!(&p.body[2], AsmStmt::Label { name, export: false, .. } if name == "draw"));
}

#[test]
fn addressing_mode_operands() {
    let f = ok(concat!(
        "module m\n",
        "proc x () {\n",
        "    move.w  (a0)+, -(a7)\n",
        "    move.l  (VDP_Ctrl).l, d0\n",
        "    move.b  4(a0, d0.w), d1\n",
        "    move.w  Player_1.x_pos, d0\n",
        "}\n"));
    let p = first_proc(&f);
    let AsmStmt::Instr(i) = &p.body[0] else { panic!() };
    assert!(matches!(&i.operands[0], Operand::PostInc(_)));
    assert!(matches!(&i.operands[1], Operand::PreDec(_)));
    let AsmStmt::Instr(i) = &p.body[1] else { panic!() };
    let Operand::Ind { size: Some(TextOrSplice::Text(s)), .. } = &i.operands[0] else { panic!() };
    assert_eq!(s, "l");
    let AsmStmt::Instr(i) = &p.body[2] else { panic!() };
    let Operand::DispInd { inner, .. } = &i.operands[0] else { panic!() };
    let Operand::Ind { parts, .. } = &**inner else { panic!() };
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[1].1, Some(TextOrSplice::Text("w".into()))); // d0.w
}

#[test]
fn statement_position_call() {
    let f = ok("module m\nproc x (a0: *Sst) {\n    spawn(SeedDef, flip: inherit)\n}\n");
    let p = first_proc(&f);
    assert!(matches!(&p.body[0], AsmStmt::Call(Expr::Call { .. })));
}

#[test]
fn export_label() {
    let f = ok("module m\nproc x () {\nexport .visible:\n    rts\n}\n");
    let p = first_proc(&f);
    assert!(matches!(&p.body[0], AsmStmt::Label { export: true, .. }));
}

#[test]
fn spaced_paren_operand_is_not_a_call() {
    let f = ok("module m\nproc x () {\n    jmp (a0)\n}\n");
    let p = first_proc(&f);
    let AsmStmt::Instr(i) = &p.body[0] else { panic!("got {:?}", p.body[0]) };
    assert!(matches!(&i.operands[0], Operand::Ind { .. }));
}

#[test]
fn call_normalized_dispind_keeps_sizes() {
    let f = ok("module m\nproc x () {\n    move.b  timer(a0, d0.w), d1\n    move.l  timer(a0).l, d0\n}\n");
    let p = first_proc(&f);
    let AsmStmt::Instr(i) = &p.body[0] else { panic!() };
    let Operand::DispInd { inner, .. } = &i.operands[0] else { panic!() };
    let Operand::Ind { parts, .. } = &**inner else { panic!() };
    assert_eq!(parts[1].1, Some(TextOrSplice::Text("w".into())));
    let AsmStmt::Instr(i) = &p.body[1] else { panic!() };
    assert_eq!(i.operands.len(), 2); // `, d0` must survive
    let Operand::DispInd { inner, .. } = &i.operands[0] else { panic!() };
    let Operand::Ind { size: Some(TextOrSplice::Text(s)), .. } = &**inner else { panic!() };
    assert_eq!(s, "l");
}

#[test]
fn mnemonic_size_is_restricted_to_bwls() {
    // typo `bne.draw` — `.draw` must become the branch-target operand, not a size
    let f = ok("module m\nproc x () {\n    bne.draw\n    bra.s .draw\n.draw:\n}\n");
    let p = first_proc(&f);
    let AsmStmt::Instr(i) = &p.body[0] else { panic!() };
    assert!(i.size.is_none());
    assert_eq!(i.operands.len(), 1);
    let AsmStmt::Instr(i) = &p.body[1] else { panic!() };
    assert_eq!(i.size, Some(TextOrSplice::Text("s".into())));
}

#[test]
fn instr_span_excludes_newline() {
    let src = "module m\nproc x () {\n    bne .draw\n}\n";
    let f = ok(src);
    let p = first_proc(&f);
    let AsmStmt::Instr(i) = &p.body[0] else { panic!() };
    assert_eq!(&src[i.span.start as usize..i.span.end as usize], "bne .draw");
}

#[test]
fn comptime_fn_with_stmts() {
    let f = ok(concat!(
        "module m\n",
        "comptime fn deform_sine(amplitude: int, period: int) -> [i8; 256] {\n",
        "    ensure(256 % period == 0, \"bad period {period}\")\n",
        "    return comptime for i in 0..256 { as.int(amplitude * as.sin(TAU * i / period)) }\n",
        "}\n"));
    let Item::ComptimeFn(cf) = &f.items[0] else { panic!() };
    assert_eq!(cf.name, "deform_sine");
    assert_eq!(cf.params.len(), 2);
    assert!(cf.ret.is_some());
    assert!(matches!(&cf.body[0], Stmt::Expr(Expr::Call { .. })));   // ensure(...)
    assert!(matches!(&cf.body[1], Stmt::Return { value: Some(Expr::For { .. }), .. }));
}

#[test]
fn let_tuple_if_while_var_block() {
    let f = ok(concat!(
        "module m\n",
        "comptime fn fstring(fmt: string) -> (Data, Code) {\n",
        "    if fmt == \"\" { return (Data.empty, Code.empty) }\n",
        "    let (lit, tok, rest) = split_token(fmt)\n",
        "    comptime block {\n",
        "        comptime var prev: int = -1\n",
        "        while prev < 0 {\n",
        "            prev = 1\n",
        "        }\n",
        "    }\n",
        "    patch count: u8\n",
        "    bind count = 3\n",
        "    return (bytes(lit), Code.empty)\n",
        "}\n"));
    let Item::ComptimeFn(cf) = &f.items[0] else { panic!() };
    assert!(matches!(&cf.body[0], Stmt::If(Expr::If { .. })));
    assert!(matches!(&cf.body[1], Stmt::LetTuple { names, .. } if names.len() == 3));
    let Stmt::ComptimeBlock { body, .. } = &cf.body[2] else { panic!() };
    assert!(matches!(&body[0], Stmt::Var { .. }));
    assert!(matches!(&body[1], Stmt::While { .. }));
    assert!(matches!(&cf.body[3], Stmt::Patch { .. }));
    assert!(matches!(&cf.body[4], Stmt::Bind { .. }));
}

#[test]
fn no_struct_lit_in_condition_position() {
    // `if fmt { }` — fmt must be a Path condition, block is the then-branch
    let f = ok("module m\ncomptime fn f(fmt: int) -> int {\n    if fmt { return 1 }\n    return 2\n}\n");
    let Item::ComptimeFn(cf) = &f.items[0] else { panic!() };
    assert!(matches!(&cf.body[0], Stmt::If(Expr::If { cond, .. }) if matches!(&**cond, Expr::Path(_))));
    // parenthesized arithmetic in condition is legal (verifies the paren
    // save/clear/restore of no_struct_lit):
    let f = ok("module m\ncomptime fn g() -> int {\n    if (1 + 2) == 3 { return 1 }\n    return 2\n}\n");
    let Item::ComptimeFn(cf) = &f.items[0] else { panic!() };
    assert!(matches!(&cf.body[0], Stmt::If(_)));
}

#[test]
fn deep_block_nesting_is_an_error_not_an_abort() {
    // newline-per-statement variant
    let opens = "if x {\n".repeat(600);
    let closes = "}\n".repeat(600);
    let src = format!("module m\ncomptime fn f(x: int) -> int {{\n{opens}{closes}return 1\n}}\nconst GOOD: u8 = 1\n");
    let (f, diags) = parse_str(&src);
    assert!(!diags.is_empty());
    assert!(diags.len() < 50, "diagnostic flood: {}", diags.len());
    assert!(f.items.iter().any(|i| matches!(i, Item::Const(c) if c.name == "GOOD")));

    // one-line variant (no newlines inside the bomb)
    let opens = "if x { ".repeat(300);
    let closes = "} ".repeat(300);
    let src = format!("module m\ncomptime fn f(x: int) -> int {{\n{opens}{closes}\nreturn 1\n}}\nconst GOOD: u8 = 1\n");
    let (f, diags) = parse_str(&src);
    assert!(!diags.is_empty());
    assert!(f.items.iter().any(|i| matches!(i, Item::Const(c) if c.name == "GOOD")));
}

#[test]
fn patch_and_bind_are_contextual_let_is_reserved() {
    // variable named patch/bind: assignment works
    let f = ok("module m\ncomptime fn f() -> int {\n    comptime var patch: int = 0\n    patch = 5\n    bind = 6\n    return patch\n}\n");
    let Item::ComptimeFn(cf) = &f.items[0] else { panic!() };
    assert!(matches!(&cf.body[1], Stmt::Assign { target, .. } if target.segments == vec!["patch"]));
    assert!(matches!(&cf.body[2], Stmt::Assign { target, .. } if target.segments == vec!["bind"]));
    // let is reserved: exactly one diagnostic, no cascade
    let (_, diags) = parse_str("module m\ncomptime fn f() -> int {\n    let = 5\n    return 1\n}\n");
    assert_eq!(diags.len(), 1, "{diags:?}");
}

#[test]
fn dangling_else_is_diagnosed() {
    let (_, diags) = parse_str("module m\ncomptime fn f() -> int {\n    else { 1 }\n    return 1\n}\n");
    assert_eq!(diags.len(), 1, "{diags:?}");
}

#[test]
fn asm_template_with_splices() {
    let f = ok(concat!(
        "module m\n",
        "comptime fn set_vdp_reg(reg: VdpReg, val: u8) -> Code {\n",
        "    return asm {\n",
        "        move.b  #{val}, (VDP_Shadow_Table + {reg}).w\n",
        "        ori.l   #{1 << reg}, (VDP_Dirty_Mask).w\n",
        "    }\n",
        "}\n"));
    let Item::ComptimeFn(cf) = &f.items[0] else { panic!() };
    let Stmt::Return { value: Some(Expr::Asm { body, .. }), .. } = &cf.body[0] else { panic!() };
    assert_eq!(body.len(), 2);
    let AsmStmt::Instr(i) = &body[0] else { panic!() };
    // #{val} → Imm(splice expr)
    assert!(matches!(&i.operands[0], Operand::Imm(Expr::Path(_))));
    // (VDP_Shadow_Table + {reg}).w → Ind with Binary part and trailing size
    let Operand::Ind { parts, size: Some(_), .. } = &i.operands[1] else { panic!() };
    assert!(matches!(&parts[0].0, Expr::Binary { .. }));
}

#[test]
fn spliced_mnemonic_width_and_label() {
    let f = ok(concat!(
        "module m\n",
        "comptime fn assert_cc(w: Width, a: Operand, cc: Cc, b: Operand) -> Code {\n",
        "    return asm {\n",
        "        cmp.{w}   {b}, {a}\n",
        "        b{cc}     .ok\n",
        "        trap      #DEBUG_TRAP\n",
        "    .ok:\n",
        "    }\n",
        "}\n"));
    let Item::ComptimeFn(cf) = &f.items[0] else { panic!() };
    let Stmt::Return { value: Some(Expr::Asm { body, .. }), .. } = &cf.body[0] else { panic!() };
    let AsmStmt::Instr(i) = &body[0] else { panic!() };
    assert!(matches!(i.size, Some(TextOrSplice::Splice(_))));
    assert!(matches!(&i.operands[0], Operand::Splice(_)));
    let AsmStmt::Instr(i) = &body[1] else { panic!() };
    assert_eq!(i.mnemonic.len(), 2); // Text("b") + Splice(cc)
    assert!(matches!(&body[3], AsmStmt::Label { .. }));
}

