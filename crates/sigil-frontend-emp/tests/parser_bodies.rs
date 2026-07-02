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
fn instr_span_excludes_newline() {
    let src = "module m\nproc x () {\n    bne .draw\n}\n";
    let f = ok(src);
    let p = first_proc(&f);
    let AsmStmt::Instr(i) = &p.body[0] else { panic!() };
    assert_eq!(&src[i.span.start as usize..i.span.end as usize], "bne .draw");
}
