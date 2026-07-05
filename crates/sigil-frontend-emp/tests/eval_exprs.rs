//! Integration tests for the pure expression evaluator (Spec 2, Plan 2 — T2).
//!
//! Each case parses a bare expression via [`parse_expr_for_tests`], evaluates it
//! with a fresh [`Evaluator`]/[`Env`], and asserts on the resulting [`Value`] and
//! the collected diagnostics.
use sigil_frontend_emp::eval::{Env, Evaluator};
use sigil_frontend_emp::parser::parse_expr_for_tests;
use sigil_frontend_emp::value::Value;
use sigil_span::Diagnostic;

/// Parse+evaluate `src`, returning the value and any diagnostics.
fn eval(src: &str) -> (Value, Vec<Diagnostic>) {
    let e = parse_expr_for_tests(src);
    let mut ev = Evaluator::new();
    let mut env = Env::new();
    let v = ev.eval_expr(&e, &mut env);
    (v, ev.diags)
}

/// Assert `src` evaluates to `want` with no diagnostics.
fn ok(src: &str, want: Value) {
    let (v, diags) = eval(src);
    assert_eq!(v, want, "value for `{src}`");
    assert!(diags.is_empty(), "expected no diagnostics for `{src}`, got {diags:?}");
}

/// Assert `src` poisons with exactly one diagnostic whose message contains `needle`.
fn poison_with(src: &str, needle: &str) {
    let (v, diags) = eval(src);
    assert_eq!(v, Value::Poison, "expected Poison for `{src}`");
    assert_eq!(diags.len(), 1, "expected exactly one diagnostic for `{src}`, got {diags:?}");
    assert!(
        diags[0].message.contains(needle),
        "diagnostic for `{src}` was {:?}, expected to contain {needle:?}",
        diags[0].message
    );
}

fn int(n: i128) -> Value {
    Value::Int(n)
}

// ---- literals ----------------------------------------------------------

#[test]
fn literals() {
    ok("42", int(42));
    ok("1.5", Value::Float(1.5));
    ok("\"hi\"", Value::Str("hi".into()));
    ok("true", Value::Bool(true));
    ok("false", Value::Bool(false));
    ok("none", Value::Unit);
}

// ---- arithmetic --------------------------------------------------------

#[test]
fn arithmetic_and_precedence() {
    ok("1 + 2 * 3", int(7));
    ok("2 - 5", int(-3));
    ok("(1 + 2) * 3", int(9));
    ok("7 / 2", int(3)); // truncates toward zero
    ok("-7 / 2", int(-3)); // truncates toward zero (not floor)
    ok("7 % 3", int(1));
    ok("-7 % 3", int(-1)); // remainder takes the sign of the dividend
    ok("-5", int(-5));
    ok("~0", int(-1));
}

#[test]
fn division_and_modulo_by_zero() {
    poison_with("7 / 0", "division by zero");
    poison_with("7 % 0", "modulo by zero");
}

#[test]
fn integer_overflow_is_an_error() {
    // 1 << 127 sets the sign bit and overflows i128's positive range.
    poison_with("1 << 127", "overflow");
    // MIN / -1 overflows.
    ok("1 << 126", int(1i128 << 126));
}

// ---- shifts ------------------------------------------------------------

#[test]
fn shifts() {
    ok("1 << 4", int(16));
    ok("256 >> 2", int(64));
    ok("-8 >> 1", int(-4)); // arithmetic (sign-extending) right shift
    poison_with("1 << 200", "out of range");
    poison_with("1 << -1", "out of range");
}

// ---- bitwise -----------------------------------------------------------

#[test]
fn bitwise() {
    ok("0b1100 & 0b1010", int(8));
    ok("0b1100 | 0b1010", int(14));
    ok("0b1100 ^ 0b1010", int(6));
}

// ---- comparisons -------------------------------------------------------

#[test]
fn comparisons_yield_bool() {
    ok("3 < 5", Value::Bool(true));
    ok("5 <= 5", Value::Bool(true));
    ok("5 > 3", Value::Bool(true));
    ok("5 >= 6", Value::Bool(false));
    ok("\"a\" < \"b\"", Value::Bool(true));
    ok("1 == 1.0", Value::Bool(true)); // numeric promotion
    ok("1 == 2", Value::Bool(false));
    ok("2 != 3", Value::Bool(true));
    ok("1 == \"x\"", Value::Bool(false)); // cross-kind structural: not equal
    ok("1 != \"x\"", Value::Bool(true));
}

#[test]
fn ordering_on_bad_types_errors() {
    poison_with("true < false", "not defined");
}

// ---- logical short-circuit --------------------------------------------

#[test]
fn logical_short_circuit() {
    // rhs would divide by zero, but && short-circuits on a false lhs.
    ok("false && (1 / 0 == 0)", Value::Bool(false));
    // rhs would divide by zero, but || short-circuits on a true lhs.
    ok("true || (1 / 0 == 0)", Value::Bool(true));
    ok("true && false", Value::Bool(false));
    ok("true && true", Value::Bool(true));
    ok("false || true", Value::Bool(true));
    poison_with("1 && true", "not defined");
}

// ---- concat ------------------------------------------------------------

#[test]
fn concat() {
    ok("\"ab\" ++ \"cd\"", Value::Str("abcd".into()));
    ok("[1, 2] ++ [3]", Value::Array(vec![int(1), int(2), int(3)]));
    poison_with("\"a\" ++ 1", "not defined");
}

// ---- ranges ------------------------------------------------------------

#[test]
fn ranges() {
    ok("0..4", Value::Range { lo: 0, hi: 4 });
    poison_with("1.0..2.0", "range");
}

// ---- array & tuple literals -------------------------------------------

#[test]
fn array_and_tuple_literals() {
    ok("[1, 2, 3]", Value::Array(vec![int(1), int(2), int(3)]));
    ok("(1, true)", Value::Tuple(vec![int(1), Value::Bool(true)]));
}

// ---- floats ------------------------------------------------------------

#[test]
fn floats() {
    ok("1.5 + 2", Value::Float(3.5)); // int promotes to float
    ok("3.0 / 2.0", Value::Float(1.5));
    ok("2 * 2.5", Value::Float(5.0));
}

// ---- poison discipline -------------------------------------------------

#[test]
fn poison_propagates_without_extra_diagnostics() {
    // The div-by-zero poisons; the outer `+` must add no second diagnostic.
    let (v, diags) = eval("(1 / 0) + 5");
    assert_eq!(v, Value::Poison);
    assert_eq!(diags.len(), 1, "outer op must not add a diagnostic: {diags:?}");
    assert!(diags[0].message.contains("division by zero"));
}

// ---- unary type errors -------------------------------------------------

#[test]
fn unary_type_errors() {
    poison_with("-true", "not defined");
    poison_with("!1", "not defined");
    poison_with("~true", "not defined");
}

// ---- unknown name ------------------------------------------------------

#[test]
fn unknown_name_errors() {
    poison_with("nonesuch", "unknown name");
}
