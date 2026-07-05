//! End-to-end evaluation corpus (Spec 2, Plan 2 — T8): whole, realistic `.emp`
//! programs parsed via [`parse_str`] (asserting a clean parse) and evaluated via
//! [`eval_const`], pinning the final [`Value`] and diagnostics. These double as
//! the acceptance proof that the comptime evaluator works on complete programs
//! and as living documentation of what the evaluator can do today.
//!
//! Syntax notes carried into every program here:
//! - Array *parameter* types are sized: `[int; N]` (the `; len` is mandatory —
//!   `[int]` does not parse). Evaluation does not yet type-check argument length
//!   against `N` (that is Plan 3), so these are shape hints, not enforced sizes.
//! - `.method` postfix only parses on a *path* receiver, so builtins on a
//!   literal go through a named binding, a call form, or the free/pipe form.
use sigil_frontend_emp::eval::eval_const;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::Value;

/// Parse `src` (asserting a clean parse) and evaluate the const named `name`.
fn eval(src: &str, name: &str) -> (Option<Value>, Vec<sigil_span::Diagnostic>) {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    eval_const(&file, name)
}

/// Evaluate `name`, assert no diagnostics, and return the value.
fn ok(src: &str, name: &str) -> Value {
    let (v, diags) = eval(src, name);
    assert!(diags.is_empty(), "unexpected diagnostics for `{name}`: {diags:?}");
    v.expect("value")
}

fn int(n: i128) -> Value {
    Value::Int(n)
}

fn arr(ns: &[i128]) -> Value {
    Value::Array(ns.iter().copied().map(Value::Int).collect())
}

/// True iff some diagnostic's message is exactly `msg`.
fn has_exact(diags: &[sigil_span::Diagnostic], msg: &str) -> bool {
    diags.iter().any(|d| d.message == msg)
}

// ---- 1. recursion: factorial + fibonacci --------------------------------

#[test]
fn recursive_factorial_and_fibonacci() {
    // Proves fn calls + recursion + `if` + arithmetic compose in one program.
    let src = "\
module engine.math

comptime fn fact(n: int) -> int {
    if n <= 1 { return 1 }
    return n * fact(n - 1)
}

comptime fn fib(n: int) -> int {
    if n < 2 { return n }
    return fib(n - 1) + fib(n - 2)
}

const F5 = fact(5)
const FIB10 = fib(10)
";
    assert_eq!(ok(src, "F5"), int(120));
    assert_eq!(ok(src, "FIB10"), int(55));
}

// ---- 2. parallax-style monotonic fold (Appendix A, pure part) -----------
//
// The real "stateful macro replaced by a comptime fold": a mutable accumulator
// (`comptime var`) walked by a side-effecting `for`, with an `ensure` guard
// enforcing a strict-monotonic invariant and interpolating the offending pair.
// This is the pure skeleton — no `as.sin`/`Data`, which are Plan 4/5.

const MONOTONIC_SRC: &str = "\
module engine.parallax

comptime fn max_cell(cells: [int; 4]) -> int {
    comptime var prev = -1
    for c in cells {
        ensure(c > prev, \"cells must strictly increase: {c} after {prev}\")
        prev = c
    }
    return prev
}

const M = max_cell([0, 4, 9, 20])
const BAD = max_cell([0, 4, 4])
";

#[test]
fn monotonic_fold_accepts_increasing_cells() {
    // A strictly-increasing input passes every guard silently; the fold's final
    // accumulator (the last, largest cell) is returned.
    assert_eq!(ok(MONOTONIC_SRC, "M"), int(20));
}

#[test]
fn monotonic_fold_reports_the_offending_pair() {
    // `[0, 4, 4]` violates the invariant at the repeated `4`. The failing
    // (non-fatal) `ensure` interpolates the exact pair and the loop continues,
    // so the fn still returns its accumulator (`4`) alongside the diagnostic —
    // matching the tested `ensure` semantics (a plain `ensure` poisons the guard
    // expression but does not abort the fn; `ensure_fatal` would).
    let (v, diags) = eval(MONOTONIC_SRC, "BAD");
    assert!(
        has_exact(&diags, "cells must strictly increase: 4 after 4"),
        "expected the interpolated monotonicity error, got {diags:?}"
    );
    assert_eq!(v, Some(int(4)), "non-fatal ensure continues, so the accumulator is returned");
}

// ---- 3. deform_sine guard + table-shape subset (Appendix A) -------------
//
// The layout/guard skeleton of Appendix A's `deform_sine`: a divisibility guard
// followed by a `for`-as-array building the table. The real table samples
// `as.sin`/`as.int` (Plan 5); here the body is plain integer arithmetic so the
// SHAPE and GUARD are exercised end-to-end today.

const DEFORM_SRC: &str = "\
module engine.parallax

comptime fn deform_shape(amplitude: int, period: int) -> [int; 8] {
    ensure(256 % period == 0, \"deform_sine: 256 not divisible by period {period}\")
    return for i in 0..8 { amplitude * i / period }
}

const T = deform_shape(amplitude: 64, period: 64)
const BADT = deform_shape(amplitude: 20, period: 60)
";

#[test]
fn deform_shape_builds_the_table_when_period_divides() {
    // amplitude=64, period=64: `64 * i / 64` == i for i in 0..8. Guard passes.
    assert_eq!(ok(DEFORM_SRC, "T"), arr(&[0, 1, 2, 3, 4, 5, 6, 7]));
}

#[test]
fn deform_shape_guard_rejects_indivisible_period() {
    // period=60 does not divide 256, so the guard fires with the interpolated
    // period (named args prove positional/named binding into the guarded fn).
    let (_, diags) = eval(DEFORM_SRC, "BADT");
    assert!(
        has_exact(&diags, "deform_sine: 256 not divisible by period 60"),
        "expected the divisibility diagnostic, got {diags:?}"
    );
}

// ---- 4. functional pipeline: lambdas + builtins + pipe ------------------

#[test]
fn functional_pipeline_with_lambdas_and_pipe() {
    // squares [1,4,9,16] -> keep >4 -> [9,16] -> sum -> 25.
    let src = "\
module engine.pipe

comptime fn pipeline(xs: [int; 4]) -> int {
    return xs |> map(|x| x * x) |> filter(|x| x > 4) |> fold(0, |a, b| a + b)
}

const P = pipeline([1, 2, 3, 4])
";
    assert_eq!(ok(src, "P"), int(25));
}

#[test]
fn functional_pipeline_with_named_fn_refs() {
    // The same pipeline, but each stage is a first-class `comptime fn` reference
    // instead of a lambda — proves fn-refs feed map/filter/fold identically.
    let src = "\
module engine.pipe

comptime fn sq(x: int) -> int { return x * x }
comptime fn keep(x: int) -> bool { return x > 4 }
comptime fn plus(a: int, b: int) -> int { return a + b }

comptime fn pipeline_fns(xs: [int; 4]) -> int {
    return xs |> map(sq) |> filter(keep) |> fold(0, plus)
}

const PF = pipeline_fns([1, 2, 3, 4])
";
    assert_eq!(ok(src, "PF"), int(25));
}

// ---- 5. const dependency graph ------------------------------------------

#[test]
fn const_dependency_graph_resolves_in_order() {
    // Consts reference each other and a fn; lazy resolution walks the graph in
    // dependency order. BASE=10, SCALED=40, fact(4)=24, RESULT=add(40,24)=64,
    // and DOUBLED=RESULT*2=128.
    let src = "\
module engine.deps

comptime fn add(a: int, b: int) -> int { return a + b }
comptime fn fact(n: int) -> int {
    if n <= 1 { return 1 }
    return n * fact(n - 1)
}

const BASE = 10
const SCALED = BASE * 4
const RESULT = add(SCALED, fact(4))
const DOUBLED = RESULT * 2
";
    assert_eq!(ok(src, "BASE"), int(10));
    assert_eq!(ok(src, "SCALED"), int(40));
    assert_eq!(ok(src, "RESULT"), int(64));
    assert_eq!(ok(src, "DOUBLED"), int(128));
}

// ---- 6. string processing: find / slice / len / val ---------------------

#[test]
fn string_processing_key_value_parse() {
    // Parse the integer tail of a `"key=1234"` pair by composing the string
    // builtins: find the `=`, slice everything after it, then `val` the tail.
    let src = "\
module engine.strings

comptime fn parse_val(s: str) -> int {
    let eq = s.find(\"=\")
    let tail = s.slice(eq + 1, s.len)
    return tail.val()
}

const KV = parse_val(\"key=1234\")
const HEX = parse_val(\"addr=$ff\")
";
    // "key=1234": '=' at char 3, tail "1234" -> 1234.
    assert_eq!(ok(src, "KV"), int(1234));
    // "addr=$ff": '=' at char 4, tail "$ff" -> val parses `$`-hex -> 255.
    assert_eq!(ok(src, "HEX"), int(255));
}
