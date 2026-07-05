//! Integration tests for the `ensure` / `ensure_fatal` comptime guards and
//! their `{interp}` message interpolation (Spec 2, Plan 2 — T7, spec §6.5/§6.7,
//! D-P2.19). Each case parses a full `.emp` file (asserting a clean parse), then
//! evaluates a named `const` whose value calls a `comptime fn` containing a
//! guard, and asserts on the resulting [`Value`] / diagnostics.
//!
//! A passing guard is silent and cheap (the message is never evaluated); a
//! failing `ensure` emits the interpolated message and poisons; a failing
//! `ensure_fatal` additionally aborts evaluation. Interpolation (`{expr}`) is
//! scoped to guard messages only — plain strings elsewhere keep `{...}`
//! literally.
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

/// True iff some diagnostic's message is exactly `msg`.
fn has_exact(diags: &[sigil_span::Diagnostic], msg: &str) -> bool {
    diags.iter().any(|d| d.message == msg)
}

// ---- passing guard is silent -------------------------------------------

#[test]
fn passing_guard_is_silent() {
    let src = "module m\ncomptime fn f(n: int) -> int { ensure(n > 0, \"n must be positive, got {n}\")\n    return n }\nconst R = f(5)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(5)));
    assert!(diags.is_empty(), "a passing guard must be silent, got {diags:?}");
}

// ---- failing guard emits the interpolated message ----------------------

#[test]
fn failing_guard_emits_interpolated_message() {
    let src = "module m\ncomptime fn f(n: int) -> int { ensure(n > 0, \"n must be positive, got {n}\")\n    return n }\nconst R = f(-3)\n";
    let (_, diags) = eval(src, "R");
    assert!(
        has_exact(&diags, "n must be positive, got -3"),
        "expected the interpolated message, got {diags:?}"
    );
}

#[test]
fn interpolation_of_len_expr() {
    let src = "module m\ncomptime fn f(xs: [int; 8]) -> int { ensure(xs.len <= 4, \"too many: {xs.len} > 4\")\n    return xs.len }\nconst R = f([1, 2, 3, 4, 5])\n";
    let (_, diags) = eval(src, "R");
    assert!(
        diags.iter().any(|d| d.message.contains("too many: 5 > 4")),
        "expected the interpolated len, got {diags:?}"
    );
}

#[test]
fn multiple_interpolations_and_escaped_braces() {
    let src = "module m\ncomptime fn f(a: int, b: int) -> int { ensure(false, \"a={a} b={b} literal={{}}\")\n    return a }\nconst R = f(1, 2)\n";
    let (_, diags) = eval(src, "R");
    assert!(
        has_exact(&diags, "a=1 b=2 literal={}"),
        "expected escaped braces + interpolation, got {diags:?}"
    );
}

// ---- ensure_fatal aborts; ensure continues -----------------------------

#[test]
fn ensure_fatal_aborts_evaluation() {
    // The `return 42` after a failing `ensure_fatal` must NOT run.
    let src = "module m\ncomptime fn f(x: int) -> int { ensure_fatal(x > 0, \"boom {x}\")\n    return 42 }\nconst R = f(-1)\n";
    let (v, diags) = eval(src, "R");
    assert!(
        has_exact(&diags, "boom -1"),
        "expected the interpolated fatal message, got {diags:?}"
    );
    assert_ne!(v, Some(int(42)), "ensure_fatal must stop before `return 42`");
}

#[test]
fn ensure_continues_after_failure() {
    // Contrast with the fatal case: a plain `ensure` poisons but keeps going,
    // so the trailing `return 42` still runs.
    let src = "module m\ncomptime fn f(x: int) -> int { ensure(x > 0, \"boom {x}\")\n    return 42 }\nconst R = f(-1)\n";
    let (v, diags) = eval(src, "R");
    assert!(
        has_exact(&diags, "boom -1"),
        "expected the interpolated message, got {diags:?}"
    );
    assert_eq!(v, Some(int(42)), "ensure must continue past the failed guard");
}

// ---- best-effort bad interpolation -------------------------------------

#[test]
fn bad_interpolation_still_diagnoses_no_crash() {
    let src = "module m\ncomptime fn f() -> int { ensure(false, \"val is {nonexistent_name}\")\n    return 0 }\nconst R = f()\n";
    let (_, diags) = eval(src, "R");
    assert!(!diags.is_empty(), "a failing guard must still emit at least one diagnostic");
}

// ---- arity / type errors -----------------------------------------------

#[test]
fn non_bool_condition_is_error() {
    let src = "module m\ncomptime fn f() -> int { ensure(1, \"x\")\n    return 0 }\nconst R = f()\n";
    let (_, diags) = eval(src, "R");
    assert!(
        diags.iter().any(|d| d.message.contains("must be bool")),
        "expected a bool-condition error, got {diags:?}"
    );
}

#[test]
fn wrong_arity_is_error() {
    let src = "module m\ncomptime fn f() -> int { ensure(false)\n    return 0 }\nconst R = f()\n";
    let (_, diags) = eval(src, "R");
    assert!(
        diags.iter().any(|d| d.message.contains("expects 2 arguments")),
        "expected an arity error, got {diags:?}"
    );
}

// ---- corpus divisibility guard -----------------------------------------

#[test]
fn divisibility_guard_passes_silently() {
    let src = "module m\ncomptime fn deform(period: int) -> int { ensure(256 % period == 0, \"256 not divisible by period {period}\")\n    return period }\nconst R = deform(64)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(64)));
    assert!(diags.is_empty(), "a passing divisibility guard must be silent, got {diags:?}");
}

#[test]
fn divisibility_guard_fails_with_interpolated_period() {
    let src = "module m\ncomptime fn deform(period: int) -> int { ensure(256 % period == 0, \"256 not divisible by period {period}\")\n    return period }\nconst R = deform(60)\n";
    let (_, diags) = eval(src, "R");
    assert!(
        has_exact(&diags, "256 not divisible by period 60"),
        "expected the interpolated period, got {diags:?}"
    );
}

// ---- budget / recursion names the chain --------------------------------

#[test]
fn non_terminating_recursion_names_the_chain() {
    // The abort message must NAME the offending call chain (§6.7), not report an
    // opaque quota.
    let src = "module m\ncomptime fn spin(n: int) -> int { return spin(n + 1) }\nconst R = spin(0)\n";
    let (_, diags) = eval(src, "R");
    assert!(
        diags.iter().any(|d| d.message.contains("recursion too deep") && d.message.contains("spin")),
        "expected a recursion diagnostic naming `spin`, got {diags:?}"
    );
}
