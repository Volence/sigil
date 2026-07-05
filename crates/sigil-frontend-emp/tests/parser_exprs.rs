use sigil_frontend_emp::ast::*;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::parser::parse_expr_for_tests;

fn expr(src: &str) -> Expr {
    parse_expr_for_tests(src)
}

#[test]
fn precedence_and_associativity() {
    // 1 + 2 * 3 == (1 + (2 * 3))
    let Expr::Binary { op: BinOp::Add, rhs, .. } = expr("1 + 2 * 3") else { panic!() };
    assert!(matches!(*rhs, Expr::Binary { op: BinOp::Mul, .. }));

    // shifts bind tighter than comparison: `1 << reg == x` → ((1<<reg) == x)
    let Expr::Binary { op: BinOp::Eq, lhs, .. } = expr("1 << reg == x") else { panic!() };
    assert!(matches!(*lhs, Expr::Binary { op: BinOp::Shl, .. }));
}

#[test]
fn calls_with_named_args() {
    let Expr::Call { callee, args, .. } = expr("parallax_section(cfg: C, bands: b)") else { panic!() };
    assert_eq!(callee.segments, vec!["parallax_section"]);
    assert_eq!(args[0].name.as_deref(), Some("cfg"));
    assert_eq!(args[1].name.as_deref(), Some("bands"));
}

#[test]
fn struct_array_and_range_literals() {
    let Expr::StructLit { ty, fields, .. } = expr("ObjDef{ code: init, size: Size{w: 16, h: 28} }") else { panic!() };
    assert_eq!(ty.segments, vec!["ObjDef"]);
    assert_eq!(fields.len(), 2);
    assert!(matches!(&fields[1].1, Expr::StructLit { .. }));

    let Expr::ArrayLit { elems, .. } = expr("[4, 113, 32]") else { panic!() };
    assert_eq!(elems.len(), 3);

    assert!(matches!(expr("0..256"), Expr::Range { .. }));
}

#[test]
fn concat_and_method_style_calls() {
    // bytes(lit) ++ d
    let Expr::Binary { op: BinOp::Concat, .. } = expr("bytes(lit) ++ d") else { panic!() };
    // bands.map(band_entry) parses as a Call with dotted callee path
    let Expr::Call { callee, .. } = expr("bands.map(band_entry)") else { panic!() };
    assert_eq!(callee.segments, vec!["bands", "map"]);
}

#[test]
fn unary_and_parens() {
    assert!(matches!(expr("-1"), Expr::Unary { op: UnOp::Neg, .. }));
    assert!(matches!(expr("!(a && b)"), Expr::Unary { op: UnOp::Not, .. }));
    assert!(matches!(expr("~x"), Expr::Unary { op: UnOp::BitNot, .. }));
}

#[test]
fn deep_nesting_is_an_error_not_an_abort() {
    let inner = format!("{}1{}", "(".repeat(600), ")".repeat(600));
    let (_, diags) = parse_str(&format!("module m\nconst X = {inner}\n"));
    assert!(!diags.is_empty());
}

#[test]
fn deep_unary_chain_is_an_error_not_an_abort() {
    let inner = format!("{}1", "-".repeat(10_000));
    let (_, diags) = parse_str(&format!("module m\nconst X = {inner}\n"));
    assert!(!diags.is_empty());
}

#[test]
fn error_arm_preserves_closers() {
    // S{a:} — one diagnostic for the missing value; the `}` must NOT be eaten,
    // so there is no bogus "expected `}`, found Eof" cascade.
    let (_, diags) = parse_str("module m\nconst X = S{a:}\n");
    assert_eq!(diags.len(), 1, "{diags:?}");
}

#[test]
fn trailing_commas_and_one_tuple() {
    let Expr::Call { args, .. } = expr("f(a: 1, b: 2,)") else { panic!() };
    assert_eq!(args.len(), 2);
    assert!(matches!(expr("(1,)"), Expr::TupleLit { elems, .. } if elems.len() == 1));
    // plain grouping unchanged:
    assert!(matches!(expr("(1)"), Expr::Int(1, _)));
}

// ---- lambdas & the `|>` pipe (T6a) ----

#[test]
fn lambda_one_param() {
    // `|x| x + 1` → Lambda with one param and a `+` body.
    let Expr::Lambda { params, body, .. } = expr("|x| x + 1") else { panic!() };
    assert_eq!(params, vec!["x"]);
    assert!(matches!(*body, Expr::Binary { op: BinOp::Add, .. }));
}

#[test]
fn lambda_multi_param() {
    let Expr::Lambda { params, body, .. } = expr("|a, b| a * b") else { panic!() };
    assert_eq!(params, vec!["a", "b"]);
    assert!(matches!(*body, Expr::Binary { op: BinOp::Mul, .. }));
}

#[test]
fn pipe_into_bare_name() {
    // `xs |> f` desugars to `f(xs)`.
    let Expr::Call { callee, args, .. } = expr("xs |> f") else { panic!() };
    assert_eq!(callee.segments, vec!["f"]);
    assert_eq!(args.len(), 1);
    assert!(args[0].name.is_none());
    assert!(matches!(&args[0].value, Expr::Path(p) if p.segments == vec!["xs"]));
}

#[test]
fn pipe_into_call_prepends_arg() {
    // `xs |> map(g)` → `map(xs, g)` — piped value is the FIRST arg.
    let Expr::Call { callee, args, .. } = expr("xs |> map(g)") else { panic!() };
    assert_eq!(callee.segments, vec!["map"]);
    assert_eq!(args.len(), 2);
    assert!(matches!(&args[0].value, Expr::Path(p) if p.segments == vec!["xs"]));
    assert!(matches!(&args[1].value, Expr::Path(p) if p.segments == vec!["g"]));
}

#[test]
fn pipe_chains_left_assoc() {
    // `xs |> f |> g` → `g(f(xs))`.
    let Expr::Call { callee, args, .. } = expr("xs |> f |> g") else { panic!() };
    assert_eq!(callee.segments, vec!["g"]);
    assert_eq!(args.len(), 1);
    let Expr::Call { callee: inner_callee, args: inner_args, .. } = &args[0].value else { panic!() };
    assert_eq!(inner_callee.segments, vec!["f"]);
    assert!(matches!(&inner_args[0].value, Expr::Path(p) if p.segments == vec!["xs"]));
}

#[test]
fn pipe_into_call_with_lambda_arg() {
    // `frames |> map(|f| f + 1)` → `map(frames, |f| f + 1)`.
    let Expr::Call { callee, args, .. } = expr("frames |> map(|f| f + 1)") else { panic!() };
    assert_eq!(callee.segments, vec!["map"]);
    assert_eq!(args.len(), 2);
    assert!(matches!(&args[0].value, Expr::Path(p) if p.segments == vec!["frames"]));
    assert!(matches!(&args[1].value, Expr::Lambda { .. }));
}

#[test]
fn pipe_is_loosest_precedence() {
    // `a + b |> f` → `f(a + b)`: `+` binds tighter than `|>`.
    let Expr::Call { callee, args, .. } = expr("a + b |> f") else { panic!() };
    assert_eq!(callee.segments, vec!["f"]);
    assert!(matches!(&args[0].value, Expr::Binary { op: BinOp::Add, .. }));
}

#[test]
fn pipe_into_non_call_target_diagnoses() {
    // `a |> f + b`: the target is a Binary (`f + b`), not a call/name — this is
    // a diagnostic, NOT an orphaned `+ b` producing an "expected end of line".
    let (_, diags) = parse_str("module m\nconst R = a |> f + b\n");
    assert_eq!(diags.len(), 1, "{diags:?}");
    let msg = &diags[0].message;
    assert!(msg.contains("|>") || msg.contains("call or name"), "{msg}");
}

#[test]
fn infix_bitor_and_or_unaffected() {
    // A `|` following a primary is still infix bit-or; `||` is logical or.
    assert!(matches!(expr("a | b"), Expr::Binary { op: BinOp::BitOr, .. }));
    assert!(matches!(expr("a || b"), Expr::Binary { op: BinOp::Or, .. }));
}

#[test]
fn method_paths_still_parse() {
    // `.` path handling is unchanged by the lambda/pipe additions.
    let Expr::Call { callee, args, .. } = expr("xs.map(g)") else { panic!() };
    assert_eq!(callee.segments, vec!["xs", "map"]);
    assert_eq!(args.len(), 1);
    assert!(matches!(expr("xs.len"), Expr::Path(p) if p.segments == vec!["xs", "len"]));
}
