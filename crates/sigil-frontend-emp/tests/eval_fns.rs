//! Integration tests for comptime-fn calls, the statement executor, and
//! `if`/`else` (Spec 2, Plan 2 — T4). Each case parses a full `.emp` file
//! (asserting a clean parse), then evaluates a named `const` whose value calls
//! a `comptime fn`, and asserts on the resulting [`Value`] / diagnostics.
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

// ---- simple calls ------------------------------------------------------

#[test]
fn simple_call_returns_value() {
    let src = "module m\ncomptime fn add(a: int, b: int) -> int { return a + b }\nconst R = add(2, 3)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(5)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn let_binding_in_body() {
    let src = "module m\ncomptime fn f(x: int) -> int {\n    let y = x * 2\n    return y + 1\n}\nconst R = f(4)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(9)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- if / else ---------------------------------------------------------

#[test]
fn statement_if_with_return() {
    // `if a > b { return a } return b` — statement-position if with early return.
    let src = "module m\ncomptime fn max(a: int, b: int) -> int {\n    if a > b { return a }\n    return b\n}\nconst HI = max(3, 7)\nconst LO = max(9, 2)\n";
    let (hi, d1) = eval(src, "HI");
    assert_eq!(hi, Some(int(7)));
    assert!(d1.is_empty(), "unexpected diagnostics: {d1:?}");
    let (lo, d2) = eval(src, "LO");
    assert_eq!(lo, Some(int(9)));
    assert!(d2.is_empty(), "unexpected diagnostics: {d2:?}");
}

#[test]
fn expression_position_if_as_let_value() {
    // `let r = if a > b { a } else { b }` — if in expression position, trailing
    // expr is the branch value.
    let src = "module m\ncomptime fn max(a: int, b: int) -> int {\n    let r = if a > b { a } else { b }\n    return r\n}\nconst R = max(4, 11)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(11)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn expression_position_if_with_return_in_branch() {
    // A `return` inside an expression-position if must still exit the fn.
    let src = "module m\ncomptime fn f(x: int) -> int {\n    let y = if x > 0 { return 100 } else { 0 }\n    return y + 1\n}\nconst R = f(5)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(100)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn if_condition_must_be_bool() {
    // A non-bool condition is an error; the (poisoned) statement-position `if`
    // then contributes nothing and execution falls through to `return 2`. The
    // emitted diagnostic still fails the build.
    let src = "module m\ncomptime fn f(x: int) -> int {\n    if x { return 1 }\n    return 2\n}\nconst R = f(5)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(2)));
    assert!(
        diags.iter().any(|d| d.message.contains("must be bool")),
        "diagnostics were {diags:?}"
    );
}

// ---- named / positional args ------------------------------------------

#[test]
fn named_args_out_of_order() {
    let src = "module m\ncomptime fn add(a: int, b: int) -> int { return a + b }\nconst R = add(b: 10, a: 1)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(11)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn positional_then_named_args() {
    let src = "module m\ncomptime fn add(a: int, b: int) -> int { return a + b }\nconst R = add(1, b: 2)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(3)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- arg-binding errors -----------------------------------------------

#[test]
fn missing_argument_errors() {
    let src = "module m\ncomptime fn add(a: int, b: int) -> int { return a + b }\nconst R = add(1)\n";
    let (_, diags) = eval(src, "R");
    assert!(
        diags.iter().any(|d| d.message.contains("missing argument")),
        "diagnostics were {diags:?}"
    );
}

#[test]
fn too_many_arguments_errors() {
    let src = "module m\ncomptime fn add(a: int, b: int) -> int { return a + b }\nconst R = add(1, 2, 3)\n";
    let (_, diags) = eval(src, "R");
    assert!(
        diags.iter().any(|d| d.message.contains("too many arguments")),
        "diagnostics were {diags:?}"
    );
}

#[test]
fn unknown_named_parameter_errors() {
    let src = "module m\ncomptime fn add(a: int, b: int) -> int { return a + b }\nconst R = add(a: 1, c: 2)\n";
    let (_, diags) = eval(src, "R");
    assert!(
        diags.iter().any(|d| d.message.contains("unknown named parameter")),
        "diagnostics were {diags:?}"
    );
}

#[test]
fn duplicate_argument_errors() {
    // `a` filled positionally then again by name.
    let src = "module m\ncomptime fn add(a: int, b: int) -> int { return a + b }\nconst R = add(1, a: 2)\n";
    let (_, diags) = eval(src, "R");
    assert!(
        diags.iter().any(|d| d.message.contains("more than once")),
        "diagnostics were {diags:?}"
    );
}

// ---- D-PP.4: named call arguments (Spec 2, Plan 7 — pitcher_plant tranche) --
//
// The paren-form binder above (`named_args_out_of_order` /
// `positional_then_named_args` / the four error tests) predates this tranche
// (T4/T6). D-PP.4's new rule is the one direction that was NOT yet rejected:
// a positional argument AFTER a named one. The tests below exercise that rule
// plus the tranche's motivating call shape (`spawn(SeedDef, offset: Vec{...},
// flip: inherit)`): struct-literal and enum-const named-arg VALUES, Reg as a
// named arg, and the "named arg to a non-comptime-fn callable" rejections
// (lambda, builtin) that must stay clean rather than silently going
// positional.

#[test]
fn positional_after_named_errors() {
    // `add(b: 2, 1)` — a positional arg trailing a named one. Today (pre
    // D-PP.4) this silently binds `1` into the first open slot (`a`),
    // producing 3 with zero diagnostics — exactly the silent misbinding
    // D-PP.4 forbids. Must be a loud, rule-naming error instead.
    let src = "module m\ncomptime fn add(a: int, b: int) -> int { return a + b }\nconst R = add(b: 2, 1)\n";
    let (_, diags) = eval(src, "R");
    assert!(
        diags.iter().any(|d| d.message.contains("positional") && d.message.contains("after")),
        "diagnostics were {diags:?}"
    );
}

#[test]
fn positional_after_named_does_not_silently_bind() {
    // Companion to the error-message assertion: the RESULT must not be the
    // silently-misbound value (3) — it must poison, proving the bad arg
    // never reaches a slot.
    let src = "module m\ncomptime fn add(a: int, b: int) -> int { return a + b }\nconst R = add(b: 2, 1)\n";
    let (v, _) = eval(src, "R");
    assert_ne!(v, Some(int(3)), "must not silently bind the trailing positional arg");
}

#[test]
fn spawn_shaped_call_positional_then_two_named() {
    // The tranche's motivating call shape: one positional arg, then two named
    // args whose VALUES are a struct literal and an enum const — ordinary
    // comptime expressions, per D-PP.4.
    let src = "module m\n\
        struct Vec { x: i16, y: i16 }\n\
        enum Flip: u8 { none = 0, inherit = 1 }\n\
        pub const inherit: Flip = Flip.inherit\n\
        comptime fn spawn(def: int, offset: Vec, flip: Flip) -> int {\n\
        \x20   return def + offset.x + offset.y\n\
        }\n\
        const R = spawn(7, offset: Vec{ x: -16, y: -4 }, flip: inherit)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(7 - 16 - 4)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn all_named_args_any_order() {
    let src = "module m\n\
        struct Vec { x: i16, y: i16 }\n\
        comptime fn spawn(def: int, offset: Vec) -> int { return def + offset.x }\n\
        const R = spawn(offset: Vec{ x: 5, y: 0 }, def: 1)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(6)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn byte_identical_named_vs_positional() {
    // `f(1, k: 2)` and `f(1, 2)` must bind to the exact same values — named
    // args are sugar over the same positional slots, not a distinct path.
    let src_named = "module m\ncomptime fn add(a: int, b: int) -> int { return a + b }\nconst R = add(1, b: 2)\n";
    let src_pos = "module m\ncomptime fn add(a: int, b: int) -> int { return a + b }\nconst R = add(1, 2)\n";
    let (v_named, d_named) = eval(src_named, "R");
    let (v_pos, d_pos) = eval(src_pos, "R");
    assert_eq!(v_named, v_pos);
    assert_eq!(v_named, Some(int(3)));
    assert!(d_named.is_empty() && d_pos.is_empty());
}

#[test]
fn no_default_values_missing_named_param_still_errors() {
    // D-PP.4 explicitly builds no defaults: leaving a param unbound (even one
    // that was never mentioned positionally OR by name) is still "missing
    // argument", exactly like the existing all-positional case.
    let src = "module m\ncomptime fn add(a: int, b: int) -> int { return a + b }\nconst R = add(a: 1)\n";
    let (_, diags) = eval(src, "R");
    assert!(
        diags.iter().any(|d| d.message.contains("missing argument") && d.message.contains("b")),
        "diagnostics were {diags:?}"
    );
}

#[test]
fn named_arg_duplicate_named_and_named() {
    let src = "module m\ncomptime fn add(a: int, b: int) -> int { return a + b }\nconst R = add(a: 1, a: 2, b: 3)\n";
    let (_, diags) = eval(src, "R");
    assert!(
        diags.iter().any(|d| d.message.contains("more than once")),
        "diagnostics were {diags:?}"
    );
}

#[test]
fn named_reg_arg_binds_like_positional_reg_arg() {
    // Reg is a comptime-only param type (D-PP.2); named-arg binding must
    // route the same bareword-wins-as-register rule through, not just the
    // positional path. `facing_abs(r: d3)` must equal `facing_abs(d3)`.
    let src_named = "module m\ncomptime fn pick(r: Reg) -> int { return 1 }\nconst R = pick(r: d3)\n";
    let src_pos = "module m\ncomptime fn pick(r: Reg) -> int { return 1 }\nconst R = pick(d3)\n";
    let (v_named, d_named) = eval(src_named, "R");
    let (v_pos, _) = eval(src_pos, "R");
    assert_eq!(v_named, Some(int(1)));
    assert_eq!(v_named, v_pos);
    assert!(d_named.is_empty(), "unexpected diagnostics: {d_named:?}");
}

#[test]
fn named_arg_to_lambda_is_a_clean_error() {
    // Lambdas have positional-only params (no name list to bind against) —
    // a named arg there must be a clean, specific diagnostic, not silent
    // positional treatment.
    let src = "module m\n\
        comptime fn go() -> int {\n\
        \x20   let f = |x| x + 1\n\
        \x20   return f(x: 10)\n\
        }\n\
        const R = go()\n";
    let (_, diags) = eval(src, "R");
    assert!(
        diags.iter().any(|d| d.message.contains("positional")),
        "diagnostics were {diags:?}"
    );
}

#[test]
fn named_arg_to_builtin_is_rejected() {
    // Builtins (map/filter/fold/...) take positional args only.
    let src = "module m\n\
        comptime fn go() -> int {\n\
        \x20   let xs = [1, 2, 3]\n\
        \x20   return xs.fold(init: 0, f: |a, b| a + b)\n\
        }\n\
        const R = go()\n";
    let (_, diags) = eval(src, "R");
    assert!(
        diags.iter().any(|d| d.message.contains("positional")),
        "diagnostics were {diags:?}"
    );
}

#[test]
fn unknown_function_errors() {
    let src = "module m\nconst R = nope(1)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("unknown function")),
        "diagnostics were {diags:?}"
    );
}

// ---- recursion ---------------------------------------------------------

#[test]
fn recursion_that_terminates() {
    let src = "module m\ncomptime fn fact(n: int) -> int {\n    if n <= 1 { return 1 }\n    return n * fact(n - 1)\n}\nconst R = fact(5)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(120)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn fibonacci_terminates() {
    let src = "module m\ncomptime fn fib(n: int) -> int {\n    if n < 2 { return n }\n    return fib(n - 1) + fib(n - 2)\n}\nconst R = fib(10)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(55)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn non_terminating_recursion_is_bounded_not_a_crash() {
    // MUST NOT stack-overflow the test process. Depth bound (D-P2.16) turns
    // unbounded recursion into a Poison result plus a named diagnostic.
    let src = "module m\ncomptime fn spin(n: int) -> int { return spin(n + 1) }\nconst R = spin(0)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(Value::Poison));
    assert!(!diags.is_empty(), "expected a diagnostic naming the runaway chain");
    assert!(
        diags.iter().any(|d| d.message.contains("spin")
            || d.message.contains("recursion")
            || d.message.contains("budget")),
        "diagnostics were {diags:?}"
    );
}

#[test]
fn return_in_argument_position_exits_caller() {
    // Regression (T4 review): a `return` inside a call argument belongs to the
    // CALLER, not the callee. `return 7` must exit `outer`, so R = 7 — not 1007
    // (callee's body must never steal the caller's pending return).
    let src = "module m\n\
        comptime fn callee(x: int) -> int { return x + 100 }\n\
        comptime fn outer(c: bool) -> int {\n\
        \x20   let r = callee(if c { return 7 } else { 2 })\n\
        \x20   return r + 1000\n\
        }\n\
        const R = outer(true)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(7)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn sequential_early_returning_calls_do_not_leak() {
    // Calling an early-returning fn twice and summing must not leak a return
    // across the call boundary: inner(1)=1, inner(2)=2, so R = 3.
    let src = "module m\n\
        comptime fn inner(x: int) -> int {\n\
        \x20   if x > 0 { return x }\n\
        \x20   return 0\n\
        }\n\
        comptime fn outer() -> int { return inner(1) + inner(2) }\n\
        const R = outer()\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(3)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn deep_abort_chain_is_truncated() {
    // The depth abort fires with 512 `spin` frames on the call stack; the
    // message names only the innermost MAX_CHAIN_FRAMES, prefixed with `...`.
    let src = "module m\ncomptime fn spin(n: int) -> int { return spin(n + 1) }\nconst R = spin(0)\n";
    let (_, diags) = eval(src, "R");
    let msg = &diags
        .iter()
        .find(|d| d.message.contains("recursion too deep"))
        .expect("expected a recursion-depth diagnostic")
        .message;
    assert!(msg.contains("... ->"), "chain not truncated: {msg:?}");
}

// ---- last-expr block value --------------------------------------------

#[test]
fn last_expr_is_block_value_without_return() {
    let src = "module m\ncomptime fn f() -> int { 41 + 1 }\nconst R = f()\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(42)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- LetTuple ----------------------------------------------------------

#[test]
fn let_tuple_binds_elements() {
    let src = "module m\ncomptime fn f() -> int {\n    let (a, b) = (1, 2)\n    return a + b\n}\nconst R = f()\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(3)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn let_tuple_arity_mismatch_errors() {
    let src = "module m\ncomptime fn f() -> int {\n    let (a, b) = (1, 2, 3)\n    return a\n}\nconst R = f()\n";
    let (_, diags) = eval(src, "R");
    assert!(!diags.is_empty(), "expected a diagnostic");
    assert!(
        diags.iter().any(|d| d.message.contains("tuple")),
        "diagnostics were {diags:?}"
    );
}

#[test]
fn let_tuple_non_tuple_errors() {
    let src = "module m\ncomptime fn f() -> int {\n    let (a, b) = 5\n    return a\n}\nconst R = f()\n";
    let (_, diags) = eval(src, "R");
    assert!(!diags.is_empty(), "expected a diagnostic");
    assert!(
        diags.iter().any(|d| d.message.contains("tuple")),
        "diagnostics were {diags:?}"
    );
}

// ---- default parameter values (t14 demanded feature) ------------------

#[test]
fn default_param_used_when_omitted() {
    let src = "module m\ncomptime fn f(a: int, b: int = 10) -> int { return a + b }\nconst R = f(5)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(15)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn default_param_overridden_positionally() {
    let src = "module m\ncomptime fn f(a: int, b: int = 10) -> int { return a + b }\nconst R = f(5, 3)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(8)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn default_param_overridden_by_name() {
    let src = "module m\ncomptime fn f(a: int, b: int = 10) -> int { return a + b }\nconst R = f(5, b: 3)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(8)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn required_param_missing_still_errors_but_default_does_not() {
    // `a` has no default (required); `b` does. Omitting both: only `a` is a
    // missing-argument error — `b` silently takes its default.
    let src = "module m\ncomptime fn g(a: int, b: int = 1) -> int { return a + b }\nconst R = g()\n";
    let (_, diags) = eval(src, "R");
    assert!(
        diags.iter().any(|d| d.message.contains("missing argument `a`")),
        "expected missing-arg for `a`, diagnostics were {diags:?}"
    );
    assert!(
        !diags.iter().any(|d| d.message.contains("missing argument `b`")),
        "`b` has a default and must NOT be a missing-arg error: {diags:?}"
    );
}

#[test]
fn default_expr_resolves_a_module_const() {
    let src = "module m\nconst D = 7\ncomptime fn f(a: int, b: int = D) -> int { return a + b }\nconst R = f(5)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(12)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn multiple_defaults_partial_named_override() {
    let src = "module m\ncomptime fn f(a: int, b: int = 2, c: int = 3) -> int { return a * 100 + b * 10 + c }\nconst A = f(9)\nconst B = f(9, c: 7)\n";
    let (a, d1) = eval(src, "A");
    assert_eq!(a, Some(int(923)));
    assert!(d1.is_empty(), "unexpected diagnostics: {d1:?}");
    let (b, d2) = eval(src, "B");
    assert_eq!(b, Some(int(927)));
    assert!(d2.is_empty(), "unexpected diagnostics: {d2:?}");
}
