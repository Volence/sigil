//! Integration tests for comptime control flow (Spec 2, Plan 2 — T5):
//! `for` (as an array expression and as a side-effecting statement), `while`,
//! `comptime block`, `comptime var`, and assignment. Each case parses a full
//! `.emp` file (asserting a clean parse), then evaluates a named `const`.
use sigil_frontend_emp::eval::eval_const;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::Value;

/// Parse `src` (asserting a clean parse) and evaluate the const named `name`.
fn eval(src: &str, name: &str) -> (Option<Value>, Vec<sigil_span::Diagnostic>) {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    eval_const(&file, name)
}

fn int(n: i128) -> Value {
    Value::Int(n)
}

fn arr(ns: &[i128]) -> Value {
    Value::Array(ns.iter().copied().map(Value::Int).collect())
}

// ---- `for` as an array expression --------------------------------------

#[test]
fn for_over_range_yields_array_of_body_values() {
    // `for i in 0..4 { i * i }` collects each body value into an Array.
    let src = "module m\n\
        comptime fn squares() -> int { return for i in 0..4 { i * i } }\n\
        const R = squares()\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(arr(&[0, 1, 4, 9])));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn for_over_array_literal_yields_array() {
    let src = "module m\n\
        comptime fn f() -> int { return for x in [10, 20, 30] { x + 1 } }\n\
        const R = f()\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(arr(&[11, 21, 31])));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn for_over_empty_range_yields_empty_array() {
    let src = "module m\n\
        comptime fn f() -> int { return for i in 5..5 { i } }\n\
        const R = f()\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(Value::Array(vec![])));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn for_over_non_iterable_errors() {
    let src = "module m\n\
        comptime fn f() -> int { return for i in 5 { i } }\n\
        const R = f()\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("for expects a range or array")),
        "diagnostics were {diags:?}"
    );
}

// ---- fold-style accumulation: comptime var + for + assign (CORE T5) ----

#[test]
fn sum_with_comptime_var_for_and_assign() {
    // The core scenario: a mutable accumulator updated by a side-effecting
    // `for` loop. sum(5) = 0+1+2+3+4 = 10.
    let src = "module m\n\
        comptime fn sum(n: int) -> int {\n\
        \x20   comptime var acc = 0\n\
        \x20   for i in 0..n { acc = acc + i }\n\
        \x20   return acc\n\
        }\n\
        const R = sum(5)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(10)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- `while` with comptime var -----------------------------------------

#[test]
fn while_countdown_with_comptime_vars() {
    let src = "module m\n\
        comptime fn countdown(n: int) -> int {\n\
        \x20   comptime var x = n\n\
        \x20   comptime var steps = 0\n\
        \x20   while x > 0 {\n\
        \x20       x = x - 1\n\
        \x20       steps = steps + 1\n\
        \x20   }\n\
        \x20   return steps\n\
        }\n\
        const R = countdown(4)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(4)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn while_false_body_never_runs() {
    let src = "module m\n\
        comptime fn f() -> int {\n\
        \x20   comptime var hits = 0\n\
        \x20   while false { hits = hits + 1 }\n\
        \x20   return hits\n\
        }\n\
        const R = f()\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(0)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn while_non_bool_condition_errors() {
    let src = "module m\n\
        comptime fn f() -> int {\n\
        \x20   while 3 { return 1 }\n\
        \x20   return 2\n\
        }\n\
        const R = f()\n";
    let (v, diags) = eval(src, "R");
    // Loop stops on the bad condition; falls through to `return 2`.
    assert_eq!(v, Some(int(2)));
    assert!(
        diags.iter().any(|d| d.message.contains("while condition must be bool")),
        "diagnostics were {diags:?}"
    );
}

// ---- `return` propagation out of loops ---------------------------------

#[test]
fn return_inside_for_body_exits_fn() {
    // The loop stops at i == 2 and the fn returns that value.
    let src = "module m\n\
        comptime fn f() -> int {\n\
        \x20   for i in 0..10 { if i == 2 { return i } }\n\
        \x20   return 99\n\
        }\n\
        const R = f()\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(2)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn return_inside_while_body_exits_fn() {
    let src = "module m\n\
        comptime fn f() -> int {\n\
        \x20   comptime var i = 0\n\
        \x20   while true { if i == 3 { return i }  i = i + 1 }\n\
        \x20   return 99\n\
        }\n\
        const R = f()\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(3)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- `comptime block` scoping ------------------------------------------

#[test]
fn comptime_var_inside_block_is_dead_after_block() {
    // `secret` is declared in a nested comptime block; referencing it after the
    // block closes is an unknown name.
    let src = "module m\n\
        comptime fn f() -> int {\n\
        \x20   comptime block {\n\
        \x20       comptime var secret = 5\n\
        \x20   }\n\
        \x20   return secret\n\
        }\n\
        const R = f()\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("unknown name `secret`")),
        "diagnostics were {diags:?}"
    );
}

#[test]
fn comptime_block_can_mutate_outer_comptime_var() {
    // A comptime block runs in the enclosing comptime context, so assigning to
    // an outer `comptime var` from inside it reaches the outer binding.
    let src = "module m\n\
        comptime fn f() -> int {\n\
        \x20   comptime var n = 1\n\
        \x20   comptime block { n = n + 41 }\n\
        \x20   return n\n\
        }\n\
        const R = f()\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(42)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- assignment errors -------------------------------------------------

#[test]
fn assign_to_let_is_immutable_error() {
    let src = "module m\n\
        comptime fn f() -> int {\n\
        \x20   let y = 1\n\
        \x20   y = 2\n\
        \x20   return y\n\
        }\n\
        const R = f()\n";
    let (v, diags) = eval(src, "R");
    // The `let` binding is unchanged, so R = 1.
    assert_eq!(v, Some(int(1)));
    assert!(
        diags.iter().any(|d| d.message.contains("immutable")),
        "diagnostics were {diags:?}"
    );
}

#[test]
fn assign_to_unbound_name_errors() {
    let src = "module m\n\
        comptime fn f() -> int {\n\
        \x20   z = 5\n\
        \x20   return 0\n\
        }\n\
        const R = f()\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(0)));
    assert!(
        diags.iter().any(|d| d.message.contains("cannot assign to unbound name `z`")),
        "diagnostics were {diags:?}"
    );
}

#[test]
fn field_assignment_is_not_yet_supported() {
    // `a.x = 2` is a multi-segment (field) assignment target — out of scope
    // until Plan 3. It must be diagnosed, not silently applied or crash.
    let src = "module m\n\
        comptime fn f() -> int {\n\
        \x20   comptime var a = Point{x: 1}\n\
        \x20   a.x = 2\n\
        \x20   return 0\n\
        }\n\
        const R = f()\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(0)));
    assert!(
        diags.iter().any(|d| d.message.contains("field assignment not yet supported")),
        "diagnostics were {diags:?}"
    );
}

#[test]
fn return_surfaced_from_for_iterable_exits_fn() {
    // A `return` inside the FOR ITERABLE expression exits the fn before any
    // iteration: f(true) returns 9 (never loops); f(false) iterates [1, 2] and
    // falls through to `return 100`.
    let src = "module m\n\
        comptime fn f(c: bool) -> int {\n\
        \x20   let a = for x in (if c { return 9 } else { [1, 2] }) { x }\n\
        \x20   return 100\n\
        }\n\
        const T = f(true)\n\
        const F = f(false)\n";
    let (t, dt) = eval(src, "T");
    assert_eq!(t, Some(int(9)));
    assert!(dt.is_empty(), "unexpected diagnostics: {dt:?}");
    let (f, df) = eval(src, "F");
    assert_eq!(f, Some(int(100)));
    assert!(df.is_empty(), "unexpected diagnostics: {df:?}");
}

// ---- boundedness -------------------------------------------------------

#[test]
fn infinite_while_is_bounded_by_step_budget_not_a_hang() {
    // `while 1 == 1 { }` never terminates on its own; the per-iteration step
    // budget must stop it with a diagnostic rather than hanging the test.
    let src = "module m\n\
        comptime fn spin() -> int {\n\
        \x20   while 1 == 1 { }\n\
        \x20   return 0\n\
        }\n\
        const R = spin()\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("budget")),
        "diagnostics were {diags:?}"
    );
}
